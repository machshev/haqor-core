//! Review decisions made against the immutable lexicon JSON shipped in the app.
//! A source hash keeps a phone sign-off from silently approving a newer edit.

use std::path::Path;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexiconReviewRecord {
    pub id: String,
    pub entry_id: String,
    /// `root` or `name`.
    pub entry_kind: String,
    /// SHA-256 of the JSON shown to the reviewer.
    pub source_hash: String,
    /// `entry`, `word`, `sense`, or `doctrine`.
    pub target_kind: String,
    /// A JSON pointer to a word/doctrine, a sense id or pointer; empty for `entry`.
    pub target_key: String,
    pub note: String,
    pub approved: bool,
    pub created_epoch: i64,
    pub updated_epoch: i64,
    /// Applied in the checkout, retained so an older bundled DB still shows approval.
    pub handled: bool,
    pub deleted: bool,
}

pub fn source_hash(json: &str) -> String {
    let digest = Sha256::digest(json.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Store a phone review in the attached progress database. Approvals use a
/// stable id (`approval:root:anan`), so a subsequent review replaces the old
/// decision. Notes use distinct ids and merge independently.
pub fn save(db: &Connection, record: &LexiconReviewRecord) -> rusqlite::Result<()> {
    let valid = !record.id.trim().is_empty()
        && !record.entry_id.trim().is_empty()
        && matches!(record.entry_kind.as_str(), "root" | "name")
        && record.source_hash.len() == 64
        && record.source_hash.bytes().all(|b| b.is_ascii_hexdigit())
        && matches!(
            record.target_kind.as_str(),
            "entry" | "word" | "sense" | "doctrine"
        )
        && (record.target_kind == "entry" || !record.target_key.trim().is_empty())
        && (!record.approved
            || (record.target_kind == "entry"
                && record.target_key.is_empty()
                && record.note.is_empty()))
        && (record.approved || !record.note.trim().is_empty());
    if !valid {
        return Err(rusqlite::Error::InvalidParameterName(
            "invalid lexicon review record".to_string(),
        ));
    }
    db.execute(
        "INSERT INTO progress.lexicon_reviews(
             id, entry_id, entry_kind, source_hash, target_kind, target_key,
             note, approved, created_epoch, updated_epoch, handled, deleted)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
            entry_id=excluded.entry_id, entry_kind=excluded.entry_kind,
            source_hash=excluded.source_hash, target_kind=excluded.target_kind,
            target_key=excluded.target_key, note=excluded.note,
            approved=excluded.approved, updated_epoch=excluded.updated_epoch,
            handled=excluded.handled, deleted=excluded.deleted
         WHERE excluded.updated_epoch > progress.lexicon_reviews.updated_epoch
            OR (excluded.updated_epoch = progress.lexicon_reviews.updated_epoch
                AND (excluded.deleted > progress.lexicon_reviews.deleted
                     OR excluded.handled > progress.lexicon_reviews.handled))",
        params![
            record.id,
            record.entry_id,
            record.entry_kind,
            record.source_hash,
            record.target_kind,
            record.target_key,
            record.note,
            record.approved,
            record.created_epoch,
            record.updated_epoch,
            record.handled,
            record.deleted,
        ],
    )?;
    Ok(())
}

pub fn read(db: &Connection) -> rusqlite::Result<Vec<LexiconReviewRecord>> {
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM progress.sqlite_master
         WHERE type='table' AND name='lexicon_reviews')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        return Ok(Vec::new());
    }
    let mut statement = db.prepare(
        "SELECT id, entry_id, entry_kind, source_hash, target_kind, target_key,
                note, approved, created_epoch, updated_epoch, handled, deleted
         FROM progress.lexicon_reviews WHERE deleted = 0
         ORDER BY created_epoch, id",
    )?;
    statement
        .query_map([], |row| {
            Ok(LexiconReviewRecord {
                id: row.get(0)?,
                entry_id: row.get(1)?,
                entry_kind: row.get(2)?,
                source_hash: row.get(3)?,
                target_kind: row.get(4)?,
                target_key: row.get(5)?,
                note: row.get(6)?,
                approved: row.get(7)?,
                created_epoch: row.get(8)?,
                updated_epoch: row.get(9)?,
                handled: row.get(10)?,
                deleted: row.get(11)?,
            })
        })?
        .collect()
}

pub fn read_file(path: &Path) -> rusqlite::Result<Vec<LexiconReviewRecord>> {
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM main.sqlite_master WHERE name='lexicon_reviews')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        return Ok(Vec::new());
    }
    let has_handled = crate::progress_sync::has_column(&db, "main", "lexicon_reviews", "handled")?;
    let handled = if has_handled { "handled" } else { "0" };
    let filter = if has_handled { "AND handled = 0" } else { "" };
    let mut stmt = db.prepare(&format!(
        "SELECT id, entry_id, entry_kind, source_hash, target_kind, target_key,
                note, approved, created_epoch, updated_epoch, {handled}, deleted
         FROM main.lexicon_reviews WHERE deleted = 0 {filter}
         ORDER BY created_epoch, id"
    ))?;
    stmt.query_map([], |row| {
        Ok(LexiconReviewRecord {
            id: row.get(0)?,
            entry_id: row.get(1)?,
            entry_kind: row.get(2)?,
            source_hash: row.get(3)?,
            target_kind: row.get(4)?,
            target_key: row.get(5)?,
            note: row.get(6)?,
            approved: row.get(7)?,
            created_epoch: row.get(8)?,
            updated_epoch: row.get(9)?,
            handled: row.get(10)?,
            deleted: row.get(11)?,
        })
    })?
    .collect()
}

