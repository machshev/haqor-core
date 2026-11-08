//! Haqor's own lexicon: one JSON file per root in `data/lexicon/` (see
//! `data/lexicon/README.md` and ADR 7), read into typed structures and checked
//! so a malformed entry or an unsupported claim cannot ship.
//!
//! The checks are the schema's rules: every sense rests on scripture, every
//! cognate on a registered source, every doctrinal statement on scripture
//! alone; every reference names a verse of the text; every source is in
//! `sources.json`.
//!
//! The types live here, not in the generator, so that the build, the app's
//! review page and its reader all read one schema.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::Connection;
use serde::Deserialize;

/// A verse packed as the database packs it.
fn pack_ref(book: i64, chapter: i64, verse: i64) -> i64 {
    (book << 16) | (chapter << 8) | verse
}

/// The book abbreviations references are written with (`Gen`, `1Sa`, `Mat`),
/// in the English Bible's order.
const BOOKS: [&str; 66] = [
    "Gen", "Exo", "Lev", "Num", "Deu", "Jos", "Jdg", "Rut", "1Sa", "2Sa", "1Ki", "2Ki", "1Ch",
    "2Ch", "Ezr", "Neh", "Est", "Job", "Psa", "Pro", "Ecc", "Sng", "Isa", "Jer", "Lam", "Ezk",
    "Dan", "Hos", "Jol", "Amo", "Oba", "Jon", "Mic", "Nam", "Hab", "Zep", "Hag", "Zec", "Mal",
    "Mat", "Mrk", "Luk", "Jhn", "Act", "Rom", "1Co", "2Co", "Gal", "Eph", "Php", "Col", "1Th",
    "2Th", "1Ti", "2Ti", "Tit", "Phm", "Heb", "Jas", "1Pe", "2Pe", "1Jn", "2Jn", "3Jn", "Jud",
    "Rev",
];

/// Haqor's book numbers for the Old Testament books in [`BOOKS`]' order:
/// Haqor numbers them in the Tanakh's order.
const TANAKH_ORDER: [u8; 39] = [
    1, 2, 3, 4, 5, 6, 7, 31, 8, 9, 10, 11, 38, 39, 36, 37, 34, 29, 27, 28, 33, 30, 12, 13, 32, 14,
    35, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
];

/// A reference as entries write it (`Gen.2.7`): Haqor's book, chapter and
/// verse.
pub fn parse_reference(reference: &str) -> Option<(u8, u8, u8)> {
    let mut parts = reference.split('.');
    let abbreviation = parts.next()?;
    let index = BOOKS.iter().position(|b| *b == abbreviation)?;
    let chapter: u8 = parts.next()?.parse().ok()?;
    let verse: u8 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || chapter == 0 || verse == 0 {
        return None;
    }
    let book = if index < 39 {
        TANAKH_ORDER[index]
    } else {
        index as u8 + 1
    };
    Some((book, chapter, verse))
}

/// The lexicon's directory in the data directory.
pub const LEXICON_DIR: &str = "lexicon";

/// The registry of external works, in the lexicon directory.
pub const SOURCES: &str = "sources.json";

/// The stems a verb's `stems` may name.
pub const STEMS: &[&str] = &[
    "qal",
    "niphal",
    "piel",
    "pual",
    "hiphil",
    "hophal",
    "hithpael",
    "polel",
    "polal",
    "hithpolel",
    "poel",
    "poal",
    "hithpoel",
    "pilpel",
    "pilpal",
    "hithpalpel",
    "piel-passive",
    "hishtaphel",
    "tiphil",
    "nithpael",
];

/// The languages a cognate may be in (see the README).
pub const COGNATE_LANGS: &[&str] = &[
    "akk", "uga", "phn", "arc", "syc", "ar", "gez", "sem", "grc", "egy",
];

/// Languages outside the Semitic family, whose words can only be loans.
pub const LOAN_ONLY_LANGS: &[&str] = &["grc", "egy"];

/// The languages a word of scripture may be in besides Hebrew, the default:
/// Biblical Aramaic, and the Syriac of the Peshitta.
pub const WORD_LANGS: &[&str] = &["arc", "syc"];

/// The stems an Aramaic verb's `stems` may name, Biblical Aramaic's and the
/// Peshitta's.
pub const ARAMAIC_STEMS: &[&str] = &[
    "peal",
    "peil",
    "pael",
    "aphel",
    "haphel",
    "shaphel",
    "hophal",
    "hithpeel",
    "hithpaal",
    "ithpeel",
    "ithpaal",
    "hishtaphel",
    "ethpeel",
    "ethpaal",
    "ettaphal",
    "eshtaphal",
    "palpel",
    "ethpalpal",
];

