use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

pub mod client;

const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;
const MIN_TOKEN_LEN: usize = 16;
const IO_TIMEOUT: Duration = Duration::from_secs(30);
/// First byte of a TLS record carrying a handshake, which is how every TLS
/// connection opens. Plain HTTP requests start with an ASCII method instead.
const TLS_HANDSHAKE_RECORD: u8 = 0x16;

/// Where the server finds learners and their progress.
#[derive(Debug, Clone)]
pub enum Accounts {
    /// One learner: every request carrying `token` reads and merges `progress`.
    Single { progress: PathBuf, token: String },
    /// One learner per subdirectory, `<dir>/<user>/token` and
    /// `<dir>/<user>/progress.db`. The bearer token picks the learner, so each
    /// person's progress stays separate. The directory is read on every request,
    /// so learners added with `join` need no restart.
    Directory(PathBuf),
}

/// How the server listens.
pub struct ServeOptions {
    pub bind: SocketAddr,
    pub accounts: Accounts,
    /// Serve HTTPS with this identity. `None` serves plain HTTP only.
    pub tls: Option<TlsIdentity>,
    /// With TLS on, still answer plain HTTP on the same port, so apps joined
    /// before HTTPS keep syncing until they rejoin.
    pub allow_plain_http: bool,
}

/// The server's self-signed certificate. Apps trust it by its fingerprint,
/// which the join link carries, rather than through a certificate authority.
pub struct TlsIdentity {
    config: Arc<rustls::ServerConfig>,
    /// Lowercase hex SHA-256 of the certificate's DER encoding.
    pub fingerprint: String,
}

impl TlsIdentity {
    /// Load `cert.der` and `key.der` from `dir`, generating a new self-signed
    /// pair on first use. The pair is kept, so the fingerprint apps were given
    /// stays valid across restarts.
    pub fn load_or_create(dir: &Path) -> Result<Self> {
        let cert_path = dir.join("cert.der");
        let key_path = dir.join("key.der");
        if !cert_path.exists() || !key_path.exists() {
            fs::create_dir_all(dir)
                .with_context(|| format!("creating TLS directory {}", dir.display()))?;
            let generated = rcgen::generate_simple_self_signed(vec![
                "haqor-sync".to_string(),
                "localhost".to_string(),
            ])
            .context("generating a self-signed certificate")?;
            // Key first: a certificate on disk without its key would be loaded
            // as a half-written identity.
            write_atomic(&key_path, &generated.signing_key.serialize_der())?;
            write_atomic(&cert_path, generated.cert.der())?;
        }
        let cert =
            fs::read(&cert_path).with_context(|| format!("reading {}", cert_path.display()))?;
        let key = fs::read(&key_path).with_context(|| format!("reading {}", key_path.display()))?;
        let fingerprint = cert_fingerprint(&cert);
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .context("configuring TLS")?
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(cert)],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key)),
        )
        .context("loading the TLS certificate and key")?;
        Ok(Self {
            config: Arc::new(config),
            fingerprint,
        })
    }
}

/// Lowercase hex SHA-256 of a DER certificate, the form join links carry.
pub fn cert_fingerprint(der: &[u8]) -> String {
    hex(ring::digest::digest(&ring::digest::SHA256, der).as_ref())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let partial = path.with_extension("partial");
    fs::write(&partial, bytes).with_context(|| format!("writing {}", partial.display()))?;
    fs::rename(&partial, path).with_context(|| format!("writing {}", path.display()))
}

/// Run the single-learner plain-HTTP service, as earlier versions did.
pub fn serve_progress(bind: SocketAddr, progress: &Path, token: &str) -> Result<()> {
    serve(ServeOptions {
        bind,
        accounts: Accounts::Single {
            progress: progress.to_path_buf(),
            token: token.to_owned(),
        },
        tls: None,
        allow_plain_http: true,
    })
}

