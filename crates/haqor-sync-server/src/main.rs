use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};
use haqor_sync_server::{Accounts, ServeOptions, TlsIdentity};

/// Serve and merge bearer-token protected learner progress on a trusted network.
#[derive(Parser, Debug)]
#[command(name = "haqor-sync-server")]
#[command(version, args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    serve: ServeArgs,
}

#[derive(Args, Debug)]
struct ServeArgs {
    /// Address to listen on. Use 0.0.0.0 to accept devices on the network.
    #[arg(long, default_value = "0.0.0.0:8788")]
    bind: SocketAddr,
    /// Single-learner mode: the canonical progress database.
    #[arg(
        long,
        default_value = "data/sync-progress.db",
        conflicts_with = "users_dir"
    )]
    progress: PathBuf,
    /// Single-learner mode: secret shared with the app (16+ characters).
    #[arg(
        long,
        conflicts_with = "users_dir",
        required_unless_present = "users_dir"
    )]
    token: Option<String>,
    /// Multi-learner mode: one `<user>/token` and `<user>/progress.db` per
    /// learner. Add learners with `haqor-sync-server join`.
    #[arg(long)]
    users_dir: Option<PathBuf>,
    /// Serve HTTPS with the self-signed certificate kept here (created on
    /// first start). Without it the server speaks plain HTTP only.
    #[arg(long)]
    tls_dir: Option<PathBuf>,
    /// With --tls-dir, also accept plain HTTP on the same port so apps joined
    /// before HTTPS keep syncing until they rejoin.
    #[arg(long, requires = "tls_dir")]
    allow_plain_http: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Print the join link and QR code for a learner, creating the learner
    /// (with a new token) if needed. Scan the QR code with a phone camera.
    Join {
        /// Learner name, for example `alice`.
        user: String,
        /// Directory holding the learners, as passed to the server.
        #[arg(long)]
        users_dir: PathBuf,
        /// The server's TLS directory, so the link can carry its fingerprint.
        #[arg(long)]
        tls_dir: Option<PathBuf>,
        /// Address the app should try, for example https://10.0.0.2:8788.
        /// Repeat it; the app tries them in this order.
        #[arg(long = "server", required = true)]
        servers: Vec<String>,
        /// Print only the link, without the QR code.
        #[arg(long)]
        no_qr: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Join {
            user,
            users_dir,
            tls_dir,
            servers,
            no_qr,
        }) => join(&user, &users_dir, tls_dir, &servers, no_qr),
        None => {
            let args = cli.serve;
            let accounts = match (args.users_dir, args.token) {
                (Some(dir), _) => Accounts::Directory(dir),
                (None, Some(token)) => Accounts::Single {
                    progress: args.progress,
                    token,
                },
                (None, None) => bail!("pass --users-dir, or --token for a single learner"),
            };
            let tls = args
                .tls_dir
                .as_deref()
                .map(TlsIdentity::load_or_create)
                .transpose()?;
            haqor_sync_server::serve(ServeOptions {
                bind: args.bind,
                accounts,
                allow_plain_http: tls.is_none() || args.allow_plain_http,
                tls,
            })
        }
    }
}

fn join(
    user: &str,
    users_dir: &std::path::Path,
    tls_dir: Option<PathBuf>,
    servers: &[String],
    no_qr: bool,
) -> Result<()> {
    for server in servers {
        if !server.starts_with("https://") && !server.starts_with("http://") {
            bail!("server {server:?} must start with https:// or http://");
        }
    }
    let wants_tls = servers.iter().any(|s| s.starts_with("https://"));
    let pin = match (&tls_dir, wants_tls) {
        (Some(dir), _) => Some(TlsIdentity::load_or_create(dir)?.fingerprint),
        (None, true) => {
            bail!("https:// servers need --tls-dir so the link can pin the certificate")
        }
        (None, false) => None,
    };
    std::fs::create_dir_all(users_dir)?;
    let (token, created) = haqor_sync_server::ensure_user(users_dir, user)?;
    if created {
        eprintln!("Created learner {user} with a new token.");
    }
    let link = haqor_sync_server::join_link(user, servers, pin.as_deref(), &token);
    if !no_qr {
        println!("{}", haqor_sync_server::terminal_qr(&link)?);
        println!("Scan with the phone camera to open it in Haqor, or paste the link into");
        println!("Haqor's sync settings. It holds {user}'s token: share it only with {user}.\n");
    }
    println!("{link}");
    Ok(())
}
