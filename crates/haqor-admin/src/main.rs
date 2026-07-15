use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// Serve the loopback-only editor for Haqor's lexical overlays.
#[derive(Debug, Parser)]
#[command(name = "haqor-admin", version, about, arg_required_else_help = true)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Serve the loopback-only browser editor for manual lexical overlays.
    Server {
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
    },
    /// Merge mobile tutor and word-info corrections into the overlay JSON.
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
    /// Download mobile bug reports and ideas from the synchronised progress data.
    PullIssues {
        /// Canonical learner-progress database held by haqor-sync-server. This is
        /// used instead of the app's saved sync-server settings.
        #[arg(long, conflicts_with = "server")]
        progress: Option<PathBuf>,

        /// Remote sync-server URL. Defaults to the Haqor app's saved setting.
        #[arg(long, conflicts_with = "progress")]
        server: Option<String>,

        /// Sync token used with --server. Defaults to the Haqor app's saved token.
        #[arg(long)]
        token: Option<String>,

        /// JSON file to replace atomically with the downloaded issue log.
        #[arg(long, default_value = "data/issue_reports.json")]
        output: PathBuf,
    },
    /// Review issue reports in a terminal UI and resolve selected entries.
    ReviewIssues {
        /// Canonical learner-progress database to edit directly.
        #[arg(long, conflicts_with = "server")]
        progress: Option<PathBuf>,

        /// Remote sync-server URL. Defaults to the Haqor app's saved setting.
        #[arg(long, conflicts_with = "progress")]
        server: Option<String>,

        /// Sync token used with --server. Defaults to the Haqor app's saved token.
        #[arg(long)]
        token: Option<String>,

        /// JSON file to replace when pulling the current issue log from the TUI.
        #[arg(long, default_value = "data/issue_reports.json")]
        output: PathBuf,
    },
}

fn remote_settings(server: Option<String>, token: Option<String>) -> Result<(String, String)> {
    let saved = if server.is_some() && token.is_some() {
        None
    } else {
        Some(haqor_admin::read_default_app_sync_settings()?)
    };
    let server = server
        .or_else(|| saved.as_ref().map(|settings| settings.server_url.clone()))
        .expect("saved settings provide a server URL");
    let token = token
        .or_else(|| saved.map(|settings| settings.token))
        .expect("saved settings provide a token");
    Ok((server, token))
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Server {
            bind,
            overlay,
            lexicon,
            hebrew,
        } => haqor_admin::serve(bind, overlay, lexicon, hebrew),
        Command::Pull {
            progress,
            server,
            token,
            overlay,
        } => {
            let count = if let Some(progress) = progress {
                haqor_admin::pull_gloss_overrides(&progress, &overlay)?
            } else {
                let (server, token) = remote_settings(server, token)?;
                haqor_admin::pull_gloss_overrides_from_server(&server, &token, &overlay)?
            };
            println!(
                "Merged {count} mobile lexicon correction(s) into {}",
                overlay.display()
            );
            Ok(())
        }
        Command::PullIssues {
            progress,
            server,
            token,
            output,
        } => {
            let count = if let Some(progress) = progress {
                haqor_admin::pull_issue_reports(&progress, &output)?
            } else {
                let (server, token) = remote_settings(server, token)?;
                haqor_admin::pull_issue_reports_from_server(&server, &token, &output)?
            };
            println!(
                "Downloaded {count} app issue report(s) to {}",
                output.display()
            );
            Ok(())
        }
        Command::ReviewIssues {
            progress,
            server,
            token,
            output,
        } => {
            let count = if let Some(progress) = progress {
                haqor_admin::review_issue_reports(&progress, &output)?
            } else {
                let (server, token) = remote_settings(server, token)?;
                haqor_admin::review_issue_reports_from_server(&server, &token, &output)?
            };
            println!("Resolved {count} app issue report(s).");
            Ok(())
        }
    }
}
