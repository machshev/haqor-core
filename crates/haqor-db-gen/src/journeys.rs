//! Haqor's own journeys (`data/journeys.json`, see `data/JOURNEYS.md`): the
//! places a journey the Bible narrates passes, in order, each with the verse
//! that takes it there, for the app to draw on its map. Built with the names,
//! whose TIPNR place records the stops name.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};
use serde_json::Value;

use crate::runtime_db::pack_ref;

/// The file of journeys, in the data directory.
pub const JOURNEYS: &str = "journeys.json";

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS journey;
DROP TABLE IF EXISTS journey_stop;

-- A journey the Bible narrates, in the file's order.
CREATE TABLE journey(
    journey_id INTEGER PRIMARY KEY,
    name       TEXT    NOT NULL,
    summary    TEXT    NOT NULL
);

-- The places it passes, in order: the place (name_entity), the verse taking
-- the journey there, what the stop is called where that is not the place's
-- name (else empty), whether it is reached by sea, whether it is drawn (0 for
-- a station Haqor gives no site), the points a sea leg passes on its way to
-- it (`longitude,latitude` split by `;`, or empty) and a line about it.
CREATE TABLE journey_stop(
    journey_id INTEGER NOT NULL,
    ord        INTEGER NOT NULL,
    entity_id  INTEGER NOT NULL,
    ref        INTEGER NOT NULL,
    label      TEXT    NOT NULL,
    by_sea     INTEGER NOT NULL,
    drawn      INTEGER NOT NULL,
    via        TEXT    NOT NULL,
    note       TEXT    NOT NULL,
    PRIMARY KEY(journey_id, ord)
) WITHOUT ROWID;
CREATE INDEX idx_journey_stop_entity ON journey_stop(entity_id);
";

/// STEP Bible's book abbreviations, as TIPNR's keys and the file's references
/// write them, in the English Bible's order.
const BOOKS: [&str; 66] = [
    "Gen", "Exo", "Lev", "Num", "Deu", "Jos", "Jdg", "Rut", "1Sa", "2Sa", "1Ki", "2Ki", "1Ch",
    "2Ch", "Ezr", "Neh", "Est", "Job", "Psa", "Pro", "Ecc", "Sng", "Isa", "Jer", "Lam", "Ezk",
    "Dan", "Hos", "Jol", "Amo", "Oba", "Jon", "Mic", "Nam", "Hab", "Zep", "Hag", "Zec", "Mal",
    "Mat", "Mrk", "Luk", "Jhn", "Act", "Rom", "1Co", "2Co", "Gal", "Eph", "Php", "Col", "1Th",
    "2Th", "1Ti", "2Ti", "Tit", "Phm", "Heb", "Jas", "1Pe", "2Pe", "1Jn", "2Jn", "3Jn", "Jud",
    "Rev",
];

/// A reference as the file writes it (`1Sa.24.1`): Haqor's book, chapter and
/// verse.
fn parse_ref(reference: &str) -> Option<(u8, u8, u8)> {
    let mut parts = reference.split('.');
    let abbreviation = parts.next()?;
    let book = BOOKS.iter().position(|b| *b == abbreviation)?;
    let chapter = parts.next()?.parse().ok()?;
    let verse = parts.next()?.parse().ok()?;
    if parts.next().is_some() || chapter == 0 || verse == 0 {
        return None;
    }
    Some((crate::tsk::book_of_key(book + 1)?, chapter, verse))
}

/// A sea leg's waypoints as the table keeps them.
fn via(stop: &Value) -> Result<String> {
    let mut points = Vec::new();
    for point in stop["via"].as_array().into_iter().flatten() {
        let (Some(longitude), Some(latitude)) = (point[0].as_f64(), point[1].as_f64()) else {
            bail!("an unreadable point {point}");
        };
        points.push(format!("{longitude},{latitude}"));
    }
    Ok(points.join(";"))
}

