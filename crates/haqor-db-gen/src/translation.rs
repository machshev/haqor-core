//! Import the unfoldingWord® Literal Text, aligned word by word to the Hebrew.
//!
//! The ULT (unfoldingWord, CC BY-SA 4.0) is a literal English translation in
//! the line of the ASV. Its USFM wraps every English word in `\zaln-s`
//! milestones naming the Hebrew word or words it renders: their text
//! (`x-content`) and which occurrence of that text in the verse they are
//! (`x-occurrence`). Those Hebrew words are the unfoldingWord Hebrew Bible's
//! (UHB, CC BY-SA 4.0), which is also fetched, because neither text numbers
//! verses as Haqor's does: both follow the English numbering, so that
//! Malachi 4:1 is Haqor's 3:19 and a psalm's title is not its first verse.
//! `scripts/fetch-unfoldingword.sh` downloads the two and [`prepare`] reduces
//! them to the words and alignments the build reads, vendored in
//! `src_texts/unfoldingWord`.
//!
//! So the import goes by words, not references. The UHB's words, book by
//! book, are lined up against the corpus by their letters: the UHB is derived
//! from the same Leningrad text, and differs where it writes the ketiv
//! (Haqor's text is the qere), divides a word differently, or adds a verse
//! Leningrad lacks (Nehemiah 7:68). Each alignment then names a UHB word, and
//! through it a word of the corpus; and each English verse is filed under the
//! Haqor verse most of its words render.
//!
//! The result is the `translation_verse` table, one row per verse, written in
//! the compact form [`haqor_core::translation`] reads.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use haqor_core::data_support::bare_letters;
use log::{debug, info};
use rusqlite::{Connection, OptionalExtension, params};

use crate::runtime_db::pack_ref;

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS translation_verse;
CREATE TABLE translation_verse(ref INTEGER PRIMARY KEY, text BLOB NOT NULL);
";

/// The translation's own zstd dictionary in `blob_dict`, beside the one the
/// rest of the build trains (1). A dictionary trained on Hebrew verses knows
/// nothing useful about English, and each compressed frame names the
/// dictionary it needs, so a reader holding both decodes either.
const DICT_ID: i64 = 2;

/// As for the other blobs: a trained dictionary is what makes compressing
/// verse-sized text worthwhile, and past level 12 zstd spends much for little.
const ZSTD_DICT_BYTES: usize = 65_536;
const ZSTD_LEVEL: i32 = 12;

/// `src_texts/unfoldingWord`.
pub fn source_dir(src_texts: &Path) -> PathBuf {
    src_texts.join("unfoldingWord")
}

/// A chapter and verse, in whichever numbering the text at hand uses.
type ChapterVerse = (u8, u8);

/// What a USFM file says, as far as the import cares.
#[derive(Debug, Clone, PartialEq)]
enum Event {
    Chapter(u8),
    Verse(u8),
    /// `\d`, a psalm's title, which may come before its chapter's first
    /// verse.
    Title,
    /// The start of an alignment: the Hebrew text it names, and which
    /// occurrence of that text in the verse.
    AlignStart {
        content: String,
        occurrence: u16,
    },
    AlignEnd,
    /// A word, `\w text|attributes\w*`.
    Word(String),
    /// Text between words: spaces, punctuation, the braces round supplied
    /// words.
    Text(String),
}

/// Markers whose content runs to the end of the line and is not verse text:
/// the book's headers, a chapter's label, Psalms' book divisions, and the
/// letters heading an acrostic's stanzas.
const LINE_MARKERS: [&str; 11] = [
    "id", "ide", "usfm", "h", "toc1", "toc2", "toc3", "mt", "mt1", "ms1", "cl",
];

/// The value of `key="…"` in a USFM attribute list.
fn attribute<'a>(attributes: &'a str, key: &str) -> Option<&'a str> {
    let start = attributes.find(&format!("{key}=\""))? + key.len() + 2;
    let len = attributes[start..].find('"')?;
    Some(&attributes[start..start + len])
}

