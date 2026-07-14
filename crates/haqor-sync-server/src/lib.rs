use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;

/// Run the deliberately small HTTP service used by Haqor apps on a trusted
/// LAN. TLS termination, if wanted, belongs in the caller's reverse proxy;
/// the built-in service always requires the supplied bearer token.
pub fn serve_progress(bind: SocketAddr, progress: &Path, token: &str) -> Result<()> {
    if token.len() < 16 {
        bail!("--token must be at least 16 characters long");
    }
    if let Some(parent) = progress.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating sync directory {}", parent.display()))?;
    }
    let listener =
        TcpListener::bind(bind).with_context(|| format!("binding sync server at {bind}"))?;
    println!("Haqor progress sync listening on http://{bind}/v1/progress");
    println!("Keep the token private: anyone with it can read and change your learning progress.");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle(stream, progress, token) {
                    eprintln!("sync request failed: {error:#}");
                }
            }
            Err(error) => eprintln!("sync connection failed: {error}"),
        }
    }
    Ok(())
}

fn handle(stream: TcpStream, progress: &Path, token: &str) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut authorization = None;
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            bail!("connection closed before request headers");
        }
        if line == "\r\n" {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            respond(stream, "400 Bad Request", b"malformed header")?;
            return Ok(());
        };
        match name.to_ascii_lowercase().as_str() {
            "authorization" => authorization = Some(value.trim().to_owned()),
            "content-length" => content_length = value.trim().parse::<usize>().ok(),
            _ => {}
        }
    }
    if authorization.as_deref() != Some(&format!("Bearer {token}")) {
        respond(stream, "401 Unauthorized", b"unauthorized")?;
        return Ok(());
    }
    let request = request_line.trim_end();
    if request == "GET /health HTTP/1.1" || request == "GET /health HTTP/1.0" {
        respond(stream, "200 OK", b"ok")?;
        return Ok(());
    }
    if request != "POST /v1/progress HTTP/1.1" && request != "POST /v1/progress HTTP/1.0" {
        respond(stream, "404 Not Found", b"not found")?;
        return Ok(());
    }
    let Some(length) = content_length else {
        respond(stream, "411 Length Required", b"content-length required")?;
        return Ok(());
    };
    if length > MAX_SNAPSHOT_BYTES {
        respond(stream, "413 Payload Too Large", b"snapshot too large")?;
        return Ok(());
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    if !haqor_core::progress_sync::is_sqlite_snapshot(&body) {
        respond(stream, "400 Bad Request", b"expected a SQLite snapshot")?;
        return Ok(());
    }
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let incoming =
        std::env::temp_dir().join(format!("haqor-sync-{}-{nonce}.db", std::process::id()));
    fs::write(&incoming, body)?;
    let result = haqor_core::progress_sync::merge_progress_files(progress, &incoming)
        .context("merging received progress snapshot");
    let _ = fs::remove_file(&incoming);
    result?;
    let snapshot = fs::read(progress).with_context(|| format!("reading {}", progress.display()))?;
    respond_with_snapshot(stream, &snapshot)
}

fn respond(mut stream: TcpStream, status: &str, body: &[u8]) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    Ok(())
}

fn respond_with_snapshot(mut stream: TcpStream, snapshot: &[u8]) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.sqlite3\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        snapshot.len()
    )?;
    stream.write_all(snapshot)?;
    Ok(())
}
