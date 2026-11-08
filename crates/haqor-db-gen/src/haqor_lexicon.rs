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
use log::info;
use rusqlite::{Connection, params};

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
                let (entity_id, _) = people
                    .get(key)
                    .with_context(|| format!("name entry `{}`: no person `{key}`", name.id))?;
                person_row.execute(params![entity_id, name.id])?;
            }
        }
    }
    tx.commit()?;
    info!(
        "Haqor lexicon: {} entries, {} names",
        lexicon.entries.len(),
        lexicon.names.len()
    );
    Ok(lexicon.entries.len())
}

/// `db gen-haqor-lexicon`: rebuild the lexicon tables of the runtime database
/// at `path` in place and re-stamp it, so syncing it to the app reinstalls it.
pub fn gen_haqor_lexicon(path: &Path, dir: &Path, src_texts: &Path) -> Result<usize> {
    let db = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
    let entries = build_haqor_lexicon(&db, dir, src_texts)?;
    crate::runtime_db::restamp_built(&db)?;
    Ok(entries)
}
