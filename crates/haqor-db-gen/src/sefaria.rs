//! Klein and Jastrow, from Sefaria's digitisations.
//!
//! Two stages, so the app never depends on Sefaria being reachable:
//!
//! - [`import_sefaria`] reads the `lexicon_entry` collection of Sefaria's
//!   nightly MongoDB dump (converted to JSON lines by `bsondump`; see
//!   `scripts/fetch-sefaria-lexicons.sh`), keeps the entries a reader of the
//!   Hebrew Bible or the Peshitta can reach, and writes them to
//!   `src_texts/Sefaria/{klein,jastrow}.jsonl`. Those files are checked in: they
//!   are the pinned source, and regenerating them is a deliberate refresh.
//! - [`load_sefaria`] reads the checked-in files during `gen-lexicon` and
//!   writes the `dictionary` and `dictionary_form` tables beside `bdb`, with
//!   each article converted from Sefaria's HTML into the same span JSON BDB
//!   entries use.
//!
//! *Klein*, A Comprehensive Etymological Dictionary of the Hebrew Language
//! (Carta, 1987), is licensed by Sefaria CC BY-NC. *Jastrow*, A Dictionary of
//! the Targumim, the Talmud Babli and Yerushalmi, and the Midrashic Literature
//! (Luzac, 1903), is in the public domain.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use log::info;
use rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::lexicon_db::{bdb_headwords, consonants};
use haqor_core::data_support::bare_letters;
use haqor_core::transliterate;

/// One Sefaria lexicon: how the dump names it, how its own cross-references
/// name it, and where its filtered entries live.
struct Source {
    /// The runtime `source` column.
    id: &'static str,
    /// `parent_lexicon` in the dump.
    lexicon: &'static str,
    /// The prefix of a `data-ref` that points at another entry of this lexicon.
    ref_prefix: &'static str,
}

const SOURCES: [Source; 2] = [
    Source {
        id: "klein",
        lexicon: "Klein Dictionary",
        ref_prefix: "Klein Dictionary, ",
    },
    Source {
        id: "jastrow",
        lexicon: "Jastrow Dictionary",
        ref_prefix: "Jastrow, ",
    },
];

/// Dump fields that do not survive the import: `_id` is Mongo's, `refs`
/// repeats the links already inside the article, `parent_lexicon` is implied
/// by the file, and `prev_hw`/`next_hw` name neighbours the filter may drop.
const DROPPED_FIELDS: [&str; 5] = ["_id", "refs", "parent_lexicon", "prev_hw", "next_hw"];

/// How many entries of one lexicon the import read and kept.
#[derive(Debug)]
pub struct ImportSummary {
    pub source: &'static str,
    pub read: usize,
    pub kept: usize,
}

/// Filter the dump's Klein and Jastrow entries into `output`.
///
/// `dump` is the `lexicon_entry` collection as JSON lines. `src_texts` supplies
/// the headwords an entry can be reached from: BDB's (from `HebrewLexicon`) and
/// SEDRA's lexemes and roots (from `SEDRA`).
pub fn import_sefaria(dump: &Path, src_texts: &Path, output: &Path) -> Result<Vec<ImportSummary>> {
    let reachable = reachable_skeletons(src_texts)?;
    info!("{} headword skeletons in BDB and SEDRA", reachable.len());

    let mut entries: HashMap<&str, Vec<Map<String, Value>>> = HashMap::new();
    let file = std::fs::File::open(dump).with_context(|| format!("opening {}", dump.display()))?;
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Value::Object(entry) = serde_json::from_str(&line)
            .with_context(|| format!("{} line {}", dump.display(), index + 1))?
        else {
            bail!("{} line {} is not an object", dump.display(), index + 1);
        };
        let lexicon = entry.get("parent_lexicon").and_then(Value::as_str);
        if let Some(source) = SOURCES.iter().find(|s| Some(s.lexicon) == lexicon) {
            entries.entry(source.id).or_default().push(entry);
        }
    }

    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    let mut summaries = Vec::new();
    for source in &SOURCES {
        let read = entries.remove(source.id).unwrap_or_default();
        if read.is_empty() {
            bail!("the dump holds no {} entries", source.lexicon);
        }
        let mut kept: Vec<Map<String, Value>> = read
            .iter()
            .filter(|entry| keep(source, entry, &reachable))
            .cloned()
            .map(|mut entry| {
                for field in DROPPED_FIELDS {
                    entry.remove(field);
                }
                entry
            })
            .collect();
        kept.sort_by(|a, b| str_field(a, "rid").cmp(str_field(b, "rid")));

        let path = output.join(format!("{}.jsonl", source.id));
        let mut out = BufWriter::new(
            std::fs::File::create(&path).with_context(|| format!("creating {}", path.display()))?,
        );
        for entry in &kept {
            serde_json::to_writer(&mut out, entry)?;
            out.write_all(b"\n")?;
        }
        out.flush()?;
        info!(
            "  {} of {} {} entries -> {}",
            kept.len(),
            read.len(),
            source.lexicon,
            path.display()
        );
        summaries.push(ImportSummary {
            source: source.id,
            read: read.len(),
            kept: kept.len(),
        });
    }
    Ok(summaries)
}

