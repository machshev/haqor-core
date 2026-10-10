//! People, places and word senses, as `haqor.db` stores them.
//!
//! STEP Bible's TIPNR tells apart the people and places that share a name —
//! the many Zechariahs, the two Bethlehems — and the build links every word
//! of the Hebrew text naming one to its record, with OpenBible.info's
//! positions for places. STEP Bible's TBESH splits a word into its senses
//! (שָׁכַב "lie down" as "sleep" or "be dead"), and the build gives each
//! occurrence its sense. See [`crate::bible::Bible::word_name`] and
//! [`crate::bible::Bible::word_sense`].

/// A word of the text: its verse and its position in the verse, from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WordAt {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
    pub position: u32,
}

/// What kind of thing a [`NameEntity`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameKind {
    Person,
    Place,
    /// A god, month, title, musical term and the like.
    Other,
}

impl NameKind {
    pub(crate) fn parse(kind: &str) -> Self {
        match kind {
            "person" => NameKind::Person,
            "place" => NameKind::Place,
            _ => NameKind::Other,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            NameKind::Person => "person",
            NameKind::Place => "place",
            NameKind::Other => "other",
        }
    }
}

/// Enough of a record to name it in a list: a relative, a place on a map.
#[derive(Clone, Debug, PartialEq)]
pub struct NameSummary {
    pub id: u32,
    pub name: String,
    pub kind: NameKind,
    /// A few words saying who or what it is ("King living at the time of
    /// Divided Monarchy"); empty for most places.
    pub description: String,
    /// A person's tribe or nation, a place's region; may be empty.
    pub origin: String,
    /// Words of the Hebrew text naming it.
    pub occurrences: u32,
}

/// One Hebrew form of a name.
#[derive(Clone, Debug, PartialEq)]
pub struct NameForm {
    pub hebrew: String,
    /// The English names translations give the form, most usual first.
    pub english: Vec<String>,
    /// How the form relates to the name: `Named`, `Spelled`, `Aramaic`,
    /// `Group` (a gentilic: Bethlehemite), `Name combined`, …
    pub significance: String,
}

/// A link from one record to another.
#[derive(Clone, Debug, PartialEq)]
pub struct NameLink {
    /// `father`, `mother`, `sibling`, `partner`, `child`, `founder` or
    /// `inhabitant`.
    pub relation: String,
    /// `a` (an ancestor rather than a parent), `d` (a people descended from
    /// them), `f` (a founder), `?` (uncertain), or empty.
    pub flag: String,
    pub other: NameSummary,
}

/// Where a place may have been.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceLocation {
    pub latitude: f64,
    pub longitude: f64,
    /// OpenBible.info's confidence in the identification, 0 to 1000 (500 and
    /// above is confident); `None` for a position with no score, from TIPNR.
    pub confidence: Option<u16>,
    /// The kind of place: `settlement`, `river`, `region`, `mountain`, …
    pub kind: String,
    /// The modern location it is identified with.
    pub label: String,
    /// The ground it covers or the course it runs, where OpenBible.info
    /// draws more than a point; `None` for a point.
    pub shape: Option<PlaceShape>,
}

/// A place drawn as more than a point: each part a list of longitude,
/// latitude pairs.
#[derive(Clone, Debug, PartialEq)]
pub enum PlaceShape {
    /// A region's rough bounds, or a lake's or a site's outline; each part a
    /// ring, its last point joined back to its first.
    Area(Vec<Vec<[f64; 2]>>),
    /// A river's, wadi's or road's course.
    Line(Vec<Vec<[f64; 2]>>),
}

impl PlaceShape {
    /// Its parts, whichever it is.
    pub fn parts(&self) -> &[Vec<[f64; 2]>] {
        match self {
            PlaceShape::Area(parts) | PlaceShape::Line(parts) => parts,
        }
    }

    /// As the database keeps it: `area:` or `line:`, then the parts split by
    /// `|`, each its points split by `;` and written `longitude,latitude`.
    pub fn encode(&self) -> String {
        let kind = match self {
            PlaceShape::Area(_) => "area",
            PlaceShape::Line(_) => "line",
        };
        let parts: Vec<String> = self
            .parts()
            .iter()
            .map(|part| {
                let points: Vec<String> = part
                    .iter()
                    .map(|[lon, lat]| format!("{},{}", round(*lon), round(*lat)))
                    .collect();
                points.join(";")
            })
            .collect();
        format!("{kind}:{}", parts.join("|"))
    }

