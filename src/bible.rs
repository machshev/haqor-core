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
    /// Empty when the source entry carried none.
    pub pos: String,
}

impl BdbEntry {
    /// True when this lexeme is a proper noun — any BDB `n.pr.*` part of
    /// speech (names of people, places, peoples, deities). A root's proper
    /// names crowd out its actual semantic range, so the app lists them under
    /// a separate heading rather than inline with the common lexemes.
    pub fn is_proper_noun(&self) -> bool {
        self.pos.starts_with("n.pr")
    }
}

/// The analysis chosen to describe one OT (Hebrew Bible) surface form, drawn
/// from the `hebrewdb` reverse-parse engine output and bridged to BDB glosses
/// via the consonantal root. Verb readings carry binyan/tense/person-gender-
/// number; noun readings carry gender/number/state. `root` is the consonantal
/// root used to pull the glossed root tree from `lexdb.bdb`.
#[derive(Debug, Default)]
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
/// skeleton, in lexicon order.
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
    stmt.query_map([&cons], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?.unwrap_or_default(),
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })
    .ok()?
    .collect::<rusqlite::Result<Vec<_>>>()
    .ok()
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

        Bible { db }
    }
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
        Ok(Bible { db })
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
    pub fn hebrew_word_info(&self, word: &str) -> Option<HebrewWord> {
        let norm = crate::generate::normalize_surface(word);

        // Top verb analysis by stored rank (attestation, then generator order).
        // `analysis_id` is unique, so it alone determines the pick; `has_bdb` is
        // still selected for the verb-vs-noun decision below.
        let verb = self
            .db
            .query_row(
                "SELECT a.root, a.binyan, a.form, a.pgn, a.prefix, a.vav_consecutive, \
                        EXISTS(SELECT 1 FROM lexdb.bdb b WHERE b.root = a.root) AS has_bdb \
                 FROM hebrewdb.analyses a \
                 JOIN hebrewdb.surface s ON s.surface_id = a.surface_id \
                 WHERE s.text = ?1 \
                 ORDER BY a.analysis_id ASC \
                 LIMIT 1",
                [&norm],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)? != 0,
                        row.get::<_, i64>(6)? != 0,
                    ))
                },
            )
            .optional()
            .ok()?;

        // Candidate noun analyses, resolved to a BDB root by folding the stem to
        // bare medial consonants and matching `bdb.cons`. The first stem that
        // resolves wins; otherwise the first candidate is kept unresolved so the
        // morphology still shows even without a lexicon bridge.
        let noun_rows = {
            let mut stmt = self
                .db
                .prepare(
                    "SELECT n.kind, n.label, n.prefix, n.stem \
                     FROM hebrewdb.noun_analyses n \
                     JOIN hebrewdb.surface s ON s.surface_id = n.surface_id \
                     WHERE s.text = ?1 ORDER BY n.analysis_id ASC",
                )
                .ok()?;
            stmt.query_map([&norm], |row| {
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
                let resolved = self.hebrew_cons_root(&fold_consonants(&stem));
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

        let verb_resolves = verb.as_ref().is_some_and(|v| v.6);
        let noun_resolves = noun.as_ref().is_some_and(|n| !n.3.is_empty());

        // Prefer a BDB-resolvable verb; else a resolvable noun; else whatever
        // analysis exists (verb before noun).
        if let Some((root, binyan, tense, pgn, prefix, vav_con, _)) =
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
                 JOIN hebrewdb.surface s ON s.surface_id = la.surface_id \
                 WHERE s.text = ?1",
                [&norm],
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
            });
        }

        None
    }

    /// Resolve a folded consonant skeleton to a BDB `(root, gloss)` via the
    /// indexed `cons` column — the noun bridge. `None` when no lexeme matches.
    fn hebrew_cons_root(&self, cons: &str) -> Option<(String, String)> {
        if cons.is_empty() {
            return None;
        }
        self.db
            .query_row(
                "SELECT root, gloss FROM lexdb.bdb WHERE cons = ?1 ORDER BY bdb_id LIMIT 1",
                [cons],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    ))
                },
            )
            .optional()
            .ok()
            .flatten()
    }

    /// First non-empty BDB gloss for a consonantal root (the root's primary
    /// lexeme leads the section, so this is the headline meaning).
    fn hebrew_root_gloss(&self, root: &str) -> String {
        self.db
            .query_row(
                "SELECT gloss FROM lexdb.bdb \
                 WHERE root = ?1 AND gloss IS NOT NULL AND gloss <> '' \
                 ORDER BY bdb_id LIMIT 1",
                [root],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
            .unwrap_or_default()
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
            "SELECT word, root, gloss, content_json, pos FROM lexdb.bdb \
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
            .map(|(word, root, gloss, content_json, pos)| BdbEntry {
                headword: normalize_hebrew_combining(&word),
                root,
                gloss,
                content_json,
                pos,
            })
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
            "SELECT word, root, gloss, content_json, pos FROM lexdb.bdb \
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
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(entries)
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
                "SELECT word, root, gloss, content_json, pos FROM lexdb.bdb \
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
        if let Some((root, gloss)) = curated_gloss(surface).or_else(|| bdb_exact(&self.db, surface)) {
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
        let (common, proper): (Vec<_>, Vec<_>) =
            tree.iter().partition(|e| !e.is_proper_noun());
        // The verb "hear" lands in the common group; the name "Simeon" in the
        // proper group.
        assert!(common.iter().any(|e| e.gloss == "hear"));
        assert!(proper.iter().any(|e| e.gloss.contains("second son of Jacob")));
        // The marker drives the split, and `prep`/`pron` never read as proper.
        assert!(proper.iter().all(|e| e.pos.starts_with("n.pr")));
        assert!(common.iter().all(|e| !e.pos.starts_with("n.pr")));
    }

    #[test]
    fn test_hebrew_bdb_xref_navigation() {
        require_data!();
        let bible = Bible::open("data").unwrap();
        // נַחְנוּ (id n.cr.am) is a cross-reference stub "v. אֲנַחְנוּ": its content
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
        assert!(!entries.is_empty(), "function word should have a lexicon entry");
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
