// Bible resource

use rusqlite::{Connection, OpenFlags, OptionalExtension};
#[cfg(feature = "embedded")]
use rust_embed::Embed;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug)]
pub struct BdbEntry {
    pub headword: String,
    pub root: String,
    pub gloss: String,
    pub content_json: String,
    /// BDB part-of-speech marker (e.g. `n.pr.m`, `n.[m.]`, `vb`), as stored.
    /// Empty when the source entry carried none; for a bare cross-reference the
    /// build inherits the target's marker so the redirect groups with it.
    pub pos: String,
    /// True for a BDB `type="root"` section header — the entry that fixes the
    /// root for the lexemes that follow. When such a header carries no part of
    /// speech of its own (it is pure root etymology, not a lexeme), the app
    /// heads it under "Root" rather than among the root's actual lexemes.
    pub is_root: bool,
}

impl BdbEntry {
    /// True when this lexeme is a proper noun — any BDB `n.pr.*` part of
    /// speech (names of people, places, peoples, deities). A root's proper
    /// names crowd out its actual semantic range, so the app lists them under
    /// a separate heading rather than inline with the common lexemes.
    pub fn is_proper_noun(&self) -> bool {
        self.pos.starts_with("n.pr")
    }

    /// A coarse part-of-speech bucket derived from the BDB `pos` marker, used by
    /// the app to head a root's lexemes under their grammatical class (verbs,
    /// nouns, adjectives, …) rather than one undifferentiated list. Returns a
    /// stable lowercase key; `"other"` covers particles, pronouns, and any entry
    /// whose marker is empty or unrecognised.
    ///
    /// The marker is normalised (whitespace stripped, lowercased) before
    /// matching so spaced variants like `n. pr. m.` and compound markers like
    /// `n.pr.m.colladj.gent` classify by their leading class. Order matters:
    /// `n.pr` is tested before the bare-noun `n` so proper names never fall
    /// through to the common-noun bucket.
    pub fn pos_category(&self) -> &'static str {
        let p: String = self
            .pos
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_lowercase();
        if p.starts_with("n.pr") {
            "proper"
        } else if p.starts_with("vb") {
            "verb"
        } else if p.starts_with("adv") {
            "adverb"
        } else if p.starts_with("adj") {
            "adjective"
        } else if p.starts_with('n') {
            "noun"
        } else if self.is_root {
            // A pos-less section header — pure root etymology, not a lexeme.
            "root"
        } else {
            "other"
        }
    }

    /// True when the entry carries something to display — a gloss or at least
    /// one structured sense. BDB heads each section with a `type="root"` entry
    /// that fixes the root for the lexemes that follow; some of those headers
    /// (e.g. the Biblical Aramaic appendix opener `xa.ac.aa`, headword `אבה`)
    /// have no definition of their own, so they reduce to an empty gloss and
    /// `{"senses":[]}`. They serve only to set the section root, and would
    /// otherwise surface as blank duplicate rows in a root tree (the Aramaic
    /// `אבה` collides with the Hebrew root `אבה` "be willing"). The row stays in
    /// the DB — cross-references still navigate to it by id — it is just hidden
    /// from the root-tree listing.
    fn has_content(&self) -> bool {
        !self.gloss.is_empty()
            || serde_json::from_str::<serde_json::Value>(&self.content_json)
                .ok()
                .and_then(|v| {
                    v.get("senses")
                        .map(|s| s.as_array().is_some_and(|a| !a.is_empty()))
                })
                .unwrap_or(false)
    }
}

/// The analysis chosen to describe one OT (Hebrew Bible) surface form, drawn
/// from the `hebrewdb` reverse-parse engine output and bridged to BDB glosses
/// via the consonantal root. Verb readings carry binyan/tense/person-gender-
/// number; noun readings carry gender/number/state. `root` is the consonantal
/// root used to pull the glossed root tree from `lexdb.bdb`.
#[derive(Debug, Default, Clone)]
pub struct HebrewWord {
    /// Normalised pointed surface form (matches `hebrewdb.surface.text`).
    pub word: String,
    /// Consonantal root bridging to `lexdb.bdb.root`. Empty if unresolved.
    pub root: String,
    /// First BDB gloss for the looked-up lexeme/root.
    pub gloss: String,
    /// Binyan (Qal, Niphal, …) for verbs; `None` for nouns.
    pub form: Option<String>,
    /// Tense/aspect (Perfect, Imperfect, Imperative, …) for verbs.
    pub tense: Option<String>,
    pub person: Option<String>,
    pub gender: Option<String>,
    pub number: Option<String>,
    /// Noun state (Absolute, Construct, …) or irregular label.
    pub state: Option<String>,
    /// Attached prefix cluster (article/preposition/vav), as pointed Hebrew.
    pub prefix: Option<String>,
    pub vav_con: bool,
    /// Pronominal object suffix PGN on a verb (e.g. `3ms` in "he struck him"),
    /// `None` when the form carries no object suffix. Used to inflect glosses
    /// ("he struck him") and to rank form complexity.
    pub obj_suffix: Option<String>,
}

/// One entry of the frequency-ordered learner vocabulary: a distinct OT
/// surface form with its exact occurrence count and a best-effort bridge to
/// root, gloss and morphology.
#[derive(Debug)]
pub struct VocabEntry {
    /// Pointed surface form as it appears in the text (trope stripped).
    pub surface: String,
    /// Exact number of OT occurrences of this surface form.
    pub occurrences: u32,
    /// Pre-filter class for surfaces that never reached the parse engine:
    /// "function" (closed-class particle) or "proper" (name).
    pub lexical_class: Option<String>,
    /// Consonantal root bridging to `lexdb.bdb.root`. Empty when unresolved.
    pub root: String,
    /// First matching BDB gloss. Empty when unresolved.
    pub gloss: String,
    /// Short human-readable morphology summary, e.g. "Qal wayyiqtol 3ms".
    /// Empty for unparsed forms.
    pub morph: String,
}

#[derive(Debug)]
pub struct SedraEntry {
    pub lexeme: String,
    pub root: String,
    pub meaning: String,
}

/// Full SEDRA information for one NT word form, drawn from the `sedradb`
/// lexicon (one row per matching `words` entry; homographs yield several).
#[derive(Debug, Default)]
pub struct SedraWord {
    /// Vocalised Hebrew form (`words.strVocalised`) — the displayed NT word.
    pub word: String,
    /// Consonantal Hebrew form (`words.strWord`).
    pub consonantal: String,
    /// Lexeme headword in Hebrew (`lexemes.strLexeme`).
    pub lexeme: String,
    /// Root in Hebrew (`roots.strRoot`).
    pub root: String,
    /// `lexemes.keyLexeme` — for root-tree and occurrence follow-up queries.
    pub key_lexeme: i64,
    /// `roots.keyRoot` — for root-tree and occurrence follow-up queries.
    pub key_root: i64,
    /// English glosses for the lexeme, in listing order.
    pub meanings: Vec<String>,
    pub gender: Option<String>,
    pub person: Option<String>,
    pub number: Option<String>,
    pub state: Option<String>,
    pub tense: Option<String>,
    pub form: Option<String>,
    pub suffix: Option<String>,
}

/// One lexeme in a root's family, used to present an overview of the whole
/// root tree alongside a looked-up word.
#[derive(Debug, Default)]
pub struct SedraLexemeSummary {
    /// Lexeme headword in Hebrew (`lexemes.strLexeme`).
    pub lexeme: String,
    /// English glosses for the lexeme, in listing order.
    pub meanings: Vec<String>,
    /// True for the lexeme of the word that was looked up.
    pub is_current: bool,
}

// SEDRA3 attribute decoders (see src_texts/SEDRA/SEDRA3.README.TXT, WORDS.TXT).
// The Rust `db gen-sedra` port stores each attribute in its own `key*` column
// rather than the packed 32-bit integer described in the README.

fn decode_gender(k: i64) -> Option<String> {
    Some(
        match k {
            1 => "Common",
            2 => "Masculine",
            3 => "Feminine",
            _ => return None,
        }
        .to_string(),
    )
}

fn decode_person(k: i64) -> Option<String> {
    Some(
        match k {
            1 => "Third",
            2 => "Second",
            3 => "First",
            _ => return None,
        }
        .to_string(),
    )
}

fn decode_number(k: i64) -> Option<String> {
    Some(
        match k {
            1 => "Singular",
            2 => "Plural",
            _ => return None,
        }
        .to_string(),
    )
}

fn decode_state(k: i64) -> Option<String> {
    Some(
        match k {
            1 => "Absolute",
            2 => "Construct",
            3 => "Emphatic",
            _ => return None,
        }
        .to_string(),
    )
}

fn decode_tense(k: i64) -> Option<String> {
    Some(
        match k {
            1 => "Perfect",
            2 => "Imperfect",
            3 => "Imperative",
            4 => "Infinitive",
            5 => "Active participle",
            6 => "Passive participle",
            7 => "Participle",
            _ => return None,
        }
        .to_string(),
    )
}

fn decode_form(k: i64) -> Option<String> {
    Some(
        match k {
            1 => "Peal",
            2 => "Ethpeal",
            3 => "Pael",
            4 => "Ethpaal",
            5 => "Aphel",
            6 => "Ettaphal",
            7 => "Shaphel",
            8 => "Eshtaphal",
            9 => "Saphel",
            10 => "Estaphal",
            11 => "Pauel",
            12 => "Ethpaual",
            13 => "Paiel",
            14 => "Ethpaial",
            15 => "Palpal",
            16 => "Ethpalpal",
            17 => "Palpel",
            18 => "Ethpalpal",
            19 => "Pamel",
            20 => "Ethpamal",
            21 => "Parel",
            22 => "Ethparal",
            23 => "Pali",
            24 => "Ethpali",
            25 => "Pahli",
            26 => "Ethpahli",
            27 => "Taphel",
            28 => "Ethaphal",
            _ => return None,
        }
        .to_string(),
    )
}

/// Compact pronominal-suffix label, e.g. `3ms suffix`. `None` when the word
/// carries no suffix.
fn decode_suffix(person: i64, gender: i64, number: i64) -> Option<String> {
    if person == 0 {
        return None;
    }
    let p = match person {
        1 => "3",
        2 => "2",
        3 => "1",
        _ => "?",
    };
    let g = match gender {
        1 => "m",
        2 => "f",
        _ => "c",
    };
    // keySuffixNumber: 0 = singular/none, 1 = plural.
    let n = if number == 1 { "p" } else { "s" };
    Some(format!("{p}{g}{n} suffix"))
}

/// Decode a verb PGN tag (e.g. `3ms`, `2fp`, empty for infinitives) into the
/// person, gender and number chip labels. Each component is independent so
/// participles (`ms`, no person) and infinitives (empty) decode cleanly.
fn decode_pgn(pgn: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut person = None;
    let mut gender = None;
    let mut number = None;
    for c in pgn.chars() {
        match c {
            '1' => person = Some("First".to_string()),
            '2' => person = Some("Second".to_string()),
            '3' => person = Some("Third".to_string()),
            'm' => gender = Some("Masculine".to_string()),
            'f' => gender = Some("Feminine".to_string()),
            'c' => gender = Some("Common".to_string()),
            's' => number = Some("Singular".to_string()),
            'p' => number = Some("Plural".to_string()),
            'd' => number = Some("Dual".to_string()),
            _ => {}
        }
    }
    (person, gender, number)
}

/// Split a noun label (e.g. `Singular Absolute`, `Plural Construct`,
/// `Irregular (God)`) into a number and a state. Irregular/atypical labels with
/// no leading number word are passed through whole as the state.
fn decode_noun_label(label: &str) -> (Option<String>, Option<String>) {
    if let Some((num, rest)) = label.split_once(' ')
        && matches!(num, "Singular" | "Plural" | "Dual")
    {
        let state = (!rest.is_empty()).then(|| rest.to_string());
        return (Some(num.to_string()), state);
    }
    let state = (!label.is_empty()).then(|| label.to_string());
    (None, state)
}

#[derive(Debug)]
pub struct WordOccurrence {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
}

/// An OT verse where some inflected form of a root occurs, tagged with the
/// surface form found there so the UI can filter a root's occurrences by form
/// (the OT analogue of the NT lexeme filter). One row per (verse, surface form).
#[derive(Debug)]
pub struct HebrewOccurrence {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
    pub form: String,
}

/// An NT verse where some lexeme of a root occurs, tagged with which lexeme of
/// the root tree it belongs to (`lexeme_index` aligns with the order returned
/// by [`Bible::sedra_root_tree`]) and the distinct word forms found there.
#[derive(Debug)]
pub struct SedraOccurrence {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
    pub lexeme_index: u32,
    pub words: Vec<String>,
}

/// BDB headwords use Unicode NFC combining order (vowels CCC=17 before dagesh/dots CCC=21-24),
/// but Cardo and the biblical text data expect traditional Hebrew order (dagesh/dots first).
/// Bubble-swap any vowel that precedes a higher-priority dot/dagesh mark.
fn normalize_hebrew_combining(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i + 1 < chars.len() {
        if is_heb_vowel(chars[i]) && is_heb_dot(chars[i + 1]) {
            chars.swap(i, i + 1);
        } else {
            i += 1;
        }
    }
    chars.into_iter().collect()
}

fn is_heb_vowel(c: char) -> bool {
    let n = c as u32;
    (0x05B0..=0x05BD).contains(&n) && n != 0x05BC || n == 0x05C7
}

fn is_heb_dot(c: char) -> bool {
    matches!(c as u32, 0x05BC | 0x05C1 | 0x05C2)
}

/// NT books (40+) store lossless SEDRA-derived Hebrew that round-trips to
/// Syriac but reads as non-idiomatic Hebrew; render it idiomatically. OT books
/// hold real pointed UXLC Hebrew and are returned untouched.
fn display_hebrew(book: u8, words: &str) -> String {
    if book >= 40 {
        crate::transliterate::hebrew_display(words)
    } else {
        words.to_owned()
    }
}

