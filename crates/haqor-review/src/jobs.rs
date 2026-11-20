use anyhow::{Context, Result, bail, ensure};
use haqor_core::{data_support::fold_consonants, haqor_lexicon, translation};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use unicode_normalization::UnicodeNormalization;

pub const SCHEMA: &str = "
CREATE TEMP TABLE IF NOT EXISTS review_inflight(source TEXT,ref INTEGER,pos INTEGER,PRIMARY KEY(source,ref,pos)) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS verse(source TEXT, ref INTEGER, packet TEXT NOT NULL, PRIMARY KEY(source,ref)) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS job(source TEXT, ref INTEGER, pos INTEGER, state TEXT NOT NULL,
 attempts INTEGER NOT NULL DEFAULT 0, result TEXT, feedback TEXT, error TEXT, usage TEXT,
 PRIMARY KEY(source,ref,pos), FOREIGN KEY(source,ref) REFERENCES verse) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS article(source TEXT,ref INTEGER,pos INTEGER,id TEXT NOT NULL,name INTEGER NOT NULL,
 base TEXT,body TEXT NOT NULL,applied INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(source,ref,pos),
 FOREIGN KEY(source,ref,pos) REFERENCES job ON DELETE CASCADE) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS corrected_headword(source TEXT,ref INTEGER,pos INTEGER,lemma TEXT NOT NULL,PRIMARY KEY(source,ref,pos),FOREIGN KEY(source,ref,pos) REFERENCES job ON DELETE CASCADE) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS retired(language TEXT,lemma TEXT,qualifier TEXT,reason TEXT NOT NULL,
 PRIMARY KEY(language,lemma,qualifier)) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS pending_articles ON article(ref,source,pos) WHERE applied=0;
CREATE TABLE IF NOT EXISTS dependency(source TEXT,ref INTEGER,pos INTEGER,id TEXT,name INTEGER,revision TEXT,
 PRIMARY KEY(source,ref,pos),FOREIGN KEY(source,ref,pos) REFERENCES job ON DELETE CASCADE) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS dependency_article ON dependency(id,name,revision);
CREATE TABLE IF NOT EXISTS attempt(source TEXT,ref INTEGER,pos INTEGER,elapsed_ms INTEGER,usage TEXT,error TEXT);
CREATE INDEX IF NOT EXISTS recent_lemmas ON job(source,json_extract(result,'$.lemma')) WHERE pos>=0;
CREATE INDEX IF NOT EXISTS recent_roots ON job(source,json_extract(result,'$.root')) WHERE pos>=0;
CREATE INDEX IF NOT EXISTS ready_verses ON job(ref,source) WHERE state='ready' AND pos=-1;
CREATE INDEX IF NOT EXISTS ready_words ON job(ref,source,pos) WHERE state='ready' AND pos>=0;
CREATE INDEX IF NOT EXISTS lexemes ON job(json_extract(result,'$.language'),json_extract(result,'$.lemma'),json_extract(result,'$.qualifier')) WHERE pos>=0;
";