/// Consonant skeletons of every headword a reader can arrive from: BDB's
/// entries, and SEDRA's lexemes and roots (already Hebrew letters once
/// transliterated, so an Aramaic lexeme meets Jastrow's headword directly).
/// Bare letters, shin and sin alike, since SEDRA's one ש may be either.
fn reachable_skeletons(src_texts: &Path) -> Result<HashSet<String>> {
    let bdb = src_texts.join("HebrewLexicon/BrownDriverBriggs.xml");
    let mut skeletons: HashSet<String> = bdb_headwords(&bdb)
        .with_context(|| format!("reading {}", bdb.display()))?
        .into_values()
        .map(|r| bare_letters(&r.headword))
        .collect();
    let sedra = src_texts.join("SEDRA");
    for (file, column) in [("tblLexemes.txt", "strLexeme"), ("tblRoots.txt", "strRoot")] {
        let path = sedra.join(file);
        let mut reader =
            csv::Reader::from_path(&path).with_context(|| format!("opening {}", path.display()))?;
        let index = reader
            .headers()?
            .iter()
            .position(|h| h == column)
            .with_context(|| format!("{file} has no `{column}` column"))?;
        for record in reader.records() {
            skeletons.insert(bare_letters(&transliterate::sedra_to_hebrew(
                &record?[index],
            )));
        }
    }
    skeletons.remove("");
    Ok(skeletons)
}

/// Whether an entry is worth carrying: its own period marker says it belongs
/// to the biblical layer, or it shares a consonant skeleton with a BDB entry or
/// a SEDRA lexeme, so a reader of either text can land on it.
///
/// Klein marks each entry with the stratum it first appears in and leaves
/// biblical words unmarked, so what goes is the modern coinages (`NH`), the
/// foreign loans (`FW`) and the medieval and post-biblical words (`MH`,
/// `PBH`) that nothing in either corpus spells. Jastrow marks words that also
/// occur in the Bible `b. h.` and Aramaic ones `ch.` (Chaldaic); the rest of
/// his rabbinic Hebrew is kept only when its spelling is reachable.
fn keep(source: &Source, entry: &Map<String, Value>, reachable: &HashSet<String>) -> bool {
    let marker = clean_lang(str_field(entry, "language_code"));
    let marked = match source.id {
        "klein" => marker.is_empty() || marker.split([' ', ',']).any(|code| code == "BH"),
        _ => marker.contains("b. h") || marker.contains("ch."),
    };
    marked || headwords(entry).any(|word| reachable.contains(&bare_letters(word)))
}

/// The headword and its listed alternative spellings.
fn headwords(entry: &Map<String, Value>) -> impl Iterator<Item = &str> {
    std::iter::once(str_field(entry, "headword")).chain(
        entry
            .get("alt_headwords")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str),
    )
}