    /// What [`PlaceShape::encode`] wrote; `None` for an empty or unreadable
    /// text.
    pub fn decode(text: &str) -> Option<PlaceShape> {
        let (kind, rest) = text.split_once(':')?;
        let parts = rest
            .split('|')
            .map(|part| {
                part.split(';')
                    .map(|point| {
                        let (lon, lat) = point.split_once(',')?;
                        Some([lon.parse().ok()?, lat.parse().ok()?])
                    })
                    .collect::<Option<Vec<_>>>()
            })
            .collect::<Option<Vec<_>>>()?;
        match kind {
            "area" => Some(PlaceShape::Area(parts)),
            "line" => Some(PlaceShape::Line(parts)),
            _ => None,
        }
    }
}

/// Degrees to five places, about a metre: finer than any shape is drawn.
fn round(degrees: f64) -> f64 {
    (degrees * 1e5).round() / 1e5
}

/// A person, place or other named thing, with all the build knows of it.
#[derive(Clone, Debug, PartialEq)]
pub struct NameEntity {
    pub summary: NameSummary,
    /// TIPNR's type: Male, Female, Group, Place, Supernatural, Title, …
    pub category: String,
    /// What the text says of it, a sentence a line.
    pub text: String,
    pub forms: Vec<NameForm>,
    pub links: Vec<NameLink>,
    /// The likeliest first; empty for all but places.
    pub locations: Vec<PlaceLocation>,
}

/// A place named in a chapter, for a map of the chapter.
#[derive(Clone, Debug, PartialEq)]
pub struct ChapterPlace {
    pub place: NameSummary,
    /// The likeliest location; a place without one is not listed.
    pub location: PlaceLocation,
    /// The verses naming it, in order.
    pub verses: Vec<u8>,
}

/// One sense of a word.
#[derive(Clone, Debug, PartialEq)]
pub struct SenseSummary {
    pub id: u32,
    /// The sense's own gloss ("be dead"): the part after the word's gloss.
    /// Empty for a word with only one sense.
    pub meaning: String,
    /// Words of the text with this sense.
    pub occurrences: u32,
}

/// The sense a word has where it stands, among the senses of its word.
#[derive(Clone, Debug, PartialEq)]
pub struct WordSense {
    /// The word's Hebrew headword.
    pub word: String,
    /// `hebrew` or `aramaic`.
    pub language: String,
    /// The word's gloss ("to lie down"), whatever its sense.
    pub gloss: String,
    pub sense: SenseSummary,
    /// Every sense of the word, this one among them, most used first. Only
    /// this one for a word with one sense.
    pub senses: Vec<SenseSummary>,
}

/// Split a stored gloss into the word's gloss and the sense's: TBESH writes
/// the two as `to lie down: be dead`.
pub(crate) fn split_gloss(gloss: &str) -> (&str, &str) {
    match gloss.split_once(": ") {
        Some((word, sense)) => (word.trim(), sense.trim()),
        None => (gloss.trim(), ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_word_and_sense_glosses() {
        assert_eq!(
            split_gloss("to lie down: be dead"),
            ("to lie down", "be dead")
        );
        assert_eq!(split_gloss("to create"), ("to create", ""));
        assert_eq!(
            split_gloss("land: country/planet"),
            ("land", "country/planet")
        );
    }

    #[test]
    fn place_shapes_round_trip() {
        let area = PlaceShape::Area(vec![
            vec![[35.1, 31.2], [35.3, 31.2], [35.2, 31.4]],
            vec![[34.0, 30.0], [34.1, 30.0], [34.0, 30.1]],
        ]);
        assert_eq!(
            area.encode(),
            "area:35.1,31.2;35.3,31.2;35.2,31.4|34,30;34.1,30;34,30.1"
        );
        assert_eq!(PlaceShape::decode(&area.encode()), Some(area));
        let line = PlaceShape::Line(vec![vec![[35.612345678, 32.7], [35.5, 31.8]]]);
        assert_eq!(line.encode(), "line:35.61235,32.7;35.5,31.8");
        assert_eq!(PlaceShape::decode(""), None);
        assert_eq!(PlaceShape::decode("circle:1,2"), None);
        assert_eq!(PlaceShape::decode("line:1,x"), None);
    }
}
