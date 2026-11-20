//! Publish accepted results before handing more work to a reviewer.
use crate::{
    articles,
    jobs::{Catalog, Job},
    people, progress, result_store,
};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::{
    collections::{BTreeSet, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub struct Live {
    src: PathBuf,
    lexicon: PathBuf,
    data: PathBuf,
    refs: HashSet<i64>,
    _lock: fs::File,
}

fn git(args: &[&str], paths: &BTreeSet<PathBuf>) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if !paths.is_empty() {
        cmd.arg("--").args(paths);
    }
    let out = cmd.output()?;
    ensure!(
        out.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(String::from_utf8(out.stdout)?)
}

impl Live {
    pub fn open(src: &Path, lexicon: &Path, data: &Path) -> Result<Self> {
        let corpus = Connection::open_with_flags(
            data.join("haqor.db"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let refs = haqor_core::haqor_lexicon::verses_of(&corpus)?;
        // Open the same directory inode even when separate queues share a checkout.
        let lock = fs::File::open(src)?;
        lock.try_lock()
            .context("another publisher owns this corpus")?;
        Ok(Self {
            src: src.into(),
            lexicon: lexicon.into(),
            data: data.into(),
            refs,
            _lock: lock,
        })
    }
    pub fn root(&self) -> PathBuf {
        self.src.join("Haqor-Review")
    }
    fn path(&self, j: &Job) -> Result<PathBuf> {
        ensure!(
            matches!(j.source.as_str(), "ot" | "peshitta" | "greek")
                && self.refs.contains(&j.reference),
            "invalid source passage"
        );
        Ok(self
            .root()
            .join(&j.source)
            .join((j.reference >> 16).to_string())
            .join(((j.reference >> 8) & 255).to_string())
            .join(format!("{}.json", j.reference & 255)))
    }
    fn packet(&self, j: &Job) -> Result<Value> {
        let path = self.path(j)?;
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .with_context(|| format!("reading {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(
                json!({"source":j.source,"book":j.reference>>16,"chapter":(j.reference>>8)&255,"verse":j.reference&255,"words":[]}),
            ),
            Err(e) => Err(e.into()),
        }
    }
    pub fn context(&self, j: &Job, packet: &mut Value) -> Result<()> {
        let old = self.packet(j)?;
        packet["published"] = if j.pos < 0 {
            old["review"].clone()
        } else {
            old["words"]
                .as_array()
                .context("invalid tracked words")?
                .iter()
                .find(|w| w["position"] == j.pos)
                .map(|w| w["analysis"].clone())
                .unwrap_or(Value::Null)
        };
        if let Some(pending) = packet["published"]
            .pointer_mut("/entry/draft/reconciliation")
            .and_then(Value::as_object_mut)
        {
            pending.remove("article");
            pending.remove("patch");
            pending.insert("source_file".into(), json!(self.path(j)?));
        }
        Ok(())
    }
    // Journal the precise allowed write set before touching files. Only a replay of this
    // publication can claim its interrupted writes. Unrelated staged changes stay staged.
    fn begin(&self, db: &Connection, j: &Job, paths: &BTreeSet<PathBuf>) -> Result<()> {
        let previous: Option<(String, i64, i64, String)> = db
            .query_row("SELECT source,ref,pos,paths FROM live_write", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .optional()?;
        if let Some((source, r, p, _)) = previous {
            ensure!(
                (source.as_str(), r, p) == j.key(),
                "resume interrupted publication first"
            );
        } else {
            let status = git(&["status", "--porcelain", "--untracked-files=all"], paths)?;
            ensure!(
                status.is_empty(),
                "publication overlaps uncommitted work:\n{status}"
            );
            db.execute(
                "INSERT INTO live_write VALUES(?,?,?,?)",
                params![j.source, j.reference, j.pos, serde_json::to_string(paths)?],
            )?;
        }
        Ok(())
    }
    fn finish(&self, db: &Connection, j: &Job) -> Result<()> {
        let raw: String = db.query_row("SELECT paths FROM live_write", [], |r| r.get(0))?;
        let paths: BTreeSet<PathBuf> = serde_json::from_str::<BTreeSet<PathBuf>>(&raw)?
            .into_iter()
            .filter(|p| p.exists())
            .collect();
        if !paths.is_empty()
            && !git(&["status", "--porcelain", "--untracked-files=all"], &paths)?.is_empty()
        {
            git(&["add"], &paths)?;
            let message = format!("Review {}", progress::label(j));
            let out = git(&["commit", "--only", "-m", &message], &paths)?;
            eprintln!("{}", out.lines().next().unwrap_or(&message));
        }
        db.execute("DELETE FROM live_write", [])?;
        Ok(())
    }
    fn merge(&self, db: &Connection, j: &Job, answer: &Value) -> Result<()> {
        let mut out = self.packet(j)?;
        if j.pos < 0 {
            for w in out["words"]
                .as_array_mut()
                .context("invalid tracked words")?
            {
                let applied:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM article WHERE source=? AND ref=? AND pos=? AND applied=1)", params![j.source,j.reference,w["position"].as_i64()], |r|r.get(0))?;
                if applied
                    && let Some(draft) = w
                        .pointer_mut("/analysis/entry/draft")
                        .and_then(Value::as_object_mut)
                {
                    draft.remove("article");
                }
            }
            out["review"] = answer.clone();
        } else {
            let raw: String = db.query_row(
                "SELECT review_input(source,ref,packet) FROM verse WHERE source=? AND ref=?",
                params![j.source, j.reference],
                |r| r.get(0),
            )?;
            let inventory: Value = serde_json::from_str(&raw)?;
            let word = inventory["words"]
                .as_array()
                .context("invalid inventory")?
                .iter()
                .find(|w| w["position"] == j.pos)
                .context("missing word")?;
            let words = out["words"]
                .as_array_mut()
                .context("invalid tracked words")?;
            let entry = json!({"position":j.pos,"surface":word["surface"],"analysis":answer});
            if let Some(w) = words.iter_mut().find(|w| w["position"] == j.pos) {
                if *w == entry {
                    return Ok(());
                }
                *w = entry;
            } else {
                words.push(entry);
                words.sort_by_key(|w| w["position"].as_i64());
            }
            // The last completed verse remains readable until the reviewer replaces it.
        }
        let path = self.path(j)?;
        let text = serde_json::to_string_pretty(&out)? + "\n";
        if fs::read(&path).ok().as_deref() == Some(text.as_bytes()) {
            return Ok(());
        }
        fs::create_dir_all(path.parent().unwrap())?;
        let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
        fs::write(&tmp, text)?;
        fs::rename(tmp, path)?;
        Ok(())
    }
    pub fn publish(&self, db: &mut Connection, j: &Job) -> Result<bool> {
        let raw: Option<String> = db
            .query_row(
                "SELECT result FROM job WHERE source=? AND ref=? AND pos=? AND state='done'",
                j.key(),
                |r| r.get(0),
            )
            .optional()?;
        let Some(raw) = raw else {
            if db.query_row(
                "SELECT EXISTS(SELECT 1 FROM live_write WHERE source=? AND ref=? AND pos=?)",
                j.key(),
                |r| r.get::<_, bool>(0),
            )? {
                self.finish(db, j)?;
            }
            return Ok(false);
        };
        let answer: Value = serde_json::from_str(&raw)?;
        if answer.get("_published").is_some() {
            return Ok(false);
        }
        let mut prior = json!({});
        self.context(j, &mut prior)?;
        let base: Option<String> = db
            .query_row(
                "SELECT digest FROM publication_base WHERE source=? AND ref=? AND pos=?",
                j.key(),
                |r| r.get(0),
            )
            .optional()?;
        if prior["published"] != answer
            && base
                .as_deref()
                .map_or(!prior["published"].is_null(), |digest| {
                    digest != articles::revision(&prior["published"].to_string())
                })
        {
            let tx = db.transaction()?;
            tx.execute("UPDATE job SET state='ready',attempts=0,result=NULL,feedback='The tracked review changed. Reconcile the latest published result.' WHERE source=? AND ref=? AND pos=?",j.key())?;
            if j.pos >= 0 {
                tx.execute("UPDATE job SET state='waiting',result=NULL,attempts=0 WHERE source=? AND ref=? AND pos=-1",params![j.source,j.reference])?;
            }
            tx.commit()?;
            return Ok(false);
        }
        let mut paths = BTreeSet::from([self.path(j)?]);
        let greek_draft = j.source == "greek" && j.pos >= 0 && answer["entry"]["draft"].is_object();
        if greek_draft {
            paths.insert(crate::greek::path(
                &self.lexicon,
                &crate::greek::id(&answer)?,
            )?);
        }
        let article: Option<(String, bool)> = db
            .query_row(
                "SELECT id,name FROM article WHERE source=? AND ref=? AND pos=? AND applied=0",
                j.key(),
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let pending_verse_articles: Vec<(Job, String, bool)> = if j.pos < 0 {
            let mut q = db.prepare(
                "SELECT source,ref,pos,id,name FROM article WHERE source=? AND ref=? AND applied=0",
            )?;
            q.query_map(params![j.source, j.reference], |r| {
                Ok((
                    Job {
                        source: r.get(0)?,
                        reference: r.get(1)?,
                        pos: r.get(2)?,
                    },
                    r.get(3)?,
                    r.get(4)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            Vec::new()
        };
        if let Some((id, name)) = &article {
            ensure!(
                !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
                "invalid article id"
            );
            paths.insert(
                if *name {
                    self.lexicon.join("names")
                } else {
                    self.lexicon.clone()
                }
                .join(format!("{id}.json")),
            );
        }
        for (_, id, name) in &pending_verse_articles {
            paths.insert(
                if *name {
                    self.lexicon.join("names")
                } else {
                    self.lexicon.clone()
                }
                .join(format!("{id}.json")),
            );
        }
        if j.pos < 0 {
            paths.extend(people::publication_paths(
                db,
                &self.src,
                &self.lexicon,
                &j.source,
                j.reference,
            )?);
        }
        self.begin(db, j, &paths)?;
        if article.is_some() {
            match articles::apply_one(db, &self.lexicon, &self.refs, j)? {
                articles::PublishOutcome::Stale | articles::PublishOutcome::Rejected => {
                    self.finish(db, j)?;
                    return Ok(true);
                }
                _ => (),
            }
        }
        if j.pos < 0 {
            people::publish_one_with_refs(
                db,
                &self.src,
                &self.data,
                &self.lexicon,
                &j.source,
                j.reference,
                &self.refs,
            )?;
            // Name drafts may have been waiting for this verse's bearer
            // decision. Apply them after people publication has merged the
            // bearer keys into their staged articles.
            for (word, _, _) in &pending_verse_articles {
                match articles::apply_one(db, &self.lexicon, &self.refs, word)? {
                    articles::PublishOutcome::Stale | articles::PublishOutcome::Rejected => {
                        self.finish(db, j)?;
                        return Ok(true);
                    }
                    articles::PublishOutcome::Deferred => {
                        let tx = db.transaction()?;
                        tx.execute("UPDATE job SET state='ready',result=NULL,feedback='The verse could not identify a bearer for this proposed personal-name article. Reconcile the lexical assignment.' WHERE source=? AND ref=? AND pos=?",word.key())?;
                        tx.execute("UPDATE job SET state='waiting',result=NULL WHERE source=? AND ref=? AND pos=?",j.key())?;
                        tx.commit()?;
                        self.finish(db, j)?;
                        return Ok(true);
                    }
                    articles::PublishOutcome::Applied | articles::PublishOutcome::NotPending => {}
                }
            }
        }
        let greek_changed =
            greek_draft && crate::greek::publish(&self.lexicon, &answer, j.reference, j.pos)?;
        self.merge(db, j, &answer)?;
        self.finish(db, j)?;
        result_store::compact(db, j, &self.root())?;
        if j.pos < 0 {
            for (word, _, _) in pending_verse_articles {
                result_store::compact(db, &word, &self.root())?;
            }
        }
        db.execute(
            "UPDATE article SET body='' WHERE source=? AND ref=? AND (?=-1 OR pos=?) AND applied=1",
            params![j.source, j.reference, j.pos, j.pos],
        )?;
        Ok(greek_changed || article.is_some() || j.pos < 0)
    }
    pub fn recover(&self, db: &mut Connection) -> Result<()> {
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS live_write(source TEXT,ref INTEGER,pos INTEGER,paths TEXT);CREATE TABLE IF NOT EXISTS publication_base(source TEXT,ref INTEGER,pos INTEGER,digest TEXT,PRIMARY KEY(source,ref,pos)) WITHOUT ROWID",
        )?;
        let mut q=db.prepare("SELECT source,ref,pos FROM job WHERE state='done' AND json_type(result,'$._published') IS NULL ORDER BY 2,1,3 DESC")?;
        let mut jobs = q
            .query_map([], |r| {
                Ok(Job {
                    source: r.get(0)?,
                    reference: r.get(1)?,
                    pos: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(q);
        if let Some(j) = db
            .query_row("SELECT source,ref,pos FROM live_write", [], |r| {
                Ok(Job {
                    source: r.get(0)?,
                    reference: r.get(1)?,
                    pos: r.get(2)?,
                })
            })
            .optional()?
        {
            jobs.insert(0, j);
        }
        if !jobs.is_empty() {
            eprintln!("Recovering {} pending source publications…", jobs.len());
        }
        for j in jobs {
            self.publish(db, &j)
                .with_context(|| format!("publishing {}", progress::label(&j)))?;
        }
        Ok(())
    }
    pub fn catalog(&self) -> Result<Catalog> {
        let mut c = Catalog::load(&self.lexicon)?;
        c.people = Some(people::People::load(&self.src, &self.data)?);
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn out_of_order_words_share_one_tracked_verse() -> Result<()> {
        let src = std::env::temp_dir().join(format!("haqor-live-{}", std::process::id()));
        fs::create_dir_all(&src)?;
        let db = Connection::open_in_memory()?;
        crate::inventory::register(&db, &src)?;
        db.execute_batch(crate::jobs::SCHEMA)?;
        let reference = 0x280102;
        let inventory =
            json!({"words":[{"position":0,"surface":"A"},{"position":1,"surface":"B"}]});
        db.execute(
            "INSERT INTO verse VALUES('peshitta',?,?)",
            params![reference, inventory.to_string()],
        )?;
        let live = Live {
            src: src.clone(),
            lexicon: src.clone(),
            data: src.clone(),
            refs: HashSet::from([reference]),
            _lock: fs::File::open(&src)?,
        };
        let mut job = Job {
            source: "peshitta".into(),
            reference,
            pos: 1,
        };
        live.merge(&db, &job, &json!({"lemma":"B"}))?;
        let mut packet = json!({});
        live.context(&job, &mut packet)?;
        assert_eq!(packet["published"]["lemma"], "B");
        job.pos = 0;
        live.merge(&db, &job, &json!({"lemma":"A"}))?;
        job.pos = -1;
        live.merge(&db, &job, &json!({"translation":"[A|0] [B|1]"}))?;
        let edition = live.packet(&job)?;
        assert_eq!(edition["words"][0]["analysis"]["lemma"], "A");
        assert_eq!(edition["words"][1]["analysis"]["lemma"], "B");
        assert_eq!(edition["review"]["translation"], "[A|0] [B|1]");
        fs::remove_dir_all(src)?;
        Ok(())
    }
}
