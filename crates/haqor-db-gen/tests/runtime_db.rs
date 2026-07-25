//! The differential test that lets `gen-runtime` precompute word info at all.
//!
//! `haqor.db` stores the reader's answer instead of the candidate space it used
//! to search (ADR 6). That is only safe if the stored answer is the same one
//! the live resolution produces, for every rendering in the corpus — so this
//! builds the database and then re-resolves every distinct (surface, OSHB
//! tagging) pair against the generation databases and compares.
//!
//! It is the guard for the follow-up too: when the resolution logic moves out
//! of `haqor-core` into this crate, this test is what says the move changed
//! nothing.

use std::path::{Path, PathBuf};

use haqor_core::bible::{Bible, HebrewWord};
use haqor_core::data_support::{TokenTagging, connection, resolve_word_info};
use haqor_db_gen::{BlobCodec, generate_runtime, pack_ref};
use rusqlite::OptionalExtension;

/// Workspace `data/`, which is two levels above this crate. Tests run with the
/// package directory as their cwd, so a relative "data" would silently skip.
fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

macro_rules! require_data {
    () => {
        if !data_dir().join("bible.db").exists() {
            eprintln!("skipping: data/*.db not generated in this checkout");
            return;
        }
    };
}

/// Rebuild the stored [`HebrewWord`] from its interned parts, exactly as a
/// runtime reader would.
fn stored_word(db: &rusqlite::Connection, info_id: i64, text: &str) -> Option<HebrewWord> {
    let some = |value: String| (!value.is_empty()).then_some(value);
    db.query_row(
        "SELECT wi.root, COALESCE(g.text, ''), c.part_of_speech, c.form, c.tense, c.person,
                c.gender, c.number, c.state, c.prefix, c.obj_suffix, wi.flags
         FROM rt.word_info wi
         JOIN rt.morph_cell c ON c.cell_id = wi.cell_id
         LEFT JOIN rt.gloss g ON g.gloss_id = wi.gloss_id
         WHERE wi.info_id = ?1",
        [info_id],
        |row| {
            let flags: i64 = row.get(11)?;
            Ok(HebrewWord {
                word: text.to_string(),
                root: row.get(0)?,
                gloss: row.get(1)?,
                part_of_speech: some(row.get(2)?),
                form: some(row.get(3)?),
                tense: some(row.get(4)?),
                person: some(row.get(5)?),
                gender: some(row.get(6)?),
                number: some(row.get(7)?),
                state: some(row.get(8)?),
                prefix: some(row.get(9)?),
                vav_con: flags & 1 != 0,
                obj_suffix: some(row.get(10)?),
                is_name: flags & 2 != 0,
            })
        },
    )
    .optional()
    .expect("reading a stored rendering")
}

#[test]
fn stored_renderings_match_live_resolution() {
    require_data!();
    let output = std::env::temp_dir().join("haqor-runtime-differential.db");
    generate_runtime(&data_dir(), &output, BlobCodec::None).expect("generating haqor.db");

    let bible = Bible::open(data_dir()).expect("opening generation databases");
    let db = connection(&bible);
    db.execute("ATTACH DATABASE ?1 AS rt", [output.to_string_lossy()])
        .expect("attaching the generated database");

    // Every distinct (surface, tagging) the corpus actually contains, with the
    // rendering the generator stored for it. One representative token per pair
    // is enough: the pair is what the resolution reads.
    let mut stmt = db
        .prepare(
            "SELECT vw.surface_id, s.text, p.source_word, p.lemma, p.morph, MIN(w.info_id)
             FROM hebrewdb.verse_word vw
             JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
             JOIN rt.word w ON w.ref = ((vw.book << 16) | (vw.chapter << 8) | vw.verse)
                           AND w.position = vw.position
             LEFT JOIN hebrewdb.oshb_primary p
               ON p.book = vw.book AND p.chapter = vw.chapter AND p.verse = vw.verse
              AND p.position = vw.position AND p.surface_id = vw.surface_id
             GROUP BY vw.surface_id, p.source_word, p.lemma, p.morph",
        )
        .expect("preparing the pair query");
    let pairs = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<i64>>(5)?,
            ))
        })
        .expect("querying pairs")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collecting pairs");
    assert!(
        pairs.len() > 50_000,
        "expected the whole corpus, got {} pairs",
        pairs.len()
    );

    let mut checked = 0;
    let mut mismatches = Vec::new();
    for (surface_id, text, source_word, lemma, morph, info_id) in &pairs {
        let tagging = match (source_word, lemma, morph) {
            (Some(source_word), Some(lemma), Some(morph)) => Some(TokenTagging {
                source_word,
                lemma,
                morph,
            }),
            _ => None,
        };
        let live = resolve_word_info(&bible, *surface_id, text, tagging);
        let stored = info_id.and_then(|id| stored_word(db, id, text));
        if live != stored && mismatches.len() < 10 {
            mismatches.push(format!(
                "{text} (surface {surface_id}): {live:?} != {stored:?}"
            ));
        }
        checked += 1;
    }
    assert!(
        mismatches.is_empty(),
        "{} of {checked} renderings differ from live resolution:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );

    // The position-free path too: word lookup by text, vocabulary lists and the
    // tutor's surface pass all resolve without a token.
    let mut stmt = db
        .prepare("SELECT surface_id, text, info_id FROM rt.surface")
        .expect("preparing the surface query");
    let surfaces = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        })
        .expect("querying surfaces")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collecting surfaces");
    for (surface_id, text, info_id) in &surfaces {
        let live = resolve_word_info(&bible, *surface_id, text, None);
        let stored = info_id.and_then(|id| stored_word(db, id, text));
        assert_eq!(live, stored, "surface {text} ({surface_id}) differs");
    }

    let _ = std::fs::remove_file(&output);
}

