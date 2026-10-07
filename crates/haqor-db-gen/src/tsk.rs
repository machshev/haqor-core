//! Import the *Treasury of Scripture Knowledge*'s thematic cross references.
//!
//! The TSK attaches references to the key words and phrases of each verse: the
//! "wide margin" kind of cross reference, curated by hand, where the
//! `quotation` table's are found by aligning roots. `src_texts/TSK/README.md`
//! describes the source file.
//!
//! The TSK numbers verses as the King James Version does. The Hebrew Bible's
//! numbering differs in a few hundred places (Psalm titles, chapter breaks in
//! Joel, Malachi and elsewhere), so every OT reference is re-numbered through
//! STEP Bible's TAHOT data, which gives both numberings for each Hebrew word.
//! The NT numbering is the KJV's already.
//!
//! The result is the `thematic_reference` table: one row per (verse, key
//! phrase), `note_id` in reading order, with the phrase's targets as packed
//! refs (`first` or `first-last`), space separated.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use log::{debug, info};
use rusqlite::{Connection, params};

use crate::runtime_db::pack_ref;

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS thematic_reference;
CREATE TABLE thematic_reference(
    note_id INTEGER PRIMARY KEY,
    ref     INTEGER NOT NULL,
    phrase  TEXT    NOT NULL,
    targets TEXT    NOT NULL
);
CREATE INDEX idx_thematic_reference_ref ON thematic_reference(ref);
";

/// A (book, chapter, verse) in Haqor's book numbering.
type Verse = (u8, u8, u8);

/// The TSK's book abbreviations, in its own (English Bible) book order.
const ABBREVIATIONS: [&str; 66] = [
    "ge", "ex", "le", "nu", "de", "jos", "jud", "ru", "1sa", "2sa", "1ki", "2ki", "1ch", "2ch",
    "ezr", "ne", "es", "job", "ps", "pr", "ec", "so", "isa", "jer", "la", "eze", "da", "ho", "joe",
    "am", "ob", "jon", "mic", "na", "hab", "zep", "hag", "zec", "mal", "mt", "mr", "lu", "joh",
    "ac", "ro", "1co", "2co", "ga", "eph", "php", "col", "1th", "2th", "1ti", "2ti", "tit", "phm",
    "heb", "jas", "1pe", "2pe", "1jo", "2jo", "3jo", "jude", "re",
];

/// Haqor's (Tanakh-order) number of each English-order OT book.
const TANAKH_ORDER: [u8; 39] = [
    1, 2, 3, 4, 5, 6, 7, 31, 8, 9, 10, 11, 38, 39, 36, 37, 34, 29, 27, 28, 33, 30, 12, 13, 32, 14,
    35, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
];

/// Haqor's book number for the TSK's 1-based `book_key`.
pub(crate) fn book_of_key(key: usize) -> Option<u8> {
    match key {
        1..=39 => Some(TANAKH_ORDER[key - 1]),
        40..=66 => Some(key as u8),
        _ => None,
    }
}

/// Haqor's book number for a TSK abbreviation. Also takes the few references
/// the file writes in another style (`Ge8_16`, capitalised) and one misspelt
/// abbreviation.
fn book_of_abbreviation(abbreviation: &str) -> Option<u8> {
    let abbreviation = abbreviation.to_ascii_lowercase();
    let abbreviation = match abbreviation.as_str() {
        "exe" => "eze",
        other => other,
    };
    let index = ABBREVIATIONS.iter().position(|&a| a == abbreviation)?;
    book_of_key(index + 1)
}

/// A target as written: a verse or an inclusive span of verses, still in the
/// KJV numbering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Target {
    first: Verse,
    last: Verse,
}

