//! Minimal local HTTP server for editing `data/lexicon_overrides.json`.

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use rusqlite::Connection;
use serde_json::{Value, json};

const EDITOR: &str = include_str!("editor.html");
const MAX_REQUEST_BYTES: usize = 4 * 1024 * 1024;
const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;
const APP_ID: &str = "org.haqor";
const LEGACY_APP_ID: &str = "com.example.haqor";

mod issue_tui;

/// The server credentials the Flutter app keeps in its platform preferences.
/// They are read only for a `pull` invocation and are never printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSyncSettings {
    pub server_url: String,
    pub token: String,
}

fn xdg_data_home() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("XDG_DATA_HOME").filter(|path| !path.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(".local/share"))
}

fn shared_preferences_path(data_home: &Path, app_id: &str) -> PathBuf {
    data_home.join(app_id).join("shared_preferences.json")
}

/// Read the settings from Haqor's XDG data directory. During the application
/// ID migration, fall back to Flutter's former template ID only when the new
/// settings file does not yet exist, so existing LAN sync credentials survive
/// the upgrade.
pub fn read_default_app_sync_settings() -> Result<AppSyncSettings> {
    read_app_sync_settings_from_data_home(&xdg_data_home()?)
}

fn read_app_sync_settings_from_data_home(data_home: &Path) -> Result<AppSyncSettings> {
    let current = shared_preferences_path(data_home, APP_ID);
    if current.exists() {
        return read_app_sync_settings(current);
    }
    let legacy = shared_preferences_path(data_home, LEGACY_APP_ID);
    if legacy.exists() {
        return read_app_sync_settings(legacy);
    }
    read_app_sync_settings(current)
}

struct SyncEndpoint {
    host: String,
    port: u16,
    path: String,
}

/// Read the sync endpoint the Haqor app has already been configured to use.
/// Flutter's Linux shared-preference backend prefixes its keys with `flutter.`.
pub fn read_app_sync_settings(path: impl AsRef<Path>) -> Result<AppSyncSettings> {
    let path = path.as_ref();
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading app preferences {}", path.display()))?;
    let preferences: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing app preferences {}", path.display()))?;
    let text = |keys: &[&str], label: &str| -> Result<String> {
        keys.iter()
            .find_map(|key| preferences.get(*key).and_then(Value::as_str))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .with_context(|| {
                format!(
                    "{label} is not configured in {}; supply explicit --server and --token instead",
                    path.display()
                )
            })
    };
    Ok(AppSyncSettings {
        server_url: text(
            &[
                "flutter.progress_sync_server_url",
                "progress_sync_server_url",
            ],
            "sync server URL",
        )?,
        token: text(
            &["flutter.progress_sync_token", "progress_sync_token"],
            "sync token",
        )?,
    })
}

/// Pull mobile lexicon corrections from the canonical sync database into the
/// hand-maintained overlay. Tutor corrections update `word_glosses`, while
/// word-info corrections update `lexicon_entries` root/header gloss rows.
pub fn pull_gloss_overrides(progress: &Path, overlay: &Path) -> Result<usize> {
    let corrections = haqor_core::progress_sync::read_gloss_overrides_file(progress)
        .with_context(|| format!("reading tutor corrections from {}", progress.display()))?;
    let lexicon_corrections = haqor_core::progress_sync::read_lexicon_entry_overrides_file(
        progress,
    )
    .with_context(|| format!("reading word-info corrections from {}", progress.display()))?;
    let mut value = haqor_core::lexicon_overlay::load(overlay)?;
    let rows = value["word_glosses"]
        .as_array_mut()
        .context("overlay `word_glosses` must be an array")?;
    for correction in &corrections {
        let row = rows
            .iter_mut()
            .find(|row| row["surface"].as_str() == Some(&correction.surface));
        let row = match row {
            Some(row) => row,
            None => {
                rows.push(json!({"surface": correction.surface, "gloss": correction.gloss}));
                rows.last_mut().expect("just pushed a row")
            }
        };
        let object = row
            .as_object_mut()
            .context("overlay word gloss row must be an object")?;
        object.insert("gloss".to_string(), Value::String(correction.gloss.clone()));
        if correction.note.is_empty() {
            object.remove("note");
        } else {
            object.insert("note".to_string(), Value::String(correction.note.clone()));
        }
    }
    let rows = value["lexicon_entries"]
        .as_array_mut()
        .context("overlay `lexicon_entries` must be an array")?;
    for correction in &lexicon_corrections {
        let row = rows
            .iter_mut()
            .find(|row| row["surface"].as_str() == Some(&correction.surface));
        let row = match row {
            Some(row) => row,
            None => {
                rows.push(json!({"surface": correction.surface}));
                rows.last_mut().expect("just pushed a row")
            }
        };
        let object = row
            .as_object_mut()
            .context("overlay lexicon entry row must be an object")?;
        object.insert("root".to_string(), Value::String(correction.root.clone()));
        object.insert("gloss".to_string(), Value::String(correction.gloss.clone()));
    }
    haqor_core::lexicon_overlay::save(overlay, &value)?;
    Ok(corrections.len() + lexicon_corrections.len())
}

