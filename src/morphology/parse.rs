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
//! consonant) is recovered by [`strip_wayyiqtol`] and matched against the
//! imperfect/jussive. Strong-verb and III-He wayyiqtol match cleanly (the
//! latter via the apocopated jussive, e.g. וַיִּבֶן). Hollow and I-Aleph
//! wayyiqtol still miss: their surface vowel comes from stress retraction
//! (nesiga, e.g. וַיָּקָם / וַיֹּאמֶר) and is partly lexical, so it is not
//! emitted by the forward generator.
//!
//! Limitations: only verbs are parsed. A noun's pattern (mishqal) cannot be
//! derived from its root, so the noun generator takes a stem rather than a
//! root and can't be driven from candidate roots. Plene/defective spelling
//! differences against the Masoretic text and ketiv/qere variation are not
//! modelled, so a form spelled differently from what the generator emits will
//! not match.

use std::collections::HashMap;

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
}

/// Parse a fully-pointed word into every verb analysis that can produce it.
pub fn parse_word(word: &str) -> Vec<VerbMatch> {
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
        let target = hebrew::render(remainder);
        let prefix = hebrew::render(&seq[..strip]);

        for letters in candidate_roots(remainder) {
            let paradigm = memo
                .entry(letters)
                .or_insert_with(|| generate_paradigm(&Root::from_letters(letters)));
            for vf in &paradigm.forms {
                // Wayyiqtol is handled by the dedicated pass below, with the
                // consecutive vav recognised as such rather than as a plain
                // proclitic; skip it here to avoid duplicate analyses.
                if vf.form == Form::Wayyiqtol || vf.text != target {
                    continue;
                }
                if seen.insert((letters, vf.binyan, vf.form, vf.pgn, strip, false)) {
                    matches.push(VerbMatch {
                        root: Root::from_letters(letters),
                        binyan: vf.binyan,
                        form: vf.form,
                        pgn: vf.pgn,
                        attested: vf.attested,
                        prefix: prefix.clone(),
                        vav_consecutive: false,
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
        for letters in candidate_roots(&seq[1..]) {
            let paradigm = memo
                .entry(letters)
                .or_insert_with(|| generate_paradigm(&Root::from_letters(letters)));
            for vf in &paradigm.forms {
                if vf.form != Form::Wayyiqtol || vf.text != target {
                    continue;
                }
                if seen.insert((letters, vf.binyan, vf.form, vf.pgn, 0, true)) {
                    matches.push(VerbMatch {
                        root: Root::from_letters(letters),
                        binyan: vf.binyan,
                        form: vf.form,
                        pgn: vf.pgn,
                        attested: vf.attested,
                        prefix: String::new(),
                        vav_consecutive: true,
                    });
                }
            }
        }
    }

    // Attested (fully-modelled) analyses first, then a stable ordering.
    matches.sort_by(|a, b| {
        b.attested
            .cmp(&a.attested)
            .then_with(|| a.root.letters.cmp(&b.root.letters))
            .then_with(|| (a.binyan as usize).cmp(&(b.binyan as usize)))
            .then_with(|| (a.form as usize).cmp(&(b.form as usize)))
            .then_with(|| a.pgn.label().cmp(&b.pgn.label()))
    });
    matches
}

/// Enumerate every triliteral root that could underlie the surface
/// consonants. The alphabet is the distinct surface consonants together with
/// the weak letters; each of the three radical slots ranges over it.
fn candidate_roots(seq: &[Cons]) -> Vec<[char; 3]> {
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

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for &a in &alphabet {
        for &b in &alphabet {
            for &c in &alphabet {
                let r = [a, b, c];
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
}
