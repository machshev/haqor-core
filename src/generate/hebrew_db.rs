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

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use log::info;
use rayon::prelude::*;
use rusqlite::Connection;

use crate::morphology::{
    IrregularVerb, NounInventory, NounMatch, ReverseIndex, VerbMatch, irregular_verb,
    parse_word_filtered, parse_word_indexed,
};

use super::lexicon_db::{load_noun_inventory, load_proper_inventory, load_root_inventory};
use super::prefilter::Prefilter;

/// OT books are 1..=39 in Haqor (Tanakh-order) numbering; NT starts at 40.
const NT_FIRST_BOOK: u8 = 40;

/// Haqor book numbers for the books carrying Biblical Aramaic.
const GENESIS: u8 = 1;
const JEREMIAH: u8 = 13;
const DANIEL: u8 = 35;
const EZRA: u8 = 36;

/// Map a book name or common abbreviation (case-insensitive), or a bare number
/// string, to its Haqor (Tanakh-order) book number 1..=39.
pub fn book_number(token: &str) -> Option<u8> {
    let t = token.trim().to_ascii_lowercase();
    if let Ok(n) = t.parse::<u8>() {
        return (1..=39).contains(&n).then_some(n);
    }
    const BOOKS: &[(u8, &[&str])] = &[
        (1, &["gen", "genesis"]),
        (2, &["exod", "ex", "exodus"]),
        (3, &["lev", "leviticus"]),
        (4, &["num", "numbers"]),
        (5, &["deut", "deuteronomy"]),
        (6, &["josh", "jos", "joshua"]),
        (7, &["judg", "judges"]),
        (8, &["1sam", "1samuel"]),
        (9, &["2sam", "2samuel"]),
        (10, &["1kgs", "1kings"]),
        (11, &["2kgs", "2kings"]),
        (12, &["isa", "is", "isaiah"]),
        (13, &["jer", "jeremiah"]),
        (14, &["ezek", "ez", "ezekiel"]),
        (15, &["hos", "hosea"]),
        (16, &["joel"]),
        (17, &["amos"]),
        (18, &["obad", "obadiah"]),
        (19, &["jonah", "jon"]),
        (20, &["mic", "micah"]),
        (21, &["nah", "nahum"]),
        (22, &["hab", "habakkuk"]),
        (23, &["zeph", "zep", "zephaniah"]),
        (24, &["hag", "haggai"]),
        (25, &["zech", "zec", "zechariah"]),
        (26, &["mal", "malachi"]),
        (27, &["ps", "psa", "psalm", "psalms"]),
        (28, &["prov", "proverbs"]),
        (29, &["job"]),
        (30, &["song", "sos", "canticles"]),
        (31, &["ruth"]),
        (32, &["lam", "lamentations"]),
        (33, &["eccl", "qoh", "ecclesiastes"]),
        (34, &["esth", "esther"]),
        (35, &["dan", "daniel"]),
        (36, &["ezra"]),
        (37, &["neh", "nehemiah"]),
        (38, &["1chr", "1chron", "1chronicles"]),
        (39, &["2chr", "2chron", "2chronicles"]),
    ];
    BOOKS
        .iter()
        .find(|(_, names)| names.contains(&t.as_str()))
        .map(|(n, _)| *n)
}

/// Parse a passage filter like `Gen` or `Gen-Deut` into an inclusive Haqor
/// book-number range. A single book yields `(n, n)`.
pub fn parse_passage(s: &str) -> Result<(u8, u8)> {
    let (lo_s, hi_s) = match s.split_once('-') {
        Some((a, b)) => (a, b),
        None => (s, s),
    };
    let lo = book_number(lo_s).with_context(|| format!("unknown book '{}'", lo_s.trim()))?;
    let hi = book_number(hi_s).with_context(|| format!("unknown book '{}'", hi_s.trim()))?;
    if lo > hi {
        anyhow::bail!("passage '{s}' starts after it ends (Tanakh order)");
    }
    Ok((lo, hi))
}

