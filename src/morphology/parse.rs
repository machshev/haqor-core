//! Reverse morphology: parse a fully-pointed Old Testament word back into the
//! set of candidate analyses that could have produced it.
//!
//! Strategy is generate-and-test. The forward generator turns a triliteral
//! root into a full verb paradigm ([`generate_paradigm`]); we run that engine
//! in reverse by:
//!
//! 1. Peeling optional proclitic prefixes (conjunction, prepositions, article,
//!    relative) off the front of the word — each peeling is tried, including
//!    the no-prefix case, so attached and bare readings are both reported.
//! 2. Exhaustively enumerating candidate roots. The radicals are drawn from the
//!    consonants present in the (de-prefixed) surface form, plus the weak
//!    letters that routinely vanish from the surface (assimilated I-Nun,
//!    elided I-Yod/Vav, the dropped middle of a hollow root, an apocopated
//!    III-He, a quiescent aleph).
//! 3. Generating each candidate root's paradigm and keeping every form whose
//!    rendered text exactly equals the surface form.
//!
//! Because the test is exact-match against generated forms, over-generating
//! candidate roots is harmless: spurious roots simply never produce the
//! surface form and drop out. Homographs (one surface form, several legitimate
//! analyses) all survive, which is the point.
//!
//! Vav-consecutive forms are handled: weqatal (וְ + perfect) falls out of the
//! ordinary prefix peeling, and wayyiqtol (וַ + a forte dagesh on the prefix
//! consonant) is matched directly against the generated Wayyiqtol forms.
//! Strong-verb, III-He (via the apocopated jussive, e.g. וַיִּבֶן), hollow
//! (וַיָּקָם) and I-Aleph (וַיֹּאמֶר) wayyiqtol all match: the forward generator
//! models the stress-retraction (nesiga) vowel changes — hollow holam → qamats,
//! I-Aleph patah → segol.
//!
//! Limitations: only verbs are parsed. A noun's pattern (mishqal) cannot be
//! derived from its root, so the noun generator takes a stem rather than a
//! root and can't be driven from candidate roots. Plene/defective spelling
//! differences against the Masoretic text and ketiv/qere variation are not
//! modelled, so a form spelled differently from what the generator emits will
//! not match.

use std::collections::{HashMap, HashSet};

use rayon::prelude::*;

use super::hebrew::{self, Cons, Vowel, letter};
use super::root::Root;
use super::verb::{Binyan, Form, Paradigm, Pgn, generate_paradigm};

/// Weak letters that may be a radical hidden from the surface form: an
/// assimilated I-Nun, an elided I-Yod/Vav, the dropped middle radical of a
/// hollow root, an apocopated III-He, or a quiescent III/I-Aleph.
const WEAK: [char; 5] = [
    letter::VAV,
    letter::YOD,
    letter::NUN,
    letter::HE,
    letter::ALEF,
];

/// Single-consonant proclitics that attach to the front of a word: the
/// conjunction ו, the prepositions ב/כ/ל/מ, the article/interrogative ה, and
/// the relative ש.
const PROCLITICS: [char; 7] = [
    letter::VAV,
    letter::BET,
    letter::KAF,
    letter::LAMED,
    letter::MEM,
    letter::HE,
    letter::SHIN,
];

/// One candidate analysis of a surface word as an inflected verb form.
#[derive(Debug, Clone)]
pub struct VerbMatch {
    pub root: Root,
    pub binyan: Binyan,
    pub form: Form,
    pub pgn: Pgn,
    /// True if the matched form came from a fully-modelled (binyan, form,
    /// gizra) combination; false if it came from a strong-verb fallback.
    pub attested: bool,
    /// The proclitic prefix consumed before the stem, rendered to Hebrew
    /// (empty if the whole word was analysed as the stem).
    pub prefix: String,
    /// True when the prefix is a vav-consecutive: a wayyiqtol (וַ + dagesh,
    /// matched against the imperfect/jussive) or a weqatal (וְ + perfect).
    pub vav_consecutive: bool,
    /// The pronominal object suffix on the verb, if any (its person/gender/
    /// number). `None` when no object suffix was matched.
    pub object_suffix: Option<Pgn>,
}

/// The canonical key defining when two rendered forms count as the same
/// surface form: equal after collapsing a plene holam/shureq vav mater onto
/// the preceding consonant and stripping the sin/shin dot. [`forms_match`] is
/// key equality, and the [`ReverseIndex`] maps each generated form's key to
/// its analyses, turning reverse parsing from per-surface generate-and-test
/// into an O(1) lookup — both paths share this one normalisation, so they
/// cannot diverge. Any future orthographic relaxation belongs here and
/// nowhere else.
///
/// Why each relaxation is safe (recovers spelling variants without merging
/// distinct words):
/// - Plene holam (a vav mater carrying holam after a vowelless consonant)
///   collapses onto that consonant, so plene יוֹשֵׁב and defective יֹשֵׁב
///   normalise alike. Plene/defective is purely orthographic — same lexeme,
///   same analysis — and a holam mater is the sole thing removed.
/// - Plene shureq (a vav bearing the dagesh-as-shureq point, no vowel)
///   collapses to qubuts on the preceding consonant, so יָמוּתוּ and יָמֻתוּ
///   normalise alike. Orthographic for the same reason.
/// - The sin/shin dot is stripped because the generator always renders ש with
///   a shin dot (it has no lexical knowledge of which roots are sin), so a
///   sin-pointed surface — עָשָׂה, נָשָׂא, שָׂם — could never exact-match an
///   otherwise-identical generated form. Candidate roots already key on the
///   bare letter ש and roots are reported by their bare letters, so collapsing
///   the dot recovers every sin verb without admitting new roots.
pub(crate) fn canonical_key(form: &str) -> String {
    // Two sequential passes — holam then shureq, each re-parsing the previous
    // pass's output — then the sin/shin-dot strip.
    fn collapse(form: &str, shureq: bool) -> String {
        let mut out: Vec<Cons> = Vec::new();
        for c in hebrew::parse_pointed(form) {
            // holam pass: a vav bearing holam; shureq pass: a vav bearing the
            // dagesh-as-shureq point with no vowel.
            let is_mater = c.letter == letter::VAV
                && if shureq { c.dagesh && c.vowel.is_none() } else { c.vowel == Some(Vowel::Holam) };
            if is_mater {
                if let Some(prev) = out.last_mut() {
                    if prev.vowel.is_none() {
                        prev.vowel = Some(if shureq { Vowel::Qubuts } else { Vowel::Holam });
                        continue;
                    }
                }
            }
            out.push(c);
        }
        hebrew::render(&out)
    }
    let holam = collapse(form, false);
    let shureq = collapse(&holam, true);
    shureq.chars().filter(|&c| c != '\u{05C1}' && c != '\u{05C2}').collect()
}

