//! Does the OT text Haqor generates say what its source says?
//!
//! The `bible` table is the root of the whole pipeline: every surface, analysis,
//! gloss and tutor card downstream is derived from it, so a word missing here is
//! a word missing everywhere, and nothing further down can notice. That is not
//! hypothetical — the importer handled `<w>` and silently ignored every other
//! element, which dropped all 1,278 *qere* readings from the corpus (2 Sam 12:31
//! lost בַּמַּלְבֵּן, Gen 8:17 lost הַיְצֵא), and no test downstream could see it
//! because they all measure themselves against the same truncated text.
//!
//! So this test compares the generated table against the source XML rather than
//! against anything derived from it, and it builds the table itself from the
//! checked-in `src_texts/` so it needs no generated database and runs on any
//! checkout.
//!
//! It compares **Hebrew letters only**. The importer deliberately transforms
//! pointing and punctuation on the way in — it strips word-internal maqaf,
//! splits glued words, and repoints the divine name — none of which changes
//! which consonants appear in which verse in which order. Letters are therefore
//! the strongest property that is still exactly assertable, and they are enough
//! to catch a dropped, duplicated, reordered or misfiled word.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use haqor_db_gen::generate_bible;
use quick_xml::Reader;
use quick_xml::events::Event;

/// The 39 OT books in Haqor's canonical (Tanakh) order, with their UXLC
/// filename stems; a book's number is its 1-based index here.
///
/// This deliberately restates what `uxlc::OT_BOOKS` says instead of importing
/// it. The mapping from filename to book number is itself a thing that can go
/// wrong, and a test that read the same list it is checking could not tell that
/// two books had swapped places.
const OT_BOOKS: &[&str] = &[
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

fn src_texts() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src_texts")
}

/// The Hebrew letters of `s`, in order. Drops vowels, dagesh, cantillation,
/// maqaf, paseq, sof pasuq, the combining grapheme joiner and any ASCII that
/// came from a note — everything the importer is allowed to move around.
fn letters(s: &str) -> String {
    s.chars()
        .filter(|c| ('\u{05d0}'..='\u{05ea}').contains(c))
        .collect()
}