/// The Hebrew letters that each stand for more than one older sound, with the
/// sounds a cognate may show behind them (see the README's "Sounds").
pub const MERGED_LETTERS: &[(&str, &[&str])] = &[
    ("ח", &["ḥ", "ḫ"]),
    ("ע", &["ʿ", "ġ"]),
    ("ש", &["š", "ṯ"]),
    ("ז", &["z", "ḏ"]),
    ("צ", &["ṣ", "ṣ\u{301}", "ṯ\u{323}"]),
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub root: String,
    #[serde(default)]
    pub homograph: Option<u8>,
    pub status: Status,
    /// The date the entry was signed off, while `status` is `reviewed`.
    #[serde(default)]
    pub reviewed: Option<String>,
    pub updated: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub gloss: String,
    pub summary: String,
    pub words: Vec<Word>,
    #[serde(default)]
    pub cognates: Vec<Cognate>,
    #[serde(default)]
    pub doctrine: Vec<Doctrine>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Draft,
    Reviewed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pos {
    Verb,
    Noun,
    Adjective,
    Adverb,
    Preposition,
    Conjunction,
    Particle,
    Pronoun,
    Numeral,
    Interjection,
    Name,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Gender {
    #[serde(rename = "m")]
    Masculine,
    #[serde(rename = "f")]
    Feminine,
    #[serde(rename = "mf")]
    Common,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Word {
    pub lemma: String,
    /// `arc` or `syc` for a word of scripture's Aramaic; Hebrew when absent.
    #[serde(default)]
    pub lang: Option<String>,
    /// The word's own root key where its spelling differs from the entry's
    /// (Aramaic דהב under Hebrew זהב).
    #[serde(default)]
    pub root: Option<String>,
    /// The BDB keys (`a.ea.ab`) whose tokens in the text are this word, as
    /// the brief lists them: how its uses in scripture are counted.
    #[serde(default)]
    pub bdb: Vec<String>,
    /// The entry's cognates whose sounds go with this word (see `Cognate::id`).
    #[serde(default)]
    pub cognate_ids: Vec<String>,
    pub pos: Pos,
    #[serde(default)]
    pub gender: Option<Gender>,
    pub gloss: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub stems: Vec<Stem>,
    #[serde(default)]
    pub senses: Vec<Sense>,
    #[serde(default)]
    pub greek: Vec<Greek>,
    #[serde(default)]
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stem {
    pub stem: String,
    pub gloss: String,
    #[serde(default)]
    pub cognate_ids: Vec<String>,
    pub senses: Vec<Sense>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sense {
    /// Kebab case, unique in the entry, so a sense of a sibling word can name
    /// it in `kin`.
    #[serde(default)]
    pub id: Option<String>,
    pub gloss: String,
    pub definition: String,
    pub evidence: Vec<Citation>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub cognate_ids: Vec<String>,
    /// The senses of the entry's other words (by `id`) that share this
    /// sense's meaning, so their evidence supports it too.
    #[serde(default)]
    pub kin: Vec<String>,
    #[serde(default)]
    pub senses: Vec<Sense>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Relation {
    Cognate,
    Possible,
    Loan,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cognate {
    /// Kebab case, unique in the entry, so a word, stem or sense can name the
    /// cognates that go with it in `cognate_ids`.
    #[serde(default)]
    pub id: Option<String>,
    pub lang: String,
    /// For a letter of the root that stands for more than one older sound
    /// (`MERGED_LETTERS`), the sound this cognate shows behind it: `{"ע": "ġ"}`.
    #[serde(default)]
    pub sounds: BTreeMap<String, String>,
    pub form: String,
    #[serde(default)]
    pub translit: String,
    pub gloss: String,
    pub relation: Relation,
    #[serde(default)]
    pub note: String,
    pub evidence: Vec<Citation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Greek {
    pub lemma: String,
    #[serde(default)]
    pub gloss: String,
    /// Whether the Greek word renders the Hebrew (the Septuagint's bridge, no
    /// evidence of meaning) or is the Hebrew word itself, borrowed.
    #[serde(default)]
    pub relation: GreekRelation,
    pub evidence: Vec<Citation>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    #[serde(default)]
    pub parallels: Vec<Related>,
    #[serde(default)]
    pub opposites: Vec<Related>,
    #[serde(default)]
    pub phrases: Vec<Phrase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Related {
    pub lemma: String,
    #[serde(default)]
    pub note: String,
    pub evidence: Vec<Citation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Phrase {
    pub phrase: String,
    pub gloss: String,
    pub evidence: Vec<Citation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Doctrine {
    pub topic: String,
    pub statement: String,
    pub evidence: Vec<Citation>,
}

/// How a word's Greek is related to it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GreekRelation {
    /// The Greek the Septuagint renders the word by: the bridge to the New
    /// Testament's vocabulary, no evidence of the Hebrew's meaning.
    #[default]
    Rendering,
    /// The Hebrew or Aramaic word borrowed into Greek (ἀρραβών from
    /// עֵרָבוֹן): the New Testament's use of it is scripture's use of the word.
    Loan,
}

/// A citation: a verse of scripture or a place in a registered source.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Citation {
    Scripture(ScriptureCitation),
    Source(SourceCitation),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptureCitation {
    #[serde(rename = "ref")]
    pub reference: String,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub role: Role,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Defining,
    #[default]
    Example,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCitation {
    pub source: String,
    pub locator: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourcesFile {
    sources: Vec<Source>,
}

/// An external work citations may name.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub citation: String,
    pub lang: String,
    pub licence: String,
    pub url: String,
    #[serde(default)]
    pub local: String,
}

/// The lexicon as read from its directory.
#[derive(Debug)]
pub struct Lexicon {
    pub sources: Vec<Source>,
    pub entries: Vec<Entry>,
}

/// What a check found, for the log and the tests.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LexiconSummary {
    pub entries: usize,
    pub reviewed: usize,
    pub words: usize,
    pub senses: usize,
    pub citations: usize,
}

/// Read every entry and the source registry from `dir`, entries in file name
/// order. Fails on JSON that does not fit the schema, naming the file.
pub fn read_lexicon(dir: &Path) -> Result<Lexicon> {
    let sources_path = dir.join(SOURCES);
    let raw = std::fs::read_to_string(&sources_path)
        .with_context(|| format!("reading {}", sources_path.display()))?;
    let sources = serde_json::from_str::<SourcesFile>(&raw)
        .with_context(|| format!("parsing {}", sources_path.display()))?
        .sources;
    let mut paths = Vec::new();
    for dirent in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = dirent?.path();
        if path.extension().is_some_and(|e| e == "json")
            && path.file_name() != Some(SOURCES.as_ref())
        {
            paths.push(path);
        }
    }
    paths.sort();
    let mut entries = Vec::new();
    for path in paths {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let entry: Entry =
            serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if stem != entry.id {
            bail!(
                "{} holds entry `{}`: the file must be named by its id",
                path.display(),
                entry.id
            );
        }
        entries.push(entry);
    }
    Ok(Lexicon { sources, entries })
}

/// Every entry file's id and text as written, in file name order: what the
/// review page shows and edits.
pub fn read_entry_texts(dir: &Path) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    let mut paths = Vec::new();
    for dirent in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = dirent?.path();
        if path.extension().is_some_and(|e| e == "json")
            && path.file_name() != Some(SOURCES.as_ref())
        {
            paths.push(path);
        }
    }
    paths.sort();
    for path in paths {
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        out.push((id, text));
    }
    Ok(out)
}

/// The scripture references an entry's text cites (`ref` and `to`), each once
/// with the verse it names, in the Bible's order. A reference that names no
/// book is left out; the checks report it.
pub fn references_in(text: &str) -> Vec<(String, (u8, u8, u8))> {
    fn walk(value: &serde_json::Value, out: &mut Vec<(String, (u8, u8, u8))>) {
        match value {
            serde_json::Value::Object(fields) => {
                for key in ["ref", "to"] {
                    if let Some(reference) = fields.get(key).and_then(|v| v.as_str())
                        && !out.iter().any(|(r, _)| r == reference)
                        && let Some(verse) = parse_reference(reference)
                    {
                        out.push((reference.to_string(), verse));
                    }
                }
                fields.values().for_each(|v| walk(v, out));
            }
            serde_json::Value::Array(items) => items.iter().for_each(|v| walk(v, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        walk(&value, &mut out);
    }
    out.sort_by_key(|(_, (book, chapter, verse))| (*book, *chapter, *verse));
    out
}

/// Save one entry's text into the lexicon in `dir`, replacing the entry of
/// the same id, after checking the whole lexicon with it in place against the
/// text's `verses`. Nothing is written when it fails. The text is written as
/// given, so the editor keeps the file's order and layout.
pub fn save_entry(dir: &Path, text: &str, verses: &HashSet<i64>) -> Result<LexiconSummary> {
    let entry: Entry = serde_json::from_str(text).context("the entry does not fit the schema")?;
    let id = entry.id.clone();
    let mut lexicon = read_lexicon(dir)?;
    lexicon.entries.retain(|e| e.id != id);
    lexicon.entries.push(entry);
    let summary = check_lexicon(&lexicon, verses, None)?;
    let path = dir.join(format!("{id}.json"));
    let temporary = dir.join(format!(".{id}.json.{}.tmp", std::process::id()));
    let mut text = text.trim_end().to_string();
    text.push('\n');
    std::fs::write(&temporary, &text)
        .with_context(|| format!("writing {}", temporary.display()))?;
    if let Err(error) = std::fs::rename(&temporary, &path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error).with_context(|| format!("replacing {}", path.display()));
    }
    Ok(summary)
}

/// Every verse of the text, packed, from a runtime database's `verse` table.
pub fn verses_of(db: &Connection) -> Result<HashSet<i64>> {
    let mut stmt = db.prepare("SELECT ref FROM verse")?;
    let verses = stmt
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(verses)
}

fn is_kebab(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// A root key as the database files roots: consonants, finals folded, a sin
/// kept apart from a bare shin.
fn is_root(s: &str) -> bool {
    is_hebrew(s) && crate::bible::fold_consonants(s) == s
}

fn is_hebrew(s: &str) -> bool {
    s.chars().any(|c| ('\u{05D0}'..='\u{05EA}').contains(&c))
}

/// A root key's letters, each with the marks that follow it (sin is `שׂ`).
fn root_letters(root: &str) -> Vec<String> {
    let mut letters: Vec<String> = Vec::new();
    for c in root.chars() {
        match letters.last_mut() {
            Some(last) if !('\u{05D0}'..='\u{05EA}').contains(&c) => last.push(c),
            _ => letters.push(c.to_string()),
        }
    }
    letters
}

/// Collects every fault with the path to it, so one run reports them all.
struct Checker<'a> {
    verses: &'a HashSet<i64>,
    sources: HashSet<&'a str>,
    /// The `id`s of the cognates of the entry being checked.
    cognates: HashSet<String>,
    /// The `id`s of the entry's senses, with the index of the word each is in.
    sense_ids: HashMap<String, usize>,
    /// The index of the word being checked.
    word: usize,
    faults: Vec<String>,
    summary: LexiconSummary,
}

/// Which kinds of citation a list accepts.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Accepts {
    Scripture,
    Source,
    Either,
}

impl Checker<'_> {
    fn fault(&mut self, at: &str, what: impl std::fmt::Display) {
        self.faults.push(format!("{at}: {what}"));
    }

    fn text(&mut self, at: &str, field: &str, value: &str) {
        if value.trim().is_empty() {
            self.fault(at, format_args!("`{field}` must not be empty"));
        }
    }

    fn tags(&mut self, at: &str, tags: &[String]) {
        for tag in tags {
            if !is_kebab(tag) {
                self.fault(at, format_args!("tag `{tag}` must be kebab case"));
            }
        }
    }

    fn verse(&mut self, at: &str, reference: &str) -> Option<i64> {
        let Some((book, chapter, verse)) = parse_reference(reference) else {
            self.fault(at, format_args!("unreadable reference `{reference}`"));
            return None;
        };
        let packed = pack_ref(book.into(), chapter.into(), verse.into());
        if !self.verses.contains(&packed) {
            self.fault(at, format_args!("`{reference}` is no verse of the text"));
            return None;
        }
        Some(packed)
    }

    fn evidence(&mut self, at: &str, evidence: &[Citation], accepts: Accepts) {
        if evidence.is_empty() {
            self.fault(at, "`evidence` must cite at least one");
        }
        for (i, citation) in evidence.iter().enumerate() {
            let at = format!("{at}.evidence[{i}]");
            self.summary.citations += 1;
            match citation {
                Citation::Scripture(c) => {
                    if accepts == Accepts::Source {
                        self.fault(&at, "must cite a source, not scripture");
                    }
                    let first = self.verse(&at, &c.reference);
                    if let Some(to) = &c.to {
                        let last = self.verse(&at, to);
                        if let (Some(first), Some(last)) = (first, last)
                            && last <= first
                        {
                            self.fault(&at, format_args!("`to` {to} must follow {}", c.reference));
                        }
                    }
                }
                Citation::Source(c) => {
                    if accepts == Accepts::Scripture {
                        self.fault(&at, "must cite scripture only");
                    }
                    if !self.sources.contains(c.source.as_str()) {
                        self.fault(
                            &at,
                            format_args!("source `{}` is not in {SOURCES}", c.source),
                        );
                    }
                    self.text(&at, "locator", &c.locator);
                }
            }
        }
    }

    fn senses(&mut self, at: &str, senses: &[Sense]) {
        for (i, sense) in senses.iter().enumerate() {
            let at = format!("{at}.senses[{i}]");
            self.summary.senses += 1;
            self.text(&at, "gloss", &sense.gloss);
            self.text(&at, "definition", &sense.definition);
            self.tags(&at, &sense.tags);
            self.cognate_ids(&at, &sense.cognate_ids);
            for kin in &sense.kin {
                match self.sense_ids.get(kin) {
                    None => self.fault(&at, format_args!("`kin` names no sense `{kin}` of the entry")),
                    Some(&word) if word == self.word => self.fault(
                        &at,
                        format_args!("`kin` `{kin}` is a sense of the same word: kin are a sibling word's senses"),
                    ),
                    Some(_) => {}
                }
            }
            self.evidence(&at, &sense.evidence, Accepts::Scripture);
            self.senses(&at, &sense.senses);
        }
    }

    fn cognate_ids(&mut self, at: &str, ids: &[String]) {
        for id in ids {
            if !self.cognates.contains(id) {
                self.fault(at, format_args!("names no cognate `{id}` of the entry"));
            }
        }
    }

    fn sounds(&mut self, at: &str, root: &str, sounds: &BTreeMap<String, String>) {
        let letters = root_letters(root);
        for (letter, sound) in sounds {
            let Some((_, allowed)) = MERGED_LETTERS.iter().find(|(l, _)| l == letter) else {
                self.fault(at, format_args!("`{letter}` stands for only one sound"));
                continue;
            };
            if !letters.iter().any(|l| l == letter) {
                self.fault(
                    at,
                    format_args!("`{letter}` is no letter of the root `{root}`"),
                );
            }
            if !allowed.contains(&sound.as_str()) {
                self.fault(
                    at,
                    format_args!(
                        "`{letter}` stands for {}, not `{sound}`",
                        allowed.join(" or ")
                    ),
                );
            }
        }
    }

    fn word(&mut self, at: &str, word: &Word) {
        self.summary.words += 1;
        if !is_hebrew(&word.lemma) {
            self.fault(at, format_args!("lemma `{}` must be Hebrew", word.lemma));
        }
        let aramaic = match word.lang.as_deref() {
            None => false,
            Some(lang) if WORD_LANGS.contains(&lang) => true,
            Some(lang) => {
                self.fault(at, format_args!("unknown language `{lang}`"));
                false
            }
        };
        if let Some(root) = &word.root
            && !is_root(root)
        {
            self.fault(at, format_args!("root `{root}` must be a root key"));
        }
        self.cognate_ids(at, &word.cognate_ids);
        self.text(at, "gloss", &word.gloss);
        if word.pos == Pos::Verb {
            if word.stems.is_empty() || !word.senses.is_empty() {
                self.fault(at, "a verb has `stems` and no `senses`");
            }
        } else if word.senses.is_empty() || !word.stems.is_empty() {
            self.fault(at, "a word other than a verb has `senses` and no `stems`");
        }
        let mut seen = HashSet::new();
        for (i, stem) in word.stems.iter().enumerate() {
            let at = format!("{at}.stems[{i}]");
            let stems = if aramaic { ARAMAIC_STEMS } else { STEMS };
            if !stems.contains(&stem.stem.as_str()) {
                self.fault(&at, format_args!("unknown stem `{}`", stem.stem));
            }
            if !seen.insert(stem.stem.as_str()) {
                self.fault(&at, format_args!("stem `{}` listed twice", stem.stem));
            }
            self.text(&at, "gloss", &stem.gloss);
            self.cognate_ids(&at, &stem.cognate_ids);
            if stem.senses.is_empty() {
                self.fault(&at, "a stem needs at least one sense");
            }
            self.senses(&at, &stem.senses);
        }
        self.senses(at, &word.senses);
        for (i, greek) in word.greek.iter().enumerate() {
            let at = format!("{at}.greek[{i}]");
            self.text(&at, "lemma", &greek.lemma);
            self.evidence(&at, &greek.evidence, Accepts::Either);
            if greek.relation == GreekRelation::Loan
                && !greek
                    .evidence
                    .iter()
                    .any(|c| matches!(c, Citation::Scripture(_)))
            {
                self.fault(&at, "a loan cites the New Testament verses that use it");
            }
        }
        for (field, related) in [
            ("parallels", &word.usage.parallels),
            ("opposites", &word.usage.opposites),
        ] {
            for (i, related) in related.iter().enumerate() {
                let at = format!("{at}.usage.{field}[{i}]");
                self.text(&at, "lemma", &related.lemma);
                self.evidence(&at, &related.evidence, Accepts::Scripture);
            }
        }
        for (i, phrase) in word.usage.phrases.iter().enumerate() {
            let at = format!("{at}.usage.phrases[{i}]");
            self.text(&at, "phrase", &phrase.phrase);
            self.text(&at, "gloss", &phrase.gloss);
            self.evidence(&at, &phrase.evidence, Accepts::Scripture);
        }
    }

    fn entry(&mut self, entry: &Entry) {
        let at = format!("{}.json", entry.id);
        self.summary.entries += 1;
        if entry.status == Status::Reviewed {
            self.summary.reviewed += 1;
        }
        if !is_kebab(&entry.id) {
            self.fault(&at, "`id` must be kebab case");
        }
        if !is_root(&entry.root) {
            self.fault(
                &at,
                format_args!(
                    "root `{}` must be a root key (consonants, no finals, sin as שׂ, shin bare)",
                    entry.root
                ),
            );
        }
        if entry.homograph == Some(0) {
            self.fault(&at, "`homograph` counts from 1");
        }
        if !is_date(&entry.updated) {
            self.fault(&at, "`updated` must be YYYY-MM-DD");
        }
        match (entry.status, &entry.reviewed) {
            (Status::Reviewed, None) => {
                self.fault(
                    &at,
                    "a `reviewed` entry needs its sign-off date in `reviewed`",
                );
            }
            (Status::Draft, Some(_)) => {
                self.fault(&at, "a `draft` entry has no `reviewed` date");
            }
            (_, Some(date)) if !is_date(date) => {
                self.fault(&at, "`reviewed` must be YYYY-MM-DD");
            }
            _ => {}
        }
        self.tags(&at, &entry.tags);
        self.text(&at, "gloss", &entry.gloss);
        self.text(&at, "summary", &entry.summary);
        if entry.words.is_empty() {
            self.fault(&at, "an entry needs at least one word");
        }
        self.cognates.clear();
        for (i, cognate) in entry.cognates.iter().enumerate() {
            let at = format!("{at}.cognates[{i}]");
            if let Some(id) = &cognate.id {
                if !is_kebab(id) {
                    self.fault(&at, "`id` must be kebab case");
                }
                if !self.cognates.insert(id.clone()) {
                    self.fault(&at, format_args!("cognate `{id}` listed twice"));
                }
            }
            if !COGNATE_LANGS.contains(&cognate.lang.as_str()) {
                self.fault(&at, format_args!("unknown language `{}`", cognate.lang));
            }
            if LOAN_ONLY_LANGS.contains(&cognate.lang.as_str())
                && cognate.relation != Relation::Loan
            {
                self.fault(
                    &at,
                    format_args!(
                        "`{}` is not a Semitic language: its words can only be loans",
                        cognate.lang
                    ),
                );
            }
            self.text(&at, "form", &cognate.form);
            self.text(&at, "gloss", &cognate.gloss);
            self.sounds(&at, &entry.root, &cognate.sounds);
            // Its own language's dictionary. Scripture's own Aramaic is a word of
            // the entry, not a cognate.
            self.evidence(&at, &cognate.evidence, Accepts::Source);
        }
        self.sense_ids.clear();
        for (i, word) in entry.words.iter().enumerate() {
            let mut ids = Vec::new();
            sense_ids_of(word, &mut ids);
            for id in ids {
                if !is_kebab(id) {
                    self.fault(
                        &format!("{at}.words[{i}]"),
                        format_args!("sense `id` `{id}` must be kebab case"),
                    );
                }
                if self.sense_ids.insert(id.to_string(), i).is_some() {
                    self.fault(
                        &format!("{at}.words[{i}]"),
                        format_args!("sense `id` `{id}` is used twice"),
                    );
                }
            }
        }
        for (i, word) in entry.words.iter().enumerate() {
            self.word = i;
            self.word(&format!("{at}.words[{i}]"), word);
        }
        for (i, doctrine) in entry.doctrine.iter().enumerate() {
            let at = format!("{at}.doctrine[{i}]");
            if !is_kebab(&doctrine.topic) {
                self.fault(&at, "`topic` must be kebab case");
            }
            self.text(&at, "statement", &doctrine.statement);
            self.evidence(&at, &doctrine.evidence, Accepts::Scripture);
        }
    }
}

/// Check the lexicon against the text's `verses` and, where `src_texts` is
/// given, the copies of its sources kept there. Fails listing every fault.
pub fn check_lexicon(
    lexicon: &Lexicon,
    verses: &HashSet<i64>,
    src_texts: Option<&Path>,
) -> Result<LexiconSummary> {
    let mut checker = Checker {
        verses,
        sources: HashSet::new(),
        cognates: HashSet::new(),
        sense_ids: HashMap::new(),
        word: 0,
        faults: Vec::new(),
        summary: LexiconSummary::default(),
    };
    let mut ids = HashSet::new();
    for source in &lexicon.sources {
        let at = format!("{SOURCES} `{}`", source.id);
        if !is_kebab(&source.id) {
            checker.fault(&at, "`id` must be kebab case");
        }
        if !ids.insert(source.id.as_str()) {
            checker.fault(&at, "listed twice");
        }
        checker.text(&at, "citation", &source.citation);
        checker.text(&at, "licence", &source.licence);
        if let Some(src_texts) = src_texts
            && !source.local.is_empty()
            && !src_texts.join(&source.local).exists()
        {
            checker.fault(&at, format_args!("no copy at src_texts/{}", source.local));
        }
    }
    checker.sources = ids;
    let mut roots = HashSet::new();
    for entry in &lexicon.entries {
        if !roots.insert((entry.root.as_str(), entry.homograph)) {
            checker.fault(
                &format!("{}.json", entry.id),
                format_args!(
                    "root `{}` has another entry; number them with `homograph`",
                    entry.root
                ),
            );
        }
        checker.entry(entry);
    }
    if !checker.faults.is_empty() {
        bail!(
            "the lexicon has {} faults:\n  {}",
            checker.faults.len(),
            checker.faults.join("\n  ")
        );
    }
    Ok(checker.summary)
}

/// `db check-lexicon`: read and check the lexicon in `dir` against the text
/// of the runtime database at `db`.
pub fn check_lexicon_dir(dir: &Path, db: &Path, src_texts: &Path) -> Result<LexiconSummary> {
    let lexicon = read_lexicon(dir)?;
    let db = Connection::open(db).with_context(|| format!("opening {}", db.display()))?;
    let summary = check_lexicon(&lexicon, &verses_of(&db)?, Some(src_texts))?;
    let unknown = unknown_bdb_keys(&db, "", &lexicon)?;
    if !unknown.is_empty() {
        bail!(
            "the lexicon has {} faults:\n  {}",
            unknown.len(),
            unknown.join("\n  ")
        );
    }
    Ok(summary)
}

/// Where cognates show which older sound stands behind a merged letter of an
/// entry's root, the senses that go with each sound: one line per sense.
#[derive(Debug, PartialEq, Eq)]
pub struct SoundSplit {
    pub entry: String,
    pub root: String,
    pub letter: String,
    /// Each sound the entry's senses show behind `letter`, with the senses
    /// (`lemma stem: gloss`) that go with it.
    pub sounds: BTreeMap<String, Vec<String>>,
}

/// Every entry's sounds behind each merged letter of its root, as its words,
/// stems and senses name them through `cognate_ids` (a sense inherits its
/// word's and stem's). An entry with two sounds behind one letter is a split.
pub fn sound_splits(lexicon: &Lexicon) -> Vec<SoundSplit> {
    fn walk<'a>(
        label: &str,
        senses: &'a [Sense],
        inherited: &[&'a str],
        out: &mut Vec<(String, Vec<&'a str>)>,
    ) {
        for sense in senses {
            let mut ids = inherited.to_vec();
            ids.extend(sense.cognate_ids.iter().map(String::as_str));
            out.push((format!("{label}: {}", sense.gloss), ids.clone()));
            walk(label, &sense.senses, &ids, out);
        }
    }
    let mut splits = Vec::new();
    for entry in &lexicon.entries {
        let sounds_of: BTreeMap<&str, &BTreeMap<String, String>> = entry
            .cognates
            .iter()
            .filter_map(|c| Some((c.id.as_deref()?, &c.sounds)))
            .collect();
        let mut senses = Vec::new();
        for word in &entry.words {
            let ids: Vec<&str> = word.cognate_ids.iter().map(String::as_str).collect();
            walk(&word.lemma, &word.senses, &ids, &mut senses);
            for stem in &word.stems {
                let mut ids = ids.clone();
                ids.extend(stem.cognate_ids.iter().map(String::as_str));
                walk(
                    &format!("{} {}", word.lemma, stem.stem),
                    &stem.senses,
                    &ids,
                    &mut senses,
                );
            }
        }
        let mut by_letter: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
        for (label, ids) in &senses {
            let mut seen = BTreeSet::new();
            for sounds in ids.iter().filter_map(|id| sounds_of.get(id)) {
                for (letter, sound) in sounds.iter() {
                    if seen.insert((letter, sound)) {
                        by_letter
                            .entry(letter.clone())
                            .or_default()
                            .entry(sound.clone())
                            .or_default()
                            .push(label.clone());
                    }
                }
            }
        }
        for (letter, sounds) in by_letter {
            splits.push(SoundSplit {
                entry: entry.id.clone(),
                root: entry.root.clone(),
                letter,
                sounds,
            });
        }
    }
    splits
}

/// [`sound_splits`] as Markdown, grouped by letter: the splits first, then the
/// entries whose senses all show one sound.
pub fn sound_splits_report(lexicon: &Lexicon) -> String {
    use std::fmt::Write;
    let splits = sound_splits(lexicon);
    let mut out = String::new();
    for (letter, sounds) in MERGED_LETTERS {
        let here: Vec<_> = splits.iter().filter(|s| s.letter == *letter).collect();
        if here.is_empty() {
            continue;
        }
        let _ = writeln!(out, "## {letter} ({})\n", sounds.join(" / "));
        let (split, single): (Vec<&SoundSplit>, Vec<&SoundSplit>) =
            here.iter().copied().partition(|s| s.sounds.len() > 1);
        for s in split.iter().chain(&single) {
            let kind = if s.sounds.len() > 1 {
                "split"
            } else {
                "one sound"
            };
            let _ = writeln!(out, "- **{}** `{}` ({kind})", s.root, s.entry);
            for (sound, senses) in &s.sounds {
                let _ = writeln!(out, "  - {sound}: {}", senses.join("; "));
            }
        }
        out.push('\n');
    }
    if out.is_empty() {
        out.push_str("No entry yet names the sound behind a merged letter.\n");
    }
    out
}

/// Every sense `id` of a word, in its stems and nested senses.
fn sense_ids_of<'a>(word: &'a Word, out: &mut Vec<&'a str>) {
    fn walk<'a>(senses: &'a [Sense], out: &mut Vec<&'a str>) {
        for sense in senses {
            out.extend(sense.id.as_deref());
            walk(&sense.senses, out);
        }
    }
    walk(&word.senses, out);
    for stem in &word.stems {
        walk(&stem.senses, out);
    }
}

/// Every sense of a word, in its stems and nested senses, with the cognate
/// ids each inherits from its word and stem, and a label (`lemma stem: gloss`).
fn senses_of(word: &Word) -> Vec<(String, &Sense, Vec<&str>)> {
    fn walk<'a>(
        label: &str,
        senses: &'a [Sense],
        inherited: &[&'a str],
        out: &mut Vec<(String, &'a Sense, Vec<&'a str>)>,
    ) {
        for sense in senses {
            let mut ids = inherited.to_vec();
            ids.extend(sense.cognate_ids.iter().map(String::as_str));
            out.push((format!("{label}: {}", sense.gloss), sense, ids.clone()));
            walk(label, &sense.senses, &ids, out);
        }
    }
    let mut out = Vec::new();
    let ids: Vec<&str> = word.cognate_ids.iter().map(String::as_str).collect();
    walk(&word.lemma, &word.senses, &ids, &mut out);
    for stem in &word.stems {
        let mut ids = ids.clone();
        ids.extend(stem.cognate_ids.iter().map(String::as_str));
        walk(
            &format!("{} {}", word.lemma, stem.stem),
            &stem.senses,
            &ids,
            &mut out,
        );
    }
    out
}

/// A cognate language's name, by the schema's code.
pub fn language_name(code: &str) -> &str {
    match code {
        "akk" => "Akkadian",
        "uga" => "Ugaritic",
        "phn" => "Phoenician",
        "arc" => "Aramaic",
        "syc" => "Syriac",
        "ar" => "Arabic",
        "gez" => "Ge'ez",
        "sem" => "Semitic",
        "grc" => "Greek",
        "egy" => "Egyptian",
        other => other,
    }
}

/// Whether two root keys are one root, the weak ו and י aside (the
/// morphology may spell a hollow root בוא where BDB has ביא).
fn same_root(a: &str, b: &str) -> bool {
    let strong = |s: &str| -> String {
        crate::data_support::bare_letters(s)
            .chars()
            .filter(|c| *c != 'ו' && *c != 'י')
            .collect()
    };
    strong(a) == strong(b)
}

/// The Old Testament tokens of BDB `key`, as `db lexicon-brief` counts them:
/// leaving out tokens filed under it by spelling whose own analysis is a verb
/// of another root BDB has verbs for. `schema` prefixes the runtime
/// database's tables (`"data."` through a `Bible`, `""` opened directly).
pub fn bdb_tokens(db: &Connection, schema: &str, key: &str, root: &str) -> rusqlite::Result<usize> {
    const NT_START: i64 = 40 << 16;
    let mut stmt = db.prepare_cached(&format!(
        "SELECT coalesce(i.root, ''), coalesce(c.part_of_speech = 'Verb', 0), count(*) \
         FROM {schema}surface_entry se \
         JOIN {schema}word w ON w.surface_id = se.surface_id \
         LEFT JOIN {schema}word_info i ON i.info_id = w.info_id \
         LEFT JOIN {schema}morph_cell c ON c.cell_id = i.cell_id \
         WHERE se.key = ?1 AND w.ref < ?2 GROUP BY 1, 2"
    ))?;
    let rows: Vec<(String, bool, usize)> = stmt
        .query_map(rusqlite::params![key, NT_START], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? as usize))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut verb_root = db.prepare_cached(&format!(
        "SELECT EXISTS(SELECT 1 FROM {schema}lexicon_entry WHERE root = ?1 AND pos LIKE 'vb%')"
    ))?;
    let mut total = 0;
    for (analysed, verb, n) in rows {
        let elsewhere = verb
            && !same_root(&analysed, root)
            && verb_root.query_row([&analysed], |r| r.get::<_, bool>(0))?;
        if !elsewhere {
            total += n;
        }
    }
    Ok(total)
}

/// The `bdb` keys the lexicon's words name that the runtime database has no
/// entry for, as `entry: key`.
pub fn unknown_bdb_keys(db: &Connection, schema: &str, lexicon: &Lexicon) -> Result<Vec<String>> {
    let mut stmt = db.prepare(&format!(
        "SELECT EXISTS(SELECT 1 FROM {schema}lexicon_entry WHERE key = ?1)"
    ))?;
    let mut unknown = Vec::new();
    for entry in &lexicon.entries {
        for word in &entry.words {
            for key in &word.bdb {
                if !stmt.query_row([key], |r| r.get::<_, bool>(0))? {
                    unknown.push(format!(
                        "{}: {} names no BDB key `{key}`",
                        entry.id, word.lemma
                    ));
                }
            }
        }
    }
    Ok(unknown)
}

/// A sibling word's sense that shares a sense of the word (`Sense::kin`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct KinSupport {
    /// The sibling's sense, `lemma stem: gloss`.
    pub sense: String,
    /// The sibling word's uses in scripture, where its `bdb` keys count them.
    pub uses: Option<usize>,
    /// Verses its sense cites, and how many of them fix the meaning.
    pub cited: usize,
    pub defining: usize,
    /// Cognates sharing its meaning (`cognate`, or a loan into Hebrew).
    pub cognates: usize,
}

/// The cognate evidence for a word or one of its senses, counted.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct CognateCount {
    /// The cognates it names (through `cognate_ids`, its own or inherited)
    /// that share its meaning: `cognate`, or a loan into Hebrew. Each as
    /// `Language form`.
    pub sharing: Vec<String>,
    /// How many languages those come from.
    pub sharing_languages: usize,
    /// The root's cognates (`cognate`, from a Semitic language) that count
    /// against it: all those it and its kin do not name, where they name
    /// none sharing its meaning; otherwise those showing an older sound
    /// behind a merged letter other than the ones named (another older
    /// root). Each as `Language form (gloss)`.
    pub differing: Vec<String>,
    pub differing_languages: usize,
    /// Cognates it names only as `possible`.
    pub possible: Vec<String>,
    /// Languages sharing its meaning less twice those with another meaning:
    /// opposing evidence weighs double.
    pub balance: i64,
}

/// The evidence for one sense of a word, counted.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SenseSupport {
    /// `lemma stem: gloss`.
    pub sense: String,
    /// Verses it cites, and how many of them fix its meaning.
    pub cited: usize,
    pub defining: usize,
    pub cognates: CognateCount,
    /// Sibling words' senses sharing its meaning.
    pub kin: Vec<KinSupport>,
}

/// The evidence for one word of an entry, counted.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WordSupport {
    pub lemma: String,
    pub lang: Option<String>,
    /// Its uses in scripture: the Old Testament tokens of its `bdb` keys.
    /// `None` when it names none.
    pub uses: Option<usize>,
    /// The verses its senses cite, and how many of them fix a meaning
    /// (`role: defining`).
    pub cited: usize,
    pub defining: usize,
    /// Across its senses: the cognates sharing any sense's meaning, and those
    /// with a meaning other than every sense's.
    pub cognates: CognateCount,
    /// Its senses' kin, each once.
    pub kin: Vec<KinSupport>,
    pub senses: Vec<SenseSupport>,
}

/// The entry's evidence for each of its words and their senses. `uses`
/// counts a word's uses in scripture from its `bdb` keys (see `bdb_tokens`);
/// it returns `None` when they cannot be counted.
pub fn entry_support(entry: &Entry, uses: &dyn Fn(&Word) -> Option<usize>) -> Vec<WordSupport> {
    let by_cognate_id: HashMap<&str, &Cognate> = entry
        .cognates
        .iter()
        .filter_map(|c| Some((c.id.as_deref()?, c)))
        .collect();
    // A loan into Greek is scripture's use, not a cognate's.
    let shares = |c: &Cognate| match c.relation {
        Relation::Cognate => true,
        Relation::Loan => c.lang != "grc",
        Relation::Possible => false,
    };
    let verses = |senses: &[&Sense]| {
        let mut cited = BTreeSet::new();
        let mut defining = BTreeSet::new();
        for sense in senses {
            for citation in &sense.evidence {
                if let Citation::Scripture(c) = citation {
                    cited.insert(c.reference.as_str());
                    if c.role == Role::Defining {
                        defining.insert(c.reference.as_str());
                    }
                }
            }
        }
        (cited.len(), defining.len())
    };
    // Count the cognates `own` names, and the root's cognates that count
    // against them. `named` is every cognate that goes with the meaning (its
    // own and its kin's). Where none of them shares it, cognates show no
    // connection to the root, and each of the root's other cognates counts
    // against it. Where one does, another word of the same root is no
    // evidence against it: a cognate counts against it only when it shows,
    // behind a merged letter, an older sound other than those it names (an
    // older root merged into the same spelling).
    let count = |own: &BTreeSet<&str>, named: &BTreeSet<&str>| {
        let mut out = CognateCount::default();
        let mut sharing = BTreeSet::new();
        let mut differing = BTreeSet::new();
        for c in own.iter().filter_map(|id| by_cognate_id.get(id)) {
            let name = format!("{} {}", language_name(&c.lang), c.form);
            if shares(c) {
                sharing.insert(c.lang.as_str());
                out.sharing.push(name);
            } else if c.relation == Relation::Possible {
                out.possible.push(name);
            }
        }
        let connecting: Vec<&Cognate> = named
            .iter()
            .filter_map(|id| by_cognate_id.get(id).copied())
            .filter(|c| shares(c))
            .collect();
        let mut shown: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for c in &connecting {
            for (letter, sound) in &c.sounds {
                shown
                    .entry(letter.as_str())
                    .or_default()
                    .insert(sound.as_str());
            }
        }
        let another_root = |c: &Cognate| {
            connecting.is_empty()
                || c.sounds.iter().any(|(letter, sound)| {
                    shown
                        .get(letter.as_str())
                        .is_some_and(|sounds| !sounds.contains(sound.as_str()))
                })
        };
        for c in &entry.cognates {
            let is_named = c.id.as_deref().is_some_and(|id| named.contains(id));
            if c.relation == Relation::Cognate
                && !LOAN_ONLY_LANGS.contains(&c.lang.as_str())
                && !is_named
                && another_root(c)
            {
                differing.insert(c.lang.as_str());
                out.differing.push(format!(
                    "{} {} ({})",
                    language_name(&c.lang),
                    c.form,
                    c.gloss
                ));
            }
        }
        out.sharing_languages = sharing.len();
        out.differing_languages = differing.len();
        // Opposing evidence weighs double.
        out.balance = sharing.len() as i64 - 2 * differing.len() as i64;
        out
    };
    let words: Vec<Vec<(String, &Sense, Vec<&str>)>> = entry.words.iter().map(senses_of).collect();
    let mut by_id: HashMap<&str, (usize, &str, &Sense, &[&str])> = HashMap::new();
    for (w, senses) in words.iter().enumerate() {
        for (label, sense, ids) in senses {
            if let Some(id) = &sense.id {
                by_id.insert(id.as_str(), (w, label.as_str(), *sense, ids.as_slice()));
            }
        }
    }
    entry
        .words
        .iter()
        .zip(&words)
        .map(|(word, senses)| {
            let mut sense_supports = Vec::new();
            let mut sense_named = Vec::new();
            let mut all_own = BTreeSet::new();
            let mut all_kin: Vec<KinSupport> = Vec::new();
            for (label, sense, ids) in senses {
                let own: BTreeSet<&str> = ids.iter().copied().collect();
                let mut named = own.clone();
                let mut kin = Vec::new();
                for id in &sense.kin {
                    let Some(&(w, kin_label, kin_sense, kin_ids)) = by_id.get(id.as_str()) else {
                        continue;
                    };
                    named.extend(kin_ids.iter().copied());
                    let (cited, defining) = verses(&[kin_sense]);
                    let kin_ids: BTreeSet<&str> = kin_ids.iter().copied().collect();
                    kin.push(KinSupport {
                        sense: kin_label.to_string(),
                        uses: uses(&entry.words[w]),
                        cited,
                        defining,
                        cognates: kin_ids
                            .iter()
                            .filter_map(|id| by_cognate_id.get(id))
                            .filter(|c| shares(c))
                            .count(),
                    });
                }
                let (cited, defining) = verses(&[*sense]);
                sense_supports.push(SenseSupport {
                    sense: label.clone(),
                    cited,
                    defining,
                    cognates: count(&own, &named),
                    kin: kin.clone(),
                });
                all_own.extend(own);
                for k in kin {
                    if !all_kin.iter().any(|seen| seen.sense == k.sense) {
                        all_kin.push(k);
                    }
                }
                sense_named.push(named);
            }
            // The word goes with every cognate any of its senses goes with.
            let all_named: BTreeSet<&str> = sense_named.into_iter().flatten().collect();
            let cognates = count(&all_own, &all_named);
            let all_senses: Vec<&Sense> = senses.iter().map(|(_, s, _)| *s).collect();
            let (cited, defining) = verses(&all_senses);
            WordSupport {
                lemma: word.lemma.clone(),
                lang: word.lang.clone(),
                uses: uses(word),
                cited,
                defining,
                cognates,
                kin: all_kin,
                senses: sense_supports,
            }
        })
        .collect()
}

/// The `uses` for `entry_support` from the runtime database: the tokens of a
/// word's `bdb` keys, `None` when it names none.
pub fn bdb_uses<'a>(
    db: &'a Connection,
    schema: &'a str,
    entry: &'a Entry,
) -> impl Fn(&Word) -> Option<usize> + 'a {
    move |word: &Word| {
        if word.bdb.is_empty() {
            return None;
        }
        let root = word.root.as_deref().unwrap_or(&entry.root);
        word.bdb
            .iter()
            .map(|key| bdb_tokens(db, schema, key, root).ok())
            .sum()
    }
}

