//! UXLC (Unicode/XML Leningrad Codex) OT source parser.
//!
//! Ported from `bm_tools.uxlc.bible`. Each OT book is one XML file under
//! `src_texts/UXLC/Books/`. The 39 books are listed in canonical (Haqor)
//! order together with their UXLC filename stem.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use quick_xml::Reader;
use quick_xml::events::Event;

/// Canonical OT book order → UXLC filename stem. Book number is the 1-based
/// index into this list.
pub const OT_BOOKS: &[&str] = &[
    "Genesis",
    "Exodus",
    "Leviticus",
    "Numbers",
    "Deuteronomy",
    "Joshua",
    "Judges",
    "Samuel_1",
    "Samuel_2",
    "Kings_1",
    "Kings_2",
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
    "Song_of_Songs",
    "Ruth",
    "Lamentations",
    "Ecclesiastes",
    "Esther",
    "Daniel",
    "Ezra",
    "Nehemiah",
    "Chronicles_1",
    "Chronicles_2",
];

/// Hebrew maqaf (U+05BE) — the word-joining hyphen.
const MAQAF: char = '\u{05BE}';

/// Strip a stray *word-internal* maqaf, one with a Hebrew letter still to come.
///
/// UXLC joins two words by trailing a maqaf on each `<w>`, so a maqaf is never
/// legitimately word-internal. The Leningrad source carries exactly one such
/// artifact: Psalm 67:2 spells יָאֵר as `יָאֵ֥־<x>c</x>ר`, the maqaf stranded on
/// the fragment before an inline editorial note. With the note dropped the
/// maqaf lands mid-word, and any downstream maqaf-splitting tokeniser (the
/// surface generator, and ../haqor's word-info panel) then breaks the word into
/// the stub יָאֵ and a bogus one-letter ר. Removing the internal maqaf restores
/// the WLC reading יָאֵר. A trailing maqaf (כָּל־, the normal word-joiner) is
/// kept untouched.
fn strip_internal_maqaf(word: &str) -> String {
    if !word.contains(MAQAF) {
        return word.to_string();
    }
    let chars: Vec<char> = word.chars().collect();
    chars
        .iter()
        .enumerate()
        .filter(|&(i, &c)| {
            !(c == MAQAF
                && chars[i + 1..]
                    .iter()
                    .any(|&d| (0x05D0..=0x05EA).contains(&(d as u32))))
        })
        .map(|(_, &c)| c)
        .collect()
}

/// A handful of UXLC `<w>` elements erroneously fuse two orthographic words
/// into one token with no separator (neither space nor maqaf) — e.g. Num 12:9
/// "וַיִּחַר אַף" is stored as the single glued `<w>וַיִּחַרְאַף`. The morphhb WLC
/// keeps these as two `<w>`, and so should we: a glued token is both
/// un-analysable by the morphology generator and visibly wrong in the app's
/// verse text. Keyed on the accent-stripped form (so the rule is robust to
/// ta'amim/meteg placement); the value is the restored space-separated words.
/// Deliberately precise — the legitimate one-word הַלְלוּיָהּ (shureq) does not
/// match the erroneous sheva-pointed הַלְלְויָהּ here.
fn split_glued_word(word: &str) -> Option<&'static str> {
    // (glued form, restored split). The glued keys are written in the readable
    // vowel-then-dagesh order; `glue_key` makes the comparison robust to UXLC's
    // dagesh-then-vowel mark ordering (and stray accents/meteg).
    const FIXES: &[(&str, &str)] = &[
        // וַיִּחַר אַף "and (the) anger burned" (Num 12:9; 2Sam 6:7 drops the sheva)
        ("וַיִּחַרְאַף", "וַיִּחַר אַף"),
        ("וַיִּחַראַף", "וַיִּחַר אַף"),
        // אֲבִי עַד "Everlasting Father" (Isa 9:5)
        ("אֲבִיעַד", "אֲבִי עַד"),
        // הַלְלוּ יָהּ "praise Yah" (Ps 106:1)
        ("הַלְלְויָהּ", "הַלְלוּ יָהּ"),
        // לְמַ בָּרִאשׁוֹנָה "because at the first" (1Chr 15:13)
        ("לְמַבָּרִאשׁוֹנָה", "לְמַ בָּרִאשׁוֹנָה"),
    ];
    let key = glue_key(word);
    FIXES
        .iter()
        .find(|(glued, _)| glue_key(glued) == key)
        .map(|(_, split)| *split)
}