/// Run the deliberately small sync service used by Haqor apps on a trusted
/// network. Every request needs a learner's bearer token.
pub fn serve(options: ServeOptions) -> Result<()> {
    match &options.accounts {
        Accounts::Single { progress, token } => {
            if token.len() < MIN_TOKEN_LEN {
                bail!("--token must be at least {MIN_TOKEN_LEN} characters long");
            }
            if let Some(parent) = progress.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("creating sync directory {}", parent.display()))?;
            }
        }
        Accounts::Directory(dir) => {
            fs::create_dir_all(dir)
                .with_context(|| format!("creating users directory {}", dir.display()))?;
            let users = list_users(dir)?;
            println!(
                "Serving {} learner(s) from {}: {}",
                users.len(),
                dir.display(),
                users.join(", ")
            );
        }
    }
    let listener = TcpListener::bind(options.bind)
        .with_context(|| format!("binding sync server at {}", options.bind))?;
    match &options.tls {
        Some(tls) => {
            println!(
                "Haqor study and progress sync listening on https://{}/v1/progress",
                options.bind
            );
            println!("Certificate SHA-256 fingerprint: {}", tls.fingerprint);
            if options.allow_plain_http {
                println!(
                    "Plain HTTP is still accepted on the same port for apps not yet rejoined."
                );
            }
        }
        None => println!(
            "Haqor study and progress sync listening on http://{}/v1/progress (plain HTTP)",
            options.bind
        ),
    }
    println!("Keep tokens private: anyone with one can read and change that learning data.");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = accept(stream, &options) {
                    eprintln!("sync request failed: {error:#}");
                }
            }
            Err(error) => eprintln!("sync connection failed: {error}"),
        }
    }
    Ok(())
}

fn accept(mut stream: TcpStream, options: &ServeOptions) -> Result<()> {
    // The server handles one connection at a time, so a client that stalls
    // must not hold it forever.
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    let mut first = [0u8; 1];
    if stream.peek(&mut first)? == 0 {
        return Ok(());
    }
    match &options.tls {
        Some(tls) if first[0] == TLS_HANDSHAKE_RECORD => {
            let connection = rustls::ServerConnection::new(tls.config.clone())
                .context("starting TLS session")?;
            let mut tls_stream = rustls::StreamOwned::new(connection, stream);
            let result = handle(&mut tls_stream, &options.accounts);
            tls_stream.conn.send_close_notify();
            let _ = tls_stream.flush();
            result
        }
        Some(_) if !options.allow_plain_http => {
            // Read the request head first: closing with it unread makes the
            // kernel reset the connection, and the client never sees why.
            let mut reader = BufReader::new(&mut stream);
            let mut line = String::new();
            while reader.read_line(&mut line)? > 0 && line != "\r\n" && line != "\n" {
                line.clear();
            }
            respond(
                &mut stream,
                "426 Upgrade Required",
                b"this server only accepts HTTPS; rejoin sync with a fresh join link",
            )
        }
        _ => handle(&mut stream, &options.accounts),
    }
}

/// A learner name is used as a directory name and shown in the app, so keep it
/// to a short, unambiguous set of characters.
pub fn validate_user_name(name: &str) -> Result<()> {
    let valid = !name.is_empty()
        && name.len() <= 32
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        && !name.starts_with(['-', '_']);
    if !valid {
        bail!(
            "learner name {name:?} must be 1–32 characters of a–z, 0–9, '-' or '_', \
             starting with a letter or digit"
        );
    }
    Ok(())
}

/// Learner names found in a users directory, sorted.
pub fn list_users(dir: &Path) -> Result<Vec<String>> {
    let mut users = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        if validate_user_name(&name).is_ok() && entry.path().join("token").is_file() {
            users.push(name);
        }
    }
    users.sort();
    Ok(users)
}

/// Return the learner's token, creating the learner with a fresh random
/// token when they do not exist yet. The boolean is true when it was created.
pub fn ensure_user(dir: &Path, name: &str) -> Result<(String, bool)> {
    validate_user_name(name)?;
    let user_dir = dir.join(name);
    let token_path = user_dir.join("token");
    if let Some(token) = read_token(&token_path)? {
        return Ok((token, false));
    }
    fs::create_dir_all(&user_dir)
        .with_context(|| format!("creating learner directory {}", user_dir.display()))?;
    let token = random_token()?;
    write_atomic(&token_path, format!("{token}\n").as_bytes())?;
    Ok((token, true))
}

fn read_token(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(raw) => {
            let token = raw.trim().to_owned();
            if token.len() < MIN_TOKEN_LEN {
                bail!(
                    "{} holds a token shorter than {MIN_TOKEN_LEN} characters",
                    path.display()
                );
            }
            Ok(Some(token))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
    }
}

