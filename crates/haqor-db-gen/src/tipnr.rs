//! Read STEP Bible's TIPNR (Translators Individualised Proper Names with all
//! References, CC BY 4.0): every person, place and other named thing in the
//! Bible, told apart — the many Zechariahs, the two Bethlehems — with their
//! family links, the forms of their names and a summary of what the text says
//! of them.
//!
//! TIPNR identifies each by a unique name (`Zechariah@2Ki.14.29-`) and by
//! disambiguated Strong's numbers, which TAHOT also tags each word with. Both
//! are used to join the two while preparing the vendored files, and the
//! Strong's numbers go no further: see [`crate::stepbible::prepare`].

use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::stepbible::name_key;

/// What kind of thing a TIPNR record names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Person,
    Place,
    /// Gods, months, titles, musical terms and the like.
    Other,
}

impl Kind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Kind::Person => "person",
            Kind::Place => "place",
            Kind::Other => "other",
        }
    }
}

/// One form a name takes in Hebrew.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Form {
    /// TIPNR's word for how the form relates to the name: `Named`, `Spelled`,
    /// `Aramaic`, `Group` (a gentilic: Bethlehemite), …
    pub significance: String,
    pub hebrew: String,
    /// The English names translations give it, most usual first.
    pub english: Vec<String>,
    /// The disambiguated Strong's numbers TAHOT tags the form's words with:
    /// join keys for preparing, never written out.
    pub strongs: Vec<String>,
}