/// Idiomatic rendering of an NT (SEDRA) Hebrew lexicon string — words, lexeme
/// headwords and roots are all stored in the lossless bijective form.
fn display(s: String) -> String {
    crate::transliterate::hebrew_display(&s)
}

/// Consonant skeleton of a pointed Hebrew word: niqqud stripped, final forms
/// folded to medial. Mirrors `lexicon_db::consonants` so a `hebrew.db` noun stem
/// can be matched to its BDB lexeme via the indexed `bdb.cons` column.
fn fold_consonants(word: &str) -> String {
    word.chars()
        .filter_map(|c| {
            let n = c as u32;
            if !(0x05D0..=0x05EA).contains(&n) {
                return None;
            }
            Some(match c {
                '\u{05DA}' => '\u{05DB}',
                '\u{05DD}' => '\u{05DE}',
                '\u{05DF}' => '\u{05E0}',
                '\u{05E3}' => '\u{05E4}',
                '\u{05E5}' => '\u{05E6}',
                other => other,
            })
        })
        .collect()
}

/// One-letter proclitic spellings tried (in order) when a vocabulary surface
/// form fails to resolve whole: conjunction vav, article, and the
/// inseparable prepositions, each with the English meaning shown on the card.
const PROCLITICS: [(&str, &str); 16] = [
    ("וְ", "and"),
    ("וּ", "and"),
    ("וַ", "and"),
    ("הַ", "the"),
    ("הָ", "the"),
    ("בְּ", "in"),
    ("בַּ", "in the"),
    ("בָּ", "in the"),
    ("לְ", "to"),
    ("לַ", "to the"),
    ("לָ", "to the"),
    ("לֵ", "to"),
    ("לִ", "to"),
    ("מִ", "from"),
    ("מֵ", "from"),
    ("כְּ", "like"),
];

/// Remainder of `surface` after removing a proclitic spelling, dropping the
/// dagesh the article/preposition doubles into the next consonant (it may
/// sit before or after that consonant's vowel). `None` when the proclitic
/// doesn't lead the surface or too little would remain.
fn strip_proclitic(surface: &str, proclitic: &str) -> Option<String> {
    let rest = surface.strip_prefix(proclitic)?;
    let mut chars: Vec<char> = rest.chars().collect();
    if chars.len() < 2 {
        return None;
    }
    for i in 1..chars.len() {
        if !(0x0591..=0x05C7).contains(&(chars[i] as u32)) {
            break;
        }
        if chars[i] == '\u{05BC}' {
            chars.remove(i);
            break;
        }
    }
    Some(chars.into_iter().collect())
}

/// Remove cantillation accents and meteg, leaving consonants and vowel
/// points — BDB headwords carry stress accents that surface forms don't.
fn strip_accents(word: &str) -> String {
    word.chars()
        .filter(|&c| {
            let n = c as u32;
            !(0x0591..=0x05AF).contains(&n) && n != 0x05BD
        })
        .collect()
}

/// Curated `(surface, root, gloss)` for high-frequency closed-class words whose
/// consonant skeleton collides with an unrelated lexeme, so the BDB lookups in
/// [`lexicon_fallback`] would otherwise bridge them to the wrong sense — כִּי
/// "that/because" shares כ-י with the verb כוה "burn", אֲשֶׁר "who/which" with
/// אשׁר "go straight", אֶת (object marker) with the pronoun "thou", אִם "if"
/// with אֵם "mother". These are among the most frequent words in the OT, so a
/// wrong gloss is conspicuous; consulting this table first pins the
/// function-word sense. Keyed by accent-stripped pointed form; root is left
/// empty for the bare particles (as their BDB headwords are).
const CURATED_GLOSSES: &[(&str, &str, &str)] = &[
    ("אֶת", "", "mark of the accusative; with"),
    ("אֲשֶׁר", "", "who; which; that"),
    ("כִּי", "", "that; because; for; when"),
    ("אִם", "", "if; whether"),
    ("אַל", "", "not; do not"),
    ("אֵין", "", "there is not; nothing; without"),
    ("לָהֶם", "", "to them; for them"),
    ("לוֹ", "", "to him; for him"),
    ("עַד", "", "until; as far as; while"),
    ("לִפְנֵי", "פנה", "before; in the presence of"),
    ("כֵּן", "", "so; thus"),
    ("סוּס", "סוס", "horse"),
    // Suffixed prepositions and pronouns whose BDB consonant group holds only
    // a cross-reference stub (or an unrelated lexeme), so the pointing-blind
    // fallback cannot reach a usable sense: אֵלַי collides with אֻלַי "see
    // אוּלַי", הֲלֹא with הָלָא "removed far off", שָׁמָּה with שַׁמָּה
    // "waste". Both regular and pausal pointings are listed where both occur.
    ("אֵלַי", "", "to me; unto me"),
    ("אֵלָי", "", "to me; unto me"),
    ("מִמֶּנִּי", "", "from me"),
    ("עִמָּדִי", "", "with me"),
    ("לָמוֹ", "", "to them (poetic)"),
    ("הוּא", "", "he; it"),
    ("הַהוּא", "", "that (m.)"),
    ("הִיא", "", "she; it"),
    ("הִוא", "", "she; it (ketiv of הִיא)"),
    ("הַהִיא", "", "that (f.)"),
    ("זֹאת", "", "this (f.)"),
    ("זֹּאת", "", "this (f.)"),
    ("אֲנַחְנוּ", "", "we"),
    ("לְמַעַן", "", "for the sake of; in order that"),
    ("לָכֵן", "", "therefore"),
    ("שָׁמָּה", "", "there; to there"),
    ("הֲלֹא", "", "is it not?; surely"),
    ("יַעַן", "", "because; on account of"),
    ("סֶלָה", "", "Selah — a pause (in Psalms)"),
    ("חִנָּם", "", "for nothing; without cause"),
    ("שְׁתֵּים", "", "two (f.)"),
    ("פִּתְאֹם", "", "suddenly"),
    // לְ + suffix spellings BDB only records as the stub לְכָה "see הָלַךְ";
    // the doubled (post-maqqef) spellings are separate keys because the
    // canonicalisation keeps dagesh.
    ("לְךָ", "", "to you; for you (m.)"),
    ("לְּךָ", "", "to you; for you (m.)"),
    ("לָךְ", "", "to you; for you (f.)"),
    ("לָּךְ", "", "to you; for you (f.)"),
    ("לָהּ", "", "to her; for her"),
    ("לָּהּ", "", "to her; for her"),
    ("לָמָּה", "", "why?"),
    ("לָמָה", "", "why?"),
    ("לִקְרַאת", "קרא", "to meet; toward"),
    ("לְבַדִּי", "בדד", "by myself; alone"),
    // Proper names whose spelling only appears in BDB as a cross-reference to
    // the fuller spelling (a different consonant group, so unreachable).
    ("יְהוֹשֻׁעַ", "", "Joshua"),
    ("אַבְשָׁלוֹם", "", "Absalom"),
    ("יְחִזְקִיָּהוּ", "", "Hezekiah"),
];

/// Curated `(root, gloss)` for a surface, ignoring cantillation and combining
/// order — the override consulted ahead of the BDB lookups (see
/// [`CURATED_GLOSSES`]).
fn curated_gloss(surface: &str) -> Option<(String, String)> {
    let canonical = normalize_hebrew_combining(&strip_accents(surface));
    CURATED_GLOSSES.iter().find_map(|(key, root, gloss)| {
        (normalize_hebrew_combining(&strip_accents(key)) == canonical)
            .then(|| (root.to_string(), gloss.to_string()))
    })
}

/// Lexicon-only `(root, gloss, prefix)` for a surface with no generated
/// analysis — the function-word / proper-noun bridge. Consults the curated
/// override first, then an exact pointed headword, then a proclitic-stripped
/// match, then a pointing-blind consonant match. The connection must have the
/// BDB lexicon attached as `lexdb` (true of both the runtime [`Bible`]
/// connection and the gen-hebrew build, which uses this to precompute the
/// `lexical_analyses` table). `prefix` is the proclitic spelling when one was
/// stripped, otherwise empty.
pub(crate) fn lexicon_fallback(db: &Connection, surface: &str) -> Option<(String, String, String)> {
    if let Some((root, gloss)) = curated_gloss(surface).or_else(|| bdb_exact(db, surface)) {
        return Some((root, gloss, String::new()));
    }
    for (proclitic, _) in PROCLITICS {
        if let Some(rest) = strip_proclitic(surface, proclitic) {
            let matched = curated_gloss(&rest)
                .or_else(|| bdb_exact(db, &rest))
                .or_else(|| {
                    (fold_consonants(&rest).chars().count() >= 3)
                        .then(|| bdb_cons(db, &rest))
                        .flatten()
                });
            if let Some((root, gloss)) = matched {
                return Some((root, gloss, proclitic.to_string()));
            }
        }
    }
    bdb_cons(db, surface).map(|(root, gloss)| (root, gloss, String::new()))
}

/// True when a BDB gloss is only a cross-reference to another article — "see
/// עלה", "אֻלַי see אוּלַי", "under אול", "see sub I. כלל." — rather than a
/// meaning. BDB files many headwords as stubs pointing into the article they
/// are treated under, and those stubs sort *before* the real article, so the
/// bridge must never serve one as a gloss. A stub needs a Hebrew target after
/// the keyword: the bare gloss "see" (the verb רָאָה) and English glosses that
/// merely start with "under" ("the under part") are kept. Leading Hebrew
/// citation words are skipped before testing.
fn cross_reference_gloss(gloss: &str) -> bool {
    let hebrew_char = |c: char| matches!(c as u32, 0x0590..=0x05FF | 0xFB1D..=0xFB4F);
    let hebrew_word = |w: &str| w.chars().any(hebrew_char);
    let mut words = gloss.split_whitespace().skip_while(|w| {
        w.chars()
            .all(|c| hebrew_char(c) || c.is_ascii_punctuation())
    });
    matches!(
        words
            .next()
            .map(|w| w.trim_matches(|c: char| c.is_ascii_punctuation())),
        Some("see" | "under")
    ) && words.any(hebrew_word)
}

/// True when a BDB gloss is only a root-header stub — the entire gloss is one
/// parenthetical remark introducing the derived words filed after it ("(√ of
/// following; meaning dubious; compare Lag BN 55 Anm).", "(meaning unknown).")
/// rather than a sense of its own. Such rows precede the real article in
/// lexicon order (the זהב root header sorts before זָהָב "gold"), so the
/// bridge must never serve one as a gloss; their `root` column is still
/// self-referential, so they may name a root. Real glosses that merely open
/// with a parenthetical ("(he)-ass", "(less oft. שַׁלֻּם) n.pr.m. king…")
/// carry English after the closing paren and are kept, as is an unbalanced
/// paren (truncated source text may still hold a sense).
fn root_stub_gloss(gloss: &str) -> bool {
    if !gloss.starts_with('(') {
        return false;
    }
    let mut depth = 0usize;
    for (i, ch) in gloss.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let rest = &gloss[i + 1..];
                    return !rest.chars().any(|c| c.is_ascii_alphabetic());
                }
            }
            _ => {}
        }
    }
    false
}

/// The glossed BDB lexeme whose pointed headword (accents stripped) matches the
/// surface exactly — the citation-form bridge. Both sides are reordered to
/// traditional combining order before comparison (surfaces store
/// vowel-before-dagesh, headwords vary).
fn bdb_exact(db: &Connection, surface: &str) -> Option<(String, String)> {
    let canonical = normalize_hebrew_combining(surface);
    bdb_rows(db, surface)?
        .into_iter()
        .find(|(word, _, _)| normalize_hebrew_combining(&strip_accents(word)) == canonical)
        .map(|(_, root, gloss)| (root, gloss))
}

/// The first glossed BDB lexeme sharing the surface's consonant skeleton — a
/// last-resort bridge that ignores pointing.
fn bdb_cons(db: &Connection, surface: &str) -> Option<(String, String)> {
    bdb_rows(db, surface)?
        .into_iter()
        .next()
        .map(|(_, root, gloss)| (root, gloss))
}

