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

/// Dagesh point — a forte here marks the doubling the definite article induces.
const DAGESH: char = '\u{05BC}';

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
    // inflected prepositions / object marker / particles carrying a pronominal
    // suffix — closed-class paradigms, never verbs, so always safe to exclude.
    // ל "to/for"
    "לִי", "לְךָ", "לָךְ", "לוֹ", "לָהּ", "לָנוּ", "לָכֶם", "לָכֶן", "לָהֶם", "לָהֶן",
    // ב "in/with"
    "בִּי", "בְּךָ", "בָּךְ", "בּוֹ", "בָּהּ", "בָּנוּ", "בָּכֶם", "בָּם", "בָּהֶם", "בָּהֶן",
    // עִם "with"
    "עִמִּי", "עִמְּךָ", "עִמָּךְ", "עִמּוֹ", "עִמָּהּ", "עִמָּנוּ", "עִמָּכֶם", "עִמָּהֶם",
    // אֵת object marker
    "אֹתִי", "אֹתְךָ", "אֹתָךְ", "אֹתוֹ", "אֹתָהּ", "אֹתָנוּ", "אֶתְכֶם", "אֶתְכֶן", "אֹתָם",
    "אֹתָן", "אֶתְהֶם", "אֶתְהֶן",
    // אֵת/אִתּ "with (accompaniment)"
    "אִתִּי", "אִתְּךָ", "אִתָּךְ", "אִתּוֹ", "אִתָּהּ", "אִתָּנוּ", "אִתְּכֶם", "אִתָּם", "אִתָּן",
    // אֶל "to/toward"
    "אֵלַי", "אֵלֶיךָ", "אֵלַיִךְ", "אֵלָיו", "אֵלֶיהָ", "אֵלֵינוּ", "אֲלֵיכֶם", "אֲלֵיכֶן",
    "אֲלֵיהֶם", "אֲלֵיהֶן",
    // עַל "upon/against"
    "עָלַי", "עָלֶיךָ", "עָלַיִךְ", "עָלָיו", "עָלֶיהָ", "עָלֵינוּ", "עֲלֵיכֶם", "עֲלֵיכֶן",
    "עֲלֵיהֶם", "עֲלֵיהֶן",
    // מִן "from"
    "מִמֶּנִּי", "מִמְּךָ", "מִמֵּךְ", "מִמֶּנּוּ", "מִמֶּנָּה", "מִכֶּם", "מִכֶּן", "מֵהֶם", "מֵהֶן",
    "מֵהֵמָּה",
    // הִנֵּה "behold" + suffix
    "הִנְנִי", "הִנֶּנִּי", "הִנּוֹ", "הִנָּם",
];

/// The Tetragrammaton and its surface variants. The lexicon carries the divine
/// name with a holem on the he (יְהֹוָה, Strong's 3068/3069), but the Masoretic
/// text writes the Qere-perpetuum pointing without it (יְהוָה / יְהוִה / יֱהוִה),
/// so it never matches the lexicon's proper-noun inventory. We add the attested
/// surface forms directly. The proclitic-peeled remainders (יהוָה / יהוִה, with no
/// shewa under the yod) let prefixed forms — לַיהוָה, בַּיהוָה, וַיהוָה — resolve too.
const DIVINE_NAMES: &[&str] = &[
    "יְהוָה", "יְהוִה", "יֱהוִה", "יהוָה", "יהוִה",
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
        // The divine name is absent from the lexicon's pointing (see DIVINE_NAMES);
        // add its attested surface forms so it is recognised as a proper noun.
        proper.extend(
            DIVINE_NAMES
                .iter()
                .map(|s| normalize_surface(s))
                .filter(|s| !s.is_empty()),
        );
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

    /// Decide whether to exclude a token from verb parsing, given whether the
    /// parser found a plausible verb reading for it.
    ///
    /// Function words are always excluded — their headwords are not verbs.
    /// A proper-noun match, however, *yields* to the verb parser when the token
    /// also has a plausible verb reading: many names are homographs of genuine
    /// verb forms (e.g. שָׁאַל "he asked" vs שָׁאוּל), and excluding those costs
    /// recall. Names with no verb reading stay excluded.
    pub fn exclude(&self, surface: &str, has_plausible_verb: bool) -> Option<&'static str> {
        match self.classify(surface) {
            Some("proper") if has_plausible_verb => None,
            other => other,
        }
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

/// Remove a dagesh (forte) from the first cluster of `form`, returning the
/// normalised remainder if one was present. When the definite article attaches —
/// either written (הַ) or assimilated into a preposition (בַּ = בְּ+הַ) — it doubles
/// the following consonant with a dagesh forte that the bare lexical form lacks.
/// Stripping it lets a peeled remainder match its citation form (הַזֶּה→זֶה,
/// בַּיּוֹם→יוֹם). Operates on a [`normalize_surface`]-ordered string, where the
/// dagesh sorts after the vowel within the cluster.
fn strip_initial_dagesh(form: &str) -> Option<String> {
    let cl = clusters(form);
    let first = cl.first()?;
    if !first.contains(DAGESH) {
        return None;
    }
    let stripped: String = first.chars().filter(|&c| c != DAGESH).collect();
    Some(std::iter::once(stripped).chain(cl[1..].iter().cloned()).collect())
}

/// The surface itself plus every remainder after peeling 1–2 leading proclitic
/// clusters (so prefixed function words/names still match their base form). For
/// each peeled remainder, the article-doubled variant is also offered with its
/// dagesh forte stripped (see [`strip_initial_dagesh`]).
fn deprefixed_forms(surface: &str) -> Vec<String> {
    let cl = clusters(surface);
    let mut forms = vec![surface.to_string()];
    let max = 2.min(cl.len().saturating_sub(1));
    for k in 1..=max {
        let all_proclitic = cl[..k]
            .iter()
            .all(|c| c.chars().next().is_some_and(|b| PROCLITICS.contains(&b)));
        if all_proclitic {
            let rem: String = cl[k..].concat();
            if let Some(bare) = strip_initial_dagesh(&rem) {
                forms.push(bare);
            }
            forms.push(rem);
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

    #[test]
    fn matches_suffixed_preposition() {
        let pf = func_only();
        // closed-class preposition + pronominal suffix — never a verb.
        assert_eq!(pf.classify(&normalize_surface("לוֹ")), Some("function"));
        assert_eq!(pf.classify(&normalize_surface("עָלָיו")), Some("function"));
        assert_eq!(pf.classify(&normalize_surface("אֹתוֹ")), Some("function"));
        assert_eq!(pf.classify(&normalize_surface("מִמֶּנּוּ")), Some("function"));
    }

    #[test]
    fn matches_divine_name_and_prefixed() {
        let pf = Prefilter {
            function: HashSet::new(),
            proper: DIVINE_NAMES.iter().map(|s| normalize_surface(s)).collect(),
        };
        assert_eq!(pf.classify(&normalize_surface("יְהוָה")), Some("proper"));
        // לַיהוָה = proclitic לַ + the peeled remainder יהוָה (no shewa on yod).
        assert_eq!(pf.classify(&normalize_surface("לַיהוָה")), Some("proper"));
    }

    #[test]
    fn matches_article_doubled_function_word() {
        let pf = func_only();
        // הַזֶּה = article הַ + זֶה, with the article doubling the zayin (dagesh
        // forte) that the bare demonstrative lacks.
        assert_eq!(pf.classify(&normalize_surface("הַזֶּה")), Some("function"));
        // הַזֹּאת = article + זֹאת.
        assert_eq!(pf.classify(&normalize_surface("הַזֹּאת")), Some("function"));
    }
}
