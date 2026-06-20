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
    /// Parse a fully-pointed OT word into every candidate analysis: verbs
    /// (root + binyan + form + person/gender/number) and, when the lexicon is
    /// available, nouns and adjectives (lemma + class + inflected slot).
    Parse {
        /// Fully-pointed Hebrew word (e.g. שָׁמַר). Cantillation is ignored.
        word: String,
        /// Lexicon database supplying the noun/adjective stem inventory. If it
        /// is missing, only the (DB-free) verb analysis is reported.
        #[arg(short, long, default_value = "data/lexicon.db")]
        lexicon_db: PathBuf,
    },
    /// Inflect a Hebrew noun stem (singular absolute) across state, number,
    /// and pronominal suffixes
    Noun {
        /// Singular absolute form, fully pointed (e.g. דָּבָר)
        stem: String,
        /// Stem class: "m" (masculine, default), "f" (feminine -ה), "ft"
        /// (feminine -ת), or "s" (segolate, e.g. מֶלֶךְ)
        #[arg(short, long, default_value = "m")]
        kind: String,
    },
    /// Parse a fully-pointed OT word into every candidate noun analysis, driven
    /// by the lexicon's noun headwords as the stem inventory.
    ParseNoun {
        /// Fully-pointed Hebrew word (e.g. מְלָכִים). Cantillation is ignored.
        word: String,
        /// Lexicon database supplying the noun-stem inventory.
        #[arg(short, long, default_value = "data/lexicon.db")]
        lexicon_db: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum DbCommands {
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
    /// Build hebrew.db: reverse-parse every OT word in the `bible` table into
    /// candidate verb analyses, storing surfaces, occurrences, analyses and
    /// roots, plus review views for the unparsed and ambiguous tokens.
    GenHebrew {
        /// Bible database path
        #[arg(short, long, default_value = "data/bible.db")]
        bible_db: PathBuf,
        /// Output database path
        #[arg(short, long, default_value = "data/hebrew.db")]
        output: PathBuf,
        /// Lexicon database; its proper nouns plus a curated closed-class list
        /// pre-filter non-verb tokens out of verb parsing. Defaults to the
        /// in-repo data/lexicon.db.
        #[arg(short, long, default_value = "data/lexicon.db")]
        lexicon_db: Option<PathBuf>,
        /// Skip the lexicon prefilter entirely (store the unfiltered parser
        /// output). Use with a throwaway `-o` path to build an eval DB whose
        /// `parse-eval --from-db` score matches the unfiltered in-memory eval
        /// (minus only the DB-join alignment floor) — not for the shipped DB.
        #[arg(long)]
        no_prefilter: bool,
        /// Wipe and rebuild the whole database. Without this, an existing
        /// database is updated incrementally: only the still-unresolved
        /// (`review_missing`) surfaces are re-analysed.
        #[arg(short, long)]
        force: bool,
        /// In incremental mode, only re-analyse the N highest-frequency missing
        /// surfaces (0 = all). Lets you iterate on the most impactful words
        /// without re-parsing the whole review_missing backlog.
        #[arg(short = 'n', long, default_value_t = 0)]
        limit: usize,
    },
    /// Fast iteration loop: re-run the *current* parser over the N highest-
    /// frequency surfaces still in `review_missing` and print what each would
    /// now resolve to, without modifying the database. Make a parser fix, run
    /// this to see which top-N missing it accounts for, repeat; commit with
    /// `gen-hebrew -n N` once satisfied.
    ReviewMissing {
        /// Hebrew database path
        #[arg(short, long, default_value = "data/hebrew.db")]
        output: PathBuf,
        /// Lexicon database (defaults to the in-repo data/lexicon.db).
        #[arg(short, long, default_value = "data/lexicon.db")]
        lexicon_db: Option<PathBuf>,
        /// Only preview the N highest-frequency missing surfaces (0 = all).
        #[arg(short = 'n', long, default_value_t = 30)]
        limit: usize,
        /// Which subset to loop on: hebrew (default), aramaic, or all.
        #[arg(short = 'L', long, default_value = "hebrew")]
        language: String,
        /// Restrict to a book or book range, e.g. "Gen" or "Gen-Deut".
        #[arg(short = 'p', long)]
        passage: Option<String>,
    },
    /// Prototype: reverse-parse every OT word in the `bible` table and report
    /// how much of the text the morphology generator can account for.
    ParseOt {
        /// Bible database path
        #[arg(short, long, default_value = "data/bible.db")]
        bible_db: PathBuf,
        /// Limit to a single OT book (Haqor numbering, 1..=39)
        #[arg(long)]
        book: Option<u8>,
        /// Cap on verses processed (0 = all)
        #[arg(short = 'n', long, default_value_t = 0)]
        limit: usize,
    },
    /// Generate the `english` Strong's gloss table from the HebrewLexicon
    /// source (HebrewStrong.xml), keyed by Strong's number for joining onto
    /// morphhb lemmas.
    GenLexicon {
        /// Source texts directory (defaults to src_texts/)
        #[arg(short, long, default_value = "src_texts")]
        src_texts: PathBuf,
        /// Output database path
        #[arg(short, long, default_value = "data/lexicon.db")]
        output: PathBuf,
    },
    /// Accuracy harness: score the reverse-parser against OSHB (morphhb) gold
    /// tags. Runs our own parser on OSHB's surface text and compares the derived
    /// analysis to the gold morphology — the lexicon is the scorer, not the
    /// source of the answer.
    ParseEval {
        /// Path to the cloned morphhb repo (expects a `wlc/` subdir)
        #[arg(short, long, default_value = "src_texts/morphhb")]
        morphhb: PathBuf,
        /// Bible database for the alignment check (None to skip)
        #[arg(short, long, default_value = "data/bible.db")]
        bible_db: PathBuf,
        /// Lexicon database; when given, restricts candidate roots to its
        /// `roots` inventory so the report measures the filtered parser.
        #[arg(short, long)]
        lexicon_db: Option<PathBuf>,
        /// Disambiguate-only: apply the root filter solely to break ties
        /// (>1 candidate), never dropping a lone parse. Requires --lexicon-db.
        #[arg(short, long)]
        soft: bool,
        /// Cap on gold verb tokens scored (0 = all)
        #[arg(short = 'n', long, default_value_t = 0)]
        limit: usize,
        /// Score an already-built hebrew.db's stored analyses directly (a DB
        /// join, no reparse) instead of re-running the parser. Ignores the
        /// lexicon/soft options.
        #[arg(long)]
        from_db: Option<PathBuf>,
        /// Print the top-N most frequent failing surfaces (unparsed or
        /// gold-analysis-missing) with their gold tags (0 = off)
        #[arg(long, default_value_t = 0)]
        misses: usize,
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

            let bible = Bible::open("data")?;

            println!("{}", bible.get(book, chapter, verse)?)
        }
        Commands::Morph { root, binyan } => {
            print_morphology(&root, binyan.as_deref())?;
        }
        Commands::Parse { word, lexicon_db } => {
            print_parse(&word, &lexicon_db)?;
        }
        Commands::Noun { stem, kind } => {
            print_noun(&stem, &kind)?;
        }
        Commands::ParseNoun { word, lexicon_db } => {
            print_parse_noun(&word, &lexicon_db)?;
        }
        Commands::Db { command } => match command {
            DbCommands::GenBible { src_texts, output } => {
                let total = haqor_core::generate::generate_bible(&src_texts, &output)?;
                println!("Wrote {} rows to {}", total, output.display());
            }
            DbCommands::GenSedra { src_texts, output } => {
                let total = haqor_core::generate::generate_sedra(&src_texts, &output)?;
                println!("Wrote {} rows to {}", total, output.display());
            }
            DbCommands::GenHebrew {
                bible_db,
                output,
                lexicon_db,
                no_prefilter,
                force,
                limit,
            } => {
                let lexicon = if no_prefilter {
                    None
                } else {
                    lexicon_db.as_deref()
                };
                let (surfaces, occurrences, parsed) =
                    haqor_core::generate::generate_hebrew(&bible_db, &output, lexicon, force, limit)?;
                println!(
                    "Wrote {} surfaces ({} parsed), {} occurrences to {}",
                    surfaces,
                    parsed,
                    occurrences,
                    output.display()
                );
            }
            DbCommands::ReviewMissing {
                output,
                lexicon_db,
                limit,
                language,
                passage,
            } => {
                let range = passage
                    .as_deref()
                    .map(haqor_core::generate::parse_passage)
                    .transpose()?;
                haqor_core::generate::preview_missing(
                    &output,
                    lexicon_db.as_deref(),
                    limit,
                    &language,
                    range,
                )?;
            }
            DbCommands::ParseOt {
                bible_db,
                book,
                limit,
            } => {
                haqor_core::generate::parse_ot_coverage(&bible_db, book, limit)?;
            }
            DbCommands::GenLexicon { src_texts, output } => {
                let total = haqor_core::generate::generate_lexicon(&src_texts, &output)?;
                println!("Wrote {} rows to {}", total, output.display());
            }
            DbCommands::ParseEval {
                morphhb,
                bible_db,
                lexicon_db,
                soft,
                limit,
                from_db,
                misses,
            } => {
                if let Some(hebrew_db) = from_db {
                    haqor_core::generate::eval_from_db(&morphhb, &hebrew_db, limit, misses)?;
                } else {
                    haqor_core::generate::parse_eval(
                        &morphhb,
                        Some(&bible_db),
                        lexicon_db.as_deref(),
                        soft,
                        limit,
                        misses,
                    )?;
                }
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
        Some(b) => Some(parse_binyan(b).with_context(|| format!("unknown binyan '{b}'"))?),
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
        if let Some(only) = filter
            && binyan != only
        {
            continue;
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
            morphology::Form::Wayyiqtol,
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

/// Combined parse report: every verb analysis (DB-free) plus every noun/
/// adjective analysis (driven by the lexicon inventory, skipped if it is
/// missing).
fn print_parse(word: &str, lexicon_db: &std::path::Path) -> Result<()> {
    println!("Word: {word}");
    println!();
    print_verb_section(word);
    println!();
    print_noun_section(word, lexicon_db)?;
    Ok(())
}

/// Verb half of the parse report.
fn print_verb_section(word: &str) {
    let matches = morphology::parse_word(word);
    if matches.is_empty() {
        println!("Verbs: no analyses found.");
        return;
    }
    println!("Verbs — {} candidate analysis/analyses:", matches.len());
    for m in &matches {
        let root: String = m.root.letters.iter().collect();
        let mark = if m.attested { " " } else { "*" };
        let prefix = if m.prefix.is_empty() {
            String::new()
        } else if m.vav_consecutive {
            format!("[{} wayyiqtol] ", m.prefix)
        } else {
            format!("[{}] ", m.prefix)
        };
        let label = m.pgn.label();
        let label = if label.is_empty() { "-" } else { &label };
        let suffix = m
            .object_suffix
            .map(|p| format!(" + obj {}", p.label()))
            .unwrap_or_default();
        println!(
            "  {mark}{prefix}root {root}  {:<8} {:<14} {}{}",
            m.binyan.name(),
            m.form.name(),
            label,
            suffix,
        );
    }
    println!("  (* = matched a strong-verb fallback; gizra rule not yet modelled)");
}

/// Noun/adjective half of the parse report. If `lexicon_db` is missing, prints a
/// note and returns Ok (so the verb-only report still works without the DB).
fn print_noun_section(word: &str, lexicon_db: &std::path::Path) -> Result<()> {
    if !lexicon_db.exists() {
        println!(
            "Nouns/adjectives: skipped (lexicon {} not found; pass --lexicon-db).",
            lexicon_db.display()
        );
        return Ok(());
    }
    let stems = haqor_core::generate::load_noun_inventory(lexicon_db)
        .with_context(|| format!("loading noun inventory from {}", lexicon_db.display()))?;
    let mut inventory = morphology::NounInventory::build(&stems);
    inventory.add_irregulars();
    inventory.add_gold_nouns();
    let matches = inventory.parse(word);

    if matches.is_empty() {
        println!("Nouns/adjectives: no analyses found.");
        return Ok(());
    }
    println!(
        "Nouns/adjectives — {} candidate analysis/analyses:",
        matches.len()
    );
    for m in &matches {
        let prefix = if m.prefix.is_empty() {
            String::new()
        } else {
            format!("[{}] ", m.prefix)
        };
        println!("  {prefix}{}  {:?}  {}", m.stem, m.kind, m.label);
    }
    Ok(())
}

fn print_noun(stem_input: &str, kind: &str) -> Result<()> {
    let stem = match kind {
        "m" => morphology::NounStem::masculine(stem_input),
        "f" => morphology::NounStem::feminine_he(stem_input),
        "ft" => morphology::NounStem::feminine_t(stem_input),
        "s" => morphology::NounStem::segolate(stem_input),
        other => {
            anyhow::bail!("unknown noun kind '{other}' (expected m, f, ft, or s)");
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

fn print_parse_noun(word: &str, lexicon_db: &std::path::Path) -> Result<()> {
    let stems = haqor_core::generate::load_noun_inventory(lexicon_db)
        .with_context(|| format!("loading noun inventory from {}", lexicon_db.display()))?;
    let mut inventory = morphology::NounInventory::build(&stems);
    inventory.add_irregulars();
    inventory.add_gold_nouns();
    let matches = inventory.parse(word);

    println!("Word: {word}");
    println!("({} noun stems in inventory)", inventory.len());
    println!();
    if matches.is_empty() {
        println!("No noun analyses found.");
        println!();
        println!(
            "(Driven by the lexicon's noun headwords; only stem classes the\n \
             generator models — segolate plus the masculine/feminine endings —\n \
             and only forms spelled exactly as the input will match.)"
        );
        return Ok(());
    }
    println!("{} candidate analysis/analyses:", matches.len());
    println!();
    for m in &matches {
        let prefix = if m.prefix.is_empty() {
            String::new()
        } else {
            format!("[{}] ", m.prefix)
        };
        println!("  {prefix}{}  {:?}  {}", m.stem, m.kind, m.label);
    }
    Ok(())
}
