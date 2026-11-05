//! Build Haqor's book outlines from `data/outlines.json` (see
//! `data/OUTLINES.md`): the structure of a book or a stretch of books as a
//! tree of sections, each starting and ending at a verse — the Psalms' five
//! books, their collections and each psalm with its title and author; the
//! collections of the Proverbs; the weekly portions of the Torah; the
//! structure of the law.
//!
//! The result is two tables: `outline` (each outline, with its facets — the
//! facts its sections are indexed by, like a psalm's author — as JSON) and
//! `outline_section` (each section in the outline's order, a parent before
//! its children, with its passage packed onto the Hebrew numbering and its
//! facts as JSON).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use log::info;
use rusqlite::{Connection, params};
use serde_json::{Value, json};

use crate::prophecy::Span;
use crate::runtime_db::pack_ref;
use crate::tsk::Versification;

pub const OUTLINES: &str = "outlines.json";

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS outline;
DROP TABLE IF EXISTS outline_section;
CREATE TABLE outline(
    ord    INTEGER PRIMARY KEY,
    key    TEXT NOT NULL UNIQUE,
    title  TEXT NOT NULL,
    note   TEXT NOT NULL,
    facets TEXT NOT NULL
);
CREATE TABLE outline_section(
    outline        INTEGER NOT NULL,
    ord            INTEGER NOT NULL,
    parent         INTEGER,
    ref            INTEGER NOT NULL,
    end_ref        INTEGER NOT NULL,
    title          TEXT NOT NULL,
    subtitle       TEXT NOT NULL,
    superscription TEXT NOT NULL,
    note           TEXT NOT NULL,
    facts          TEXT NOT NULL,
    PRIMARY KEY(outline, ord)
) WITHOUT ROWID;
CREATE INDEX idx_outline_section_ref ON outline_section(ref);
";

/// What a build wrote, for the log and the tests.
#[derive(Debug, Default)]
pub struct OutlinesSummary {
    pub outlines: usize,
    pub sections: usize,
}

/// The verses of the corpus in order, and where each chapter's run of them
/// starts and ends.
struct Corpus {
    order: Vec<i64>,
    position: HashMap<i64, usize>,
    chapters: BTreeMap<(u8, u8), (i64, i64)>,
}

impl Corpus {
    fn new(verses: &HashSet<i64>) -> Self {
        let mut order: Vec<i64> = verses.iter().copied().collect();
        order.sort_unstable();
        let position = order.iter().enumerate().map(|(i, v)| (*v, i)).collect();
        let mut chapters = BTreeMap::<(u8, u8), (i64, i64)>::new();
        for &v in &order {
            let key = ((v >> 16) as u8, (v >> 8 & 0xff) as u8);
            let run = chapters.entry(key).or_insert((v, v));
            run.1 = v;
        }
        Corpus {
            order,
            position,
            chapters,
        }
    }

    /// Whether `next` is the verse after `last` in the corpus.
    fn follows(&self, last: i64, next: i64) -> bool {
        self.position
            .get(&last)
            .is_some_and(|&i| self.order.get(i + 1) == Some(&next))
    }
}

/// One end of a passage as written: a book, a chapter, and a verse unless
/// it names the whole chapter.
type Point = (u8, u8, Option<u8>);

/// `Gen.3`, `Gen.3.15`.
fn parse_point(text: &str) -> Option<Point> {
    let mut parts = text.split('.');
    let book = parts.next()?;
    let chapter: u8 = parts.next()?.parse().ok()?;
    let verse = parts.next().map(str::parse::<u8>);
    if parts.next().is_some() || chapter == 0 {
        return None;
    }
    let verse = match verse {
        None => None,
        Some(Ok(v)) if v > 0 => Some(v),
        Some(_) => return None,
    };
    // The book's number, as `parse_ref` finds it.
    let (book, _, _) = crate::journeys::parse_ref(&format!("{book}.1.1"))?;
    Some((book, chapter, verse))
}

