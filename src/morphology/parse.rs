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
use std::sync::{Arc, LazyLock, RwLock};

use rayon::prelude::*;

use super::hebrew::{self, Cons, Vowel, letter};
use super::root::Root;
use super::verb::{Binyan, Form, Paradigm, Pgn, generate_paradigm, host_object_suffixes};
use super::{Gender, Number, Person};

/// Dedup key for object-suffix verb matches: root, binyan, form, host PGN,
/// optional object-suffix PGN, strip length, and a flag distinguishing variants.
type ObjSuffixSeen = HashSet<([char; 3], Binyan, Form, Pgn, Option<Pgn>, usize, bool)>;

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
    /// How closely this candidate's generated spelling matches the surface —
    /// the primary spelling-quality signal for ranking (see [`MatchFidelity`]).
    /// Assigned by [`assign_fidelity`] after matching, so every construction
    /// site initialises it to [`MatchFidelity::Folded`] and the post-pass
    /// overwrites it.
    pub fidelity: MatchFidelity,
}

/// How closely a candidate's generated spelling matches the surface. The
/// matcher accepts a candidate when its generated form shares the surface's
/// [`canonical_key`] — but that key deliberately folds out orthographic
/// distinctions (a variably-omitted sonorant dagesh, plene/defective matres, a
/// guttural's hataf colour), so a *folded* match is less certain than one whose
/// bytes are identical. Ranking exact matches first floats the natural reading
/// up: וַתַּעֲלֶנָה matches the fp afformative exactly, but the energic-3fs+object
/// reading only after the §20m dagesh fold — so the former outranks the latter
/// without dropping it. Variants are ordered best-first (`Exact < Folded <
/// Skeleton`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchFidelity {
    /// The surface is byte-identical to a generated form (after the lossless
    /// combining-mark normalisation of `parse_pointed`/`render`).
    Exact,
    /// The surface matched only through a `canonical_key` relaxation.
    Folded,
    /// An unpointed-ketiv consonantal-skeleton match — the loosest tier, with
    /// no vowels to compare at all.
    Skeleton,
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
/// - Qamats-qatan (U+05C7) folds to plain qamats (U+05B8): the generator
///   distinguishes the o-class qamats (Hophal hoqṭal הׇקְטַל), but the
///   Masoretic text writes both with the one qamats sign, so a generated
///   qamats-qatan could never match a real surface.
/// - The sin/shin dot is stripped because the generator always renders ש with
///   a shin dot (it has no lexical knowledge of which roots are sin), so a
///   sin-pointed surface — עָשָׂה, נָשָׂא, שָׂם — could never exact-match an
///   otherwise-identical generated form. Candidate roots already key on the
///   bare letter ש and roots are reported by their bare letters, so collapsing
///   the dot recovers every sin verb without admitting new roots.
/// - Plene hiriq/tsere (a bare yod mater — no vowel, no dagesh — after a hiriq
///   or tsere) drops, so plene וַיָּשִׂימוּ and defective וַיָּשִׂמוּ normalise
///   alike, as do plene הֵבֵיאתָ and defective הֵבֵאתָ (tsere-yod ê-mater).
///   A vowelless, dagesh-less yod after an î/ê vowel is always a mater (a
///   consonantal yod there would carry its own vowel or sheva), so only
///   orthography is removed. Word-final ־ִי / ־ֵי also collapse, which is safe
///   for the same reason: no form ends in a bare hiriq/tsere consonant, so keys
///   can only meet other î/ê-yod keys (e.g. the mp construct -ê שֹׁמְרֵי).
/// - A guttural's hataf vowel (hataf-patah/segol/qamats) folds to one: the
///   colour is phonologically conditioned, not lexically contrastive, so a
///   generated הֲיוֹת and the attested הֱיוֹת normalise alike.
/// - A dagesh on resh, aleph, het, or ayin is dropped: these letters cannot
///   be doubled, so a generated dagesh there (a strong-pattern forte landing
///   on them, e.g. geminate הֵפֵרּוּ, תָּרֵעּוּ) is an artefact, and the rare
///   Masoretic anomalies normalise onto the same key. He is left untouched —
///   its dagesh point is the mappiq (the consonantal 3fs suffix ־ָהּ), which
///   is meaningful.
pub(crate) fn canonical_key(form: &str) -> String {
    // Two sequential passes — holam then shureq, each re-parsing the previous
    // pass's output — then a hiriq-yod/resh pass and the sin/shin-dot strip.
    fn collapse(form: &str, shureq: bool) -> String {
        let mut out: Vec<Cons> = Vec::new();
        for c in hebrew::parse_pointed(form) {
            // holam pass: a vav bearing holam; shureq pass: a vav bearing the
            // dagesh-as-shureq point with no vowel.
            let is_mater = c.letter == letter::VAV
                && if shureq {
                    c.dagesh && c.vowel.is_none()
                } else {
                    c.vowel == Some(Vowel::Holam)
                };
            if is_mater
                && let Some(prev) = out.last_mut()
                && prev.vowel.is_none()
            {
                prev.vowel = Some(if shureq { Vowel::Qubuts } else { Vowel::Holam });
                continue;
            }
            out.push(c);
        }
        hebrew::render(&out)
    }
    let holam = collapse(form, false);
    let shureq = collapse(&holam, true);
    let mut out: Vec<Cons> = Vec::new();
    for mut c in hebrew::parse_pointed(&shureq) {
        if matches!(
            c.letter,
            letter::RESH | letter::ALEF | letter::HET | letter::AYIN
        ) {
            c.dagesh = false;
        }
        // The MT omits dagesh forte variably on the sonorants ל/מ/נ (GKC §20m):
        // the generator always doubles a Piel/Pual/Hithpael C2 or a geminate
        // radical, but the Masoretes sometimes leave it bare — תְּכַלֶנָּה
        // (Gen 6:16, Piel 2ms+suffix of כלה) for the regular תְּכַלֶּנָּה. A
        // forte dagesh on these letters is thus non-contrastive for the verb
        // index; fold it out so the marked and bare spellings share a key.
        // (Resh, just above, is the same phenomenon — already folded there.)
        if matches!(c.letter, letter::LAMED | letter::MEM | letter::NUN) {
            c.dagesh = false;
        }
        // A guttural's hataf colour (hataf-patah / -segol / -qamats) is
        // phonologically conditioned, not lexically contrastive: the generator
        // writes one (the III-He inf-construct of היה comes out הֲיוֹת with
        // hataf-patah) where the Masoretes wrote another (הֱיוֹת, hataf-segol).
        // The consonantal skeleton and every other vowel are identical, so
        // folding the three hatafs to one on a guttural recovers the spelling
        // variant without ever merging distinct analyses.
        if hebrew::is_guttural(c.letter)
            && matches!(
                c.vowel,
                Some(Vowel::HatafPatah | Vowel::HatafSegol | Vowel::HatafQamats)
            )
        {
            c.vowel = Some(Vowel::HatafPatah);
        }
        // A hataf vowel on a NON-guttural consonant is a Masoretic colouring of
        // an ordinary vocal sheva — the generator only ever writes hatafs on
        // gutturals (via apply_guttural), so on any other letter the hataf is
        // non-contrastive. Fold it back to a sheva: a half-guttural resh
        // (נִבְרֲכוּ ← נִבְרְכוּ), a geminate C2 (גֹּזֲזֵי ← גֹּזְזֵי), the
        // consonant before a -ḵā suffix (מַדְרִיכֲךָ), etc. Symmetric, so it only
        // ever adds matches.
        else if matches!(
            c.vowel,
            Some(Vowel::HatafPatah | Vowel::HatafSegol | Vowel::HatafQamats)
        ) {
            c.vowel = Some(Vowel::Sheva);
        }
        if c.letter == letter::YOD
            && !c.dagesh
            && c.vowel.is_none()
            && out
                .last()
                .is_some_and(|p| matches!(p.vowel, Some(Vowel::Hiriq | Vowel::Tsere)))
        {
            continue;
        }
        out.push(c);
    }
    // A silent sheva on the consonant before a word-final quiescent aleph
    // (שָׁוְא → שָׁוא) is non-contrastive: the aleph carries no vowel and closes
    // no syllable, so the preceding sheva merely marks the closed syllable.
    // bible.db routinely drops it; fold it out so both spellings share a key.
    if let [.., penult, last] = out.as_mut_slice()
        && last.letter == letter::ALEF
        && last.vowel.is_none()
        && !last.dagesh
        && penult.vowel == Some(Vowel::Sheva)
    {
        penult.vowel = None;
    }
    // A mappiq (dagesh) on a word-final he is non-contrastive in a verb form —
    // the verb index holds only verb forms, whose III-He final he is a silent
    // mater, never a consonantal he. The MT occasionally marks one anyway
    // (אֶעֱשֶׂהּ, Gen 2:18, 1cs imperfect of עשה). Strip it so the marked and
    // unmarked spellings share a key. (Not done in the noun parser's norm_key:
    // there a mappiq-he is the 3fs suffix and *is* contrastive — סוּסָהּ vs סוּסָה.)
    if let Some(last) = out.last_mut()
        && last.letter == letter::HE
        && last.dagesh
    {
        last.dagesh = false;
    }
    hebrew::render(&out)
        .chars()
        .filter(|&c| c != '\u{05C1}' && c != '\u{05C2}')
        .map(|c| if c == '\u{05C7}' { '\u{05B8}' } else { c })
        .collect()
}

/// A surface carries no niqqud at all — every slot is a bare consonant with no
/// vowel and no dagesh. This is the *ketiv* case: the OSHB written tradition
/// records some words consonant-only (the qere supplies the pointing), so the
/// vocalised candidates can never be reached through [`canonical_key`], which
/// preserves vowels. Detecting it lets the parser fall back to a consonantal
/// skeleton match (see [`skeleton`]). A genuinely pointed word always carries at
/// least one vowel (even an all-sheva spelling marks the shevas), so this never
/// misfires on vocalised text.
fn is_unpointed(seq: &[Cons]) -> bool {
    !seq.is_empty() && seq.iter().all(|c| c.vowel.is_none() && !c.dagesh)
}

/// The consonantal skeleton of a (possibly pointed) form — just its letters,
/// with final-form normalisation already applied by [`hebrew::parse_pointed`].
/// Used only to match a fully-unpointed ketiv surface against the vocalised
/// candidates: their niqqud is stripped to the same bare-consonant sequence.
/// Matres are kept (they are consonants in the skeleton); [`fold_matres`]
/// removes them when a defective/plene divergence would otherwise block the
/// match.
fn skeleton(form: &str) -> String {
    hebrew::parse_pointed(form)
        .iter()
        .map(|c| c.letter)
        .collect()
}

/// Drop the *matres lectionis* — the non-initial vav and yod that spell a long
/// vowel rather than a consonant — from a consonant skeleton. The ketiv often
/// writes a vowel plene (אכתוב) where the generator spells it defective
/// (אֶכְתֹּב, skeleton אכתב), or vice versa; folding both skeletons to their
/// vowel-letter-free core lets the two align. Applied symmetrically to surface
/// and candidate, it can only *add* skeleton matches, never remove one — so it
/// never costs recall, only the precision of a surface that is already a bare
/// unpointed ketiv (a degenerate, multiply-ambiguous case regardless).
///
/// The first letter is always kept: a word-initial vav/yod is the conjunction
/// proclitic or a pe-vav/pe-yod radical, never a vowel letter. Aleph and he are
/// left in place — they double as quiescent radicals far more often, and the
/// observed divergences are all vav/yod plene.
fn fold_matres(skel: &str) -> String {
    skel.chars()
        .enumerate()
        .filter(|&(i, c)| i == 0 || (c != letter::VAV && c != letter::YOD))
        .map(|(_, c)| c)
        .collect()
}