/// Parse one `;`-separated reference group: `ps 33:6,9`, `ps 104:3,5-9`,
/// `le 12:1-13:59`, or the older `Ge8_16`. A verse list continues in the
/// chapter the previous item ended in. `None` for anything else.
fn parse_group(group: &str) -> Option<Vec<Target>> {
    let group = group.trim();
    let (book, rest) = match group.split_once(' ') {
        Some((book, rest)) => (book_of_abbreviation(book)?, rest.trim().to_string()),
        None => {
            // `Ge8_16`: book letters, then chapter_verse.
            let split = group
                .char_indices()
                .skip(1)
                .find(|(_, c)| c.is_ascii_digit())?
                .0;
            let (book, rest) = group.split_at(split);
            (book_of_abbreviation(book)?, rest.replacen('_', ":", 1))
        }
    };
    let (chapter, verses) = rest.split_once(':')?;
    let mut chapter: u8 = chapter.trim().parse().ok()?;
    let mut targets = Vec::new();
    for item in verses.split(',') {
        let item = item.trim();
        let (first, last) = match item.split_once('-') {
            Some((first, last)) => (first, Some(last)),
            None => (item, None),
        };
        let first: u8 = first.parse().ok()?;
        let start = (book, chapter, first);
        let end = match last {
            None => start,
            Some(last) => match last.split_once(':') {
                Some((c, v)) => {
                    chapter = c.parse().ok()?;
                    (book, chapter, v.parse().ok()?)
                }
                None => (book, chapter, last.parse().ok()?),
            },
        };
        if end < start {
            return None;
        }
        targets.push(Target {
            first: start,
            last: end,
        });
    }
    Some(targets)
}

/// Write the divine name where the King James wording of a TSK phrase
/// substitutes a title for it: "the LORD" (and the small-capital "GOD" of
/// "the Lord GOD") become "Yahweh", "the LORD'S" "Yahweh's", "JAH" "Yah", and
/// the Jehovah names ("Jehovahjireh") "Yahweh-jireh". An all-capital "THE
/// LORD" becomes "YAHWEH". Other words, "Lord" and "God" among them, are
/// left as they are.
pub(crate) fn name_the_lord(phrase: &str) -> String {
    let mut out = String::with_capacity(phrase.len());
    let mut rest = phrase;
    while let Some(start) = rest.find(|c: char| c.is_ascii_alphabetic()) {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let end = rest
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(rest.len());
        let (word, after) = rest.split_at(end);
        rest = after;
        match word {
            "LORD" => {
                // Its article goes with it: "the LORD" is the name alone.
                let mut capitals = false;
                for article in ["the ", "The ", "THE "] {
                    if let Some(kept) = out.strip_suffix(article)
                        && !kept.ends_with(|c: char| c.is_ascii_alphabetic())
                    {
                        capitals = article == "THE ";
                        out.truncate(kept.len());
                        break;
                    }
                }
                out.push_str(if capitals { "YAHWEH" } else { "Yahweh" });
                if let Some(after) = rest.strip_prefix("'S") {
                    out.push_str(if capitals { "'S" } else { "'s" });
                    rest = after;
                }
            }
            "GOD" | "JEHOVAH" => out.push_str("Yahweh"),
            "JAH" => out.push_str("Yah"),
            _ => match word.strip_prefix("Jehovah") {
                Some("") => out.push_str("Yahweh"),
                Some(title) => {
                    out.push_str("Yahweh-");
                    out.push_str(title);
                }
                None => out.push_str(word),
            },
        }
    }
    out.push_str(rest);
    out
}

/// One line of `tskxref.txt`: a key phrase of a verse and its reference groups.
#[derive(Debug, PartialEq, Eq)]
struct Entry {
    verse: Verse,
    order: u32,
    phrase: String,
    groups: Vec<String>,
}

fn parse_line(line: &str) -> Option<Entry> {
    let mut fields = line.trim_end_matches(['\r', '\n']).split('\t');
    let book = book_of_key(fields.next()?.parse().ok()?)?;
    let chapter = fields.next()?.parse().ok()?;
    let verse = fields.next()?.parse().ok()?;
    let order = fields.next()?.parse().ok()?;
    let phrase = fields.next()?.trim().to_string();
    let groups = fields
        .next()?
        .split(';')
        .map(str::trim)
        .filter(|g| !g.is_empty())
        .map(str::to_string)
        .collect();
    Some(Entry {
        verse: (book, chapter, verse),
        order,
        phrase,
        groups,
    })
}

/// `src_texts/TSK/tskxref.txt`.
pub fn source_path(src_texts: &Path) -> PathBuf {
    src_texts.join("TSK").join("tskxref.txt")
}

/// KJV → Hebrew verse numbering, for the OT verses where the two differ: each
/// KJV verse's first and last Hebrew verse (a KJV verse can straddle two).
#[derive(Default)]
pub(crate) struct Versification {
    hebrew: HashMap<Verse, (Verse, Verse)>,
}

