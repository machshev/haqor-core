//! Hebrew lexicon database generator.
//!
//! Builds a standalone `lexicon.db` from the OpenScriptures `HebrewLexicon`
//! sources (CC BY 4.0). Three tables:
//!
//! - `english` — Strong's, from `HebrewStrong.xml`, keyed by the integer
//!   Strong's number (which is exactly the lemma morphhb attaches to every OT
//!   word token, so glosses join straight onto a parsed token's lemma). Columns:
//!   `strong` (PK, `H` prefix dropped), `lang` (`heb`/`arc`/`x-pn`), `word`
//!   (pointed headword), `xlit`/`pron`/`pos`, `gloss` (the `<def>` texts joined
//!   `; `), `meaning` (full `<meaning>`, tags stripped), `usage` (KJV), `source`
//!   (derivation).
//! - `bdb` — Brown-Driver-Briggs full entries, from `BrownDriverBriggs.xml`,
//!   keyed by BDB id (e.g. `a.ae.ab`). Columns: `bdb_id` (PK), `word`, `pos`,
//!   `gloss` (top + sense `<def>` texts joined `; `), `definition` (the full
//!   article flattened, sense numbers preserved as `(n)`), `status`.
//! - `lexical_index` — the glue, from `LexicalIndex.xml`: one row per `<xref>`,
//!   mapping an OSHB lemma to a `strong` number (nullable), a `bdb_id`, and a
//!   `twot` number. Indexed on `strong` and `bdb_id`.
//!
//! So a token's Strong's lemma reaches its full BDB entry via
//! `english.strong → lexical_index.strong → lexical_index.bdb_id → bdb.bdb_id`
//! (a many-to-many join: BDB groups by root, Strong's by lexeme).
//!
//! Cross-reference `<w src="…">` elements inside the prose are flattened to
//! their text.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use log::info;
use quick_xml::Reader;
use quick_xml::events::Event;
use rusqlite::Connection;

use crate::morphology::{NounStem, Root};

/// Which prose section text is currently being accumulated into.
#[derive(Clone, Copy, PartialEq)]
enum Section {
    None,
    Source,
    Meaning,
    Usage,
}

#[derive(Default)]
struct Entry {
    strong: i64,
    lang: String,
    word: String,
    xlit: String,
    pron: String,
    pos: String,
    gloss_parts: Vec<String>,
    meaning: String,
    usage: String,
    source: String,
}

