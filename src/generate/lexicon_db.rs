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
use serde_json::{Map, Value, json};

use crate::morphology::{Gizra, NounStem, Root, hebrew};

/// Map a BDB scripture-reference book code (the `r="Lev.19.28"` attribute) to
/// the full English book name the app's BDB cross-reference handler expects
/// (`Genesis`, `I Samuel`, `Song of Songs`, …). Covers the 39 OT books plus a
/// few spelling variants that occur in the source; unknown codes yield `None`
/// (the reference still renders as text, just not tappable).
fn osis_book(code: &str) -> Option<&'static str> {
    Some(match code {
        "Gen" => "Genesis",
        "Exod" | "Ex" => "Exodus",
        "Lev" => "Leviticus",
        "Num" => "Numbers",
        "Deut" => "Deuteronomy",
        "Josh" | "Jos" => "Joshua",
        "Judg" | "Jugd" => "Judges",
        "Ruth" => "Ruth",
        "1Sam" => "I Samuel",
        "2Sam" => "II Samuel",
        "1Kgs" | "iKgs" => "I Kings",
        "2Kgs" => "II Kings",
        "Isa" | "Is" => "Isaiah",
        "Jer" => "Jeremiah",
        "Ezek" | "Ez" => "Ezekiel",
        "Hos" | "Ho" | "Hosea" => "Hosea",
        "Joel" => "Joel",
        "Amos" => "Amos",
        "Obad" => "Obadiah",
        "Jonah" => "Jonah",
        "Mic" => "Micah",
        "Nah" => "Nahum",
        "Hab" => "Habakkuk",
        "Zeph" | "Zp" => "Zephaniah",
        "Hag" => "Haggai",
        "Zech" | "Zec" => "Zechariah",
        "Mal" => "Malachi",
        "Ps" => "Psalms",
        "Prov" => "Proverbs",
        "Job" | "Jb" => "Job",
        "Song" => "Song of Songs",
        "Lam" => "Lamentations",
        "Eccl" => "Ecclesiastes",
        "Esth" => "Esther",
        "Dan" => "Daniel",
        "Ezra" => "Ezra",
        "Neh" => "Nehemiah",
        "1Chr" => "I Chronicles",
        "2Chr" => "II Chronicles",
        _ => return None,
    })
}

/// Convert a BDB `r` reference attribute (`Lev.19.28`) into the `Book C:V` href
/// the app parses (`Leviticus 19:28`). Returns `None` when the book code is
/// unknown or the shape isn't `Book.Chapter.Verse`.
fn ref_href(r: &str) -> Option<String> {
    let mut parts = r.split('.');
    let book = osis_book(parts.next()?)?;
    let chapter = parts.next()?;
    let verse = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if chapter.parse::<u32>().is_err() || verse.parse::<u32>().is_err() {
        return None;
    }
    Some(format!("{book} {chapter}:{verse}"))
}

/// One styled run of text in a BDB definition, mirroring the span objects the
/// app's `_BdbContent` widget renders: `t` (text) plus optional `b`/`i`/`s`/
/// `rtl` style flags, an `href` scripture-reference target, and an `xref` BDB
/// entry id (the `<w src>` cross-reference target the app navigates to).
#[derive(Clone, Copy, Default)]
struct Style {
    b: bool,
    i: bool,
    s: bool,
    rtl: bool,
}

fn span(text: &str, style: Style, href: Option<&str>, xref: Option<&str>) -> Option<Value> {
    if text.is_empty() {
        return None;
    }
    let mut m = Map::new();
    m.insert("t".into(), json!(text));
    if style.b {
        m.insert("b".into(), json!(true));
    }
    if style.i {
        m.insert("i".into(), json!(true));
    }
    if style.s {
        m.insert("s".into(), json!(true));
    }
    if style.rtl {
        m.insert("rtl".into(), json!(true));
    }
    if let Some(h) = href {
        m.insert("href".into(), json!(h));
    }
    if let Some(x) = xref {
        m.insert("xref".into(), json!(x));
    }
    Some(Value::Object(m))
}

