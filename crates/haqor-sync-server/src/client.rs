//! A minimal client for the sync server, shared by command-line tools.
//!
//! `https://` servers are trusted by certificate fingerprint ("pin") instead of
//! a certificate authority: the server's self-signed certificate is accepted
//! only when its SHA-256 matches the pin from the join link, and the TLS
//! handshake signature is still verified, so the server must hold the key.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};

const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;

/// One parsed server address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub tls: bool,
    pub host: String,
    pub port: u16,
    pub path: String,
}

impl Endpoint {
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();
        let (tls, rest) = if let Some(rest) = input.strip_prefix("https://") {
            (true, rest)
        } else if let Some(rest) = input.strip_prefix("http://") {
            (false, rest)
        } else {
            return Err(format!(
                "sync server {input:?} must start with https:// or http://"
            ));
        };
        let (authority, path) = match rest.find('/') {
            Some(index) if &rest[index..] == "/" => (&rest[..index], "/v1/progress"),
            Some(index) => (&rest[..index], &rest[index..]),
            None => (rest, "/v1/progress"),
        };
        if authority.is_empty() || authority.contains('@') {
            return Err(format!("sync server address {input:?} is invalid"));
        }
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) if !host.is_empty() => (
                host.to_string(),
                port.parse::<u16>()
                    .map_err(|_| format!("sync server port in {input:?} is invalid"))?,
            ),
            _ => (authority.to_string(), if tls { 443 } else { 80 }),
        };
        Ok(Self {
            tls,
            host,
            port,
            path: path.to_string(),
        })
    }

    pub fn display(&self) -> String {
        format!(
            "{}://{}:{}",
            if self.tls { "https" } else { "http" },
            self.host,
            self.port
        )
    }
}

/// Normalise a fingerprint as written in a join link or typed by hand:
/// lowercase hex, with any `:` separators removed.
pub fn normalise_pin(pin: &str) -> Result<String, String> {
    let pin: String = pin
        .trim()
        .chars()
        .filter(|c| *c != ':')
        .collect::<String>()
        .to_ascii_lowercase();
    if pin.len() != 64 || !pin.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("certificate fingerprint must be 64 hex characters (SHA-256)".to_string());
    }
    Ok(pin)
}

#[derive(Debug)]
struct PinnedCertificate {
    fingerprint: String,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for PinnedCertificate {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if crate::cert_fingerprint(end_entity) == self.fingerprint {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "server certificate does not match the sync join link's fingerprint".into(),
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

trait Stream: Read + Write {}
impl<T: Read + Write> Stream for T {}

fn connect(
    endpoint: &Endpoint,
    pin: Option<&str>,
    connect_timeout: Duration,
) -> Result<Box<dyn Stream>, String> {
    let address = format!("{}:{}", endpoint.host, endpoint.port);
    let socket = address
        .to_socket_addrs()
        .map_err(|e| format!("could not resolve {}: {e}", endpoint.host))?
        .next()
        .ok_or_else(|| format!("could not resolve {}", endpoint.host))?;
    let stream = TcpStream::connect_timeout(&socket, connect_timeout)
        .map_err(|e| format!("could not reach {}: {e}", endpoint.display()))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    if !endpoint.tls {
        return Ok(Box::new(stream));
    }
    let pin = pin.ok_or_else(|| {
        format!(
            "{} needs the certificate fingerprint from the sync join link",
            endpoint.display()
        )
    })?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("configuring TLS: {e}"))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(PinnedCertificate {
            fingerprint: normalise_pin(pin)?,
            provider,
        }))
        .with_no_client_auth();
    let name = ServerName::try_from(endpoint.host.clone())
        .map_err(|e| format!("invalid sync server name {}: {e}", endpoint.host))?;
    let connection = rustls::ClientConnection::new(Arc::new(config), name)
        .map_err(|e| format!("starting TLS with {}: {e}", endpoint.display()))?;
    Ok(Box::new(rustls::StreamOwned::new(connection, stream)))
}

/// Where and how to reach one learner's progress on the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    /// Addresses tried in order until one answers.
    pub servers: Vec<String>,
    /// Certificate fingerprint for `https://` servers.
    pub pin: Option<String>,
    pub token: String,
}

/// The outcome of one request when it failed.
#[derive(Debug)]
pub struct RequestError {
    /// True when no server answered at all, so trying another address or
    /// trying later may help. False when a server answered with an error.
    pub unreachable: bool,
    pub message: String,
}

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RequestError {}

