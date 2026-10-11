//! Import James Murdock's translation of the Peshitta New Testament (1852,
//! public domain), the English beside the Syriac.
//!
//! Murdock translated the Peshitta as printed in his day, all 27 books (the
//! five the Peshitta lacks from the later Syriac versions), and numbers its
//! verses as Haqor's Peshitta does almost everywhere. The text comes from
//! the CrossWire SWORD module, which `scripts/fetch-murdock.sh` exports with
//! `mod2imp`; [`prepare`] reduces that export to `src_texts/Murdock`'s
//! `murdock.tsv`, which is what is committed.
//!
//! Preparing it leaves out Murdock's footnotes and the colophons closing the
//! Gospels and Acts, and keeps his italics, which mark words supplied for the
//! sense, as the stored form's braces. A verse set entirely in italics is one
//! some editions lack, not one supplied, and is kept plain. The module also
//! runs a few verses together, puts some under the wrong number, and follows
//! the KJV in a few places the Peshitta differs from it: [`FIXES`] puts each
//! right, checked against the Peshitta's own words.
//! A few words whose sense has moved since are given the ULT's for the same
//! thing ([`MODERN_WORDS`]).
//!
//! The English renders no particular Syriac words: a verse is one span of
//! text, its supplied words apart.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::runtime_db::pack_ref;

/// `src_texts/Murdock`.
pub fn source_dir(src_texts: &Path) -> PathBuf {
    src_texts.join("Murdock")
}

/// The prepared file: a line a verse, its book (40 Matthew … 66 Revelation),
/// its chapter and verse (`1:1`), and its English in the stored form.
const PREPARED: &str = "murdock.tsv";

/// The module's names for the books, in order from Matthew.
const BOOK_NAMES: [&str; 27] = [
    "Matthew",
    "Mark",
    "Luke",
    "John",
    "Acts",
    "Romans",
    "I Corinthians",
    "II Corinthians",
    "Galatians",
    "Ephesians",
    "Philippians",
    "Colossians",
    "I Thessalonians",
    "II Thessalonians",
    "I Timothy",
    "II Timothy",
    "Titus",
    "Philemon",
    "Hebrews",
    "James",
    "I Peter",
    "II Peter",
    "I John",
    "II John",
    "III John",
    "Jude",
    "Revelation of John",
];

/// A verse: Haqor's book, chapter and verse.
type Verse = (u8, u8, u8);

