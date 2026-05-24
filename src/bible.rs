// Bible resource

use rusqlite::{Connection, MAIN_DB};
use rust_embed::Embed;

#[derive(Debug)]
pub struct WordMorphology {
    pub raw: String,
    pub word: String,
    pub root: String,
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
    pub root: String,
    pub gloss: String,
    pub content_json: String,
}

#[derive(Debug)]
pub struct AraMorphology {
    pub raw: String,
    pub word: String,
    pub root: String,
    pub count: i32,
    pub gender: Option<String>,
    pub person: Option<String>,
    pub number: Option<String>,
    pub state: Option<String>,
    pub tense: Option<String>,
    pub form: Option<String>,
    pub suffix: Option<String>,
}

#[derive(Debug)]
pub struct AraEntry {
    pub headword: String,
    pub root: String,
    pub gloss: String,
    pub content_json: String,
}

#[derive(Debug)]
pub struct SedraEntry {
    pub lexeme: String,
    pub root: String,
    pub meaning: String,
}

fn strip_cantillation(word: &str) -> String {
    word.chars()
        .filter(|&c| {
            let n = c as u32;
            (0x05D0..=0x05EA).contains(&n)  // Hebrew letters
                || (0x05B0..=0x05BD).contains(&n)  // niqqud
                || n == 0x05BF  // rafe
                || (0x05C1..=0x05C2).contains(&n)  // shin/sin dots
                || (0x05C4..=0x05C5).contains(&n)  // upper/lower dots
                || n == 0x05C7 // qamats qatan
        })
        .collect()
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
            "SELECT raw, word, root, count, unknown, vav_con, article, prepositions, gender, number, prefix, suffix FROM words WHERE raw = ?1",
            [raw],
            |row| {
                Ok(WordMorphology {
                    raw: row.get(0)?,
                    word: row.get(1)?,
                    root: row.get(2)?,
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

    pub fn lex_lookup(&self, word: &str) -> rusqlite::Result<Vec<BdbEntry>> {
        let stripped = strip_cantillation(word);
        let root: String = self.db.query_row(
            "SELECT root FROM words WHERE raw = ?1",
            [&stripped],
            |row| row.get(0),
        )?;
        let mut stmt = self
            .db
            .prepare("SELECT headword, root, gloss, content_json FROM bdb WHERE root = ?1")?;
        let entries = stmt
            .query_map([&root], |row| {
                Ok(BdbEntry {
                    headword: row.get(0)?,
                    root: row.get(1)?,
                    gloss: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    content_json: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(entries)
    }

    pub fn get_word_morphology_ara(&self, raw: &str) -> rusqlite::Result<AraMorphology> {
        self.db.query_row(
            "SELECT raw, word, root, count, gender, person, number, state, tense, form, suffix \
             FROM words_aramaic WHERE raw = ?1",
            [raw],
            |row| {
                let ne = |s: Option<String>| s.filter(|v| !v.is_empty());
                Ok(AraMorphology {
                    raw: row.get(0)?,
                    word: row.get(1)?,
                    root: row.get(2)?,
                    count: row.get(3)?,
                    gender: ne(row.get(4)?),
                    person: ne(row.get(5)?),
                    number: ne(row.get(6)?),
                    state: ne(row.get(7)?),
                    tense: ne(row.get(8)?),
                    form: ne(row.get(9)?),
                    suffix: ne(row.get(10)?),
                })
            },
        )
    }

    pub fn lex_lookup_ara(&self, word: &str) -> rusqlite::Result<Vec<AraEntry>> {
        let root: String = self.db.query_row(
            "SELECT root FROM words_aramaic WHERE raw = ?1",
            [word],
            |row| row.get(0),
        )?;
        let mut stmt = self.db.prepare(
            "SELECT headword, root, gloss, content_json FROM bdb_aramaic WHERE root = ?1",
        )?;
        let entries = stmt
            .query_map([&root], |row| {
                Ok(AraEntry {
                    headword: row.get(0)?,
                    root: row.get(1)?,
                    gloss: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    content_json: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(entries)
    }

    pub fn sedra_lookup(&self, word: &str) -> rusqlite::Result<Vec<SedraEntry>> {
        let root: String = self.db.query_row(
            "SELECT root FROM words_aramaic WHERE raw = ?1",
            [word],
            |row| row.get(0),
        )?;
        let mut stmt = self
            .db
            .prepare("SELECT lexeme, root, meaning FROM sedra WHERE root = ?1")?;
        let entries = stmt
            .query_map([&root], |row| {
                Ok(SedraEntry {
                    lexeme: row.get(0)?,
                    root: row.get(1)?,
                    meaning: row.get(2)?,
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
    fn test_lex_lookup() {
        let bible = Bible::default();
        // יִבְרָא (yiḇrā) - root ברא, BDB entries exist for "create"
        let entries = bible.lex_lookup("יִבְרָא").unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().all(|e| e.root == "ברא"));
        assert!(!entries[0].content_json.is_empty());
    }

    #[test]
    fn test_get_word_morphology() {
        let bible = Bible::default();
        // אֱלֹהִים (Elohim) - a simple word without prefix/suffix complications
        let morph = bible.get_word_morphology("אֱלֹהִים").unwrap();
        assert!(!morph.root.is_empty());
        assert!(morph.count > 0);
    }
}
