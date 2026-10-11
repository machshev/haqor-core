//! Archaeological finds that bear on the Bible: inscriptions naming its
//! people, sites it describes and early copies of its text, each with the
//! people it names and the passages it bears on, every link with its
//! strength and source. Haqor's own curation (`haqor-db-gen`'s `artifacts`
//! module, from `data/artifacts.json`).
//!
//! A find corroborates a person, a place or an event; it is not proof of the
//! Bible as a whole, and finds are not all equally certain, so each link
//! carries its own [`EvidenceStrength`].

use std::collections::{BTreeMap, HashMap};

use rusqlite::{OptionalExtension, Row};

use crate::bible::{Bible, VerseRef, VerseSpan, chapter_range};
use crate::names::{NameKind, NameSummary};

/// How strongly a find's link holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceStrength {
    /// The find names the person in a way that can only be the Bible's, or
    /// matches the passage directly.
    Strong,
    /// Likely, but the reading is restored or damaged, the identification
    /// debated, or the match is to the circumstances rather than the words.
    Reasonable,
    /// Argued for by some, doubted by many.
    Possible,
}

impl EvidenceStrength {
    fn parse(text: &str) -> Self {
        match text {
            "strong" => EvidenceStrength::Strong,
            "reasonable" => EvidenceStrength::Reasonable,
            _ => EvidenceStrength::Possible,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceStrength::Strong => "strong",
            EvidenceStrength::Reasonable => "reasonable",
            EvidenceStrength::Possible => "possible",
        }
    }
}

/// A work a link cites.
#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactSource {
    pub id: String,
    pub citation: String,
    pub url: String,
}

/// A person a find names.
#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactAttestation {
    pub person: NameSummary,
    pub strength: EvidenceStrength,
    /// What the find says and how it matches, a sentence.
    pub how: String,
    /// An [`ArtifactSource::id`].
    pub source: String,
}

/// A passage a find bears on.
#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactPassage {
    pub passage: VerseSpan,
    pub strength: EvidenceStrength,
    pub how: String,
    pub source: String,
}

/// An archaeological find.
#[derive(Clone, Debug, PartialEq)]
pub struct Artifact {
    pub id: u32,
    /// The curation's own id (`mesha-stele`).
    pub key: String,
    pub name: String,
    pub other_names: Vec<String>,
    /// stele, inscription, relief, prism, cylinder, tablet, seal, bulla,
    /// ostracon, papyrus, amulet, ossuary, manuscript, temple or site.
    pub kind: String,
    /// As a reader would write it: "c. 840 BC".
    pub date: String,
    /// To sort by: negative for BC.
    pub year: i32,
    /// The language or script, and what it is written on where notable.
    pub script: String,
    /// Where it was found, and when, where known; the TIPNR place of the
    /// site where there is one.
    pub found_site: String,
    pub found_year: Option<i32>,
    pub found_place: Option<NameSummary>,
    /// Where it is now, or "In place"; empty where not confirmed.
    pub museum: String,
    /// Its Wikipedia article's title and Wikidata id, where it has them.
    pub wikipedia: String,
    pub wikidata: String,
    pub summary: String,
    pub attestations: Vec<ArtifactAttestation>,
    pub passages: Vec<ArtifactPassage>,
}

/// Every find, in time order, with the works they cite.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Archaeology {
    pub artifacts: Vec<Artifact>,
    pub sources: Vec<ArtifactSource>,
}

/// A find that bears on a chapter: through a passage in it (`verses`), or
/// by naming someone the chapter names (`people`), or both.
#[derive(Clone, Debug, PartialEq)]
pub struct ChapterArtifact {
    pub artifact: Artifact,
    /// The chapter's verses its passages cover, in order; empty if it is
    /// here only for the people it names.
    pub verses: Vec<u8>,
    /// The people it names who are named in the chapter.
    pub people: Vec<NameSummary>,
}

fn span(first: i64, last: i64) -> VerseSpan {
    let unpack = |r: i64| VerseRef {
        book: (r >> 16) as u8,
        chapter: ((r >> 8) & 255) as u8,
        verse: (r & 255) as u8,
    };
    VerseSpan {
        first: unpack(first),
        last: unpack(last),
    }
}