impl Versification {
    pub(crate) fn from_tahot(dir: &Path) -> Result<Self> {
        let mut spans = HashMap::<Verse, (Verse, Verse)>::new();
        for (english, hebrew) in crate::stepbible::verse_numberings(dir)? {
            let span = spans.entry(english).or_insert((hebrew, hebrew));
            span.0 = span.0.min(hebrew);
            span.1 = span.1.max(hebrew);
        }
        spans.retain(|english, (first, last)| first != english || last != english);
        Ok(Versification { hebrew: spans })
    }

    fn first(&self, verse: Verse) -> Verse {
        self.hebrew.get(&verse).map_or(verse, |span| span.0)
    }

    fn last(&self, verse: Verse) -> Verse {
        self.hebrew.get(&verse).map_or(verse, |span| span.1)
    }
}

/// What an import found, for the log and the tests.
#[derive(Debug, Default)]
pub struct TskSummary {
    pub notes: usize,
    pub targets: usize,
    /// Reference groups that did not parse.
    pub unparsed: usize,
    /// Targets naming a verse the corpus does not have.
    pub missing: usize,
    /// Entries whose own verse the corpus does not have.
    pub orphaned: usize,
}

/// One `thematic_reference` row before it is written.
struct Note {
    verse: i64,
    order: u32,
    phrase: String,
    targets: Vec<(i64, i64)>,
}

fn read_notes(
    path: &Path,
    versification: &Versification,
    verses: &HashSet<i64>,
) -> Result<(Vec<Note>, TskSummary)> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let pack = |v: Verse| pack_ref(v.0.into(), v.1.into(), v.2.into());
    let mut chapter_ends = HashMap::<i64, i64>::new();
    for &verse in verses {
        let end = chapter_ends.entry(verse >> 8).or_insert(verse);
        *end = (*end).max(verse);
    }
    let mut summary = TskSummary::default();
    let mut notes = Vec::new();
    for line in BufReader::new(file).split(b'\n') {
        // The file is Latin-1 (one "Shemuël"), whose bytes are the first 256
        // code points.
        let line: String = line?.into_iter().map(char::from).collect();
        let Some(entry) = parse_line(&line) else {
            bail!("unreadable TSK line: {line:?}");
        };
        let verse = pack(versification.first(entry.verse));
        if !verses.contains(&verse) {
            summary.orphaned += 1;
            continue;
        }
        let mut targets = Vec::new();
        for group in &entry.groups {
            let Some(parsed) = parse_group(group) else {
                debug!("TSK {:?}: unparsed {group:?}", entry.verse);
                summary.unparsed += 1;
                continue;
            };
            for target in parsed {
                let first = pack(versification.first(target.first));
                let mut last = pack(versification.last(target.last));
                // Whole-chapter spans sometimes overshoot their chapter
                // (`isa 36:1-37`): end them at its last verse.
                if first != last
                    && !verses.contains(&last)
                    && let Some(&end) = chapter_ends.get(&(last >> 8))
                    && end < last
                {
                    last = end.max(first);
                }
                if !verses.contains(&first) || !verses.contains(&last) {
                    debug!("TSK {:?}: no verse for {target:?}", entry.verse);
                    summary.missing += 1;
                    continue;
                }
                if !targets.contains(&(first, last)) {
                    targets.push((first, last));
                }
            }
        }
        if targets.is_empty() {
            continue;
        }
        summary.targets += targets.len();
        notes.push(Note {
            verse,
            order: entry.order,
            phrase: name_the_lord(&entry.phrase),
            targets,
        });
    }
    // Re-numbering can bring two KJV verses' entries onto one Hebrew verse;
    // a stable sort keeps each verse's own order.
    notes.sort_by_key(|note| (note.verse, note.order));
    summary.notes = notes.len();
    Ok((notes, summary))
}