/// Collapse a reduplicated consonant — an adjacent identical pair — to a single
/// letter in a consonant skeleton. Biblical Hebrew has a few quadriliteral verbs
/// built by reduplicating a radical of a triliteral (חצצר "sound a trumpet",
/// from חצר); their plene ketiv spells the doubled radical out (מחצצרים, skeleton
/// מחצצרם) where the qere and the generator use the reduced triliteral (מַחְצְרִים,
/// skeleton מחצרם). Collapsing adjacent duplicates on both the surface and
/// candidate skeletons converts the quadriliteral ketiv to its triliteral
/// paradigm so the two align. Applied symmetrically and only in the already-
/// degenerate unpointed-ketiv path, so — like [`fold_matres`] — it can only *add*
/// skeleton matches, never remove one, and never costs recall.
fn fold_reduplication(skel: &str) -> String {
    let mut out = String::new();
    let mut prev = None;
    for c in skel.chars() {
        if Some(c) != prev {
            out.push(c);
        }
        prev = Some(c);
    }
    out
}

/// The unpointed-ketiv skeleton key: matres folded out, then any reduplicated
/// consonant collapsed. Shared by surface and candidate so the two paths stay
/// symmetric (see [`fold_matres`], [`fold_reduplication`]).
fn ketiv_skeleton(form: &str) -> String {
    fold_reduplication(&fold_matres(&skeleton(form)))
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
    target_keys.contains(&key)
}

