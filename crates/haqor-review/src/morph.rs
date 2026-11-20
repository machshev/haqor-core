//! Backfill `morph` and `segments`, the machine-readable morphology, on
//! occurrences reviewed before the harness asked for them. An OT word whose
//! prose morphology agrees with the OSHB's reading at that position takes
//! the OSHB code and segmentation. Every other occurrence goes back to its
//! worker with the OSHB reading to check.
use anyhow::{Context, Result};
use haqor_core::bible::{HebrewWord, decode_oshb_morph, oshb_parts_error};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde_json::{Value, json};
use std::{fs, path::Path};

/// The words a prose morphology must use for each decoded field.
fn says(prose: &str, word: &HebrewWord) -> Vec<String> {
    let prose = prose.to_lowercase();
    let any = |options: &[&str]| options.iter().any(|o| prose.contains(o));
    let mut missing = Vec::new();
    let pos = word.part_of_speech.as_deref().unwrap_or("");
    let pos_words: &[&str] = match pos {
        // Gender, number and state already say a noun is one.
        "Noun" => &[],
        "Proper noun" => &["proper", "name"],
        "Adjective" => &["adjective", "numeral", "cardinal", "ordinal", "gentilic"],
        "Pronoun" => &["pronoun"],
        "Preposition" => &["preposition"],
        "Conjunction" => &["conjunction"],
        "Adverb" => &["adverb"],
        "Particle" => &[
            "particle",
            "marker",
            "article",
            "negat",
            "interrogative",
            "interjection",
        ],
        _ => &[],
    };
    if !pos_words.is_empty() && !any(pos_words) {
        missing.push(pos.to_string());
    }
    let mut need = |label: &Option<String>, options: &[&str]| {
        if let Some(label) = label
            && !any(options)
        {
            missing.push(label.clone());
        }
    };
    let stem = word.form.as_deref().map(str::to_lowercase);
    need(&word.form, &[stem.as_deref().unwrap_or("")]);
    let tense: &[&str] = match word.tense.as_deref() {
        Some("Perfect") => &["perfect", "qatal"],
        Some("Imperfect") => &["imperfect", "yiqtol"],
        Some("Wayyiqtol") => &["wayyiqtol", "consecutive imperfect", "vav-consecutive"],
        Some("Participle (act.)" | "Participle (pass.)") => &["participle"],
        Some("Inf. Construct") => &["infinitive construct"],
        Some("Inf. Absolute") => &["infinitive absolute"],
        Some("Imperative") => &["imperative"],
        Some("Jussive") => &["jussive"],
        Some("Cohortative") => &["cohortative"],
        _ => &[""],
    };
    need(&word.tense, tense);
    let person: &[&str] = match word.person.as_deref() {
        Some("First") => &["first", "1st", "1c", "1m", "1f"],
        Some("Second") => &["second", "2nd", "2m", "2f"],
        _ => &["third", "3rd", "3m", "3f", "3c"],
    };
    need(&word.person, person);
    let gender = word.gender.as_deref().map(str::to_lowercase);
    let gender: Vec<&str> = match gender.as_deref() {
        // The OSHB's "both" (a noun of either gender) is not a reading prose states.
        Some("both") => vec![""],
        Some(g) => vec![g],
        None => vec![],
    };
    need(&word.gender, &gender);
    let number = word.number.as_deref().map(str::to_lowercase);
    need(&word.number, &[number.as_deref().unwrap_or("")]);
    let state: &[&str] = match word.state.as_deref() {
        Some("Construct") => &["construct"],
        Some("Determined") => &["determined", "emphatic"],
        // Absolute is the unmarked state, often left unsaid.
        _ => &[""],
    };
    need(&word.state, state);
    missing
}

