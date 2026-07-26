//! Curate the four generation databases into the single runtime `haqor.db`.
//!
//! See `doc/adr/0006-single-runtime-database.md`. The generation databases stay
//! as the pipeline's cache — they are what the fast iteration loops rebuild —
//! and this stage distils them into what the reader actually needs:
//!
//! - **decisions, not candidate space.** `analyses` / `noun_analyses` do not
//!   ship. Every distinct (surface, OSHB tagging) pair that occurs in the
//!   corpus is resolved here, once, into a `word_info` row; the runtime looks
//!   it up instead of searching. 304,229 tokens collapse to ~93k renderings.
//! - **references packed.** `(book, chapter, verse)` becomes a single `ref`
//!   integer, so verse-keyed tables carry one column and one index.
//! - **strings interned.** Glosses and the closed morphology enumerations
//!   become dictionary rows; `oshb_primary.source_word` (16 MiB of pointed text
//!   read only to derive a proclitic prefix) is not carried at all.
//! - **generation-only tables dropped.** `occurrences` duplicates `verse_word`,
//!   `lexicon.english` and `entry_index` feed the build alone, and the review
//!   views describe work in progress rather than the reader's data.
//!
//! Naming follows the runtime's function, not the sources: `bdb` becomes
//! `lexicon_entry`, `sedra.words` becomes `syriac_word`. Attribution lives in
//! the READMEs and the app's About view.

use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use haqor_core::bible::HebrewWord;
use haqor_core::resolve::{TokenTagging, resolve};
use log::info;
use rusqlite::{Connection, params};

/// Schema version of the emitted database. Bump when a reader would have to
/// change to keep understanding `haqor.db`.
pub const SCHEMA_VERSION: u32 = 1;

/// How `verse.words` and `lexicon_entry.body` are stored. Both are only ever
/// fetched whole, never queried, so compressing them costs nothing at read
/// time — but it does cost the ability to read the database with `sqlite3`,
/// which is worth keeping in a debug build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlobCodec {
    None,
    Zstd,
}

impl BlobCodec {
    pub fn as_str(self) -> &'static str {
        match self {
            BlobCodec::None => "none",
            BlobCodec::Zstd => "zstd",
        }
    }
}

impl std::str::FromStr for BlobCodec {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "none" => Ok(BlobCodec::None),
            "zstd" => Ok(BlobCodec::Zstd),
            other => bail!("unknown blob codec {other:?} (expected \"none\" or \"zstd\")"),
        }
    }
}

/// Pack a verse reference into the single integer the runtime keys on.
pub fn pack_ref(book: i64, chapter: i64, verse: i64) -> i64 {
    (book << 16) | (chapter << 8) | verse
}

/// Interning pool: distinct strings in, small integer ids out.
#[derive(Default)]
struct Pool {
    ids: HashMap<String, i64>,
}

impl Pool {
    /// Id for `value`, inserting it on first sight. Returns `None` for the
    /// empty string so callers can store a NULL instead of pooling a blank.
    fn intern(&mut self, value: &str) -> Option<i64> {
        if value.is_empty() {
            return None;
        }
        let next = self.ids.len() as i64 + 1;
        Some(*self.ids.entry(value.to_string()).or_insert(next))
    }

    fn rows(&self) -> Vec<(i64, &str)> {
        let mut rows: Vec<(i64, &str)> = self.ids.iter().map(|(k, v)| (*v, k.as_str())).collect();
        rows.sort_unstable();
        rows
    }
}

/// The closed morphology enumerations of a [`HebrewWord`], interned as a unit:
/// a resolved reading repeats the same combination thousands of times.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct MorphCell {
    part_of_speech: String,
    form: String,
    tense: String,
    person: String,
    gender: String,
    number: String,
    state: String,
    prefix: String,
    obj_suffix: String,
}

impl MorphCell {
    fn of(word: &HebrewWord) -> Self {
        let text = |value: &Option<String>| value.clone().unwrap_or_default();
        MorphCell {
            part_of_speech: text(&word.part_of_speech),
            form: text(&word.form),
            tense: text(&word.tense),
            person: text(&word.person),
            gender: text(&word.gender),
            number: text(&word.number),
            state: text(&word.state),
            prefix: text(&word.prefix),
            obj_suffix: text(&word.obj_suffix),
        }
    }
}

const FLAG_VAV_CON: i64 = 1;
const FLAG_IS_NAME: i64 = 2;

/// A resolved rendering, keyed by the surface and the OSHB tagging that
/// produced it. Tokens sharing both share a row.
#[derive(Clone, PartialEq, Eq, Hash)]
struct InfoKey {
    surface_id: i64,
    tagging: Option<(String, String, String)>,
}

/// A stored rendering: what the reader ends up displaying, with the strings
/// already interned. Distinct taggings that resolve to the same thing — the
/// common case, since a tagging differs by pointing the resolution discards —
/// share one row.
type InfoRow = (i64, String, Option<i64>, i64, i64);