/// Candidate stem renderings for one proclitic peeling: the bare peeled
/// `remainder` plus the sandhi variants that restore a stem the proclitic
/// altered (conjunction וְ+yəC / וְ+hataf, a quiesced pe-aleph hataf, and the
/// article's forte dagesh). Shared by [`parse_word_filtered`] and
/// [`parse_word_indexed`] so both peel identically.
fn peeling_targets(seq: &[Cons], strip: usize, remainder: &[Cons]) -> Vec<String> {
    let mut targets = vec![hebrew::render(remainder)];
    // וְ/לְ/בְּ/כְּ + yəC → CִyC: a sheva-bearing yod quiesces to a mater after a
    // proclitic that took hiriq — the conjunction (וִיהִי) and the ל/ב/כ
    // prepositions alike (לִירוֹת for lə+yərôṯ, בִּימֵי, כִּירֹא).
    if strip > 0
        && matches!(
            seq[strip - 1].letter,
            letter::VAV | letter::LAMED | letter::BET | letter::KAF
        )
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
    // A proclitic's added syllable can propretonically reduce a I-guttural
    // stem's full vowel to a hataf: the interrogative/article he onto a Qal
    // perfect — heḥŏḏaltî הֶחֳדַלְתִּי for ḥāḏaltî חָדַלְתִּי — drops the het's
    // qamats to hataf-qamats. The generator writes the full vowel, so raise the
    // hataf back to its matching grade (hataf-qamats → qamats, etc.) to match.
    if strip > 0
        && remainder.first().is_some_and(|c| {
            hebrew::is_guttural(c.letter)
                && matches!(
                    c.vowel,
                    Some(Vowel::HatafQamats | Vowel::HatafPatah | Vowel::HatafSegol)
                )
        })
    {
        let full = match remainder[0].vowel {
            Some(Vowel::HatafQamats) => Vowel::Qamats,
            Some(Vowel::HatafPatah) => Vowel::Patah,
            _ => Vowel::Segol,
        };
        let mut alt = remainder.to_vec();
        alt[0].vowel = Some(full);
        targets.push(hebrew::render(&alt));
    }
    // A conjunction can hataf-colour even a non-guttural's initial sheva
    // (וּשֲׁמָע, וּזֲהַב): restore the plain sheva.
    if strip > 0
        && remainder.first().is_some_and(|c| {
            !hebrew::is_guttural(c.letter)
                && matches!(
                    c.vowel,
                    Some(Vowel::HatafPatah | Vowel::HatafSegol | Vowel::HatafQamats)
                )
        })
    {
        let mut alt = remainder.to_vec();
        alt[0].vowel = Some(Vowel::Sheva);
        targets.push(hebrew::render(&alt));
    }
    // He-syncope: a ל/ב/כ preposition before an infinitive can absorb the
    // preformative he of a Niphal/Hiphil infinitive, taking that he's vowel as
    // the he elides — lᵊ+hērāʾôṯ → lērāʾôṯ לֵרָאוֹת. The generator always keeps
    // the he, so restore it (with the proclitic's vowel) to match. Limited to
    // the tsere/patah/qamats grades that flag the absorbed infinitive he — the
    // qamats covers the hollow Hiphil inf-construct lā+hāḇîʾ → lāḇîʾ (לָבִיא).
    if strip > 0
        && matches!(
            seq[strip - 1].letter,
            letter::LAMED | letter::BET | letter::KAF
        )
        && matches!(
            seq[strip - 1].vowel,
            Some(Vowel::Tsere | Vowel::Patah | Vowel::Qamats)
        )
        && remainder.first().is_some_and(|c| c.letter != letter::HE)
    {
        let vowel = seq[strip - 1].vowel.unwrap();
        let mut alt = vec![Cons::new(letter::HE).with_vowel(vowel)];
        alt.extend_from_slice(remainder);
        targets.push(hebrew::render(&alt));
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
    // A bare word whose first begedkefet consonant lacks the dagesh lene was
    // preceded by a vowel-final word in close juncture (לֹא תְגַלֵּה) — the
    // generator always writes the word-initial dagesh, so restore it.
    if strip == 0
        && remainder
            .first()
            .is_some_and(|c| hebrew::is_begedkefet(c.letter) && !c.dagesh)
    {
        let mut alt = remainder.to_vec();
        alt[0].dagesh = true;
        targets.push(hebrew::render(&alt));
    }
    // After a proclitic prefix, a begedkefet consonant in the stem may carry
    // a dagesh lene that the generator does not produce (e.g. לִזְבֹּחַ vs
    // לִזְבֹחַ). Emit a variant with all begedkefet dageshes stripped so the
    // forms_match comparison can still succeed. Applied to every target
    // accumulated above, not just the bare remainder — the transforms
    // compose (לַחְפֹּר needs both the hataf restore and the dagesh strip).
    if strip > 0 {
        for t in targets.clone() {
            let mut alt = hebrew::parse_pointed(&t);
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
    }
    targets
}

/// Strip a recognised pronominal object-suffix ending off a parsed verb surface,
/// returning `(host stem before the suffix, object Pgn)` for every ending that
/// matches. The host's linking vowel stays on the returned stem — it is part of
/// the reduced host theme, not the suffix (peeling `-ēhû` off yišmᵊrēhû
/// יִשְׁמְרֵהוּ leaves yišmᵊrē יִשְׁמְרֵ). Ambiguous endings yield several
/// candidates; callers treat each additively.
///
/// This is increment 1 of the object-suffix architecture in
/// `doc/adr/0004-object-suffix-handling.md` (the inverse of the suffix-append
/// the object-suffix *builders* perform). It is verified by unit tests and is
/// the foundation for the host link-stem index + parse-time peeling that the
/// later increments add; it is intentionally not yet wired into the parsers.
fn peel_object_suffix(seq: &[Cons]) -> Vec<(Vec<Cons>, Pgn)> {
    let n = seq.len();
    if n < 2 {
        return Vec::new();
    }
    let last = &seq[n - 1];
    let prev = &seq[n - 2];
    let pgn = |p, g, num| Pgn::new(p, g, num);
    // A shureq is rendered as a bare vav carrying the dagesh point (וּ).
    let is_shureq = |c: &Cons| c.letter == letter::VAV && c.dagesh && c.vowel.is_none();
    let mut out = Vec::new();

    // -hû (הוּ) 3ms: he + shureq. The contracted -attû (a dagesh-doubled tav +
    // shureq, the 3fs-perfect host ʔăḵālattû אֲכָלָתּוּ) is 3ms too.
    if is_shureq(last)
        && ((prev.letter == letter::HE && prev.vowel.is_none())
            || (prev.letter == letter::TAV && prev.dagesh))
    {
        out.push((
            seq[..n - 2].to_vec(),
            pgn(Person::Third, Gender::Masculine, Number::Singular),
        ));
    }
    // -nû (נוּ) 1cp vs energic -ennû (נּוּ) 3ms: nun + shureq, split on the dagesh.
    if is_shureq(last) && prev.letter == letter::NUN {
        let (p, g, num) = if prev.dagesh {
            (Person::Third, Gender::Masculine, Number::Singular)
        } else {
            (Person::First, Gender::Common, Number::Plural)
        };
        out.push((seq[..n - 2].to_vec(), pgn(p, g, num)));
    }
    // -nî (נִי) 1cs (energic -ennî/-ēnnî identical for our purposes): nun + yod.
    if last.letter == letter::YOD
        && last.vowel.is_none()
        && prev.letter == letter::NUN
        && matches!(prev.vowel, Some(Vowel::Hiriq) | None)
    {
        out.push((
            seq[..n - 2].to_vec(),
            pgn(Person::First, Gender::Common, Number::Singular),
        ));
    }
    // -ḵem (כֶם) 2mp: kaf-segol + mem.
    if last.letter == letter::MEM && prev.letter == letter::KAF && prev.vowel == Some(Vowel::Segol)
    {
        out.push((
            seq[..n - 2].to_vec(),
            pgn(Person::Second, Gender::Masculine, Number::Plural),
        ));
    }
    // -ḵā (ךָ) 2ms: kaf-qamats; also the he-mater spelling -ḵâ (כָה), a
    // kaf-qamats followed by a bare he.
    if last.letter == letter::KAF && last.vowel == Some(Vowel::Qamats) {
        out.push((
            seq[..n - 1].to_vec(),
            pgn(Person::Second, Gender::Masculine, Number::Singular),
        ));
    }
    if last.letter == letter::HE
        && last.vowel.is_none()
        && !last.dagesh
        && prev.letter == letter::KAF
        && prev.vowel == Some(Vowel::Qamats)
    {
        out.push((
            seq[..n - 2].to_vec(),
            pgn(Person::Second, Gender::Masculine, Number::Singular),
        ));
    }
    // bare final kaf: -eḵ 2fs and pausal -āḵ 2ms are homographs (the
    // preceding vowel distinguishes them) — emit both, additively.
    if last.letter == letter::KAF && matches!(last.vowel, None | Some(Vowel::Sheva)) {
        out.push((
            seq[..n - 1].to_vec(),
            pgn(Person::Second, Gender::Feminine, Number::Singular),
        ));
        out.push((
            seq[..n - 1].to_vec(),
            pgn(Person::Second, Gender::Masculine, Number::Singular),
        ));
    }
    // -hā (הָ) 3fs: he-qamats; the mappiq -āh (he with a dagesh point, its
    // qamats on the preceding consonant); or the energic -ennâ (a bare he after
    // a dagesh-doubled nun, niṯʾaḵlennâ נִתְאַכְלֶנָּה).
    if last.letter == letter::HE
        && (last.vowel == Some(Vowel::Qamats)
            || (last.dagesh && last.vowel.is_none())
            || (last.vowel.is_none() && prev.letter == letter::NUN && prev.dagesh))
    {
        out.push((
            seq[..n - 1].to_vec(),
            pgn(Person::Third, Gender::Feminine, Number::Singular),
        ));
    }
    // -ô (וֹ) 3ms: vav-holam (also a mater in the host — ambiguous, additive).
    // Preceded by mem it is also the poetic -mô 3mp (yišmᵊrēmô).
    if last.letter == letter::VAV && last.vowel == Some(Vowel::Holam) {
        out.push((
            seq[..n - 1].to_vec(),
            pgn(Person::Third, Gender::Masculine, Number::Singular),
        ));
        if prev.letter == letter::MEM {
            out.push((
                seq[..n - 2].to_vec(),
                pgn(Person::Third, Gender::Masculine, Number::Plural),
            ));
        }
    }
    // Bare final vav (no vowel, no shureq dagesh): the -w 3ms (after a 1cs
    // perfect, ʔăḵaltîw) and, after a yod, the plural-host -āyw (his …s).
    if last.letter == letter::VAV && last.vowel.is_none() && !last.dagesh {
        out.push((
            seq[..n - 1].to_vec(),
            pgn(Person::Third, Gender::Masculine, Number::Singular),
        ));
        if prev.letter == letter::YOD {
            out.push((
                seq[..n - 2].to_vec(),
                pgn(Person::Third, Gender::Masculine, Number::Singular),
            ));
        }
    }
    // -ām / -ēm / -m (ם) 3mp: final mem. (The -ḵem 2mp above keys off
    // kaf-segol; a mem after a root kaf still takes the plain 3mp reading.)
    if last.letter == letter::MEM {
        out.push((
            seq[..n - 1].to_vec(),
            pgn(Person::Third, Gender::Masculine, Number::Plural),
        ));
    }
    // -n / -ān (ן) 3fp: final nun with no vowel of its own.
    if last.letter == letter::NUN && last.vowel.is_none() {
        out.push((
            seq[..n - 1].to_vec(),
            pgn(Person::Third, Gender::Feminine, Number::Plural),
        ));
    }
    out
}

/// Lean candidate roots for the object-suffix peel fallback: triliterals (every
/// position drawn from, so geminates included) over just the distinct consonants
/// present in the peeled stems. An object-suffix host is a finite verb that keeps
/// all its radicals on the surface for the strong and guttural classes, so the
/// stem's own consonants already contain the root and we can skip
/// [`candidate_roots`]' weak-letter + lamed padding — which cubes the root count
/// and is what would make this per-surface generate-and-test prohibitive when run
/// over every word (the parser fires the fallback on any zero-match surface). A
/// stem left with fewer than three distinct consonants (a hidden weak radical)
/// pads with the weak letters, cheap because that alphabet is then tiny.
fn fallback_roots(stems: &[&[Cons]], roots: Option<&HashSet<[char; 3]>>) -> Vec<[char; 3]> {
    let mut alphabet: Vec<char> = Vec::new();
    for stem in stems {
        for c in stem.iter() {
            if !alphabet.contains(&c.letter) {
                alphabet.push(c.letter);
            }
        }
    }
    if alphabet.len() < 3 {
        for &w in &WEAK {
            if !alphabet.contains(&w) {
                alphabet.push(w);
            }
        }
    }
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for &a in &alphabet {
        for &b in &alphabet {
            for &c in &alphabet {
                let r = [a, b, c];
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

/// Object-suffix peel fallback for the **generate-and-test** parser (ADR-0004
/// Option C): recover a suffixed surface whose host grade the generator's own
/// suffix dispatch never threaded. For each proclitic strip level, peel a
/// recognised pronominal ending to learn the candidate object `Pgn`s and a
/// reduced stem, enumerate the stem's candidate roots, and re-apply the host
/// suffix builders ([`host_object_suffixes`]) to every *bare* host form of each
/// root — including the theme/guttural and post-pass twins that arrive as their
/// own `forms` entries. A form is kept only when its rendered surface exactly
/// matches the (de-prefixed) target: generate-and-test, so a host grade that
/// can't produce the surface drops out and there are no false positives.
///
/// The indexed parser uses [`ReverseIndex::obj_index`] (a precomputed lookup)
/// for the same job; this generate-driven variant serves [`parse_word_filtered`]
/// when there is no index. Run only when ordinary parsing found nothing, so the
/// paradigm generation it needs is paid on the small minority of surfaces that
/// otherwise fail, and it can only *add* analyses — never remove or change one.
fn object_suffix_fallback(
    seq: &[Cons],
    roots: Option<&HashSet<[char; 3]>>,
    matches: &mut Vec<VerbMatch>,
    seen: &mut ObjSuffixSeen,
) {
    let max_strip = 2usize.min(seq.len().saturating_sub(2));
    let mut memo: HashMap<[char; 3], Paradigm> = HashMap::new();
    for strip in 0..=max_strip {
        if seq[..strip].iter().any(|c| !PROCLITICS.contains(&c.letter)) {
            continue;
        }
        let remainder = &seq[strip..];
        let prefix = hebrew::render(&seq[..strip]);
        let target_key = canonical_key(&hebrew::render(remainder));
        // Peel a pronominal ending to learn which object Pgns the surface could
        // carry and a reduced stem.
        let peels = peel_object_suffix(remainder);
        if peels.is_empty() {
            continue;
        }
        let obj_pgns: HashSet<Pgn> = peels.iter().map(|(_, p)| *p).collect();
        let stems: Vec<&[Cons]> = peels.iter().map(|(s, _)| s.as_slice()).collect();
        for letters in fallback_roots(&stems, roots) {
            let root = Root::from_letters(letters);
            let paradigm = memo
                .entry(letters)
                .or_insert_with(|| generate_paradigm(&root));
            for vf in paradigm
                .forms
                .iter()
                .filter(|vf| vf.object_suffix.is_none())
            {
                // A Wayyiqtol host carries its own וַ, so it can only match the
                // unstripped surface (strip 0).
                if vf.form == Form::Wayyiqtol && strip > 0 {
                    continue;
                }
                for (obj, surface) in
                    host_object_suffixes(vf.binyan, vf.form, vf.pgn, &vf.text, &root)
                {
                    if !obj_pgns.contains(&obj) || canonical_key(&surface) != target_key {
                        continue;
                    }
                    let vav = vf.form == Form::Wayyiqtol;
                    let key = (
                        letters,
                        vf.binyan,
                        vf.form,
                        vf.pgn,
                        Some(obj),
                        if vav { 0 } else { strip },
                        vav,
                    );
                    if seen.insert(key) {
                        matches.push(VerbMatch {
                            root: Root::from_letters(letters),
                            binyan: vf.binyan,
                            form: vf.form,
                            pgn: vf.pgn,
                            attested: false,
                            prefix: if vav { String::new() } else { prefix.clone() },
                            vav_consecutive: vav,
                            object_suffix: Some(obj),
                            fidelity: MatchFidelity::Folded,
                        });
                    }
                }
            }
        }
    }
}

/// Object-suffix fallback for the **indexed** parser: the lookup twin of
/// [`object_suffix_fallback`]. A suffixed surface that failed to parse is, at
/// each proclitic strip level and with the same sandhi variants the main loop
/// tries, **peeled** ([`peel_object_suffix`]) to its connecting stem(s); the
/// stem key ([`connecting_stem_key`]) finds the host entries in
/// [`ReverseIndex::obj_index`], and the candidate's full canonical surface hash
/// must match one of an entry's `endings` to confirm the exact form. This is the
/// ADR-0004 Option C lookup: the index holds connecting stems, not the
/// host×suffix cross-product, but the surface-hash gate makes a match yield
/// exactly what the old full-surface index did.
///
/// An un-peelable surface (none of the recognised endings apply) is also probed
/// directly under its own canonical-surface key, which is where the builder
/// files such surfaces — so coverage is a superset of the peel paths and nothing
/// the old index held becomes unreachable.
///
/// The gating on a failed parse keeps this in step with the generate-and-test
/// parser (neither fires for a surface that already parses); the work is a peel
/// plus a hash lookup, so it stays cheap even when run over every word of a text.
fn object_suffix_fallback_indexed(
    seq: &[Cons],
    index: &ReverseIndex,
    roots: Option<&HashSet<[char; 3]>>,
    matches: &mut Vec<VerbMatch>,
    seen: &mut ObjSuffixSeen,
) {
    let in_filter = |letters: &[char; 3]| roots.is_none_or(|set| set.contains(letters));
    let max_strip = 2usize.min(seq.len().saturating_sub(2));
    for strip in 0..=max_strip {
        if seq[..strip].iter().any(|c| !PROCLITICS.contains(&c.letter)) {
            continue;
        }
        let remainder = &seq[strip..];
        let prefix = hebrew::render(&seq[..strip]);
        for t in peeling_targets(seq, strip, remainder) {
            let target_hash = (key_hash(&canonical_key(&t)) as u64) & ENDING_HASH_MASK;
            // Stem keys to probe: every peel of the target, plus the target's own
            // canonical-surface key for un-peelable entries the builder filed
            // whole. The surface-hash gate below rejects any spurious stem hit.
            let mut keys: Vec<u64> = peel_object_suffix(&hebrew::parse_pointed(&t))
                .iter()
                .map(|(s, _)| connecting_stem_key(s))
                .collect();
            keys.push(key_hash(&canonical_key(&t)) as u64);
            keys.sort_unstable();
            keys.dedup();
            for k in keys {
                for (_, e) in index.obj_get(k) {
                    let letters = e.letters();
                    if !in_filter(&letters) {
                        continue;
                    }
                    // A Wayyiqtol host carries its own וַ — only the unstripped
                    // surface (strip 0) can match it.
                    let vav = e.form == Form::Wayyiqtol;
                    if vav && strip > 0 {
                        continue;
                    }
                    for &packed in index
                        .obj_endings(e)
                        .iter()
                        .filter(|&&p| p & ENDING_HASH_MASK == target_hash)
                    {
                        let obj = pgn_decode((packed & 0xFF) as u8);
                        let key = (
                            letters,
                            e.binyan,
                            e.form,
                            e.pgn,
                            Some(obj),
                            if vav { 0 } else { strip },
                            vav,
                        );
                        if seen.insert(key) {
                            matches.push(VerbMatch {
                                root: Root::from_letters(letters),
                                binyan: e.binyan,
                                form: e.form,
                                pgn: e.pgn,
                                attested: false,
                                prefix: if vav { String::new() } else { prefix.clone() },
                                vav_consecutive: vav,
                                object_suffix: Some(obj),
                                fidelity: MatchFidelity::Folded,
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Label twins added after matching, shared by the generate-and-test and
/// index-backed parsers so they cannot diverge.
///
/// - **Jussive → Imperfect**: the "short imperfect" is morphology, not mood —
///   gold analyses tag forms like יְהִי and וְיָשֹׁב as plain Imperfect when the
///   context isn't volitive. Every Jussive analysis therefore also stands as an
///   Imperfect analysis of the same surface.
/// - **וַ-peel → Wayyiqtol**: a match found by peeling a patah/qamats vav off an
///   Imperfect/Jussive stem (the peel's article rule absorbs the forte dagesh:
///   וַיֶּאֱהַב, וַיִּצֹק) *is* a wayyiqtol — the dedicated Wayyiqtol forms only
///   cover the short-base spellings, so re-label the peeled analyses too.
fn add_label_twins(matches: &mut Vec<VerbMatch>) {
    let mut seen: std::collections::HashSet<_> = matches
        .iter()
        .map(|m| {
            (
                m.root.letters,
                m.binyan,
                m.form,
                m.pgn,
                m.object_suffix,
                m.prefix.clone(),
                m.vav_consecutive,
            )
        })
        .collect();
    let mut twins = Vec::new();
    for m in matches.iter() {
        if m.form == Form::Jussive {
            let mut t = m.clone();
            t.form = Form::Imperfect;
            twins.push(t);
        }
        // And the reverse: a long imperfect surface is tagged Jussive when the
        // context is volitive (יִהְיוּ "let them be").
        if m.form == Form::Imperfect {
            let mut t = m.clone();
            t.form = Form::Jussive;
            twins.push(t);
        }
        // A paragogic-he first-person form is tagged either Cohortative or
        // plain Imperfect by OSHB (אַגִּידָה vs וְאַגִּידָה) — emit both labels.
        if m.form == Form::Cohortative {
            let mut t = m.clone();
            t.form = Form::Imperfect;
            twins.push(t);
        }
        let vav_peel = {
            let mut ch = m.prefix.chars();
            ch.next() == Some('\u{05D5}')
                && matches!(ch.next(), Some('\u{05B7}' | '\u{05B8}'))
                && ch.next().is_none()
        };
        if vav_peel && matches!(m.form, Form::Imperfect | Form::Jussive) {
            let mut t = m.clone();
            t.form = Form::Wayyiqtol;
            t.prefix = String::new();
            t.vav_consecutive = true;
            twins.push(t);
        }
    }
    for t in twins {
        let key = (
            t.root.letters,
            t.binyan,
            t.form,
            t.pgn,
            t.object_suffix,
            t.prefix.clone(),
            t.vav_consecutive,
        );
        if seen.insert(key) {
            matches.push(t);
        }
    }
}

/// Assign each match its [`MatchFidelity`] by testing whether the surface stem
/// is byte-identical to one of the generator's spellings for the candidate's
/// cell (via the [`cached_root_fidelity`] per-root index). Shared by both parse
/// entry points (the indexed parser keeps only a key hash, not the generated
/// spelling, so neither path can set this at match time) so the two rank
/// identically. Twin-aware: the cell's set holds every alternant spelling, so an
/// exact match against any twin counts.
fn assign_fidelity(seq: &[Cons], matches: &mut [VerbMatch]) {
    // Nothing to rank with a single (or no) candidate, so skip the paradigm
    // regeneration entirely — this is the overwhelming majority of surfaces in
    // a bulk build, and the leading cost saver there.
    if matches.len() <= 1 {
        return;
    }
    if is_unpointed(seq) {
        for m in matches.iter_mut() {
            m.fidelity = MatchFidelity::Skeleton;
        }
        return;
    }
    let normalized = undot(&hebrew::render(seq));
    for m in matches.iter_mut() {
        // The surface stem this candidate must spell exactly: the normalised
        // surface with the match's proclitic prefix peeled off the front.
        let undot_prefix = undot(&m.prefix);
        let Some(stem) = normalized.strip_prefix(&undot_prefix) else {
            continue;
        };
        let idx = cached_root_fidelity(m.root.letters);
        let key = cell_key(m.binyan, m.form, m.pgn, m.object_suffix);
        if idx.cells.get(&key).is_some_and(|set| set.contains(stem)) {
            m.fidelity = MatchFidelity::Exact;
        }
    }
}

/// Strip the sin/shin dot: the generator does not always emit it and
/// `canonical_key` folds it out anyway, so a missing dot is a cosmetic
/// difference, not the kind of spelling fold (dagesh, mater, hataf) that should
/// demote a reading from exact.
fn undot(s: &str) -> String {
    s.chars()
        .filter(|&c| c != '\u{05C1}' && c != '\u{05C2}')
        .collect()
}

/// [`MatchFidelity`] for an indexed match: `Exact` when the surface stem's raw
/// (undotted) spelling hash equals the matched entry's, else `Folded` (the match
/// survived only a `canonical_key` fold). See [`IndexEntry::raw_hash`].
fn fidelity_from_raw(entry_raw: u32, target_raw: u32) -> MatchFidelity {
    if entry_raw == target_raw {
        MatchFidelity::Exact
    } else {
        MatchFidelity::Folded
    }
}

/// A cell key: the (binyan, form, pgn, object-suffix) tuple identifying one
/// paradigm slot, as small hashable scalars/labels.
type CellKey = (usize, usize, String, Option<String>);

fn cell_key(binyan: Binyan, form: Form, pgn: Pgn, suffix: Option<Pgn>) -> CellKey {
    (
        binyan as usize,
        form as usize,
        pgn.label(),
        suffix.map(|p| p.label()),
    )
}

/// Per-root exact-spelling index for [`MatchFidelity`]: each paradigm cell
/// mapped to the set of undotted spellings the generator emits for it (one per
/// alternant twin). A candidate is `Exact` when the surface stem (the surface
/// with the match's proclitic prefix peeled) is in its cell's set.
struct RootFidelity {
    cells: HashMap<CellKey, HashSet<String>>,
}

/// Process-wide cache of [`RootFidelity`] indexes, keyed by root letters. Built
/// once per root from its paradigm and reused for every surface that shares the
/// root, turning fidelity assignment into an O(1) lookup per candidate instead
/// of an allocate-and-compare scan over the whole paradigm. `generate_paradigm`
/// is pure, so a stale read is impossible; the `RwLock` lets the rayon-parallel
/// build read concurrently and serialises only the first-miss insert per root.
static FIDELITY_CACHE: LazyLock<RwLock<HashMap<[char; 3], Arc<RootFidelity>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

fn cached_root_fidelity(letters: [char; 3]) -> Arc<RootFidelity> {
    if let Some(p) = FIDELITY_CACHE.read().unwrap().get(&letters) {
        return p.clone();
    }
    let paradigm = generate_paradigm(&Root::from_letters(letters));
    let mut cells: HashMap<CellKey, HashSet<String>> = HashMap::new();
    for vf in &paradigm.forms {
        cells
            .entry(cell_key(vf.binyan, vf.form, vf.pgn, vf.object_suffix))
            .or_default()
            .insert(undot(&vf.text));
    }
    let idx = Arc::new(RootFidelity { cells });
    FIDELITY_CACHE
        .write()
        .unwrap()
        .entry(letters)
        .or_insert(idx)
        .clone()
}

/// Stable ordering shared by all parse entry points: attested (fully-modelled)
/// analyses first, bare forms before object-suffixed, then exact spelling
/// matches before fold-rescued ones, then by root/binyan/form/pgn/suffix.
///
/// Fidelity is deliberately a *tiebreaker below* the attested and bare-vs-
/// suffixed priors, not the primary key. An exact byte-match is not the same as
/// the correct reading — a spurious analysis can spell the surface exactly while
/// the intended one only matched through a fold — so ranking fidelity first
/// promotes those spurious exact matches over more plausible folded readings
/// (Hophal-imperative-of-a-weak-root outranking the obvious parse). Kept below
/// the proven priors, it only discriminates *within* an equally-ranked class —
/// e.g. two bare modelled forms — where preferring the exact spelling is safe.
fn sort_matches(matches: &mut [VerbMatch]) {
    matches.sort_by(|a, b| {
        b.attested
            .cmp(&a.attested)
            .then_with(|| a.object_suffix.is_some().cmp(&b.object_suffix.is_some()))
            .then_with(|| a.fidelity.cmp(&b.fidelity))
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
    // Fully-unpointed ketiv surfaces can't reach a vocalised candidate through
    // canonical_key, so match on the bare consonant skeleton instead.
    let unpointed = is_unpointed(&seq);

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
        let target_skels: Vec<String> = if unpointed {
            targets.iter().map(|t| ketiv_skeleton(t)).collect()
        } else {
            Vec::new()
        };

        for letters in candidate_roots(remainder, roots) {
            let paradigm = memo
                .entry(letters)
                .or_insert_with(|| generate_paradigm(&Root::from_letters(letters)));
            for vf in &paradigm.forms {
                let hit = if unpointed {
                    target_skels.contains(&ketiv_skeleton(&vf.text))
                } else {
                    forms_match(&vf.text, &targets, &target_keys)
                };
                // Wayyiqtol is handled by the dedicated pass below, with the
                // consecutive vav recognised as such rather than as a plain
                // proclitic; skip it here to avoid duplicate analyses.
                if vf.form == Form::Wayyiqtol || !hit {
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
                        fidelity: MatchFidelity::Folded,
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
        c.letter == letter::VAV
            && (matches!(c.vowel, Some(Vowel::Patah | Vowel::Qamats)) || unpointed)
    }) && seq.len() >= 3
    {
        let targets = [hebrew::render(&seq)];
        let target_keys = [canonical_key(&targets[0])];
        let target_skel = ketiv_skeleton(&targets[0]);
        for letters in candidate_roots(&seq[1..], roots) {
            let paradigm = memo
                .entry(letters)
                .or_insert_with(|| generate_paradigm(&Root::from_letters(letters)));
            for vf in &paradigm.forms {
                let hit = if unpointed {
                    ketiv_skeleton(&vf.text) == target_skel
                } else {
                    forms_match(&vf.text, &targets, &target_keys)
                };
                if vf.form != Form::Wayyiqtol || !hit {
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
                        fidelity: MatchFidelity::Folded,
                    });
                }
            }
        }
    }

    if matches.is_empty() {
        object_suffix_fallback(&seq, roots, &mut matches, &mut seen);
    }
    add_label_twins(&mut matches);
    assign_fidelity(&seq, &mut matches);
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
    /// A `u32` hash of the *raw* generated spelling (sin/shin dot stripped, see
    /// [`undot`]) — the exactness signal for [`MatchFidelity`]. The `u128` key is
    /// the `canonical_key` hash (folded), so it cannot tell an exact spelling
    /// from a fold-rescued one; this lets the indexed parser do that in O(1) by
    /// comparing against the surface stem's raw hash, without regenerating the
    /// paradigm. `u32` (not `u64`) so it fits the struct's existing alignment
    /// padding — zero added footprint across the ~54M-entry index. A 32-bit
    /// collision only ever mislabels one folded reading as exact — a ranking
    /// nudge, never a recall change.
    raw_hash: u32,
}

impl IndexEntry {
    /// The radical letters as Hebrew `char`s, decoded from the packed indices.
    fn letters(&self) -> [char; 3] {
        self.radicals.map(|i| HEBREW_CONSONANTS[i as usize])
    }
}

/// An object-suffix host in the [`ReverseIndex::obj_index`], keyed by its
/// *connecting stem* rather than by each suffixed surface (ADR-0004 Option C).
///
/// The generator expands every bare host into ~15 suffixed surfaces. Indexing
/// each surface separately (the earlier design) repeats the host's
/// `(binyan, form, pgn)` metadata 15× and multiplies the obj index. Instead we
/// file the host once per *connecting-stem key* ([`connecting_stem_key`] — the
/// reduced stem the suffixes attach to, with its final link vowel cleared so all
/// suffixes of one grade share a key) and carry the suffixed surfaces as a run of
/// packed `u64` endings ([`pack_ending`]) in the shared `obj_endings` arena. The
/// stem key only narrows the candidate set; the per-ending surface hash is the
/// exactness gate, so a lookup yields exactly what the full-surface index did —
/// the equivalence is by construction, since build and parse both reach the key
/// through the same [`peel_object_suffix`] + `canonical_key`.
///
/// `attested` is always false for object-suffixed forms, so it is not stored.
/// The host's suffixed surfaces are not stored inline — a `Vec` per entry would
/// cost a header plus an allocation across tens of millions of entries — but as
/// a contiguous `[end_start, end_start + end_len)` run of packed endings in the
/// index's shared [`ReverseIndex::obj_endings`] arena (CSR layout).
#[derive(Clone)]
struct ObjStemEntry {
    radicals: [u8; 3],
    binyan: Binyan,
    form: Form,
    pgn: Pgn,
    end_start: u32,
    end_len: u32,
}

impl ObjStemEntry {
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
    letter::ALEF,
    letter::BET,
    letter::GIMEL,
    letter::DALET,
    letter::HE,
    letter::VAV,
    letter::ZAYIN,
    letter::HET,
    letter::TET,
    letter::YOD,
    letter::KAF,
    letter::LAMED,
    letter::MEM,
    letter::NUN,
    letter::SAMEKH,
    letter::AYIN,
    letter::PE,
    letter::TSADE,
    letter::QOF,
    letter::RESH,
    letter::SHIN,
    letter::TAV,
];

/// A reverse lookup index mapping each generated form's `canonical_key` to the
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
    /// Object-suffixed forms of every bare host (via [`host_object_suffixes`]),
    /// keyed like `entries` and consulted only by the indexed parser's
    /// object-suffix fallback. Kept separate from `entries` so it is queried
    /// solely when a surface fails to parse — that gating is what keeps the
    /// indexed and generate-and-test parsers in agreement on parsing surfaces
    /// (the fallback never fires for them). Each entry's `object_suffix` is set.
    /// This precomputes the suffix expansion once at build, so the fallback is a
    /// hash lookup, not a per-surface generate-and-test (which would cost minutes
    /// over a full text, since it fires on every non-verb word too).
    ///
    /// Keyed by *connecting stem* ([`connecting_stem_key`]), not by suffixed
    /// surface — see [`ObjStemEntry`]. Each entry holds the host metadata once
    /// and points at a run of its suffixed surfaces in `obj_endings`, so the obj
    /// index no longer stores the full host×suffix cross-product. The key is a
    /// `u64` (not the `u128` of `entries`) because the per-ending surface hash,
    /// not the stem key, is the exactness gate, so a stem-key collision is
    /// harmless (it only widens the candidate scan).
    obj_index: Vec<(u64, ObjStemEntry)>,
    /// CSR arena of packed endings ([`pack_ending`]) shared by every
    /// [`ObjStemEntry`]; an entry's endings are `obj_endings[end_start..][..end_len]`.
    obj_endings: Vec<u64>,
}

/// A 128-bit hash of a [`canonical_key`], used as the [`ReverseIndex`] key in
/// place of the owned `String`. Two independently-seeded 64-bit SipHashes are
/// concatenated; `DefaultHasher::new` uses fixed keys, so the hash is
/// deterministic across runs. Collisions across the index's ~34M distinct keys
/// are vanishingly improbable in a 128-bit space.
fn key_hash(canonical: &str) -> u128 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hi = DefaultHasher::new();
    canonical.hash(&mut hi);
    let mut lo = DefaultHasher::new();
    // A distinct salt so the two halves are independent.
    lo.write_u8(0x9e);
    canonical.hash(&mut lo);
    ((hi.finish() as u128) << 64) | (lo.finish() as u128)
}

/// The connecting-stem key for an object-suffix host (ADR-0004 Option C): the
/// hash of the canonical reduced stem with its final consonant's vowel cleared.
///
/// [`peel_object_suffix`] strips the suffix consonants but leaves the *linking*
/// vowel on the last stem consonant (qᵊṭāl-**a**-nî, qᵊṭāl-**ᵊ**-ḵā, qᵊṭāl-ô).
/// Clearing it folds every suffix of one host grade onto a single key, which is
/// what collapses the host×suffix cross-product. Exactness is not lost: the
/// caller still gates on the full canonical surface hash, so the key need only
/// be a deterministic function of the peeled stem that build and parse share.
///
/// The stem is canonicalised *before* the final vowel is cleared, not after: a
/// plene stem ending in a holam/shureq **mater** (a vav written for the vowel)
/// must fold onto the preceding consonant — as `canonical_key` does — so it keys
/// identically to the defective spelling the generator emits. Clearing first
/// would strip the mater's vowel and leave a bare vav that no longer folds,
/// splitting plene and defective spellings of one stem into two keys and dropping
/// the plene surface (it still passes the surface-hash gate, but the stem lookup
/// never reaches it).
fn connecting_stem_key(stem: &[Cons]) -> u64 {
    let mut s = hebrew::parse_pointed(&canonical_key(&hebrew::render(stem)));
    if let Some(last) = s.last_mut() {
        last.vowel = None;
    }
    key_hash(&hebrew::render(&s)) as u64
}

/// Pack an object-suffix [`Pgn`] into 6 bits (the low byte of a stored ending).
/// Object suffixes always fill all three axes, but the codec round-trips `None`
/// axes too (code 0) so it is total. Each axis is 0–3, two bits each.
fn pgn_code(p: Pgn) -> u8 {
    let person = match p.person {
        None => 0,
        Some(Person::First) => 1,
        Some(Person::Second) => 2,
        Some(Person::Third) => 3,
    };
    let gender = match p.gender {
        None => 0,
        Some(Gender::Masculine) => 1,
        Some(Gender::Feminine) => 2,
        Some(Gender::Common) => 3,
    };
    let number = match p.number {
        None => 0,
        Some(Number::Singular) => 1,
        Some(Number::Plural) => 2,
        Some(Number::Dual) => 3,
    };
    (person << 4) | (gender << 2) | number
}

/// Inverse of [`pgn_code`].
fn pgn_decode(code: u8) -> Pgn {
    let person = match (code >> 4) & 0b11 {
        1 => Some(Person::First),
        2 => Some(Person::Second),
        3 => Some(Person::Third),
        _ => None,
    };
    let gender = match (code >> 2) & 0b11 {
        1 => Some(Gender::Masculine),
        2 => Some(Gender::Feminine),
        3 => Some(Gender::Common),
        _ => None,
    };
    let number = match code & 0b11 {
        1 => Some(Number::Singular),
        2 => Some(Number::Plural),
        3 => Some(Number::Dual),
        _ => None,
    };
    Pgn {
        person,
        gender,
        number,
    }
}

/// An object-suffix host's suffixed surface, packed into one `u64` for the CSR
/// ending arena: the high 56 bits are the canonical surface hash, the low 8 the
/// suffix [`pgn_code`]. The 56-bit hash is the exactness gate; sacrificing the
/// low 8 bits of the hash keeps the ending at 8 bytes, and a 56-bit collision
/// would have to land inside the same stem entry's handful of endings to mislead.
const ENDING_HASH_MASK: u64 = !0xFFu64;

fn pack_ending(surf_hash: u64, obj: Pgn) -> u64 {
    (surf_hash & ENDING_HASH_MASK) | pgn_code(obj) as u64
}

impl ReverseIndex {
    /// Generate the paradigm of every one of the 22³ triliteral roots and index
    /// every form by its canonical key. The all-roots space is a superset of any
    /// surface's candidate roots, and a non-matching root simply contributes keys
    /// no surface hits — so lookups return exactly what [`parse_word`] would find.
    ///
    /// Each form's key is a 128-bit hash of its `canonical_key` (see
    /// `key_hash`): 16 inline bytes, no allocation, and at 128 bits an
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
        Self::build_from_roots(&roots)
    }

    /// Build a [`ReverseIndex`] over an explicit set of roots rather than the
    /// whole 22³ space. [`build`](Self::build) is this over every triliteral;
    /// tests use it to construct a small, cheap index whose lookups are still
    /// exact for the roots it contains (a non-indexed root simply never matches).
    fn build_from_roots(roots: &[[char; 3]]) -> Self {
        type RootBuild = (Vec<(u128, IndexEntry)>, Vec<(u64, ObjStemEntry)>, Vec<u64>);
        let per_root: Vec<RootBuild> = roots
            .par_iter()
            .map(|&letters| {
                let radicals = pack_radicals(letters);
                let root = Root::from_letters(letters);
                let para = generate_paradigm(&root);
                let entries: Vec<(u128, IndexEntry)> = para
                    .forms
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
                                raw_hash: key_hash(&undot(&vf.text)) as u32,
                            },
                        )
                    })
                    .collect();
                // Object-suffix the bare hosts (twins included, since every twin
                // is its own bare `forms` entry). This catches the host grades the
                // generator's own suffix dispatch never threads — the obj-suffix
                // coverage gap — without expanding the host×suffix cross-product
                // into `entries`. Filed per *connecting stem* (ADR-0004 Option C):
                // each host's ~15 suffixed surfaces collapse onto the few stem
                // grades they attach to, carried as packed endings in this root's
                // local arena (offset into the global arena at assembly time).
                let mut obj: Vec<(u64, ObjStemEntry)> = Vec::new();
                let mut endings_arena: Vec<u64> = Vec::new();
                for vf in para.forms.iter().filter(|vf| vf.object_suffix.is_none()) {
                    // Group this host's suffixed surfaces by connecting-stem key.
                    let mut by_stem: HashMap<u64, Vec<u64>> = HashMap::new();
                    for (obj_pgn, surface) in
                        host_object_suffixes(vf.binyan, vf.form, vf.pgn, &vf.text, &root)
                    {
                        let packed =
                            pack_ending(key_hash(&canonical_key(&surface)) as u64, obj_pgn);
                        let peels = peel_object_suffix(&hebrew::parse_pointed(&surface));
                        // The stem key(s) this surface lands under. An un-peelable
                        // surface is filed under its own full-surface key, which
                        // the parser also probes directly, so it stays reachable.
                        let mut keys: Vec<u64> = if peels.is_empty() {
                            vec![key_hash(&canonical_key(&surface)) as u64]
                        } else {
                            peels.iter().map(|(s, _)| connecting_stem_key(s)).collect()
                        };
                        keys.sort_unstable();
                        keys.dedup();
                        for k in keys {
                            let v = by_stem.entry(k).or_default();
                            if !v.contains(&packed) {
                                v.push(packed);
                            }
                        }
                    }
                    for (stem_key, endings) in by_stem {
                        let end_start = endings_arena.len() as u32;
                        endings_arena.extend_from_slice(&endings);
                        obj.push((
                            stem_key,
                            ObjStemEntry {
                                radicals,
                                binyan: vf.binyan,
                                form: vf.form,
                                pgn: vf.pgn,
                                end_start,
                                end_len: endings.len() as u32,
                            },
                        ));
                    }
                }
                (entries, obj, endings_arena)
            })
            .collect();

        let total: usize = per_root.iter().map(|(e, _, _)| e.len()).sum();
        let obj_total: usize = per_root.iter().map(|(_, o, _)| o.len()).sum();
        let end_total: usize = per_root.iter().map(|(_, _, d)| d.len()).sum();
        let mut entries: Vec<(u128, IndexEntry)> = Vec::with_capacity(total);
        let mut obj_index: Vec<(u64, ObjStemEntry)> = Vec::with_capacity(obj_total);
        let mut obj_endings: Vec<u64> = Vec::with_capacity(end_total);
        for (chunk, mut obj, endings) in per_root {
            entries.extend(chunk);
            // Re-base each entry's arena range onto the growing global arena
            // before appending this root's endings to it.
            let offset = obj_endings.len() as u32;
            for (_, e) in obj.iter_mut() {
                e.end_start += offset;
            }
            obj_index.extend(obj);
            obj_endings.extend(endings);
        }
        entries.par_sort_unstable_by_key(|&(k, _)| k);
        // Sorting reorders the (key, entry) pairs; each entry's arena range
        // travels with it and the arena itself is untouched, so the ranges stay
        // valid.
        obj_index.par_sort_unstable_by_key(|&(k, _)| k);
        ReverseIndex {
            entries,
            obj_index,
            obj_endings,
        }
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

    /// Like [`ReverseIndex::get`] but over the object-suffix index — the run of
    /// connecting-stem hosts whose stem-key hash equals `key`.
    fn obj_get(&self, key: u64) -> &[(u64, ObjStemEntry)] {
        let start = self.obj_index.partition_point(|&(k, _)| k < key);
        let mut end = start;
        while end < self.obj_index.len() && self.obj_index[end].0 == key {
            end += 1;
        }
        &self.obj_index[start..end]
    }

    /// The packed endings ([`pack_ending`]) of an [`ObjStemEntry`], read from the
    /// shared CSR arena.
    fn obj_endings(&self, e: &ObjStemEntry) -> &[u64] {
        let start = e.end_start as usize;
        &self.obj_endings[start..start + e.end_len as usize]
    }
}

/// Index-backed twin of [`parse_word_filtered`]: identical proclitic peeling,
/// sandhi target variants, wayyiqtol handling, dedup and ordering, but each
/// candidate-root enumeration + paradigm match is replaced by a hash lookup of
/// the target's `canonical_key`. With the same `roots` filter it returns the
/// same analyses as `parse_word_filtered`, far faster in bulk.
pub fn parse_word_indexed(
    word: &str,
    index: &ReverseIndex,
    roots: Option<&HashSet<[char; 3]>>,
) -> Vec<VerbMatch> {
    let seq = hebrew::parse_pointed(word);
    // The index is keyed by canonical_key, which keeps vowels, so a fully
    // unpointed ketiv surface can never hit it. Fall back to the generate-and-
    // test parser, which skeleton-matches such surfaces (and stays fast because
    // the root filter bounds the enumeration).
    if is_unpointed(&seq) {
        return parse_word_filtered(word, roots);
    }
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
            // The surface stem's raw hash: exact iff it equals a matched entry's
            // raw spelling hash (see [`IndexEntry::raw_hash`]).
            let t_raw = key_hash(&undot(t)) as u32;
            for (_, e) in index.get(key_hash(&canonical_key(t))) {
                let letters = e.letters();
                // Wayyiqtol is handled by the dedicated pass below.
                if e.form == Form::Wayyiqtol || !in_filter(&letters) {
                    continue;
                }
                if seen.insert((
                    letters,
                    e.binyan,
                    e.form,
                    e.pgn,
                    e.object_suffix,
                    strip,
                    false,
                )) {
                    matches.push(VerbMatch {
                        root: Root::from_letters(letters),
                        binyan: e.binyan,
                        form: e.form,
                        pgn: e.pgn,
                        attested: e.attested,
                        prefix: prefix.clone(),
                        vav_consecutive: false,
                        object_suffix: e.object_suffix,
                        fidelity: fidelity_from_raw(e.raw_hash, t_raw),
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
        let whole = hebrew::render(&seq);
        let t_raw = key_hash(&undot(&whole)) as u32;
        let key = key_hash(&canonical_key(&whole));
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
                    fidelity: fidelity_from_raw(e.raw_hash, t_raw),
                });
            }
        }
    }

    if matches.is_empty() {
        object_suffix_fallback_indexed(&seq, index, roots, &mut matches, &mut seen);
    }
    add_label_twins(&mut matches);
    // Fidelity is set inline above from each entry's raw-spelling hash — an O(1)
    // exactness check that needs no paradigm regeneration. The object-suffix
    // fallback (fires only when nothing else matched) and label twins keep the
    // `Folded` default. Unpointed-ketiv surfaces never reach here — they are
    // delegated to the live parser at the top of this function.
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
    disambiguate_matches(parse_word(word), roots)
}

/// Index-backed twin of [`parse_word_disambiguated`]: the unrestricted parse is
/// a [`ReverseIndex`] lookup instead of generate-and-test, then the same
/// disambiguate-only inventory filter is applied to the result.
pub fn parse_word_indexed_disambiguated(
    word: &str,
    index: &ReverseIndex,
    roots: Option<&HashSet<[char; 3]>>,
) -> Vec<VerbMatch> {
    // An unpointed ketiv surface bypasses the index; route it through the
    // root-filtered generate-and-test parser directly rather than the
    // unrestricted index lookup (which would enumerate every triliteral on the
    // skeleton fallback). The inventory filter is the disambiguation here.
    if is_unpointed(&hebrew::parse_pointed(word)) {
        return parse_word_filtered(word, roots);
    }
    disambiguate_matches(parse_word_indexed(word, index, None), roots)
}

/// The disambiguate-only inventory filter shared by the generate-and-test and
/// index-backed soft parsers (see [`parse_word_disambiguated`] for semantics).
///
/// Exposed so a caller that already holds a parsed candidate list can thin it
/// against the lexicon without re-parsing (the db build applies it as a
/// post-filter so the prefilter's proper-noun rescue still sees every
/// candidate). Recall-neutral for in-inventory roots: it never empties a list
/// that has any in-inventory candidate, so it only prunes over-generated
/// ambiguity. When *every* candidate is out-of-inventory it preserves them (a
/// genuine weak verb whose canonical-root paradigm can't spell the surface
/// survives, pending curation) — except fully-collapsed all-weak roots, which
/// are dropped (see below).
pub fn disambiguate_matches(
    matches: Vec<VerbMatch>,
    roots: Option<&HashSet<[char; 3]>>,
) -> Vec<VerbMatch> {
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
    if !kept.is_empty() {
        return kept;
    }
    // Fallback: every candidate is out-of-inventory. Preserve them so a genuine
    // weak verb whose canonical root can't spell the surface survives — but drop
    // any root made *entirely* of weak letters (ייי, יוי, נוי). A real verb
    // always leaves a strong radical in the surface, so an all-weak out-of-
    // inventory root only ever matches a noun/numeral fragment: שְׁנֵי "two of"
    // reads as a 1cp imperfect of ייי once שְׁ is peeled and נ taken as the
    // preformative. Real all-weak roots (היה, ינה, נוה) are in the inventory and
    // never reach this branch.
    matches
        .into_iter()
        .filter(|m| !m.root.letters.iter().all(|c| WEAK.contains(c)))
        .collect()
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

    /// Does `peel_object_suffix` recover the expected (stem, object-Pgn) for a
    /// known suffixed surface?
    fn peels_to(surface: &str, stem: &str, obj: Pgn) -> bool {
        // Normalise the expected stem through the same parse/render round-trip
        // so combining-mark order matches the peeled output.
        let want = hebrew::render(&hebrew::parse_pointed(stem));
        let seq = hebrew::parse_pointed(surface);
        peel_object_suffix(&seq)
            .into_iter()
            .any(|(s, p)| p == obj && hebrew::render(&s) == want)
    }

    #[test]
    fn peels_pronominal_object_suffixes() {
        let p = |pe, g, n| Pgn::new(pe, g, n);
        // yišmᵊrēhû יִשְׁמְרֵהוּ → host yišmᵊrē + 3ms.
        assert!(peels_to(
            "יִשְׁמְרֵהוּ",
            "יִשְׁמְרֵ",
            p(Person::Third, Gender::Masculine, Number::Singular)
        ));
        // yišmᵊrēnî יִשְׁמְרֵנִי → 1cs.
        assert!(peels_to(
            "יִשְׁמְרֵנִי",
            "יִשְׁמְרֵ",
            p(Person::First, Gender::Common, Number::Singular)
        ));
        // energic yišmᵊrennû יִשְׁמְרֶנּוּ → 3ms.
        assert!(peels_to(
            "יִשְׁמְרֶנּוּ",
            "יִשְׁמְרֶ",
            p(Person::Third, Gender::Masculine, Number::Singular)
        ));
        // yišmārḵā יִשְׁמָרְךָ → 2ms.
        assert!(peels_to(
            "יִשְׁמָרְךָ",
            "יִשְׁמָרְ",
            p(Person::Second, Gender::Masculine, Number::Singular)
        ));
        // qûmēnî קוּמֵנִי (hollow imperative host) → 1cs.
        assert!(peels_to(
            "קוּמֵנִי",
            "קוּמֵ",
            p(Person::First, Gender::Common, Number::Singular)
        ));
        // A bare form with no pronominal ending peels nothing spurious as 3ms-hû.
        let bare = hebrew::parse_pointed("יִשְׁמֹר");
        assert!(
            !peel_object_suffix(&bare)
                .iter()
                .any(|(_, p)| *p == Pgn::new(Person::Third, Gender::Masculine, Number::Singular))
        );
    }

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
    fn parses_lamed_aleph_imperfect_nun() {
        // תִּקְרֶאנָה — III-aleph 3fp imperfect of קרא: the quiescent aleph
        // takes no vowel and C2 (resh) carries segol before the -nâ ending.
        let matches = parse_word("תִּקְרֶאנָה");
        assert!(has_match(
            &matches,
            "קרא",
            Binyan::Qal,
            Form::Imperfect,
            "3fp"
        ));
    }

    #[test]
    fn parses_hollow_hiphil_perfect_tsere_grade() {
        // הֲקֵמֹתָ — Hiphil perfect 2ms of קום in the tsere grade of the
        // linking-ô stem (beside the î-plene הֲקִימֹתָ).
        let matches = parse_word("הֲקֵמֹתָ");
        assert!(has_match(
            &matches,
            "קומ",
            Binyan::Hiphil,
            Form::Perfect,
            "2ms"
        ));
    }

    #[test]
    fn parses_pe_guttural_a_theme_pausal_plural() {
        // יֶחְדָּלוּ — stative I-guttural 3mp imperfect of חדל: the silent-sheva
        // C1 with the patah theme restored to qamats under the bare-plural stress.
        let matches = parse_word("יֶחְדָּלוּ");
        assert!(has_match(
            &matches,
            "חדל",
            Binyan::Qal,
            Form::Imperfect,
            "3mp"
        ));
    }

    #[test]
    fn parses_unpointed_ketiv_wayyiqtol() {
        // ויאמר — an unpointed ketiv surface. Its niqqud-less skeleton can't
        // reach the vocalised candidates through canonical_key, so the parser
        // falls back to a consonant-skeleton match: וַיֹּאמֶר (Qal Wayyiqtol 3ms
        // of אמר) shares the skeleton ויאמר.
        let matches = parse_word("ויאמר");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "אמר"
                && m.binyan == Binyan::Qal
                && m.form == Form::Wayyiqtol
                && m.pgn.label() == "3ms"
                && m.vav_consecutive
        }));
    }

    #[test]
    fn parses_unpointed_ketiv_plene_imperfect() {
        // אכתוב — an unpointed ketiv that spells the holam plene with a vav,
        // where the generator writes the Qal imperfect 1cs defective (אֶכְתֹּב,
        // skeleton אכתב). Folding the matres off both skeletons lets ketiv
        // plene align with generated defective.
        let matches = parse_word("אכתוב");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "כתב"
                && m.binyan == Binyan::Qal
                && m.form == Form::Imperfect
                && m.pgn.label() == "1cs"
        }));
    }

    #[test]
    fn parses_pointed_quadriliteral_trumpeter() {
        // מַחְצְרִים — the qere of the quadriliteral trumpeter (H2690 חצצר),
        // tagged Hiphil participle active mp. Its pointed surface is already
        // triliteral-shaped (root חצר, ח/צ/ר); the silent-sheva grade of the
        // reduced Hiphil participle (maḥṣᵊrîm) is what occurs.
        let matches = parse_word("מַחְצְרִים");
        assert!(has_match(
            &matches,
            "חצר",
            Binyan::Hiphil,
            Form::ParticipleActive,
            "mp"
        ));
    }

    #[test]
    fn parses_unpointed_quadriliteral_ketiv() {
        // מחצצרים — the plene ketiv of the same trumpeter, spelling the
        // reduplicated radical out (ח-צ-צ-ר). Collapsing the doubled צ on the
        // skeleton converts the quadriliteral to its triliteral חצר paradigm so
        // the Hiphil participle active mp is recovered.
        let matches = parse_word("מחצצרים");
        assert!(matches.iter().any(|m| {
            m.root.letters.iter().collect::<String>() == "חצר"
                && m.binyan == Binyan::Hiphil
                && m.form == Form::ParticipleActive
                && m.pgn.label() == "mp"
        }));
    }

    #[test]
    fn pointed_word_is_not_skeleton_matched() {
        // A normally-pointed surface must NOT pick up skeleton matches: שָׁמַר
        // parses to exactly its Qal perfect 3ms, not to every form sharing the
        // skeleton שמר (e.g. an imperfect would need different vowels).
        let matches = parse_word("שָׁמַר");
        assert!(has_match(
            &matches,
            "שמר",
            Binyan::Qal,
            Form::Perfect,
            "3ms"
        ));
        assert!(!matches.iter().any(|m| m.form == Form::Imperfect));
    }

    #[test]
    fn parses_qal_passive_participle_fs_construct() {
        // אֲהֻבַת — construct of the Qal fs passive participle אֲהוּבָה (root
        // אהב); the -â shortens to -aṯ. Generated plene (אֲהוּבַת), the
        // defective qubuts spelling matches via the shureq collapse.
        let matches = parse_word("אֲהֻבַת");
        assert!(has_match(
            &matches,
            "אהב",
            Binyan::Qal,
            Form::ParticiplePassive,
            "fs"
        ));
    }

    #[test]
    fn parses_guttural_fs_participle_aat() {
        // נִשְׁמַעַת — Niphal fs participle of שמע; the final guttural opens the
        // segolate to the ־ַעַת ending.
        assert!(has_match(
            &parse_word("נִשְׁמַעַת"),
            "שמע",
            Binyan::Niphal,
            Form::ParticipleActive,
            "fs"
        ));
        // מִתְלַקַּחַת — the same for the Hithpael of לקח.
        assert!(has_match(
            &parse_word("מִתְלַקַּחַת"),
            "לקח",
            Binyan::Hithpael,
            Form::ParticipleActive,
            "fs"
        ));
    }

    #[test]
    fn parses_pe_guttural_niphal_fs_participle() {
        // נֶהְפָּכֶת — Niphal fs participle of הפך: the pe-guttural closes with a
        // silent sheva (not a hataf), and the segolate keeps qamats on C2.
        assert!(has_match(
            &parse_word("נֶהְפָּכֶת"),
            "הפכ",
            Binyan::Niphal,
            Form::ParticipleActive,
            "fs"
        ));
    }

    #[test]
    fn parses_qamats_grade_segolate_fs_participle() {
        // עֹמָדֶת — Qal fs active participle of עמד in the qamats grade
        // (qōṭāleṯ), beside the segol-grade עֹמֶדֶת the builder makes.
        let matches = parse_word("עֹמָדֶת");
        assert!(has_match(
            &matches,
            "עמד",
            Binyan::Qal,
            Form::ParticipleActive,
            "fs"
        ));
        // מְסֻתָּרֶת — the same qamats grade in a derived stem (Pual of סתר).
        let matches = parse_word("מְסֻתָּרֶת");
        assert!(has_match(
            &matches,
            "סתר",
            Binyan::Pual,
            Form::ParticipleActive,
            "fs"
        ));
    }

    #[test]
    fn parses_ayin_guttural_piel_perfect_hiriq() {
        // נִאֲצוּ — Piel perfect 3cp of נאץ: the C2 aleph forgoes the forte, so
        // the C1 prefix keeps its short hiriq (beside the lengthened נֵאֲצוּ).
        let matches = parse_word("נִאֲצוּ");
        assert!(has_match(
            &matches,
            "נאצ",
            Binyan::Piel,
            Form::Perfect,
            "3cp"
        ));
    }

    #[test]
    fn parses_lamed_aleph_derived_perfect_qamats() {
        // קֹרָא — Pual perfect 3ms of קרא: the patah before the word-final
        // quiescent aleph lengthens to qamats (was generating קֹרַא).
        let matches = parse_word("קֹרָא");
        assert!(has_match(
            &matches,
            "קרא",
            Binyan::Pual,
            Form::Perfect,
            "3ms"
        ));
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
        assert!(has_match(
            &matches,
            "עשה",
            Binyan::Qal,
            Form::Perfect,
            "3ms"
        ));
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

    // The all-roots oracle: every triliteral's full paradigm. This is the
    // exhaustive correctness check, but building the whole index costs minutes
    // and many gigabytes, so it can't run under the default (parallel,
    // process-per-test) CI run — two concurrent builds OOM the runner. Run it
    // explicitly during the grind. CI coverage of the indexed path comes from
    // the scoped `indexed_matches_per_surface_scoped` below, which builds a tiny
    // index whose lookups are still exact for the roots it contains.
    #[test]
    #[ignore = "builds the full index; run explicitly"]
    fn indexed_matches_per_surface() {
        // The reverse index must return exactly what parse_word does. Compare the
        // sorted (root, binyan, form, pgn, suffix, prefix, vav) tuples on a varied
        // sample spanning strong, weak, prefixed, suffixed and wayyiqtol forms.
        let index = ReverseIndex::build();
        let sample = [
            "קָטַל",
            "שָׁמַר",
            "וְשָׁמַר",
            "יִשְׁמֹר",
            "וַיִּשְׁמֹר",
            "הִקְטִיל",
            "נִשְׁמַר",
            "עָשָׂה",
            "יַעֲשֶׂה",
            "וַיַּעַשׂ",
            "לֵאמֹר",
            "שְׁלָחַנִי",
            "יְבָרֶכְךָ",
            "וַיִּתְּנֵם",
            "בְּמָלְכוֹ",
            "הוֹלִידוֹ",
            "מֶלֶךְ",
            "וַיָּקָם",
            "יֵלֵכוּ",
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
            let mut b: Vec<_> = parse_word_indexed(w, &index, None)
                .iter()
                .map(key)
                .collect();
            a.sort();
            b.sort();
            assert_eq!(a, b, "index/per-surface mismatch for {w}");
        }
    }

    #[test]
    fn indexed_matches_per_surface_scoped() {
        // CI-affordable twin of `indexed_matches_per_surface`: build a tiny index
        // over only the roots the sample touches, then assert the indexed parser
        // agrees with generate-and-test *restricted to the same roots*. Filtering
        // both sides to the same root set makes the comparison exact — the index
        // holds exactly these roots, and the obj-suffix fallback gate (which keys
        // off whether the non-suffixed pass found anything) then fires identically
        // on both. So this is the per-surface equivalence theorem, scoped to the
        // roots in `index`; the full all-roots oracle above proves it everywhere.
        // Parse the roots so each is stored as base (non-final) radicals, the
        // form `build_from_roots`/`pack_radicals` require.
        let root_letters: Vec<[char; 3]> = [
            "קטל", "שמר", "עשה", "אמר", "שלח", "ברך", "נתן", "מלך", "ילד", "קום", "הלך",
        ]
        .iter()
        .map(|r| Root::parse(r).unwrap().letters)
        .collect();
        let index = ReverseIndex::build_from_roots(&root_letters);
        let scope: HashSet<[char; 3]> = root_letters.iter().copied().collect();
        let sample = [
            "קָטַל",
            "שָׁמַר",
            "וְשָׁמַר",
            "יִשְׁמֹר",
            "וַיִּשְׁמֹר",
            "הִקְטִיל",
            "נִשְׁמַר",
            "עָשָׂה",
            "יַעֲשֶׂה",
            "וַיַּעַשׂ",
            "לֵאמֹר",
            "שְׁלָחַנִי",
            "יְבָרֶכְךָ",
            "וַיִּתְּנֵם",
            "בְּמָלְכוֹ",
            "וַיָּקָם",
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
            let mut a: Vec<_> = parse_word_filtered(w, Some(&scope))
                .iter()
                .map(key)
                .collect();
            let mut b: Vec<_> = parse_word_indexed(w, &index, None)
                .iter()
                .map(key)
                .collect();
            a.sort();
            b.sort();
            assert_eq!(a, b, "scoped index/per-surface mismatch for {w}");
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
        assert!(
            m1.iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "נתנ"
                    && m.form == Form::Perfect
                    && m.pgn.label() == "1cs"
                    && m.object_suffix.map(|p| p.label()).as_deref() == Some("2ms"))
        );
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
        assert!(
            m1.iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "צוה"
                    && m.binyan == Binyan::Piel
                    && m.form == Form::ParticipleActive
                    && m.object_suffix.map(|p| p.label()).as_deref() == Some("2ms"))
        );
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
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "בוא"
                    && m.binyan == Binyan::Hiphil
                    && m.object_suffix.map(|p| p.label()).as_deref() == Some("3ms"))
        );
    }

    #[test]
    fn parses_lamed_he_doubled_apocope() {
        // וַיֵּשְׁתְּ — שתה Qal wayyiqtol ("and he drank"); the he drops and C2
        // doubles into a tsere-prefixed monosyllable (wayyēšt).
        let matches = parse_word("וַיֵּשְׁתְּ");
        assert!(matches.iter().any(
            |m| m.root.letters.iter().collect::<String>() == "שתה" && m.form == Form::Wayyiqtol
        ));
    }

    #[test]
    fn parses_pe_yod_as_pe_nun_imperfect() {
        // וַיִּצֹק — יצק Qal wayyiqtol ("and he poured"); the I-yod drops like a
        // I-nun, the prefix taking hiriq and the theme holam (yiṣōq).
        let matches = parse_word("וַיִּצֹק");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "יצק"
                    && m.binyan == Binyan::Qal
                    && m.form == Form::Imperfect)
        );
    }

    #[test]
    fn parses_proclitic_iguttural_silent_sheva() {
        // לַעְזֹר — לְ + עֲזֹר ("to help"); the proclitic closes the ayin on a
        // silent sheva, which the parser restores to the generator's hataf.
        let matches = parse_word("לַעְזֹר");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "עזר")
        );
    }

    #[test]
    fn parses_gava_strong_not_hollow() {
        // וַיִּגְוַע — גוע wayyiqtol ("and he expired"); the medial vav is a true
        // radical, so it inflects as a strong III-guttural, not a hollow verb.
        let matches = parse_word("וַיִּגְוַע");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "גוע"
                    && m.binyan == Binyan::Qal)
        );
    }

    #[test]
    fn parses_pe_aleph_patah_wayyiqtol() {
        // וַיַּאַסְפוּ — אסף Qal wayyiqtol 3mp ("and they gathered"); the I-aleph
        // patah grade (yaʾasp̄û) beside the builder's segol yeʾesp̄û.
        let matches = parse_word("וַיַּאַסְפוּ");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "אספ"
                    && m.binyan == Binyan::Qal)
        );
    }

    #[test]
    fn parses_lamed_he_hiphil_apocope() {
        // וַיַּשְׁקְ — שקה Hiphil apocopated wayyiqtol ("and he watered"); the
        // segol/î + he drops, the qof closing on a silent sheva.
        let matches = parse_word("וַיַּשְׁקְ");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "שקה"
                    && m.binyan == Binyan::Hiphil)
        );
    }

    #[test]
    fn parses_hollow_hiphil_perfect_object_suffix() {
        // וַהֲשִׁבֹתִים — שוב Hiphil perfect 1cs + 3mp ("and I will restore them");
        // the suffix attaches to the linking-ô perfect stem (hăšîḇōṯî-).
        let matches = parse_word("וַהֲשִׁבֹתִים");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "שוב"
                    && m.binyan == Binyan::Hiphil
                    && m.form == Form::Perfect
                    && m.object_suffix.map(|p| p.label()).as_deref() == Some("3mp"))
        );
    }

    #[test]
    fn parses_qal_participle_fs_a_form() {
        // יוֹלֵדָה — ילד Qal active participle fs ("woman in labour"); the -â
        // feminine (qōṭēlâ), beside the segolate yōleḏeṯ.
        let matches = parse_word("יוֹלֵדָה");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "ילד"
                    && m.form == Form::ParticipleActive)
        );
    }

    #[test]
    fn parses_hollow_niphal_perfect() {
        // נָפֹצוּ — פוץ Niphal perfect 3cp ("they were scattered"); the hollow ô
        // sits on C1 (nāp̄ôṣû), not the strong נִפְוַץ.
        let matches = parse_word("נָפֹצוּ");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "פוצ"
                    && m.binyan == Binyan::Niphal
                    && m.form == Form::Perfect)
        );
    }

    #[test]
    fn parses_geminate_niphal_perfect() {
        // נָשַׁמּוּ — שמם Niphal perfect 3cp ("they were made desolate"); the
        // doubled radical contracts to nāCaC, doubling before the vocalic -û.
        let matches = parse_word("נָשַׁמּוּ");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "שממ"
                    && m.binyan == Binyan::Niphal
                    && m.form == Form::Perfect
                    && m.pgn.label() == "3cp")
        );
    }

    #[test]
    fn parses_pausal_qotol_imperative() {
        // אֱמָר — אמר Qal imperative 2ms in pause; the theme holam lengthens to
        // qamats (ʾĕmār), beside the contextual ʾĕmōr.
        let matches = parse_word("אֱמָר");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "אמר"
                    && m.binyan == Binyan::Qal
                    && m.form == Form::Imperative)
        );
    }

    #[test]
    fn parses_iguttural_silent_sheva_imperfect_plural() {
        // וְיַחְפְּרוּ — חפר Qal imperfect 3mp; the I-guttural closes the prefix
        // syllable on a silent sheva, the pe taking a dagesh lene (yaḥpᵊrû).
        let matches = parse_word("וְיַחְפְּרוּ");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "חפר"
                    && m.form == Form::Imperfect
                    && m.pgn.label() == "3mp")
        );
    }

    #[test]
    fn parses_pausal_imperfect_plural() {
        // יִשְׁמָעוּ — שמע Qal imperfect 3mp in pause; the theme restores to
        // qamats (yišmāʕû), unlike the contextual yišmᵊʕû.
        let matches = parse_word("יִשְׁמָעוּ");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "שמע"
                    && m.form == Form::Imperfect
                    && m.pgn.label() == "3mp")
        );
    }

    #[test]
    fn parses_nun_retained_infinitive_construct() {
        // בִּנְפֹל — נפל Qal inf construct ("in falling"); the nun is kept (nᵊp̄ōl),
        // not assimilated to the dropped pōl.
        let matches = parse_word("בִּנְפֹל");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "נפל"
                    && m.form == Form::InfinitiveConstruct)
        );
    }

    #[test]
    fn parses_geminate_hiphil_perfect() {
        // הֵחֵלּוּ — חלל Hiphil perfect 3cp ("they began"); the doubled radical
        // contracts to the hēCēC shape, doubling before the vocalic -û.
        let matches = parse_word("הֵחֵלּוּ");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "חלל"
                    && m.binyan == Binyan::Hiphil
                    && m.form == Form::Perfect
                    && m.pgn.label() == "3cp")
        );
    }

    #[test]
    fn parses_hollow_hiphil_infinitive_object_suffix() {
        // לַהֲמִיתוֹ — מות Hiphil inf construct + 3ms ("to kill him"); the hāmîṯ
        // infinitive reduces to hămîṯ- before the suffix.
        let matches = parse_word("לַהֲמִיתוֹ");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "מות"
                    && m.binyan == Binyan::Hiphil
                    && m.form == Form::InfinitiveConstruct)
        );
    }

    #[test]
    fn parses_hiphil_lamed_he_perfect_patah_suffix() {
        // הִרְאַנִי — ראה Hiphil perfect 3ms + 1cs ("he showed me"); the III-He
        // Hiphil links the suffix on a patah, not the Qal's qamats.
        let matches = parse_word("הִרְאַנִי");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "ראה"
                    && m.binyan == Binyan::Hiphil
                    && m.object_suffix.map(|p| p.label()).as_deref() == Some("1cs"))
        );
        // Regression: the Qal qamats grade still parses (עָשָׂהוּ).
        let m2 = parse_word("עָשָׂהוּ");
        assert!(
            m2.iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "עשה")
        );
    }

    #[test]
    fn parses_iguttural_cohortative_silent_sheva() {
        // אֶעְבְּרָה — עבר Qal cohortative 1cs ("let me cross over"); the I-guttural
        // takes a silent sheva closing the prefix syllable, the bet a dagesh lene.
        let matches = parse_word("אֶעְבְּרָה");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "עבר"
                    && m.binyan == Binyan::Qal)
        );
    }

    #[test]
    fn parses_enemy_and_mp_participle_1cs_suffix() {
        // אוֹיְבַי — איב (now a true-triliteral qōṭēl, not hollow) participle mp +
        // 1cs ("my enemies").
        let m1 = parse_word("אוֹיְבַי");
        assert!(
            m1.iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "איב"
                    && m.form == Form::ParticipleActive
                    && m.object_suffix.map(|p| p.label()).as_deref() == Some("1cs"))
        );
        // The -ay form cascades to any mp participle (שֹׁמְרַי "those keeping me").
        let m2 = parse_word("שֹׁמְרַי");
        assert!(
            m2.iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "שמר"
                    && m.object_suffix.map(|p| p.label()).as_deref() == Some("1cs"))
        );
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
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "ענה"
                    && m.object_suffix.map(|p| p.label()).as_deref() == Some("1cs"))
        );
    }

    #[test]
    fn parses_pe_guttural_wayyiqtol_object_suffix() {
        // וַיַּעַזְבֵנִי — עזב Qal wayyiqtol + 1cs ("and he forsook me"); the C1
        // ayin's hataf fills to a full patah as the suffixed stem closes it.
        let matches = parse_word("וַיַּעַזְבֵנִי");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "עזב"
                    && m.object_suffix.map(|p| p.label()).as_deref() == Some("1cs"))
        );
    }

    #[test]
    fn parses_pe_nun_niphal_participle() {
        // נִבְּאִים — נבא Niphal participle mp ("prophesying"); the radical nun
        // assimilates into the bet (niḇbᵊʔîm), as in the perfect.
        let matches = parse_word("נִבְּאִים");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "נבא"
                    && m.binyan == Binyan::Niphal
                    && m.form == Form::ParticipleActive)
        );
    }

    #[test]
    fn parses_lamed_aleph_plural_object_suffix() {
        // וַיִּמְצָאֻהוּ — מצא Qal imperfect 3mp + 3ms ("and they found him"); the
        // quiescent aleph carries the subject û defectively as a qubuts.
        let matches = parse_word("וַיִּמְצָאֻהוּ");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "מצא"
                    && m.object_suffix.map(|p| p.label()).as_deref() == Some("3ms"))
        );
    }

    #[test]
    fn parses_pe_vav_niphal_imperfect() {
        // וַיִּוָּעַץ — יעץ Niphal wayyiqtol ("and he took counsel"); the I-vav
        // root doubles the vav (yiwwāʕaṣ), not the yod the builder defaults to.
        let matches = parse_word("וַיִּוָּעַץ");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "יעצ"
                    && m.binyan == Binyan::Niphal)
        );
    }

    #[test]
    fn parses_nun_retained_imperative() {
        // נְטֵה — נטה Qal imperative ("stretch out"); the nun is kept, not
        // assimilated.
        let matches = parse_word("נְטֵה");
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "נטה"
                    && m.binyan == Binyan::Qal
                    && m.form == Form::Imperative)
        );
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
        assert!(matches.iter().any(
            |m| m.root.letters.iter().collect::<String>() == "אבד" && m.form == Form::Imperfect
        ));
        // תִּשְׁמָעוּן — שמע, the qamats (a-theme) twin.
        let m2 = parse_word("תִּשְׁמָעוּן");
        assert!(m2.iter().any(
            |m| m.root.letters.iter().collect::<String>() == "שמע" && m.form == Form::Imperfect
        ));
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
        assert!(
            matches
                .iter()
                .any(|m| m.root.letters.iter().collect::<String>() == "עשה"
                    && m.binyan == Binyan::Qal
                    && m.form == Form::Imperfect
                    && m.pgn.label() == "1cp")
        );
    }
}