fn write_notes(db: &Connection, notes: &[Note]) -> Result<()> {
    let tx = db.unchecked_transaction()?;
    tx.execute_batch(SCHEMA)?;
    {
        let mut insert = tx.prepare(
            "INSERT INTO thematic_reference(note_id, ref, phrase, targets) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for (index, note) in notes.iter().enumerate() {
            let targets = note
                .targets
                .iter()
                .map(|&(first, last)| {
                    if first == last {
                        first.to_string()
                    } else {
                        format!("{first}-{last}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            insert.execute(params![index as i64 + 1, note.verse, note.phrase, targets])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Rebuild the `thematic_reference` table of a runtime database in place from
/// `src_texts/TSK`, re-numbered through `src_texts/STEPBible-Data`.
pub fn build_thematic_references(db: &Connection, src_texts: &Path) -> Result<TskSummary> {
    let tahot = crate::stepbible::source_dir(src_texts);
    let versification = Versification::from_tahot(&tahot).with_context(|| {
        format!(
            "reading the KJV/Hebrew verse numbering from {} (run scripts/fetch-stepbible-data.sh)",
            tahot.display()
        )
    })?;
    let verses = db
        .prepare("SELECT ref FROM verse")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<HashSet<i64>>>()?;
    let (notes, summary) = read_notes(&source_path(src_texts), &versification, &verses)?;
    write_notes(db, &notes)?;
    info!(
        "Thematic references: {} key phrases with {} targets ({} groups unparsed, \
         {} targets and {} phrases outside the corpus)",
        summary.notes, summary.targets, summary.unparsed, summary.missing, summary.orphaned
    );
    Ok(summary)
}

/// `db gen-tsk`: rebuild the `thematic_reference` table of the runtime database
/// at `path` in place and re-stamp it, so syncing it to the app reinstalls it.
pub fn gen_tsk(path: &Path, src_texts: &Path) -> Result<TskSummary> {
    let db = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
    let summary = build_thematic_references(&db, src_texts)?;
    crate::runtime_db::restamp_built(&db)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn books_map_to_tanakh_order() {
        assert_eq!(book_of_key(1), Some(1));
        assert_eq!(book_of_key(8), Some(31)); // Ruth
        assert_eq!(book_of_key(19), Some(27)); // Psalms
        assert_eq!(book_of_key(27), Some(35)); // Daniel
        assert_eq!(book_of_key(39), Some(26)); // Malachi
        assert_eq!(book_of_key(40), Some(40));
        assert_eq!(book_of_key(66), Some(66));
        assert_eq!(book_of_abbreviation("jude"), Some(65));
        assert_eq!(book_of_abbreviation("exe"), Some(14));
        // Every OT book appears exactly once.
        let mut books = TANAKH_ORDER.to_vec();
        books.sort();
        assert_eq!(books, (1..=39).collect::<Vec<u8>>());
    }

    #[test]
    fn parses_reference_groups() {
        let t = |first, last| Target { first, last };
        assert_eq!(
            parse_group("ps 104:3,5-9"),
            Some(vec![
                t((27, 104, 3), (27, 104, 3)),
                t((27, 104, 5), (27, 104, 9))
            ])
        );
        assert_eq!(
            parse_group("joh 1:1-3"),
            Some(vec![t((43, 1, 1), (43, 1, 3))])
        );
        assert_eq!(
            parse_group(" le 12:1-13:59"),
            Some(vec![t((3, 12, 1), (3, 13, 59))])
        );
        assert_eq!(parse_group("Ge8_16"), Some(vec![t((1, 8, 16), (1, 8, 16))]));
        assert_eq!(parse_group("eze 44:4, 5").map(|v| v.len()), Some(2));
        assert_eq!(parse_group("SIZE="), None);
        assert_eq!(parse_group("it is:"), None);
    }

    #[test]
    fn writes_the_divine_name() {
        for (kjv, named) in [
            ("the LORD", "Yahweh"),
            ("The LORD", "Yahweh"),
            ("O LORD", "O Yahweh"),
            ("the LORD'S anger", "Yahweh's anger"),
            (
                "is the LORD'S: it is holy unto the LORD.",
                "is Yahweh's: it is holy unto Yahweh.",
            ),
            ("saith the Lord GOD.", "saith the Lord Yahweh."),
            ("the LORD God of Israel", "Yahweh God of Israel"),
            (
                "The LORD our God is one LORD:",
                "Yahweh our God is one Yahweh:",
            ),
            ("THE LORD OUR RIGHTEOUSNESS", "YAHWEH OUR RIGHTEOUSNESS"),
            ("JEHOVAH", "Yahweh"),
            ("JAH", "Yah"),
            ("Jehovahjireh", "Yahweh-jireh"),
            ("Jehovah-shalom", "Yahweh-shalom"),
            // Titles that are not the name stay.
            ("the Lord said unto my Lord", "the Lord said unto my Lord"),
            (
                "O Lord GOD, thou hast begun",
                "O Lord Yahweh, thou hast begun",
            ),
            ("breathe LORDS", "breathe LORDS"),
        ] {
            assert_eq!(name_the_lord(kjv), named, "{kjv}");
        }
    }

    #[test]
    fn parses_a_line() {
        let line = "1\t1\t1\t2\tbeginning\tpr 8:22-24;pr 16:4;mr 13:19\r";
        assert_eq!(
            parse_line(line),
            Some(Entry {
                verse: (1, 1, 1),
                order: 2,
                phrase: "beginning".into(),
                groups: vec!["pr 8:22-24".into(), "pr 16:4".into(), "mr 13:19".into()],
            })
        );
    }

    #[test]
    fn renumbers_kjv_verses() {
        let mut versification = Versification::default();
        versification
            .hebrew
            .insert((26, 4, 1), ((26, 3, 19), (26, 3, 19)));
        assert_eq!(versification.first((26, 4, 1)), (26, 3, 19));
        assert_eq!(versification.first((26, 3, 1)), (26, 3, 1));
    }

    /// The whole source against the fetched TAHOT numbering and the built
    /// corpus: KJV-numbered verses land on their Hebrew numbers and nearly
    /// every reference resolves.
    #[test]
    fn complete_source_imports() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let src_texts = root.join("src_texts");
        let database = root.join("data/haqor.db");
        let tahot = crate::stepbible::source_dir(&src_texts);
        if !tahot.exists() || !database.exists() {
            eprintln!("skipping: fetched TAHOT source or data/haqor.db unavailable");
            return;
        }
        let versification = Versification::from_tahot(&tahot).unwrap();
        // Malachi 4:1 (KJV) is 3:19; Psalm 51:1 is 51:3; Joel 2:28 is 3:1.
        assert_eq!(versification.first((26, 4, 1)), (26, 3, 19));
        assert_eq!(versification.first((27, 51, 1)), (27, 51, 3));
        assert_eq!(versification.first((16, 2, 28)), (16, 3, 1));

        let db = Connection::open(database).unwrap();
        let verses = db
            .prepare("SELECT ref FROM verse")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<HashSet<i64>>>()
            .unwrap();
        let (notes, summary) =
            read_notes(&source_path(&src_texts), &versification, &verses).unwrap();
        eprintln!("{summary:?}");
        assert!(summary.notes > 63_000);
        assert!(summary.targets > 370_000);
        // Spans like `mt 13:54-4`, which lost the end's chapter.
        assert!(summary.unparsed < 30);
        assert!(summary.missing < 100);
        assert_eq!(summary.orphaned, 0);
        // The phrases name Yahweh where the KJV writes "the LORD".
        assert!(!notes.iter().any(|n| n.phrase.contains("LORD")));
        assert!(notes.iter().any(|n| n.phrase == "Yahweh"));

        // Genesis 1:1's first phrase is "beginning", pointing at John 1:1-3.
        let genesis = notes.iter().find(|n| n.verse == pack_ref(1, 1, 1)).unwrap();
        assert_eq!(genesis.phrase, "beginning");
        assert!(
            genesis
                .targets
                .contains(&(pack_ref(43, 1, 1), pack_ref(43, 1, 3)))
        );
        // Malachi 4:5 (KJV), "I will [send you Elijah]", is Hebrew 3:23 and
        // points at Matthew 11:14; Matthew 11:14 points back at Hebrew 3:23.
        let elijah = notes
            .iter()
            .find(|n| n.verse == pack_ref(26, 3, 23) && n.phrase == "I will")
            .unwrap();
        assert!(
            elijah
                .targets
                .contains(&(pack_ref(40, 11, 14), pack_ref(40, 11, 14)))
        );
        let matthew = notes
            .iter()
            .find(|n| n.verse == pack_ref(40, 11, 14) && n.phrase == "this")
            .unwrap();
        assert!(
            matthew
                .targets
                .contains(&(pack_ref(26, 3, 23), pack_ref(26, 3, 23)))
        );
    }
}
