//! Hebrew database generator (incremental).
//!
//! Builds a standalone `hebrew.db` for the Old Testament from what the
//! algorithmic morphology engine can derive today: every OT word token is
//! reverse-parsed ([`crate::morphology::parse_word`]) into candidate verb
//! analyses. The schema deliberately mirrors the spirit of the SEDRA db
//! (`roots` / `words` / `occurrences`) but is built to grow: the bits the
//! engine can't yet account for are not dropped, they are recorded so they can
//! be reviewed and the engine improved over successive passes.
//!
//! Tables:
//! - `surface`    — one row per distinct OT token: its text, how often it
//!                  occurs, how many candidate analyses it has, and whether any
//!                  is attested. This is the review backbone.
//! - `occurrences`— one row per token position (surface_id → book/chapter/verse),
//!                  mirroring SEDRA's occurrences table.
//! - `analyses`   — one row per candidate analysis (root/binyan/form/pgn/…);
//!                  empty for unparsed surfaces, one for unambiguous, many for
//!                  homographs.
//! - `roots`      — distinct roots seen, with gizra and frequency.
//!
//! Two views make the gaps reviewable, frequency-ranked so the highest-impact
//! cases come first:
//! - `review_missing`   — surfaces with no analysis (the "missing bits").
//! - `review_ambiguous` — surfaces with >1 candidate, joined to their analyses
//!                        (the "multi-candidate roots").
//!
//! The DB is fully regenerable from the engine; manual review is meant to drive
//! fixes in the morphology code (or a future lexeme inventory), not hand-edits
//! to the data.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use log::info;
use rayon::prelude::*;
use rusqlite::Connection;

use crate::morphology::{VerbMatch, parse_word};

/// OT books are 1..=39 in Haqor numbering; NT starts at 40.
const NT_FIRST_BOOK: u8 = 40;

/// Hebrew maqaf joins two orthographic words; split on it so each is parsed
/// independently.
const MAQAF: char = '\u{05BE}';

fn tokenize(words: &str) -> impl Iterator<Item = &str> {
    words
        .split(|c: char| c.is_whitespace() || c == MAQAF)
        .filter(|t| !t.is_empty())
}

fn has_hebrew_letter(token: &str) -> bool {
    token
        .chars()
        .any(|c| (0x05D0..=0x05EA).contains(&(c as u32)))
}

/// A single OT token position.
struct Occurrence {
    surface_id: usize,
    book: u8,
    chapter: u8,
    verse: u8,
}

/// Read every OT token from `bible.db`, returning the distinct surface forms
/// (in first-seen order) and the full list of occurrences referencing them by
/// index.
fn collect_tokens(bible_db: &Path) -> Result<(Vec<String>, Vec<Occurrence>)> {
    let db =
        Connection::open(bible_db).with_context(|| format!("opening {}", bible_db.display()))?;
    let mut stmt = db.prepare(&format!(
        "SELECT book, chapter, verse, words FROM bible \
         WHERE book < {NT_FIRST_BOOK} ORDER BY book, chapter, verse"
    ))?;

    let mut index: HashMap<String, usize> = HashMap::new();
    let mut surfaces: Vec<String> = Vec::new();
    let mut occurrences: Vec<Occurrence> = Vec::new();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, u8>(0)?,
            row.get::<_, u8>(1)?,
            row.get::<_, u8>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    for row in rows {
        let (book, chapter, verse, words) = row?;
        for token in tokenize(&words) {
            if !has_hebrew_letter(token) {
                continue;
            }
            let surface_id = match index.get(token) {
                Some(&id) => id,
                None => {
                    let id = surfaces.len();
                    index.insert(token.to_owned(), id);
                    surfaces.push(token.to_owned());
                    id
                }
            };
            occurrences.push(Occurrence {
                surface_id,
                book,
                chapter,
                verse,
            });
        }
    }

    Ok((surfaces, occurrences))
}

/// Render a root's gizra classes to a stable comma-separated string.
fn gizra_label(m: &VerbMatch) -> String {
    m.root
        .classes
        .iter()
        .map(|g| format!("{g:?}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn create_schema(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE surface(
            surface_id   INTEGER PRIMARY KEY,
            text         TEXT    NOT NULL,
            occurrences  INTEGER NOT NULL,
            n_candidates INTEGER NOT NULL,
            parsed       INTEGER NOT NULL,
            attested     INTEGER NOT NULL
         );
         CREATE TABLE occurrences(
            surface_id INTEGER NOT NULL,
            book       INTEGER NOT NULL,
            chapter    INTEGER NOT NULL,
            verse      INTEGER NOT NULL
         );
         CREATE TABLE analyses(
            analysis_id     INTEGER PRIMARY KEY,
            surface_id      INTEGER NOT NULL,
            root            TEXT    NOT NULL,
            gizra           TEXT    NOT NULL,
            binyan          TEXT    NOT NULL,
            form            TEXT    NOT NULL,
            pgn             TEXT    NOT NULL,
            prefix          TEXT    NOT NULL,
            vav_consecutive INTEGER NOT NULL,
            attested        INTEGER NOT NULL
         );
         CREATE TABLE roots(
            root          TEXT PRIMARY KEY,
            gizra         TEXT NOT NULL,
            n_forms       INTEGER NOT NULL,
            n_occurrences INTEGER NOT NULL
         );",
    )?;
    Ok(())
}

