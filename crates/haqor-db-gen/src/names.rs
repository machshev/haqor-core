//! Import the people, places and word senses STEP Bible tells apart, and
//! where OpenBible.info places them.
//!
//! The prepared STEP Bible files (see [`crate::stepbible::prepare`]) tag
//! each TAHOT word with the TIPNR record it names, if any, and the TBESH
//! sense it has. Here TAHOT's words are aligned to the corpus as for its
//! glosses, so each tag lands on a word of Haqor's text:
//!
//! - `word_name` links a word to the person or place (`name_entity`) it
//!   names, so a reader can tell which Zechariah a verse means. The record's
//!   name forms, family and other links, and positions on a map
//!   (`name_location`, OpenBible.info's, or TIPNR's where it has none, or
//!   Haqor's own where `data/place_overrides.json` gives them) come with it.
//! - `surface_sense` gives each surface the sense (`sense`) most of its
//!   occurrences have, and `word_sense` the occurrences that differ — the
//!   same spelling read as "lie down" in one verse and "be dead" in another.
//!   Sense 0 in `word_sense` marks an occurrence with no sense at all.
//!
//! Nothing is keyed by Strong's numbers: ids are the prepared files' own.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use haqor_core::names::PlaceShape;
use log::info;
use rusqlite::{Connection, params};
use serde_json::Value;

use crate::geocoding::{Location, Place};
use crate::stepbible::{PREPARED_NAMES, PREPARED_SENSES};

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS name_entity;
DROP TABLE IF EXISTS name_form;
DROP TABLE IF EXISTS name_link;
DROP TABLE IF EXISTS name_location;
DROP TABLE IF EXISTS word_name;
DROP TABLE IF EXISTS sense;
DROP TABLE IF EXISTS surface_sense;
DROP TABLE IF EXISTS word_sense;

-- A person, place or other named thing (TIPNR). `kind` is person, place or
-- other; `category` TIPNR's finer type (Male, Female, Group, Place,
-- Supernatural, …); `description` a few words saying who or what it is;
-- `summary` what the text says of it, a line a sentence; `origin` a
-- person's tribe or nation, a place's region. `occurrences` counts the words
-- of the corpus naming it.
CREATE TABLE name_entity(
    entity_id   INTEGER PRIMARY KEY,
    name        TEXT    NOT NULL,
    kind        TEXT    NOT NULL,
    category    TEXT    NOT NULL,
    description TEXT    NOT NULL,
    summary     TEXT    NOT NULL,
    origin      TEXT    NOT NULL,
    occurrences INTEGER NOT NULL
);

-- The Hebrew forms of its name, with the English names translations give
-- each (`; ` between them) and TIPNR's word for the form (Named, Spelled,
-- Aramaic, Group, …).
CREATE TABLE name_form(
    entity_id    INTEGER NOT NULL,
    ord          INTEGER NOT NULL,
    hebrew       TEXT    NOT NULL,
    english      TEXT    NOT NULL,
    significance TEXT    NOT NULL,
    PRIMARY KEY(entity_id, ord)
) WITHOUT ROWID;

-- Links to other records: father, mother, sibling, partner, child, founder,
-- inhabitant. `flag` is TIPNR's: a (an ancestor rather than a parent), d (a
-- people descended from them), f (a founder), ? (uncertain), or empty.
CREATE TABLE name_link(
    entity_id INTEGER NOT NULL,
    ord       INTEGER NOT NULL,
    relation  TEXT    NOT NULL,
    other_id  INTEGER NOT NULL,
    flag      TEXT    NOT NULL,
    PRIMARY KEY(entity_id, ord)
) WITHOUT ROWID;

