//! Snapshot and merge support for a learner's writable `progress.db`.
//!
//! The corpus databases are identical, read-only app assets.  Only the small
//! progress database is synchronised.  A merge is deliberately monotonic: a
//! card state with the most recently recorded review wins, while one-time
//! concepts and activity records are unioned.  This lets two devices make
//! progress offline and converge when they next meet the LAN server.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::Connection;

use crate::tutor::{GlossOverride, IssueReport, LexiconEntryOverride, init_progress_schema};

/// SQLite's file header. Checking it before attaching keeps a bad HTTP body
/// from becoming an opaque "file is not a database" error later on.
pub const SQLITE_HEADER: &[u8] = b"SQLite format 3\0";

/// Write a transactionally consistent copy of the attached `progress` schema
/// to `destination`. `destination` must not already exist.
pub fn export_progress_snapshot(db: &Connection, destination: &Path) -> rusqlite::Result<()> {
    db.execute(
        "VACUUM progress INTO ?1",
        [destination.to_string_lossy().as_ref()],
    )?;
    Ok(())
}

/// Merge a progress snapshot into the already-attached `progress` schema.
///
/// The supplied file is opened read/write only long enough to add a harmless
/// migration column for snapshots made by older app versions. Callers normally
/// pass a private temporary file received from the sync server.
pub fn merge_progress_snapshot(db: &Connection, snapshot: &Path) -> rusqlite::Result<()> {
    db.execute(
        "ATTACH DATABASE ?1 AS sync",
        [snapshot.to_string_lossy().as_ref()],
    )?;
    let result = merge_attached_snapshot(db);
    let detach = db.execute_batch("DETACH DATABASE sync");
    result.and(detach)
}

/// Merge `incoming` into a canonical on-disk progress database. Used by the
/// LAN server; it creates and migrates the canonical database on first use.
pub fn merge_progress_files(canonical: &Path, incoming: &Path) -> rusqlite::Result<()> {
    let db = Connection::open_in_memory()?;
    db.execute(
        "ATTACH DATABASE ?1 AS progress",
        [canonical.to_string_lossy().as_ref()],
    )?;
    init_progress_schema(&db)?;
    merge_progress_snapshot(&db, incoming)
}

fn has_column(db: &Connection, schema: &str, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = db.prepare(&format!("PRAGMA {schema}.table_info({table})"))?;
    stmt.query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map(|columns| columns.iter().any(|name| name == column))
}

fn ensure_updated_epochs(db: &Connection, schema: &str) -> rusqlite::Result<()> {
    for table in ["glyph_srs", "word_srs", "form_srs", "suffix_srs"] {
        if !has_column(db, schema, table, "updated_epoch")? {
            db.execute_batch(&format!(
                "ALTER TABLE {schema}.{table} ADD COLUMN updated_epoch INTEGER NOT NULL DEFAULT 0"
            ))?;
        }
    }
    Ok(())
}

fn ensure_gloss_overrides(db: &Connection, schema: &str) -> rusqlite::Result<()> {
    db.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {schema}.gloss_overrides(
            surface TEXT PRIMARY KEY,
            gloss TEXT NOT NULL,
            note TEXT NOT NULL DEFAULT '',
            updated_epoch INTEGER NOT NULL,
            deleted INTEGER NOT NULL DEFAULT 0
        )"
    ))?;
    if !has_column(db, schema, "gloss_overrides", "deleted")? {
        db.execute_batch(&format!(
            "ALTER TABLE {schema}.gloss_overrides \
             ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0"
        ))?;
    }
    Ok(())
}

fn ensure_issue_reports(db: &Connection, schema: &str) -> rusqlite::Result<()> {
    db.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {schema}.issue_reports(
            id            TEXT PRIMARY KEY,
            report_type   TEXT NOT NULL,
            note          TEXT NOT NULL,
            context_json  TEXT NOT NULL,
            created_epoch INTEGER NOT NULL,
            updated_epoch INTEGER NOT NULL,
            deleted       INTEGER NOT NULL DEFAULT 0
        )"
    ))?;
    if !has_column(db, schema, "issue_reports", "deleted")? {
        db.execute_batch(&format!(
            "ALTER TABLE {schema}.issue_reports \
             ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0"
        ))?;
    }
    Ok(())
}