/// Write the journeys of `path` into the tables, each stop's place found in
/// `places` (TIPNR place keys to their ids and names). Fails, naming every
/// fault, on a stop whose key is no place record's, whose name is not the
/// record's, whose place has no position, whose reference cannot be read, or,
/// in the Old Testament, whose verse does not name its place. Returns the
/// number of journeys and of stops.
pub(crate) fn build(
    tx: &Connection,
    path: &Path,
    places: &HashMap<String, (u32, String)>,
) -> Result<(usize, usize)> {
    let json: Value = serde_json::from_str(
        &std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
    )
    .with_context(|| format!("parsing {}", path.display()))?;
    tx.execute_batch(SCHEMA)?;
    let mut journey =
        tx.prepare("INSERT INTO journey(journey_id, name, summary) VALUES (?1, ?2, ?3)")?;
    let mut insert = tx.prepare(
        "INSERT INTO journey_stop(journey_id, ord, entity_id, ref, label, by_sea, drawn, via, \
         note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    let mut located =
        tx.prepare("SELECT EXISTS(SELECT 1 FROM name_location WHERE entity_id = ?1)")?;
    let mut named =
        tx.prepare("SELECT EXISTS(SELECT 1 FROM word_name WHERE ref = ?1 AND entity_id = ?2)")?;
    let mut faults = Vec::new();
    let (mut journeys, mut stops) = (0, 0);
    for (i, j) in json["journeys"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let id = i as i64 + 1;
        let name = j["name"].as_str().unwrap_or_default();
        if name.is_empty() {
            faults.push(format!("journey {id} has no name"));
        }
        journey.execute(params![id, name, j["summary"].as_str().unwrap_or_default()])?;
        journeys += 1;
        let list = j["stops"].as_array().map(Vec::as_slice).unwrap_or_default();
        if list.len() < 2 {
            faults.push(format!("{name}: fewer than two stops"));
        }
        for (ord, stop) in list.iter().enumerate() {
            let text = |key: &str| stop[key].as_str().unwrap_or_default();
            let at = format!("{name}, stop {} ({})", ord + 1, text("name"));
            let Some((entity, record_name)) = places.get(text("key")) else {
                faults.push(format!("{at}: {} is no TIPNR place record", text("key")));
                continue;
            };
            if record_name != text("name") {
                faults.push(format!(
                    "{at}: TIPNR's record {} is {record_name}",
                    text("key")
                ));
            }
            if !located.query_row([entity], |row| row.get::<_, bool>(0))? {
                faults.push(format!("{at}: the place has no position"));
            }
            let Some((book, chapter, verse)) = parse_ref(text("ref")) else {
                faults.push(format!("{at}: an unreadable reference {:?}", text("ref")));
                continue;
            };
            let reference = pack_ref(book.into(), chapter.into(), verse.into());
            if book < 40
                && !named.query_row(params![reference, entity], |row| row.get::<_, bool>(0))?
            {
                faults.push(format!("{at}: {} does not name the place", text("ref")));
            }
            let by_sea = match stop["by"].as_str() {
                None | Some("land") => false,
                Some("sea") => true,
                Some(other) => {
                    faults.push(format!("{at}: by {other:?}, not land or sea"));
                    false
                }
            };
            let via = via(stop).with_context(|| at.clone())?;
            insert.execute(params![
                id,
                ord as i64,
                entity,
                reference,
                text("label"),
                by_sea,
                stop["drawn"].as_bool().unwrap_or(true),
                via,
                text("note"),
            ])?;
            stops += 1;
        }
    }
    if !faults.is_empty() {
        bail!("{} has faults:\n  {}", path.display(), faults.join("\n  "));
    }
    Ok((journeys, stops))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_are_read_in_haqors_numbering() {
        assert_eq!(parse_ref("Gen.12.6"), Some((1, 12, 6)));
        // Ruth and 1 Samuel by the Hebrew Bible's order of books.
        assert_eq!(parse_ref("Rut.1.1"), Some((31, 1, 1)));
        assert_eq!(parse_ref("1Sa.24.1"), Some((8, 24, 1)));
        assert_eq!(parse_ref("Act.13.4"), Some((44, 13, 4)));
        assert_eq!(parse_ref("Act.13"), None);
        assert_eq!(parse_ref("Acts.13.4"), None);
        assert_eq!(parse_ref("Gen.0.1"), None);
    }

    /// A stop's faults are all reported: an unknown key, a wrong name, a verse
    /// not naming the place, a leg neither by land nor sea.
    #[test]
    fn faulty_stops_fail_the_build() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE name_location(entity_id INTEGER);
             INSERT INTO name_location VALUES (1), (2);
             CREATE TABLE word_name(ref INTEGER, position INTEGER, entity_id INTEGER);",
        )
        .unwrap();
        db.execute(
            "INSERT INTO word_name VALUES (?1, 0, 1)",
            [pack_ref(1, 11, 31)],
        )
        .unwrap();
        let places: HashMap<String, (u32, String)> = [
            ("Ur@Gen.11.28".to_string(), (1, "Ur".to_string())),
            ("Haran@Gen.11.31".to_string(), (2, "Haran".to_string())),
        ]
        .into();
        let dir = std::env::temp_dir().join(format!("haqor-journeys-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(JOURNEYS);
        let write = |stops: &str| {
            std::fs::write(
                &path,
                format!(r#"{{"journeys":[{{"name":"Abram","summary":"","stops":[{stops}]}}]}}"#),
            )
            .unwrap()
        };
        write(
            r#"{"key":"Ur@Gen.11.28","name":"Ur","ref":"Gen.11.31"},
               {"key":"Haran@Gen.11.31","name":"Haran","ref":"Act.7.4","by":"sea","via":[[36,35]]}"#,
        );
        assert_eq!(build(&db, &path, &places).unwrap(), (1, 2));
        let via: String = db
            .query_row("SELECT via FROM journey_stop WHERE ord = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(via, "36,35");

        write(
            r#"{"key":"Ur@Gen.11.28","name":"Ur of the Chaldeans","ref":"Gen.11.31"},
               {"key":"Haran@Gen.11.31","name":"Haran","ref":"Gen.11.31","by":"air"},
               {"key":"Nowhere@Gen.1.1","name":"Nowhere","ref":"Gen.1.1"}"#,
        );
        let error = build(&db, &path, &places).unwrap_err().to_string();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(
            error.contains("TIPNR's record Ur@Gen.11.28 is Ur"),
            "{error}"
        );
        assert!(
            error.contains("Gen.11.31 does not name the place"),
            "{error}"
        );
        assert!(error.contains("by \"air\""), "{error}");
        assert!(
            error.contains("Nowhere@Gen.1.1 is no TIPNR place record"),
            "{error}"
        );
    }
}
