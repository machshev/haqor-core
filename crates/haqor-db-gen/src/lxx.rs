//! The Septuagint's Greek for each Hebrew word, from MACULA Hebrew's alignment
//! (its `greek` and `greekstrong` word attributes), for the Haqor lexicon's
//! briefs (ADR 7). `db import-lxx` reads MACULA's lowfat XML and writes the
//! checked-in `src_texts/MACULA-LXX/lxx.tsv`, naming each Greek word's lemma
//! from Strong's Greek dictionary so that no Strong's number is kept.

use std::collections::HashMap;
use std::io::Write as _;
use std::path::Path;

use anyhow::{Context, Result, bail};
use quick_xml::events::{BytesStart, Event};

/// The prepared file.
pub const LXX_TSV: &str = "lxx.tsv";

fn attr(e: &BytesStart<'_>, key: &str) -> Result<String> {
    Ok(e.try_get_attribute(key)?
        .map(|a| a.normalized_value(crate::xml::VERSION))
        .transpose()?
        .map(|s| s.into_owned())
        .unwrap_or_default())
}

/// Strong's Greek numbers to lemmas, from Open Scriptures'
/// `strongs-greek-dictionary.js` (a JavaScript assignment of one JSON object).
pub fn strongs_lemmas(path: &Path) -> Result<HashMap<u32, String>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let start = text.find('{').context("no dictionary object")?;
    let end = text.rfind('}').context("no dictionary object")?;
    let value: serde_json::Value = serde_json::from_str(&text[start..=end])
        .with_context(|| format!("parsing {}", path.display()))?;
    let mut out = HashMap::new();
    for (key, entry) in value
        .as_object()
        .context("the dictionary is not an object")?
    {
        let number = key
            .trim_start_matches('G')
            .parse()
            .with_context(|| format!("unreadable Strong's key {key:?}"))?;
        if let Some(lemma) = entry["lemma"].as_str() {
            out.insert(number, lemma.to_string());
        }
    }
    Ok(out)
}

/// What an import wrote.
#[derive(Debug, Default)]
pub struct LxxSummary {
    pub words: usize,
    pub aligned: usize,
    pub unnamed: usize,
}

/// `db import-lxx`: every word of MACULA's lowfat chapter files in `lowfat`
/// with Greek, as `verse \t word \t Hebrew lemma \t Hebrew text \t Greek form
/// \t Greek lemma`, into `output/lxx.tsv`. A Greek word whose Strong's number
/// the dictionary lacks keeps its form as its lemma, and is counted.
pub fn import_lxx(lowfat: &Path, strongs: &Path, output: &Path) -> Result<LxxSummary> {
    let lemmas = strongs_lemmas(strongs)?;
    let mut files: Vec<_> = std::fs::read_dir(lowfat)
        .with_context(|| format!("reading {}", lowfat.display()))?
        .map(|d| d.map(|d| d.path()))
        .collect::<std::io::Result<_>>()?;
    files.retain(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with("-lowfat.xml") && n.as_bytes()[0].is_ascii_digit())
    });
    files.sort();
    if files.is_empty() {
        bail!("no MACULA chapter files in {}", lowfat.display());
    }
    std::fs::create_dir_all(output)?;
    let path = output.join(LXX_TSV);
    let mut out = std::io::BufWriter::new(std::fs::File::create(&path)?);
    let mut summary = LxxSummary::default();
    for file in &files {
        let mut reader = crate::xml::Reader::open(file)?;
        loop {
            match reader.next()? {
                Event::Start(e) | Event::Empty(e) if e.name().as_ref() == "w" => {
                    summary.words += 1;
                    let greek = attr(&e, "greek")?;
                    // Some words carry a placeholder (`’’`) where the Greek has no
                    // equivalent.
                    if !greek.chars().any(|c| {
                        ('\u{0370}'..='\u{03FF}').contains(&c)
                            || ('\u{1F00}'..='\u{1FFF}').contains(&c)
                    }) {
                        continue;
                    }
                    let reference = attr(&e, "ref")?;
                    let Some((verse, word)) = reference.split_once('!') else {
                        bail!(
                            "{}: a word reference without its number: {reference:?}",
                            file.display()
                        );
                    };
                    let lemma = match attr(&e, "greekstrong")?
                        .parse::<u32>()
                        .ok()
                        .and_then(|n| lemmas.get(&n))
                    {
                        Some(lemma) => lemma.clone(),
                        None => {
                            summary.unnamed += 1;
                            greek.clone()
                        }
                    };
                    writeln!(
                        out,
                        "{verse}\t{word}\t{}\t{}\t{greek}\t{lemma}",
                        attr(&e, "lemma")?,
                        attr(&e, "unicode")?
                    )?;
                    summary.aligned += 1;
                }
                Event::Eof => break,
                _ => {}
            }
        }
    }
    out.flush()?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_words_and_names_their_lemmas() {
        let dir = std::env::temp_dir().join(format!("haqor-lxx-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("lowfat")).unwrap();
        std::fs::write(
            dir.join("strongs.js"),
            r#"var strongsGreekDictionary = {"G5590":{"lemma":"ψυχή"},"G2532":{"lemma":"καί"}};"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("lowfat/01-Gen-002-lowfat.xml"),
            r#"<book><sentence id="GEN 2:7"><wg>
              <w ref="GEN 2:7!1" lemma="וְ" unicode="וַ" greek="καὶ" greekstrong="2532">וַ</w>
              <w ref="GEN 2:7!4" lemma="אֵת" unicode="אֶת">אֶת</w>
              <w ref="GEN 2:7!5" lemma="הַ" unicode="הָ" greek="’’">הָ</w>
              <w ref="GEN 2:7!15" lemma="נֶפֶשׁ" unicode="נֶ֥פֶשׁ" greek="ψυξὴν" greekstrong="5590">נֶ֥פֶשׁ</w>
            </wg></sentence></book>"#,
        )
        .unwrap();
        let summary = import_lxx(&dir.join("lowfat"), &dir.join("strongs.js"), &dir).unwrap();
        let tsv = std::fs::read_to_string(dir.join(LXX_TSV)).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!((summary.words, summary.aligned, summary.unnamed), (4, 2, 0));
        assert_eq!(
            tsv,
            "GEN 2:7\t1\tוְ\tוַ\tκαὶ\tκαί\nGEN 2:7\t15\tנֶפֶשׁ\tנֶ֥פֶשׁ\tψυξὴν\tψυχή\n"
        );
    }
}
