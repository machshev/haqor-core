//! Module generation: build Haqor data tables from original source texts.
//!
//! This is the Rust port of the `bible-modules` Python pipeline, moved over a
//! table at a time. Currently it generates the `bible` table (OT text from
//! UXLC plus NT Syriac transliterated into Hebrew letters from SEDRA).

mod geocoding;
mod harness;
mod hebrew_db;
mod lexicon_db;
mod names;
mod occurrences;
mod oshb;
mod prefilter;
mod proper_names;
mod quotations;
mod runtime_db;
mod sedra;
mod sedra_db;
mod sefaria;
mod stepbible;
mod syntax;
mod tipnr;
mod translation;
mod tsk;
mod uxlc;
mod xml;

pub use haqor_core::normalize_surface;
pub use harness::{eval_from_db, parse_eval};
pub use hebrew_db::{
    book_name, book_number, generate_hebrew, generate_hebrew_with_sources, parse_passage,
    preview_missing, refresh_reader_glosses,
};
pub use lexicon_db::{
    generate_lexicon, load_noun_inventory, load_proper_inventory, load_root_inventory,
};
pub use names::{NamesSummary, build_names, gen_names};
pub use occurrences::parse_ot_coverage;
pub use quotations::{
    GenQuotesOptions, KNOWN_FALSE, KNOWN_PARALLELS, KNOWN_QUOTATIONS, LinkKind, Matcher,
    MatcherParams, build_quotations, explain_pair, gen_quotes, ref_label,
};
pub use runtime_db::{BlobCodec, SCHEMA_VERSION, generate_runtime, open_generation_dbs, pack_ref};
pub use sefaria::{ImportSummary, import_sefaria};
pub use stepbible::source_dir as stepbible_source_dir;
pub use syntax::{SyntaxSummary, build_syntax_trees, gen_syntax};
pub use translation::{TranslationSummary, build_translation, gen_translation};
pub use tsk::{TskSummary, build_thematic_references, gen_tsk};

use std::path::Path;

use anyhow::{Context, Result};
use log::info;
use rusqlite::Connection;

/// Generate a standalone SQLite database containing the `bible` table.
///
/// `src_texts` is the directory holding `UXLC/Books` and `SEDRA`. `output` is
/// the SQLite file to (re)create.
pub fn generate_bible(src_texts: &Path, output: &Path) -> Result<usize> {
    let books_dir = src_texts.join("UXLC").join("Books");
    let sedra_dir = src_texts.join("SEDRA");

    info!("Parsing OT (UXLC) from {}", books_dir.display());
    let ot = uxlc::parse_all(&books_dir)?;
    info!("  {} OT verses", ot.len());

    info!("Parsing NT (SEDRA) from {}", sedra_dir.display());
    let nt = sedra::parse_all(&sedra_dir)?;
    info!("  {} NT verses", nt.len());

    if output.exists() {
        std::fs::remove_file(output)
            .with_context(|| format!("removing existing {}", output.display()))?;
    }

    let mut db =
        Connection::open(output).with_context(|| format!("opening {}", output.display()))?;
    db.execute(
        "CREATE TABLE bible(book INT, chapter INT, verse INT, words TEXT)",
        [],
    )?;
    // The written forms the running text shows a qere for. `span` counts the
    // running-text tokens the ketiv answers to, and is 0 for a ketiv that is
    // never read; see `uxlc::Ketiv`.
    db.execute(
        "CREATE TABLE ketiv(
             book     INT NOT NULL,
             chapter  INT NOT NULL,
             verse    INT NOT NULL,
             position INT NOT NULL,
             span     INT NOT NULL,
             text     TEXT NOT NULL,
             PRIMARY KEY (book, chapter, verse, position)
         )",
        [],
    )?;

    let mut ketivs = 0usize;
    let tx = db.transaction()?;
    {
        let mut stmt = tx.prepare("INSERT INTO bible VALUES (?1, ?2, ?3, ?4)")?;
        let mut ketiv_stmt = tx.prepare(
            "INSERT INTO ketiv(book, chapter, verse, position, span, text) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for v in ot.iter().chain(nt.iter()) {
            stmt.execute((v.book, v.chapter, v.verse, &v.words))?;
            for k in &v.ketivs {
                ketiv_stmt.execute((v.book, v.chapter, v.verse, k.position, k.span, &k.text))?;
                ketivs += 1;
            }
        }
    }
    tx.commit()?;
    info!("  {ketivs} ketiv readings");

    let total = ot.len() + nt.len();
    info!("Wrote {total} rows to {}", output.display());
    Ok(total)
}

/// Generate a standalone SQLite database mirroring the SEDRA source files
/// losslessly, with transliteration columns rendered into Hebrew Unicode.
pub fn generate_sedra(src_texts: &Path, output: &Path) -> Result<usize> {
    sedra_db::generate_sedra(src_texts, output)
}

/// A downloaded dataset that `prepare` reduces to the files vendored in
/// `src_texts`: what the build reads, in place of the dataset as published.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparedSource {
    Stepbible,
    MaculaHebrew,
    Unfoldingword,
    OpenbibleGeocoding,
}

impl PreparedSource {
    /// The dataset's directory in `src_texts`.
    pub fn default_dir(self, src_texts: &Path) -> std::path::PathBuf {
        match self {
            PreparedSource::Stepbible => stepbible::source_dir(src_texts),
            PreparedSource::MaculaHebrew => syntax::source_dir(src_texts),
            PreparedSource::OpenbibleGeocoding => geocoding::source_dir(src_texts),
            PreparedSource::Unfoldingword => translation::source_dir(src_texts),
        }
    }
}

impl std::str::FromStr for PreparedSource {
    type Err = String;

    fn from_str(name: &str) -> std::result::Result<Self, String> {
        match name {
            "stepbible" => Ok(PreparedSource::Stepbible),
            "macula-hebrew" => Ok(PreparedSource::MaculaHebrew),
            "openbible-geocoding" => Ok(PreparedSource::OpenbibleGeocoding),
            "unfoldingword" => Ok(PreparedSource::Unfoldingword),
            _ => Err(format!("unknown dataset {name:?}")),
        }
    }
}

/// `db prepare`: reduce the dataset downloaded at `from` to its prepared files
/// in `to`. Returns what was written, for the log.
pub fn prepare(source: PreparedSource, from: &Path, to: &Path) -> Result<String> {
    match source {
        PreparedSource::Stepbible => {
            let s = stepbible::prepare(from, to)?;
            Ok(format!(
                "{} TAHOT words ({} with a sense, {} naming a person or place; {} naming one \
                 TIPNR lacks), {} senses and {} TIPNR records",
                s.words, s.sense_words, s.name_words, s.unknown_names, s.senses, s.names
            ))
        }
        PreparedSource::OpenbibleGeocoding => {
            let places = geocoding::prepare(from, to)?;
            Ok(format!("the positions of {places} places"))
        }
        PreparedSource::MaculaHebrew => {
            let verses = syntax::prepare(from, to)?;
            Ok(format!("the syntax trees of {verses} verses"))
        }
        PreparedSource::Unfoldingword => {
            let summary = translation::prepare(from, to)?;
            Ok(format!(
                "{} UHB words and {} ULT verses ({} alignments naming no UHB word left out)",
                summary.hebrew_words, summary.verses, summary.unresolved
            ))
        }
    }
}