struct Builder {
    cells: HashMap<MorphCell, i64>,
    glosses: Pool,
    infos: HashMap<InfoKey, i64>,
    /// Emitted `word_info` rows, in id order: (surface_id, root, gloss_id,
    /// cell_id, flags).
    info_rows: Vec<InfoRow>,
    /// Row content back to its id, so an identical rendering is stored once.
    row_ids: HashMap<InfoRow, i64>,
}

impl Builder {
    fn new() -> Self {
        Builder {
            cells: HashMap::new(),
            glosses: Pool::default(),
            infos: HashMap::new(),
            info_rows: Vec::new(),
            row_ids: HashMap::new(),
        }
    }

    fn cell_id(&mut self, cell: MorphCell) -> i64 {
        let next = self.cells.len() as i64 + 1;
        *self.cells.entry(cell).or_insert(next)
    }

    /// Id of the `word_info` row for this (surface, tagging), resolving it on
    /// first sight. `None` when the surface carries no readable analysis at
    /// all — the reader shows the bare word.
    fn info_id(&mut self, db: &Connection, key: InfoKey, surface_text: &str) -> Option<i64> {
        if let Some(id) = self.infos.get(&key) {
            return Some(*id);
        }
        let tagging = key
            .tagging
            .as_ref()
            .map(|(source_word, lemma, morph)| TokenTagging {
                source_word,
                lemma,
                morph,
            });
        let word = resolve(db, key.surface_id, surface_text, tagging)?;
        let cell_id = self.cell_id(MorphCell::of(&word));
        let gloss_id = self.glosses.intern(&word.gloss);
        let flags = if word.vav_con { FLAG_VAV_CON } else { 0 }
            | if word.is_name { FLAG_IS_NAME } else { 0 };
        let row: InfoRow = (key.surface_id, word.root.clone(), gloss_id, cell_id, flags);
        let id = match self.row_ids.get(&row) {
            Some(id) => *id,
            None => {
                let id = self.info_rows.len() as i64 + 1;
                self.info_rows.push(row.clone());
                self.row_ids.insert(row, id);
                id
            }
        };
        self.infos.insert(key, id);
        Some(id)
    }
}

/// The four generation databases, attached under the schema names
/// [`haqor_core::resolve`] and this module's copy queries expect.
///
/// The generator opens them itself rather than through `Bible`, which now
/// opens the curated `haqor.db` — the very file this stage produces.
pub fn open_generation_dbs(data_dir: &Path) -> Result<Connection> {
    const SCHEMAS: [(&str, &str); 4] = [
        ("bible.db", "bibledb"),
        ("sedra.db", "sedradb"),
        ("hebrew.db", "hebrewdb"),
        ("lexicon.db", "lexdb"),
    ];
    let db = Connection::open_in_memory().context("opening the generation connection")?;
    for (file, schema) in SCHEMAS {
        let path = data_dir.join(file);
        if !path.exists() {
            bail!(
                "{} is missing; run the earlier db gen- stages first",
                path.display()
            );
        }
        db.execute(
            &format!("ATTACH DATABASE ?1 AS {schema}"),
            [path.to_string_lossy()],
        )
        .with_context(|| format!("attaching {}", path.display()))?;
    }

    attach_lexicon_views(&db)?;
    Ok(db)
}

/// Give `haqor-core`'s shared lexicon helpers the table names they expect, over
/// a connection with the *generation* `lexdb` attached.
///
/// Those helpers (`curated_gloss`, `bdb_rows`, the learner-gloss lookups) name
/// the runtime's tables without a schema, so that one implementation serves both
/// the app — where the names are real tables in `haqor.db` — and the build,
/// where they are these views.
///
/// `lexicon_entry.norm` is the one runtime column with no shim: it is computed
/// in Rust at copy time, and no build-time caller needs it (the concordance
/// queries that join on it run only in the app).
///
/// **Every build-time connection that calls into those helpers needs this.** The
/// shim is load-bearing, and its absence is silent: the helpers swallow a
/// "no such table" into `None`, so a connection without the views resolves
/// nothing and reports success. That is not hypothetical — `gen-hebrew`'s
/// lexical bridge ran without it and wrote an empty `lexical_analyses`, which
/// cost 930 function words (`לֹא`, `מִי`, `אֲנִי` …) their word info, and showed
/// up only when someone rebuilt with `--force`. The differential tests cannot
/// see this class of fault, because both sides of the comparison go through the
/// same helpers.
pub(crate) fn attach_lexicon_views(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TEMP VIEW IF NOT EXISTS lexicon_entry AS
           SELECT bdb_id AS key, root, word, cons, pos, gloss,
                  content_json AS body, type AS kind
           FROM lexdb.bdb;
         CREATE TEMP VIEW IF NOT EXISTS entry_root AS
           SELECT bdb_id AS key, root, ord FROM lexdb.entry_root;
         CREATE TEMP VIEW IF NOT EXISTS surface_override AS
           SELECT surface, root, gloss FROM lexdb.lexicon_overrides;
         CREATE TEMP VIEW IF NOT EXISTS word_gloss AS
           SELECT surface, gloss, note, is_name, reader_override
           FROM lexdb.word_glosses;",
    )
    .context("creating the generation-side lexicon views")?;
    Ok(())
}