#[derive(Clone, Debug)]
pub struct Job {
    pub source: String,
    pub reference: i64,
    pub pos: i64,
}
impl Job {
    pub fn key(&self) -> (&str, i64, i64) {
        (&self.source, self.reference, self.pos)
    }
    fn positions(&self, db: &Connection) -> Result<BTreeSet<u16>> {
        Ok(db
            .prepare_cached("SELECT pos FROM job WHERE source=? AND ref=? AND pos>=0")?
            .query_map(params![self.source, self.reference], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    }
}

#[derive(Default)]
pub struct Catalog {
    pub people: Option<crate::people::People>,
    words: BTreeMap<String, Vec<Value>>,
    pub revisions: BTreeMap<(String, bool), String>,
}
impl Catalog {
    pub fn load(path: &Path) -> Result<Self> {
        let mut catalog = Self::default();
        for (_, text) in haqor_lexicon::read_entry_texts(path)?
            .into_iter()
            .chain(haqor_lexicon::read_name_texts(path)?)
        {
            let revision = crate::articles::revision(&text);
            let e: Value = serde_json::from_str(&text)?;
            catalog.revisions.insert(
                (
                    e["id"].as_str().context("missing article id")?.to_owned(),
                    e.get("name").is_some(),
                ),
                revision.clone(),
            );
            let words = e["words"].as_array().cloned().unwrap_or_else(|| {
                vec![json!({"lemma":e["name"],"pos":"name","gloss":e["gloss"]})]
            });
            for w in words {
                let root = w["root"].as_str().or(e["root"].as_str()).unwrap_or("");
                let lemma = w["lemma"].as_str().unwrap_or("");
                let summary = json!({"entry":e["id"],"revision":revision,"name":e.get("name").is_some(),"root":root,"lemma":lemma,"language":w["lang"].as_str().unwrap_or("he"),
                    "pos":w["pos"],"gloss":w["gloss"],"senses":w["senses"].as_array().into_iter().flatten().map(|s| &s["gloss"]).collect::<Vec<_>>()});
                for k in BTreeSet::from([fold_consonants(root), fold_consonants(lemma)]) {
                    if !k.is_empty() {
                        catalog.words.entry(k).or_default().push(summary.clone());
                    }
                }
            }
        }
        let greek = path.join("greek");
        if greek.try_exists()? {
            for entry in std::fs::read_dir(&greek)? {
                let file = entry?.path();
                if file.extension().and_then(|x| x.to_str()) != Some("json") {
                    continue;
                }
                let raw = std::fs::read_to_string(&file)?;
                let a: Value = serde_json::from_str(&raw)?;
                let key = crate::greek::id(&a)?;
                ensure!(
                    a["id"] == key && crate::greek::path(path, &key)? == file,
                    "misfiled Greek article {}",
                    file.display()
                );
                let revision = crate::articles::revision(&raw);
                catalog
                    .revisions
                    .insert((key.clone(), false), revision.clone());
                catalog.words.entry(a["lemma"].as_str().unwrap().into()).or_default().push(json!({"entry":key,"name":false,"language":"grc","lemma":a["lemma"],"qualifier":a["qualifier"],"root":a["root"],"revision":revision,"senses":a["senses"].as_object().context("Greek dictionary senses missing")?.iter().map(|(sense,d)|json!({"sense":sense,"gloss":d["gloss"],"definition":d["definition"]})).collect::<Vec<_>>()}));
            }
        }
        Ok(catalog)
    }
}

pub fn next(db: &Connection) -> Result<Option<Job>> {
    // Ready verse reviews get the first free slot, across both kinds of worker.
    for predicate in ["pos=-1", "pos>=0"] {
        let sql = format!(
            "SELECT source,ref,pos FROM job WHERE state='ready' AND {predicate} AND NOT EXISTS(SELECT 1 FROM review_inflight AS i WHERE i.source=job.source AND i.ref=job.ref AND i.pos=job.pos) ORDER BY ref,source,pos LIMIT 1"
        );
        if let Some(job) = db
            .prepare_cached(&sql)?
            .query_row([], |r| {
                Ok(Job {
                    source: r.get(0)?,
                    reference: r.get(1)?,
                    pos: r.get(2)?,
                })
            })
            .optional()?
        {
            return Ok(Some(job));
        }
    }
    Ok(None)
}

pub fn packet(db: &Connection, j: &Job, catalog: &Catalog) -> Result<Value> {
    let (raw, feedback, error): (String, Option<String>, Option<String>) = db.query_row(
        "SELECT review_input(verse.source,verse.ref,packet),feedback,error FROM verse JOIN job USING(source,ref)
      WHERE source=? AND ref=? AND pos=?",
        j.key(),
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let verse: Value = serde_json::from_str(&raw)?;
    let date: String = db.query_row("SELECT date('now')", [], |r| r.get(0))?;
    let mut p = json!({"date":date,"source":j.source,"book":j.reference>>16,"chapter":(j.reference>>8)&255,"verse":j.reference&255,
        "position":j.pos,"feedback":feedback,"last_error":error});
    let words = verse["words"].as_array().context("invalid inventory")?;
    if j.pos >= 0 {
        let word = words
            .iter()
            .find(|w| w["position"] == j.pos)
            .context("missing word")?;
        p["kind"] = json!("word");
        p["text"] = json!(
            words
                .iter()
                .map(|w| json!([w["position"], w["surface"]]))
                .collect::<Vec<_>>()
        );
        p["word"] = word.clone();
        let corrected: Option<String> = db
            .query_row(
                "SELECT lemma FROM corrected_headword WHERE source=? AND ref=? AND pos=?",
                j.key(),
                |r| r.get(0),
            )
            .optional()?;
        let corrected = json!(corrected);
        let mut candidates = Vec::new();
        for key in [
            &corrected,
            &word["source_analysis"]["root"],
            &word["source_analysis"]["lemma"],
            &word["surface"],
        ] {
            if let Some(items) = key.as_str().and_then(|k| {
                catalog.words.get(&if j.source == "greek" {
                    k.nfc().collect::<String>()
                } else {
                    fold_consonants(k)
                })
            }) {
                for item in items {
                    if !candidates.contains(item) {
                        candidates.push(item.clone());
                    }
                }
            }
        }
        let surface = word["surface"].as_str().unwrap_or("");
        let lemma = word["source_analysis"]["lemma"].as_str().unwrap_or(surface);
        let language = match j.source.as_str() {
            "peshitta" => "syc",
            "greek" => "grc",
            _ if word["source_analysis"]["oshb_morph"]
                .as_str()
                .is_some_and(|s| s.starts_with('A')) =>
            {
                "arc"
            }
            _ => "he",
        };
        candidates.sort_by_key(|c| {
            let head = c["lemma"].as_str().unwrap_or("");
            (
                c["lemma"] != corrected,
                c["language"] != language,
                head != lemma && head != surface,
                fold_consonants(head) != fold_consonants(lemma),
            )
        });
        candidates.truncate(24);
        p["candidates"] = json!(candidates);
        let root = word["source_analysis"]["root"].as_str().unwrap_or("");
        let lemma = word["source_analysis"]["lemma"]
            .as_str()
            .or(word["surface"].as_str())
            .unwrap_or("")
            .nfc()
            .collect::<String>();
        let mut stmt=db.prepare_cached("SELECT ref,pos,review_result(result) FROM job WHERE source=?1 AND pos>=0 AND json_extract(result,'$.lemma')=?2
            UNION ALL SELECT ref,pos,review_result(result) FROM job WHERE source=?1 AND pos>=0 AND ?3<>'' AND json_extract(result,'$.root')=?3 LIMIT 12")?;
        let mut rows = stmt.query(params![j.source, lemma, root])?;
        let mut previous = Vec::new();
        let mut examples = BTreeSet::new();
        while let Some(row) = rows.next()? {
            let a: Value = serde_json::from_str(&row.get::<_, String>(2)?)?;
            if !examples.insert(
                json!([
                    a["language"],
                    a["lemma"],
                    a["qualifier"],
                    a["root"],
                    a["sense"]
                ])
                .to_string(),
            ) {
                continue;
            }
            previous.push(json!({"at":[row.get::<_,i64>(0)?,row.get::<_,i64>(1)?],"language":a["language"],"lemma":a["lemma"],"qualifier":a["qualifier"],"root":a["root"],"sense":a["sense"],"entry":a["entry"]}));
        }
        p["previous_analyses"] = json!(previous);
    } else {
        p["kind"] = json!("verse");
        p["english"] = verse["english"].clone();
        let mut stmt=db.prepare_cached("SELECT pos,review_result(result) FROM job WHERE source=? AND ref=? AND pos>=0 AND state='done' ORDER BY pos")?;
        let analyses = stmt
            .query_map(params![j.source, j.reference], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ensure!(analyses.len() == words.len(), "verse has unfinished words");
        let mut reviewed = Vec::new();
        for ((pos, result), word) in analyses.into_iter().zip(words) {
            ensure!(word["position"] == pos, "position mismatch");
            reviewed.push(json!({"position":pos,"surface":word["surface"],"analysis":serde_json::from_str::<Value>(&result)?}));
        }
        if let Some(people) = &catalog.people {
            p["people_and_places"] = people.context(db, j, &reviewed)?;
        }
        p["words"] = json!(reviewed);
    }
    Ok(p)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Word {
    language: String,
    lemma: String,
    qualifier: String,
    root: String,
    morphology: String,
    sense: String,
    gloss: String,
    evidence: String,
    entry: Value,
}
const WORD: &str = r#"Analyse only the marked occurrence in its verse. Return one JSON object with string fields language (he, arc, syc, grc), lemma (pointed dictionary form, Hebrew script for Semitic), qualifier (empty unless needed to distinguish a homograph, e.g. noun), root (Haqor consonantal root; empty if unknown), morphology, sense, gloss, evidence (explain using the text), and entry. For entry use {"existing":"article id"} when the candidate covers this word and sense. Otherwise provide {"draft":{"definition":"...","evidence":"...","change":"..."}} proposing the new or corrected lexical content. If a previous analysis covers exactly your language, lemma, qualifier, root and sense, copy those fields and return entry {"reuse":[ref,pos]} using its at coordinates. For a new or amended Semitic entry include draft.article, the COMPLETE draft Haqor JSON article, following data/lexicon/README.md, with its lemma, root, senses and verse evidence. For a personal-name article preserve its existing people list; a new article may leave people empty until verse review assigns its bearers. Read the existing article before editing it. Use the candidate revision as draft.base, or null for a new file. If you discover an existing article outside the listed candidates use its SHA256 file digest. For Greek use an existing Greek dictionary candidate if it covers this sense, or provide draft.definition, draft.evidence and draft.change to update the shared Greek entry. Use Greek script for Greek. Treat source tags and candidate roots as evidence to check. If the occurrence cannot be assigned responsibly, return only {"unresolved":"explain the competing readings and what evidence is missing"}. Prefixes and suffixes belong in morphology. Use read-only local reference material when needed."#;
const VERSE: &str = r#"Review every word's gloss in context, then read the whole verse in continuous English. Return {"translation":"...","glosses":[{"position":0,"gloss":"..."}]} with exactly one gloss per listed position. In translation encode English spans as [English|position] (multiple positions comma-separated only when necessary), supplied words in braces. Account for every source position. Preserve the project's translation conventions. If a word analysis needs changing first, instead return {"rework":[{"position":0,"reason":"..."}]}. Give precise reasons and return only the affected positions. On every completed verse include entities: an array of {position,key,evidence} for EACH explicit person or place name. Empty means no such names. Match the actual bearer using verse context, family, role and location. Check all same-name candidates; source tags are evidence. Reuse the existing bearer key wherever supported, including previous proposals. If a bearer is missing, add new:{name,kind,description} to its mention, with kind person or place and a description distinguishing it from homonyms. Use a stable Name@Book.Chapter.Verse key anchored to evidence; when the same verse introduces two namesakes append # plus a distinguishing role. Distinct bearers may have identical names and dictionary forms. Preserve uncertain identity for review by returning {"unresolved":"reason"} rather than inventing a bearer. Do not identify mere pronouns or ordinary nouns as names."#;
pub fn prompt(packet: &Value) -> String {
    format!(
        "{}\nUse the published analysis or translation in this packet as the current edition when making corrections. Preserve prior supported content. Return JSON only. Source packet:\n{}",
        if packet["kind"] == "word" {
            WORD
        } else {
            VERSE
        },
        packet
    )
}

fn nonempty(v: &Value, k: &str) -> Result<()> {
    ensure!(
        v[k].as_str().is_some_and(|s| !s.trim().is_empty()),
        "missing {k}"
    );
    Ok(())
}

pub fn accept(
    db: &mut Connection,
    j: &Job,
    mut answer: Value,
    usage: &Value,
    packet: &Value,
) -> Result<()> {
    if j.pos < 0 && packet.get("people_and_places").is_some() {
        crate::people::request_names(db, j, &answer)?;
    }
    let tx = db.transaction()?;
    if answer.get("unresolved").is_some() {
        nonempty(&answer, "unresolved")?;
        tx.execute("UPDATE job SET state='unresolved',feedback=?,result=NULL,usage=?,error=NULL WHERE source=? AND ref=? AND pos=?",
                params![answer["unresolved"].as_str(),usage.to_string(),j.source,j.reference,j.pos])?;
        tx.commit()?;
        return Ok(());
    }
    if j.pos >= 0 {
        let word: Word = serde_json::from_value(answer.clone())?;
        for field in ["lemma", "morphology", "gloss", "sense", "evidence"] {
            nonempty(&answer, field)?;
        }
        ensure!(
            match j.source.as_str() {
                "ot" => matches!(word.language.as_str(), "he" | "arc"),
                "peshitta" => word.language == "syc",
                "greek" => word.language == "grc",
                _ => false,
            },
            "incorrect language"
        );
        for key in ["lemma", "qualifier", "root"] {
            let s = answer[key]
                .as_str()
                .context("invalid key")?
                .trim()
                .nfc()
                .collect::<String>();
            ensure!(!s.chars().any(char::is_numeric), "numeric lexical key");
            answer[key] = json!(s);
        }
        let lemma = answer["lemma"].as_str().unwrap();
        let retired: Option<String> = tx
            .query_row(
                "SELECT reason FROM retired WHERE language=? AND lemma=? AND qualifier=?",
                params![word.language, lemma, answer["qualifier"].as_str()],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(reason) = retired {
            bail!("retired lexical key: {reason}")
        }
        ensure!(
            lemma.chars().any(|c| if word.language == "grc" {
                matches!(c,'\u{0370}'..='\u{03ff}'|'\u{1f00}'..='\u{1fff}')
            } else {
                matches!(c, 'א'..='ת')
            }),
            "lemma needs source script"
        );
        if word.language != "grc" && !word.root.is_empty() {
            ensure!(
                word.root == fold_consonants(&word.root),
                "expected consonantal root key"
            );
        }
        if let Some(at) = word.entry.get("reuse") {
            let prior = packet["previous_analyses"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|a| {
                    &a["at"] == at
                        && ["language", "lemma", "qualifier", "root", "sense"]
                            .iter()
                            .all(|k| a[k] == answer[k])
                })
                .context("reuse must identify a matching prior analysis")?;
            let reference = at[0].as_i64().context("invalid reuse address")?;
            let pos = at[1].as_i64().context("invalid reuse position")?;
            let raw: String = tx
                .query_row(
                    "SELECT review_result(result) FROM job WHERE source=? AND ref=? AND pos=? AND state='done'",
                    params![j.source, reference, pos],
                    |r| r.get(0),
                )
                .context("prior word no longer current")?;
            let current: Value = serde_json::from_str(&raw)?;
            ensure!(
                ["language", "lemma", "qualifier", "root", "sense", "entry"]
                    .iter()
                    .all(|k| prior[k] == current[k]),
                "prior analysis changed"
            );
            answer["entry"] = prior["entry"].clone();
            tx.execute(
                "DELETE FROM dependency WHERE source=? AND ref=? AND pos=?",
                j.key(),
            )?;
            tx.execute(
                "DELETE FROM article WHERE source=? AND ref=? AND pos=?",
                j.key(),
            )?;
            tx.execute("INSERT INTO dependency SELECT source,?1,?2,id,name,revision FROM dependency WHERE source=?3 AND ref=?4 AND pos=?5",
                params![j.reference,j.pos,j.source,reference,pos])?;
            tx.execute("INSERT INTO article SELECT source,?1,?2,id,name,base,body,applied FROM article WHERE source=?3 AND ref=?4 AND pos=?5",
                params![j.reference,j.pos,j.source,reference,pos])?;
        } else {
            if let Some(id) = word.entry["existing"].as_str() {
                let candidate = packet["candidates"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|r| {
                        if r["entry"] != id
                            || r["root"] != answer["root"]
                            || (word.language == "grc" && r["qualifier"] != answer["qualifier"])
                        {
                            return false;
                        }
                        let Some(head) = r["lemma"].as_str() else {
                            return false;
                        };
                        if r["name"] == true && matches!(word.language.as_str(), "arc" | "syc") {
                            return fold_consonants(head) == fold_consonants(lemma);
                        }
                        r["language"] == answer["language"] && head.nfc().eq(lemma.chars())
                    })
                    .context("article must cover the resolved dictionary form and root")?;
                tx.execute(
                    "INSERT OR REPLACE INTO dependency VALUES(?,?,?,?,?,?)",
                    params![
                        j.source,
                        j.reference,
                        j.pos,
                        id,
                        candidate["name"].as_bool(),
                        candidate["revision"].as_str()
                    ],
                )?;
            } else {
                tx.execute(
                    "DELETE FROM dependency WHERE source=? AND ref=? AND pos=?",
                    j.key(),
                )?;
                for field in ["definition", "evidence", "change"] {
                    nonempty(&word.entry["draft"], field)?
                }
            }
            crate::articles::stage(&tx, j, &mut answer, packet)?;
            if j.source == "greek" && answer["entry"]["draft"].is_object() {
                answer["entry"]["draft"]["article_id"] = json!(crate::greek::id(&answer)?);
            }
        }
    } else {
        let positions = j.positions(&tx)?;
        if let Some(rework) = answer.get("rework") {
            let items = rework.as_array().context("invalid rework")?;
            ensure!(!items.is_empty(), "empty rework");
            for item in items {
                let pos = position(item, &positions)?;
                nonempty(item, "reason")?;
                tx.execute("UPDATE job SET state='ready', feedback=?,result=NULL WHERE source=? AND ref=? AND pos=?",
                    params![item["reason"].as_str(),j.source,j.reference,pos])?;
            }
            tx.execute("UPDATE job SET state='waiting',result=NULL,feedback=?,usage=? WHERE source=? AND ref=? AND pos=-1",
                params![answer.to_string(),usage.to_string(),j.source,j.reference])?;
            tx.commit()?;
            return Ok(());
        }
        if packet.get("people_and_places").is_some() {
            crate::people::validate(&tx, j, &mut answer, packet)?;
        }
        nonempty(&answer, "translation")?;
        let spans = translation::parse(
            answer["translation"].as_str().unwrap(),
            (j.reference >> 8) as u8,
            j.reference as u8,
        )
        .context("invalid translation spans")?;
        let mut linked = BTreeSet::new();
        for s in spans {
            for w in s.words {
                ensure!(
                    !s.text.trim().is_empty()
                        && w.chapter == (j.reference >> 8) as u8
                        && w.verse == j.reference as u8,
                    "invalid span/link"
                );
                linked.insert(w.position);
            }
        }
        ensure!(
            linked == positions,
            "translation links do not cover the verse"
        );
        let mut glossed = BTreeSet::new();
        for item in answer["glosses"].as_array().context("missing glosses")? {
            nonempty(item, "gloss")?;
            ensure!(
                glossed.insert(position(item, &positions)?),
                "duplicate gloss position"
            );
        }
        ensure!(glossed == positions, "missing gloss positions");
    }
    tx.execute("UPDATE job SET state='done',result=?,usage=?,error=NULL WHERE source=? AND ref=? AND pos=?",
        params![answer.to_string(),usage.to_string(),j.source,j.reference,j.pos])?;
    if j.pos >= 0 {
        tx.execute("UPDATE job SET state='ready' WHERE source=?1 AND ref=?2 AND pos=-1 AND state='waiting'
          AND NOT EXISTS(SELECT 1 FROM job WHERE source=?1 AND ref=?2 AND pos>=0 AND state<>'done')",params![j.source,j.reference])?;
    }
    tx.commit()?;
    Ok(())
}
fn position(item: &Value, positions: &BTreeSet<u16>) -> Result<u16> {
    let p = u16::try_from(item["position"].as_u64().context("invalid position")?)?;
    if !positions.contains(&p) {
        bail!("unknown position {p}")
    }
    Ok(p)
}
