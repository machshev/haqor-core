//! Curated learner glosses for the most frequent OT surface forms.
//!
//! The automatic BDB bridge ([`crate::bible::Bible::hebrew_word_info`]) keys on
//! citation form and often mis-glosses closed-class particles, pronominal-
//! suffixed forms and construct chains (and cannot see homograph context). This
//! table pins a concise learner gloss — and an optional composition note — for
//! the highest-frequency such words, so the tutor shows the right meaning
//! regardless of how the parser resolved the surface.
//!
//! This data lives here, in the core, rather than in the Flutter layer: the app
//! is presentation only. Lookups normalise both the stored surface and the
//! table's keys through [`vocab_key`], so combining-mark order and dagesh
//! variants (בֶּן/בֶן) collapse to one key.

use std::collections::HashMap;
use std::sync::OnceLock;

/// A curated gloss for a surface: the learner-facing meaning and an optional
/// composition/teaching note ("לְ (to) + ־וֹ (him)").
#[derive(Debug, Clone, Copy)]
pub struct CuratedGloss {
    pub gloss: &'static str,
    pub note: Option<&'static str>,
}

/// Reduce a pointed surface form to a stable lookup key: dagesh, meteg and
/// cantillation dropped, the remaining marks sorted within each letter's
/// cluster. Mirrors the app's former `vocabKey` so the same literals match.
pub fn vocab_key(surface: &str) -> String {
    let mut out = String::new();
    let mut marks: Vec<u32> = Vec::new();
    for c in surface.chars() {
        let u = c as u32;
        let is_mark = (0x0591..=0x05C7).contains(&u);
        if !is_mark {
            flush_marks(&mut out, &mut marks);
            out.push(c);
        } else if u != 0x05BC && u != 0x05BD && !(0x0591..=0x05AF).contains(&u) {
            // Keep vowel points and shin/sin dots; drop dagesh, meteg, accents.
            marks.push(u);
        }
    }
    flush_marks(&mut out, &mut marks);
    out
}

fn flush_marks(out: &mut String, marks: &mut Vec<u32>) {
    marks.sort_unstable();
    for &m in marks.iter() {
        if let Some(ch) = char::from_u32(m) {
            out.push(ch);
        }
    }
    marks.clear();
}

/// The curated gloss for `surface`, if one is registered. Normalises the input
/// through [`vocab_key`] before lookup.
pub fn curated_gloss(surface: &str) -> Option<CuratedGloss> {
    index().get(&vocab_key(surface)).copied()
}

/// Whether `surface` is one of the curated proper names — names whose BDB
/// entry is missing or oddly glossed, so the automatic `n.pr` detection
/// ([`crate::bible::is_name_gloss`]) can't see them. Complements that check
/// wherever the tutor classifies names.
pub fn curated_name(surface: &str) -> bool {
    static NAMES: OnceLock<std::collections::HashSet<String>> = OnceLock::new();
    NAMES
        .get_or_init(|| {
            crate::lexicon_overlay::word_glosses()
                .filter(|entry| entry.is_name)
                .map(|entry| vocab_key(entry.surface))
                .collect()
        })
        .contains(&vocab_key(surface))
}

fn index() -> &'static HashMap<String, CuratedGloss> {
    static INDEX: OnceLock<HashMap<String, CuratedGloss>> = OnceLock::new();
    INDEX.get_or_init(|| {
        crate::lexicon_overlay::word_glosses()
            .filter(|entry| !entry.gloss.is_empty())
            .map(|entry| {
                (
                    vocab_key(entry.surface),
                    CuratedGloss {
                        gloss: entry.gloss,
                        note: entry.note,
                    },
                )
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocab_key_drops_dagesh_and_sorts_marks() {
        // בֶּן (bet + dagesh + segol) and בֶן (bet + segol) collapse.
        assert_eq!(vocab_key("בֶּן"), vocab_key("בֶן"));
        assert_eq!(vocab_key("כָּל"), vocab_key("כָל"));
        // Cantillation and meteg are dropped.
        assert_eq!(vocab_key("דָּבָר"), vocab_key("דָּבָר\u{0591}"));
        // Combining-mark order is normalised: mem + segol + dagesh (database NFC
        // order, vowel before dagesh) matches mem + dagesh + segol (traditional).
        let nfc: String = ['\u{05DE}', '\u{05B6}', '\u{05BC}'].iter().collect();
        let traditional: String = ['\u{05DE}', '\u{05BC}', '\u{05B6}'].iter().collect();
        assert_eq!(vocab_key(&nfc), vocab_key(&traditional));
        // Meaningful distinctions are kept: different vowels, and the shin/sin dot.
        assert_ne!(vocab_key("עַם"), vocab_key("עִם"));
        assert_ne!(vocab_key("שׁ"), vocab_key("שׂ"));
    }

    #[test]
    fn curated_gloss_matches_dagesh_variants() {
        // "the word" is registered under הַדָּבָר; a dagesh-stripped spelling
        // still resolves.
        assert!(curated_gloss("הַדָּבָר").is_some());
        assert_eq!(curated_gloss("אֶת").unwrap().gloss, "←");
        assert_eq!(curated_gloss("אֵת").unwrap().gloss, "←");
        assert_eq!(curated_gloss("וְאֶת").unwrap().gloss, "and ←");
        assert_eq!(curated_gloss("וְאֵת").unwrap().gloss, "and ←");
        assert!(curated_gloss("כִּי").unwrap().note.is_none());
        // An ordinary content word is not curated.
        assert!(curated_gloss("בָּרָא").is_none());
    }
}