/// Build `haqor.db` from the four generation databases in `data_dir`.
///
/// The output is created empty, then attached to the connection the generation
/// databases already live on: the bulk tables copy schema-to-schema in SQL, and
/// only the resolved renderings — which need the reader's own logic — travel
/// through Rust.
pub fn generate_runtime(data_dir: &Path, output: &Path, codec: BlobCodec) -> Result<usize> {
    if output.exists() {
        std::fs::remove_file(output)
            .with_context(|| format!("removing existing {}", output.display()))?;
    }
    Connection::open(output)
        .with_context(|| format!("creating {}", output.display()))?
        .execute_batch(SCHEMA)
        .with_context(|| format!("writing the schema of {}", output.display()))?;

    let connection = open_generation_dbs(data_dir)?;
    let db = &connection;
    db.execute("ATTACH DATABASE ?1 AS out", [output.to_string_lossy()])
        .with_context(|| format!("attaching {}", output.display()))?;

    let mut encoder = Encoder::new(db, codec)?;
    let mut builder = Builder::new();
    let mut reader_glosses = Pool::default();
    let words = {
        let tx = db.unchecked_transaction()?;
        let verses = copy_verses(db, &mut encoder)?;
        let ketivs = copy_ketiv(db)?;
        copy_surfaces(db)?;
        copy_roots(db)?;
        copy_verse_stats(db)?;
        copy_lexicon(db, &mut encoder)?;
        copy_syriac(db)?;

        let words = build_words(db, &mut builder, &mut reader_glosses)?;
        resolve_surfaces(db, &mut builder)?;
        write_word_info(db, &builder)?;
        write_glosses(db, &builder.glosses, &reader_glosses)?;
        write_meta(db, codec, &encoder)?;
        tx.commit()?;
        info!("Copied {verses} verses and {ketivs} ketiv readings");
        words
    };
    db.execute_batch(INDEXES)?;
    db.execute_batch("DETACH DATABASE out")?;

    // VACUUM cannot run on an attached database, so the reclaim happens on the
    // finished file. It matters: the bulk load leaves the free pages that make
    // up the difference between the working size and the shipped size.
    Connection::open(output)?.execute_batch("VACUUM")?;
    info!(
        "Wrote {words} words, {} renderings and {} morphology cells to {}",
        builder.info_rows.len(),
        builder.cells.len(),
        output.display()
    );
    Ok(words)
}

const SCHEMA: &str = "
CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);

CREATE TABLE verse(ref INTEGER PRIMARY KEY, words BLOB NOT NULL);

-- What the consonantal text writes where `verse` shows the qere the Masoretes
-- read instead. `position` is the token in the verse the reading begins at and
-- `span` how many tokens it covers; `span` is 0 for the eight readings that are
-- written but never read, where `position` is where the word would have stood.
CREATE TABLE ketiv(
    ref      INTEGER NOT NULL,
    position INTEGER NOT NULL,
    span     INTEGER NOT NULL,
    text     TEXT    NOT NULL,
    PRIMARY KEY(ref, position)
) WITHOUT ROWID;

-- Present only when meta.blob_codec names a codec that needs it.
CREATE TABLE blob_dict(dict_id INTEGER PRIMARY KEY, data BLOB NOT NULL);

CREATE TABLE surface(
    surface_id    INTEGER PRIMARY KEY,
    text          TEXT    NOT NULL,
    occurrences   INTEGER NOT NULL,
    n_candidates  INTEGER NOT NULL,
    lexical_class TEXT,
    language      TEXT,
    info_id       INTEGER
);

CREATE TABLE word(
    ref        INTEGER NOT NULL,
    position   INTEGER NOT NULL,
    surface_id INTEGER NOT NULL,
    info_id    INTEGER,
    gloss_id   INTEGER,
    PRIMARY KEY(ref, position)
) WITHOUT ROWID;

-- `word` with its reference unpacked. The tutor scores whole-corpus
-- aggregates by book/chapter/verse; `ref` passes through so a query that
-- filters on a reference still uses the primary key.
CREATE VIEW verse_word AS
  SELECT ref >> 16 AS book, (ref >> 8) & 255 AS chapter, ref & 255 AS verse,
         position, surface_id, info_id, gloss_id, ref
  FROM word;

CREATE TABLE word_info(
    info_id    INTEGER PRIMARY KEY,
    surface_id INTEGER NOT NULL,
    root       TEXT    NOT NULL,
    gloss_id   INTEGER,
    cell_id    INTEGER NOT NULL,
    flags      INTEGER NOT NULL
);

