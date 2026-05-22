// Bible resource

use rusqlite::{Connection, MAIN_DB};
use rust_embed::Embed;

#[derive(Debug)]
pub struct WordMorphology {
    pub raw: String,
    pub word: String,
    pub consonants: String,
    pub count: i32,
    pub unknown: bool,
    pub vav_con: bool,
    pub article: bool,
    pub prepositions: Option<String>,
    pub gender: Option<String>,
    pub number: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}

#[derive(Debug)]
pub struct BdbEntry {
    pub headword: String,
    pub consonants: String,
    pub gloss: String,
    pub content_json: String,
}

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

    pub fn get_chapter(
        &self,
        book: u8,
        chapter: u8,
        syriac: bool,
    ) -> rusqlite::Result<Vec<(u8, String)>> {
        let table = if syriac { "syriac" } else { "hebrew" };
        let mut stmt = self.db.prepare(&format!(
            "SELECT verse, words FROM {table} WHERE book = ?1 AND chapter = ?2 ORDER BY verse",
        ))?;
        let verses = stmt
            .query_map([book, chapter], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(verses)
    }

    pub fn get_word_morphology(&self, raw: &str) -> rusqlite::Result<WordMorphology> {
        self.db.query_row(
            "SELECT raw, word, constanants, count, unknown, vav_con, article, prepositions, gender, number, prefix, suffix FROM words WHERE raw = ?1",
            [raw],
            |row| {
                Ok(WordMorphology {
                    raw: row.get(0)?,
                    word: row.get(1)?,
                    consonants: row.get(2)?,
                    count: row.get(3)?,
                    unknown: row.get(4)?,
                    vav_con: row.get(5)?,
                    article: row.get(6)?,
                    prepositions: row.get(7)?,
                    gender: row.get(8)?,
                    number: row.get(9)?,
                    prefix: row.get(10)?,
                    suffix: row.get(11)?,
                })
            },
        )
    }

    pub fn get_bdb_by_consonants(&self, consonants: &str) -> rusqlite::Result<Vec<BdbEntry>> {
        let mut stmt = self.db.prepare(
            "SELECT headword, consonants, gloss, content_json FROM bdb WHERE consonants = ?1",
        )?;
        let entries = stmt
            .query_map([consonants], |row| {
                Ok(BdbEntry {
                    headword: row.get(0)?,
                    consonants: row.get(1)?,
                    gloss: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    content_json: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(entries)
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

    #[test]
    fn test_get_bdb_by_consonants() {
        let bible = Bible::default();
        let entries = bible.get_bdb_by_consonants("אלהים").unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().all(|e| e.consonants == "אלהים"));
        assert!(!entries[0].content_json.is_empty());
    }

    #[test]
    fn test_get_word_morphology() {
        let bible = Bible::default();
        // אֱלֹהִים (Elohim) - a simple word without prefix/suffix complications
        let morph = bible.get_word_morphology("אֱלֹהִים").unwrap();
        assert_eq!(morph.consonants, "אלהים");
        assert!(morph.count > 0);
    }
}
