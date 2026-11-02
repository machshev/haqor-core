//! Run the real server on loopback and talk to it through the pinned client.

use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use haqor_sync_server::client::Remote;
use haqor_sync_server::{Accounts, ServeOptions, TlsIdentity, ensure_user};

fn temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("haqor-sync-it-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[test]
fn pinned_https_plain_fallback_and_wrong_pin() {
    let dir = temp_dir();
    let users = dir.join("users");
    let (token, _) = ensure_user(&users, "alice").unwrap();
    let tls = TlsIdentity::load_or_create(&dir.join("tls")).unwrap();
    let pin = tls.fingerprint.clone();
    let port = free_port();
    std::thread::spawn(move || {
        haqor_sync_server::serve(ServeOptions {
            bind: format!("127.0.0.1:{port}").parse().unwrap(),
            accounts: Accounts::Directory(users),
            tls: Some(tls),
            allow_plain_http: true,
        })
    });
    std::thread::sleep(Duration::from_millis(300));

    let dead = free_port();
    let remote = |servers: Vec<String>, pin: Option<String>, token: &str| Remote {
        servers,
        pin,
        token: token.to_string(),
    };
    let health = |p: u16, scheme: &str| format!("{scheme}://127.0.0.1:{p}/health");

    // The first address is down, so the second (HTTPS, pinned) answers.
    let (body, used) = remote(
        vec![health(dead, "https"), health(port, "https")],
        Some(pin.to_uppercase()),
        &token,
    )
    .request("GET", None)
    .unwrap();
    assert_eq!(body, b"ok");
    assert_eq!(used, format!("https://127.0.0.1:{port}"));

    // Plain HTTP is still accepted on the same port during migration.
    let (body, _) = remote(vec![health(port, "http")], None, &token)
        .request("GET", None)
        .unwrap();
    assert_eq!(body, b"ok");

    // A certificate that does not match the pin is refused before the token
    // is sent, and counts as unreachable so the next address would be tried.
    let error = remote(vec![health(port, "https")], Some("00".repeat(32)), &token)
        .request("GET", None)
        .unwrap_err();
    assert!(error.unreachable, "{error}");
    assert!(error.message.contains("fingerprint"), "{error}");

    // A wrong token reaches the server and is rejected there.
    let error = remote(
        vec![health(port, "https")],
        Some(pin),
        "wrong-token-wrong-token",
    )
    .request("GET", None)
    .unwrap_err();
    assert!(!error.unreachable);
    assert!(error.message.contains("401"), "{error}");
    let _ = std::fs::remove_dir_all(&dir);
}