/// A node in a BDB entry's sense tree: an optional `num` (sense number), an
/// optional `form` (the `<stem>`, e.g. `Qal`/`Niph`), its definition spans, and
/// any nested sub-senses.
#[derive(Default)]
struct Sense {
    num: Option<String>,
    form: String,
    definition: Vec<Value>,
    senses: Vec<Sense>,
}

impl Sense {
    fn to_json(&self) -> Value {
        let mut m = Map::new();
        if let Some(n) = &self.num {
            m.insert("num".into(), json!(n));
        }
        let form = self.form.trim();
        if !form.is_empty() {
            m.insert("form".into(), json!(form));
        }
        if !self.definition.is_empty() {
            m.insert("definition".into(), Value::Array(self.definition.clone()));
        }
        if !self.senses.is_empty() {
            m.insert(
                "senses".into(),
                Value::Array(self.senses.iter().map(Sense::to_json).collect()),
            );
        }
        Value::Object(m)
    }
}

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

/// Normalise the text of one definition span. The BDB XML is mixed content
/// laid out across lines, so raw text nodes carry the source file's newlines
/// and tab indentation between inline elements — which the app renders
/// verbatim as stray whitespace. Collapse internal runs to single spaces, but
/// keep a single leading/trailing space when the original had one so adjacent
/// styled runs stay separated (`"= "` + `"decide"`). A whitespace-only node is
/// structural indentation when it spans a line break (drop it) or a real
/// same-line gap otherwise (keep one space).
fn tidy_span_text(s: &str) -> String {
    let core = tidy(s);
    if core.is_empty() {
        return if s.contains('\n') {
            String::new()
        } else {
            " ".to_string()
        };
    }
    let mut out = String::with_capacity(core.len() + 2);
    if s.starts_with(char::is_whitespace) {
        out.push(' ');
    }
    out.push_str(&core);
    if s.ends_with(char::is_whitespace) {
        out.push(' ');
    }
    out
}

/// Expand the BDB abbreviation `v.` (Latin *vide*, "see") to the English word
/// in definition text, so a cross-reference reads `see X` rather than `v. X`.
/// Only the standalone token is touched, so `adv.`/`deriv.`/`subv.` are left
/// intact; and `v.` immediately before a number is a verse citation, not
/// *vide*, so it is kept too. Leading/trailing spacing is preserved (the spans
/// abut styled runs, so the gap carries meaning).
fn expand_vide(s: &str) -> String {
    let toks: Vec<&str> = s.split_whitespace().collect();
    if toks.is_empty() {
        return s.to_string();
    }
    let mut parts: Vec<&str> = Vec::with_capacity(toks.len());
    for (i, t) in toks.iter().enumerate() {
        let next_is_num = toks
            .get(i + 1)
            .and_then(|n| n.chars().next())
            .is_some_and(|c| c.is_ascii_digit());
        parts.push(if *t == "v." && !next_is_num { "see" } else { t });
    }
    let mut out = String::with_capacity(s.len() + 4);
    if s.starts_with(char::is_whitespace) {
        out.push(' ');
    }
    out.push_str(&parts.join(" "));
    if s.ends_with(char::is_whitespace) {
        out.push(' ');
    }
    out
}