fn summary_of(row: &Row, at: usize) -> rusqlite::Result<NameSummary> {
    Ok(NameSummary {
        id: row.get(at)?,
        name: row.get(at + 1)?,
        kind: NameKind::parse(&row.get::<_, String>(at + 2)?),
        description: row.get(at + 3)?,
        origin: row.get(at + 4)?,
        occurrences: row.get(at + 5)?,
    })
}

const SUMMARY_COLUMNS: &str = "e.entity_id, e.name, e.kind, e.description, e.origin, e.occurrences";

impl Bible {
    fn has_artifacts(&self) -> rusqlite::Result<bool> {
        self.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM data.sqlite_master \
             WHERE type = 'table' AND name = 'artifact')",
            [],
            |row| row.get(0),
        )
    }

    /// Every find, oldest first, each with its links; empty in a database
    /// built before there were any.
    pub fn artifacts(&self) -> rusqlite::Result<Archaeology> {
        if !self.has_artifacts()? {
            return Ok(Archaeology::default());
        }
        let sources = self
            .conn()
            .prepare_cached("SELECT source_id, citation, url FROM data.artifact_source")?
            .query_map([], |row| {
                Ok(ArtifactSource {
                    id: row.get(0)?,
                    citation: row.get(1)?,
                    url: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(Archaeology {
            artifacts: self.query_artifacts(None)?,
            sources,
        })
    }

    /// The finds `ids` names (all of them for `None`), oldest first.
    fn query_artifacts(&self, ids: Option<&[u32]>) -> rusqlite::Result<Vec<Artifact>> {
        let conn = self.conn();
        let mut attestations: HashMap<u32, Vec<ArtifactAttestation>> = HashMap::new();
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT t.artifact_id, t.strength, t.how, t.source, {SUMMARY_COLUMNS} \
             FROM data.artifact_attestation t JOIN data.name_entity e USING(entity_id) \
             ORDER BY t.artifact_id, t.ord"
        ))?;
        for row in stmt.query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                ArtifactAttestation {
                    strength: EvidenceStrength::parse(&row.get::<_, String>(1)?),
                    how: row.get(2)?,
                    source: row.get(3)?,
                    person: summary_of(row, 4)?,
                },
            ))
        })? {
            let (id, attestation) = row?;
            attestations.entry(id).or_default().push(attestation);
        }
        let mut passages: HashMap<u32, Vec<ArtifactPassage>> = HashMap::new();
        let mut stmt = conn.prepare_cached(
            "SELECT artifact_id, ref, end_ref, strength, how, source \
             FROM data.artifact_passage ORDER BY artifact_id, ord",
        )?;
        for row in stmt.query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                ArtifactPassage {
                    passage: span(row.get(1)?, row.get(2)?),
                    strength: EvidenceStrength::parse(&row.get::<_, String>(3)?),
                    how: row.get(4)?,
                    source: row.get(5)?,
                },
            ))
        })? {
            let (id, passage) = row?;
            passages.entry(id).or_default().push(passage);
        }
        let mut place = conn.prepare_cached(&format!(
            "SELECT {SUMMARY_COLUMNS} FROM data.name_entity e WHERE e.entity_id = ?1"
        ))?;
        let mut stmt = conn.prepare_cached(
            "SELECT artifact_id, key, name, other_names, kind, date, year, script, found_site, \
                    found_year, found_entity, museum, wikipedia, wikidata, summary \
             FROM data.artifact ORDER BY year, artifact_id",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, Option<u32>>(10)?,
                    Artifact {
                        id: row.get(0)?,
                        key: row.get(1)?,
                        name: row.get(2)?,
                        other_names: row
                            .get::<_, String>(3)?
                            .split("; ")
                            .filter(|n| !n.is_empty())
                            .map(str::to_string)
                            .collect(),
                        kind: row.get(4)?,
                        date: row.get(5)?,
                        year: row.get(6)?,
                        script: row.get(7)?,
                        found_site: row.get(8)?,
                        found_year: row.get(9)?,
                        found_place: None,
                        museum: row.get(11)?,
                        wikipedia: row.get(12)?,
                        wikidata: row.get(13)?,
                        summary: row.get(14)?,
                        attestations: Vec::new(),
                        passages: Vec::new(),
                    },
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut out = Vec::new();
        for (found_entity, mut artifact) in rows {
            if ids.is_some_and(|ids| !ids.contains(&artifact.id)) {
                continue;
            }
            if let Some(entity) = found_entity {
                artifact.found_place = place
                    .query_row([entity], |row| summary_of(row, 0))
                    .optional()?;
            }
            artifact.attestations = attestations.remove(&artifact.id).unwrap_or_default();
            artifact.passages = passages.remove(&artifact.id).unwrap_or_default();
            out.push(artifact);
        }
        Ok(out)
    }

    /// The finds that bear on a chapter: first those with a passage in it,
    /// by the first verse they cover, then those naming someone the chapter
    /// names, oldest first. A famous name reaches far (David is named in
    /// some 250 chapters), so a caller should keep the two apart.
    pub fn chapter_artifacts(
        &self,
        book: u8,
        chapter: u8,
    ) -> rusqlite::Result<Vec<ChapterArtifact>> {
        if !self.has_artifacts()? {
            return Ok(Vec::new());
        }
        let conn = self.conn();
        let (low, high) = chapter_range(book, chapter);
        // The chapter's verses each find's passages cover.
        let mut verses: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
        let mut stmt = conn.prepare_cached(
            "SELECT artifact_id, ref, end_ref FROM data.artifact_passage \
             WHERE ref <= ?2 AND end_ref >= ?1",
        )?;
        for row in stmt.query_map([low, high], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })? {
            let (id, first, last) = row?;
            let covered = verses.entry(id).or_default();
            let from = if first < low { 1 } else { (first & 255) as u8 };
            let to = if last > high { 255 } else { (last & 255) as u8 };
            covered.extend(from..=to);
        }
        // A passage running past the chapter's end covers its last verse,
        // not 255 of them.
        let last_verse: Option<u8> = conn.query_row(
            "SELECT MAX(ref & 255) FROM data.verse WHERE ref BETWEEN ?1 AND ?2",
            [low, high],
            |row| row.get(0),
        )?;
        for covered in verses.values_mut() {
            covered.retain(|v| last_verse.is_none_or(|last| *v <= last));
            covered.sort_unstable();
            covered.dedup();
        }
        // The people each find names whom the chapter names, Old Testament
        // word by word and New Testament by verse.
        let mut people: BTreeMap<u32, Vec<NameSummary>> = BTreeMap::new();
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT t.artifact_id, {SUMMARY_COLUMNS} FROM data.artifact_attestation t \
             JOIN data.name_entity e USING(entity_id) \
             WHERE EXISTS(SELECT 1 FROM data.word_name w WHERE w.entity_id = t.entity_id \
                          AND w.ref BETWEEN ?1 AND ?2) \
                OR EXISTS(SELECT 1 FROM data.name_verse v WHERE v.entity_id = t.entity_id \
                          AND v.ref BETWEEN ?1 AND ?2) \
             ORDER BY t.artifact_id, t.ord"
        ))?;
        for row in stmt.query_map([low, high], |row| {
            Ok((row.get::<_, u32>(0)?, summary_of(row, 1)?))
        })? {
            let (id, person) = row?;
            people.entry(id).or_default().push(person);
        }
        let ids: Vec<u32> = verses.keys().chain(people.keys()).copied().collect();
        let mut found: Vec<ChapterArtifact> = self
            .query_artifacts(Some(&ids))?
            .into_iter()
            .map(|artifact| ChapterArtifact {
                verses: verses.remove(&artifact.id).unwrap_or_default(),
                people: people.remove(&artifact.id).unwrap_or_default(),
                artifact,
            })
            .collect();
        // Passages first, by verse; then the rest in time order, as fetched.
        found.sort_by_key(|f| (f.verses.is_empty(), f.verses.first().copied().unwrap_or(0)));
        Ok(found)
    }

    /// The finds naming a person, oldest first: for a person's page.
    pub fn person_artifacts(&self, entity_id: u32) -> rusqlite::Result<Vec<Artifact>> {
        if !self.has_artifacts()? {
            return Ok(Vec::new());
        }
        let ids: Vec<u32> = self
            .conn()
            .prepare_cached(
                "SELECT DISTINCT artifact_id FROM data.artifact_attestation WHERE entity_id = ?1",
            )?
            .query_map([entity_id], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        self.query_artifacts(Some(&ids))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn bible() -> Option<Bible> {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("haqor.db").exists() {
            eprintln!("skipping: data/haqor.db not generated in this checkout");
            return None;
        }
        let bible = Bible::open(&data).unwrap();
        if !bible.has_artifacts().unwrap() {
            eprintln!("skipping: haqor.db has no artifact table");
            return None;
        }
        Some(bible)
    }

    #[test]
    fn every_find_is_listed_oldest_first_with_its_links() {
        let Some(bible) = bible() else { return };
        let Archaeology { artifacts, sources } = bible.artifacts().unwrap();
        assert!(artifacts.len() >= 50, "{}", artifacts.len());
        assert!(artifacts.windows(2).all(|w| w[0].year <= w[1].year));
        let mesha = artifacts.iter().find(|a| a.key == "mesha-stele").unwrap();
        assert_eq!(mesha.attestations[0].person.name, "Mesha");
        assert_eq!(mesha.attestations[0].strength, EvidenceStrength::Strong);
        assert_eq!(mesha.found_place.as_ref().unwrap().name, "Dibon");
        assert_eq!(mesha.passages[0].passage.first.chapter, 3);
        // Every link cites a source the list gives.
        assert!(artifacts.iter().all(|a| {
            a.attestations
                .iter()
                .map(|t| &t.source)
                .chain(a.passages.iter().map(|p| &p.source))
                .all(|s| sources.iter().any(|source| &source.id == s))
        }));
    }

    #[test]
    fn a_chapter_has_its_passages_first_then_its_people() {
        let Some(bible) = bible() else { return };
        // 2 Kings 18: Sennacherib's annals and the Lachish reliefs by passage;
        // Hezekiah's bulla by Hezekiah.
        let found = bible.chapter_artifacts(11, 18).unwrap();
        let keys: Vec<&str> = found.iter().map(|f| f.artifact.key.as_str()).collect();
        let at = |key: &str| keys.iter().position(|k| *k == key).unwrap();
        assert!(at("sennacherib-annals") < at("hezekiah-bulla"), "{keys:?}");
        let annals = &found[at("sennacherib-annals")];
        assert_eq!(annals.verses.first(), Some(&13));
        assert_eq!(annals.verses.last(), Some(&37), "{:?}", annals.verses);
        let bulla = &found[at("hezekiah-bulla")];
        assert!(bulla.verses.is_empty());
        assert!(bulla.people.iter().any(|p| p.name == "Hezekiah"));
        // A chapter no find bears on.
        assert!(bible.chapter_artifacts(3, 1).unwrap().is_empty());
        // The Isaiah scroll covers every verse of each chapter.
        let isaiah_53 = bible.chapter_artifacts(12, 53).unwrap();
        let scroll = isaiah_53
            .iter()
            .find(|f| f.artifact.key == "isaiah-scroll")
            .unwrap();
        assert_eq!(scroll.verses, (1..=12).collect::<Vec<u8>>());
    }

    #[test]
    fn a_person_has_the_finds_naming_them() {
        let Some(bible) = bible() else { return };
        let id: u32 = bible
            .conn()
            .query_row(
                "SELECT entity_id FROM data.name_entity WHERE name = 'Hezekiah' \
                 AND description LIKE 'King%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let keys: Vec<String> = bible
            .person_artifacts(id)
            .unwrap()
            .into_iter()
            .map(|a| a.key)
            .collect();
        assert!(keys.contains(&"hezekiah-bulla".to_string()), "{keys:?}");
        assert!(keys.contains(&"sennacherib-annals".to_string()), "{keys:?}");
    }
}
