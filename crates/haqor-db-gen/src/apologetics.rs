//! Build Haqor's apologetics from `data/apologetics.json` (see
//! `data/APOLOGETICS.md`): the evidence for the resurrection and the records,
//! the Bible's teaching on the subjects it is most often misread on, and
//! answers on the passages brought against that teaching.
//!
//! The result is three tables: `apologetic_subject`, `apologetic` (an entry,
//! with its points, objections and related entries as JSON in `detail`, its
//! passages packed onto the Hebrew numbering) and `apologetic_passage`, the
//! verses the reader marks with each entry.

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result, bail};
use log::info;
use rusqlite::{Connection, params};
use serde_json::{Value, json};

use crate::prophecy::{Span, parse_span};
use crate::tsk::Versification;

pub const APOLOGETICS: &str = "apologetics.json";

/// What an entry is: evidence for the Bible's truth, the Bible's teaching on a
/// subject, or an answer on a passage brought against it.
pub const KINDS: &[&str] = &["evidence", "doctrine", "passage"];

pub const LICENCES: &[&str] = &["public-domain", "copyright"];

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS apologetic_subject;
DROP TABLE IF EXISTS apologetic;
DROP TABLE IF EXISTS apologetic_passage;
CREATE TABLE apologetic_subject(
    ord   INTEGER PRIMARY KEY,
    key   TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    note  TEXT NOT NULL
);
CREATE TABLE apologetic(
    entry_id INTEGER PRIMARY KEY,
    key      TEXT NOT NULL UNIQUE,
    subject  TEXT NOT NULL,
    kind     TEXT NOT NULL,
    title    TEXT NOT NULL,
    summary  TEXT NOT NULL,
    detail   TEXT NOT NULL
);
CREATE TABLE apologetic_passage(
    entry_id INTEGER NOT NULL,
    ord      INTEGER NOT NULL,
    ref      INTEGER NOT NULL,
    end_ref  INTEGER NOT NULL,
    PRIMARY KEY(entry_id, ord)
) WITHOUT ROWID;
CREATE INDEX idx_apologetic_passage_ref ON apologetic_passage(ref);
";

/// What a build wrote, for the log and the tests.
#[derive(Debug, Default)]
pub struct ApologeticsSummary {
    pub subjects: usize,
    pub entries: usize,
    pub passages: usize,
}

/// A subject's key, title and note.
type Subject = (String, String, String);

struct Entry {
    key: String,
    subject: String,
    kind: String,
    title: String,
    summary: String,
    passages: Vec<Span>,
    detail: Value,
}

/// Reads the file's fields, collecting every fault instead of stopping at the
/// first.
struct Checker<'a> {
    versification: &'a Versification,
    verses: &'a HashSet<i64>,
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

    fn spans(&mut self, value: &Value, what: &str) -> Vec<Span> {
        let mut spans = Vec::new();
        for passage in value.as_array().into_iter().flatten() {
            let text = passage.as_str().unwrap_or("");
            match parse_span(text, self.versification) {
                Some(s) if self.verses.contains(&s.0) && self.verses.contains(&s.1) => {
                    spans.push(s)
                }
                Some(_) => self
                    .faults
                    .push(format!("{what}: {text} is outside the corpus")),
                None => self.faults.push(format!("{what}: cannot read {text:?}")),
            }
        }
        spans
    }
}

fn packed(spans: &[Span]) -> Value {
    spans.iter().map(|s| json!([s.0, s.1])).collect()
}