/// 32 random bytes as 64 lowercase hex characters.
pub fn random_token() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| anyhow::anyhow!("generating a token: {e}"))?;
    Ok(hex(&bytes))
}

/// Compare without stopping at the first differing byte, so response timing
/// says nothing about how much of a guessed token was right.
fn tokens_match(given: &str, expected: &str) -> bool {
    given.len() == expected.len()
        && given
            .bytes()
            .zip(expected.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

/// The progress database a bearer token unlocks, if any.
fn resolve_progress(accounts: &Accounts, authorization: Option<&str>) -> Result<Option<PathBuf>> {
    let Some(given) = authorization.and_then(|value| value.strip_prefix("Bearer ")) else {
        return Ok(None);
    };
    let given = given.trim();
    match accounts {
        Accounts::Single { progress, token } => {
            Ok(tokens_match(given, token).then(|| progress.clone()))
        }
        Accounts::Directory(dir) => {
            let mut found = None;
            // Check every learner rather than returning early, for the same
            // reason the comparison itself does not short-circuit.
            for user in list_users(dir)? {
                let user_dir = dir.join(&user);
                if let Some(token) = read_token(&user_dir.join("token"))?
                    && tokens_match(given, &token)
                {
                    found = Some(user_dir.join("progress.db"));
                }
            }
            Ok(found)
        }
    }
}

fn handle<S: Read + Write>(stream: &mut S, accounts: &Accounts) -> Result<()> {
    let mut reader = BufReader::new(stream);
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
            return respond(reader.get_mut(), "400 Bad Request", b"malformed header");
        };
        match name.to_ascii_lowercase().as_str() {
            "authorization" => authorization = Some(value.trim().to_owned()),
            "content-length" => content_length = value.trim().parse::<usize>().ok(),
            _ => {}
        }
    }
    let Some(progress) = resolve_progress(accounts, authorization.as_deref())? else {
        return respond(reader.get_mut(), "401 Unauthorized", b"unauthorized");
    };
    let request = request_line.trim_end();
    if request == "GET /health HTTP/1.1" || request == "GET /health HTTP/1.0" {
        return respond(reader.get_mut(), "200 OK", b"ok");
    }
    if request == "GET /v1/progress HTTP/1.1" || request == "GET /v1/progress HTTP/1.0" {
        if !progress.exists() {
            return respond(reader.get_mut(), "404 Not Found", b"no progress synced yet");
        }
        let snapshot =
            fs::read(&progress).with_context(|| format!("reading {}", progress.display()))?;
        return respond_with_snapshot(reader.get_mut(), &snapshot);
    }
    if request != "POST /v1/progress HTTP/1.1" && request != "POST /v1/progress HTTP/1.0" {
        return respond(reader.get_mut(), "404 Not Found", b"not found");
    }
    let Some(length) = content_length else {
        return respond(
            reader.get_mut(),
            "411 Length Required",
            b"content-length required",
        );
    };
    if length > MAX_SNAPSHOT_BYTES {
        return respond(
            reader.get_mut(),
            "413 Payload Too Large",
            b"snapshot too large",
        );
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    if !haqor_core::progress_sync::is_sqlite_snapshot(&body) {
        return respond(
            reader.get_mut(),
            "400 Bad Request",
            b"expected a SQLite snapshot",
        );
    }
    if let Some(parent) = progress.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating sync directory {}", parent.display()))?;
    }
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let incoming =
        std::env::temp_dir().join(format!("haqor-sync-{}-{nonce}.db", std::process::id()));
    fs::write(&incoming, body)?;
    let result = haqor_core::progress_sync::merge_progress_files(&progress, &incoming)
        .context("merging received progress snapshot");
    let _ = fs::remove_file(&incoming);
    result?;
    let snapshot =
        fs::read(&progress).with_context(|| format!("reading {}", progress.display()))?;
    respond_with_snapshot(reader.get_mut(), &snapshot)
}

fn respond(stream: &mut impl Write, status: &str, body: &[u8]) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

fn respond_with_snapshot(stream: &mut impl Write, snapshot: &[u8]) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.sqlite3\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        snapshot.len()
    )?;
    stream.write_all(snapshot)?;
    stream.flush()?;
    Ok(())
}

