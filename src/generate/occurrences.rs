//! Prototype: reverse-parse the OT text into morphological analyses and measure
//! coverage.
//!
//! Walks every Old Testament word token in the `bible` table, runs the reverse
//! morphology parser ([`crate::morphology::parse_word`]) on each, and tallies
//! how many tokens the generator can account for. This is the exploratory step
//! toward a SEDRA-equivalent `occurrences` table for the Hebrew Bible: before
//! committing to a schema we want to know what fraction of real OT words the
//! generate-and-test engine actually matches, and how ambiguous the matches
//! are.
//!
//! Nothing is written to disk — this only prints statistics.

use std::path::Path;

use anyhow::{Context, Result};
use rayon::prelude::*;
use rusqlite::Connection;

use crate::morphology::parse_word;

/// OT books are 1..=39 in Haqor numbering; NT starts at 40.
const NT_FIRST_BOOK: u8 = 40;

/// Hebrew maqaf joins two orthographic words; we split on it so each gets parsed
/// independently.
const MAQAF: char = '\u{05BE}';

#[derive(Default)]
struct Stats {
    verses: usize,
    tokens: usize,
    parsed: usize,
    attested: usize,
    unambiguous: usize,
    /// candidate-count histogram bucket index: 0,1,2,3,4,5+ (index 5).
    ambiguity: [usize; 6],
}

impl Stats {
    fn merge(mut self, other: Stats) -> Stats {
        self.verses += other.verses;
        self.tokens += other.tokens;
        self.parsed += other.parsed;
        self.attested += other.attested;
        self.unambiguous += other.unambiguous;
        for i in 0..self.ambiguity.len() {
            self.ambiguity[i] += other.ambiguity[i];
        }
        self
    }
}

/// Split a verse's `words` field into individual orthographic word tokens.
/// Whitespace and maqaf are separators; everything else is left for the parser,
/// which ignores cantillation and punctuation on its own.
fn tokenize(words: &str) -> impl Iterator<Item = &str> {
    words
        .split(|c: char| c.is_whitespace() || c == MAQAF)
        .filter(|t| !t.is_empty())
}

/// True if a token contains at least one Hebrew consonant (so it is worth
/// parsing — bare punctuation/maqqef remnants are skipped).
fn has_hebrew_letter(token: &str) -> bool {
    token
        .chars()
        .any(|c| (0x05D0..=0x05EA).contains(&(c as u32)))
}

/// Reverse-parse OT tokens and print coverage statistics.
///
/// `book_filter` limits to a single OT book (Haqor numbering, 1..=39); `None`
/// scans all of them. `limit` caps the number of verses processed (0 = no cap),
/// which keeps quick sampling runs fast.
pub fn parse_ot_coverage(bible_db: &Path, book_filter: Option<u8>, limit: usize) -> Result<()> {
    let db =
        Connection::open(bible_db).with_context(|| format!("opening {}", bible_db.display()))?;

    let mut sql =
        format!("SELECT book, chapter, verse, words FROM bible WHERE book < {NT_FIRST_BOOK}");
    if let Some(b) = book_filter {
        sql.push_str(&format!(" AND book = {b}"));
    }
    sql.push_str(" ORDER BY book, chapter, verse");

    let mut stmt = db.prepare(&sql)?;
    let verses: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(3))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let verses = match limit {
        0 => &verses[..],
        n => &verses[..n.min(verses.len())],
    };

    // Flatten to the word tokens worth parsing, then analyse them in parallel.
    // `parse_word` is pure, so each token is independent.
    let tokens: Vec<&str> = verses
        .iter()
        .flat_map(|words| tokenize(words))
        .filter(|t| has_hebrew_letter(t))
        .collect();

    let stats = tokens
        .par_iter()
        .map(|token| {
            let matches = parse_word(token);
            let n = matches.len();
            let mut s = Stats {
                tokens: 1,
                ..Default::default()
            };
            s.ambiguity[n.min(5)] = 1;
            if n > 0 {
                s.parsed = 1;
                if n == 1 {
                    s.unambiguous = 1;
                }
                if matches.iter().any(|m| m.attested) {
                    s.attested = 1;
                }
            }
            s
        })
        .reduce(Stats::default, Stats::merge);

    let mut stats = stats;
    stats.verses = verses.len();
    print_report(&stats);
    Ok(())
}

fn pct(n: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        100.0 * n as f64 / total as f64
    }
}

fn print_report(s: &Stats) {
    let t = s.tokens;
    println!("OT reverse-parse coverage");
    println!("  verses scanned:      {}", s.verses);
    println!("  word tokens:         {}", t);
    println!();
    println!(
        "  parsed (>=1 match):  {:>7}  ({:.1}%)",
        s.parsed,
        pct(s.parsed, t)
    );
    println!(
        "    of which attested: {:>7}  ({:.1}% of tokens)",
        s.attested,
        pct(s.attested, t)
    );
    println!(
        "  unparsed:            {:>7}  ({:.1}%)",
        t - s.parsed,
        pct(t - s.parsed, t)
    );
    println!();
    println!("  ambiguity (candidate analyses per token):");
    let labels = ["0", "1", "2", "3", "4", "5+"];
    for (i, label) in labels.iter().enumerate() {
        println!(
            "    {:>2}: {:>7}  ({:.1}%)",
            label,
            s.ambiguity[i],
            pct(s.ambiguity[i], t)
        );
    }
    println!();
    println!(
        "  unambiguous (exactly 1): {} ({:.1}% of parsed)",
        s.unambiguous,
        pct(s.unambiguous, s.parsed)
    );
}