/// A link to another record, by its key ([`name_key`]), with TIPNR's flag on
/// it, if any: `a` (an ancestor rather than a parent), `d` (a people descended
/// from them), `f` (a founder), `?` (an uncertain identification).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Link {
    pub key: String,
    pub flag: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Record {
    pub key: String,
    pub kind: Kind,
    /// The name as the English Bible most often gives it.
    pub name: String,
    /// TIPNR's Type: Male, Female, Group, Place, Supernatural, …
    pub category: String,
    /// A few words saying who or what it is ("King living at the time of
    /// Divided Monarchy"); empty for places, whose header has none.
    pub description: String,
    /// TIPNR's summary in plain text: one sentence or several, a line each.
    pub summary: String,
    /// A person's tribe or nation; a place's region.
    pub origin: String,
    /// `father`, `mother`, `sibling`, `partner`, `child`, `founder`,
    /// `inhabitant`.
    pub links: Vec<(&'static str, Link)>,
    pub forms: Vec<Form>,
    /// A place's position as TIPNR gives it (from OpenBible.info's 2007
    /// data), latitude then longitude.
    pub coordinates: Option<(f64, f64)>,
    /// The place's name in OpenBible.info's geocoding (`Bethlehem 1`), which
    /// tells apart places TIPNR gives one name; empty if it has none.
    pub openbible: String,
}

/// `text` without the Strong's numbers TIPNR now and then leaves in its prose
/// ("mother of Jezreel, Lo-ruhamah, Lo-ammi H3818"), and without the
/// parentheses left empty where one stood.
pub(crate) fn strip_strongs(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let starts = matches!(chars[i], 'H' | 'G')
            && (i == 0 || !chars[i - 1].is_alphanumeric())
            && chars.len() >= i + 5
            && chars[i + 1..i + 5].iter().all(char::is_ascii_digit);
        if starts {
            i += 5;
            while i < chars.len() && chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    let out = out.replace("()", "").replace("( )", "");
    out.split('\n')
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .map(|line| line.replace(" ,", ",").replace(" .", "."))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A record's description as prose: [`strip_strongs`], and the records it
/// names by their names, not their keys ("People from Ram@Job.32.2" is
/// "People from Ram"). A description that repeats a linked record's name and
/// Hebrew before saying what it is ("Eber@Gen.10.21-Luk עֵבֶר (Eber, ) People
/// from Eber@…") keeps only the saying.
pub(crate) fn plain_description(text: &str) -> String {
    let text = strip_strongs(text);
    let text = if text.chars().any(|c| ('\u{05D0}'..='\u{05EA}').contains(&c)) {
        text.rsplit_once(") ")
            .map_or(text.as_str(), |(_, saying)| saying)
            .to_string()
    } else {
        text
    };
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '@' {
            // A key's reference: `Gen.10.21-Luk`, `Jdg.4.11-`.
            while chars
                .peek()
                .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
            {
                chars.next();
            }
            // `Shelah@Gen.38.5-1Ch(?)`: the doubt, apart from the name.
            if chars.peek() == Some(&'(') {
                out.push(' ');
            }
            continue;
        }
        out.push(if c == '_' { ' ' } else { c });
    }
    out
}

/// Turn TIPNR's summary markup into plain text: `<ref="…">Gen.35.16</ref>`
/// and `<strong="H1035G">Bethlehem</strong>` keep only their text, `<br>`
/// starts a new line, and a closing parenthesis TIPNR leaves unopened
/// ("first mentioned at Gen.35.16)") is dropped. A leading `#`, TIPNR's
/// marker for the summary field, goes too.
pub(crate) fn plain_summary(markup: &str) -> String {
    let markup = markup.trim().trim_start_matches(['#', '3']).trim();
    let mut out = String::new();
    let mut rest = markup;
    let mut depth = 0usize;
    while let Some(c) = rest.chars().next() {
        if c == '<' {
            let end = rest.find('>').map_or(rest.len(), |e| e + 1);
            let tag = rest[..end].to_ascii_lowercase();
            if tag.starts_with("<br") {
                out.push('\n');
            }
            rest = &rest[end..];
            continue;
        }
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => {
                rest = &rest[1..];
                continue;
            }
            ')' => depth -= 1,
            _ => {}
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    strip_strongs(&out)
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .map(|line| line.trim_end_matches(';').trim().to_string())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The links in a list field: unique names separated by commas, or by `+`
/// in a parents field (`Father + Mother`).
fn links(field: &str) -> Vec<Link> {
    field
        .split([',', '+'])
        .map(str::trim)
        .filter(|item| item.contains('@'))
        .map(|item| {
            let flag = item
                .rsplit_once('(')
                .and_then(|(_, f)| f.strip_suffix(')'))
                .unwrap_or_default()
                .to_string();
            Link {
                key: name_key(item),
                flag,
            }
        })
        .collect()
}

/// A Google Maps link's position: `https://www.google.com/maps/@31.70,35.21,14z`.
fn coordinates(url: &str) -> Option<(f64, f64)> {
    let at = url.split_once("/@")?.1;
    let mut parts = at.split(',');
    let lat = parts.next()?.parse().ok()?;
    let lon = parts.next()?.parse().ok()?;
    Some((lat, lon))
}

/// The English names in a translated-names field:
/// `Zechariah =ESV,NIV; Zachariah =KJV` gives Zechariah and Zachariah.
fn english_names(field: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for part in field.split(';') {
        let name = part.split('=').next().unwrap_or(part);
        // A name joined with another word is marked with that word's number:
        // `Caleb Ephrathah<H3613>`.
        let name = name.split('<').next().unwrap_or(name);
        let name = strip_strongs(name);
        let name = name.trim();
        let name = name.trim_end_matches(',').trim();
        if name.is_empty() || name.starts_with('[') || names.iter().any(|n| n == name) {
            continue;
        }
        names.push(name.to_string());
    }
    names
}

/// A form line's Hebrew field: `H2148P«H2148a=זְכַרְיָהוּ`, or for a name
/// combined of several words, a part for each joined by `+`
/// (`H1391«H1391=גִבְעוֹן+H0001I«H0001=אֲבִי`, "father of Gibeon").
fn form(significance: &str, field: &str, english: &str) -> Option<Form> {
    let mut hebrew = Vec::new();
    let mut strongs = Vec::new();
    for part in field.split('+') {
        let (tags, word) = part.split_once('=')?;
        let strong = tags.split('«').next()?.trim();
        if !strong.starts_with('H') {
            return None;
        }
        strongs.push(strong.to_string());
        hebrew.push(word.trim());
    }
    Some(Form {
        significance: significance.to_string(),
        hebrew: hebrew.join(" "),
        english: english_names(english),
        strongs,
    })
}

/// Read every record of a TIPNR file.
pub(crate) fn parse(text: &str) -> Result<Vec<Record>> {
    let mut records: Vec<Record> = Vec::new();
    let mut kind = None;
    // Whether the line after a section marker, the record's header, is due.
    let mut header_due = false;
    // Whether the lines since the last header belong to a record: the field
    // descriptions at the head of the file show sample form lines too.
    let mut in_record = false;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(section) = line.strip_prefix("$==========") {
            let section = section.split('\t').next().unwrap_or_default().trim();
            kind = match section {
                "PERSON(s)" => Some(Kind::Person),
                "PLACE" | "PERSON+PLACE" => Some(Kind::Place),
                "OTHER" => Some(Kind::Other),
                _ => None,
            };
            header_due = kind.is_some();
            in_record = false;
            continue;
        }
        let Some(kind) = kind else { continue };
        let fields: Vec<&str> = line.split('\t').collect();
        let field = |n: usize| fields.get(n).copied().unwrap_or_default().trim();
        if header_due {
            header_due = false;
            // The field descriptions at the head of the file use the same
            // markers; their "header" names no record.
            if !field(0).contains('@') {
                continue;
            }
            let key = name_key(field(0));
            let name = key.split('@').next().unwrap_or_default().replace('_', " ");
            let mut record = Record {
                key,
                kind,
                name,
                category: field(8).to_string(),
                description: String::new(),
                summary: plain_summary(field(7)),
                origin: strip_strongs(field(6).trim_end_matches('>'))
                    .trim()
                    .to_string(),
                links: Vec::new(),
                forms: Vec::new(),
                coordinates: None,
                openbible: String::new(),
            };
            match kind {
                Kind::Person => {
                    record.description = plain_description(field(1));
                    let mut parents = field(2).splitn(2, '+');
                    for (relation, part) in ["father", "mother"].into_iter().zip(parents.by_ref()) {
                        record
                            .links
                            .extend(links(part).into_iter().map(|l| (relation, l)));
                    }
                    for (relation, n) in [("sibling", 3), ("partner", 4), ("child", 5)] {
                        record
                            .links
                            .extend(links(field(n)).into_iter().map(|l| (relation, l)));
                    }
                }
                Kind::Place => {
                    record
                        .links
                        .extend(links(field(2)).into_iter().map(|l| ("founder", l)));
                    record
                        .links
                        .extend(links(field(3)).into_iter().map(|l| ("inhabitant", l)));
                    record.coordinates = coordinates(field(4));
                    // `Abronah= near Ezion-geber (…)`: the name, then what TIPNR says it
                    // is near.
                    let openbible = field(1).split('=').next().unwrap_or_default();
                    record.openbible = strip_strongs(openbible).trim().replace('_', " ");
                }
                Kind::Other => record.description = plain_description(field(1)),
            }
            if record.description == ">" {
                record.description.clear();
            }
            records.push(record);
            in_record = true;
            continue;
        }
        let Some(significance) = line.strip_prefix("– ") else {
            continue;
        };
        let significance = significance.split('\t').next().unwrap_or_default().trim();
        if !in_record || significance == "Total" || fields.len() < 4 {
            continue;
        }
        let record = records
            .last_mut()
            .context("a name form before any record")?;
        if let Some(form) = form(significance, field(2), field(3)) {
            record.forms.push(form);
        }
    }
    Ok(records)
}

/// Each disambiguated Strong's number to the one record it names, by index;
/// numbers two records share are left out.
pub(crate) fn by_strong(records: &[Record]) -> HashMap<String, usize> {
    let mut map: HashMap<String, Option<usize>> = HashMap::new();
    for (index, record) in records.iter().enumerate() {
        for strong in record.forms.iter().flat_map(|form| &form.strongs) {
            map.entry(strong.clone())
                .and_modify(|existing| {
                    if *existing != Some(index) {
                        *existing = None;
                    }
                })
                .or_insert(Some(index));
        }
    }
    map.into_iter()
        .filter_map(|(strong, index)| Some((strong, index?)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZECHARIAH: &str = "$========== PERSON(s)\t\t\t
Zechariah@2Ki.14.29-=H2148P\tKing living at the time of Divided Monarchy\tJeroboam@2Ki.13.13-Amo + \t\t\t\tIsrael\t#A king of Northern Israel, living at the time of Divided Monarchy, first mentioned at <ref=\"2Ki.14.29\">2Ki.14.29</ref>) <br>only referred to as <strong=\"H2148P\">Zechariah</strong> (זְכַרְיָהוּ); <br>a son of <strong=\"H3379H\">Jeroboam</strong>.\tMale\t\t
– Named\tZechariah@2Ki.14.29-\tH2148P«H2148a=זְכַרְיָהוּ\tZechariah =ESV,NIV; Zachariah =KJV\thttps://www.stepbible.org/\t2Ki.14.29; 2Ki.15.8
– Total\tZechariah\tH2148P\t2Ki.14.29; 15.8,11\t3
@Briefest= King of N. Israel
$========== PLACE\t\t
Bethlehem@Gen.35.16-Jhn=H1035G\tBethlehem_1\tSalma@1Ch.2.51-=H8007H\t\thttps://www.google.com/maps/@31.70536,35.21026,14z\thttps://palopenmaps.org/\tTribe of Judah\t#A location in Judah Tribe\tPlace
– Named\tBethlehem@Gen.35.16-Jhn\tH1035G«H1035=בֵּית לֶ֫חֶם\tBethlehem\turl\tGen.35.19
– Greek\tBethlehem@Gen.35.16-Jhn\tG0965«G0965=Βηθλεέμ\tBethlehem\turl\tMat.2.1
– Named\tEphrath|Bethlehem@Gen.35.16-Jhn\tH0672H«H0672=אֶפְרָ֫תָה\tEphrath\turl\tGen.35.16
$========== EXCLUDED OTHER
Aramaic@2Ki.18.26-Rev=H0762\tA language\t\t\t\t\t>\t#A language\tLanguage
";

    #[test]
    fn names_records_in_descriptions_by_name() {
        for (raw, plain) in [
            ("People from Ram@Job.32.2", "People from Ram"),
            (
                "Ancestors of Heber@Jdg.4.11- or Hobab@Num.10.29-Jdg",
                "Ancestors of Heber or Hobab",
            ),
            (
                "People from Shelah@Gen.38.5-1Ch(?)",
                "People from Shelah (?)",
            ),
            (
                "Eber@Gen.10.21-Luk עֵבֶר (Eber, ) People from Eber@Gen.10.21-Luk",
                "People from Eber",
            ),
            (
                "King living at the time of Divided Monarchy",
                "King living at the time of Divided Monarchy",
            ),
        ] {
            assert_eq!(plain_description(raw), plain, "{raw}");
        }
    }

    #[test]
    fn strips_strongs_numbers_from_prose() {
        assert_eq!(
            strip_strongs("mother of Jezreel, Lo-ammi H3818."),
            "mother of Jezreel, Lo-ammi."
        );
        assert_eq!(strip_strongs("Abiel (H0022H) of Gibeon"), "Abiel of Gibeon");
        assert_eq!(strip_strongs("THE H1234 HOUSE"), "THE HOUSE");
    }

    #[test]
    fn reads_people_and_places() {
        let records = parse(ZECHARIAH).unwrap();
        assert_eq!(records.len(), 2);
        let zechariah = &records[0];
        assert_eq!(zechariah.key, "Zechariah@2Ki.14.29");
        assert_eq!(zechariah.kind, Kind::Person);
        assert_eq!(zechariah.category, "Male");
        assert_eq!(zechariah.origin, "Israel");
        assert_eq!(
            zechariah.summary,
            "A king of Northern Israel, living at the time of Divided Monarchy, first \
             mentioned at 2Ki.14.29\nonly referred to as Zechariah (זְכַרְיָהוּ)\na son of \
             Jeroboam."
        );
        assert_eq!(
            zechariah.links,
            [(
                "father",
                Link {
                    key: "Jeroboam@2Ki.13.13".into(),
                    flag: String::new()
                }
            )]
        );
        assert_eq!(zechariah.forms[0].english, ["Zechariah", "Zachariah"]);
        assert_eq!(zechariah.forms[0].hebrew, "זְכַרְיָהוּ");

        let bethlehem = &records[1];
        assert_eq!(bethlehem.name, "Bethlehem");
        assert_eq!(bethlehem.kind, Kind::Place);
        assert_eq!(bethlehem.coordinates, Some((31.70536, 35.21026)));
        assert_eq!(bethlehem.openbible, "Bethlehem 1");
        assert_eq!(bethlehem.links[0].0, "founder");
        // The Greek form is the New Testament's, and left out.
        assert_eq!(bethlehem.forms.len(), 2);
        assert_eq!(by_strong(&records).get("H0672H"), Some(&1));
    }
}