/// Compare a generated form against the parse targets up to the orthographic
/// normalisation of [`canonical_key`]. Key equality is the single definition
/// of a match, shared with the [`ReverseIndex`], so the direct and indexed
/// parse paths cannot diverge. `target_keys` are the targets' precomputed
/// canonical keys — computed once per peeling, not once per generated form.
///
/// Two fast paths skip the allocation-heavy key computation for almost every
/// generated form: byte equality, and a first-letter guard — canonicalisation
/// never touches the first letter (a vav mater only collapses onto a
/// *previous* consonant, and the stripped sin/shin dot follows its letter),
/// so forms whose first letters differ can never share a key.
fn forms_match(generated: &str, targets: &[String], target_keys: &[String]) -> bool {
    if targets.iter().any(|t| t == generated) {
        return true;
    }
    let g0 = generated.chars().next();
    if !targets.iter().any(|t| t.chars().next() == g0) {
        return false;
    }
    let key = canonical_key(generated);
    target_keys.iter().any(|k| *k == key)
}

/// Candidate stem renderings for one proclitic peeling: the bare peeled
/// `remainder` plus the sandhi variants that restore a stem the proclitic
/// altered (conjunction וְ+yəC / וְ+hataf, a quiesced pe-aleph hataf, and the
/// article's forte dagesh). Shared by [`parse_word_filtered`] and
/// [`parse_word_indexed`] so both peel identically.
fn peeling_targets(seq: &[Cons], strip: usize, remainder: &[Cons]) -> Vec<String> {
    let mut targets = vec![hebrew::render(remainder)];
    // וְ + yəC → וִyC: a sheva-bearing yod quiesces to a mater after a hiriq-vav.
    if strip > 0
        && seq[strip - 1].letter == letter::VAV
        && seq[strip - 1].vowel == Some(Vowel::Hiriq)
        && remainder
            .first()
            .is_some_and(|c| c.letter == letter::YOD && c.vowel.is_none())
    {
        let mut alt = remainder.to_vec();
        alt[0].vowel = Some(Vowel::Sheva);
        targets.push(hebrew::render(&alt));
    }
    // וְ + Cˇ(hataf) → וִ + Cˇ(sheva): a guttural's hataf reduces after a hiriq-vav.
    if strip > 0
        && seq[strip - 1].letter == letter::VAV
        && seq[strip - 1].vowel == Some(Vowel::Hiriq)
        && remainder
            .first()
            .is_some_and(|c| hebrew::is_guttural(c.letter) && c.vowel == Some(Vowel::Sheva))
    {
        for hataf in [Vowel::HatafSegol, Vowel::HatafPatah] {
            let mut alt = remainder.to_vec();
            alt[0].vowel = Some(hataf);
            targets.push(hebrew::render(&alt));
        }
    }
    // Any proclitic onto a I-guttural stem may close that guttural's syllable on
    // a silent sheva where the generator writes a hataf (לַעְזֹר for la-ʕăzōr,
    // בְּעְבֹר): restore the hataf so the forms_match comparison succeeds.
    if strip > 0
        && remainder
            .first()
            .is_some_and(|c| hebrew::is_guttural(c.letter) && c.vowel == Some(Vowel::Sheva))
    {
        for hataf in [Vowel::HatafPatah, Vowel::HatafSegol, Vowel::HatafQamats] {
            let mut alt = remainder.to_vec();
            alt[0].vowel = Some(hataf);
            targets.push(hebrew::render(&alt));
        }
    }
    // A proclitic onto a pe-aleph stem swallows the aleph's hataf (לֵאמֹר): restore it.
    if strip > 0
        && remainder
            .first()
            .is_some_and(|c| c.letter == letter::ALEF && c.vowel.is_none())
    {
        for hataf in [Vowel::HatafPatah, Vowel::HatafSegol] {
            let mut alt = remainder.to_vec();
            alt[0].vowel = Some(hataf);
            targets.push(hebrew::render(&alt));
        }
    }
    // Dehiq / conjunctive doubling: a word whose first consonant is a
    // non-begedkefet letter carrying a dagesh forte (נַּעֲשֶׂה, נּוֹרָא, יּוֹסִיף)
    // was doubled by the preceding word's stress (the dagesh is never
    // etymological word-initially), so the generator never produces it. Strip
    // it and re-match the bare stem.
    if strip == 0
        && remainder
            .first()
            .is_some_and(|c| c.dagesh && !hebrew::is_begedkefet(c.letter))
    {
        let mut alt = remainder.to_vec();
        alt[0].dagesh = false;
        targets.push(hebrew::render(&alt));
    }
    // The article / relative doubles the first stem consonant; strip that dagesh.
    if strip > 0 && remainder.first().is_some_and(|c| c.dagesh) {
        let mut alt = remainder.to_vec();
        alt[0].dagesh = false;
        targets.push(hebrew::render(&alt));
    }
    // After a proclitic prefix, a begedkefet consonant in the stem may carry
    // a dagesh lene that the generator does not produce (e.g. לִזְבֹּחַ vs
    // לִזְבֹחַ). Emit a variant with all begedkefet dageshes stripped so the
    // forms_match comparison can still succeed.
    if strip > 0 {
        let mut alt = remainder.to_vec();
        let mut changed = false;
        for c in &mut alt {
            if hebrew::is_begedkefet(c.letter) && c.dagesh {
                c.dagesh = false;
                changed = true;
            }
        }
        if changed {
            targets.push(hebrew::render(&alt));
        }
    }
    targets
}

/// Stable ordering shared by all parse entry points: attested analyses first,
/// bare forms before object-suffixed, then by root/binyan/form/pgn/suffix.
fn sort_matches(matches: &mut [VerbMatch]) {
    matches.sort_by(|a, b| {
        b.attested
            .cmp(&a.attested)
            .then_with(|| a.object_suffix.is_some().cmp(&b.object_suffix.is_some()))
            .then_with(|| a.root.letters.cmp(&b.root.letters))
            .then_with(|| (a.binyan as usize).cmp(&(b.binyan as usize)))
            .then_with(|| (a.form as usize).cmp(&(b.form as usize)))
            .then_with(|| a.pgn.label().cmp(&b.pgn.label()))
            .then_with(|| {
                a.object_suffix
                    .map(|p| p.label())
                    .cmp(&b.object_suffix.map(|p| p.label()))
            })
    });
}

/// Parse a fully-pointed word into every verb analysis that can produce it.
pub fn parse_word(word: &str) -> Vec<VerbMatch> {
    parse_word_filtered(word, None)
}

