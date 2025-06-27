// Bible resource

use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Bible {
    db: Connection,
}

impl Default for Bible {
    fn default() -> Self {
        let file_path: PathBuf = dirs::data_dir()
            .expect("Can't access data dir")
            .join("haqor/haqor.db");

        if !file_path.exists() {
            panic!("Bible database file doesn't exist: {:?}", file_path)
        }

        let db = Connection::open(&file_path).unwrap();

        Bible { db }
    }
}

impl Bible {
    pub fn get(&self, book: u8, chapter: u8, verse: u8) -> rusqlite::Result<String> {
        self.db.query_row(
            "SELECT words FROM hebrew WHERE book == ?1 AND chapter == ?2 AND verse == ?3",
            [book, chapter, verse],
            |row| row.get(0),
        )
    }

    /*        let meta = db.query_row("SELECT Title, Version from Details", (), |row| {
                Ok(BibleMeta {
                    description: row.get(0)?,
                    version: row.get(1)?,
                })
            })?;
    */
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_nothing() {
        let _bible = Bible::default();
    }
}