/// Fetch the canonical progress snapshot from an authenticated LAN sync server
/// and merge its mobile lexicon corrections into the local overlay JSON.
pub fn pull_gloss_overrides_from_server(
    server_url: &str,
    token: &str,
    overlay: &Path,
) -> Result<usize> {
    with_remote_progress(server_url, token, "glosses", |progress| {
        pull_gloss_overrides(progress, overlay)
    })
}

/// Export the synchronised mobile bug/idea log as deterministic, pretty JSON.
pub fn pull_issue_reports(progress: &Path, output: &Path) -> Result<usize> {
    if !haqor_core::progress_sync::has_issue_reports_file(progress)
        .with_context(|| format!("checking issue-report support in {}", progress.display()))?
    {
        bail!(
            "the synced progress snapshot has no issue_reports table; the running \
             haqor-sync-server is out of date. Update and restart it, then sync \
             the app again; reports remain saved on the device"
        );
    }
    let reports = haqor_core::progress_sync::read_issue_reports_file(progress)
        .with_context(|| format!("reading issue reports from {}", progress.display()))?;
    let rows = reports
        .iter()
        .map(|report| {
            let context: Value = serde_json::from_str(&report.context_json)
                .with_context(|| format!("parsing context for issue report {}", report.id))?;
            Ok(json!({
                "id": report.id,
                "type": report.report_type,
                "note": report.note,
                "createdEpoch": report.created_epoch,
                "updatedEpoch": report.updated_epoch,
                "context": context,
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    let rendered = serde_json::to_string_pretty(&json!({"issueReports": rows}))? + "\n";
    let file_name = output
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("issue_reports.json");
    let temporary = output.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()));
    std::fs::write(&temporary, rendered.as_bytes())
        .with_context(|| format!("writing temporary issue log {}", temporary.display()))?;
    if let Err(error) = std::fs::rename(&temporary, output) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error).with_context(|| format!("replacing issue log {}", output.display()));
    }
    Ok(reports.len())
}

/// Fetch the canonical progress snapshot from the LAN server and export its
/// issue/idea log to a local JSON file.
pub fn pull_issue_reports_from_server(
    server_url: &str,
    token: &str,
    output: &Path,
) -> Result<usize> {
    with_remote_progress(server_url, token, "issues", |progress| {
        pull_issue_reports(progress, output)
    })
}

/// Run an interactive terminal review of the issue log in a local progress
/// snapshot. Pressing `s` in the review marks selected reports resolved in
/// that snapshot; tombstones prevent stale devices from restoring them.
pub fn review_issue_reports(progress: &Path, output: &Path) -> Result<usize> {
    let reports = haqor_core::progress_sync::read_issue_reports_file(progress)
        .with_context(|| format!("reading issue reports from {}", progress.display()))?;
    issue_tui::review(reports, |action, _current| match action {
        issue_tui::Action::Pull => {
            let count = pull_issue_reports(progress, output)?;
            Ok(issue_tui::ActionResult {
                reports: haqor_core::progress_sync::read_issue_reports_file(progress)?,
                resolved: 0,
                message: format!("Pulled {count} issue report(s) to {}", output.display()),
            })
        }
        issue_tui::Action::Sync(selected) => {
            let updated_epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
            let count = haqor_core::progress_sync::resolve_issue_reports_file(
                progress,
                &selected,
                updated_epoch,
            )
            .with_context(|| format!("resolving issue reports in {}", progress.display()))?;
            Ok(issue_tui::ActionResult {
                reports: haqor_core::progress_sync::read_issue_reports_file(progress)?,
                resolved: count,
                message: format!("Resolved {count} issue report(s) locally"),
            })
        }
    })
}

/// Review the server's live issue log, then upload the selected resolution
/// tombstones through the normal authenticated progress-sync route.
pub fn review_issue_reports_from_server(
    server_url: &str,
    token: &str,
    output: &Path,
) -> Result<usize> {
    if token.trim().is_empty() {
        bail!("--token must not be empty");
    }
    let endpoint = parse_sync_endpoint(server_url)?;
    eprintln!(
        "Downloading app issue reports from {}:{}",
        endpoint.host, endpoint.port
    );
    let snapshot = download_remote_snapshot(&endpoint, token, "app issue reports")?;
    if !haqor_core::progress_sync::is_sqlite_snapshot(&snapshot) {
        bail!("sync server returned an invalid progress snapshot");
    }
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let temporary = std::env::temp_dir().join(format!(
        "haqor-admin-review-issues-{}-{nonce}.db",
        std::process::id()
    ));
    std::fs::write(&temporary, snapshot)
        .with_context(|| format!("writing temporary sync snapshot {}", temporary.display()))?;
    let result = (|| {
        let reports = haqor_core::progress_sync::read_issue_reports_file(&temporary)?;
        issue_tui::review(reports, |action, current| match action {
            issue_tui::Action::Pull => {
                let snapshot = download_remote_snapshot(&endpoint, token, "app issue reports")?;
                std::fs::write(&temporary, snapshot)?;
                let count = pull_issue_reports(&temporary, output)?;
                Ok(issue_tui::ActionResult {
                    reports: haqor_core::progress_sync::read_issue_reports_file(&temporary)?,
                    resolved: 0,
                    message: format!("Pulled {count} issue report(s) to {}", output.display()),
                })
            }
            issue_tui::Action::Sync(selected) => {
                let updated_epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
                haqor_core::progress_sync::resolve_issue_reports_file(
                    &temporary,
                    &selected,
                    updated_epoch,
                )?;
                eprintln!(
                    "Syncing {} resolved app issue report(s) back to the server",
                    selected.len()
                );
                let merged = post_sync_snapshot(&endpoint, token, &std::fs::read(&temporary)?)?;
                if !haqor_core::progress_sync::is_sqlite_snapshot(&merged) {
                    bail!("sync server returned an invalid progress snapshot");
                }
                std::fs::write(&temporary, merged)?;
                let merged_reports =
                    haqor_core::progress_sync::read_issue_reports_file(&temporary)?;
                if selected.iter().any(|id| {
                    current.iter().any(|report| report.id == *id)
                        && merged_reports.iter().any(|report| report.id == *id)
                }) {
                    bail!(
                        "the sync server did not retain resolved issue reports; update and restart \
                         haqor-sync-server before resolving reports"
                    );
                }
                Ok(issue_tui::ActionResult {
                    reports: merged_reports,
                    resolved: selected.len(),
                    message: format!("Synced {} resolved issue report(s)", selected.len()),
                })
            }
        })
    })();
    let _ = std::fs::remove_file(&temporary);
    result
}

fn download_remote_snapshot(
    endpoint: &SyncEndpoint,
    token: &str,
    purpose: &str,
) -> Result<Vec<u8>> {
    eprintln!(
        "Downloading {purpose} from {}:{}",
        endpoint.host, endpoint.port
    );
    let snapshot = match fetch_sync_snapshot(endpoint, token) {
        Ok(snapshot) => snapshot,
        Err(error) if error.to_string().contains("404 Not Found") => {
            eprintln!(
                "Sync server does not support snapshot downloads; using its compatible sync route"
            );
            post_sync_snapshot(endpoint, token, &empty_progress_snapshot()?)?
        }
        Err(error) => return Err(error),
    };
    if !haqor_core::progress_sync::is_sqlite_snapshot(&snapshot) {
        bail!("sync server returned an invalid progress snapshot");
    }
    Ok(snapshot)
}

fn with_remote_progress<T>(
    server_url: &str,
    token: &str,
    purpose: &str,
    operation: impl FnOnce(&Path) -> Result<T>,
) -> Result<T> {
    if token.trim().is_empty() {
        bail!("--token must not be empty");
    }
    let endpoint = parse_sync_endpoint(server_url)?;
    eprintln!(
        "Pulling synced {purpose} from {}:{}",
        endpoint.host, endpoint.port,
    );
    let snapshot = match fetch_sync_snapshot(&endpoint, token) {
        Ok(snapshot) => snapshot,
        Err(error) if error.to_string().contains("404 Not Found") => {
            eprintln!(
                "Sync server does not support snapshot downloads; using its compatible sync route"
            );
            let empty_snapshot = empty_progress_snapshot()?;
            post_sync_snapshot(&endpoint, token, &empty_snapshot)?
        }
        Err(error) => return Err(error),
    };
    if !haqor_core::progress_sync::is_sqlite_snapshot(&snapshot) {
        bail!("sync server returned an invalid progress snapshot");
    }
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let temporary = std::env::temp_dir().join(format!(
        "haqor-admin-pull-{purpose}-{}-{nonce}.db",
        std::process::id(),
    ));
    std::fs::write(&temporary, snapshot)
        .with_context(|| format!("writing temporary sync snapshot {}", temporary.display()))?;
    let result = operation(&temporary);
    let _ = std::fs::remove_file(&temporary);
    result
}

fn parse_sync_endpoint(input: &str) -> Result<SyncEndpoint> {
    let rest = input
        .trim()
        .strip_prefix("http://")
        .context("sync server must start with http://")?;
    let (authority, path) = match rest.find('/') {
        Some(index) if &rest[index..] == "/" => (&rest[..index], "/v1/progress"),
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/v1/progress"),
    };
    if authority.is_empty() || authority.contains('@') {
        bail!("sync server address is invalid");
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => (
            host.to_string(),
            port.parse().context("sync server port is invalid")?,
        ),
        _ => (authority.to_string(), 80),
    };
    Ok(SyncEndpoint {
        host,
        port,
        path: path.to_string(),
    })
}

fn fetch_sync_snapshot(endpoint: &SyncEndpoint, token: &str) -> Result<Vec<u8>> {
    let address = format!("{}:{}", endpoint.host, endpoint.port);
    let socket = address
        .to_socket_addrs()
        .with_context(|| format!("resolving sync server {}", endpoint.host))?
        .next()
        .context("sync server address did not resolve")?;
    let mut stream = TcpStream::connect_timeout(&socket, Duration::from_secs(10))
        .context("connecting to sync server")?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    write!(
        stream,
        "GET {} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nConnection: close\r\n\r\n",
        endpoint.path, endpoint.host, token
    )?;

    let mut reader = BufReader::new(stream);
    let mut status = String::new();
    reader.read_line(&mut status)?;
    if !status.starts_with("HTTP/1.1 200") && !status.starts_with("HTTP/1.0 200") {
        bail!("sync server returned {}", status.trim());
    }
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            bail!("sync server closed the response headers early");
        }
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let length = content_length.context("sync server omitted Content-Length")?;
    if length > MAX_SNAPSHOT_BYTES {
        bail!("sync server returned an unexpectedly large snapshot");
    }
    let mut snapshot = vec![0; length];
    reader.read_exact(&mut snapshot)?;
    Ok(snapshot)
}

