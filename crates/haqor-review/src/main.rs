//! One Rust process coordinates independent, read-only word and verse agents.
mod articles;
mod chat;
mod greek;
mod inventory;
mod jobs;
mod lexemes;
mod live;
mod morph;
mod people;
mod progress;
mod quota;
mod result_store;
use anyhow::{Context, Result, bail, ensure};
use clap::{Parser, Subcommand, ValueEnum};
use jobs::Catalog;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Source {
    Ot,
    Peshitta,
    Greek,
}
impl Source {
    fn name(self) -> &'static str {
        match self {
            Self::Ot => "ot",
            Self::Peshitta => "peshitta",
            Self::Greek => "greek",
        }
    }
}
#[derive(Parser)]
struct Cli {
    /// Persistent local queue. Use a separate file for an independent run.
    #[arg(long, global = true, default_value = ".haqor-review/progress.sqlite")]
    state: PathBuf,
    #[command(subcommand)]
    command: Action,
}
#[derive(Subcommand)]
enum Action {
    /// Snapshot all matching verses. Reseeding changed verses invalidates their results.
    Seed {
        #[arg(long, default_value = "data")]
        data: PathBuf,
        #[arg(long, value_enum)]
        source: Option<Source>,
        #[arg(long,value_parser=clap::value_parser!(u8).range(1..=66))]
        book: Option<u8>,
        #[arg(long, requires = "book")]
        chapter: Option<u8>,
        #[arg(long, requires = "chapter")]
        verse: Option<u8>,
    },
    /// Dispatch fresh agents up to a shared concurrency limit.
    Run {
        #[arg(long, default_value_t = 4)]
        max_agents: usize,
        /// Stop automatically retrying a job after this many attempts.
        #[arg(long, default_value_t = 3)]
        max_attempts: u32,
        /// Optional executable taking one prompt on stdin and returning one JSON result on stdout.
        #[arg(long)]
        worker: Option<PathBuf>,
        /// Full URL of a chat-completions endpoint for packet-only agents.
        #[arg(long, requires = "model", conflicts_with = "worker")]
        chat_endpoint: Option<String>,
        #[arg(long, conflicts_with = "worker")]
        model: Option<String>,
        /// Model for verse reviews; defaults to --model.
        #[arg(long, conflicts_with = "worker")]
        verse_model: Option<String>,
        /// Codex reasoning effort for word jobs (e.g. low, medium, high, xhigh);
        /// defaults to the Codex configuration.
        #[arg(long, conflicts_with_all = ["worker", "chat_endpoint"])]
        effort: Option<String>,
        /// Codex reasoning effort for verse reviews; defaults to --effort.
        #[arg(long, conflicts_with_all = ["worker", "chat_endpoint"])]
        verse_effort: Option<String>,
        #[arg(long, default_value = "data/lexicon")]
        lexicon: PathBuf,
        #[arg(long, default_value = "data")]
        data: PathBuf,
        #[arg(long, default_value = "src_texts")]
        src_texts: PathBuf,
    },
    /// Apply verse-reviewed Semitic draft articles through Haqor's lexicon validator.
    ApplyArticles {
        #[arg(long, default_value = "data/lexicon")]
        lexicon: PathBuf,
        #[arg(long, default_value = "data")]
        data: PathBuf,
    },
    /// Publish reviewed occurrences and new people or places into source data.
    ApplyNames {
        #[arg(long, default_value = "data")]
        data: PathBuf,
        #[arg(long, default_value = "src_texts")]
        src_texts: PathBuf,
        #[arg(long, default_value = "data/lexicon")]
        lexicon: PathBuf,
    },
    /// Recover accepted results into version-controlled sources without starting workers.
    Publish {
        #[arg(long, default_value = "data/lexicon")]
        lexicon: PathBuf,
        #[arg(long, default_value = "data")]
        data: PathBuf,
        #[arg(long, default_value = "src_texts")]
        src_texts: PathBuf,
    },
    Status,
    /// Make failed jobs eligible for a fresh set of attempts.
    Retry {
        /// Also revisit unresolved occurrences with this additional evidence.
        #[arg(long)]
        evidence: Option<String>,
    },
    /// Revisit every occurrence of this exact lexical key and its verse review.
    Reassign {
        #[arg(long)]
        language: String,
        #[arg(long)]
        lemma: String,
        #[arg(long, default_value = "")]
        qualifier: String,
        #[arg(long)]
        reason: String,
        /// Reject this obsolete spelling in subsequent word results.
        #[arg(long)]
        retire: bool,
    },
    /// Record the lexeme index on occurrences reviewed before it was required;
    /// reopen those whose lexeme cannot be decided from the article.
    LinkLexemes {
        #[arg(long, default_value = "data/lexicon")]
        lexicon: PathBuf,
    },
    /// Record the sense id on occurrences reviewed before it was required, where
    /// the lexeme has one sense or the prose sense names one.
    LinkSenses {
        #[arg(long, default_value = "data/lexicon")]
        lexicon: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Record the machine-readable morphology on occurrences reviewed before it
    /// was required, from agreeing OSHB tags; reopen the rest.
    FillMorph,
    /// Write reviewed verse packets including lexical drafts and contextual glosses to stdout as JSONL.
    Export,
}

fn writer_lock(path: &Path) -> Result<File> {
    let mut sidecar = path.as_os_str().to_owned();
    sidecar.push(".lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(sidecar)?;
    file.try_lock().context("another harness owns this queue")?;
    Ok(file)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let writing = !matches!(cli.command, Action::Status | Action::Export);
    if matches!(cli.command, Action::Seed { .. }) {
        if let Some(p) = cli.state.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(p)?;
        }
    } else {
        ensure!(cli.state.is_file(), "seed the review queue first");
    }
    let _lock = writing.then(|| writer_lock(&cli.state)).transpose()?;
    let mut db = if writing {
        Connection::open(&cli.state)?
    } else {
        Connection::open_with_flags(&cli.state, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?
    };
    db.busy_timeout(std::time::Duration::from_secs(5))?;
    if writing {
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        db.execute_batch(jobs::SCHEMA)?;
        db.execute_batch(people::SCHEMA)?;
        // The exclusive OS lock disappears on process exit. Only this coordinator writes results.
        db.execute(
            "UPDATE job SET state='ready', attempts=max(0,attempts-1) WHERE state='running'",
            [],
        )?;
    }
    let source_dir = match &cli.command {
        Action::Run { src_texts, .. }
        | Action::Publish { src_texts, .. }
        | Action::ApplyNames { src_texts, .. } => Some(src_texts.clone()),
        _ => None,
    };
    if writing {
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS review_config(key TEXT PRIMARY KEY,value TEXT)",
        )?;
        if let Some(src) = &source_dir {
            db.execute(
                "INSERT OR REPLACE INTO review_config VALUES('sources',?)",
                [src.canonicalize()?
                    .to_str()
                    .context("invalid source path")?],
            )?;
        }
    }
    let configured: Option<String> = db
        .query_row(
            "SELECT value FROM review_config WHERE key='sources'",
            [],
            |r| r.get(0),
        )
        .ok();
    let src = source_dir
        .or_else(|| configured.map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("src_texts"));
    result_store::register(&db, &src.join("Haqor-Review"))?;
    let explicit_data = match &cli.command {
        Action::Seed { data, .. }
        | Action::Publish { data, .. }
        | Action::Run { data, .. }
        | Action::ApplyNames { data, .. } => Some(data),
        _ => None,
    };
    if writing && let Some(data) = explicit_data {
        db.execute(
            "INSERT OR REPLACE INTO review_config VALUES('data',?)",
            [data.to_str().context("invalid data path")?],
        )?;
    }
    let data: String = db
        .query_row(
            "SELECT value FROM review_config WHERE key='data'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "data".into());
    inventory::register(&db, Path::new(&data))?;
    if matches!(
        cli.command,
        Action::Seed { .. } | Action::Publish { .. } | Action::Run { .. }
    ) {
        inventory::compact(&db)?;
    }
    match cli.command {
        Action::Seed {
            data,
            source,
            book,
            chapter,
            verse,
        } => {
            let low = (i64::from(book.unwrap_or(0)) << 16)
                + (i64::from(chapter.unwrap_or(0)) << 8)
                + i64::from(verse.unwrap_or(0));
            let high = (i64::from(book.unwrap_or(66)) << 16)
                + (i64::from(chapter.unwrap_or(255)) << 8)
                + i64::from(verse.unwrap_or(255));
            let sources = source
                .map(|s| vec![s])
                .unwrap_or_else(|| vec![Source::Ot, Source::Peshitta, Source::Greek]);
            inventory::seed(&mut db, &data, &sources, low, high)
        }
        Action::Run {
            max_agents,
            max_attempts,
            worker,
            chat_endpoint,
            data,
            src_texts,
            model,
            verse_model,
            effort,
            verse_effort,
            lexicon,
        } => {
            ensure!(
                max_agents > 0 && max_attempts > 0,
                "limits must be positive"
            );
            eprintln!("Preparing review queue {}…", cli.state.display());
            let _quota = (worker.is_none() && chat_endpoint.is_none()).then(quota::monitor);
            let live = live::Live::open(&src_texts, &lexicon, &data)?;
            live.recover(&mut db)?;
            let mut catalog = live.catalog()?;
            catalog.people.as_ref().unwrap().refresh(&mut db)?;
            articles::refresh_dependencies(&mut db, &catalog.revisions)?;
            jobs::backfill_carry_keys(&mut db)?;
            schedule(
                &mut db,
                &mut catalog,
                Some(&live),
                max_agents,
                max_attempts,
                &|packet| {
                    let verse = packet["kind"] == "verse";
                    let model = if verse {
                        verse_model.as_deref().or(model.as_deref())
                    } else {
                        model.as_deref()
                    };
                    let effort = if verse {
                        verse_effort.as_deref().or(effort.as_deref())
                    } else {
                        effort.as_deref()
                    };
                    if let Some(endpoint) = &chat_endpoint {
                        return chat::review(
                            endpoint,
                            model.context("missing model")?,
                            packet,
                            &lexicon,
                        );
                    }
                    invoke(worker.as_deref(), model, effort, &jobs::prompt(packet))
                },
            )
        }
        Action::Retry { evidence } => {
            if let Some(evidence) = evidence {
                ensure!(
                    !evidence.trim().is_empty(),
                    "supply evidence for unresolved review jobs"
                );
                println!("Reopened {} unresolved review jobs.",db.execute("UPDATE job SET state='ready',attempts=0,feedback=feedback||char(10)||? WHERE state='unresolved'",[evidence])?);
            }
            println!(
                "Reset {} failed jobs.",
                db.execute(
                    "UPDATE job SET state='ready',attempts=0 WHERE state='failed'",
                    []
                )?
            );
            Ok(())
        }
        Action::Publish {
            lexicon,
            data,
            src_texts,
        } => live::Live::open(&src_texts, &lexicon, &data)?.recover(&mut db),
        Action::Status => {
            let mut q=db.prepare("SELECT source,iif(pos=-1,'verse','word'),state,count(*) FROM job GROUP BY 1,2,3 ORDER BY 1,2,3")?;
            let mut rows = q.query([])?;
            while let Some(r) = rows.next()? {
                println!(
                    "{} {} {} {}",
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?
                );
            }
            let (attempts,input,output):(i64,i64,i64)=db.query_row("SELECT count(*),coalesce(sum(json_extract(usage,'$.input_tokens')),0),coalesce(sum(json_extract(usage,'$.output_tokens')),0) FROM attempt",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
            println!(
                "Finished attempts {attempts}, reported input tokens {input}, output tokens {output}"
            );
            Ok(())
        }
        Action::Reassign {
            language,
            lemma,
            qualifier,
            reason,
            retire,
        } => {
            use unicode_normalization::UnicodeNormalization;
            ensure!(
                !reason.trim().is_empty(),
                "supply a reason for reassignment"
            );
            let tx = db.transaction()?;
            if retire {
                tx.execute(
                    "INSERT OR REPLACE INTO retired VALUES(?,?,?,?)",
                    params![
                        language,
                        lemma.nfc().collect::<String>(),
                        qualifier.nfc().collect::<String>(),
                        reason
                    ],
                )?;
            }
            tx.execute("CREATE TEMP TABLE affected AS SELECT source,ref,pos FROM job WHERE pos>=0
                AND json_extract(result,'$.language')=? AND json_extract(result,'$.lemma')=? AND json_extract(result,'$.qualifier')=?",
                params![language,lemma.nfc().collect::<String>(),qualifier.nfc().collect::<String>()])?;
            tx.execute("UPDATE job SET state='waiting',result=NULL,attempts=0 WHERE pos=-1 AND (source,ref) IN (SELECT source,ref FROM affected)",[])?;
            let count=tx.execute("UPDATE job SET state='ready', result=NULL,attempts=0, feedback=? WHERE (source,ref,pos) IN (SELECT * FROM affected)",[reason])?;
            tx.commit()?;
            println!("Queued {count} occurrences for reassignment.");
            Ok(())
        }
        Action::ApplyArticles { lexicon, data } => {
            let result = articles::apply(&mut db, &lexicon, &data);
            articles::refresh_dependencies(&mut db, &Catalog::load(&lexicon)?.revisions)?;
            result
        }
        Action::ApplyNames {
            data,
            src_texts,
            lexicon,
        } => people::publish(&db, &src_texts, &data, &lexicon),
        Action::Export => export(&db),
        Action::LinkSenses { lexicon, dry_run } => {
            lexemes::link_senses(&src.join("Haqor-Review"), &lexicon, dry_run)
        }
        Action::FillMorph => morph::fill(&mut db, &src.join("Haqor-Review"), Path::new(&data)),
        Action::LinkLexemes { lexicon } => {
            lexemes::link(&mut db, &src.join("Haqor-Review"), &lexicon)
        }
    }
}

fn export(db: &Connection) -> Result<()> {
    let mut q=db.prepare("SELECT source,ref,review_result(result),review_input(verse.source,verse.ref,packet) FROM job JOIN verse USING(source,ref) WHERE pos=-1 AND state='done' ORDER BY ref,source")?;
    let mut rows = q.query([])?;
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    while let Some(r) = rows.next()? {
        let source: String = r.get(0)?;
        let reference: i64 = r.get(1)?;
        let packet: Value = serde_json::from_str(&r.get::<_, String>(3)?)?;
        let mut stmt = db.prepare_cached(
            "SELECT pos,review_result(result) FROM job WHERE source=? AND ref=? AND pos>=0 ORDER BY pos",
        )?;
        let mut words = Vec::new();
        let mut results = stmt.query(params![source, reference])?;
        while let Some(row) = results.next()? {
            let pos: i64 = row.get(0)?;
            words.push(json!({"position":pos,"surface":packet["words"][words.len()]["surface"],"analysis":serde_json::from_str::<Value>(&row.get::<_,String>(1)?)?}));
        }
        writeln!(
            stdout,
            "{}",
            json!({"source":source,"book":reference>>16,"chapter":(reference>>8)&255,"verse":reference&255,
            "words":words,"review":serde_json::from_str::<Value>(&r.get::<_,String>(2)?)?})
        )?;
    }
    stdout.flush()?;
    Ok(())
}

fn schedule(
    db: &mut Connection,
    catalog: &mut Catalog,
    live: Option<&live::Live>,
    limit: usize,
    max_attempts: u32,
    worker: &(impl Fn(&Value) -> Result<(Value, Value)> + Sync),
) -> Result<()> {
    let (tx, rx) = mpsc::channel();
    let (done, total) = db.query_row(
        "SELECT count(*) FILTER (WHERE state='done'), count(*) FROM job WHERE pos=-1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let mut progress = progress::Progress {
        active: Default::default(),
        done,
        total,
        limit,
    };
    progress.log("Starting review");
    let mut worker_failure = None;
    thread::scope(|scope| -> Result<()> {
        loop {
            while progress.active.len() < limit && worker_failure.is_none() {
                let Some(job) = jobs::next(db)? else { break };
                let attempts: u32 = db.query_row(
                    "SELECT attempts FROM job WHERE source=? AND ref=? AND pos=?",
                    job.key(),
                    |r| r.get(0),
                )?;
                if attempts >= max_attempts {
                    db.execute("UPDATE job SET state='failed', error=coalesce(error,'review attempt limit reached') WHERE source=? AND ref=? AND pos=?",job.key())?;
                    progress.log(&format!(
                        "Failed {} (attempt limit reached)",
                        progress::label(&job)
                    ));
                    continue;
                }
                let mut packet = jobs::packet(db, &job, catalog)?;
                if let Some(live) = live {
                    live.context(&job, &mut packet)?;
                    db.execute(
                        "INSERT OR REPLACE INTO publication_base VALUES(?,?,?,?)",
                        params![
                            job.source,
                            job.reference,
                            job.pos,
                            articles::revision(&packet["published"].to_string())
                        ],
                    )?;
                }
                db.execute("UPDATE job SET state='running',attempts=attempts+1 WHERE source=? AND ref=? AND pos=?",job.key())?;
                db.execute("INSERT INTO review_inflight VALUES(?,?,?)", job.key())?;
                let tx = tx.clone();
                let label = progress::label(&job);
                progress
                    .active
                    .insert(label.clone(), std::time::Instant::now());
                progress.log(&format!(
                    "Started {label} (attempt {}/{max_attempts})",
                    attempts + 1
                ));
                scope.spawn(move || {
                    let start = std::time::Instant::now();
                    // A word reviewed identically before needs no agent (jobs::carry).
                    let result = match packet.get("carry") {
                        Some(answer) => Ok((answer.clone(), json!({"carried": true}))),
                        None => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            worker(&packet)
                        }))
                        .unwrap_or_else(|_| Err(anyhow::anyhow!("review worker panicked"))),
                    };
                    let _ = tx.send((job, packet, result, start.elapsed()));
                });
            }
            if progress.active.is_empty() {
                break;
            }
            let (job, packet, result, elapsed) =
                match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(result) => result,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        progress.heartbeat();
                        continue;
                    }
                    Err(e) => return Err(e.into()),
                };
            let label = progress::label(&job);
            progress.active.remove(&label);
            db.execute(
                "DELETE FROM review_inflight WHERE source=? AND ref=? AND pos=?",
                job.key(),
            )?;
            if let Err(error) = &result {
                worker_failure = Some(format!("{error:#}"));
            }
            let usage = result.as_ref().ok().map(|(_, u)| u.to_string());
            if let Some(live) = live
                && let Ok((answer, _)) = &result
            {
                let running: bool = db.query_row(
                    "SELECT state='running' FROM job WHERE source=? AND ref=? AND pos=?",
                    job.key(),
                    |r| r.get(0),
                )?;
                if !running {
                    progress.log(&format!("Discarded superseded {label}"));
                    continue;
                }
                let mut current = jobs::packet(db, &job, catalog)?;
                live.context(&job, &mut current)?;
                let unseen_greek = job.source == "greek"
                    && answer["entry"]["draft"].is_object()
                    && crate::greek::id(answer).ok().is_some_and(|id| {
                        catalog.revisions.contains_key(&(id.clone(), false))
                            && !packet["candidates"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .any(|c| c["entry"] == id)
                    });
                if unseen_greek {
                    db.execute(
                        "INSERT OR REPLACE INTO corrected_headword VALUES(?,?,?,?)",
                        params![job.source, job.reference, job.pos, answer["lemma"].as_str()],
                    )?;
                }
                let changed = unseen_greek
                    || ["candidates", "words", "people_and_places", "published"]
                        .iter()
                        .any(|k| current[*k] != packet[*k]);
                if changed {
                    db.execute("UPDATE job SET state='ready',attempts=max(0,attempts-1),feedback='Published source changed while reviewing. Reconcile the current packet.' WHERE source=? AND ref=? AND pos=?",job.key())?;
                    progress.log(&format!("Requeued {label} after a source change"));
                    continue;
                }
            }
            let outcome =
                result.and_then(|(answer, usage)| jobs::accept(db, &job, answer, &usage, &packet));
            db.execute(
                "INSERT INTO attempt VALUES(?,?,?,?,?,?)",
                params![
                    job.source,
                    job.reference,
                    job.pos,
                    elapsed.as_millis() as i64,
                    usage,
                    outcome.as_ref().err().map(|e| format!("{e:#}"))
                ],
            )?;
            if let Err(e) = &outcome {
                db.execute("UPDATE job SET state=iif(attempts>=?,'failed','ready'), error=? WHERE source=? AND ref=? AND pos=?",
                    params![max_attempts,format!("{e:#}"),job.source,job.reference,job.pos])?;
            }
            if outcome.is_ok()
                && let Some(live) = live
                && live
                    .publish(db, &job)
                    .with_context(|| format!("publishing {label}"))?
            {
                let next = live.catalog()?;
                if catalog.revisions != next.revisions {
                    articles::refresh_dependencies(db, &next.revisions)?;
                    progress.done = db.query_row(
                        "SELECT count(*) FROM job WHERE state='done' AND pos=-1",
                        [],
                        |r| r.get(0),
                    )?;
                    // The current verse is counted just below.
                    if job.pos < 0
                        && db.query_row(
                            "SELECT state='done' FROM job WHERE source=? AND ref=? AND pos=?",
                            job.key(),
                            |r| r.get::<_, bool>(0),
                        )?
                    {
                        progress.done -= 1;
                    }
                }
                *catalog = next;
            }
            let state: String = db.query_row(
                "SELECT state FROM job WHERE source=? AND ref=? AND pos=?",
                job.key(),
                |r| r.get(0),
            )?;
            progress.done += i64::from(job.pos < 0 && state == "done");
            let detail = outcome
                .err()
                .map(|e| format!(": {e:#}"))
                .unwrap_or_default();
            progress.log(&format!(
                "{label} → {state} ({}s){detail}",
                elapsed.as_secs()
            ));
        }
        Ok(())
    })?;
    progress.log("Dispatch finished");
    if let Some(reason) = worker_failure {
        bail!("worker stopped: {reason}. Remaining jobs are checkpointed.");
    }
    let failed: i64 = db.query_row("SELECT count(*) FROM job WHERE state='failed'", [], |r| {
        r.get(0)
    })?;
    ensure!(
        failed == 0,
        "{failed} failed jobs; inspect errors in SQLite, then use retry"
    );
    let unresolved: i64 = db.query_row(
        "SELECT count(*) FROM job WHERE state='unresolved'",
        [],
        |r| r.get(0),
    )?;
    ensure!(
        unresolved == 0,
        "{unresolved} unresolved review jobs await evidence"
    );
    Ok(())
}

