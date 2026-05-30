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

use std::path::Path;

use anyhow::{Context, Result};
use log::info;
use quick_xml::Reader;
use quick_xml::events::Event;
use rusqlite::Connection;

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
                    }
                    b"w" => {
                        // The headword is the only <w> carrying an `xlit` attr;
                        // cross-reference <w src="…"> elements lack it and just
                        // contribute their text to the active prose section.
                        if let Some(ent) = entry.as_mut()
                            && let Some(a) = e.try_get_attribute("xlit")?
                        {
                            in_headword = true;
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

    let total = strongs + bdb + index;
    info!("Wrote {total} rows to {}", output.display());
    Ok(total)
}