CREATE TABLE morph_cell(
    cell_id        INTEGER PRIMARY KEY,
    part_of_speech TEXT NOT NULL,
    form           TEXT NOT NULL,
    tense          TEXT NOT NULL,
    person         TEXT NOT NULL,
    gender         TEXT NOT NULL,
    number         TEXT NOT NULL,
    state          TEXT NOT NULL,
    prefix         TEXT NOT NULL,
    obj_suffix     TEXT NOT NULL
);

CREATE TABLE gloss(gloss_id INTEGER PRIMARY KEY, text TEXT NOT NULL);
CREATE TABLE reader_gloss(gloss_id INTEGER PRIMARY KEY, text TEXT NOT NULL);

CREATE TABLE root(
    root_id       INTEGER PRIMARY KEY,
    root          TEXT    NOT NULL,
    gizra         TEXT    NOT NULL,
    n_forms       INTEGER NOT NULL,
    n_occurrences INTEGER NOT NULL
);

-- Keyed by lexeme text, not by root_id: a noun stem is a legitimate
-- concordance key and most stems are not generated verb roots.
--
-- `sources` records which side of the union the pair came from (1 = a verb
-- analysis carrying the root, 2 = a noun analysis whose stem it is). Root
-- concordance treats them differently — the noun side reaches its root through
-- the lexicon rather than carrying it — so collapsing them would change which
-- verses a root lists.
CREATE TABLE root_surface(
    lexeme     TEXT    NOT NULL,
    surface_id INTEGER NOT NULL,
    sources    INTEGER NOT NULL,
    PRIMARY KEY(lexeme, surface_id)
) WITHOUT ROWID;

CREATE TABLE verse_stat(
    ref            INTEGER PRIMARY KEY,
    word_count     INTEGER NOT NULL,
    distinct_count INTEGER NOT NULL,
    min_occ        INTEGER NOT NULL,
    sum_occ        INTEGER NOT NULL,
    mask           INTEGER NOT NULL
);

-- `norm` is the headword put through the same normalisation as a corpus
-- surface (cantillation dropped, combining marks ordered), which is the form
-- the generated analyses name a noun stem by. BDB points its citation forms
-- with accents (אֱלִיעֶ֫זֶר), so a raw `word = stem` join silently loses one
-- noun stem in eight — every one of those lexemes then missing from its root's
-- concordance.
CREATE TABLE lexicon_entry(
    entry_id INTEGER PRIMARY KEY,
    key      TEXT NOT NULL,
    root     TEXT NOT NULL,
    word     TEXT,
    norm     TEXT,
    cons     TEXT,
    pos      TEXT,
    gloss    TEXT,
    body     BLOB,
    kind     TEXT
);

-- Every root an entry belongs to, `ord 0` being the BDB section it is printed
-- in. A compound name has more than one: אֱלִיעֶ֫זֶר (God is help) is both אלה
-- and עזר, and BDB can only print it under one of them. The word-info sheet
-- offers the alternatives, and a root's lexeme tree and concordance both read
-- membership from here rather than from `lexicon_entry.root`, so a name stands
-- in the lists of every root it is made of.
CREATE TABLE entry_root(
    key  TEXT    NOT NULL,
    root TEXT    NOT NULL,
    ord  INTEGER NOT NULL,
    PRIMARY KEY(key, root)
) WITHOUT ROWID;

CREATE TABLE word_gloss(
    surface         TEXT PRIMARY KEY,
    gloss           TEXT NOT NULL,
    note            TEXT,
    is_name         INTEGER NOT NULL,
    reader_override INTEGER NOT NULL
);

CREATE TABLE surface_override(
    surface TEXT PRIMARY KEY,
    root    TEXT NOT NULL,
    gloss   TEXT NOT NULL
);

CREATE TABLE syriac_root(root_id INTEGER PRIMARY KEY, root TEXT);
CREATE TABLE syriac_lexeme(
    lexeme_id INTEGER PRIMARY KEY,
    root_id   INTEGER,
    lexeme    TEXT
);
-- The morphology keys are what the NT word-info sheet reads; SEDRA's other
-- ~15 columns per word are generation-only.
CREATE TABLE syriac_word(
    word_id        INTEGER PRIMARY KEY,
    lexeme_id      INTEGER,
    word           TEXT,
    vocalised      TEXT,
    gender         INTEGER,
    person         INTEGER,
    number         INTEGER,
    state          INTEGER,
    tense          INTEGER,
    form           INTEGER,
    suffix_person  INTEGER,
    suffix_gender  INTEGER,
    suffix_number  INTEGER
);
CREATE TABLE syriac_gloss(
    gloss_id  INTEGER PRIMARY KEY,
    lexeme_id INTEGER,
    before    TEXT,
    meaning   TEXT,
    after     TEXT
);
CREATE TABLE nt_word(
    ref     INTEGER NOT NULL,
    ord     INTEGER NOT NULL,
    word_id INTEGER NOT NULL,
    PRIMARY KEY(ref, ord)
) WITHOUT ROWID;
";

