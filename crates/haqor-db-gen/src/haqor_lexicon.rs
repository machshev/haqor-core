//! Haqor's own lexicon (ADR 7). The schema and its checks live in
//! [`haqor_core::haqor_lexicon`], shared with the app; the generator uses them
//! from there, and builds the entries into the runtime database here.
//!
//! Three tables for the root entries: `haqor_lexicon` (each entry's id, root
//! and JSON as written), `haqor_lexicon_root` (every root an entry answers to:
//! its own and its words', so Aramaic אֲתַר finds the entry of אשר) and
//! `haqor_lexicon_bdb` (the BDB keys of its words, so a rootless particle
//! finds it too). Two for the name entries: `haqor_lexicon_name` (each with
//! the root entry it links back to) and `haqor_lexicon_person` (the people
//! each names, by `name_entity` id).

use std::path::Path;

use anyhow::{Context, Result, bail};
use log::{info, warn};
use rusqlite::{Connection, params};
use unicode_normalization::UnicodeNormalization;

pub use haqor_core::haqor_lexicon::*;

const SCHEMA: &str = "
DROP TABLE IF EXISTS haqor_lexicon;
DROP TABLE IF EXISTS haqor_lexicon_root;
DROP TABLE IF EXISTS haqor_lexicon_bdb;
DROP TABLE IF EXISTS haqor_lexicon_name;
DROP TABLE IF EXISTS haqor_lexicon_person;
CREATE TABLE haqor_lexicon (
    id TEXT PRIMARY KEY,
    root TEXT NOT NULL,
    json TEXT NOT NULL
);
CREATE TABLE haqor_lexicon_root (
    root TEXT NOT NULL,
    id TEXT NOT NULL,
    PRIMARY KEY (root, id)
) WITHOUT ROWID;
CREATE TABLE haqor_lexicon_bdb (
    key TEXT PRIMARY KEY,
    id TEXT NOT NULL
) WITHOUT ROWID;
-- A name entry (data/lexicon/names): its id, the id of the root entry it
-- links back to, and its JSON as written.
CREATE TABLE haqor_lexicon_name (
    id TEXT PRIMARY KEY,
    entry TEXT NOT NULL,
    json TEXT NOT NULL
);
CREATE INDEX idx_haqor_lexicon_name_entry ON haqor_lexicon_name(entry);
-- The people (name_entity) each name entry names.
CREATE TABLE haqor_lexicon_person (
    entity_id INTEGER NOT NULL,
    id TEXT NOT NULL,
    PRIMARY KEY (entity_id, id)
) WITHOUT ROWID;
";

/// Check the lexicon in `dir` against the text of `db` and write it into
/// `db`'s tables, replacing what was there. Returns the number of entries.
pub fn build_haqor_lexicon(db: &Connection, dir: &Path, src_texts: &Path) -> Result<usize> {
    let lexicon = read_lexicon(dir)?;
    check_lexicon(&lexicon, &verses_of(db)?, Some(src_texts))?;
    let unknown = unknown_bdb_keys(db, "", &lexicon)?;
    if !unknown.is_empty() {
        bail!(
            "the lexicon has {} faults:\n  {}",
            unknown.len(),
            unknown.join("\n  ")
        );
    }
    let texts = read_entry_texts(dir)?;
    let tx = db.unchecked_transaction()?;
    tx.execute_batch(SCHEMA)?;
    {
        let mut entry_row =
            tx.prepare("INSERT INTO haqor_lexicon (id, root, json) VALUES (?1, ?2, ?3)")?;
        let mut root_row =
            tx.prepare("INSERT OR IGNORE INTO haqor_lexicon_root (root, id) VALUES (?1, ?2)")?;
        let mut bdb_row = tx.prepare("INSERT INTO haqor_lexicon_bdb (key, id) VALUES (?1, ?2)")?;
        for entry in &lexicon.entries {
            let (_, json) = texts
                .iter()
                .find(|(id, _)| *id == entry.id)
                .with_context(|| format!("no text for entry `{}`", entry.id))?;
            entry_row.execute(params![entry.id, entry.root, json])?;
            root_row.execute(params![entry.root, entry.id])?;
            for word in &entry.words {
                if let Some(root) = &word.root {
                    root_row.execute(params![root, entry.id])?;
                }
                for key in &word.bdb {
                    bdb_row
                        .execute(params![key, entry.id])
                        .with_context(|| format!("BDB key {key} is in two entries"))?;
                }
            }
        }
        let names = read_name_texts(dir)?;
        let people = if lexicon.names.is_empty() {
            Default::default()
        } else {
            people_records(src_texts)?
        };
        let mut name_row =
            tx.prepare("INSERT INTO haqor_lexicon_name (id, entry, json) VALUES (?1, ?2, ?3)")?;
        let mut person_row =
            tx.prepare("INSERT INTO haqor_lexicon_person (entity_id, id) VALUES (?1, ?2)")?;
        for name in &lexicon.names {
            let (_, json) = names
                .iter()
                .find(|(id, _)| *id == name.id)
                .with_context(|| format!("no text for name entry `{}`", name.id))?;
            name_row.execute(params![name.id, name.entry, json])?;
            for key in &name.people {
                let (entity_id, ..) = people
                    .get(key)
                    .with_context(|| format!("name entry `{}`: no person `{key}`", name.id))?;
                person_row.execute(params![entity_id, name.id])?;
            }
        }
    }
    tx.commit()?;
    check_review_lexemes(&lexicon, src_texts)?;
    info!(
        "Haqor lexicon: {} entries, {} names",
        lexicon.entries.len(),
        lexicon.names.len()
    );
    Ok(lexicon.entries.len())
}

