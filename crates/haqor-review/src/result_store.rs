//! Durable storage for completed review payloads.
//!
//! The queue database keeps scheduling state and a small locator. The complete
//! worker result lives in a source-backed file, so a large run can be resumed
//! without making SQLite the corpus archive.
use anyhow::{Context, Result};
use rusqlite::{
    Connection,
    functions::{Context as SqlContext, FunctionFlags},
    params,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

use crate::jobs::Job;

fn relative(job: &Job) -> String {
    format!(
        "{}/{}/{}/{}/{}.json",
        job.source,
        job.reference >> 16,
        (job.reference >> 8) & 255,
        job.reference & 255,
        job.pos
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
        FunctionFlags::SQLITE_UTF8,
        move |ctx: &mut SqlContext<'_>| {
            let value: Option<String> = ctx.get(0)?;
            let Some(value) = value else {
                return Ok(None::<String>);
            };
            let Some(locator) = value.strip_prefix("@/") else {
                return Ok(Some(value));
            };
            if locator.contains("..") || Path::new(locator).is_absolute() {
                return Err(rusqlite::Error::UserFunctionError(
                    "invalid review result locator".into(),
                ));
            }
            fs::read_to_string(root.join(locator))
                .map(Some)
                .map_err(|e| rusqlite::Error::UserFunctionError(e.into()))
        },
    )?;
    Ok(())
}

/// Move one completed result to its deterministic source-backed file and
/// replace the database payload by its locator. Calling this repeatedly is
/// idempotent. The caller should invoke it after the result transaction has
/// committed and before dispatching a later packet.
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
    let text = if let Some(locator) = result.strip_prefix("@/") {
        fs::read_to_string(root.join(locator))?
    } else {
        result.clone()
    };
    let value: Value = serde_json::from_str(&text).context("invalid completed review result")?;
    let path = root.join(relative(job));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let existing = fs::read_to_string(&path).ok();
    if existing.as_deref() != Some(&text) {
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, &text)?;
        fs::rename(tmp, &path)?;
    }
    let locator = format!("@/{}", relative(job));
    let language = value["language"].as_str();
    let lemma = value["lemma"].as_str();
    let root_key = value["root"].as_str();
    let qualifier = value["qualifier"].as_str();
    db.execute("UPDATE job SET result=?, result_digest=?, result_language=?, result_lemma=?, result_root=?, result_qualifier=? WHERE source=? AND ref=? AND pos=?",
        params![locator, digest(&text), language, lemma, root_key, qualifier, job.source, job.reference, job.pos])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    #[test]
    fn locator_function_reads_source_and_leaves_raw_json() {
        let root = std::env::temp_dir().join(format!("haqor-review-result-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let p = root.join("x.json");
        fs::write(&p, "{\"ok\":true}").unwrap();
        let db = Connection::open_in_memory().unwrap();
        register(&db, &root).unwrap();
        let raw: String = db
            .query_row("SELECT review_result(?)", ["@/x.json"], |r| r.get(0))
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