fn ensure_lexicon_entry_overrides(db: &Connection, schema: &str) -> rusqlite::Result<()> {
    db.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {schema}.lexicon_entry_overrides(
            surface       TEXT PRIMARY KEY,
            root          TEXT NOT NULL DEFAULT '',
            gloss         TEXT NOT NULL,
            updated_epoch INTEGER NOT NULL
        )"
    ))
}

fn merge_attached_snapshot(db: &Connection) -> rusqlite::Result<()> {
    ensure_updated_epochs(db, "progress")?;
    ensure_updated_epochs(db, "sync")?;
    ensure_gloss_overrides(db, "progress")?;
    ensure_gloss_overrides(db, "sync")?;
    ensure_lexicon_entry_overrides(db, "progress")?;
    ensure_lexicon_entry_overrides(db, "sync")?;
    ensure_issue_reports(db, "progress")?;
    ensure_issue_reports(db, "sync")?;
    db.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| {
        // `updated_epoch` is the normal conflict resolution key. The `reps`
        // fallback keeps older snapshots useful, even though they predate the
        // timestamp column.
        db.execute_batch(
            "INSERT INTO progress.glyph_srs(
                 glyph, ease, interval_days, due_epoch, reps, lapses,
                 introduced_epoch, last_grade, updated_epoch)
             SELECT glyph, ease, interval_days, due_epoch, reps, lapses,
                    introduced_epoch, last_grade, updated_epoch
             FROM sync.glyph_srs WHERE true
             ON CONFLICT(glyph) DO UPDATE SET
                ease=excluded.ease, interval_days=excluded.interval_days,
                due_epoch=excluded.due_epoch, reps=excluded.reps,
                lapses=excluded.lapses, last_grade=excluded.last_grade,
                updated_epoch=excluded.updated_epoch
             WHERE excluded.updated_epoch > progress.glyph_srs.updated_epoch
                OR (excluded.updated_epoch = progress.glyph_srs.updated_epoch
                    AND excluded.reps > progress.glyph_srs.reps);

             INSERT INTO progress.word_srs(
                 surface, surface_id, ease, interval_days, due_epoch, reps,
                 lapses, introduced_epoch, last_grade, updated_epoch)
             SELECT surface, surface_id, ease, interval_days, due_epoch, reps,
                    lapses, introduced_epoch, last_grade, updated_epoch
             FROM sync.word_srs WHERE true
             ON CONFLICT(surface) DO UPDATE SET
                surface_id=excluded.surface_id, ease=excluded.ease,
                interval_days=excluded.interval_days, due_epoch=excluded.due_epoch,
                reps=excluded.reps, lapses=excluded.lapses,
                last_grade=excluded.last_grade, updated_epoch=excluded.updated_epoch
             WHERE excluded.updated_epoch > progress.word_srs.updated_epoch
                OR (excluded.updated_epoch = progress.word_srs.updated_epoch
                    AND excluded.reps > progress.word_srs.reps);

             INSERT INTO progress.form_srs(
                 surface, surface_id, ease, interval_days, due_epoch, reps,
                 lapses, introduced_epoch, last_grade, updated_epoch)
             SELECT surface, surface_id, ease, interval_days, due_epoch, reps,
                    lapses, introduced_epoch, last_grade, updated_epoch
             FROM sync.form_srs WHERE true
             ON CONFLICT(surface) DO UPDATE SET
                surface_id=excluded.surface_id, ease=excluded.ease,
                interval_days=excluded.interval_days, due_epoch=excluded.due_epoch,
                reps=excluded.reps, lapses=excluded.lapses,
                last_grade=excluded.last_grade, updated_epoch=excluded.updated_epoch
             WHERE excluded.updated_epoch > progress.form_srs.updated_epoch
                OR (excluded.updated_epoch = progress.form_srs.updated_epoch
                    AND excluded.reps > progress.form_srs.reps);

             INSERT INTO progress.suffix_srs(
                 key, ease, interval_days, due_epoch, reps, lapses,
                 introduced_epoch, last_grade, updated_epoch)
             SELECT key, ease, interval_days, due_epoch, reps, lapses,
                    introduced_epoch, last_grade, updated_epoch
             FROM sync.suffix_srs WHERE true
             ON CONFLICT(key) DO UPDATE SET
                ease=excluded.ease, interval_days=excluded.interval_days,
                due_epoch=excluded.due_epoch, reps=excluded.reps,
                lapses=excluded.lapses, last_grade=excluded.last_grade,
                updated_epoch=excluded.updated_epoch
             WHERE excluded.updated_epoch > progress.suffix_srs.updated_epoch
                OR (excluded.updated_epoch = progress.suffix_srs.updated_epoch
                    AND excluded.reps > progress.suffix_srs.reps);

             INSERT INTO progress.concepts_seen(concept, introduced_epoch)
             SELECT concept, introduced_epoch FROM sync.concepts_seen WHERE true
             ON CONFLICT(concept) DO UPDATE SET introduced_epoch=MIN(
                progress.concepts_seen.introduced_epoch, excluded.introduced_epoch);
             INSERT INTO progress.concepts_unlocked(concept, unlocked_epoch)
             SELECT concept, unlocked_epoch FROM sync.concepts_unlocked WHERE true
             ON CONFLICT(concept) DO UPDATE SET unlocked_epoch=MIN(
                progress.concepts_unlocked.unlocked_epoch, excluded.unlocked_epoch);
             INSERT INTO progress.marks_seen(mark, introduced_epoch)
             SELECT mark, introduced_epoch FROM sync.marks_seen WHERE true
             ON CONFLICT(mark) DO UPDATE SET introduced_epoch=MIN(
                progress.marks_seen.introduced_epoch, excluded.introduced_epoch);

             INSERT INTO progress.reviews(epoch, day, track, grade)
             SELECT r.epoch, r.day, r.track, r.grade FROM sync.reviews r
             WHERE NOT EXISTS (
                SELECT 1 FROM progress.reviews p WHERE p.epoch=r.epoch
                  AND p.day=r.day AND p.track=r.track AND p.grade=r.grade);

             INSERT INTO progress.meta(key, value)
             SELECT key, value FROM sync.meta WHERE key IN ('intro.letters', 'intro.words')
             ON CONFLICT(key) DO UPDATE SET value=MAX(
                CAST(progress.meta.value AS INTEGER), CAST(excluded.value AS INTEGER));",
        )?;
        db.execute_batch(
            "INSERT INTO progress.gloss_overrides(
                 surface, gloss, note, updated_epoch, deleted)
             SELECT surface, gloss, note, updated_epoch, deleted
             FROM sync.gloss_overrides WHERE true
             ON CONFLICT(surface) DO UPDATE SET
                gloss=excluded.gloss, note=excluded.note,
                updated_epoch=excluded.updated_epoch, deleted=excluded.deleted
             WHERE excluded.updated_epoch > progress.gloss_overrides.updated_epoch;",
        )?;
        db.execute_batch(
            "INSERT INTO progress.issue_reports(
                 id, report_type, note, context_json, created_epoch, updated_epoch, deleted)
             SELECT id, report_type, note, context_json, created_epoch, updated_epoch, deleted
             FROM sync.issue_reports WHERE true
             ON CONFLICT(id) DO UPDATE SET
                report_type=excluded.report_type, note=excluded.note,
                context_json=excluded.context_json,
                created_epoch=MIN(
                    progress.issue_reports.created_epoch,
                    excluded.created_epoch
                ),
                updated_epoch=excluded.updated_epoch,
                deleted=excluded.deleted
             WHERE excluded.updated_epoch > progress.issue_reports.updated_epoch;",
        )?;
        db.execute_batch(
            "INSERT INTO progress.lexicon_entry_overrides(
                 surface, root, gloss, updated_epoch)
             SELECT surface, root, gloss, updated_epoch
             FROM sync.lexicon_entry_overrides WHERE true
             ON CONFLICT(surface) DO UPDATE SET
                root=excluded.root, gloss=excluded.gloss,
                updated_epoch=excluded.updated_epoch
             WHERE excluded.updated_epoch >
                   progress.lexicon_entry_overrides.updated_epoch;",
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => db.execute_batch("COMMIT"),
        Err(error) => {
            let _ = db.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

/// Check a received HTTP body before writing it as a SQLite snapshot.
pub fn is_sqlite_snapshot(bytes: &[u8]) -> bool {
    bytes.starts_with(SQLITE_HEADER)
}

/// Read the pending mobile tutor corrections from a canonical sync database.
/// A pre-admin database simply has no corrections yet.
pub fn read_gloss_overrides_file(path: &Path) -> rusqlite::Result<Vec<GlossOverride>> {
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='gloss_overrides')",
        [],
        |r| r.get(0),
    )?;
    if !exists {
        return Ok(Vec::new());
    }
    let active_filter = if has_column(&db, "main", "gloss_overrides", "deleted")? {
        "WHERE deleted = 0"
    } else {
        ""
    };
    let mut statement = db.prepare(&format!(
        "SELECT surface, gloss, note, updated_epoch FROM gloss_overrides
         {active_filter} ORDER BY updated_epoch, surface"
    ))?;
    statement
        .query_map([], |r| {
            Ok(GlossOverride {
                surface: r.get(0)?,
                gloss: r.get(1)?,
                note: r.get(2)?,
                updated_epoch: r.get(3)?,
            })
        })?
        .collect()
}