/// A passage as the file writes it, in STEP Bible's book abbreviations: a
/// whole chapter (`Psa.51`), a run of chapters (`Psa.1-41`), a verse
/// (`Gen.3.15`), a run of verses within a chapter (`Pro.24.23-34`), across
/// chapters (`Gen.6.9-11.32`) or across books (`Exo.20.1-Num.10.10`). Verses
/// are in the English Bible's numbering and renumbered onto the Hebrew text;
/// a whole chapter is the Hebrew text's chapter, with a psalm's title.
fn parse_span(text: &str, versification: &Versification, corpus: &Corpus) -> Option<Span> {
    let (start, end) = match text.split_once('-') {
        Some((start, end)) => (start, Some(end)),
        None => (text, None),
    };
    let first = parse_point(start)?;
    let last = match end {
        None => first,
        Some(end) => {
            let parts: Vec<&str> = end.split('.').collect();
            let number = |s: &str| s.parse::<u8>().ok().filter(|n| *n > 0);
            match (parts.as_slice(), first.2) {
                // A bare number is a verse after a verse, a chapter after a
                // chapter.
                ([n], Some(_)) => (first.0, first.1, Some(number(n)?)),
                ([n], None) => (first.0, number(n)?, None),
                ([c, v], Some(_)) if number(c).is_some() => (first.0, number(c)?, Some(number(v)?)),
                // Another book.
                _ => parse_point(end)?,
            }
        }
    };
    let pack = |v: (u8, u8, u8)| pack_ref(v.0.into(), v.1.into(), v.2.into());
    let from = match first {
        (b, c, Some(v)) => pack(versification.first((b, c, v))),
        (b, c, None) => corpus.chapters.get(&(b, c))?.0,
    };
    let to = match last {
        (b, c, Some(v)) => pack(versification.last((b, c, v))),
        (b, c, None) => corpus.chapters.get(&(b, c))?.1,
    };
    (from <= to).then_some((from, to))
}

/// A section read from the file, its children after it.
#[derive(Debug)]
struct Section {
    parent: Option<usize>,
    span: Span,
    title: String,
    subtitle: String,
    superscription: String,
    note: String,
    /// Each facet's values, in the facets' order.
    facts: Vec<(String, Vec<String>)>,
}

#[derive(Debug)]
struct Outline {
    key: String,
    title: String,
    note: String,
    facets: Vec<(String, String)>,
    sections: Vec<Section>,
}

/// Reads the file's fields, collecting every fault instead of stopping at the
/// first.
struct Checker<'a> {
    versification: &'a Versification,
    corpus: &'a Corpus,
    faults: Vec<String>,
}