/// Read the events of a USFM file. Footnotes are skipped (the UHB's give the
/// qere of a ketiv, the ULT's alternative renderings), as are alternative
/// verse and chapter numbers and the headings [`LINE_MARKERS`] names.
fn read_usfm(usfm: &str) -> Result<Vec<Event>> {
    let mut events = Vec::new();
    let mut rest = usfm;
    // Text before the next marker.
    let text_end = |rest: &str| rest.find('\\').unwrap_or(rest.len());
    while !rest.is_empty() {
        if !rest.starts_with('\\') {
            let end = text_end(rest);
            events.push(Event::Text(rest[..end].to_string()));
            rest = &rest[end..];
            continue;
        }
        let name_len = rest[1..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .unwrap_or(rest.len() - 1);
        let name = &rest[1..1 + name_len];
        rest = &rest[1 + name_len..];
        // A closing marker (`\qs*`), or a milestone's end (`\zaln-e\*`).
        if let Some(r) = rest.strip_prefix('*') {
            rest = r;
            continue;
        }
        if let Some(r) = rest.strip_prefix("\\*") {
            rest = r;
            if name == "zaln-e" {
                events.push(Event::AlignEnd);
            }
            continue;
        }
        // Up to (not including) `end`, after which reading resumes.
        let mut until = |end: &str| -> Result<&str> {
            let at = rest
                .find(end)
                .with_context(|| format!("\\{name} without {end}"))?;
            let content = &rest[..at];
            rest = &rest[at + end.len()..];
            Ok(content)
        };
        match name {
            "c" | "v" => {
                let content = rest.trim_start();
                let len = content
                    .find(|c: char| !c.is_ascii_digit())
                    .unwrap_or(content.len());
                let number = content[..len]
                    .parse()
                    .with_context(|| format!("\\{name} without a number"))?;
                rest = &content[len..];
                events.push(if name == "c" {
                    Event::Chapter(number)
                } else {
                    Event::Verse(number)
                });
            }
            "d" => events.push(Event::Title),
            "zaln-s" => {
                let attributes = until("\\*")?;
                let content =
                    attribute(attributes, "x-content").context("an alignment without x-content")?;
                let occurrence = attribute(attributes, "x-occurrence")
                    .and_then(|o| o.parse().ok())
                    .context("an alignment without x-occurrence")?;
                events.push(Event::AlignStart {
                    content: content.to_string(),
                    occurrence,
                });
            }
            "w" => {
                let word = until("\\w*")?;
                let text = word.split_once('|').map_or(word, |(text, _)| text);
                events.push(Event::Word(text.trim().to_string()));
            }
            "f" => {
                until("\\f*")?;
            }
            "va" => {
                until("\\va*")?;
            }
            "ca" => {
                until("\\ca*")?;
            }
            "qa" => {
                // The ULT closes an acrostic heading (`\qa Aleph\qa*`), but
                // USFM allows it to run to the end of its line.
                let end = rest.find('\n').unwrap_or(rest.len());
                match rest[..end].find("\\qa*") {
                    Some(at) => rest = &rest[at + 4..],
                    None => rest = &rest[end..],
                }
            }
            name if LINE_MARKERS.contains(&name) => {
                rest = &rest[rest.find('\n').unwrap_or(rest.len())..];
            }
            // Paragraph and poetry markers, and the ULT's chunk breaks: their
            // text is the verse's, and their layout is not kept.
            _ => events.push(Event::Text(" ".into())),
        }
    }
    Ok(events)
}

/// A Hebrew word's identity as the alignments name it: its text, without
/// the word joiners the UHB puts between morphemes, and with the marks on
/// each letter in a fixed order (the ULT and UHB order a dagesh and a vowel
/// differently here and there). With `accents` false, also without
/// cantillation, which a few alignments count occurrences without.
fn identity(word: &str, accents: bool) -> String {
    let mut out = String::new();
    let mut marks: Vec<char> = Vec::new();
    for c in word.chars().filter(|&c| c != '\u{2060}') {
        let is_mark = ('\u{591}'..='\u{5c7}').contains(&c)
            && !matches!(c, '\u{5be}' | '\u{5c0}' | '\u{5c3}' | '\u{5c6}');
        if is_mark {
            // Without accents, also without meteg, which the accents govern.
            if accents || !(('\u{591}'..='\u{5af}').contains(&c) || c == '\u{5bd}') {
                marks.push(c);
            }
            continue;
        }
        marks.sort_unstable();
        out.extend(marks.drain(..));
        out.push(c);
    }
    marks.sort_unstable();
    out.extend(marks);
    out
}

/// A word of the UHB.
#[derive(Debug)]
struct HebrewWord {
    /// Its verse, in the UHB's (English) numbering; verse 0 for a psalm title
    /// set before its chapter's first verse.
    verse: ChapterVerse,
    /// Its [`identity`] and the occurrence of that in its verse, with and
    /// without accents.
    identity: (String, u16),
    bare_identity: (String, u16),
    /// The word as it is written: its letters line it up against the corpus.
    text: String,
}

fn hebrew_words(events: &[Event]) -> Vec<HebrewWord> {
    let mut words = Vec::new();
    let mut verse = (0, 0);
    let mut seen: HashMap<String, u16> = HashMap::new();
    let occurrence = |seen: &mut HashMap<String, u16>, identity: String| {
        let n = seen.entry(identity.clone()).or_default();
        *n += 1;
        (identity, *n)
    };
    for event in events {
        match event {
            Event::Chapter(c) => {
                verse = (*c, 0);
                seen.clear();
            }
            Event::Verse(v) => {
                verse.1 = *v;
                seen.clear();
            }
            Event::Word(text) => words.push(HebrewWord {
                verse,
                identity: occurrence(&mut seen, identity(text, true)),
                bare_identity: occurrence(&mut seen, format!("~{}", identity(text, false))),
                text: text.clone(),
            }),
            _ => {}
        }
    }
    words
}

/// Line up two word sequences by their letters: for each word of `from`, the
/// index of the word of `to` it falls in, if any.
///
/// Mostly the two agree word for word. Where they divide words differently,
/// a run of up to three words on each side spelling the same letters is
/// matched by where each word of `from` starts. Anything else is a stretch
/// one text has and the other lacks or spells differently; reading resumes
/// at the nearest place both agree for three words running, words of the
/// stretch pairing off in order. A long stretch (a whole verse one text
/// lacks) widens the search until it is found.
fn line_up(from: &[&str], to: &[&str]) -> Vec<Option<usize>> {
    const AGREE: usize = 3;
    let mut placed = vec![None; from.len()];
    let (mut i, mut j) = (0, 0);
    while i < from.len() && j < to.len() {
        if from[i] == to[j] {
            placed[i] = Some(j);
            i += 1;
            j += 1;
            continue;
        }
        if let Some((k, l)) = regrouped(&from[i..], &to[j..]) {
            let mut owner = Vec::new();
            for (n, word) in to[j..j + l].iter().enumerate() {
                owner.extend(std::iter::repeat_n(j + n, word.chars().count()));
            }
            let mut offset = 0;
            for (n, word) in from[i..i + k].iter().enumerate() {
                placed[i + n] = owner.get(offset).or(owner.last()).copied();
                offset += word.chars().count();
            }
            i += k;
            j += l;
            continue;
        }
        let agree = |di: usize, dj: usize| {
            let (a, b) = (&from[i + di..], &to[j + dj..]);
            let n = AGREE.min(a.len()).min(b.len());
            n > 0 && a[..n] == b[..n]
        };
        let mut window = 8;
        let resume = loop {
            let found = (1..=2 * window).find_map(|distance| {
                (0..=distance)
                    .map(|di| (di, distance - di))
                    .filter(|&(di, dj)| di <= window && dj <= window)
                    .filter(|&(di, dj)| i + di < from.len() && j + dj < to.len())
                    .find(|&(di, dj)| agree(di, dj))
            });
            if found.is_some() || window >= 1024 {
                break found;
            }
            window *= 4;
        };
        let Some((di, dj)) = resume else { break };
        for n in 0..di.min(dj) {
            placed[i + n] = Some(j + n);
        }
        i += di;
        j += dj;
    }
    placed
}

/// The shortest runs at the start of `from` and `to`, of up to three words
/// each and not both a single word, that spell the same letters.
fn regrouped(from: &[&str], to: &[&str]) -> Option<(usize, usize)> {
    for k in 1..=3.min(from.len()) {
        let spelled: String = from[..k].concat();
        for l in 1..=3.min(to.len()) {
            if (k, l) != (1, 1) && to[..l].concat() == spelled {
                return Some((k, l));
            }
        }
    }
    None
}

/// A word of the corpus: its verse's ref and its position in the verse.
type CorpusWord = (i64, u16);

/// A piece of an English verse as read: a word with the Hebrew it renders,
/// or text between words.
#[derive(Debug)]
enum Piece {
    Word {
        text: String,
        supplied: bool,
        renders: Vec<CorpusWord>,
    },
    Text(String),
}

/// A verse of the ULT, its alignments resolved to the corpus.
#[derive(Debug)]
struct EnglishVerse {
    verse: ChapterVerse,
    pieces: Vec<Piece>,
}

/// How a book's alignments resolved, for the log and the tests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TranslationSummary {
    /// Verses written.
    pub verses: usize,
    /// English words, and how many of them render a word of the corpus.
    pub words: usize,
    pub linked: usize,
    /// Alignments naming a UHB word the corpus does not have: mostly a ketiv
    /// read differently, or never read at all.
    pub unplaced: usize,
    /// English verses rendering no word of the corpus, which have nowhere to
    /// go (Nehemiah 7:68, which Leningrad lacks).
    pub dropped: usize,
    /// Bytes stored, compressed or not, all verses together.
    pub bytes: usize,
}

