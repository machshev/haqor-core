//! Module generation: build Haqor data tables from original source texts.
//!
//! This is the Rust port of the `bible-modules` Python pipeline, moved over a
//! table at a time. Currently it generates the `bible` table (OT text from
//! UXLC plus NT Syriac transliterated into Hebrew letters from SEDRA).

mod haqor_db;
mod harness;
mod hebrew_db;
mod lexicon_db;
mod occurrences;
mod prefilter;
mod proper_names;
mod sedra;
mod sedra_db;
mod uxlc;

pub use haqor_db::shrink_haqor;
pub use harness::{eval_from_db, parse_eval};
pub use hebrew_db::{
    book_number, generate_hebrew, normalize_surface, parse_passage, preview_missing,
};
pub use lexicon_db::{
    generate_lexicon, load_noun_inventory, load_proper_inventory, load_root_inventory,
};
pub use occurrences::parse_ot_coverage;

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

    let tx = db.transaction()?;
    {
        let mut stmt = tx.prepare("INSERT INTO bible VALUES (?1, ?2, ?3, ?4)")?;
        for v in ot.iter().chain(nt.iter()) {
            stmt.execute((v.book, v.chapter, v.verse, &v.words))?;
        }
    }
    tx.commit()?;

    let total = ot.len() + nt.len();
    info!("Wrote {total} rows to {}", output.display());
    Ok(total)
}

/// Generate a standalone SQLite database mirroring the SEDRA source files
/// losslessly, with transliteration columns rendered into Hebrew Unicode.
pub fn generate_sedra(src_texts: &Path, output: &Path) -> Result<usize> {
    sedra_db::generate_sedra(src_texts, output)
}