/// An empty, fully initialized snapshot safely exercises a pre-download sync
/// server's POST route: merging it cannot change any learner data, and the
/// server returns its canonical snapshot in response.
fn empty_progress_snapshot() -> Result<Vec<u8>> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let temporary = std::env::temp_dir().join(format!(
        "haqor-admin-empty-progress-{}-{nonce}.db",
        std::process::id()
    ));
    let result = (|| {
        let db = Connection::open_in_memory()?;
        db.execute(
            "ATTACH DATABASE ?1 AS progress",
            [temporary.to_string_lossy().as_ref()],
        )?;
        haqor_core::tutor::init_progress_schema(&db)?;
        drop(db);
        std::fs::read(&temporary).context("reading empty progress snapshot")
    })();
    let _ = std::fs::remove_file(&temporary);
    result
}

fn post_sync_snapshot(endpoint: &SyncEndpoint, token: &str, snapshot: &[u8]) -> Result<Vec<u8>> {
    let address = format!("{}:{}", endpoint.host, endpoint.port);
    let socket = address
        .to_socket_addrs()
        .with_context(|| format!("resolving sync server {}", endpoint.host))?
        .next()
        .context("sync server address did not resolve")?;
    let mut stream = TcpStream::connect_timeout(&socket, Duration::from_secs(10))
        .context("connecting to sync server")?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    write!(
        stream,
        "POST {} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nContent-Type: application/vnd.sqlite3\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        endpoint.path,
        endpoint.host,
        token,
        snapshot.len(),
    )?;
    stream.write_all(snapshot)?;

    let mut reader = BufReader::new(stream);
    let mut status = String::new();
    reader.read_line(&mut status)?;
    if !status.starts_with("HTTP/1.1 200") && !status.starts_with("HTTP/1.0 200") {
        bail!("sync server returned {}", status.trim());
    }
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            bail!("sync server closed the response headers early");
        }
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let length = content_length.context("sync server omitted Content-Length")?;
    if length > MAX_SNAPSHOT_BYTES {
        bail!("sync server returned an unexpectedly large snapshot");
    }
    let mut merged = vec![0; length];
    reader.read_exact(&mut merged)?;
    Ok(merged)
}

