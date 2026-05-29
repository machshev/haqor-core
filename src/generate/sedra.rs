//! SEDRA NT source parser.
//!
//! Ported from `bm_tools.sedra.bible` / `bm_tools.sedra.db`. The NT text is
//! assembled from two checked-in source files:
//!   - `tblWords.txt`  — SEDRA3 words table; `keyWord` → `strVocalised`.
//!   - `BFBS.cache`     — `book,chapter,verse,wid wid ...` per verse, where
//!                        `book` is the SEDRA book id (Matthew = 52).
//!
//! Each word's vocalised transliteration is converted to Hebrew letters.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::generate::uxlc::Verse;
use crate::transliterate;

/// SEDRA book ids start at 52 (Matthew); Haqor book numbers start at 40.
const SEDRA_BOOK_OFFSET: u8 = 12;

/// Load `tblWords.txt` into a `keyWord` → `strVocalised` map.
fn load_words(path: &Path) -> Result<HashMap<u64, String>> {
    let mut reader = csv::Reader::from_path(path)
        .with_context(|| format!("opening {}", path.display()))?;

    let headers = reader.headers()?.clone();
    let key_idx = column(&headers, "keyWord")?;
    let voc_idx = column(&headers, "strVocalised")?;

    let mut words = HashMap::new();
    for record in reader.records() {
        let record = record?;
        let key: u64 = record[key_idx].parse()?;
        words.insert(key, record[voc_idx].to_owned());
    }
    Ok(words)
}

fn column(headers: &csv::StringRecord, name: &str) -> Result<usize> {
    headers
        .iter()
        .position(|h| h == name)
        .with_context(|| format!("tblWords.txt missing `{name}` column"))
}

/// Parse the NT, returning verses with Hebrew-transliterated words.
pub fn parse_all(sedra_dir: &Path) -> Result<Vec<Verse>> {
    let words = load_words(&sedra_dir.join("tblWords.txt"))?;
    let cache = std::fs::read_to_string(sedra_dir.join("BFBS.cache"))
        .with_context(|| format!("reading BFBS.cache in {}", sedra_dir.display()))?;

    let mut verses = Vec::new();
    for line in cache.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut fields = line.splitn(4, ',');
        let sedra_book: u8 = fields.next().context("cache line missing book")?.parse()?;
        let chapter: u8 = fields.next().context("cache line missing chapter")?.parse()?;
        let verse: u8 = fields.next().context("cache line missing verse")?.parse()?;
        let ids = fields.next().context("cache line missing word ids")?;

        let mut hebrew_words = Vec::new();
        for id in ids.split(' ') {
            let key: u64 = id.parse()?;
            let vocalised = words
                .get(&key)
                .with_context(|| format!("word id {key} not found in tblWords.txt"))?;
            hebrew_words.push(transliterate::sedra_to_hebrew(vocalised));
        }

        verses.push(Verse {
            book: sedra_book - SEDRA_BOOK_OFFSET,
            chapter,
            verse,
            words: hebrew_words.join(" "),
        });
    }

    verses.sort_by_key(|v| (v.book, v.chapter, v.verse));
    Ok(verses)
}