pub fn has_reviews_file(path: &Path) -> rusqlite::Result<bool> {
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='lexicon_reviews')",
        [],
        |row| row.get(0),
    )
}

pub fn resolve_file(path: &Path, id: &str, epoch: i64) -> rusqlite::Result<bool> {
    let db = Connection::open(path)?;
    Ok(db.execute(
        "UPDATE lexicon_reviews SET deleted = 1, updated_epoch = MAX(updated_epoch + 1, ?2)
         WHERE id = ?1 AND deleted = 0",
        params![id, epoch],
    )? != 0)
}

pub fn mark_handled_file(path: &Path, id: &str, epoch: i64) -> rusqlite::Result<bool> {
    let db = Connection::open(path)?;
    if !crate::progress_sync::has_column(&db, "main", "lexicon_reviews", "handled")? {
        db.execute_batch(
            "ALTER TABLE lexicon_reviews ADD COLUMN handled INTEGER NOT NULL DEFAULT 0",
        )?;
    }
    Ok(db.execute(
        "UPDATE lexicon_reviews SET handled = 1,
             updated_epoch = MAX(updated_epoch + 1, ?2)
         WHERE id = ?1 AND approved = 1 AND deleted = 0 AND handled = 0",
        params![id, epoch],
    )? != 0)
}

pub fn update_note_file(path: &Path, id: &str, note: &str, epoch: i64) -> rusqlite::Result<bool> {
    if note.trim().is_empty() {
        return Err(rusqlite::Error::InvalidParameterName(
            "empty review note".into(),
        ));
    }
    let db = Connection::open(path)?;
    Ok(db.execute(
        "UPDATE lexicon_reviews SET note = ?2,
             updated_epoch = MAX(updated_epoch + 1, ?3)
         WHERE id = ?1 AND deleted = 0 AND approved = 0",
        params![id, note.trim(), epoch],
    )? != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_and_targeted_note_survive_sync_and_resolution() -> anyhow::Result<()> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let folder = std::env::temp_dir().join(format!("haqor-lexicon-review-{nonce}"));
        std::fs::create_dir(&folder)?;
        let local = folder.join("local.db");
        let canonical = folder.join("canonical.db");
        let db = Connection::open_in_memory()?;
        db.execute("ATTACH DATABASE ?1 AS progress", [local.to_str().unwrap()])?;
        crate::tutor::init_progress_schema(&db)?;
        let hash = source_hash("{\"id\":\"anan\",\"status\":\"draft\"}");
        let approval = LexiconReviewRecord {
            id: "approval:root:anan".into(),
            entry_id: "anan".into(),
            entry_kind: "root".into(),
            source_hash: hash.clone(),
            target_kind: "entry".into(),
            target_key: String::new(),
            note: String::new(),
            approved: true,
            created_epoch: 100,
            updated_epoch: 100,
            handled: false,
            deleted: false,
        };
        save(&db, &approval)?;
        let note = LexiconReviewRecord {
            id: "note:root:anan:101".into(),
            target_kind: "sense".into(),
            target_key: "cloud-sky".into(),
            note: "Check the citation".into(),
            approved: false,
            created_epoch: 101,
            updated_epoch: 101,
            ..approval.clone()
        };
        save(&db, &note)?;
        assert_eq!(read(&db)?.len(), 2);
        drop(db);
        crate::progress_sync::merge_progress_files(&canonical, &local)?;
        assert_eq!(read_file(&canonical)?.len(), 2);
        assert!(resolve_file(&canonical, &note.id, 102)?);
        crate::progress_sync::merge_progress_files(&canonical, &local)?;
        assert_eq!(read_file(&canonical)?.len(), 1);
        assert_eq!(read_file(&canonical)?[0].source_hash, hash);
        assert!(mark_handled_file(&canonical, &approval.id, 103)?);
        crate::progress_sync::merge_progress_files(&canonical, &local)?;
        assert!(read_file(&canonical)?.is_empty());
        let reopened = Connection::open_in_memory()?;
        reopened.execute(
            "ATTACH DATABASE ?1 AS progress",
            [canonical.to_str().unwrap()],
        )?;
        assert_eq!(read(&reopened)?.len(), 1);
        assert!(read(&reopened)?[0].handled);
        drop(reopened);
        std::fs::remove_dir_all(folder)?;
        Ok(())
    }
}
