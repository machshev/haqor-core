//! OpenBible.info's Bible geocoding data (CC BY 4.0): where each place named
//! in the Bible is thought to have been, every identification scholarship has
//! made with how confident it is in each.
//!
//! `scripts/fetch-openbible-geocoding.sh` downloads its `ancient.jsonl` (11
//! MB, mostly image credits, Wikidata links and the like) and [`prepare`]
//! keeps only the positions, in `src_texts/OpenBible-Geocoding/places.tsv`.
//! A TIPNR record finds its place by the OpenBible name it gives
//! (`Bethlehem 1`), or failing that by the TIPNR keys OpenBible links its
//! places to (see [`crate::tipnr`]).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::stepbible::name_key;

/// `src_texts/OpenBible-Geocoding`.
pub fn source_dir(src_texts: &Path) -> PathBuf {
    src_texts.join("OpenBible-Geocoding")
}

/// The prepared file: one line per identification, a place's best first:
/// the place's OpenBible name, the TIPNR keys it links to (comma separated),
/// latitude, longitude, confidence (OpenBible's current
/// score, 0 to 1000), the kind of place (`settlement`, `river`, `region`, …)
/// and the modern location it is identified with.
const PREPARED: &str = "places.tsv";

/// A place's position as one identification has it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub confidence: u16,
    pub kind: String,
    pub label: String,
}

/// One place of OpenBible's and its identifications, best first.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Place {
    pub name: String,
    pub keys: Vec<String>,
    pub locations: Vec<Location>,
}

/// Plain text from OpenBible's description markup, which wraps the names of
/// modern locations in tags: `<modern id="m2bcf45">Tel Avdon</modern>`.
fn plain(markup: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in markup.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            '\t' | '\n' => out.push(' '),
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One ancient place, with its located identifications.
fn place(line: &str) -> Result<Place> {
    let place: Value = serde_json::from_str(line)?;
    let name = place["friendly_id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let keys: Vec<String> = place["linked_data"]
        .as_object()
        .into_iter()
        .flat_map(|links| links.values())
        .filter_map(|link| link["id"].as_str())
        .filter(|id| id.contains('@'))
        .map(name_key)
        .collect();
    let mut locations = Vec::new();
    for identification in place["identifications"].as_array().into_iter().flatten() {
        let Some(resolution) = identification["resolutions"].get(0) else {
            continue;
        };
        let Some((longitude, latitude)) = resolution["lonlat"]
            .as_str()
            .and_then(|lonlat| lonlat.split_once(','))
            .and_then(|(lon, lat)| Some((lon.parse().ok()?, lat.parse().ok()?)))
        else {
            continue;
        };
        locations.push(Location {
            latitude,
            longitude,
            confidence: identification["score"]["time_total"]
                .as_u64()
                .unwrap_or_default()
                .min(1000) as u16,
            kind: resolution["type"].as_str().unwrap_or_default().to_string(),
            label: plain(identification["description"].as_str().unwrap_or_default()),
        });
    }
    // OpenBible lists them best first already; a stable sort keeps its order
    // among equals.
    locations.sort_by_key(|l| std::cmp::Reverse(l.confidence));
    Ok(Place {
        name,
        keys,
        locations,
    })
}

/// `db prepare openbible-geocoding`: read `ancient.jsonl` from `from` and
/// write the prepared positions into `out_dir`. Returns the number of places
/// written.
pub fn prepare(from: &Path, out_dir: &Path) -> Result<usize> {
    let path = from.join("ancient.jsonl");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut out = String::new();
    let mut places = 0;
    for (number, line) in text.lines().enumerate() {
        let place = place(line)
            .with_context(|| format!("reading place {} of {}", number + 1, path.display()))?;
        if place.locations.is_empty() || place.name.contains(['\t', ',']) {
            continue;
        }
        places += 1;
        let keys = place.keys.join(",");
        for l in &place.locations {
            out.push_str(&format!(
                "{}\t{keys}\t{}\t{}\t{}\t{}\t{}\n",
                place.name, l.latitude, l.longitude, l.confidence, l.kind, l.label
            ));
        }
    }
    std::fs::create_dir_all(out_dir)?;
    let path = out_dir.join(PREPARED);
    std::fs::write(&path, out).with_context(|| format!("writing {}", path.display()))?;
    Ok(places)
}

/// The prepared places, in OpenBible's order.
pub(crate) fn read_prepared(src_texts: &Path) -> Result<Vec<Place>> {
    let path = source_dir(src_texts).join(PREPARED);
    let text = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "reading {} (run scripts/fetch-openbible-geocoding.sh)",
            path.display()
        )
    })?;
    let mut places: Vec<Place> = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        let [name, keys, latitude, longitude, confidence, kind, label] = fields[..] else {
            bail!("an unreadable line in {}: {line:?}", path.display());
        };
        if places.last().is_none_or(|p| p.name != name) {
            places.push(Place {
                name: name.to_string(),
                keys: keys
                    .split(',')
                    .filter(|k| !k.is_empty())
                    .map(str::to_string)
                    .collect(),
                locations: Vec::new(),
            });
        }
        places.last_mut().unwrap().locations.push(Location {
            latitude: latitude.parse()?,
            longitude: longitude.parse()?,
            confidence: confidence.parse()?,
            kind: kind.to_string(),
            label: label.to_string(),
        });
    }
    Ok(places)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_positions_by_tipnr_key() {
        let line = r#"{"friendly_id":"Abdon","linked_data":{"s1":{"id":"Abdon@Jos.21.30-1Ch"},"s2":{"id":"Q123"}},"identifications":[{"description":"<modern id=\"m2\">Khirbet Abda</modern>","score":{"time_total":120},"resolutions":[{"lonlat":"35.2,33.1","type":"ruin"}]},{"description":"<modern id=\"m1\">Tel Avdon</modern>","score":{"time_total":826},"resolutions":[{"lonlat":"35.161916,33.047692","type":"settlement"}]},{"description":"unknown","score":{"time_total":54},"resolutions":[]}]}"#;
        let Place {
            name,
            keys,
            locations,
        } = place(line).unwrap();
        assert_eq!(name, "Abdon");
        assert_eq!(keys, ["Abdon@Jos.21.30"]);
        assert_eq!(locations.len(), 2);
        assert_eq!(locations[0].label, "Tel Avdon");
        assert_eq!(locations[0].confidence, 826);
        assert_eq!(locations[0].latitude, 33.047692);
        assert_eq!(locations[0].kind, "settlement");
    }
}