impl Checker<'_> {
    fn text(&mut self, value: &Value, what: &str) -> String {
        match value.as_str() {
            Some(s) if !s.trim().is_empty() => s.to_string(),
            _ => {
                self.faults.push(format!("{what}: missing"));
                String::new()
            }
        }
    }

    /// An optional text: empty when absent, a fault when not text.
    fn optional(&mut self, value: &Value, what: &str) -> String {
        match value {
            Value::Null => String::new(),
            _ => self.text(value, what),
        }
    }

    fn span(&mut self, value: &Value, what: &str) -> Option<Span> {
        let Some(text) = value.as_str() else {
            self.faults.push(format!("{what}: no passage"));
            return None;
        };
        match parse_span(text, self.versification, self.corpus) {
            Some(s)
                if self.corpus.position.contains_key(&s.0)
                    && self.corpus.position.contains_key(&s.1) =>
            {
                Some(s)
            }
            Some(_) => {
                self.faults
                    .push(format!("{what}: {text} is outside the corpus"));
                None
            }
            None => {
                self.faults.push(format!("{what}: cannot read {text:?}"));
                None
            }
        }
    }

    /// The sections of `list` and theirs, under `parent` (its index and
    /// span), into `out`; with `contiguous`, each list must run on without
    /// a gap, a parent's from its first verse to its last.
    fn sections(
        &mut self,
        list: &Value,
        parent: Option<(usize, Span)>,
        facets: &[(String, String)],
        contiguous: bool,
        outline: &str,
        out: &mut Vec<Section>,
    ) {
        let mut previous: Option<Span> = None;
        let items = list.as_array().cloned().unwrap_or_default();
        for s in &items {
            let title = self.text(&s["title"], &format!("{outline}: a section's title"));
            let what = format!("{outline} {title}");
            let Some(span) = self.span(&s["passage"], &what) else {
                continue;
            };
            if let Some((_, outer)) = parent
                && !(outer.0 <= span.0 && span.1 <= outer.1)
            {
                self.faults.push(format!("{what}: outside its parent"));
            }
            match previous {
                Some(before) if span.0 <= before.1 => self.faults.push(format!(
                    "{what}: overlaps or comes before the one before it"
                )),
                Some(before) if contiguous && !self.corpus.follows(before.1, span.0) => self
                    .faults
                    .push(format!("{what}: a gap after the one before it")),
                None if contiguous && parent.is_some_and(|(_, outer)| outer.0 != span.0) => {
                    self.faults.push(format!("{what}: starts after its parent"))
                }
                _ => {}
            }
            previous = Some(span);

            let mut facts = Vec::new();
            let given = s["facts"].as_object().cloned().unwrap_or_default();
            for key in given.keys() {
                if !facets.iter().any(|(k, _)| k == key) {
                    self.faults.push(format!("{what}: unknown facet {key:?}"));
                }
            }
            for (key, _) in facets {
                let values: Vec<String> = match &given.get(key) {
                    None => continue,
                    Some(Value::String(v)) => vec![v.clone()],
                    Some(Value::Array(vs)) => vs
                        .iter()
                        .map(|v| self.text(v, &format!("{what} {key}")))
                        .collect(),
                    Some(_) => {
                        self.faults.push(format!("{what} {key}: not text"));
                        continue;
                    }
                };
                if values.is_empty() || values.iter().any(|v| v.trim().is_empty()) {
                    self.faults.push(format!("{what} {key}: empty"));
                }
                facts.push((key.clone(), values));
            }

            let index = out.len();
            out.push(Section {
                parent: parent.map(|(i, _)| i),
                span,
                subtitle: self.optional(&s["subtitle"], &format!("{what} subtitle")),
                superscription: self
                    .optional(&s["superscription"], &format!("{what} superscription")),
                note: self.optional(&s["note"], &format!("{what} note")),
                title,
                facts,
            });
            if !s["sections"].is_null() {
                if s["sections"].as_array().is_none_or(Vec::is_empty) {
                    self.faults.push(format!("{what}: empty sections"));
                }
                self.sections(
                    &s["sections"],
                    Some((index, span)),
                    facets,
                    contiguous,
                    outline,
                    out,
                );
            }
        }
        if contiguous
            && let (Some((_, outer)), Some(last)) = (parent, previous)
            && outer.1 != last.1
        {
            self.faults.push(format!(
                "{outline}: the sections of {} end before it",
                out[parent.unwrap().0].title
            ));
        }
    }
}