/// Like [`parse_word`], but restrict candidate roots to those in `roots` (the
/// real-root inventory). With a filter, the generate-and-test enumeration skips
/// roots the lexicon doesn't attest, which both prunes spurious homograph
/// analyses and speeds parsing. Passing `None` enumerates every triliteral, the
/// behaviour of [`parse_word`].
pub fn parse_word_filtered(word: &str, roots: Option<&HashSet<[char; 3]>>) -> Vec<VerbMatch> {
    let seq = hebrew::parse_pointed(word);
    let mut matches = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut memo: HashMap<[char; 3], Paradigm> = HashMap::new();

    // Peel 0, 1, or 2 leading proclitics. We always keep at least two
    // consonants of stem so there is something to analyse as a verb.
    let max_strip = 2usize.min(seq.len().saturating_sub(2));
    for strip in 0..=max_strip {
        if seq[..strip].iter().any(|c| !PROCLITICS.contains(&c.letter)) {
            continue;
        }
        let remainder = &seq[strip..];
        let prefix = hebrew::render(&seq[..strip]);
        let targets = peeling_targets(&seq, strip, remainder);
        let target_keys: Vec<String> = targets.iter().map(|t| canonical_key(t)).collect();

        for letters in candidate_roots(remainder, roots) {
            let paradigm = memo
                .entry(letters)
                .or_insert_with(|| generate_paradigm(&Root::from_letters(letters)));
            for vf in &paradigm.forms {
                // Wayyiqtol is handled by the dedicated pass below, with the
                // consecutive vav recognised as such rather than as a plain
                // proclitic; skip it here to avoid duplicate analyses.
                if vf.form == Form::Wayyiqtol
                    || !forms_match(&vf.text, &targets, &target_keys)
                {
                    continue;
                }
                if seen.insert((
                    letters,
                    vf.binyan,
                    vf.form,
                    vf.pgn,
                    vf.object_suffix,
                    strip,
                    false,
                )) {
                    matches.push(VerbMatch {
                        root: Root::from_letters(letters),
                        binyan: vf.binyan,
                        form: vf.form,
                        pgn: vf.pgn,
                        attested: vf.attested,
                        prefix: prefix.clone(),
                        vav_consecutive: false,
                        object_suffix: vf.object_suffix,
                    });
                }
            }
        }
    }

    // Wayyiqtol (vav-consecutive imperfect): the generator emits these as a
    // dedicated form, וַ prefix and forte dagesh included, so we match the whole
    // word against the Wayyiqtol forms directly. Candidate roots come from the
    // consonants after the consecutive vav. Weqatal (וְ + perfect) needs no
    // special handling — it surfaces above as a plain vav prefix on a perfect.
    if seq.first().is_some_and(|c| {
        c.letter == letter::VAV && matches!(c.vowel, Some(Vowel::Patah | Vowel::Qamats))
    }) && seq.len() >= 3
    {
        let targets = [hebrew::render(&seq)];
        let target_keys = [canonical_key(&targets[0])];
        for letters in candidate_roots(&seq[1..], roots) {
            let paradigm = memo
                .entry(letters)
                .or_insert_with(|| generate_paradigm(&Root::from_letters(letters)));
            for vf in &paradigm.forms {
                if vf.form != Form::Wayyiqtol || !forms_match(&vf.text, &targets, &target_keys) {
                    continue;
                }
                if seen.insert((
                    letters,
                    vf.binyan,
                    vf.form,
                    vf.pgn,
                    vf.object_suffix,
                    0,
                    true,
                )) {
                    matches.push(VerbMatch {
                        root: Root::from_letters(letters),
                        binyan: vf.binyan,
                        form: vf.form,
                        pgn: vf.pgn,
                        attested: vf.attested,
                        prefix: String::new(),
                        vav_consecutive: true,
                        object_suffix: vf.object_suffix,
                    });
                }
            }
        }
    }

    sort_matches(&mut matches);
    matches
}

/// One analysis in the reverse index — the surface-independent part of a
/// [`VerbMatch`] (everything but the peeled prefix and vav-consecutive flag,
/// which are decided at lookup time).
///
/// The index holds ~54M of these (every form of every triliteral root), so the
/// struct is kept as small as possible: the three radicals are stored as their
/// index into [`HEBREW_CONSONANTS`] (a single byte each) rather than as `char`
/// (four bytes each), cutting the struct from 24 to 13 bytes and saving roughly
/// half a gigabyte across the whole index. [`IndexEntry::letters`] rebuilds the
/// `[char; 3]` on demand at lookup time.
#[derive(Clone)]
struct IndexEntry {
    radicals: [u8; 3],
    binyan: Binyan,
    form: Form,
    pgn: Pgn,
    object_suffix: Option<Pgn>,
    attested: bool,
}

impl IndexEntry {
    /// The radical letters as Hebrew `char`s, decoded from the packed indices.
    fn letters(&self) -> [char; 3] {
        self.radicals.map(|i| HEBREW_CONSONANTS[i as usize])
    }
}

/// Pack a triliteral root's `char` radicals into their [`HEBREW_CONSONANTS`]
/// indices. Every root indexed comes from that array, so each radical is always
/// found.
fn pack_radicals(letters: [char; 3]) -> [u8; 3] {
    letters.map(|c| {
        HEBREW_CONSONANTS
            .iter()
            .position(|&h| h == c)
            .expect("root radical is a base Hebrew consonant") as u8
    })
}

/// The 22 base Hebrew consonants (no final forms) — the radical alphabet.
const HEBREW_CONSONANTS: [char; 22] = [
    letter::ALEF, letter::BET, letter::GIMEL, letter::DALET, letter::HE, letter::VAV,
    letter::ZAYIN, letter::HET, letter::TET, letter::YOD, letter::KAF, letter::LAMED,
    letter::MEM, letter::NUN, letter::SAMEKH, letter::AYIN, letter::PE, letter::TSADE,
    letter::QOF, letter::RESH, letter::SHIN, letter::TAV,
];

/// A reverse lookup index mapping each generated form's [`canonical_key`] to the
/// analyses that produce it. Built once over every triliteral root, it turns
/// reverse parsing from per-surface generate-and-test (O(surfaces × roots ×
/// paradigm)) into O(surfaces × log N) lookups. Use [`parse_word_indexed`] to
/// query.
///
/// The index holds ~54M form entries over ~34M distinct keys. A
/// `HashMap<_, Vec<IndexEntry>>` would pay for 34M separate per-key `Vec`
/// allocations, a hashtable rounded up to the next power of two (~1.7× slack at
/// this size), and a fragmenting parallel build. Instead the entries live in one
/// flat `Vec` sorted by key hash: no per-key allocation, no table slack, and a
/// lookup is a binary search for the key's run. Resting footprint is just the
/// 54M packed entries (~1.7 GB) rather than the multi-gigabyte `HashMap`.
pub struct ReverseIndex {
    /// `(key_hash, entry)` pairs sorted by `key_hash`. All entries sharing a key
    /// form a contiguous run, found by [`ReverseIndex::get`].
    entries: Vec<(u128, IndexEntry)>,
}

/// A 128-bit hash of a [`canonical_key`], used as the [`ReverseIndex`] key in
/// place of the owned `String`. Two independently-seeded 64-bit SipHashes are
/// concatenated; `DefaultHasher::new` uses fixed keys, so the hash is
/// deterministic across runs. Collisions across the index's ~34M distinct keys
/// are vanishingly improbable in a 128-bit space.
fn key_hash(canonical: &str) -> u128 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut hi = DefaultHasher::new();
    canonical.hash(&mut hi);
    let mut lo = DefaultHasher::new();
    // A distinct salt so the two halves are independent.
    lo.write_u8(0x9e);
    canonical.hash(&mut lo);
    ((hi.finish() as u128) << 64) | (lo.finish() as u128)
}

