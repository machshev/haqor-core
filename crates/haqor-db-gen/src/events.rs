//! The events of the Bible on a timeline, from Theographic Bible Metadata
//! (`src_texts/Theographic`, CC BY-SA 4.0), corrected by Haqor's own
//! `data/event_overrides.json` (see `data/EVENTS.md`).
//!
//! Theographic dates its events by Floyd Nolen Jones's chronology of the Old
//! Testament, from the creation in 4004 BC, and Acts by Todd Atwood's. Each
//! event has a year, how long it lasted, the verses telling it, and the
//! people and places taking part, which are found here among TIPNR's records
//! (`name_entity`) by name and the verses naming them. Jesus is not taken to
//! have been present before he was born.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use log::{debug, info};
use rusqlite::{Connection, params};
use serde_json::{Value, json};

use crate::runtime_db::pack_ref;
use crate::tsk::Versification;

/// The file of Haqor's corrections to the events, in the data directory.
pub const EVENT_OVERRIDES: &str = "event_overrides.json";

/// The prepared files [`prepare`] writes: one event a line, and one person
/// or place an event names a line.
const PREPARED_EVENTS: &str = "events.jsonl";
const PREPARED_NAMES: &str = "names.jsonl";

/// TIPNR's record of Jesus, and the event of his birth, before which he takes
/// part in none.
const JESUS: &str = "Jesus@Isa.7.14";
const BIRTH_OF_JESUS: &str = "Birth of Jesus";

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS bible_event;
DROP TABLE IF EXISTS bible_event_passage;
DROP TABLE IF EXISTS bible_event_name;

-- An event of the Bible, by Theographic's id, in time order (`ord`). `year`
-- is the year it began, BC negative, with no year 0; it lasted `duration`
-- `unit`s (years, months, weeks or days), 0 and empty where none is given.
CREATE TABLE bible_event(
    event_id INTEGER PRIMARY KEY,
    ord      INTEGER NOT NULL,
    title    TEXT    NOT NULL,
    year     INTEGER NOT NULL,
    duration INTEGER NOT NULL,
    unit     TEXT    NOT NULL,
    note     TEXT    NOT NULL
);

-- The verses telling it, as spans of Haqor's numbering.
CREATE TABLE bible_event_passage(
    event_id  INTEGER NOT NULL,
    ord       INTEGER NOT NULL,
    first_ref INTEGER NOT NULL,
    last_ref  INTEGER NOT NULL,
    PRIMARY KEY(event_id, ord)
) WITHOUT ROWID;

-- The people and places (name_entity) taking part.
CREATE TABLE bible_event_name(
    event_id  INTEGER NOT NULL,
    ord       INTEGER NOT NULL,
    entity_id INTEGER NOT NULL,
    PRIMARY KEY(event_id, ord)
) WITHOUT ROWID;
CREATE INDEX idx_bible_event_name_entity ON bible_event_name(entity_id);
";

/// `src_texts/Theographic`, holding the prepared files.
pub fn source_dir(src_texts: &Path) -> PathBuf {
    src_texts.join("Theographic")
}

/// The OSIS book abbreviations Theographic writes, in the English Bible's
/// order.
const OSIS_BOOKS: [&str; 66] = [
    "Gen", "Exod", "Lev", "Num", "Deut", "Josh", "Judg", "Ruth", "1Sam", "2Sam", "1Kgs", "2Kgs",
    "1Chr", "2Chr", "Ezra", "Neh", "Esth", "Job", "Ps", "Prov", "Eccl", "Song", "Isa", "Jer",
    "Lam", "Ezek", "Dan", "Hos", "Joel", "Amos", "Obad", "Jonah", "Mic", "Nah", "Hab", "Zeph",
    "Hag", "Zech", "Mal", "Matt", "Mark", "Luke", "John", "Acts", "Rom", "1Cor", "2Cor", "Gal",
    "Eph", "Phil", "Col", "1Thess", "2Thess", "1Tim", "2Tim", "Titus", "Phlm", "Heb", "Jas",
    "1Pet", "2Pet", "1John", "2John", "3John", "Jude", "Rev",
];

/// A verse in the English Bible's numbering, its book counted from 1 in the
/// English order: how Theographic and TIPNR's keys both give them.
type EnglishVerse = (usize, u8, u8);

