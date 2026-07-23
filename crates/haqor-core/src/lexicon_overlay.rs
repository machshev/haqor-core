//! Hand-maintained lexical data layered over the imported lexicons.
//!
//! Edit `data/lexicon_overrides.json`, then run `db gen-lexicon`. The generator
//! validates this file and writes its entries into `lexicon.db`.

use std::collections::HashSet;

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// Parse and validate an overlay supplied to database generation.
pub fn load(path: &std::path::Path) -> Result<Value> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading overlay {}", path.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing overlay {}", path.display()))?;
    validate(&value)?;
    Ok(value)
}

/// Validate and atomically save an overlay, returning the canonical pretty JSON.
pub fn save(path: &std::path::Path, value: &Value) -> Result<String> {
    validate(value)?;
    let rendered = serde_json::to_string_pretty(value)? + "\n";
    let file_name = path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("overlay.json");
    let temporary = path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()));
    std::fs::write(&temporary, rendered.as_bytes())
        .with_context(|| format!("writing temporary overlay {}", temporary.display()))?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error).with_context(|| format!("replacing overlay {}", path.display()));
    }
    Ok(rendered)
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
            if row
                .get("reader_override")
                .is_some_and(|value| !value.is_boolean())
            {
                bail!("{section}[{i}].reader_override must be a boolean");
            }
            if !seen.insert(surface) {
                bail!("duplicate surface `{surface}` in {section}");
            }
        }
    }
    let rows = value
        .get("primary_analyses")
        .and_then(Value::as_array)
        .context("overlay `primary_analyses` must be an array")?;
    let mut seen = HashSet::new();
    for (i, row) in rows.iter().enumerate() {
        let analysis_type = row
            .get("analysis_type")
            .and_then(Value::as_str)
            .unwrap_or("verb");
        if !matches!(analysis_type, "verb" | "noun") {
            bail!("primary_analyses[{i}].analysis_type must be `verb` or `noun`");
        }
        let required: &[&str] = if analysis_type == "noun" {
            &["surface", "stem", "kind", "label", "prefix"]
        } else {
            &[
                "surface",
                "root",
                "binyan",
                "form",
                "pgn",
                "prefix",
                "obj_suffix",
            ]
        };
        for field in required {
            if !row.get(*field).is_some_and(Value::is_string) {
                bail!("primary_analyses[{i}].{field} must be a string");
            }
        }
        if row["surface"].as_str().unwrap().trim().is_empty() {
            bail!("primary_analyses[{i}].surface must not be empty");
        }
        if analysis_type == "verb" && !row.get("vav_consecutive").is_some_and(Value::is_boolean) {
            bail!("primary_analyses[{i}].vav_consecutive must be a boolean");
        }
        if !seen.insert(row["surface"].as_str().unwrap()) {
            bail!("duplicate surface `{}` in primary_analyses", row["surface"]);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_surfaces_are_rejected() {
        let value = serde_json::json!({
            "lexicon_entries": [
                {"surface": "א", "root": "", "gloss": "one"},
                {"surface": "א", "root": "", "gloss": "two"}
            ],
            "word_glosses": [],
            "primary_analyses": []
        });
        assert!(
            validate(&value)
                .unwrap_err()
                .to_string()
                .contains("duplicate surface")
        );
    }

    #[test]
    fn save_validates_before_replacing_the_file() {
        let path = std::env::temp_dir().join(format!(
            "haqor-overlay-save-{}-{}.json",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::write(&path, "original").unwrap();
        let invalid = serde_json::json!({"lexicon_entries": [], "word_glosses": [{}], "primary_analyses": []});
        assert!(save(&path, &invalid).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
        std::fs::remove_file(path).unwrap();
    }
}
