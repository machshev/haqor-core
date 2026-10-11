//! The Greek lexicons, Abbott-Smith and Thayer, joined to the TR's dictionary
//! forms, and the root each dictionary form is built on.
//!
//! - G. Abbott-Smith, *A Manual Greek Lexicon of the New Testament* (1922),
//!   as the Translatable Exegetical Tools project marks it up in TEI: public
//!   domain.
//! - J. H. Thayer, *A Greek-English Lexicon of the New Testament* (1889), as
//!   a machine-read dataset of it gives it: CC0.
//!
//! Both are prepared with STEP Bible's files ([`crate::stepbible::prepare`]):
//! an entry finds the TR's dictionary forms it describes by the Strong's
//! numbers the three share, failing that by its spelling, and is written out
//! keyed by those forms alone. A form's root is where its derivation ends:
//! Abbott-Smith's `(< λέγω)`, Thayer's `(fr. ἀγαθοποιός)`, a compound's last
//! element (`ἐκ-βάλλω`), followed back until a word derived from nothing
//! given. A compound of two words has both their roots, its last element's
//! first; a preposition, a privative or an adverb prefixed to a word is no root.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use log::info;
use quick_xml::events::{BytesStart, Event};
use rusqlite::{Connection, params};
use serde_json::{Map, Value, json};

use crate::tagnt::{canonical, robinson};

/// Where the downloaded sources stand in the directory `db prepare
/// stepbible` reads.
pub(crate) const ABBOTT_SMITH_FILE: &str = "Abbott-Smith/abbott-smith.tei_lemma.xml";
pub(crate) const THAYER_FILE: &str = "thayers-greek-lexicon-dataset/thayer_lexicon.jsonl";

/// The prepared files.
///
/// - `greek_lexicon.jsonl`: an entry a line: its `source` (`abbott-smith` or
///   `thayer`), `headword`, `gloss`, the TR's dictionary `forms` it
///   describes, and its `body`, the span JSON the app's lexicon entries
///   render.
/// - `greek_roots.tsv`: every dictionary form of the TR, a line for each root
///   it is built on: the form, the root and its order, 0 first.
pub(crate) const PREPARED_LEXICON: &str = "greek_lexicon.jsonl";
pub(crate) const PREPARED_ROOTS: &str = "greek_roots.tsv";

pub(crate) const ABBOTT_SMITH: &str = "abbott-smith";
pub(crate) const THAYER: &str = "thayer";

/// What [`prepare`] wrote, for the log.
#[derive(Debug, Default)]
pub struct LexiconSummary {
    /// Entries read, and kept for describing a dictionary form of the TR.
    pub abbott_smith: (usize, usize),
    pub thayer: (usize, usize),
    /// Dictionary forms with an entry in either, of all the TR's.
    pub described: usize,
    pub forms: usize,
    /// Distinct roots, and forms built on another word.
    pub roots: usize,
    pub derived: usize,
}

/// A lexicon entry as read.
#[derive(Debug, Default)]
struct Entry {
    headword: String,
    gloss: String,
    /// Strong's numbers, unprefixed: join keys, never written out.
    strongs: Vec<u32>,
    body: Value,
    /// The words it is derived from, as written, its last element last.
    parents: Vec<String>,
    /// A compound's last element, as its headword is hyphenated
    /// (`ἐκ-βάλλω`); a root only if it is a word.
    tail: Option<String>,
}

/// One styled run of an entry's text: the span objects the app renders.
#[derive(Clone, Debug, Default, PartialEq)]
struct Style {
    bold: bool,
    italic: bool,
    greek: bool,
    rtl: bool,
    href: Option<String>,
    /// Another entry of the same lexicon, by its headword.
    dref: Option<String>,
}

#[derive(Debug, Default)]
struct Spans(Vec<(String, Style)>);

impl Spans {
    fn push(&mut self, text: &str, style: &Style) {
        let cleaned = crate::tsk::yahweh_for_jehovah(&without_numbers(text));
        let text = cleaned.as_str();
        if text.is_empty() {
            return;
        }
        match self.0.last_mut() {
            Some((last, s)) if s == style && style.href.is_none() && style.dref.is_none() => {
                last.push_str(text)
            }
            _ => self.0.push((text.to_string(), style.clone())),
        }
    }

    fn is_blank(&self) -> bool {
        self.0.iter().all(|(t, _)| t.trim().is_empty())
    }

    /// The runs as span JSON, whitespace collapsed and the ends trimmed.
    fn to_json(&self) -> Value {
        let mut runs: Vec<(String, &Style)> = Vec::new();
        for (text, style) in &self.0 {
            let mut collapsed = String::with_capacity(text.len());
            let mut space = false;
            for c in text.chars() {
                if c.is_whitespace() {
                    space = true;
                } else {
                    if space {
                        collapsed.push(' ');
                    }
                    space = false;
                    collapsed.push(c);
                }
            }
            if space {
                collapsed.push(' ');
            }
            // A run of spaces between two runs is one space.
            if collapsed.starts_with(' ')
                && runs
                    .last()
                    .is_some_and(|(t, _)| t.ends_with(' ') || t.is_empty())
            {
                collapsed.remove(0);
            }
            if runs.is_empty() {
                collapsed = collapsed.trim_start().to_string();
            }
            if !collapsed.is_empty() {
                runs.push((collapsed, style));
            }
        }
        if let Some((last, _)) = runs.last_mut() {
            *last = last.trim_end().to_string();
        }
        Value::Array(
            runs.into_iter()
                .filter(|(t, _)| !t.is_empty())
                .map(|(t, s)| {
                    let mut m = Map::new();
                    m.insert("t".into(), json!(t));
                    if s.bold {
                        m.insert("b".into(), json!(true));
                    }
                    if s.italic {
                        m.insert("i".into(), json!(true));
                    }
                    if s.greek {
                        m.insert("g".into(), json!(true));
                    }
                    if s.rtl {
                        m.insert("rtl".into(), json!(true));
                    }
                    if let Some(h) = &s.href {
                        m.insert("href".into(), json!(h));
                    }
                    if let Some(d) = &s.dref {
                        m.insert("dref".into(), json!(d));
                    }
                    Value::Object(m)
                })
                .collect(),
        )
    }
}

