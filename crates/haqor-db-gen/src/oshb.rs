//! Import and align the Open Scriptures Hebrew Bible morphology.
//!
//! Haqor's displayed OT text comes from UXLC while OSHB follows the WLC.  The
//! two are extremely close, but their token streams are not identical (most
//! notably around ketiv/qere).  We therefore align each verse by consonantal
//! word identity instead of assuming that source positions are interchangeable.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use haqor_core::normalize_surface;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use super::hebrew_db::{Occurrence, book_number};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceToken {
    pub word: String,
    pub lemma: String,
    pub morph: String,
    pub id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PrimaryAnalysis {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
    pub position: usize,
    pub surface_id: usize,
    pub source: SourceToken,
    pub exact_surface: bool,
}

type VerseRef = (u8, u8, u8);

fn attr(
    e: &BytesStart<'_>,
    reader: &Reader<std::io::BufReader<std::fs::File>>,
    key: &[u8],
) -> Result<String> {
    Ok(e.try_get_attribute(key)?
        .map(|a| a.decode_and_unescape_value(reader.decoder()))
        .transpose()?
        .map(|s| s.into_owned())
        .unwrap_or_default())
}

fn parse_verse_ref(value: &str) -> Option<VerseRef> {
    let mut parts = value.split('.');
    let book = book_number(parts.next()?)?;
    let chapter = parts.next()?.parse().ok()?;
    let verse = parts.next()?.parse().ok()?;
    Some((book, chapter, verse))
}

fn parse_book(path: &Path, verses: &mut HashMap<VerseRef, Vec<SourceToken>>) -> Result<()> {
    let mut reader = Reader::from_file(path)?;
    let mut buf = Vec::new();
    let mut verse_ref = None;
    let mut note_depth = 0usize;
    let mut current: Option<SourceToken> = None;

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if e.name().as_ref() == b"verse" => {
                verse_ref = parse_verse_ref(&attr(&e, &reader, b"osisID")?);
            }
            Event::Start(e) if e.name().as_ref() == b"note" => note_depth += 1,
            Event::End(e) if e.name().as_ref() == b"note" => {
                note_depth = note_depth.saturating_sub(1);
            }
            Event::Start(e) if e.name().as_ref() == b"w" => {
                // UXLC omits its <k>/<q> apparatus from the displayed verse.
                // Ignore both OSHB's top-level ketiv and the qere nested in a
                // variant note; ordinary explanatory notes contain no words.
                let skip_word = note_depth > 0 || attr(&e, &reader, b"type")? == "x-ketiv";
                current = (!skip_word).then(|| SourceToken {
                    word: String::new(),
                    lemma: attr(&e, &reader, b"lemma").unwrap_or_default(),
                    morph: attr(&e, &reader, b"morph").unwrap_or_default(),
                    id: attr(&e, &reader, b"id").unwrap_or_default(),
                });
            }
            Event::Text(t) if current.is_some() => {
                let fragment = t.unescape()?;
                if fragment.as_ref() > "z" {
                    current
                        .as_mut()
                        .expect("checked above")
                        .word
                        .push_str(&fragment);
                }
            }
            Event::End(e) if e.name().as_ref() == b"w" => {
                if let (Some(reference), Some(token)) = (verse_ref, current.take())
                    && !token.morph.is_empty()
                    && !normalize_surface(&token.word).is_empty()
                {
                    verses.entry(reference).or_default().push(token);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

pub(crate) fn read_tokens(morphhb_dir: &Path) -> Result<HashMap<VerseRef, Vec<SourceToken>>> {
    let wlc = morphhb_dir.join("wlc");
    let mut paths: Vec<_> = std::fs::read_dir(&wlc)
        .with_context(|| format!("reading {}", wlc.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "xml"))
        .collect();
    paths.sort();

    let mut verses = HashMap::new();
    for path in paths {
        parse_book(&path, &mut verses)
            .with_context(|| format!("parsing OSHB morphology from {}", path.display()))?;
    }
    Ok(verses)
}

fn consonants(value: &str) -> String {
    value
        .chars()
        .filter(|c| matches!(*c as u32, 0x05D0..=0x05EA))
        .collect()
}

/// Weighted LCS alignment. Exact pointed matches outrank consonantal matches;
/// both outrank gaps. Source/current substitutions are never accepted as an
/// authoritative mapping.
fn align_verse(current: &[String], source: &[SourceToken]) -> Vec<(usize, usize, bool)> {
    let current_norm: Vec<String> = current.iter().map(|s| normalize_surface(s)).collect();
    let source_norm: Vec<String> = source.iter().map(|s| normalize_surface(&s.word)).collect();
    let current_cons: Vec<String> = current_norm.iter().map(|s| consonants(s)).collect();
    let source_cons: Vec<String> = source_norm.iter().map(|s| consonants(s)).collect();
    let mut score = vec![vec![0u32; source.len() + 1]; current.len() + 1];

    for i in 0..current.len() {
        for j in 0..source.len() {
            let weight = if current_norm[i] == source_norm[j] {
                Some(2)
            } else if !current_cons[i].is_empty() && current_cons[i] == source_cons[j] {
                Some(1)
            } else {
                None
            };
            score[i + 1][j + 1] = score[i][j + 1].max(score[i + 1][j]);
            if let Some(weight) = weight {
                score[i + 1][j + 1] = score[i + 1][j + 1].max(score[i][j] + weight);
            }
        }
    }

    let mut out = Vec::new();
    let (mut i, mut j) = (current.len(), source.len());
    while i > 0 && j > 0 {
        let exact = current_norm[i - 1] == source_norm[j - 1];
        let weight = if exact {
            Some(2)
        } else if !current_cons[i - 1].is_empty() && current_cons[i - 1] == source_cons[j - 1] {
            Some(1)
        } else {
            None
        };
        if weight.is_some_and(|weight| score[i][j] == score[i - 1][j - 1] + weight) {
            out.push((i - 1, j - 1, exact));
            i -= 1;
            j -= 1;
        } else if score[i - 1][j] >= score[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    out.reverse();
    out
}

pub(crate) fn align_primary(
    morphhb_dir: &Path,
    surfaces: &[String],
    occurrences: &[Occurrence],
) -> Result<Vec<PrimaryAnalysis>> {
    let source = read_tokens(morphhb_dir)?;
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
        for (current_index, source_index, exact_surface) in align_verse(&texts, source_words) {
            let (position, surface_id) = words[current_index];
            out.push(PrimaryAnalysis {
                book: reference.0,
                chapter: reference.1,
                verse: reference.2,
                position,
                surface_id,
                source: source_words[source_index].clone(),
                exact_surface,
            });
        }
    }
    out.sort_by_key(|a| (a.book, a.chapter, a.verse, a.position));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(word: &str) -> SourceToken {
        SourceToken {
            word: word.to_string(),
            lemma: String::new(),
            morph: String::new(),
            id: String::new(),
        }
    }

    #[test]
    fn verse_alignment_skips_source_only_words_without_shifting() {
        let current = vec!["אָב".to_string(), "בֵּן".to_string(), "אֵם".to_string()];
        let source = vec![token("אָב"), token("קֶרֶן"), token("בֵּן"), token("אֵם")];
        assert_eq!(
            align_verse(&current, &source),
            vec![(0, 0, true), (1, 2, true), (2, 3, true)]
        );
    }

    #[test]
    fn verse_alignment_accepts_pointing_drift_on_same_consonants() {
        let current = vec!["יַהְוֶה".to_string()];
        let source = vec![token("יְהוָה")];
        assert_eq!(align_verse(&current, &source), vec![(0, 0, false)]);
    }
}