pub fn serve(bind: SocketAddr, overlay: PathBuf, lexicon: PathBuf, hebrew: PathBuf) -> Result<()> {
    if !bind.ip().is_loopback() {
        bail!(
            "refusing to expose the unauthenticated overlay editor on non-loopback address {bind}"
        );
    }
    haqor_core::lexicon_overlay::load(&overlay)?;
    read_lexicon(&lexicon)?;
    read_ambiguous(&hebrew)?;
    let listener =
        TcpListener::bind(bind).with_context(|| format!("binding admin server to {bind}"))?;
    eprintln!("Overlay editor: http://{bind}");
    eprintln!("Editing: {}", overlay.display());
    eprintln!("Browsing: {}", lexicon.display());
    eprintln!("Reviewing: {}", hebrew.display());
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let path = overlay.clone();
                let lexicon = lexicon.clone();
                let hebrew = hebrew.clone();
                std::thread::spawn(move || {
                    if let Err(error) = handle(stream, &path, &lexicon, &hebrew) {
                        eprintln!("overlay editor request failed: {error:#}");
                    }
                });
            }
            Err(error) => eprintln!("overlay editor connection failed: {error}"),
        }
    }
    Ok(())
}

fn handle(mut stream: TcpStream, overlay: &Path, lexicon: &Path, hebrew: &Path) -> Result<()> {
    let (method, target, body) = read_request(&mut stream)?;
    match (method.as_str(), target.as_str()) {
        ("GET", "/") => respond(&mut stream, 200, "text/html; charset=utf-8", EDITOR),
        ("GET", "/api/overlay") => match std::fs::read_to_string(overlay) {
            Ok(body) => respond(&mut stream, 200, "application/json; charset=utf-8", &body),
            Err(error) => error_response(&mut stream, 500, &error.to_string()),
        },
        ("GET", "/api/lexicon") => match read_lexicon(lexicon) {
            Ok(value) => respond(
                &mut stream,
                200,
                "application/json; charset=utf-8",
                &value.to_string(),
            ),
            Err(error) => error_response(&mut stream, 500, &format!("{error:#}")),
        },
        ("GET", "/api/ambiguous") => match read_ambiguous(hebrew) {
            Ok(value) => respond(
                &mut stream,
                200,
                "application/json; charset=utf-8",
                &value.to_string(),
            ),
            Err(error) => error_response(&mut stream, 500, &format!("{error:#}")),
        },
        ("PUT", "/api/overlay") => match serde_json::from_slice(&body)
            .context("request body is not valid JSON")
            .and_then(|value| haqor_core::lexicon_overlay::save(overlay, &value))
        {
            Ok(_) => respond(
                &mut stream,
                200,
                "application/json; charset=utf-8",
                &json!({"ok": true}).to_string(),
            ),
            Err(error) => error_response(&mut stream, 400, &format!("{error:#}")),
        },
        _ => error_response(&mut stream, 404, "not found"),
    }
}