/// The compressed build has to be readable, and readable *only* with what the
/// database itself carries: the blobs are too short to compress alone, so they
/// are written against a trained dictionary that ships in `blob_dict`. This is
/// the read path the runtime will use.
#[test]
fn compressed_verse_text_round_trips_through_the_shipped_dictionary() {
    require_data!();
    let output = std::env::temp_dir().join("haqor-runtime-zstd.db");
    generate_runtime(&data_dir(), &output, BlobCodec::Zstd).expect("generating a compressed build");

    let db = rusqlite::Connection::open(&output).expect("opening the compressed build");
    let codec: String = db
        .query_row("SELECT value FROM meta WHERE key = 'blob_codec'", [], |r| {
            r.get(0)
        })
        .expect("reading the codec");
    assert_eq!(codec, "zstd");
    let dictionary: Vec<u8> = db
        .query_row("SELECT data FROM blob_dict WHERE dict_id = 1", [], |r| {
            r.get(0)
        })
        .expect("reading the blob dictionary");
    let mut decompressor =
        zstd::bulk::Decompressor::with_dictionary(&dictionary).expect("preparing the decompressor");

    let source = Bible::open(data_dir()).expect("opening generation databases");
    let mut stmt = connection(&source)
        .prepare("SELECT book, chapter, verse, words FROM bibledb.bible")
        .expect("preparing");
    let mut rows = stmt.query([]).expect("querying");
    let mut checked = 0;
    while let Some(row) = rows.next().expect("reading a verse") {
        let reference = pack_ref(
            row.get(0).unwrap(),
            row.get(1).unwrap(),
            row.get(2).unwrap(),
        );
        let expected: String = row.get(3).unwrap();
        let stored: Vec<u8> = db
            .query_row("SELECT words FROM verse WHERE ref = ?1", [reference], |r| {
                r.get(0)
            })
            .unwrap_or_else(|e| panic!("verse {reference} missing from the build: {e}"));
        // 64 KiB is far above the longest verse (Esther 8:9, ~1 KiB).
        let plain = decompressor
            .decompress(&stored, 64 * 1024)
            .unwrap_or_else(|e| panic!("verse {reference} does not decompress: {e}"));
        assert_eq!(
            String::from_utf8(plain).expect("verse text is not UTF-8"),
            expected,
            "verse {reference} does not round trip"
        );
        checked += 1;
    }
    assert_eq!(checked, 31_171, "the whole corpus should be stored");

    let _ = std::fs::remove_file(&output);
}

/// Root concordance is the one place the candidate space is not collapsed:
/// looking up a lexeme finds every surface that *any* analysis reads that way,
/// not only the surfaces whose resolved reading agrees. `root_surface` has to
/// reproduce that union exactly — including noun stems, most of which are not
/// generated verb roots and would vanish if the table were keyed by root id.
#[test]
fn root_surface_reproduces_the_concordance_union() {
    require_data!();
    let output = std::env::temp_dir().join("haqor-runtime-roots.db");
    generate_runtime(&data_dir(), &output, BlobCodec::None).expect("generating haqor.db");

    let bible = Bible::open(data_dir()).expect("opening generation databases");
    let db = connection(&bible);
    db.execute("ATTACH DATABASE ?1 AS rt", [output.to_string_lossy()])
        .expect("attaching the generated database");

    let missing: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM (
               SELECT root AS lexeme, surface_id FROM hebrewdb.analyses
               UNION
               SELECT stem, surface_id FROM hebrewdb.noun_analyses
               EXCEPT
               SELECT lexeme, surface_id FROM rt.root_surface)",
            [],
            |row| row.get(0),
        )
        .expect("counting missing concordance pairs");
    let extra: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM (
               SELECT lexeme, surface_id FROM rt.root_surface
               EXCEPT
               SELECT root, surface_id FROM hebrewdb.analyses
               EXCEPT
               SELECT stem, surface_id FROM hebrewdb.noun_analyses)",
            [],
            |row| row.get(0),
        )
        .expect("counting extra concordance pairs");
    assert_eq!((missing, extra), (0, 0), "concordance pairs differ");

    let stems_kept: i64 = db
        .query_row(
            "SELECT COUNT(DISTINCT n.stem) FROM hebrewdb.noun_analyses n
             WHERE n.stem NOT IN (SELECT root FROM hebrewdb.roots)
               AND EXISTS(SELECT 1 FROM rt.root_surface rs WHERE rs.lexeme = n.stem)",
            [],
            |row| row.get(0),
        )
        .expect("counting kept noun stems");
    assert!(
        stems_kept > 6_000,
        "noun stems outside the generated roots were dropped ({stems_kept} kept)"
    );

    let _ = std::fs::remove_file(&output);
}

#[test]
fn packed_references_round_trip_the_corpus() {
    require_data!();
    let bible = Bible::open(data_dir()).expect("opening generation databases");
    let db = connection(&bible);
    // Chapters and verses both exceed a byte's range in no book, which is what
    // the 8-bit packing assumes; a Psalm 119 or a 176-verse chapter would show
    // up here first.
    let (max_chapter, max_verse): (i64, i64) = db
        .query_row(
            "SELECT MAX(chapter), MAX(verse) FROM bibledb.bible",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("reading corpus bounds");
    assert!(max_chapter <= 255, "chapter {max_chapter} overflows a byte");
    assert!(max_verse <= 255, "verse {max_verse} overflows a byte");

    let mut stmt = db
        .prepare("SELECT book, chapter, verse FROM bibledb.bible")
        .expect("preparing");
    let refs = stmt
        .query_map([], |row| {
            Ok(pack_ref(row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .expect("querying")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collecting");
    let mut sorted = refs.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), refs.len(), "packed references collide");
}
