//! SEDRA database generator.
//!
//! Builds a standalone SQLite database whose tables mirror the SEDRA 3 source
//! files (`tblRoots.txt`, `tblLexemes.txt`, `tblWords.txt`, `tblEnglish.txt`)
//! losslessly. Every column from each source file is preserved; the only
//! transformation is that the Syriac transliteration columns (`strRoot`,
//! `strLexeme`, `strWord`, `strVocalised`) are rendered into Hebrew Unicode via
//! the bijective map in [`crate::transliterate`], so the original SEDRA
//! transliteration round-trips exactly (`hebrew_to_sedra`).
//!
//! Numeric columns (those whose name starts with `key`/`int`) get INTEGER
//! affinity; everything else is TEXT. The first column of each file is its
//! primary key. Sorting keys and English meaning text are stored verbatim.

use std::path::Path;

use anyhow::{Context, Result};
use log::info;
use rusqlite::Connection;

use crate::transliterate;

/// One source file → one table. `translit` lists the columns whose SEDRA
/// transliteration is rendered into Hebrew Unicode.
struct TableSpec {
    table: &'static str,
    file: &'static str,
    translit: &'static [&'static str],
    /// Columns to build a secondary index on (the join keys into parent tables).
    indexes: &'static [&'static str],
}

const TABLES: &[TableSpec] = &[
    TableSpec {
        table: "roots",
        file: "tblRoots.txt",
        translit: &["strRoot"],
        indexes: &[],
    },
    TableSpec {
        table: "lexemes",
        file: "tblLexemes.txt",
        translit: &["strLexeme"],
        indexes: &["keyRoot"],
    },
    TableSpec {
        table: "words",
        file: "tblWords.txt",
        translit: &["strWord", "strVocalised"],
        indexes: &["keyLexeme"],
    },
    TableSpec {
        table: "english",
        file: "tblEnglish.txt",
        translit: &[],
        indexes: &["keyLexeme"],
    },
];

/// SQLite type affinity inferred from a SEDRA column name.
fn column_type(name: &str) -> &'static str {
    if name.starts_with("key") || name.starts_with("int") {
        "INTEGER"
    } else {
        "TEXT"
    }
}

/// Load one SEDRA table into the database. Returns the number of rows written.
fn load_table(db: &mut Connection, sedra_dir: &Path, spec: &TableSpec) -> Result<usize> {
    let path = sedra_dir.join(spec.file);
    let mut reader = csv::Reader::from_path(&path)
        .with_context(|| format!("opening {}", path.display()))?;

    let headers: Vec<String> = reader.headers()?.iter().map(str::to_owned).collect();

    let columns_sql: Vec<String> = headers
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let pk = if i == 0 { " PRIMARY KEY" } else { "" };
            format!("{name} {}{pk}", column_type(name))
        })
        .collect();
    db.execute(
        &format!("CREATE TABLE {}({})", spec.table, columns_sql.join(", ")),
        [],
    )?;

    let placeholders: Vec<String> =
        (1..=headers.len()).map(|n| format!("?{n}")).collect();
    let insert_sql = format!(
        "INSERT INTO {} VALUES ({})",
        spec.table,
        placeholders.join(", ")
    );

    let translit: Vec<bool> = headers
        .iter()
        .map(|h| spec.translit.contains(&h.as_str()))
        .collect();

    let tx = db.transaction()?;
    let mut rows = 0;
    {
        let mut stmt = tx.prepare(&insert_sql)?;
        for record in reader.records() {
            let record = record?;
            let values: Vec<String> = record
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    if translit[i] {
                        transliterate::sedra_to_hebrew(field)
                    } else {
                        field.to_owned()
                    }
                })
                .collect();
            stmt.execute(rusqlite::params_from_iter(values.iter()))?;
            rows += 1;
        }
    }
    tx.commit()?;

    for column in spec.indexes {
        db.execute(
            &format!(
                "CREATE INDEX idx_{table}_{column} ON {table}({column})",
                table = spec.table,
            ),
            [],
        )?;
    }

    Ok(rows)
}

/// Generate a standalone SQLite database mirroring the SEDRA source files, with
/// transliteration columns rendered into Hebrew Unicode.
pub fn generate_sedra(src_texts: &Path, output: &Path) -> Result<usize> {
    let sedra_dir = src_texts.join("SEDRA");

    if output.exists() {
        std::fs::remove_file(output)
            .with_context(|| format!("removing existing {}", output.display()))?;
    }

    let mut db = Connection::open(output)
        .with_context(|| format!("opening {}", output.display()))?;

    let mut total = 0;
    for spec in TABLES {
        let rows = load_table(&mut db, &sedra_dir, spec)?;
        info!("  {} rows -> {}", rows, spec.table);
        total += rows;
    }

    let occ = load_occurrences(&mut db, &sedra_dir)?;
    info!("  {occ} rows -> occurrences");
    total += occ;

    info!("Wrote {total} rows to {}", output.display());
    Ok(total)
}

/// SEDRA book ids start at 52 (Matthew); Haqor book numbers start at 40.
const SEDRA_BOOK_OFFSET: u8 = 12;

/// Build the NT `occurrences` table from `BFBS.cache`: one row per word token,
/// mapping `keyWord` to its (book, chapter, verse). This lets us list verses by
/// lexeme or root via joins onto `words`/`lexemes`. Books use Haqor numbering
/// (Matthew = 40), matching the `bible` table.
fn load_occurrences(db: &mut Connection, sedra_dir: &Path) -> Result<usize> {
    let cache = std::fs::read_to_string(sedra_dir.join("BFBS.cache"))
        .with_context(|| format!("reading BFBS.cache in {}", sedra_dir.display()))?;

    db.execute(
        "CREATE TABLE occurrences(keyWord INTEGER, book INTEGER, chapter INTEGER, verse INTEGER)",
        [],
    )?;

    let tx = db.transaction()?;
    let mut rows = 0;
    {
        let mut stmt = tx.prepare("INSERT INTO occurrences VALUES (?1, ?2, ?3, ?4)")?;
        for line in cache.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut fields = line.splitn(4, ',');
            let sedra_book: u8 = fields.next().context("cache line missing book")?.parse()?;
            let chapter: u8 = fields.next().context("cache line missing chapter")?.parse()?;
            let verse: u8 = fields.next().context("cache line missing verse")?.parse()?;
            let ids = fields.next().context("cache line missing word ids")?;
            let book = sedra_book - SEDRA_BOOK_OFFSET;
            for id in ids.split(' ') {
                let key: i64 = id.parse()?;
                stmt.execute((key, book, chapter, verse))?;
                rows += 1;
            }
        }
    }
    tx.commit()?;

    db.execute(
        "CREATE INDEX idx_occurrences_keyWord ON occurrences(keyWord)",
        [],
    )?;

    Ok(rows)
}
