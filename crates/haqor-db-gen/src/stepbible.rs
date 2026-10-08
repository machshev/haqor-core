//! Import and align STEP Bible's occurrence-level TAHOT translations, and
//! prepare the vendored STEP Bible files.
//!
//! The BDB/Strong's sources are dictionaries: useful for word information, but
//! too broad and fragmentary for a flowing interlinear. TAHOT instead supplies
//! a context-sensitive translation for every Hebrew token. Its Leningrad text
//! still differs slightly from Haqor's UXLC stream, so the same weighted verse
//! alignment used for OSHB morphology is applied here too.
//!
//! TAHOT, TIPNR and TBESH are not read as STEP Bible publishes them:
//! [`prepare`] joins them and writes what the build needs into
//! `src_texts/STEPBible-Data`, which is vendored.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
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

/// `src_texts/STEPBible-Data`, holding the prepared files [`prepare`] writes.
pub fn source_dir(src_texts: &Path) -> PathBuf {
    src_texts.join("STEPBible-Data")
}

/// The prepared files.
///
/// - `tahot.tsv`: every TAHOT word, a line each: its reference, Hebrew and
///   translation as TAHOT writes them, then the id of its sense in
///   `senses.tsv` and of the person or place it names in `names.jsonl`, each
///   empty when it has none.
/// - `senses.tsv`: the TBESH senses those words have: id, the id of the
///   lexeme it is a sense of, its Hebrew headword, language and gloss.
/// - `names.jsonl`: the TIPNR records, one JSON object a line.
const PREPARED_TAHOT: &str = "tahot.tsv";
pub(crate) const PREPARED_SENSES: &str = "senses.tsv";
pub(crate) const PREPARED_NAMES: &str = "names.jsonl";

/// Parse TAHOT's English reference, preferring the parenthesised Hebrew
/// versification when it is present (`Mal.4.6(3.24)` -> Malachi 3:24).
fn parse_reference(field: &str) -> Option<VerseRef> {
    parse_numberings(field).map(|(_, hebrew)| hebrew)
}

/// Both numberings of a TAHOT reference, English then Hebrew: the same verse
/// unless a parenthesised Hebrew one is given. English verse 0 is a Psalm
/// title, which the English numbering leaves unnumbered.
fn parse_numberings(field: &str) -> Option<(VerseRef, VerseRef)> {
    let reference = field.split_once('#')?.0;
    let (english, hebrew) = match reference.split_once('(') {
        Some((english, hebrew)) => (english, Some(hebrew.strip_suffix(')')?)),
        None => (reference, None),
    };
    let (book_name, english) = english.split_once('.')?;
    let book = book_number(book_name)?;
    let chapter_verse = |text: &str| -> Option<(u8, u8)> {
        let mut parts = text.split('.');
        let chapter = parts.next()?.parse().ok()?;
        let verse = parts.next()?.parse().ok()?;
        parts.next().is_none().then_some((chapter, verse))
    };
    let (chapter, verse) = chapter_verse(english)?;
    let (hebrew_chapter, hebrew_verse) = match hebrew {
        Some(hebrew) => chapter_verse(hebrew)?,
        None => (chapter, verse),
    };
    Some(((book, chapter, verse), (book, hebrew_chapter, hebrew_verse)))
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

/// The word a TAHOT Hebrew field writes. Backslashes introduce
/// punctuation/section markers. Slashes divide the word's prefixes, lexeme
/// and suffixes, all of which belong to one UXLC token and therefore need to
/// be joined before normalisation.
fn source_word(hebrew: &str) -> String {
    normalize_surface(&hebrew.split('\\').next().unwrap_or(hebrew).replace('/', ""))
}

/// One line of the prepared TAHOT.
struct Row<'a> {
    reference: &'a str,
    hebrew: &'a str,
    translation: &'a str,
    #[allow(dead_code)] // Read by the names import, which follows.
    tag: WordTag,
}