/// Glossed BDB `(word, root, gloss)` rows matching the surface's consonant
/// skeleton, best gloss first. Cross-reference stubs ([`cross_reference_gloss`])
/// are dropped outright — bridging to "see עלה" (and the stub's root, often a
/// neighbouring article's) is worse than no bridge — as are root-header stubs
/// ([`root_stub_gloss`]), which otherwise beat the real article by lexicon
/// order (זהב's "(√ of following…)" vs "gold"). Among the rest, glosses
/// that open with English rank before those led by a Hebrew citation
/// ("עָ֑ל subst. height"), which mark secondary sub-entries; the sort is
/// stable, so lexicon order breaks ties.
fn bdb_rows(db: &Connection, surface: &str) -> Option<Vec<(String, String, String)>> {
    let cons = fold_consonants(surface);
    if cons.is_empty() {
        return None;
    }
    let mut stmt = db
        .prepare(
            "SELECT word, root, gloss FROM lexdb.bdb \
             WHERE cons = ?1 AND gloss IS NOT NULL AND gloss <> '' \
             ORDER BY bdb_id",
        )
        .ok()?;
    let mut rows = stmt
        .query_map([&cons], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .ok()?
        .collect::<rusqlite::Result<Vec<_>>>()
        .ok()?;
    rows.retain(|(_, _, gloss)| !cross_reference_gloss(gloss) && !root_stub_gloss(gloss));
    rows.sort_by_key(|(_, _, gloss)| {
        gloss
            .chars()
            .next()
            .is_some_and(|c| matches!(c as u32, 0x0590..=0x05FF))
    });
    Some(rows)
}

/// Compact human-readable morphology line for a vocabulary card, e.g.
/// "Qal wayyiqtol 3ms" for verbs or "noun, plural construct" for nouns,
/// prefixed with any attached cluster ("הַ־ + …").
fn morph_summary(info: &HebrewWord) -> String {
    let body = if let Some(binyan) = &info.form {
        let mut s = binyan.clone();
        if let Some(tense) = &info.tense {
            s.push(' ');
            s.push_str(&tense.to_lowercase());
        }
        let pgn: String = [
            info.person.as_deref().map(|p| match p {
                "First" => "1",
                "Second" => "2",
                _ => "3",
            }),
            info.gender.as_deref().map(|g| match g {
                "Masculine" => "m",
                "Feminine" => "f",
                _ => "c",
            }),
            info.number.as_deref().map(|n| match n {
                "Singular" => "s",
                "Plural" => "p",
                _ => "d",
            }),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !pgn.is_empty() {
            s.push(' ');
            s.push_str(&pgn);
        }
        s
    } else {
        let mut parts = vec!["noun".to_string()];
        if let Some(number) = &info.number {
            parts.push(number.to_lowercase());
        }
        if let Some(state) = &info.state {
            parts.push(state.to_lowercase());
        }
        parts.join(" ")
    };
    match &info.prefix {
        Some(prefix) => format!("{prefix}־ + {body}"),
        None => body,
    }
}

// --- English gloss inflection --------------------------------------------------
//
// The BDB gloss is a lexeme sense ("say", "send"); a learner meets an inflected
// *form* ("and he said", "his word"). [`inflected_gloss`] turns the lexeme gloss
// plus the parsed morphology into a natural English rendering of the specific
// form. It is deliberately mechanical — verbs read as past/future/etc., nouns
// take number/possessive/preposition — and rough on modal nuance; the curated
// Dart overrides still win for the words where it matters most.

/// Irregular English simple-past forms, for verb glosses. Only verbs that occur
/// as common Biblical senses need cover here; anything absent falls back to the
/// regular `-ed` rule in [`past_tense`].
const IRREGULAR_PAST: &[(&str, &str)] = &[
    ("say", "said"),
    ("go", "went"),
    ("come", "came"),
    ("see", "saw"),
    ("give", "gave"),
    ("take", "took"),
    ("make", "made"),
    ("know", "knew"),
    ("eat", "ate"),
    ("do", "did"),
    ("find", "found"),
    ("hear", "heard"),
    ("tell", "told"),
    ("become", "became"),
    ("build", "built"),
    ("send", "sent"),
    ("keep", "kept"),
    ("stand", "stood"),
    ("fall", "fell"),
    ("bring", "brought"),
    ("buy", "bought"),
    ("seek", "sought"),
    ("fight", "fought"),
    ("put", "put"),
    ("set", "set"),
    ("cut", "cut"),
    ("let", "let"),
    ("sit", "sat"),
    ("speak", "spoke"),
    ("write", "wrote"),
    ("bear", "bore"),
    ("break", "broke"),
    ("choose", "chose"),
    ("rise", "rose"),
    ("fear", "feared"),
    ("hold", "held"),
    ("lay", "laid"),
    ("lead", "led"),
    ("leave", "left"),
    ("meet", "met"),
    ("read", "read"),
    ("run", "ran"),
    ("show", "showed"),
    ("shut", "shut"),
    ("sell", "sold"),
    ("throw", "threw"),
    ("draw", "drew"),
    ("dwell", "dwelt"),
    ("weep", "wept"),
    ("bind", "bound"),
    ("wear", "wore"),
    ("swear", "swore"),
    ("smite", "smote"),
    ("slay", "slew"),
    ("flee", "fled"),
    ("hide", "hid"),
    ("shake", "shook"),
    ("swim", "swam"),
    ("drink", "drank"),
];

/// Irregular English plurals for noun glosses; regular nouns take the `-s`/`-es`
/// rule in [`pluralize`].
const IRREGULAR_PLURAL: &[(&str, &str)] = &[
    ("man", "men"),
    ("woman", "women"),
    ("child", "children"),
    ("foot", "feet"),
    ("tooth", "teeth"),
    ("ox", "oxen"),
    ("person", "people"),
    ("life", "lives"),
    ("wife", "wives"),
    ("knife", "knives"),
    ("leaf", "leaves"),
];

/// The primary lexeme sense of a (possibly multi-part) BDB gloss suitable for
/// English inflection: the first clean clause. BDB glosses are littered with
/// cross-references ("see דָּאָה"), embedded Hebrew, parentheticals and
/// grammatical abbreviations ("n.pr.m."); a clause carrying any of those is
/// skipped, and an empty result signals the caller to leave the gloss
/// uninflected rather than emit garbage like "see דָּאָהed".
/// Whether a BDB gloss describes a proper name — a person, place or people
/// marked `n.pr.m` / `n.pr.f` / `n.pr.loc` / `n.pr.gent` (the marker appears
/// either leading the gloss or parenthesised inside it). Names carry no
/// meaning to quiz and their bridged roots are usually spurious, so the tutor
/// treats them separately (see `is_name` in [`crate::tutor`]).
pub(crate) fn is_name_gloss(gloss: &str) -> bool {
    gloss.contains("n.pr")
}

/// The human part of a BDB proper-name gloss — the citation minus its
/// `n.pr.*` markers, any leading Hebrew headword and joining punctuation:
/// "n.pr.m. father of one of David's men" → "father of one of David's men";
/// "חֶצְרַי (n.pr.m.)—one of David's heroes" → "one of David's heroes".
pub(crate) fn name_description(gloss: &str) -> String {
    let mut s = gloss.to_string();
    while let Some(i) = s.find("n.pr") {
        let end = s[i..]
            .char_indices()
            .find(|&(_, c)| c.is_whitespace() || matches!(c, ')' | ']' | '—' | ',' | ';'))
            .map_or(s.len(), |(j, _)| i + j);
        s.replace_range(i..end, "");
    }
    s.trim_matches(|c: char| {
        c.is_whitespace()
            || matches!(c as u32, 0x0590..=0x05FF)
            || matches!(c, '(' | ')' | '—' | '-' | '.' | ',' | ';' | ':')
    })
    .to_string()
}

/// A curated proper name behind one or two proclitics (לְיַעֲקֹב, וּלְיַעֲקֹב):
/// the name's curated gloss composed with the prefixes' senses — `("to Jacob",
/// note)`, `("and to Jacob", note)`. Without this the bridge serves the name's
/// homograph root instead ("to heel"). `None` when no proclitic chain ends at
/// a curated name.
pub(crate) fn prefixed_name_gloss(surface: &str) -> Option<(String, String)> {
    type Chain = Vec<(&'static str, &'static str)>;
    fn strip_names(surface: &str, depth: u8) -> Option<(Chain, String, &'static str)> {
        for (proclitic, sense) in PROCLITICS {
            let Some(rest) = strip_proclitic(surface, proclitic) else {
                continue;
            };
            // Names take no article, so "to the"-style senses drop it.
            let sense = sense.trim_end_matches(" the");
            if crate::vocab_gloss::curated_name(&rest)
                && let Some(c) = crate::vocab_gloss::curated_gloss(&rest)
            {
                return Some((vec![(proclitic, sense)], rest, c.gloss));
            }
            if depth > 0
                && let Some((mut chain, stem, gloss)) = strip_names(&rest, depth - 1)
            {
                chain.insert(0, (proclitic, sense));
                return Some((chain, stem, gloss));
            }
        }
        None
    }
    let (chain, stem, gloss) = strip_names(surface, 1)?;
    let senses: Vec<&str> = chain.iter().map(|&(_, s)| s).collect();
    let note = chain
        .iter()
        .map(|&(p, s)| format!("{p} ({s})"))
        .chain([format!("{stem} ({gloss})")])
        .collect::<Vec<_>>()
        .join(" + ");
    Some((format!("{} {gloss}", senses.join(" ")), note))
}

fn primary_sense(gloss: &str) -> String {
    for clause in gloss.split([';', ',']) {
        let c = clause.trim();
        if c.is_empty() {
            continue;
        }
        // Embedded Hebrew (a cross-reference), a parenthetical, or a "see …" /
        // "√ …" / "cf …" reference — not a usable English sense.
        let has_hebrew = c.chars().any(|ch| ('\u{0590}'..='\u{05FF}').contains(&ch));
        let lower = c.to_lowercase();
        let is_ref = lower.starts_with("see ")
            || lower.starts_with("cf")
            || lower.starts_with("id.")
            || lower.contains("n.pr")
            || c.starts_with('√')
            || c.contains('(');
        if has_hebrew || is_ref {
            continue;
        }
        return c.trim_start_matches("to ").trim().to_string();
    }
    String::new()
}

/// English simple past of a base verb (`say` → `said`, `walk` → `walked`).
fn past_tense(verb: &str) -> String {
    if let Some((_, past)) = IRREGULAR_PAST.iter().find(|(v, _)| *v == verb) {
        return (*past).to_string();
    }
    if verb == "be" {
        return "was".to_string();
    }
    regular_suffix(verb, "ed")
}

/// English `-ing` form of a base verb (`say` → `saying`, `make` → `making`).
fn ing_form(verb: &str) -> String {
    if let Some(stem) = verb.strip_suffix('e')
        && !verb.ends_with("ee")
        && verb.len() > 2
    {
        return format!("{stem}ing");
    }
    format!("{verb}ing")
}

/// Apply a regular verbal/plural suffix, handling silent-e and consonant-y:
/// `love`+`ed` → `loved`, `carry`+`ed` → `carried`, `walk`+`ed` → `walked`.
fn regular_suffix(word: &str, suffix: &str) -> String {
    let ed = suffix == "ed";
    if let Some(stem) = word.strip_suffix('y')
        && !stem.ends_with(['a', 'e', 'i', 'o', 'u'])
        && !stem.is_empty()
    {
        return format!("{stem}i{suffix}");
    }
    if ed && word.ends_with('e') {
        return format!("{word}d");
    }
    format!("{word}{suffix}")
}

/// English plural of a base noun.
fn pluralize(noun: &str) -> String {
    if let Some((_, pl)) = IRREGULAR_PLURAL.iter().find(|(s, _)| *s == noun) {
        return (*pl).to_string();
    }
    if noun.ends_with(['s', 'x', 'z']) || noun.ends_with("ch") || noun.ends_with("sh") {
        return format!("{noun}es");
    }
    regular_suffix(noun, "s")
}

/// Subject pronoun for a verb's person/gender/number (`he`, `she`, `they`, …),
/// or `None` for a form with no person (participle, infinitive).
fn subject_pronoun(w: &HebrewWord) -> Option<&'static str> {
    let plural = matches!(w.number.as_deref(), Some("Plural") | Some("Dual"));
    match w.person.as_deref()? {
        "First" => Some(if plural { "we" } else { "I" }),
        "Second" => Some("you"),
        "Third" => Some(match (w.gender.as_deref(), plural) {
            (_, true) => "they",
            (Some("Feminine"), false) => "she",
            _ => "he",
        }),
        _ => None,
    }
}

/// Object pronoun for a verb's pronominal object suffix (`him`, `her`, `them`, …).
fn object_pronoun(pgn: &str) -> Option<&'static str> {
    Some(match pgn {
        "3ms" => "him",
        "3fs" => "her",
        "3mp" | "3fp" | "3cp" => "them",
        "1cs" => "me",
        "1cp" => "us",
        s if s.starts_with('2') => "you",
        _ => return None,
    })
}

/// Objective pronoun used as the subject of a let-clause (jussive/cohortative):
/// `let him …`, `let me …`.
fn let_subject(w: &HebrewWord) -> &'static str {
    let plural = matches!(w.number.as_deref(), Some("Plural") | Some("Dual"));
    match w.person.as_deref() {
        Some("First") => {
            if plural {
                "us"
            } else {
                "me"
            }
        }
        Some("Second") => "you",
        _ => match (w.gender.as_deref(), plural) {
            (_, true) => "them",
            (Some("Feminine"), false) => "her",
            _ => "him",
        },
    }
}

/// The English sense that a proclitic conjunction/preposition contributes when
/// attached to a noun, keyed by the first letter of the pointed prefix cluster.
fn proclitic_word(prefix: &str) -> Option<&'static str> {
    match prefix.chars().next()? {
        '\u{05D5}' => Some("and"),  // vav
        '\u{05DC}' => Some("to"),   // lamed
        '\u{05D1}' => Some("in"),   // bet
        '\u{05DB}' => Some("like"), // kaf
        '\u{05DE}' => Some("from"), // mem
        '\u{05D4}' => Some("the"),  // he (article)
        _ => None,
    }
}

/// Render the specific inflected form of a word in English, from its lexeme
/// gloss plus parsed morphology — "and he said", "his word", "the kings". Falls
/// back to the bare gloss for function words, proper nouns, and anything with no
/// usable sense.
pub(crate) fn inflected_gloss(w: &HebrewWord) -> String {
    let base = primary_sense(&w.gloss);
    if base.is_empty() {
        return w.gloss.clone();
    }
    if w.form.is_some() {
        inflect_verb(w, &base)
    } else if w.tense.is_none() && (w.number.is_some() || w.state.is_some()) {
        inflect_noun(w, &base)
    } else {
        // Function word / proper noun: nothing mechanical to add.
        w.gloss.clone()
    }
}

