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
    /// True when the matched lemma is an adjective headword (agreement
    /// inflection), so callers can report nouns and adjectives separately.
    pub is_adjective: bool,
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
    /// Rendered lemma + class + adjective flag, indexed by stem id.
    stems: Vec<(String, NounStemKind, bool)>,
    /// Generated surface form → (stem id, slot label).
    forms: HashMap<String, Vec<(usize, String)>>,
}

/// Normalise a rendered form for indexing/lookup: collapse the qamats-qatan
/// point (U+05C7) to a plain qamats (U+05B8). The generator emits qamats-qatan
/// under o-class segolate bases (קׇדְשׁוֹ), but the WLC text writes a plain
/// qamats there (קָדְשׁוֹ); collapsing both ends keeps the match exact.
fn norm_key(s: &str) -> String {
    s.replace('\u{05C7}', "\u{05B8}")
}

/// Pausal alternant of a dual -ayim ending: the patah of the dual suffix
/// lengthens to qamats in pause, so šāmayim (שָׁמַיִם) and mayim (מַיִם) also
/// appear as šāmāyim (שָׁמָיִם) and māyim (מָיִם). Returns the pausal spelling
/// when `text` ends in -ayim (consonant+patah, yod+hiriq, final mem), else
/// `None`. Tightly scoped to that ending so it never perturbs other forms.
fn pausal_dual_variant(text: &str) -> Option<String> {
    use super::hebrew::Vowel;
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 3 {
        return None;
    }
    if seq[n - 1].letter == letter::MEM
        && seq[n - 1].vowel.is_none()
        && seq[n - 2].letter == letter::YOD
        && seq[n - 2].vowel == Some(Vowel::Hiriq)
        && seq[n - 3].vowel == Some(Vowel::Patah)
    {
        seq[n - 3].vowel = Some(Vowel::Qamats);
        return Some(hebrew::render(&seq));
    }
    None
}

impl NounInventory {
    /// Compile a set of stems into a reverse-parsing inventory.
    pub fn build(stems: &[NounStem]) -> Self {
        let mut forms: HashMap<String, Vec<(usize, String)>> = HashMap::new();
        let mut rendered = Vec::with_capacity(stems.len());
        for (id, stem) in stems.iter().enumerate() {
            rendered.push((
                hebrew::render(&stem.absolute_singular),
                stem.kind,
                stem.is_adjective,
            ));
            for inflection in inflect_noun(stem) {
                if let Some(pausal) = pausal_dual_variant(&inflection.text) {
                    forms
                        .entry(norm_key(&pausal))
                        .or_default()
                        .push((id, inflection.label.clone()));
                }
                forms
                    .entry(norm_key(&inflection.text))
                    .or_default()
                    .push((id, inflection.label));
            }
        }
        NounInventory {
            stems: rendered,
            forms,
        }
    }

    /// Register the curated irregular-noun inventory: lemmas whose inflected
    /// forms are suppletive or otherwise unmodelable by [`inflect_noun`]
    /// (בֵּן, אִישׁ→אֲנָשִׁים, …). Each gold-attested proclitic-free surface is
    /// indexed to its lemma, so matching stays exact; proclitics are peeled by
    /// [`Self::parse`]. Forms are run through the same parse→render round-trip
    /// as generated forms so their mark order matches the lookup key.
    pub fn add_irregulars(&mut self) {
        for noun in super::irregular_noun::IRREGULAR_NOUNS {
            let id = self.stems.len();
            self.stems.push((
                hebrew::render(&hebrew::parse_pointed(noun.lemma)),
                NounStemKind::Masculine,
                false,
            ));
            let label = format!("Irregular ({})", noun.gloss);
            for form in noun.forms {
                let key = norm_key(&hebrew::render(&hebrew::parse_pointed(form)));
                self.forms.entry(key).or_default().push((id, label.clone()));
            }
        }
    }

