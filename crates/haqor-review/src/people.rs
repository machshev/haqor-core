//! Identify a bearer in a particular verse independently of the name's lemma.
use crate::jobs::Job;
use anyhow::{Context, Result, ensure};
use haqor_core::{
    data_support::fold_consonants,
    haqor_lexicon,
    reviewed_names::{Bearer, Mention, PATH, ReviewedNames},
};
use rusqlite::{Connection, OpenFlags, params};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::Path,
};

pub const SCHEMA:&str="CREATE TABLE IF NOT EXISTS proposed_bearer(source TEXT,ref INTEGER,pos INTEGER,key TEXT,name TEXT,body TEXT,
 PRIMARY KEY(source,ref,pos)) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS proposed_by_name ON proposed_bearer(name);
CREATE INDEX IF NOT EXISTS proposed_by_key ON proposed_bearer(key);
CREATE TABLE IF NOT EXISTS name_lookup(source TEXT,ref INTEGER,name TEXT,PRIMARY KEY(source,ref,name)) WITHOUT ROWID;
CREATE TEMP TABLE IF NOT EXISTS known_bearer(key TEXT PRIMARY KEY,name TEXT,kind TEXT) WITHOUT ROWID;";

pub struct People {
    records: BTreeMap<String, Bearer>,
    ids: HashMap<u32, String>,
    forms: HashMap<String, BTreeSet<String>>,
    corpus: Connection,
    reviewed: ReviewedNames,
}