fn str_field<'a>(entry: &'a Map<String, Value>, key: &str) -> &'a str {
    entry.get(key).and_then(Value::as_str).unwrap_or("")
}

/// A period or language marker without the punctuation it was cut out of the
/// printed line with: `(b. h.;` → `b. h.`, `PBH,` → `PBH`. Jastrow's markers
/// keep their own dots (`ch.`); Klein's codes are bare capitals.
fn clean_lang(raw: &str) -> String {
    let trimmed = raw.trim_matches(|c: char| c.is_whitespace() || "(),;".contains(c));
    if trimmed
        .chars()
        .all(|c| c.is_ascii_uppercase() || " ,.".contains(c))
    {
        trimmed.trim_end_matches('.').to_string()
    } else {
        trimmed.to_string()
    }
}

/// Write the checked-in entries into `lexicon.db`. Returns the entries loaded.
pub(crate) fn load_sefaria(db: &mut Connection, dir: &Path) -> Result<usize> {
    db.execute_batch(
        "CREATE TABLE dictionary(
            source       TEXT NOT NULL,
            key          TEXT NOT NULL,
            word         TEXT NOT NULL,
            cons         TEXT NOT NULL,
            lang         TEXT,
            pos          TEXT,
            gloss        TEXT,
            content_json TEXT NOT NULL,
            PRIMARY KEY(source, key));
         CREATE TABLE dictionary_form(
            source TEXT NOT NULL,
            key    TEXT NOT NULL,
            cons   TEXT NOT NULL,
            PRIMARY KEY(source, key, cons));",
    )?;
    let tx = db.transaction()?;
    let mut total = 0;
    {
        let mut entry_stmt = tx.prepare(
            "INSERT INTO dictionary(source, key, word, cons, lang, pos, gloss, content_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        let mut form_stmt = tx.prepare(
            "INSERT OR IGNORE INTO dictionary_form(source, key, cons) VALUES (?1, ?2, ?3)",
        )?;
        for source in &SOURCES {
            let path = dir.join(format!("{}.jsonl", source.id));
            let entries = read_entries(&path)?;
            let links = Links::new(source, &entries);
            for entry in &entries {
                let article = Article::from_entry(entry, &links);
                let key = str_field(entry, "rid");
                let word = str_field(entry, "headword");
                entry_stmt.execute((
                    source.id,
                    key,
                    word,
                    consonants(word),
                    Some(article.lang.as_str()).filter(|l| !l.is_empty()),
                    Some(article.pos.as_str()).filter(|p| !p.is_empty()),
                    Some(article.gloss.as_str()).filter(|g| !g.is_empty()),
                    article.content.to_string(),
                ))?;
                for form in headwords(entry).map(consonants).filter(|c| !c.is_empty()) {
                    form_stmt.execute((source.id, key, form))?;
                }
            }
            info!(
                "  {} {} entries -> dictionary",
                entries.len(),
                source.lexicon
            );
            total += entries.len();
        }
    }
    tx.commit()?;
    Ok(total)
}

fn read_entries(path: &Path) -> Result<Vec<Map<String, Value>>> {
    let file = std::fs::File::open(path).with_context(|| {
        format!(
            "opening {} (refresh it with scripts/fetch-sefaria-lexicons.sh)",
            path.display()
        )
    })?;
    let mut entries = Vec::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        match serde_json::from_str(&line?)
            .with_context(|| format!("{} line {}", path.display(), index + 1))?
        {
            Value::Object(entry) => entries.push(entry),
            _ => bail!("{} line {} is not an object", path.display(), index + 1),
        }
    }
    Ok(entries)
}

/// Resolves the `data-ref` of a link inside an article: another entry of the
/// same lexicon becomes a `dref` to its key, and a verse of the Hebrew Bible an
/// `href` in the `Book C:V` form the app already follows for BDB. Anything
/// else — a Talmud folio, a Midrash, an entry the filter dropped — stays as
/// the text it is printed with.
struct Links<'a> {
    prefix: &'static str,
    keys: HashMap<&'a str, &'a str>,
}