fn read(
    path: &Path,
    versification: &Versification,
    verses: &HashSet<i64>,
) -> Result<(Vec<Subject>, Vec<Entry>)> {
    let file: Value = serde_json::from_str(
        &std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
    )
    .with_context(|| format!("parsing {}", path.display()))?;
    let mut check = Checker {
        versification,
        verses,
        faults: Vec::new(),
    };

    let mut subjects = Vec::new();
    for s in file["subjects"].as_array().into_iter().flatten() {
        let key = check.text(&s["id"], "subject id");
        let what = format!("subject {key}");
        subjects.push((
            key,
            check.text(&s["title"], &format!("{what} title")),
            check.text(&s["note"], &format!("{what} note")),
        ));
    }
    let subject_keys: HashSet<&str> = subjects.iter().map(|s| s.0.as_str()).collect();

    let works = file["works"].as_object().cloned().unwrap_or_default();
    for (id, work) in &works {
        let licence = work["licence"].as_str().unwrap_or("");
        if !LICENCES.contains(&licence) {
            check
                .faults
                .push(format!("work {id}: unknown licence {licence:?}"));
        }
        check.text(&work["title"], &format!("work {id} title"));
    }

    let raw = file["entries"].as_array().cloned().unwrap_or_default();
    let keys: Vec<&str> = raw.iter().filter_map(|e| e["id"].as_str()).collect();
    let mut seen = HashSet::new();
    let mut entries = Vec::new();
    for e in &raw {
        let key = check.text(&e["id"], "entry id");
        let what = format!("entry {key}");
        if !seen.insert(key.clone()) {
            check.faults.push(format!("{what}: repeated id"));
        }
        let subject = check.text(&e["subject"], &format!("{what} subject"));
        if !subject.is_empty() && !subject_keys.contains(subject.as_str()) {
            check
                .faults
                .push(format!("{what}: unknown subject {subject:?}"));
        }
        let kind = check.text(&e["kind"], &format!("{what} kind"));
        if !kind.is_empty() && !KINDS.contains(&kind.as_str()) {
            check.faults.push(format!("{what}: unknown kind {kind:?}"));
        }
        let title = check.text(&e["title"], &format!("{what} title"));
        let summary = check.text(&e["summary"], &format!("{what} summary"));
        let passages = check.spans(&e["passages"], &what);
        if passages.is_empty() {
            check.faults.push(format!("{what}: no passages"));
        }

        let mut points = Vec::new();
        for (i, p) in e["points"].as_array().into_iter().flatten().enumerate() {
            let at = format!("{what} point {}", i + 1);
            let mut sources = Vec::new();
            for s in p["sources"].as_array().into_iter().flatten() {
                let id = s["work"].as_str().unwrap_or("");
                let Some(work) = works.get(id) else {
                    check.faults.push(format!("{at}: unknown work {id:?}"));
                    continue;
                };
                let quote = s["quote"].as_str().unwrap_or("");
                if !quote.is_empty() && work["licence"] != "public-domain" {
                    check
                        .faults
                        .push(format!("{at}: quotes {id}, which is under copyright"));
                }
                sources.push(json!({
                    "title": work["title"].as_str().unwrap_or(""),
                    "author": work["author"].as_str().unwrap_or(""),
                    "loc": s["loc"].as_str().unwrap_or(""),
                    "quote": quote,
                }));
            }
            points.push(json!({
                "claim": check.text(&p["claim"], &format!("{at} claim")),
                "passages": packed(&check.spans(&p["passages"], &at)),
                "sources": sources,
            }));
        }
        let mut objections = Vec::new();
        for (i, o) in e["objections"].as_array().into_iter().flatten().enumerate() {
            let at = format!("{what} objection {}", i + 1);
            objections.push(json!({
                "objection": check.text(&o["objection"], &format!("{at} objection")),
                "answer": check.text(&o["answer"], &format!("{at} answer")),
                "passages": packed(&check.spans(&o["passages"], &at)),
            }));
        }
        let mut related = Vec::new();
        for r in e["related"].as_array().into_iter().flatten() {
            let r = r.as_str().unwrap_or("");
            if keys.contains(&r) && r != key {
                related.push(r.to_string());
            } else {
                check.faults.push(format!("{what}: unknown related {r:?}"));
            }
        }

        entries.push(Entry {
            key,
            subject,
            kind,
            title,
            summary,
            passages,
            detail: json!({
                "points": points,
                "objections": objections,
                "related": related,
            }),
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
    Ok((subjects, entries))
}

fn write(db: &Connection, subjects: &[Subject], entries: &[Entry]) -> Result<usize> {
    let tx = db.unchecked_transaction()?;
    tx.execute_batch(SCHEMA)?;
    let mut subject = tx
        .prepare("INSERT INTO apologetic_subject(ord, key, title, note) VALUES (?1, ?2, ?3, ?4)")?;
    for (i, (key, title, note)) in subjects.iter().enumerate() {
        subject.execute(params![i as i64, key, title, note])?;
    }
    let mut entry = tx.prepare(
        "INSERT INTO apologetic(entry_id, key, subject, kind, title, summary, detail) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    let mut passage = tx.prepare(
        "INSERT INTO apologetic_passage(entry_id, ord, ref, end_ref) VALUES (?1, ?2, ?3, ?4)",
    )?;
    let mut passages = 0;
    for (i, e) in entries.iter().enumerate() {
        let id = i as i64 + 1;
        entry.execute(params![
            id,
            e.key,
            e.subject,
            e.kind,
            e.title,
            e.summary,
            e.detail.to_string()
        ])?;
        for (ord, span) in e.passages.iter().enumerate() {
            passage.execute(params![id, ord as i64, span.0, span.1])?;
            passages += 1;
        }
    }
    drop((subject, entry, passage));
    tx.commit()?;
    Ok(passages)
}

/// Build the apologetics tables of the runtime database `db` from `path`
/// (`data/apologetics.json`). Fails, naming every fault, on a passage that
/// cannot be read or is outside the corpus, an unknown subject, kind, work,
/// licence or related entry, a repeated id, or a quote from a work under
/// copyright.
pub fn build_apologetics(
    db: &Connection,
    path: &Path,
    src_texts: &Path,
) -> Result<ApologeticsSummary> {
    let tahot = crate::stepbible::source_dir(src_texts);
    let versification = Versification::from_tahot(&tahot).with_context(|| {
        format!(
            "reading the KJV/Hebrew verse numbering from {}",
            tahot.display()
        )
    })?;
    let mut verses: HashSet<i64> = db
        .prepare("SELECT ref FROM verse")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    // The Greek NT has a few verses the Peshitta does not.
    verses.extend(
        db.prepare("SELECT DISTINCT ref FROM greek_word")?
            .query_map([], |row| row.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?,
    );
    let (subjects, entries) = read(path, &versification, &verses)?;
    let passages = write(db, &subjects, &entries)?;
    let summary = ApologeticsSummary {
        subjects: subjects.len(),
        entries: entries.len(),
        passages,
    };
    info!(
        "Apologetics: {} entries in {} subjects, marking {} passages",
        summary.entries, summary.subjects, summary.passages
    );
    Ok(summary)
}

/// `db gen-apologetics`: rebuild the apologetics tables of the runtime
/// database at `path` in place and re-stamp it, so syncing it to the app
/// reinstalls it.
pub fn gen_apologetics(path: &Path, file: &Path, src_texts: &Path) -> Result<ApologeticsSummary> {
    let db = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
    let summary = build_apologetics(&db, file, src_texts)?;
    crate::runtime_db::restamp_built(&db)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_db::pack_ref;

    /// The file reads cleanly against the built database, and its passages
    /// land on the Hebrew numbering.
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
                "CREATE TABLE {table} AS SELECT * FROM src.{table}"
            ))
            .unwrap();
        }
        let summary =
            build_apologetics(&db, &root.join("data").join(APOLOGETICS), &src_texts).unwrap();
        assert!(summary.entries >= 10);
        // Luke 23:42-43 marks the thief's request and the answer.
        let key: String = db
            .query_row(
                "SELECT a.key FROM apologetic a JOIN apologetic_passage p USING(entry_id) \
                 WHERE p.ref <= ?1 AND p.end_ref >= ?1",
                [pack_ref(42, 23, 43)],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(key, "luke-23-43");
        // Malachi 4:1-3 (KJV) is Hebrew 3:19-21.
        let marked: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM apologetic_passage WHERE ref = ?1)",
                [pack_ref(26, 3, 19)],
                |r| r.get(0),
            )
            .unwrap();
        assert!(marked);
    }

    #[test]
    fn faults_are_named() {
        let dir = std::env::temp_dir().join(format!("haqor-apologetics-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(APOLOGETICS);
        std::fs::write(
            &path,
            r#"{
              "subjects": [{ "id": "god", "title": "God", "note": "One." }],
              "works": { "w": { "title": "W", "licence": "copyright" } },
              "entries": [
                { "id": "a", "subject": "nope", "kind": "doctrine", "title": "T",
                  "summary": "S", "passages": ["Mat.1.1"],
                  "points": [{ "claim": "C", "passages": ["Xyz.1.1"],
                               "sources": [{ "work": "w", "quote": "q" }] }],
                  "related": ["b"] }
              ]
            }"#,
        )
        .unwrap();
        let verses: HashSet<i64> = [pack_ref(40, 1, 1)].into();
        let err = read(&path, &Versification::default(), &verses)
            .err()
            .unwrap()
            .to_string();
        std::fs::remove_dir_all(&dir).ok();
        for fault in [
            "unknown subject",
            "cannot read \"Xyz.1.1\"",
            "under copyright",
            "unknown related \"b\"",
        ] {
            assert!(err.contains(fault), "{fault} not in {err}");
        }
    }
}