impl ReverseIndex {
    /// Generate the paradigm of every one of the 22³ triliteral roots and index
    /// every form by its canonical key. The all-roots space is a superset of any
    /// surface's candidate roots, and a non-matching root simply contributes keys
    /// no surface hits — so lookups return exactly what [`parse_word`] would find.
    ///
    /// Each form's key is a 128-bit hash of its [`canonical_key`] (see
    /// [`key_hash`]): 16 inline bytes, no allocation, and at 128 bits an
    /// accidental collision across the ~34M distinct keys is astronomically
    /// unlikely (~1e-24), so lookups return exactly what a string-keyed index
    /// would.
    ///
    /// Paradigm generation runs in parallel (one `Vec` of `(key, entry)` pairs
    /// per root); the per-root vectors are then concatenated into the single flat
    /// `entries` vector and sorted by key. The concatenation frees each root's
    /// vector as it is consumed, so peak build memory is roughly the flat vector
    /// plus the still-pending per-root vectors, not a multiple of the final size.
    pub fn build() -> Self {
        let mut roots: Vec<[char; 3]> = Vec::with_capacity(22 * 22 * 22);
        for &a in &HEBREW_CONSONANTS {
            for &b in &HEBREW_CONSONANTS {
                for &c in &HEBREW_CONSONANTS {
                    roots.push([a, b, c]);
                }
            }
        }
        let per_root: Vec<Vec<(u128, IndexEntry)>> = roots
            .par_iter()
            .map(|&letters| {
                let radicals = pack_radicals(letters);
                let para = generate_paradigm(&Root::from_letters(letters));
                para.forms
                    .iter()
                    .map(|vf| {
                        (
                            key_hash(&canonical_key(&vf.text)),
                            IndexEntry {
                                radicals,
                                binyan: vf.binyan,
                                form: vf.form,
                                pgn: vf.pgn,
                                object_suffix: vf.object_suffix,
                                attested: vf.attested,
                            },
                        )
                    })
                    .collect()
            })
            .collect();

        let total: usize = per_root.iter().map(Vec::len).sum();
        let mut entries: Vec<(u128, IndexEntry)> = Vec::with_capacity(total);
        for chunk in per_root {
            entries.extend(chunk);
        }
        entries.par_sort_unstable_by_key(|&(k, _)| k);
        ReverseIndex { entries }
    }

    /// The `(key, analysis)` pairs whose key hash equals `key`, as a contiguous
    /// run of the sorted `entries`. Empty when no form hashes to `key`. Callers
    /// read the [`IndexEntry`] from each pair's `.1`.
    fn get(&self, key: u128) -> &[(u128, IndexEntry)] {
        let start = self.entries.partition_point(|&(k, _)| k < key);
        let mut end = start;
        while end < self.entries.len() && self.entries[end].0 == key {
            end += 1;
        }
        &self.entries[start..end]
    }
}

/// Index-backed twin of [`parse_word_filtered`]: identical proclitic peeling,
/// sandhi target variants, wayyiqtol handling, dedup and ordering, but each
/// candidate-root enumeration + paradigm match is replaced by a hash lookup of
/// the target's [`canonical_key`]. With the same `roots` filter it returns the
/// same analyses as `parse_word_filtered`, far faster in bulk.
pub fn parse_word_indexed(
    word: &str,
    index: &ReverseIndex,
    roots: Option<&HashSet<[char; 3]>>,
) -> Vec<VerbMatch> {
    let seq = hebrew::parse_pointed(word);
    let mut matches = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let in_filter = |letters: &[char; 3]| roots.is_none_or(|set| set.contains(letters));

    let max_strip = 2usize.min(seq.len().saturating_sub(2));
    for strip in 0..=max_strip {
        if seq[..strip].iter().any(|c| !PROCLITICS.contains(&c.letter)) {
            continue;
        }
        let remainder = &seq[strip..];
        let prefix = hebrew::render(&seq[..strip]);
        let targets = peeling_targets(&seq, strip, remainder);

        for t in &targets {
            for (_, e) in index.get(key_hash(&canonical_key(t))) {
                let letters = e.letters();
                // Wayyiqtol is handled by the dedicated pass below.
                if e.form == Form::Wayyiqtol || !in_filter(&letters) {
                    continue;
                }
                if seen.insert((letters, e.binyan, e.form, e.pgn, e.object_suffix, strip, false)) {
                    matches.push(VerbMatch {
                        root: Root::from_letters(letters),
                        binyan: e.binyan,
                        form: e.form,
                        pgn: e.pgn,
                        attested: e.attested,
                        prefix: prefix.clone(),
                        vav_consecutive: false,
                        object_suffix: e.object_suffix,
                    });
                }
            }
        }
    }

    // Wayyiqtol: the generator emits the וַ + forte dagesh in the form itself, so
    // the whole word's key hits the Wayyiqtol entries directly.
    if seq.first().is_some_and(|c| {
        c.letter == letter::VAV && matches!(c.vowel, Some(Vowel::Patah | Vowel::Qamats))
    }) && seq.len() >= 3
    {
        let key = key_hash(&canonical_key(&hebrew::render(&seq)));
        for (_, e) in index.get(key) {
            let letters = e.letters();
            if e.form != Form::Wayyiqtol || !in_filter(&letters) {
                continue;
            }
            if seen.insert((letters, e.binyan, e.form, e.pgn, e.object_suffix, 0, true)) {
                matches.push(VerbMatch {
                    root: Root::from_letters(letters),
                    binyan: e.binyan,
                    form: e.form,
                    pgn: e.pgn,
                    attested: e.attested,
                    prefix: String::new(),
                    vav_consecutive: true,
                    object_suffix: e.object_suffix,
                });
            }
        }
    }

    sort_matches(&mut matches);
    matches
}

/// Disambiguate-only filter: parse unrestricted, then drop candidates whose
/// root the inventory doesn't attest — but only when that *leaves* at least one
/// analysis and there were several to begin with. A lone analysis is kept even
/// if its root is absent (the inventory can't disambiguate a single candidate,
/// so filtering there only costs recall), and a token whose every candidate is
/// out-of-inventory is returned untouched rather than zeroed. So this never
/// loses a parse relative to [`parse_word`]; it only prunes genuine ambiguity.
pub fn parse_word_disambiguated(word: &str, roots: Option<&HashSet<[char; 3]>>) -> Vec<VerbMatch> {
    disambiguate(parse_word(word), roots)
}

/// Index-backed twin of [`parse_word_disambiguated`]: the unrestricted parse is
/// a [`ReverseIndex`] lookup instead of generate-and-test, then the same
/// disambiguate-only inventory filter is applied to the result.
pub fn parse_word_indexed_disambiguated(
    word: &str,
    index: &ReverseIndex,
    roots: Option<&HashSet<[char; 3]>>,
) -> Vec<VerbMatch> {
    disambiguate(parse_word_indexed(word, index, None), roots)
}

/// The disambiguate-only inventory filter shared by the generate-and-test and
/// index-backed soft parsers (see [`parse_word_disambiguated`] for semantics).
fn disambiguate(matches: Vec<VerbMatch>, roots: Option<&HashSet<[char; 3]>>) -> Vec<VerbMatch> {
    let Some(set) = roots else {
        return matches;
    };
    if matches.len() <= 1 {
        return matches;
    }
    let kept: Vec<VerbMatch> = matches
        .iter()
        .filter(|m| set.contains(&m.root.letters))
        .cloned()
        .collect();
    if kept.is_empty() { matches } else { kept }
}