/// A reference `Book.chapter.verse`, its book one of `books`. A letter after
/// the verse (TIPNR's `Gen.19.37b`) is dropped.
fn parse_english(reference: &str, books: &[&str; 66]) -> Option<EnglishVerse> {
    let mut parts = reference.split('.');
    let abbreviation = parts.next()?;
    let book = books.iter().position(|b| *b == abbreviation)? + 1;
    let chapter = parts.next()?.parse().ok()?;
    let verse = parts
        .next()?
        .trim_end_matches(|c: char| c.is_ascii_alphabetic())
        .parse()
        .ok()?;
    if parts.next().is_some() || chapter == 0 || verse == 0 {
        return None;
    }
    Some((book, chapter, verse))
}

/// A Theographic start date, `-4003` or `0030-05-01`, as a year written BC
/// negative without a year 0. Theographic counts years astronomically, 1 BC
/// as year 0, so its `-4003` is 4004 BC. A month and day given in the
/// Roman calendar are dropped: the timeline's months are the Bible's.
fn parse_year(date: &str) -> Option<i32> {
    let (negative, rest) = match date.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, date),
    };
    let year: i32 = rest.split('-').next()?.parse().ok()?;
    let year = if negative { -year } else { year };
    Some(if year <= 0 { year - 1 } else { year })
}

/// A duration, `930Y`, `3M`, `1W` or `7D`, as a number and its unit; empty
/// for none.
fn parse_duration(duration: &str) -> Option<(u32, &'static str)> {
    if duration.is_empty() {
        return Some((0, ""));
    }
    let (amount, unit) = duration.split_at(duration.len() - 1);
    let unit = match unit {
        "Y" => "years",
        "M" => "months",
        "W" => "weeks",
        "D" => "days",
        _ => return None,
    };
    Some((amount.parse().ok()?, unit))
}

/// A name as matching compares it.
fn name_key(name: &str) -> String {
    name.trim().replace('_', " ").to_lowercase()
}

/// `db prepare theographic`: from Theographic's `CSV` directory, the events
/// with their dates, verses, people and places, and the people and places
/// they name with the names and verses that find their TIPNR records.
/// Returns the number of events and of names.
pub fn prepare(from: &Path, out_dir: &Path) -> Result<(usize, usize)> {
    let read = |name: &str| -> Result<Vec<HashMap<String, String>>> {
        let path = from.join("CSV").join(name);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        csv::Reader::from_reader(text.trim_start_matches('\u{feff}').as_bytes())
            .deserialize()
            .collect::<std::result::Result<_, _>>()
            .with_context(|| format!("parsing {}", path.display()))
    };
    let list = |value: &str| -> Vec<String> {
        value
            .split(',')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .collect()
    };
    let field =
        |row: &HashMap<String, String>, key: &str| row.get(key).cloned().unwrap_or_default();

    let mut events = read("Events.csv")?;
    events.sort_by(|a, b| {
        let sort = |row: &HashMap<String, String>| {
            field(row, "sortKey").parse::<f64>().unwrap_or(f64::MAX)
        };
        sort(a).total_cmp(&sort(b))
    });
    let mut named = HashSet::new();
    let mut out = String::new();
    for row in &events {
        let people = list(&field(row, "participants"));
        let places = list(&field(row, "locations"));
        named.extend(people.iter().cloned());
        named.extend(places.iter().cloned());
        let event = json!({
            "id": field(row, "eventID").parse::<u32>()
                .with_context(|| format!("an event without an id: {row:?}"))?,
            "title": field(row, "title"),
            "start": field(row, "startDate"),
            "duration": field(row, "duration"),
            "verses": list(&field(row, "verses")),
            "people": people,
            "places": places,
            "note": field(row, "notes").trim(),
        });
        out.push_str(&event.to_string());
        out.push('\n');
    }
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join(PREPARED_EVENTS), out)?;

    let mut names = Vec::new();
    for row in read("People.csv")? {
        let id = field(&row, "personLookup");
        if !named.contains(&id) {
            continue;
        }
        let mut aliases = list(&field(&row, "alsoCalled"));
        aliases.retain(|a| a != &field(&row, "name"));
        names.push(json!({
            "id": id,
            "kind": "person",
            "names": [field(&row, "name"), field(&row, "displayTitle")],
            "aliases": aliases,
            "verses": list(&field(&row, "verses")),
        }));
    }
    for row in read("Places.csv")? {
        let id = field(&row, "placeLookup");
        if !named.contains(&id) {
            continue;
        }
        names.push(json!({
            "id": id,
            "kind": "place",
            "names": [field(&row, "displayTitle"), field(&row, "kjvName"), field(&row, "esvName")],
            "aliases": list(&field(&row, "aliases")),
            "verses": list(&field(&row, "verses")),
        }));
    }
    names.sort_by_key(|n| n["id"].as_str().unwrap_or_default().to_string());
    let mut out = String::new();
    for name in &names {
        out.push_str(&name.to_string());
        out.push('\n');
    }
    std::fs::write(out_dir.join(PREPARED_NAMES), out)?;
    Ok((events.len(), names.len()))
}

