//! # Haqor
//!
//! `haqor-core` is a cli app that provides convenient access to the
//! functionality in `haqor-core` library. At the moment this is mostly used
//! for testing during development although this may expand to become a fully
//! fledged CLI based bible app.

use anyhow::Result;
use clap::{Parser, Subcommand};
use log::info;
use std::env;

use haqor_core::{bible::Bible, library::Library, repo::ResourceRepo};

/// Summarise bible resource
#[derive(Parser, Debug)]
#[command(name = "haqor")]
#[command(author = "James McCorrie <djmccorrie@gmail.com>")]
#[command(version = "0.1")]
#[command(
    about = "CLI for haqor",
    long_about = "This tool is mostly for testing purposes. It allows basic
    operations with the backend rust based engine. It's not expected to have
    utility beyond that at this stage."
)]
struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Fetch bible from remote repository
    #[command(arg_required_else_help = true)]
    Fetch {
        /// Name of the bible to download
        name: String,
    },
    /// List bibles in library
    List {},
    /// Show bible description
    Show { name: String },
    /// Read bible verse
    Read { name: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // If $RUST_LOG is not explicitly set, then use the number of -v flags to
    // determine the log level defaulting to Errors only.
    if env::var("RUST_LOG").is_err() {
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe {
            env::set_var(
                "RUST_LOG",
                match cli.verbose {
                    0 => "Error",
                    1 => "Info",
                    2 => "Debug",
                    _ => "Trace",
                },
            )
        };
    }
    env_logger::init();

    let library = Library::default();

    match cli.command {
        Commands::Fetch { name } => {
            // TODO: These operations should probably be wrapped in a higher level
            // library.download(name: &str, repo: ResourceRepo)
            let repo = ResourceRepo::default();
            let bible_data = repo.fetch_bible(&name).unwrap();

            library.save_bible(&name, &bible_data)?;
        }
        Commands::List {} => {
            let bibles = library.list_bibles()?;

            print!("{:?}", bibles);
        }
        Commands::Show { name } => {
            let bible: Bible = library.get_bible(&name);

            print!("Bible loaded:\n{}", bible);
        }
        Commands::Read { name } => {
            let bible: Bible = library.get_bible(&name);

            info!("Bible loaded:\n{}", bible);
        }
    }
    Ok(())
}
