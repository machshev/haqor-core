use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// Serve the loopback-only editor for Haqor's lexical overlays.
#[derive(Debug, Parser)]
#[command(name = "haqor-admin", version, about)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

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

#[derive(Debug, Subcommand)]
enum Command {
    /// Merge mobile tutor gloss corrections from the sync database into the overlay JSON.
    Pull {
        /// Canonical learner-progress database held by haqor-sync-server. This is
        /// used instead of the app's saved sync-server settings.
        #[arg(long, conflicts_with = "server")]
        progress: Option<PathBuf>,

        /// Remote sync-server URL, for example http://192.168.1.10:8788. Defaults
        /// to the Haqor app's saved sync-server setting.
        #[arg(long, conflicts_with = "progress")]
        server: Option<String>,

        /// Sync token used with --server. Defaults to the Haqor app's saved token.
        #[arg(long)]
        token: Option<String>,

        /// Overlay JSON file to update atomically.
        #[arg(long, default_value = "data/lexicon_overrides.json")]
        overlay: PathBuf,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Some(Command::Pull {
            progress,
            server,
            token,
            overlay,
        }) => {
            let count = if let Some(progress) = progress {
                haqor_admin::pull_gloss_overrides(&progress, &overlay)?
            } else {
                let saved = if server.is_some() && token.is_some() {
                    None
                } else {
                    Some(haqor_admin::read_app_sync_settings(
                        haqor_admin::DEFAULT_APP_SHARED_PREFERENCES,
                    )?)
                };
                let server = server
                    .or_else(|| saved.as_ref().map(|settings| settings.server_url.clone()))
                    .expect("saved settings provide a server URL");
                let token = token
                    .or_else(|| saved.map(|settings| settings.token))
                    .expect("saved settings provide a token");
                haqor_admin::pull_gloss_overrides_from_server(&server, &token, &overlay)?
            };
            println!(
                "Merged {count} tutor gloss correction(s) into {}",
                overlay.display()
            );
            Ok(())
        }
        None => haqor_admin::serve(args.bind, args.overlay, args.lexicon, args.hebrew),
    }
}
