//! Haqor's own lexicon: one JSON file per root in `data/lexicon/` (see
//! `data/lexicon/README.md` and ADR 7), read into typed structures and checked
//! so a malformed entry or an unsupported claim cannot ship.
//!
//! The checks are the schema's rules: every sense rests on scripture, every
//! cognate on a registered source, every doctrinal statement on scripture
//! alone; every reference names a verse of the text; every source is in
//! `sources.json`.

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::Connection;
use serde::Deserialize;

use crate::runtime_db::pack_ref;

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
pub const COGNATE_LANGS: &[&str] = &["akk", "uga", "phn", "arc", "syc", "ar", "gez", "sem"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub root: String,
    #[serde(default)]
    pub homograph: Option<u8>,
    pub status: Status,
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
    pub senses: Vec<Sense>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sense {
    pub gloss: String,
    pub definition: String,
    pub evidence: Vec<Citation>,
    #[serde(default)]
    pub tags: Vec<String>,
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
    pub lang: String,
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
    is_hebrew(s) && haqor_core::data_support::fold_consonants(s) == s
}

fn is_hebrew(s: &str) -> bool {
    s.chars().any(|c| ('\u{05D0}'..='\u{05EA}').contains(&c))
}

/// Collects every fault with the path to it, so one run reports them all.
struct Checker<'a> {
    verses: &'a HashSet<i64>,
    sources: HashSet<&'a str>,
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
        let Some((book, chapter, verse)) = crate::journeys::parse_ref(reference) else {
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
            self.evidence(&at, &sense.evidence, Accepts::Scripture);
            self.senses(&at, &sense.senses);
        }
    }

    fn word(&mut self, at: &str, word: &Word) {
        self.summary.words += 1;
        if !is_hebrew(&word.lemma) {
            self.fault(at, format_args!("lemma `{}` must be Hebrew", word.lemma));
        }
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
            if !STEMS.contains(&stem.stem.as_str()) {
                self.fault(&at, format_args!("unknown stem `{}`", stem.stem));
            }
            if !seen.insert(stem.stem.as_str()) {
                self.fault(&at, format_args!("stem `{}` listed twice", stem.stem));
            }
            self.text(&at, "gloss", &stem.gloss);
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
        self.tags(&at, &entry.tags);
        self.text(&at, "gloss", &entry.gloss);
        self.text(&at, "summary", &entry.summary);
        if entry.words.is_empty() {
            self.fault(&at, "an entry needs at least one word");
        }
        for (i, word) in entry.words.iter().enumerate() {
            self.word(&format!("{at}.words[{i}]"), word);
        }
        for (i, cognate) in entry.cognates.iter().enumerate() {
            let at = format!("{at}.cognates[{i}]");
            if !COGNATE_LANGS.contains(&cognate.lang.as_str()) {
                self.fault(&at, format_args!("unknown language `{}`", cognate.lang));
            }
            self.text(&at, "form", &cognate.form);
            self.text(&at, "gloss", &cognate.gloss);
            // Its own language's dictionary, or a verse where scripture itself has
            // the cognate: Biblical Aramaic, or the Peshitta's Syriac.
            self.evidence(&at, &cognate.evidence, Accepts::Either);
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

/// Check the lexicon against the text's `verses` and the copies kept under
/// `src_texts`. Fails listing every fault.
pub fn check_lexicon(
    lexicon: &Lexicon,
    verses: &HashSet<i64>,
    src_texts: &Path,
) -> Result<LexiconSummary> {
    let mut checker = Checker {
        verses,
        sources: HashSet::new(),
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
        if !source.local.is_empty() && !src_texts.join(&source.local).exists() {
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
    check_lexicon(&lexicon, &verses_of(&db)?, src_texts)
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
        check_lexicon(&lexicon, &verses(), Path::new("/nonexistent"))
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
}
