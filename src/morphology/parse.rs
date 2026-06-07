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

/// Compare two rendered forms ignoring the sin/shin dot. The generator always
/// renders ש with a shin dot (it has no lexical knowledge of which roots are
/// sin), so a sin-pointed surface — עָשָׂה, נָשָׂא, שָׂם — could never exact-match an
/// otherwise-identical generated form. Candidate roots already key on the bare
/// letter ש and we report roots by their bare letters, so collapsing the dot in
/// the surface match recovers every sin verb without admitting new roots.
fn forms_match(generated: &str, target: &str) -> bool {
    fn strip_dot(s: &str) -> String {
        s.chars().filter(|&c| c != '\u{05C1}' && c != '\u{05C2}').collect()
    }
    // Collapse a plene holam (a vav mater carrying holam after a vowelless
    // consonant) onto the preceding consonant, so plene יוֹשֵׁב and defective
    // יֹשֵׁב normalise to the same form. Plene/defective is purely orthographic —
    // the same lexeme, same analysis — so this only recovers spelling variants
    // and never merges distinct words (a holam mater is the sole thing removed).
    fn collapse_holam(s: &str) -> String {
        let mut out: Vec<Cons> = Vec::new();
        for c in hebrew::parse_pointed(s) {
            if c.letter == letter::VAV && c.vowel == Some(Vowel::Holam) {
                if let Some(prev) = out.last_mut() {
                    if prev.vowel.is_none() {
                        prev.vowel = Some(Vowel::Holam);
                        continue;
                    }
                }
            }
            out.push(c);
        }
        hebrew::render(&out)
    }
    // Collapse a plene shureq (a vav mater bearing the dagesh-as-shureq point
    // after a vowelless consonant) onto the preceding consonant as a qubuts, so
    // plene יָמוּתוּ and defective יָמֻתוּ normalise alike. Like holam this is
    // purely orthographic — same lexeme, same analysis — and exact-match still
    // governs, so it only recovers spelling variants.
    fn collapse_shureq(s: &str) -> String {
        let mut out: Vec<Cons> = Vec::new();
        for c in hebrew::parse_pointed(s) {
            if c.letter == letter::VAV && c.dagesh && c.vowel.is_none() {
                if let Some(prev) = out.last_mut() {
                    if prev.vowel.is_none() {
                        prev.vowel = Some(Vowel::Qubuts);
                        continue;
                    }
                }
            }
            out.push(c);
        }
        hebrew::render(&out)
    }
    if generated == target || strip_dot(generated) == strip_dot(target) {
        return true;
    }
    let g = collapse_shureq(&collapse_holam(generated));
    let t = collapse_shureq(&collapse_holam(target));
    g == t || strip_dot(&g) == strip_dot(&t)
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

        // Candidate stem renderings. Normally just the peeled remainder, but a
        // proclitic (ל/ב/כ) prefixed to a pe-aleph stem swallows the aleph's
        // hataf: the preposition takes the contracted vowel and the aleph
        // quiesces vowel-less — לֵאמֹר = lē + ʔ(ă)mōr, the Qal inf. construct of
        // אמר. Restore the aleph's hataf-patah so the bare generated form
        // (אֲמֹר) matches. Exact-match keeps this safe: it only ever recovers
        // the genuine contracted reading.
        let mut targets = vec![hebrew::render(remainder)];
        // Conjunction sandhi וְ + yəC → וִyC: before a sheva-bearing yod the
        // conjunction takes hiriq and the yod quiesces into a mater, so a weqatal
        // / conjoined I-yod form surfaces with a vowel-less yod (וִידַעְתֶּם) where
        // the bare stem has yod + sheva (יְדַעְתֶּם). Restore the sheva so the stem
        // matches. Only when the peeled proclitic was a hiriq-vav.
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
        // Conjunction sandhi וְ + Cˇ(hataf) → וִ + Cˇ(sheva): before a guttural
        // bearing a hataf vowel the conjunction takes hiriq and the hataf reduces
        // to a plain (silent) sheva — wihyîtem (וִהְיִיתֶם) from the bare hĕyîtem
        // (הֱיִיתֶם). Restore the hataf so the bare stem matches. Only after a
        // peeled hiriq-vav.
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
        // The article (and the relative ש) double the first stem consonant with
        // a forte dagesh — הַיֹּשְׁבִים = ha + (y)yōšḇîm. The generated bare stem
        // carries no such dagesh, so strip it from the first consonant to expose
        // the underlying form. Only attempted when a proclitic was peeled, and
        // exact-match still governs, so this only recovers genuine readings.
        if strip > 0 && remainder.first().is_some_and(|c| c.dagesh) {
            let mut alt = remainder.to_vec();
            alt[0].dagesh = false;
            targets.push(hebrew::render(&alt));
        }

        for letters in candidate_roots(remainder, roots) {
            let paradigm = memo
                .entry(letters)
                .or_insert_with(|| generate_paradigm(&Root::from_letters(letters)));
            for vf in &paradigm.forms {
                // Wayyiqtol is handled by the dedicated pass below, with the
                // consecutive vav recognised as such rather than as a plain
                // proclitic; skip it here to avoid duplicate analyses.
                if vf.form == Form::Wayyiqtol
                    || !targets.iter().any(|t| forms_match(&vf.text, t))
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
        let target = hebrew::render(&seq);
        for letters in candidate_roots(&seq[1..], roots) {
            let paradigm = memo
                .entry(letters)
                .or_insert_with(|| generate_paradigm(&Root::from_letters(letters)));
            for vf in &paradigm.forms {
                if vf.form != Form::Wayyiqtol || !forms_match(&vf.text, &target) {
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

    // Attested (fully-modelled) analyses first, then a stable ordering.
    matches.sort_by(|a, b| {
        b.attested
            .cmp(&a.attested)
            // Bare forms before object-suffixed ones, so a first-match lookup
            // prefers the simpler analysis.
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
    let matches = parse_word(word);
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
    fn parses_wayyiqtol_object_suffix() {
        // וַיִּתְּנֵם — wayyiqtol of נתן + 3mp object ("and he gave them"), the
        // weak I-Nun base form carrying through to the suffixed form.
        let matches = parse_word("וַיִּתְּנֵם");
        // root.letters stores base (non-final) forms: nun נ, not ן.
        assert!(has_obj(&matches, "נתנ", Form::Wayyiqtol, "3mp"));
    }
}