fn read_ambiguous(path: &Path) -> Result<serde_json::Value> {
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("opening Hebrew database {}", path.display()))?;
    let mut statement = db.prepare(
        "WITH chosen AS (
           SELECT surface_id FROM surface
           WHERE n_candidates > 1 AND lexical_class IS NULL AND language IS NULL
           ORDER BY occurrences DESC, surface_id LIMIT 500
         )
         SELECT s.surface_id, s.text, s.occurrences, a.analysis_id, 'verb',
                a.root, a.binyan, a.form, a.pgn, a.prefix, a.vav_consecutive,
                a.obj_suffix, a.attested, '', '', ''
         FROM chosen c JOIN surface s USING(surface_id) JOIN analyses a USING(surface_id)
         UNION ALL
         SELECT s.surface_id, s.text, s.occurrences, n.analysis_id, 'noun',
                '', '', '', '', n.prefix, 0, '', 1, n.stem, n.kind, n.label
         FROM chosen c JOIN surface s USING(surface_id) JOIN noun_analyses n USING(surface_id)
         ORDER BY 3 DESC, 1, 5 DESC, 4",
    )?;
    let mut surfaces: Vec<serde_json::Value> = Vec::new();
    let mut identities = HashSet::new();
    let mapped = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            json!({"analysis_id": row.get::<_, i64>(3)?, "analysis_type": row.get::<_, String>(4)?,
          "root": row.get::<_, String>(5)?, "binyan": row.get::<_, String>(6)?,
          "form": row.get::<_, String>(7)?, "pgn": row.get::<_, String>(8)?,
          "prefix": row.get::<_, String>(9)?, "vav_consecutive": row.get::<_, i64>(10)? != 0,
          "obj_suffix": row.get::<_, String>(11)?, "attested": row.get::<_, i64>(12)? != 0,
          "stem": row.get::<_, String>(13)?, "kind": row.get::<_, String>(14)?,
          "label": row.get::<_, String>(15)?}),
        ))
    })?;
    for row in mapped {
        let (id, text, occurrences, analysis) = row?;
        if surfaces.last().and_then(|v| v["surface_id"].as_i64()) != Some(id) {
            surfaces.push(json!({"surface_id": id, "surface": text, "occurrences": occurrences, "analyses": []}));
            identities.clear();
        }
        let fields: &[&str] = if analysis["analysis_type"] == "noun" {
            &["analysis_type", "stem", "kind", "label", "prefix"]
        } else {
            &[
                "analysis_type",
                "root",
                "binyan",
                "form",
                "pgn",
                "prefix",
                "vav_consecutive",
                "obj_suffix",
            ]
        };
        let identity = fields
            .iter()
            .map(|field| analysis[*field].to_string())
            .collect::<Vec<_>>()
            .join("\u{1f}");
        if !identities.insert(identity) {
            continue;
        }
        surfaces.last_mut().unwrap()["analyses"]
            .as_array_mut()
            .unwrap()
            .push(analysis);
    }
    Ok(json!({"surfaces": surfaces}))
}

fn read_lexicon(path: &Path) -> Result<serde_json::Value> {
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("opening lexicon database {}", path.display()))?;
    let mut rows = Vec::new();
    {
        let mut statement = db.prepare(
            "SELECT 'BDB', bdb_id, word, root, gloss FROM bdb
             WHERE word IS NOT NULL AND word <> '' AND gloss IS NOT NULL AND gloss <> ''
             UNION ALL
             SELECT 'Strong', 'H' || strong, word, '', gloss FROM english
             WHERE word IS NOT NULL AND word <> '' AND gloss IS NOT NULL AND gloss <> ''
             ORDER BY 3, 1, 2",
        )?;
        let mapped = statement.query_map([], |row| {
            Ok(json!({
                "source": row.get::<_, String>(0)?,
                "id": row.get::<_, String>(1)?,
                "surface": row.get::<_, String>(2)?,
                "root": row.get::<_, String>(3)?,
                "gloss": row.get::<_, String>(4)?,
            }))
        })?;
        for row in mapped {
            rows.push(row?);
        }
    }
    Ok(json!({"entries": rows}))
}