/// Indexes are created after the bulk load: building them once over sorted data
/// is faster than maintaining them per insert.
const INDEXES: &str = "
CREATE UNIQUE INDEX out.idx_surface_text ON surface(text);
CREATE INDEX out.idx_word_surface ON word(surface_id);
CREATE INDEX out.idx_word_info_surface ON word_info(surface_id);
CREATE INDEX out.idx_root_surface_surface ON root_surface(surface_id);
CREATE UNIQUE INDEX out.idx_lexicon_entry_key ON lexicon_entry(key);
CREATE INDEX out.idx_lexicon_entry_root ON lexicon_entry(root);
CREATE INDEX out.idx_lexicon_entry_norm ON lexicon_entry(norm);
CREATE INDEX out.idx_lexicon_entry_cons ON lexicon_entry(cons);
CREATE INDEX out.idx_entry_root_root ON entry_root(root);
CREATE INDEX out.idx_syriac_lexeme_root ON syriac_lexeme(root_id);
CREATE INDEX out.idx_syriac_word_lexeme ON syriac_word(lexeme_id);
CREATE INDEX out.idx_syriac_word_vocalised ON syriac_word(vocalised);
CREATE INDEX out.idx_syriac_gloss_lexeme ON syriac_gloss(lexeme_id);
CREATE INDEX out.idx_nt_word_word ON nt_word(word_id);
";

/// Compression level. Past ~12 zstd spends a lot of time for very little on
/// inputs this small, and every blob here is one verse or one lexicon entry.
const ZSTD_LEVEL: i32 = 12;

/// Trained-dictionary size. The blobs are a few hundred bytes each, far too
/// short for zstd to learn anything within one of them, so without a shared
/// dictionary compression barely pays; with one, every verse starts knowing
/// what Hebrew text looks like.
const ZSTD_DICT_BYTES: usize = 112_640;

/// Writes `verse.words` and `lexicon_entry.body` in whichever form `meta`
/// advertises.
enum Encoder {
    Plain,
    Zstd {
        dictionary: Vec<u8>,
        compressor: zstd::bulk::Compressor<'static>,
    },
}

impl Encoder {
    /// Build the encoder, training the dictionary from the corpus it is about
    /// to compress when one is called for.
    fn new(db: &Connection, codec: BlobCodec) -> Result<Self> {
        if codec == BlobCodec::None {
            return Ok(Encoder::Plain);
        }
        let mut samples: Vec<Vec<u8>> = Vec::new();
        // Every 4th verse and every 4th entry body: enough to characterise both
        // kinds of blob without making training the slowest part of the build.
        let mut stmt = db.prepare(
            "SELECT words FROM bibledb.bible WHERE rowid % 4 = 0
             UNION ALL
             SELECT content_json FROM lexdb.bdb WHERE rowid % 4 = 0 AND content_json IS NOT NULL",
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            samples.push(row.get::<_, String>(0)?.into_bytes());
        }
        let dictionary = zstd::dict::from_samples(&samples, ZSTD_DICT_BYTES)
            .context("training the zstd dictionary")?;
        let compressor = zstd::bulk::Compressor::with_dictionary(ZSTD_LEVEL, &dictionary)
            .context("preparing the zstd compressor")?;
        info!("Trained a {} byte blob dictionary", dictionary.len());
        Ok(Encoder::Zstd {
            dictionary,
            compressor,
        })
    }

    fn encode(&mut self, value: &str) -> Result<Vec<u8>> {
        match self {
            Encoder::Plain => Ok(value.as_bytes().to_vec()),
            Encoder::Zstd { compressor, .. } => compressor
                .compress(value.as_bytes())
                .context("compressing a blob"),
        }
    }

    /// The dictionary a reader needs to decompress what was written.
    fn dictionary(&self) -> Option<&[u8]> {
        match self {
            Encoder::Plain => None,
            Encoder::Zstd { dictionary, .. } => Some(dictionary),
        }
    }
}

fn copy_verses(db: &Connection, encoder: &mut Encoder) -> Result<usize> {
    let mut stmt = db.prepare("SELECT book, chapter, verse, words FROM bibledb.bible")?;
    let mut insert = db.prepare("INSERT INTO out.verse(ref, words) VALUES (?1, ?2)")?;
    let mut rows = stmt.query([])?;
    let mut count = 0;
    while let Some(row) = rows.next()? {
        let reference = pack_ref(row.get(0)?, row.get(1)?, row.get(2)?);
        let words: String = row.get(3)?;
        insert.execute(params![reference, encoder.encode(&words)?])?;
        count += 1;
    }
    Ok(count)
}