pub fn fill(db: &mut Connection, review: &Path, data: &Path) -> Result<()> {
    let hebrew =
        Connection::open_with_flags(data.join("hebrew.db"), OpenFlags::SQLITE_OPEN_READ_ONLY)
            .context("the OT backfill reads OSHB tags from hebrew.db")?;
    let mut paths = Vec::new();
    let mut dirs = vec![review.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        if !dir.try_exists()? {
            continue;
        }
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "json") {
                paths.push(path);
            }
        }
    }
    paths.sort();
    let (mut filled, mut reopened) = (0, 0);
    let tx = db.transaction()?;
    for path in paths {
        let mut verse: Value = serde_json::from_str(&fs::read_to_string(&path)?)
            .with_context(|| format!("parsing {}", path.display()))?;
        let source = verse["source"]
            .as_str()
            .context("review source")?
            .to_owned();
        let (book, chapter, number) = (
            verse["book"].as_i64().context("book")?,
            verse["chapter"].as_i64().context("chapter")?,
            verse["verse"].as_i64().context("verse")?,
        );
        let reference = (book << 16) + (chapter << 8) + number;
        let mut changed = false;
        for word in verse["words"].as_array_mut().into_iter().flatten() {
            let pos = word["position"].as_i64().context("position")?;
            let surface = word["surface"].as_str().unwrap_or("").to_owned();
            let analysis = &mut word["analysis"];
            if !analysis.is_object() || analysis.get("morph").is_some() {
                continue;
            }
            let tag: Option<(String, String)> = if source == "ot" {
                hebrew
                    .query_row(
                        "SELECT morph, source_word FROM oshb_primary
                         WHERE book = ?1 AND chapter = ?2 AND verse = ?3 AND position = ?4",
                        params![book, chapter, number, pos],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()?
            } else {
                None
            };
            let reason = match &tag {
                None => {
                    "Give morph and segments: the machine-readable morphology of this occurrence."
                        .to_owned()
                }
                Some((code, source_word)) => {
                    // A word of one part is the reader's own spelling, which can differ
                    // from the OSHB's pointing (the qere of יהוה).
                    let segments = if code.contains('/') {
                        haqor_core::bible::normalize_oshb_segments(source_word)
                    } else {
                        surface.clone()
                    };
                    let language = if code.starts_with('A') { "arc" } else { "he" };
                    let prose = analysis["morphology"].as_str().unwrap_or("");
                    let missing = says(prose, &decode_oshb_morph(code, &segments));
                    match oshb_parts_error(code, &segments, &surface) {
                        None if missing.is_empty() && analysis["language"] == language => {
                            analysis["morph"] = json!(code);
                            analysis["segments"] = json!(segments);
                            changed = true;
                            filled += 1;
                            continue;
                        }
                        error => format!(
                            "Give morph and segments, the machine-readable morphology. The OSHB reads {code} over {segments}; your morphology {prose:?} {}. Check the OSHB reading against the text and give the code that fits.",
                            match error {
                                Some(e) => format!("could not be matched to it ({e})"),
                                None if missing.is_empty() => "names another language".to_owned(),
                                None => format!("does not state its {}", missing.join(", ")),
                            }
                        ),
                    }
                }
            };
            let n = tx.execute(
                "UPDATE job SET state='ready',attempts=0,result=NULL,feedback=? WHERE source=? AND ref=? AND pos=? AND state='done'",
                params![reason, source, reference, pos],
            )?;
            if n > 0 {
                tx.execute(
                    "UPDATE job SET state='waiting',attempts=0,result=NULL WHERE source=? AND ref=? AND pos=-1",
                    params![source, reference],
                )?;
                tx.execute(
                    "DELETE FROM dependency WHERE source=? AND ref=? AND pos=?",
                    params![source, reference, pos],
                )?;
                reopened += 1;
            }
        }
        if changed {
            let text = serde_json::to_string_pretty(&verse)? + "\n";
            let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
            fs::write(&tmp, text)?;
            fs::rename(tmp, &path)?;
        }
    }
    tx.commit()?;
    println!(
        "Filled the morphology of {filled} reviewed occurrences from agreeing OSHB tags. Reopened {reopened} for a worker to give it."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prose_must_state_each_decoded_field() {
        let verb = decode_oshb_morph("HC/Vpw3ms", "וַ/יְבָרֶךְ");
        assert!(
            says(
                "Piel wayyiqtol, third person masculine singular, with prefixed וַ־.",
                &verb
            )
            .is_empty()
        );
        assert_eq!(
            says("Qal wayyiqtol, third person masculine singular", &verb),
            ["Piel"]
        );
        let noun = decode_oshb_morph("HNcmpa", "אֱלֹהִים");
        assert!(says("Masculine plural common noun in the absolute state", &noun).is_empty());
        assert_eq!(says("Masculine singular noun", &noun), ["Plural"]);
        let marker = decode_oshb_morph("HTo", "אֶת");
        assert!(says("Direct-object marker without prefix or suffix", &marker).is_empty());
    }
}
