//! Durable storage for completed review payloads.
//!
//! The queue database keeps scheduling state and a small locator. The complete
//! worker result lives in a source-backed file, so a large run can be resumed
//! without making SQLite the corpus archive.
use anyhow::{Context, Result, ensure};
use rusqlite::{
    Connection,
    functions::{Context as SqlContext, FunctionFlags},
    params,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

use crate::jobs::Job;

fn relative(source: &str, reference: i64) -> String {
    format!(
        "{}/{}/{}/{}.json",
        source,
        reference >> 16,
        (reference >> 8) & 255,
        reference & 255
    )
}

fn digest(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Install `review_result(result)`, which expands a source locator and leaves
/// raw legacy JSON unchanged. Queries can therefore read old and compacted
/// queues during migration with one expression.
pub fn register(db: &Connection, root: &Path) -> Result<()> {
    let root = root.to_path_buf();
    db.create_scalar_function(
        "review_result",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DIRECTONLY,
        move |ctx: &mut SqlContext<'_>| {
            let value: Option<String> = ctx.get(0)?;
            let Some(value) = value else {
                return Ok(None::<String>);
            };
            let parsed: Value = match serde_json::from_str(&value) {
                Ok(v) => v,
                Err(_) => return Ok(Some(value)),
            };
            let Some(published) = parsed.get("_published").and_then(Value::as_array) else {
                return Ok(Some(value));
            };
            let source = published.first().and_then(Value::as_str).ok_or_else(|| {
                rusqlite::Error::UserFunctionError("invalid published result".into())
            })?;
            if source.is_empty()
                || source.contains('/')
                || source.contains('\\')
                || source.contains("..")
            {
                return Err(rusqlite::Error::UserFunctionError(
                    "invalid published source".into(),
                ));
            }
            let reference = published.get(1).and_then(Value::as_i64).ok_or_else(|| {
                rusqlite::Error::UserFunctionError("invalid published reference".into())
            })?;
            let pos = published.get(2).and_then(Value::as_i64).ok_or_else(|| {
                rusqlite::Error::UserFunctionError("invalid published position".into())
            })?;
            let raw = fs::read_to_string(root.join(relative(source, reference)))
                .map_err(|e| rusqlite::Error::UserFunctionError(e.into()))?;
            let verse: Value = serde_json::from_str(&raw)
                .map_err(|e| rusqlite::Error::UserFunctionError(e.into()))?;
            if pos < 0 {
                return Ok(verse
                    .get("review")
                    .cloned()
                    .or(Some(verse))
                    .map(|v| v.to_string()));
            }
            Ok(verse["words"]
                .as_array()
                .and_then(|words| {
                    words
                        .iter()
                        .find(|word| word["position"].as_i64() == Some(pos))
                })
                .and_then(|word| word.get("analysis"))
                .map(Value::to_string))
        },
    )?;
    Ok(())
}

/// Verify that the canonical source-backed review contains this completed
/// result, then replace the large payload with a compact publication marker.
pub fn compact(db: &mut Connection, job: &Job, root: &Path) -> Result<()> {
    let (state, result): (String, Option<String>) = db.query_row(
        "SELECT state,result FROM job WHERE source=? AND ref=? AND pos=?",
        job.key(),
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if state != "done" || result.is_none() {
        return Ok(());
    }
    let result = result.unwrap();
    let value: Value = serde_json::from_str(&result).context("invalid completed review result")?;
    let path = root.join(relative(&job.source, job.reference));
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("canonical review missing: {}", path.display()))?;
    let verse: Value = serde_json::from_str(&raw).context("invalid canonical review")?;
    let canonical = if job.pos < 0 {
        verse.get("review").cloned().unwrap_or(verse)
    } else {
        verse["words"]
            .as_array()
            .and_then(|words| {
                words
                    .iter()
                    .find(|word| word["position"].as_i64() == Some(job.pos))
            })
            .and_then(|word| word.get("analysis"))
            .cloned()
            .context("canonical review lacks word analysis")?
    };
    ensure!(
        canonical == value,
        "canonical review differs from accepted result"
    );
    let text = canonical.to_string();
    let published = json!({"_published":[job.source,job.reference,job.pos],"language":value["language"],"lemma":value["lemma"],"root":value["root"],"qualifier":value["qualifier"],"digest":digest(&text)});
    db.execute(
        "UPDATE job SET result=? WHERE source=? AND ref=? AND pos=?",
        params![published.to_string(), job.source, job.reference, job.pos],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    #[test]
    fn published_function_reads_source_and_leaves_raw_json() {
        let root = std::env::temp_dir().join(format!("haqor-review-result-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let p = root.join("peshitta/40/1/2.json");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, "{\"ok\":true}").unwrap();
        let db = Connection::open_in_memory().unwrap();
        register(&db, &root).unwrap();
        let raw: String = db
            .query_row(
                "SELECT review_result(?)",
                [r#"{"_published":["peshitta",2621698,-1]}"#],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(raw, "{\"ok\":true}");
        let same: String = db
            .query_row("SELECT review_result(?)", ["{\"raw\":1}"], |r| r.get(0))
            .unwrap();
        assert_eq!(same, "{\"raw\":1}");
        let _ = fs::remove_file(p);
        let _ = fs::remove_dir(root);
    }
}
