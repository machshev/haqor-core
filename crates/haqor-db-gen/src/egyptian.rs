//! The vendored Egyptian and Coptic dictionaries (`src_texts/Egyptian`), read
//! for the lexicon brief's loanword lookups (ADR 7): the AED's list of
//! Egyptian words (TLA transliteration, German and English glosses, Wb
//! references) and the Comprehensive Coptic Lexicon (Coptic forms, English
//! glosses, Crum references).

use std::path::Path;

use anyhow::Result;
use quick_xml::events::{BytesStart, Event};

/// One dictionary entry, reduced to what a brief shows.
#[derive(Debug, Default, Clone)]
pub struct EgyptianEntry {
    pub id: String,
    pub forms: Vec<String>,
    pub pos: String,
    pub glosses: Vec<String>,
    pub bibl: Vec<String>,
}

fn attr(e: &BytesStart<'_>, key: &str) -> Result<String> {
    Ok(e.try_get_attribute(key)?
        .map(|a| a.normalized_value(crate::xml::VERSION))
        .transpose()?
        .map(|s| s.into_owned())
        .unwrap_or_default())
}

/// Read a TEI dictionary of either file's shape: `entry` with `xml:id`,
/// `orth` forms, `term` or `pos`, glosses in `quote` or `def` (English kept,
/// German kept only where there is no English), and `bibl`.
pub fn read_tei(path: &Path) -> Result<Vec<EgyptianEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut reader = crate::xml::Reader::open(path)?;
    let mut entries = Vec::new();
    let mut entry: Option<EgyptianEntry> = None;
    let mut german: Vec<String> = Vec::new();
    // The element whose text is being read, and the language in force.
    let mut field = "";
    let mut lang = String::new();
    loop {
        match reader.next()? {
            Event::Start(e) => {
                let name = e.name();
                let name: &str = name.as_ref();
                // A `cit` sets the language afresh; elsewhere only a stated one
                // replaces it.
                let l = attr(&e, "xml:lang")?;
                if name == "cit" || !l.is_empty() {
                    lang = l;
                }
                match name {
                    "entry" if entry.is_none() => {
                        entry = Some(EgyptianEntry {
                            id: attr(&e, "xml:id")?,
                            ..Default::default()
                        });
                        german.clear();
                    }
                    "orth" => field = "orth",
                    "term" | "pos" => field = "pos",
                    "quote" | "def" => field = "gloss",
                    "bibl" => field = "bibl",
                    _ => {}
                }
            }
            Event::End(e) => {
                let name = e.name();
                let name: &str = name.as_ref();
                match name {
                    "entry" => {
                        if let Some(mut done) = entry.take() {
                            if done.glosses.is_empty() {
                                done.glosses = std::mem::take(&mut german);
                            }
                            done.forms.dedup();
                            done.bibl.dedup();
                            entries.push(done);
                        }
                    }
                    "orth" | "term" | "pos" | "quote" | "def" | "bibl" => field = "",
                    _ => {}
                }
            }
            Event::Text(t) => {
                let Some(open) = entry.as_mut() else { continue };
                let text = t.xml10_content();
                let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                if text.is_empty() {
                    continue;
                }
                match field {
                    "orth" => {
                        if !open.forms.contains(&text) {
                            open.forms.push(text);
                        }
                    }
                    "pos" if open.pos.is_empty() => open.pos = text,
                    "gloss" if lang == "en" => open.glosses.push(text),
                    "gloss" if lang == "de" || lang.is_empty() => german.push(text),
                    "bibl" => open.bibl.push(text),
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(entries)
}

/// A transliterated Egyptian word as both Wiktionary and the TLA can be
/// compared: lower case, the aleph, ayin and yod in one form each, no
/// separators.
pub fn egyptian_key(word: &str) -> String {
    word.chars()
        .filter_map(|c| match c {
            'Ꜣ' | 'ꜣ' | '3' => Some('ꜣ'),
            'Ꜥ' | 'ꜥ' | 'ʿ' => Some('ꜥ'),
            'ꞽ' | 'Ꞽ' | 'ỉ' | 'i' | 'j' | 'J' => Some('j'),
            '.' | '=' | '-' | ' ' | '(' | ')' | '\u{0357}' | '\u{0313}' | '\u{0486}' => None,
            c => c.to_lowercase().next(),
        })
        .collect()
}

/// A Coptic form without its combining marks (supralinear strokes and the
/// like), for matching.
pub fn coptic_key(word: &str) -> String {
    word.chars()
        .filter(|c| !('\u{0300}'..='\u{036F}').contains(c) && *c != '\u{FE26}' && *c != '-')
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn egyptian_keys_unify_transliterations() {
        assert_eq!(egyptian_key("jtrw"), egyptian_key("ꞽtr.w"));
        assert_eq!(egyptian_key("ꜥꜣ"), egyptian_key("ʿ3"));
    }

    #[test]
    fn reads_both_shapes() {
        let dir = std::env::temp_dir().join(format!("haqor-egy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let aed = r#"<TEI><text><body>
          <entry xml:id="tla1"><form><orth>ꞽtr.w</orth></form>
            <gramGrp><term>substantive</term></gramGrp>
            <sense><cit type="translation" xml:lang="de"><quote>Fluss</quote></cit>
            <cit><bibl>Wb 1, 146</bibl></cit>
            <cit type="translation" xml:lang="en"><quote>river</quote></cit></sense>
          </entry></body></text></TEI>"#;
        std::fs::write(dir.join("aed.xml"), aed).unwrap();
        let coptic = r#"<TEI><text><body>
          <entry type="hom" xml:id="C9"><form type="lemma"><orth>ⲉⲓⲟⲟⲣ</orth>
            <gramGrp><pos>Subst.</pos></gramGrp></form>
            <sense><cit type="translation"><def xml:lang="de">Fluss</def>
            <def xml:lang="en">river, canal</def><bibl>CD 76a</bibl></cit></sense>
          </entry></body></text></TEI>"#;
        std::fs::write(dir.join("coptic.xml"), coptic).unwrap();
        let aed = read_tei(&dir.join("aed.xml")).unwrap();
        let coptic = read_tei(&dir.join("coptic.xml")).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(aed[0].id, "tla1");
        assert_eq!(aed[0].forms, ["ꞽtr.w"]);
        assert_eq!(aed[0].glosses, ["river"]);
        assert_eq!(aed[0].bibl, ["Wb 1, 146"]);
        assert_eq!(coptic[0].glosses, ["river, canal"]);
        assert_eq!(coptic[0].pos, "Subst.");
    }
}