fn inflect_verb(w: &HebrewWord, base: &str) -> String {
    let obj = w.obj_suffix.as_deref().and_then(object_pronoun);
    let with_obj = |s: String| match obj {
        Some(o) => format!("{s} {o}"),
        None => s,
    };
    // Is there a leading conjunction (vav-consecutive, or a proclitic vav)?
    let and = w.vav_con
        || w.prefix
            .as_deref()
            .and_then(proclitic_word)
            .is_some_and(|word| word == "and");
    let subj = subject_pronoun(w);
    let clause = |verb: String| {
        let mut s = String::new();
        if and {
            s.push_str("and ");
        }
        if let Some(su) = subj {
            s.push_str(su);
            s.push(' ');
        }
        s.push_str(&verb);
        s
    };

    match w.tense.as_deref() {
        Some("Perfect") => with_obj(clause(past_tense(base))),
        Some("Wayyiqtol") => {
            // The wayyiqtol vav is intrinsic ("and …"), regardless of prefix.
            let mut s = String::from("and ");
            if let Some(su) = subj {
                s.push_str(su);
                s.push(' ');
            }
            s.push_str(&past_tense(base));
            with_obj(s)
        }
        Some("Imperfect") => with_obj(clause(format!("will {base}"))),
        Some("Cohortative") => with_obj(format!("let {} {base}", let_subject(w))),
        Some("Jussive") => with_obj(format!("let {} {base}", let_subject(w))),
        Some("Imperative") => with_obj(format!("{base}!")),
        Some("Inf. Construct") | Some("Inf. Absolute") => format!("to {base}"),
        Some("Participle (act.)") | Some("Participle") => with_obj(ing_form(base)),
        Some("Participle (pas.)") | Some("Participle (pass.)") => past_tense(base),
        _ => with_obj(clause(base.to_string())),
    }
}

/// Up to three *other* inflected glosses of the same word, contrasting the
/// grammatical form — for a "which form is this?" multiple-choice drill. A
/// finite verb varies its person/gender/number ("he said" vs "she said" vs
/// "they said"); a participle or infinitive (no person/gender/number axis
/// changes its gloss) varies tense instead ("saying" vs "he said" vs "to
/// say"); a suffixed noun varies its possessor ("his word" vs "their word");
/// a plain noun varies number and state ("king" vs "kings" vs "king of").
/// Empty when no meaningful contrast exists (the app then falls back to
/// reveal-and-self-grade).
pub(crate) fn form_distractors(w: &HebrewWord) -> Vec<String> {
    let correct = inflected_gloss(w);
    let mut out: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    seen.insert(correct.to_lowercase());
    let mut consider = |variant: &HebrewWord, out: &mut Vec<String>| {
        let g = inflected_gloss(variant);
        if !g.is_empty() && seen.insert(g.to_lowercase()) {
            out.push(g);
        }
    };

    if w.form.is_some() && w.person.is_some() {
        // Verb: contrast the subject (and keep the same tense/binyan).
        for (p, g, n) in [
            ("Third", "Masculine", "Singular"),
            ("Third", "Feminine", "Singular"),
            ("Third", "Masculine", "Plural"),
            ("First", "Common", "Singular"),
            ("Second", "Masculine", "Singular"),
            ("First", "Common", "Plural"),
        ] {
            let mut v = w.clone();
            v.person = Some(p.to_string());
            v.gender = Some(g.to_string());
            v.number = Some(n.to_string());
            consider(&v, &mut out);
            if out.len() >= 3 {
                break;
            }
        }
    } else if w.form.is_some() {
        // Participle or infinitive: person/gender/number don't change the
        // English gloss (an act. participle is always "-ing"; an infinitive
        // is always "to …"), so contrast the tense instead ("saying" vs "he
        // said" vs "to say"). Perfect/Imperfect need a subject to render;
        // default to third-masculine-singular.
        for tense in [
            "Perfect",
            "Imperfect",
            "Imperative",
            "Participle",
            "Inf. Construct",
        ] {
            let mut v = w.clone();
            v.tense = Some(tense.to_string());
            if matches!(tense, "Perfect" | "Imperfect") {
                v.person = Some("Third".to_string());
                v.gender = Some("Masculine".to_string());
                v.number = Some("Singular".to_string());
            }
            consider(&v, &mut out);
            if out.len() >= 3 {
                break;
            }
        }
    } else if w.form.is_none() {
        let state = w.state.as_deref().unwrap_or("");
        if let Some((num, _)) = state.split_once('+') {
            // Suffixed noun: contrast the possessor.
            let num = num.trim();
            for sfx in ["3ms", "3fs", "3mp", "1cs", "2ms", "1cp"] {
                let mut v = w.clone();
                v.state = Some(format!("{num} + {sfx}"));
                consider(&v, &mut out);
                if out.len() >= 3 {
                    break;
                }
            }
        } else {
            // Plain noun: contrast number and state.
            for (num, st) in [
                ("Singular", "Absolute"),
                ("Plural", "Absolute"),
                ("Singular", "Construct"),
            ] {
                let mut v = w.clone();
                v.number = Some(num.to_string());
                v.state = Some(st.to_string());
                consider(&v, &mut out);
                if out.len() >= 3 {
                    break;
                }
            }
        }
    }
    out.truncate(3);
    out
}

fn inflect_noun(w: &HebrewWord, base: &str) -> String {
    // The noun label lives in `state`, e.g. "Absolute", "Construct", "Sg + 3ms".
    let state = w.state.as_deref().unwrap_or("");
    let plural =
        matches!(w.number.as_deref(), Some("Plural") | Some("Dual")) || state.starts_with("Pl");
    let head = if plural {
        pluralize(base)
    } else {
        base.to_string()
    };

    // Pronominal-suffix labels look like "Sg + 3ms" / "Pl + 1cs".
    let head = if let Some((_, sfx)) = state.split_once('+') {
        let sfx = sfx.trim();
        let poss = match sfx.get(..3).unwrap_or(sfx) {
            "3ms" => "his",
            "3fs" => "her",
            "3mp" | "3fp" | "3cp" => "their",
            "2ms" | "2fs" | "2mp" | "2fp" => "your",
            "1cs" => "my",
            "1cp" => "our",
            _ => "",
        };
        if poss.is_empty() {
            head
        } else {
            format!("{poss} {head}")
        }
    } else if state == "Construct" {
        format!("{head} of")
    } else {
        head
    };

    // Attached preposition / conjunction / article. A gentilic gloss already
    // leads with its article ("the Carmelite") — don't double it.
    match w.prefix.as_deref().and_then(proclitic_word) {
        Some("the") if head.starts_with("the ") || head.starts_with("The ") => head,
        Some("the") => format!("the {head}"),
        Some(prep) => format!("{prep} {head}"),
        None => head,
    }
}

#[cfg(feature = "embedded")]
#[derive(Embed)]
#[folder = "data/"]
struct Asset;

/// The databases attached to an otherwise-empty main connection, paired with
/// the schema names the queries expect.
const ATTACHED_DBS: [(&str, &str); 4] = [
    // Rust-generated bible text.
    ("bible.db", "bibledb"),
    // Rust-generated SEDRA lexicon (roots, lexemes, words, english) in
    // Hebrew Unicode.
    ("sedra.db", "sedradb"),
    // Rust reverse-parse engine output for the OT (Hebrew Bible): distinct
    // surface forms, candidate verb/noun analyses, roots and occurrences.
    ("hebrew.db", "hebrewdb"),
    // OpenScriptures HebrewLexicon (Strong's + BrownDriverBriggs). The `bdb`
    // table is root-keyed so it joins to `hebrewdb` analyses to give glossed
    // root trees with structured definitions.
    ("lexicon.db", "lexdb"),
];

#[derive(Debug)]
pub struct Bible {
    db: Connection,
}

#[cfg(feature = "embedded")]
impl Default for Bible {
    fn default() -> Self {
        let mut db = Connection::open_in_memory().unwrap();

        for (file, schema) in ATTACHED_DBS {
            db.execute_batch(&format!("ATTACH DATABASE ':memory:' AS {schema}"))
                .unwrap();
            let asset = Asset::get(file).unwrap();
            let data = Box::new(asset.data.into_owned());
            db.deserialize_bytes(schema, Box::leak(data)).unwrap();
        }

        register_sql_functions(&db).unwrap();
        Bible { db }
    }
}

/// Register the crate's custom SQLite scalar functions. `popcount(x)` returns the
/// number of set bits in an integer (NULL → 0), used by the tutor to count how
/// many *new* glyphs a word/verse introduces (`popcount(glyph_mask & ~known)`).
fn register_sql_functions(db: &Connection) -> rusqlite::Result<()> {
    use rusqlite::functions::FunctionFlags;
    db.create_scalar_function(
        "popcount",
        1,
        FunctionFlags::SQLITE_UTF8
            | FunctionFlags::SQLITE_DETERMINISTIC
            | FunctionFlags::SQLITE_INNOCUOUS,
        |ctx| {
            Ok(ctx
                .get::<Option<i64>>(0)?
                .map_or(0i64, |n| (n as u64).count_ones() as i64))
        },
    )
}

impl Bible {
    /// Open the databases file-backed and read-only from `data_dir`, which
    /// must contain the four attached databases (`bible.db`, `sedra.db`,
    /// `hebrew.db`, `lexicon.db`).
    ///
    /// The files are opened with `immutable=1`, so SQLite creates no journal
    /// or lock files and the directory may be read-only — but the files must
    /// not be modified while the connection is open.
    pub fn open<P: AsRef<Path>>(data_dir: P) -> rusqlite::Result<Self> {
        let dir = data_dir.as_ref();
        // Empty in-memory main schema; all data lives in the attached files.
        // The URI flag is what lets the ATTACH below use `file:...?immutable=1`.
        let db = Connection::open_with_flags(
            ":memory:",
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        for (file, schema) in ATTACHED_DBS {
            db.execute(
                &format!("ATTACH DATABASE ?1 AS {schema}"),
                [db_uri(dir, file)],
            )?;
        }
        register_sql_functions(&db)?;
        Ok(Bible { db })
    }

    /// Attach a writable `progress.db` (created if absent) under the `progress`
    /// schema and ensure its tables exist. Unlike the corpus databases — which
    /// [`Bible::open`] attaches read-only (`immutable=1`) — this one is
    /// read-write: the spaced-repetition tutor ([`crate::tutor`]) persists its
    /// review scheduling here. Call once after opening; tutor methods assume it.
    pub fn attach_progress<P: AsRef<Path>>(&self, progress_db: P) -> rusqlite::Result<()> {
        self.db.execute(
            "ATTACH DATABASE ?1 AS progress",
            [progress_db.as_ref().to_string_lossy().as_ref()],
        )?;
        crate::tutor::init_progress_schema(&self.db)
    }

    /// Crate-internal access to the underlying connection (all corpus schemas
    /// plus, once [`Bible::attach_progress`] has run, `progress`), for sibling
    /// modules such as [`crate::tutor`] that query across them.
    pub(crate) fn conn(&self) -> &Connection {
        &self.db
    }
}

/// SQLite URI for a read-only database file. Note that SQLite %-decodes URI
/// paths, so this would mangle a directory containing literal `%` characters;
/// app data directories never do.
fn db_uri(dir: &Path, file: &str) -> String {
    format!("file:{}?immutable=1", dir.join(file).display())
}

impl Bible {
    pub fn get(&self, book: u8, chapter: u8, verse: u8) -> rusqlite::Result<String> {
        let words: String = self.db.query_row(
            "SELECT words FROM bibledb.bible WHERE book == ?1 AND chapter == ?2 AND verse == ?3",
            [book, chapter, verse],
            |row| row.get(0),
        )?;
        Ok(display_hebrew(book, &words))
    }

    pub fn get_chapter(
        &self,
        book: u8,
        chapter: u8,
        syriac: bool,
    ) -> rusqlite::Result<Vec<(u8, String)>> {
        let mut stmt = self.db.prepare(
            "SELECT verse, words FROM bibledb.bible WHERE book = ?1 AND chapter = ?2 ORDER BY verse",
        )?;
        let verses = stmt
            .query_map([book, chapter], |row| {
                let verse: u8 = row.get(0)?;
                let words: String = row.get(1)?;
                let words = if syriac {
                    crate::transliterate::hebrew_to_syriac(&words)
                } else {
                    display_hebrew(book, &words)
                };
                Ok((verse, words))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(verses)
    }

    /// Reverse-parse a single OT surface form via `hebrewdb`, choosing the most
    /// plausible analysis and bridging it to a BDB gloss through the consonantal
    /// root. The input is normalised with the same [`crate::generate::
    /// normalize_surface`] the parse engine used, so callers may pass raw
    /// pointed/cantillated text. Returns `None` when no surface matches or the
    /// surface carries no verb or noun analysis.
    ///
    /// Disambiguation: pick the top-ranked candidate verb analysis. Rows are
    /// stored in `analysis_id` order, which the build sets to OSHB corpus
    /// attestation (most-attested reading first — lifts top-1 from ~53% to ~98%),
    /// then the generator's own `sort_matches` order (attested-before-fallback,
    /// bare-before-suffixed, exact-before-folded) for the unattested tail. A verb
    /// reading is chosen over a noun reading only when its root resolves in BDB;
    /// otherwise a resolvable noun reading wins, falling back to whatever exists.
    /// Exception: when the noun reading resolves *and* carries the definite
    /// article, a verb reading that merely shadows the article loses to it —
    /// the article never prefixes a finite verb, so הַמֶּלֶךְ is "the king",
    /// not a he-peeled imperative of הלך (article + participle stays a verb
    /// reading: that combination is real Hebrew).
    pub fn hebrew_word_info(&self, word: &str) -> Option<HebrewWord> {
        let norm = crate::generate::normalize_surface(word);
        // `surface.text` is not indexed, so resolve the surface_id once here and
        // key the (indexed) child-table lookups off it — one scan, not three.
        let surface_id: i64 = self
            .db
            .query_row(
                "SELECT surface_id FROM hebrewdb.surface WHERE text = ?1",
                [&norm],
                |r| r.get(0),
            )
            .optional()
            .ok()??;
        self.hebrew_word_by_surface_id(surface_id, norm)
    }

    /// [`Bible::hebrew_word_info`] keyed by a known `surface_id` and its already
    /// normalised `text`, so the analysis lookups hit `idx_analyses_surface` /
    /// `idx_noun_analyses_surface` directly rather than scanning through the
    /// unindexed `surface.text`. Used by the tutor's `surface_meta` build, which
    /// resolves every surface in the corpus in one pass.
    pub(crate) fn hebrew_word_by_surface_id(
        &self,
        surface_id: i64,
        norm: String,
    ) -> Option<HebrewWord> {
        // Top verb analysis by stored rank (attestation, then generator order).
        // `analysis_id` is unique, so it alone determines the pick; `has_bdb` is
        // still selected for the verb-vs-noun decision below.
        let verb = self
            .db
            .query_row(
                "SELECT a.root, a.binyan, a.form, a.pgn, a.prefix, a.vav_consecutive, \
                        a.obj_suffix, a.attested, \
                        EXISTS(SELECT 1 FROM lexdb.bdb b WHERE b.root = a.root) AS has_bdb \
                 FROM hebrewdb.analyses a \
                 WHERE a.surface_id = ?1 \
                 ORDER BY a.analysis_id ASC \
                 LIMIT 1",
                [surface_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)? != 0,
                        row.get::<_, String>(6)?,
                        row.get::<_, i64>(7)? != 0,
                        row.get::<_, i64>(8)? != 0,
                    ))
                },
            )
            .optional()
            .ok()?;

