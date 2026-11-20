//! Stage complete Semitic article changes outside agent contexts. Compare
//! their source revision before publishing via the normal lexicon validator.
use anyhow::{Context, Result, ensure};
use haqor_core::haqor_lexicon::{self, Entry, NameEntry};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::{fs, path::Path};

use crate::jobs::Job;

pub fn revision(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn identity(article: &Value) -> Result<(&str, bool)> {
    let id = article["id"].as_str().context("article needs id")?;
    ensure!(
        !id.is_empty()
            && id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
        "invalid article id"
    );
    let name = article.get("name").is_some();
    if name {
        serde_json::from_value::<NameEntry>(article.clone())?;
    } else {
        serde_json::from_value::<Entry>(article.clone())?;
    }
    ensure!(
        article["status"] == "draft",
        "article changes must be drafts"
    );
    Ok((id, name))
}

pub fn stage(tx: &Connection, j: &Job, answer: &mut Value, packet: &Value) -> Result<()> {
    tx.execute(
        "DELETE FROM article WHERE source=? AND ref=? AND pos=?",
        j.key(),
    )?;
    let draft = &answer["entry"]["draft"];
    if draft.is_null() || j.source == "greek" {
        return Ok(());
    }
    let article = &draft["article"];
    let (id, name) = identity(article)?;
    let expected = packet["candidates"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| r["entry"] == id && r["name"] == name))
        .map(|r| r["revision"].clone())
        .unwrap_or(Value::Null);
    // A discovered existing article outside the candidate list may also be
    // amended when its exact base hash was read by the worker.
    let base = draft.get("base").unwrap_or(&expected);
    ensure!(
        base.is_null()
            || base
                .as_str()
                .is_some_and(|s| s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())),
        "invalid article base revision"
    );
    if !expected.is_null() {
        ensure!(base == &expected, "incorrect article base revision");
    }
    let covers = |word: &Value| {
        use unicode_normalization::UnicodeNormalization;
        let lemma = word["lemma"].as_str().unwrap_or("");
        lemma
            .nfc()
            .eq(answer["lemma"].as_str().unwrap_or("").chars())
            && word["lang"].as_str().unwrap_or("he") == answer["language"]
            && word.get("root").unwrap_or(&article["root"]) == &answer["root"]
    };
    if !name {
        ensure!(
            article["words"]
                .as_array()
                .is_some_and(|w| w.iter().any(covers)),
            "draft lacks the resolved word and root"
        );
    } else {
        use unicode_normalization::UnicodeNormalization;
        ensure!(
            article["name"]
                .as_str()
                .is_some_and(|s| s.nfc().eq(answer["lemma"].as_str().unwrap_or("").chars())),
            "draft name differs from lemma"
        );
    }
    tx.execute(
        "INSERT INTO article(source,ref,pos,id,name,base,body) VALUES(?,?,?,?,?,?,?)",
        params![
            j.source,
            j.reference,
            j.pos,
            id,
            name,
            base.as_str(),
            serde_json::to_string_pretty(article)?
        ],
    )?;
    let article_id = id.to_owned();
    answer["entry"]["draft"]
        .as_object_mut()
        .unwrap()
        .remove("article");
    answer["entry"]["draft"]["article_id"] = json!(article_id);
    Ok(())
}

