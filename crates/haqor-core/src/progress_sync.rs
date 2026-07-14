//! Snapshot and merge support for a learner's writable `progress.db`.
//!
//! The corpus databases are identical, read-only app assets.  Only the small
//! progress database is synchronised.  A merge is deliberately monotonic: a
//! card state with the most recently recorded review wins, while one-time
//! concepts and activity records are unioned.  This lets two devices make
//! progress offline and converge when they next meet the LAN server.

use std::path::Path;

use rusqlite::Connection;

use crate::tutor::init_progress_schema;

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

fn merge_attached_snapshot(db: &Connection) -> rusqlite::Result<()> {
    ensure_updated_epochs(db, "progress")?;
    ensure_updated_epochs(db, "sync")?;
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
}
