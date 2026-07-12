//! Minimal local HTTP server for editing `data/lexicon_overrides.json`.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::json;

const EDITOR: &str = include_str!("overlay_admin.html");
const MAX_REQUEST_BYTES: usize = 4 * 1024 * 1024;

pub fn serve(bind: SocketAddr, overlay: PathBuf) -> Result<()> {
    if !bind.ip().is_loopback() {
        bail!(
            "refusing to expose the unauthenticated overlay editor on non-loopback address {bind}"
        );
    }
    crate::lexicon_overlay::load(&overlay)?;
    let listener =
        TcpListener::bind(bind).with_context(|| format!("binding admin server to {bind}"))?;
    eprintln!("Overlay editor: http://{bind}");
    eprintln!("Editing: {}", overlay.display());
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let path = overlay.clone();
                std::thread::spawn(move || {
                    if let Err(error) = handle(stream, &path) {
                        eprintln!("overlay editor request failed: {error:#}");
                    }
                });
            }
            Err(error) => eprintln!("overlay editor connection failed: {error}"),
        }
    }
    Ok(())
}

fn handle(mut stream: TcpStream, overlay: &Path) -> Result<()> {
    let (method, target, body) = read_request(&mut stream)?;
    match (method.as_str(), target.as_str()) {
        ("GET", "/") => respond(&mut stream, 200, "text/html; charset=utf-8", EDITOR),
        ("GET", "/api/overlay") => match std::fs::read_to_string(overlay) {
            Ok(body) => respond(&mut stream, 200, "application/json; charset=utf-8", &body),
            Err(error) => error_response(&mut stream, 500, &error.to_string()),
        },
        ("PUT", "/api/overlay") => match serde_json::from_slice(&body)
            .context("request body is not valid JSON")
            .and_then(|value| crate::lexicon_overlay::save(overlay, &value))
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
        assert!(!EDITOR.contains("Lexicon overlay JSON"));
    }

    fn request(path: &Path, request: String) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let overlay = path.to_owned();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle(stream, &overlay).unwrap();
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
        let original = json!({"lexicon_entries": [], "word_glosses": []});
        std::fs::write(&path, serde_json::to_string(&original).unwrap()).unwrap();

        let response = request(
            &path,
            "GET /api/overlay HTTP/1.1\r\nHost: localhost\r\n\r\n".into(),
        );
        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("word_glosses"));

        let invalid = r#"{"lexicon_entries":[],"word_glosses":[{}]}"#;
        let response = request(
            &path,
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
    }
}