/// Reviewed occurrences address a lexeme by its index in the article's
/// `words`. Warn of any in `src_texts/Haqor-Review` whose index no longer
/// names a word with the reviewed dictionary form, as after an article's
/// words were reordered by hand.
pub fn check_review_lexemes(lexicon: &Lexicon, src_texts: &Path) -> Result<usize> {
    let root = src_texts.join("Haqor-Review");
    if !root.try_exists()? {
        return Ok(0);
    }
    let nfc = |s: &str| s.nfc().collect::<String>();
    let mut files = Vec::new();
    let mut dirs = vec![root];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "json") {
                files.push(path);
            }
        }
    }
    files.sort();
    let mut wrong = Vec::new();
    for path in files {
        let packet: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)
            .with_context(|| format!("parsing {}", path.display()))?;
        if packet["source"] == "greek" {
            continue;
        }
        for word in packet["words"].as_array().into_iter().flatten() {
            let analysis = &word["analysis"];
            let entry = &analysis["entry"];
            let (Some(id), Some(index)) = (
                entry["existing"]
                    .as_str()
                    .or(entry["draft"]["article_id"].as_str()),
                entry["word"].as_u64(),
            ) else {
                continue;
            };
            let lemma = nfc(analysis["lemma"].as_str().unwrap_or(""));
            // An Aramaic or Syriac name matches its Hebrew name article by its
            // consonants, as haqor-review accepts it.
            let syriac = matches!(analysis["language"].as_str(), Some("arc" | "syc"));
            let fold = haqor_core::data_support::fold_consonants;
            // A name article may share its id with an ordinary one.
            let named = index == 0
                && lexicon.names.iter().any(|n| {
                    n.id == id && (nfc(&n.name) == lemma || syriac && fold(&n.name) == fold(&lemma))
                });
            let found = lexicon
                .entries
                .iter()
                .find(|e| e.id == id)
                .and_then(|e| e.words.get(index as usize))
                .map(|w| nfc(&w.lemma));
            // An article not published yet is checked when it is.
            let unpublished = found.is_none()
                && !lexicon.entries.iter().any(|e| e.id == id)
                && !lexicon.names.iter().any(|n| n.id == id);
            if !named && !unpublished && found.as_deref() != Some(lemma.as_str()) {
                wrong.push(format!(
                    "{} position {}: {id} word {index} is {found:?}, reviewed as {lemma}",
                    path.display(),
                    word["position"]
                ));
            }
        }
    }
    if !wrong.is_empty() {
        warn!(
            "{} reviewed occurrences name a lexeme index whose word differs; reopen them in haqor-review:\n  {}",
            wrong.len(),
            wrong
                .iter()
                .take(20)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n  ")
        );
    }
    Ok(wrong.len())
}

/// `db gen-haqor-lexicon`: rebuild the lexicon tables of the runtime database
/// at `path` in place and re-stamp it, so syncing it to the app reinstalls it.
pub fn gen_haqor_lexicon(path: &Path, dir: &Path, src_texts: &Path) -> Result<usize> {
    let db = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
    let entries = build_haqor_lexicon(&db, dir, src_texts)?;
    crate::runtime_db::restamp_built(&db)?;
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_review_lexeme_index_must_name_the_reviewed_word() -> Result<()> {
        let src = std::env::temp_dir().join(format!("haqor-review-lexemes-{}", std::process::id()));
        let dir = src.join("Haqor-Review/ot/1/1");
        std::fs::create_dir_all(&dir)?;
        let word = |position: u32, lemma: &str, index: u32| serde_json::json!({"position":position,"surface":"x","analysis":{"lemma":lemma,"entry":{"existing":"barak","word":index}}});
        let write = |words: Vec<serde_json::Value>| {
            std::fs::write(
                dir.join("1.json"),
                serde_json::json!({"source":"ot","book":1,"chapter":1,"verse":1,"words":words})
                    .to_string(),
            )
        };
        let mut lexicon = Lexicon {
            sources: Vec::new(),
            entries: Vec::new(),
            names: Vec::new(),
        };
        let entry: Entry = serde_json::from_value(serde_json::json!({
            "id":"barak","root":"ברכ","status":"draft","updated":"2026-11-20","gloss":"bless","summary":"",
            "words":[{"lemma":"בָּרַךְ","pos":"verb","gloss":"bless"},{"lemma":"בְּרָכָה","pos":"noun","gloss":"blessing"}]
        }))?;
        lexicon.entries.push(entry);
        write(vec![word(0, "בָּרַךְ", 0), word(1, "בְּרָכָה", 1)])?;
        assert_eq!(check_review_lexemes(&lexicon, &src)?, 0);
        write(vec![word(0, "בְּרָכָה", 0), word(1, "בְּרָכָה", 7)])?;
        assert_eq!(check_review_lexemes(&lexicon, &src)?, 2);
        std::fs::remove_dir_all(src)?;
        Ok(())
    }
}