fn row(line: &str) -> Option<Row<'_>> {
    let mut fields = line.split('\t');
    let reference = fields.next()?;
    let hebrew = fields.next()?;
    let translation = fields.next()?;
    let id = |field: Option<&str>| field.and_then(|f| f.parse().ok());
    let tag = WordTag {
        sense: id(fields.next()),
        name: id(fields.next()),
    };
    Some(Row {
        reference,
        hebrew,
        translation,
        tag,
    })
}

/// The prepared TAHOT in `dir`.
fn read_tahot(dir: &Path) -> Result<String> {
    let path = dir.join(PREPARED_TAHOT);
    std::fs::read_to_string(&path).with_context(|| {
        format!(
            "reading {} (run scripts/fetch-stepbible-data.sh)",
            path.display()
        )
    })
}

fn parse_row(line: &str) -> Option<(VerseRef, SourceGloss)> {
    let row = row(line)?;
    let reference = parse_reference(row.reference)?;
    let word = source_word(row.hebrew);
    let gloss = clean_translation(row.translation);
    (!word.is_empty()).then_some((reference, SourceGloss { word, gloss }))
}

/// The (English, Hebrew) verse numbers of every TAHOT word, Psalm titles
/// (English verse 0) left out: the data for re-numbering an English-numbered
/// reference onto the Hebrew text.
pub(crate) fn verse_numberings(dir: &Path) -> Result<Vec<(VerseRef, VerseRef)>> {
    let mut out = Vec::new();
    for line in read_tahot(dir)?.lines() {
        if let Some((english, hebrew)) = line.split('\t').next().and_then(parse_numberings)
            && english.2 != 0
        {
            out.push((english, hebrew));
        }
    }
    Ok(out)
}

fn read_glosses(dir: &Path) -> Result<HashMap<VerseRef, Vec<SourceGloss>>> {
    let mut verses = HashMap::<VerseRef, Vec<SourceGloss>>::new();
    for line in read_tahot(dir)?.lines() {
        if let Some((reference, gloss)) = parse_row(line) {
            verses.entry(reference).or_default().push(gloss);
        }
    }
    Ok(verses)
}

/// A TAHOT word's sense (in `senses.tsv`) and the person or place it names
/// (in `names.jsonl`), by their ids in the prepared files.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct WordTag {
    pub sense: Option<u32>,
    pub name: Option<u32>,
}

#[allow(dead_code)] // Read by the names import, which follows.
/// Every prepared TAHOT word's tag, placed on the runtime database's words:
/// a packed verse reference and a word position, as in `word`. Words TAHOT
/// and the corpus cannot align safely, and words with no tag, are left out.
pub(crate) fn align_tags(
    dir: &Path,
    db: &rusqlite::Connection,
) -> Result<Vec<(i64, i64, WordTag)>> {
    let text = read_tahot(dir)?;
    let mut source = HashMap::<VerseRef, Vec<(String, WordTag)>>::new();
    for line in text.lines() {
        let Some(row) = row(line) else { continue };
        let Some(reference) = parse_reference(row.reference) else {
            continue;
        };
        let word = source_word(row.hebrew);
        if !word.is_empty() {
            source.entry(reference).or_default().push((word, row.tag));
        }
    }

    let mut words = db.prepare(
        "SELECT w.position, s.text FROM word w JOIN surface s USING(surface_id) \
         WHERE w.ref = ?1 ORDER BY w.position",
    )?;
    let mut references: Vec<_> = source.keys().copied().collect();
    references.sort();
    let mut out = Vec::new();
    for reference in references {
        let packed =
            crate::runtime_db::pack_ref(reference.0.into(), reference.1.into(), reference.2.into());
        let current = words
            .query_map([packed], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if current.is_empty() {
            continue;
        }
        let tagged = &source[&reference];
        let texts: Vec<String> = current.iter().map(|(_, text)| text.clone()).collect();
        let source_texts: Vec<String> = tagged.iter().map(|(word, _)| word.clone()).collect();
        for (current_index, source_index, _) in align_verse(&texts, &source_texts) {
            let tag = tagged[source_index].1;
            if tag != WordTag::default() {
                out.push((packed, current[current_index].0, tag));
            }
        }
    }
    Ok(out)
}

// Preparing: TAHOT, TIPNR and TBESH as STEP Bible publishes them, joined by
// their Strong's numbers and reduced to the files the build reads.

/// A TAHOT word's tagging as published: what TIPNR and TBESH join on.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SourceTag {
    /// The word's own (braced) disambiguated Strong's number, leaving out the
    /// prefixes and suffixes written on it: `H7901J`.
    strong: Option<String>,
    /// The TIPNR person or place the word names, as `Name@Ref`, with TIPNR's
    /// trailing range dropped (`Abraham@Gen.11.26`).
    name: Option<String>,
}

