// Bible resource

use rusqlite::{Connection, MAIN_DB};
use rust_embed::Embed;
use std::collections::HashMap;

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

/// Full SEDRA information for one NT word form, drawn from the `sedradb`
/// lexicon (one row per matching `words` entry; homographs yield several).
#[derive(Debug, Default)]
pub struct SedraWord {
    /// Vocalised Hebrew form (`words.strVocalised`) — the displayed NT word.
    pub word: String,
    /// Consonantal Hebrew form (`words.strWord`).
    pub consonantal: String,
    /// Lexeme headword in Hebrew (`lexemes.strLexeme`).
    pub lexeme: String,
    /// Root in Hebrew (`roots.strRoot`).
    pub root: String,
    /// `lexemes.keyLexeme` — for root-tree and occurrence follow-up queries.
    pub key_lexeme: i64,
    /// `roots.keyRoot` — for root-tree and occurrence follow-up queries.
    pub key_root: i64,
    /// English glosses for the lexeme, in listing order.
    pub meanings: Vec<String>,
    pub gender: Option<String>,
    pub person: Option<String>,
    pub number: Option<String>,
    pub state: Option<String>,
    pub tense: Option<String>,
    pub form: Option<String>,
    pub suffix: Option<String>,
}

/// One lexeme in a root's family, used to present an overview of the whole
/// root tree alongside a looked-up word.
#[derive(Debug, Default)]
pub struct SedraLexemeSummary {
    /// Lexeme headword in Hebrew (`lexemes.strLexeme`).
    pub lexeme: String,
    /// English glosses for the lexeme, in listing order.
    pub meanings: Vec<String>,
    /// True for the lexeme of the word that was looked up.
    pub is_current: bool,
}

// SEDRA3 attribute decoders (see src_texts/SEDRA/SEDRA3.README.TXT, WORDS.TXT).
// The Rust `db gen-sedra` port stores each attribute in its own `key*` column
// rather than the packed 32-bit integer described in the README.

fn decode_gender(k: i64) -> Option<String> {
    Some(match k {
        1 => "Common",
        2 => "Masculine",
        3 => "Feminine",
        _ => return None,
    }
    .to_string())
}

fn decode_person(k: i64) -> Option<String> {
    Some(match k {
        1 => "Third",
        2 => "Second",
        3 => "First",
        _ => return None,
    }
    .to_string())
}

fn decode_number(k: i64) -> Option<String> {
    Some(match k {
        1 => "Singular",
        2 => "Plural",
        _ => return None,
    }
    .to_string())
}

fn decode_state(k: i64) -> Option<String> {
    Some(match k {
        1 => "Absolute",
        2 => "Construct",
        3 => "Emphatic",
        _ => return None,
    }
    .to_string())
}

fn decode_tense(k: i64) -> Option<String> {
    Some(match k {
        1 => "Perfect",
        2 => "Imperfect",
        3 => "Imperative",
        4 => "Infinitive",
        5 => "Active participle",
        6 => "Passive participle",
        7 => "Participle",
        _ => return None,
    }
    .to_string())
}

fn decode_form(k: i64) -> Option<String> {
    Some(match k {
        1 => "Peal",
        2 => "Ethpeal",
        3 => "Pael",
        4 => "Ethpaal",
        5 => "Aphel",
        6 => "Ettaphal",
        7 => "Shaphel",
        8 => "Eshtaphal",
        9 => "Saphel",
        10 => "Estaphal",
        11 => "Pauel",
        12 => "Ethpaual",
        13 => "Paiel",
        14 => "Ethpaial",
        15 => "Palpal",
        16 => "Ethpalpal",
        17 => "Palpel",
        18 => "Ethpalpal",
        19 => "Pamel",
        20 => "Ethpamal",
        21 => "Parel",
        22 => "Ethparal",
        23 => "Pali",
        24 => "Ethpali",
        25 => "Pahli",
        26 => "Ethpahli",
        27 => "Taphel",
        28 => "Ethaphal",
        _ => return None,
    }
    .to_string())
}