/// A TIPNR record as matching needs it.
struct Record {
    id: u32,
    key: String,
    place: bool,
    /// Its name and the English names of its forms, as [`name_key`] gives
    /// them.
    names: HashSet<String>,
    /// The verse of its key: where TIPNR first finds it.
    first: Option<EnglishVerse>,
}

impl Record {
    fn read(record: &Value) -> Option<Self> {
        let key = record["key"].as_str()?.to_string();
        let mut names: HashSet<String> = [name_key(record["name"].as_str()?)].into();
        for form in record["forms"].as_array().into_iter().flatten() {
            for english in form["english"].as_array().into_iter().flatten() {
                names.extend(english.as_str().map(name_key));
            }
        }
        let first = key
            .split_once('@')
            .and_then(|(_, at)| parse_english(at, &crate::journeys::BOOKS));
        Some(Record {
            id: record["id"].as_u64()? as u32,
            place: record["kind"].as_str() == Some("place"),
            key,
            names,
            first,
        })
    }
}

/// Words of a name too common to find a record by: `Mount Sinai` is found by
/// `Sinai`, not `Mount`.
const COMMON_WORDS: [&str; 6] = ["mount", "of", "the", "river", "valley", "king"];

/// The TIPNR record a Theographic person or place is, among those of its kind
/// called by one of its names: its main names first, then a word of them
/// (`Peter` of `Simon Peter`), then another name it goes by. A record must
/// have evidence: TIPNR first finds it in a verse Theographic names it in, or
/// Haqor's text names it (`tagged`, by Haqor reference) in one of `refs`,
/// Theographic's verses in Haqor's numbering. Failing that, the one record of
/// its kind and main name, if there is only one. None where two are as
/// likely.
fn match_name(
    name: &Value,
    records: &[Record],
    refs: &HashSet<i64>,
    tagged: &HashMap<u32, HashSet<i64>>,
) -> Option<u32> {
    let place = name["kind"].as_str() == Some("place");
    let keys = |field: &str| -> HashSet<String> {
        name[field]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter(|n| !n.is_empty())
            .map(name_key)
            .collect()
    };
    let main = keys("names");
    let aliases = keys("aliases");
    let words: HashSet<String> = main
        .iter()
        .flat_map(|n| n.split_whitespace())
        .filter(|w| w.len() >= 3 && !COMMON_WORDS.contains(w) && !main.contains(*w))
        .map(str::to_string)
        .collect();
    let verses: HashSet<EnglishVerse> = name["verses"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| parse_english(v.as_str()?, &OSIS_BOOKS))
        .collect();
    let mut scored: Vec<((bool, u8, bool, usize), u32)> = records
        .iter()
        .filter(|r| r.place == place)
        .filter_map(|r| {
            let level = if !r.names.is_disjoint(&main) {
                2
            } else if !r.names.is_disjoint(&words) {
                1
            } else if !r.names.is_disjoint(&aliases) {
                0
            } else {
                return None;
            };
            let first = r.first.is_some_and(|f| verses.contains(&f));
            let overlap = tagged
                .get(&r.id)
                .map_or(0, |t| t.intersection(refs).count());
            Some(((first || overlap > 0, level, first, overlap), r.id))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let (best, id) = *scored.first()?;
    if !best.0 {
        let main_only = scored.iter().filter(|(s, _)| s.1 == 2).count();
        return (best.1 == 2 && main_only == 1).then_some(id);
    }
    match scored.get(1) {
        Some((second, _)) if *second == best => None,
        _ => Some(id),
    }
}

/// A Theographic reference's first verse in Haqor's numbering, packed.
fn haqor_ref(reference: &str, versification: &Versification) -> Option<i64> {
    let (book, chapter, verse) = parse_english(reference, &OSIS_BOOKS)?;
    let (b, c, v) = versification.first((crate::tsk::book_of_key(book)?, chapter, verse));
    Some(pack_ref(b.into(), c.into(), v.into()))
}

/// The verses of `references` as spans of Haqor's numbering, in order, verses
/// running on (within a chapter, or into the next chapter's first verse)
/// joined into one span. `None` names a reference that cannot be read.
fn passages(
    references: &[&str],
    versification: &Versification,
) -> std::result::Result<Vec<(i64, i64)>, String> {
    let mut spans = Vec::new();
    for reference in references {
        let (book, chapter, verse) =
            parse_english(reference, &OSIS_BOOKS).ok_or_else(|| reference.to_string())?;
        let book = crate::tsk::book_of_key(book).ok_or_else(|| reference.to_string())?;
        let pack = |(b, c, v): (u8, u8, u8)| pack_ref(b.into(), c.into(), v.into());
        spans.push((
            pack(versification.first((book, chapter, verse))),
            pack(versification.last((book, chapter, verse))),
        ));
    }
    spans.sort();
    let mut merged: Vec<(i64, i64)> = Vec::new();
    for (first, last) in spans {
        if let Some(span) = merged.last_mut() {
            let (book, chapter) = (span.1 >> 16, (span.1 >> 8) & 0xff);
            if first <= span.1 + 1 || first == pack_ref(book, chapter + 1, 1) {
                span.1 = span.1.max(last);
                continue;
            }
        }
        merged.push((first, last));
    }
    Ok(merged)
}

/// Haqor's corrections to one event, by Theographic's id.
#[derive(Default)]
struct EventOverride {
    /// The event's title as Theographic gives it, checked so that a changed
    /// id cannot correct another event.
    was: String,
    title: Option<String>,
    year: Option<i32>,
    duration: Option<String>,
    note: Option<String>,
    /// The verses it tells of, in place of Theographic's, as [`verses`] reads them.
    verses: Option<Vec<String>>,
    drop: bool,
}

struct Overrides {
    events: HashMap<u32, EventOverride>,
    /// A person or place's TIPNR key, or none to leave it out.
    names: HashMap<String, Option<String>>,
    /// Haqor's own events, written as Theographic's are (`id`, `title`,
    /// `duration`, `note`, `verses`, `people`, `places`), each with the id of
    /// the event it comes `after`, whose year it takes unless it gives a `year`.
    added: Vec<(u32, Value)>,
}

/// The first id of Haqor's own events, clear of Theographic's.
const FIRST_ADDED: u64 = 10001;

/// The verses of an override, a range (`Matt.4.1-4`) written out verse by verse.
fn verses(entry: &Value) -> Option<Vec<String>> {
    let list = entry["verses"].as_array()?;
    let mut out = Vec::new();
    for reference in list.iter().filter_map(Value::as_str) {
        let range = reference.rsplit_once('.').and_then(|(head, tail)| {
            let (first, last) = tail.split_once('-')?;
            Some((head, first.parse::<u32>().ok()?, last.parse::<u32>().ok()?))
        });
        match range {
            Some((head, first, last)) if first <= last => {
                out.extend((first..=last).map(|v| format!("{head}.{v}")))
            }
            _ => out.push(reference.to_string()),
        }
    }
    Some(out)
}

impl Overrides {
    fn read(path: &Path) -> Result<Self> {
        let json: Value = serde_json::from_str(
            &std::fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?,
        )
        .with_context(|| format!("parsing {}", path.display()))?;
        let mut events = HashMap::new();
        for entry in json["events"].as_array().into_iter().flatten() {
            let text = |key: &str| entry[key].as_str().map(str::to_string);
            let Some(id) = entry["id"].as_u64() else {
                bail!("an event in {} without an id: {entry}", path.display());
            };
            let year = match &entry["year"] {
                Value::Null => None,
                year => match year.as_i64() {
                    Some(year) if year != 0 => Some(year as i32),
                    _ => bail!("event {id} in {}: a year {year}", path.display()),
                },
            };
            let duplicate = events.insert(
                id as u32,
                EventOverride {
                    was: text("was").unwrap_or_default(),
                    title: text("title"),
                    year,
                    duration: text("duration"),
                    note: text("note"),
                    verses: verses(entry),
                    drop: entry["drop"].as_bool().unwrap_or(false),
                },
            );
            if duplicate.is_some() {
                bail!("event {id} twice in {}", path.display());
            }
        }
        let mut added = Vec::new();
        for entry in json["added"].as_array().into_iter().flatten() {
            let (Some(id), Some(after)) = (entry["id"].as_u64(), entry["after"].as_u64()) else {
                bail!(
                    "an added event in {} without an id or after: {entry}",
                    path.display()
                );
            };
            if id < FIRST_ADDED || events.contains_key(&(id as u32)) {
                bail!(
                    "added event {id} in {}: ids start at {FIRST_ADDED}",
                    path.display()
                );
            }
            let mut event = entry.clone();
            event["verses"] = json!(verses(entry).unwrap_or_default());
            added.push((after as u32, event));
        }
        let mut names = HashMap::new();
        for entry in json["names"].as_array().into_iter().flatten() {
            let Some(id) = entry["id"].as_str() else {
                bail!("a name in {} without an id: {entry}", path.display());
            };
            names.insert(id.to_string(), entry["key"].as_str().map(str::to_string));
        }
        Ok(Overrides {
            events,
            names,
            added,
        })
    }
}

/// What a build wrote, for the log and the tests.
#[derive(Debug, Default, PartialEq)]
pub struct EventsSummary {
    pub events: usize,
    pub passages: usize,
    /// People and places tagged on events, and those of Theographic's that
    /// no TIPNR record was found for, left off.
    pub names: usize,
    pub unmatched: Vec<String>,
}

/// Write the events prepared in `src_texts`, corrected by `overrides`, into
/// the tables, their people and places found among `records` (TIPNR's,
/// as prepared) and their verses renumbered by `versification`. Fails,
/// naming every fault, on an override for no event or a changed one, a
/// person or place pinned to no record, or a date, duration or verse that
/// cannot be read.
pub(crate) fn build(
    tx: &Connection,
    src_texts: &Path,
    overrides: &Path,
    records: &[Value],
    versification: &Versification,
) -> Result<EventsSummary> {
    let dir = source_dir(src_texts);
    let read = |name: &str| -> Result<Vec<Value>> {
        let path = dir.join(name);
        std::fs::read_to_string(&path)
            .with_context(|| {
                format!(
                    "reading {} (run scripts/fetch-theographic.sh)",
                    path.display()
                )
            })?
            .lines()
            .map(|line| serde_json::from_str(line).context("reading a prepared line"))
            .collect::<Result<_>>()
            .with_context(|| format!("reading {}", path.display()))
    };
    let events = read(PREPARED_EVENTS)?;
    let names = read(PREPARED_NAMES)?;
    let mut overrides = Overrides::read(overrides)?;
    let added = std::mem::take(&mut overrides.added);
    let tipnr: Vec<Record> = records.iter().filter_map(Record::read).collect();
    let by_key: HashMap<&str, u32> = tipnr.iter().map(|r| (r.key.as_str(), r.id)).collect();
    let mut faults = Vec::new();
    let mut summary = EventsSummary::default();

    // The verses Haqor's text names each record in.
    let mut tagged: HashMap<u32, HashSet<i64>> = HashMap::new();
    for row in tx
        .prepare("SELECT entity_id, ref FROM word_name")?
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, i64>(1)?)))?
    {
        let (entity, reference) = row?;
        tagged.entry(entity).or_default().insert(reference);
    }
    let mut entity_of: HashMap<String, u32> = HashMap::new();
    for name in &names {
        let id = name["id"].as_str().unwrap_or_default();
        let entity = match overrides.names.remove(id) {
            Some(Some(key)) => match by_key.get(key.as_str()) {
                Some(&entity) => Some(entity),
                None => {
                    faults.push(format!("{id} is pinned to {key}, no TIPNR record"));
                    None
                }
            },
            Some(None) => None,
            None => {
                let refs: HashSet<i64> = name["verses"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| haqor_ref(v.as_str()?, versification))
                    .collect();
                let found = match_name(name, &tipnr, &refs, &tagged);
                if found.is_none() {
                    debug!("no TIPNR record for {id}");
                    summary.unmatched.push(id.to_string());
                }
                found
            }
        };
        if let Some(entity) = entity {
            entity_of.insert(id.to_string(), entity);
        }
    }
    for id in overrides.names.keys() {
        faults.push(format!("a name override for {id}, which no event names"));
    }

    /// An event as it is written.
    struct Row<'a> {
        id: u32,
        year: i32,
        title: String,
        duration: u32,
        unit: &'static str,
        note: String,
        spans: Vec<(i64, i64)>,
        event: &'a Value,
    }
    let mut rows = Vec::new();
    for event in &events {
        let id = event["id"].as_u64().unwrap_or_default() as u32;
        let text = |key: &str| event[key].as_str().unwrap_or_default();
        let at = format!("event {id} ({})", text("title"));
        let fix = overrides.events.remove(&id).unwrap_or_default();
        if !fix.was.is_empty() && fix.was != text("title") {
            faults.push(format!("{at}: overridden as {:?}", fix.was));
        }
        if fix.drop {
            continue;
        }
        let year = fix.year.or_else(|| parse_year(text("start")));
        let duration = fix.duration.as_deref().unwrap_or(text("duration"));
        let (Some(year), Some((duration, unit))) = (year, parse_duration(duration)) else {
            faults.push(format!(
                "{at}: an unreadable date {:?} or duration {duration:?}",
                text("start")
            ));
            continue;
        };
        let references: Vec<&str> = match &fix.verses {
            Some(verses) => verses.iter().map(String::as_str).collect(),
            None => event["verses"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect(),
        };
        let spans = match passages(&references, versification) {
            Ok(spans) => spans,
            Err(reference) => {
                faults.push(format!("{at}: an unreadable verse {reference:?}"));
                continue;
            }
        };
        rows.push(Row {
            id,
            year,
            title: fix.title.unwrap_or_else(|| text("title").to_string()),
            duration,
            unit,
            note: fix.note.unwrap_or_else(|| text("note").to_string()),
            spans,
            event,
        });
    }
    // By year, Theographic's order kept within one, so that a year Haqor
    // corrects takes its event to its place.
    rows.sort_by_key(|row| row.year);

    // Haqor's own events, each just after the event it follows, in its year.
    for (after, event) in &added {
        let id = event["id"].as_u64().unwrap_or_default() as u32;
        let text = |key: &str| event[key].as_str().unwrap_or_default();
        let at = format!("added event {id} ({})", text("title"));
        let Some(before) = rows.iter().position(|row| row.id == *after) else {
            faults.push(format!("{at}: after event {after}, which there is not"));
            continue;
        };
        if text("title").is_empty() {
            faults.push(format!("{at}: no title"));
            continue;
        }
        let Some((duration, unit)) = parse_duration(text("duration")) else {
            faults.push(format!(
                "{at}: an unreadable duration {:?}",
                text("duration")
            ));
            continue;
        };
        let references: Vec<&str> = event["verses"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let spans = match passages(&references, versification) {
            Ok(spans) => spans,
            Err(reference) => {
                faults.push(format!("{at}: an unreadable verse {reference:?}"));
                continue;
            }
        };
        let row = Row {
            id,
            year: match event["year"].as_i64() {
                Some(year) if year != 0 => year as i32,
                _ => rows[before].year,
            },
            title: text("title").to_string(),
            duration,
            unit,
            note: text("note").to_string(),
            spans,
            event,
        };
        rows.insert(before + 1, row);
    }
    // An added event that gives its own year goes to its place in that year.
    rows.sort_by_key(|row| row.year);

    let jesus = by_key.get(JESUS).copied();
    let birth = rows
        .iter()
        .position(|row| row.event["title"].as_str() == Some(BIRTH_OF_JESUS));
    let (Some(jesus), Some(birth)) = (jesus, birth) else {
        bail!("no TIPNR record {JESUS} or event {BIRTH_OF_JESUS:?} to date his birth by");
    };

    tx.execute_batch(SCHEMA)?;
    let mut insert_event = tx.prepare(
        "INSERT INTO bible_event(event_id, ord, title, year, duration, unit, note) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    let mut insert_passage = tx.prepare(
        "INSERT INTO bible_event_passage(event_id, ord, first_ref, last_ref) \
         VALUES (?1, ?2, ?3, ?4)",
    )?;
    let mut insert_name =
        tx.prepare("INSERT INTO bible_event_name(event_id, ord, entity_id) VALUES (?1, ?2, ?3)")?;
    for (ord, row) in rows.iter().enumerate() {
        insert_event.execute(params![
            row.id,
            ord as i64,
            crate::tsk::name_the_lord(&row.title),
            row.year,
            row.duration,
            row.unit,
            row.note
        ])?;
        summary.events += 1;
        for (i, (first, last)) in row.spans.iter().enumerate() {
            insert_passage.execute(params![row.id, i as i64, first, last])?;
            summary.passages += 1;
        }
        let mut seen = HashSet::new();
        for name in ["people", "places"]
            .iter()
            .flat_map(|field| row.event[*field].as_array().into_iter().flatten())
            .filter_map(Value::as_str)
        {
            let Some(&entity) = entity_of.get(name) else {
                continue;
            };
            if (ord < birth && entity == jesus) || !seen.insert(entity) {
                continue;
            }
            insert_name.execute(params![row.id, seen.len() as i64 - 1, entity])?;
            summary.names += 1;
        }
    }
    for id in overrides.events.keys() {
        faults.push(format!("an override for event {id}, which there is not"));
    }
    if !faults.is_empty() {
        bail!("the events have faults:\n  {}", faults.join("\n  "));
    }
    info!(
        "Events: {} with {} passages and {} people and places ({} of Theographic's \
         without a TIPNR record)",
        summary.events,
        summary.passages,
        summary.names,
        summary.unmatched.len()
    );
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn years_are_written_without_a_year_zero() {
        assert_eq!(parse_year("-4003"), Some(-4004));
        assert_eq!(parse_year("0"), Some(-1));
        assert_eq!(parse_year("57"), Some(57));
        assert_eq!(parse_year("0030-05-01"), Some(30));
        assert_eq!(parse_year("-0004-12-25"), Some(-5));
        assert_eq!(parse_year("c. 30"), None);
    }

    #[test]
    fn override_verses_write_out_a_range() {
        let entry = json!({"verses": ["Matt.4.5-7", "Luke.4.13"]});
        assert_eq!(
            verses(&entry).unwrap(),
            ["Matt.4.5", "Matt.4.6", "Matt.4.7", "Luke.4.13"]
        );
        assert_eq!(verses(&json!({})), None);
    }

    #[test]
    fn durations_are_read() {
        assert_eq!(parse_duration("930Y"), Some((930, "years")));
        assert_eq!(parse_duration("1W"), Some((1, "weeks")));
        assert_eq!(parse_duration(""), Some((0, "")));
        assert_eq!(parse_duration("7X"), None);
    }

    /// Verses running on make one span, into the next chapter too; Malachi
    /// 4 is Malachi 3 in the Hebrew Bible.
    #[test]
    fn verses_join_into_spans_in_haqors_numbering() {
        let mut versification = Versification::default();
        versification.insert((26, 4, 1), (26, 3, 19));
        let spans = passages(
            &["Gen.1.31", "Gen.2.1", "Gen.2.2", "Gen.2.4", "Mal.4.1"],
            &versification,
        )
        .unwrap();
        assert_eq!(
            spans,
            [
                (pack_ref(1, 1, 31), pack_ref(1, 2, 2)),
                (pack_ref(1, 2, 4), pack_ref(1, 2, 4)),
                (pack_ref(26, 3, 19), pack_ref(26, 3, 19)),
            ]
        );
        assert_eq!(passages(&["Gen.1"], &versification), Err("Gen.1".into()));
    }

    /// A name is found by the verse TIPNR first finds it in, or the verses
    /// Haqor's text names it in, its main names before a word of them and
    /// those before its other names; failing that, by its main name alone
    /// where only one record has it.
    #[test]
    fn names_find_their_records() {
        let record = |id: u64, key: &str, name: &str, kind: &str| {
            Record::read(&json!({"id": id, "key": key, "name": name, "kind": kind, "forms": []}))
                .unwrap()
        };
        let records = [
            record(1, "Eden@Gen.2.8", "Eden", "place"),
            record(2, "Eden@2Ki.19.12", "Eden", "place"),
            record(3, "Eden@2Ch.29.12", "Eden", "person"),
            record(4, "Ur@Gen.11.28", "Ur", "place"),
            record(5, "LORD@Gen.1.1", "LORD", "other"),
            record(6, "Jesus@Isa.7.14", "Jesus", "person"),
            record(7, "Shiloh@Gen.49.10", "Shiloh", "person"),
            record(8, "Peter@Mat.4.18", "Peter", "person"),
            record(9, "Simon@Mat.10.4", "Simon", "person"),
            record(10, "Bethlehem@Gen.35.16", "Bethlehem", "place"),
            record(11, "Bethlehem@Jos.19.15", "Bethlehem", "place"),
        ];
        let name = |kind: &str, names: &[&str], aliases: &[&str], verses: &[&str]| json!({"id": "x", "kind": kind, "names": names, "aliases": aliases, "verses": verses});
        let none = HashMap::new();
        let find = |name: &Value| match_name(name, &records, &HashSet::new(), &none);
        let eden = name("place", &["Eden"], &[], &["Gen.2.8", "Gen.2.10"]);
        assert_eq!(find(&eden), Some(1));
        let unsure = name("place", &["Eden"], &[], &["Isa.51.3"]);
        assert_eq!(find(&unsure), None);
        let ur = name("place", &["Ur"], &[], &[]);
        assert_eq!(find(&ur), Some(4));
        let god = name("person", &["God"], &["LORD", "Father"], &["Gen.1.1"]);
        assert_eq!(find(&god), Some(5));
        // An alias alone, without a verse, is not enough.
        let lord = name("person", &["Lord of hosts"], &["LORD"], &[]);
        assert_eq!(find(&lord), None);
        // Both records are first found in his verses; his own name wins.
        let jesus = name(
            "person",
            &["Jesus"],
            &["Shiloh"],
            &["Isa.7.14", "Gen.49.10"],
        );
        assert_eq!(find(&jesus), Some(6));
        let peter = name("person", &["Simon", "Simon Peter"], &[], &["Matt.4.18"]);
        assert_eq!(find(&peter), Some(8));
        // Neither Bethlehem is first found in its verses, but Haqor's text
        // names one of them there.
        let bethlehem = name("place", &["Bethlehem"], &[], &["Ruth.1.1"]);
        assert_eq!(find(&bethlehem), None);
        let ruth = pack_ref(31, 1, 1);
        let tagged = HashMap::from([(10, HashSet::from([ruth]))]);
        assert_eq!(
            match_name(&bethlehem, &records, &HashSet::from([ruth]), &tagged),
            Some(10)
        );
    }

    /// The whole of the prepared files against TIPNR's.
    #[test]
    fn complete_source_imports() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let src_texts = root.join("src_texts");
        if !source_dir(&src_texts).join(PREPARED_EVENTS).exists() {
            eprintln!("skipping: the prepared Theographic files are unavailable");
            return;
        }
        let database = root.join("data/haqor.db");
        if !database.exists() {
            eprintln!("skipping: data/haqor.db unavailable");
            return;
        }
        let records: Vec<Value> = std::fs::read_to_string(
            crate::stepbible::source_dir(&src_texts).join(crate::stepbible::PREPARED_NAMES),
        )
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
        let versification =
            Versification::from_tahot(&crate::stepbible::source_dir(&src_texts)).unwrap();
        // Haqor's tags of the names, into a copy.
        let db = Connection::open_in_memory().unwrap();
        db.execute("ATTACH DATABASE ?1 AS src", [database.to_string_lossy()])
            .unwrap();
        db.execute_batch(
            "CREATE TABLE word_name AS SELECT * FROM src.word_name; DETACH DATABASE src;",
        )
        .unwrap();
        let summary = build(
            &db,
            &src_texts,
            &root.join("data").join(EVENT_OVERRIDES),
            &records,
            &versification,
        )
        .unwrap();
        eprintln!("{summary:?}");
        assert!(summary.events > 400, "{summary:?}");
        assert!(summary.names > 1000, "{summary:?}");

        let year = |title: &str| -> i32 {
            db.query_row(
                "SELECT year FROM bible_event WHERE title = ?1",
                [title],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(year("Creation of all things"), -4004);
        assert_eq!(year("The Great Flood begins"), -2348);
        // No event before the creation.
        let earliest: i32 = db
            .query_row("SELECT min(year) FROM bible_event", [], |row| row.get(0))
            .unwrap();
        assert_eq!(earliest, -4004);

        // Jesus takes part in nothing before his birth.
        let jesus = records
            .iter()
            .find(|r| r["key"] == JESUS)
            .and_then(|r| r["id"].as_i64())
            .unwrap();
        let (first, birth): (i64, i64) = db
            .query_row(
                "SELECT min(e.ord), (SELECT ord FROM bible_event WHERE title = ?2) \
                 FROM bible_event_name n JOIN bible_event e USING(event_id) \
                 WHERE n.entity_id = ?1",
                params![jesus, BIRTH_OF_JESUS],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert!(
            first >= birth,
            "Jesus is tagged at {first}, born at {birth}"
        );
    }
}