/// Carry the ketiv readings across, packing their references.
///
/// Small enough (about 1,250 rows) to copy in one statement; the text is left
/// uncompressed because it is queried per verse alongside the reader's other
/// per-word data rather than fetched whole like a verse or a lexicon article.
fn copy_ketiv(db: &Connection) -> Result<usize> {
    let copied = db.execute(
        "INSERT INTO out.ketiv(ref, position, span, text)
         SELECT (book << 16) | (chapter << 8) | verse, position, span, text
         FROM bibledb.ketiv",
        [],
    )?;
    Ok(copied)
}

fn copy_surfaces(db: &Connection) -> Result<()> {
    db.execute_batch(
        "INSERT INTO out.surface(surface_id, text, occurrences, n_candidates, lexical_class, language)
         SELECT surface_id, text, occurrences, n_candidates, lexical_class, language
         FROM hebrewdb.surface",
    )?;
    Ok(())
}

/// `root` keeps the generated roots; `root_surface` keeps the surface breadth
/// that root concordance needs, which legitimately spans every candidate
/// analysis rather than only the resolved one.
fn copy_roots(db: &Connection) -> Result<()> {
    db.execute_batch(
        "INSERT INTO out.root(root, gizra, n_forms, n_occurrences)
           SELECT root, gizra, n_forms, n_occurrences FROM hebrewdb.roots;
         CREATE UNIQUE INDEX out.idx_root ON root(root);
         INSERT INTO out.root_surface(lexeme, surface_id, sources)
           SELECT lexeme, surface_id, sum(source) FROM (
             SELECT DISTINCT root AS lexeme, surface_id, 1 AS source FROM hebrewdb.analyses
             UNION ALL
             SELECT DISTINCT stem, surface_id, 2 FROM hebrewdb.noun_analyses)
           GROUP BY lexeme, surface_id",
    )?;
    Ok(())
}

/// The 28 one-bit concept columns become a bitmask; a view restores the column
/// names the tutor's generated `WHERE` clauses use.
const STAT_FLAGS: [&str; 28] = [
    "object_marker",
    "relative",
    "article",
    "conj_ve",
    "preposition",
    "prep_suffix",
    "prep_be",
    "prep_le",
    "prep_ke",
    "prep_min",
    "perfect",
    "imperfect",
    "wayyiqtol",
    "weqatal",
    "imperative",
    "infinitive",
    "participle",
    "jussive_cohortative",
    "binyan_niphal",
    "binyan_piel",
    "binyan_pual",
    "binyan_hiphil",
    "binyan_hophal",
    "binyan_hithpael",
    "noun_plural",
    "construct",
    "suffix_possessive",
    "object_suffix",
];

fn copy_verse_stats(db: &Connection) -> Result<()> {
    let mask = STAT_FLAGS
        .iter()
        .enumerate()
        .map(|(bit, name)| format!("({name} << {bit})"))
        .collect::<Vec<_>>()
        .join(" | ");
    let sql = format!(
        "SELECT book, chapter, verse, word_count, distinct_count, min_occ, sum_occ, {mask}
         FROM hebrewdb.verse_stats"
    );
    let mut stmt = db.prepare(&sql)?;
    let mut insert = db.prepare(
        "INSERT INTO out.verse_stat(ref, word_count, distinct_count, min_occ, sum_occ, mask)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        insert.execute(params![
            pack_ref(row.get(0)?, row.get(1)?, row.get(2)?),
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
        ])?;
    }

    let columns = STAT_FLAGS
        .iter()
        .enumerate()
        .map(|(bit, name)| format!("((mask >> {bit}) & 1) AS {name}"))
        .collect::<Vec<_>>()
        .join(", ");
    db.execute_batch(&format!(
        "CREATE VIEW out.verse_stats AS SELECT ref, word_count, distinct_count, min_occ, sum_occ,
         {columns} FROM verse_stat"
    ))?;
    Ok(())
}

/// The lexicon and its curated overlay. `english` (Strong's) and
/// `lexical_index` stay behind: they bridge a token's Strong's lemma to an
/// entry, which is resolved here rather than at read time.
fn copy_lexicon(db: &Connection, encoder: &mut Encoder) -> Result<()> {
    let mut stmt = db.prepare(
        "SELECT bdb_id, root, word, cons, pos, gloss, content_json, type FROM lexdb.bdb",
    )?;
    let mut insert = db.prepare(
        "INSERT INTO out.lexicon_entry(key, root, word, norm, cons, pos, gloss, body, kind)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let body: Option<String> = row.get(6)?;
        let body = body.map(|b| encoder.encode(&b)).transpose()?;
        let word: Option<String> = row.get(2)?;
        // The stem side of `root_surface` is normalised, so the headword is
        // stored in that form too and the join needs no runtime function.
        let norm = word
            .as_deref()
            .map(haqor_core::normalize_surface)
            .filter(|norm| !norm.is_empty());
        insert.execute(params![
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            word,
            norm,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            body,
            row.get::<_, Option<String>>(7)?,
        ])?;
    }

    db.execute_batch(
        "INSERT INTO out.entry_root(key, root, ord)
           SELECT bdb_id, root, ord FROM lexdb.entry_root;
         INSERT INTO out.word_gloss(surface, gloss, note, is_name, reader_override)
           SELECT surface, gloss, note, is_name, reader_override FROM lexdb.word_glosses;
         INSERT INTO out.surface_override(surface, root, gloss)
           SELECT surface, root, gloss FROM lexdb.lexicon_overrides",
    )?;
    Ok(())
}