impl<'a> Links<'a> {
    fn new(source: &Source, entries: &'a [Map<String, Value>]) -> Self {
        let mut keys = HashMap::new();
        for entry in entries {
            let key = str_field(entry, "rid");
            // The headword wins over another entry's alternative spelling.
            keys.insert(str_field(entry, "headword"), key);
        }
        for entry in entries {
            let key = str_field(entry, "rid");
            for word in headwords(entry).skip(1) {
                keys.entry(word).or_insert(key);
            }
        }
        Links {
            prefix: source.ref_prefix,
            keys,
        }
    }

    fn resolve(&self, data_ref: &str) -> Option<Link> {
        if let Some(target) = data_ref.strip_prefix(self.prefix) {
            // `אָבוֹת 1`: the trailing number is Sefaria's segment, not part of
            // the headword.
            let headword = match target.rsplit_once(' ') {
                Some((word, segment)) if segment.chars().all(|c| c.is_ascii_digit()) => word,
                _ => target,
            };
            return self
                .keys
                .get(headword)
                .map(|key| Link::Entry(key.to_string()));
        }
        bible_href(data_ref).map(Link::Verse)
    }
}

enum Link {
    Entry(String),
    Verse(String),
}

/// The Hebrew Bible's books under the names both Sefaria and the app's
/// reference parser use.
const OT_BOOKS: [&str; 39] = [
    "Genesis",
    "Exodus",
    "Leviticus",
    "Numbers",
    "Deuteronomy",
    "Joshua",
    "Judges",
    "Ruth",
    "I Samuel",
    "II Samuel",
    "I Kings",
    "II Kings",
    "Isaiah",
    "Jeremiah",
    "Ezekiel",
    "Hosea",
    "Joel",
    "Amos",
    "Obadiah",
    "Jonah",
    "Micah",
    "Nahum",
    "Habakkuk",
    "Zephaniah",
    "Haggai",
    "Zechariah",
    "Malachi",
    "Psalms",
    "Proverbs",
    "Job",
    "Song of Songs",
    "Lamentations",
    "Ecclesiastes",
    "Esther",
    "Daniel",
    "Ezra",
    "Nehemiah",
    "I Chronicles",
    "II Chronicles",
];

/// `Genesis 1:1` or the start of a range (`Genesis 1:1-3`) as a tappable
/// verse; a bare chapter (`Exodus 19`) names no verse to open, so it is text.
fn bible_href(data_ref: &str) -> Option<String> {
    let (book, place) = data_ref.rsplit_once(' ')?;
    if !OT_BOOKS.contains(&book) {
        return None;
    }
    let start = place.split('-').next()?;
    let (chapter, verse) = start.split_once(':')?;
    chapter.parse::<u32>().ok()?;
    verse.parse::<u32>().ok()?;
    Some(format!("{book} {chapter}:{verse}"))
}

/// One entry as the runtime stores it.
struct Article {
    lang: String,
    pos: String,
    gloss: String,
    content: Value,
}