/// A correction to the module's text, applied in [`FIXES`]' order. Each
/// fails the import if the text it expects is not there, so a changed module
/// is looked at again rather than mangled.
enum Fix {
    /// Move the text of a verse from `at` onwards into another verse, in
    /// place of whatever it had; `at` itself is kept, or dropped when it is
    /// the module's stray verse label.
    Split {
        verse: Verse,
        at: &'static str,
        keep: bool,
        into: Verse,
    },
    /// Give each verse's text another number, all at once.
    Renumber(&'static [(Verse, Verse)]),
    /// Leave a verse out.
    Drop(Verse),
    /// Replace text in a verse.
    Replace(Verse, &'static str, &'static str),
    /// Cut a verse short from `at`.
    Cut(Verse, &'static str),
}

/// What the module gets wrong, put right.
const FIXES: &[Fix] = &[
    // Verses run into the one before, sometimes behind a stray label.
    Fix::Split {
        verse: (40, 26, 29),
        at: "And they sang praises",
        keep: true,
        into: (40, 26, 30),
    },
    Fix::Split {
        verse: (41, 4, 9),
        at: "10) ",
        keep: false,
        into: (41, 4, 10),
    },
    Fix::Split {
        verse: (41, 8, 18),
        at: "(l9) ",
        keep: false,
        into: (41, 8, 19),
    },
    Fix::Split {
        verse: (41, 9, 30),
        at: "(3l) ",
        keep: false,
        into: (41, 9, 31),
    },
    Fix::Split {
        verse: (41, 11, 18),
        at: "(l9) ",
        keep: false,
        into: (41, 11, 19),
    },
    Fix::Split {
        verse: (42, 18, 34),
        at: "And as they came near",
        keep: true,
        into: (42, 18, 35),
    },
    Fix::Split {
        verse: (44, 19, 40),
        at: "And having said these",
        keep: true,
        into: (44, 19, 41),
    },
    Fix::Split {
        verse: (44, 20, 16),
        at: "And from Miletus",
        keep: true,
        into: (44, 20, 17),
    },
    Fix::Split {
        verse: (64, 1, 14),
        at: "[ (III John 1:15) ",
        keep: false,
        into: (64, 1, 15),
    },
    Fix::Replace((64, 1, 15), " ]", ""),
    // Matthew 26:45 and 46 are both under 46.
    Fix::Renumber(&[((40, 26, 46), (40, 26, 45))]),
    Fix::Split {
        verse: (40, 26, 45),
        at: "Arise, let us go.",
        keep: true,
        into: (40, 26, 46),
    },
    // The Peshitta's 2 Corinthians 13:12 is the KJV's first half, and its
    // 13:13 the second.
    Fix::Renumber(&[((47, 13, 13), (47, 13, 14))]),
    Fix::Split {
        verse: (47, 13, 12),
        at: "All the saints salute you.",
        keep: true,
        into: (47, 13, 13),
    },
    // The Peshitta has no Comma Johanneum: its 1 John 5:7 is the end of the
    // KJV's 5:6.
    Fix::Drop((62, 5, 7)),
    Fix::Split {
        verse: (62, 5, 6),
        at: "And the Spirit testifieth",
        keep: true,
        into: (62, 5, 7),
    },
    // The Peshitta closes Romans with the grace (16:27), after the doxology,
    // as Murdock's note says; he numbers them as the KJV does.
    Fix::Renumber(&[
        ((45, 16, 24), (45, 16, 27)),
        ((45, 16, 25), (45, 16, 24)),
        ((45, 16, 26), (45, 16, 25)),
        ((45, 16, 27), (45, 16, 26)),
    ]),
    // Philemon 1:1 is at the end of Colossians, and Titus 2:15 in its place.
    Fix::Split {
        verse: (51, 4, 18),
        at: "philemon1:01 ",
        keep: false,
        into: (57, 1, 1),
    },
    // A verse the KJV divides in two, bracketed with the second's number.
    Fix::Replace((45, 7, 25), "[ (Romans 7:26) ", ""),
    Fix::Replace((45, 7, 25), " ]", ""),
    Fix::Replace((66, 12, 17), "[ (Revelation of John 12:18) ", ""),
    Fix::Replace((66, 12, 17), " ]", ""),
    // A misprint.
    Fix::Replace((42, 22, 34), "1 tell", "I tell"),
    // The colophons.
    Fix::Cut((40, 28, 20), " Completion of the"),
    Fix::Cut((41, 16, 20), " Completion of the"),
    Fix::Cut((42, 24, 53), " Completion of the"),
    Fix::Cut((43, 21, 25), " Completion of the"),
    Fix::Cut((44, 28, 31), " Completion of the"),
];

/// Apply [`FIXES`] to the verses.
fn apply_fixes(verses: &mut BTreeMap<Verse, String>, fixes: &[Fix]) -> Result<()> {
    let text = |verses: &mut BTreeMap<Verse, String>, verse: Verse| -> Result<String> {
        verses
            .get(&verse)
            .cloned()
            .with_context(|| format!("Murdock has no verse {verse:?}"))
    };
    for fix in fixes {
        match fix {
            Fix::Split {
                verse,
                at,
                keep,
                into,
            } => {
                let whole = text(verses, *verse)?;
                let start = whole
                    .find(at)
                    .with_context(|| format!("Murdock {verse:?}: no {at:?}"))?;
                let moved = if *keep {
                    &whole[start..]
                } else {
                    &whole[start + at.len()..]
                };
                verses.insert(*into, moved.trim().to_string());
                verses.insert(*verse, whole[..start].trim().to_string());
            }
            Fix::Renumber(moves) => {
                let mut moved = Vec::new();
                for (from, to) in moves.iter() {
                    moved.push((*to, text(verses, *from)?));
                }
                for (from, _) in moves.iter() {
                    verses.remove(from);
                }
                verses.extend(moved);
            }
            Fix::Drop(verse) => {
                verses
                    .remove(verse)
                    .with_context(|| format!("Murdock has no verse {verse:?}"))?;
            }
            Fix::Replace(verse, from, to) => {
                let whole = text(verses, *verse)?;
                if !whole.contains(from) {
                    bail!("Murdock {verse:?}: no {from:?}");
                }
                verses.insert(*verse, whole.replacen(from, to, 1));
            }
            Fix::Cut(verse, at) => {
                let whole = text(verses, *verse)?;
                let start = whole
                    .find(at)
                    .with_context(|| format!("Murdock {verse:?}: no {at:?}"))?;
                verses.insert(*verse, whole[..start].to_string());
            }
        }
    }
    Ok(())
}

/// The New Testament's verses in a `mod2imp` export: each entry a `$$$`
/// line naming it (`Matthew 1:1`) and its text on the lines after. The
/// module's encoding is Latin-1, but for the dash it writes as `\x0f`; its
/// footnotes (`<RF>…<Rf>`), and the dagger marking one (`\x86`), are left
/// out here. Book and chapter headings, verse 0, and empty entries are
/// skipped.
fn read_export(bytes: &[u8]) -> Result<BTreeMap<Verse, String>> {
    let text: String = bytes
        .iter()
        .map(|&b| if b == 0x0f { '—' } else { char::from(b) })
        .collect();
    let mut verses = BTreeMap::new();
    let mut entries = text.split("$$$").skip(1);
    for entry in entries.by_ref() {
        let (key, body) = entry.split_once('\n').unwrap_or((entry, ""));
        let Some((book, chapter_verse)) = key.trim().rsplit_once(' ') else {
            continue;
        };
        let Some(index) = BOOK_NAMES.iter().position(|b| *b == book) else {
            continue;
        };
        let (chapter, verse) = chapter_verse
            .split_once(':')
            .and_then(|(c, v)| Some((c.parse::<u8>().ok()?, v.parse::<u8>().ok()?)))
            .with_context(|| format!("Murdock: an unreadable entry {key:?}"))?;
        let mut body = body.replace('\n', " ");
        while let Some(start) = body.find("<RF>") {
            let end = body[start..]
                .find("<Rf>")
                .with_context(|| format!("Murdock {key}: an unclosed footnote"))?;
            body.replace_range(start..start + end + 4, "");
        }
        let body = body.replace('\u{86}', "");
        if chapter == 0 || verse == 0 || body.trim().is_empty() {
            continue;
        }
        verses.insert((40 + index as u8, chapter, verse), body);
    }
    Ok(verses)
}

/// Words of Murdock's whose sense has moved since 1852, and the words the
/// ULT uses for the same thing. His thee and thou, and his -eth, are his
/// English and are kept.
const MODERN_WORDS: [(&str, &str); 3] = [
    ("candlesticks", "lampstands"),
    ("candlestick", "lampstand"),
    // Love, which "charity" no longer says (Hebrews 6:10, 3 John 1:6).
    ("charity", "love"),
];

/// A verse with each of [`MODERN_WORDS`] replaced where it stands as a whole
/// word, a capital kept.
fn modernised(text: &str) -> String {
    crate::translation::replace_words(text, &MODERN_WORDS)
}

/// A verse's text in the stored form: Murdock's italics braced as supplied,
/// a verse all in italics plain, the text the stored form gives meaning to
/// escaped, and spaces collapsed.
fn stored(verse: Verse, text: &str) -> Result<String> {
    let mut text = text.trim();
    if let Some(inner) = text
        .strip_prefix("<FI>")
        .and_then(|t| t.strip_suffix("<Fi>"))
        .filter(|inner| !inner.contains("<FI>"))
    {
        text = inner;
    }
    let mut out = String::new();
    let mut supplied = false;
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if let Some(r) = rest.strip_prefix("<FI>") {
            if !supplied {
                out.push('{');
                supplied = true;
            }
            rest = r;
            continue;
        }
        if let Some(r) = rest.strip_prefix("<Fi>") {
            if supplied {
                out.push('}');
                supplied = false;
            }
            rest = r;
            continue;
        }
        match c {
            '<' | '>' => bail!("Murdock {verse:?}: markup left in {text:?}"),
            c if c.is_whitespace() => {
                if !out.ends_with(' ') && !out.ends_with('{') {
                    out.push(' ');
                }
            }
            c if c.is_control() || !(c.is_ascii() || c == '—') => {
                bail!("Murdock {verse:?}: an unexpected character {c:?}")
            }
            '\\' | '[' | '{' | '}' => {
                out.push('\\');
                out.push(c);
            }
            ',' | ';' | ':' | '.' | '?' | '!' if out.ends_with(' ') => {
                out.pop();
                out.push(c);
            }
            c => out.push(c),
        }
        rest = &rest[c.len_utf8()..];
    }
    if supplied {
        out.push('}');
    }
    // A space just inside a brace belongs outside it.
    let out = out.replace(" }", "} ").replace("{ ", " {");
    Ok(out.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// What `db prepare murdock` wrote, for the log.
#[derive(Debug, Default)]
pub struct PrepareSummary {
    pub verses: usize,
    pub fixes: usize,
}

/// `db prepare murdock`: read the module's `mod2imp` export at `from` and
/// write the prepared file into `out_dir`.
pub fn prepare(from: &Path, out_dir: &Path) -> Result<PrepareSummary> {
    let bytes = std::fs::read(from).with_context(|| format!("reading {}", from.display()))?;
    let mut verses = read_export(&bytes)?;
    apply_fixes(&mut verses, FIXES)?;
    let mut out = String::new();
    for (&(book, chapter, verse), text) in &verses {
        let text = stored((book, chapter, verse), &modernised(text))?;
        if text.is_empty() {
            bail!("Murdock {book} {chapter}:{verse} is empty");
        }
        out.push_str(&format!("{book}\t{chapter}:{verse}\t{text}\n"));
    }
    std::fs::create_dir_all(out_dir)?;
    let path = out_dir.join(PREPARED);
    std::fs::write(&path, out).with_context(|| format!("writing {}", path.display()))?;
    Ok(PrepareSummary {
        verses: verses.len(),
        fixes: FIXES.len(),
    })
}

/// The prepared verses, by ref, in the stored form.
pub(crate) fn read_prepared(dir: &Path) -> Result<Vec<(i64, String)>> {
    let path = dir.join(PREPARED);
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {} (run scripts/fetch-murdock.sh)", path.display()))?;
    text.lines()
        .map(|line| {
            let mut fields = line.splitn(3, '\t');
            let verse = (|| {
                let book: i64 = fields.next()?.parse().ok()?;
                let (chapter, verse) = fields.next()?.split_once(':')?;
                let reference = pack_ref(book, chapter.parse().ok()?, verse.parse().ok()?);
                Some((reference, fields.next()?.to_string()))
            })();
            verse.with_context(|| format!("{}: an unreadable line {line:?}", path.display()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPORT: &[u8] = b"$$$[ Module Heading ]\n\n$$$Matthew 0:0\n\n\
$$$Matthew 1:1\nTHE book of the nativity.\n\
$$$Matthew 1:2\nAnd Abraham begat <FI>Isaac<Fi>;\x0fand so on\x86 .<RF>A note.<Rf>\n\
$$$Matthew 1:3\nAnd Judah begat Pharez. 4) And Pharez begat Hezron.\n\
$$$Matthew 1:4\n\n\
$$$Acts 8:37\n<RF>Not in early editions.<Rf><FI>And Philip said: If thou believest.<Fi>\n\
$$$Mark 1:0\nAnd they went.\n";

    #[test]
    fn reads_and_puts_right_the_export() {
        let mut verses = read_export(EXPORT).unwrap();
        assert_eq!(verses.len(), 4);
        apply_fixes(
            &mut verses,
            &[Fix::Split {
                verse: (40, 1, 3),
                at: "4) ",
                keep: false,
                into: (40, 1, 4),
            }],
        )
        .unwrap();
        let stored: Vec<String> = verses.iter().map(|(&v, t)| stored(v, t).unwrap()).collect();
        assert_eq!(
            stored,
            [
                "THE book of the nativity.",
                "And Abraham begat {Isaac};—and so on.",
                "And Judah begat Pharez.",
                "And Pharez begat Hezron.",
                "And Philip said: If thou believest.",
            ]
        );
        let spans = haqor_core::translation::parse(&stored[1], 1, 2).unwrap();
        assert!(
            spans.iter().any(|s| s.supplied && s.text == "Isaac"),
            "{spans:?}"
        );
    }

    #[test]
    fn modernises_whole_words() {
        assert_eq!(
            modernised("the Candlestick and two candlesticks; charity, charityless"),
            "the Lampstand and two lampstands; love, charityless"
        );
    }

    #[test]
    fn a_fix_that_no_longer_applies_fails() {
        let mut verses = read_export(EXPORT).unwrap();
        assert!(apply_fixes(&mut verses, &[Fix::Cut((40, 1, 1), "Completion")]).is_err());
        assert!(apply_fixes(&mut verses, &[Fix::Drop((40, 9, 9))]).is_err());
    }

    #[test]
    fn renumbers_all_at_once() {
        let mut verses: BTreeMap<Verse, String> = [
            ((45, 16, 24), "a".to_string()),
            ((45, 16, 25), "b".to_string()),
        ]
        .into();
        apply_fixes(
            &mut verses,
            &[Fix::Renumber(&[
                ((45, 16, 24), (45, 16, 25)),
                ((45, 16, 25), (45, 16, 24)),
            ])],
        )
        .unwrap();
        assert_eq!(verses[&(45, 16, 24)], "b");
        assert_eq!(verses[&(45, 16, 25)], "a");
    }
}
