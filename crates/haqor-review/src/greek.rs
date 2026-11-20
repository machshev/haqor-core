//! The shared Greek draft dictionary, keyed by dictionary form and homograph.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn component(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() || matches!(c, '-' | '_' | ' ') {
            out.push(c);
        } else {
            for b in c.to_string().bytes() {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

pub fn id(a: &Value) -> Result<String> {
    let lemma = a["lemma"].as_str().context("missing Greek lemma")?;
    let qualifier = a["qualifier"].as_str().context("missing Greek qualifier")?;
    let mut key = component(lemma);
    if !qualifier.is_empty() {
        key.push('~');
        key.push_str(&component(qualifier));
    }
    ensure!(
        !key.is_empty() && key.len() < 248,
        "Greek dictionary key is too long or empty"
    );
    Ok(format!("grc:{key}"))
}
pub fn path(lexicon: &Path, id: &str) -> Result<PathBuf> {
    let key = id.strip_prefix("grc:").context("invalid Greek key")?;
    ensure!(
        !key.is_empty()
            && key
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '%' | '~' | '-' | '_' | ' ')),
        "invalid Greek dictionary filename"
    );
    Ok(lexicon.join("greek").join(format!("{key}.json")))
}

pub fn publish(lexicon: &Path, answer: &Value, reference: i64, position: i64) -> Result<bool> {
    let key = id(answer)?;
    let path = path(lexicon, &key)?;
    let prior = match fs::read(&path) {
        Ok(bytes) => Some(serde_json::from_slice::<Value>(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let mut article=prior.clone().unwrap_or_else(||json!({"id":key,"language":"grc","lemma":answer["lemma"],"qualifier":answer["qualifier"],"root":answer["root"],"status":"draft","senses":{}}));
    let senses = article["senses"]
        .as_object_mut()
        .context("invalid Greek dictionary senses")?;
    let sense = answer["sense"].as_str().context("missing sense")?;
    let draft = &answer["entry"]["draft"];
    let entry = senses
        .entry(sense)
        .or_insert_with(|| json!({"evidence":{}}));
    entry["gloss"] = answer["gloss"].clone();
    entry["definition"] = draft["definition"].clone();
    let at = format!(
        "{}.{}.{}[{}]",
        reference >> 16,
        (reference >> 8) & 255,
        reference & 255,
        position
    );
    entry["evidence"][at] = draft["evidence"].clone();
    article["root"] = answer["root"].clone();
    if Some(&article) == prior.as_ref() {
        return Ok(false);
    }
    fs::create_dir_all(path.parent().unwrap())?;
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&temp, serde_json::to_string_pretty(&article)? + "\n")?;
    fs::rename(temp, path)?;
    Ok(true)
}