impl std::ops::AddAssign for TranslationSummary {
    fn add_assign(&mut self, other: Self) {
        self.verses += other.verses;
        self.words += other.words;
        self.linked += other.linked;
        self.unplaced += other.unplaced;
        self.dropped += other.dropped;
        self.bytes += other.bytes;
    }
}

/// A piece of an English verse as the prepared files keep it: a word with
/// the UHB words (by index in its book) it renders, or text between words.
#[derive(Debug, Clone, PartialEq)]
enum SourcePiece {
    Word {
        text: String,
        supplied: bool,
        renders: Vec<usize>,
    },
    Text(String),
}

/// A verse of the ULT, in its own numbering, its alignments resolved to the
/// UHB but not yet to the corpus.
#[derive(Debug, Clone, PartialEq)]
struct SourceVerse {
    verse: ChapterVerse,
    pieces: Vec<SourcePiece>,
}

/// One book as the prepared files keep it: the UHB's words in order, and the
/// ULT's verses aligned to them. Everything the build needs, and nothing that
/// depends on Haqor's corpus.
#[derive(Debug, Default, Clone, PartialEq)]
struct PreparedBook {
    hebrew: Vec<String>,
    verses: Vec<SourceVerse>,
}

/// Read a book of the ULT, resolving each word's alignments to the UHB's
/// words (`hebrew`). Returns its verses and the alignments that named a
/// Hebrew word the UHB does not have where they say.
fn source_verses(events: &[Event], hebrew: &[HebrewWord]) -> (Vec<SourceVerse>, usize) {
    let mut by_identity: HashMap<(ChapterVerse, &(String, u16)), usize> = HashMap::new();
    for (index, word) in hebrew.iter().enumerate() {
        by_identity.insert((word.verse, &word.identity), index);
        by_identity
            .entry((word.verse, &word.bare_identity))
            .or_insert(index);
    }
    // A UHB word by its verse and identity. A psalm title is verse 0 in one
    // text and part of verse 1 in the other as often as not, so each also
    // tries the other.
    let find = |verse: ChapterVerse, content: &str, occurrence: u16| -> Option<usize> {
        let alternative = match verse.1 {
            0 => Some((verse.0, 1)),
            1 => Some((verse.0, 0)),
            _ => None,
        };
        let full = (identity(content, true), occurrence);
        let bare = (format!("~{}", identity(content, false)), occurrence);
        [Some(verse), alternative]
            .into_iter()
            .flatten()
            .find_map(|v| {
                by_identity
                    .get(&(v, &full))
                    .or_else(|| by_identity.get(&(v, &bare)))
                    .copied()
            })
    };

    let mut verses: Vec<SourceVerse> = Vec::new();
    let mut unresolved = 0;
    let mut chapter = 0;
    // The open alignments, outermost first.
    let mut open: Vec<(String, u16)> = Vec::new();
    let mut supplied = false;
    for event in events {
        match event {
            Event::Chapter(c) => {
                chapter = *c;
                // A title opens the chapter's first verse when it comes before it.
                verses.push(SourceVerse {
                    verse: (chapter, 0),
                    pieces: Vec::new(),
                });
            }
            Event::Verse(v) => {
                supplied = false;
                match verses.last_mut() {
                    // No words before the first verse: no title.
                    Some(last) if last.verse == (chapter, 0) && !has_source_words(last) => {
                        last.verse.1 = *v;
                        last.pieces.clear();
                    }
                    _ => verses.push(SourceVerse {
                        verse: (chapter, *v),
                        pieces: Vec::new(),
                    }),
                }
            }
            Event::Title => {}
            Event::AlignStart {
                content,
                occurrence,
            } => open.push((content.clone(), *occurrence)),
            Event::AlignEnd => {
                if open.pop().is_none() {
                    debug!("ULT {chapter}: an alignment closed that was never opened");
                }
            }
            Event::Word(text) => {
                let Some(verse) = verses.last_mut() else {
                    continue;
                };
                let mut renders = Vec::new();
                // Innermost first: the word the English most nearly renders.
                for (content, occurrence) in open.iter().rev() {
                    let Some(index) = find(verse.verse, content, *occurrence) else {
                        debug!(
                            "ULT {:?}: no {content} #{occurrence} in the UHB",
                            verse.verse
                        );
                        unresolved += 1;
                        continue;
                    };
                    if !renders.contains(&index) {
                        renders.push(index);
                    }
                }
                verse.pieces.push(SourcePiece::Word {
                    text: text.clone(),
                    supplied,
                    renders,
                });
            }
            Event::Text(text) => {
                let Some(verse) = verses.last_mut() else {
                    continue;
                };
                let mut plain = String::new();
                for c in text.chars() {
                    match c {
                        '{' => supplied = true,
                        '}' => supplied = false,
                        c => plain.push(c),
                    }
                }
                // Text between two words is one piece, however many markers
                // it was read between.
                match verse.pieces.last_mut() {
                    Some(SourcePiece::Text(last)) => last.push_str(&plain),
                    _ => verse.pieces.push(SourcePiece::Text(plain)),
                }
            }
        }
    }
    verses.retain(has_source_words);
    (verses, unresolved)
}

fn has_source_words(verse: &SourceVerse) -> bool {
    verse
        .pieces
        .iter()
        .any(|p| matches!(p, SourcePiece::Word { .. }))
}