fn copy_syriac(db: &Connection) -> Result<()> {
    db.execute_batch(
        "INSERT INTO out.syriac_root(root_id, root)
           SELECT keyRoot, strRoot FROM sedradb.roots;
         INSERT INTO out.syriac_lexeme(lexeme_id, root_id, lexeme)
           SELECT keyLexeme, keyRoot, strLexeme FROM sedradb.lexemes;
         INSERT INTO out.syriac_word(word_id, lexeme_id, word, vocalised, gender, person,
                                     number, state, tense, form, suffix_person,
                                     suffix_gender, suffix_number)
           SELECT keyWord, keyLexeme, strWord, strVocalised, keyGender, keyPerson,
                  keyNumber, keyState, keyTense, keyForm, keySuffixPerson,
                  keySuffixGender, keySuffixNumber
           FROM sedradb.words;
         INSERT INTO out.syriac_gloss(gloss_id, lexeme_id, before, meaning, after)
           SELECT keyEnglish, keyLexeme, strBefore, strMeaning, strAfter FROM sedradb.english",
    )?;

    // The NT reader walks a chapter's words in source-token order, which the
    // occurrence rowids preserve. Storing that order explicitly turns an
    // unindexed 109k-row scan per chapter into a covered range scan.
    let mut stmt =
        db.prepare("SELECT book, chapter, verse, keyWord FROM sedradb.occurrences ORDER BY rowid")?;
    let mut insert =
        db.prepare("INSERT INTO out.nt_word(ref, ord, word_id) VALUES (?1, ?2, ?3)")?;
    let mut rows = stmt.query([])?;
    let mut ordinals: HashMap<i64, i64> = HashMap::new();
    while let Some(row) = rows.next()? {
        let reference = pack_ref(row.get(0)?, row.get(1)?, row.get(2)?);
        let ord = ordinals.entry(reference).or_insert(0);
        insert.execute(params![reference, *ord, row.get::<_, i64>(3)?])?;
        *ord += 1;
    }
    Ok(())
}

/// One row per corpus token, pointing at the rendering it resolves to. This is
/// where the (surface, tagging) pairs are discovered: the reader's per-word
/// work happens once, here.
fn build_words(db: &Connection, builder: &mut Builder, reader_glosses: &mut Pool) -> Result<usize> {
    let mut stmt = db.prepare(
        "SELECT vw.book, vw.chapter, vw.verse, vw.position, vw.surface_id, s.text,
                vw.reader_gloss, p.source_word, p.lemma, p.morph
         FROM hebrewdb.verse_word vw
         JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
         LEFT JOIN hebrewdb.oshb_primary p
           ON p.book = vw.book AND p.chapter = vw.chapter AND p.verse = vw.verse
          AND p.position = vw.position AND p.surface_id = vw.surface_id
         ORDER BY vw.book, vw.chapter, vw.verse, vw.position",
    )?;
    let mut insert = db.prepare(
        "INSERT INTO out.word(ref, position, surface_id, info_id, gloss_id) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    let mut rows = stmt.query([])?;
    let mut count = 0;
    while let Some(row) = rows.next()? {
        let reference = pack_ref(row.get(0)?, row.get(1)?, row.get(2)?);
        let position: i64 = row.get(3)?;
        let surface_id: i64 = row.get(4)?;
        let text: String = row.get(5)?;
        let reader_gloss: String = row.get(6)?;
        let source_word: Option<String> = row.get(7)?;
        let tagging = match (
            source_word,
            row.get::<_, Option<String>>(8)?,
            row.get::<_, Option<String>>(9)?,
        ) {
            (Some(source_word), Some(lemma), Some(morph)) => Some((source_word, lemma, morph)),
            _ => None,
        };
        let info_id = builder.info_id(
            db,
            InfoKey {
                surface_id,
                tagging,
            },
            &text,
        );
        insert.execute(params![
            reference,
            position,
            surface_id,
            info_id,
            reader_glosses.intern(&reader_gloss),
        ])?;
        count += 1;
        if count % 50_000 == 0 {
            info!("  resolved {count} tokens");
        }
    }
    Ok(count)
}

/// The position-free rendering of every surface, for callers with no verse
/// context (vocabulary lists, the tutor's surface pass, word lookup by text).
fn resolve_surfaces(db: &Connection, builder: &mut Builder) -> Result<()> {
    let mut stmt = db.prepare("SELECT surface_id, text FROM hebrewdb.surface")?;
    let mut update = db.prepare("UPDATE out.surface SET info_id = ?2 WHERE surface_id = ?1")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let surface_id: i64 = row.get(0)?;
        let text: String = row.get(1)?;
        let info_id = builder.info_id(
            db,
            InfoKey {
                surface_id,
                tagging: None,
            },
            &text,
        );
        update.execute(params![surface_id, info_id])?;
    }
    Ok(())
}

