//! Hand-maintained lexical data layered over the imported lexicons.
//!
//! Edit `data/lexicon_overrides.json`, then run `db gen-lexicon`. The generator
//! validates this file and writes its entries into `lexicon.db`.

use std::collections::HashSet;
use std::sync::OnceLock;

use anyhow::{Context, Result, bail};
use serde_json::Value;

const SOURCE: &str = include_str!("../data/lexicon_overrides.json");

#[derive(Debug, Clone, Copy)]
pub struct LexiconEntry {
    pub surface: &'static str,
    pub root: &'static str,
    pub gloss: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct WordGloss {
    pub surface: &'static str,
    pub gloss: &'static str,
    pub note: Option<&'static str>,
    pub is_name: bool,
}

fn source() -> &'static Value {
    static SOURCE_JSON: OnceLock<Value> = OnceLock::new();
    SOURCE_JSON.get_or_init(|| serde_json::from_str(SOURCE).expect("valid lexicon_overrides.json"))
}

fn text(row: &'static Value, field: &str) -> &'static str {
    row.get(field).and_then(Value::as_str).unwrap_or_default()
}

pub fn lexicon_entries() -> impl Iterator<Item = LexiconEntry> {
    source()["lexicon_entries"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| LexiconEntry {
            surface: text(row, "surface"),
            root: text(row, "root"),
            gloss: text(row, "gloss"),
        })
}

pub fn word_glosses() -> impl Iterator<Item = WordGloss> {
    source()["word_glosses"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| WordGloss {
            surface: text(row, "surface"),
            gloss: text(row, "gloss"),
            note: row.get("note").and_then(Value::as_str),
            is_name: row.get("is_name").and_then(Value::as_bool).unwrap_or(false),
        })
}

/// Parse and validate an overlay supplied to database generation.
pub fn load(path: &std::path::Path) -> Result<Value> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading overlay {}", path.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing overlay {}", path.display()))?;
    validate(&value)?;
    Ok(value)
}

pub fn validate(value: &Value) -> Result<()> {
    for section in ["lexicon_entries", "word_glosses"] {
        let rows = value
            .get(section)
            .and_then(Value::as_array)
            .with_context(|| format!("overlay `{section}` must be an array"))?;
        let mut seen = HashSet::new();
        for (i, row) in rows.iter().enumerate() {
            let surface = row
                .get("surface")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            let gloss = row
                .get("gloss")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            if surface.is_empty() {
                bail!("{section}[{i}].surface must not be empty");
            }
            if gloss.is_empty() && !row.get("is_name").and_then(Value::as_bool).unwrap_or(false) {
                bail!("{section}[{i}].gloss must not be empty");
            }
            if !seen.insert(surface) {
                bail!("duplicate surface `{surface}` in {section}");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_overlay_is_valid_and_contains_both_kinds() {
        validate(source()).unwrap();
        assert!(lexicon_entries().any(|e| e.surface == "כִּי" && e.gloss.contains("because")));
        assert!(word_glosses().any(|e| e.surface == "אֶת" && !e.gloss.is_empty()));
    }

    #[test]
    fn duplicate_surfaces_are_rejected() {
        let value = serde_json::json!({
            "lexicon_entries": [
                {"surface": "א", "root": "", "gloss": "one"},
                {"surface": "א", "root": "", "gloss": "two"}
            ],
            "word_glosses": []
        });
        assert!(
            validate(&value)
                .unwrap_err()
                .to_string()
                .contains("duplicate surface")
        );
    }
}