fn create_indexes_and_views(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE INDEX idx_occurrences_surface ON occurrences(surface_id);
         CREATE INDEX idx_analyses_surface ON analyses(surface_id);
         CREATE INDEX idx_analyses_root ON analyses(root);

         CREATE VIEW review_missing AS
            SELECT surface_id, text, occurrences
            FROM surface
            WHERE n_candidates = 0
            ORDER BY occurrences DESC;

         CREATE VIEW review_ambiguous AS
            SELECT s.surface_id, s.text, s.occurrences, s.n_candidates,
                   a.root, a.gizra, a.binyan, a.form, a.pgn,
                   a.prefix, a.vav_consecutive, a.attested
            FROM surface s
            JOIN analyses a ON a.surface_id = s.surface_id
            WHERE s.n_candidates > 1
            ORDER BY s.occurrences DESC, s.surface_id, a.attested DESC;",
    )?;
    Ok(())
}

/// Build `hebrew.db` from `bible.db`. Returns (distinct surfaces, occurrences,
/// parsed surfaces).
pub fn generate_hebrew(bible_db: &Path, output: &Path) -> Result<(usize, usize, usize)> {
    info!("Reading OT tokens from {}", bible_db.display());
    let (surfaces, occurrences) = collect_tokens(bible_db)?;
    info!(
        "  {} distinct surfaces, {} occurrences",
        surfaces.len(),
        occurrences.len()
    );

    // Reverse-parse every distinct surface in parallel; parse_word is pure.
    info!("Reverse-parsing {} distinct surfaces", surfaces.len());
    let analyses: Vec<Vec<VerbMatch>> = surfaces.par_iter().map(|t| parse_word(t)).collect();

    // Occurrence counts per surface.
    let mut counts = vec![0u32; surfaces.len()];
    for occ in &occurrences {
        counts[occ.surface_id] += 1;
    }

    if output.exists() {
        std::fs::remove_file(output)
            .with_context(|| format!("removing existing {}", output.display()))?;
    }
    let mut db =
        Connection::open(output).with_context(|| format!("opening {}", output.display()))?;
    create_schema(&db)?;

    let parsed_surfaces = analyses.iter().filter(|a| !a.is_empty()).count();

    let tx = db.transaction()?;
    {
        let mut surf_stmt = tx.prepare(
            "INSERT INTO surface(surface_id, text, occurrences, n_candidates, parsed, attested) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        let mut ana_stmt = tx.prepare(
            "INSERT INTO analyses(surface_id, root, gizra, binyan, form, pgn, prefix, \
             vav_consecutive, attested) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        for (id, matches) in analyses.iter().enumerate() {
            let any_attested = matches.iter().any(|m| m.attested);
            surf_stmt.execute((
                id as i64,
                &surfaces[id],
                counts[id] as i64,
                matches.len() as i64,
                (!matches.is_empty()) as i64,
                any_attested as i64,
            ))?;
            for m in matches {
                let root: String = m.root.letters.iter().collect();
                ana_stmt.execute((
                    id as i64,
                    root,
                    gizra_label(m),
                    m.binyan.name(),
                    m.form.name(),
                    m.pgn.label(),
                    &m.prefix,
                    m.vav_consecutive as i64,
                    m.attested as i64,
                ))?;
            }
        }

        let mut occ_stmt =
            tx.prepare("INSERT INTO occurrences(surface_id, book, chapter, verse) VALUES (?1, ?2, ?3, ?4)")?;
        for occ in &occurrences {
            occ_stmt.execute((
                occ.surface_id as i64,
                occ.book as i64,
                occ.chapter as i64,
                occ.verse as i64,
            ))?;
        }
    }
    tx.commit()?;

    // Roots aggregated from analyses, weighting each distinct surface by its
    // occurrence count (so n_occurrences reflects how often the root is seen,
    // not just how many forms reference it).
    db.execute_batch(
        "INSERT INTO roots(root, gizra, n_forms, n_occurrences)
         SELECT root, gizra, COUNT(*) AS n_forms, SUM(occ) AS n_occurrences
         FROM (
            SELECT a.root AS root, MIN(a.gizra) AS gizra, s.occurrences AS occ
            FROM analyses a
            JOIN surface s ON s.surface_id = a.surface_id
            GROUP BY a.root, a.surface_id
         )
         GROUP BY root;",
    )?;

    create_indexes_and_views(&db)?;

    info!(
        "Wrote {} surfaces ({} parsed), {} occurrences to {}",
        surfaces.len(),
        parsed_surfaces,
        occurrences.len(),
        output.display()
    );
    Ok((surfaces.len(), occurrences.len(), parsed_surfaces))
}