/// Resolve a book's prepared verses to the corpus: `placed` says where each
/// UHB word fell in it (`corpus`).
fn english_verses(
    verses: &[SourceVerse],
    placed: &[Option<usize>],
    corpus: &[CorpusWord],
    summary: &mut TranslationSummary,
) -> Vec<EnglishVerse> {
    let mut out = Vec::new();
    for verse in verses {
        let mut pieces = Vec::new();
        for piece in &verse.pieces {
            match piece {
                SourcePiece::Text(text) => pieces.push(Piece::Text(text.clone())),
                SourcePiece::Word {
                    text,
                    supplied,
                    renders: hebrew,
                } => {
                    summary.words += 1;
                    let mut renders = Vec::new();
                    for &index in hebrew {
                        let Some(at) = placed.get(index).copied().flatten() else {
                            summary.unplaced += 1;
                            continue;
                        };
                        if !renders.contains(&corpus[at]) {
                            renders.push(corpus[at]);
                        }
                    }
                    if !renders.is_empty() {
                        summary.linked += 1;
                    }
                    pieces.push(Piece::Word {
                        text: text.clone(),
                        supplied: *supplied,
                        renders,
                    });
                }
            }
        }
        out.push(EnglishVerse {
            verse: verse.verse,
            pieces,
        });
    }
    out
}

/// The Haqor verse an English verse is filed under: the one most of its
/// words render, the earliest of those tied.
fn home_verse(verse: &EnglishVerse) -> Option<i64> {
    let mut counts: HashMap<i64, usize> = HashMap::new();
    for piece in &verse.pieces {
        if let Piece::Word { renders, .. } = piece {
            for (reference, _) in renders {
                *counts.entry(*reference).or_default() += 1;
            }
        }
    }
    counts
        .into_iter()
        .max_by_key(|&(reference, count)| (count, std::cmp::Reverse(reference)))
        .map(|(reference, _)| reference)
}

/// A link from a verse filed under `home` to `word`, as short as it can be
/// written.
fn link(home: i64, (reference, position): CorpusWord) -> String {
    let (chapter, verse) = ((reference >> 8) & 0xff, reference & 0xff);
    if reference == home {
        position.to_string()
    } else if reference >> 8 == home >> 8 {
        format!("{verse}:{position}")
    } else {
        format!("{chapter}:{verse}:{position}")
    }
}

fn push_escaped(out: &mut String, text: &str, special: &[char]) {
    for c in text.chars() {
        if c == '\\' || special.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
}

/// A run of an English verse as it is written: words rendering the same
/// Hebrew (none, for plain text), all supplied or none.
#[derive(Debug)]
struct Run {
    text: String,
    supplied: bool,
    renders: Vec<CorpusWord>,
    /// Text between words, which a following word may not join.
    between: bool,
}

/// Write an English verse, filed under `home`, in the compact form: words
/// rendering the same Hebrew next to each other gather into one span, spaces
/// are collapsed, and supplied words are braced.
fn write_verse(pieces: &[Piece], home: i64) -> String {
    let mut runs: Vec<Run> = Vec::new();
    for piece in pieces {
        match piece {
            Piece::Text(text) => match runs.last_mut() {
                Some(last) if last.between => last.text.push_str(text),
                _ => runs.push(Run {
                    text: text.clone(),
                    supplied: false,
                    renders: Vec::new(),
                    between: true,
                }),
            },
            Piece::Word {
                text,
                supplied,
                renders,
            } => {
                let joins = |run: &Run| {
                    !run.between && run.supplied == *supplied && run.renders == *renders
                };
                let len = runs.len();
                // Only a space between this and a run it joins.
                if len >= 2
                    && runs[len - 1].between
                    && runs[len - 1].text.trim().is_empty()
                    && joins(&runs[len - 2])
                {
                    runs.pop();
                    let run = runs.last_mut().unwrap();
                    run.text.push(' ');
                    run.text.push_str(text);
                } else if let Some(run) = runs.last_mut().filter(|r| joins(r)) {
                    run.text.push_str(text);
                } else {
                    runs.push(Run {
                        text: text.clone(),
                        supplied: *supplied,
                        renders: renders.clone(),
                        between: false,
                    });
                }
            }
        }
    }

    let mut out = String::new();
    for run in &runs {
        if run.between {
            let mut last_space = false;
            for c in run.text.chars() {
                if c.is_whitespace() {
                    if !last_space {
                        out.push(' ');
                    }
                    last_space = true;
                } else {
                    push_escaped(&mut out, c.encode_utf8(&mut [0; 4]), &['[', '{', '}']);
                    last_space = false;
                }
            }
            continue;
        }
        if run.supplied {
            out.push('{');
        }
        if run.renders.is_empty() {
            push_escaped(&mut out, &run.text, &['[', '{', '}']);
        } else {
            out.push('[');
            push_escaped(&mut out, &run.text, &['|']);
            out.push('|');
            let links: Vec<String> = run.renders.iter().map(|&w| link(home, w)).collect();
            out.push_str(&links.join(","));
            out.push(']');
        }
        if run.supplied {
            out.push('}');
        }
    }
    out.trim().to_string()
}

/// Read one book's USFM, the ULT's and the UHB's, into its prepared form.
/// Returns the book and the alignments that named a Hebrew word the UHB does
/// not have where they say.
#[cfg(test)]
fn prepare_book(ult: &str, uhb: &str) -> Result<(PreparedBook, usize)> {
    let hebrew = hebrew_words(&read_usfm(uhb)?);
    let (verses, unresolved) = source_verses(&read_usfm(ult)?, &hebrew);
    let hebrew = hebrew.into_iter().map(|word| word.text).collect();
    Ok((PreparedBook { hebrew, verses }, unresolved))
}

/// One prepared book, and the corpus's words for the book in order, each with
/// its letters. Returns each verse's compact English by ref.
fn build_book(
    book: &PreparedBook,
    corpus: &[(CorpusWord, String)],
    summary: &mut TranslationSummary,
) -> Vec<(i64, String)> {
    let letters: Vec<String> = book.hebrew.iter().map(|w| bare_letters(w)).collect();
    let from: Vec<&str> = letters.iter().map(String::as_str).collect();
    let to: Vec<&str> = corpus.iter().map(|(_, l)| l.as_str()).collect();
    let placed = line_up(&from, &to);
    let words: Vec<CorpusWord> = corpus.iter().map(|(w, _)| *w).collect();
    let english = english_verses(&book.verses, &placed, &words, summary);

    let mut verses: Vec<(i64, String)> = Vec::new();
    for verse in english {
        let Some(home) = home_verse(&verse) else {
            debug!("ULT {:?}: renders no word of the corpus", verse.verse);
            summary.dropped += 1;
            continue;
        };
        let text = write_verse(&verse.pieces, home);
        match verses.iter_mut().find(|(r, _)| *r == home) {
            // Two English verses rendering the same Hebrew one.
            Some((_, existing)) => {
                existing.push(' ');
                existing.push_str(&text);
            }
            None => verses.push((home, text)),
        }
    }
    verses.sort_by_key(|(r, _)| *r);
    summary.verses += verses.len();
    verses
}

/// A book's USFM file in `dir`, by the number its name starts with.
fn book_file(dir: &Path, number: usize) -> Result<PathBuf> {
    let prefix = format!("{number:02}-");
    std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&prefix) && n.ends_with(".usfm"))
        })
        .with_context(|| format!("no book {number} in {}", dir.display()))
}

