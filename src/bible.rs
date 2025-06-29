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
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_database_open() {
        let _bible = Bible::default();
    }
}