fn bind_worker_lifetime(cmd: &mut Command) {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        let parent = std::process::id();
        // SAFETY: only async-signal-safe system calls run between fork and
        // exec. Check getppid as well to close the fork/prctl race.
        unsafe {
            cmd.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) < 0 {
                    return Err(io::Error::last_os_error());
                }
                if libc::getppid() as u32 != parent {
                    libc::_exit(1);
                }
                Ok(())
            });
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = cmd;
}

fn invoke(
    worker: Option<&Path>,
    model: Option<&str>,
    effort: Option<&str>,
    prompt: &str,
) -> Result<(Value, Value)> {
    let mut cmd = Command::new(worker.unwrap_or(Path::new("codex")));
    if worker.is_none() {
        cmd.args(["exec", "--ephemeral", "--sandbox", "read-only", "--json"]);
        for feature in ["apps", "plugins", "memories", "multi_agent"] {
            cmd.args(["--disable", feature]);
        }
        cmd.args(["--enable", "omit_skill_descriptions"]);
        if let Some(m) = model {
            cmd.args(["--model", m]);
        }
        if let Some(e) = effort {
            cmd.args(["-c", &format!("model_reasoning_effort=\"{e}\"")]);
        }
        cmd.arg("-");
    }
    bind_worker_lifetime(&mut cmd);
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("start review worker")?;
    let sent = child
        .stdin
        .take()
        .context("worker stdin")?
        .write_all(prompt.as_bytes());
    let output = child.wait_with_output()?;
    ensure!(
        output.status.success(),
        "worker {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
            .chars()
            .take(2000)
            .collect::<String>()
    );
    sent?;
    if worker.is_some() {
        return Ok((serde_json::from_slice(&output.stdout)?, Value::Null));
    }
    let mut last = None;
    let mut usage = Value::Null;
    for line in std::str::from_utf8(&output.stdout)?.lines() {
        let event: Value = serde_json::from_str(line)?;
        match event["type"].as_str() {
            Some("item.completed") if event["item"]["type"] == "agent_message" => {
                last = event["item"]["text"].as_str().map(str::to_owned)
            }
            Some("turn.completed") => usage = event["usage"].clone(),
            Some("turn.failed") => bail!("Codex turn failed: {}", event["error"]),
            _ => {}
        }
    }
    Ok((
        serde_json::from_str(&last.context("Codex returned no final answer")?)?,
        usage,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Duration;

    fn fixture(counts: &[usize]) -> Connection {
        let db = Connection::open_in_memory().unwrap();
        inventory::register(&db, Path::new(".")).unwrap();
        result_store::register(&db, Path::new(".")).unwrap();
        db.execute_batch(jobs::SCHEMA).unwrap();
        for (i, count) in counts.iter().enumerate() {
            let reference = (43 << 16) + (11 << 8) + 1 + i as i64;
            let words = (0..*count)
                .map(|p| json!({"position":p,"surface":"λόγος","source_analysis":{}}))
                .collect::<Vec<_>>();
            db.execute(
                "INSERT INTO verse VALUES('greek',?,?)",
                params![reference, json!({"words":words,"english":[]}).to_string()],
            )
            .unwrap();
            db.execute(
                "INSERT INTO job(source,ref,pos,state) VALUES('greek',?,-1,'waiting')",
                [reference],
            )
            .unwrap();
            for p in 0..*count {
                db.execute(
                    "INSERT INTO job(source,ref,pos,state) VALUES('greek',?,?,'ready')",
                    params![reference, p as i64],
                )
                .unwrap();
            }
        }
        db
    }
    fn word() -> Value {
        json!({"language":"grc","lemma":"λόγος","root":"","qualifier":"","morphology":"noun",
            "morph":"N-NSM","segments":"","sense":"word","gloss":"word","evidence":"fixture occurrence","entry":{"draft":{"definition":"word","change":"new","evidence":"fixture"},"word":0}})
    }
    fn verse(p: &Value) -> Value {
        let words = p["words"].as_array().unwrap();
        let spans = words
            .iter()
            .map(|w| format!("[word|{}]", w["position"]))
            .collect::<Vec<_>>()
            .join(" ");
        json!({"entities":[],"translation":spans,"glosses":words.iter().map(|w|json!({"position":w["position"],"gloss":"word"})).collect::<Vec<_>>()})
    }
    #[test]
    fn later_words_overlap_ready_verse_with_one_shared_limit() {
        let mut db = fixture(&[1, 3, 2]);
        let active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let gate = (Mutex::new(false), Condvar::new());
        schedule(&mut db, &mut Catalog::default(), None, 2, 1, &|p| {
            let n = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(n, Ordering::SeqCst);
            let answer = if p["kind"] == "word" {
                if p["verse"] == 2 && p["position"] == 0 {
                    let started = gate.0.lock().unwrap();
                    let (started, _) = gate
                        .1
                        .wait_timeout_while(started, Duration::from_secs(3), |b| !*b)
                        .unwrap();
                    ensure!(*started, "later word could not overlap a ready verse");
                }
                word()
            } else {
                if p["verse"] == 1 {
                    *gate.0.lock().unwrap() = true;
                    gate.1.notify_all();
                }
                let expected = if p["verse"] == 1 {
                    1
                } else if p["verse"] == 2 {
                    3
                } else {
                    2
                };
                assert_eq!(p["words"].as_array().unwrap().len(), expected);
                verse(p)
            };
            active.fetch_sub(1, Ordering::SeqCst);
            Ok((answer, Value::Null))
        })
        .unwrap();
        assert_eq!(peak.load(Ordering::SeqCst), 2);
        assert_eq!(
            db.query_row("SELECT count(*) FROM job WHERE state='done'", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            9
        );
        schedule(&mut db, &mut Catalog::default(), None, 2, 1, &|_| {
            bail!("completed work redispatched")
        })
        .unwrap();
    }
    #[test]
    fn failed_word_blocks_only_its_verse_and_rework_targets_one_position() {
        let mut db = fixture(&[2, 1]);
        let result = schedule(&mut db, &mut Catalog::default(), None, 2, 1, &|p| {
            let answer = if p["kind"] == "word" {
                if p["verse"] == 1 && p["position"] == 1 {
                    json!({})
                } else {
                    word()
                }
            } else {
                assert_eq!(p["verse"], 2);
                verse(p)
            };
            Ok((answer, Value::Null))
        });
        assert!(result.is_err());
        db.execute(
            "UPDATE job SET state='ready',attempts=0 WHERE state='failed'",
            [],
        )
        .unwrap();
        schedule(&mut db, &mut Catalog::default(), None, 2, 3, &|p| {
            assert_eq!(p["verse"], 1);
            let answer = if p["kind"] == "word" {
                if p["position"] == 0 {
                    assert_eq!(p["feedback"], "check morphology");
                }
                word()
            } else if p["feedback"].is_null() {
                json!({"rework":[{"position":0,"reason":"check morphology"}]})
            } else {
                verse(p)
            };
            Ok((answer, Value::Null))
        })
        .unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM job WHERE state='done'", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            5
        );
    }

    #[test]
    fn changed_lexicon_article_reopens_only_dependent_words_and_verses() {
        let mut db = fixture(&[2, 1]);
        db.execute("UPDATE job SET state='done'", []).unwrap();
        db.execute("INSERT INTO dependency SELECT source,ref,pos,'example',0,'old' FROM job WHERE ref=(SELECT min(ref) FROM verse) AND pos=1",[]).unwrap();
        articles::refresh_dependencies(&mut db, &Default::default()).unwrap();
        let jobs: Vec<(i64, String)> = db
            .prepare("SELECT pos,state FROM job ORDER BY ref,pos")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            jobs,
            [
                (-1, "waiting".into()),
                (0, "done".into()),
                (1, "ready".into()),
                (-1, "done".into()),
                (0, "done".into())
            ]
        );
    }
    #[test]
    fn retired_keys_fail_and_unresolved_words_keep_the_verse_waiting() {
        let mut db = fixture(&[1]);
        db.execute(
            "INSERT INTO retired VALUES('grc','λόγος','','correct the fixture spelling')",
            [],
        )
        .unwrap();
        let error = schedule(&mut db, &mut Catalog::default(), None, 1, 1, &|_| {
            Ok((word(), Value::Null))
        });
        assert!(error.is_err());
        let message: String = db
            .query_row("SELECT error FROM job WHERE pos=0", [], |r| r.get(0))
            .unwrap();
        assert!(message.contains("retired lexical key"));
        db.execute(
            "UPDATE job SET state='ready',attempts=0 WHERE state='failed'",
            [],
        )
        .unwrap();
        assert!(
            schedule(&mut db, &mut Catalog::default(), None, 1, 1, &|p| {
                assert_eq!(p["kind"], "word");
                Ok((
                    json!({"unresolved":"Two possible analyses need further evidence."}),
                    Value::Null,
                ))
            })
            .is_err()
        );
        let state: String = db
            .query_row("SELECT state FROM job WHERE pos=-1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(state, "waiting");
    }
}
