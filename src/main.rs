//! # Haqor
//!
//! `haqor-core` is a cli app that provides convenient access to the
//! functionality in `haqor-core` library. At the moment this is mostly used
//! for testing during development although this may expand to become a fully
//! fledged CLI based bible app.

use anyhow::Result;
use clap::{Parser, Subcommand};
use haqor_core::bible::Bible;
use log::info;
use std::env;

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
    /// Get bible verse
    Get { book: u8, chapter: u8, verse: u8 },
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

    match cli.command {
        Commands::Get {
            book,
            chapter,
            verse,
        } => {
            info!("Bible reference:{} {}:{}", book, chapter, verse);

            let bible: Bible = Bible::default();

            println!("{}", bible.get(book, chapter, verse)?)
        }
    }
    Ok(())
}