/// Is `(book, chapter, verse)` inside one of the Biblical Aramaic sections?
///
/// Biblical Hebrew prose has well-defined Aramaic passages that the Hebrew
/// morphology engine cannot (and should not) parse: Daniel 2:4b–7:28, Ezra
/// 4:8–6:18 and 7:12–26, Jeremiah 10:11, plus two words in Genesis 31:47. We
/// mark these at verse granularity. The boundary verse Daniel 2:4 is mixed
/// (Hebrew "…אֲרָמִית" then Aramaic); it is included, but the per-surface
/// "occurs only in Aramaic verses" test that consumes this keeps shared Hebrew
/// words (וַיְדַבְּרוּ, אֲרָמִית, …) — which also occur in Hebrew verses — from
/// being mismarked.
fn is_aramaic(book: u8, chapter: u8, verse: u8) -> bool {
    match book {
        GENESIS => chapter == 31 && verse == 47,
        JEREMIAH => chapter == 10 && verse == 11,
        DANIEL => {
            (chapter == 2 && verse >= 4)
                || (3..=6).contains(&chapter)
                || (chapter == 7 && verse <= 28)
        }
        EZRA => {
            (chapter == 4 && verse >= 8)
                || chapter == 5
                || (chapter == 6 && verse <= 18)
                || (chapter == 7 && (12..=26).contains(&verse))
        }
        _ => false,
    }
}

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
pub fn normalize_surface(token: &str) -> String {
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
            lexical_class     TEXT,
            language          TEXT
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
            attested        INTEGER NOT NULL,
            obj_suffix      TEXT    NOT NULL
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
                  AND lexical_class IS NULL AND language IS NULL
            ORDER BY occurrences DESC;

         CREATE VIEW review_nouns AS
            SELECT s.surface_id, s.text, s.occurrences, s.lexical_class,
                   n.stem, n.kind, n.label, n.prefix
            FROM surface s
            JOIN noun_analyses n ON n.surface_id = s.surface_id
            WHERE s.language IS NULL
            ORDER BY s.occurrences DESC, s.surface_id, n.stem;

         CREATE VIEW review_ambiguous AS
            SELECT s.surface_id, s.text, s.occurrences, s.n_candidates,
                   a.root, a.gizra, a.binyan, a.form, a.pgn,
                   a.prefix, a.vav_consecutive, a.attested
            FROM surface s
            JOIN analyses a ON a.surface_id = s.surface_id
            WHERE s.n_candidates > 1 AND s.lexical_class IS NULL
                  AND s.language IS NULL
            ORDER BY s.occurrences DESC, s.surface_id, a.attested DESC;

         CREATE VIEW review_aramaic AS
            SELECT surface_id, text, occurrences, parsed
            FROM surface
            WHERE language = 'aramaic'
            ORDER BY occurrences DESC;",
    )?;
    Ok(())
}

/// The analyses derived for a batch of surfaces: the refined lexical class, the
/// kept verb analyses, and the noun analyses — one entry per input surface, in
/// the same order. Shared by the full build and the incremental pass so both
/// classify and parse identically.
struct SurfaceAnalysis {
    classes: Vec<Option<&'static str>>,
    verb: Vec<Vec<VerbMatch>>,
    noun: Vec<Vec<NounMatch>>,
    /// Exact-match readings from the curated unmodeled-stem verb table
    /// ([`irregular_verb`]). These contribute to `n_candidates` (they are verbs)
    /// and are stored in `analyses` like generated verb matches.
    gold: Vec<Vec<&'static IrregularVerb>>,
}

/// How the verb pass enumerates candidate analyses. Both return the same matches
/// for a given surface (modulo the optional root filter); they only differ in the
/// time/space tradeoff — see [`analyze_surfaces`].
enum VerbStrategy<'a> {
    /// Build the all-roots reverse index once, then O(1)-look-up each surface.
    /// Worth its ~22s/2GB build only for large batches (the full rebuild).
    Indexed,
    /// Per-surface generate-and-test, optionally restricted to a lexicon root
    /// inventory. Cheap for a handful of surfaces; with a filter, cheap for many.
    PerSurface(Option<&'a HashSet<[char; 3]>>),
}

