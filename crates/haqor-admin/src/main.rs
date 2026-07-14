use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Serve the loopback-only editor for Haqor's lexical overlays.
#[derive(Debug, Parser)]
#[command(name = "haqor-admin", version, about)]
struct Args {
    /// Loopback address for the editor. Non-loopback addresses are rejected.
    #[arg(long, default_value = "127.0.0.1:8787")]
    bind: SocketAddr,

    /// Overlay JSON file to edit.
    #[arg(long, default_value = "data/lexicon_overrides.json")]
    overlay: PathBuf,

    /// Generated lexicon database whose imported glosses can be browsed.
    #[arg(long, default_value = "data/lexicon.db")]
    lexicon: PathBuf,

    /// Generated Hebrew database whose ambiguous analyses can be reviewed.
    #[arg(long, default_value = "data/hebrew.db")]
    hebrew: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    haqor_admin::serve(args.bind, args.overlay, args.lexicon, args.hebrew)
}