/// Read the app issue/idea log from a canonical sync database. Older sync
/// databases simply return an empty log.
pub fn read_issue_reports_file(path: &Path) -> rusqlite::Result<Vec<IssueReport>> {
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    if !has_issue_reports(&db)? {
        return Ok(Vec::new());
    }
    let active_filter = if has_column(&db, "main", "issue_reports", "deleted")? {
        " WHERE deleted = 0"
    } else {
        ""
    };
    let mut statement = db.prepare(&format!(
        "SELECT id, report_type, note, context_json, created_epoch, updated_epoch
         FROM issue_reports{active_filter} ORDER BY created_epoch, id",
    ))?;
    statement
        .query_map([], |row| {
            Ok(IssueReport {
                id: row.get(0)?,
                report_type: row.get(1)?,
                note: row.get(2)?,
                context_json: row.get(3)?,
                created_epoch: row.get(4)?,
                updated_epoch: row.get(5)?,
            })
        })?
        .collect()
}

/// Mark issue reports as resolved in a writable progress snapshot. Resolved
/// rows remain as tombstones so a stale phone snapshot cannot reintroduce them
/// during the next normal progress sync.
pub fn resolve_issue_reports_file(
    path: &Path,
    ids: &[String],
    updated_epoch: i64,
) -> rusqlite::Result<usize> {
    let mut db = Connection::open(path)?;
    ensure_issue_reports(&db, "main")?;
    let transaction = db.transaction()?;
    let mut resolved = 0;
    for id in ids {
        resolved += transaction.execute(
            "UPDATE issue_reports
             SET deleted = 1, updated_epoch = MAX(updated_epoch + 1, ?2)
             WHERE id = ?1 AND deleted = 0",
            rusqlite::params![id, updated_epoch],
        )?;
    }
    transaction.commit()?;
    Ok(resolved)
}

