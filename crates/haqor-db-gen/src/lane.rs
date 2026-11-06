//! Lane's *Arabic-English Lexicon* (1863–93), from the Perseus Digital
//! Library's TEI as corrected by the laneslexicon project, prepared for the
//! Haqor lexicon's cognate search (ADR 7): `db import-lane` turns the XML into
//! the checked-in `src_texts/Lane/lane.jsonl`, one line per entry, its Arabic
//! in Unicode.
//!
//! The XML writes Arabic in Buckwalter transliteration, with `A^`, `A_`, `A=`,
//! `A@`, `w^` and `y^` for the hamza and alif forms XML text cannot carry as
//! `>`, `<` and `|`. Each root is a `div2` whose `n` is the root in that
//! transliteration; each `<pb n>` starts printed page `n`, the page an entry
//! is cited by.

use std::io::Write as _;
use std::path::Path;

use anyhow::{Context, Result, bail};
use quick_xml::events::{BytesStart, Event};
use serde_json::json;

/// The prepared file in `src_texts/Lane`.
pub const LANE_JSONL: &str = "lane.jsonl";

/// Lane's Buckwalter as Unicode Arabic. Unknown characters pass through.
pub fn buckwalter_to_arabic(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() * 2);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        // Two-character hamza and alif forms.
        let pair = match (c, next) {
            ('A', Some('^')) => Some('أ'),
            ('A', Some('_')) => Some('إ'),
            ('A', Some('=')) => Some('آ'),
            ('A', Some('@')) => Some('ٱ'),
            ('w', Some('^')) => Some('ؤ'),
            ('y', Some('^')) => Some('ئ'),
            _ => None,
        };
        if let Some(letter) = pair {
            out.push(letter);
            i += 2;
            continue;
        }
        out.push(match c {
            '\'' => 'ء',
            '|' => 'آ',
            '>' => 'أ',
            '&' => 'ؤ',
            '<' => 'إ',
            '}' => 'ئ',
            'A' => 'ا',
            'b' => 'ب',
            'p' => 'ة',
            't' => 'ت',
            'v' => 'ث',
            'j' => 'ج',
            'H' => 'ح',
            'x' => 'خ',
            'd' => 'د',
            '*' => 'ذ',
            'r' => 'ر',
            'z' => 'ز',
            's' => 'س',
            '$' => 'ش',
            'S' => 'ص',
            'D' => 'ض',
            'T' => 'ط',
            'Z' => 'ظ',
            'E' => 'ع',
            'g' => 'غ',
            '_' => 'ـ',
            'f' => 'ف',
            'q' => 'ق',
            'k' => 'ك',
            'l' => 'ل',
            'm' => 'م',
            'n' => 'ن',
            'h' => 'ه',
            'w' => 'و',
            'Y' => 'ى',
            'y' => 'ي',
            'F' => '\u{064B}',
            'N' => '\u{064C}',
            'K' => '\u{064D}',
            'a' => '\u{064E}',
            'u' => '\u{064F}',
            'i' => '\u{0650}',
            '~' => '\u{0651}',
            'o' => '\u{0652}',
            '`' => '\u{0670}',
            '{' => 'ٱ',
            'P' => 'پ',
            'J' => 'چ',
            'G' => 'گ',
            other => other,
        });
        i += 1;
    }
    out
}

/// A root's letters as the cognate search keys them: every hamza form one ء,
/// alif maqṣūra a ي, no vowels. A root has no long ā, so its
/// alif (Lane writes `ArD` for أرض) is a hamza too.
pub fn arabic_root_key(root: &str) -> String {
    root.chars()
        .filter_map(|c| match c {
            'ا' | 'أ' | 'إ' | 'آ' | 'ؤ' | 'ئ' | 'ٱ' | 'ء' => Some('ء'),
            'ى' => Some('ي'),
            c if ('\u{0621}'..='\u{064A}').contains(&c) => Some(c),
            _ => None,
        })
        .collect()
}

fn attr(e: &BytesStart<'_>, key: &str) -> Result<String> {
    Ok(e.try_get_attribute(key)?
        .map(|a| a.normalized_value(crate::xml::VERSION))
        .transpose()?
        .map(|s| s.into_owned())
        .unwrap_or_default())
}

/// One entry as prepared.
#[derive(Debug)]
struct Entry {
    root: String,
    root_bw: String,
    id: String,
    key: String,
    headword: String,
    page: Option<u32>,
    text: String,
}

