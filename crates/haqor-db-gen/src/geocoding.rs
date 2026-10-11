//! OpenBible.info's Bible geocoding data (CC BY 4.0; its OpenStreetMap
//! geometry ODbL): where each place named in the Bible is thought to have
//! been, every identification scholarship has made with how confident it is
//! in each, and for a region, a river or the like, the ground it covers or the
//! course it runs.
//!
//! `scripts/fetch-openbible-geocoding.sh` checks out the dataset (its
//! `data/ancient.jsonl`, 11 MB, mostly image credits, Wikidata links and the
//! like, and a GeoJSON file for each shape) and [`prepare`] keeps only the
//! positions and shapes, in `src_texts/OpenBible-Geocoding/places.tsv`, and
//! from `data/modern.jsonl` and `data/image.jsonl` the openly licensed
//! photographs of each modern location and its Google views, in `images.tsv`
//! (see [`location_images`]). A TIPNR record finds its place by the OpenBible
//! name it gives (`Bethlehem 1`), or failing that by the TIPNR keys OpenBible
//! links its places to (see [`crate::tipnr`]).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use haqor_core::names::PlaceShape;
use serde_json::Value;

use crate::stepbible::name_key;

/// `src_texts/OpenBible-Geocoding`.
pub fn source_dir(src_texts: &Path) -> PathBuf {
    src_texts.join("OpenBible-Geocoding")
}

/// The prepared file: one line per identification, a place's best first:
/// the place's OpenBible name, the TIPNR keys it links to (comma separated),
/// latitude, longitude, confidence (OpenBible's current
/// score, 0 to 1000), the kind of place (`settlement`, `river`, `region`, …),
/// the modern location it is identified with, and its shape
/// ([`PlaceShape::encode`]), empty for a point.
const PREPARED: &str = "places.tsv";

/// A place's position as one identification has it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub confidence: u16,
    pub kind: String,
    pub label: String,
    pub shape: Option<PlaceShape>,
    /// OpenBible's id of the modern location it is identified with (`m9b8daa`),
    /// which [`Image`]s are keyed by; empty where it gives none.
    pub modern: String,
}

/// One place of OpenBible's and its identifications, best first.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Place {
    pub name: String,
    pub keys: Vec<String>,
    pub locations: Vec<Location>,
}

/// The prepared images: one line per image of a modern location, its best
/// first: the location's OpenBible id, the role (`photo` for a photograph on
/// Wikimedia Commons, `view` for a link to a view of the site on Google Maps:
/// Street View, a photosphere or an aerial view), what it shows, the page to
/// credit and open, the Commons thumbnail URL with `####` for its width (empty
/// for a view), the original's width and height, the credit, the licence
/// (OpenBible's code: `CC-BY-SA-4.0`, `PD`, `GFDL`, …), and up to three
/// colours to stand in for it while it loads (`#94abd4,#7e8592,#575451`).
const PREPARED_IMAGES: &str = "images.tsv";

/// An image of a modern location, as the prepared images file has it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Image {
    pub role: String,
    pub description: String,
    pub page_url: String,
    pub thumbnail_url: String,
    pub width: u32,
    pub height: u32,
    pub credit: String,
    pub license: String,
    pub placeholder: String,
}

