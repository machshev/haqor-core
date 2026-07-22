//! Import and align STEP Bible's occurrence-level TAHOT translations.
//!
//! The BDB/Strong's sources are dictionaries: useful for word information, but
//! too broad and fragmentary for a flowing interlinear. TAHOT instead supplies
//! a context-sensitive translation for every Hebrew token. Its Leningrad text
//! still differs slightly from Haqor's UXLC stream, so the same weighted verse
//! alignment used for OSHB morphology is applied here too.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use haqor_core::normalize_surface;

use super::hebrew_db::{Occurrence, book_number};
use super::oshb::align_verse;

pub(crate) const SOURCE_ID: &str = "stepbible-tahot";
pub(crate) const SOURCE_NAME: &str = "STEP Bible TAHOT";
pub(crate) const SOURCE_URL: &str = "https://github.com/STEPBible/STEPBible-Data";
pub(crate) const SOURCE_LICENSE: &str = "CC BY 4.0";

type VerseRef = (u8, u8, u8);

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceGloss {
    word: String,
    gloss: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AlignedGloss {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
    pub position: usize,
    pub gloss: String,
    pub exact_surface: bool,
}

/// Directory created by `scripts/fetch-stepbible-data.sh` inside `src_texts`.
pub fn source_dir(src_texts: &Path) -> PathBuf {
    src_texts
        .join("STEPBible-Data")
        .join("Translators Amalgamated OT+NT")
}

/// Parse TAHOT's English reference, preferring the parenthesised Hebrew
/// versification when it is present (`Mal.4.6(3.24)` -> Malachi 3:24).
fn parse_reference(field: &str) -> Option<VerseRef> {
    let reference = field.split_once('#')?.0;
    let (english, hebrew) = match reference.split_once('(') {
        Some((english, hebrew)) => (english, Some(hebrew.strip_suffix(')')?)),
        None => (reference, None),
    };
    let book_name = english.split_once('.')?.0;
    let book = book_number(book_name)?;
    let selected = hebrew.unwrap_or_else(|| english.split_once('.').unwrap().1);
    let mut parts = selected.split('.');
    let chapter = parts.next()?.parse().ok()?;
    let verse = parts.next()?.parse().ok()?;
    (parts.next().is_none()).then_some((book, chapter, verse))
}

/// Convert TAHOT's translation markup into compact reader text. Slash-separated
/// morphemes become ordinary spaces, square-bracketed implied English is kept,
/// and angle-bracketed words which TAHOT says should be omitted are removed.
fn clean_translation(raw: &str) -> String {
    let mut text = String::with_capacity(raw.len());
    let mut omitted = None::<String>;
    for ch in raw.chars() {
        match ch {
            '<' => omitted = Some(String::new()),
            '>' => {
                if omitted.as_deref() == Some("obj.") {
                    text.push('←');
                }
                omitted = None;
            }
            _ if omitted.is_some() => omitted.as_mut().unwrap().push(ch),
            '[' | ']' => {}
            '/' | '_' => text.push(' '),
            _ => text.push(ch),
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_row(line: &str) -> Option<(VerseRef, SourceGloss)> {
    let mut fields = line.trim_end_matches('\r').split('\t');
    let reference = parse_reference(fields.next()?)?;
    let hebrew = fields.next()?;
    let _transliteration = fields.next()?;
    let gloss = clean_translation(fields.next()?);

    // Backslashes introduce punctuation/section markers. Slashes divide the
    // word's prefixes, lexeme and suffixes, all of which belong to one UXLC
    // token and therefore need to be joined before normalisation.
    let word = normalize_surface(&hebrew.split('\\').next().unwrap_or(hebrew).replace('/', ""));
    (!word.is_empty()).then_some((reference, SourceGloss { word, gloss }))
}

fn read_glosses(dir: &Path) -> Result<HashMap<VerseRef, Vec<SourceGloss>>> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("reading STEP Bible TAHOT directory {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("TAHOT ") && name.ends_with(".txt"))
        })
        .collect();
    paths.sort();

    let mut verses = HashMap::<VerseRef, Vec<SourceGloss>>::new();
    for path in paths {
        let file = File::open(&path).with_context(|| format!("opening {}", path.display()))?;
        for line in BufReader::new(file).lines() {
            if let Some((reference, gloss)) = parse_row(&line?) {
                verses.entry(reference).or_default().push(gloss);
            }
        }
    }
    Ok(verses)
}

pub(crate) fn align_glosses(
    dir: &Path,
    surfaces: &[String],
    occurrences: &[Occurrence],
) -> Result<Vec<AlignedGloss>> {
    let source = read_glosses(dir)?;
    let mut current = HashMap::<VerseRef, Vec<(usize, usize)>>::new();
    let mut positions = HashMap::<VerseRef, usize>::new();
    for occurrence in occurrences {
        let reference = (occurrence.book, occurrence.chapter, occurrence.verse);
        let position = positions.entry(reference).or_default();
        current
            .entry(reference)
            .or_default()
            .push((*position, occurrence.surface_id));
        *position += 1;
    }

    let mut out = Vec::new();
    for (reference, words) in current {
        let Some(source_words) = source.get(&reference) else {
            continue;
        };
        let texts: Vec<String> = words
            .iter()
            .map(|(_, surface_id)| surfaces[*surface_id].clone())
            .collect();
        let source_texts: Vec<String> = source_words
            .iter()
            .map(|token| token.word.clone())
            .collect();
        for (current_index, source_index, exact_surface) in align_verse(&texts, &source_texts) {
            let (position, _) = words[current_index];
            out.push(AlignedGloss {
                book: reference.0,
                chapter: reference.1,
                verse: reference.2,
                position,
                gloss: source_words[source_index].gloss.clone(),
                exact_surface,
            });
        }
    }
    out.sort_by_key(|gloss| (gloss.book, gloss.chapter, gloss.verse, gloss.position));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hebrew_versification_wins_over_english_reference() {
        assert_eq!(parse_reference("Gen.1.2#03=L"), Some((1, 1, 2)));
        assert_eq!(parse_reference("Mal.4.6(3.24)#15=L"), Some((26, 3, 24)));
    }

    #[test]
    fn translation_markup_becomes_flowing_reader_text() {
        assert_eq!(
            clean_translation("and/ [the] spirit of"),
            "and the spirit of"
        );
        assert_eq!(clean_translation("<it> was"), "was");
        assert_eq!(clean_translation("<obj.>"), "←");
        assert_eq!(clean_translation("and/ <obj.>"), "and ←");
    }

    #[test]
    fn parses_tahot_word_row() {
        let line = "Gen.1.2#09=L\tוְ/ר֣וּחַ\tve./Ru.ach\tand/ [the] spirit of\tH9002/{H7307G}";
        assert_eq!(
            parse_row(line),
            Some((
                (1, 1, 2),
                SourceGloss {
                    word: "וְרוּחַ".to_string(),
                    gloss: "and the spirit of".to_string(),
                },
            ))
        );
    }

    #[test]
    fn complete_tahot_source_aligns_to_bundled_ot() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dir = source_dir(&root.join("src_texts"));
        let database = root.join("data/hebrew.db");
        if !dir.exists() || !database.exists() {
            eprintln!("skipping: fetched TAHOT source or data/hebrew.db unavailable");
            return;
        }

        let db = rusqlite::Connection::open(database).unwrap();
        let surfaces = db
            .prepare("SELECT text FROM surface ORDER BY surface_id")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        let occurrences = db
            .prepare(
                "SELECT surface_id, book, chapter, verse FROM verse_word \
                 ORDER BY book, chapter, verse, position",
            )
            .unwrap()
            .query_map([], |row| {
                Ok(Occurrence {
                    surface_id: row.get::<_, i64>(0)? as usize,
                    book: row.get(1)?,
                    chapter: row.get(2)?,
                    verse: row.get(3)?,
                })
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();

        let aligned = align_glosses(&dir, &surfaces, &occurrences).unwrap();
        let nonempty = aligned.iter().filter(|row| !row.gloss.is_empty()).count();
        eprintln!(
            "aligned {} of {} OT tokens ({} nonempty)",
            aligned.len(),
            occurrences.len(),
            nonempty
        );
        assert!(aligned.len() * 100 > occurrences.len() * 97);
        assert!(nonempty * 100 > occurrences.len() * 95);
    }
}