/// Order-insensitive match key: drop cantillation accents and meteg, then sort
/// the combining marks within each consonant cluster, so a vowel-then-dagesh
/// spelling compares equal to UXLC's dagesh-then-vowel one.
fn glue_key(word: &str) -> Vec<char> {
    let mut out: Vec<char> = Vec::new();
    let mut cluster: Vec<char> = Vec::new();
    let flush = |cluster: &mut Vec<char>, out: &mut Vec<char>| {
        cluster.sort_unstable();
        out.append(cluster);
    };
    for c in word.chars() {
        let cp = c as u32;
        if matches!(cp, 0x0591..=0x05AF | 0x05BD) {
            continue; // cantillation accent / meteg
        }
        if (0x05D0..=0x05EA).contains(&cp) {
            flush(&mut cluster, &mut out); // base letter → close prior cluster
            out.push(c);
        } else {
            cluster.push(c); // point (vowel / dagesh / sin-shin dot)
        }
    }
    flush(&mut cluster, &mut out);
    out
}

/// A single verse: book, chapter, verse, and the space-joined words.
pub struct Verse {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
    pub words: String,
}

/// Parse every OT book and return verses in book/chapter/verse order.
pub fn parse_all(books_dir: &Path) -> Result<Vec<Verse>> {
    let mut verses = Vec::new();
    for (idx, stem) in OT_BOOKS.iter().enumerate() {
        let book = (idx + 1) as u8;
        let path = books_dir.join(format!("{stem}.xml"));
        parse_book(&path, book, &mut verses)
            .with_context(|| format!("parsing OT book {stem} ({})", path.display()))?;
    }
    Ok(verses)
}

/// Parse one UXLC book file, appending its verses to `out`.
fn parse_book(path: &Path, book: u8, out: &mut Vec<Verse>) -> Result<()> {
    let mut reader = Reader::from_file(path)?;

    // chapter -> verse -> words. BTreeMap keeps numeric order regardless of the
    // order elements appear in the source (mirrors the Python sort()).
    let mut chapters: BTreeMap<u8, BTreeMap<u8, Vec<String>>> = BTreeMap::new();

    let mut chapter: u8 = 0;
    let mut verse: u8 = 0;
    let mut in_word = false;
    let mut word = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => match e.name().as_ref() {
                b"c" => chapter = attr_n(&e, &reader)?,
                b"v" => {
                    verse = attr_n(&e, &reader)?;
                    chapters
                        .entry(chapter)
                        .or_default()
                        .entry(verse)
                        .or_default();
                }
                b"w" => {
                    in_word = true;
                    word.clear();
                }
                _ => {}
            },
            Event::End(e) => {
                if e.name().as_ref() == b"w" {
                    in_word = false;
                    let assembled = strip_internal_maqaf(&std::mem::take(&mut word));
                    let assembled =
                        split_glued_word(&assembled).map_or(assembled, str::to_string);
                    chapters
                        .entry(chapter)
                        .or_default()
                        .entry(verse)
                        .or_default()
                        .push(assembled);
                }
            }
            Event::Text(t) if in_word => {
                let text = t.unescape()?;
                // Keep only Hebrew fragments, dropping nested <x> note text.
                // Mirrors the Python `e > "z"` filter: UTF-8 byte ordering
                // matches code-point ordering for this comparison.
                if text.as_ref() > "z" {
                    word.push_str(text.as_ref());
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    for (chapter, vmap) in chapters {
        for (verse, words) in vmap {
            out.push(Verse {
                book,
                chapter,
                verse,
                words: words.join(" "),
            });
        }
    }

    Ok(())
}

/// Read the `n` attribute of an element as a number.
fn attr_n(
    e: &quick_xml::events::BytesStart,
    reader: &Reader<std::io::BufReader<std::fs::File>>,
) -> Result<u8> {
    let attr = e
        .try_get_attribute("n")?
        .context("element missing `n` attribute")?;
    let value = attr.decode_and_unescape_value(reader.decoder())?;
    Ok(value.parse()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_only_word_internal_maqaf() {
        // Ps 67:2 יָאֵ֥־ר → the stray internal maqaf is removed, recovering יָאֵר.
        assert_eq!(strip_internal_maqaf("יָאֵ֥־ר"), "יָאֵ֥ר");
        // A trailing maqaf (the normal word-joiner, e.g. כָּל־) is left untouched.
        assert_eq!(strip_internal_maqaf("כָּל־"), "כָּל־");
        // A word with no maqaf is returned verbatim.
        assert_eq!(strip_internal_maqaf("שָׁמַיִם"), "שָׁמַיִם");
    }
}