/// A short fallback gloss for a BDB entry whose headword carried no bold
/// `<def>`, so [`load_bdb`] collected nothing into `gloss`. These are mostly
/// proper names and cross-references ("v. אבה"), whose meaning lives in the
/// part-of-speech marker and the plain descriptive text of the senses rather
/// than a tagged definition — without this the app's Lexicon tab shows a blank
/// gloss for ~40% of entries (every Simeon, Shimei, place name, …).
///
/// We take the entry's leading sense plus its first descriptive sub-sense,
/// concatenating their plain span text (scripture citations dropped — they are
/// locators, not meaning) and stripping the leading BDB occurrence-count digits.
/// The part-of-speech marker (`n.pr.m.`) is kept so a bare name still reads as
/// one. Empty when the entry has no usable definition text at all.
fn fallback_gloss(senses: &[Value]) -> String {
    let sense_text = |s: &Value| -> String {
        let Some(defs) = s.get("definition").and_then(Value::as_array) else {
            return String::new();
        };
        let mut out = String::new();
        for sp in defs {
            if sp.get("href").is_some() {
                continue;
            }
            if let Some(t) = sp.get("t").and_then(Value::as_str) {
                out.push_str(t);
            }
        }
        tidy(&out)
    };

    // Leading sense (often just the POS marker), then the first sense that adds
    // descriptive words — enough to name a proper noun, not the whole list.
    let mut parts: Vec<String> = Vec::new();
    for s in senses {
        let t = sense_text(s);
        if !t.is_empty() {
            parts.push(t);
        }
        if parts.len() >= 2 {
            break;
        }
    }
    let joined = parts.join(" ");

    // Drop a leading bare occurrence count ("44 n.pr.m. …" → "n.pr.m. …").
    let body = joined
        .split_once(' ')
        .filter(|(head, _)| !head.is_empty() && head.chars().all(|c| c.is_ascii_digit()))
        .map_or(joined.as_str(), |(_, rest)| rest);

    let cleaned = body.trim().trim_end_matches([',', ';', ':']).trim();

    // Safety cap: keep glosses to a single readable phrase, on a word boundary.
    const MAX: usize = 80;
    if cleaned.chars().count() <= MAX {
        return cleaned.to_string();
    }
    let cut = cleaned
        .char_indices()
        .nth(MAX)
        .map_or(cleaned.len(), |(i, _)| i);
    let cut = cleaned[..cut].rfind(' ').unwrap_or(cut);
    format!("{}…", cleaned[..cut].trim_end())
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

/// Parse BrownDriverBriggs.xml into the `bdb` table, preserving the sense tree
/// as structured `content_json` and keying every entry to its triliteral root.
/// Returns rows written.
///
/// BDB groups its entries into `<section>`s, each headed by a `type="root"`
/// entry whose headword fixes the root for that section; the derivative entries
/// that follow (nouns, adjectives, …) inherit it. We reduce the root entry's
/// headword to a triliteral via [`Root::parse`] — the same normalisation the
/// reverse parser and `roots` table use — so a stored root joins directly onto
/// `hebrew.db`'s `analyses.root`.
///
/// Each entry's prose becomes a `content_json` of the form
/// `{"senses":[{num?,form?,definition:[{t,b?,i?,s?,rtl?,href?,xref?}],senses?}]}`,
/// matching the span schema the app's `_BdbContent` widget renders: `<def>`
/// becomes a bold span, `<ref>` an href span (book code mapped to the app's
/// `Book C:V` form), `<w>`/`<foreign>` an RTL/italic span (a `<w src>` also
/// carries `xref`, the target entry id the app navigates to), and `<stem>` the
/// sense's `form`. The leading headword gloss is also kept flat in `gloss`.
/// Consonant skeleton of a pointed Hebrew word: niqqud and any non-letter marks
/// stripped, final-form letters folded to their medial base. Used as the noun
/// bridge — `hebrew.db` noun stems carry final forms and vowels, so matching a
/// stem to its BDB lexeme (and thence its root) needs both sides reduced to bare
/// medial consonants.
fn consonants(word: &str) -> String {
    word.chars()
        .filter_map(|c| {
            let n = c as u32;
            if !(0x05D0..=0x05EA).contains(&n) {
                return None;
            }
            Some(match c {
                '\u{05DA}' => '\u{05DB}',
                '\u{05DD}' => '\u{05DE}',
                '\u{05DF}' => '\u{05E0}',
                '\u{05E3}' => '\u{05E4}',
                '\u{05E5}' => '\u{05E6}',
                other => other,
            })
        })
        .collect()
}

/// Pre-scan BDB to map every entry id to its headword (the first `<w>`'s text).
/// Cross-references inside an entry point at a target by id alone — the empty
/// `<w src="a.eg.aa"/>` form carries no text — so [`load_bdb`] needs this map to
/// fill in what the target reads as. One cheap pass; the file is ~20MB.
fn bdb_headwords(path: &Path) -> Result<std::collections::HashMap<String, String>> {
    let mut reader =
        Reader::from_file(path).with_context(|| format!("opening {}", path.display()))?;
    let mut buf = Vec::new();
    let mut map = std::collections::HashMap::new();
    let mut id = String::new();
    let mut word = String::new();
    let mut in_headword = false;
    let mut headword_done = false;
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => match e.name().as_ref() {
                b"entry" => {
                    id = e
                        .try_get_attribute("id")?
                        .map(|a| a.decode_and_unescape_value(reader.decoder()))
                        .transpose()?
                        .map(|v| v.into_owned())
                        .unwrap_or_default();
                    word.clear();
                    headword_done = false;
                }
                b"w" if !headword_done => {
                    in_headword = true;
                    headword_done = true;
                }
                _ => {}
            },
            Event::Text(t) if in_headword => word.push_str(&t.unescape()?),
            Event::End(e) => match e.name().as_ref() {
                b"w" => in_headword = false,
                b"entry" if !id.is_empty() => {
                    map.insert(id.clone(), tidy(&word));
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(map)
}

fn load_bdb(db: &mut Connection, path: &Path) -> Result<usize> {
    db.execute(
        "CREATE TABLE bdb(\
            bdb_id TEXT PRIMARY KEY, root TEXT NOT NULL, word TEXT, cons TEXT, pos TEXT, \
            gloss TEXT, content_json TEXT, status TEXT)",
        [],
    )?;

    // Cross-reference targets are resolved to their headword text via this map
    // (built in a cheap first pass, since a target may be defined later).
    let headwords = bdb_headwords(path)?;

    let mut reader =
        Reader::from_file(path).with_context(|| format!("opening {}", path.display()))?;
    let mut buf = Vec::new();

    // Per-entry parse state.
    let mut bdb_id = String::new();
    let mut is_root_entry = false;
    let mut word = String::new();
    let mut pos = String::new();
    let mut status = String::new();
    // Sense stack: index 0 is the entry's intro (headword pos/def), deeper
    // indices are nested <sense> elements currently open.
    let mut stack: Vec<Sense> = Vec::new();
    let mut gloss_parts: Vec<String> = Vec::new();

    // Inline styling + routing flags.
    let mut headword_done = false;
    let mut in_headword = false;
    let mut in_stem = false;
    let mut in_status = false;
    let mut in_pos = false;
    let mut style = Style::default();
    let mut href: Option<String> = None;
    // BDB entry id a cross-reference `<w src>` points at, for the open `<w>`.
    let mut xref: Option<String> = None;

    // Root inherited across a section from its `type="root"` entry.
    let mut current_root = String::new();

    let tx = db.transaction()?;
    let mut rows = 0;
    {
        let mut stmt =
            tx.prepare("INSERT OR REPLACE INTO bdb VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)")?;
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => match e.name().as_ref() {
                    b"section" => current_root.clear(),
                    b"entry" => {
                        bdb_id = e
                            .try_get_attribute("id")?
                            .map(|a| a.decode_and_unescape_value(reader.decoder()))
                            .transpose()?
                            .map(|v| v.into_owned())
                            .unwrap_or_default();
                        is_root_entry = e
                            .try_get_attribute("type")?
                            .map(|a| a.decode_and_unescape_value(reader.decoder()))
                            .transpose()?
                            .map(|v| v == "root")
                            .unwrap_or(false);
                        word.clear();
                        pos.clear();
                        status.clear();
                        gloss_parts.clear();
                        stack = vec![Sense::default()];
                        headword_done = false;
                        in_headword = false;
                        in_stem = false;
                        in_status = false;
                        in_pos = false;
                        style = Style::default();
                        href = None;
                        xref = None;
                    }
                    // First <w> is the headword; later <w src="…"> are inline
                    // cross-references rendered as RTL Hebrew spans, tappable to
                    // navigate to the entry named by `src`.
                    b"w" if !stack.is_empty() && !headword_done => {
                        in_headword = true;
                        headword_done = true;
                    }
                    b"w" => {
                        style.rtl = true;
                        xref = e
                            .try_get_attribute("src")?
                            .map(|a| a.decode_and_unescape_value(reader.decoder()))
                            .transpose()?
                            .map(|v| v.into_owned());
                    }
                    b"sense" => {
                        let num = e
                            .try_get_attribute("n")?
                            .map(|a| a.decode_and_unescape_value(reader.decoder()))
                            .transpose()?
                            .map(|v| v.into_owned());
                        stack.push(Sense {
                            num,
                            ..Sense::default()
                        });
                    }
                    b"stem" => in_stem = true,
                    b"def" => style.b = true,
                    b"pos" => {
                        style.i = true;
                        in_pos = true;
                    }
                    b"foreign" => style.i = true,
                    b"ref" => {
                        href = e
                            .try_get_attribute("r")?
                            .map(|a| a.decode_and_unescape_value(reader.decoder()))
                            .transpose()?
                            .and_then(|r| ref_href(&r));
                    }
                    b"status" => in_status = true,
                    _ => {}
                },
                Event::Text(t) => {
                    let txt = t.unescape()?;
                    if in_headword {
                        word.push_str(&txt);
                    } else if in_stem {
                        if let Some(top) = stack.last_mut() {
                            top.form.push_str(&txt);
                        }
                    } else if in_status {
                        status.push_str(&txt);
                    } else {
                        let is_top = stack.len() == 1;
                        if let Some(top) = stack.last_mut() {
                            if in_pos {
                                pos.push_str(txt.trim());
                            }
                            // Headword-level <def> text is the entry's gloss.
                            if style.b && is_top {
                                let g = tidy(&txt);
                                if !g.is_empty() {
                                    gloss_parts.push(g);
                                }
                            }
                            if let Some(sp) = span(
                                &expand_vide(&tidy_span_text(&txt)),
                                style,
                                href.as_deref(),
                                xref.as_deref(),
                            ) {
                                top.definition.push(sp);
                            }
                        }
                    }
                }
                // A self-closing `<w src="…"/>` cross-reference carries no text,
                // so its target headword is unknown until resolved through the
                // pre-built map. Emit it as a tappable RTL span (the same shape a
                // text-bearing `<w src>` produces) so the entry no longer reads
                // as a bare "v.".
                Event::Empty(e) if e.name().as_ref() == b"w" => {
                    if let Some(src) = e
                        .try_get_attribute("src")?
                        .map(|a| a.decode_and_unescape_value(reader.decoder()))
                        .transpose()?
                        .map(|v| v.into_owned())
                        && let Some(target) = headwords.get(&src)
                        && let Some(top) = stack.last_mut()
                    {
                        let rtl = Style {
                            rtl: true,
                            ..Style::default()
                        };
                        if let Some(sp) = span(target, rtl, None, Some(&src)) {
                            top.definition.push(sp);
                        }
                    }
                }
                Event::End(e) => match e.name().as_ref() {
                    b"w" => {
                        if in_headword {
                            in_headword = false;
                        } else {
                            style.rtl = false;
                            xref = None;
                        }
                    }
                    b"stem" => in_stem = false,
                    b"def" => style.b = false,
                    b"pos" => {
                        style.i = false;
                        in_pos = false;
                    }
                    b"foreign" => style.i = false,
                    b"ref" => href = None,
                    b"status" => in_status = false,
                    b"sense" => {
                        if stack.len() > 1 {
                            let done = stack.pop().unwrap();
                            stack.last_mut().unwrap().senses.push(done);
                        }
                    }
                    b"entry" => {
                        let intro = stack.pop().unwrap_or_default();
                        stack.clear();

                        // The intro's own definition spans (headword pos/def)
                        // become a leading sense; its collected <sense> children
                        // follow.
                        let mut senses: Vec<Value> = Vec::new();
                        if !intro.definition.is_empty() {
                            senses.push(
                                Sense {
                                    definition: intro.definition.clone(),
                                    ..Sense::default()
                                }
                                .to_json(),
                            );
                        }
                        senses.extend(intro.senses.iter().map(Sense::to_json));
                        // The headline gloss is the headword's bold <def> text;
                        // when an entry has none (proper names, cross-references)
                        // derive a readable fallback from the sense text so the
                        // Lexicon tab is never blank.
                        let gloss = if gloss_parts.is_empty() {
                            fallback_gloss(&senses)
                        } else {
                            gloss_parts.join("; ")
                        };

                        let content_json = Value::Object({
                            let mut m = Map::new();
                            m.insert("senses".into(), Value::Array(senses));
                            m
                        })
                        .to_string();

                        let word = tidy(&word);
                        if is_root_entry && let Ok(r) = Root::parse(&word) {
                            current_root = r.letters.iter().collect();
                        }
                        // Derivatives inherit the section root; if it is still
                        // unknown, fall back to parsing this headword.
                        let root = if !current_root.is_empty() {
                            current_root.clone()
                        } else {
                            Root::parse(&word)
                                .map(|r| r.letters.iter().collect())
                                .unwrap_or_default()
                        };

                        stmt.execute((
                            &bdb_id,
                            &root,
                            &word,
                            consonants(&word),
                            tidy(&pos),
                            gloss,
                            &content_json,
                            tidy(&status),
                        ))?;
                        rows += 1;
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

    db.execute("CREATE INDEX idx_bdb_root ON bdb(root)", [])?;
    db.execute("CREATE INDEX idx_bdb_cons ON bdb(cons)", [])?;
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
        let mut stmt =
            db.prepare("SELECT word FROM english WHERE pos NOT LIKE 'n-pr%' AND pos <> 'np'")?;
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

/// Load a triliteral-root set from `lexicon.db`, selecting rows with the given
/// SQL predicate over the `roots` table. Each stored root is three folded
/// consonants; hollow roots additionally contribute their medial vav/yod twin
/// (see below). Shared by [`load_root_inventory`] and
/// [`load_canonical_root_inventory`].
fn load_roots_where(lexicon_db: &Path, predicate: &str) -> Result<HashSet<[char; 3]>> {
    let db = Connection::open(lexicon_db)
        .with_context(|| format!("opening {}", lexicon_db.display()))?;
    let mut stmt = db.prepare(&format!("SELECT root FROM roots WHERE {predicate}"))?;
    let mut set = HashSet::new();
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    for root in rows {
        let chars: Vec<char> = root?.chars().collect();
        if let [a, b, c] = chars[..] {
            set.insert([a, b, c]);
            // A hollow root's medial vav/yod spelling is lexically arbitrary,
            // but it selects the û-class (יָקוּם) vs î-class (יָשִׂים) inflection.
            // BDB records only one spelling per hollow lexeme (שׂום, לון, מוש —
            // never the yod twin), so î-class surfaces (יָשִׂים, שִׂים, יָלִין)
            // of a vav-spelled root — and vice versa — would be pruned by the
            // filter. Admit the medial twin so both inflection classes survive.
            let root = Root::from_letters([a, b, c]);
            if root.has(Gizra::Hollow) {
                let twin = match b {
                    hebrew::letter::VAV => hebrew::letter::YOD,
                    _ => hebrew::letter::VAV,
                };
                set.insert([a, twin, c]);
            }
        }
    }
    Ok(set)
}

/// Load the `roots` inventory from a built `lexicon.db` into a set of
/// triliterals, the form [`crate::morphology::parse_word_filtered`] consumes to
/// prune candidate roots. Each stored root is exactly three folded consonants.
pub fn load_root_inventory(lexicon_db: &Path) -> Result<HashSet<[char; 3]>> {
    load_roots_where(lexicon_db, "1")
}

/// Load only the *canonical* roots — those carried by the lexicographers' etym
/// tree (`from_etym = 1`) — plus their hollow twins. This is a strict subset of
/// [`load_root_inventory`]: it excludes roots that exist only as the folded
/// skeleton of a Strong's lemma (`from_etym = 0`), which for nouns like יוֹם
/// "day" (→ יומ/יימ) or מַיִם "water" (→ מימ/מום) are not real verb roots and
/// whose generated paradigms collide en masse with noun morphology (the ־ִים
/// masculine plural especially). Real hollow verbs whose î-spelling looks
/// strong-only (חיל, דיש, זוד) are recovered via the hollow twin of their
/// canonical vav-spelled etym root (חול, דוש, זיד). Used to suppress spurious
/// verb readings on surfaces the noun pass already explains.
pub fn load_canonical_root_inventory(lexicon_db: &Path) -> Result<HashSet<[char; 3]>> {
    load_roots_where(lexicon_db, "from_etym = 1")
}

/// Load a noun-stem inventory from a built `lexicon.db` for the reverse noun
/// parser. Every common-noun and adjective headword (`pos` of `n-m`/`n-f`/`n`
/// or `a`/`a-…`, which excludes proper nouns `n-pr*`/`np`, verbs, and adverbs)
/// is classified into an inflection class by [`NounStem::classify`]. Returns one
/// [`NounStem`] per distinct headword.
pub fn load_noun_inventory(lexicon_db: &Path) -> Result<Vec<NounStem>> {
    let db = Connection::open(lexicon_db)
        .with_context(|| format!("opening {}", lexicon_db.display()))?;

    // Two sources, unioned: Strong's `english` headwords (n-m/n-f/n/adjective)
    // and the much larger BDB common-noun set (`bdb.pos` like `n.m`/`n.f`/`n.[m.]`,
    // excluding proper nouns `n.pr*`, which `load_proper_inventory` covers). BDB
    // adds ~1,265 stems the Strong's list lacks (נֶסֶךְ, קֶרֶב, …). Dedup on the
    // cantillation-stripped form so the same lemma from both sources is one stem;
    // exact-match downstream means a stray stem only ever loses recall, never
    // invents an analysis.
    let mut seen: HashSet<String> = HashSet::new();
    let mut stems = Vec::new();
    let add =
        |word: String, is_adj: bool, stems: &mut Vec<NounStem>, seen: &mut HashSet<String>| {
            let key: String = word.chars().filter(|c| !is_cantillation(*c)).collect();
            if !key.is_empty() && seen.insert(key) {
                // Segolates expand to all three base classes (see class_variants);
                // other stems yield just themselves. The adjective flag enables
                // agreement inflection (feminine sg/pl) and rides through both.
                stems.extend(
                    NounStem::classify(&word)
                        .with_adjective(is_adj)
                        .class_variants(),
                );
            }
        };

    // Strong's `english` headwords: nouns (n-m/n-f/n) and adjectives (a/a-*).
    let mut stmt = db.prepare(
        "SELECT DISTINCT word, (pos = 'a' OR pos LIKE 'a-%') AS is_adj FROM english \
         WHERE word <> '' AND (\
            pos LIKE 'n-m%' OR pos LIKE 'n-f%' OR pos = 'n' \
            OR pos = 'a' OR pos LIKE 'a-%')",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, bool>(1)?)))?;
    for row in rows {
        let (word, is_adj) = row?;
        add(word, is_adj, &mut stems, &mut seen);
    }

    // BDB common nouns (`n.*`, excluding proper nouns `n.pr*`) plus adjectives
    // (`adj*` — adj, adj.gent, …; ~550 stems the Strong's list and the n.* query
    // both miss). Proper-noun adjectives stay excluded (they start `n.pr`).
    let mut bdb = db.prepare(
        "SELECT DISTINCT word, (pos LIKE 'adj%') AS is_adj FROM bdb \
         WHERE word <> '' AND (\
            (pos LIKE 'n.%' AND pos NOT LIKE 'n.pr%') OR pos LIKE 'adj%')",
    )?;
    let rows = bdb.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, bool>(1)?)))?;
    for row in rows {
        let (word, is_adj) = row?;
        add(word, is_adj, &mut stems, &mut seen);
    }

    Ok(stems)
}

/// Hebrew cantillation accents (te'amim) and meteg, which BDB lemmas carry but
/// the parser's surfaces do not. Stripped when keying noun stems for dedup.
fn is_cantillation(c: char) -> bool {
    matches!(c as u32, 0x0591..=0x05AF | 0x05BD | 0x05BF)
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

    let mut seen: HashSet<String> = HashSet::new();
    let mut stems = Vec::new();
    let add = |word: String, stems: &mut Vec<NounStem>, seen: &mut HashSet<String>| {
        let key: String = word.chars().filter(|c| !is_cantillation(*c)).collect();
        if !key.is_empty() && seen.insert(key) {
            stems.push(NounStem::classify(&word));
        }
    };

    for word in rows {
        let word = word?;
        // Some proper nouns in the lexicon are multi-word (e.g. "עֲבֵד נְגוֹ").
        // Since the Bible tokenizer splits on spaces, we need the individual
        // parts in the inventory too.
        for part in word.split_whitespace() {
            add(part.to_string(), &mut stems, &mut seen);
        }
        add(word, &mut stems, &mut seen);
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Build a `senses` array shaped like the one `load_bdb` assembles, so the
    // fallback runs against the same JSON the entry handler passes in.
    fn senses(v: Value) -> Vec<Value> {
        v.as_array().unwrap().clone()
    }

    #[test]
    fn expand_vide_replaces_the_standalone_token_only() {
        // The cross-reference stub (its own text node, spaces preserved).
        assert_eq!(expand_vide(" v. "), " see ");
        // Mid-phrase vide.
        assert_eq!(expand_vide("in Naphtali, v. "), "in Naphtali, see ");
        // "see also", "see supr.".
        assert_eq!(expand_vide(" v. also "), " see also ");
        // Tokens that merely end in "v." are untouched.
        assert_eq!(expand_vide("adv. of negation"), "adv. of negation");
        assert_eq!(expand_vide(" deriv. unknown "), " deriv. unknown ");
        // "v." before a number is a verse citation, not vide.
        assert_eq!(expand_vide("v. 18"), "v. 18");
        // Whitespace-only / empty nodes pass through unchanged.
        assert_eq!(expand_vide(" "), " ");
        assert_eq!(expand_vide(""), "");
    }

    #[test]
    fn fallback_names_a_proper_noun_from_its_first_descriptive_sense() {
        // שִׁמְעוֹן: leading sense is just the occurrence count + POS marker; the
        // identity lives in the first numbered sense.
        let s = senses(json!([
            {"definition": [{"t": " 44 "}, {"i": true, "t": "n.pr.m"}, {"t": ". "}]},
            {"num": "1", "definition": [{"t": "second son of Jacob and Leah"}]},
            {"num": "2", "definition": [{"t": "tribal name"}]},
        ]));
        assert_eq!(fallback_gloss(&s), "n.pr.m. second son of Jacob and Leah");
    }

    #[test]
    fn fallback_uses_inline_description_and_drops_citations() {
        // שָׁמָע: one sense, POS + description + a scripture ref locator.
        let s = senses(json!([
            {"definition": [
                {"t": " "}, {"i": true, "t": "n.pr.m"}, {"t": ". a hero of David "},
                {"href": "I Chronicles 11:44", "t": "1 Ch 11:44"},
            ]},
        ]));
        assert_eq!(fallback_gloss(&s), "n.pr.m. a hero of David");
    }

    #[test]
    fn fallback_is_empty_when_there_is_no_text() {
        assert_eq!(fallback_gloss(&[]), "");
        // Only a scripture ref — no descriptive words to show.
        let s = senses(json!([{"definition": [{"t": " "}, {"href": "x", "t": "2 S 23:31"}]}]));
        assert_eq!(fallback_gloss(&s), "");
    }

    #[test]
    fn fallback_truncates_on_a_word_boundary() {
        let long = "a".repeat(40) + " " + &"b".repeat(60);
        let s = senses(json!([{"definition": [{"t": long}]}]));
        let g = fallback_gloss(&s);
        assert!(g.ends_with('…'));
        assert!(!g.contains('b'), "should cut at the space before the long run");
    }
}