/// Run the lexical pre-filter, verb parser and noun parser over `surfaces`.
/// Loads the prefilter and noun inventory from `lexicon_db` when given; without
/// a lexicon, every surface is verb-parsed with no class and no noun pass.
/// `strategy` selects the verb-enumeration tradeoff (see [`VerbStrategy`]).
fn analyze_surfaces(
    surfaces: &[String],
    lexicon_db: Option<&Path>,
    strategy: VerbStrategy,
) -> Result<SurfaceAnalysis> {
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

    // Reverse-parse every distinct surface in parallel. Two strategies (see
    // [`VerbStrategy`]) that the caller picks by batch size:
    //
    // - `Indexed` (full rebuild, ~40k surfaces): build the all-roots reverse index
    //   once (every triliteral root's ~2.7k-form paradigm, keyed by canonical
    //   form) and look each surface up. The ~22s/2GB index build amortises over
    //   tens of thousands of O(1) lookups, and it stays unfiltered.
    // - `PerSurface(roots)` (preview / small `-n`): skip the index and run
    //   per-surface generate-and-test. With a lexicon root filter each surface
    //   only generates paradigms for the *real* roots its letters could spell (a
    //   handful), instead of the ~700 letter-combinations an unfiltered scan would
    //   — and the 22s index build is avoided entirely. This is what makes
    //   `db review-missing -n N` snappy. Note: a root filter means the verb pass
    //   sees only attested roots, so the preview is faithful to the *filtered*
    //   analysis; a surface that would only resolve to a non-lexicon (likely
    //   spurious) root shows as still-missing here. `PerSurface(None)` matches the
    //   unfiltered index exactly but regenerates every candidate paradigm, so it is
    //   only sensible for a few surfaces.
    //
    // Function words always bypass the parser; proper nouns are still parsed (the
    // verb-aware rule lets a name yield to a plausible verb reading); unclassified
    // tokens parse normally.
    let raw: Vec<Vec<VerbMatch>> = match strategy {
        VerbStrategy::Indexed => {
            info!("Building reverse-parse index over all triliteral roots");
            let index = ReverseIndex::build();
            info!(
                "Reverse-parsing {} distinct surfaces (indexed)",
                surfaces.len()
            );
            surfaces
                .par_iter()
                .zip(lexical.par_iter())
                .map(|(t, class)| {
                    if *class == Some("function") {
                        Vec::new()
                    } else {
                        parse_word_indexed(t, &index, None)
                    }
                })
                .collect()
        }
        VerbStrategy::PerSurface(roots) => {
            info!(
                "Reverse-parsing {} distinct surfaces (per-surface{})",
                surfaces.len(),
                if roots.is_some() {
                    ", lexicon-filtered"
                } else {
                    ""
                }
            );
            surfaces
                .par_iter()
                .zip(lexical.par_iter())
                .map(|(t, class)| {
                    if *class == Some("function") {
                        Vec::new()
                    } else {
                        parse_word_filtered(t, roots)
                    }
                })
                .collect()
        }
    };

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
    let verb: Vec<Vec<VerbMatch>> = raw
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
            let mut inv = NounInventory::build(&stems);
            inv.add_irregulars();
            inv.add_gold_nouns();
            info!(
                "Noun-parsing {} surfaces against {} stems",
                surfaces.len(),
                inv.len()
            );
            Some(inv)
        }
        None => None,
    };
    let noun: Vec<Vec<NounMatch>> = match &noun_inventory {
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
        let noun_parsed = noun.iter().filter(|a| !a.is_empty()).count();
        info!("  {noun_parsed} surfaces matched a noun analysis");
    }

    // Curated unmodeled-stem verb table: exact (full-surface) match. Gold-precise,
    // so it only adds the correct reading and never false-matches a surface the
    // generator already covers (those surfaces are absent from the table).
    let iv = irregular_verb::lookup();
    let gold: Vec<Vec<&'static IrregularVerb>> = surfaces
        .iter()
        .map(|t| iv.get(t.as_str()).cloned().unwrap_or_default())
        .collect();

    Ok(SurfaceAnalysis {
        classes,
        verb,
        noun,
        gold,
    })
}

