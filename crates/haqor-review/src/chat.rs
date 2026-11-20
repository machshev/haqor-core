//! Small-context workers for an explicitly configured chat-completions server.
//! Each new job starts with exactly one prompt. A word can request one bounded
//! local article read before submitting a draft. The server gets no tools.
use crate::jobs;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

pub fn review(
    endpoint: &str,
    model: &str,
    packet: &Value,
    lexicon: &Path,
) -> Result<(Value, Value)> {
    let mut messages = vec![json!({"role":"user","content":format!("{}\n{}",
        jobs::prompt(packet),
        if packet["kind"]=="word"{GUIDE}else{""})})];
    let (mut answer, mut usage) = complete(endpoint, model, &messages)?;
    if let Some(request) = answer.get("read_article") {
        ensure!(
            packet["kind"] == "word",
            "only word agents can request an article"
        );
        let id = request
            .as_str()
            .context("read_article needs a candidate id")?;
        let candidate = packet["candidates"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|r| r["entry"] == id)
            .context("requested article absent from candidates")?;
        let path = if candidate["language"] == "grc" {
            crate::greek::path(lexicon, id)?
        } else {
            ensure!(
                id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
                "invalid article id"
            );
            crate::articles::article_path(lexicon, id, candidate["name"] == true, None)
        };
        let article = fs::read_to_string(path)?;
        ensure!(
            crate::articles::revision(&article) == candidate["revision"],
            "requested article changed"
        );
        // If the entry cannot fit, let the reviewer record uncertainty or link
        // an existing word. Truncating JSON could produce a destructive edit.
        let material = if article.len() <= 48_000 {
            article
        } else {
            "The complete article exceeds this worker's read budget. Leave the word unresolved if the entry requires amendment.".into()
        };
        messages.push(json!({"role":"assistant","content":answer.to_string()}));
        messages.push(json!({"role":"user","content":format!("Candidate article follows. Now return the final word result.\n{material}")}));
        let (final_answer, extra) = complete(endpoint, model, &messages)?;
        for k in ["input_tokens", "output_tokens"] {
            usage[k] = json!(usage[k].as_i64().unwrap_or(0) + extra[k].as_i64().unwrap_or(0));
        }
        answer = final_answer;
    }
    Ok((answer, usage))
}

const GUIDE: &str = r#"This worker can request one complete candidate article by returning {"read_article":"candidate-id"}. New Semitic root articles use id (kebab-case article filename), root, status "draft", created and updated (YYYY-MM-DD), gloss, summary and words. Each word has a pointed lemma, pos, gloss and senses. Each sense has gloss, definition and evidence such as [{"ref":"Gen.1.1","role":"defining"}]. Use lang "arc" or "syc" for Aramaic. Use the packet date for draft metadata. For a proper name use the separate Haqor name schema. If the packet cannot support a responsible decision, record it as unresolved."#;

fn complete(endpoint: &str, model: &str, messages: &[Value]) -> Result<(Value, Value)> {
    let mut command = Command::new("curl");
    command.args([
        "--disable",
        "--silent",
        "--show-error",
        "--fail-with-body",
        "--max-time",
        "600",
        "--header",
        "Content-Type: application/json",
        "--data-binary",
        "@-",
    ]);
    if std::env::var_os("HAQOR_REVIEW_API_KEY").is_some() {
        // curl reads the secret from its inherited environment. It never
        // appears in the command line, packet or local SQLite database.
        command.args([
            "--variable",
            "%HAQOR_REVIEW_API_KEY",
            "--expand-header",
            "Authorization: Bearer {{HAQOR_REVIEW_API_KEY}}",
        ]);
    }
    command.args(["--proto", "=https,http", "--url", endpoint]);
    crate::bind_worker_lifetime(&mut command);
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("start curl for packet worker")?;
    let payload =
        json!({"model":model,"messages":messages,"response_format":{"type":"json_object"}});
    let sent = child
        .stdin
        .take()
        .context("curl stdin")?
        .write_all(payload.to_string().as_bytes());
    let output = child.wait_with_output()?;
    ensure!(
        output.status.success(),
        "chat request failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    sent?;
    let response: Value = serde_json::from_slice(&output.stdout)?;
    let content = response["choices"][0]["message"]["content"]
        .as_str()
        .context("chat response lacks message content")?;
    let usage = &response["usage"];
    Ok((
        serde_json::from_str(content).map_err(crate::MalformedAnswer)?,
        json!({"input_tokens":usage["prompt_tokens"],"output_tokens":usage["completion_tokens"]}),
    ))
}