-- Where a place may have been, the likeliest first. `confidence` is
-- OpenBible.info's current score for the identification, 0 to 1000; NULL for
-- TIPNR's own position, used where OpenBible has none. `kind` is the kind of
-- place (settlement, river, region, …), `label` the modern location it is
-- identified with and `shape` the ground it covers or the course it runs
-- (haqor_core::names::PlaceShape::encode), empty for a point.
CREATE TABLE name_location(
    entity_id  INTEGER NOT NULL,
    ord        INTEGER NOT NULL,
    latitude   REAL    NOT NULL,
    longitude  REAL    NOT NULL,
    confidence INTEGER,
    kind       TEXT    NOT NULL,
    label      TEXT    NOT NULL,
    shape      TEXT    NOT NULL DEFAULT '',
    PRIMARY KEY(entity_id, ord)
) WITHOUT ROWID;

CREATE TABLE word_name(
    ref       INTEGER NOT NULL,
    position  INTEGER NOT NULL,
    entity_id INTEGER NOT NULL,
    PRIMARY KEY(ref, position)
) WITHOUT ROWID;

-- A sense of a word (TBESH): `lexeme_id` gathers the senses of one word,
-- `word` is its Hebrew headword, `language` hebrew or aramaic, and `gloss`
-- the word's gloss then the sense's, after a colon where the word has
-- several (\"to lie down: be dead\"). `occurrences` counts the corpus's
-- words with the sense.
CREATE TABLE sense(
    sense_id    INTEGER PRIMARY KEY,
    lexeme_id   INTEGER NOT NULL,
    word        TEXT    NOT NULL,
    language    TEXT    NOT NULL,
    gloss       TEXT    NOT NULL,
    occurrences INTEGER NOT NULL
);

CREATE TABLE surface_sense(
    surface_id INTEGER PRIMARY KEY,
    sense_id   INTEGER NOT NULL
);

CREATE TABLE word_sense(
    ref      INTEGER NOT NULL,
    position INTEGER NOT NULL,
    sense_id INTEGER NOT NULL,
    PRIMARY KEY(ref, position)
) WITHOUT ROWID;
";

const INDEXES: &str = "
CREATE INDEX idx_word_name_entity ON word_name(entity_id, ref);
CREATE INDEX idx_name_link_other ON name_link(other_id);
CREATE INDEX idx_sense_lexeme ON sense(lexeme_id);
CREATE INDEX idx_surface_sense_sense ON surface_sense(sense_id);
CREATE INDEX idx_word_sense_sense ON word_sense(sense_id);
";

/// What an import wrote, for the log and the tests.
#[derive(Debug, Default)]
pub struct NamesSummary {
    pub entities: usize,
    /// Words of the corpus naming a person, place or other named thing.
    pub name_words: usize,
    /// Places with a position, and of those, how many from OpenBible.info and
    /// how many from Haqor's own identifications.
    pub located: usize,
    pub geocoded: usize,
    pub overridden: usize,
    pub senses: usize,
    /// Words of the corpus with a sense, and the rows it took to say so:
    /// one per surface, and one per occurrence differing from its surface.
    pub sense_words: usize,
    pub surface_senses: usize,
    pub word_senses: usize,
}

fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or_default().to_string()
}

/// The file of Haqor's own place identifications, in the data directory
/// (see `data/PLACE_OVERRIDES.md`).
pub const PLACE_OVERRIDES: &str = "place_overrides.json";

/// A position Haqor gives a place: latitude, longitude, kind and label.
type OverrideLocation = (f64, f64, String, String);

/// Haqor's own identifications of places, which replace OpenBible.info's:
/// each TIPNR key's locations, likeliest first, with the record name the
/// entry was written for.
struct PlaceOverrides(HashMap<String, (String, Vec<OverrideLocation>)>);

impl PlaceOverrides {
    fn read(path: &Path) -> Result<Self> {
        let json: Value = serde_json::from_str(
            &std::fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?,
        )
        .with_context(|| format!("parsing {}", path.display()))?;
        let mut out = HashMap::new();
        for entry in json["places"].as_array().into_iter().flatten() {
            let key = text(entry, "key");
            let mut locations = Vec::new();
            for l in entry["locations"].as_array().into_iter().flatten() {
                let (Some(latitude), Some(longitude)) =
                    (l["latitude"].as_f64(), l["longitude"].as_f64())
                else {
                    bail!(
                        "a location of {key} in {} without a position",
                        path.display()
                    );
                };
                locations.push((latitude, longitude, text(l, "kind"), text(l, "label")));
            }
            if key.is_empty() || locations.is_empty() {
                bail!(
                    "an entry of {} without a key or locations: {entry}",
                    path.display()
                );
            }
            if out
                .insert(key.clone(), (text(entry, "name"), locations))
                .is_some()
            {
                bail!("{key} twice in {}", path.display());
            }
        }
        Ok(Self(out))
    }
}

