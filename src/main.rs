//! # Haqor
//!
//! `haqor-core` is a cli app that provides convenient access to the
//! functionality in `haqor-core` library. At the moment this is mostly used
//! for testing during development although this may expand to become a fully
//! fledged CLI based bible app.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use haqor_core::bible::Bible;
use haqor_core::morphology;
use log::info;
use std::env;
use std::path::PathBuf;

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
    /// Database management
    Db {
        #[command(subcommand)]
        command: DbCommands,
    },
    /// Generate the verb paradigm of a 3-letter Hebrew root
    Morph {
        /// 3-letter Hebrew root (e.g. קטל). Niqqud is ignored; final-form
        /// letters are normalised back to their base forms.
        root: String,
        /// Limit output to a specific binyan (Qal, Niphal, Piel, Pual,
        /// Hithpael, Hiphil, Hophal)
        #[arg(short, long)]
        binyan: Option<String>,
    },
    /// Inflect a Hebrew noun stem (singular absolute) across state, number,
    /// and pronominal suffixes
    Noun {
        /// Singular absolute form, fully pointed (e.g. דָּבָר)
        stem: String,
        /// Stem class: "m" (masculine, default), "f" (feminine -ה), or "ft"
        /// (feminine -ת)
        #[arg(short, long, default_value = "m")]
        kind: String,
    },
}

#[derive(Subcommand, Debug)]
enum DbCommands {
    /// Copy haqor.db from bible-modules output to data/haqor.db
    Update {
        /// Source path (defaults to ../bible-modules/modules/haqor/haqor.db)
        #[arg(short, long)]
        source: Option<PathBuf>,
    },
    /// Generate the `bible` table (OT UXLC + NT SEDRA transliterated) into a
    /// standalone SQLite database from the checked-in source texts.
    GenBible {
        /// Source texts directory (defaults to src_texts/)
        #[arg(short, long, default_value = "src_texts")]
        src_texts: PathBuf,
        /// Output database path
        #[arg(short, long, default_value = "data/bible.db")]
        output: PathBuf,
    },
    /// Generate the SEDRA tables (roots, lexemes, words, english) mirroring the
    /// SEDRA source files losslessly, with transliteration columns rendered into
    /// Hebrew Unicode.
    GenSedra {
        /// Source texts directory (defaults to src_texts/)
        #[arg(short, long, default_value = "src_texts")]
        src_texts: PathBuf,
        /// Output database path
        #[arg(short, long, default_value = "data/sedra.db")]
        output: PathBuf,
    },
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
        Commands::Morph { root, binyan } => {
            print_morphology(&root, binyan.as_deref())?;
        }
        Commands::Noun { stem, kind } => {
            print_noun(&stem, &kind)?;
        }
        Commands::Db { command } => match command {
            DbCommands::Update { source } => {
                let src = source.unwrap_or_else(|| {
                    PathBuf::from("../bible-modules/modules/haqor/haqor.db")
                });
                let dst = PathBuf::from("data/haqor.db");

                info!("Copying {} -> {}", src.display(), dst.display());

                let bytes = std::fs::copy(&src, &dst).with_context(|| {
                    format!("Failed to copy {} to {}", src.display(), dst.display())
                })?;

                println!(
                    "Updated {} ({} bytes)",
                    dst.display(),
                    bytes
                );
            }
            DbCommands::GenBible { src_texts, output } => {
                let total = haqor_core::generate::generate_bible(&src_texts, &output)?;
                println!("Wrote {} rows to {}", total, output.display());
            }
            DbCommands::GenSedra { src_texts, output } => {
                let total = haqor_core::generate::generate_sedra(&src_texts, &output)?;
                println!("Wrote {} rows to {}", total, output.display());
            }
        },
    }
    Ok(())
}

fn parse_binyan(s: &str) -> Option<morphology::Binyan> {
    match s.to_ascii_lowercase().as_str() {
        "qal" | "q" => Some(morphology::Binyan::Qal),
        "niphal" | "nifal" | "n" => Some(morphology::Binyan::Niphal),
        "piel" | "p" => Some(morphology::Binyan::Piel),
        "pual" | "pu" => Some(morphology::Binyan::Pual),
        "hithpael" | "hitpael" | "ht" => Some(morphology::Binyan::Hithpael),
        "hiphil" | "hifil" | "h" => Some(morphology::Binyan::Hiphil),
        "hophal" | "hofal" | "ho" => Some(morphology::Binyan::Hophal),
        _ => None,
    }
}

fn print_morphology(root_input: &str, binyan_filter: Option<&str>) -> Result<()> {
    let root = morphology::Root::parse(root_input)
        .with_context(|| format!("could not parse root '{root_input}'"))?;

    let filter = match binyan_filter {
        Some(b) => Some(
            parse_binyan(b)
                .with_context(|| format!("unknown binyan '{b}'"))?,
        ),
        None => None,
    };

    println!("Root: {}", root_input);
    print!("Gizra:");
    for g in &root.classes {
        print!(" {:?}", g);
    }
    println!();
    println!();

    let paradigm = morphology::generate_paradigm(&root);

    for &binyan in &morphology::Binyan::ALL {
        if let Some(only) = filter {
            if binyan != only {
                continue;
            }
        }
        let any = paradigm.forms.iter().any(|f| f.binyan == binyan);
        if !any {
            continue;
        }
        println!("================ {} ================", binyan.name());
        let forms_in_order = [
            morphology::Form::Perfect,
            morphology::Form::Imperfect,
            morphology::Form::Imperative,
            morphology::Form::Cohortative,
            morphology::Form::Jussive,
            morphology::Form::InfinitiveConstruct,
            morphology::Form::InfinitiveAbsolute,
            morphology::Form::ParticipleActive,
            morphology::Form::ParticiplePassive,
        ];
        for form in forms_in_order {
            let entries: Vec<&morphology::VerbForm> = paradigm
                .forms
                .iter()
                .filter(|f| f.binyan == binyan && f.form == form)
                .collect();
            if entries.is_empty() {
                continue;
            }
            println!("  -- {} --", form.name());
            for f in entries {
                let mark = if f.attested { " " } else { "*" };
                let label = f.pgn.label();
                let label_pad = if label.is_empty() {
                    "   ".to_string()
                } else {
                    format!("{label:>3}")
                };
                println!("    {label_pad}{mark} {}", f.text);
            }
        }
        println!();
    }
    println!("(* = generated from strong-verb fallback; gizra rule not yet modelled)");
    Ok(())
}

fn print_noun(stem_input: &str, kind: &str) -> Result<()> {
    let stem = match kind {
        "m" => morphology::NounStem::masculine(stem_input),
        "f" => morphology::NounStem::feminine_he(stem_input),
        other => {
            anyhow::bail!("unknown noun kind '{other}' (expected m, f, or ft)");
        }
    };
    let forms = morphology::inflect_noun(&stem);
    println!("Stem: {stem_input}");
    println!();
    for f in forms {
        println!("  {:<24} {}", f.label, f.text);
    }
    Ok(())
}