/// Compact pronominal-suffix label, e.g. `3ms suffix`. `None` when the word
/// carries no suffix.
fn decode_suffix(person: i64, gender: i64, number: i64) -> Option<String> {
    if person == 0 {
        return None;
    }
    let p = match person {
        1 => "3",
        2 => "2",
        3 => "1",
        _ => "?",
    };
    let g = match gender {
        1 => "m",
        2 => "f",
        _ => "c",
    };
    // keySuffixNumber: 0 = singular/none, 1 = plural.
    let n = if number == 1 { "p" } else { "s" };
    Some(format!("{p}{g}{n} suffix"))
}

#[derive(Debug)]
pub struct WordOccurrence {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
}

/// An NT verse where some lexeme of a root occurs, tagged with which lexeme of
/// the root tree it belongs to (`lexeme_index` aligns with the order returned
/// by [`Bible::sedra_root_tree`]) and the distinct word forms found there.
#[derive(Debug)]
pub struct SedraOccurrence {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
    pub lexeme_index: u32,
    pub words: Vec<String>,
}

/// BDB headwords use Unicode NFC combining order (vowels CCC=17 before dagesh/dots CCC=21-24),
/// but Cardo and the biblical text data expect traditional Hebrew order (dagesh/dots first).
/// Bubble-swap any vowel that precedes a higher-priority dot/dagesh mark.
fn normalize_hebrew_combining(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i + 1 < chars.len() {
        if is_heb_vowel(chars[i]) && is_heb_dot(chars[i + 1]) {
            chars.swap(i, i + 1);
        } else {
            i += 1;
        }
    }
    chars.into_iter().collect()
}

fn is_heb_vowel(c: char) -> bool {
    let n = c as u32;
    (0x05B0..=0x05BD).contains(&n) && n != 0x05BC || n == 0x05C7
}

fn is_heb_dot(c: char) -> bool {
    matches!(c as u32, 0x05BC | 0x05C1 | 0x05C2)
}

/// NT books (40+) store lossless SEDRA-derived Hebrew that round-trips to
/// Syriac but reads as non-idiomatic Hebrew; render it idiomatically. OT books
/// hold real pointed UXLC Hebrew and are returned untouched.
fn display_hebrew(book: u8, words: &str) -> String {
    if book >= 40 {
        crate::transliterate::hebrew_display(words)
    } else {
        words.to_owned()
    }
}

/// Idiomatic rendering of an NT (SEDRA) Hebrew lexicon string — words, lexeme
/// headwords and roots are all stored in the lossless bijective form.
fn display(s: String) -> String {
    crate::transliterate::hebrew_display(&s)
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
        let mut db = Connection::open_in_memory().unwrap();

        // Legacy Python-generated DB: lexicon, morphology, occurrences, syriac.
        let haqor_db = Asset::get("haqor.db").unwrap();
        let haqor_data = Box::new(haqor_db.data.into_owned());
        db.deserialize_bytes(MAIN_DB, Box::leak(haqor_data)).unwrap();

        // Rust-generated bible text, attached as a second schema `bibledb`.
        db.execute_batch("ATTACH DATABASE ':memory:' AS bibledb")
            .unwrap();
        let bible_db = Asset::get("bible.db").unwrap();
        let bible_data = Box::new(bible_db.data.into_owned());
        db.deserialize_bytes(c"bibledb", Box::leak(bible_data)).unwrap();

        // Rust-generated SEDRA lexicon (roots, lexemes, words, english) in
        // Hebrew Unicode, attached as a third schema `sedradb`.
        db.execute_batch("ATTACH DATABASE ':memory:' AS sedradb")
            .unwrap();
        let sedra_db = Asset::get("sedra.db").unwrap();
        let sedra_data = Box::new(sedra_db.data.into_owned());
        db.deserialize_bytes(c"sedradb", Box::leak(sedra_data)).unwrap();

        Bible { db }
    }
}

impl Bible {
    pub fn get(&self, book: u8, chapter: u8, verse: u8) -> rusqlite::Result<String> {
        let words: String = self.db.query_row(
            "SELECT words FROM bibledb.bible WHERE book == ?1 AND chapter == ?2 AND verse == ?3",
            [book, chapter, verse],
            |row| row.get(0),
        )?;
        Ok(display_hebrew(book, &words))
    }