/// The OpenBible place a TIPNR place record is: the one of the OpenBible name
/// the record gives, or else of those OpenBible links to the record, the one
/// of the record's own name, or the one identified most confidently. Several
/// of OpenBible's places can link to one record (Ephrath and Bethlehem 3 both
/// to Bethlehem of Judah), so their positions are not pooled: one may be
/// another place of the same name.
fn place_of<'a>(record: &Value, places: &'a [Place]) -> Option<&'a Place> {
    let openbible = text(record, "openbible");
    if !openbible.is_empty()
        && let Some(place) = places.iter().find(|p| p.name == openbible)
    {
        return Some(place);
    }
    let key = text(record, "key");
    let name = text(record, "name");
    let linked: Vec<&Place> = places.iter().filter(|p| p.keys.contains(&key)).collect();
    linked
        .iter()
        .find(|p| p.name == name)
        .or_else(|| linked.iter().max_by_key(|p| p.locations[0].confidence))
        .copied()
}

/// A place's locations without repeats: identifications differing only in
/// the modern name they give keep the first, most confident, of a position.
fn distinct(locations: &[Location]) -> Vec<&Location> {
    let mut out: Vec<&Location> = Vec::new();
    for location in locations {
        let same = |l: &&Location| {
            (l.latitude - location.latitude).abs() < 1e-6
                && (l.longitude - location.longitude).abs() < 1e-6
        };
        if !out.iter().any(same) {
            out.push(location);
        }
    }
    out
}