fn write_word_info(db: &Connection, builder: &Builder) -> Result<()> {
    let mut insert = db.prepare(
        "INSERT INTO out.word_info(info_id, surface_id, root, gloss_id, cell_id, flags)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for (index, (surface_id, root, gloss_id, cell_id, flags)) in
        builder.info_rows.iter().enumerate()
    {
        insert.execute(params![
            index as i64 + 1,
            surface_id,
            root,
            gloss_id,
            cell_id,
            flags
        ])?;
    }

    let mut cells: Vec<(&MorphCell, i64)> = builder.cells.iter().map(|(c, i)| (c, *i)).collect();
    cells.sort_unstable_by_key(|(_, id)| *id);
    let mut insert = db.prepare(
        "INSERT INTO out.morph_cell(cell_id, part_of_speech, form, tense, person, gender, number,
                                state, prefix, obj_suffix)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;
    for (cell, id) in cells {
        insert.execute(params![
            id,
            cell.part_of_speech,
            cell.form,
            cell.tense,
            cell.person,
            cell.gender,
            cell.number,
            cell.state,
            cell.prefix,
            cell.obj_suffix,
        ])?;
    }
    Ok(())
}

fn write_glosses(db: &Connection, word_info: &Pool, reader: &Pool) -> Result<()> {
    let mut insert = db.prepare("INSERT INTO out.gloss(gloss_id, text) VALUES (?1, ?2)")?;
    for (id, text) in word_info.rows() {
        insert.execute(params![id, text])?;
    }
    let mut insert = db.prepare("INSERT INTO out.reader_gloss(gloss_id, text) VALUES (?1, ?2)")?;
    for (id, text) in reader.rows() {
        insert.execute(params![id, text])?;
    }
    Ok(())
}

fn write_meta(db: &Connection, codec: BlobCodec, encoder: &Encoder) -> Result<()> {
    // The build stamp is the database's version: nothing is maintained by hand,
    // the app compares it to decide whether to reinstall, and it orders so a
    // sync server can tell whether it holds a newer build (ADR 6).
    let built = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("reading the system clock")?
        .as_secs();
    let mut insert = db.prepare("INSERT INTO out.meta(key, value) VALUES (?1, ?2)")?;
    insert.execute(params!["schema_version", SCHEMA_VERSION.to_string()])?;
    insert.execute(params!["built", format_timestamp(built)])?;
    insert.execute(params!["blob_codec", codec.as_str()])?;
    if let Some(dictionary) = encoder.dictionary() {
        // The blobs are too short to compress on their own, so a reader cannot
        // decode them without the dictionary they were trained against; it
        // ships in the database rather than in the app.
        db.execute(
            "INSERT INTO out.blob_dict(dict_id, data) VALUES (1, ?1)",
            [dictionary],
        )?;
    }
    Ok(())
}

/// UTC ISO-8601 from a Unix timestamp, without pulling in a date library for
/// the one string this crate formats.
fn format_timestamp(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let time = seconds % 86_400;
    // Civil-from-days, Howard Hinnant's algorithm, shifted to a March-based
    // year so leap days land at the end.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = era * 400 + yoe + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        time / 3600,
        (time % 3600) / 60,
        time % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_and_orders_references() {
        assert_eq!(pack_ref(1, 1, 1), 65_793);
        assert!(pack_ref(1, 1, 2) > pack_ref(1, 1, 1));
        assert!(pack_ref(1, 2, 1) > pack_ref(1, 1, 255));
        assert!(pack_ref(2, 1, 1) > pack_ref(1, 255, 255));
    }

    #[test]
    fn formats_timestamps_as_utc_iso8601() {
        assert_eq!(format_timestamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_timestamp(1_000_000_000), "2001-09-09T01:46:40Z");
        // A leap day, which the March-based civil algorithm has to place.
        assert_eq!(format_timestamp(1_709_164_800), "2024-02-29T00:00:00Z");
    }

    #[test]
    fn pool_skips_empty_and_reuses_ids() {
        let mut pool = Pool::default();
        assert_eq!(pool.intern(""), None);
        let first = pool.intern("and he said").unwrap();
        assert_eq!(pool.intern("and he said"), Some(first));
        assert_ne!(pool.intern("the king"), Some(first));
        assert_eq!(pool.rows().len(), 2);
    }
}
