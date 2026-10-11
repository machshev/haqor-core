//! Build the archaeological finds that bear on the Bible from Haqor's own
//! curation, `data/artifacts.json` (see `data/ARTIFACTS.md`): inscriptions
//! naming its people, sites it describes, early copies of its text and other
//! ancient writings, each linked to the TIPNR people and places it names and the passages it bears on, every
//! link with its strength and its source.
//!
//! The result is the `artifact`, `artifact_attestation`, `artifact_passage`
//! and `artifact_source` tables.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use log::info;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::runtime_db::pack_ref;

/// The curation file in the data directory.
pub const ARTIFACTS: &str = "artifacts.json";

/// How strongly a link holds, strongest first (see `data/ARTIFACTS.md`).
pub const STRENGTHS: &[&str] = &["strong", "reasonable", "possible"];

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS artifact;
DROP TABLE IF EXISTS artifact_attestation;
DROP TABLE IF EXISTS artifact_passage;
DROP TABLE IF EXISTS artifact_source;

-- A find, in the curation's order. `key` is the curation's own id
-- (`mesha-stele`); `other_names` are joined by '; '; `year` is negative for
-- BC, to sort by; `found_year` and `found_entity` (the TIPNR place record of
-- the find site) are NULL where unknown.
CREATE TABLE artifact(
    artifact_id  INTEGER PRIMARY KEY,
    key          TEXT    NOT NULL UNIQUE,
    name         TEXT    NOT NULL,
    other_names  TEXT    NOT NULL,
    kind         TEXT    NOT NULL,
    date         TEXT    NOT NULL,
    year         INTEGER NOT NULL,
    script       TEXT    NOT NULL,
    found_site   TEXT    NOT NULL,
    found_year   INTEGER,
    found_entity INTEGER,
    museum       TEXT    NOT NULL,
    wikipedia    TEXT    NOT NULL,
    wikidata     TEXT    NOT NULL,
    summary      TEXT    NOT NULL
);

-- A person or place a find names: `entity_id` a name_entity, `strength` one of
-- strong, reasonable or possible, `how` a sentence on how it matches, and
-- `source` an artifact_source.
CREATE TABLE artifact_attestation(
    artifact_id INTEGER NOT NULL,
    ord         INTEGER NOT NULL,
    entity_id   INTEGER NOT NULL,
    strength    TEXT    NOT NULL,
    how         TEXT    NOT NULL,
    source      TEXT    NOT NULL,
    PRIMARY KEY(artifact_id, ord)
) WITHOUT ROWID;
CREATE INDEX idx_artifact_attestation_entity ON artifact_attestation(entity_id);

-- A passage a find bears on, `ref` to `end_ref` inclusive.
CREATE TABLE artifact_passage(
    artifact_id INTEGER NOT NULL,
    ord         INTEGER NOT NULL,
    ref         INTEGER NOT NULL,
    end_ref     INTEGER NOT NULL,
    strength    TEXT    NOT NULL,
    how         TEXT    NOT NULL,
    source      TEXT    NOT NULL,
    PRIMARY KEY(artifact_id, ord)
) WITHOUT ROWID;
CREATE INDEX idx_artifact_passage_ref ON artifact_passage(ref);

-- The works the links cite.
CREATE TABLE artifact_source(
    source_id TEXT PRIMARY KEY,
    citation  TEXT NOT NULL,
    url       TEXT NOT NULL
);
";

/// What a build wrote, for the log and the tests.
#[derive(Debug, Default)]
pub struct ArtifactsSummary {
    pub artifacts: usize,
    pub attestations: usize,
    pub passages: usize,
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}

/// The prepared TIPNR records (`names.jsonl`) by key: id and name.
fn tipnr_records(src_texts: &Path) -> Result<HashMap<String, (u32, String)>> {
    let path = crate::stepbible::source_dir(src_texts).join(crate::stepbible::PREPARED_NAMES);
    let contents = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "reading {} (run scripts/fetch-stepbible-data.sh)",
            path.display()
        )
    })?;
    let mut records = HashMap::new();
    for line in contents.lines() {
        let record: Value = serde_json::from_str(line)?;
        let id = record["id"]
            .as_u64()
            .context("a TIPNR record without an id")? as u32;
        records.insert(
            text(&record, "key").to_string(),
            (id, text(&record, "name").to_string()),
        );
    }
    Ok(records)
}

/// A reference as the file writes it (`2Ki.18.13`), packed; the verse must
/// be in the text.
fn verse(reference: &str, verses: &HashSet<i64>) -> Option<i64> {
    let (book, chapter, verse) = crate::journeys::parse_ref(reference)?;
    let packed = pack_ref(book.into(), chapter.into(), verse.into());
    verses.contains(&packed).then_some(packed)
}