impl Article {
    /// The span JSON: `senses` exactly as BDB's (`num`, `form`, `definition`,
    /// nested `senses`, plus the sense's own period marker `lang`), and beside
    /// them what BDB has no slot for — `etymology`, `derivatives`, `plural` and
    /// `alternatives`.
    fn from_entry(entry: &Map<String, Value>, links: &Links) -> Self {
        let content_node = entry.get("content");
        let senses: Vec<Value> = content_node
            .and_then(|c| c.get("senses"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|s| sense(s, links))
            .collect();

        // Jastrow keeps his etymological remark (`√בר`, `cmp. כָּרָה`) as
        // `language_reference`, and where the digitisation misfiled one it sits
        // in `language_code`; Klein's bracketed etymology is `notes`.
        let raw_lang = str_field(entry, "language_code");
        let mut etymology = Vec::new();
        for html in [
            str_field(entry, "notes"),
            str_field(entry, "language_reference"),
        ] {
            append(&mut etymology, spans(html, links));
        }
        let lang = if raw_lang.contains('<') {
            append(&mut etymology, spans(raw_lang, links));
            String::new()
        } else {
            clean_lang(raw_lang)
        };

        let derivatives = spans(
            str_field(entry, "derivatives")
                .trim_start()
                .trim_start_matches("Derivatives:"),
            links,
        );
        let plural = match entry.get("plural_form") {
            Some(Value::String(html)) => spans(html, links),
            Some(Value::Array(forms)) => {
                let joined: Vec<&str> = forms.iter().filter_map(Value::as_str).collect();
                spans(&joined.join(", "), links)
            }
            _ => Vec::new(),
        };
        let alternatives: Vec<&str> = headwords(entry).skip(1).collect();

        let mut content = Map::new();
        content.insert("senses".into(), Value::Array(senses.clone()));
        for (key, value) in [
            ("etymology", etymology),
            ("derivatives", derivatives),
            ("plural", plural),
        ] {
            if !value.is_empty() {
                content.insert(key.into(), Value::Array(value));
            }
        }
        if !alternatives.is_empty() {
            content.insert("alternatives".into(), json!(alternatives));
        }

        let pos = content_node
            .and_then(|c| c.get("morphology"))
            .or_else(|| entry.get("morphology"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        Article {
            lang,
            pos,
            gloss: gloss(&senses),
            content: Value::Object(content),
        }
    }
}

fn sense(node: &Value, links: &Links) -> Value {
    let mut out = Map::new();
    if let Some(num) = node.get("number").and_then(Value::as_str) {
        out.insert("num".into(), json!(num));
    }
    let grammar = node.get("grammar");
    if let Some(stem) = grammar
        .and_then(|g| g.get("verbal_stem"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        out.insert("form".into(), json!(stem));
    }
    let lang = node
        .get("language_code")
        .or_else(|| grammar.and_then(|g| g.get("language_code")))
        .and_then(Value::as_str)
        .map(clean_lang)
        .unwrap_or_default();
    if !lang.is_empty() {
        out.insert("lang".into(), json!(lang));
    }

    let mut definition = Vec::new();
    for key in ["alternative", "definition", "plural_form", "notes"] {
        if let Some(html) = node.get(key).and_then(Value::as_str) {
            if !definition.is_empty() {
                definition.push(json!({"t": " "}));
            }
            append(&mut definition, spans(html, links));
        }
    }
    if !definition.is_empty() {
        out.insert("definition".into(), Value::Array(definition));
    }
    let children: Vec<Value> = node
        .get("senses")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|s| sense(s, links))
        .collect();
    if !children.is_empty() {
        out.insert("senses".into(), Value::Array(children));
    }
    Value::Object(out)
}

/// The first definition's plain text, as a one-line gloss: `father.` →
/// `father`. Citations are locators, not meaning, so they are left out.
fn gloss(senses: &[Value]) -> String {
    fn first(senses: &[Value]) -> Option<String> {
        for sense in senses {
            if let Some(spans) = sense.get("definition").and_then(Value::as_array) {
                let text: String = spans
                    .iter()
                    .filter(|s| s.get("href").is_none())
                    .filter_map(|s| s.get("t").and_then(Value::as_str))
                    .collect();
                let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                if text.chars().any(char::is_alphabetic) {
                    return Some(text);
                }
            }
            if let Some(found) = sense
                .get("senses")
                .and_then(Value::as_array)
                .and_then(|children| first(children))
            {
                return Some(found);
            }
        }
        None
    }
    let text = first(senses).unwrap_or_default();
    // Jastrow often opens a sense with the punctuation that followed the
    // headword's grammar (`; pl. שַׁלְמִין`), which a gloss does not need.
    let text = text
        .trim_matches(|c: char| c.is_whitespace() || ".,;:—".contains(c))
        .trim();
    const MAX: usize = 80;
    if text.chars().count() <= MAX {
        return text.to_string();
    }
    let cut = text.char_indices().nth(MAX).map_or(text.len(), |(i, _)| i);
    let cut = text[..cut].rfind(' ').unwrap_or(cut);
    format!("{}…", text[..cut].trim_end())
}

/// Append spans, merging the join when both sides carry the same styling.
fn append(out: &mut Vec<Value>, spans: Vec<Value>) {
    for span in spans {
        push_span(out, span);
    }
}

fn push_span(out: &mut Vec<Value>, span: Value) {
    if let (Some(Value::Object(last)), Value::Object(next)) = (out.last_mut(), &span) {
        let same_style = |a: &Map<String, Value>, b: &Map<String, Value>| {
            a.iter()
                .filter(|(k, _)| *k != "t")
                .eq(b.iter().filter(|(k, _)| *k != "t"))
        };
        let linked = last.contains_key("href") || last.contains_key("dref");
        if !linked && same_style(last, next) {
            let joined = format!(
                "{}{}",
                last["t"].as_str().unwrap_or(""),
                next["t"].as_str().unwrap_or("")
            );
            last.insert("t".into(), json!(joined));
            return;
        }
    }
    out.push(span);
}

/// Convert one of Sefaria's HTML fragments into spans. The markup is a small,
/// regular subset — `<i>`, `<b>`, `<sup>`, `<sub>`, `<span dir="rtl">` and
/// `<a class="refLink" data-ref="…">`, occasionally nested — so a tag scanner
/// is enough; there is no need for an HTML parser.
fn spans(html: &str, links: &Links) -> Vec<Value> {
    #[derive(Clone, Default)]
    struct Frame {
        tag: String,
        i: bool,
        b: bool,
        rtl: bool,
        link: Option<(&'static str, String)>,
    }

    let mut out = Vec::new();
    let mut stack = vec![Frame::default()];
    let mut rest = html;
    while !rest.is_empty() {
        let (text, after) = match rest.find('<') {
            Some(at) => rest.split_at(at),
            None => (rest, ""),
        };
        if !text.is_empty() {
            let top = stack.last().unwrap();
            let mut span = Map::new();
            span.insert(
                "t".into(),
                json!(decode_entities(&collapse_whitespace(text))),
            );
            if top.b {
                span.insert("b".into(), json!(true));
            }
            if top.i {
                span.insert("i".into(), json!(true));
            }
            if top.rtl {
                span.insert("rtl".into(), json!(true));
            }
            if let Some((key, target)) = &top.link {
                span.insert((*key).into(), json!(target));
            }
            push_span(&mut out, Value::Object(span));
        }
        if after.is_empty() {
            break;
        }
        let Some(end) = after.find('>') else {
            // A stray `<` with no tag after it is text.
            push_span(&mut out, json!({"t": after}));
            break;
        };
        let tag = &after[1..end];
        rest = &after[end + 1..];

        if let Some(name) = tag.strip_prefix('/') {
            let name = name.trim();
            if let Some(at) = stack.iter().rposition(|f| f.tag == name)
                && at > 0
            {
                stack.truncate(at);
            }
            continue;
        }
        let name = tag.split_whitespace().next().unwrap_or("").to_string();
        if name == "br" || tag.ends_with('/') {
            continue;
        }
        let mut frame = stack.last().unwrap().clone();
        frame.tag = name.clone();
        match name.as_str() {
            "i" => frame.i = true,
            "b" => frame.b = true,
            "span" | "a" => {
                if attribute(tag, "dir") == Some("rtl") {
                    frame.rtl = true;
                }
                if name == "a"
                    && let Some(data_ref) = attribute(tag, "data-ref")
                {
                    frame.link = match links.resolve(&decode_entities(data_ref)) {
                        Some(Link::Entry(key)) => Some(("dref", key)),
                        Some(Link::Verse(href)) => Some(("href", href)),
                        None => None,
                    };
                }
            }
            _ => {}
        }
        stack.push(frame);
    }
    out.retain(|s| s["t"].as_str().is_some_and(|t| !t.is_empty()));
    out
}

/// The value of `name="…"` inside a tag.
fn attribute<'t>(tag: &'t str, name: &str) -> Option<&'t str> {
    let needle = format!("{name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let len = tag[start..].find('"')?;
    Some(&tag[start..start + len])
}

/// Whitespace runs become one space, keeping a boundary space so adjacent
/// styled runs stay separated.
fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(c);
            in_space = false;
        }
    }
    out
}

