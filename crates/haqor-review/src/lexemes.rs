//! Backfill `entry.word`, the lexeme index, on occurrences reviewed before
//! the harness asked for one. An occurrence whose dictionary form matches one
//! lexeme of its article is linked in its tracked verse file. Every other
//! occurrence goes back to its worker with the article's candidate lexemes.
use anyhow::{Context, Result};
use haqor_core::{data_support::fold_consonants, haqor_lexicon};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use unicode_normalization::UnicodeNormalization;

struct Lexeme {
    index: usize,
    name: bool,
    lemma: String,
    language: String,
    root: String,
    pos: String,
}

fn nfc(s: &str) -> String {
    s.nfc().collect()
}

/// Semitic articles' lexemes by article id. Name articles hold one lexeme.
fn articles(lexicon: &Path) -> Result<BTreeMap<String, Vec<Lexeme>>> {
    let mut out: BTreeMap<String, Vec<Lexeme>> = BTreeMap::new();
    for (_, text) in haqor_lexicon::read_entry_texts(lexicon)?
        .into_iter()
        .chain(haqor_lexicon::read_name_texts(lexicon)?)
    {
        let e: Value = serde_json::from_str(&text)?;
        let id = e["id"].as_str().context("missing article id")?.to_owned();
        let lexemes = out.entry(id).or_default();
        if let Some(name) = e["name"].as_str() {
            lexemes.push(Lexeme {
                index: 0,
                name: true,
                lemma: nfc(name),
                language: e["lang"].as_str().unwrap_or("he").to_owned(),
                root: e["root"].as_str().unwrap_or("").to_owned(),
                pos: "name".into(),
            });
            continue;
        }
        for (index, w) in e["words"].as_array().into_iter().flatten().enumerate() {
            lexemes.push(Lexeme {
                index,
                name: false,
                lemma: nfc(w["lemma"].as_str().unwrap_or("")),
                language: w["lang"].as_str().unwrap_or("he").to_owned(),
                root: w["root"]
                    .as_str()
                    .or(e["root"].as_str())
                    .unwrap_or("")
                    .to_owned(),
                pos: w["pos"].as_str().unwrap_or("").to_owned(),
            });
        }
    }
    Ok(out)
}

/// The lexeme index an analysis resolves to, or why it cannot be decided.
fn resolve(
    articles: &BTreeMap<String, Vec<Lexeme>>,
    source: &str,
    analysis: &Value,
) -> std::result::Result<u64, String> {
    if source == "greek" {
        return Ok(0);
    }
    let entry = &analysis["entry"];
    let Some(id) = entry["existing"]
        .as_str()
        .or(entry["draft"]["article_id"].as_str())
        .or(entry["draft"]["article"]["id"].as_str())
    else {
        return Err("The occurrence names no lexicon article.".into());
    };
    let Some(lexemes) = articles.get(id) else {
        return Err(format!("Article {id} is not in the lexicon."));
    };
    let lemma = nfc(analysis["lemma"].as_str().unwrap_or(""));
    let language = analysis["language"].as_str().unwrap_or("he");
    let root = analysis["root"].as_str().unwrap_or("");
    let matches: Vec<&Lexeme> = lexemes
        .iter()
        .filter(|l| {
            if l.name && matches!(language, "arc" | "syc") {
                return fold_consonants(&l.lemma) == fold_consonants(&lemma);
            }
            l.lemma == lemma && l.language == language && (l.name || l.root == root)
        })
        .collect();
    let chosen: Vec<&&Lexeme> = match matches.len() {
        0 | 1 => matches.iter().collect(),
        _ => {
            // Homographs: a qualifier naming one lexeme's part of speech decides.
            let qualifier = analysis["qualifier"].as_str().unwrap_or("");
            matches.iter().filter(|l| l.pos == qualifier).collect()
        }
    };
    if let [one] = chosen.as_slice() {
        return Ok(one.index as u64);
    }
    let listed = lexemes
        .iter()
        .filter(|l| !l.name)
        .map(|l| {
            format!(
                "{} {} ({}, {}, root {})",
                l.index, l.lemma, l.language, l.pos, l.root
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    Err(format!(
        "Give entry.word, the index of this occurrence's lexeme in article {id}. {} Its words are: {listed}.",
        if matches.is_empty() {
            format!("No word of the article is {lemma} ({language}, root {root}).")
        } else {
            format!("Several words are {lemma} ({language}).")
        }
    ))
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            files(&path, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
            out.push(path);
        }
    }
    Ok(())
}

pub fn link(db: &mut Connection, review: &Path, lexicon: &Path) -> Result<()> {
    let articles = articles(lexicon)?;
    let mut paths = Vec::new();
    if review.try_exists()? {
        files(review, &mut paths)?;
    }
    paths.sort();
    let (mut linked, mut reopened) = (0, 0);
    let tx = db.transaction()?;
    for path in paths {
        let raw = fs::read_to_string(&path)?;
        let mut verse: Value =
            serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        let source = verse["source"]
            .as_str()
            .context("review source")?
            .to_owned();
        let reference = (verse["book"].as_i64().context("book")? << 16)
            + (verse["chapter"].as_i64().context("chapter")? << 8)
            + verse["verse"].as_i64().context("verse")?;
        let mut changed = false;
        for word in verse["words"].as_array_mut().into_iter().flatten() {
            let pos = word["position"].as_i64().context("position")?;
            let analysis = &mut word["analysis"];
            if !analysis["entry"].is_object() || analysis["entry"].get("word").is_some() {
                continue;
            }
            match resolve(&articles, &source, analysis) {
                Ok(index) => {
                    analysis["entry"]["word"] = json!(index);
                    changed = true;
                    linked += 1;
                }
                Err(reason) => {
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
        "Linked {linked} reviewed occurrences to their lexeme. Reopened {reopened} for a worker to decide."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn barak() -> BTreeMap<String, Vec<Lexeme>> {
        let l = |index, lemma: &str, language: &str, pos: &str| Lexeme {
            index,
            name: false,
            lemma: nfc(lemma),
            language: language.into(),
            root: "ברכ".into(),
            pos: pos.into(),
        };
        BTreeMap::from([(
            "barak".to_owned(),
            vec![
                l(0, "בָּרַךְ", "he", "verb"),
                l(4, "בְּרַךְ", "arc", "verb"),
                l(5, "בְּרַךְ", "arc", "noun"),
            ],
        )])
    }

    fn analysis(lemma: &str, language: &str, qualifier: &str) -> Value {
        json!({"entry":{"existing":"barak"},"lemma":lemma,"language":language,"qualifier":qualifier,"root":"ברכ"})
    }

    #[test]
    fn a_unique_lexeme_is_linked_and_homographs_need_a_part_of_speech() {
        let a = barak();
        assert_eq!(resolve(&a, "ot", &analysis("בָּרַךְ", "he", "")), Ok(0));
        let err = resolve(&a, "ot", &analysis("בְּרַךְ", "arc", "")).unwrap_err();
        assert!(
            err.contains("Several words") && err.contains("5 בְּרַךְ"),
            "{err}"
        );
        assert_eq!(resolve(&a, "ot", &analysis("בְּרַךְ", "arc", "noun")), Ok(5));
        assert!(resolve(&a, "ot", &analysis("בְּרָכָה", "he", "")).is_err());
        assert_eq!(resolve(&a, "greek", &json!({})), Ok(0));
    }
}
