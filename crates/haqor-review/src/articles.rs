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
    let path = article_path(lexicon, &id, name);
    // Every entry is named by its root, so a draft whose content derives
    // another id has changed its root: the article it was drafted from stays.
    if let Some(old) = &replaces {
        return reject(
            db,
            job,
            format!(
                "The draft's content makes it the article `{id}`, not `{old}`, which it was drafted from. Amend `{id}` itself, or keep `{old}`'s root and words."
            ),
        );
    }
    let current = match fs::read_to_string(&path) {
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
                path.display()
            );
            tx.execute("UPDATE job SET state='ready',feedback=?,attempts=0,result=NULL WHERE source=? AND ref=? AND pos=?", params![reason,job.source,job.reference,job.pos])?;
            tx.execute("UPDATE job SET state='waiting',attempts=0,result=NULL WHERE source=? AND ref=? AND pos=-1", params![job.source,job.reference])?;
            tx.commit()?;
            return Ok(PublishOutcome::Stale);
        }
        // A review only adds to an existing article: everything else in its
        // draft is discarded.
        let publish = match &current {
            Some(current) => additive(current, &draft, name)?,
            None => Some(body.clone()),
        };
        if let Some(body) = publish {
            if let Err(error) = haqor_lexicon::save_entry(lexicon, &body, verses)
                .with_context(|| format!("validating {id}"))
            {
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

/// The file of article `id`: under `names/` for a name, and at the top level
/// for a root's entry.
pub fn article_path(lexicon: &Path, id: &str, name: bool) -> PathBuf {
    let file = format!("{id}.json");
    if name {
        lexicon.join(haqor_lexicon::NAMES_DIR).join(file)
    } else {
        lexicon.join(file)
    }
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

/// What a review may publish into an existing article: the current file with
/// only the draft's additions — new lexemes appended after the existing ones,
/// new stems and senses (by id) inside existing lexemes, the cognates those
/// additions cite, and for a name its newly identified bearers. Every other
/// field of the draft is ignored, so a reviewer that re-types the article
/// cannot alter or drop existing glosses, definitions, quotations or keys.
/// Returns `None` when the draft adds nothing.
pub fn additive(current: &str, draft: &Value, name: bool) -> Result<Option<String>> {
    let mut merged: Value = serde_json::from_str(current)?;
    let before = merged.clone();
    let mut cited = Vec::new();
    if name {
        if let (Some(have), Some(new)) = (
            merged.get_mut("people").and_then(Value::as_array_mut),
            draft["people"].as_array(),
        ) {
            for person in new {
                if !have.contains(person) {
                    have.push(person.clone());
                }
            }
        }
    } else {
        // A noun promoted to its root's entry takes the entry's identity.
        if merged["id"] != draft["id"] {
            merged["id"] = draft["id"].clone();
            merged["root"] = draft["root"].clone();
        }
        let empty = Vec::new();
        let new_words = draft["words"].as_array().unwrap_or(&empty);
        if !merged["words"].is_array() {
            merged["words"] = json!([]);
        }
        let words = merged["words"].as_array_mut().unwrap();
        for (index, word) in new_words.iter().enumerate() {
            match words.get_mut(index) {
                Some(have) => add_senses_of_word(have, word, &mut cited),
                None => {
                    collect_cognate_ids(word, &mut cited);
                    words.push(word.clone());
                }
            }
        }
        if !cited.is_empty() {
            if !merged["cognates"].is_array() {
                merged["cognates"] = json!([]);
            }
            let cognates = merged["cognates"].as_array_mut().unwrap();
            for cognate in draft["cognates"].as_array().into_iter().flatten() {
                let id = &cognate["id"];
                if id.is_string()
                    && cited.iter().any(|c| c == id)
                    && !cognates.iter().any(|c| &c["id"] == id)
                {
                    cognates.push(cognate.clone());
                }
            }
        }
    }
    if merged == before {
        return Ok(None);
    }
    // An addition reopens the article for sign-off.
    merged["status"] = json!("draft");
    if let Some(object) = merged.as_object_mut() {
        object.remove("reviewed");
    }
    if draft["updated"].is_string() {
        merged["updated"] = draft["updated"].clone();
    }
    Ok(Some(serde_json::to_string_pretty(&merged)? + "\n"))
}

fn add_senses_of_word(have: &mut Value, draft: &Value, cited: &mut Vec<Value>) {
    let mut ids = Vec::new();
    sense_ids(have, &mut ids);
    add_senses(have, &draft["senses"], &ids, cited);
    for stem in draft["stems"].as_array().into_iter().flatten() {
        let Some(object) = have.as_object_mut() else {
            return;
        };
        let stems = object.entry("stems").or_insert_with(|| json!([]));
        if stems.is_null() {
            *stems = json!([]);
        }
        let Some(stems) = stems.as_array_mut() else {
            return;
        };
        match stems.iter_mut().find(|s| s["stem"] == stem["stem"]) {
            Some(existing) => add_senses(existing, &stem["senses"], &ids, cited),
            None => {
                collect_cognate_ids(stem, cited);
                stems.push(stem.clone());
            }
        }
    }
}

/// Append to `parent`'s senses the draft's senses whose id the lexeme lacks,
/// descending into the narrower senses of those it has. Keys are only
/// created when something is added.
fn add_senses(parent: &mut Value, draft: &Value, ids: &[Value], cited: &mut Vec<Value>) {
    let Some(new) = draft.as_array() else { return };
    for sense in new {
        let present = parent
            .get_mut("senses")
            .and_then(Value::as_array_mut)
            .and_then(|list| list.iter_mut().find(|s| s["id"] == sense["id"]));
        match present {
            Some(existing) => add_senses(existing, &sense["senses"], ids, cited),
            None if sense["id"].is_string() && !ids.contains(&sense["id"]) => {
                let Some(object) = parent.as_object_mut() else {
                    return;
                };
                let list = object.entry("senses").or_insert_with(|| json!([]));
                if list.is_null() {
                    *list = json!([]);
                }
                if let Some(list) = list.as_array_mut() {
                    collect_cognate_ids(sense, cited);
                    list.push(sense.clone());
                }
            }
            None => {}
        }
    }
}

fn sense_ids(node: &Value, out: &mut Vec<Value>) {
    for key in ["senses", "stems"] {
        for item in node[key].as_array().into_iter().flatten() {
            if key == "senses" && item["id"].is_string() {
                out.push(item["id"].clone());
            }
            sense_ids(item, out);
        }
    }
}

fn collect_cognate_ids(node: &Value, out: &mut Vec<Value>) {
    match node {
        Value::Object(map) => {
            for (key, value) in map {
                if key == "cognate_ids" {
                    out.extend(value.as_array().into_iter().flatten().cloned());
                } else {
                    collect_cognate_ids(value, out);
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|v| collect_cognate_ids(v, out)),
        _ => {}
    }
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
    fn reviews_only_add_lexemes_and_senses() -> Result<()> {
        let current = json!({
            "id":"zh","root":"זה","status":"reviewed","reviewed":"2026-11-01","updated":"2026-11-01",
            "gloss":"this","cognates":[{"id":"ar-dha","lang":"ar","form":"ذا","gloss":"this"}],
            "words":[{"lemma":"זֶה","pos":"pronoun","bdb":["g.ah.aa"],"senses":[
                {"id":"this","gloss":"this","definition":"Points at a thing.",
                 "evidence":[{"ref":"Gen.37.19","note":"הַלָּזֶה בָּא","role":"defining"}],
                 "senses":[{"id":"this-near","gloss":"this here","evidence":[{"ref":"Gen.5.29"}]}]}]}]
        })
        .to_string();
        // The draft re-types everything: it corrupts a quotation, drops a
        // role and a citation, empties a key and changes the gloss, while
        // adding a narrower sense, a new top sense and a new lexeme.
        let draft = json!({
            "id":"zh","root":"זה","status":"draft","updated":"2026-11-21",
            "gloss":"that","summary":"rewritten",
            "cognates":[{"id":"ar-dha","lang":"ar","form":"ذا","gloss":"that"},
                        {"id":"gez-ze","lang":"gez","form":"ዝ","gloss":"this"},
                        {"id":"sab-dhn","lang":"sem","form":"ḏn","gloss":"this"}],
            "words":[
                {"lemma":"זֶה","pos":"pronoun","bdb":[],"senses":[
                    {"id":"this","gloss":"that","definition":"Changed.",
                     "evidence":[{"ref":"Gen.37.19","note":"הַלָּזֶה הַלָּזֶה בָּא"}],
                     "senses":[{"id":"this-near","gloss":"x","evidence":[]},
                               {"id":"this-time","gloss":"now","evidence":[{"ref":"Gen.27.36"}]}]},
                    {"id":"this-one","gloss":"this one","evidence":[{"ref":"Gen.15.7"}],"cognate_ids":["gez-ze"]}]},
                {"lemma":"זוֹ","pos":"pronoun","senses":[{"id":"zo","gloss":"this (f)","evidence":[{"ref":"Hos.7.16"}]}]}]
        });
        let merged: Value = serde_json::from_str(&additive(&current, &draft, false)?.unwrap())?;
        let zeh = &merged["words"][0];
        assert_eq!(merged["gloss"], "this");
        assert!(merged.get("summary").is_none());
        assert_eq!(zeh["bdb"], json!(["g.ah.aa"]));
        let this = &zeh["senses"][0];
        assert_eq!(this["definition"], "Points at a thing.");
        assert_eq!(this["evidence"][0]["note"], "הַלָּזֶה בָּא");
        assert_eq!(this["evidence"][0]["role"], "defining");
        assert_eq!(this["senses"][0]["evidence"], json!([{"ref":"Gen.5.29"}]));
        assert_eq!(this["senses"][1]["id"], "this-time");
        assert_eq!(zeh["senses"][1]["id"], "this-one");
        assert_eq!(merged["words"][1]["lemma"], "זוֹ");
        // Only the cognate a new sense cites is added; the old one is kept.
        assert_eq!(merged["cognates"].as_array().unwrap().len(), 2);
        assert_eq!(merged["cognates"][0]["gloss"], "this");
        assert_eq!(merged["cognates"][1]["id"], "gez-ze");
        // The addition reopens the article.
        assert_eq!(merged["status"], "draft");
        assert!(merged.get("reviewed").is_none());
        assert_eq!(merged["updated"], "2026-11-21");
        Ok(())
    }

    #[test]
    fn a_review_adding_nothing_leaves_the_article() -> Result<()> {
        let current =
            json!({"id":"zh","words":[{"lemma":"זֶה","senses":[{"id":"this","gloss":"this"}]}]});
        let draft = json!({"id":"zh","words":[{"lemma":"זֶה","senses":[{"id":"this","gloss":"rewritten"}]}]});
        assert_eq!(additive(&current.to_string(), &draft, false)?, None);
        Ok(())
    }

    #[test]
    fn a_name_review_only_adds_bearers() -> Result<()> {
        let current = json!({"id":"n","name":"שֵׁם","gloss":"name","people":["A@Gen.1.1"]});
        let draft = json!({"id":"n","name":"שֵׁם","gloss":"changed","people":["B@Gen.2.2"]});
        let merged: Value =
            serde_json::from_str(&additive(&current.to_string(), &draft, true)?.unwrap())?;
        assert_eq!(merged["gloss"], "name");
        assert_eq!(merged["people"], json!(["A@Gen.1.1", "B@Gen.2.2"]));
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
        // The checked-in entry may be signed off; a review drafts it.
        article["status"] = json!("draft");
        article.as_object_mut().unwrap().remove("reviewed");
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
        assert_eq!(
            article_path(&lexicon, "26m", false),
            lexicon.join("26m.json")
        );
        assert_eq!(
            article_path(&lexicon, "yrd", true),
            lexicon.join("names/yrd.json")
        );
    }
}
