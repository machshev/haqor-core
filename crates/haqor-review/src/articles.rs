//! Stage complete Semitic article changes outside agent contexts. Compare
//! their source revision before publishing via the normal lexicon validator.
use anyhow::{Context, Result, ensure};
use haqor_core::haqor_lexicon::{self, Entry, NameEntry};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::{
    fs,
    path::{Path, PathBuf},
};

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
    let index = crate::jobs::lexeme_index(answer)?;
    if draft.is_null() {
        return Ok(());
    }
    if j.source == "greek" {
        ensure!(
            index == 0,
            "a Greek article holds one lexeme: word must be 0"
        );
        return Ok(());
    }
    // An article's id is derived from its content (a root entry's from its
    // root, a noun's from its lemma, a name's from the name), never chosen by
    // the worker.
    let mut article = draft["article"].clone();
    let drafted = article["id"].as_str().unwrap_or("").to_owned();
    let (derived, _) = haqor_lexicon::article_id(&serde_json::to_string(&article)?)
        .context("the draft article does not fit the lexicon schema")?;
    article["id"] = json!(derived);
    answer["entry"]["draft"]["article"] = article.clone();
    let draft = &answer["entry"]["draft"];
    let (id, name) = identity(&article)?;
    let candidate = |id: &str| {
        packet["candidates"]
            .as_array()
            .and_then(|rows| rows.iter().find(|r| r["entry"] == id && r["name"] == name))
            .map(|r| r["revision"].clone())
    };
    // A draft of an existing article whose content now derives another id: a
    // noun that gains a verb or a second lexeme becomes its root's entry.
    let replaces = (!name && drafted != id && candidate(&drafted).is_some()).then_some(drafted);
    let expected = match &replaces {
        Some(old) => candidate(old),
        None => candidate(id),
    }
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
            article["words"].get(index as usize).is_some_and(covers),
            "draft word {index} is not the resolved word and root"
        );
        crate::jobs::check_sense(
            answer,
            false,
            &json!(crate::jobs::sense_list(&article["words"][index as usize])),
        )?;
    } else {
        use unicode_normalization::UnicodeNormalization;
        ensure!(
            article["name"]
                .as_str()
                .is_some_and(|s| s.nfc().eq(answer["lemma"].as_str().unwrap_or("").chars())),
            "draft name differs from lemma"
        );
        crate::jobs::check_sense(answer, true, &Value::Null)?;
        ensure!(
            index == 0,
            "a personal-name article holds one lexeme: word must be 0"
        );
    }
    tx.execute(
        "INSERT INTO article(source,ref,pos,id,name,base,body,replaces) VALUES(?,?,?,?,?,?,?,?)",
        params![
            j.source,
            j.reference,
            j.pos,
            id,
            name,
            base.as_str(),
            serde_json::to_string_pretty(&article)?,
            replaces
        ],
    )?;
    let article_id = id.to_owned();
    let await_bearer = name
        && article["people"]
            .as_array()
            .is_some_and(|people| people.is_empty());
    if !await_bearer {
        answer["entry"]["draft"]
            .as_object_mut()
            .unwrap()
            .remove("article");
    }
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

/// Result of attempting to publish one accepted word proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishOutcome {
    Applied,
    NotPending,
    Stale,
    Rejected,
    Deferred,
}

/// Publish exactly one accepted word proposal. This is the live-run API. It
/// never scans unrelated queue rows, and a validator error belongs only to
/// this proposal, so the coordinator can record it and continue dispatching.
pub fn apply_one(
    db: &mut Connection,
    lexicon: &Path,
    verses: &std::collections::HashSet<i64>,
    job: &Job,
) -> Result<PublishOutcome> {
    let writer = fs::File::open(lexicon.join("sources.json"))?;
    writer
        .try_lock()
        .context("another review queue is applying lexicon articles")?;
    apply_one_locked(db, lexicon, verses, job)
}