/// Insert the verb/noun analysis rows for one surface into the prepared
/// statements. Shared by the full build and the incremental update.
fn insert_analyses(
    id: i64,
    verb: &[VerbMatch],
    noun: &[NounMatch],
    gold: &[&IrregularVerb],
    ana_stmt: &mut rusqlite::Statement<'_>,
    noun_stmt: &mut rusqlite::Statement<'_>,
) -> Result<()> {
    for m in verb {
        let root: String = m.root.letters.iter().collect();
        ana_stmt.execute((
            id,
            root,
            gizra_label(m),
            m.binyan.name(),
            m.form.name(),
            m.pgn.label(),
            &m.prefix,
            m.vav_consecutive as i64,
            m.attested as i64,
            m.object_suffix.map(|p| p.label()).unwrap_or_default(),
        ))?;
    }
    // Curated unmodeled-stem verbs: gold-precise, so always attested; the whole
    // surface is the matched form (no separated proclitic), and they carry no
    // generated object suffix.
    for g in gold {
        ana_stmt.execute((
            id,
            g.root,
            "Irregular",
            g.binyan,
            g.form,
            g.pgn,
            "",
            0_i64,
            1_i64,
            "",
        ))?;
    }
    for n in noun {
        noun_stmt.execute((id, &n.stem, format!("{:?}", n.kind), &n.label, &n.prefix))?;
    }
    Ok(())
}

/// Recompute the `roots` aggregate table from the current `analyses` rows.
/// Idempotent: clears and rebuilds, so it is safe to call after either a full
/// build or an incremental update.
fn rebuild_roots(db: &Connection) -> Result<()> {
    db.execute_batch(
        "DELETE FROM roots;
         INSERT INTO roots(root, gizra, n_forms, n_occurrences)
         SELECT root, gizra, COUNT(*) AS n_forms, SUM(occ) AS n_occurrences
         FROM (
            SELECT a.root AS root, MIN(a.gizra) AS gizra, s.occurrences AS occ
            FROM analyses a
            JOIN surface s ON s.surface_id = a.surface_id
            GROUP BY a.root, a.surface_id
         )
         GROUP BY root;",
    )?;
    Ok(())
}

/// Build `hebrew.db`, or incrementally improve an existing one. Returns
/// (distinct surfaces, occurrences, parsed surfaces).
///
/// By default this is **incremental**: if `output` already exists, only the
/// surfaces still in `review_missing` (no verb/noun analysis and no lexical
/// class) are re-analysed and updated in place — fast iteration when the engine
/// or lexicon improves. `limit` (when non-zero) caps the incremental pass to the
/// `limit` highest-frequency missing surfaces, for even faster iteration. Pass
/// `force = true` (or run with no existing DB) to wipe and rebuild everything
/// from `bible_db`; `limit` is ignored in that case.
pub fn generate_hebrew(
    bible_db: &Path,
    output: &Path,
    lexicon_db: Option<&Path>,
    force: bool,
    limit: usize,
) -> Result<(usize, usize, usize)> {
    if !force && output.exists() {
        return update_missing(output, lexicon_db, limit);
    }
    build_hebrew(bible_db, output, lexicon_db)
}