/// The first `{…}` group of a field: TAHOT braces a word's main morpheme.
fn braced(field: &str) -> Option<&str> {
    let start = field.find('{')? + 1;
    let end = start + field[start..].find('}')?;
    Some(&field[start..end])
}

fn parse_tag(dstrongs: &str, expanded: &str) -> SourceTag {
    let strong = braced(dstrongs)
        .filter(|tag| tag.starts_with('H'))
        .map(str::to_string);
    // `{H0087=אַבְרָם=Abram»Abraham@Gen.11.26-1Pe}`: after `»`, a name's TIPNR
    // key, or a common word's sense label (`»word:1_word`), which has no `@`.
    let name = braced(expanded)
        .and_then(|main| main.split_once('»'))
        .map(|(_, tail)| tail.split('=').next().unwrap_or(tail))
        .filter(|key| key.contains('@'))
        .map(name_key);
    SourceTag { strong, name }
}

/// A TIPNR unique name reduced to what identifies its person or place: the
/// alternative name before `|` and the range after the first reference are
/// dropped, as are decision flags (`(?)`, `(a)`). TAHOT and TIPNR agree on
/// this much where their full keys drift apart (`Ahiram@Gen.46.21` in one,
/// `Ahiram@Gen.46.21-1Ch` in the other).
pub(crate) fn name_key(unique: &str) -> String {
    let unique = unique.trim();
    let unique = unique.split('=').next().unwrap_or(unique);
    let unique = unique.rsplit('|').next().unwrap_or(unique);
    let unique = unique.split('(').next().unwrap_or(unique).trim();
    let Some((name, reference)) = unique.split_once('@') else {
        return unique.to_string();
    };
    let reference = reference.trim_start_matches('@');
    let reference = reference.split('-').next().unwrap_or(reference);
    format!("{}@{}", name.trim(), reference.trim())
}

/// The published files `prepare` reads, by their path in the repository.
const TAHOT_DIR: &str = "Translators Amalgamated OT+NT";
const TIPNR_FILE: &str = "Proper Nouns/TIPNR - Translators Individualised Proper Names with all References - STEPBible.org CC BY.txt";
const TBESH_FILE: &str = "Lexicons/TBESH - Translators Brief lexicon of Extended Strongs for Hebrew - STEPBible.org CC BY.txt";

/// The published TAHOT files in `dir`, in name order.
fn source_files(dir: &Path) -> Result<Vec<PathBuf>> {
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
    Ok(paths)
}

/// A TBESH row: one sense of a lexeme.
struct Sense {
    /// The lexeme's (extended) Strong's number, which its senses share.
    lexeme: String,
    hebrew: String,
    /// `H` (Hebrew), `A` (Aramaic) or `N` (a name, in either).
    language: String,
    gloss: String,
}

/// TBESH's senses by disambiguated Strong's number. Its Meaning column, the
/// Online Bible's abridged BDB, needs that publisher's permission to use and
/// is not read; the glosses are Tyndale House's own.
fn read_tbesh(text: &str) -> HashMap<String, Sense> {
    let mut senses = HashMap::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.trim_end_matches('\r').split('\t').collect();
        if fields.len() < 7 || !fields[0].starts_with('H') {
            continue;
        }
        let Some(strong) = fields[1].split_whitespace().next() else {
            continue;
        };
        senses.insert(
            strong.to_string(),
            Sense {
                lexeme: fields[0].trim().to_string(),
                hebrew: fields[3].trim().to_string(),
                language: fields[5].split(':').next().unwrap_or_default().to_string(),
                gloss: fields[6].trim().to_string(),
            },
        );
    }
    senses
}

