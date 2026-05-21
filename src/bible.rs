// Bible resource

use rusqlite::{Connection, MAIN_DB};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "data/"]
struct Asset;

#[derive(Debug)]
pub struct Bible {
    db: Connection,
}

impl Default for Bible {
    fn default() -> Self {
        let haqor_db = Asset::get("haqor.db").unwrap();
        let data = Box::new(haqor_db.data.into_owned());

        let mut db = Connection::open_in_memory().unwrap();
        db.deserialize_bytes(MAIN_DB, Box::leak(data)).unwrap();

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

    pub fn get_chapter(&self, book: u8, chapter: u8) -> rusqlite::Result<Vec<(u8, String)>> {
        let mut stmt = self.db.prepare(
            "SELECT verse, words FROM hebrew WHERE book = ?1 AND chapter = ?2 ORDER BY verse",
        )?;
        let verses = stmt
            .query_map([book, chapter], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(verses)
    }

    pub fn chapter_count(&self, book: u8) -> rusqlite::Result<u8> {
        self.db.query_row(
            "SELECT MAX(chapter) FROM hebrew WHERE book = ?1",
            [book],
            |row| row.get(0),
        )
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_database_open() {
        let _bible = Bible::default();
    }
}