fn decode_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(values: Value) -> Vec<Map<String, Value>> {
        values
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_object().unwrap().clone())
            .collect()
    }

    fn klein_links(list: &[Map<String, Value>]) -> Links<'_> {
        Links::new(&SOURCES[0], list)
    }

    #[test]
    fn markup_becomes_styled_spans() {
        let list = entries(json!([
            {"rid": "A00006", "headword": "אָב ᴵ"},
            {"rid": "A00010", "headword": "אָבוֹת"},
        ]));
        let links = klein_links(&list);
        let html = "[cp. Aram. <a dir=\"rtl\" class=\"refLink\" href=\"/Klein Dictionary,_אָבוֹת.1\" \
                    data-ref=\"Klein Dictionary, אָבוֹת 1\">אָבוֹת</a>, Ugar. <i>’b</i>, \
                    <span dir=\"rtl\">שָׁלוֹם</span>.]";
        assert_eq!(
            Value::Array(spans(html, &links)),
            json!([
                {"t": "[cp. Aram. "},
                {"t": "אָבוֹת", "rtl": true, "dref": "A00010"},
                {"t": ", Ugar. "},
                {"t": "’b", "i": true},
                {"t": ", "},
                {"t": "שָׁלוֹם", "rtl": true},
                {"t": ".]"},
            ])
        );
    }

    #[test]
    fn links_open_verses_and_entries_but_not_other_texts() {
        let list = entries(json!([{"rid": "A00006", "headword": "אָב ᴵ"}]));
        let links = klein_links(&list);
        assert!(matches!(
            links.resolve("Klein Dictionary, אָב ᴵ 1"),
            Some(Link::Entry(key)) if key == "A00006"
        ));
        // Filtered out, or never an entry of this lexicon.
        assert!(links.resolve("Klein Dictionary, טֶלֶפוֹן 1").is_none());
        assert!(links.resolve("Jastrow, אָב 1").is_none());
        assert!(matches!(
            links.resolve("I Samuel 3:4-6"),
            Some(Link::Verse(href)) if href == "I Samuel 3:4"
        ));
        assert!(links.resolve("Exodus 19").is_none());
        assert!(links.resolve("Shabbat 104a").is_none());
    }

    #[test]
    fn nested_links_close_cleanly() {
        let list = entries(json!([{"rid": "K00001", "headword": "קוּם"}]));
        let links = Links::new(&SOURCES[1], &list);
        let html = " <a dir=\"rtl\" class=\"refLink\" href=\"/Jastrow,_קוּם.1\" data-ref=\"Jastrow, קוּם 1\">\
                    <a dir=\"rtl\" class=\"refLink\" href=\"/Jastrow,_קוּם.1\" data-ref=\"Jastrow, קוּם 1\">קוּם</a>)</a> end";
        assert_eq!(
            Value::Array(spans(html, &links)),
            json!([
                {"t": " "},
                {"t": "קוּם", "rtl": true, "dref": "K00001"},
                {"t": ")", "rtl": true, "dref": "K00001"},
                {"t": " end"},
            ])
        );
    }

    #[test]
    fn markers_lose_the_punctuation_they_were_cut_with() {
        assert_eq!(clean_lang("(b. h.;"), "b. h.");
        assert_eq!(clean_lang(" ch. "), "ch.");
        assert_eq!(clean_lang("PBH,"), "PBH");
        assert_eq!(clean_lang("PBH."), "PBH");
        assert_eq!(clean_lang("(NH"), "NH");
        assert_eq!(clean_lang(""), "");
    }

    #[test]
    fn keeps_the_biblical_layer_and_what_either_corpus_spells() {
        let reachable: HashSet<String> = ["שלמ".to_string()].into();
        let klein = &SOURCES[0];
        let jastrow = &SOURCES[1];
        let entry = |v: Value| v.as_object().unwrap().clone();
        // Unmarked Klein entries are biblical.
        assert!(keep(klein, &entry(json!({"headword": "אָב ᴵ"})), &reachable));
        assert!(keep(
            klein,
            &entry(json!({"headword": "x", "language_code": "BH,"})),
            &reachable
        ));
        // A modern coinage nothing spells goes; one either corpus spells stays.
        assert!(!keep(
            klein,
            &entry(json!({"headword": "טֶלֶפוֹן", "language_code": "FW"})),
            &reachable
        ));
        assert!(!keep(
            klein,
            &entry(json!({"headword": "x", "language_code": "PBH"})),
            &reachable
        ));
        assert!(keep(
            klein,
            &entry(json!({"headword": "שָׁלֵם", "language_code": "NH"})),
            &reachable
        ));
        // Jastrow: biblical and Aramaic markers, or a reachable alternative spelling.
        assert!(keep(
            jastrow,
            &entry(json!({"headword": "x", "language_code": "(b. h.;"})),
            &reachable
        ));
        assert!(keep(
            jastrow,
            &entry(json!({"headword": "x", "language_code": " ch. "})),
            &reachable
        ));
        assert!(!keep(jastrow, &entry(json!({"headword": "x"})), &reachable));
        assert!(keep(
            jastrow,
            &entry(json!({"headword": "x", "alt_headwords": ["שְׁלָם"]})),
            &reachable
        ));
    }

    #[test]
    fn article_keeps_senses_etymology_and_a_short_gloss() {
        let list = entries(json!([{
            "rid": "A00006",
            "headword": "אָב ᴵ",
            "content": {
                "morphology": "m.n.",
                "senses": [
                    {"number": "1", "definition": "father."},
                    {"number": "8", "definition": "parent (male).", "language_code": "PBH"},
                    {"grammar": {"verbal_stem": "Qal"},
                     "senses": [{"number": "1", "definition": "to be whole."}]},
                ]
            },
            "notes": "[cp. Ugar. <i>’b</i>.]",
            "derivatives": "Derivatives: <span dir=\"rtl\">אַבְהוּת</span>.",
        }]));
        let links = klein_links(&list);
        let article = Article::from_entry(&list[0], &links);
        assert_eq!(article.gloss, "father");
        assert_eq!(article.pos, "m.n.");
        assert_eq!(
            article.content,
            json!({
                "senses": [
                    {"num": "1", "definition": [{"t": "father."}]},
                    {"num": "8", "lang": "PBH", "definition": [{"t": "parent (male)."}]},
                    {"form": "Qal", "senses": [{"num": "1", "definition": [{"t": "to be whole."}]}]},
                ],
                "etymology": [{"t": "[cp. Ugar. "}, {"t": "’b", "i": true}, {"t": ".]"}],
                "derivatives": [{"t": " "}, {"t": "אַבְהוּת", "rtl": true}, {"t": "."}],
            })
        );
    }

    #[test]
    fn gloss_skips_citations_and_reaches_into_sub_senses() {
        let senses = json!([
            {"definition": [{"t": "Gen 1:1", "href": "Genesis 1:1"}]},
            {"senses": [{"definition": [{"t": "to be "}, {"t": "whole", "i": true}, {"t": "."}]}]},
        ]);
        assert_eq!(gloss(senses.as_array().unwrap()), "to be whole");
        let senses = json!([{"definition": [{"t": "; pl. "}, {"t": "שַׁלְמִין", "rtl": true}]}]);
        assert_eq!(gloss(senses.as_array().unwrap()), "pl. שַׁלְמִין");
    }
}