    pub fn get_chapter(
        &self,
        book: u8,
        chapter: u8,
        syriac: bool,
    ) -> rusqlite::Result<Vec<(u8, String)>> {
        let mut stmt = self.db.prepare(
            "SELECT verse, words FROM bibledb.bible WHERE book = ?1 AND chapter = ?2 ORDER BY verse",
        )?;
        let verses = stmt
            .query_map([book, chapter], |row| {
                let verse: u8 = row.get(0)?;
                let words: String = row.get(1)?;
                let words = if syriac {
                    crate::transliterate::hebrew_to_syriac(&words)
                } else {
                    display_hebrew(book, &words)
                };
                Ok((verse, words))
            })?
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
                    headword: normalize_hebrew_combining(row.get::<_, String>(0)?.as_str()),
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
                    headword: normalize_hebrew_combining(row.get::<_, String>(0)?.as_str()),
                    root: row.get(1)?,
                    gloss: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    content_json: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(entries)
    }

    /// Full SEDRA lexicon entry for an NT word. `vocalised` is the displayed
    /// Hebrew word (matched directly against `sedradb.words.strVocalised`,
    /// since the NT bible text is the same bijective transliteration). Returns
    /// one [`SedraWord`] per matching word form (homographs yield several).
    pub fn sedra_word_info(&self, vocalised: &str) -> rusqlite::Result<Vec<SedraWord>> {
        let mut stmt = self.db.prepare(
            "SELECT w.keyLexeme, l.keyRoot, w.strWord, w.strVocalised, l.strLexeme, r.strRoot, \
                    w.keyGender, w.keyPerson, w.keyNumber, w.keyState, w.keyTense, w.keyForm, \
                    w.keySuffixPerson, w.keySuffixGender, w.keySuffixNumber \
             FROM sedradb.words w \
             JOIN sedradb.lexemes l ON w.keyLexeme = l.keyLexeme \
             JOIN sedradb.roots r ON l.keyRoot = r.keyRoot \
             WHERE replace(replace(w.strVocalised, char(1471), ''), char(95), '') = ?1 \
             ORDER BY w.keyWord",
        )?;
        let key = crate::transliterate::lookup_key(vocalised);
        let mut words = stmt
            .query_map([key], |row| {
                Ok(SedraWord {
                    key_lexeme: row.get(0)?,
                    key_root: row.get(1)?,
                    consonantal: display(row.get::<_, String>(2)?),
                    word: display(row.get::<_, String>(3)?),
                    lexeme: display(row.get::<_, String>(4)?),
                    root: display(row.get::<_, String>(5)?),
                    gender: decode_gender(row.get(6)?),
                    person: decode_person(row.get(7)?),
                    number: decode_number(row.get(8)?),
                    state: decode_state(row.get(9)?),
                    tense: decode_tense(row.get(10)?),
                    form: decode_form(row.get(11)?),
                    suffix: decode_suffix(row.get(12)?, row.get(13)?, row.get(14)?),
                    meanings: Vec::new(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        for word in words.iter_mut() {
            word.meanings = self.sedra_meanings(word.key_lexeme)?;
        }

        Ok(words)
    }

    /// English glosses for a lexeme, each composed as `before meaning after`.
    fn sedra_meanings(&self, key_lexeme: i64) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self.db.prepare(
            "SELECT strBefore, strMeaning, strAfter FROM sedradb.english \
             WHERE keyLexeme = ?1 ORDER BY keyEnglish",
        )?;
        stmt.query_map([key_lexeme], |row| {
            let before: String = row.get(0)?;
            let meaning: String = row.get(1)?;
            let after: String = row.get(2)?;
            Ok([before, meaning, after]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" "))
        })?
        .collect()
    }

    /// All lexemes sharing a root, giving an overview of the root family.
    /// `current_key_lexeme` flags the looked-up word's own lexeme.
    pub fn sedra_root_tree(
        &self,
        key_root: i64,
        current_key_lexeme: i64,
    ) -> rusqlite::Result<Vec<SedraLexemeSummary>> {
        let mut stmt = self.db.prepare(
            "SELECT keyLexeme, strLexeme FROM sedradb.lexemes \
             WHERE keyRoot = ?1 ORDER BY keyLexeme",
        )?;
        let lexemes = stmt
            .query_map([key_root], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut tree = Vec::with_capacity(lexemes.len());
        for (key_lexeme, lexeme) in lexemes {
            tree.push(SedraLexemeSummary {
                lexeme: display(lexeme),
                meanings: self.sedra_meanings(key_lexeme)?,
                is_current: key_lexeme == current_key_lexeme,
            });
        }
        Ok(tree)
    }

    /// NT verses where any word form of the given lexeme occurs.
    pub fn sedra_lexeme_occurrences(
        &self,
        key_lexeme: i64,
    ) -> rusqlite::Result<Vec<WordOccurrence>> {
        let mut stmt = self.db.prepare(
            "SELECT DISTINCT o.book, o.chapter, o.verse FROM sedradb.occurrences o \
             JOIN sedradb.words w ON o.keyWord = w.keyWord \
             WHERE w.keyLexeme = ?1 ORDER BY o.book, o.chapter, o.verse",
        )?;
        stmt.query_map([key_lexeme], |row| {
            Ok(WordOccurrence {
                book: row.get(0)?,
                chapter: row.get(1)?,
                verse: row.get(2)?,
            })
        })?
        .collect()
    }

    /// NT verses where any lexeme of the given root occurs.
    pub fn sedra_root_occurrences(&self, key_root: i64) -> rusqlite::Result<Vec<WordOccurrence>> {
        let mut stmt = self.db.prepare(
            "SELECT DISTINCT o.book, o.chapter, o.verse FROM sedradb.occurrences o \
             JOIN sedradb.words w ON o.keyWord = w.keyWord \
             JOIN sedradb.lexemes l ON w.keyLexeme = l.keyLexeme \
             WHERE l.keyRoot = ?1 ORDER BY o.book, o.chapter, o.verse",
        )?;
        stmt.query_map([key_root], |row| {
            Ok(WordOccurrence {
                book: row.get(0)?,
                chapter: row.get(1)?,
                verse: row.get(2)?,
            })
        })?
        .collect()
    }

    /// OT (Hebrew Bible) occurrences of the same consonantal root as a SEDRA
    /// NT root, pulled from the legacy haqor.db `occurrences`/`words` tables.
    /// The SEDRA root is rendered with medial letter forms while OT roots use
    /// final forms, so OT roots are folded to medial before matching. Restricted
    /// to OT books (<40) so these never duplicate the SEDRA-derived NT
    /// occurrences. Roots without a Hebrew cognate simply yield nothing.
    pub fn ot_root_occurrences(&self, sedra_key_root: i64) -> rusqlite::Result<Vec<WordOccurrence>> {
        let root: String = self.db.query_row(
            "SELECT strRoot FROM sedradb.roots WHERE keyRoot = ?1",
            [sedra_key_root],
            |row| row.get(0),
        )?;
        let key = crate::transliterate::lookup_key(&root);
        let mut stmt = self.db.prepare(
            "SELECT DISTINCT o.book, o.chapter, o.verse \
             FROM occurrences o JOIN words w ON o.raw = w.raw \
             WHERE o.book < 40 AND replace(replace(replace(replace(replace( \
                 w.root, char(1498), char(1499)), char(1501), char(1502)), \
                 char(1503), char(1504)), char(1507), char(1508)), \
                 char(1509), char(1510)) = ?1 \
             ORDER BY o.book, o.chapter, o.verse",
        )?;
        stmt.query_map([key], |row| {
            Ok(WordOccurrence {
                book: row.get(0)?,
                chapter: row.get(1)?,
                verse: row.get(2)?,
            })
        })?
        .collect()
    }

    /// NT occurrences of every lexeme of a root, each tagged with the lexeme's
    /// position in the root tree so the UI can filter by lexeme. `lexeme_index`
    /// matches the ordering of [`Bible::sedra_root_tree`] (lexemes ordered by
    /// `keyLexeme`). Adjacent rows for the same verse+lexeme are merged, with
    /// distinct word forms collected.
    pub fn sedra_root_occurrences_detailed(
        &self,
        key_root: i64,
    ) -> rusqlite::Result<Vec<SedraOccurrence>> {
        // Map keyLexeme -> index in keyLexeme order (same as sedra_root_tree).
        let mut idx_stmt = self.db.prepare(
            "SELECT keyLexeme FROM sedradb.lexemes WHERE keyRoot = ?1 ORDER BY keyLexeme",
        )?;
        let mut lexeme_index = HashMap::new();
        let keys = idx_stmt
            .query_map([key_root], |row| row.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (i, key) in keys.into_iter().enumerate() {
            lexeme_index.insert(key, i as u32);
        }

        let mut stmt = self.db.prepare(
            "SELECT o.book, o.chapter, o.verse, w.keyLexeme, w.strVocalised \
             FROM sedradb.occurrences o \
             JOIN sedradb.words w ON o.keyWord = w.keyWord \
             JOIN sedradb.lexemes l ON w.keyLexeme = l.keyLexeme \
             WHERE l.keyRoot = ?1 \
             ORDER BY o.book, o.chapter, o.verse, w.keyLexeme",
        )?;
        let rows = stmt
            .query_map([key_root], |row| {
                Ok((
                    row.get::<_, u8>(0)?,
                    row.get::<_, u8>(1)?,
                    row.get::<_, u8>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut out: Vec<SedraOccurrence> = Vec::new();
        for (book, chapter, verse, key_lexeme, word) in rows {
            let index = *lexeme_index.get(&key_lexeme).unwrap_or(&0);
            match out.last_mut() {
                Some(last)
                    if last.book == book
                        && last.chapter == chapter
                        && last.verse == verse
                        && last.lexeme_index == index =>
                {
                    if !last.words.contains(&word) {
                        last.words.push(word);
                    }
                }
                _ => out.push(SedraOccurrence {
                    book,
                    chapter,
                    verse,
                    lexeme_index: index,
                    words: vec![word],
                }),
            }
        }
        Ok(out)
    }

    /// Lexicon lookup for an NT word, backed by the `sedradb` lexicon. Returns
    /// one entry per (lexeme, meaning) pair across all matching word forms.
    pub fn sedra_lookup(&self, word: &str) -> rusqlite::Result<Vec<SedraEntry>> {
        let words = self.sedra_word_info(word)?;
        let mut entries = Vec::new();
        for w in &words {
            for meaning in &w.meanings {
                entries.push(SedraEntry {
                    lexeme: w.lexeme.clone(),
                    root: w.root.clone(),
                    meaning: meaning.clone(),
                });
            }
        }
        Ok(entries)
    }

    pub fn word_occurrences(&self, raw: &str) -> rusqlite::Result<Vec<WordOccurrence>> {
        let mut stmt = self.db.prepare(
            "SELECT book, chapter, verse FROM occurrences \
             WHERE raw = ?1 ORDER BY book, chapter, verse",
        )?;
        let occurrences = stmt
            .query_map([raw], |row| {
                Ok(WordOccurrence {
                    book: row.get(0)?,
                    chapter: row.get(1)?,
                    verse: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(occurrences)
    }

    pub fn word_occurrences_root(&self, root: &str) -> rusqlite::Result<Vec<WordOccurrence>> {
        let mut stmt = self.db.prepare(
            "SELECT DISTINCT book, chapter, verse FROM occurrences \
             WHERE constanants = ?1 ORDER BY book, chapter, verse",
        )?;
        let occurrences = stmt
            .query_map([root], |row| {
                Ok(WordOccurrence {
                    book: row.get(0)?,
                    chapter: row.get(1)?,
                    verse: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(occurrences)
    }

    pub fn chapter_count(&self, book: u8) -> rusqlite::Result<u8> {
        self.db.query_row(
            "SELECT MAX(chapter) FROM bibledb.bible WHERE book = ?1",
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
    fn test_get_reads_bible_table() {
        let bible = Bible::default();

        // OT (Genesis 1:1) comes from the UXLC source: 7 words, ends with sof
        // pasuq, first letter is bet.
        let ot = bible.get(1, 1, 1).unwrap();
        assert_eq!(ot.split(' ').count(), 7);
        assert!(ot.starts_with('ב'));
        assert!(ot.ends_with('׃'));

        // NT (Matthew 1:1, book 40) is SEDRA transliterated into Hebrew: 8
        // words, first word is כּתָבָא (kaf with dagesh).
        let matt = bible.get(40, 1, 1).unwrap();
        assert_eq!(matt.split(' ').count(), 8);
        assert!(matt.starts_with('כ'));
    }

    #[test]
    fn nt_hebrew_round_trips_through_syriac() {
        let bible = Bible::default();
        let mut stmt = bible
            .db
            .prepare("SELECT words FROM bibledb.bible WHERE book >= 40")
            .unwrap();
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(rows.len(), 7958);
        for hebrew in rows {
            let syriac = crate::transliterate::hebrew_to_syriac(&hebrew);
            let back = crate::transliterate::syriac_to_hebrew(&syriac);
            assert_eq!(back, hebrew, "round trip failed for NT verse");
        }
    }

    #[test]
    fn test_chapter_count() {
        let bible = Bible::default();
        assert_eq!(bible.chapter_count(1).unwrap(), 50); // Genesis has 50 chapters
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
    fn test_sedra_word_info() {
        let bible = Bible::default();
        // First word of Matthew 1:1 (NT) is כתבא "book/writing/Scripture".
        let matt = bible.get(40, 1, 1).unwrap();
        let first = matt.split(' ').next().unwrap();
        let info = bible.sedra_word_info(first).unwrap();
        assert!(!info.is_empty(), "no SEDRA match for {first}");
        assert!(!info[0].root.is_empty());
        assert!(!info[0].lexeme.is_empty());
        assert!(
            info.iter().any(|w| w.meanings.iter().any(|m| m.contains("book"))),
            "expected a 'book' gloss"
        );
        // sedra_lookup flattens the same data into (lexeme, meaning) entries.
        let entries = bible.sedra_lookup(first).unwrap();
        assert!(!entries.is_empty());

        // Root tree: all lexemes of the root, with the current one flagged.
        let w = &info[0];
        let tree = bible.sedra_root_tree(w.key_root, w.key_lexeme).unwrap();
        assert!(tree.len() > 1, "root should have several lexemes");
        assert_eq!(tree.iter().filter(|l| l.is_current).count(), 1);

        // OT occurrences of the same root (כתב "write") come from legacy
        // haqor.db, are all OT (<40), and never overlap the NT SEDRA set.
        let ot_occ = bible.ot_root_occurrences(w.key_root).unwrap();
        assert!(!ot_occ.is_empty(), "expected OT occurrences for root כתב");
        assert!(ot_occ.iter().all(|o| o.book < 40));

        // Occurrences: lexeme is a subset of the root family, both non-empty.
        let lex_occ = bible.sedra_lexeme_occurrences(w.key_lexeme).unwrap();
        let root_occ = bible.sedra_root_occurrences(w.key_root).unwrap();
        assert!(!lex_occ.is_empty());
        assert!(root_occ.len() >= lex_occ.len());
        assert!(root_occ.iter().all(|o| o.book >= 40));

        // Detailed root occurrences: every row tags a valid lexeme index, all
        // are NT, and distinct verses match the flat root-occurrence count.
        let detailed = bible.sedra_root_occurrences_detailed(w.key_root).unwrap();
        assert!(!detailed.is_empty());
        assert!(detailed.iter().all(|o| o.book >= 40));
        assert!(detailed.iter().all(|o| (o.lexeme_index as usize) < tree.len()));
        assert!(detailed.iter().all(|o| !o.words.is_empty()));
        let distinct_verses: std::collections::HashSet<_> = detailed
            .iter()
            .map(|o| (o.book, o.chapter, o.verse))
            .collect();
        assert_eq!(distinct_verses.len(), root_occ.len());
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