fn read(path: &Path, versification: &Versification, corpus: &Corpus) -> Result<Vec<Outline>> {
    let file: Value = serde_json::from_str(
        &std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
    )
    .with_context(|| format!("parsing {}", path.display()))?;
    let mut check = Checker {
        versification,
        corpus,
        faults: Vec::new(),
    };
    let mut outlines = Vec::new();
    let mut keys = HashSet::new();
    for o in file["outlines"].as_array().into_iter().flatten() {
        let key = check.text(&o["id"], "outline id");
        if !keys.insert(key.clone()) {
            check.faults.push(format!("outline {key}: repeated id"));
        }
        let mut facets: Vec<(String, String)> = Vec::new();
        for f in o["facets"].as_array().into_iter().flatten() {
            let id = check.text(&f["id"], &format!("{key}: a facet's id"));
            if facets.iter().any(|(k, _)| *k == id) {
                check.faults.push(format!("{key}: facet {id} repeated"));
            }
            let title = check.text(&f["title"], &format!("{key} facet {id} title"));
            facets.push((id, title));
        }
        let contiguous = o["contiguous"].as_bool().unwrap_or(false);
        let mut sections = Vec::new();
        check.sections(
            &o["sections"],
            None,
            &facets,
            contiguous,
            &key,
            &mut sections,
        );
        if sections.is_empty() {
            check.faults.push(format!("{key}: no sections"));
        }
        outlines.push(Outline {
            title: check.text(&o["title"], &format!("{key} title")),
            note: check.text(&o["note"], &format!("{key} note")),
            key,
            facets,
            sections,
        });
    }
    if !check.faults.is_empty() {
        bail!(
            "{} has {} faults:\n  {}",
            path.display(),
            check.faults.len(),
            check.faults.join("\n  ")
        );
    }
    Ok(outlines)
}