/// Build the link the app opens to join sync, for example from a phone
/// camera scanning its QR code:
///
/// `haqor://join?user=alice&server=https://10.0.0.2:8788&server=https://sync-host.lan:8788&pin=…&token=…`
///
/// Servers are listed in the order the app should try them. `pin` is the
/// certificate fingerprint the app trusts for `https://` servers.
pub fn join_link(user: &str, servers: &[String], pin: Option<&str>, token: &str) -> String {
    let mut link = format!("haqor://join?user={}", encode_query_value(user));
    for server in servers {
        link.push_str("&server=");
        link.push_str(&encode_query_value(server));
    }
    if let Some(pin) = pin {
        link.push_str("&pin=");
        link.push_str(&encode_query_value(pin));
    }
    link.push_str("&token=");
    link.push_str(&encode_query_value(token));
    link
}

/// Percent-encode a query value, leaving `:` and `/` (both legal in a query)
/// readable so addresses stay short in the QR code.
fn encode_query_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~:/".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Render `text` as a QR code drawn with Unicode half blocks for a terminal.
pub fn terminal_qr(text: &str) -> Result<String> {
    let code = qrcode::QrCode::new(text.as_bytes()).context("encoding the join link as QR")?;
    Ok(code
        .render::<qrcode::render::unicode::Dense1x2>()
        .dark_color(qrcode::render::unicode::Dense1x2::Light)
        .light_color(qrcode::render::unicode::Dense1x2::Dark)
        .quiet_zone(true)
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let dir = std::env::temp_dir().join(format!(
            "haqor-sync-test-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn link_lists_servers_in_order_and_escapes_values() {
        let link = join_link(
            "alice",
            &[
                "https://10.0.0.2:8788".to_string(),
                "https://sync-host.lan:8788".to_string(),
            ],
            Some("00ff"),
            "ab&c#d",
        );
        assert_eq!(
            link,
            "haqor://join?user=alice&server=https://10.0.0.2:8788\
             &server=https://sync-host.lan:8788&pin=00ff&token=ab%26c%23d"
        );
    }

    #[test]
    fn user_names_are_restricted() {
        assert!(validate_user_name("alice").is_ok());
        assert!(validate_user_name("a-1_b").is_ok());
        for bad in ["", "Alice", "../x", "a b", "-x", &"a".repeat(33)] {
            assert!(validate_user_name(bad).is_err(), "{bad:?} accepted");
        }
    }

    #[test]
    fn each_token_unlocks_only_its_own_learner() {
        let dir = temp_dir("resolve");
        let (alice, created) = ensure_user(&dir, "alice").unwrap();
        assert!(created);
        let (again, created) = ensure_user(&dir, "alice").unwrap();
        assert!(!created);
        assert_eq!(again, alice);
        let (other, _) = ensure_user(&dir, "other").unwrap();
        assert_ne!(alice, other);
        assert_eq!(alice.len(), 64);

        let accounts = Accounts::Directory(dir.clone());
        let bearer = |t: &str| format!("Bearer {t}");
        assert_eq!(
            resolve_progress(&accounts, Some(&bearer(&alice))).unwrap(),
            Some(dir.join("alice/progress.db"))
        );
        assert_eq!(
            resolve_progress(&accounts, Some(&bearer(&other))).unwrap(),
            Some(dir.join("other/progress.db"))
        );
        assert_eq!(
            resolve_progress(&accounts, Some(&bearer("not-a-real-token-at-all"))).unwrap(),
            None
        );
        assert_eq!(resolve_progress(&accounts, None).unwrap(), None);
        assert_eq!(list_users(&dir).unwrap(), ["alice", "other"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn existing_token_file_is_kept() {
        let dir = temp_dir("existing");
        fs::create_dir_all(dir.join("alice")).unwrap();
        fs::write(dir.join("alice/token"), "0123456789abcdef0123\n").unwrap();
        let (token, created) = ensure_user(&dir, "alice").unwrap();
        assert!(!created);
        assert_eq!(token, "0123456789abcdef0123");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn tls_identity_is_created_once_and_reloaded() {
        let dir = temp_dir("tls");
        let first = TlsIdentity::load_or_create(&dir).unwrap();
        let second = TlsIdentity::load_or_create(&dir).unwrap();
        assert_eq!(first.fingerprint, second.fingerprint);
        assert_eq!(first.fingerprint.len(), 64);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn terminal_qr_renders() {
        let qr = terminal_qr("haqor://join?user=alice").unwrap();
        assert!(qr.lines().count() > 10);
    }
}
