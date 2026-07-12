//! Minimal local HTTP server for editing `data/lexicon_overrides.json`.

use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rusqlite::Connection;
use serde_json::json;

const EDITOR: &str = include_str!("overlay_admin.html");
const MAX_REQUEST_BYTES: usize = 4 * 1024 * 1024;

pub fn serve(bind: SocketAddr, overlay: PathBuf, lexicon: PathBuf, hebrew: PathBuf) -> Result<()> {
    if !bind.ip().is_loopback() {
        bail!(
            "refusing to expose the unauthenticated overlay editor on non-loopback address {bind}"
        );
    }
    crate::lexicon_overlay::load(&overlay)?;
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
}