    /// Register the gold-harvested common-noun inventory: attested surface
    /// forms the generator can't produce (broken plurals, reduced construct
    /// stems — דִּבְרֵי, יְמֵי, צְבָאוֹת), each mapped to its lexicon lemma. Like
    /// [`Self::add_irregulars`], forms are run through the parse→render
    /// round-trip so their mark order matches the lookup key.
    pub fn add_gold_nouns(&mut self) {
        for &(form, lemma, gloss) in super::gold_noun::GOLD_NOUNS {
            let id = self.stems.len();
            self.stems.push((
                hebrew::render(&hebrew::parse_pointed(lemma)),
                NounStemKind::Masculine,
                false,
            ));
            let label = format!("Noun ({gloss})");
            let key = norm_key(&hebrew::render(&hebrew::parse_pointed(form)));
            self.forms.entry(key).or_default().push((id, label));
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
        let mut seen: HashSet<(String, NounStemKind, bool, String, String)> = HashSet::new();

        let max_strip = 2usize.min(seq.len().saturating_sub(2));
        for strip in 0..=max_strip {
            if seq[..strip].iter().any(|c| !PROCLITICS.contains(&c.letter)) {
                continue;
            }
            let prefix = hebrew::render(&seq[..strip]);
            // Two candidate stem renderings for the peeled remainder: as written,
            // and (when a proclitic was peeled) with the first consonant's dagesh
            // forte removed. The definite article — written (הַ) or assimilated
            // into the preposition (בַּ = בְּ+הַ) — doubles the following consonant,
            // so the bare lexical form carries no such dagesh (הַיּוֹם→יוֹם).
            let mut rest = seq[strip..].to_vec();
            let mut targets = vec![norm_key(&hebrew::render(&rest))];
            if strip > 0 && rest.first().is_some_and(|c| c.dagesh) {
                rest[0].dagesh = false;
                targets.push(norm_key(&hebrew::render(&rest)));
            }
            for target in targets {
                let Some(entries) = self.forms.get(&target) else {
                    continue;
                };
                for (id, label) in entries {
                    let (stem, kind, is_adjective) = &self.stems[*id];
                    let key = (
                        stem.clone(),
                        *kind,
                        *is_adjective,
                        label.clone(),
                        prefix.clone(),
                    );
                    if seen.insert(key) {
                        matches.push(NounMatch {
                            stem: stem.clone(),
                            kind: *kind,
                            is_adjective: *is_adjective,
                            label: label.clone(),
                            prefix: prefix.clone(),
                        });
                    }
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
    fn parses_adjective_feminine_singular() {
        // רָחָב "broad" (adjective) → feminine singular רְחָבָה, with propretonic
        // reduction of the first qamats. וּרְחָבָה in Exod 3:8.
        let stems = vec![NounStem::masculine("רָחָב").with_adjective(true)];
        let m = parse_noun_word("רְחָבָה", &stems);
        assert!(
            m.iter()
                .any(|m| m.label == "Feminine Singular" && m.prefix.is_empty()),
            "expected feminine-singular adjective, got {m:?}"
        );
        // …and with the conjunction peeled.
        let m = parse_noun_word("וּרְחָבָה", &stems);
        assert!(
            m.iter()
                .any(|m| m.label == "Feminine Singular" && !m.prefix.is_empty()),
            "expected prefixed feminine-singular adjective, got {m:?}"
        );
    }

    #[test]
    fn parses_adjective_feminine_plural() {
        // גָּדוֹל "great" → feminine plural גְּדוֹלוֹת (plene -ôt).
        let stems = vec![NounStem::masculine("גָּדוֹל").with_adjective(true)];
        let m = parse_noun_word("גְּדוֹלוֹת", &stems);
        assert!(
            m.iter().any(|m| m.label == "Feminine Plural"),
            "expected feminine-plural adjective, got {m:?}"
        );
    }

    #[test]
    fn parses_guttural_adjective_feminine() {
        // חָדָשׁ "new" → חֲדָשָׁה: the guttural C1 takes hataf-patah, not sheva.
        let stems = vec![NounStem::masculine("חָדָשׁ").with_adjective(true)];
        let m = parse_noun_word("חֲדָשָׁה", &stems);
        assert!(
            m.iter().any(|m| m.label == "Feminine Singular"),
            "expected guttural feminine-singular adjective, got {m:?}"
        );
    }

    #[test]
    fn non_adjective_masculine_has_no_feminine() {
        // A plain masculine noun must NOT sprout a feminine -â (it would invent
        // spurious analyses). דָּבָר → no דְּבָרָה.
        let stems = vec![NounStem::masculine("דָּבָר")];
        let m = parse_noun_word("דְּבָרָה", &stems);
        assert!(
            !m.iter().any(|m| m.label.starts_with("Feminine")),
            "plain masculine noun should not feminize, got {m:?}"
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

    #[test]
    fn parses_with_definite_article() {
        // הַיּוֹם = article הַ + יוֹם, doubling the yod with a dagesh forte the
        // bare lemma lacks; the forte is stripped so the stem still matches.
        let stems = vec![NounStem::masculine("יוֹם")];
        let matches = parse_noun_word("הַיּוֹם", &stems);
        assert!(
            matches
                .iter()
                .any(|m| m.label == "Singular Absolute" && !m.prefix.is_empty()),
            "expected article-prefixed singular, got {matches:?}"
        );
    }

    #[test]
    fn parses_preposition_plus_article() {
        // בַּיּוֹם = בְּ + הַ + יוֹם; the article assimilates into the preposition,
        // leaving a forte on the yod. Peeling ב and stripping the forte recovers
        // the lemma.
        let stems = vec![NounStem::masculine("יוֹם")];
        let matches = parse_noun_word("בַּיּוֹם", &stems);
        assert!(
            matches
                .iter()
                .any(|m| m.label == "Singular Absolute" && !m.prefix.is_empty()),
            "expected preposition+article singular, got {matches:?}"
        );
    }
}
