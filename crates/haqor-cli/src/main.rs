//! # Haqor
//!
//! `haqor` is a CLI app that provides convenient access to the functionality
//! in the `haqor-core` library. At the moment this is mostly used
//! for testing during development although this may expand to become a fully
//! fledged CLI based bible app.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use haqor_core::bible::Bible;
use haqor_core::morphology;
use log::info;
use std::env;
use std::net::SocketAddr;
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
    /// Serve the local browser editor for manual lexicon overlays.
    Admin {
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
    /// Serve and merge bearer-token protected learner progress on your LAN.
    SyncServer {
        /// LAN address to listen on. Use 0.0.0.0 to accept devices on the LAN.
        #[arg(long, default_value = "0.0.0.0:8788")]
        bind: SocketAddr,
        /// Canonical learner-progress database held by this server.
        #[arg(long, default_value = "data/sync-progress.db")]
        progress: PathBuf,
        /// Secret shared with the app. Must be at least 16 characters.
        #[arg(long)]
        token: String,
    },
    // ---- Paradigm generators (lemma → inflected forms) ----
    /// Generate the verb paradigm of a 3-letter Hebrew root. (Alias: morph)
    #[command(visible_alias = "morph")]
    Verb {
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
        /// Stem class: "m" (masculine, default), "f" (feminine -ה), "ft"
        /// (feminine -ת), or "s" (segolate, e.g. מֶלֶךְ)
        #[arg(short, long, default_value = "m")]
        kind: String,
    },
    /// Inflect a Hebrew adjective stem (masculine singular absolute) across
    /// gender/number agreement, state, number, and pronominal suffixes.
    Adjective {
        /// Masculine singular absolute form, fully pointed (e.g. גָּדוֹל)
        stem: String,
        /// Stem class: "m" (masculine, default), "f" (feminine -ה), "ft"
        /// (feminine -ת), or "s" (segolate)
        #[arg(short, long, default_value = "m")]
        kind: String,
    },

    // ---- Parsers (surface word → candidate analyses) ----
    /// Parse a fully-pointed OT word into every candidate analysis, trying each
    /// part of speech quickest-to-slowest: verbs (DB-free) first, then nouns
    /// and adjectives (driven by the lexicon inventory, skipped if it is
    /// missing).
    Parse {
        /// Fully-pointed Hebrew word (e.g. שָׁמַר). Cantillation is ignored.
        word: String,
        /// Lexicon database supplying the noun/adjective stem inventory. If it
        /// is missing, only the (DB-free) verb analysis is reported.
        #[arg(short, long, default_value = "data/lexicon.db")]
        lexicon_db: PathBuf,
    },
    /// Parse a fully-pointed OT word into every candidate verb analysis
    /// (root + binyan + form + person/gender/number). DB-free.
    ParseVerb {
        /// Fully-pointed Hebrew word (e.g. שָׁמַר). Cantillation is ignored.
        word: String,
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
    /// Parse a fully-pointed OT word into every candidate adjective analysis,
    /// driven by the lexicon's adjective headwords as the stem inventory.
    ParseAdjective {
        /// Fully-pointed Hebrew word (e.g. גְּדוֹלָה). Cantillation is ignored.
        word: String,
        /// Lexicon database supplying the adjective-stem inventory.
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
        /// Source texts directory holding the morphhb/ OSHB tagging, used to
        /// rank each surface's analyses by corpus attestation (most-attested
        /// first). If absent, the generator's own ordering is kept.
        #[arg(short, long, default_value = "src_texts")]
        src_texts: PathBuf,
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
    /// Refresh only occurrence-level reader glosses in an existing hebrew.db,
    /// without rerunning morphology generation.
    RefreshReaderGlosses {
        /// Existing Hebrew database to update transactionally.
        #[arg(short, long, default_value = "data/hebrew.db")]
        output: PathBuf,
        /// Source texts directory containing the fetched STEP Bible data.
        #[arg(short, long, default_value = "src_texts")]
        src_texts: PathBuf,
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
    /// Filter the Klein and Jastrow entries out of Sefaria's `lexicon_entry`
    /// collection (as JSON lines, from bsondump) into the checked-in
    /// src_texts/Sefaria files. Run by scripts/fetch-sefaria-lexicons.sh.
    ImportSefaria {
        /// The `lexicon_entry` collection as JSON lines
        #[arg(short, long)]
        input: PathBuf,
        /// Source texts directory, for the BDB and SEDRA headwords the filter
        /// keeps entries reachable from
        #[arg(short, long, default_value = "src_texts")]
        src_texts: PathBuf,
        /// Directory the filtered files are written to
        #[arg(short, long, default_value = "src_texts/Sefaria")]
        output: PathBuf,
    },
    /// Curate the four generation databases into the single runtime haqor.db
    /// the app ships: references packed, strings interned, candidate analyses
    /// resolved once into word_info, and generation-only tables dropped.
    /// See doc/adr/0006-single-runtime-database.md.
    GenRuntime {
        /// Directory holding the generation databases.
        #[arg(short, long, default_value = "data")]
        data_dir: PathBuf,
        /// Output database path
        #[arg(short, long, default_value = "data/haqor.db")]
        output: PathBuf,
        /// Source texts directory: the thematic cross references are read from
        /// its TSK and STEPBible-Data folders, the syntax trees from
        /// MACULA-Hebrew, the translation from unfoldingWord.
        #[arg(short, long, default_value = "src_texts")]
        src_texts: PathBuf,
        /// How to store verse text and lexicon entry bodies. `zstd` is ~7 MiB
        /// smaller; `none` keeps the database readable with sqlite3, which is
        /// why it is the default for local builds.
        #[arg(long, default_value = "none")]
        blob_codec: String,
    },
    /// Rebuild the `thematic_reference` table of a runtime haqor.db in place
    /// from the Treasury of Scripture Knowledge (gen-runtime also builds it),
    /// re-numbering its KJV verse references onto the Hebrew text.
    GenTsk {
        /// Runtime database to update.
        #[arg(short, long, default_value = "data/haqor.db")]
        db: PathBuf,
        /// Source texts directory holding TSK and STEPBible-Data.
        #[arg(short, long, default_value = "src_texts")]
        src_texts: PathBuf,
    },
    /// Rebuild the people, places and word senses of a runtime haqor.db in
    /// place (gen-runtime also builds them), from STEP Bible's TIPNR and
    /// TBESH and OpenBible.info's geocoding.
    GenNames {
        /// Runtime database to update.
        #[arg(short, long, default_value = "data/haqor.db")]
        db: PathBuf,
        /// Source texts directory holding STEPBible-Data and OpenBible-Geocoding.
        #[arg(short, long, default_value = "src_texts")]
        src_texts: PathBuf,
        /// Haqor's own place identifications, which replace OpenBible.info's.
        #[arg(long, default_value = "data/place_overrides.json")]
        place_overrides: PathBuf,
    },
    /// Reduce a downloaded dataset to the prepared files vendored in
    /// src_texts, which the build reads. The scripts/fetch-*.sh scripts run
    /// this after downloading; it is not needed for an ordinary build.
    Prepare {
        /// The dataset: stepbible, macula-hebrew, unfoldingword or
        /// openbible-geocoding.
        source: haqor_db_gen::PreparedSource,
        /// The downloaded dataset, as its fetch script lays it out.
        #[arg(long)]
        from: PathBuf,
        /// Where to write the prepared files: the dataset's directory in
        /// src_texts by default.
        #[arg(long)]
        to: Option<PathBuf>,
        /// Source texts directory, for the default `--to`.
        #[arg(short, long, default_value = "src_texts")]
        src_texts: PathBuf,
    },
    /// Rebuild the `syntax_tree` table of a runtime haqor.db in place from
    /// MACULA Hebrew's syntax trees (gen-runtime also builds it).
    GenSyntax {
        /// Runtime database to update.
        #[arg(short, long, default_value = "data/haqor.db")]
        db: PathBuf,
        /// Source texts directory holding MACULA-Hebrew.
        #[arg(short, long, default_value = "src_texts")]
        src_texts: PathBuf,
    },
    /// Rebuild the `translation_verse` table of a runtime haqor.db in place
    /// from the unfoldingWord Literal Text, aligned to the Hebrew (gen-runtime
    /// also builds it). Fetch it first with scripts/fetch-unfoldingword.sh.
    GenTranslation {
        /// Runtime database to update.
        #[arg(short, long, default_value = "data/haqor.db")]
        db: PathBuf,
        /// Source texts directory holding unfoldingWord.
        #[arg(short, long, default_value = "src_texts")]
        src_texts: PathBuf,
    },
    /// Find NT (Peshitta) quotations of the Hebrew OT, and parallels within
    /// each testament, by root alignment and rebuild the `quotation` table of a
    /// runtime haqor.db in place (gen-runtime also builds it). Prints the
    /// strongest pairs of each kind and the recall on lists of well-known
    /// quotations and parallels.
    GenQuotes {
        /// Runtime database to update.
        #[arg(short, long, default_value = "data/haqor.db")]
        db: PathBuf,
        /// How many of the top-ranked pairs of each kind to print.
        #[arg(short = 'n', long, default_value_t = 40)]
        top: usize,
        /// Instead of rebuilding, explain one pair of verses, either order and
        /// any testaments: `BOOK:CH:V=BOOK:CH:V` (book numbers), e.g.
        /// `40:2:15=15:11:1`.
        #[arg(long)]
        explain: Option<String>,
        /// Override a matcher parameter, `NAME=VALUE` (repeatable), e.g.
        /// `--set min_score=8`; `cross.NAME` / `within.NAME` set only the
        /// OT/NT or the same-testament parameters. Names: see `MatcherParams`.
        #[arg(long = "set", value_name = "NAME=VALUE")]
        set: Vec<String>,
        /// Evaluate each value of one parameter against the known quotation
        /// lists instead of building, e.g. `--sweep gap_penalty=0.4,0.6,0.8`.
        #[arg(long, value_name = "NAME=V1,V2,...")]
        sweep: Option<String>,
        /// Report without writing the table.
        #[arg(long)]
        dry_run: bool,
    },
    /// Exhaustive lexicon-coverage audit: run every distinct surface form in
    /// the corpus through the exact lookup the app's word-info sheet performs
    /// (word info + BDB bridge) and list the surfaces that end up with no
    /// lexicon entry — either no word info at all ("Not found in database")
    /// or word info whose Lexicon tab would be empty.
    LexiconScan {
        /// Data directory holding bible.db, sedra.db, hebrew.db, lexicon.db.
        #[arg(short, long, default_value = "data")]
        data_dir: PathBuf,
        /// Which subset to report: hebrew (default), aramaic, or all.
        #[arg(short = 'L', long, default_value = "hebrew")]
        language: String,
        /// Print only the N most frequent gap surfaces (0 = all).
        #[arg(short = 'n', long, default_value_t = 0)]
        limit: usize,
        /// Write the full gap list as tab-separated values to this file (the
        /// stdout listing stays capped by -n).
        #[arg(long)]
        tsv: Option<PathBuf>,
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
        Commands::Verb { root, binyan } => {
            print_morphology(&root, binyan.as_deref())?;
        }
        Commands::Noun { stem, kind } => {
            print_noun(&stem, &kind, false)?;
        }
        Commands::Adjective { stem, kind } => {
            print_noun(&stem, &kind, true)?;
        }
        Commands::Parse { word, lexicon_db } => {
            print_parse(&word, &lexicon_db)?;
        }
        Commands::ParseVerb { word } => {
            println!("Word: {word}");
            println!();
            print_verb_section(&word);
        }
        Commands::ParseNoun { word, lexicon_db } => {
            print_parse_pos(&word, &lexicon_db, false)?;
        }
        Commands::ParseAdjective { word, lexicon_db } => {
            print_parse_pos(&word, &lexicon_db, true)?;
        }
        Commands::Admin {
            bind,
            overlay,
            lexicon,
            hebrew,
        } => {
            haqor_admin::serve(bind, overlay, lexicon, hebrew)?;
        }
        Commands::SyncServer {
            bind,
            progress,
            token,
        } => haqor_sync_server::serve_progress(bind, &progress, &token)?,
        Commands::Db { command } => match command {
            DbCommands::GenBible { src_texts, output } => {
                let total = haqor_db_gen::generate_bible(&src_texts, &output)?;
                println!("Wrote {} rows to {}", total, output.display());
            }
            DbCommands::GenSedra { src_texts, output } => {
                let total = haqor_db_gen::generate_sedra(&src_texts, &output)?;
                println!("Wrote {} rows to {}", total, output.display());
            }
            DbCommands::GenHebrew {
                bible_db,
                output,
                lexicon_db,
                src_texts,
                no_prefilter,
                force,
                limit,
            } => {
                let lexicon = if no_prefilter {
                    None
                } else {
                    lexicon_db.as_deref()
                };
                let morphhb = src_texts.join("morphhb");
                let tahot = haqor_db_gen::stepbible_source_dir(&src_texts);
                let (surfaces, occurrences, parsed) = haqor_db_gen::generate_hebrew_with_sources(
                    &bible_db,
                    &output,
                    lexicon,
                    Some(&morphhb),
                    Some(&tahot),
                    force,
                    limit,
                )?;
                println!(
                    "Wrote {} surfaces ({} parsed), {} occurrences to {}",
                    surfaces,
                    parsed,
                    occurrences,
                    output.display()
                );
            }
            DbCommands::RefreshReaderGlosses { output, src_texts } => {
                let tahot = haqor_db_gen::stepbible_source_dir(&src_texts);
                let total = haqor_db_gen::refresh_reader_glosses(&output, &tahot)?;
                println!(
                    "Wrote {total} STEP Bible reader glosses to {}",
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
                    .map(haqor_db_gen::parse_passage)
                    .transpose()?;
                haqor_db_gen::preview_missing(
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
                haqor_db_gen::parse_ot_coverage(&bible_db, book, limit)?;
            }
            DbCommands::GenLexicon { src_texts, output } => {
                let total = haqor_db_gen::generate_lexicon(&src_texts, &output)?;
                println!("Wrote {} rows to {}", total, output.display());
            }
            DbCommands::ImportSefaria {
                input,
                src_texts,
                output,
            } => {
                for summary in haqor_db_gen::import_sefaria(&input, &src_texts, &output)? {
                    println!(
                        "{}: kept {} of {} entries",
                        summary.source, summary.kept, summary.read
                    );
                }
            }
            DbCommands::GenRuntime {
                data_dir,
                src_texts,
                output,
                blob_codec,
            } => {
                let codec: haqor_db_gen::BlobCodec = blob_codec.parse()?;
                let words = haqor_db_gen::generate_runtime(&data_dir, &src_texts, &output, codec)?;
                println!("Wrote {} words to {}", words, output.display());
            }
            DbCommands::GenTsk { db, src_texts } => {
                let summary = haqor_db_gen::gen_tsk(&db, &src_texts)?;
                println!(
                    "Wrote {} key phrases with {} targets to {} ({} reference groups \
                     unparsed, {} targets outside the corpus)",
                    summary.notes,
                    summary.targets,
                    db.display(),
                    summary.unparsed,
                    summary.missing
                );
            }
            DbCommands::GenNames {
                db,
                src_texts,
                place_overrides,
            } => {
                let s = haqor_db_gen::gen_names(&db, &src_texts, &place_overrides)?;
                println!(
                    "Wrote {} people, places and other names ({} words naming one; {} places \
                     located, {} by Haqor's own identification) and {} senses ({} words) to {}",
                    s.entities,
                    s.name_words,
                    s.located,
                    s.overridden,
                    s.senses,
                    s.sense_words,
                    db.display()
                );
            }
            DbCommands::Prepare {
                source,
                from,
                to,
                src_texts,
            } => {
                let to = to.unwrap_or_else(|| source.default_dir(&src_texts));
                let report = haqor_db_gen::prepare(source, &from, &to)?;
                println!("Prepared {report} into {}", to.display());
            }
            DbCommands::GenSyntax { db, src_texts } => {
                let summary = haqor_db_gen::gen_syntax(&db, &src_texts)?;
                println!(
                    "Wrote the syntax trees of {} verses to {} ({} spelled differently \
                     from the corpus)",
                    summary.verses,
                    db.display(),
                    summary.misaligned
                );
            }
            DbCommands::GenTranslation { db, src_texts } => {
                let summary = haqor_db_gen::gen_translation(&db, &src_texts)?;
                println!(
                    "Wrote the English of {} verses to {} ({} of {} words linked to the Hebrew)",
                    summary.verses,
                    db.display(),
                    summary.linked,
                    summary.words
                );
            }
            DbCommands::GenQuotes {
                db,
                top,
                explain,
                set,
                sweep,
                dry_run,
            } => match explain {
                Some(pair) => {
                    let parse = |s: &str| -> Result<i64> {
                        let n: Vec<i64> = s.split(':').map(str::parse).collect::<Result<_, _>>()?;
                        anyhow::ensure!(n.len() == 3, "expected BOOK:CH:V, got {s}");
                        Ok(haqor_db_gen::pack_ref(n[0], n[1], n[2]))
                    };
                    let (x, y) = pair.split_once('=').context("expected A=B")?;
                    haqor_db_gen::explain_pair(&db, parse(x)?, parse(y)?, &set)?;
                }
                None => haqor_db_gen::gen_quotes(
                    &db,
                    &haqor_db_gen::GenQuotesOptions {
                        set,
                        sweep,
                        dry_run,
                        top,
                    },
                )?,
            },
            DbCommands::LexiconScan {
                data_dir,
                language,
                limit,
                tsv,
            } => {
                lexicon_scan(&data_dir, &language, limit, tsv.as_deref())?;
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
                    haqor_db_gen::eval_from_db(&morphhb, &hebrew_db, limit, misses)?;
                } else {
                    haqor_db_gen::parse_eval(
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

/// Run the word-info lexicon audit over the whole corpus and print a report:
/// summary counts, then the gap surfaces in descending occurrence order with
/// their first-occurrence reference so each can be inspected in context.
fn lexicon_scan(
    data_dir: &std::path::Path,
    language: &str,
    limit: usize,
    tsv: Option<&std::path::Path>,
) -> Result<()> {
    let bible = Bible::open(data_dir)
        .with_context(|| format!("opening databases in {}", data_dir.display()))?;
    let all = bible.lexicon_coverage_gaps()?;
    let gaps: Vec<_> = all
        .into_iter()
        .filter(|g| match language {
            "aramaic" => g.aramaic,
            "all" => true,
            _ => !g.aramaic,
        })
        .collect();

    let unresolved = gaps.iter().filter(|g| g.unresolved).count();
    let no_entries = gaps.len() - unresolved;
    let tokens: u64 = gaps.iter().map(|g| u64::from(g.occurrences)).sum();
    println!(
        "{} gap surfaces ({language}) covering {tokens} tokens: \
         {unresolved} with no word info at all, {no_entries} with word info but an empty Lexicon tab",
        gaps.len(),
    );
    println!();

    if let Some(path) = tsv {
        let mut out = String::from("surface\toccurrences\tkind\troot\tgloss\treference\n");
        for g in &gaps {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{} {}:{}\n",
                g.surface,
                g.occurrences,
                if g.unresolved {
                    "unresolved"
                } else {
                    "no-bdb-entry"
                },
                g.root,
                g.gloss,
                haqor_db_gen::book_name(g.book),
                g.chapter,
                g.verse,
            ));
        }
        std::fs::write(path, out).with_context(|| format!("writing {}", path.display()))?;
        println!("Full list written to {}", path.display());
        println!();
    }

    let shown = if limit == 0 {
        gaps.len()
    } else {
        limit.min(gaps.len())
    };
    for g in gaps.iter().take(shown) {
        println!(
            "{:>6}x  {}  [{}]  root={}  gloss={}  ({} {}:{})",
            g.occurrences,
            g.surface,
            if g.unresolved {
                "unresolved"
            } else {
                "no-bdb-entry"
            },
            if g.root.is_empty() { "-" } else { &g.root },
            if g.gloss.is_empty() { "-" } else { &g.gloss },
            haqor_db_gen::book_name(g.book),
            g.chapter,
            g.verse,
        );
    }
    if shown < gaps.len() {
        println!(
            "... and {} more (raise -n or use --tsv)",
            gaps.len() - shown
        );
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

/// Combined parse report, tried quickest-to-slowest: every verb analysis
/// (DB-free) first, then the lexicon-driven noun and adjective analyses. If
/// `lexicon_db` is missing, only the verb half is shown.
fn print_parse(word: &str, lexicon_db: &std::path::Path) -> Result<()> {
    println!("Word: {word}");
    println!();
    print_verb_section(word);
    println!();
    if !lexicon_db.exists() {
        println!(
            "Nouns/adjectives: skipped (lexicon {} not found; pass --lexicon-db).",
            lexicon_db.display()
        );
        return Ok(());
    }
    let (adjectives, nouns): (Vec<_>, Vec<_>) = parse_inventory(word, lexicon_db)?
        .into_iter()
        .partition(|m| m.is_adjective);
    print_pos_section("Nouns", &nouns);
    println!();
    print_pos_section("Adjectives", &adjectives);
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
        let fid = match m.fidelity {
            morphology::MatchFidelity::Exact => "exact ",
            morphology::MatchFidelity::Folded => "folded",
            morphology::MatchFidelity::Skeleton => "skel  ",
        };
        println!(
            "  {fid} {mark}{prefix}root {root}  {:<8} {:<14} {}{}",
            m.binyan.name(),
            m.form.name(),
            label,
            suffix,
        );
    }
    println!("  (fidelity: exact=byte-identical, folded=matched via a spelling fold;");
    println!("   * = matched a strong-verb fallback; gizra rule not yet modelled)");
}

/// Build the lexicon-driven inventory (common nouns + adjectives + the
/// irregular/gold harvests) and parse `word` into every candidate analysis.
fn parse_inventory(word: &str, lexicon_db: &std::path::Path) -> Result<Vec<morphology::NounMatch>> {
    let stems = haqor_db_gen::load_noun_inventory(lexicon_db)
        .with_context(|| format!("loading noun inventory from {}", lexicon_db.display()))?;
    let mut inventory = morphology::NounInventory::build(&stems);
    inventory.add_irregulars();
    inventory.add_gold_nouns();
    Ok(inventory.parse(word))
}

/// Print one part-of-speech section of a parse report.
fn print_pos_section(label: &str, matches: &[morphology::NounMatch]) {
    if matches.is_empty() {
        println!("{label}: no analyses found.");
        return;
    }
    println!("{label} — {} candidate analysis/analyses:", matches.len());
    for m in matches {
        print_match_row(m);
    }
}

/// Print one noun/adjective analysis row: optional proclitic prefix, lemma,
/// stem class, and the inflected slot label.
fn print_match_row(m: &morphology::NounMatch) {
    let prefix = if m.prefix.is_empty() {
        String::new()
    } else {
        format!("[{}] ", m.prefix)
    };
    println!("  {prefix}{}  {:?}  {}", m.stem, m.kind, m.label);
}

fn print_noun(stem_input: &str, kind: &str, is_adjective: bool) -> Result<()> {
    let stem = match kind {
        "m" => morphology::NounStem::masculine(stem_input),
        "f" => morphology::NounStem::feminine_he(stem_input),
        "ft" => morphology::NounStem::feminine_t(stem_input),
        "s" => morphology::NounStem::segolate(stem_input),
        other => {
            anyhow::bail!("unknown stem kind '{other}' (expected m, f, ft, or s)");
        }
    };
    // Adjective stems get agreement inflection (feminine sg/pl) on top of the
    // shared state/number/suffix paradigm.
    let stem = stem.with_adjective(is_adjective);
    let forms = morphology::inflect_noun(&stem);
    println!("Stem: {stem_input}");
    println!();
    for f in forms {
        println!("  {:<24} {}", f.label, f.text);
    }
    Ok(())
}

/// Single-part-of-speech parse report: nouns only (`want_adjective = false`) or
/// adjectives only (`want_adjective = true`), filtered out of the lexicon-driven
/// inventory.
fn print_parse_pos(word: &str, lexicon_db: &std::path::Path, want_adjective: bool) -> Result<()> {
    let pos = if want_adjective { "adjective" } else { "noun" };
    let matches: Vec<_> = parse_inventory(word, lexicon_db)?
        .into_iter()
        .filter(|m| m.is_adjective == want_adjective)
        .collect();

    println!("Word: {word}");
    println!();
    if matches.is_empty() {
        println!("No {pos} analyses found.");
        println!();
        println!(
            "(Driven by the lexicon's {pos} headwords; only stem classes the\n \
             generator models — segolate plus the masculine/feminine endings —\n \
             and only forms spelled exactly as the input will match.)"
        );
        return Ok(());
    }
    println!("{} candidate analysis/analyses:", matches.len());
    println!();
    for m in &matches {
        print_match_row(m);
    }
    Ok(())
}