/// What the source says each verse contains, read straight from the XML.
///
/// A verse's running text is its `<w>` words together with its `<q>` *qere*
/// readings — the form the Masoretes direct the reader to say. `<k>` *ketiv* is
/// excluded: it is the written form that the qere supersedes, and where a ketiv
/// has no qere at all (the eight *ketiv wela qere*, written but explicitly not
/// read) there is nothing to say either. Letters inside a `<w>` that are wrapped
/// in `<s>` — the large, small and suspended scribal letters, such as Deut 6:4's
/// oversized ע and ד — are part of their word and are collected with it.
fn source_verses() -> BTreeMap<(u8, u8, u8), String> {
    let mut out = BTreeMap::new();
    for (index, stem) in OT_BOOKS.iter().enumerate() {
        let book = u8::try_from(index + 1).expect("39 books fit in a u8");
        let path = src_texts()
            .join("UXLC")
            .join("Books")
            .join(format!("{stem}.xml"));
        let mut reader = Reader::from_file(&path).expect("opening the UXLC book");
        let mut buf = Vec::new();
        let (mut chapter, mut verse) = (0u8, 0u8);
        // Depth of nesting inside a text-bearing element, so that a `<w>`
        // containing an `<s>` keeps collecting across the inner element.
        let mut spoken = 0usize;
        let mut written = 0usize;
        loop {
            match reader.read_event_into(&mut buf).expect("reading the UXLC book") {
                Event::Start(e) => match e.name().as_ref() {
                    b"c" => chapter = numbered(&e),
                    b"v" => {
                        verse = numbered(&e);
                        out.entry((book, chapter, verse)).or_insert_with(String::new);
                    }
                    b"w" | b"q" => spoken += 1,
                    b"k" => written += 1,
                    _ => {}
                },
                Event::End(e) => match e.name().as_ref() {
                    b"w" | b"q" => spoken -= 1,
                    b"k" => written -= 1,
                    _ => {}
                },
                // Text inside a ketiv is not read, so it is not collected; text
                // inside a word is, once the note text has been filtered out by
                // keeping letters alone.
                Event::Text(t) if spoken > 0 && written == 0 => {
                    let text = t.unescape().expect("unescaping verse text");
                    out.entry((book, chapter, verse))
                        .or_default()
                        .push_str(&letters(&text));
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
    }
    out
}

/// Read an element's `n` attribute as a number.
fn numbered(e: &quick_xml::events::BytesStart<'_>) -> u8 {
    let raw = e
        .attributes()
        .flatten()
        .find(|a| a.key.as_ref() == b"n")
        .expect("chapter and verse elements carry n");
    std::str::from_utf8(&raw.value)
        .expect("n is ASCII")
        .parse()
        .expect("n is a number")
}

/// The generated table, keyed the same way.
fn generated_verses(db: &Path) -> BTreeMap<(u8, u8, u8), String> {
    let conn = rusqlite::Connection::open(db).expect("opening the generated database");
    let mut stmt = conn
        .prepare("SELECT book, chapter, verse, words FROM bible WHERE book < 40")
        .expect("preparing");
    let rows = stmt
        .query_map([], |row| {
            Ok((
                (row.get::<_, u8>(0)?, row.get::<_, u8>(1)?, row.get::<_, u8>(2)?),
                letters(&row.get::<_, String>(3)?),
            ))
        })
        .expect("querying")
        .collect::<rusqlite::Result<BTreeMap<_, _>>>()
        .expect("collecting");
    rows
}

/// Compare two verse maps and panic with the first differences spelled out.
fn assert_same(expected: &BTreeMap<(u8, u8, u8), String>, actual: &BTreeMap<(u8, u8, u8), String>) {
    let mut differing = Vec::new();
    for (reference, want) in expected {
        match actual.get(reference) {
            Some(got) if got == want => {}
            got => differing.push((*reference, want.clone(), got.cloned())),
        }
    }
    let missing: Vec<_> = actual.keys().filter(|r| !expected.contains_key(r)).collect();

    assert!(
        missing.is_empty(),
        "{} verses in the generated table are not in the source, first: {:?}",
        missing.len(),
        &missing[..missing.len().min(5)]
    );
    if differing.is_empty() {
        return;
    }
    let mut report = format!(
        "{} of {} verses do not match the UXLC source\n",
        differing.len(),
        expected.len()
    );
    for (reference, want, got) in differing.iter().take(5) {
        let (book, chapter, verse) = reference;
        let got = got.clone().unwrap_or_else(|| "<verse absent>".to_string());
        // Point at the first letter that differs; the tail after it is usually
        // enough to recognise which word went missing.
        let at = want
            .chars()
            .zip(got.chars())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| want.chars().count().min(got.chars().count()));
        let tail = |s: &String| s.chars().skip(at).take(16).collect::<String>();
        report.push_str(&format!(
            "  {book} {chapter}:{verse} diverges at letter {at}\n    \
             source: …{}\n    table:  …{}\n",
            tail(want),
            tail(&got),
        ));
    }
    panic!("{report}");
}

/// The generated table must contain exactly the source's letters, verse by
/// verse. Builds from `src_texts/` so it needs no generated database.
#[test]
fn generated_ot_text_matches_the_uxlc_source() {
    let expected = source_verses();
    assert_eq!(
        expected.len(),
        23_213,
        "the Leningrad Codex OT is 23,213 verses"
    );

    let output = std::env::temp_dir().join("haqor-uxlc-source-check.db");
    generate_bible(&src_texts(), &output).expect("generating the bible table");
    let actual = generated_verses(&output);

    assert_same(&expected, &actual);
    let _ = std::fs::remove_file(&output);
}

/// The qere readings specifically, which were absent from the corpus entirely
/// until the importer learned to read `<q>`. Spot-checks are cheap insurance
/// that a future change cannot quietly reintroduce the same loss while still
/// satisfying a letters-only comparison somewhere else.
#[test]
fn qere_readings_reach_the_bible_table() {
    let output = std::env::temp_dir().join("haqor-uxlc-qere-check.db");
    generate_bible(&src_texts(), &output).expect("generating the bible table");
    let conn = rusqlite::Connection::open(&output).expect("opening the generated database");

    // Compare on letter-skeletons rather than on pointed literals. A pointed
    // string in a test file has to reproduce the source's combining-mark order
    // exactly to match — dagesh before or after the vowel — which is a
    // property of how the file was typed, not of whether the word is present.
    let words = |book: u8, chapter: u8, verse: u8| -> Vec<String> {
        let raw: String = conn
            .query_row(
                "SELECT words FROM bible WHERE book = ?1 AND chapter = ?2 AND verse = ?3",
                rusqlite::params![book, chapter, verse],
                |row| row.get(0),
            )
            .expect("verse is present");
        raw.split_whitespace()
            .map(letters)
            .filter(|w| !w.is_empty())
            .collect()
    };

    // 2 Sam 12:31 — ketiv במלכן, qere בַּמַּלְבֵּן "in the brickkiln".
    let samuel = words(9, 12, 31);
    assert!(
        samuel.iter().any(|w| w == "במלבן"),
        "2 Sam 12:31 lost its qere: {samuel:?}"
    );
    // Gen 8:17 — ketiv הוצא, qere הַיְצֵא "bring out".
    let genesis = words(1, 8, 17);
    assert!(
        genesis.iter().any(|w| w == "היצא"),
        "Gen 8:17 lost its qere: {genesis:?}"
    );
    // Jer 38:16 — a *ketiv wela qere*: את stands written between יהוה and אשר
    // but is not read, so the running text goes straight from one to the other.
    // This is the other half of the rule, and the half that a "collect every
    // element" fix would silently break.
    let jeremiah = words(13, 38, 16);
    assert!(
        jeremiah
            .windows(2)
            .any(|pair| pair == ["יהוה".to_string(), "אשר".to_string()]),
        "Jer 38:16 gained a ketiv that is not read: {jeremiah:?}"
    );

    let _ = std::fs::remove_file(&output);
}