pub fn apply(db: &mut Connection, lexicon: &Path, data: &Path) -> Result<()> {
    let corpus =
        Connection::open_with_flags(data.join("haqor.db"), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let verses = haqor_lexicon::verses_of(&corpus)?;
    let writer = fs::File::open(lexicon.join("sources.json"))?;
    writer
        .try_lock()
        .context("another review queue is applying lexicon articles")?;
    let (applied, stale) = apply_pending_locked(db, lexicon, &verses)?;
    println!("Applied {applied} reviewed article drafts. Requeued {stale} stale proposals.");
    Ok(())
}

/// Publish word-level article proposals as soon as their word result is
/// accepted. The caller owns the `sources.json` lock for the duration of this
/// operation. A proposal whose base has become stale is returned to the word
/// queue so the next worker reads the current article.
pub fn apply_pending(db: &mut Connection, lexicon: &Path, data: &Path) -> Result<(usize, usize)> {
    let corpus =
        Connection::open_with_flags(data.join("haqor.db"), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let verses = haqor_lexicon::verses_of(&corpus)?;
    let writer = fs::File::open(lexicon.join("sources.json"))?;
    writer
        .try_lock()
        .context("another review queue is applying lexicon articles")?;
    apply_pending_locked(db, lexicon, &verses)
}

/// Result of attempting to publish one accepted word proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishOutcome {
    Applied,
    NotPending,
    Stale,
}

/// Publish exactly one accepted word proposal. This is the live-run API. It
/// never scans unrelated queue rows, and a validator error belongs only to
/// this proposal, so the coordinator can record it and continue dispatching.
pub fn apply_one(
    db: &mut Connection,
    lexicon: &Path,
    data: &Path,
    job: &Job,
) -> Result<PublishOutcome> {
    let corpus =
        Connection::open_with_flags(data.join("haqor.db"), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let verses = haqor_lexicon::verses_of(&corpus)?;
    let writer = fs::File::open(lexicon.join("sources.json"))?;
    writer
        .try_lock()
        .context("another review queue is applying lexicon articles")?;
    apply_one_locked(db, lexicon, &verses, job)
}

fn apply_pending_locked(
    db: &mut Connection,
    lexicon: &Path,
    verses: &HashSet<i64>,
) -> Result<(usize, usize)> {
    let mut applied = 0;
    let mut stale = 0;
    loop {
        let row = db
            .query_row(
                "SELECT a.source,a.ref,a.pos,a.id,a.name,a.base,a.body FROM article a
            JOIN job w ON w.source=a.source AND w.ref=a.ref AND w.pos=a.pos
            WHERE a.applied=0 AND w.state='done'
            ORDER BY a.ref,a.source,a.pos LIMIT 1",
                [],
                |r| {
                    Ok((
                        Job {
                            source: r.get(0)?,
                            reference: r.get(1)?,
                            pos: r.get(2)?,
                        },
                        r.get::<_, String>(3)?,
                        r.get::<_, bool>(4)?,
                        r.get::<_, Option<String>>(5)?,
                        r.get::<_, String>(6)?,
                    ))
                },
            )
            .optional()?;
        let Some((job, _, _, _, _)) = row else {
            break;
        };
        match apply_one_locked(db, lexicon, verses, &job) {
            Ok(PublishOutcome::Applied) => applied += 1,
            Ok(PublishOutcome::Stale) => stale += 1,
            Ok(PublishOutcome::NotPending) => (),
            Err(error) => {
                if !is_validation_error(&error) {
                    return Err(error);
                }
                // Preserve the draft for inspection while allowing unrelated
                // legacy proposals to be recovered in this pass.
                db.execute(
                    "UPDATE job SET state='failed',error=? WHERE source=? AND ref=? AND pos=?",
                    params![
                        format!("article publication: {error:#}"),
                        job.source,
                        job.reference,
                        job.pos
                    ],
                )?;
                eprintln!(
                    "{} {} position {}: article not published: {error:#}",
                    job.source, job.reference, job.pos
                );
            }
        }
    }
    Ok((applied, stale))
}

fn is_validation_error(error: &anyhow::Error) -> bool {
    let text = format!("{error:#}");
    text.starts_with("validating ") && text.contains("lexicon has")
}

fn apply_one_locked(
    db: &mut Connection,
    lexicon: &Path,
    verses: &HashSet<i64>,
    job: &Job,
) -> Result<PublishOutcome> {
    let row = db.query_row(
        "SELECT id,name,base,body FROM article WHERE source=? AND ref=? AND pos=? AND applied=0",
        job.key(), |r| Ok((r.get::<_,String>(0)?, r.get::<_,bool>(1)?, r.get::<_,Option<String>>(2)?, r.get::<_,String>(3)?))).optional()?;
    let Some((id, name, base, body)) = row else {
        return Ok(PublishOutcome::NotPending);
    };
    let path = if name {
        lexicon.join("names")
    } else {
        lexicon.to_path_buf()
    }
    .join(format!("{id}.json"));
    let current = match fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    if current
        .as_ref()
        .is_none_or(|s| s.trim_end() != body.trim_end())
    {
        if current.as_deref().map(revision) != base {
            let tx = db.transaction()?;
            let reason = format!(
                "Article {} changed. Read its current file and reconcile the proposed changes.",
                path.display()
            );
            tx.execute("UPDATE job SET state='ready',feedback=?,attempts=0,result=NULL WHERE source=? AND ref=? AND pos=?", params![reason,job.source,job.reference,job.pos])?;
            tx.execute("UPDATE job SET state='waiting',attempts=0,result=NULL WHERE source=? AND ref=? AND pos=-1", params![job.source,job.reference])?;
            tx.commit()?;
            return Ok(PublishOutcome::Stale);
        }
        haqor_lexicon::save_entry(lexicon, &body, verses)
            .with_context(|| format!("validating {id}"))?;
    }
    let tx = db.transaction()?;
    tx.execute(
        "INSERT OR REPLACE INTO dependency VALUES(?,?,?,?,?,?)",
        params![
            job.source,
            job.reference,
            job.pos,
            id,
            name,
            revision(&fs::read_to_string(&path)?)
        ],
    )?;
    tx.execute(
        "UPDATE article SET applied=1 WHERE source=? AND ref=? AND pos=?",
        job.key(),
    )?;
    tx.commit()?;
    Ok(PublishOutcome::Applied)
}

// Reopen only words that actually used a changed canonical article.
pub fn refresh_dependencies(
    db: &mut Connection,
    revisions: &std::collections::BTreeMap<(String, bool), String>,
) -> Result<()> {
    let tx = db.transaction()?;
    let deps = tx
        .prepare("SELECT DISTINCT id,name,revision FROM dependency")?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, bool>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (id, name, rev) in deps {
        if revisions.get(&(id.clone(), name)) == Some(&rev) {
            continue;
        }
        let mut affected = tx.prepare(
            "SELECT source,ref,pos FROM dependency WHERE id=? AND name=? AND revision=?",
        )?;
        let words = affected
            .query_map(params![id, name, rev], |r| {
                Ok(Job {
                    source: r.get(0)?,
                    reference: r.get(1)?,
                    pos: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for job in words {
            tx.execute("UPDATE job SET state='waiting',attempts=0,result=NULL WHERE source=? AND ref=? AND pos=-1",params![job.source,job.reference])?;
            tx.execute("UPDATE job SET state='ready', attempts=0, result=NULL,feedback='The referenced lexicon article changed. Check this occurrence again.' WHERE source=? AND ref=? AND pos=?",job.key())?;
            tx.execute(
                "DELETE FROM dependency WHERE source=? AND ref=? AND pos=?",
                job.key(),
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}