/// Collapse internal whitespace runs to single spaces and trim.
fn tidy(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse HebrewStrong.xml and insert one row per entry. Returns rows written.
fn load_strongs(db: &mut Connection, path: &Path) -> Result<usize> {
    db.execute(
        "CREATE TABLE english(\
            strong INTEGER PRIMARY KEY, \
            lang TEXT, word TEXT, xlit TEXT, pron TEXT, pos TEXT, \
            gloss TEXT, meaning TEXT, usage TEXT, source TEXT)",
        [],
    )?;

    let mut reader =
        Reader::from_file(path).with_context(|| format!("opening {}", path.display()))?;
    let mut buf = Vec::new();

    let mut entry: Option<Entry> = None;
    let mut section = Section::None;
    let mut in_headword = false;
    let mut headword_done = false;
    let mut in_def = false;
    let mut def_buf = String::new();

    let tx = db.transaction()?;
    let mut rows = 0;
    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO english \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )?;
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => match e.name().as_ref() {
                    b"entry" => {
                        let mut ent = Entry::default();
                        if let Some(a) = e.try_get_attribute("id")? {
                            let id = a.decode_and_unescape_value(reader.decoder())?;
                            ent.strong = id.trim_start_matches('H').parse().unwrap_or(0);
                        }
                        entry = Some(ent);
                        section = Section::None;
                        headword_done = false;
                    }
                    b"w" => {
                        // The headword is the FIRST <w> in the entry. Alternate
                        // spellings quoted inside <source> (e.g. הִיא, לוֹא) also
                        // carry an `xlit` attr, so guard on `headword_done` or
                        // they get appended to the headword; their text instead
                        // belongs to the active prose section.
                        if let Some(ent) = entry.as_mut()
                            && !headword_done
                            && let Some(a) = e.try_get_attribute("xlit")?
                        {
                            in_headword = true;
                            headword_done = true;
                            ent.xlit = a.decode_and_unescape_value(reader.decoder())?.into_owned();
                            if let Some(p) = e.try_get_attribute("pron")? {
                                ent.pron =
                                    p.decode_and_unescape_value(reader.decoder())?.into_owned();
                            }
                            if let Some(p) = e.try_get_attribute("pos")? {
                                ent.pos =
                                    p.decode_and_unescape_value(reader.decoder())?.into_owned();
                            }
                            if let Some(p) = e.try_get_attribute("xml:lang")? {
                                ent.lang =
                                    p.decode_and_unescape_value(reader.decoder())?.into_owned();
                            }
                        }
                    }
                    b"source" => section = Section::Source,
                    b"meaning" => section = Section::Meaning,
                    b"usage" => section = Section::Usage,
                    b"def" if section == Section::Meaning => {
                        in_def = true;
                        def_buf.clear();
                    }
                    _ => {}
                },
                Event::Text(t) => {
                    let txt = t.unescape()?;
                    if let Some(ent) = entry.as_mut() {
                        if in_headword {
                            ent.word.push_str(&txt);
                        } else {
                            match section {
                                Section::Source => ent.source.push_str(&txt),
                                Section::Meaning => ent.meaning.push_str(&txt),
                                Section::Usage => ent.usage.push_str(&txt),
                                Section::None => {}
                            }
                            if in_def {
                                def_buf.push_str(&txt);
                            }
                        }
                    }
                }
                Event::End(e) => match e.name().as_ref() {
                    b"w" => in_headword = false,
                    b"def" => {
                        if in_def {
                            if let Some(ent) = entry.as_mut() {
                                let g = tidy(&def_buf);
                                if !g.is_empty() {
                                    ent.gloss_parts.push(g);
                                }
                            }
                            in_def = false;
                        }
                    }
                    b"source" | b"meaning" | b"usage" => section = Section::None,
                    b"entry" => {
                        if let Some(ent) = entry.take() {
                            stmt.execute((
                                ent.strong,
                                ent.lang,
                                tidy(&ent.word),
                                ent.xlit,
                                ent.pron,
                                ent.pos,
                                ent.gloss_parts.join("; "),
                                tidy(&ent.meaning),
                                tidy(&ent.usage),
                                tidy(&ent.source),
                            ))?;
                            rows += 1;
                        }
                    }
                    _ => {}
                },
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
    }
    tx.commit()?;
    Ok(rows)
}

#[derive(Default)]
struct BdbEntry {
    bdb_id: String,
    word: String,
    pos: String,
    gloss_parts: Vec<String>,
    definition: String,
    status: String,
}