#[cfg(test)]
mod peel_coverage {
    use super::*;

    /// The peeler is the inverse of the object-suffix builders: every suffixed
    /// surface the generator produces should peel back to its object Pgn. Assert
    /// that holds for ≥99% of generated suffixed forms across a weak-root sample
    /// (the residue is the ambiguous bare -î 1cs we deliberately don't peel).
    #[test]
    fn peeler_inverts_generated_suffixes() {
        let roots = [
            "שמר", "קטל", "ברך", "בוא", "קום", "שית", "עשה", "בנה", "ידע", "נתן", "שלח", "אכל",
        ];
        let (mut total, mut ok) = (0usize, 0usize);
        for r in roots {
            let root = Root::parse(r).unwrap();
            for vf in &generate_paradigm(&root).forms {
                let Some(obj) = vf.object_suffix else {
                    continue;
                };
                total += 1;
                let seq = hebrew::parse_pointed(&vf.text);
                if peel_object_suffix(&seq).into_iter().any(|(_, p)| p == obj) {
                    ok += 1;
                }
            }
        }
        let pct = 100.0 * ok as f64 / total as f64;
        assert!(
            pct >= 99.0,
            "peeler coverage {ok}/{total} = {pct:.1}% < 99%"
        );
    }

    /// ADR-0004 Option C equivalence oracle, scoped to the rewrite itself: the
    /// connecting-stem `obj_index` must still recover every suffixed surface the
    /// old full-surface index held. We drive [`object_suffix_fallback_indexed`]
    /// directly (the parser gates it behind a failed parse, which would mask the
    /// index under main-`entries` hits and proclitic alternatives — a pre-existing
    /// parity concern, not this index's job) and assert each generated suffixed
    /// surface peels back to its host. Coverage in this direction proves nothing
    /// became unreachable; the surface-hash gate makes the reverse (no spurious
    /// match) hold by construction. Spans strong, weak and twin hosts.
    #[test]
    fn obj_index_recovers_every_generated_suffix() {
        let roots = [
            "שמר", "קטל", "ברך", "בוא", "קום", "עשה", "בנה", "נתן", "שלח", "אכל", "ראה", "ישב",
            "סבב", "מצא", "לקח",
        ];
        // The probe only touches these roots, and a root absent from the index
        // simply never matches — so a scoped index over exactly these roots is
        // equivalent here and avoids the minutes/many-GB full all-roots build
        // (which OOMs the parallel CI run).
        let root_letters: Vec<[char; 3]> = roots
            .iter()
            .map(|r| Root::parse(r).unwrap().letters)
            .collect();
        let index = ReverseIndex::build_from_roots(&root_letters);
        let (mut total, mut missed) = (0usize, 0usize);
        for r in roots {
            let root = Root::parse(r).unwrap();
            for vf in generate_paradigm(&root)
                .forms
                .iter()
                .filter(|vf| vf.object_suffix.is_none())
            {
                for (obj, surface) in
                    host_object_suffixes(vf.binyan, vf.form, vf.pgn, &vf.text, &root)
                {
                    total += 1;
                    let seq = hebrew::parse_pointed(&surface);
                    let mut matches = Vec::new();
                    let mut seen = HashSet::new();
                    object_suffix_fallback_indexed(&seq, &index, None, &mut matches, &mut seen);
                    let found = matches.iter().any(|m| {
                        m.root.letters == root.letters
                            && m.binyan == vf.binyan
                            && m.form == vf.form
                            && m.pgn == vf.pgn
                            && m.object_suffix == Some(obj)
                    });
                    if !found {
                        missed += 1;
                        eprintln!(
                            "obj_index miss: {surface} (root {r}, {:?} {:?} {} + {})",
                            vf.binyan,
                            vf.form,
                            vf.pgn.label(),
                            obj.label()
                        );
                    }
                }
            }
        }
        assert_eq!(
            missed, 0,
            "{missed}/{total} generated suffixed surfaces unrecoverable"
        );
    }

