//! Post-processing for the legacy Python-generated `haqor.db`.
//!
//! The only legacy data still consumed at runtime is
//! `Bible::ot_root_occurrences`: the OT verses sharing a consonantal root, shown
//! alongside an NT (SEDRA) word's root tree. Everything else in the 77 MB
//! Python-built file — the `bdb`/`bdb_aramaic` lexicons (replaced by
//! `lexicon.db`), the `words`/`words_aramaic` morphology (replaced by the
//! `hebrew.db` reverse-parse engine), per-surface occurrence lookups and the
//! `hebrew`/`syriac`/`count`/`sedra`/`lex_consonants` tables — is dead code
//! paths or duplicated by the Rust-generated databases.
//!
//! `db update` therefore reduces the copied file to a single precomputed
//! table, one row per distinct (root, OT verse) pair:
//!
//! ```sql
//! root_occurrences(root, book, chapter, verse)  -- PK, WITHOUT ROWID
//! ```
//!
//! `root` is folded from final to medial letter forms at build time so it
//! matches `transliterate::lookup_key` output directly at query time.

use std::path::Path;

use anyhow::{Context, Result, bail};
use log::info;
use rusqlite::Connection;

/// Shrink a freshly copied `haqor.db` in place, reducing it to the
/// `root_occurrences` table described in the module docs.
///
/// Idempotent: a file that already has `root_occurrences` is left untouched.
/// Returns the size of the file in bytes after shrinking.
pub fn shrink_haqor(db_path: &Path) -> Result<u64> {
    let db = Connection::open(db_path).with_context(|| format!("opening {}", db_path.display()))?;

    let migrated: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema \
         WHERE type = 'table' AND name = 'root_occurrences')",
        [],
        |row| row.get(0),
    )?;
    if migrated {
        info!("{} already shrunk, leaving as is", db_path.display());
        return Ok(std::fs::metadata(db_path)?.len());
    }

    db.execute_batch(
        "BEGIN;
         CREATE TABLE root_occurrences(
             root    TEXT NOT NULL,
             book    INT NOT NULL,
             chapter INT NOT NULL,
             verse   INT NOT NULL,
             PRIMARY KEY(root, book, chapter, verse)
         ) WITHOUT ROWID;
         -- Final -> medial letter folding (ך ם ן ף ץ), matching the fold
         -- lookup_key applies to the SEDRA root on the query side.
         INSERT OR IGNORE INTO root_occurrences
             SELECT replace(replace(replace(replace(replace(
                        w.root, char(1498), char(1499)), char(1501), char(1502)),
                        char(1503), char(1504)), char(1507), char(1508)),
                        char(1509), char(1510)),
                    o.book, o.chapter, o.verse
             FROM occurrences o JOIN words w ON w.raw = o.raw
             WHERE o.book < 40;
         DROP TABLE occurrences;
         DROP TABLE words;
         DROP TABLE words_aramaic;
         DROP TABLE bdb;
         DROP TABLE bdb_aramaic;
         DROP TABLE IF EXISTS hebrew;
         DROP TABLE IF EXISTS syriac;
         DROP TABLE IF EXISTS \"count\";
         DROP TABLE IF EXISTS sedra;
         DROP TABLE IF EXISTS lex_consonants;
         COMMIT;",
    )?;

    let rows: i64 = db.query_row("SELECT COUNT(*) FROM root_occurrences", [], |row| {
        row.get(0)
    })?;
    let roots: i64 = db.query_row(
        "SELECT COUNT(DISTINCT root) FROM root_occurrences",
        [],
        |row| row.get(0),
    )?;
    if rows == 0 {
        bail!("root_occurrences came out empty");
    }

    db.execute_batch("VACUUM;")?;
    drop(db);

    let bytes = std::fs::metadata(db_path)?.len();
    info!(
        "Shrunk {}: {rows} root/verse rows over {roots} roots",
        db_path.display(),
    );
    Ok(bytes)
}