/// Parse BrownDriverBriggs.xml into the `bdb` table. Returns rows written.
fn load_bdb(db: &mut Connection, path: &Path) -> Result<usize> {
    db.execute(
        "CREATE TABLE bdb(\
            bdb_id TEXT PRIMARY KEY, word TEXT, pos TEXT, \
            gloss TEXT, definition TEXT, status TEXT)",
        [],
    )?;

    let mut reader =
        Reader::from_file(path).with_context(|| format!("opening {}", path.display()))?;
    let mut buf = Vec::new();

    let mut entry: Option<BdbEntry> = None;
    let mut headword_done = false;
    let mut in_headword = false;
    let mut in_pos = false;
    let mut in_def = false;
    let mut in_status = false;
    let mut def_buf = String::new();

    let tx = db.transaction()?;
    let mut rows = 0;
    {
        let mut stmt = tx.prepare("INSERT OR REPLACE INTO bdb VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?;
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => match e.name().as_ref() {
                    b"entry" => {
                        let mut ent = BdbEntry::default();
                        if let Some(a) = e.try_get_attribute("id")? {
                            ent.bdb_id =
                                a.decode_and_unescape_value(reader.decoder())?.into_owned();
                        }
                        entry = Some(ent);
                        headword_done = false;
                    }
                    // The headword is the first <w> in the entry; later <w
                    // src="…"> are cross-references whose text stays in the prose.
                    b"w" if entry.is_some() && !headword_done => {
                        in_headword = true;
                        headword_done = true;
                    }
                    b"pos" => in_pos = true,
                    b"def" => {
                        in_def = true;
                        def_buf.clear();
                    }
                    b"sense" => {
                        // Preserve the sense number inline in the flat article.
                        if let Some(ent) = entry.as_mut()
                            && let Some(a) = e.try_get_attribute("n")?
                        {
                            let n = a.decode_and_unescape_value(reader.decoder())?;
                            ent.definition.push_str(&format!(" ({n}) "));
                        }
                    }
                    b"status" => in_status = true,
                    _ => {}
                },
                Event::Text(t) => {
                    if let Some(ent) = entry.as_mut() {
                        let txt = t.unescape()?;
                        if in_headword {
                            ent.word.push_str(&txt);
                        } else if in_status {
                            ent.status.push_str(&txt);
                        } else {
                            if in_pos {
                                ent.pos.push_str(&txt);
                            }
                            ent.definition.push_str(&txt);
                            if in_def {
                                def_buf.push_str(&txt);
                            }
                        }
                    }
                }
                Event::End(e) => match e.name().as_ref() {
                    b"w" => in_headword = false,
                    b"pos" => in_pos = false,
                    b"def" => {
                        if let Some(ent) = entry.as_mut() {
                            let g = tidy(&def_buf);
                            if !g.is_empty() {
                                ent.gloss_parts.push(g);
                            }
                        }
                        in_def = false;
                    }
                    b"status" => in_status = false,
                    b"entry" => {
                        if let Some(ent) = entry.take() {
                            stmt.execute((
                                ent.bdb_id,
                                tidy(&ent.word),
                                tidy(&ent.pos),
                                ent.gloss_parts.join("; "),
                                tidy(&ent.definition),
                                tidy(&ent.status),
                            ))?;
                            rows += 1;
                        }
                    }
                    _ => {}
                },
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
    }
    tx.commit()?;
    Ok(rows)
}

/// Parse LexicalIndex.xml into the `lexical_index` mapping table (one row per
/// `<xref>`). Returns rows written.
fn load_lexical_index(db: &mut Connection, path: &Path) -> Result<usize> {
    db.execute(
        "CREATE TABLE lexical_index(\
            oshb_id TEXT, word TEXT, strong INTEGER, bdb_id TEXT, twot TEXT)",
        [],
    )?;

    let mut reader =
        Reader::from_file(path).with_context(|| format!("opening {}", path.display()))?;
    let mut buf = Vec::new();

    let mut oshb_id = String::new();
    let mut word = String::new();
    let mut in_word = false;

    let tx = db.transaction()?;
    let mut rows = 0;
    {
        let mut stmt = tx.prepare("INSERT INTO lexical_index VALUES (?1, ?2, ?3, ?4, ?5)")?;
        loop {
            // <xref> is empty so it arrives as an Empty event; <w> wraps text.
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => match e.name().as_ref() {
                    b"entry" => {
                        oshb_id = e
                            .try_get_attribute("id")?
                            .map(|a| a.decode_and_unescape_value(reader.decoder()))
                            .transpose()?
                            .map(|v| v.into_owned())
                            .unwrap_or_default();
                        word.clear();
                    }
                    b"w" => in_word = true,
                    _ => {}
                },
                Event::Text(t) if in_word => word.push_str(&t.unescape()?),
                Event::Empty(e) if e.name().as_ref() == b"xref" => {
                    let strong: Option<i64> = e
                        .try_get_attribute("strong")?
                        .map(|a| a.decode_and_unescape_value(reader.decoder()))
                        .transpose()?
                        .and_then(|v| v.parse().ok());
                    let bdb_id = e
                        .try_get_attribute("bdb")?
                        .map(|a| a.decode_and_unescape_value(reader.decoder()))
                        .transpose()?
                        .map(|v| v.into_owned());
                    let twot = e
                        .try_get_attribute("twot")?
                        .map(|a| a.decode_and_unescape_value(reader.decoder()))
                        .transpose()?
                        .map(|v| v.into_owned());
                    stmt.execute((&oshb_id, tidy(&word), strong, bdb_id, twot))?;
                    rows += 1;
                }
                Event::End(e) if e.name().as_ref() == b"w" => in_word = false,
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
    }
    tx.commit()?;

    db.execute_batch(
        "CREATE INDEX idx_lexical_index_strong ON lexical_index(strong);
         CREATE INDEX idx_lexical_index_bdb ON lexical_index(bdb_id);",
    )?;
    Ok(rows)
}

/// Harvest the explicit `etym root="…"` attributes from LexicalIndex.xml. These
/// are the lexicographers' canonical roots (e.g. `אבד`), already unpointed
/// consonants. Returns the raw root strings; normalisation to a triliteral is
/// done by the caller via [`Root::parse`].
fn collect_etym_roots(path: &Path) -> Result<Vec<String>> {
    let mut reader =
        Reader::from_file(path).with_context(|| format!("opening {}", path.display()))?;
    let mut buf = Vec::new();
    let mut out = Vec::new();
    loop {
        // <etym> may carry text children, so it is a Start event; the root is
        // on the attribute, not the body.
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == b"etym" => {
                if let Some(a) = e.try_get_attribute("root")? {
                    out.push(a.decode_and_unescape_value(reader.decoder())?.into_owned());
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

/// Build the authoritative `roots` inventory: every distinct triliteral root
/// the lexicon knows about, used to prune the reverse-parser's over-generated
/// candidate roots. Two independent sources are unioned, each normalised
/// through [`Root::parse`] (folds final forms, strips niqqud, keeps only
/// exactly-triliteral entries):
///
/// - **Strong's lemmas** — `english.word`, excluding proper nouns (`n-pr*`,
///   `np`), whose names fold to spurious triliterals. A verb root frequently
///   surfaces only as a noun/adjective lemma, so restricting to verbs alone
///   dropped real roots; every common lexeme contributes its root.
/// - **`etym root` attributes** from LexicalIndex.xml — the lexicographers'
///   canonical roots, which cover roots that never surface as a lemma.
///
/// Neither source alone is complete (etym misses common verbs like עשה/ישב),
/// so the union is the inventory.
fn load_roots(db: &mut Connection, lexical_index: &Path) -> Result<usize> {
    db.execute(
        "CREATE TABLE roots(\
            root_id INTEGER PRIMARY KEY, \
            root TEXT NOT NULL UNIQUE, \
            gizra TEXT NOT NULL, \
            from_strong INTEGER NOT NULL, \
            from_etym INTEGER NOT NULL)",
        [],
    )?;

    // Source 1: lemmas already loaded into `english`, minus proper nouns.
    let mut strong_roots: BTreeSet<String> = BTreeSet::new();
    {
        let mut stmt = db.prepare(
            "SELECT word FROM english WHERE pos NOT LIKE 'n-pr%' AND pos <> 'np'",
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        for word in rows {
            if let Ok(root) = Root::parse(&word?) {
                strong_roots.insert(root.letters.iter().collect());
            }
        }
    }

    // Source 2: explicit etym roots.
    let mut etym_roots: BTreeSet<String> = BTreeSet::new();
    for raw in collect_etym_roots(lexical_index)? {
        if let Ok(root) = Root::parse(&raw) {
            etym_roots.insert(root.letters.iter().collect());
        }
    }

    let all: BTreeSet<&String> = strong_roots.union(&etym_roots).collect();

    let tx = db.transaction()?;
    let mut rows = 0;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO roots(root, gizra, from_strong, from_etym) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for root_str in &all {
            let gizra = Root::parse(root_str)
                .map(|r| {
                    r.classes
                        .iter()
                        .map(|g| format!("{g:?}"))
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            stmt.execute((
                root_str,
                gizra,
                strong_roots.contains(*root_str) as i64,
                etym_roots.contains(*root_str) as i64,
            ))?;
            rows += 1;
        }
    }
    tx.commit()?;
    Ok(rows)
}

/// Load the `roots` inventory from a built `lexicon.db` into a set of
/// triliterals, the form [`crate::morphology::parse_word_filtered`] consumes to
/// prune candidate roots. Each stored root is exactly three folded consonants.
pub fn load_root_inventory(lexicon_db: &Path) -> Result<HashSet<[char; 3]>> {
    let db = Connection::open(lexicon_db)
        .with_context(|| format!("opening {}", lexicon_db.display()))?;
    let mut stmt = db.prepare("SELECT root FROM roots")?;
    let mut set = HashSet::new();
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    for root in rows {
        let chars: Vec<char> = root?.chars().collect();
        if let [a, b, c] = chars[..] {
            set.insert([a, b, c]);
        }
    }
    Ok(set)
}

/// Load a noun-stem inventory from a built `lexicon.db` for the reverse noun
/// parser. Every common-noun and adjective headword (`pos` of `n-m`/`n-f`/`n`
/// or `a`/`a-…`, which excludes proper nouns `n-pr*`/`np`, verbs, and adverbs)
/// is classified into an inflection class by [`NounStem::classify`]. Returns one
/// [`NounStem`] per distinct headword.
pub fn load_noun_inventory(lexicon_db: &Path) -> Result<Vec<NounStem>> {
    let db = Connection::open(lexicon_db)
        .with_context(|| format!("opening {}", lexicon_db.display()))?;
    let mut stmt = db.prepare(
        "SELECT DISTINCT word FROM english \
         WHERE word <> '' AND (\
            pos LIKE 'n-m%' OR pos LIKE 'n-f%' OR pos = 'n' \
            OR pos = 'a' OR pos LIKE 'a-%')",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut stems = Vec::new();
    for word in rows {
        stems.push(NounStem::classify(&word?));
    }
    Ok(stems)
}

/// Load a proper-noun stem inventory from a built `lexicon.db` for the reverse
/// noun parser. Every proper-noun headword (`pos` `n-pr*` / `np`) is classified
/// into an inflection class by [`NounStem::classify`]. Names overwhelmingly
/// occur as the bare lemma (optionally with proclitics), but classifying them
/// lets the few that take pronominal suffixes or gentilic/plural forms match
/// too; because downstream matching is exact, a mis-guessed class only loses
/// recall and never invents a spurious analysis.
pub fn load_proper_inventory(lexicon_db: &Path) -> Result<Vec<NounStem>> {
    let db = Connection::open(lexicon_db)
        .with_context(|| format!("opening {}", lexicon_db.display()))?;
    let mut stmt = db.prepare(
        "SELECT DISTINCT word FROM english \
         WHERE word <> '' AND (pos LIKE 'n-pr%' OR pos = 'np')",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut stems = Vec::new();
    for word in rows {
        stems.push(NounStem::classify(&word?));
    }
    Ok(stems)
}

/// Generate a standalone SQLite database with the Strong's `english`, full
/// `bdb`, and `lexical_index` glue tables from the HebrewLexicon source.
pub fn generate_lexicon(src_texts: &Path, output: &Path) -> Result<usize> {
    let dir = src_texts.join("HebrewLexicon");

    if output.exists() {
        std::fs::remove_file(output)
            .with_context(|| format!("removing existing {}", output.display()))?;
    }

    let mut db =
        Connection::open(output).with_context(|| format!("opening {}", output.display()))?;

    let strongs = load_strongs(&mut db, &dir.join("HebrewStrong.xml"))?;
    info!("  {strongs} rows -> english");
    let bdb = load_bdb(&mut db, &dir.join("BrownDriverBriggs.xml"))?;
    info!("  {bdb} rows -> bdb");
    let index = load_lexical_index(&mut db, &dir.join("LexicalIndex.xml"))?;
    info!("  {index} rows -> lexical_index");
    let roots = load_roots(&mut db, &dir.join("LexicalIndex.xml"))?;
    info!("  {roots} rows -> roots");

    let total = strongs + bdb + index + roots;
    info!("Wrote {total} rows to {}", output.display());
    Ok(total)
}