/// Text with any Strong's number it carries taken out: Abbott-Smith now
/// and then writes one as text (`(H7592)`), and none is kept.
fn without_numbers(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let open = chars[i] == '(';
        let start = i + usize::from(open);
        let letter = chars.get(start).is_some_and(|c| matches!(c, 'H' | 'G'));
        let before = start == 0 || !chars[start - 1].is_alphanumeric();
        let digits = chars[(start + 1).min(chars.len())..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count();
        let end = start + 1 + digits;
        let after = chars.get(end).is_none_or(|c| !c.is_alphanumeric());
        if letter && before && after && digits >= 1 {
            let close = open && chars.get(end) == Some(&')');
            if !open || close {
                i = end + usize::from(close);
                // The space the number stood in, once.
                if out.ends_with(' ') && chars.get(i) == Some(&' ') {
                    i += 1;
                }
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// The English book name the app's verse links use (`I Samuel 3:4`), for an
/// OSIS book code; none for a book outside the canon.
fn book_name(osis: &str) -> Option<&'static str> {
    Some(match osis {
        "Gen" => "Genesis",
        "Exod" => "Exodus",
        "Lev" => "Leviticus",
        "Num" => "Numbers",
        "Deut" => "Deuteronomy",
        "Josh" => "Joshua",
        "Judg" => "Judges",
        "Ruth" => "Ruth",
        "1Sam" => "I Samuel",
        "2Sam" => "II Samuel",
        "1Kgs" => "I Kings",
        "2Kgs" => "II Kings",
        "1Chr" => "I Chronicles",
        "2Chr" => "II Chronicles",
        "Ezra" => "Ezra",
        "Neh" => "Nehemiah",
        "Esth" => "Esther",
        "Job" => "Job",
        "Ps" => "Psalms",
        "Prov" => "Proverbs",
        "Eccl" => "Ecclesiastes",
        "Song" => "Song of Songs",
        "Isa" => "Isaiah",
        "Jer" => "Jeremiah",
        "Lam" => "Lamentations",
        "Ezek" => "Ezekiel",
        "Dan" => "Daniel",
        "Hos" => "Hosea",
        "Joel" => "Joel",
        "Amos" => "Amos",
        "Obad" => "Obadiah",
        "Jonah" => "Jonah",
        "Mic" => "Micah",
        "Nah" => "Nahum",
        "Hab" => "Habakkuk",
        "Zeph" => "Zephaniah",
        "Hag" => "Haggai",
        "Zech" => "Zechariah",
        "Mal" => "Malachi",
        "Matt" => "Matthew",
        "Mark" => "Mark",
        "Luke" => "Luke",
        "John" => "John",
        "Acts" => "Acts",
        "Rom" => "Romans",
        "1Cor" => "I Corinthians",
        "2Cor" => "II Corinthians",
        "Gal" => "Galatians",
        "Eph" => "Ephesians",
        "Phil" => "Philippians",
        "Col" => "Colossians",
        "1Thess" => "I Thessalonians",
        "2Thess" => "II Thessalonians",
        "1Tim" => "I Timothy",
        "2Tim" => "II Timothy",
        "Titus" => "Titus",
        "Phlm" => "Philemon",
        "Heb" => "Hebrews",
        "Jas" => "James",
        "1Pet" => "I Peter",
        "2Pet" => "II Peter",
        "1John" => "I John",
        "2John" => "II John",
        "3John" => "III John",
        "Jude" => "Jude",
        "Rev" => "Revelation",
        _ => return None,
    })
}

/// `Matt.1.1` as the app's link, `Matthew 1:1`.
fn osis_href(osis: &str) -> Option<String> {
    let mut parts = osis.split('.');
    let book = book_name(parts.next()?)?;
    let chapter: u32 = parts.next()?.parse().ok()?;
    let verse: u32 = parts.next()?.split('-').next()?.parse().ok()?;
    Some(format!("{book} {chapter}:{verse}"))
}

fn attribute(e: &BytesStart<'_>, key: &str) -> Option<String> {
    e.try_get_attribute(key)
        .ok()
        .flatten()
        .and_then(|a| a.normalized_value(crate::xml::VERSION).ok())
        .map(|v| v.into_owned())
}

/// `G976`, `G0976`, `G2424G` as 976 and 2424.
fn strong_number(text: &str) -> Option<u32> {
    let digits: String = text
        .trim()
        .strip_prefix('G')?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// Whether a character is Greek, letter or diacritic.
fn is_greek(c: char) -> bool {
    matches!(c, '\u{0370}'..='\u{03FF}' | '\u{1F00}'..='\u{1FFF}')
        || ('\u{0300}'..='\u{036F}').contains(&c)
}

fn is_hebrew(c: char) -> bool {
    ('\u{0590}'..='\u{05FF}').contains(&c)
}

/// A sense being read, with the senses inside it.
#[derive(Default)]
struct Sense {
    num: Option<String>,
    definition: Spans,
    senses: Vec<Sense>,
}

impl Sense {
    fn to_json(&self) -> Value {
        let mut m = Map::new();
        if let Some(n) = &self.num {
            m.insert("num".into(), json!(n));
        }
        if !self.definition.is_blank() {
            m.insert("definition".into(), self.definition.to_json());
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

/// Which part of an Abbott-Smith entry text goes to.
#[derive(Clone, Copy, PartialEq)]
enum Part {
    None,
    Orth,
    Form,
    Derivation,
    Septuagint,
    Sense,
    Related,
}

/// Abbott-Smith's entries, from the TEI.
fn read_abbott_smith(path: &Path) -> Result<Vec<Entry>> {
    let mut reader = crate::xml::Reader::open(path)?;
    parse_abbott_smith(|| reader.next())
}

fn parse_abbott_smith(mut next: impl FnMut() -> Result<Event<'static>>) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    // The entry being read, with what is gathered for it.
    let mut entry: Option<Entry> = None;
    let mut part = Part::None;
    let mut forms = Spans::default();
    let mut etymology = Spans::default();
    let mut septuagint = Spans::default();
    let mut related = Spans::default();
    let mut stack: Vec<Sense> = Vec::new();
    let mut senses: Vec<Sense> = Vec::new();
    let mut glosses: Vec<String> = Vec::new();
    // Styles open, innermost last.
    let mut styles: Vec<Style> = vec![Style::default()];
    // Elements open within the entry, to know what each end closes.
    let mut open: Vec<(Vec<u8>, Part)> = Vec::new();
    // Whether text is in a derivation (`(< λέγω, …)`), and the Greek words
    // read there.
    let mut deriving = false;
    let mut gloss_text: Option<String> = None;
    let mut in_note = false;
    let mut orth_tail: Option<String> = None;

    loop {
        let event = next()?;
        match event {
            Event::Eof => break,
            Event::Start(e) if e.name().as_ref() == "entry" => {
                let lemma = attribute(&e, "lemma").unwrap_or_default();
                let mut strongs: Vec<u32> = attribute(&e, "strong")
                    .unwrap_or_default()
                    .split('|')
                    .filter_map(strong_number)
                    .collect();
                let mut fields = lemma.split('|');
                let headword = canonical(fields.next().unwrap_or_default().trim());
                strongs.extend(fields.filter_map(strong_number));
                entry = Some(Entry {
                    headword,
                    strongs,
                    ..Entry::default()
                });
                part = Part::None;
                open.clear();
                styles.truncate(1);
            }
            Event::End(e) if e.name().as_ref() == "entry" => {
                let Some(mut done) = entry.take() else {
                    continue;
                };
                while let Some(sense) = stack.pop() {
                    match stack.last_mut() {
                        Some(parent) => parent.senses.push(sense),
                        None => senses.push(sense),
                    }
                }
                let mut body = Map::new();
                body.insert(
                    "senses".into(),
                    Value::Array(senses.iter().map(Sense::to_json).collect()),
                );
                for (key, spans) in [
                    ("forms", &forms),
                    ("etymology", &etymology),
                    ("lxx", &septuagint),
                    ("related", &related),
                ] {
                    if !spans.is_blank() {
                        body.insert(key.into(), spans.to_json());
                    }
                }
                done.body = Value::Object(body);
                done.tail = orth_tail.take().filter(|t| !t.is_empty());
                done.gloss = glosses
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                entries.push(done);
                forms = Spans::default();
                etymology = Spans::default();
                septuagint = Spans::default();
                related = Spans::default();
                senses.clear();
                glosses.clear();
                deriving = false;
            }
            _ if entry.is_none() => {}
            Event::Start(e) => {
                let name = e.name().as_ref().as_bytes().to_vec();
                let previous = part;
                let mut style = styles.last().cloned().unwrap_or_default();
                let mut styled = true;
                match name.as_slice() {
                    b"orth" => part = Part::Orth,
                    b"form" => part = Part::Form,
                    b"seg" => {
                        part = match attribute(&e, "type").as_deref() {
                            Some("derivation") => Part::Derivation,
                            Some("septuagint") => Part::Septuagint,
                            _ => part,
                        }
                    }
                    b"sense" => {
                        part = Part::Sense;
                        stack.push(Sense {
                            num: attribute(&e, "n"),
                            ..Sense::default()
                        });
                    }
                    b"re" => part = Part::Related,
                    b"note" => in_note = true,
                    b"gloss" => {
                        style.bold = true;
                        if part == Part::Sense && stack.len() == 1 {
                            gloss_text = Some(String::new());
                        }
                    }
                    b"emph" | b"hi" => style.italic = true,
                    b"foreign" => match attribute(&e, "xml:lang").as_deref() {
                        Some("grc") => style.greek = true,
                        Some("heb") | Some("arc") => style.rtl = true,
                        _ => style.italic = true,
                    },
                    b"ref" => {
                        if let Some(osis) = attribute(&e, "osisRef") {
                            style.href = osis_href(&osis);
                        } else if let Some(target) = attribute(&e, "target") {
                            style.dref = Some(canonical(target.trim_start_matches('#')));
                        } else {
                            // `<ref><foreign>ἀγαθός</foreign></ref>`: the
                            // entry is named by the text inside.
                            style.dref = Some(String::new());
                        }
                    }
                    _ => styled = false,
                }
                if !styled {
                    style = styles.last().cloned().unwrap_or_default();
                }
                styles.push(style);
                open.push((name, previous));
            }
            Event::End(_) => {
                let Some((name, previous)) = open.pop() else {
                    continue;
                };
                styles.pop();
                match name.as_slice() {
                    b"sense" => {
                        if let Some(sense) = stack.pop() {
                            match stack.last_mut() {
                                Some(parent) => parent.senses.push(sense),
                                None => senses.push(sense),
                            }
                        }
                    }
                    b"note" => in_note = false,
                    b"gloss" => {
                        if let Some(g) = gloss_text.take() {
                            let g = g.split_whitespace().collect::<Vec<_>>().join(" ");
                            if !g.is_empty() && !glosses.contains(&g) {
                                glosses.push(g);
                            }
                        }
                    }
                    b"form" | b"seg" => deriving = false,
                    _ => {}
                }
                part = previous;
                if !stack.is_empty() && part == Part::None {
                    part = Part::Sense;
                }
            }
            Event::Empty(e) => {
                if e.name().as_ref() == "lb" {
                    let style = styles.last().cloned().unwrap_or_default();
                    if let Some(spans) = target(
                        part,
                        &mut forms,
                        &mut etymology,
                        &mut septuagint,
                        &mut related,
                        &mut stack,
                    ) {
                        spans.push(" ", &style);
                    }
                }
            }
            Event::Text(t) => {
                if in_note {
                    continue;
                }
                let text = t.xml10_content().into_owned();
                let mut style = styles.last().cloned().unwrap_or_default();
                if style.dref.as_deref() == Some("") {
                    style.dref = Some(canonical(text.trim()));
                }
                let entry = entry.as_mut().expect("inside an entry");
                if part == Part::Orth {
                    // `ἐκ-βάλλω`: a compound's last element is what it is built on.
                    if entry.headword.is_empty() {
                        entry.headword = canonical(&text.trim().replace('-', ""));
                    }
                    if let Some((_, tail)) = text.trim().rsplit_once('-') {
                        orth_tail = Some(canonical(tail.trim()));
                    }
                    continue;
                }
                if let Some(g) = gloss_text.as_mut() {
                    g.push_str(&text);
                }
                if matches!(part, Part::Form | Part::Derivation) {
                    if style.greek {
                        let word = text.trim();
                        if deriving
                            && !word.is_empty()
                            && !word.starts_with('-')
                            && !word.ends_with('-')
                            && !word.contains(' ')
                        {
                            entry.parents.push(canonical(word));
                        }
                    } else {
                        if text.contains('<') || text.contains("dim. of") {
                            deriving = true;
                        }
                        if text.contains(')')
                            || text.contains(';')
                            || text.contains('=')
                            || text.contains("cf.")
                        {
                            deriving = text.contains('<') && !text.contains(')');
                        }
                    }
                }
                let mut text = text.as_str();
                if part == Part::Form {
                    // The frequency marks before a headword.
                    text = text.trim_start_matches(['*', '†', ' ', '\n']);
                    if forms.is_blank() {
                        text = text.trim_start_matches([',', ' ', '\n']);
                    }
                }
                if let Some(spans) = target(
                    part,
                    &mut forms,
                    &mut etymology,
                    &mut septuagint,
                    &mut related,
                    &mut stack,
                ) {
                    spans.push(text, &style);
                }
            }
            _ => {}
        }
    }
    Ok(entries)
}

/// Where text in `part` goes.
fn target<'a>(
    part: Part,
    forms: &'a mut Spans,
    etymology: &'a mut Spans,
    septuagint: &'a mut Spans,
    related: &'a mut Spans,
    stack: &'a mut [Sense],
) -> Option<&'a mut Spans> {
    match part {
        Part::Form => Some(forms),
        Part::Derivation => Some(etymology),
        Part::Septuagint => Some(septuagint),
        Part::Related => Some(related),
        Part::Sense => stack.last_mut().map(|s| &mut s.definition),
        Part::None | Part::Orth => None,
    }
}

/// Thayer's abbreviations for the books of the Bible, as the app's links
/// name them. The longer of two sharing a beginning comes first.
const THAYER_BOOKS: &[(&str, &str)] = &[
    ("1 Thess.", "I Thessalonians"),
    ("2 Thess.", "II Thessalonians"),
    ("1 Th.", "I Thessalonians"),
    ("2 Th.", "II Thessalonians"),
    ("1 Co.", "I Corinthians"),
    ("2 Co.", "II Corinthians"),
    ("1 Cor.", "I Corinthians"),
    ("2 Cor.", "II Corinthians"),
    ("1 Tim.", "I Timothy"),
    ("2 Tim.", "II Timothy"),
    ("1 Pet.", "I Peter"),
    ("2 Pet.", "II Peter"),
    ("1 Jn.", "I John"),
    ("2 Jn.", "II John"),
    ("3 Jn.", "III John"),
    ("1 S.", "I Samuel"),
    ("2 S.", "II Samuel"),
    ("1 K.", "I Kings"),
    ("2 K.", "II Kings"),
    ("1 Chr.", "I Chronicles"),
    ("2 Chr.", "II Chronicles"),
    ("Mt.", "Matthew"),
    ("Mk.", "Mark"),
    ("Lk.", "Luke"),
    ("Jn.", "John"),
    ("Acts", "Acts"),
    ("Ro.", "Romans"),
    ("Rom.", "Romans"),
    ("Gal.", "Galatians"),
    ("Eph.", "Ephesians"),
    ("Philem.", "Philemon"),
    ("Phil.", "Philippians"),
    ("Col.", "Colossians"),
    ("Tit.", "Titus"),
    ("Heb.", "Hebrews"),
    ("Jas.", "James"),
    ("Jude", "Jude"),
    ("Rev.", "Revelation"),
    ("Gen.", "Genesis"),
    ("Ex.", "Exodus"),
    ("Lev.", "Leviticus"),
    ("Num.", "Numbers"),
    ("Deut.", "Deuteronomy"),
    ("Josh.", "Joshua"),
    ("Judg.", "Judges"),
    ("Ruth", "Ruth"),
    ("Neh.", "Nehemiah"),
    ("Esth.", "Esther"),
    ("Job", "Job"),
    ("Ps.", "Psalms"),
    ("Prov.", "Proverbs"),
    ("Eccl.", "Ecclesiastes"),
    ("Cant.", "Song of Songs"),
    ("Is.", "Isaiah"),
    ("Jer.", "Jeremiah"),
    ("Lam.", "Lamentations"),
    ("Ezek.", "Ezekiel"),
    ("Dan.", "Daniel"),
    ("Hos.", "Hosea"),
    ("Joel", "Joel"),
    ("Am.", "Amos"),
    ("Obad.", "Obadiah"),
    ("Jon.", "Jonah"),
    ("Mic.", "Micah"),
    ("Nah.", "Nahum"),
    ("Hab.", "Habakkuk"),
    ("Zeph.", "Zephaniah"),
    ("Hag.", "Haggai"),
    ("Zech.", "Zechariah"),
    ("Mal.", "Malachi"),
];

/// A lower-case Roman numeral's value.
fn roman(text: &str) -> Option<u32> {
    if text.is_empty() {
        return None;
    }
    let mut total = 0;
    let mut previous = 0;
    for c in text.chars().rev() {
        let value = match c {
            'i' => 1,
            'v' => 5,
            'x' => 10,
            'l' => 50,
            'c' => 100,
            _ => return None,
        };
        if value < previous {
            total -= value;
        } else {
            total += value;
            previous = value;
        }
    }
    Some(total)
}

/// Thayer's text as spans: Greek and Hebrew marked, and his references
/// (`Mt. i. 1`, `Lk. iv. 17, 20; Jn. xx. 30`, `Rev. xiii. 8; xvii. 8`) linked.
fn thayer_spans(text: &str) -> Spans {
    let mut spans = Spans::default();
    // Plain text runs are split by script as they are pushed.
    let push_plain = |spans: &mut Spans, run: &str| {
        let mut current = String::new();
        let mut kind = 0u8;
        for c in run.chars() {
            let k = if is_greek(c) {
                1
            } else if is_hebrew(c) {
                2
            } else if c.is_alphanumeric() {
                0
            } else {
                kind
            };
            if k != kind && !current.is_empty() {
                let style = Style {
                    greek: kind == 1,
                    rtl: kind == 2,
                    ..Style::default()
                };
                spans.push(&current, &style);
                current.clear();
            }
            kind = k;
            current.push(c);
        }
        let style = Style {
            greek: kind == 1,
            rtl: kind == 2,
            ..Style::default()
        };
        spans.push(&current, &style);
    };
    let bytes = text.as_bytes();
    let mut plain_start = 0;
    let mut at = 0;
    // The book and chapter a bare continuation refers to.
    let mut context: Option<(&str, u32)> = None;
    while at < text.len() {
        if !text.is_char_boundary(at) {
            at += 1;
            continue;
        }
        let rest = &text[at..];
        let starts_word = at == 0 || !bytes[at - 1].is_ascii_alphanumeric();
        // A book, a chapter in Roman numerals and a verse.
        let book = starts_word
            .then(|| {
                THAYER_BOOKS
                    .iter()
                    .find(|(abbreviation, _)| rest.starts_with(abbreviation))
            })
            .flatten();
        if let Some((abbreviation, name)) = book
            && let Some((chapter, verse, length)) = chapter_verse(&rest[abbreviation.len()..])
        {
            push_plain(&mut spans, &text[plain_start..at]);
            let end = at + abbreviation.len() + length;
            spans.push(
                &text[at..end],
                &Style {
                    href: Some(format!("{name} {chapter}:{verse}")),
                    ..Style::default()
                },
            );
            context = Some((name, chapter));
            at = end;
            plain_start = at;
            continue;
        }
        // A continuation: `, 20` or `; xvii. 8` after a reference.
        if let Some((name, chapter)) = context {
            let separator = rest.starts_with(", ") || rest.starts_with("; ");
            if separator {
                let after = &rest[2..];
                if let Some((next_chapter, verse, length)) = chapter_verse(&format!(" {after}"))
                    .filter(|_| after.chars().next().is_some_and(|c| c.is_ascii_lowercase()))
                {
                    push_plain(&mut spans, &text[plain_start..at + 2]);
                    let start = at + 2;
                    let end = start + length - 1;
                    spans.push(
                        &text[start..end],
                        &Style {
                            href: Some(format!("{name} {next_chapter}:{verse}")),
                            ..Style::default()
                        },
                    );
                    context = Some((name, next_chapter));
                    at = end;
                    plain_start = at;
                    continue;
                }
                let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
                let followed = after[digits.len()..]
                    .chars()
                    .next()
                    .is_none_or(|c| !c.is_alphanumeric() && c != '.');
                if rest.starts_with(", ") && !digits.is_empty() && followed {
                    push_plain(&mut spans, &text[plain_start..at + 2]);
                    let start = at + 2;
                    let end = start + digits.len();
                    spans.push(
                        &text[start..end],
                        &Style {
                            href: Some(format!("{name} {chapter}:{digits}")),
                            ..Style::default()
                        },
                    );
                    at = end;
                    plain_start = at;
                    continue;
                }
            }
            let c = rest.chars().next().unwrap_or(' ');
            if !(c.is_ascii_digit() || matches!(c, ' ' | ',' | ';' | '.' | '[' | ']' | '-' | '–'))
                && !c.is_ascii_uppercase()
            {
                context = None;
            }
        }
        at += rest.chars().next().map_or(1, char::len_utf8);
    }
    push_plain(&mut spans, &text[plain_start..]);
    spans
}

/// ` i. 1` as chapter 1, verse 1 and the length read.
fn chapter_verse(text: &str) -> Option<(u32, u32, usize)> {
    let rest = text.strip_prefix(' ')?;
    let numeral: String = rest
        .chars()
        .take_while(|c| c.is_ascii_lowercase())
        .collect();
    let chapter = roman(&numeral)?;
    let rest = rest[numeral.len()..].strip_prefix(". ")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let verse: u32 = digits.parse().ok()?;
    Some((chapter, verse, 1 + numeral.len() + 2 + digits.len()))
}

/// The word a Thayer derivation names: the first in the first parenthesis
/// of his grammar that begins with `fr.`, `dimin. of` or a Greek word
/// (`(fr. the adj. γενέσιος fr. γένεσις)` is γενέσιος). A second word is
/// most often another spelling of the first (`(βρόω, βιβρώσκω)`), and a
/// stem written in capitals (`ΕΡΓΩ`) is no word.
fn thayer_parents(grammar: &str) -> Vec<String> {
    let mut depth = 0;
    let mut inner = String::new();
    let mut groups = Vec::new();
    for c in grammar.chars() {
        match c {
            '(' | '[' => {
                if depth == 0 && c == '(' {
                    inner.clear();
                }
                depth += 1;
            }
            ')' | ']' => {
                depth -= 1;
                if depth == 0 && c == ')' {
                    groups.push(std::mem::take(&mut inner));
                }
            }
            _ if depth == 1 => inner.push(c),
            _ => {}
        }
    }
    for group in groups {
        let group = group.trim();
        let rest = group
            .strip_prefix("fr. ")
            .or_else(|| group.strip_prefix("dimin. of "))
            .or_else(|| group.strip_prefix("dim. of "))
            .or_else(|| group.chars().next().filter(|&c| is_greek(c)).map(|_| group));
        let Some(rest) = rest else {
            continue;
        };
        let rest = rest.split(" fr. ").next().unwrap_or(rest);
        let rest = rest.split([';', '=']).next().unwrap_or(rest);
        let rest = rest.split(" cf.").next().unwrap_or(rest);
        let word = rest
            .split(|c: char| !is_greek(c) && c != '-')
            .find(|w| w.chars().any(is_greek) && !w.starts_with('-') && !w.ends_with('-'))
            .map(|w| canonical(&w.replace('-', "")))
            .filter(|w| w.chars().count() > 1 && !is_capitals(w));
        return word.into_iter().collect();
    }
    Vec::new()
}

/// Whether a word is written in capitals, as a lexicon writes a stem it
/// reconstructs.
fn is_capitals(word: &str) -> bool {
    word.chars().filter(|c| c.is_alphabetic()).count() > 1
        && word
            .chars()
            .filter(|c| c.is_alphabetic())
            .all(char::is_uppercase)
}

/// Thayer's entries, an entry split across pages joined.
fn read_thayer(path: &Path) -> Result<Vec<Entry>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse_thayer(&text)
}

fn parse_thayer(text: &str) -> Result<Vec<Entry>> {
    // (id, lemma as written, grammar, paragraphs, strongs)
    let mut records: Vec<(String, String, String, Vec<String>, Vec<u32>)> = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let row: Value = serde_json::from_str(line).context("reading a Thayer entry")?;
        let id = row["id"].as_str().unwrap_or_default().to_string();
        let lemma = row["lemma"].as_str().unwrap_or_default().to_string();
        let grammar = row["grammar"].as_str().unwrap_or_default().to_string();
        let paragraphs: Vec<String> = row["paragraphs"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let strongs: Vec<u32> = match &row["strongs"] {
            Value::String(s) => s.split(['|', ',', ' ']).filter_map(strong_number).collect(),
            Value::Array(a) => a
                .iter()
                .filter_map(Value::as_str)
                .filter_map(strong_number)
                .collect(),
            _ => Vec::new(),
        };
        match records.last_mut() {
            // The rest of the entry before, over a page.
            Some(last) if last.0 == id && !id.is_empty() => {
                let mut paragraphs = paragraphs.into_iter();
                if let (Some(tail), Some(first)) = (last.3.last_mut(), paragraphs.next()) {
                    if !tail.ends_with(' ') && !first.starts_with(' ') {
                        tail.push(' ');
                    }
                    tail.push_str(&first);
                }
                last.3.extend(paragraphs);
            }
            _ => records.push((id, lemma, grammar, paragraphs, strongs)),
        }
    }
    let mut entries = Vec::new();
    for (_, lemma, grammar, mut paragraphs, strongs) in records {
        let written = lemma
            .split(',')
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        let headword = canonical(&written.replace('-', ""));
        if headword.is_empty() {
            continue;
        }
        // The first paragraph may repeat the headword and its grammar.
        if let Some(first) = paragraphs.first_mut() {
            let mut rest = first.as_str();
            if let Some(after) = rest.strip_prefix(lemma.as_str()) {
                rest = after;
            } else if let Some(after) = rest.strip_prefix(written.as_str()) {
                rest = after;
            }
            let trimmed_grammar = grammar.trim().trim_start_matches(", ");
            let after_comma = rest.trim_start_matches(", ");
            if !trimmed_grammar.is_empty() && after_comma.starts_with(trimmed_grammar) {
                rest = &after_comma[trimmed_grammar.len()..];
            }
            *first = rest.trim_start_matches([',', ' ']).to_string();
        }
        let mut parents = thayer_parents(&grammar);
        if parents.is_empty()
            && let Some(first) = paragraphs.first()
        {
            parents = thayer_parents(&first.chars().take(160).collect::<String>());
        }
        let tail = written
            .rsplit_once('-')
            .map(|(_, tail)| canonical(tail))
            .filter(|t| !t.is_empty());
        let mut body = Map::new();
        let senses: Vec<Value> = paragraphs
            .iter()
            .filter(|p| !p.trim().is_empty())
            .map(|p| json!({ "definition": thayer_spans(p).to_json() }))
            .collect();
        body.insert("senses".into(), Value::Array(senses));
        let forms = grammar
            .trim()
            .trim_start_matches(", ")
            .trim_end_matches(',');
        if !forms.is_empty() {
            body.insert("forms".into(), thayer_spans(forms).to_json());
        }
        entries.push(Entry {
            headword,
            gloss: String::new(),
            strongs,
            body: Value::Object(body),
            parents,
            tail,
        });
    }
    Ok(entries)
}

/// Prefixes that make no root: the prepositions, the privative and the
/// adverbs a compound is built with, in Robinson's transliteration.
const PREFIXES: &[&str] = &[
    "a", "an", "amfi", "ana", "anti", "apo", "dia", "eiv", "ek", "ex", "en", "epi", "kata", "meta",
    "para", "peri", "pro", "prov", "sun", "uper", "upo", "eu", "duv", "hmi", "ou", "mh", "palin",
    "ama", "eso", "exw", "panu",
];

/// The TR's dictionary forms, with the lexicon entries describing each and
/// the roots each is built on.
struct Joined {
    /// Entries by source, with the forms each describes.
    entries: Vec<(&'static str, Entry, Vec<String>)>,
    roots: BTreeMap<String, Vec<String>>,
}

/// A Greek word as a key insensitive to accents and breathings.
fn loose(word: &str) -> String {
    robinson(word).replace('v', "s")
}

fn join(forms: &[(String, u32)], abbott_smith: Vec<Entry>, thayer: Vec<Entry>) -> Joined {
    let mut by_strong: HashMap<u32, Vec<&str>> = HashMap::new();
    let mut by_spelling: HashMap<String, Vec<&str>> = HashMap::new();
    let mut by_loose: HashMap<String, Vec<&str>> = HashMap::new();
    let mut all: Vec<&str> = Vec::new();
    for (form, strong) in forms.iter().filter(|(f, _)| !f.is_empty()) {
        let list = by_strong.entry(*strong).or_default();
        if !list.contains(&form.as_str()) {
            list.push(form);
        }
        if all.contains(&form.as_str()) {
            continue;
        }
        all.push(form);
        // `ὅς, ἥ`: each spelling names the form.
        for spelling in form.split(", ") {
            by_spelling
                .entry(spelling.to_string())
                .or_default()
                .push(form);
            by_loose.entry(loose(spelling)).or_default().push(form);
        }
    }
    let find = |word: &str| -> Option<&str> {
        if let Some(found) = by_spelling.get(word) {
            return found.first().copied();
        }
        match by_loose.get(&loose(word)).map(Vec::as_slice) {
            Some([one]) => Some(*one),
            _ => None,
        }
    };
    let describes = |entry: &Entry| -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for strong in &entry.strongs {
            for form in by_strong.get(strong).into_iter().flatten() {
                if !out.iter().any(|f| f == form) {
                    out.push(form.to_string());
                }
            }
        }
        if out.is_empty()
            && let Some(form) = find(&entry.headword)
        {
            out.push(form.to_string());
        }
        out
    };

    // A word either lexicon has an entry for, or the TR uses.
    let mut headwords: HashSet<String> = HashSet::new();
    let mut loose_headwords: HashSet<String> = HashSet::new();
    for entry in abbott_smith.iter().chain(&thayer) {
        headwords.insert(entry.headword.clone());
        loose_headwords.insert(loose(&entry.headword));
    }
    let is_word = |word: &str| {
        find(word).is_some() || headwords.contains(word) || loose_headwords.contains(&loose(word))
    };

    // The words each word is derived from: Abbott-Smith's, or failing that
    // Thayer's, by headword and by the forms the entry describes. Thayer's
    // derivations and either's hyphenated last element count only where they
    // name a word.
    let mut parents: HashMap<String, Vec<String>> = HashMap::new();
    let mut entries = Vec::new();
    for (source, list) in [(ABBOTT_SMITH, abbott_smith), (THAYER, thayer)] {
        for entry in list {
            let described = describes(&entry);
            let mut own: Vec<String> = entry
                .parents
                .iter()
                .filter(|p| **p != entry.headword && !is_capitals(p))
                .filter(|p| source == ABBOTT_SMITH || is_word(p))
                .cloned()
                .collect();
            if let Some(tail) = &entry.tail
                && *tail != entry.headword
                && is_word(tail)
                && !own.iter().any(|p| loose(p) == loose(tail))
            {
                own.push(tail.clone());
            }
            let mut keys = vec![entry.headword.clone()];
            keys.extend(described.iter().cloned());
            for key in keys {
                let known = parents.entry(key).or_default();
                if known.is_empty() {
                    known.extend(own.iter().cloned());
                }
            }
            if !described.is_empty() {
                entries.push((source, entry, described));
            }
        }
    }

    // A word as the node it is: the TR's form it names, or itself.
    let node = |word: &str| -> String {
        find(word)
            .map(str::to_string)
            .unwrap_or_else(|| word.to_string())
    };
    let mut memo: HashMap<String, Vec<String>> = HashMap::new();
    fn roots_of(
        word: &str,
        parents: &HashMap<String, Vec<String>>,
        node: &dyn Fn(&str) -> String,
        memo: &mut HashMap<String, Vec<String>>,
        visiting: &mut HashSet<String>,
    ) -> Vec<String> {
        if let Some(known) = memo.get(word) {
            return known.clone();
        }
        if !visiting.insert(word.to_string()) {
            return vec![word.to_string()];
        }
        let own: Vec<String> = word
            .split(", ")
            .flat_map(|w| parents.get(w).into_iter().flatten())
            .chain(parents.get(word).into_iter().flatten())
            .filter(|p| !PREFIXES.contains(&loose(p).as_str()))
            .map(|p| node(p))
            .filter(|p| p != word)
            .collect();
        let mut roots = Vec::new();
        // The last element first: a compound is its head's before its
        // modifier's.
        for parent in own.iter().rev() {
            for root in roots_of(parent, parents, node, memo, visiting) {
                if !roots.contains(&root) {
                    roots.push(root);
                }
            }
        }
        if roots.is_empty() {
            roots.push(word.to_string());
        }
        visiting.remove(word);
        memo.insert(word.to_string(), roots.clone());
        roots
    }
    let mut roots = BTreeMap::new();
    for form in &all {
        let found = roots_of(form, &parents, &node, &mut memo, &mut HashSet::new());
        roots.insert(form.to_string(), found);
    }
    Joined { entries, roots }
}

/// Read both lexicons from the downloaded sources in `from` and write the
/// prepared files' text: the entries, and the roots. `forms` are the TR's
/// dictionary forms with each one's Strong's number.
pub(crate) fn prepare(
    from: &Path,
    forms: &[(String, u32)],
) -> Result<(String, String, LexiconSummary)> {
    let abbott_smith = read_abbott_smith(&from.join(ABBOTT_SMITH_FILE))?;
    let thayer = read_thayer(&from.join(THAYER_FILE))?;
    Ok(prepared(forms, abbott_smith, thayer))
}

fn prepared(
    forms: &[(String, u32)],
    abbott_smith: Vec<Entry>,
    thayer: Vec<Entry>,
) -> (String, String, LexiconSummary) {
    let mut summary = LexiconSummary {
        abbott_smith: (abbott_smith.len(), 0),
        thayer: (thayer.len(), 0),
        ..LexiconSummary::default()
    };
    let joined = join(forms, abbott_smith, thayer);
    let mut lexicon = String::new();
    let mut described: HashSet<&str> = HashSet::new();
    for (source, entry, forms) in &joined.entries {
        if *source == ABBOTT_SMITH {
            summary.abbott_smith.1 += 1;
        } else {
            summary.thayer.1 += 1;
        }
        described.extend(forms.iter().map(String::as_str));
        let line = json!({
            "source": source,
            "headword": entry.headword,
            "gloss": entry.gloss,
            "forms": forms,
            "body": entry.body,
        });
        lexicon.push_str(&line.to_string());
        lexicon.push('\n');
    }
    let mut roots = String::new();
    let mut distinct: HashSet<&str> = HashSet::new();
    for (form, list) in &joined.roots {
        summary.forms += 1;
        if list.iter().any(|r| r != form) {
            summary.derived += 1;
        }
        for (ord, root) in list.iter().enumerate() {
            distinct.insert(root);
            roots.push_str(&format!("{form}\t{root}\t{ord}\n"));
        }
    }
    summary.described = described.len();
    summary.roots = distinct.len();
    (lexicon, roots, summary)
}

// Building: the prepared files read into the runtime database.

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS greek_dictionary;
DROP TABLE IF EXISTS greek_dictionary_form;
DROP TABLE IF EXISTS greek_root;

-- Abbott-Smith's and Thayer's entries (`source`), each with its headword,
-- its gloss (empty where the lexicon gives none apart) and its `body`, the
-- span JSON the app renders, stored as `meta.blob_codec` says.
CREATE TABLE greek_dictionary(
    entry_id INTEGER PRIMARY KEY,
    source   TEXT NOT NULL,
    headword TEXT NOT NULL,
    gloss    TEXT NOT NULL,
    body     BLOB NOT NULL
);

-- The dictionary forms of the TR (`greek_lemma.lemma`) each entry describes.
CREATE TABLE greek_dictionary_form(
    lemma    TEXT    NOT NULL,
    entry_id INTEGER NOT NULL,
    PRIMARY KEY(lemma, entry_id)
) WITHOUT ROWID;

-- The roots each dictionary form of the TR is built on, `ord 0` first: the
-- word its derivation ends at, which may be a word the New Testament does
-- not use. A form derived from nothing given is its own root.
CREATE TABLE greek_root(
    lemma TEXT    NOT NULL,
    root  TEXT    NOT NULL,
    ord   INTEGER NOT NULL,
    PRIMARY KEY(lemma, root)
) WITHOUT ROWID;
CREATE INDEX idx_greek_root_root ON greek_root(root);
";

/// The lexicon's own zstd dictionary in `blob_dict`, beside the build's (1)
/// and the English translation's (2).
const DICT_ID: i64 = 3;
const ZSTD_DICT_BYTES: usize = 65_536;
const ZSTD_LEVEL: i32 = 12;

/// What [`build`] wrote, for the log.
#[derive(Debug, Default)]
pub struct LexiconBuild {
    pub entries: usize,
    pub forms: usize,
    pub roots: usize,
    pub bytes: usize,
}

/// Rebuild the lexicon and root tables of a runtime database from the
/// prepared files in `dir`.
pub(crate) fn build(db: &Connection, dir: &Path) -> Result<LexiconBuild> {
    let read = |name: &str| {
        let path = dir.join(name);
        std::fs::read_to_string(&path).with_context(|| {
            format!(
                "reading {} (run scripts/fetch-stepbible-data.sh)",
                path.display()
            )
        })
    };
    let lexicon = read(PREPARED_LEXICON)?;
    let roots = read(PREPARED_ROOTS)?;
    let mut entries: Vec<(String, String, String, String, Vec<String>)> = Vec::new();
    for line in lexicon.lines() {
        let row: Value = serde_json::from_str(line).context("reading a prepared Greek entry")?;
        let text = |key: &str| row[key].as_str().unwrap_or_default().to_string();
        let forms = row["forms"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        entries.push((
            text("source"),
            text("headword"),
            text("gloss"),
            row["body"].to_string(),
            forms,
        ));
    }

    let tx = db.unchecked_transaction()?;
    tx.execute_batch(SCHEMA)?;
    let mut summary = LexiconBuild::default();
    let mut compressor = if crate::translation::compresses_blobs(&tx)? {
        let samples: Vec<&[u8]> = entries.iter().map(|e| e.3.as_bytes()).collect();
        let dictionary = zstd::dict::from_samples(&samples, ZSTD_DICT_BYTES)
            .context("training the Greek lexicon's zstd dictionary")?;
        let others: Vec<Vec<u8>> = tx
            .prepare("SELECT data FROM blob_dict WHERE dict_id != ?1")?
            .query_map([DICT_ID], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let id = crate::translation::dictionary_id(&dictionary)
            .context("an unreadable trained dictionary")?;
        if others
            .iter()
            .any(|other| crate::translation::dictionary_id(other) == Some(id))
        {
            bail!("the Greek lexicon's zstd dictionary has the same id ({id}) as another");
        }
        tx.execute(
            "INSERT OR REPLACE INTO blob_dict(dict_id, data) VALUES (?1, ?2)",
            params![DICT_ID, dictionary],
        )?;
        Some(
            zstd::bulk::Compressor::with_dictionary(ZSTD_LEVEL, &dictionary)
                .context("preparing the Greek lexicon's zstd compressor")?,
        )
    } else {
        None
    };
    {
        let mut insert = tx.prepare(
            "INSERT INTO greek_dictionary(entry_id, source, headword, gloss, body) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        let mut form = tx.prepare(
            "INSERT OR IGNORE INTO greek_dictionary_form(lemma, entry_id) VALUES (?1, ?2)",
        )?;
        for (index, (source, headword, gloss, body, forms)) in entries.iter().enumerate() {
            let id = index as i64 + 1;
            let blob = match compressor.as_mut() {
                Some(c) => c
                    .compress(body.as_bytes())
                    .context("compressing an entry")?,
                None => body.as_bytes().to_vec(),
            };
            summary.bytes += blob.len();
            insert.execute(params![id, source, headword, gloss, blob])?;
            for lemma in forms {
                form.execute(params![lemma, id])?;
                summary.forms += 1;
            }
            summary.entries += 1;
        }
        let mut insert =
            tx.prepare("INSERT OR IGNORE INTO greek_root(lemma, root, ord) VALUES (?1, ?2, ?3)")?;
        for line in roots.lines() {
            let mut fields = line.split('\t');
            let (Some(lemma), Some(root), Some(ord)) =
                (fields.next(), fields.next(), fields.next())
            else {
                bail!("an unreadable Greek root {line:?}");
            };
            insert.execute(params![lemma, root, ord.parse::<i64>()?])?;
            summary.roots += 1;
        }
    }
    tx.commit()?;
    info!(
        "Greek lexicons: {} entries ({} bytes) describing {} dictionary forms, {} roots",
        summary.entries, summary.bytes, summary.forms, summary.roots
    );
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn abbott_smith(xml: &str) -> Vec<Entry> {
        let mut reader = crate::xml::Reader::from_str(xml);
        parse_abbott_smith(|| reader.next()).unwrap()
    }

    const BIBLOS: &str = r#"<TEI><entry lemma="βίβλος" strong="G976">
  <note type="occurrencesNT">10</note>
  <form><orth>βίβλος</orth>, <foreign xml:lang="grc">-ου, ἡ</foreign> (variant form of <ref><foreign xml:lang="grc">βύβλος</foreign></ref>), </form>
<etym>
  <seg type="septuagint">[in LXX for <foreign xml:lang="heb" n="H5612">סֶפֶר</foreign>;]</seg>
</etym>
  <sense><gloss>a book</gloss>, <gloss>a roll</gloss>: <ref osisRef="Matt.1.1">Mt 1:1</ref> <ref osisRef="Mark.12.26">Mk 12:26</ref>.†</sense>
</entry>
<entry lemma="βιβλαρίδιον" strong="G974">
  <form>*† <orth>βιβλαρίδιον</orth>, <foreign xml:lang="grc">-ου, τό</foreign></form>
<etym>
  <seg type="derivation">(dim. of <ref><foreign xml:lang="grc">βιβλάριον</foreign></ref>, dim. of <ref><foreign xml:lang="grc">βίβλος</foreign></ref>),</seg>
</etym>
  <sense n="1."><gloss>a little book</gloss>: <ref osisRef="Rev.10.2">Re 10:2</ref>.<sense n="(a)">within</sense></sense>
</entry>
<entry lemma="ἐκβάλλω" strong="G1544"><form><orth>ἐκ-βάλλω </orth></form><sense><gloss>to cast out</gloss></sense></entry>
<entry lemma="λόγος" strong="G3056"><form><orth>λόγος</orth>, <foreign xml:lang="grc">-ου, ὁ</foreign> (&lt; <foreign xml:lang="grc">λέγω</foreign>),</form><sense><gloss>a word</gloss></sense></entry>
</TEI>"#;

    #[test]
    fn reads_abbott_smith() {
        let entries = abbott_smith(BIBLOS);
        assert_eq!(entries.len(), 4);
        let biblos = &entries[0];
        assert_eq!(biblos.headword, "βίβλος");
        assert_eq!(biblos.strongs, [976]);
        assert_eq!(biblos.gloss, "a book, a roll");
        assert!(biblos.parents.is_empty());
        let senses = biblos.body["senses"].as_array().unwrap();
        assert_eq!(
            senses[0]["definition"],
            json!([
                {"t": "a book", "b": true},
                {"t": ", "},
                {"t": "a roll", "b": true},
                {"t": ": "},
                {"t": "Mt 1:1", "href": "Matthew 1:1"},
                {"t": " "},
                {"t": "Mk 12:26", "href": "Mark 12:26"},
                {"t": ".†"},
            ])
        );
        assert_eq!(
            biblos.body["lxx"],
            json!([
                {"t": "[in LXX for "},
                {"t": "סֶפֶר", "rtl": true},
                {"t": ";]"},
            ])
        );
        assert_eq!(biblos.body["forms"][0], json!({"t": "-ου, ἡ", "g": true}));
        assert_eq!(biblos.body["forms"][2]["dref"], "βύβλος");

        let little = &entries[1];
        assert_eq!(little.parents, ["βιβλάριον", "βίβλος"]);
        let senses = little.body["senses"].as_array().unwrap();
        assert_eq!(senses[0]["num"], "1.");
        assert_eq!(senses[0]["senses"][0]["num"], "(a)");

        assert_eq!(entries[2].headword, "ἐκβάλλω");
        assert_eq!(entries[3].parents, ["λέγω"]);
    }

    #[test]
    fn drops_strongs_numbers_written_as_text() {
        assert_eq!(
            without_numbers("(109):10 (H7592) Si 40:28"),
            "(109):10 Si 40:28"
        );
        assert_eq!(without_numbers("see G976 and H1, Hab."), "see and , Hab.");
        assert_eq!(without_numbers("(Hdt.) GH2"), "(Hdt.) GH2");
    }

    #[test]
    fn links_thayers_references() {
        let spans = thayer_spans(
            "a scroll: Lk. iv. 17, 20; Jn. xx. 30; xxi. 2; 1 Pet. ii. 14. [Sir. xlii. 14.]",
        );
        let links: Vec<&str> = spans
            .0
            .iter()
            .filter_map(|(_, s)| s.href.as_deref())
            .collect();
        assert_eq!(
            links,
            [
                "Luke 4:17",
                "Luke 4:20",
                "John 20:30",
                "John 21:2",
                "I Peter 2:14"
            ]
        );
        let greek = thayer_spans("β. ἀποστασίου [bill]");
        assert!(greek.0[0].1.greek);
    }

    #[test]
    fn reads_thayers_derivations() {
        assert_eq!(thayer_parents(", -ῶ: fut. κακώσω; (κακός);"), ["κακός"]);
        assert_eq!(
            thayer_parents(", -ων, τά, (fr. the adj. γενέσιος fr. γένεσις),"),
            ["γενέσιος"]
        );
        assert_eq!(thayer_parents(", -εως, ἡ, (βρόω, βιβρώσκω);"), ["βρόω"]);
        assert!(thayer_parents("(δήμιος public, and ΕΡΓΩ)").len() == 1);
        assert!(thayer_parents("(fr. ΕΡΓΩ)").is_empty());
        assert!(thayer_parents("-ή, -όν,").is_empty());
    }

    #[test]
    fn joins_a_page_split_thayer_entry() {
        let text = concat!(
            r#"{"id":"G975","strongs":"G975","lemma":"βιβλίον","grammar":", -ου, τό, (dimin. of βίβλος),","paragraphs":["a small book: 2 Tim. iv. "]}"#,
            "\n",
            r#"{"id":"G975","strongs":"G975","lemma":"βιβλίον","grammar":null,"paragraphs":["13, etc."]}"#,
            "\n",
            r#"{"id":"T0003","strongs":null,"lemma":"ἀγαθο-ποιέω","grammar":", -ῶ; (fr. ἀγαθοποιός);","paragraphs":["ἀγαθο-ποιέω, -ῶ; (fr. ἀγαθοποιός); to do good"]}"#,
        );
        let entries = parse_thayer(text).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].parents, ["βίβλος"]);
        let definition = &entries[0].body["senses"][0]["definition"];
        assert_eq!(
            definition[1],
            json!({"t": "2 Tim. iv. 13", "href": "II Timothy 4:13"})
        );
        assert_eq!(entries[1].headword, "ἀγαθοποιέω");
        assert_eq!(entries[1].parents, ["ἀγαθοποιός"]);
        assert_eq!(entries[1].tail.as_deref(), Some("ποιέω"));
        assert_eq!(
            entries[1].body["senses"][0]["definition"][0]["t"],
            "to do good"
        );
    }

    #[test]
    fn finds_roots_and_writes_no_strongs() {
        let forms: Vec<(String, u32)> = [
            ("βίβλος", 976),
            ("βιβλαρίδιον", 974),
            ("βιβλίον", 975),
            ("βάλλω", 906),
            ("ἐκβάλλω", 1544),
            ("λέγω", 3004),
            ("λόγος", 3056),
            ("ἀγαθός", 18),
            ("ποιέω", 4160),
            ("ἀγαθοποιός", 17),
        ]
        .iter()
        .map(|(f, s)| (f.to_string(), *s))
        .collect();
        let mut entries = abbott_smith(BIBLOS);
        entries.push(Entry {
            headword: "ἀγαθοποιός".into(),
            strongs: vec![17],
            parents: vec!["ἀγαθός".into(), "ποιέω".into()],
            ..Entry::default()
        });
        let thayer = parse_thayer(
            r#"{"id":"G975","strongs":"G975","lemma":"βιβλίον","grammar":", -ου, τό, (dimin. of βίβλος),","paragraphs":["a small book"]}"#,
        )
        .unwrap();
        let (lexicon, roots, summary) = prepared(&forms, entries, thayer);
        assert!(!crate::stepbible::has_strong(&lexicon));
        assert!(!crate::stepbible::has_strong(&roots));
        assert!(roots.contains("βιβλαρίδιον\tβίβλος\t0\n"));
        assert!(roots.contains("βιβλίον\tβίβλος\t0\n"));
        assert!(roots.contains("ἐκβάλλω\tβάλλω\t0\n"));
        assert!(roots.contains("λόγος\tλέγω\t0\n"));
        assert!(roots.contains("ἀγαθοποιός\tποιέω\t0\nἀγαθοποιός\tἀγαθός\t1\n"));
        assert!(roots.contains("βίβλος\tβίβλος\t0\n"));
        assert_eq!(summary.forms, 10);
        let first: Value = serde_json::from_str(lexicon.lines().next().unwrap()).unwrap();
        assert_eq!(first["source"], "abbott-smith");
        assert_eq!(first["forms"], json!(["βίβλος"]));
    }
}