/// The prepared files in `src_texts/unfoldingWord`. Each line starts with
/// the book's number in the English order (`01` Genesis … `39` Malachi) and
/// a chapter and verse in the texts' own numbering, which only make the files
/// readable: the build goes by words.
///
/// - `uhb.tsv`: every word of the UHB, in order, as it is written.
/// - `ult.tsv`: every verse of the ULT, in the form [`write_pieces`] writes.
const PREPARED_UHB: &str = "uhb.tsv";
const PREPARED_ULT: &str = "ult.tsv";

/// Characters a backslash escapes in the prepared ULT: the escape itself,
/// the brackets that delimit a word and the braces round a supplied one, the
/// bar between a word and its links, and the line and field separators
/// (written `\n` and `\t`).
fn push_prepared(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\\' | '[' | ']' | '{' | '}' | '|' => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
}

/// Write a verse's pieces: text between words as it is, escaped; a word as
/// `[text|links]`, its links the indices of the UHB words it renders in its
/// book, comma separated; a supplied word braced, `{[text|links]}`.
fn write_pieces(pieces: &[SourcePiece]) -> String {
    let mut out = String::new();
    for piece in pieces {
        match piece {
            SourcePiece::Text(text) => push_prepared(&mut out, text),
            SourcePiece::Word {
                text,
                supplied,
                renders,
            } => {
                if *supplied {
                    out.push('{');
                }
                out.push('[');
                push_prepared(&mut out, text);
                out.push('|');
                let links: Vec<String> = renders.iter().map(usize::to_string).collect();
                out.push_str(&links.join(","));
                out.push(']');
                if *supplied {
                    out.push('}');
                }
            }
        }
    }
    out
}

/// Read pieces written by [`write_pieces`].
fn parse_pieces(line: &str) -> Result<Vec<SourcePiece>> {
    let mut pieces = Vec::new();
    let mut chars = line.chars();
    let mut text = String::new();
    let mut supplied = false;
    let escaped = |c: Option<char>| -> Result<char> {
        Ok(match c.context("a trailing backslash")? {
            'n' => '\n',
            't' => '\t',
            c => c,
        })
    };
    while let Some(c) = chars.next() {
        match c {
            '\\' => text.push(escaped(chars.next())?),
            '{' => supplied = true,
            '}' => supplied = false,
            '[' => {
                if !text.is_empty() {
                    pieces.push(SourcePiece::Text(std::mem::take(&mut text)));
                }
                let mut word = String::new();
                loop {
                    match chars.next().context("an unterminated word")? {
                        '\\' => word.push(escaped(chars.next())?),
                        '|' => break,
                        c => word.push(c),
                    }
                }
                let links: String = chars.by_ref().take_while(|&c| c != ']').collect();
                let renders = links
                    .split(',')
                    .filter(|l| !l.is_empty())
                    .map(|l| l.parse().with_context(|| format!("a link {l:?}")))
                    .collect::<Result<_>>()?;
                pieces.push(SourcePiece::Word {
                    text: word,
                    supplied,
                    renders,
                });
            }
            c => text.push(c),
        }
    }
    if !text.is_empty() {
        pieces.push(SourcePiece::Text(text));
    }
    Ok(pieces)
}

/// What `db prepare unfoldingword` wrote, for the log.
#[derive(Debug, Default)]
pub struct PrepareSummary {
    pub hebrew_words: usize,
    pub verses: usize,
    /// Alignments naming a Hebrew word the UHB does not have where they say,
    /// which the prepared files leave out.
    pub unresolved: usize,
}

