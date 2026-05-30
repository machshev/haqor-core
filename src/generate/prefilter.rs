//! Lexical pre-filter: recognise tokens that are *not* verbs so they can be
//! kept out of the verb parser entirely.
//!
//! The reverse parser is verb-only and works by generate-and-test, so it
//! happily emits spurious "imperative/infinitive of a fabricated root" analyses
//! for closed-class words (pronouns, prepositions, particles) and proper nouns.
//! Those readings dominate the ambiguity review even though the token is never
//! a verb. We can't derive part-of-speech from the parser, so we recognise these
//! forms up front from two references:
//!
//! - a curated list of **closed-class function words** (their headwords are
//!   stored mangled in Strong's, so a hand list is more reliable there);
//! - **proper nouns** from the lexicon (`english.pos` `n-pr*` / `np`).
//!
//! Matching is by **exact pointed form** (cantillation-stripped via
//! [`normalize_surface`]): we exclude the precise vocalised non-verb form, so a
//! genuine verb that merely shares the consonants (e.g. שָׁאַל "he asked" vs the
//! name שָׁאוּל) is left untouched. Leading proclitics are peeled before matching
//! so וְלֹא, בְּמִצְרַיִם resolve to לֹא, מִצְרַיִם.

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;

use super::hebrew_db::normalize_surface;

/// Single-consonant proclitics (conjunction ו, prepositions ב/כ/ל/מ, article ה,
/// relative ש) — the same set the verb parser peels.
const PROCLITICS: [char; 7] = [
    '\u{05D5}', '\u{05D1}', '\u{05DB}', '\u{05DC}', '\u{05DE}', '\u{05D4}', '\u{05E9}',
];

/// Curated closed-class function words (exact pointed forms). Pronouns,
/// demonstratives, interrogatives/relative, independent prepositions, and the
/// common particles/negatives/adverbs. Deliberately omits forms that are also
/// common verbs (e.g. עוֹד) so the filter never costs verb recall.
const FUNCTION_WORDS: &[&str] = &[
    // personal pronouns
    "אֲנִי", "אָנֹכִי", "אַתָּה", "אַתְּ", "אַתֶּם", "אַתֶּן", "הוּא", "הִיא", "הֵם", "הֵמָּה", "הֵן",
    "הֵנָּה", "אֲנַחְנוּ", "נַחְנוּ",
    // demonstratives
    "זֶה", "זֹאת", "זוֹ", "אֵלֶּה",
    // interrogatives / relative
    "מִי", "מָה", "מַה", "מֶה", "אֲשֶׁר",
    // independent prepositions
    "אֶל", "עַל", "עַד", "אַחַר", "אַחֲרֵי", "בֵּין", "תַּחַת", "נֶגֶד", "לִפְנֵי", "עִם", "מִן",
    "אֵת", "אֶת", "יַעַן", "לְמַעַן",
    // particles / negatives / adverbs
    "לֹא", "אַל", "אִם", "כִּי", "גַּם", "אַף", "רַק", "אַךְ", "הִנֵּה", "נָא", "פֶּן", "שָׁם",
    "פֹּה", "כֹּה", "כֵּן", "עַתָּה", "אָז", "מְאֹד", "אֵין", "יֵשׁ", "אוֹ", "כֹּל", "כָּל",
];

/// Recognises non-verb tokens by exact pointed form.
pub struct Prefilter {
    function: HashSet<String>,
    proper: HashSet<String>,
}

impl Prefilter {
    /// Build from the curated function words plus the lexicon's proper nouns.
    pub fn load(lexicon_db: &Path) -> Result<Self> {
        let function: HashSet<String> = FUNCTION_WORDS
            .iter()
            .map(|s| normalize_surface(s))
            .filter(|s| !s.is_empty())
            .collect();

        let db = Connection::open(lexicon_db)
            .with_context(|| format!("opening {}", lexicon_db.display()))?;
        let mut stmt =
            db.prepare("SELECT word FROM english WHERE pos LIKE 'n-pr%' OR pos = 'np'")?;
        let mut proper = HashSet::new();
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        for w in rows {
            let n = normalize_surface(&w?);
            if !n.is_empty() {
                proper.insert(n);
            }
        }
        Ok(Self { function, proper })
    }

    /// Classify a cantillation-normalised surface. Returns `"function"` or
    /// `"proper"` if the form — or a de-prefixed remainder — is a known
    /// non-verb, else `None` (parse it as a verb).
    pub fn classify(&self, surface: &str) -> Option<&'static str> {
        for form in deprefixed_forms(surface) {
            if self.function.contains(&form) {
                return Some("function");
            }
            if self.proper.contains(&form) {
                return Some("proper");
            }
        }
        None
    }
}

/// Split a pointed string into clusters of `base letter + following points`.
fn clusters(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in s.chars() {
        let is_base = (0x05D0..=0x05EA).contains(&(c as u32));
        if is_base || out.is_empty() {
            out.push(c.to_string());
        } else {
            out.last_mut().unwrap().push(c);
        }
    }
    out
}

/// The surface itself plus every remainder after peeling 1–2 leading proclitic
/// clusters (so prefixed function words/names still match their base form).
fn deprefixed_forms(surface: &str) -> Vec<String> {
    let cl = clusters(surface);
    let mut forms = vec![surface.to_string()];
    let max = 2.min(cl.len().saturating_sub(1));
    for k in 1..=max {
        let all_proclitic = cl[..k]
            .iter()
            .all(|c| c.chars().next().is_some_and(|b| PROCLITICS.contains(&b)));
        if all_proclitic {
            forms.push(cl[k..].concat());
        }
    }
    forms
}

#[cfg(test)]
mod tests {
    use super::*;

    fn func_only() -> Prefilter {
        Prefilter {
            function: FUNCTION_WORDS
                .iter()
                .map(|s| normalize_surface(s))
                .collect(),
            proper: HashSet::new(),
        }
    }

    #[test]
    fn matches_bare_function_word() {
        let pf = func_only();
        assert_eq!(pf.classify(&normalize_surface("לֹא")), Some("function"));
        assert_eq!(pf.classify(&normalize_surface("הוּא")), Some("function"));
    }

    #[test]
    fn matches_prefixed_function_word() {
        let pf = func_only();
        // וְלֹא = conjunction ו + לֹא
        assert_eq!(pf.classify(&normalize_surface("וְלֹא")), Some("function"));
    }

    #[test]
    fn ignores_ordinary_verb() {
        let pf = func_only();
        assert_eq!(pf.classify(&normalize_surface("שָׁמַר")), None);
    }
}