fn form(text: &str) -> String {
    let he = fold_consonants(text);
    if he.is_empty() {
        text.trim().to_lowercase()
    } else {
        he
    }
}
impl People {
    pub fn load(src: &Path, data: &Path) -> Result<Self> {
        let corpus =
            Connection::open_with_flags(data.join("haqor.db"), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let mut this = Self {
            records: Default::default(),
            ids: Default::default(),
            forms: Default::default(),
            corpus,
            reviewed: ReviewedNames::read(src)?,
        };
        for line in fs::read_to_string(src.join(haqor_lexicon::PEOPLE_RECORDS))?
            .lines()
            .filter(|s| !s.is_empty())
        {
            let r: Value = serde_json::from_str(line)?;
            let b: Bearer = serde_json::from_value(
                json!({"name":r["name"],"description":r["summary"].as_str().unwrap_or_default().lines().filter(|s|!s.contains("referred to as")).collect::<Vec<_>>().join(". "),"kind":r["kind"]}),
            )?;
            let key = r["key"].as_str().context("missing TIPNR key")?;
            this.add(
                r["id"].as_u64().context("missing TIPNR id")?.try_into()?,
                key,
                b,
            )?;
            for f in r["forms"].as_array().into_iter().flatten() {
                for s in f["english"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .chain(std::iter::once(&f["hebrew"]))
                    .filter_map(Value::as_str)
                {
                    this.forms.entry(form(s)).or_default().insert(key.into());
                }
            }
        }
        let splits: Value = serde_json::from_slice(&fs::read(data.join("place_overrides.json"))?)?;
        for r in splits["splits"].as_array().into_iter().flatten() {
            this.add(
                r["id"].as_u64().context("missing place id")?.try_into()?,
                r["key"].as_str().context("missing place key")?,
                Bearer {
                    name: r["name"].as_str().context("missing name")?.into(),
                    kind: "place".into(),
                    description: r["summary"].as_str().unwrap_or("").into(),
                },
            )?;
        }
        for (key, r) in &this.reviewed.records {
            ensure!(
                this.ids.insert(r.id, key.clone()).is_none()
                    && this.records.insert(key.clone(), r.bearer.clone()).is_none(),
                "duplicate reviewed identity {key}"
            );
            for s in [&r.bearer.name, &r.form] {
                this.forms.entry(form(s)).or_default().insert(key.clone());
            }
        }
        Ok(this)
    }
    fn add(&mut self, id: u32, key: &str, b: Bearer) -> Result<()> {
        self.forms
            .entry(form(&b.name))
            .or_default()
            .insert(key.into());
        ensure!(
            self.ids.insert(id, key.into()).is_none()
                && self.records.insert(key.into(), b).is_none(),
            "duplicate entity {key}"
        );
        Ok(())
    }
    pub fn context(&self, queue: &Connection, j: &Job, words: &[Value]) -> Result<Value> {
        let mut keys = BTreeSet::new();
        let mut hints = Vec::new();
        let sql = if j.source == "greek" {
            "SELECT position,entity_id FROM greek_word WHERE ref=? AND entity_id IS NOT NULL"
        } else {
            "SELECT position,entity_id FROM word_name WHERE ref=?"
        };
        for (pos, id) in self
            .corpus
            .prepare_cached(sql)?
            .query_map([j.reference], |r| {
                Ok((r.get::<_, u32>(0)?, r.get::<_, u32>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
        {
            if let Some(key) = self.ids.get(&id) {
                keys.insert(key.clone());
                hints.push(json!({"position":pos,"key":key}));
            }
        }
        if j.source == "peshitta" {
            for id in self
                .corpus
                .prepare_cached("SELECT entity_id FROM name_verse WHERE ref=?")?
                .query_map([j.reference], |r| r.get::<_, u32>(0))?
            {
                if let Some(key) = self.ids.get(&id?) {
                    keys.insert(key.clone());
                }
            }
        }
        let old = self
            .reviewed
            .verses
            .get(&j.source)
            .and_then(|v| v.get(&j.reference));
        for word in old.into_iter().flatten() {
            if let Some(key) = &word.key {
                keys.insert(key.clone());
            }
        }
        let mut spellings = queue
            .prepare_cached("SELECT name FROM name_lookup WHERE source=? AND ref=?")?
            .query_map(params![j.source, j.reference], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<BTreeSet<_>>>()?;
        for w in words {
            for s in [
                &w["surface"],
                &w["analysis"]["lemma"],
                &w["analysis"]["gloss"],
            ]
            .into_iter()
            .filter_map(Value::as_str)
            {
                spellings.insert(form(s));
            }
        }
        for k in &keys {
            if let Some(b) = self.records.get(k) {
                spellings.insert(form(&b.name));
            }
        }
        let mut drafts = BTreeMap::new();
        let mut previous=queue.prepare_cached("SELECT DISTINCT key,body FROM proposed_bearer p WHERE name=? AND EXISTS(SELECT 1 FROM job j WHERE j.source=p.source AND j.ref=p.ref AND pos=-1 AND state='done')")?;
        for s in spellings {
            if let Some(siblings) = self.forms.get(&s) {
                keys.extend(siblings.iter().cloned());
            }
            for row in previous.query_map([s], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })? {
                let (k, v) = row?;
                if !self.records.contains_key(&k) {
                    drafts.insert(k, serde_json::from_str::<Bearer>(&v)?);
                }
            }
        }
        let mut all_keys = keys.clone();
        for key in &keys {
            if let Some(record) = self.records.get(key) {
                if let Some(same) = self.forms.get(&form(&record.name)) {
                    all_keys.extend(same.iter().cloned());
                }
            }
        }
        let candidates: BTreeMap<_, _> = all_keys
            .into_iter()
            .filter_map(|k| {
                self.records
                    .get(&k)
                    .filter(|b| matches!(b.kind.as_str(), "person" | "place"))
                    .map(|b| (k, b))
            })
            .collect();
        Ok(
            json!({"candidates":candidates,"current_word_tags":hints,"previous_proposals":drafts,"published":old}),
        )
    }
}

/// Keep lookup hints even when the result transaction rolls back. On a
/// retry the verse reviewer sees namesakes added by another worker.
pub fn request_names(db: &Connection, j: &Job, answer: &Value) -> Result<()> {
    for m in answer["entities"].as_array().into_iter().flatten() {
        if let Some(name) = m["new"]["name"].as_str() {
            db.execute(
                "INSERT OR IGNORE INTO name_lookup VALUES(?,?,?)",
                params![j.source, j.reference, form(name)],
            )?;
        }
    }
    Ok(())
}

pub fn validate(db: &Connection, j: &Job, answer: &mut Value, packet: &Value) -> Result<()> {
    let mentions = answer["entities"]
        .as_array_mut()
        .context("missing entities list")?;
    let mut introduced = BTreeMap::new();
    for mention in mentions.iter() {
        if let (Some(key), Some(record)) = (mention["key"].as_str(), mention.get("new")) {
            ensure!(
                introduced
                    .insert(key.to_owned(), record.clone())
                    .is_none_or(|previous| previous == *record),
                "conflicting descriptions for {key}"
            );
        }
    }
    let mut seen = BTreeSet::new();
    db.execute(
        "DELETE FROM proposed_bearer WHERE source=? AND ref=?",
        params![j.source, j.reference],
    )?;
    for m in mentions {
        let pos = m["position"].as_u64().context("entity needs position")?;
        ensure!(
            seen.insert(pos)
                && packet["words"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|w| w["position"] == pos),
            "invalid or repeated entity position"
        );
        let key = m["key"].as_str().context("entity needs a key")?.to_owned();
        ensure!(
            m["evidence"].as_str().is_some_and(|s| !s.trim().is_empty()),
            "entity identification needs evidence"
        );
        let context = &packet["people_and_places"];
        let known: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM known_bearer WHERE key=? AND kind IN ('person','place'))",
            [&key],
            |r| r.get(0),
        )?;
        if known || !context["candidates"][&key].is_null() {
            ensure!(
                m.get("new").is_none(),
                "existing bearer {key} cannot be redefined"
            );
            continue;
        }
        if let Some(prior) = context["previous_proposals"].get(&key) {
            ensure!(
                m.get("new").is_none_or(|b| b == prior),
                "inconsistent proposed bearer {key}"
            );
            m["new"] = prior.clone();
        }
        if m.get("new").is_none() {
            if let Some(record) = introduced.get(&key) {
                m["new"] = record.clone();
            }
        }
        let bearer: Bearer = serde_json::from_value(m["new"].clone())
            .context("unknown bearer requires a new record")?;
        let (name, anchor) = key.split_once('@').context("new key must be Name@Ref")?;
        let anchor = anchor.split('#').next().unwrap_or(anchor);
        ensure!(
            haqor_lexicon::parse_reference(anchor).is_some()
                && !name.trim().is_empty()
                && name == bearer.name
                && matches!(bearer.kind.as_str(), "person" | "place")
                && !bearer.description.trim().is_empty(),
            "invalid new bearer {key}"
        );
        ensure!(
            !db.query_row(
                "SELECT EXISTS(SELECT 1 FROM known_bearer WHERE key=?)",
                [&key],
                |r| r.get::<_, bool>(0)
            )?,
            "key {key} already belongs to another entity"
        );
        let mut siblings=db.prepare_cached("SELECT key FROM known_bearer WHERE name=?1 AND kind IN ('person','place')
            UNION SELECT key FROM proposed_bearer p WHERE name=?1 AND EXISTS(SELECT 1 FROM job j WHERE j.source=p.source AND j.ref=p.ref AND j.pos=-1 AND j.state='done')")?;
        for sibling in siblings.query_map([form(&bearer.name)], |r| r.get::<_, String>(0))? {
            let sibling = sibling?;
            ensure!(
                context["candidates"].get(&sibling).is_some()
                    || context["previous_proposals"].get(&sibling).is_some(),
                "Other bearers of {} are available. Review the refreshed candidates before creating {key}.",
                bearer.name
            );
        }
        let body = serde_json::to_string(&bearer)?;
        let conflict:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM proposed_bearer p JOIN job j ON j.source=p.source AND j.ref=p.ref AND j.pos=-1 WHERE p.key=? AND p.body<>? AND (j.state='done' OR (p.source=? AND p.ref=?)))",params![key,body,j.source,j.reference],|r|r.get(0))?;
        ensure!(!conflict, "conflicting proposals for {key}");
        db.execute(
            "INSERT INTO proposed_bearer VALUES(?,?,?,?,?,?)",
            params![
                j.source,
                j.reference,
                pos as i64,
                key,
                form(&bearer.name),
                body
            ],
        )?;
    }
    answer["name_review_base"] = packet["people_and_places"]["published"].clone();
    Ok(())
}

pub fn publish(queue: &Connection, src: &Path, data: &Path, lexicon: &Path) -> Result<()> {
    let lock = fs::File::open(src.join(haqor_lexicon::PEOPLE_RECORDS))?;
    lock.try_lock()
        .context("another queue is publishing identities")?;
    let catalog = People::load(src, data)?;
    let mut next = catalog.ids.keys().copied().max().unwrap_or(0).max(100_000);
    let mut index = catalog.reviewed;
    let mut add_to_article: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut n = 0;
    let mut verses=queue.prepare("SELECT j.source,j.ref,review_result(j.result),v.packet FROM job j JOIN verse v USING(source,ref) WHERE pos=-1 AND state='done' ORDER BY ref,source")?;
    let mut rows = verses.query([])?;
    let mut analysis = queue.prepare_cached(
        "SELECT review_result(result) FROM job WHERE source=? AND ref=? AND pos=?",
    )?;
    while let Some(row) = rows.next()? {
        let edition: String = row.get(0)?;
        let reference: i64 = row.get(1)?;
        let review: Value = serde_json::from_str(&row.get::<_, String>(2)?)?;
        let identified = review["entities"]
            .as_array()
            .context("legacy verse: run its identity review first")?;
        let packet: Value = serde_json::from_str(&row.get::<_, String>(3)?)?;
        let mut words = Vec::new();
        for w in packet["words"].as_array().context("invalid verse")? {
            let position = w["position"].as_u64().context("invalid position")? as u32;
            let mention = identified.iter().find(|m| m["position"] == position);
            let key = mention.and_then(|m| m["key"].as_str());
            if let Some((m, key)) = mention.zip(key) {
                let word: Value = serde_json::from_str(
                    &analysis.query_row(params![edition, reference, position], |r| {
                        r.get::<_, String>(0)
                    })?,
                )?;
                if let Some(proposal) = m.get("new") {
                    let b: Bearer = serde_json::from_value(proposal.clone())?;
                    ensure!(
                        !catalog.records.contains_key(key) || index.records.contains_key(key),
                        "new identity collides with {key}"
                    );
                    index.add_record(
                        key,
                        &b,
                        word["lemma"].as_str().context("missing lemma")?,
                        &mut next,
                    )?;
                }
                let b = catalog
                    .records
                    .get(key)
                    .or_else(|| index.records.get(key).map(|r| &r.bearer))
                    .with_context(|| format!("unknown identity {key}"))?;
                let name_article:bool=queue.query_row("SELECT EXISTS(SELECT 1 FROM dependency WHERE source=?1 AND ref=?2 AND pos=?3 AND name=1 UNION ALL SELECT 1 FROM article WHERE source=?1 AND ref=?2 AND pos=?3 AND name=1)",params![edition,reference,position],|r|r.get(0))?;
                if b.kind == "person" && name_article {
                    if let Some(id) = word["entry"]["existing"]
                        .as_str()
                        .or(word["entry"]["draft"]["article_id"].as_str())
                    {
                        add_to_article
                            .entry(id.into())
                            .or_default()
                            .insert(key.into());
                    }
                }
            }
            words.push(Mention {
                position,
                surface: w["surface"].as_str().context("missing surface")?.into(),
                key: key.map(str::to_owned),
                evidence: mention
                    .and_then(|m| m["evidence"].as_str())
                    .map(str::to_owned),
            });
        }
        let current = index.verses.entry(edition.clone()).or_default();
        let old = current.get(&reference);
        if old != Some(&words) {
            if serde_json::to_value(old)? != review["name_review_base"]
                && old.is_some_and(|prior| {
                    prior.len() == words.len()
                        && prior.iter().zip(&words).all(|(a, b)| {
                            a.position == b.position && a.surface == b.surface && a.key == b.key
                        })
                })
            {
                n += 1;
                continue;
            }
            ensure!(
                serde_json::to_value(old)? == review["name_review_base"],
                "another queue changed {edition} {reference}; reopen its verse review"
            );
            current.insert(reference, words);
        }
        n += 1;
    }
    let path = src.join(PATH);
    fs::create_dir_all(path.parent().unwrap())?;
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&tmp, serde_json::to_string_pretty(&index)? + "\n")?;
    fs::rename(tmp, path)?;
    // Add a newly distinguished person to the shared name article without
    // replacing its other bearers. Only existing name articles are amended.
    let writer = fs::File::open(lexicon.join("sources.json"))?;
    writer.try_lock().context("lexicon is being edited")?;
    let refs = haqor_lexicon::verses_of(&catalog.corpus)?;
    for (id, keys) in add_to_article {
        ensure!(
            id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
            "invalid article id"
        );
        // A newly written name article may still be waiting for its verse
        // reviewer to identify the people who bear it.
        let mut stmt = queue
            .prepare("SELECT DISTINCT body FROM article WHERE id=? AND name=1 AND applied=0")?;
        let bodies = stmt
            .query_map([&id], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if !bodies.is_empty() {
            for body in bodies {
                let mut draft: Value = serde_json::from_str(&body)?;
                if add_bearers(&mut draft, &keys)? {
                    queue.execute(
                        "UPDATE article SET body=? WHERE id=? AND name=1 AND applied=0 AND body=?",
                        params![serde_json::to_string_pretty(&draft)?, id, body],
                    )?;
                }
            }
            continue;
        }
        let path = lexicon.join("names").join(format!("{id}.json"));
        if !path.try_exists()? {
            continue;
        }
        let mut a: Value = serde_json::from_slice(&fs::read(path)?)?;
        if add_bearers(&mut a, &keys)? {
            a["status"] = json!("draft");
            a.as_object_mut().unwrap().remove("reviewed");
            a["updated"] =
                json!(queue.query_row("SELECT date('now')", [], |r| r.get::<_, String>(0))?);
            haqor_lexicon::save_entry(lexicon, &serde_json::to_string_pretty(&a)?, &refs)?;
        }
    }
    println!("Published people and place identifications for {n} source verses.");
    Ok(())
}

/// Publish one completed verse while the review coordinator is running.
///
/// The records file is locked for the whole read/merge/write operation. This
/// makes a verse the publication boundary and means that two worker results
/// cannot overwrite each other's people or place assignments. The operation is
/// idempotent, so a coordinator restart may safely retry it.
pub fn publish_one(
    queue: &Connection,
    src: &Path,
    data: &Path,
    lexicon: &Path,
    edition: &str,
    reference: i64,
) -> Result<()> {
    let corpus =
        Connection::open_with_flags(data.join("haqor.db"), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let refs = haqor_lexicon::verses_of(&corpus)?;
    publish_one_with_refs(queue, src, data, lexicon, edition, reference, &refs)
}

/// Variant for a coordinator that has cached the immutable corpus verse set.
/// Caching this set avoids scanning the runtime database for every completed
/// verse in a long review run.
pub fn publish_one_with_refs(
    queue: &Connection,
    src: &Path,
    data: &Path,
    lexicon: &Path,
    edition: &str,
    reference: i64,
    refs: &std::collections::HashSet<i64>,
) -> Result<()> {
    let lock = fs::File::open(src.join(haqor_lexicon::PEOPLE_RECORDS))?;
    lock.try_lock()
        .context("another process is publishing people and places")?;
    let catalog = People::load(src, data)?;
    let review: Value = queue
        .query_row(
            "SELECT review_result(result) FROM job WHERE source=? AND ref=? AND pos=-1 AND state='done'",
            params![edition, reference],
            |r| r.get::<_, String>(0),
        )
        .map(|s| serde_json::from_str(&s))??;
    let packet: Value = queue
        .query_row(
            "SELECT packet FROM verse WHERE source=? AND ref=?",
            params![edition, reference],
            |r| r.get::<_, String>(0),
        )
        .map(|s| serde_json::from_str(&s))??;
    let identified = review["entities"]
        .as_array()
        .context("legacy verse: run its identity review first")?;
    let mut index = catalog.reviewed;
    let mut next = catalog.ids.keys().copied().max().unwrap_or(0).max(100_000);
    let mut add_to_article: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut analysis = queue.prepare_cached(
        "SELECT review_result(result) FROM job WHERE source=? AND ref=? AND pos=? AND state='done'",
    )?;
    let mut words = Vec::new();
    for w in packet["words"].as_array().context("invalid verse")? {
        let position = w["position"].as_u64().context("invalid position")? as u32;
        let mention = identified.iter().find(|m| m["position"] == position);
        let key = mention.and_then(|m| m["key"].as_str());
        if let Some((m, key)) = mention.zip(key) {
            let word: Value = serde_json::from_str(
                &analysis.query_row(params![edition, reference, position], |r| {
                    r.get::<_, String>(0)
                })?,
            )?;
            if let Some(proposal) = m.get("new") {
                let b: Bearer = serde_json::from_value(proposal.clone())?;
                index.add_record(
                    key,
                    &b,
                    word["lemma"].as_str().context("missing lemma")?,
                    &mut next,
                )?;
            }
            let b = catalog
                .records
                .get(key)
                .or_else(|| index.records.get(key).map(|r| &r.bearer))
                .with_context(|| format!("unknown identity {key}"))?;
            let name_article: bool = queue.query_row(
                "SELECT EXISTS(SELECT 1 FROM dependency WHERE source=?1 AND ref=?2 AND pos=?3 AND name=1 UNION ALL SELECT 1 FROM article WHERE source=?1 AND ref=?2 AND pos=?3 AND name=1)",
                params![edition, reference, position], |r| r.get(0))?;
            if b.kind == "person" && name_article {
                if let Some(id) = word["entry"]["existing"]
                    .as_str()
                    .or(word["entry"]["draft"]["article_id"].as_str())
                {
                    add_to_article
                        .entry(id.into())
                        .or_default()
                        .insert(key.into());
                }
            }
        }
        words.push(Mention {
            position,
            surface: w["surface"].as_str().context("missing surface")?.into(),
            key: key.map(str::to_owned),
            evidence: mention
                .and_then(|m| m["evidence"].as_str())
                .map(str::to_owned),
        });
    }
    let current = index.verses.entry(edition.to_owned()).or_default();
    let old = current.get(&reference);
    if accept_published_mentions(old, &review["name_review_base"], &words)? {
        current.insert(reference, words);
        let path = src.join(PATH);
        fs::create_dir_all(path.parent().unwrap())?;
        let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
        fs::write(&tmp, serde_json::to_string_pretty(&index)? + "\n")?;
        fs::rename(tmp, path)?;
    }
    let writer = fs::File::open(lexicon.join("sources.json"))?;
    writer.try_lock().context("lexicon is being edited")?;
    for (id, keys) in add_to_article {
        ensure!(
            id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
            "invalid article id"
        );
        let path = lexicon.join("names").join(format!("{id}.json"));
        if !path.try_exists()? {
            continue;
        }
        let mut article: Value = serde_json::from_slice(&fs::read(&path)?)?;
        if add_bearers(&mut article, &keys)? {
            article["status"] = json!("draft");
            article.as_object_mut().unwrap().remove("reviewed");
            article["updated"] =
                json!(queue.query_row("SELECT date('now')", [], |r| r.get::<_, String>(0))?);
            haqor_lexicon::save_entry(lexicon, &serde_json::to_string_pretty(&article)?, &refs)?;
        }
    }
    Ok(())
}

fn accept_published_mentions(
    current: Option<&Vec<Mention>>,
    base: &Value,
    proposed: &Vec<Mention>,
) -> Result<bool> {
    if current == Some(proposed) {
        return Ok(false);
    }
    ensure!(
        serde_json::to_value(current)? == *base,
        "another process changed the verse's identities; the verse needs reconciliation"
    );
    Ok(true)
}

/// Return the tracked paths that a verse publication may modify. This is
/// useful to a coordinator that commits each accepted verse immediately.
pub fn publication_paths(
    queue: &Connection,
    src: &Path,
    lexicon: &Path,
    edition: &str,
    reference: i64,
) -> Result<BTreeSet<std::path::PathBuf>> {
    let mut paths = BTreeSet::from([src.join(PATH)]);
    let entities: Value = queue
        .query_row(
        "SELECT review_result(result) FROM job WHERE source=? AND ref=? AND pos=-1 AND state='done'",
            params![edition, reference],
            |r| r.get::<_, String>(0),
        )
        .map(|s| serde_json::from_str(&s))??;
    let mut analysis = queue.prepare_cached(
        "SELECT review_result(result) FROM job WHERE source=? AND ref=? AND pos=? AND state='done'",
    )?;
    for mention in entities["entities"].as_array().into_iter().flatten() {
        let position = mention["position"]
            .as_i64()
            .context("entity needs position")?;
        let word: Value = serde_json::from_str(
            &analysis.query_row(params![edition, reference, position], |r| {
                r.get::<_, String>(0)
            })?,
        )?;
        let id = word["entry"]["existing"]
            .as_str()
            .or(word["entry"]["draft"]["article_id"].as_str());
        if let Some(id) = id {
            let path = lexicon.join("names").join(format!("{id}.json"));
            if path.try_exists()? {
                paths.insert(path);
            }
        }
    }
    Ok(paths)
}

fn add_bearers(article: &mut Value, keys: &BTreeSet<String>) -> Result<bool> {
    let people = article["people"]
        .as_array_mut()
        .context("name article lacks people")?;
    let before = people.len();
    for key in keys {
        if !people.contains(&json!(key)) {
            people.push(json!(key));
        }
    }
    Ok(people.len() != before)
}

impl People {
    // Preserve word work from older queues. Reopen a verse if another queue
    // published a different identification after this reviewer saw its input.
    pub fn refresh(&self, queue: &mut Connection) -> Result<()> {
        let tx = queue.transaction()?;
        tx.execute("DELETE FROM known_bearer", [])?;
        {
            let mut add = tx.prepare("INSERT INTO known_bearer VALUES(?,?,?)")?;
            for (key, bearer) in &self.records {
                add.execute(params![key, form(&bearer.name), bearer.kind])?;
            }
        }

        tx.execute("UPDATE job SET state='ready',attempts=0,feedback=json_object('instruction','Review people and places, preserving prior approved English where sound.','prior_review',json(review_result(result))),result=NULL
            WHERE pos=-1 AND state='done' AND json_type(review_result(result),'$.entities') IS NULL",[])?;
        let mut stale = Vec::new();
        {
            let mut stmt = tx.prepare(
                "SELECT source,ref,review_result(result) FROM job WHERE pos=-1 AND state='done'",
            )?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let edition: String = row.get(0)?;
                let reference: i64 = row.get(1)?;
                let review: Value = serde_json::from_str(&row.get::<_, String>(2)?)?;
                let published = self
                    .reviewed
                    .verses
                    .get(&edition)
                    .and_then(|verses| verses.get(&reference));
                if review["name_review_base"] == serde_json::to_value(published)? {
                    continue;
                }
                let actual: BTreeMap<_, _> = published
                    .into_iter()
                    .flatten()
                    .filter_map(|m| Some((u64::from(m.position), m.key.as_deref()?)))
                    .collect();
                let proposed: BTreeMap<_, _> = review["entities"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|m| Some((m["position"].as_u64()?, m["key"].as_str()?)))
                    .collect();
                if published.is_none() || actual != proposed {
                    stale.push((edition, reference));
                }
            }
        }
        for (edition, reference) in stale {
            tx.execute("UPDATE job SET state='ready',attempts=0,feedback=json_object('instruction','Published identities changed. Reconcile this verse.','prior_review',json(review_result(result))),result=NULL WHERE source=? AND ref=? AND pos=-1",params![edition,reference])?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_spelling_can_identify_existing_and_new_bearers() -> Result<()> {
        let db = Connection::open_in_memory()?;
        db.execute_batch(crate::jobs::SCHEMA)?;
        db.execute_batch(SCHEMA)?;
        let job = Job {
            source: "ot".into(),
            reference: 65793,
            pos: -1,
        };
        let packet = json!({"words":[{"position":0},{"position":1},{"position":2}],"people_and_places":{"candidates":{"A@Gen.2.3":{"name":"A"}}}});
        let existing = json!({"position":0,"key":"A@Gen.2.3","evidence":"father"});
        let new = json!({"position":1,"key":"A@Gen.1.1","evidence":"son","new":{"name":"A","kind":"person","description":"son of A"}});
        let repeated = json!({"position":2,"key":"A@Gen.1.1","evidence":"Same son named again"});
        let mut result = json!({"entities":[existing,repeated,new]});
        validate(&db, &job, &mut result, &packet)?;
        assert_eq!(
            db.query_row("SELECT count(*) FROM proposed_bearer", [], |r| r
                .get::<_, u32>(0))?,
            2
        );
        let mut invalid = json!({"entities":[existing,existing]});
        assert!(validate(&db, &job, &mut invalid, &packet).is_err());
        let mut unknown = json!({"entities":[{"position":0,"key":"Unknown","evidence":"fixture"}]});
        assert!(validate(&db, &job, &mut unknown, &packet).is_err());
        Ok(())
    }

    #[test]
    fn live_publication_is_idempotent_and_rejects_stale_base() -> Result<()> {
        let proposed = vec![Mention {
            position: 0,
            surface: "David".into(),
            key: Some("David@1Sam.17.1".into()),
            evidence: Some("the named king".into()),
        }];
        assert!(!accept_published_mentions(
            Some(&proposed),
            &json!(proposed),
            &proposed
        )?);
        assert!(accept_published_mentions(None, &Value::Null, &proposed)?);
        assert!(accept_published_mentions(Some(&vec![]), &Value::Null, &proposed).is_err());
        Ok(())
    }

    #[test]
    fn bearer_article_merge_is_idempotent() -> Result<()> {
        let mut article = json!({"people":["David@1Sam.17.1"]});
        let keys = BTreeSet::from(["David@1Sam.17.1".to_owned(), "David@2Sam.5.1".to_owned()]);
        assert!(add_bearers(&mut article, &keys)?);
        assert!(!add_bearers(&mut article, &keys)?);
        assert_eq!(article["people"].as_array().map(Vec::len), Some(2));
        Ok(())
    }
}