/// Whether `text` holds what looks like a Strong's number (`H1234`, `G0965`),
/// which no prepared file may.
fn has_strong(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.windows(5).enumerate().any(|(at, w)| {
        matches!(w[0], b'H' | b'G')
            && w[1..].iter().all(u8::is_ascii_digit)
            && (at == 0 || !bytes[at - 1].is_ascii_alphanumeric())
    })
}

/// What `db prepare stepbible` wrote, for the log.
#[derive(Debug, Default)]
pub struct PrepareSummary {
    pub words: usize,
    pub senses: usize,
    pub names: usize,
    /// Words tagged with a sense, and with a person or place.
    pub sense_words: usize,
    pub name_words: usize,
    /// Words TAHOT tags as naming a person or place TIPNR has no record of.
    pub unknown_names: usize,
}

/// `db prepare stepbible`: read TAHOT, TIPNR and TBESH as published, laid out
/// in `from` as in the STEPBible-Data repository, and write the prepared files
/// into `out_dir`.
///
/// The three join on Strong's numbers, so this is where those are used up:
/// each word's number becomes the id of its sense or of the person or place
/// it names, and none is written out.
pub fn prepare(from: &Path, out_dir: &Path) -> Result<PrepareSummary> {
    let read = |path: &Path| {
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
    };
    let records = crate::tipnr::parse(&read(&from.join(TIPNR_FILE))?)?;
    let by_strong = crate::tipnr::by_strong(&records);
    let by_key: HashMap<&str, usize> = records
        .iter()
        .enumerate()
        .map(|(index, record)| (record.key.as_str(), index))
        .collect();
    let tbesh = read_tbesh(&read(&from.join(TBESH_FILE))?);

    let mut summary = PrepareSummary::default();
    // Senses and lexemes are numbered in the order the text first uses them.
    let mut sense_ids: HashMap<String, u32> = HashMap::new();
    let mut lexeme_ids: HashMap<String, u32> = HashMap::new();
    let mut senses_out = String::new();
    let mut tahot_out = String::new();
    for path in source_files(&from.join(TAHOT_DIR))? {
        for line in read(&path)?.lines() {
            let fields: Vec<&str> = line.trim_end_matches('\r').split('\t').collect();
            if fields.len() < 12 || parse_numberings(fields[0]).is_none() {
                continue;
            }
            let tag = parse_tag(fields[4], fields[11]);
            let name = tag
                .name
                .as_deref()
                .and_then(|key| by_key.get(key))
                .or_else(|| tag.strong.as_deref().and_then(|s| by_strong.get(s)))
                .map(|&index| index as u32 + 1);
            if tag.name.is_some() && name.is_none() {
                summary.unknown_names += 1;
            }
            let sense = tag
                .strong
                .as_deref()
                .filter(|_| name.is_none())
                .and_then(|strong| Some((strong, tbesh.get(strong)?)))
                .filter(|(_, sense)| sense.language != "N")
                .map(|(strong, sense)| {
                    let next = sense_ids.len() as u32 + 1;
                    *sense_ids.entry(strong.to_string()).or_insert_with(|| {
                        let lexeme_next = lexeme_ids.len() as u32 + 1;
                        let lexeme = *lexeme_ids
                            .entry(sense.lexeme.clone())
                            .or_insert(lexeme_next);
                        let language = if sense.language == "A" {
                            "aramaic"
                        } else {
                            "hebrew"
                        };
                        senses_out.push_str(&format!(
                            "{next}\t{lexeme}\t{}\t{language}\t{}\n",
                            sense.hebrew, sense.gloss
                        ));
                        next
                    })
                });
            let id = |id: Option<u32>| id.map(|i| i.to_string()).unwrap_or_default();
            tahot_out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                fields[0],
                fields[1],
                fields[3],
                id(sense),
                id(name)
            ));
            summary.words += 1;
            summary.sense_words += usize::from(sense.is_some());
            summary.name_words += usize::from(name.is_some());
        }
    }
    summary.senses = sense_ids.len();

    let mut names_out = String::new();
    for record in &records {
        let links: Vec<serde_json::Value> = record
            .links
            .iter()
            .filter_map(|(relation, link)| {
                let other = *by_key.get(link.key.as_str())? as u32 + 1;
                Some(serde_json::json!([relation, other, link.flag]))
            })
            .collect();
        let forms: Vec<serde_json::Value> = record
            .forms
            .iter()
            .map(|form| {
                serde_json::json!({
                    "significance": form.significance,
                    "hebrew": form.hebrew,
                    "english": form.english,
                })
            })
            .collect();
        let line = serde_json::json!({
            "id": by_key[record.key.as_str()] as u32 + 1,
            "key": record.key,
            "kind": record.kind.as_str(),
            "name": record.name,
            "category": record.category,
            "description": record.description,
            "summary": record.summary,
            "origin": record.origin,
            "forms": forms,
            "links": links,
            "coordinates": record.coordinates.map(|(lat, lon)| [lat, lon]),
        });
        names_out.push_str(&line.to_string());
        names_out.push('\n');
    }
    summary.names = records.len();

    std::fs::create_dir_all(out_dir)?;
    for (name, text) in [
        (PREPARED_TAHOT, &tahot_out),
        (PREPARED_SENSES, &senses_out),
        (PREPARED_NAMES, &names_out),
    ] {
        if let Some(line) = text.lines().find(|line| has_strong(line)) {
            bail!("a Strong's number would be written to {name}: {line}");
        }
        let path = out_dir.join(name);
        std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(summary)
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
        assert_eq!(
            parse_numberings("Psa.51.1(51.3)#01=L"),
            Some(((27, 51, 1), (27, 51, 3)))
        );
        assert_eq!(
            parse_numberings("Psa.51.0(51.1)#01=L").unwrap().0,
            (27, 51, 0)
        );
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
        let line = "Gen.1.2#09=L\tוְ/ר֣וּחַ\tand/ [the] spirit of\t12\t";
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
    fn word_tags_name_their_sense_and_person() {
        let tag = parse_tag(
            "H9002/{H2039G}",
            "H9002=ו=and/{H2039G=הָרָן=Haran»Haran@Gen.11.26-}",
        );
        assert_eq!(tag.strong.as_deref(), Some("H2039G"));
        assert_eq!(tag.name.as_deref(), Some("Haran@Gen.11.26"));
        let tag = parse_tag(
            "{H1697G}\\H9014",
            "{H1697G=דָּבָר=: word»word:1_word;_speech;_command}\\H9014=־=link",
        );
        assert_eq!(tag.strong.as_deref(), Some("H1697G"));
        assert_eq!(tag.name, None);
    }

    #[test]
    fn name_keys_drop_ranges_alternatives_and_flags() {
        assert_eq!(name_key("Abram|Abraham@Gen.11.26-1Pe"), "Abraham@Gen.11.26");
        assert_eq!(name_key("Zechariah@2Ki.14.29-"), "Zechariah@2Ki.14.29");
        assert_eq!(name_key("Canaan@Gen.9.18-1Ch(d)"), "Canaan@Gen.9.18");
        assert_eq!(name_key("Salma@1Ch.2.51-=H8007H"), "Salma@1Ch.2.51");
        assert_eq!(
            name_key("Beth-horon_Lower@@Jos.16.3"),
            "Beth-horon_Lower@Jos.16.3"
        );
        assert_eq!(name_key("Jahdai@1Ch.2.47(?)"), "Jahdai@1Ch.2.47");
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