impl Remote {
    /// GET or POST the progress route, returning the response body and the
    /// address that answered. Addresses are tried in order; the first one that
    /// gives any HTTP response is used, even an error response. A POST resent
    /// to a second address is harmless because merging is idempotent.
    pub fn request(
        &self,
        method: &str,
        body: Option<&[u8]>,
    ) -> Result<(Vec<u8>, String), RequestError> {
        if self.token.trim().is_empty() {
            return Err(RequestError {
                unreachable: false,
                message: "sync token is empty".to_string(),
            });
        }
        let endpoints = self
            .servers
            .iter()
            .map(|s| Endpoint::parse(s))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|message| RequestError {
                unreachable: false,
                message,
            })?;
        if endpoints.is_empty() {
            return Err(RequestError {
                unreachable: false,
                message: "no sync server address is configured".to_string(),
            });
        }
        let mut failures = Vec::new();
        for endpoint in &endpoints {
            match request_one(endpoint, self.pin.as_deref(), &self.token, method, body) {
                Ok(Ok(response)) => return Ok((response, endpoint.display())),
                Ok(Err(status)) => {
                    return Err(RequestError {
                        unreachable: false,
                        message: format!("sync server {} returned {status}", endpoint.display()),
                    });
                }
                Err(message) => failures.push(message),
            }
        }
        Err(RequestError {
            unreachable: true,
            message: failures.join("; "),
        })
    }
}

/// `Err` when no HTTP response arrived; `Ok(Err(status))` for a non-200 one.
#[allow(clippy::type_complexity)]
fn request_one(
    endpoint: &Endpoint,
    pin: Option<&str>,
    token: &str,
    method: &str,
    body: Option<&[u8]>,
) -> Result<Result<Vec<u8>, String>, String> {
    if let Some(body) = body
        && body.len() > MAX_SNAPSHOT_BYTES
    {
        return Err("local progress snapshot is unexpectedly large".to_string());
    }
    let mut stream = connect(endpoint, pin, Duration::from_secs(4))?;
    let send = |stream: &mut Box<dyn Stream>| -> std::io::Result<()> {
        match body {
            Some(body) => {
                write!(
                    stream,
                    "{method} {} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/vnd.sqlite3\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    endpoint.path,
                    endpoint.host,
                    body.len()
                )?;
                stream.write_all(body)?;
            }
            None => write!(
                stream,
                "{method} {} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n",
                endpoint.path, endpoint.host
            )?,
        }
        stream.flush()
    };
    send(&mut stream).map_err(|e| format!("{}: {e}", endpoint.display()))?;
    let mut reader = BufReader::new(stream);
    let mut status = String::new();
    reader
        .read_line(&mut status)
        .map_err(|e| format!("{}: {e}", endpoint.display()))?;
    if status.is_empty() {
        return Err(format!("{} closed the connection", endpoint.display()));
    }
    let status = status.trim().to_string();
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|e| format!("reading the sync response: {e}"))?;
        if read == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    if !status.starts_with("HTTP/1.1 200") && !status.starts_with("HTTP/1.0 200") {
        return Ok(Err(status));
    }
    let length =
        content_length.ok_or_else(|| "sync server omitted its response length".to_string())?;
    if length > MAX_SNAPSHOT_BYTES {
        return Ok(Err("an unexpectedly large snapshot".to_string()));
    }
    let mut response = vec![0; length];
    reader
        .read_exact(&mut response)
        .map_err(|e| format!("reading the progress snapshot: {e}"))?;
    Ok(Ok(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_parse_scheme_port_and_default_path() {
        assert_eq!(
            Endpoint::parse("https://10.0.0.2:8788").unwrap(),
            Endpoint {
                tls: true,
                host: "10.0.0.2".into(),
                port: 8788,
                path: "/v1/progress".into()
            }
        );
        let plain = Endpoint::parse(" http://sync.example:8788/ ").unwrap();
        assert!(!plain.tls);
        assert_eq!(plain.path, "/v1/progress");
        assert_eq!(Endpoint::parse("https://sync-host.lan").unwrap().port, 443);
        assert!(Endpoint::parse("ftp://x").is_err());
        assert!(Endpoint::parse("http://user@x").is_err());
    }

    #[test]
    fn pins_are_normalised() {
        let hex = "AB".repeat(32);
        assert_eq!(normalise_pin(&hex).unwrap(), "ab".repeat(32));
        let colons = vec!["ab"; 32].join(":");
        assert_eq!(normalise_pin(&colons).unwrap(), "ab".repeat(32));
        assert!(normalise_pin("abc").is_err());
    }
}
