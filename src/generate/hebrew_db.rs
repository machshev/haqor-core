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
//! - `surface`    — one row per distinct OT token (cantillation-normalised, so
//!                  forms differing only in te'amim collapse together): its
//!                  text, how often it occurs, how many candidate analyses it
//!                  has, and whether any is attested. This is the review
//!                  backbone.
//! - `occurrences`— one row per token position (surface_id → book/chapter/verse),
//!                  mirroring SEDRA's occurrences table.
//! - `analyses`   — one row per candidate verb analysis (root/binyan/form/pgn/…);
//!                  empty for unparsed surfaces, one for unambiguous, many for
//!                  homographs.
//! - `noun_analyses` — one row per candidate noun analysis (stem/class/inflected
//!                  slot/prefix), produced by reverse-parsing each surface
//!                  against the lexicon's common-, adjective- and proper-noun
//!                  paradigms. Independent of the verb pass, so a true homograph
//!                  carries rows in both. Only populated when a lexicon is given.
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

use crate::morphology::{NounInventory, NounMatch, VerbMatch, parse_word};

use super::lexicon_db::{load_noun_inventory, load_proper_inventory};
use super::prefilter::Prefilter;

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

/// Canonical combining class for the niqqud points [`normalize_surface`] keeps;
/// base letters and anything else return 0 (a non-combining boundary). Used to
/// reorder marks within a cluster so source forms that differ only in mark order
/// (e.g. shin-dot before vs. after a vowel) collapse to one surface.
fn combining_class(c: char) -> u8 {
    match c as u32 {
        0x05B0 => 10,
        0x05B1 => 11,
        0x05B2 => 12,
        0x05B3 => 13,
        0x05B4 => 14,
        0x05B5 => 15,
        0x05B6 => 16,
        0x05B7 => 17,
        0x05B8 | 0x05C7 => 18,
        0x05B9 => 19,
        0x05BB => 20,
        0x05BC => 21,
        0x05C1 => 24,
        0x05C2 => 25,
        _ => 0,
    }
}

