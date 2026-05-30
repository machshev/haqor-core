//! Reverse noun morphology: parse a fully-pointed word back into the noun
//! analyses that could have produced it.
//!
//! A noun's pattern (mishqal) can't be derived from its root the way a verb's
//! paradigm can (see [`crate::morphology::parse`]), so there is no
//! candidate-root enumeration to drive generate-and-test. Instead the engine is
//! driven by a *stem inventory* — the pointed noun headwords supplied by the
//! caller (in practice, harvested from the lexicon). For each stem we run the
//! forward generator [`inflect_noun`] and keep every inflected form whose
//! rendered text exactly equals the surface (after peeling proclitics), exactly
//! as the verb parser keeps exact matches against generated forms.
//!
//! Coverage is therefore bounded by two things: which stems the inventory
//! contains, and which stem classes [`inflect_noun`] models (segolate plus the
//! masculine/feminine endings). Irregular and broken plurals are not modelled,
//! so they simply fail to match rather than producing a wrong analysis.

use std::collections::{HashMap, HashSet};

use super::hebrew::{self, letter};
use super::noun::{NounStem, NounStemKind, inflect_noun};

/// Single-consonant proclitics peeled off the front of a word: conjunction ו,
/// prepositions ב/כ/ל/מ, article/interrogative ה, relative ש. The same set the
/// verb parser strips.
const PROCLITICS: [char; 7] = [
    letter::VAV,
    letter::BET,
    letter::KAF,
    letter::LAMED,
    letter::MEM,
    letter::HE,
    letter::SHIN,
];

/// One candidate analysis of a surface word as an inflected noun.
#[derive(Debug, Clone)]
pub struct NounMatch {
    /// The lemma (singular absolute headword) this analysis inflects, rendered
    /// to Hebrew.
    pub stem: String,
    /// The lemma's stem class.
    pub kind: NounStemKind,
    /// Which inflected slot matched, e.g. "Plural Construct" or "Sg + 3ms".
    pub label: String,
    /// The proclitic prefix consumed before the stem, rendered to Hebrew (empty
    /// if the whole word was analysed as the noun).
    pub prefix: String,
}

/// An inventory of noun stems pre-compiled for reverse parsing: every inflected
/// surface form is indexed to the analyses that generate it, so parsing a word
/// is a hash lookup rather than a scan over the whole inventory.
pub struct NounInventory {
    /// Rendered lemma + class, indexed by stem id.
    stems: Vec<(String, NounStemKind)>,
    /// Generated surface form → (stem id, slot label).
    forms: HashMap<String, Vec<(usize, String)>>,
}

impl NounInventory {
    /// Compile a set of stems into a reverse-parsing inventory.
    pub fn build(stems: &[NounStem]) -> Self {
        let mut forms: HashMap<String, Vec<(usize, String)>> = HashMap::new();
        let mut rendered = Vec::with_capacity(stems.len());
        for (id, stem) in stems.iter().enumerate() {
            rendered.push((hebrew::render(&stem.absolute_singular), stem.kind));
            for inflection in inflect_noun(stem) {
                forms
                    .entry(inflection.text)
                    .or_default()
                    .push((id, inflection.label));
            }
        }
        NounInventory {
            stems: rendered,
            forms,
        }
    }

    /// Number of stems in the inventory.
    pub fn len(&self) -> usize {
        self.stems.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stems.is_empty()
    }

    /// Parse a fully-pointed word into every noun analysis the inventory can
    /// produce, trying the word bare and with 0/1/2 proclitics peeled.
    pub fn parse(&self, word: &str) -> Vec<NounMatch> {
        let seq = hebrew::parse_pointed(word);
        let mut matches = Vec::new();
        // Dedup by the analysis content, not the stem id: distinct lexicon
        // lemmas (e.g. the common noun מֶלֶךְ and the name "Molech") can render
        // to the same stem text and inflect identically, which would otherwise
        // emit duplicate rows.
        let mut seen: HashSet<(String, NounStemKind, String, String)> = HashSet::new();

        let max_strip = 2usize.min(seq.len().saturating_sub(2));
        for strip in 0..=max_strip {
            if seq[..strip].iter().any(|c| !PROCLITICS.contains(&c.letter)) {
                continue;
            }
            let target = hebrew::render(&seq[strip..]);
            let prefix = hebrew::render(&seq[..strip]);
            let Some(entries) = self.forms.get(&target) else {
                continue;
            };
            for (id, label) in entries {
                let (stem, kind) = &self.stems[*id];
                let key = (stem.clone(), *kind, label.clone(), prefix.clone());
                if seen.insert(key) {
                    matches.push(NounMatch {
                        stem: stem.clone(),
                        kind: *kind,
                        label: label.clone(),
                        prefix: prefix.clone(),
                    });
                }
            }
        }

        matches.sort_by(|a, b| {
            a.stem
                .cmp(&b.stem)
                .then_with(|| a.label.cmp(&b.label))
                .then_with(|| a.prefix.cmp(&b.prefix))
        });
        matches
    }
}

/// Convenience wrapper: build an inventory from `stems` and parse one word.
/// Prefer [`NounInventory::build`] when parsing many words against the same
/// inventory, so the paradigms are compiled once.
pub fn parse_noun_word(word: &str, stems: &[NounStem]) -> Vec<NounMatch> {
    NounInventory::build(stems).parse(word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_segolate_plural() {
        // מְלָכִים — plural absolute of מֶלֶךְ.
        let stems = vec![NounStem::segolate("מֶלֶךְ")];
        let matches = parse_noun_word("מְלָכִים", &stems);
        assert!(
            matches
                .iter()
                .any(|m| m.label == "Plural Absolute" && m.prefix.is_empty()),
            "expected plural-absolute analysis, got {matches:?}"
        );
    }

    #[test]
    fn parses_with_proclitic() {
        // וּמְלָכִים — conjunction ו + plural of מֶלֶךְ. The prefix is reported
        // separately. (parse_pointed ignores the shureq dagesh on the vav.)
        let stems = vec![NounStem::segolate("מֶלֶךְ")];
        let matches = parse_noun_word("וּמְלָכִים", &stems);
        assert!(
            matches
                .iter()
                .any(|m| m.label == "Plural Absolute" && !m.prefix.is_empty()),
            "expected prefixed plural analysis, got {matches:?}"
        );
    }

    #[test]
    fn parses_masculine_lemma() {
        // The bare lemma matches its own singular-absolute slot.
        let stems = vec![NounStem::masculine("דָּבָר")];
        let matches = parse_noun_word("דָּבָר", &stems);
        assert!(matches.iter().any(|m| m.label == "Singular Absolute"));
    }
}