/// `db prepare unfoldingword`: read the ULT's and UHB's USFM from `from`'s
/// `en_ult` and `hbo_uhb` and write the prepared files into `out_dir`.
pub fn prepare(from: &Path, out_dir: &Path) -> Result<PrepareSummary> {
    let mut summary = PrepareSummary::default();
    let (mut uhb_out, mut ult_out) = (String::new(), String::new());
    for number in 1..=39 {
        let read = |name: &str| -> Result<String> {
            let path = book_file(&from.join(name), number)?;
            std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
        };
        let uhb = read("hbo_uhb")?;
        let hebrew = hebrew_words(&read_usfm(&uhb)?);
        let (verses, unresolved) = source_verses(&read_usfm(&read("en_ult")?)?, &hebrew);
        summary.unresolved += unresolved;
        for word in &hebrew {
            if word.text.contains(['\t', '\n']) {
                bail!("a UHB word the prepared form cannot hold: {:?}", word.text);
            }
            let (chapter, verse) = word.verse;
            uhb_out.push_str(&format!("{number:02}\t{chapter}:{verse}\t{}\n", word.text));
        }
        for verse in &verses {
            let (chapter, v) = verse.verse;
            ult_out.push_str(&format!(
                "{number:02}\t{chapter}:{v}\t{}\n",
                write_pieces(&verse.pieces)
            ));
        }
        summary.hebrew_words += hebrew.len();
        summary.verses += verses.len();
    }
    std::fs::create_dir_all(out_dir)?;
    for (name, text) in [(PREPARED_UHB, uhb_out), (PREPARED_ULT, ult_out)] {
        let path = out_dir.join(name);
        std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(summary)
}

/// The prepared books, by number (1 Genesis … 39 Malachi).
fn read_prepared(dir: &Path) -> Result<HashMap<usize, PreparedBook>> {
    let read = |name: &str| -> Result<String> {
        let path = dir.join(name);
        std::fs::read_to_string(&path).with_context(|| {
            format!(
                "reading {} (run scripts/fetch-unfoldingword.sh)",
                path.display()
            )
        })
    };
    let fields = |line: &str| -> Result<(usize, String)> {
        let mut parts = line.splitn(3, '\t');
        let number = parts.next().and_then(|n| n.parse().ok());
        let _verse = parts.next();
        match (number, parts.next()) {
            (Some(number), Some(rest)) => Ok((number, rest.to_string())),
            _ => bail!("an unreadable prepared line: {line:?}"),
        }
    };
    let mut books: HashMap<usize, PreparedBook> = HashMap::new();
    for line in read(PREPARED_UHB)?.lines() {
        let (number, word) = fields(line)?;
        books.entry(number).or_default().hebrew.push(word);
    }
    for line in read(PREPARED_ULT)?.lines() {
        let (number, pieces) = fields(line)?;
        let verse = line.split('\t').nth(1).unwrap_or_default();
        let verse = verse
            .split_once(':')
            .and_then(|(c, v)| Some((c.parse().ok()?, v.parse().ok()?)))
            .with_context(|| format!("an unreadable verse {verse:?}"))?;
        books.entry(number).or_default().verses.push(SourceVerse {
            verse,
            pieces: parse_pieces(&pieces)?,
        });
    }
    Ok(books)
}

/// Rebuild the `translation_verse` table of a runtime database in place from
/// the prepared files in `src_texts/unfoldingWord`.
pub fn build_translation(db: &Connection, src_texts: &Path) -> Result<TranslationSummary> {
    let books = read_prepared(&source_dir(src_texts))?;
    let tx = db.unchecked_transaction()?;
    tx.execute_batch(SCHEMA)?;
    let mut summary = TranslationSummary::default();
    let mut verses: Vec<(i64, String)> = Vec::new();
    {
        let mut words = tx.prepare(
            "SELECT w.ref, w.position, s.text FROM word w JOIN surface s USING(surface_id) \
             WHERE w.ref BETWEEN ?1 AND ?2 ORDER BY w.ref, w.position",
        )?;
        // Both texts number their books in the English order, as the TSK's
        // book keys do.
        for number in 1..=39 {
            let book = crate::tsk::book_of_key(number).context("an Old Testament book")?;
            let prepared = books
                .get(&number)
                .with_context(|| format!("no book {number} in the prepared ULT"))?;
            let first = pack_ref(book.into(), 0, 0);
            let corpus = words
                .query_map([first, first | 0xffff], |row| {
                    Ok((
                        (row.get(0)?, row.get(1)?),
                        bare_letters(&row.get::<_, String>(2)?),
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            if corpus.is_empty() {
                bail!("book {book} has no words in the corpus");
            }
            let mut book_summary = TranslationSummary::default();
            verses.extend(build_book(prepared, &corpus, &mut book_summary));
            debug!("ULT book {number}: {book_summary:?}");
            summary += book_summary;
        }
    }
    summary.bytes = write_verses(&tx, &verses)?;
    tx.commit()?;
    info!(
        "Translation: {} verses ({} KiB); {} of {} English words linked ({} on words the \
         corpus lacks; {} verses dropped)",
        summary.verses,
        summary.bytes / 1024,
        summary.linked,
        summary.words,
        summary.unplaced,
        summary.dropped
    );
    Ok(summary)
}

/// Whether the database stores its blobs compressed (`meta.blob_codec`).
/// A database without a `meta` table, as the tests build, does not.
fn compresses_blobs(db: &Connection) -> Result<bool> {
    let has_meta: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'meta')",
        [],
        |row| row.get(0),
    )?;
    if !has_meta {
        return Ok(false);
    }
    let codec: Option<String> = db
        .query_row(
            "SELECT value FROM meta WHERE key = 'blob_codec'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    match codec.as_deref() {
        None | Some("none") => Ok(false),
        Some("zstd") => Ok(true),
        Some(other) => bail!("unknown blob codec {other:?}"),
    }
}

/// The id a zstd dictionary declares in its header, which the frames
/// compressed with it repeat.
fn dictionary_id(dictionary: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(dictionary.get(4..8)?.try_into().ok()?))
}

/// Write the verses in the form `meta.blob_codec` asks for: as they are, or
/// compressed against a dictionary trained on them, stored as [`DICT_ID`].
/// Returns the bytes written.
fn write_verses(db: &Connection, verses: &[(i64, String)]) -> Result<usize> {
    let mut insert = db.prepare("INSERT INTO translation_verse(ref, text) VALUES (?1, ?2)")?;
    let mut bytes = 0;
    if !compresses_blobs(db)? {
        for (reference, text) in verses {
            insert.execute(params![reference, text.as_bytes()])?;
            bytes += text.len();
        }
        return Ok(bytes);
    }
    let samples: Vec<&[u8]> = verses.iter().map(|(_, t)| t.as_bytes()).collect();
    let dictionary = zstd::dict::from_samples(&samples, ZSTD_DICT_BYTES)
        .context("training the translation's zstd dictionary")?;
    // The two dictionaries must differ in id for a frame to name its own.
    let others: Vec<Vec<u8>> = db
        .prepare("SELECT data FROM blob_dict WHERE dict_id != ?1")?
        .query_map([DICT_ID], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let id = dictionary_id(&dictionary).context("an unreadable trained dictionary")?;
    if others.iter().any(|other| dictionary_id(other) == Some(id)) {
        bail!("the translation's zstd dictionary has the same id ({id}) as another");
    }
    db.execute(
        "INSERT OR REPLACE INTO blob_dict(dict_id, data) VALUES (?1, ?2)",
        params![DICT_ID, dictionary],
    )?;
    let mut compressor = zstd::bulk::Compressor::with_dictionary(ZSTD_LEVEL, &dictionary)
        .context("preparing the translation's zstd compressor")?;
    for (reference, text) in verses {
        let blob = compressor
            .compress(text.as_bytes())
            .context("compressing a verse")?;
        bytes += blob.len();
        insert.execute(params![reference, blob])?;
    }
    info!(
        "Trained a {} byte dictionary for the translation",
        dictionary.len()
    );
    Ok(bytes)
}

/// `db gen-translation`: rebuild the `translation_verse` table of the runtime
/// database at `path` in place and re-stamp it, so syncing it to the app
/// reinstalls it.
pub fn gen_translation(path: &Path, src_texts: &Path) -> Result<TranslationSummary> {
    let db = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
    let summary = build_translation(&db, src_texts)?;
    crate::runtime_db::restamp_built(&db)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Genesis 1:1-2 as the UHB has them, abridged.
    const UHB: &str = "\\id GEN unfoldingWord® Hebrew Bible
\\c 1
\\p
\\v 1
\\w בְּ\u{2060}רֵאשִׁ֖ית|lemma=\"רֵאשִׁית\"\\w*
\\w בָּרָ֣א|lemma=\"בָּרָא\"\\w*
\\w אֱלֹהִ֑ים|lemma=\"אֱלֹהִים\"\\w*
\\w אֵ֥ת|lemma=\"אֵת\"\\w*
\\w הַ\u{2060}שָּׁמַ֖יִם|lemma=\"שָׁמַיִם\"\\w*׃
\\v 2
\\w וְ\u{2060}חֹ֖שֶׁךְ|lemma=\"חֹשֶׁךְ\"\\w*
\\w עַל|lemma=\"עַל\"\\w*־\\w פְּנֵ֣י|lemma=\"פָּנִים\"\\w*
\\w תְה֑וֹם|lemma=\"תְּהוֹם\"\\w*
\\f + \\ft Q \\+w תְּה֑וֹם|lemma=\"תְּהוֹם\"\\+w*\\f*
";

    /// Their ULT, abridged: \"the heavens\" renders two words, nested, and
    /// \"was\" is supplied.
    const ULT: &str = "\\id GEN EN_ULT
\\h Genesis
\\c 1
\\p
\\v 1 \\zaln-s |x-content=\"בְּ\u{2060}רֵאשִׁ֖ית\" x-occurrence=\"1\"\\*\\w In|x-occurrence=\"1\"\\w*
\\w the|x-occurrence=\"1\"\\w*
\\w beginning|x-occurrence=\"1\"\\w*\\zaln-e\\*
\\zaln-s |x-content=\"אֱלֹהִ֑ים\" x-occurrence=\"1\"\\*\\w God|x-occurrence=\"1\"\\w*\\zaln-e\\*
\\zaln-s |x-content=\"בָּרָ֣א\" x-occurrence=\"1\"\\*\\w created|x-occurrence=\"1\"\\w*\\zaln-e\\*
\\zaln-s |x-content=\"אֵ֥ת\" x-occurrence=\"1\"\\*\\zaln-s |x-content=\"הַ\u{2060}שָּׁמַ֖יִם\" x-occurrence=\"1\"\\*\\w the|x-occurrence=\"2\"\\w*
\\w heavens|x-occurrence=\"1\"\\w*\\zaln-e\\*\\zaln-e\\*.

\\ts\\*
\\v 2 \\zaln-s |x-content=\"וְ\u{2060}חֹ֖שֶׁךְ\" x-occurrence=\"1\"\\*\\w and|x-occurrence=\"1\"\\w*
\\w darkness|x-occurrence=\"1\"\\w*\\zaln-e\\* {\\zaln-s |x-content=\"עַל\" x-occurrence=\"1\"\\*\\w was|x-occurrence=\"1\"\\w*}
\\w over|x-occurrence=\"1\"\\w*\\zaln-e\\*
\\zaln-s |x-content=\"פְּנֵ֣י\" x-occurrence=\"1\"\\*\\w the|x-occurrence=\"1\"\\w*
\\w surface|x-occurrence=\"1\"\\w*
\\w of|x-occurrence=\"1\"\\w*\\zaln-e\\*
\\zaln-s |x-content=\"תְה֑וֹם\" x-occurrence=\"1\"\\*\\w the|x-occurrence=\"2\"\\w*
\\w deep|x-occurrence=\"1\"\\w*\\zaln-e\\*\\f + \\ft Or \\fqa the abyss\\fqa*\\f*.
";

    fn corpus(verses: &[&[&str]]) -> Vec<(CorpusWord, String)> {
        verses
            .iter()
            .enumerate()
            .flat_map(|(v, words)| {
                words
                    .iter()
                    .enumerate()
                    .map(move |(p, w)| ((pack_ref(1, 1, v as i64 + 1), p as u16), bare_letters(w)))
            })
            .collect()
    }

    fn import(ult: &str, uhb: &str, corpus: &[(CorpusWord, String)]) -> Vec<(i64, String)> {
        let mut summary = TranslationSummary::default();
        let (book, _) = prepare_book(ult, uhb).unwrap();
        // Through the prepared form, as the build reads it.
        let ult: String = book
            .verses
            .iter()
            .map(|v| write_pieces(&v.pieces) + "\n")
            .collect();
        let reread: Vec<_> = ult.lines().map(|l| parse_pieces(l).unwrap()).collect();
        let original: Vec<_> = book.verses.iter().map(|v| v.pieces.clone()).collect();
        assert_eq!(reread, original);
        build_book(&book, corpus, &mut summary)
    }

    #[test]
    fn writes_aligned_english() {
        let corpus = corpus(&[
            &["בְּרֵאשִׁית", "בָּרָא", "אֱלֹהִים", "אֵת", "הַשָּׁמַיִם"],
            &["וְחֹשֶׁךְ", "עַל", "פְּנֵי", "תְהוֹם"],
        ]);
        let verses = import(ULT, UHB, &corpus);
        assert_eq!(
            verses,
            [
                (
                    pack_ref(1, 1, 1),
                    "[In the beginning|0] [God|2] [created|1] [the heavens|4,3].".to_string()
                ),
                (
                    pack_ref(1, 1, 2),
                    "[and darkness|0] {[was|1]} [over|1] [the surface of|2] [the deep|3]."
                        .to_string()
                ),
            ]
        );
        // And the core reads back what was written.
        let spans = haqor_core::translation::parse(&verses[1].1, 1, 2).unwrap();
        let text: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "and darkness was over the surface of the deep.");
        assert!(spans[2].supplied, "{spans:?}");
    }

    /// The English numbering does not decide where a verse goes: the Hebrew
    /// its words render does. Here the UHB's verse 1 is the corpus's verse 2.
    #[test]
    fn files_verses_by_the_hebrew_they_render() {
        let corpus = corpus(&[
            &["שִׁיר", "הַמַּעֲלוֹת"],
            &["בְּרֵאשִׁית", "בָּרָא", "אֱלֹהִים", "אֵת", "הַשָּׁמַיִם"],
            &["וְחֹשֶׁךְ", "עַל", "פְּנֵי", "תְהוֹם"],
        ]);
        let verses = import(ULT, UHB, &corpus);
        assert_eq!(verses[0].0, pack_ref(1, 1, 2));
        assert_eq!(verses[1].0, pack_ref(1, 1, 3));
    }

    /// The corpus writes the qere where the UHB writes the ketiv: the word
    /// still pairs off with its neighbours agreeing.
    #[test]
    fn lines_up_across_differences() {
        let placed = line_up(
            &["א", "כתיב", "ב", "ג", "ד", "הללו", "יה", "ה"],
            &["א", "קרי", "ב", "ג", "ד", "הללויה", "ה"],
        );
        assert_eq!(
            placed,
            [
                Some(0),
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                Some(5),
                Some(5),
                Some(6)
            ]
        );
        // A verse one text lacks is stepped over.
        let extra: Vec<String> = (0..40).map(|n| format!("x{n}")).collect();
        let mut from = vec!["א", "ב", "ג"];
        from.extend(extra.iter().map(String::as_str));
        from.extend(["ד", "ה", "ו", "ז"]);
        let placed = line_up(&from, &["א", "ב", "ג", "ד", "ה", "ו", "ז"]);
        assert_eq!(placed[43..], [Some(3), Some(4), Some(5), Some(6)]);
        assert!(placed[3..43].iter().all(Option::is_none));
    }

    #[test]
    fn identity_ignores_mark_order_and_joiners() {
        // Patah then dagesh, and dagesh then patah.
        assert_eq!(
            identity("מִ\u{2060}מַּתָּנָה", true),
            identity("מִמּ\u{5b7}תָּנָה", true)
        );
        assert_ne!(identity("תֹ֨הוּ֙", true), identity("תֹהוּ", true));
        assert_eq!(identity("תֹ֨הוּ֙", false), identity("תֹהוּ", false));
    }

    #[test]
    fn reads_usfm_events() {
        let events = read_usfm(
            "\\c 3\n\\d \\w A|x\\w* \\v 1 \\qa Aleph\\qa*\\q1 \\w b|x\\w*\\f + \\ft note\\f*.",
        )
        .unwrap();
        assert_eq!(
            events,
            [
                Event::Chapter(3),
                Event::Text("\n".into()),
                Event::Title,
                Event::Text(" ".into()),
                Event::Word("A".into()),
                Event::Text(" ".into()),
                Event::Verse(1),
                Event::Text(" ".into()),
                Event::Text(" ".into()),
                Event::Text(" ".into()),
                Event::Word("b".into()),
                Event::Text(".".into()),
            ]
        );
    }

    /// A database storing its blobs compressed gets the translation
    /// compressed too, against a dictionary of its own beside the build's.
    #[test]
    fn compresses_when_the_database_does() {
        let verses: Vec<(i64, String)> = (0..2000)
            .map(|n| {
                (
                    n,
                    format!("[And God|{n}] [said|1], “[Let there be|2] [light|3].” {n}"),
                )
            })
            .collect();
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(SCHEMA).unwrap();
        db.execute_batch(
            "CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT);
             CREATE TABLE blob_dict(dict_id INTEGER PRIMARY KEY, data BLOB NOT NULL);
             INSERT INTO meta VALUES ('blob_codec', 'zstd');",
        )
        .unwrap();
        let hebrew: Vec<Vec<u8>> = (0..400)
            .map(|n| format!("בְּרֵאשִׁית בָּרָא אֱלֹהִים {n}").into_bytes())
            .collect();
        let other = zstd::dict::from_samples(&hebrew, 4096).unwrap();
        db.execute("INSERT INTO blob_dict VALUES (1, ?1)", [&other])
            .unwrap();

        let bytes = write_verses(&db, &verses).unwrap();
        let plain: usize = verses.iter().map(|(_, t)| t.len()).sum();
        assert!(bytes * 2 < plain, "{bytes} of {plain}");

        let dictionary: Vec<u8> = db
            .query_row("SELECT data FROM blob_dict WHERE dict_id = 2", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_ne!(dictionary_id(&dictionary), dictionary_id(&other));
        let mut decompressor = zstd::bulk::Decompressor::with_dictionary(&dictionary).unwrap();
        let stored: Vec<u8> = db
            .query_row(
                "SELECT text FROM translation_verse WHERE ref = 7",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let text = decompressor.decompress(&stored, 1024).unwrap();
        assert_eq!(String::from_utf8(text).unwrap(), verses[7].1);
    }

    /// The whole source against the built corpus.
    #[test]
    fn complete_source_imports() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let src_texts = root.join("src_texts");
        let database = root.join("data/haqor.db");
        if !source_dir(&src_texts).exists() || !database.exists() {
            eprintln!("skipping: prepared unfoldingWord texts or data/haqor.db unavailable");
            return;
        }
        // Into a copy: the test must not rewrite the shared database.
        let db = Connection::open_in_memory().unwrap();
        db.execute("ATTACH DATABASE ?1 AS src", [database.to_string_lossy()])
            .unwrap();
        db.execute_batch(
            "CREATE TABLE word(ref INTEGER, position INTEGER, surface_id INTEGER,
                               PRIMARY KEY(ref, position)) WITHOUT ROWID;
             INSERT INTO word SELECT ref, position, surface_id FROM src.word
               WHERE ref < (40 << 16);
             CREATE TABLE surface(surface_id INTEGER PRIMARY KEY, text TEXT);
             INSERT INTO surface SELECT surface_id, text FROM src.surface;
             DETACH DATABASE src;",
        )
        .unwrap();
        let summary = build_translation(&db, &src_texts).unwrap();
        eprintln!("{summary:?}");
        // Nearly every Hebrew verse has its English, and nearly every English
        // word renders a word of the corpus.
        assert!(summary.verses > 23_150, "{}", summary.verses);
        assert!(summary.linked * 1000 > summary.words * 999, "{summary:?}");
        assert!(summary.dropped < 5, "{}", summary.dropped);

        let text: Vec<u8> = db
            .query_row(
                "SELECT text FROM translation_verse WHERE ref = ?1",
                [pack_ref(1, 1, 1)],
                |row| row.get(0),
            )
            .unwrap();
        let text = String::from_utf8(text).unwrap();
        assert!(
            text.starts_with("[In the beginning|0] [God|2] [created|1]"),
            "{text}"
        );
        // Every verse parses.
        let mut stmt = db
            .prepare("SELECT ref, text FROM translation_verse")
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
            })
            .unwrap();
        for row in rows {
            let (reference, text) = row.unwrap();
            let text = String::from_utf8(text).unwrap();
            let (chapter, verse) = (((reference >> 8) & 0xff) as u8, (reference & 0xff) as u8);
            assert!(
                haqor_core::translation::parse(&text, chapter, verse).is_some(),
                "{reference}: {text}"
            );
        }
    }
}