/// Full build: read every OT token from `bible_db` and write a fresh `hebrew.db`.
fn build_hebrew(
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

    let SurfaceAnalysis {
        classes,
        verb: analyses,
        noun: noun_analyses,
        gold: gold_analyses,
    } = analyze_surfaces(&surfaces, lexicon_db, VerbStrategy::Indexed)?;

    // Occurrence counts per surface, plus a per-surface flag for surfaces that
    // occur *only* in Biblical Aramaic verses (and so are Aramaic, not Hebrew
    // the engine failed on). A surface shared with any Hebrew verse is treated
    // as Hebrew, so words on the mixed Daniel 2:4 boundary stay analysable.
    let mut counts = vec![0u32; surfaces.len()];
    let mut has_hebrew = vec![false; surfaces.len()];
    for occ in &occurrences {
        counts[occ.surface_id] += 1;
        if !is_aramaic(occ.book, occ.chapter, occ.verse) {
            has_hebrew[occ.surface_id] = true;
        }
    }
    let aramaic_only: Vec<bool> = has_hebrew.iter().map(|&h| !h).collect();

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
        .zip(gold_analyses.iter())
        .filter(|((verb, noun), gold)| !verb.is_empty() || !noun.is_empty() || !gold.is_empty())
        .count();

    let tx = db.transaction()?;
    {
        let mut surf_stmt = tx.prepare(
            "INSERT INTO surface(surface_id, text, occurrences, n_candidates, \
             n_noun_candidates, parsed, attested, lexical_class, language) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        let mut ana_stmt = tx.prepare(
            "INSERT INTO analyses(surface_id, root, gizra, binyan, form, pgn, prefix, \
             vav_consecutive, attested, obj_suffix) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )?;
        let mut noun_stmt = tx.prepare(
            "INSERT INTO noun_analyses(surface_id, stem, kind, label, prefix) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for (id, matches) in analyses.iter().enumerate() {
            let nouns = &noun_analyses[id];
            let gold = &gold_analyses[id];
            // Gold readings are exact-match and always attested; they count as
            // verb candidates (n_candidates) so the surface leaves review_missing.
            let any_attested = matches.iter().any(|m| m.attested) || !gold.is_empty();
            surf_stmt.execute((
                id as i64,
                &surfaces[id],
                counts[id] as i64,
                (matches.len() + gold.len()) as i64,
                nouns.len() as i64,
                (!matches.is_empty() || !nouns.is_empty() || !gold.is_empty()) as i64,
                any_attested as i64,
                classes[id],
                aramaic_only[id].then_some("aramaic"),
            ))?;
            insert_analyses(
                id as i64,
                matches,
                nouns,
                gold,
                &mut ana_stmt,
                &mut noun_stmt,
            )?;
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
    rebuild_roots(&db)?;

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

/// Read-only preview for the fast iteration loop: take the `limit` highest-
/// frequency surfaces still in `review_missing` and run the *current* analysis
/// pipeline over them, printing what each would now resolve to — verb analysis,
/// noun analysis, lexical class, or still-missing — without touching the DB.
///
/// This is the tight inner loop: make a parser/lexicon change, run
/// `db review-missing -n N` to see which of the top-N missing your change now
/// accounts for, repeat. When satisfied, commit the change to the DB with the
/// incremental `db gen-hebrew -n N` pass. Returns (previewed, would_resolve).
///
/// `language` selects which subset to loop on: `"hebrew"` (the default — the
/// Hebrew backlog), `"aramaic"` (surfaces in the Biblical Aramaic sections), or
/// `"all"`. `passage`, when given, is an inclusive Haqor book-number range that
/// restricts the preview to surfaces occurring in those books (see
/// [`parse_passage`]) — e.g. just Genesis, or Genesis–Deuteronomy.
pub fn preview_missing(
    output: &Path,
    lexicon_db: Option<&Path>,
    limit: usize,
    language: &str,
    passage: Option<(u8, u8)>,
) -> Result<(usize, usize)> {
    let db = Connection::open(output).with_context(|| format!("opening {}", output.display()))?;

    let missing: Vec<(i64, String, i64)> = {
        let mut sql = String::from(
            "SELECT surface_id, text, occurrences FROM surface \
             WHERE n_candidates = 0 AND n_noun_candidates = 0 AND lexical_class IS NULL",
        );
        match language {
            "aramaic" => sql.push_str(" AND language = 'aramaic'"),
            "all" => {}
            "hebrew" => sql.push_str(" AND language IS NULL"),
            other => anyhow::bail!("unknown --language '{other}' (use hebrew, aramaic, or all)"),
        }
        if let Some((lo, hi)) = passage {
            sql.push_str(&format!(
                " AND surface_id IN (SELECT surface_id FROM occurrences \
                  WHERE book BETWEEN {lo} AND {hi})"
            ));
        }
        sql.push_str(" ORDER BY occurrences DESC, surface_id");
        if limit > 0 {
            sql.push_str(&format!(" LIMIT {limit}"));
        }
        let mut stmt = db.prepare(&sql)?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        rows.collect::<rusqlite::Result<_>>()?
    };

    let texts: Vec<String> = missing.iter().map(|(_, t, _)| t.clone()).collect();
    // Pick the verb-parse strategy by batch size. The all-roots index costs a
    // fixed ~22s to build but then resolves any number of surfaces; per-surface
    // generate-and-test (filtered to the lexicon's real roots) costs ~0.2s/surface
    // but nothing up front. For the small top-N the tight inner loop runs, going
    // per-surface skips the 22s build entirely (~6s for the default -n 30); past
    // the crossover the index is cheaper, so larger previews fall back to it.
    const INDEX_CROSSOVER: usize = 80;
    let roots = match lexicon_db {
        Some(p) => Some(load_root_inventory(p)?),
        None => None,
    };
    let strategy = if texts.len() <= INDEX_CROSSOVER {
        VerbStrategy::PerSurface(roots.as_ref())
    } else {
        VerbStrategy::Indexed
    };
    let SurfaceAnalysis {
        classes,
        verb,
        noun,
        gold,
    } = analyze_surfaces(&texts, lexicon_db, strategy)?;

    let mut resolved = 0usize;
    let scope = match passage {
        Some((lo, hi)) if lo == hi => format!(" in book {lo}"),
        Some((lo, hi)) => format!(" in books {lo}–{hi}"),
        None => String::new(),
    };
    println!(
        "Previewing top {} {language} missing surfaces{scope}:\n",
        missing.len()
    );
    for (i, (_, text, occ)) in missing.iter().enumerate() {
        let v = &verb[i];
        let n = &noun[i];
        let g = &gold[i];
        let class = classes[i];
        let result = if let Some(c) = class {
            format!("→ lexical:{c}")
        } else if !g.is_empty() {
            let m = g[0];
            let more = if g.len() > 1 {
                format!(" (+{} more)", g.len() - 1)
            } else {
                String::new()
            };
            format!(
                "→ verb* {} {} {} {}{}",
                m.root, m.binyan, m.form, m.pgn, more
            )
        } else if !v.is_empty() {
            let m = &v[0];
            let root: String = m.root.letters.iter().collect();
            let more = if v.len() > 1 {
                format!(" (+{} more)", v.len() - 1)
            } else {
                String::new()
            };
            let suf = m
                .object_suffix
                .map(|p| format!(" +{}", p.label()))
                .unwrap_or_default();
            format!(
                "→ verb {root} {} {} {}{}{}",
                m.binyan.name(),
                m.form.name(),
                m.pgn.label(),
                suf,
                more
            )
        } else if !n.is_empty() {
            let m = &n[0];
            let more = if n.len() > 1 {
                format!(" (+{} more)", n.len() - 1)
            } else {
                String::new()
            };
            format!("→ noun {} {}{}", m.stem, m.label, more)
        } else {
            "still missing".to_string()
        };
        let resolves = class.is_some() || !v.is_empty() || !n.is_empty() || !g.is_empty();
        if resolves {
            resolved += 1;
        }
        let mark = if resolves { "✓" } else { " " };
        println!("  {mark} {text:<20} (×{occ:<3})  {result}");
    }
    println!("\n{resolved} of {} would now resolve.", missing.len());
    Ok((missing.len(), resolved))
}

/// Incremental pass: re-analyse only the surfaces still in `review_missing`
/// (no verb/noun analysis, no lexical class) and update them in place. Existing
/// rows for already-resolved surfaces are left untouched, so this is cheap to
/// re-run as the engine or lexicon improves. Returns the whole-DB
/// (surfaces, occurrences, parsed) so callers can report current totals.
fn update_missing(
    output: &Path,
    lexicon_db: Option<&Path>,
    limit: usize,
) -> Result<(usize, usize, usize)> {
    let mut db =
        Connection::open(output).with_context(|| format!("opening {}", output.display()))?;

    // The surfaces with nothing yet — exactly the review_missing population,
    // highest-frequency first so a `limit` keeps the most impactful words.
    let missing: Vec<(i64, String)> = {
        let mut sql = String::from(
            "SELECT surface_id, text FROM surface \
             WHERE n_candidates = 0 AND n_noun_candidates = 0 AND lexical_class IS NULL \
             AND language IS NULL \
             ORDER BY occurrences DESC, surface_id",
        );
        if limit > 0 {
            sql.push_str(&format!(" LIMIT {limit}"));
        }
        let mut stmt = db.prepare(&sql)?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    info!(
        "Incremental: re-analysing {} missing surfaces",
        missing.len()
    );

    let texts: Vec<String> = missing.iter().map(|(_, t)| t.clone()).collect();
    // This pass writes analyses into the DB, so it must match the full rebuild's
    // unfiltered semantics — use the same indexed (all-roots) verb pass.
    let SurfaceAnalysis {
        classes,
        verb,
        noun,
        gold,
    } = analyze_surfaces(&texts, lexicon_db, VerbStrategy::Indexed)?;

    let mut resolved = 0usize;
    let tx = db.transaction()?;
    {
        let mut surf_stmt = tx.prepare(
            "UPDATE surface SET n_candidates = ?2, n_noun_candidates = ?3, \
             parsed = ?4, attested = ?5, lexical_class = ?6 WHERE surface_id = ?1",
        )?;
        let mut ana_stmt = tx.prepare(
            "INSERT INTO analyses(surface_id, root, gizra, binyan, form, pgn, prefix, \
             vav_consecutive, attested, obj_suffix) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )?;
        let mut noun_stmt = tx.prepare(
            "INSERT INTO noun_analyses(surface_id, stem, kind, label, prefix) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for (i, (id, _)) in missing.iter().enumerate() {
            let matches = &verb[i];
            let nouns = &noun[i];
            let golds = &gold[i];
            let class = classes[i];
            // Skip rows that are still unresolved, so we never rewrite a row to
            // its existing (empty) state.
            if matches.is_empty() && nouns.is_empty() && golds.is_empty() && class.is_none() {
                continue;
            }
            resolved += 1;
            let any_attested = matches.iter().any(|m| m.attested) || !golds.is_empty();
            surf_stmt.execute((
                id,
                (matches.len() + golds.len()) as i64,
                nouns.len() as i64,
                (!matches.is_empty() || !nouns.is_empty() || !golds.is_empty()) as i64,
                any_attested as i64,
                class,
            ))?;
            insert_analyses(*id, matches, nouns, golds, &mut ana_stmt, &mut noun_stmt)?;
        }
    }
    tx.commit()?;

    rebuild_roots(&db)?;

    let (surfaces, parsed, occurrences): (usize, usize, usize) = db.query_row(
        "SELECT (SELECT COUNT(*) FROM surface), \
                (SELECT COUNT(*) FROM surface WHERE parsed = 1), \
                (SELECT COUNT(*) FROM occurrences)",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    info!(
        "Incremental: resolved {resolved} of {} missing surfaces",
        missing.len()
    );
    Ok((surfaces, occurrences, parsed))
}