/// `db lexicon-support`: each entry's words, and where their senses differ,
/// their senses, with the evidence counted; and the cognates no word names.
pub fn support_report(lexicon: &Lexicon, db: &Connection, schema: &str) -> String {
    use std::fmt::Write;
    fn listed(items: &[String], languages: usize) -> String {
        if items.is_empty() {
            return "0".to_string();
        }
        format!(
            "{} in {languages} language{}: {}",
            items.len(),
            if languages == 1 { "" } else { "s" },
            items.join(", ")
        )
    }
    fn kin(kin: &[KinSupport]) -> String {
        if kin.is_empty() {
            return "–".to_string();
        }
        kin.iter()
            .map(|k| {
                format!(
                    "{} ({} uses, {} cited ({} defining), {} cognates)",
                    k.sense,
                    k.uses.map_or("–".to_string(), |n| n.to_string()),
                    k.cited,
                    k.defining,
                    k.cognates
                )
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
    let row = |out: &mut String,
               name: &str,
               uses: &str,
               cited: usize,
               defining: usize,
               c: &CognateCount,
               k: &[KinSupport]| {
        let possible = if c.possible.is_empty() {
            "0".to_string()
        } else {
            format!("{}: {}", c.possible.len(), c.possible.join(", "))
        };
        let _ = writeln!(
            out,
            "| {name} | {uses} | {cited} ({defining}) | {} | {} | {} | {possible} | {} |",
            listed(&c.sharing, c.sharing_languages),
            listed(&c.differing, c.differing_languages),
            c.balance,
            kin(k)
        );
    };
    let mut out = String::new();
    out.push_str(
        "Uses: the word's Old Testament tokens (its `bdb` keys). Balance: languages \
         with cognates sharing the meaning, less twice the languages with cognates \
         against it (opposing evidence weighs double). Against a word with no \
         cognate of its own (nor its kin's) count all the root's cognates; against \
         one with cognates, only those showing another older sound behind a merged \
         letter. Its senses are listed under it where they differ.\n\n",
    );
    for entry in &lexicon.entries {
        let _ = writeln!(out, "## {} `{}`\n", entry.root, entry.id);
        let _ = writeln!(
            out,
            "| Word | Uses | Verses cited (defining) | Cognates sharing its meaning | Cognates against it | Balance | Possible | Kin senses |"
        );
        let _ = writeln!(out, "|---|---|---|---|---|---|---|---|");
        let uses = bdb_uses(db, schema, entry);
        for s in entry_support(entry, &uses) {
            let lang = s
                .lang
                .as_deref()
                .map(|l| format!(" ({l})"))
                .unwrap_or_default();
            let n = s.uses.map_or("–".to_string(), |n| n.to_string());
            row(
                &mut out,
                &format!("**{}**{lang}", s.lemma),
                &n,
                s.cited,
                s.defining,
                &s.cognates,
                &s.kin,
            );
            let differ = s
                .senses
                .windows(2)
                .any(|w| w[0].cognates != w[1].cognates || w[0].kin != w[1].kin);
            if differ {
                for sense in &s.senses {
                    let gloss = sense
                        .sense
                        .rsplit_once(": ")
                        .map_or(sense.sense.as_str(), |(_, gloss)| gloss);
                    let stem = sense
                        .sense
                        .strip_prefix(&s.lemma)
                        .and_then(|rest| rest.split_once(':'))
                        .map(|(stem, _)| stem.trim())
                        .filter(|stem| !stem.is_empty())
                        .map(|stem| format!("{stem}: "))
                        .unwrap_or_default();
                    row(
                        &mut out,
                        &format!("↳ {stem}{gloss}"),
                        "",
                        sense.cited,
                        sense.defining,
                        &sense.cognates,
                        &sense.kin,
                    );
                }
            }
        }
        let named: BTreeSet<&str> = entry
            .words
            .iter()
            .flat_map(|w| {
                senses_of(w)
                    .into_iter()
                    .flat_map(|(_, _, ids)| ids)
                    .collect::<Vec<_>>()
            })
            .chain(entry.words.iter().flat_map(|w| {
                w.cognate_ids
                    .iter()
                    .chain(w.stems.iter().flat_map(|s| &s.cognate_ids))
                    .map(String::as_str)
            }))
            .collect();
        let unnamed: Vec<String> = entry
            .cognates
            .iter()
            .filter(|c| c.id.as_deref().is_none_or(|id| !named.contains(id)))
            .map(|c| format!("{} {}", language_name(&c.lang), c.form))
            .collect();
        if !unnamed.is_empty() {
            let _ = writeln!(
                out,
                "\nCognates no word names (give them an `id` and name them in `cognate_ids`): {}",
                unnamed.join(", ")
            );
        }
        out.push('\n');
    }
    out
}

/// `db lexicon-support` on the lexicon in `dir`, counting uses in the
/// runtime database at `db`.
pub fn support_report_dir(dir: &Path, db: &Path) -> Result<String> {
    let lexicon = read_lexicon(dir)?;
    let db = Connection::open(db).with_context(|| format!("opening {}", db.display()))?;
    Ok(support_report(&lexicon, &db, ""))
}

/// The evidence for each word of the entry written as `json`, counting uses
/// in `bible`'s text, as JSON (a list of `WordSupport`): for the review page.
pub fn entry_support_json(bible: &crate::bible::Bible, json: &str) -> Result<String> {
    let entry: Entry = serde_json::from_str(json)?;
    let db = bible.conn();
    let uses = bdb_uses(db, "data.", &entry);
    Ok(serde_json::to_string(&entry_support(&entry, &uses))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verses() -> HashSet<i64> {
        [pack_ref(1, 2, 7), pack_ref(1, 2, 8), pack_ref(1, 1, 24)]
            .into_iter()
            .collect()
    }

    fn sources() -> Vec<Source> {
        vec![Source {
            id: "lane".into(),
            citation: "Lane, Arabic-English Lexicon".into(),
            lang: "ar".into(),
            licence: "Public domain".into(),
            url: String::new(),
            local: String::new(),
        }]
    }

    const ENTRY: &str = r#"{
        "id": "nephesh",
        "root": "נפש",
        "status": "draft",
        "updated": "2026-11-06",
        "tags": ["life"],
        "gloss": "breath; living being",
        "summary": "A living, breathing creature.",
        "words": [{
            "lemma": "נֶפֶשׁ",
            "pos": "noun",
            "gender": "f",
            "gloss": "living being",
            "senses": [{
                "gloss": "living being",
                "definition": "A creature that breathes.",
                "evidence": [
                    { "ref": "Gen.2.7", "role": "defining" },
                    { "ref": "Gen.1.24" }
                ]
            }]
        }],
        "cognates": [{
            "lang": "ar", "form": "نفس", "gloss": "breath", "relation": "cognate",
            "evidence": [{ "source": "lane", "locator": "p. 2826" }]
        }],
        "doctrine": [{
            "topic": "mortality",
            "statement": "Animals are also living beings.",
            "evidence": [{ "ref": "Gen.1.24" }]
        }]
    }"#;

    fn check(json: &str) -> Result<LexiconSummary> {
        let entry: Entry = serde_json::from_str(json)?;
        let lexicon = Lexicon {
            sources: sources(),
            entries: vec![entry],
        };
        check_lexicon(&lexicon, &verses(), None)
    }

    #[test]
    fn accepts_a_well_formed_entry() {
        let summary = check(ENTRY).unwrap();
        assert_eq!(
            summary,
            LexiconSummary {
                entries: 1,
                reviewed: 0,
                words: 1,
                senses: 1,
                citations: 4,
            }
        );
    }

    #[test]
    fn a_signed_off_entry_carries_its_date() {
        let signed = ENTRY.replace(
            "\"status\": \"draft\",",
            "\"status\": \"reviewed\", \"reviewed\": \"2026-11-08\",",
        );
        assert_eq!(check(&signed).unwrap().reviewed, 1);

        let undated = ENTRY.replace("\"status\": \"draft\",", "\"status\": \"reviewed\",");
        let error = check(&undated).unwrap_err().to_string();
        assert!(error.contains("needs its sign-off date"), "{error}");

        let draft = ENTRY.replace(
            "\"status\": \"draft\",",
            "\"status\": \"draft\", \"reviewed\": \"2026-11-08\",",
        );
        let error = check(&draft).unwrap_err().to_string();
        assert!(error.contains("has no `reviewed` date"), "{error}");

        let bad = ENTRY.replace(
            "\"status\": \"draft\",",
            "\"status\": \"reviewed\", \"reviewed\": \"8 Nov\",",
        );
        let error = check(&bad).unwrap_err().to_string();
        assert!(error.contains("`reviewed` must be YYYY-MM-DD"), "{error}");
    }

    #[test]
    fn rejects_an_unknown_field() {
        let json = ENTRY.replace(
            "\"tags\": [\"life\"],",
            "\"tags\": [\"life\"], \"extra\": 1,",
        );
        assert!(check(&json).is_err());
    }

    #[test]
    fn doctrine_rests_on_scripture_only() {
        let json = ENTRY.replace(
            "\"evidence\": [{ \"ref\": \"Gen.1.24\" }]\n        }]\n    }",
            "\"evidence\": [{ \"source\": \"lane\", \"locator\": \"p. 1\" }]\n        }]\n    }",
        );
        let error = check(&json).unwrap_err().to_string();
        assert!(
            error.contains("doctrine[0].evidence[0]: must cite scripture only"),
            "{error}"
        );
    }

    #[test]
    fn a_cognate_cites_a_registered_source() {
        let json = ENTRY.replace("\"source\": \"lane\"", "\"source\": \"nowhere\"");
        let error = check(&json).unwrap_err().to_string();
        assert!(
            error.contains("source `nowhere` is not in sources.json"),
            "{error}"
        );
    }

    #[test]
    fn rejects_a_reference_to_no_verse() {
        let json = ENTRY.replace("Gen.2.7", "Gen.2.99");
        let error = check(&json).unwrap_err().to_string();
        assert!(
            error.contains("`Gen.2.99` is no verse of the text"),
            "{error}"
        );
    }

    #[test]
    fn a_root_is_a_root_key() {
        for root in ["נפשׁ", "מלך"] {
            let json = ENTRY.replace("\"root\": \"נפש\"", &format!("\"root\": \"{root}\""));
            let error = check(&json).unwrap_err().to_string();
            assert!(error.contains("must be a root key"), "{root}: {error}");
        }
    }

    #[test]
    fn a_verb_has_stems() {
        let json = ENTRY.replace("\"pos\": \"noun\"", "\"pos\": \"verb\"");
        let error = check(&json).unwrap_err().to_string();
        assert!(
            error.contains("a verb has `stems` and no `senses`"),
            "{error}"
        );
    }

    #[test]
    fn references_name_haqor_books() {
        assert_eq!(parse_reference("Gen.2.7"), Some((1, 2, 7)));
        assert_eq!(parse_reference("Psa.33.9"), Some((27, 33, 9)));
        assert_eq!(parse_reference("Dan.3.19"), Some((35, 3, 19)));
        assert_eq!(parse_reference("Rom.10.6"), Some((45, 10, 6)));
        assert_eq!(parse_reference("Xyz.1.1"), None);
        assert_eq!(parse_reference("Gen.0.1"), None);
    }

    #[test]
    fn lists_an_entrys_references_once() {
        let refs: Vec<String> = references_in(ENTRY).into_iter().map(|(r, _)| r).collect();
        assert_eq!(refs, ["Gen.1.24", "Gen.2.7"]);
    }

    #[test]
    fn saves_only_a_sound_entry() {
        let dir = std::env::temp_dir().join(format!("haqor-lexicon-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(SOURCES),
            r#"{"sources": [{"id": "lane", "citation": "Lane", "lang": "ar", "licence": "PD", "url": ""}]}"#,
        )
        .unwrap();
        let summary = save_entry(&dir, ENTRY, &verses()).unwrap();
        assert_eq!(summary.entries, 1);
        let saved = std::fs::read_to_string(dir.join("nephesh.json")).unwrap();
        assert_eq!(saved.trim_end(), ENTRY.trim_end());

        // A citation of no verse is refused, and the file is left as it was.
        let broken = ENTRY.replace("Gen.2.7", "Gen.2.99");
        assert!(save_entry(&dir, &broken, &verses()).is_err());
        assert_eq!(
            std::fs::read_to_string(dir.join("nephesh.json")).unwrap(),
            saved
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The checked-in lexicon passes its own checks.
    #[test]
    fn the_lexicon_checks() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let database = root.join("data/haqor.db");
        if !database.exists() {
            eprintln!("skipping: data/haqor.db unavailable");
            return;
        }
        let summary = check_lexicon_dir(
            &root.join("data").join(LEXICON_DIR),
            &database,
            &root.join("src_texts"),
        )
        .unwrap();
        eprintln!("{summary:?}");
    }

    /// One spelling, two older roots: evening behind ġ, pledge behind ʿ, with
    /// an Aramaic word of scripture inside the entry.
    const EREV: &str = r#"{
        "id": "erev",
        "root": "ערב",
        "status": "draft",
        "updated": "2026-11-07",
        "gloss": "evening; pledge",
        "summary": "Two older roots spelled alike.",
        "words": [{
            "lemma": "עֶרֶב",
            "pos": "noun",
            "gloss": "evening",
            "cognate_ids": ["ar-gharb"],
            "senses": [{
                "gloss": "evening",
                "definition": "The end of the day.",
                "evidence": [{ "ref": "Gen.1.24" }]
            }]
        }, {
            "lemma": "עָרַב",
            "pos": "verb",
            "gloss": "stand surety",
            "stems": [{
                "stem": "qal",
                "gloss": "stand surety",
                "cognate_ids": ["ar-arabun"],
                "senses": [{
                    "gloss": "pledge",
                    "definition": "Give a pledge.",
                    "evidence": [{ "ref": "Gen.2.7" }]
                }]
            }]
        }, {
            "lemma": "עֲרַב",
            "lang": "arc",
            "pos": "verb",
            "gloss": "mix",
            "stems": [{
                "stem": "hithpaal",
                "gloss": "be mixed",
                "senses": [{
                    "gloss": "be mixed",
                    "definition": "Be mixed together.",
                    "evidence": [{ "ref": "Gen.2.8" }]
                }]
            }]
        }],
        "cognates": [{
            "id": "ar-gharb", "lang": "ar", "form": "غرب", "gloss": "set (of the sun)",
            "relation": "cognate", "sounds": { "ע": "ġ" },
            "evidence": [{ "source": "lane", "locator": "p. 2238" }]
        }, {
            "id": "ar-arabun", "lang": "ar", "form": "عربون", "gloss": "earnest money",
            "relation": "cognate", "sounds": { "ע": "ʿ" },
            "evidence": [{ "source": "lane", "locator": "p. 1995" }]
        }]
    }"#;

    #[test]
    fn kin_names_a_sibling_words_sense() {
        let with_kin = |kin: &str| {
            EREV.replace(
                r#""gloss": "evening",
                "definition""#,
                &format!(
                    r#""gloss": "evening",
                "kin": ["{kin}"],
                "definition""#
                ),
            )
            .replace(
                r#""gloss": "pledge","#,
                r#""id": "pledge", "gloss": "pledge","#,
            )
        };
        check(&with_kin("pledge")).unwrap();
        let error = check(&with_kin("dusk")).unwrap_err().to_string();
        assert!(error.contains("`kin` names no sense `dusk`"), "{error}");
        let same = with_kin("evening").replace(
            r#""gloss": "evening",
                "kin""#,
            r#""id": "evening", "gloss": "evening",
                "kin""#,
        );
        let error = check(&same).unwrap_err().to_string();
        assert!(error.contains("is a sense of the same word"), "{error}");
    }

    #[test]
    fn a_cognate_of_another_meaning_counts_double() {
        let entry: Entry = serde_json::from_str(EREV).unwrap();
        let support = entry_support(&entry, &|word| (word.lemma == "עֶרֶב").then_some(134));
        let evening = &support[0];
        assert_eq!(evening.uses, Some(134));
        assert_eq!((evening.cited, evening.defining), (1, 0));
        assert_eq!(evening.cognates.sharing, ["Arabic غرب"]);
        assert_eq!(evening.cognates.differing, ["Arabic عربون (earnest money)"]);
        // One language for, one against: the one against counts twice.
        assert_eq!(evening.cognates.balance, -1);
        // The Aramaic "be mixed" shares neither cognate's meaning.
        assert_eq!(support[2].cognates.differing_languages, 1);
        assert_eq!(support[2].cognates.balance, -2);
    }

    #[test]
    fn another_word_of_the_same_older_root_does_not_count_against_it() {
        // A second ġ cognate, "west", named on no word: another word of the
        // evening's older root, so no evidence against it.
        let json = EREV.replace(
            r#""cognates": [{"#,
            r#""cognates": [{
            "id": "ar-maghrib", "lang": "ar", "form": "مغرب", "gloss": "west",
            "relation": "cognate", "sounds": { "ע": "ġ" },
            "evidence": [{ "source": "lane", "locator": "p. 2239" }]
        }, {"#,
        );
        let entry: Entry = serde_json::from_str(&json).unwrap();
        let support = entry_support(&entry, &|_| None);
        let evening = &support[0];
        // Only the ʿ cognate, another older root, counts against it.
        assert_eq!(evening.cognates.differing, ["Arabic عربون (earnest money)"]);
        // The Aramaic "be mixed" names no cognate: every one counts against it.
        assert_eq!(support[2].cognates.differing.len(), 3);
    }

    #[test]
    fn a_kin_senses_cognates_do_not_count_against_it() {
        let json = EREV
            .replace(
                r#""gloss": "be mixed",
                    "definition""#,
                r#""gloss": "be mixed",
                    "kin": ["pledge"],
                    "definition""#,
            )
            .replace(
                r#""gloss": "pledge","#,
                r#""id": "pledge", "gloss": "pledge","#,
            );
        let entry: Entry = serde_json::from_str(&json).unwrap();
        let support = entry_support(&entry, &|_| None);
        let mixed = &support[2];
        assert_eq!(mixed.kin.len(), 1);
        assert_eq!(mixed.kin[0].sense, "עָרַב qal: pledge");
        assert_eq!(mixed.kin[0].cognates, 1);
        // عربون goes with its kin; only غرب has another meaning.
        assert_eq!(mixed.cognates.differing, ["Arabic غرب (set (of the sun))"]);
    }

    #[test]
    fn accepts_aramaic_words_and_sounds() {
        let summary = check(EREV).unwrap();
        assert_eq!(summary.words, 3);
    }

    #[test]
    fn an_aramaic_verb_has_aramaic_stems() {
        let error = check(&EREV.replace("\"hithpaal\"", "\"hithpael\""))
            .unwrap_err()
            .to_string();
        assert!(error.contains("unknown stem `hithpael`"), "{error}");
        let error = check(&EREV.replace("\"qal\"", "\"peal\""))
            .unwrap_err()
            .to_string();
        assert!(error.contains("unknown stem `peal`"), "{error}");
        let error = check(&EREV.replace("\"arc\"", "\"aram\""))
            .unwrap_err()
            .to_string();
        assert!(error.contains("unknown language `aram`"), "{error}");
    }

    #[test]
    fn a_sound_is_one_its_letter_can_stand_for() {
        let error = check(&EREV.replace("\"ע\": \"ġ\"", "\"ע\": \"ḫ\""))
            .unwrap_err()
            .to_string();
        assert!(error.contains("`ע` stands for ʿ or ġ, not `ḫ`"), "{error}");
        let error = check(&EREV.replace("\"ע\": \"ġ\"", "\"ח\": \"ḫ\""))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("`ח` is no letter of the root `ערב`"),
            "{error}"
        );
        let error = check(&EREV.replace("\"ע\": \"ġ\"", "\"ר\": \"r\""))
            .unwrap_err()
            .to_string();
        assert!(error.contains("`ר` stands for only one sound"), "{error}");
    }

    #[test]
    fn a_bare_shin_is_not_a_sin() {
        assert_eq!(root_letters("עשׂה"), ["ע", "שׂ", "ה"]);
        assert!(!root_letters("עשׂה").iter().any(|l| l == "ש"));
    }

    #[test]
    fn cognate_ids_name_the_entrys_cognates() {
        let error = check(&EREV.replace("[\"ar-arabun\"]", "[\"ar-urbun\"]"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("names no cognate `ar-urbun`"), "{error}");
    }

    #[test]
    fn scriptures_aramaic_is_a_word_not_a_cognate() {
        let json = ENTRY.replace(
            r#""lang": "ar", "form": "نفس", "gloss": "breath", "relation": "cognate",
            "evidence": [{ "source": "lane", "locator": "p. 2826" }]"#,
            r#""lang": "arc", "form": "נְפַשׁ", "gloss": "self", "relation": "cognate",
            "evidence": [{ "ref": "Gen.2.7" }]"#,
        );
        let error = check(&json).unwrap_err().to_string();
        assert!(
            error.contains("must cite a source, not scripture"),
            "{error}"
        );
    }

    #[test]
    fn finds_a_sound_split() {
        let lexicon = Lexicon {
            sources: sources(),
            entries: vec![serde_json::from_str(EREV).unwrap()],
        };
        let splits = sound_splits(&lexicon);
        assert_eq!(splits.len(), 1);
        assert_eq!(splits[0].letter, "ע");
        assert_eq!(splits[0].sounds["ġ"], ["עֶרֶב: evening"]);
        assert_eq!(splits[0].sounds["ʿ"], ["עָרַב qal: pledge"]);
        assert!(sound_splits_report(&lexicon).contains("**ערב** `erev` (split)"));
    }

    #[test]
    fn greek_cognates_are_loans_only() {
        let json = ENTRY.replace(
            r#""lang": "ar", "form": "نفس""#,
            r#""lang": "grc", "form": "ψυχή""#,
        );
        let error = check(&json).unwrap_err().to_string();
        assert!(error.contains("`grc` is not a Semitic language"), "{error}");
        let loan = json.replace(r#""relation": "cognate""#, r#""relation": "loan""#);
        check(&loan).unwrap();
    }

    #[test]
    fn egyptian_cognates_are_loans_only() {
        let json = ENTRY.replace(
            r#""lang": "ar", "form": "نفس""#,
            r#""lang": "egy", "form": "pr-ꜥꜣ""#,
        );
        let error = check(&json).unwrap_err().to_string();
        assert!(error.contains("`egy` is not a Semitic language"), "{error}");
        let loan = json.replace(r#""relation": "cognate""#, r#""relation": "loan""#);
        check(&loan).unwrap();
    }

    #[test]
    fn a_greek_loan_cites_its_verses() {
        let with = |evidence: &str| {
            ENTRY.replace(
                r#""gloss": "living being",
            "senses""#,
                &format!(
                    r#""gloss": "living being",
            "greek": [{{ "lemma": "ἀρραβών", "relation": "loan", "evidence": [{evidence}] }}],
            "senses""#
                ),
            )
        };
        check(&with(r#"{ "ref": "Gen.2.7" }"#)).unwrap();
        let error = check(&with(r#"{ "source": "lane", "locator": "p. 1" }"#))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("a loan cites the New Testament verses"),
            "{error}"
        );
    }
}