/// Read one of Lane's XML files into its entries, in order.
fn read_file(path: &Path) -> Result<Vec<Entry>> {
    let mut reader = crate::xml::Reader::open(path)?;
    let mut entries: Vec<Entry> = Vec::new();
    let mut root_bw = String::new();
    let mut page: Option<u32> = None;
    let mut entry: Option<Entry> = None;
    // Inside `<foreign lang="ar">` or `<orth lang="ar">`: how deep, and
    // whether the run is the headword.
    let mut arabic = 0usize;
    let mut in_orth = false;
    loop {
        match reader.next()? {
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == "pb" => {
                let n: u32 = attr(&e, "n")?.parse().with_context(|| {
                    format!("{}: a page break without a number", path.display())
                })?;
                // Entries before the file's first break are on the page before it.
                if page.is_none() {
                    for earlier in &mut entries {
                        earlier.page = Some(n - 1);
                    }
                    if let Some(open) = entry.as_mut() {
                        open.page = Some(n - 1);
                    }
                }
                page = Some(n);
            }
            Event::Start(e) => match e.name().as_ref() {
                "div2" if attr(&e, "type")? == "root" => root_bw = attr(&e, "n")?,
                "entryFree" => {
                    entry = Some(Entry {
                        root: buckwalter_to_arabic(&root_bw),
                        root_bw: root_bw.clone(),
                        id: attr(&e, "id")?,
                        key: attr(&e, "key")?,
                        headword: String::new(),
                        page,
                        text: String::new(),
                    })
                }
                "foreign" | "orth" if attr(&e, "lang")? == "ar" => {
                    arabic += 1;
                    in_orth = e.name().as_ref() == "orth";
                }
                _ => {}
            },
            Event::End(e) => match e.name().as_ref() {
                "entryFree" => {
                    if let Some(mut done) = entry.take() {
                        done.text = done.text.split_whitespace().collect::<Vec<_>>().join(" ");
                        entries.push(done);
                    }
                }
                "foreign" | "orth" if arabic > 0 => {
                    arabic -= 1;
                    in_orth = false;
                }
                "form" => {
                    if let Some(open) = entry.as_mut() {
                        open.text.push(' ');
                    }
                }
                _ => {}
            },
            Event::Text(t) => {
                if let Some(open) = entry.as_mut() {
                    let raw = t.xml10_content();
                    if arabic > 0 {
                        // A lone `*` is a placeholder, not a ذ.
                        if raw.trim() == "*" {
                            continue;
                        }
                        let word = buckwalter_to_arabic(raw.trim());
                        if in_orth && open.headword.is_empty() {
                            open.headword = word;
                        } else {
                            open.text.push_str(&word);
                        }
                    } else {
                        open.text.push_str(&raw);
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(entries)
}

/// What an import wrote.
#[derive(Debug, Default)]
pub struct LaneSummary {
    pub files: usize,
    pub roots: usize,
    pub entries: usize,
}

/// `db import-lane`: read every XML file of the laneslexicon `lexicon_xml`
/// checkout in `xml_dir` and write `output/lane.jsonl`, entries in printed
/// page order.
pub fn import_lane(xml_dir: &Path, output: &Path) -> Result<LaneSummary> {
    let mut paths: Vec<_> = std::fs::read_dir(xml_dir)
        .with_context(|| format!("reading {}", xml_dir.display()))?
        .map(|d| d.map(|d| d.path()))
        .collect::<std::io::Result<_>>()?;
    paths.retain(|p| p.extension().is_some_and(|e| e == "xml"));
    paths.sort();
    if paths.is_empty() {
        bail!("no XML files in {}", xml_dir.display());
    }
    let mut summary = LaneSummary::default();
    let mut entries = Vec::new();
    for path in &paths {
        let read = read_file(path)?;
        if read.iter().any(|e| e.page.is_none()) {
            bail!("{} has an entry before any page break", path.display());
        }
        entries.extend(read);
        summary.files += 1;
    }
    // Printed order: by page, then by place in the file.
    entries.sort_by_key(|e| e.page);
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    let path = output.join(LANE_JSONL);
    let mut out = std::io::BufWriter::new(
        std::fs::File::create(&path).with_context(|| format!("creating {}", path.display()))?,
    );
    let mut last_root = String::new();
    for e in &entries {
        if e.root_bw != last_root {
            summary.roots += 1;
            last_root.clone_from(&e.root_bw);
        }
        let line = json!({
            "root": e.root,
            "root_key": arabic_root_key(&e.root),
            "root_bw": e.root_bw,
            "id": e.id,
            "key": e.key,
            "headword": e.headword,
            "page": e.page,
            "text": e.text,
        });
        writeln!(out, "{line}")?;
        summary.entries += 1;
    }
    out.flush()?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckwalter_becomes_arabic() {
        assert_eq!(buckwalter_to_arabic("Eayon"), "عَيْن");
        assert_eq!(buckwalter_to_arabic("A^ab"), "أَب");
        assert_eq!(buckwalter_to_arabic("A@ll~`hu"), "ٱللّٰهُ");
        assert_eq!(buckwalter_to_arabic("$aA=biybu"), "شَآبِيبُ");
    }

    #[test]
    fn root_keys_fold_hamza() {
        assert_eq!(arabic_root_key(&buckwalter_to_arabic("A^rD")), "ءرض");
        assert_eq!(arabic_root_key(&buckwalter_to_arabic("ArD")), "ءرض");
        assert_eq!(arabic_root_key(&buckwalter_to_arabic("bnY")), "بني");
    }

    #[test]
    fn reads_roots_entries_and_pages() {
        let dir = std::env::temp_dir().join(format!("haqor-lane-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let xml = r#"<TEI.2><text><body>
            <div2 n="smE" type="root">
              <entryFree id="n1" key="samiEa" type="main">
                <form><orth lang="ar">samiEa</orth><orth lang="ar">*</orth></form> and ↓ <orth type="arrow" lang="ar">AstmE</orth>
                He <hi>heard</hi> it, <foreign lang="ar">sumoEN</foreign>.
              </entryFree>
              <pb n="1427"/>
              <entryFree id="n2" key="samoE" type="main">
                <form><orth lang="ar">samoEN</orth></form> The ear.
              </entryFree>
            </div2></body></text></TEI.2>"#;
        std::fs::write(dir.join("s0.xml"), xml).unwrap();
        let entries = read_file(&dir.join("s0.xml")).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].root, "سمع");
        assert_eq!(entries[0].headword, "سَمِعَ");
        assert_eq!(entries[0].page, Some(1426));
        assert_eq!(entries[0].text, "and ↓ استمع He heard it, سُمْعٌ.");
        assert_eq!(entries[1].page, Some(1427));
    }
}