fn read_request(stream: &mut TcpStream) -> Result<(String, String, Vec<u8>)> {
    let mut data = Vec::new();
    let mut chunk = [0_u8; 8192];
    let header_end;
    loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            bail!("connection closed before request headers");
        }
        data.extend_from_slice(&chunk[..read]);
        if data.len() > MAX_REQUEST_BYTES {
            bail!("request exceeds {MAX_REQUEST_BYTES} bytes");
        }
        if let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            header_end = end + 4;
            break;
        }
    }
    let headers =
        std::str::from_utf8(&data[..header_end]).context("request headers are not UTF-8")?;
    let mut lines = headers.split("\r\n");
    let mut request_line = lines.next().unwrap_or_default().split_whitespace();
    let method = request_line
        .next()
        .context("missing HTTP method")?
        .to_string();
    let target = request_line
        .next()
        .context("missing request target")?
        .split('?')
        .next()
        .unwrap()
        .to_string();
    let content_length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map(|(_, value)| value.trim().parse::<usize>())
        .transpose()
        .context("invalid Content-Length")?
        .unwrap_or(0);
    if header_end + content_length > MAX_REQUEST_BYTES {
        bail!("request exceeds {MAX_REQUEST_BYTES} bytes");
    }
    while data.len() < header_end + content_length {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            bail!("connection closed before request body");
        }
        data.extend_from_slice(&chunk[..read]);
    }
    Ok((
        method,
        target,
        data[header_end..header_end + content_length].to_vec(),
    ))
}

fn error_response(stream: &mut TcpStream, status: u16, message: &str) -> Result<()> {
    respond(
        stream,
        status,
        "application/json; charset=utf-8",
        &json!({"ok": false, "error": message}).to_string(),
    )
}