/// Enumerate every triliteral root that could underlie the surface
/// consonants. The alphabet is the distinct surface consonants together with
/// the weak letters; each of the three radical slots ranges over it.
fn candidate_roots(seq: &[Cons], roots: Option<&HashSet<[char; 3]>>) -> Vec<[char; 3]> {
    let mut alphabet: Vec<char> = Vec::new();
    for c in seq {
        if !alphabet.contains(&c.letter) {
            alphabet.push(c.letter);
        }
    }
    for &w in &WEAK {
        if !alphabet.contains(&w) {
            alphabet.push(w);
        }
    }
    // לקח assimilates its C1 lamed into a dagesh (יִקַּח) or drops it entirely
    // (קַח, קַחַת), so the surface may lack the lamed. Include it so that root
    // is reachable; the lexicon filter and exact surface match keep it safe.
    if !alphabet.contains(&letter::LAMED) {
        alphabet.push(letter::LAMED);
    }

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for &a in &alphabet {
        for &b in &alphabet {
            for &c in &alphabet {
                let r = [a, b, c];
                // Drop roots the inventory doesn't attest before they reach the
                // (expensive) paradigm generation and surface match.
                if roots.is_some_and(|set| !set.contains(&r)) {
                    continue;
                }
                if seen.insert(r) {
                    out.push(r);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_match(
        matches: &[VerbMatch],
        root: &str,
        binyan: Binyan,
        form: Form,
        pgn_label: &str,
    ) -> bool {
        let root: Vec<char> = root.chars().collect();
        matches.iter().any(|m| {
            m.root.letters.as_slice() == root.as_slice()
                && m.binyan == binyan
                && m.form == form
                && m.pgn.label() == pgn_label
                && m.prefix.is_empty()
        })
    }

    #[test]
    fn parses_strong_qal_perfect() {
        // קָטַל — Qal perfect 3ms of קטל.
        let matches = parse_word("קָטַל");
        assert!(has_match(
            &matches,
            "קטל",
            Binyan::Qal,
            Form::Perfect,
            "3ms"
        ));
    }

    #[test]
    fn parses_shamar() {
        // שָׁמַר — Qal perfect 3ms of שמר.
        let matches = parse_word("שָׁמַר");
        assert!(has_match(
            &matches,
            "שמר",
            Binyan::Qal,
            Form::Perfect,
            "3ms"
        ));
    }

    #[test]
    fn parses_wayyiqtol() {
        // וַיִּקְטֹל — vav-consecutive imperfect of קטל (yod prefix doubled).
        let matches = parse_word("וַיִּקְטֹל");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "קטל"
                && m.binyan == Binyan::Qal
                && m.form == Form::Wayyiqtol
                && m.pgn.label() == "3ms"
                && m.vav_consecutive
        }));
    }

    #[test]
    fn parses_hollow_wayyiqtol() {
        // וַיָּקָם — wayyiqtol of קום, via hollow nesiga (holam → qamats).
        let matches = parse_word("וַיָּקָם");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "קומ"
                && m.binyan == Binyan::Qal
                && m.form == Form::Wayyiqtol
                && m.pgn.label() == "3ms"
                && m.vav_consecutive
        }));
    }

    #[test]
    fn parses_lamed_he_wayyiqtol() {
        // וַיִּבֶן — wayyiqtol of בנה, built on the apocopated jussive stem.
        let matches = parse_word("וַיִּבֶן");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "בנה"
                && m.binyan == Binyan::Qal
                && m.form == Form::Wayyiqtol
                && m.pgn.label() == "3ms"
                && m.vav_consecutive
        }));
    }

    #[test]
    fn parses_pe_aleph_wayyiqtol() {
        // וַיֹּאמֶר — wayyiqtol of אמר, the single most common verb form in the
        // OT. The C2 patah of the imperfect (יֹאמַר) retracts to segol.
        let matches = parse_word("וַיֹּאמֶר");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "אמר"
                && m.binyan == Binyan::Qal
                && m.form == Form::Wayyiqtol
                && m.pgn.label() == "3ms"
                && m.vav_consecutive
        }));
    }

    #[test]
    fn parses_pe_aleph_wayyiqtol_plural() {
        // וַיֹּאמְרוּ — wayyiqtol 3mp of אמר; the vocalic suffix keeps C2 sheva.
        let matches = parse_word("וַיֹּאמְרוּ");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "אמר"
                && m.binyan == Binyan::Qal
                && m.form == Form::Wayyiqtol
                && m.pgn.label() == "3mp"
                && m.vav_consecutive
        }));
    }

    #[test]
    fn parses_lamed_aleph_imperfect() {
        // יִקְרָא — Qal imperfect of קרא; the quiescent alef lengthens the
        // thematic holam to qamats.
        let matches = parse_word("יִקְרָא");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "קרא"
                && m.binyan == Binyan::Qal
                && m.form == Form::Imperfect
                && m.pgn.label() == "3ms"
        }));
    }

    #[test]
    fn parses_lamed_guttural_imperfect() {
        // יִשְׁלַח — Qal imperfect of שלח; the final guttural lowers the thematic
        // holam to patah.
        let matches = parse_word("יִשְׁלַח");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "שלח"
                && m.binyan == Binyan::Qal
                && m.form == Form::Imperfect
                && m.pgn.label() == "3ms"
        }));
    }

    #[test]
    fn parses_lamed_he_inf_construct() {
        // לַעֲשׂוֹת — ל + Qal infinitive construct of עשה; the he becomes a tav
        // and the linking vowel is a plene holam on a vav mater (עֲשׂוֹת).
        let matches = parse_word("לַעֲשׂוֹת");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "עשה"
                && m.binyan == Binyan::Qal
                && m.form == Form::InfinitiveConstruct
        }));
    }

    #[test]
    fn parses_lamed_he_imperfect_persons() {
        // III-He imperfect beyond 3ms: יִבְנֶה (3ms, segol + he kept), תִּבְנֶה
        // (3fs/2ms, same grade), and the vocalic-suffix plural יִבְנוּ (3mp, he
        // elides and C2 loses its vowel).
        let m1 = parse_word("תִּבְנֶה");
        assert!(m1.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "בנה"
                && m.binyan == Binyan::Qal
                && m.form == Form::Imperfect
        }));
        let m2 = parse_word("יִבְנוּ");
        assert!(m2.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "בנה"
                && m.binyan == Binyan::Qal
                && m.form == Form::Imperfect
                && m.pgn.label() == "3mp"
        }));
    }

    #[test]
    fn parses_lamed_he_guttural_wayyiqtol() {
        // וַיַּעַשׂ — apocopated wayyiqtol of עשה. The dropped he leaves a C1-C2
        // cluster broken by a helping vowel; the guttural C1 (ayin) takes patah,
        // not the segol a non-guttural C1 would (cf. וַיִּבֶן).
        let matches = parse_word("וַיַּעַשׂ");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "עשה"
                && m.binyan == Binyan::Qal
                && m.form == Form::Wayyiqtol
        }));
    }

    #[test]
    fn parses_lemor_contraction() {
        // לֵאמֹר — ל + Qal infinitive construct of אמר. The preposition swallows
        // the pe-aleph's hataf (the aleph quiesces vowel-less) and takes a
        // tsere; restoring the aleph's hataf lets the bare form (אֲמֹר) match.
        let matches = parse_word("לֵאמֹר");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "אמר"
                && m.binyan == Binyan::Qal
                && m.form == Form::InfinitiveConstruct
                && !m.prefix.is_empty()
        }));
    }

    #[test]
    fn parses_haya_imperfect() {
        // יִהְיֶה — Qal imperfect 3ms of היה. Lexically irregular: the C1 he
        // closes the prefix syllable with a silent sheva (not a hataf) and the
        // hiriq prefix is kept, unlike a regular I-guttural III-He verb.
        let matches = parse_word("יִהְיֶה");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "היה"
                && m.binyan == Binyan::Qal
                && m.form == Form::Imperfect
                && m.pgn.label() == "3ms"
        }));
    }

    #[test]
    fn parses_pe_guttural_imperfect() {
        // יַעֲמֹד — Qal imperfect of עמד; the initial guttural lowers the hiriq
        // prefix to patah and takes a hataf-patah.
        let matches = parse_word("יַעֲמֹד");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "עמד"
                && m.binyan == Binyan::Qal
                && m.form == Form::Imperfect
                && m.pgn.label() == "3ms"
        }));
    }

    #[test]
    fn parses_sin_root_qal_perfect() {
        // עָשָׂה — Qal perfect 3ms of עשׂה (sin). The generator renders ש with a
        // shin dot, so the match must ignore the sin/shin distinction.
        let matches = parse_word("עָשָׂה");
        assert!(has_match(&matches, "עשה", Binyan::Qal, Form::Perfect, "3ms"));
    }

    #[test]
    fn parses_with_vav_prefix() {
        // וְשָׁמַר — conjunction ו + שָׁמַר. The stem still parses; the prefix
        // is reported separately.
        let matches = parse_word("וְשָׁמַר");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "שמר"
                && m.binyan == Binyan::Qal
                && m.form == Form::Perfect
                && m.pgn.label() == "3ms"
                && !m.prefix.is_empty()
        }));
    }

    /// Does any candidate carry this root/form/object-suffix?
    fn has_obj(matches: &[VerbMatch], root: &str, form: Form, obj: &str) -> bool {
        matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == root
                && m.form == form
                && m.object_suffix.map(|p| p.label()).as_deref() == Some(obj)
        })
    }

    #[test]
    fn parses_perfect_object_suffix() {
        // שְׁלָחַנִי — Qal perfect 3ms of שלח + 1cs object ("he sent me").
        let matches = parse_word("שְׁלָחַנִי");
        assert!(has_obj(&matches, "שלח", Form::Perfect, "1cs"));
    }

    #[test]
    fn parses_imperfect_object_suffix() {
        // יְבָרֶכְךָ — Piel imperfect 3ms of ברך + 2ms object ("he will bless you").
        let matches = parse_word("יְבָרֶכְךָ");
        assert!(matches.iter().any(|m| {
            // root.letters stores base (non-final) forms: kaf כ, not ך.
            m.root.letters.iter().collect::<String>() == "ברכ"
                && m.binyan == Binyan::Piel
                && m.form == Form::Imperfect
                && m.object_suffix.map(|p| p.label()).as_deref() == Some("2ms")
        }));
    }

    #[test]
    fn indexed_matches_per_surface() {
        // The reverse index must return exactly what parse_word does. Compare the
        // sorted (root, binyan, form, pgn, suffix, prefix, vav) tuples on a varied
        // sample spanning strong, weak, prefixed, suffixed and wayyiqtol forms.
        let index = ReverseIndex::build();
        let sample = [
            "קָטַל", "שָׁמַר", "וְשָׁמַר", "יִשְׁמֹר", "וַיִּשְׁמֹר", "הִקְטִיל", "נִשְׁמַר",
            "עָשָׂה", "יַעֲשֶׂה", "וַיַּעַשׂ", "לֵאמֹר", "שְׁלָחַנִי", "יְבָרֶכְךָ",
            "וַיִּתְּנֵם", "בְּמָלְכוֹ", "הוֹלִידוֹ", "מֶלֶךְ", "וַיָּקָם", "יֵלֵכוּ",
        ];
        let key = |m: &VerbMatch| {
            (
                m.root.letters,
                m.binyan as usize,
                m.form as usize,
                m.pgn.label(),
                m.object_suffix.map(|p| p.label()),
                m.prefix.clone(),
                m.vav_consecutive,
            )
        };
        for w in sample {
            let mut a: Vec<_> = parse_word(w).iter().map(key).collect();
            let mut b: Vec<_> = parse_word_indexed(w, &index, None).iter().map(key).collect();
            a.sort();
            b.sort();
            assert_eq!(a, b, "index/per-surface mismatch for {w}");
        }
    }

    #[test]
    fn parses_wayyiqtol_object_suffix() {
        // וַיִּתְּנֵם — wayyiqtol of נתן + 3mp object ("and he gave them"), the
        // weak I-Nun base form carrying through to the suffixed form.
        let matches = parse_word("וַיִּתְּנֵם");
        // root.letters stores base (non-final) forms: nun נ, not ן.
        assert!(has_obj(&matches, "נתנ", Form::Wayyiqtol, "3mp"));
    }

    #[test]
    fn parses_perfect_nonsubject_object_suffix() {
        // נְתַתִּיךָ — Qal perfect 1cs of נתן + 2ms ("I gave you").
        let m1 = parse_word("נְתַתִּיךָ");
        assert!(m1.iter().any(|m| m.root.letters.iter().collect::<String>() == "נתנ"
            && m.form == Form::Perfect
            && m.pgn.label() == "1cs"
            && m.object_suffix.map(|p| p.label()).as_deref() == Some("2ms")));
        // אֲהַבְתָּנוּ — Qal perfect 2ms of אהב + 1cp ("you loved us").
        let m2 = parse_word("אֲהַבְתָּנוּ");
        assert!(has_obj(&m2, "אהב", Form::Perfect, "1cp"));
        // גְּנָבוּךָ — Qal perfect 3cp of גנב + 2ms ("they stole you").
        let m3 = parse_word("גְּנָבוּךָ");
        assert!(has_obj(&m3, "גנב", Form::Perfect, "2ms"));
    }

    #[test]
    fn parses_participle_object_suffix() {
        // מְצַוְּךָ — Piel participle ms of צוה (III-He) + 2ms ("the one commanding you").
        let m1 = parse_word("מְצַוְּךָ");
        assert!(m1.iter().any(|m| m.root.letters.iter().collect::<String>() == "צוה"
            && m.binyan == Binyan::Piel
            && m.form == Form::ParticipleActive
            && m.object_suffix.map(|p| p.label()).as_deref() == Some("2ms")));
        // שֹׁמֶרְךָ — Qal participle ms of שמר + 2ms ("the one keeping you").
        let m2 = parse_word("שֹׁמֶרְךָ");
        assert!(has_obj(&m2, "שמר", Form::ParticipleActive, "2ms"));
    }

    #[test]
    fn parses_hiphil_imperative_object_suffix() {
        // הַצִּילֵנִי — Hiphil imperative 2ms of נצל + 1cs ("deliver me").
        let matches = parse_word("הַצִּילֵנִי");
        assert!(has_obj(&matches, "נצל", Form::Imperative, "1cs"));
    }

    #[test]
    fn parses_defective_hollow_hiphil_object_suffix() {
        // וַיְבִאֵהוּ — בוא Hiphil + 3ms ("and he brought him"); the î mater is
        // written defectively (yᵉḇiʔ-, not the plene yᵉḇîʔ-).
        let matches = parse_word("וַיְבִאֵהוּ");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "בוא"
            && m.binyan == Binyan::Hiphil
            && m.object_suffix.map(|p| p.label()).as_deref() == Some("3ms")));
    }

    #[test]
    fn parses_lamed_he_doubled_apocope() {
        // וַיֵּשְׁתְּ — שתה Qal wayyiqtol ("and he drank"); the he drops and C2
        // doubles into a tsere-prefixed monosyllable (wayyēšt).
        let matches = parse_word("וַיֵּשְׁתְּ");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "שתה"
            && m.form == Form::Wayyiqtol));
    }

    #[test]
    fn parses_pe_yod_as_pe_nun_imperfect() {
        // וַיִּצֹק — יצק Qal wayyiqtol ("and he poured"); the I-yod drops like a
        // I-nun, the prefix taking hiriq and the theme holam (yiṣōq).
        let matches = parse_word("וַיִּצֹק");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "יצק"
            && m.binyan == Binyan::Qal
            && m.form == Form::Imperfect));
    }

    #[test]
    fn parses_proclitic_iguttural_silent_sheva() {
        // לַעְזֹר — לְ + עֲזֹר ("to help"); the proclitic closes the ayin on a
        // silent sheva, which the parser restores to the generator's hataf.
        let matches = parse_word("לַעְזֹר");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "עזר"));
    }

    #[test]
    fn parses_gava_strong_not_hollow() {
        // וַיִּגְוַע — גוע wayyiqtol ("and he expired"); the medial vav is a true
        // radical, so it inflects as a strong III-guttural, not a hollow verb.
        let matches = parse_word("וַיִּגְוַע");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "גוע"
            && m.binyan == Binyan::Qal));
    }

    #[test]
    fn parses_pe_aleph_patah_wayyiqtol() {
        // וַיַּאַסְפוּ — אסף Qal wayyiqtol 3mp ("and they gathered"); the I-aleph
        // patah grade (yaʾasp̄û) beside the builder's segol yeʾesp̄û.
        let matches = parse_word("וַיַּאַסְפוּ");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "אספ"
            && m.binyan == Binyan::Qal));
    }

    #[test]
    fn parses_lamed_he_hiphil_apocope() {
        // וַיַּשְׁקְ — שקה Hiphil apocopated wayyiqtol ("and he watered"); the
        // segol/î + he drops, the qof closing on a silent sheva.
        let matches = parse_word("וַיַּשְׁקְ");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "שקה"
            && m.binyan == Binyan::Hiphil));
    }

    #[test]
    fn parses_hollow_hiphil_perfect_object_suffix() {
        // וַהֲשִׁבֹתִים — שוב Hiphil perfect 1cs + 3mp ("and I will restore them");
        // the suffix attaches to the linking-ô perfect stem (hăšîḇōṯî-).
        let matches = parse_word("וַהֲשִׁבֹתִים");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "שוב"
            && m.binyan == Binyan::Hiphil
            && m.form == Form::Perfect
            && m.object_suffix.map(|p| p.label()).as_deref() == Some("3mp")));
    }

    #[test]
    fn parses_qal_participle_fs_a_form() {
        // יוֹלֵדָה — ילד Qal active participle fs ("woman in labour"); the -â
        // feminine (qōṭēlâ), beside the segolate yōleḏeṯ.
        let matches = parse_word("יוֹלֵדָה");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "ילד"
            && m.form == Form::ParticipleActive));
    }

    #[test]
    fn parses_hollow_niphal_perfect() {
        // נָפֹצוּ — פוץ Niphal perfect 3cp ("they were scattered"); the hollow ô
        // sits on C1 (nāp̄ôṣû), not the strong נִפְוַץ.
        let matches = parse_word("נָפֹצוּ");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "פוצ"
            && m.binyan == Binyan::Niphal
            && m.form == Form::Perfect));
    }

    #[test]
    fn parses_geminate_niphal_perfect() {
        // נָשַׁמּוּ — שמם Niphal perfect 3cp ("they were made desolate"); the
        // doubled radical contracts to nāCaC, doubling before the vocalic -û.
        let matches = parse_word("נָשַׁמּוּ");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "שממ"
            && m.binyan == Binyan::Niphal
            && m.form == Form::Perfect
            && m.pgn.label() == "3cp"));
    }

    #[test]
    fn parses_pausal_qotol_imperative() {
        // אֱמָר — אמר Qal imperative 2ms in pause; the theme holam lengthens to
        // qamats (ʾĕmār), beside the contextual ʾĕmōr.
        let matches = parse_word("אֱמָר");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "אמר"
            && m.binyan == Binyan::Qal
            && m.form == Form::Imperative));
    }

    #[test]
    fn parses_iguttural_silent_sheva_imperfect_plural() {
        // וְיַחְפְּרוּ — חפר Qal imperfect 3mp; the I-guttural closes the prefix
        // syllable on a silent sheva, the pe taking a dagesh lene (yaḥpᵊrû).
        let matches = parse_word("וְיַחְפְּרוּ");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "חפר"
            && m.form == Form::Imperfect
            && m.pgn.label() == "3mp"));
    }

    #[test]
    fn parses_pausal_imperfect_plural() {
        // יִשְׁמָעוּ — שמע Qal imperfect 3mp in pause; the theme restores to
        // qamats (yišmāʕû), unlike the contextual yišmᵊʕû.
        let matches = parse_word("יִשְׁמָעוּ");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "שמע"
            && m.form == Form::Imperfect
            && m.pgn.label() == "3mp"));
    }

    #[test]
    fn parses_nun_retained_infinitive_construct() {
        // בִּנְפֹל — נפל Qal inf construct ("in falling"); the nun is kept (nᵊp̄ōl),
        // not assimilated to the dropped pōl.
        let matches = parse_word("בִּנְפֹל");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "נפל"
            && m.form == Form::InfinitiveConstruct));
    }

    #[test]
    fn parses_geminate_hiphil_perfect() {
        // הֵחֵלּוּ — חלל Hiphil perfect 3cp ("they began"); the doubled radical
        // contracts to the hēCēC shape, doubling before the vocalic -û.
        let matches = parse_word("הֵחֵלּוּ");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "חלל"
            && m.binyan == Binyan::Hiphil
            && m.form == Form::Perfect
            && m.pgn.label() == "3cp"));
    }

    #[test]
    fn parses_hollow_hiphil_infinitive_object_suffix() {
        // לַהֲמִיתוֹ — מות Hiphil inf construct + 3ms ("to kill him"); the hāmîṯ
        // infinitive reduces to hămîṯ- before the suffix.
        let matches = parse_word("לַהֲמִיתוֹ");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "מות"
            && m.binyan == Binyan::Hiphil
            && m.form == Form::InfinitiveConstruct));
    }

    #[test]
    fn parses_hiphil_lamed_he_perfect_patah_suffix() {
        // הִרְאַנִי — ראה Hiphil perfect 3ms + 1cs ("he showed me"); the III-He
        // Hiphil links the suffix on a patah, not the Qal's qamats.
        let matches = parse_word("הִרְאַנִי");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "ראה"
            && m.binyan == Binyan::Hiphil
            && m.object_suffix.map(|p| p.label()).as_deref() == Some("1cs")));
        // Regression: the Qal qamats grade still parses (עָשָׂהוּ).
        let m2 = parse_word("עָשָׂהוּ");
        assert!(m2.iter().any(|m| m.root.letters.iter().collect::<String>() == "עשה"));
    }

    #[test]
    fn parses_iguttural_cohortative_silent_sheva() {
        // אֶעְבְּרָה — עבר Qal cohortative 1cs ("let me cross over"); the I-guttural
        // takes a silent sheva closing the prefix syllable, the bet a dagesh lene.
        let matches = parse_word("אֶעְבְּרָה");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "עבר"
            && m.binyan == Binyan::Qal));
    }

    #[test]
    fn parses_enemy_and_mp_participle_1cs_suffix() {
        // אוֹיְבַי — איב (now a true-triliteral qōṭēl, not hollow) participle mp +
        // 1cs ("my enemies").
        let m1 = parse_word("אוֹיְבַי");
        assert!(m1.iter().any(|m| m.root.letters.iter().collect::<String>() == "איב"
            && m.form == Form::ParticipleActive
            && m.object_suffix.map(|p| p.label()).as_deref() == Some("1cs")));
        // The -ay form cascades to any mp participle (שֹׁמְרַי "those keeping me").
        let m2 = parse_word("שֹׁמְרַי");
        assert!(m2.iter().any(|m| m.root.letters.iter().collect::<String>() == "שמר"
            && m.object_suffix.map(|p| p.label()).as_deref() == Some("1cs")));
    }

    #[test]
    fn parses_segholate_infinitive_object_suffix_segol_grade() {
        // בְּלֶכְתּוֹ — הלך Qal inf construct (leḵeṯ) + 3ms ("in his going"); C1
        // keeps segol (leḵtô), unlike the I-yod hiriq grade (šibtô).
        let matches = parse_word("בְּלֶכְתּוֹ");
        assert!(matches.iter().any(|m| m.form == Form::InfinitiveConstruct
            && m.object_suffix.map(|p| p.label()).as_deref() == Some("3ms")));
        // Regression: the hiriq grade still parses (בְּשִׁבְתְּךָ, ישב).
        let m2 = parse_word("בְּשִׁבְתְּךָ");
        assert!(m2.iter().any(|m| m.form == Form::InfinitiveConstruct));
    }

    #[test]
    fn parses_pe_yod_perfect_3cp_object_suffix() {
        // יְדָעוּם — ידע Qal perfect 3cp + 3mp ("they knew them"); the I-yod
        // perfect is regular, so the qᵊṭāl-û connecting stem applies.
        let matches = parse_word("יְדָעוּם");
        assert!(has_obj(&matches, "ידע", Form::Perfect, "3mp"));
    }

    #[test]
    fn parses_lamed_he_imperfect_object_suffix() {
        // וַיַּעֲנֵנִי — ענה Qal wayyiqtol + 1cs ("and he answered me"); the he
        // elides and the suffix links on a tsere.
        let matches = parse_word("וַיַּעֲנֵנִי");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "ענה"
            && m.object_suffix.map(|p| p.label()).as_deref() == Some("1cs")));
    }

    #[test]
    fn parses_pe_guttural_wayyiqtol_object_suffix() {
        // וַיַּעַזְבֵנִי — עזב Qal wayyiqtol + 1cs ("and he forsook me"); the C1
        // ayin's hataf fills to a full patah as the suffixed stem closes it.
        let matches = parse_word("וַיַּעַזְבֵנִי");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "עזב"
            && m.object_suffix.map(|p| p.label()).as_deref() == Some("1cs")));
    }

    #[test]
    fn parses_pe_nun_niphal_participle() {
        // נִבְּאִים — נבא Niphal participle mp ("prophesying"); the radical nun
        // assimilates into the bet (niḇbᵊʔîm), as in the perfect.
        let matches = parse_word("נִבְּאִים");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "נבא"
            && m.binyan == Binyan::Niphal
            && m.form == Form::ParticipleActive));
    }

    #[test]
    fn parses_lamed_aleph_plural_object_suffix() {
        // וַיִּמְצָאֻהוּ — מצא Qal imperfect 3mp + 3ms ("and they found him"); the
        // quiescent aleph carries the subject û defectively as a qubuts.
        let matches = parse_word("וַיִּמְצָאֻהוּ");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "מצא"
            && m.object_suffix.map(|p| p.label()).as_deref() == Some("3ms")));
    }

    #[test]
    fn parses_pe_vav_niphal_imperfect() {
        // וַיִּוָּעַץ — יעץ Niphal wayyiqtol ("and he took counsel"); the I-vav
        // root doubles the vav (yiwwāʕaṣ), not the yod the builder defaults to.
        let matches = parse_word("וַיִּוָּעַץ");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "יעצ"
            && m.binyan == Binyan::Niphal));
    }

    #[test]
    fn parses_nun_retained_imperative() {
        // נְטֵה — נטה Qal imperative ("stretch out"); the nun is kept, not
        // assimilated.
        let matches = parse_word("נְטֵה");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "נטה"
            && m.binyan == Binyan::Qal
            && m.form == Form::Imperative));
    }

    #[test]
    fn parses_geminate_imperative_object_suffix() {
        // חָנֵּנִי — חנן Qal imperative 2ms + 1cs ("be gracious to me"); the
        // geminate radical contracts to a single dageshed nun.
        let matches = parse_word("חָנֵּנִי");
        assert!(has_obj(&matches, "חננ", Form::Imperative, "1cs"));
    }

    #[test]
    fn parses_theme_restored_paragogic_nun() {
        // תֹּאבֵדוּן — אבד Qal Imperfect 2mp: the energic -ûn restores the C2
        // tsere the bare plural reduced (tōʔḇᵊḏû → tōʔḇēḏûn).
        let matches = parse_word("תֹּאבֵדוּן");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "אבד"
            && m.form == Form::Imperfect));
        // תִּשְׁמָעוּן — שמע, the qamats (a-theme) twin.
        let m2 = parse_word("תִּשְׁמָעוּן");
        assert!(m2.iter().any(|m| m.root.letters.iter().collect::<String>() == "שמע"
            && m.form == Form::Imperfect));
    }

    #[test]
    fn parses_imperfect_energic_3fs_object_suffix() {
        // אֶתְּנֶנָּה — Qal imperfect 1cs of נתן + energic 3fs ("I will give it").
        let matches = parse_word("אֶתְּנֶנָּה");
        assert!(has_obj(&matches, "נתנ", Form::Imperfect, "3fs"));
    }

    #[test]
    fn strips_word_initial_dehiq_dagesh() {
        // נַּעֲשֶׂה — עשה Qal Imperfect 1cp ("we will do") with a conjunctive
        // (dehiq) dagesh forte on the word-initial nun; the generator never
        // produces the doubled nun, so the parser must strip it.
        let matches = parse_word("נַּעֲשֶׂה");
        assert!(matches.iter().any(|m| m.root.letters.iter().collect::<String>() == "עשה"
            && m.binyan == Binyan::Qal
            && m.form == Form::Imperfect
            && m.pgn.label() == "1cp"));
    }
}