/// Rebuild the name and sense tables of a runtime database in place from the
/// prepared files in `src_texts`, placing the places `place_overrides` names
/// where it says.
pub fn build_names(
    db: &Connection,
    src_texts: &Path,
    place_overrides: &Path,
) -> Result<NamesSummary> {
    let dir = crate::stepbible::source_dir(src_texts);
    let read = |name: &str| {
        let path = dir.join(name);
        std::fs::read_to_string(&path).with_context(|| {
            format!(
                "reading {} (run scripts/fetch-stepbible-data.sh)",
                path.display()
            )
        })
    };
    let records: Vec<Value> = read(PREPARED_NAMES)?
        .lines()
        .map(serde_json::from_str)
        .collect::<serde_json::Result<_>>()
        .context("reading the prepared TIPNR records")?;
    let places = crate::geocoding::read_prepared(src_texts)?;
    let mut overrides = PlaceOverrides::read(place_overrides)?.0;
    let tags = crate::stepbible::align_tags(&dir, db)?;

    let tx = db.unchecked_transaction()?;
    tx.execute_batch(SCHEMA)?;
    let mut summary = NamesSummary::default();

    // Which words name what, and how many name each.
    let mut occurrences: HashMap<u32, i64> = HashMap::new();
    {
        let mut insert =
            tx.prepare("INSERT INTO word_name(ref, position, entity_id) VALUES (?1, ?2, ?3)")?;
        for (reference, position, tag) in &tags {
            if let Some(entity) = tag.name {
                insert.execute(params![reference, position, entity])?;
                *occurrences.entry(entity).or_default() += 1;
                summary.name_words += 1;
            }
        }
    }

    {
        let mut entity = tx.prepare(
            "INSERT INTO name_entity(entity_id, name, kind, category, description, summary, \
             origin, occurrences) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        let mut form = tx.prepare(
            "INSERT INTO name_form(entity_id, ord, hebrew, english, significance) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        let mut link = tx.prepare(
            "INSERT INTO name_link(entity_id, ord, relation, other_id, flag) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        let mut location = tx.prepare(
            "INSERT INTO name_location(entity_id, ord, latitude, longitude, confidence, kind, \
             label, shape) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        for record in &records {
            let id = record["id"]
                .as_u64()
                .context("a TIPNR record without an id")? as u32;
            entity.execute(params![
                id,
                text(record, "name"),
                text(record, "kind"),
                text(record, "category"),
                text(record, "description"),
                text(record, "summary"),
                text(record, "origin"),
                occurrences.get(&id).copied().unwrap_or_default(),
            ])?;
            for (ord, f) in record["forms"].as_array().into_iter().flatten().enumerate() {
                let english: Vec<&str> = f["english"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                form.execute(params![
                    id,
                    ord as i64,
                    text(f, "hebrew"),
                    english.join("; "),
                    text(f, "significance"),
                ])?;
            }
            for (ord, l) in record["links"].as_array().into_iter().flatten().enumerate() {
                let (Some(relation), Some(other), Some(flag)) =
                    (l[0].as_str(), l[1].as_u64(), l[2].as_str())
                else {
                    bail!("an unreadable link of TIPNR record {id}: {l}");
                };
                link.execute(params![id, ord as i64, relation, other as i64, flag])?;
            }
            if text(record, "kind") != "place" {
                continue;
            }
            if let Some((name, locations)) = overrides.remove(&text(record, "key")) {
                if name != text(record, "name") {
                    bail!(
                        "{} names {name}, but TIPNR's record {} is {}",
                        place_overrides.display(),
                        text(record, "key"),
                        text(record, "name")
                    );
                }
                for (ord, (latitude, longitude, kind, label)) in locations.iter().enumerate() {
                    location.execute(params![
                        id,
                        ord as i64,
                        latitude,
                        longitude,
                        None::<i64>,
                        kind,
                        label,
                        ""
                    ])?;
                }
                summary.located += 1;
                summary.overridden += 1;
                continue;
            }
            match place_of(record, &places) {
                Some(place) => {
                    for (ord, l) in distinct(&place.locations).into_iter().enumerate() {
                        location.execute(params![
                            id,
                            ord as i64,
                            l.latitude,
                            l.longitude,
                            l.confidence,
                            l.kind,
                            l.label,
                            l.shape.as_ref().map(PlaceShape::encode).unwrap_or_default()
                        ])?;
                    }
                    summary.located += 1;
                    summary.geocoded += 1;
                }
                None => {
                    if let Some([lat, lon]) = record["coordinates"]
                        .as_array()
                        .and_then(|c| Some([c.first()?.as_f64()?, c.get(1)?.as_f64()?]))
                    {
                        location.execute(params![id, 0, lat, lon, None::<i64>, "", "", ""])?;
                        summary.located += 1;
                    }
                }
            }
        }
        summary.entities = records.len();
    }
    if let Some(key) = overrides.keys().next() {
        bail!(
            "{} places {key}, which no TIPNR place record has",
            place_overrides.display()
        );
    }

    // Senses, keyed by surface where they can be: each surface takes the
    // sense most of its occurrences have, and only the others are listed.
    let surface_of: HashMap<(i64, i64), i64> = tx
        .prepare("SELECT ref, position, surface_id FROM word")?
        .query_map([], |row| Ok(((row.get(0)?, row.get(1)?), row.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut by_surface: HashMap<i64, HashMap<u32, usize>> = HashMap::new();
    let mut sense_counts: HashMap<u32, i64> = HashMap::new();
    for (reference, position, tag) in &tags {
        let Some(&surface) = surface_of.get(&(*reference, *position)) else {
            continue;
        };
        let sense = tag.sense.unwrap_or(0);
        *by_surface
            .entry(surface)
            .or_default()
            .entry(sense)
            .or_default() += 1;
        if sense != 0 {
            *sense_counts.entry(sense).or_default() += 1;
            summary.sense_words += 1;
        }
    }
    let surface_sense: HashMap<i64, u32> = by_surface
        .iter()
        .filter_map(|(&surface, counts)| {
            let (&sense, _) = counts
                .iter()
                .filter(|(sense, _)| **sense != 0)
                .max_by_key(|&(sense, count)| (count, std::cmp::Reverse(*sense)))?;
            Some((surface, sense))
        })
        .collect();
    {
        let mut insert =
            tx.prepare("INSERT INTO surface_sense(surface_id, sense_id) VALUES (?1, ?2)")?;
        for (surface, sense) in &surface_sense {
            insert.execute(params![surface, sense])?;
        }
        summary.surface_senses = surface_sense.len();
        let mut insert =
            tx.prepare("INSERT INTO word_sense(ref, position, sense_id) VALUES (?1, ?2, ?3)")?;
        for (reference, position, tag) in &tags {
            let Some(&surface) = surface_of.get(&(*reference, *position)) else {
                continue;
            };
            let Some(&usual) = surface_sense.get(&surface) else {
                continue;
            };
            let sense = tag.sense.unwrap_or(0);
            if sense != usual {
                insert.execute(params![reference, position, sense])?;
                summary.word_senses += 1;
            }
        }
    }
    {
        let mut insert = tx.prepare(
            "INSERT INTO sense(sense_id, lexeme_id, word, language, gloss, occurrences) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for line in read(PREPARED_SENSES)?.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            let [id, lexeme, word, language, gloss] = fields[..] else {
                bail!("an unreadable prepared sense: {line:?}");
            };
            let id: u32 = id.parse()?;
            let Some(&count) = sense_counts.get(&id) else {
                continue;
            };
            insert.execute(params![
                id,
                lexeme.parse::<u32>()?,
                word,
                language,
                gloss,
                count
            ])?;
            summary.senses += 1;
        }
    }
    tx.execute_batch(INDEXES)?;
    tx.commit()?;
    info!(
        "Names: {} records, {} words naming one; {} places located ({} by OpenBible.info). \
         Senses: {} on {} words ({} surfaces, {} occurrences differing from theirs)",
        summary.entities,
        summary.name_words,
        summary.located,
        summary.geocoded,
        summary.senses,
        summary.sense_words,
        summary.surface_senses,
        summary.word_senses
    );
    Ok(summary)
}

/// `db gen-names`: rebuild the name and sense tables of the runtime database
/// at `path` in place and re-stamp it, so syncing it to the app reinstalls it.
pub fn gen_names(path: &Path, src_texts: &Path, place_overrides: &Path) -> Result<NamesSummary> {
    let db = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
    let summary = build_names(&db, src_texts, place_overrides)?;
    crate::runtime_db::restamp_built(&db)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole of the prepared files against the built corpus.
    #[test]
    fn complete_source_imports() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let src_texts = root.join("src_texts");
        let database = root.join("data/haqor.db");
        if !database.exists() {
            eprintln!("skipping: data/haqor.db unavailable");
            return;
        }
        // Into a copy: the test must not rewrite the shared database.
        let db = Connection::open_in_memory().unwrap();
        db.execute("ATTACH DATABASE ?1 AS src", [database.to_string_lossy()])
            .unwrap();
        db.execute_batch(
            "CREATE TABLE word(ref INTEGER, position INTEGER, surface_id INTEGER,
                               PRIMARY KEY(ref, position)) WITHOUT ROWID;
             INSERT INTO word SELECT ref, position, surface_id FROM src.word
               WHERE ref < (40 << 16);
             CREATE TABLE surface(surface_id INTEGER PRIMARY KEY, text TEXT);
             INSERT INTO surface SELECT surface_id, text FROM src.surface;
             DETACH DATABASE src;",
        )
        .unwrap();
        let summary =
            build_names(&db, &src_texts, &root.join("data").join(PLACE_OVERRIDES)).unwrap();
        eprintln!("{summary:?}");
        assert!(summary.name_words > 35_000, "{summary:?}");
        assert!(summary.sense_words > 240_000, "{summary:?}");
        assert!(summary.geocoded > 900, "{summary:?}");

        // The Zechariah of 2 Kings 14:29 is Jeroboam's son, the king.
        let (name, description, father): (String, String, String) = db
            .query_row(
                "SELECT e.name, e.description, f.name FROM word_name w \
                 JOIN name_entity e USING(entity_id) \
                 JOIN name_link l ON l.entity_id = e.entity_id AND l.relation = 'father' \
                 JOIN name_entity f ON f.entity_id = l.other_id \
                 WHERE w.ref = ?1 AND w.position = 8",
                [crate::runtime_db::pack_ref(11, 14, 29)],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            (name.as_str(), father.as_str()),
            ("Zechariah", "Jeroboam"),
            "{description}"
        );

        // Bethlehem of Judah is on the map, south of Jerusalem.
        let (latitude, longitude): (f64, f64) = db
            .query_row(
                "SELECT latitude, longitude FROM name_location l \
                 JOIN name_entity e USING(entity_id) \
                 WHERE e.name = 'Bethlehem' AND e.origin = 'Tribe of Judah' AND l.ord = 0",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert!((31.6..31.8).contains(&latitude) && (35.1..35.3).contains(&longitude));
        // And only there: Bethlehem of Zebulun, in Galilee, is another place.
        let north: i64 = db
            .query_row(
                "SELECT count(*) FROM name_location l JOIN name_entity e USING(entity_id) \
                 WHERE e.name = 'Bethlehem' AND e.origin = 'Tribe of Judah' AND latitude > 32",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(north, 0);

        // Haqor's own identifications replace OpenBible.info's: Mount Sinai
        // is Jabal al-Lawz, in Midian, and only there.
        assert!(summary.overridden >= 8, "{summary:?}");
        let sinai: Vec<(f64, f64, String)> = db
            .prepare(
                "SELECT latitude, longitude, label FROM name_location l \
                 JOIN name_entity e USING(entity_id) WHERE e.name = 'Sinai' ORDER BY ord",
            )
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(sinai.len(), 1, "{sinai:?}");
        let (latitude, longitude, label) = &sinai[0];
        assert!((latitude - 28.654).abs() < 0.01 && (longitude - 35.306).abs() < 0.01);
        assert!(label.contains("Jabal al-Lawz"), "{label}");

        // Regions are drawn as their ground and rivers as their course;
        // settlements stay points.
        let shape = |name: &str| -> Option<PlaceShape> {
            let text: String = db
                .query_row(
                    "SELECT shape FROM name_location l JOIN name_entity e USING(entity_id) \
                     WHERE e.name = ?1 AND l.ord = 0 ORDER BY e.occurrences DESC LIMIT 1",
                    [name],
                    |row| row.get(0),
                )
                .unwrap();
            PlaceShape::decode(&text)
        };
        let Some(PlaceShape::Area(egypt)) = shape("Egypt") else {
            panic!("Egypt has no area");
        };
        // The Nile delta, Memphis and Ain Shams are in it.
        let ring = &egypt[0];
        let (west, east) = ring.iter().fold((f64::MAX, f64::MIN), |(w, e), [lon, _]| {
            (w.min(*lon), e.max(*lon))
        });
        assert!(west < 31.2 && east > 31.3, "{ring:?}");
        let Some(PlaceShape::Line(jordan)) = shape("Jordan") else {
            panic!("the Jordan has no course");
        };
        // From the Sea of Galilee's south down to the Dead Sea, with its
        // meanders.
        let points: Vec<&[f64; 2]> = jordan.iter().flatten().collect();
        assert!(points.len() > 100, "{}", points.len());
        assert!(points.iter().any(|[_, lat]| *lat < 31.8));
        assert!(points.iter().any(|[_, lat]| *lat > 32.7));
        assert_eq!(shape("Bethel"), None);

        // No Strong's number anywhere in the tables.
        for (table, column) in [
            ("name_entity", "name || description || summary || origin"),
            ("name_form", "hebrew || english || significance"),
            ("name_location", "kind || label"),
            ("sense", "word || gloss"),
        ] {
            let found: i64 = db
                .query_row(
                    &format!(
                        "SELECT count(*) FROM {table} WHERE {column} GLOB '*[HG][0-9][0-9][0-9][0-9]*'"
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(found, 0, "{table}");
        }
    }
}