fn respond(stream: &mut TcpStream, status: u16, content_type: &str, body: &str) -> Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_exposes_structured_overlay_controls() {
        assert!(EDITOR.contains("Add lexicon entry"));
        assert!(EDITOR.contains("Add word gloss"));
        assert!(EDITOR.contains("Proper name"));
        assert!(EDITOR.contains("Imported glosses"));
        assert!(EDITOR.contains("Create overlay"));
        assert!(EDITOR.contains("Ambiguous analyses"));
        assert!(!EDITOR.contains("Lexicon overlay JSON"));
    }

    fn request(path: &Path, lexicon: &Path, request: String) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let overlay = path.to_owned();
        let lexicon = lexicon.to_owned();
        let hebrew = lexicon.clone();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle(stream, &overlay, &lexicon, &hebrew).unwrap();
        });
        let mut client = TcpStream::connect(address).unwrap();
        client.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        server.join().unwrap();
        response
    }

    #[test]
    #[ignore = "requires loopback sockets, which some build sandboxes disable"]
    fn api_reads_and_validates_saves() {
        let path = std::env::temp_dir().join(format!("haqor-admin-{}.json", std::process::id()));
        let original = json!({"lexicon_entries": [], "word_glosses": [], "primary_analyses": []});
        std::fs::write(&path, serde_json::to_string(&original).unwrap()).unwrap();
        let lexicon =
            std::env::temp_dir().join(format!("haqor-admin-lexicon-{}.db", std::process::id()));
        let db = Connection::open(&lexicon).unwrap();
        db.execute_batch(
            "CREATE TABLE bdb(bdb_id TEXT, word TEXT, root TEXT, gloss TEXT);
             CREATE TABLE english(strong INTEGER, word TEXT, gloss TEXT);
             INSERT INTO bdb VALUES ('a.b', 'אָב', 'אב', 'father');",
        )
        .unwrap();
        drop(db);

        let response = request(
            &path,
            &lexicon,
            "GET /api/overlay HTTP/1.1\r\nHost: localhost\r\n\r\n".into(),
        );
        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("word_glosses"));

        let invalid = r#"{"lexicon_entries":[],"word_glosses":[{}]}"#;
        let response = request(
            &path,
            &lexicon,
            format!(
                "PUT /api/overlay HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{invalid}",
                invalid.len()
            ),
        );
        assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(&path).unwrap())
                .unwrap(),
            original
        );
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(lexicon).unwrap();
    }

    #[test]
    fn lexicon_catalogue_reads_imported_glosses() {
        let path =
            std::env::temp_dir().join(format!("haqor-admin-catalogue-{}.db", std::process::id()));
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE bdb(bdb_id TEXT, word TEXT, root TEXT, gloss TEXT);
             CREATE TABLE english(strong INTEGER, word TEXT, gloss TEXT);
             INSERT INTO bdb VALUES ('a.b', 'אָב', 'אב', 'father');
             INSERT INTO english VALUES (1, 'אָב', 'father');",
        )
        .unwrap();
        drop(db);
        let value = read_lexicon(&path).unwrap();
        assert_eq!(value["entries"].as_array().unwrap().len(), 2);
        assert_eq!(value["entries"][0]["surface"], "אָב");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ambiguity_catalogue_includes_noun_candidates() {
        let path =
            std::env::temp_dir().join(format!("haqor-admin-ambiguity-{}.db", std::process::id()));
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE surface(surface_id INTEGER, text TEXT, occurrences INTEGER,
                n_candidates INTEGER, lexical_class TEXT, language TEXT);
             CREATE TABLE analyses(analysis_id INTEGER, surface_id INTEGER, root TEXT,
                binyan TEXT, form TEXT, pgn TEXT, prefix TEXT, vav_consecutive INTEGER,
                obj_suffix TEXT, attested INTEGER);
             CREATE TABLE noun_analyses(analysis_id INTEGER, surface_id INTEGER, stem TEXT,
                kind TEXT, label TEXT, prefix TEXT);
             INSERT INTO surface VALUES (1, 'אִישׁ', 10, 2, NULL, NULL);
             INSERT INTO analyses VALUES (1, 1, 'איש', 'Qal', 'Imperative', '2ms', '', 0, '', 1);
             INSERT INTO noun_analyses VALUES (2, 1, 'אִישׁ', 'Masculine', 'Irregular (man)', '');
             INSERT INTO noun_analyses VALUES (3, 1, 'אִישׁ', 'Masculine', 'Irregular (man)', '');",
        )
        .unwrap();
        drop(db);
        let value = read_ambiguous(&path).unwrap();
        let analyses = value["surfaces"][0]["analyses"].as_array().unwrap();
        assert!(analyses.iter().any(|a| a["analysis_type"] == "verb"));
        assert!(analyses.iter().any(|a| a["analysis_type"] == "noun"));
        assert_eq!(analyses.len(), 2, "equivalent noun rows are deduplicated");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn pull_merges_mobile_glosses_without_losing_name_metadata() -> Result<()> {
        let base = std::env::temp_dir().join(format!("haqor-admin-pull-{}", std::process::id()));
        let overlay = base.with_extension("json");
        let progress = base.with_extension("db");
        let _ = std::fs::remove_file(&overlay);
        let _ = std::fs::remove_file(&progress);
        std::fs::write(
            &overlay,
            r#"{"lexicon_entries":[],"word_glosses":[{"surface":"דָּבָר","gloss":"word","is_name":true}],"primary_analyses":[]}"#,
        )?;
        let db = Connection::open(&progress)?;
        db.execute_batch(
            "CREATE TABLE gloss_overrides(surface TEXT PRIMARY KEY, gloss TEXT NOT NULL,
                note TEXT NOT NULL DEFAULT '', updated_epoch INTEGER NOT NULL);
             INSERT INTO gloss_overrides VALUES ('דָּבָר', 'matter', 'In this context.', 1);
             INSERT INTO gloss_overrides VALUES ('טוֹב', 'good', '', 2);
             CREATE TABLE lexicon_entry_overrides(
                surface TEXT PRIMARY KEY, root TEXT NOT NULL DEFAULT '',
                gloss TEXT NOT NULL, updated_epoch INTEGER NOT NULL);
             INSERT INTO lexicon_entry_overrides
                VALUES ('דָּבָר', 'דבר', 'speech, word', 3);",
        )?;
        drop(db);

        assert_eq!(pull_gloss_overrides(&progress, &overlay)?, 3);
        let value: Value = serde_json::from_str(&std::fs::read_to_string(&overlay)?)?;
        let rows = value["word_glosses"].as_array().unwrap();
        assert_eq!(rows[0]["gloss"], "matter");
        assert_eq!(rows[0]["note"], "In this context.");
        assert_eq!(rows[0]["is_name"], true);
        assert_eq!(rows[1]["surface"], "טוֹב");
        assert!(rows[1].get("note").is_none());
        let entry = &value["lexicon_entries"][0];
        assert_eq!(entry["surface"], "דָּבָר");
        assert_eq!(entry["root"], "דבר");
        assert_eq!(entry["gloss"], "speech, word");
        std::fs::remove_file(overlay)?;
        std::fs::remove_file(progress)?;
        Ok(())
    }

    #[test]
    fn pull_exports_issue_reports_with_structured_context() -> Result<()> {
        let base =
            std::env::temp_dir().join(format!("haqor-admin-pull-issues-{}", std::process::id()));
        let progress = base.with_extension("db");
        let output = base.with_extension("json");
        let _ = std::fs::remove_file(&progress);
        let _ = std::fs::remove_file(&output);
        let db = Connection::open(&progress)?;
        db.execute_batch(
            "CREATE TABLE issue_reports(
                id TEXT PRIMARY KEY, report_type TEXT NOT NULL, note TEXT NOT NULL,
                context_json TEXT NOT NULL, created_epoch INTEGER NOT NULL,
                updated_epoch INTEGER NOT NULL);
             INSERT INTO issue_reports VALUES (
                'phone-1', 'bug', 'Wrong answer shown',
                '{\"source\":\"tutor_card\",\"card\":{\"kind\":\"review_word\"}}',
                100, 100);",
        )?;
        drop(db);

        assert_eq!(pull_issue_reports(&progress, &output)?, 1);
        let value: Value = serde_json::from_str(&std::fs::read_to_string(&output)?)?;
        let report = &value["issueReports"][0];
        assert_eq!(report["id"], "phone-1");
        assert_eq!(report["type"], "bug");
        assert_eq!(report["context"]["source"], "tutor_card");
        assert_eq!(report["context"]["card"]["kind"], "review_word");

        std::fs::remove_file(progress)?;
        std::fs::remove_file(output)?;
        Ok(())
    }

    #[test]
    fn pull_issues_rejects_a_legacy_server_snapshot() -> Result<()> {
        let base = std::env::temp_dir().join(format!(
            "haqor-admin-pull-legacy-issues-{}",
            std::process::id()
        ));
        let progress = base.with_extension("db");
        let output = base.with_extension("json");
        let _ = std::fs::remove_file(&progress);
        let _ = std::fs::remove_file(&output);
        Connection::open(&progress)?
            .execute_batch("CREATE TABLE legacy_progress(value INTEGER NOT NULL);")?;

        let error = pull_issue_reports(&progress, &output).unwrap_err();
        assert!(error.to_string().contains("sync-server is out of date"));
        assert!(!output.exists());

        std::fs::remove_file(progress)?;
        Ok(())
    }

    #[test]
    fn reads_flutter_sync_settings_without_exposing_the_token() -> Result<()> {
        let preferences = std::env::temp_dir().join(format!(
            "haqor-admin-preferences-{}-{}.json",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::write(
            &preferences,
            r#"{"flutter.progress_sync_server_url":" http://192.168.1.10:8788 ","flutter.progress_sync_token":" secret "}"#,
        )?;
        assert_eq!(
            read_app_sync_settings(&preferences)?,
            AppSyncSettings {
                server_url: "http://192.168.1.10:8788".to_string(),
                token: "secret".to_string(),
            }
        );
        std::fs::remove_file(preferences)?;
        Ok(())
    }

    #[test]
    fn root_server_url_uses_the_progress_endpoint() -> Result<()> {
        let endpoint = parse_sync_endpoint("http://sync.example:8788/")?;

        assert_eq!(endpoint.host, "sync.example");
        assert_eq!(endpoint.port, 8788);
        assert_eq!(endpoint.path, "/v1/progress");
        Ok(())
    }

    #[test]
    fn empty_snapshot_fallback_preserves_server_progress() -> Result<()> {
        let base =
            std::env::temp_dir().join(format!("haqor-admin-empty-snapshot-{}", std::process::id()));
        let canonical = base.with_extension("canonical.db");
        let incoming = base.with_extension("incoming.db");
        let _ = std::fs::remove_file(&canonical);
        let _ = std::fs::remove_file(&incoming);
        let db = Connection::open_in_memory()?;
        db.execute(
            "ATTACH DATABASE ?1 AS progress",
            [canonical.to_string_lossy().as_ref()],
        )?;
        haqor_core::tutor::init_progress_schema(&db)?;
        db.execute(
            "INSERT INTO progress.gloss_overrides(surface, gloss, note, updated_epoch)
             VALUES ('דָּבָר', 'word', '', 1)",
            [],
        )?;
        drop(db);

        std::fs::write(&incoming, empty_progress_snapshot()?)?;
        haqor_core::progress_sync::merge_progress_files(&canonical, &incoming)?;
        let canonical_db = Connection::open(&canonical)?;
        assert_eq!(
            canonical_db.query_row(
                "SELECT gloss FROM gloss_overrides WHERE surface = 'דָּבָר'",
                [],
                |row| row.get::<_, String>(0),
            )?,
            "word"
        );
        std::fs::remove_file(canonical)?;
        std::fs::remove_file(incoming)?;
        Ok(())
    }

    #[test]
    fn default_settings_follow_xdg_data_home_and_migrate_the_template_id() -> Result<()> {
        let data_home = std::env::temp_dir().join(format!(
            "haqor-admin-xdg-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let legacy = shared_preferences_path(&data_home, LEGACY_APP_ID);
        std::fs::create_dir_all(legacy.parent().unwrap())?;
        std::fs::write(
            &legacy,
            r#"{"flutter.progress_sync_server_url":"http://sync:8788","flutter.progress_sync_token":"token"}"#,
        )?;
        assert_eq!(
            read_app_sync_settings_from_data_home(&data_home)?,
            AppSyncSettings {
                server_url: "http://sync:8788".to_string(),
                token: "token".to_string(),
            }
        );
        assert_eq!(
            shared_preferences_path(&data_home, APP_ID),
            data_home.join("org.haqor/shared_preferences.json")
        );
        std::fs::remove_dir_all(data_home)?;
        Ok(())
    }
}