fn apply_pending_locked(
    db: &mut Connection,
    lexicon: &Path,
    verses: &HashSet<i64>,
) -> Result<(usize, usize)> {
    let mut applied = 0;
    let mut stale = 0;
    let jobs=db.prepare("SELECT a.source,a.ref,a.pos FROM article a JOIN job w USING(source,ref,pos) WHERE a.applied=0 AND w.state='done' ORDER BY a.ref,a.source,a.pos")?
        .query_map([],|r|Ok(Job{source:r.get(0)?, reference:r.get(1)?,pos:r.get(2)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
    for job in jobs {
        match apply_one_locked(db, lexicon, verses, &job)? {
            PublishOutcome::Applied => applied += 1,
            PublishOutcome::Stale | PublishOutcome::Rejected => stale += 1,
            PublishOutcome::Deferred | PublishOutcome::NotPending => (),
        }
    }
    Ok((applied, stale))
}

fn is_validation_error(error: &anyhow::Error) -> bool {
    let text = format!("{error:#}");
    text.starts_with("validating ") && text.contains("lexicon has")
}

/// Read a deferred draft from its published occurrence after the transient
/// publication payload has been compacted.
pub fn draft_body(db: &Connection, job: &Job, staged: String) -> Result<String> {
    if !staged.is_empty() {
        return Ok(staged);
    }
    let article: String = db.query_row(
        "SELECT json_extract(review_result(result),'$.entry.draft.article') FROM job WHERE source=? AND ref=? AND pos=?",
        job.key(), |row| row.get(0),
    ).context("reading deferred name draft from its published word")?;
    Ok(serde_json::to_string_pretty(
        &serde_json::from_str::<Value>(&article)?,
    )?)
}

fn apply_one_locked(
    db: &mut Connection,
    lexicon: &Path,
    verses: &HashSet<i64>,
    job: &Job,
) -> Result<PublishOutcome> {
    let row = db.query_row(
        "SELECT id,name,base,body,replaces FROM article WHERE source=? AND ref=? AND pos=? AND applied=0",
        job.key(), |r| Ok((r.get::<_,String>(0)?, r.get::<_,bool>(1)?, r.get::<_,Option<String>>(2)?, r.get::<_,String>(3)?, r.get::<_,Option<String>>(4)?))).optional()?;
    let Some((id, name, base, body, replaces)) = row else {
        return Ok(PublishOutcome::NotPending);
    };
    let body = draft_body(db, job, body)?;
    let path = article_path(lexicon, &id, name, Some(&body));
    // A promoted noun is read, checked and replaced at its own file.
    let old_path = replaces
        .as_deref()
        .map(|old| article_path(lexicon, old, false, None));
    if let (Some(old), Some(old_path)) = (&replaces, &old_path) {
        let noun = old_path.parent() == Some(lexicon.join(haqor_lexicon::NOUNS_DIR).as_path());
        if !noun || path.exists() {
            return reject(
                db,
                job,
                format!(
                    "The draft's content makes it the article `{id}`, not `{old}`, which it was drafted from. Amend `{id}` itself, or keep `{old}`'s root and words."
                ),
            );
        }
    }
    let current_path = old_path.as_ref().unwrap_or(&path);
    let current = match fs::read_to_string(current_path) {
        Ok(s) => Some(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let draft: Value = serde_json::from_str(&body)?;
    if name
        && draft["people"]
            .as_array()
            .is_some_and(|people| people.is_empty())
    {
        return Ok(PublishOutcome::Deferred);
    }
    if current
        .as_ref()
        .is_none_or(|s| s.trim_end() != body.trim_end())
    {
        let moved = match (&current, name) {
            (Some(current), false) => moved_lexeme(current, &draft)?,
            _ => None,
        };
        if let Some(reason) = moved {
            let tx = db.transaction()?;
            tx.execute("UPDATE job SET state='ready',feedback=?,result=NULL WHERE source=? AND ref=? AND pos=?",
                params![reason,job.source,job.reference,job.pos])?;
            tx.execute(
                "UPDATE job SET state='waiting',result=NULL WHERE source=? AND ref=? AND pos=-1",
                params![job.source, job.reference],
            )?;
            tx.commit()?;
            return Ok(PublishOutcome::Rejected);
        }
        if current.as_deref().map(revision) != base {
            let tx = db.transaction()?;
            let reason = format!(
                "Article {} changed. Read its current file and reconcile the proposed changes.",
                current_path.display()
            );
            tx.execute("UPDATE job SET state='ready',feedback=?,attempts=0,result=NULL WHERE source=? AND ref=? AND pos=?", params![reason,job.source,job.reference,job.pos])?;
            tx.execute("UPDATE job SET state='waiting',attempts=0,result=NULL WHERE source=? AND ref=? AND pos=-1", params![job.source,job.reference])?;
            tx.commit()?;
            return Ok(PublishOutcome::Stale);
        }
        // The noun's file goes before the root entry that replaces it is
        // validated, or the lexicon would hold two entries for one root.
        if let Some(old_path) = &old_path {
            fs::remove_file(old_path)?;
        }
        if let Err(error) = haqor_lexicon::save_entry(lexicon, &body, verses)
            .with_context(|| format!("validating {id}"))
        {
            if let (Some(old_path), Some(current)) = (&old_path, &current) {
                fs::write(old_path, current)?;
            }
            if !is_validation_error(&error) {
                return Err(error);
            }
            let tx = db.transaction()?;
            let reason = format!("{error:#}");
            tx.execute("UPDATE job SET state='ready',feedback=?,result=NULL WHERE source=? AND ref=? AND pos=?",
                params![reason,job.source,job.reference,job.pos])?;
            tx.execute(
                "UPDATE job SET state='waiting',result=NULL WHERE source=? AND ref=? AND pos=-1",
                params![job.source, job.reference],
            )?;
            tx.commit()?;
            return Ok(PublishOutcome::Rejected);
        }
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
    // The promoted noun's readers now read its root entry: its words keep
    // their indices, so only the id and revision move. (Their review files are
    // repointed by the publication, which commits them with the article.)
    if let (Some(old), Some(current)) = (&replaces, &current) {
        let new = revision(&fs::read_to_string(&path)?);
        tx.execute(
            "UPDATE dependency SET id=?1, revision=?2 WHERE id=?3 AND name=0 AND revision=?4",
            params![id, new, old, revision(current)],
        )?;
        tx.execute(
            "UPDATE dependency SET id=?1 WHERE id=?2 AND name=0",
            params![id, old],
        )?;
        tx.execute(
            "UPDATE article SET id=?1 WHERE id=?2 AND name=0 AND applied=0",
            params![id, old],
        )?;
    }
    tx.execute(
        "UPDATE article SET applied=1 WHERE source=? AND ref=? AND pos=?",
        job.key(),
    )?;
    tx.commit()?;
    Ok(PublishOutcome::Applied)
}

/// Send a word back to its worker with `reason`, and its verse to wait.
fn reject(db: &mut Connection, job: &Job, reason: String) -> Result<PublishOutcome> {
    let tx = db.transaction()?;
    tx.execute(
        "UPDATE job SET state='ready',feedback=?,result=NULL WHERE source=? AND ref=? AND pos=?",
        params![reason, job.source, job.reference, job.pos],
    )?;
    tx.execute(
        "UPDATE job SET state='waiting',result=NULL WHERE source=? AND ref=? AND pos=-1",
        params![job.source, job.reference],
    )?;
    tx.commit()?;
    Ok(PublishOutcome::Rejected)
}

/// The file of article `id`: under `names/` for a name, under `nouns/` for a
/// noun (by the draft `body` where there is one, or else where the file is),
/// and at the top level for a root's entry.
pub fn article_path(lexicon: &Path, id: &str, name: bool, body: Option<&str>) -> PathBuf {
    let file = format!("{id}.json");
    if name {
        return lexicon.join(haqor_lexicon::NAMES_DIR).join(file);
    }
    let nouns = lexicon.join(haqor_lexicon::NOUNS_DIR).join(&file);
    let noun = match body
        .filter(|b| !b.trim().is_empty())
        .and_then(|b| haqor_lexicon::article_id(b).ok())
    {
        Some((_, dir)) => dir == haqor_lexicon::NOUNS_DIR,
        None => nouns.exists(),
    };
    if noun { nouns } else { lexicon.join(file) }
}

/// The review files whose occurrences name article `old`, rewritten (unless
/// `dry`) to name `new`: a promoted noun's readers.
pub fn repoint_reviews(review: &Path, old: &str, new: &str, dry: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
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
                files.push(path);
            }
        }
    }
    let mut changed = Vec::new();
    for path in files {
        let mut verse: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        if verse["source"] == "greek" {
            continue;
        }
        let mut hit = false;
        for word in verse["words"].as_array_mut().into_iter().flatten() {
            let entry = &mut word["analysis"]["entry"];
            for pointer in ["/existing", "/draft/article_id"] {
                if let Some(slot) = entry.pointer_mut(pointer)
                    && slot == old
                {
                    *slot = json!(new);
                    hit = true;
                }
            }
        }
        if hit {
            if !dry {
                let text = serde_json::to_string_pretty(&verse)? + "\n";
                let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
                fs::write(&tmp, text)?;
                fs::rename(tmp, &path)?;
            }
            changed.push(path);
        }
    }
    Ok(changed)
}

/// Reviewed occurrences address a lexeme by its index in `words`, so an edit
/// may change a lexeme's content or append new ones but never move or remove
/// one. Returns the reason a draft breaks that.
pub fn moved_lexeme(current: &str, draft: &Value) -> Result<Option<String>> {
    use unicode_normalization::UnicodeNormalization;
    let current: Value = serde_json::from_str(current)?;
    let identity = |w: &Value| {
        (
            w["lemma"].as_str().unwrap_or("").nfc().collect::<String>(),
            w["lang"].as_str().unwrap_or("he").to_owned(),
            w["pos"].clone(),
        )
    };
    let empty = Vec::new();
    let after = draft["words"].as_array().unwrap_or(&empty);
    for (index, word) in current["words"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .enumerate()
    {
        let (lemma, lang, pos) = identity(word);
        if after.get(index).map(identity) != Some((lemma.clone(), lang.clone(), pos.clone())) {
            return Ok(Some(format!(
                "The draft moves or removes word {index} ({lemma}, {lang}, {pos}) of the current article. Reviewed occurrences address lexemes by index: keep every existing word at its index and append new lexemes at the end."
            )));
        }
        // Occurrences name the sense they bear by id, so a lexeme keeps them all.
        let ids = |w: &Value| -> Vec<String> {
            crate::jobs::sense_list(w)
                .iter()
                .filter_map(|s| s["id"].as_str().map(str::to_owned))
                .collect()
        };
        let kept = ids(&after[index]);
        if let Some(id) = ids(word).into_iter().find(|id| !kept.contains(id)) {
            return Ok(Some(format!(
                "The draft removes or renames sense `{id}` of word {index} ({lemma}). Reviewed occurrences name the sense they bear by id: keep every existing sense id in its word."
            )));
        }
    }
    Ok(None)
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn empty_name_article_waits_for_verse_bearer() -> Result<()> {
        let root = std::env::temp_dir().join(format!(
            "haqor-review-name-stage-{}",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        fs::create_dir_all(root.join("names"))?;
        fs::write(root.join("sources.json"), "{}")?;
        let body = json!({
            "id":"new-name", "name":"שֵׁם", "english":"Name", "root":"שם",
            "entry":"shem", "status":"draft", "updated":"2026-11-20",
            "people":[], "gloss":"name", "summary":"a name", "evidence":[]
        });
        let mut db = Connection::open_in_memory()?;
        db.execute_batch(crate::jobs::SCHEMA)?;
        db.execute("INSERT INTO verse VALUES('ot',1,'{}')", [])?;
        db.execute(
            "INSERT INTO job(source,ref,pos,state,result) VALUES('ot',1,0,'done','{}')",
            [],
        )?;
        db.execute(
            "INSERT INTO article(source,ref,pos,id,name,base,body,applied) VALUES('ot',1,0,'new-name',1,NULL,?,0)",
            [body.to_string()],
        )?;
        let job = Job {
            source: "ot".into(),
            reference: 1,
            pos: 0,
        };
        assert_eq!(
            apply_one(&mut db, &root, &HashSet::new(), &job)?,
            PublishOutcome::Deferred
        );
        assert_eq!(
            db.query_row("SELECT applied FROM article", [], |r| r.get::<_, i64>(0))?,
            0
        );
        crate::result_store::register(&db, &root)?;
        let dir = root.join("ot/0/0");
        fs::create_dir_all(&dir)?;
        fs::write(
            dir.join("1.json"),
            json!({
                "words":[{"position":0,"analysis":{"entry":{"draft":{"article":body}}}}]
            })
            .to_string(),
        )?;
        db.execute("UPDATE article SET body=''", [])?;
        db.execute(
            "UPDATE job SET result=?",
            [json!({"_published":["ot",1,0]}).to_string()],
        )?;
        assert_eq!(
            apply_one(&mut db, &root, &HashSet::new(), &job)?,
            PublishOutcome::Deferred
        );
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn drafts_keep_existing_lexemes_at_their_index() -> Result<()> {
        let current = json!({"words":[
            {"lemma":"בְּרַךְ","lang":"arc","pos":"verb"},
            {"lemma":"בְּרַךְ","lang":"arc","pos":"noun"}]})
        .to_string();
        let appended = json!({"words":[
            {"lemma":"בְּרַךְ","lang":"arc","pos":"verb","gloss":"bless"},
            {"lemma":"בְּרַךְ","lang":"arc","pos":"noun"},
            {"lemma":"בְּרָכָה","pos":"noun"}]});
        assert_eq!(moved_lexeme(&current, &appended)?, None);
        let swapped = json!({"words":[
            {"lemma":"בְּרַךְ","lang":"arc","pos":"noun"},
            {"lemma":"בְּרַךְ","lang":"arc","pos":"verb"}]});
        assert!(
            moved_lexeme(&current, &swapped)?
                .unwrap()
                .contains("word 0")
        );
        let removed = json!({"words":[{"lemma":"בְּרַךְ","lang":"arc","pos":"verb"}]});
        assert!(
            moved_lexeme(&current, &removed)?
                .unwrap()
                .contains("word 1")
        );
        Ok(())
    }

    fn lexicon() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/lexicon")
    }

    #[test]
    fn a_draft_takes_the_id_its_root_derives() -> Result<()> {
        let Ok(text) = fs::read_to_string(lexicon().join("26m.json")) else {
            return Ok(());
        };
        let mut article: Value = serde_json::from_str(&text)?;
        article["id"] = json!("atam-shut");
        let mut answer = json!({"language":"he","lemma":"אָטַם","root":"אטמ",
            "entry":{"draft":{"article":article,"base":null,"definition":"d","evidence":"e","change":"c"},
                "word":0,"sense":"stop"}});
        let db = Connection::open_in_memory()?;
        db.execute_batch(crate::jobs::SCHEMA)?;
        db.execute("INSERT INTO verse VALUES('ot',1,'{}')", [])?;
        db.execute(
            "INSERT INTO job(source,ref,pos,state) VALUES('ot',1,0,'running')",
            [],
        )?;
        let job = Job {
            source: "ot".into(),
            reference: 1,
            pos: 0,
        };
        stage(&db, &job, &mut answer, &json!({"candidates":[]}))?;
        assert_eq!(answer["entry"]["draft"]["article_id"], "26m");
        let (id, body): (String, String) =
            db.query_row("SELECT id,body FROM article", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?;
        assert_eq!(id, "26m");
        assert_eq!(serde_json::from_str::<Value>(&body)?["id"], "26m");
        Ok(())
    }

    #[test]
    fn articles_are_filed_by_kind() {
        let lexicon = lexicon();
        let Some(noun) = fs::read_dir(lexicon.join("nouns")).ok().and_then(|mut d| {
            d.next()?
                .ok()?
                .path()
                .file_stem()?
                .to_str()
                .map(str::to_owned)
        }) else {
            return;
        };
        assert_eq!(
            article_path(&lexicon, &noun, false, None),
            lexicon.join("nouns").join(format!("{noun}.json"))
        );
        assert_eq!(
            article_path(&lexicon, "26m", false, None),
            lexicon.join("26m.json")
        );
        assert_eq!(
            article_path(&lexicon, "yrd", true, None),
            lexicon.join("names/yrd.json")
        );
    }

    #[test]
    fn a_promoted_nouns_readers_are_repointed() -> Result<()> {
        let root = std::env::temp_dir().join(format!(
            "haqor-review-repoint-{}",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        let dir = root.join("ot/1/2");
        fs::create_dir_all(&dir)?;
        let verse = json!({"source":"ot","words":[
            {"position":0,"analysis":{"entry":{"existing":"noun-x","word":0}}},
            {"position":1,"analysis":{"entry":{"draft":{"article_id":"noun-x"},"word":0}}},
            {"position":2,"analysis":{"entry":{"existing":"other","word":0}}}]});
        fs::write(dir.join("3.json"), verse.to_string())?;
        fs::write(
            root.join("ot/1/2/4.json"),
            json!({"source":"ot","words":[]}).to_string(),
        )?;
        assert_eq!(
            repoint_reviews(&root, "noun-x", "rootx", true)?,
            [dir.join("3.json")]
        );
        let unchanged: Value = serde_json::from_str(&fs::read_to_string(dir.join("3.json"))?)?;
        assert_eq!(
            unchanged["words"][0]["analysis"]["entry"]["existing"],
            "noun-x"
        );
        repoint_reviews(&root, "noun-x", "rootx", false)?;
        let after: Value = serde_json::from_str(&fs::read_to_string(dir.join("3.json"))?)?;
        assert_eq!(after["words"][0]["analysis"]["entry"]["existing"], "rootx");
        assert_eq!(
            after["words"][1]["analysis"]["entry"]["draft"]["article_id"],
            "rootx"
        );
        assert_eq!(after["words"][2]["analysis"]["entry"]["existing"], "other");
        fs::remove_dir_all(root)?;
        Ok(())
    }
}