/// The images OpenBible gives a modern location (`media` in
/// `data/modern.jsonl`, each described in `data/image.jsonl`) that the app
/// may show: its thumbnail, alternates and those taken nearby, where the
/// image is on Wikimedia Commons under an open licence, and its Google views
/// as links. Satellite tiles (in `thumbnails.zip` only, and the app has a
/// map) and copyrighted photographs are left out.
fn location_images(
    location: &Value,
    images: &HashMap<String, Value>,
    placeholders: &HashMap<String, String>,
) -> Vec<Image> {
    let media = &location["media"];
    let id = location["id"].as_str().unwrap_or_default();
    let mut out = Vec::new();
    let photos = media["thumbnail"]
        .as_object()
        .map(|_| &media["thumbnail"])
        .into_iter()
        .chain(media["alternate"].as_array().into_iter().flatten())
        .chain(media["near"].as_array().into_iter().flatten());
    for entry in photos {
        let Some(image) = entry["image_id"].as_str().and_then(|i| images.get(i)) else {
            continue;
        };
        let license = image["license"].as_str().unwrap_or_default();
        let pattern = image["thumbnail_url_pattern"].as_str().unwrap_or_default();
        if matches!(license, "" | "copyright" | "sentinel") || !pattern.contains("####") {
            continue;
        }
        out.push(Image {
            role: "photo".into(),
            description: plain(entry["description"].as_str().unwrap_or_default()),
            page_url: image["credit_url"]
                .as_str()
                .or(image["url"].as_str())
                .unwrap_or_default()
                .to_string(),
            thumbnail_url: pattern.to_string(),
            width: image["width"].as_u64().unwrap_or_default() as u32,
            height: image["height"].as_u64().unwrap_or_default() as u32,
            credit: image["credit"].as_str().unwrap_or_default().to_string(),
            license: license.to_string(),
            placeholder: entry["placeholder"]
                .as_str()
                .map(str::to_string)
                .or_else(|| {
                    placeholders
                        .get(&format!("{id}.{}", image["id"].as_str()?))
                        .cloned()
                })
                .unwrap_or_default(),
        });
    }
    for entry in media["google"].as_array().into_iter().flatten() {
        let Some(url) = entry["image_id"]
            .as_str()
            .and_then(|i| images.get(i))
            .and_then(|image| image["url"].as_str())
        else {
            continue;
        };
        out.push(Image {
            role: "view".into(),
            description: plain(entry["description"].as_str().unwrap_or_default()),
            page_url: url.to_string(),
            thumbnail_url: String::new(),
            width: 0,
            height: 0,
            credit: String::new(),
            license: String::new(),
            placeholder: String::new(),
        });
    }
    out
}

/// `prepare`'s images, from `data/modern.jsonl` and `data/image.jsonl`, as
/// the lines of the prepared file. Returns them and how many locations have
/// any.
fn prepare_images(from: &Path) -> Result<(String, usize)> {
    let read = |name: &str| {
        let path = from.join(name);
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
    };
    let mut images = HashMap::new();
    // An image's placeholder colours, by location and image: only its
    // thumbnails have them.
    let mut placeholders = HashMap::new();
    for line in read("data/image.jsonl")?.lines() {
        let image: Value = serde_json::from_str(line)?;
        let Some(id) = image["id"].as_str().map(str::to_string) else {
            continue;
        };
        for (location, thumbnail) in image["thumbnails"].as_object().into_iter().flatten() {
            if let Some(colours) = thumbnail["placeholder"].as_str() {
                placeholders.insert(format!("{location}.{id}"), colours.to_string());
            }
        }
        images.insert(id, image);
    }
    let mut out = String::new();
    let mut located = 0;
    for line in read("data/modern.jsonl")?.lines() {
        let location: Value = serde_json::from_str(line)?;
        let id = location["id"].as_str().unwrap_or_default();
        let found = location_images(&location, &images, &placeholders);
        located += usize::from(!found.is_empty());
        for i in found {
            let fields = [
                &i.description,
                &i.page_url,
                &i.thumbnail_url,
                &i.credit,
                &i.license,
                &i.placeholder,
            ];
            if fields.iter().any(|f| f.contains(['\t', '\n'])) {
                bail!("a tab or newline in an image of {id}: {i:?}");
            }
            out.push_str(&format!(
                "{id}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                i.role,
                i.description,
                i.page_url,
                i.thumbnail_url,
                i.width,
                i.height,
                i.credit,
                i.license,
                i.placeholder
            ));
        }
    }
    Ok((out, located))
}

/// The prepared images, by modern location id, best first. Empty where the
/// file is missing, as in a checkout prepared before there were images.
pub(crate) fn read_images(src_texts: &Path) -> Result<HashMap<String, Vec<Image>>> {
    let path = source_dir(src_texts).join(PREPARED_IMAGES);
    let mut out: HashMap<String, Vec<Image>> = HashMap::new();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        let [
            location,
            role,
            description,
            page_url,
            thumbnail_url,
            width,
            height,
            credit,
            license,
            placeholder,
        ] = fields[..]
        else {
            bail!("an unreadable line in {}: {line:?}", path.display());
        };
        out.entry(location.to_string()).or_default().push(Image {
            role: role.to_string(),
            description: description.to_string(),
            page_url: page_url.to_string(),
            thumbnail_url: thumbnail_url.to_string(),
            width: width.parse()?,
            height: height.parse()?,
            credit: credit.to_string(),
            license: license.to_string(),
            placeholder: placeholder.to_string(),
        });
    }
    Ok(out)
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