/// Reduce a raw token to exactly the characters the morphology parser consumes:
/// the consonants (including final forms) and the niqqud points it recognises.
/// Cantillation (te'amim), meteg, rafe and other marks are dropped, so tokens
/// that differ only in cantillation collapse to a single surface — the parser
/// already ignores those marks, so they never affect the analysis. Marks within
/// each consonant cluster are sorted into canonical (combining-class) order so
/// order variants in the source collapse too. The result is still readable
/// pointed Hebrew and serves as the stored display form.
pub(crate) fn normalize_surface(token: &str) -> String {
    let kept: Vec<char> = token
        .chars()
        .filter(|&c| {
            let n = c as u32;
            (0x05D0..=0x05EA).contains(&n)
                || matches!(
                    n,
                    0x05B0..=0x05B9 | 0x05BB | 0x05BC | 0x05C1 | 0x05C2 | 0x05C7
                )
        })
        .collect();

    // Stable canonical reordering: sort each run of combining marks (ccc > 0)
    // by combining class, leaving base letters as fixed boundaries.
    let mut out = String::with_capacity(kept.len() * 2);
    let mut i = 0;
    while i < kept.len() {
        if combining_class(kept[i]) == 0 {
            out.push(kept[i]);
            i += 1;
        } else {
            let start = i;
            while i < kept.len() && combining_class(kept[i]) != 0 {
                i += 1;
            }
            let mut run = kept[start..i].to_vec();
            run.sort_by_key(|&c| combining_class(c));
            out.extend(run);
        }
    }
    out
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
            let norm = normalize_surface(token);
            if !has_hebrew_letter(&norm) {
                continue;
            }
            let surface_id = match index.get(&norm) {
                Some(&id) => id,
                None => {
                    let id = surfaces.len();
                    index.insert(norm.clone(), id);
                    surfaces.push(norm);
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
            surface_id        INTEGER PRIMARY KEY,
            text              TEXT    NOT NULL,
            occurrences       INTEGER NOT NULL,
            n_candidates      INTEGER NOT NULL,
            n_noun_candidates INTEGER NOT NULL,
            parsed            INTEGER NOT NULL,
            attested          INTEGER NOT NULL,
            lexical_class     TEXT
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
         CREATE TABLE noun_analyses(
            analysis_id INTEGER PRIMARY KEY,
            surface_id  INTEGER NOT NULL,
            stem        TEXT    NOT NULL,
            kind        TEXT    NOT NULL,
            label       TEXT    NOT NULL,
            prefix      TEXT    NOT NULL
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
         CREATE INDEX idx_noun_analyses_surface ON noun_analyses(surface_id);

         CREATE VIEW review_missing AS
            SELECT surface_id, text, occurrences
            FROM surface
            WHERE n_candidates = 0 AND n_noun_candidates = 0
                  AND lexical_class IS NULL
            ORDER BY occurrences DESC;

         CREATE VIEW review_nouns AS
            SELECT s.surface_id, s.text, s.occurrences, s.lexical_class,
                   n.stem, n.kind, n.label, n.prefix
            FROM surface s
            JOIN noun_analyses n ON n.surface_id = s.surface_id
            ORDER BY s.occurrences DESC, s.surface_id, n.stem;

         CREATE VIEW review_ambiguous AS
            SELECT s.surface_id, s.text, s.occurrences, s.n_candidates,
                   a.root, a.gizra, a.binyan, a.form, a.pgn,
                   a.prefix, a.vav_consecutive, a.attested
            FROM surface s
            JOIN analyses a ON a.surface_id = s.surface_id
            WHERE s.n_candidates > 1 AND s.lexical_class IS NULL
            ORDER BY s.occurrences DESC, s.surface_id, a.attested DESC;",
    )?;
    Ok(())
}

/// Build `hebrew.db` from `bible.db`. Returns (distinct surfaces, occurrences,
/// parsed surfaces).
pub fn generate_hebrew(
    bible_db: &Path,
    output: &Path,
    lexicon_db: Option<&Path>,
) -> Result<(usize, usize, usize)> {
    info!("Reading OT tokens from {}", bible_db.display());
    let (surfaces, occurrences) = collect_tokens(bible_db)?;
    info!(
        "  {} distinct surfaces, {} occurrences",
        surfaces.len(),
        occurrences.len()
    );

    // Optional lexical pre-filter: recognise non-verb tokens (closed-class
    // function words + proper nouns) so they bypass the verb parser entirely.
    let prefilter = match lexicon_db {
        Some(p) => Some(Prefilter::load(p)?),
        None => None,
    };
    let lexical: Vec<Option<&'static str>> = surfaces
        .iter()
        .map(|t| prefilter.as_ref().and_then(|pf| pf.classify(t)))
        .collect();

    // Reverse-parse every distinct surface in parallel; parse_word is pure.
    // Function words always bypass the parser. Proper nouns are still parsed:
    // the verb-aware rule lets a name yield to a plausible (attested) verb
    // reading rather than being excluded outright (many names are verb
    // homographs). Unclassified tokens parse normally.
    info!("Reverse-parsing {} distinct surfaces", surfaces.len());
    let raw: Vec<Vec<VerbMatch>> = surfaces
        .par_iter()
        .zip(lexical.par_iter())
        .map(|(t, class)| {
            if *class == Some("function") {
                Vec::new()
            } else {
                parse_word(t)
            }
        })
        .collect();

    // Final exclusion is verb-aware: a proper-noun match yields to the parser
    // when it has a plausible verb reading. The stored `lexical_class` reflects
    // this refined decision, and analyses for still-excluded tokens are dropped.
    let classes: Vec<Option<&'static str>> = surfaces
        .iter()
        .zip(raw.iter())
        .map(|(t, matches)| {
            let has_plausible = matches.iter().any(|m| m.attested);
            prefilter
                .as_ref()
                .and_then(|pf| pf.exclude(t, has_plausible))
        })
        .collect();
    let analyses: Vec<Vec<VerbMatch>> = raw
        .into_iter()
        .zip(classes.iter())
        .map(|(matches, class)| if class.is_some() { Vec::new() } else { matches })
        .collect();
    if prefilter.is_some() {
        let filtered = classes.iter().filter(|c| c.is_some()).count();
        let rescued = lexical
            .iter()
            .zip(classes.iter())
            .filter(|(lex, fin)| **lex == Some("proper") && fin.is_none())
            .count();
        info!(
            "  pre-filtered {filtered} non-verb surfaces (skipped); \
             {rescued} proper-noun surfaces rescued as plausible verbs"
        );
    }

    // Noun pass: when a lexicon is available, reverse-parse every surface
    // against the inflectional paradigms of its common-noun, adjective and
    // proper-noun headwords. This is independent of the verb parser, so a
    // genuine noun/verb homograph keeps both readings; function words (which
    // are never nouns) are skipped, mirroring the verb pass.
    let noun_inventory = match lexicon_db {
        Some(p) => {
            let mut stems = load_noun_inventory(p)?;
            stems.extend(load_proper_inventory(p)?);
            let inv = NounInventory::build(&stems);
            info!(
                "Noun-parsing {} surfaces against {} stems",
                surfaces.len(),
                inv.len()
            );
            Some(inv)
        }
        None => None,
    };
    let noun_analyses: Vec<Vec<NounMatch>> = match &noun_inventory {
        Some(inv) => surfaces
            .par_iter()
            .zip(classes.par_iter())
            .map(|(t, class)| {
                if *class == Some("function") {
                    Vec::new()
                } else {
                    inv.parse(t)
                }
            })
            .collect(),
        None => vec![Vec::new(); surfaces.len()],
    };
    if noun_inventory.is_some() {
        let noun_parsed = noun_analyses.iter().filter(|a| !a.is_empty()).count();
        info!("  {noun_parsed} surfaces matched a noun analysis");
    }

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

    let parsed_surfaces = analyses
        .iter()
        .zip(noun_analyses.iter())
        .filter(|(verb, noun)| !verb.is_empty() || !noun.is_empty())
        .count();

    let tx = db.transaction()?;
    {
        let mut surf_stmt = tx.prepare(
            "INSERT INTO surface(surface_id, text, occurrences, n_candidates, \
             n_noun_candidates, parsed, attested, lexical_class) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        let mut ana_stmt = tx.prepare(
            "INSERT INTO analyses(surface_id, root, gizra, binyan, form, pgn, prefix, \
             vav_consecutive, attested) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        let mut noun_stmt = tx.prepare(
            "INSERT INTO noun_analyses(surface_id, stem, kind, label, prefix) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for (id, matches) in analyses.iter().enumerate() {
            let nouns = &noun_analyses[id];
            let any_attested = matches.iter().any(|m| m.attested);
            surf_stmt.execute((
                id as i64,
                &surfaces[id],
                counts[id] as i64,
                matches.len() as i64,
                nouns.len() as i64,
                (!matches.is_empty() || !nouns.is_empty()) as i64,
                any_attested as i64,
                classes[id],
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
            for n in nouns {
                noun_stmt.execute((
                    id as i64,
                    &n.stem,
                    format!("{:?}", n.kind),
                    &n.label,
                    &n.prefix,
                ))?;
            }
        }

        let mut occ_stmt = tx.prepare(
            "INSERT INTO occurrences(surface_id, book, chapter, verse) VALUES (?1, ?2, ?3, ?4)",
        )?;
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