fn has_issue_reports(db: &Connection) -> rusqlite::Result<bool> {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='issue_reports')",
        [],
        |row| row.get(0),
    )
}

/// Whether a progress snapshot was created by a core version that knows how
/// to store issue reports. This lets admin tooling distinguish a genuinely
/// empty report log from an old sync server that silently discarded the table.
pub fn has_issue_reports_file(path: &Path) -> rusqlite::Result<bool> {
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    has_issue_reports(&db)
}

/// Count uploaded issue reports that the server's merged response did not
/// acknowledge at the same or a newer revision. Ordinary learning progress can
/// still be merged locally before callers surface this compatibility failure.
pub fn unmerged_issue_report_count(uploaded: &Path, merged: &Path) -> rusqlite::Result<usize> {
    let merged_updates = read_issue_reports_file(merged)?
        .into_iter()
        .map(|report| (report.id, report.updated_epoch))
        .collect::<HashMap<_, _>>();
    Ok(read_issue_reports_file(uploaded)?
        .into_iter()
        .filter(|report| {
            merged_updates
                .get(&report.id)
                .is_none_or(|updated| *updated < report.updated_epoch)
        })
        .count())
}

/// Read mobile word-info root/header corrections from a canonical sync DB.
pub fn read_lexicon_entry_overrides_file(
    path: &Path,
) -> rusqlite::Result<Vec<LexiconEntryOverride>> {
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let exists: bool = db.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type='table' AND name='lexicon_entry_overrides'
        )",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        return Ok(Vec::new());
    }
    let mut statement = db.prepare(
        "SELECT surface, root, gloss, updated_epoch
         FROM lexicon_entry_overrides ORDER BY updated_epoch, surface",
    )?;
    statement
        .query_map([], |row| {
            Ok(LexiconEntryOverride {
                surface: row.get(0)?,
                root: row.get(1)?,
                gloss: row.get(2)?,
                updated_epoch: row.get(3)?,
            })
        })?
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use std::fs;

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("haqor-sync-{name}-{}", std::process::id()))
    }

    #[test]
    fn later_review_wins_and_concepts_are_unioned() -> rusqlite::Result<()> {
        let canonical = temp_path("canonical.db");
        let incoming = temp_path("incoming.db");
        let _ = fs::remove_file(&canonical);
        let _ = fs::remove_file(&incoming);
        for (path, grade, updated, concept) in [
            (&canonical, 2, 100_i64, "intro_rtl"),
            (&incoming, 0, 200_i64, "intro_vowels"),
        ] {
            let db = Connection::open_in_memory()?;
            db.execute(
                "ATTACH DATABASE ?1 AS progress",
                [path.to_string_lossy().as_ref()],
            )?;
            init_progress_schema(&db)?;
            db.execute(
                "INSERT INTO progress.word_srs(surface, surface_id, ease, interval_days,
                    due_epoch, reps, lapses, introduced_epoch, last_grade, updated_epoch)
                 VALUES ('דָּבָר', 7, 2.5, 1, 1, 3, 0, 1, ?1, ?2)",
                params![grade, updated],
            )?;
            db.execute(
                "INSERT INTO progress.concepts_seen(concept, introduced_epoch) VALUES (?1, 1)",
                [concept],
            )?;
        }
        merge_progress_files(&canonical, &incoming)?;
        let db = Connection::open(&canonical)?;
        assert_eq!(
            db.query_row(
                "SELECT last_grade FROM word_srs WHERE surface='דָּבָר'",
                [],
                |r| r.get::<_, i64>(0)
            )?,
            0
        );
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM concepts_seen", [], |r| r
                .get::<_, i64>(0))?,
            2
        );
        let _ = fs::remove_file(&canonical);
        let _ = fs::remove_file(&incoming);
        Ok(())
    }

    #[test]
    fn export_creates_a_standalone_sqlite_snapshot() -> anyhow::Result<()> {
        let source = temp_path("export-source.db");
        let snapshot = temp_path("export-snapshot.db");
        let _ = fs::remove_file(&source);
        let _ = fs::remove_file(&snapshot);
        let db = Connection::open_in_memory()?;
        db.execute(
            "ATTACH DATABASE ?1 AS progress",
            [source.to_string_lossy().as_ref()],
        )?;
        init_progress_schema(&db)?;
        db.execute(
            "INSERT INTO progress.concepts_seen(concept, introduced_epoch) VALUES ('intro_rtl', 1)",
            [],
        )?;
        export_progress_snapshot(&db, &snapshot)?;
        assert!(is_sqlite_snapshot(&fs::read(&snapshot)?));
        let copy = Connection::open(&snapshot)?;
        assert_eq!(
            copy.query_row("SELECT COUNT(*) FROM concepts_seen", [], |r| r
                .get::<_, i64>(0))?,
            1
        );
        let _ = fs::remove_file(&source);
        let _ = fs::remove_file(&snapshot);
        Ok(())
    }

    #[test]
    fn newer_gloss_override_wins_and_is_readable_from_the_canonical_db() -> anyhow::Result<()> {
        let canonical = temp_path("canonical-gloss.db");
        let incoming = temp_path("incoming-gloss.db");
        let _ = fs::remove_file(&canonical);
        let _ = fs::remove_file(&incoming);
        for (path, gloss, updated) in [
            (&canonical, "older", 100_i64),
            (&incoming, "newer", 200_i64),
        ] {
            let db = Connection::open_in_memory()?;
            db.execute(
                "ATTACH DATABASE ?1 AS progress",
                [path.to_string_lossy().as_ref()],
            )?;
            init_progress_schema(&db)?;
            db.execute(
                "INSERT INTO progress.gloss_overrides(surface, gloss, note, updated_epoch)
                 VALUES ('דָּבָר', ?1, '', ?2)",
                params![gloss, updated],
            )?;
        }
        merge_progress_files(&canonical, &incoming)?;
        assert_eq!(
            read_gloss_overrides_file(&canonical)?,
            vec![GlossOverride {
                surface: "דָּבָר".to_string(),
                gloss: "newer".to_string(),
                note: String::new(),
                updated_epoch: 200,
            }]
        );
        let _ = fs::remove_file(&canonical);
        let _ = fs::remove_file(&incoming);
        Ok(())
    }

    #[test]
    fn gloss_override_tombstone_syncs_and_is_not_exported_for_review() -> anyhow::Result<()> {
        let canonical = temp_path("canonical-gloss-delete.db");
        let incoming = temp_path("incoming-gloss-delete.db");
        let _ = fs::remove_file(&canonical);
        let _ = fs::remove_file(&incoming);
        for (path, deleted, updated) in [(&canonical, 0_i64, 100_i64), (&incoming, 1_i64, 200_i64)]
        {
            let db = Connection::open_in_memory()?;
            db.execute(
                "ATTACH DATABASE ?1 AS progress",
                [path.to_string_lossy().as_ref()],
            )?;
            init_progress_schema(&db)?;
            db.execute(
                "INSERT INTO progress.gloss_overrides(
                     surface, gloss, note, updated_epoch, deleted)
                 VALUES ('דָּבָר', ?1, '', ?2, ?3)",
                params![if deleted == 0 { "matter" } else { "" }, updated, deleted],
            )?;
        }

        merge_progress_files(&canonical, &incoming)?;
        assert!(read_gloss_overrides_file(&canonical)?.is_empty());
        let db = Connection::open(&canonical)?;
        assert_eq!(
            db.query_row(
                "SELECT deleted FROM gloss_overrides WHERE surface = 'דָּבָר'",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            1
        );
        let _ = fs::remove_file(&canonical);
        let _ = fs::remove_file(&incoming);
        Ok(())
    }

    #[test]
    fn issue_reports_are_unioned_and_newer_edits_win() -> anyhow::Result<()> {
        let canonical = temp_path("canonical-issues.db");
        let incoming = temp_path("incoming-issues.db");
        let _ = fs::remove_file(&canonical);
        let _ = fs::remove_file(&incoming);
        for (path, id, note, created, updated) in [
            (&canonical, "device-a-1", "old note", 100_i64, 100_i64),
            (&incoming, "device-a-1", "new note", 100_i64, 200_i64),
        ] {
            let db = Connection::open_in_memory()?;
            db.execute(
                "ATTACH DATABASE ?1 AS progress",
                [path.to_string_lossy().as_ref()],
            )?;
            init_progress_schema(&db)?;
            db.execute(
                "INSERT INTO progress.issue_reports(
                     id, report_type, note, context_json, created_epoch, updated_epoch)
                 VALUES (?1, 'bug', ?2, '{\"source\":\"tutor\"}', ?3, ?4)",
                params![id, note, created, updated],
            )?;
        }
        {
            let db = Connection::open(&incoming)?;
            db.execute(
                "INSERT INTO issue_reports(
                     id, report_type, note, context_json, created_epoch, updated_epoch)
                 VALUES ('device-b-1', 'idea', 'another report',
                         '{\"source\":\"word_info\"}', 150, 150)",
                [],
            )?;
        }

        merge_progress_files(&canonical, &incoming)?;
        let reports = read_issue_reports_file(&canonical)?;
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].id, "device-a-1");
        assert_eq!(reports[0].note, "new note");
        assert_eq!(reports[0].updated_epoch, 200);
        assert_eq!(reports[1].id, "device-b-1");
        assert_eq!(reports[1].report_type, "idea");

        let _ = fs::remove_file(&canonical);
        let _ = fs::remove_file(&incoming);
        Ok(())
    }

    #[test]
    fn resolved_issue_report_tombstone_syncs_and_hides_the_report() -> anyhow::Result<()> {
        let canonical = temp_path("canonical-issue-delete.db");
        let incoming = temp_path("incoming-issue-delete.db");
        let _ = fs::remove_file(&canonical);
        let _ = fs::remove_file(&incoming);
        for path in [&canonical, &incoming] {
            let db = Connection::open_in_memory()?;
            db.execute(
                "ATTACH DATABASE ?1 AS progress",
                [path.to_string_lossy().as_ref()],
            )?;
            init_progress_schema(&db)?;
            db.execute(
                "INSERT INTO progress.issue_reports(
                     id, report_type, note, context_json, created_epoch, updated_epoch)
                 VALUES ('phone-1', 'bug', 'already fixed', '{}', 100, 100)",
                [],
            )?;
        }

        assert_eq!(
            resolve_issue_reports_file(&incoming, &["phone-1".to_string()], 200)?,
            1
        );
        merge_progress_files(&canonical, &incoming)?;
        assert!(read_issue_reports_file(&canonical)?.is_empty());
        let db = Connection::open(&canonical)?;
        assert_eq!(
            db.query_row(
                "SELECT deleted FROM issue_reports WHERE id = 'phone-1'",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            1
        );

        let _ = fs::remove_file(&canonical);
        let _ = fs::remove_file(&incoming);
        Ok(())
    }

    #[test]
    fn old_server_snapshot_does_not_acknowledge_issue_reports() -> anyhow::Result<()> {
        let uploaded = temp_path("uploaded-issues.db");
        let legacy_response = temp_path("legacy-response.db");
        let _ = fs::remove_file(&uploaded);
        let _ = fs::remove_file(&legacy_response);
        {
            let db = Connection::open_in_memory()?;
            db.execute(
                "ATTACH DATABASE ?1 AS progress",
                [uploaded.to_string_lossy().as_ref()],
            )?;
            init_progress_schema(&db)?;
            db.execute(
                "INSERT INTO progress.issue_reports(
                     id, report_type, note, context_json, created_epoch, updated_epoch)
                 VALUES ('phone-1', 'bug', 'Missing report', '{}', 100, 100)",
                [],
            )?;
        }
        Connection::open(&legacy_response)?
            .execute_batch("CREATE TABLE legacy_progress(value INTEGER NOT NULL);")?;

        assert!(has_issue_reports_file(&uploaded)?);
        assert!(!has_issue_reports_file(&legacy_response)?);
        assert_eq!(unmerged_issue_report_count(&uploaded, &legacy_response)?, 1);

        merge_progress_files(&legacy_response, &uploaded)?;
        assert!(has_issue_reports_file(&legacy_response)?);
        assert_eq!(unmerged_issue_report_count(&uploaded, &legacy_response)?, 0);
        let _ = fs::remove_file(&uploaded);
        let _ = fs::remove_file(&legacy_response);
        Ok(())
    }

    #[test]
    fn lexicon_entry_overrides_sync_and_are_readable() -> anyhow::Result<()> {
        let canonical = temp_path("canonical-lexicon-entry.db");
        let incoming = temp_path("incoming-lexicon-entry.db");
        let _ = fs::remove_file(&canonical);
        let _ = fs::remove_file(&incoming);
        for (path, root, gloss, updated) in [
            (&canonical, "דבר", "word", 100_i64),
            (&incoming, "דבר", "matter", 200_i64),
        ] {
            let db = Connection::open_in_memory()?;
            db.execute(
                "ATTACH DATABASE ?1 AS progress",
                [path.to_string_lossy().as_ref()],
            )?;
            init_progress_schema(&db)?;
            db.execute(
                "INSERT INTO progress.lexicon_entry_overrides(
                     surface, root, gloss, updated_epoch)
                 VALUES ('דָּבָר', ?1, ?2, ?3)",
                params![root, gloss, updated],
            )?;
        }

        merge_progress_files(&canonical, &incoming)?;
        assert_eq!(
            read_lexicon_entry_overrides_file(&canonical)?,
            vec![LexiconEntryOverride {
                surface: "דָּבָר".to_string(),
                root: "דבר".to_string(),
                gloss: "matter".to_string(),
                updated_epoch: 200,
            }]
        );

        let _ = fs::remove_file(&canonical);
        let _ = fs::remove_file(&incoming);
        Ok(())
    }
}