/// OpenBible's shapes, by geometry id: `data/geometry.jsonl` says what each
/// is and names its GeoJSON file in `geometry/`.
pub(crate) struct Geometries {
    dir: PathBuf,
    entries: HashMap<String, Value>,
}

impl Geometries {
    /// Read the index from a checkout of the dataset.
    pub(crate) fn read(checkout: &Path) -> Result<Geometries> {
        let path = checkout.join("data/geometry.jsonl");
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let mut entries = HashMap::new();
        for line in text.lines() {
            let entry: Value = serde_json::from_str(line)?;
            if let Some(id) = entry["id"].as_str() {
                entries.insert(id.to_string(), entry);
            }
        }
        Ok(Geometries {
            dir: checkout.join("geometry"),
            entries,
        })
    }

    /// No shapes, for a test.
    #[cfg(test)]
    fn none() -> Geometries {
        Geometries {
            dir: PathBuf::new(),
            entries: HashMap::new(),
        }
    }

    fn file(&self, name: &str) -> Result<Value> {
        let path = self.dir.join(name);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        Ok(serde_json::from_str(&text)?)
    }

    /// The shape geometry `id` draws: a region's rough bounds (the band of
    /// isobands OpenBible suggests as one), a lake's or a site's outline, or
    /// a river's or road's course. `None` for one with no such shape, as a
    /// probability surface.
    pub(crate) fn shape(&self, id: &str) -> Result<Option<PlaceShape>> {
        let Some(entry) = self.entries.get(id) else {
            return Ok(None);
        };
        let file = || entry["geojson_file"].as_str().context("no geojson_file");
        let shape = match entry["geometry"].as_str().unwrap_or_default() {
            "rough_boundary" | "polygon" => {
                PlaceShape::Area(rings(&self.file(file()?)?["geometry"]))
            }
            "path" | "rough_path" => PlaceShape::Line(lines(&self.file(file()?)?["geometry"])),
            "isobands" => {
                if let Some(boundary) = entry["suggested"]["rough_boundary"].as_array() {
                    let ring = boundary
                        .iter()
                        .filter_map(|p| {
                            let (lon, lat) = p.as_str()?.split_once(',')?;
                            Some([lon.parse().ok()?, lat.parse().ok()?])
                        })
                        .collect();
                    PlaceShape::Area(vec![ring])
                } else {
                    // The widest band's outline: the bands nest, so the
                    // largest ring bounds them all.
                    let name = entry["isobands_geojson_file"]
                        .as_str()
                        .context("no isobands_geojson_file")?;
                    let largest = rings(&self.file(name)?["geometry"])
                        .into_iter()
                        .max_by(|a, b| area(a).total_cmp(&area(b)));
                    PlaceShape::Area(largest.into_iter().collect())
                }
            }
            _ => return Ok(None),
        };
        Ok(simplify(shape))
    }
}

fn point(value: &Value) -> Option<[f64; 2]> {
    Some([value.get(0)?.as_f64()?, value.get(1)?.as_f64()?])
}

fn points(value: &Value) -> Vec<[f64; 2]> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(point)
        .collect()
}

/// A GeoJSON polygon's or multipolygon's outer rings; holes are left out.
fn rings(geometry: &Value) -> Vec<Vec<[f64; 2]>> {
    let coordinates = &geometry["coordinates"];
    match geometry["type"].as_str() {
        Some("Polygon") => vec![points(&coordinates[0])],
        Some("MultiPolygon") => coordinates
            .as_array()
            .into_iter()
            .flatten()
            .map(|polygon| points(&polygon[0]))
            .collect(),
        _ => Vec::new(),
    }
}