        // Candidate noun analyses, resolved to a BDB root by folding the stem to
        // bare medial consonants and matching `bdb.cons`; the curated overrides
        // are consulted first so a homograph collision (סוּס the horse vs BDB's
        // preceding "swallow; swift" bird entry) picks the intended lexeme. The
        // first stem that resolves wins; otherwise the first candidate is kept
        // unresolved so the morphology still shows even without a lexicon
        // bridge.
        let noun_rows = {
            let mut stmt = self
                .db
                .prepare(
                    "SELECT n.kind, n.label, n.prefix, n.stem \
                     FROM hebrewdb.noun_analyses n \
                     WHERE n.surface_id = ?1 ORDER BY n.analysis_id ASC",
                )
                .ok()?;
            stmt.query_map([surface_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .ok()?
            .collect::<rusqlite::Result<Vec<_>>>()
            .ok()?
        };

        // (kind, label, prefix, root, gloss): root/gloss empty if unresolved.
        let noun: Option<(String, String, String, String, String)> = {
            let mut chosen: Option<(String, String, String, String, String)> = None;
            for (kind, label, prefix, stem) in noun_rows {
                let resolved = curated_gloss(&stem)
                    .or_else(|| self.hebrew_cons_root(&fold_consonants(&stem)));
                let resolves = resolved.is_some();
                let (root, gloss) = resolved.unwrap_or_default();
                if resolves {
                    chosen = Some((kind, label, prefix, root, gloss));
                    break;
                }
                chosen.get_or_insert((kind, label, prefix, root, gloss));
            }
            chosen
        };

        let noun_resolves = noun.as_ref().is_some_and(|n| !n.3.is_empty());
        // A resolvable noun reading led by the definite article beats a verb
        // reading that only exists by mistreating that article: a he-peeled
        // non-participle (the article never prefixes a finite verb — הָעָם is
        // not an imperative of עמה) or a strong-verb fallback that buries the
        // article inside the root (הַיּוֹם as *הימ). The he must carry real
        // article pointing (patah/qamats/segol): a hataf-patah he is the
        // interrogative, whose verb reading is genuine (הֲתֵלֵךְ "will you
        // go?" is not "your mound"). Article + participle is real Hebrew, so
        // participles keep their verb reading.
        let article_pointed = |prefix: &str| {
            let mut cs = prefix.chars();
            cs.next() == Some('\u{05D4}')
                && matches!(cs.next(), Some('\u{05B7}' | '\u{05B8}' | '\u{05B6}'))
        };
        let article_noun = noun_resolves && noun.as_ref().is_some_and(|n| article_pointed(&n.2));
        let verb_shadows_article = verb.as_ref().is_some_and(|v| {
            let participle = v.2.contains("Participle");
            (!participle && v.4.starts_with('\u{05D4}')) || (!v.7 && v.0.starts_with('\u{05D4}'))
        });
        let verb_resolves =
            verb.as_ref().is_some_and(|v| v.8) && !(article_noun && verb_shadows_article);

        // Prefer a BDB-resolvable verb; else a resolvable noun; else whatever
        // analysis exists (verb before noun).
        if let Some((root, binyan, tense, pgn, prefix, vav_con, obj_suffix, _, _)) =
            verb.as_ref().filter(|_| verb_resolves || !noun_resolves)
        {
            let (person, gender, number) = decode_pgn(pgn);
            let gloss = self.hebrew_root_gloss(root);
            return Some(HebrewWord {
                word: norm,
                root: root.clone(),
                gloss,
                form: (!binyan.is_empty()).then(|| binyan.clone()),
                tense: (!tense.is_empty()).then(|| tense.clone()),
                person,
                gender,
                number,
                state: None,
                prefix: (!prefix.is_empty()).then(|| prefix.clone()),
                vav_con: *vav_con,
                obj_suffix: (!obj_suffix.is_empty()).then(|| obj_suffix.clone()),
            });
        }

        if let Some((kind, label, prefix, root, gloss)) = noun {
            let (number, state) = decode_noun_label(&label);
            return Some(HebrewWord {
                word: norm,
                root,
                gloss,
                form: None,
                tense: None,
                person: None,
                gender: (!kind.is_empty()).then_some(kind),
                number,
                state,
                prefix: (!prefix.is_empty()).then_some(prefix),
                vav_con: false,
                obj_suffix: None,
            });
        }

        // Closed-class function words (and proper nouns) carry a surface row but
        // no generated verb/noun analysis — the prefilter strips their spurious
        // verb readings and they are not nouns. The gen-hebrew build precomputes
        // a lexicon bridge for them into `lexical_analyses`; read it back so the
        // app shows a gloss instead of "no OT parse". A missing table (an older
        // db) just yields `None`, the previous behaviour.
        let bridge = self
            .db
            .query_row(
                "SELECT la.root, la.gloss, la.prefix \
                 FROM hebrewdb.lexical_analyses la \
                 WHERE la.surface_id = ?1",
                [surface_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .ok()
            .flatten();
        if let Some((root, gloss, prefix)) = bridge {
            return Some(HebrewWord {
                word: norm,
                root,
                gloss,
                form: None,
                tense: None,
                person: None,
                gender: None,
                number: None,
                state: None,
                prefix: (!prefix.is_empty()).then_some(prefix),
                vav_con: false,
                obj_suffix: None,
            });
        }

        None
    }

    /// Resolve a folded consonant skeleton to a BDB `(root, gloss)` via the
    /// indexed `cons` column — the noun bridge. Goes through the ranked
    /// [`bdb_rows`] selection so cross-reference stubs are never bridged and
    /// English-led glosses beat Hebrew-citation sub-entries, exactly like the
    /// function-word bridge. When no glossed lexeme survives, a gloss-less
    /// lexeme still names the root — as does a root-header stub (paren-led
    /// gloss), whose root column is self-referential even though its gloss is
    /// unusable. `None` when no lexeme matches at all.
    fn hebrew_cons_root(&self, cons: &str) -> Option<(String, String)> {
        if cons.is_empty() {
            return None;
        }
        if let Some(hit) = bdb_cons(&self.db, cons) {
            return Some(hit);
        }
        self.db
            .query_row(
                "SELECT root FROM lexdb.bdb \
                 WHERE cons = ?1 AND (gloss IS NULL OR gloss = '' OR gloss LIKE '(%') \
                 ORDER BY bdb_id LIMIT 1",
                [cons],
                |row| Ok((row.get::<_, String>(0)?, String::new())),
            )
            .optional()
            .ok()
            .flatten()
    }

    /// Headline BDB gloss for a consonantal root: the first non-stub gloss in
    /// lexicon order, with English-led glosses ranked before Hebrew-citation
    /// sub-entries (mirroring [`bdb_rows`], but keyed by the `root` column).
    fn hebrew_root_gloss(&self, root: &str) -> String {
        let Ok(mut stmt) = self.db.prepare(
            "SELECT gloss FROM lexdb.bdb \
             WHERE root = ?1 AND gloss IS NOT NULL AND gloss <> '' \
             ORDER BY bdb_id",
        ) else {
            return String::new();
        };
        let Ok(rows) = stmt
            .query_map([root], |row| row.get::<_, String>(0))
            .and_then(|rows| rows.collect::<rusqlite::Result<Vec<_>>>())
        else {
            return String::new();
        };
        let mut glosses: Vec<String> = rows
            .into_iter()
            .filter(|gloss| !cross_reference_gloss(gloss) && !root_stub_gloss(gloss))
            .collect();
        glosses.sort_by_key(|gloss| {
            gloss
                .chars()
                .next()
                .is_some_and(|c| matches!(c as u32, 0x0590..=0x05FF))
        });
        glosses.into_iter().next().unwrap_or_default()
    }

    /// BDB lexeme(s) for a bridged surface that has no triliteral root — the
    /// function words and particles whose BDB entry carries an empty `root`
    /// column (so [`Bible::hebrew_bdb_by_root`] can never reach them), plus the
    /// curated closed-class glosses. The lookup mirrors the bridge that produced
    /// the gloss: any stored proclitic is stripped, then the exact pointed
    /// headword is preferred — so מִי resolves to "who?" alone rather than the
    /// whole מ־י consonant group (which also holds מַי "waters"). When no
    /// headword matches exactly it falls back to the consonant group, the same
    /// last resort the bridge uses.
    pub fn hebrew_bdb_for_surface(
        &self,
        word: &str,
        prefix: &str,
    ) -> rusqlite::Result<Vec<BdbEntry>> {
        let target = if prefix.is_empty() {
            word.to_string()
        } else {
            strip_proclitic(word, prefix).unwrap_or_else(|| word.to_string())
        };
        let cons = fold_consonants(&target);
        if cons.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = self.db.prepare(
            "SELECT word, root, gloss, content_json, pos, type FROM lexdb.bdb \
             WHERE cons = ?1 ORDER BY bdb_id",
        )?;
        let rows = stmt
            .query_map([&cons], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(5)?.as_deref() == Some("root"),
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        // Prefer the exact pointed headword (accents stripped on both sides, as
        // in `bdb_exact`); keep the whole consonant group only when none matches.
        let canonical = normalize_hebrew_combining(&strip_accents(&target));
        let has_exact = rows
            .iter()
            .any(|(w, ..)| normalize_hebrew_combining(&strip_accents(w)) == canonical);
        Ok(rows
            .into_iter()
            .filter(|(w, ..)| {
                !has_exact || normalize_hebrew_combining(&strip_accents(w)) == canonical
            })
            .map(|(word, root, gloss, content_json, pos, is_root)| BdbEntry {
                headword: normalize_hebrew_combining(&word),
                root,
                gloss,
                content_json,
                pos,
                is_root,
            })
            .filter(BdbEntry::has_content)
            .collect())
    }

    /// The glossed root tree for an OT word: every BDB lexeme sharing the
    /// consonantal root, each with its structured definition JSON. This is the
    /// OT analogue of [`Bible::sedra_root_tree`].
    pub fn hebrew_bdb_by_root(&self, root: &str) -> rusqlite::Result<Vec<BdbEntry>> {
        if root.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = self.db.prepare(
            "SELECT word, root, gloss, content_json, pos, type FROM lexdb.bdb \
             WHERE root = ?1 ORDER BY bdb_id",
        )?;
        let entries = stmt
            .query_map([root], |row| {
                Ok(BdbEntry {
                    headword: normalize_hebrew_combining(
                        row.get::<_, Option<String>>(0)?
                            .unwrap_or_default()
                            .as_str(),
                    ),
                    root: row.get(1)?,
                    gloss: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    content_json: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    pos: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    is_root: row.get::<_, Option<String>>(5)?.as_deref() == Some("root"),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        // Drop content-less section root-headers (empty gloss + no senses); they
        // would render as blank rows in the tree. See [`BdbEntry::has_content`].
        Ok(entries.into_iter().filter(BdbEntry::has_content).collect())
    }

    /// The single BDB lexeme with this entry id (`bdb.bdb_id`), or `None` if no
    /// row matches. Follows a Lexicon cross-reference: a `<w src>` span carries
    /// the target entry id, and the resolved entry's `root` drives the
    /// destination root tree the app navigates to.
    pub fn hebrew_bdb_by_id(&self, bdb_id: &str) -> rusqlite::Result<Option<BdbEntry>> {
        if bdb_id.is_empty() {
            return Ok(None);
        }
        self.db
            .query_row(
                "SELECT word, root, gloss, content_json, pos, type FROM lexdb.bdb \
                 WHERE bdb_id = ?1",
                [bdb_id],
                |row| {
                    Ok(BdbEntry {
                        headword: normalize_hebrew_combining(
                            row.get::<_, Option<String>>(0)?
                                .unwrap_or_default()
                                .as_str(),
                        ),
                        root: row.get(1)?,
                        gloss: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        content_json: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                        pos: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                        is_root: row.get::<_, Option<String>>(5)?.as_deref() == Some("root"),
                    })
                },
            )
            .optional()
    }

    /// The learner vocabulary: distinct Hebrew (non-Aramaic) surface forms in
    /// descending occurrence order, each bridged to a BDB gloss where
    /// possible. Resolution order per surface: the parse engine's best
    /// analysis ([`Bible::hebrew_word_info`]); an exact pointed-headword BDB
    /// match; the first glossed BDB lexeme sharing the consonant skeleton;
    /// the same lexicon lookups after stripping a leading vav conjunction.
    pub fn vocab(&self, limit: u32, offset: u32) -> rusqlite::Result<Vec<VocabEntry>> {
        let mut stmt = self.db.prepare(
            "SELECT text, occurrences, lexical_class FROM hebrewdb.surface \
             WHERE language IS NULL \
             ORDER BY occurrences DESC, surface_id \
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt
            .query_map([limit, offset], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows
            .into_iter()
            .map(|(surface, occurrences, lexical_class)| {
                let (root, gloss, morph) = self.vocab_resolve(&surface);
                VocabEntry {
                    surface,
                    occurrences,
                    lexical_class,
                    root,
                    gloss,
                    morph,
                }
            })
            .collect())
    }

    /// Best-effort `(root, gloss, morph)` for one vocabulary surface form.
    ///
    /// Citation-form lexicon matches are trusted over candidate parses, which
    /// otherwise read common singular nouns as spurious verb forms (מֶלֶךְ as
    /// "go!" rather than "king"); for the same reason a proclitic-stripped
    /// citation match (הַ + מֶלֶךְ) is tried before the parser too. The parser
    /// then covers genuinely inflected forms, and a pointing-blind consonant
    /// match is the last resort.
    fn vocab_resolve(&self, surface: &str) -> (String, String, String) {
        if let Some((root, gloss)) = curated_gloss(surface).or_else(|| bdb_exact(&self.db, surface))
        {
            return (root, gloss, String::new());
        }
        // One-letter proclitics (and/the/in/to/from/like) hide many frequent
        // forms from the lexicon; retry on the remainder. The pointing-blind
        // fallback needs three consonants left — short remainders (ךָ, נֵי)
        // match unrelated lexemes.
        for (proclitic, meaning) in PROCLITICS {
            if let Some(rest) = strip_proclitic(surface, proclitic) {
                let matched = curated_gloss(&rest)
                    .or_else(|| bdb_exact(&self.db, &rest))
                    .or_else(|| {
                        (fold_consonants(&rest).chars().count() >= 3)
                            .then(|| bdb_cons(&self.db, &rest))
                            .flatten()
                    });
                if let Some((root, gloss)) = matched {
                    return (root, gloss, format!("{proclitic}־ ({meaning}) + {rest}"));
                }
            }
        }
        if let Some(info) = self
            .hebrew_word_info(surface)
            .filter(|i| !i.gloss.is_empty())
        {
            let morph = morph_summary(&info);
            return (info.root, info.gloss, morph);
        }
        if let Some((root, gloss)) = bdb_cons(&self.db, surface) {
            return (root, gloss, String::new());
        }
        (String::new(), String::new(), String::new())
    }

    // The BDB lexicon bridge lives in free functions ([`lexicon_fallback`] and
    // friends) so the gen-hebrew build can precompute it against the same
    // `lexdb.bdb` schema with no `Bible` instance.

    /// OT verses where this exact surface form occurs.
    pub fn hebrew_surface_occurrences(&self, word: &str) -> rusqlite::Result<Vec<WordOccurrence>> {
        let norm = crate::generate::normalize_surface(word);
        let mut stmt = self.db.prepare(
            "SELECT o.book, o.chapter, o.verse FROM hebrewdb.occurrences o \
             JOIN hebrewdb.surface s ON s.surface_id = o.surface_id \
             WHERE s.text = ?1 ORDER BY o.book, o.chapter, o.verse",
        )?;
        stmt.query_map([&norm], |row| {
            Ok(WordOccurrence {
                book: row.get(0)?,
                chapter: row.get(1)?,
                verse: row.get(2)?,
            })
        })?
        .collect()
    }

    /// OT verses where any surface form of the given consonantal root occurs —
    /// both verb forms (root carried directly on the analysis) and noun forms
    /// (stem resolved to the same root via BDB).
    pub fn hebrew_root_occurrences(&self, root: &str) -> rusqlite::Result<Vec<WordOccurrence>> {
        if root.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = self.db.prepare(
            "SELECT DISTINCT o.book, o.chapter, o.verse FROM hebrewdb.occurrences o \
             WHERE o.surface_id IN ( \
                 SELECT a.surface_id FROM hebrewdb.analyses a WHERE a.root = ?1 \
                 UNION \
                 SELECT n.surface_id FROM hebrewdb.noun_analyses n \
                 JOIN lexdb.bdb b ON b.word = n.stem AND b.root = ?1 \
             ) \
             ORDER BY o.book, o.chapter, o.verse",
        )?;
        stmt.query_map([root], |row| {
            Ok(WordOccurrence {
                book: row.get(0)?,
                chapter: row.get(1)?,
                verse: row.get(2)?,
            })
        })?
        .collect()
    }

    /// OT root occurrences tagged with the surface form found in each verse, so
    /// the UI can filter the root's occurrences by inflected form. Same root
    /// matching as [`Bible::hebrew_root_occurrences`], but emits one row per
    /// (verse, surface form) instead of collapsing to distinct verses.
    pub fn hebrew_root_occurrences_detailed(
        &self,
        root: &str,
    ) -> rusqlite::Result<Vec<HebrewOccurrence>> {
        if root.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = self.db.prepare(
            "SELECT DISTINCT o.book, o.chapter, o.verse, s.text \
             FROM hebrewdb.occurrences o \
             JOIN hebrewdb.surface s ON s.surface_id = o.surface_id \
             WHERE o.surface_id IN ( \
                 SELECT a.surface_id FROM hebrewdb.analyses a WHERE a.root = ?1 \
                 UNION \
                 SELECT n.surface_id FROM hebrewdb.noun_analyses n \
                 JOIN lexdb.bdb b ON b.word = n.stem AND b.root = ?1 \
             ) \
             ORDER BY o.book, o.chapter, o.verse, s.text",
        )?;
        stmt.query_map([root], |row| {
            Ok(HebrewOccurrence {
                book: row.get(0)?,
                chapter: row.get(1)?,
                verse: row.get(2)?,
                form: row.get(3)?,
            })
        })?
        .collect()
    }

    /// Full SEDRA lexicon entry for an NT word. `vocalised` is the displayed
    /// Hebrew word (matched directly against `sedradb.words.strVocalised`,
    /// since the NT bible text is the same bijective transliteration). Returns
    /// one [`SedraWord`] per matching word form (homographs yield several).
    pub fn sedra_word_info(&self, vocalised: &str) -> rusqlite::Result<Vec<SedraWord>> {
        let mut stmt = self.db.prepare(
            "SELECT w.keyLexeme, l.keyRoot, w.strWord, w.strVocalised, l.strLexeme, r.strRoot, \
                    w.keyGender, w.keyPerson, w.keyNumber, w.keyState, w.keyTense, w.keyForm, \
                    w.keySuffixPerson, w.keySuffixGender, w.keySuffixNumber \
             FROM sedradb.words w \
             JOIN sedradb.lexemes l ON w.keyLexeme = l.keyLexeme \
             JOIN sedradb.roots r ON l.keyRoot = r.keyRoot \
             WHERE replace(replace(w.strVocalised, char(1471), ''), char(95), '') = ?1 \
             ORDER BY w.keyWord",
        )?;
        let key = crate::transliterate::lookup_key(vocalised);
        let mut words = stmt
            .query_map([key], |row| {
                Ok(SedraWord {
                    key_lexeme: row.get(0)?,
                    key_root: row.get(1)?,
                    consonantal: display(row.get::<_, String>(2)?),
                    word: display(row.get::<_, String>(3)?),
                    lexeme: display(row.get::<_, String>(4)?),
                    root: display(row.get::<_, String>(5)?),
                    gender: decode_gender(row.get(6)?),
                    person: decode_person(row.get(7)?),
                    number: decode_number(row.get(8)?),
                    state: decode_state(row.get(9)?),
                    tense: decode_tense(row.get(10)?),
                    form: decode_form(row.get(11)?),
                    suffix: decode_suffix(row.get(12)?, row.get(13)?, row.get(14)?),
                    meanings: Vec::new(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        for word in words.iter_mut() {
            word.meanings = self.sedra_meanings(word.key_lexeme)?;
        }

        Ok(words)
    }

    /// English glosses for a lexeme, each composed as `before meaning after`.
    fn sedra_meanings(&self, key_lexeme: i64) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self.db.prepare(
            "SELECT strBefore, strMeaning, strAfter FROM sedradb.english \
             WHERE keyLexeme = ?1 ORDER BY keyEnglish",
        )?;
        stmt.query_map([key_lexeme], |row| {
            let before: String = row.get(0)?;
            let meaning: String = row.get(1)?;
            let after: String = row.get(2)?;
            Ok([before, meaning, after]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" "))
        })?
        .collect()
    }

    /// All lexemes sharing a root, giving an overview of the root family.
    /// `current_key_lexeme` flags the looked-up word's own lexeme.
    pub fn sedra_root_tree(
        &self,
        key_root: i64,
        current_key_lexeme: i64,
    ) -> rusqlite::Result<Vec<SedraLexemeSummary>> {
        let mut stmt = self.db.prepare(
            "SELECT keyLexeme, strLexeme FROM sedradb.lexemes \
             WHERE keyRoot = ?1 ORDER BY keyLexeme",
        )?;
        let lexemes = stmt
            .query_map([key_root], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut tree = Vec::with_capacity(lexemes.len());
        for (key_lexeme, lexeme) in lexemes {
            tree.push(SedraLexemeSummary {
                lexeme: display(lexeme),
                meanings: self.sedra_meanings(key_lexeme)?,
                is_current: key_lexeme == current_key_lexeme,
            });
        }
        Ok(tree)
    }

    /// NT verses where any word form of the given lexeme occurs.
    pub fn sedra_lexeme_occurrences(
        &self,
        key_lexeme: i64,
    ) -> rusqlite::Result<Vec<WordOccurrence>> {
        let mut stmt = self.db.prepare(
            "SELECT DISTINCT o.book, o.chapter, o.verse FROM sedradb.occurrences o \
             JOIN sedradb.words w ON o.keyWord = w.keyWord \
             WHERE w.keyLexeme = ?1 ORDER BY o.book, o.chapter, o.verse",
        )?;
        stmt.query_map([key_lexeme], |row| {
            Ok(WordOccurrence {
                book: row.get(0)?,
                chapter: row.get(1)?,
                verse: row.get(2)?,
            })
        })?
        .collect()
    }

    /// NT verses where any lexeme of the given root occurs.
    pub fn sedra_root_occurrences(&self, key_root: i64) -> rusqlite::Result<Vec<WordOccurrence>> {
        let mut stmt = self.db.prepare(
            "SELECT DISTINCT o.book, o.chapter, o.verse FROM sedradb.occurrences o \
             JOIN sedradb.words w ON o.keyWord = w.keyWord \
             JOIN sedradb.lexemes l ON w.keyLexeme = l.keyLexeme \
             WHERE l.keyRoot = ?1 ORDER BY o.book, o.chapter, o.verse",
        )?;
        stmt.query_map([key_root], |row| {
            Ok(WordOccurrence {
                book: row.get(0)?,
                chapter: row.get(1)?,
                verse: row.get(2)?,
            })
        })?
        .collect()
    }

    /// OT (Hebrew Bible) occurrences of the same consonantal root as a SEDRA
    /// NT root, answered from `hebrewdb`/`lexdb` like
    /// [`Bible::hebrew_root_occurrences`]. The SEDRA root is rendered with
    /// medial letter forms, so its [`crate::transliterate::lookup_key`]
    /// matches the medial-form roots in those databases directly. Unlike the
    /// Hebrew lookup, the noun arm also accepts a consonantal-headword match
    /// (`bdb.cons`): SEDRA roots are often biliteral (יד, לב, הר) where BDB
    /// keys the noun under an empty or geminate root. OT books only, so these
    /// never duplicate the SEDRA-derived NT occurrences. Roots without a
    /// Hebrew cognate simply yield nothing.
    pub fn ot_root_occurrences(
        &self,
        sedra_key_root: i64,
    ) -> rusqlite::Result<Vec<WordOccurrence>> {
        let root: String = self.db.query_row(
            "SELECT strRoot FROM sedradb.roots WHERE keyRoot = ?1",
            [sedra_key_root],
            |row| row.get(0),
        )?;
        let key = crate::transliterate::lookup_key(&root);
        if key.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = self.db.prepare(
            "SELECT DISTINCT o.book, o.chapter, o.verse FROM hebrewdb.occurrences o \
             WHERE o.surface_id IN ( \
                 SELECT a.surface_id FROM hebrewdb.analyses a WHERE a.root = ?1 \
                 UNION \
                 SELECT n.surface_id FROM hebrewdb.noun_analyses n \
                 JOIN lexdb.bdb b ON b.word = n.stem \
                 WHERE b.root = ?1 OR b.cons = ?1 \
             ) \
             ORDER BY o.book, o.chapter, o.verse",
        )?;
        stmt.query_map([key], |row| {
            Ok(WordOccurrence {
                book: row.get(0)?,
                chapter: row.get(1)?,
                verse: row.get(2)?,
            })
        })?
        .collect()
    }

    /// NT occurrences of every lexeme of a root, each tagged with the lexeme's
    /// position in the root tree so the UI can filter by lexeme. `lexeme_index`
    /// matches the ordering of [`Bible::sedra_root_tree`] (lexemes ordered by
    /// `keyLexeme`). Adjacent rows for the same verse+lexeme are merged, with
    /// distinct word forms collected.
    pub fn sedra_root_occurrences_detailed(
        &self,
        key_root: i64,
    ) -> rusqlite::Result<Vec<SedraOccurrence>> {
        // Map keyLexeme -> index in keyLexeme order (same as sedra_root_tree).
        let mut idx_stmt = self.db.prepare(
            "SELECT keyLexeme FROM sedradb.lexemes WHERE keyRoot = ?1 ORDER BY keyLexeme",
        )?;
        let mut lexeme_index = HashMap::new();
        let keys = idx_stmt
            .query_map([key_root], |row| row.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (i, key) in keys.into_iter().enumerate() {
            lexeme_index.insert(key, i as u32);
        }

        let mut stmt = self.db.prepare(
            "SELECT o.book, o.chapter, o.verse, w.keyLexeme, w.strVocalised \
             FROM sedradb.occurrences o \
             JOIN sedradb.words w ON o.keyWord = w.keyWord \
             JOIN sedradb.lexemes l ON w.keyLexeme = l.keyLexeme \
             WHERE l.keyRoot = ?1 \
             ORDER BY o.book, o.chapter, o.verse, w.keyLexeme",
        )?;
        let rows = stmt
            .query_map([key_root], |row| {
                Ok((
                    row.get::<_, u8>(0)?,
                    row.get::<_, u8>(1)?,
                    row.get::<_, u8>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut out: Vec<SedraOccurrence> = Vec::new();
        for (book, chapter, verse, key_lexeme, word) in rows {
            let index = *lexeme_index.get(&key_lexeme).unwrap_or(&0);
            match out.last_mut() {
                Some(last)
                    if last.book == book
                        && last.chapter == chapter
                        && last.verse == verse
                        && last.lexeme_index == index =>
                {
                    if !last.words.contains(&word) {
                        last.words.push(word);
                    }
                }
                _ => out.push(SedraOccurrence {
                    book,
                    chapter,
                    verse,
                    lexeme_index: index,
                    words: vec![word],
                }),
            }
        }
        Ok(out)
    }

    /// Lexicon lookup for an NT word, backed by the `sedradb` lexicon. Returns
    /// one entry per (lexeme, meaning) pair across all matching word forms.
    pub fn sedra_lookup(&self, word: &str) -> rusqlite::Result<Vec<SedraEntry>> {
        let words = self.sedra_word_info(word)?;
        let mut entries = Vec::new();
        for w in &words {
            for meaning in &w.meanings {
                entries.push(SedraEntry {
                    lexeme: w.lexeme.clone(),
                    root: w.root.clone(),
                    meaning: meaning.clone(),
                });
            }
        }
        Ok(entries)
    }

    pub fn chapter_count(&self, book: u8) -> rusqlite::Result<u8> {
        self.db.query_row(
            "SELECT MAX(chapter) FROM bibledb.bible WHERE book = ?1",
            [book],
            |row| row.get(0),
        )
    }
}

/// `Bible::default()` only exists with the `embedded` feature, so its test
/// lives in its own module; run with `cargo test --features embedded`.
#[cfg(all(test, feature = "embedded"))]
mod embedded_tests {
    use super::*;

    #[test]
    fn test_embedded_database_open() {
        if Asset::get("bible.db").is_none() {
            eprintln!("skipping: data/*.db not embedded in this build");
            return;
        }
        let bible = Bible::default();
        assert!(bible.get(1, 1, 1).unwrap().starts_with('ב'));
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    /// The data/*.db files are generated locally (`db gen-*` / legacy Python
    /// pipeline) and not committed, so CI checkouts have an empty data/
    /// folder; skip the DB-backed tests in that case.
    macro_rules! require_data {
        () => {
            if !Path::new("data/bible.db").exists() {
                eprintln!("skipping: data/*.db not generated in this checkout");
                return;
            }
        };
    }

    #[test]
    fn test_database_open() {
        require_data!();
        let bible = Bible::open("data").unwrap();

        // One query per attached schema to prove every ATTACH succeeded.
        let ot = bible.get(1, 1, 1).unwrap();
        assert!(ot.starts_with('ב'));
        assert!(!bible.sedra_word_info("כּתָבָא").unwrap().is_empty());
        assert!(bible.hebrew_word_info("בָּרָא").is_some());
        assert!(!bible.hebrew_bdb_by_root("ברא").unwrap().is_empty());
    }

    #[test]
    fn test_get_reads_bible_table() {
        require_data!();
        let bible = Bible::open("data").unwrap();

        // OT (Genesis 1:1) comes from the UXLC source: 7 words, ends with sof
        // pasuq, first letter is bet.
        let ot = bible.get(1, 1, 1).unwrap();
        assert_eq!(ot.split(' ').count(), 7);
        assert!(ot.starts_with('ב'));
        assert!(ot.ends_with('׃'));

        // NT (Matthew 1:1, book 40) is SEDRA transliterated into Hebrew: 8
        // words, first word is כּתָבָא (kaf with dagesh).
        let matt = bible.get(40, 1, 1).unwrap();
        assert_eq!(matt.split(' ').count(), 8);
        assert!(matt.starts_with('כ'));
    }

    #[test]
    fn nt_hebrew_round_trips_through_syriac() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        let mut stmt = bible
            .db
            .prepare("SELECT words FROM bibledb.bible WHERE book >= 40")
            .unwrap();
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(rows.len(), 7958);
        for hebrew in rows {
            let syriac = crate::transliterate::hebrew_to_syriac(&hebrew);
            let back = crate::transliterate::syriac_to_hebrew(&syriac);
            assert_eq!(back, hebrew, "round trip failed for NT verse");
        }
    }

    #[test]
    fn test_chapter_count() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        assert_eq!(bible.chapter_count(1).unwrap(), 50); // Genesis has 50 chapters
    }

    #[test]
    fn test_sedra_word_info() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        // First word of Matthew 1:1 (NT) is כתבא "book/writing/Scripture".
        let matt = bible.get(40, 1, 1).unwrap();
        let first = matt.split(' ').next().unwrap();
        let info = bible.sedra_word_info(first).unwrap();
        assert!(!info.is_empty(), "no SEDRA match for {first}");
        assert!(!info[0].root.is_empty());
        assert!(!info[0].lexeme.is_empty());
        assert!(
            info.iter()
                .any(|w| w.meanings.iter().any(|m| m.contains("book"))),
            "expected a 'book' gloss"
        );
        // sedra_lookup flattens the same data into (lexeme, meaning) entries.
        let entries = bible.sedra_lookup(first).unwrap();
        assert!(!entries.is_empty());

        // Root tree: all lexemes of the root, with the current one flagged.
        let w = &info[0];
        let tree = bible.sedra_root_tree(w.key_root, w.key_lexeme).unwrap();
        assert!(tree.len() > 1, "root should have several lexemes");
        assert_eq!(tree.iter().filter(|l| l.is_current).count(), 1);

        // OT occurrences of the same root (כתב "write") come from the
        // hebrewdb/lexdb lookup, are all OT (<40), and never overlap the NT
        // SEDRA set.
        let ot_occ = bible.ot_root_occurrences(w.key_root).unwrap();
        assert!(!ot_occ.is_empty(), "expected OT occurrences for root כתב");
        assert!(ot_occ.iter().all(|o| o.book < 40));

        // Occurrences: lexeme is a subset of the root family, both non-empty.
        let lex_occ = bible.sedra_lexeme_occurrences(w.key_lexeme).unwrap();
        let root_occ = bible.sedra_root_occurrences(w.key_root).unwrap();
        assert!(!lex_occ.is_empty());
        assert!(root_occ.len() >= lex_occ.len());
        assert!(root_occ.iter().all(|o| o.book >= 40));

        // Detailed root occurrences: every row tags a valid lexeme index, all
        // are NT, and distinct verses match the flat root-occurrence count.
        let detailed = bible.sedra_root_occurrences_detailed(w.key_root).unwrap();
        assert!(!detailed.is_empty());
        assert!(detailed.iter().all(|o| o.book >= 40));
        assert!(
            detailed
                .iter()
                .all(|o| (o.lexeme_index as usize) < tree.len())
        );
        assert!(detailed.iter().all(|o| !o.words.is_empty()));
        let distinct_verses: std::collections::HashSet<_> = detailed
            .iter()
            .map(|o| (o.book, o.chapter, o.verse))
            .collect();
        assert_eq!(distinct_verses.len(), root_occ.len());
    }

    #[test]
    #[ignore]
    fn inspect_real_inflected_glosses() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        for surface in [
            "בָּרָא",   // Gen 1:1 "created"
            "וַיֹּאמֶר", // "and he said"
            "וַיַּרְא",  // "and he saw"
            "יִשְׁלַח",  // "he will send"
            "שְׁמַע",   // "hear!"
            "דְּבָרִים", // "words"
            "דְּבָרוֹ",  // "his word"
            "הַמֶּלֶךְ",  // "the king"
            "מְלָכִים", // "kings"
        ] {
            match bible.hebrew_word_info(surface) {
                Some(w) => eprintln!(
                    "{surface:14} [{}] -> {}",
                    morph_summary(&w),
                    inflected_gloss(&w)
                ),
                None => eprintln!("{surface:14} -> (no parse)"),
            }
        }
    }

    #[test]
    fn inflected_gloss_renders_forms_in_english() {
        let verb = |tense: &str, pgn: (&str, &str, &str), gloss: &str| HebrewWord {
            gloss: gloss.to_string(),
            form: Some("Qal".to_string()),
            tense: Some(tense.to_string()),
            person: (!pgn.0.is_empty()).then(|| pgn.0.to_string()),
            gender: (!pgn.1.is_empty()).then(|| pgn.1.to_string()),
            number: (!pgn.2.is_empty()).then(|| pgn.2.to_string()),
            ..Default::default()
        };

        // Perfect → past, with subject pronoun from PGN.
        assert_eq!(
            inflected_gloss(&verb("Perfect", ("Third", "Masculine", "Singular"), "say")),
            "he said"
        );
        // The first clause of a multi-part gloss is the sense used.
        assert_eq!(
            inflected_gloss(&verb(
                "Perfect",
                ("Third", "Feminine", "Singular"),
                "utter; say"
            )),
            "she uttered"
        );
        assert_eq!(
            inflected_gloss(&verb("Perfect", ("First", "Common", "Singular"), "keep")),
            "I kept"
        );
        // Wayyiqtol prepends "and"; regular -ed with silent e.
        assert_eq!(
            inflected_gloss(&verb(
                "Wayyiqtol",
                ("Third", "Masculine", "Singular"),
                "love"
            )),
            "and he loved"
        );
        // Imperfect → will + base; imperative → base!.
        assert_eq!(
            inflected_gloss(&verb(
                "Imperfect",
                ("Second", "Masculine", "Singular"),
                "send"
            )),
            "you will send"
        );
        assert_eq!(
            inflected_gloss(&verb(
                "Imperative",
                ("Second", "Masculine", "Singular"),
                "hear"
            )),
            "hear!"
        );
        // Infinitive → to + base; active participle → -ing.
        assert_eq!(
            inflected_gloss(&verb("Inf. Construct", ("", "", ""), "keep")),
            "to keep"
        );
        assert_eq!(
            inflected_gloss(&verb(
                "Participle (act.)",
                ("", "Masculine", "Singular"),
                "make"
            )),
            "making"
        );

        // Object suffix appends an object pronoun.
        let mut struck = verb("Wayyiqtol", ("Third", "Masculine", "Singular"), "smite");
        struck.obj_suffix = Some("3ms".to_string());
        assert_eq!(inflected_gloss(&struck), "and he smote him");

        // Nouns: plural, construct, possessive suffix, article, preposition.
        let noun = |number: Option<&str>, state: Option<&str>, gloss: &str| HebrewWord {
            gloss: gloss.to_string(),
            number: number.map(str::to_string),
            state: state.map(str::to_string),
            ..Default::default()
        };
        assert_eq!(
            inflected_gloss(&noun(Some("Plural"), Some("Absolute"), "king")),
            "kings"
        );
        assert_eq!(
            inflected_gloss(&noun(Some("Plural"), Some("Absolute"), "man")),
            "men"
        );
        assert_eq!(
            inflected_gloss(&noun(Some("Singular"), Some("Construct"), "word")),
            "word of"
        );
        assert_eq!(
            inflected_gloss(&noun(None, Some("Sg + 3ms"), "word")),
            "his word"
        );
        let mut the_king = noun(Some("Singular"), Some("Absolute"), "king");
        the_king.prefix = Some("הַ".to_string());
        assert_eq!(inflected_gloss(&the_king), "the king");

        // A gentilic gloss already leading with "the" doesn't get a second
        // article from the הַ prefix (הַכַּרְמְלִי is "the Carmelite", not
        // "the the Carmelite").
        let mut the_carmelite = noun(
            Some("Singular"),
            Some("Absolute"),
            "the Carmelite; the Carmelitess",
        );
        the_carmelite.prefix = Some("הַ".to_string());
        assert_eq!(inflected_gloss(&the_carmelite), "the Carmelite");

        // Function words / proper nouns pass through unchanged.
        let particle = HebrewWord {
            gloss: "that; because".to_string(),
            ..Default::default()
        };
        assert_eq!(inflected_gloss(&particle), "that; because");
    }

    #[test]
    fn form_distractors_contrasts_tense_for_participle_and_infinitive() {
        // Participles and infinitives have no person (and, for infinitives, no
        // gender/number either), so the person/gender/number contrast the verb
        // branch relies on can't fire for them — they must fall back to
        // contrasting tense instead of coming back empty.
        let verb = |tense: &str, pgn: (&str, &str, &str), gloss: &str| HebrewWord {
            gloss: gloss.to_string(),
            form: Some("Qal".to_string()),
            tense: Some(tense.to_string()),
            person: (!pgn.0.is_empty()).then(|| pgn.0.to_string()),
            gender: (!pgn.1.is_empty()).then(|| pgn.1.to_string()),
            number: (!pgn.2.is_empty()).then(|| pgn.2.to_string()),
            ..Default::default()
        };

        let participle = verb("Participle (act.)", ("", "Masculine", "Singular"), "say");
        let d = form_distractors(&participle);
        assert!(
            !d.is_empty(),
            "participle should get form distractors, got none"
        );
        assert!(
            !d.contains(&"saying".to_string()),
            "must not include its own gloss"
        );

        let infinitive = verb("Inf. Construct", ("", "", ""), "say");
        let d = form_distractors(&infinitive);
        assert!(
            !d.is_empty(),
            "infinitive should get form distractors, got none"
        );
        assert!(
            !d.contains(&"to say".to_string()),
            "must not include its own gloss"
        );
    }

    #[test]
    fn test_hebrew_word_info_verb() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        // בָּרָא "created" (Gen 1:1), root ברא — a strong III-aleph verb that
        // bridges directly to BDB.
        let info = bible.hebrew_word_info("בָּרָא").expect("verb should parse");
        assert_eq!(info.root, "ברא");
        assert!(info.gloss.to_lowercase().contains("create"));
        assert_eq!(info.tense.as_deref(), Some("Perfect"));
        assert_eq!(info.person.as_deref(), Some("Third"));

        // Root tree: glossed BDB lexemes of the root, with structured content.
        let tree = bible.hebrew_bdb_by_root(&info.root).unwrap();
        assert!(!tree.is_empty());
        assert!(tree.iter().all(|e| e.root == "ברא"));
        assert!(tree.iter().any(|e| !e.content_json.is_empty()));

        // Occurrences: this form is a subset of the whole root's occurrences.
        let form = bible.hebrew_surface_occurrences("בָּרָא").unwrap();
        let root = bible.hebrew_root_occurrences(&info.root).unwrap();
        assert!(!form.is_empty());
        assert!(root.len() >= form.len());
        assert!(root.iter().all(|o| o.book < 40));
    }

    #[test]
    fn test_hebrew_bdb_proper_noun_grouping() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        // Root שמע holds both common lexemes (שָׁמַע "hear") and a crowd of
        // proper names (שִׁמְעוֹן Simeon, שִׁמְעִי Shimei, …). The app splits the
        // tree on `is_proper_noun` to head the names off on their own.
        let tree = bible.hebrew_bdb_by_root("שמע").unwrap();
        let (common, proper): (Vec<_>, Vec<_>) = tree.iter().partition(|e| !e.is_proper_noun());
        // The verb "hear" lands in the common group; the name "Simeon" in the
        // proper group.
        assert!(common.iter().any(|e| e.gloss == "hear"));
        assert!(
            proper
                .iter()
                .any(|e| e.gloss.contains("second son of Jacob"))
        );
        // The marker drives the split, and `prep`/`pron` never read as proper.
        assert!(proper.iter().all(|e| e.pos.starts_with("n.pr")));
        assert!(common.iter().all(|e| !e.pos.starts_with("n.pr")));
    }

    #[test]
    fn test_hebrew_bdb_pos_category() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        let tree = bible.hebrew_bdb_by_root("אבה").unwrap();
        let cat = |id: &str| {
            tree.iter()
                .find(|e| e.gloss.starts_with(id) || e.headword == id)
                .map(BdbEntry::pos_category)
        };
        // The verb heads the "verb" group; the names group as "proper".
        assert_eq!(cat("be willing"), Some("verb"));
        assert_eq!(cat("my father is joy"), Some("proper")); // אֲבִיגַיִל
        // אבוגיל is a bare cross-reference ("see אֲבִיגַיִל"): it carries no pos of
        // its own but inherits the target's, so it groups with the proper names
        // rather than falling through to "other".
        let abugil = bible.hebrew_bdb_by_id("a.ae.bd").unwrap().unwrap();
        assert!(abugil.gloss.starts_with("see"));
        assert_eq!(abugil.pos_category(), "proper");
        // The pos-less "father" section header (type="root") is the root's
        // etymology, not a lexeme; it heads the "root" group.
        let header = bible.hebrew_bdb_by_id("a.ae.aa").unwrap().unwrap();
        assert!(header.is_root && header.pos.is_empty());
        assert_eq!(header.pos_category(), "root");
        // A root header that *does* carry a pos (the verb אָבָה) stays a verb.
        let verb = bible.hebrew_bdb_by_id("a.ad.aa").unwrap().unwrap();
        assert!(verb.is_root);
        assert_eq!(verb.pos_category(), "verb");
    }

    #[test]
    fn test_hebrew_bdb_xref_navigation() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        // נַחְנוּ (id n.cr.am) is a cross-reference stub "see אֲנַחְנוּ": its content
        // carries the target's entry id as an `xref` the app navigates to.
        let stub = bible
            .hebrew_bdb_by_id("n.cr.am")
            .unwrap()
            .expect("stub entry exists");
        assert!(stub.content_json.contains("\"xref\":\"a.ef.ac\""));

        // Following that id resolves to a real lexeme with a root, so the app
        // can land on the target's root tree.
        let target = bible
            .hebrew_bdb_by_id("a.ef.ac")
            .unwrap()
            .expect("xref target exists");
        assert!(!target.root.is_empty());
        assert!(!bible.hebrew_bdb_by_root(&target.root).unwrap().is_empty());

        // Empty id and unknown id resolve to nothing rather than erroring.
        assert!(bible.hebrew_bdb_by_id("").unwrap().is_none());
        assert!(bible.hebrew_bdb_by_id("no.such.id").unwrap().is_none());
    }

    #[test]
    fn test_hebrew_bdb_root_tree_hides_empty_section_headers() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        // The Hebrew root אבה ("be willing", header a.ae.aa) and the Biblical
        // Aramaic appendix section opener (xa.ac.aa, also headword אבה) share the
        // reduced root "אבה". The Aramaic header has no gloss and `{"senses":[]}`,
        // so it must not appear as a blank second row in the tree.
        let tree = bible.hebrew_bdb_by_root("אבה").unwrap();
        assert!(!tree.is_empty());
        assert!(
            tree.iter().all(BdbEntry::has_content),
            "root tree must not list content-less section headers"
        );
        // The empty stub stays reachable by id (one cross-reference targets it).
        let stub = bible
            .hebrew_bdb_by_id("xa.ac.aa")
            .unwrap()
            .expect("section header still resolvable by id");
        assert!(!stub.has_content());
    }

    #[test]
    fn test_hebrew_word_info_noun() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        // אֱלֹהִים "God" — a noun whose stem matches a BDB headword (root אלה).
        let info = bible.hebrew_word_info("אֱלֹהִים").expect("noun should parse");
        assert_eq!(info.root, "אלה");
        assert_eq!(info.gender.as_deref(), Some("Masculine"));
        let tree = bible.hebrew_bdb_by_root(&info.root).unwrap();
        assert!(!tree.is_empty());

        // הָאָרֶץ "the earth" — prefixed noun with a final-tsade stem (אֶרֶץ).
        // The pointed stem misses BDB's headword spelling, so the consonant
        // bridge (fold to medial ארצ) is what resolves it to root ארצ.
        let earth = bible.hebrew_word_info("הָאָרֶץ").expect("noun should parse");
        assert_eq!(earth.root, "ארצ");
        assert!(!bible.hebrew_bdb_by_root(&earth.root).unwrap().is_empty());
        assert!(
            !bible
                .hebrew_root_occurrences(&earth.root)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_hebrew_word_info_noun_homograph_curated() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        // סוּס "horse" — a noun analysis whose consonant group holds two BDB
        // homographs, with the rare bird ("swallow; swift") first by bdb_id
        // and both rows carrying the neighbouring article's root סוכ. The
        // noun bridge must take the curated horse entry, not the first row.
        let info = bible.hebrew_word_info("סוּס").expect("noun should parse");
        assert_eq!(info.gloss, "horse");
        assert_eq!(info.root, "סוס");
    }

    #[test]
    fn test_hebrew_word_info_function_word() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        // וְעַתָּה "and now" — a closed-class adverb with a surface row but no
        // generated verb/noun analysis (the prefilter strips its spurious verb
        // reading). The lexicon fallback strips the vav and bridges to BDB.
        let info = bible
            .hebrew_word_info("וְעַתָּה")
            .expect("function word should resolve via lexicon");
        assert!(info.gloss.to_lowercase().contains("now"));
        assert!(info.prefix.is_some());
        assert!(info.form.is_none());
        assert!(info.tense.is_none());
    }

    #[test]
    fn test_curated_gloss_overrides_homograph() {
        // The curated override pins the function-word sense for closed-class
        // words whose consonant skeleton collides with an unrelated lexeme,
        // ahead of any BDB lookup. כִּי "that/because" must not bridge to the
        // verb כוה "burn"; אֲשֶׁר "who/which" not to אשׁר "go straight".
        assert_eq!(
            curated_gloss("כִּי"),
            Some((String::new(), "that; because; for; when".to_string()))
        );
        let (_, asher) = curated_gloss("אֲשֶׁר").expect("relative particle is curated");
        assert!(asher.starts_with("who"));
        // Matching ignores cantillation, so an accented surface still resolves.
        assert!(curated_gloss("אֲשֶׁ\u{0596}ר").is_some());
        // An ordinary word is left for the BDB lookups.
        assert_eq!(curated_gloss("מֶלֶךְ"), None);
    }

    #[test]
    fn test_cross_reference_gloss() {
        // Stubs: a "see"/"under" keyword pointing at a Hebrew target, with or
        // without leading Hebrew citations.
        assert!(cross_reference_gloss("see עלה"));
        assert!(cross_reference_gloss("see sub I. כלל."));
        assert!(cross_reference_gloss("אֻלַי see אוּלַי"));
        assert!(cross_reference_gloss("עֵלָּא see עלה"));
        assert!(cross_reference_gloss("under אול"));
        assert!(cross_reference_gloss("חִיאֵל under חיה"));
        // Not stubs: the verb רָאָה glossed as bare "see", English senses of
        // "under", a Hebrew-citation-led real gloss, and a gloss that only
        // mentions a reference after real content.
        assert!(!cross_reference_gloss("see"));
        assert!(!cross_reference_gloss("seeing"));
        assert!(!cross_reference_gloss("the under part; underneath; below"));
        assert!(!cross_reference_gloss("עָ֑ל subst. height"));
        assert!(!cross_reference_gloss(
            "n.pr.loc. pass in Naphtali, see נקב."
        ));
    }

    #[test]
    fn test_root_stub_gloss() {
        // Root-header stubs: the whole gloss is one parenthetical remark.
        assert!(root_stub_gloss(
            "(√ of following; meaning dubious; compare Lag BN 55 Anm)."
        ));
        assert!(root_stub_gloss("(meaning unknown)."));
        assert!(root_stub_gloss("(= בקק)."));
        assert!(root_stub_gloss("(quadrilit. √ of following; see reff. below)"));
        // Real glosses that merely open with a parenthetical.
        assert!(!root_stub_gloss("(he)-ass"));
        assert!(!root_stub_gloss("(less oft. שַׁלֻּם) n.pr.m. king of N. Israel"));
        // Unbalanced paren (truncated source) may still hold a sense.
        assert!(!root_stub_gloss("(† אֱדֹם n.pr.m. Edom"));
        // Ordinary glosses.
        assert!(!root_stub_gloss("gold"));
        assert!(!root_stub_gloss("n.pr.m. (√ & meaning unknown) king of Gomorrah"));
    }

    #[test]
    fn test_cons_bridge_skips_root_header_stubs() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        // זהב: the root-header stub "(√ of following; meaning dubious…)"
        // precedes the real article "gold" in lexicon order; the noun bridge
        // must serve the article. This carded the stub on the זָהָב tutor word.
        let (root, gloss) = bible.hebrew_cons_root("זהב").expect("זהב bridges");
        assert_eq!(root, "זהב");
        assert!(gloss.starts_with("gold"), "got {gloss:?}");
        // A stub-only consonant group (לשכ holds just the root header) still
        // names its self-referential root, but with no gloss.
        let (root, gloss) = bible.hebrew_cons_root("לשכ").expect("לשכ names a root");
        assert_eq!(root, "לשכ");
        assert_eq!(gloss, "");
    }

    #[test]
    fn test_lexicon_fallback_skips_cross_reference_stubs() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        // עַל: BDB files the preposition under the עלה article, leaving a
        // "see עלה" stub first in the consonant group; the bridge must serve
        // a real gloss (the prepositional article), never the stub.
        let (_, gloss, _) = lexicon_fallback(bible.conn(), "עַל").expect("עַל bridges");
        assert!(gloss.starts_with("upon"), "got {gloss:?}");
        // גַּם: the stub "see גמם" precedes the real article "also; moreover".
        let (_, gloss, _) = lexicon_fallback(bible.conn(), "גַּם").expect("גַּם bridges");
        assert!(gloss.starts_with("also"), "got {gloss:?}");
    }

    #[test]
    fn test_hebrew_word_info_curated_function_word() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        // כִּי bridges through the precomputed lexical_analyses table; the
        // curated gloss must win over the homographic verb root כוה ("burn").
        let info = bible
            .hebrew_word_info("כִּי")
            .expect("כִּי should resolve via the lexicon bridge");
        assert!(info.gloss.contains("because"));
        assert!(!info.gloss.to_lowercase().contains("burn"));
    }

    #[test]
    fn test_hebrew_bdb_for_surface_function_word() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        // מִי ("who?") has an empty BDB root, so the by-root tree is empty but the
        // surface lookup finds the lexeme — and the exact-headword preference
        // excludes the homographic מַי ("waters") sharing the מ־י skeleton.
        let info = bible.hebrew_word_info("מִי").expect("מִי should bridge");
        assert!(info.root.is_empty());
        assert!(bible.hebrew_bdb_by_root(&info.root).unwrap().is_empty());

        let entries = bible
            .hebrew_bdb_for_surface(&info.word, info.prefix.as_deref().unwrap_or(""))
            .unwrap();
        assert!(
            !entries.is_empty(),
            "function word should have a lexicon entry"
        );
        assert!(entries.iter().any(|e| e.gloss.contains("who")));
        assert!(
            entries.iter().all(|e| !e.gloss.contains("waters")),
            "exact headword match must exclude מַי (waters)"
        );
        assert!(
            entries.iter().any(|e| !e.content_json.is_empty()),
            "the Lexicon tab needs definition content"
        );
    }
}
