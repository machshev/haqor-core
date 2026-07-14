use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Serve and merge bearer-token protected learner progress on a trusted LAN.
#[derive(Parser, Debug)]
#[command(name = "haqor-sync-server")]
#[command(version)]
struct Cli {
    /// LAN address to listen on. Use 0.0.0.0 to accept devices on the LAN.
    #[arg(long, default_value = "0.0.0.0:8788")]
    bind: SocketAddr,
    /// Canonical learner-progress database held by this server.
    #[arg(long, default_value = "data/sync-progress.db")]
    progress: PathBuf,
    /// Secret shared with the app. Must be at least 16 characters.
    #[arg(long)]
    token: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    haqor_sync_server::serve_progress(cli.bind, &cli.progress, &cli.token)
}