/// A GeoJSON line's or multiline's parts.
fn lines(geometry: &Value) -> Vec<Vec<[f64; 2]>> {
    let coordinates = &geometry["coordinates"];
    match geometry["type"].as_str() {
        Some("LineString") => vec![points(coordinates)],
        Some("MultiLineString") => coordinates
            .as_array()
            .into_iter()
            .flatten()
            .map(points)
            .collect(),
        _ => Vec::new(),
    }
}

/// A ring's area in square degrees, by the shoelace formula.
fn area(ring: &[[f64; 2]]) -> f64 {
    let mut sum = 0.0;
    for (i, [x0, y0]) in ring.iter().enumerate() {
        let [x1, y1] = ring[(i + 1) % ring.len()];
        sum += x0 * y1 - x1 * y0;
    }
    sum.abs() / 2.0
}

/// The shape with points closer than a map can show dropped (Douglas and
/// Peucker). A course keeps detail to a fifteen-hundredth of its extent,
/// between about 20 m and 1 km, so the Jordan keeps its meanders and the Nile
/// is not thousands of points; an area, drawn as a tint under the places in
/// it, to a four-hundredth, between about 30 m and 5 km. Parts too small to
/// draw are dropped; `None` when none is left.
fn simplify(shape: PlaceShape) -> Option<PlaceShape> {
    let all = shape.parts().iter().flatten();
    let (mut west, mut south) = (f64::INFINITY, f64::INFINITY);
    let (mut east, mut north) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for [lon, lat] in all {
        west = west.min(*lon);
        east = east.max(*lon);
        south = south.min(*lat);
        north = north.max(*lat);
    }
    let extent = (east - west).max(north - south);
    let tolerance = match shape {
        PlaceShape::Area(_) => (extent / 400.0).clamp(0.0003, 0.05),
        PlaceShape::Line(_) => (extent / 1500.0).clamp(0.0002, 0.01),
    };
    let keep = |parts: &[Vec<[f64; 2]>], least: usize| -> Vec<Vec<[f64; 2]>> {
        parts
            .iter()
            .map(|part| douglas_peucker(part, tolerance))
            .filter(|part| part.len() >= least)
            .collect()
    };
    let shape = match &shape {
        PlaceShape::Area(parts) => PlaceShape::Area(keep(parts, 3)),
        PlaceShape::Line(parts) => PlaceShape::Line(keep(parts, 2)),
    };
    (!shape.parts().is_empty()).then_some(shape)
}

fn douglas_peucker(points: &[[f64; 2]], tolerance: f64) -> Vec<[f64; 2]> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut stack = vec![(0, points.len() - 1)];
    while let Some((first, last)) = stack.pop() {
        let [ax, ay] = points[first];
        let [bx, by] = points[last];
        let (dx, dy) = (bx - ax, by - ay);
        let length = (dx * dx + dy * dy).sqrt();
        let mut farthest = (0.0, first);
        for (i, [px, py]) in points.iter().enumerate().take(last).skip(first + 1) {
            let distance = if length == 0.0 {
                ((px - ax).powi(2) + (py - ay).powi(2)).sqrt()
            } else {
                ((px - ax) * dy - (py - ay) * dx).abs() / length
            };
            if distance > farthest.0 {
                farthest = (distance, i);
            }
        }
        if farthest.0 > tolerance {
            keep[farthest.1] = true;
            stack.push((first, farthest.1));
            stack.push((farthest.1, last));
        }
    }
    points
        .iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}

/// The shape a resolution has, where the place is the shape itself (no
/// modifier), the region about the point (`>`) or somewhere within it (`<`).
/// A place near a feature or along a river has the feature's shape, not its
/// own, and is left a point.
fn resolution_shape(resolution: &Value, geometries: &Geometries) -> Result<Option<PlaceShape>> {
    if !matches!(
        resolution["modifier"].as_str().unwrap_or_default(),
        "" | ">" | "<"
    ) {
        return Ok(None);
    }
    // OpenStreetMap's exact geometry before OpenBible's own rougher one.
    for key in ["precise_geometry_id", "geometry_id"] {
        if let Some(id) = resolution[key].as_str()
            && let Some(shape) = geometries.shape(id)?
        {
            return Ok(Some(shape));
        }
    }
    Ok(None)
}

