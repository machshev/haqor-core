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

        /// Remote sync-server URL, for example https://10.0.0.2:8788; repeat it to
        /// try several addresses in order. Defaults
        /// to the Haqor app's saved sync-server setting.
        #[arg(long, conflicts_with = "progress")]
        server: Vec<String>,

        /// Certificate SHA-256 fingerprint for https:// servers. Defaults to the
        /// Haqor app's saved fingerprint.
        #[arg(long)]
        pin: Option<String>,

        /// Sync token used with --server. Defaults to the Haqor app's saved token.
        #[arg(long)]
        token: Option<String>,

        /// Overlay JSON file to update atomically.
        #[arg(long, default_value = "data/lexicon_overrides.json")]
        overlay: PathBuf,
    },
    /// Remove malformed blank tutor-gloss rows from the canonical sync server.
    ClearInvalidGlosses {
        /// Remote sync-server URL; repeat it to try several addresses in order.
        /// Defaults to the Haqor app's saved setting.
        #[arg(long)]
        server: Vec<String>,

        /// Certificate SHA-256 fingerprint for https:// servers. Defaults to the
        /// Haqor app's saved fingerprint.
        #[arg(long)]
        pin: Option<String>,

        /// Sync token used with --server. Defaults to the Haqor app's saved token.
        #[arg(long)]
        token: Option<String>,
    },
    /// Download mobile bug reports and ideas from the synchronised progress data.
    PullIssues {
        /// Canonical learner-progress database held by haqor-sync-server. This is
        /// used instead of the app's saved sync-server settings.
        #[arg(long, conflicts_with = "server")]
        progress: Option<PathBuf>,

        /// Remote sync-server URL; repeat it to try several addresses in order.
        /// Defaults to the Haqor app's saved setting.
        #[arg(long, conflicts_with = "progress")]
        server: Vec<String>,

        /// Certificate SHA-256 fingerprint for https:// servers. Defaults to the
        /// Haqor app's saved fingerprint.
        #[arg(long)]
        pin: Option<String>,

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

        /// Remote sync-server URL; repeat it to try several addresses in order.
        /// Defaults to the Haqor app's saved setting.
        #[arg(long, conflicts_with = "progress")]
        server: Vec<String>,

        /// Certificate SHA-256 fingerprint for https:// servers. Defaults to the
        /// Haqor app's saved fingerprint.
        #[arg(long)]
        pin: Option<String>,

        /// Sync token used with --server. Defaults to the Haqor app's saved token.
        #[arg(long)]
        token: Option<String>,

        /// JSON file to replace when pulling the current issue log from the TUI.
        #[arg(long, default_value = "data/issue_reports.json")]
        output: PathBuf,
    },
}

fn remote_settings(
    server: Vec<String>,
    pin: Option<String>,
    token: Option<String>,
) -> Result<haqor_admin::Remote> {
    if let (false, Some(token)) = (server.is_empty(), &token) {
        return Ok(haqor_admin::Remote {
            servers: server,
            pin,
            token: token.clone(),
        });
    }
    let saved = haqor_admin::read_default_app_sync_settings()?;
    Ok(haqor_admin::Remote {
        servers: if server.is_empty() {
            saved.servers
        } else {
            server
        },
        pin: pin.or(saved.pin),
        token: token.unwrap_or(saved.token),
    })
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
            pin,
            token,
            overlay,
        } => {
            let result = if let Some(progress) = progress {
                haqor_admin::pull_gloss_overrides(&progress, &overlay)?
            } else {
                let remote = remote_settings(server, pin, token)?;
                haqor_admin::pull_gloss_overrides_from_server(&remote, &overlay)?
            };
            println!(
                "Merged {} mobile lexicon correction(s) into {} ({} new)",
                result.total,
                overlay.display(),
                result.new
            );
            Ok(())
        }
        Command::ClearInvalidGlosses { server, pin, token } => {
            let remote = remote_settings(server, pin, token)?;
            let count = haqor_admin::clear_invalid_gloss_overrides_from_server(&remote)?;
            println!("Cleared {count} invalid synced tutor gloss override(s).");
            Ok(())
        }
        Command::PullIssues {
            progress,
            server,
            pin,
            token,
            output,
        } => {
            let count = if let Some(progress) = progress {
                haqor_admin::pull_issue_reports(&progress, &output)?
            } else {
                let remote = remote_settings(server, pin, token)?;
                haqor_admin::pull_issue_reports_from_server(&remote, &output)?
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
            pin,
            token,
            output,
        } => {
            let count = if let Some(progress) = progress {
                haqor_admin::review_issue_reports(&progress, &output)?
            } else {
                let remote = remote_settings(server, pin, token)?;
                haqor_admin::review_issue_reports_from_server(&remote, &output)?
            };
            println!("Resolved {count} app issue report(s).");
            Ok(())
        }
    }
}