/// Rebuild the artifact tables of a runtime database in place from the
/// curation at `curation`. Fails on a link to a TIPNR record the database
/// lacks or names otherwise, a reference to no verse of the text, or an
/// unknown strength or source, so a mistake cannot ship silently.
pub fn build_artifacts(
    db: &Connection,
    curation: &Path,
    src_texts: &Path,
) -> Result<ArtifactsSummary> {
    let file: Value = serde_json::from_str(
        &std::fs::read_to_string(curation)
            .with_context(|| format!("reading {}", curation.display()))?,
    )
    .with_context(|| format!("parsing {}", curation.display()))?;
    let verses: HashSet<i64> = db
        .prepare("SELECT ref FROM verse")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    // TIPNR records by key, as the prepared names give them; the names build
    // keys name_entity by the same ids.
    let records = tipnr_records(src_texts)?;
    let mut entity_exists = db.prepare("SELECT name FROM name_entity WHERE entity_id = ?1")?;
    let mut entity = |key: &str, name: Option<&str>| -> Result<u32> {
        let Some((id, record_name)) = records.get(key) else {
            bail!(
                "{} links to {key}, which no TIPNR record has",
                curation.display()
            );
        };
        if let Some(name) = name
            && name != record_name
        {
            bail!(
                "{} names {key} {name}, but TIPNR's record is {record_name}",
                curation.display()
            );
        }
        if entity_exists
            .query_row([id], |row| row.get::<_, String>(0))
            .optional()?
            .is_none()
        {
            bail!("{key} is not in the database's name_entity: build the names first");
        }
        Ok(*id)
    };

    let tx = db.unchecked_transaction()?;
    tx.execute_batch(SCHEMA)?;
    let mut sources = HashSet::new();
    {
        let mut insert = tx
            .prepare("INSERT INTO artifact_source(source_id, citation, url) VALUES (?1, ?2, ?3)")?;
        for source in file["sources"].as_array().into_iter().flatten() {
            let id = text(source, "id");
            insert.execute(params![id, text(source, "citation"), text(source, "url")])?;
            sources.insert(id.to_string());
        }
    }
    let check = |id: &str, link: &Value| -> Result<()> {
        let strength = text(link, "strength");
        if !STRENGTHS.contains(&strength) {
            bail!("{id}: an unknown strength {strength:?}");
        }
        let source = text(link, "source");
        if !sources.contains(source) {
            bail!("{id}: an unknown source {source:?}");
        }
        Ok(())
    };

    let mut summary = ArtifactsSummary::default();
    let mut keys = HashSet::new();
    let mut artifact = tx.prepare(
        "INSERT INTO artifact(artifact_id, key, name, other_names, kind, date, year, script, \
         found_site, found_year, found_entity, museum, wikipedia, wikidata, summary) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
    )?;
    let mut attestation = tx.prepare(
        "INSERT INTO artifact_attestation(artifact_id, ord, entity_id, strength, how, source) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    let mut passage = tx.prepare(
        "INSERT INTO artifact_passage(artifact_id, ord, ref, end_ref, strength, how, source) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for (index, find) in file["artifacts"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let id = index as u32 + 1;
        let key = text(find, "id");
        if key.is_empty() || !keys.insert(key.to_string()) {
            bail!(
                "{}: a find without an id, or a repeated one: {key:?}",
                curation.display()
            );
        }
        for field in ["name", "kind", "date", "summary"] {
            if text(find, field).is_empty() {
                bail!("{key}: no {field}");
            }
        }
        let year = find["year"]
            .as_i64()
            .with_context(|| format!("{key}: no year"))?;
        let found = &find["found"];
        let found_entity = match found["key"].as_str() {
            Some(place) => Some(entity(place, None)?),
            None => None,
        };
        let other_names: Vec<&str> = find["other_names"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        artifact.execute(params![
            id,
            key,
            text(find, "name"),
            other_names.join("; "),
            text(find, "kind"),
            text(find, "date"),
            year,
            text(find, "script"),
            text(found, "site"),
            found["year"].as_i64(),
            found_entity,
            text(find, "museum"),
            text(find, "wikipedia"),
            text(find, "wikidata"),
            text(find, "summary"),
        ])?;
        for (ord, link) in find["attests"].as_array().into_iter().flatten().enumerate() {
            check(key, link)?;
            let person = entity(text(link, "key"), Some(text(link, "name")))?;
            attestation.execute(params![
                id,
                ord as i64,
                person,
                text(link, "strength"),
                text(link, "how"),
                text(link, "source"),
            ])?;
            summary.attestations += 1;
        }
        for (ord, link) in find["passages"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            check(key, link)?;
            let from = text(link, "from");
            let to = link["to"].as_str().unwrap_or(from);
            let (Some(first), Some(last)) = (verse(from, &verses), verse(to, &verses)) else {
                bail!("{key}: {from}–{to} is not a passage of the text");
            };
            if last < first {
                bail!("{key}: {from}–{to} runs backwards");
            }
            passage.execute(params![
                id,
                ord as i64,
                first,
                last,
                text(link, "strength"),
                text(link, "how"),
                text(link, "source"),
            ])?;
            summary.passages += 1;
        }
        summary.artifacts += 1;
    }
    drop((artifact, attestation, passage));
    tx.commit()?;
    info!(
        "Artifacts: {} finds naming people and places {} times and bearing on {} passages",
        summary.artifacts, summary.attestations, summary.passages
    );
    Ok(summary)
}

/// `db gen-artifacts`: rebuild the artifact tables of the runtime database
/// at `path` in place and re-stamp it, so syncing it to the app reinstalls it.
pub fn gen_artifacts(path: &Path, curation: &Path, src_texts: &Path) -> Result<ArtifactsSummary> {
    let db = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
    let summary = build_artifacts(&db, curation, src_texts)?;
    crate::runtime_db::restamp_built(&db)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The curation builds against the built database, and the finds land
    /// where they should.
    #[test]
    fn complete_build() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let database = root.join("data/haqor.db");
        if !database.exists() {
            eprintln!("skipping: data/haqor.db unavailable");
            return;
        }
        // Built in memory from a copy, so the test leaves the database alone.
        let db = Connection::open_in_memory().unwrap();
        db.execute("ATTACH DATABASE ?1 AS src", [database.to_string_lossy()])
            .unwrap();
        for table in ["verse", "name_entity"] {
            db.execute_batch(&format!(
                "CREATE TABLE {table} AS SELECT * FROM src.{table}"
            ))
            .unwrap();
        }
        let summary = build_artifacts(
            &db,
            &root.join("data").join(ARTIFACTS),
            &root.join("src_texts"),
        )
        .unwrap();
        eprintln!("{summary:?}");
        assert!(summary.artifacts >= 50, "{summary:?}");
        assert!(summary.attestations >= 60, "{summary:?}");

        // The Mesha Stele names Mesha, strongly, and bears on 2 Kings 3.
        let (name, strength): (String, String) = db
            .query_row(
                "SELECT e.name, t.strength FROM artifact a \
                 JOIN artifact_attestation t USING(artifact_id) \
                 JOIN name_entity e USING(entity_id) \
                 WHERE a.key = 'mesha-stele' AND t.ord = 0",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!((name.as_str(), strength.as_str()), ("Mesha", "strong"));
        let bears_on_2_kings_3: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM artifact a JOIN artifact_passage p \
                 USING(artifact_id) WHERE a.key = 'mesha-stele' AND p.ref <= ?1 \
                 AND p.end_ref >= ?1)",
                [pack_ref(11, 3, 5)],
                |row| row.get(0),
            )
            .unwrap();
        assert!(bears_on_2_kings_3);
    }

    #[test]
    fn rejects_a_link_to_no_verse() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE verse(ref INTEGER PRIMARY KEY);
             CREATE TABLE name_entity(entity_id INTEGER PRIMARY KEY, name TEXT);",
        )
        .unwrap();
        db.execute("INSERT INTO verse VALUES (?1)", [pack_ref(11, 3, 4)])
            .unwrap();
        let dir = std::env::temp_dir().join(format!("haqor-artifacts-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("data")).unwrap();
        std::fs::create_dir_all(dir.join("src_texts/STEPBible-Data")).unwrap();
        std::fs::write(
            dir.join("src_texts/STEPBible-Data")
                .join(crate::stepbible::PREPARED_NAMES),
            "",
        )
        .unwrap();
        let curation = dir.join("data").join(ARTIFACTS);
        std::fs::write(
            &curation,
            r#"{"sources":[{"id":"s","citation":"","url":""}],"artifacts":[{"id":"x","name":"X","kind":"stele","date":"c. 840 BC","year":-840,"summary":"S","attests":[],"passages":[{"from":"2Ki.3.4","to":"2Ki.3.99","strength":"strong","how":"","source":"s"}]}]}"#,
        )
        .unwrap();
        let error = build_artifacts(&db, &curation, &dir.join("src_texts"))
            .unwrap_err()
            .to_string();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(error.contains("2Ki.3.4–2Ki.3.99"), "{error}");
    }
}