/// One ancient place, with its located identifications.
fn place(line: &str, geometries: &Geometries) -> Result<Place> {
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
            shape: resolution_shape(resolution, geometries)?,
            modern: resolution["modern_basis_id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
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

/// `db prepare openbible-geocoding`: read a checkout of the dataset from
/// `from` and write the prepared positions and images into `out_dir`. Returns the number
/// of places written.
pub fn prepare(from: &Path, out_dir: &Path) -> Result<usize> {
    let path = from.join("data/ancient.jsonl");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let geometries = Geometries::read(from)?;
    let mut out = String::new();
    let mut places = 0;
    for (number, line) in text.lines().enumerate() {
        let place = place(line, &geometries)
            .with_context(|| format!("reading place {} of {}", number + 1, path.display()))?;
        if place.locations.is_empty() || place.name.contains(['\t', ',']) {
            continue;
        }
        places += 1;
        let keys = place.keys.join(",");
        for l in &place.locations {
            out.push_str(&format!(
                "{}\t{keys}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                place.name,
                l.latitude,
                l.longitude,
                l.confidence,
                l.kind,
                l.label,
                l.shape.as_ref().map(PlaceShape::encode).unwrap_or_default(),
                l.modern
            ));
        }
    }
    std::fs::create_dir_all(out_dir)?;
    let path = out_dir.join(PREPARED);
    std::fs::write(&path, out).with_context(|| format!("writing {}", path.display()))?;
    let (images, located) = prepare_images(from)?;
    let path = out_dir.join(PREPARED_IMAGES);
    std::fs::write(&path, images).with_context(|| format!("writing {}", path.display()))?;
    log::info!("Prepared the images of {located} modern locations");
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
        let [
            name,
            keys,
            latitude,
            longitude,
            confidence,
            kind,
            label,
            shape,
            modern,
        ] = fields[..]
        else {
            bail!("an unreadable line in {}: {line:?}", path.display());
        };
        let decoded = PlaceShape::decode(shape);
        if !shape.is_empty() && decoded.is_none() {
            bail!("an unreadable shape in {}: {line:?}", path.display());
        }
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
            shape: decoded,
            modern: modern.to_string(),
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
        } = place(line, &Geometries::none()).unwrap();
        assert_eq!(name, "Abdon");
        assert_eq!(keys, ["Abdon@Jos.21.30"]);
        assert_eq!(locations.len(), 2);
        assert_eq!(locations[0].label, "Tel Avdon");
        assert_eq!(locations[0].confidence, 826);
        assert_eq!(locations[0].latitude, 33.047692);
        assert_eq!(locations[0].kind, "settlement");
        assert_eq!(locations[0].shape, None);
    }

    #[test]
    fn keeps_open_photos_and_google_views() {
        let images: HashMap<String, Value> = [
            r#"{"id":"i1","license":"CC-BY-SA-4.0","credit":"Avi1111","credit_url":"https://commons.wikimedia.org/wiki/File:Tel_Malut.jpg","thumbnail_url_pattern":"https://upload.wikimedia.org/wikipedia/commons/thumb/4/4d/Tel_Malut.jpg/####px-Tel_Malut.jpg","width":4160,"height":3120}"#,
            r##"{"id":"i2","license":"PD","credit":"Someone","credit_url":"https://commons.wikimedia.org/wiki/File:Wall.jpg","thumbnail_url_pattern":"https://upload.wikimedia.org/wikipedia/commons/thumb/a/ab/Wall.jpg/####px-Wall.jpg","width":800,"height":600,"thumbnails":{"m1":{"placeholder":"#111111,#222222"}}}"##,
            r#"{"id":"i3","license":"copyright","url":"https://holylandphotos.org/browse.asp?img=X"}"#,
            r#"{"id":"i4","license":"sentinel","credit":"Contains modified Copernicus Sentinel data 2019"}"#,
            r#"{"id":"i5","license":"copyright","url":"https://www.google.com/maps/@31.8,35.5,3a"}"#,
        ]
        .into_iter()
        .map(|line| {
            let image: Value = serde_json::from_str(line).unwrap();
            (image["id"].as_str().unwrap().to_string(), image)
        })
        .collect();
        let placeholders = HashMap::from([("m1.i2".to_string(), "#111111,#222222".to_string())]);
        let location: Value = serde_json::from_str(
            r##"{"id":"m1","media":{
                "thumbnail":{"image_id":"i1","description":"summit of <modern id=\"m1\">Tell Melat</modern>","placeholder":"#80a0bc,#4c5959"},
                "alternate":[{"image_id":"i2","description":"wall at <modern id=\"m1\">Tell Melat</modern>"},{"image_id":"i4","description":"satellite view"}],
                "copyrighted":[{"image_id":"i3","description":"panorama"}],
                "near":[{"image_id":"i3","description":"near"}],
                "google":[{"image_id":"i5","description":"Google Streetview of <modern id=\"m1\">Tell Melat</modern>","role":"google_streetview"}]
            }}"##,
        )
        .unwrap();
        let found = location_images(&location, &images, &placeholders);
        let summary: Vec<_> = found
            .iter()
            .map(|i| {
                (
                    i.role.as_str(),
                    i.description.as_str(),
                    i.placeholder.as_str(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("photo", "summit of Tell Melat", "#80a0bc,#4c5959"),
                ("photo", "wall at Tell Melat", "#111111,#222222"),
                ("view", "Google Streetview of Tell Melat", ""),
            ]
        );
        assert_eq!(found[0].license, "CC-BY-SA-4.0");
        assert_eq!(found[0].width, 4160);
        assert_eq!(
            found[0].page_url,
            "https://commons.wikimedia.org/wiki/File:Tel_Malut.jpg"
        );
        assert_eq!(
            found[2].page_url,
            "https://www.google.com/maps/@31.8,35.5,3a"
        );
    }

    #[test]
    fn keeps_the_shapes_of_regions_and_rivers() {
        let dir = std::env::temp_dir().join(format!("haqor-geometry-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("data")).unwrap();
        std::fs::create_dir_all(dir.join("geometry")).unwrap();
        std::fs::write(
            dir.join("data/geometry.jsonl"),
            [
                r#"{"geometry":"isobands","id":"g1","suggested":{"rough_boundary":["35,31","36,31","35.5,32","35,31"]}}"#,
                r#"{"geometry":"path","id":"g2","geojson_file":"g2.geometry.geojson"}"#,
                r#"{"geometry":"probability","id":"g3"}"#,
            ]
            .join("\n"),
        )
        .unwrap();
        // A straight run of points, the middle ones dropped.
        std::fs::write(
            dir.join("geometry/g2.geometry.geojson"),
            r#"{"geometry":{"type":"LineString","coordinates":[[35.5,33],[35.5,32.5],[35.5,32],[35.6,31.8]]}}"#,
        )
        .unwrap();
        let geometries = Geometries::read(&dir).unwrap();
        let line = r#"{"friendly_id":"Somewhere","identifications":[
            {"description":"","score":{"time_total":900},"resolutions":[{"lonlat":"35.5,31.5","type":"region","modifier":">","geometry_id":"g1"}]},
            {"description":"","score":{"time_total":800},"resolutions":[{"lonlat":"35.5,32.5","type":"river","precise_geometry_id":"g2","geometry_id":"g3"}]},
            {"description":"","score":{"time_total":700},"resolutions":[{"lonlat":"35.5,32.5","type":"settlement","modifier":"along","precise_geometry_id":"g2"}]},
            {"description":"","score":{"time_total":600},"resolutions":[{"lonlat":"35.5,32.5","type":"region","geometry_id":"g3"}]}
        ]}"#
        .replace('\n', "");
        let locations = place(&line, &geometries).unwrap().locations;
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            locations[0].shape,
            Some(PlaceShape::Area(vec![vec![
                [35.0, 31.0],
                [36.0, 31.0],
                [35.5, 32.0],
                [35.0, 31.0]
            ]]))
        );
        assert_eq!(
            locations[1].shape,
            Some(PlaceShape::Line(vec![vec![
                [35.5, 33.0],
                [35.5, 32.0],
                [35.6, 31.8]
            ]]))
        );
        // Along the river is not the river; a probability surface is no
        // outline.
        assert_eq!(locations[2].shape, None);
        assert_eq!(locations[3].shape, None);
    }
}