fn write(db: &Connection, outlines: &[Outline]) -> Result<usize> {
    let tx = db.unchecked_transaction()?;
    tx.execute_batch(SCHEMA)?;
    let mut outline = tx.prepare(
        "INSERT INTO outline(ord, key, title, note, facets) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    let mut section = tx.prepare(
        "INSERT INTO outline_section(outline, ord, parent, ref, end_ref, title, subtitle, \
         superscription, note, facts) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;
    let mut count = 0;
    for (i, o) in outlines.iter().enumerate() {
        let facets: Vec<Value> = o.facets.iter().map(|(k, t)| json!([k, t])).collect();
        outline.execute(params![
            i as i64,
            o.key,
            o.title,
            o.note,
            json!(facets).to_string()
        ])?;
        for (ord, s) in o.sections.iter().enumerate() {
            let facts: Vec<Value> = s.facts.iter().map(|(k, v)| json!([k, v])).collect();
            section.execute(params![
                i as i64,
                ord as i64,
                s.parent.map(|p| p as i64),
                s.span.0,
                s.span.1,
                s.title,
                s.subtitle,
                s.superscription,
                s.note,
                json!(facts).to_string()
            ])?;
            count += 1;
        }
    }
    drop((outline, section));
    tx.commit()?;
    Ok(count)
}

/// The verses of the runtime database `db`: the OT's and the Peshitta's, with
/// the few the Greek NT has beyond them.
fn corpus_verses(db: &Connection) -> Result<HashSet<i64>> {
    let mut verses: HashSet<i64> = db
        .prepare("SELECT ref FROM verse")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    verses.extend(
        db.prepare("SELECT DISTINCT ref FROM greek_word")?
            .query_map([], |row| row.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?,
    );
    Ok(verses)
}

/// Build the outline tables of the runtime database `db` from `path`
/// (`data/outlines.json`). Fails, naming every fault, on a passage that
/// cannot be read or is outside the corpus, a section outside its parent or
/// overlapping the one before it (or, in a contiguous outline, leaving a
/// gap), an unknown facet, or a repeated id.
pub fn build_outlines(db: &Connection, path: &Path, src_texts: &Path) -> Result<OutlinesSummary> {
    let tahot = crate::stepbible::source_dir(src_texts);
    let versification = Versification::from_tahot(&tahot).with_context(|| {
        format!(
            "reading the KJV/Hebrew verse numbering from {}",
            tahot.display()
        )
    })?;
    let corpus = Corpus::new(&corpus_verses(db)?);
    let outlines = read(path, &versification, &corpus)?;
    let sections = write(db, &outlines)?;
    let summary = OutlinesSummary {
        outlines: outlines.len(),
        sections,
    };
    info!(
        "Outlines: {} with {} sections",
        summary.outlines, summary.sections
    );
    Ok(summary)
}

/// `db gen-outlines`: rebuild the outline tables of the runtime database at
/// `path` in place and re-stamp it, so syncing it to the app reinstalls it.
pub fn gen_outlines(path: &Path, file: &Path, src_texts: &Path) -> Result<OutlinesSummary> {
    let db = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
    let summary = build_outlines(&db, file, src_texts)?;
    crate::runtime_db::restamp_built(&db)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        let mut verses = HashSet::new();
        // Psalm 3 with its title as verse 1, Psalm 4, and Genesis 1–2.
        for v in 1..=9 {
            verses.insert(pack_ref(27, 3, v));
        }
        for v in 1..=9 {
            verses.insert(pack_ref(27, 4, v));
        }
        for v in 1..=31 {
            verses.insert(pack_ref(1, 1, v));
        }
        for v in 1..=25 {
            verses.insert(pack_ref(1, 2, v));
        }
        Corpus::new(&verses)
    }

    #[test]
    fn passages_parse() {
        let corpus = corpus();
        let mut v = Versification::default();
        // Psalm 3:1 (KJV) is 3:2 in the Hebrew text.
        v.insert((27, 3, 1), (27, 3, 2));
        let span = |t: &str| parse_span(t, &v, &corpus);
        // A whole chapter keeps the title.
        assert_eq!(
            span("Psa.3"),
            Some((pack_ref(27, 3, 1), pack_ref(27, 3, 9)))
        );
        assert_eq!(
            span("Psa.3-4"),
            Some((pack_ref(27, 3, 1), pack_ref(27, 4, 9)))
        );
        assert_eq!(
            span("Psa.3.1"),
            Some((pack_ref(27, 3, 2), pack_ref(27, 3, 2)))
        );
        assert_eq!(
            span("Gen.1.3-2.4"),
            Some((pack_ref(1, 1, 3), pack_ref(1, 2, 4)))
        );
        assert_eq!(
            span("Gen.1.3-5"),
            Some((pack_ref(1, 1, 3), pack_ref(1, 1, 5)))
        );
        assert_eq!(
            span("Gen.2.1-Psa.3.1"),
            Some((pack_ref(1, 2, 1), pack_ref(27, 3, 2)))
        );
        assert_eq!(
            span("Gen.2-Psa.3"),
            Some((pack_ref(1, 2, 1), pack_ref(27, 3, 9)))
        );
        assert_eq!(span("Psa.4-3"), None);
        assert_eq!(span("Psa.9"), None);
        assert_eq!(span("Xyz.1"), None);
        assert_eq!(span("Psa.0"), None);
    }

    #[test]
    fn contiguity() {
        let corpus = corpus();
        assert!(corpus.follows(pack_ref(27, 3, 9), pack_ref(27, 4, 1)));
        assert!(!corpus.follows(pack_ref(27, 3, 8), pack_ref(27, 4, 1)));
    }

    fn checked(file: Value) -> Result<Vec<Outline>> {
        let dir = std::env::temp_dir().join(format!("outlines-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{}.json", rand_name(&file)));
        std::fs::write(&path, file.to_string()).unwrap();
        let result = read(&path, &Versification::default(), &corpus());
        std::fs::remove_file(&path).ok();
        result
    }

    fn rand_name(v: &Value) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        v.to_string().hash(&mut h);
        h.finish()
    }

    #[test]
    fn faults_are_found() {
        let ok = checked(json!({"outlines": [{
            "id": "ps", "title": "Psalms", "note": "n", "contiguous": true,
            "facets": [{"id": "author", "title": "Author"}],
            "sections": [{"title": "All", "passage": "Psa.3-4", "sections": [
                {"title": "Psalm 3", "passage": "Psa.3", "facts": {"author": "David"}},
                {"title": "Psalm 4", "passage": "Psa.4",
                 "facts": {"author": ["David", "Asaph"]}}
            ]}]
        }]}))
        .unwrap();
        let s = &ok[0].sections;
        assert_eq!(s.len(), 3);
        assert_eq!(s[1].parent, Some(0));
        assert_eq!(
            s[2].facts,
            vec![("author".into(), vec!["David".into(), "Asaph".into()])]
        );

        let gap = checked(json!({"outlines": [{
            "id": "ps", "title": "Psalms", "note": "n", "contiguous": true,
            "sections": [{"title": "All", "passage": "Psa.3-4", "sections": [
                {"title": "Psalm 3", "passage": "Psa.3.1-8"},
                {"title": "Psalm 4", "passage": "Psa.4"}
            ]}]
        }]}))
        .unwrap_err()
        .to_string();
        assert!(gap.contains("a gap"), "{gap}");

        let bad = checked(json!({"outlines": [{
            "id": "ps", "title": "Psalms", "note": "n",
            "sections": [
                {"title": "B", "passage": "Psa.4", "facts": {"who": "x"}},
                {"title": "A", "passage": "Psa.3"},
                {"title": "C", "passage": "Psa.4.1", "sections": [
                    {"title": "D", "passage": "Psa.3"}
                ]}
            ]
        }]}))
        .unwrap_err()
        .to_string();
        assert!(bad.contains("unknown facet"), "{bad}");
        assert!(bad.contains("A: overlaps or comes before"), "{bad}");
        assert!(bad.contains("D: outside its parent"), "{bad}");
    }

    /// The file reads cleanly against the corpus and the real verse
    /// numbering, and its outlines have the shape their notes promise.
    #[test]
    fn complete_build() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let src_texts = root.join("src_texts");
        let database = root.join("data/haqor.db");
        if !crate::stepbible::source_dir(&src_texts).exists() || !database.exists() {
            eprintln!("skipping: TAHOT source or data/haqor.db unavailable");
            return;
        }
        let db = Connection::open_in_memory().unwrap();
        db.execute("ATTACH DATABASE ?1 AS src", [database.to_string_lossy()])
            .unwrap();
        for table in ["verse", "greek_word"] {
            db.execute_batch(&format!(
                "CREATE TABLE {table} AS SELECT ref FROM src.{table}"
            ))
            .unwrap();
        }
        db.execute_batch("DETACH DATABASE src").unwrap();
        let summary = build_outlines(&db, &root.join("data").join(OUTLINES), &src_texts).unwrap();
        eprintln!("{summary:?}");

        let leaves = |key: &str| -> Vec<(i64, i64, String)> {
            db.prepare(
                "SELECT s.ref, s.end_ref, s.title FROM outline_section s \
                 JOIN outline o ON o.ord = s.outline WHERE o.key = ?1 AND NOT EXISTS \
                 (SELECT 1 FROM outline_section c WHERE c.outline = s.outline \
                  AND c.parent = s.ord) ORDER BY s.ord",
            )
            .unwrap()
            .query_map([key], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
        };
        // Every psalm, each its whole chapter with its title.
        let psalms = leaves("psalms");
        assert_eq!(psalms.len(), 150);
        for (i, (start, end, _)) in psalms.iter().enumerate() {
            let chapter = i as i64 + 1;
            assert_eq!(*start, pack_ref(27, chapter, 1), "Psalm {chapter}");
            assert_eq!(*end >> 8, pack_ref(27, chapter, 0) >> 8, "Psalm {chapter}");
        }
        // The 54 portions, from Genesis 1:1 to Deuteronomy 34:12.
        let portions = leaves("torah-portions");
        assert_eq!(portions.len(), 54);
        assert_eq!(portions[0].0, pack_ref(1, 1, 1));
        assert_eq!(portions[53].1, pack_ref(5, 34, 12));
        // Psalm 3's author.
        let facts: String = db
            .query_row(
                "SELECT facts FROM outline_section WHERE ref = ?1 AND end_ref < ?2",
                [pack_ref(27, 3, 1), pack_ref(27, 4, 0)],
                |r| r.get(0),
            )
            .unwrap();
        assert!(facts.contains("David"), "{facts}");
    }
}