    /// Report the obj-index shrink (ADR-0004 Option C). The old design stored one
    /// `(key, entry)` per suffixed surface; the new one stores one `ObjStemEntry`
    /// per connecting stem, listing those surfaces as `endings`. The sum of
    /// `endings` lengths is exactly the old entry count, so their ratio is the
    /// entry-count shrink. Run with `--nocapture` to see the numbers.
    #[test]
    #[ignore = "diagnostic: builds the full index; run explicitly with --nocapture"]
    fn obj_index_shrink_stats() {
        let index = ReverseIndex::build();
        let new_entries = index.obj_index.len();
        // One ending == one suffixed surface == one entry in the old full-surface
        // design, so the arena length is exactly the old entry count.
        let old_entries = index.obj_endings.len();
        let new_bytes = index.obj_index.capacity() * std::mem::size_of::<(u64, ObjStemEntry)>()
            + index.obj_endings.capacity() * std::mem::size_of::<u64>();
        let old_bytes = old_entries * std::mem::size_of::<(u128, IndexEntry)>();
        eprintln!(
            "obj_index: {new_entries} stem entries + {old_entries} endings vs {old_entries} \
             surface entries ({:.2}× fewer entries); ~{} MB vs ~{} MB ({:.2}× smaller)",
            old_entries as f64 / new_entries as f64,
            new_bytes / 1_000_000,
            old_bytes / 1_000_000,
            old_bytes as f64 / new_bytes as f64,
        );
    }
}
