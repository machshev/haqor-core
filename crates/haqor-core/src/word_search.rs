//! Finding the Bible's words by what a reader types: a Hebrew or Aramaic word
//! or root, pointed or not; a Greek word, with or without its accents; a
//! Syriac word; or an English meaning. Each match is a dictionary word — a
//! BDB entry, a Greek dictionary form or a SEDRA lexeme — with what an app
//! needs to open it at its occurrences, so a search can start from a word the
//! reader has not first found in the text.

use std::collections::{HashMap, HashSet};

use rusqlite::OptionalExtension;

use crate::bible::{
    Bible, LEXICON_ROOT_SURFACES, SIN_DOT, bare_letters, decode_category, fold_consonants,
    normalize_hebrew_combining,
};

/// Which text a matched word is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WordSearchLanguage {
    /// The Old Testament's Hebrew, and its Aramaic, as BDB files them.
    Hebrew,
    /// The Peshitta New Testament, as SEDRA files it.
    Aramaic,
    /// The Greek New Testament, the Textus Receptus.
    Greek,
}

impl WordSearchLanguage {
    pub fn as_str(self) -> &'static str {
        match self {
            WordSearchLanguage::Hebrew => "hebrew",
            WordSearchLanguage::Aramaic => "aramaic",
            WordSearchLanguage::Greek => "greek",
        }
    }
}

/// What about a word the search matched, the closest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WordSearchField {
    /// Its dictionary form is spelled as typed.
    Headword,
    /// One of its forms in the text is spelled as typed: ויאמר finds אמר.
    Form,
    /// It is built on the root typed.
    Root,
    /// Its dictionary form begins as typed (Greek, three letters or more).
    Prefix,
    /// Its gloss says what was typed, in English.
    Gloss,
}

impl WordSearchField {
    pub fn as_str(self) -> &'static str {
        match self {
            WordSearchField::Headword => "headword",
            WordSearchField::Form => "form",
            WordSearchField::Root => "root",
            WordSearchField::Prefix => "prefix",
            WordSearchField::Gloss => "gloss",
        }
    }
}

/// One dictionary word a search found.
#[derive(Clone, Debug, PartialEq)]
pub struct WordSearchMatch {
    pub language: WordSearchLanguage,
    pub matched: WordSearchField,
    /// The dictionary form: BDB's pointed headword, the Greek lemma, or the
    /// SEDRA lexeme in Hebrew letters.
    pub headword: String,
    /// The root it is filed under; empty for a Greek word derived from none.
    pub root: String,
    pub gloss: String,
    /// The part of speech, as its lexicon names it (BDB's `vb`, `n.f`;
    /// SEDRA's `Noun`). Empty for Greek, whose parts of speech are its forms'.
    pub part_of_speech: String,
    /// A form of it in the text to open it by: the BDB headword's own form
    /// where the text has it, else its commonest; the Peshitta lexeme's
    /// commonest vocalised form; the Greek lemma itself.
    pub word: String,
    /// Hebrew: the forms of the text BDB files under the entry, so a list of
    /// its root's occurrences can be narrowed to this word's. Empty otherwise.
    pub forms: Vec<String>,
    /// Aramaic: the lexeme as an NT occurrence names it, for the same
    /// narrowing. Empty otherwise.
    pub lexeme: String,
    /// How many words of the text it is.
    pub occurrences: u32,
    /// The BDB entry key, the Greek lemma or the SEDRA lexeme id: one per
    /// dictionary word, whatever it was matched by.
    pub key: String,
}

/// How many matches of each language a search returns at most.
const PER_LANGUAGE: usize = 30;

/// How many bare ש of a Hebrew query are tried as both shin and sin; a query
/// with more is tried as typed beyond them.
const SHIN_VARIANTS: usize = 4;

impl Bible {
    /// The dictionary words `query` names, by script: Hebrew letters search
    /// the Old Testament's Hebrew and the Peshitta alike, Syriac letters the
    /// Peshitta first, Greek letters the Greek New Testament, and Latin
    /// letters every lexicon's English glosses. Within each language the
    /// closest matches ([`WordSearchField`]) come first, then the most used.
    /// Words the text never uses are left out.
    pub fn search_words(&self, query: &str) -> rusqlite::Result<Vec<WordSearchMatch>> {
        let query = query.trim();
        let mut out = Vec::new();
        match Script::of(query) {
            Some(Script::Hebrew) => {
                out.extend(self.search_hebrew(query)?);
                out.extend(self.search_aramaic(query)?);
            }
            Some(Script::Syriac) => {
                let hebrew = crate::transliterate::syriac_to_hebrew(query);
                out.extend(self.search_aramaic(&hebrew)?);
                out.extend(self.search_hebrew(&hebrew)?);
            }
            Some(Script::Greek) => out.extend(self.search_greek(query)?),
            Some(Script::Latin) => {
                let english = english_key(query);
                if english.is_empty() {
                    return Ok(out);
                }
                out.extend(self.search_hebrew_glosses(&english)?);
                out.extend(self.search_greek_glosses(&english)?);
                out.extend(self.search_aramaic_glosses(&english)?);
            }
            None => {}
        }
        Ok(out)
    }

    // --- Hebrew -----------------------------------------------------------

    fn search_hebrew(&self, query: &str) -> rusqlite::Result<Vec<WordSearchMatch>> {
        let keys = shin_variants(&fold_consonants(query));
        if keys.is_empty() || keys[0].is_empty() {
            return Ok(Vec::new());
        }
        let db = self.conn();
        let in_list = placeholders(keys.len());
        let mut found: HashMap<String, WordSearchField> = HashMap::new();
        let mut note = |key: String, field: WordSearchField| {
            found
                .entry(key)
                .and_modify(|f| *f = (*f).min(field))
                .or_insert(field);
        };

        // A pointed query names one form, and the forms spelled so; an
        // unpointed one every form with its letters.
        let pointed = query.chars().any(is_hebrew_point);
        if pointed {
            let norm = crate::normalize_surface(query);
            let mut stmt = db.prepare_cached(
                "SELECT key FROM data.lexicon_entry WHERE norm = ?1 \
                 UNION SELECT se.key FROM data.surface s \
                 JOIN data.surface_entry se ON se.surface_id = s.surface_id WHERE s.text = ?1",
            )?;
            for key in stmt.query_map([&norm], |row| row.get::<_, String>(0))? {
                note(key?, WordSearchField::Headword);
            }
        }
        let mut stmt = db.prepare(&format!(
            "SELECT key, CASE WHEN cons IN ({in_list}) THEN 0 ELSE 2 END \
             FROM data.lexicon_entry WHERE cons IN ({in_list}) OR root IN ({in_list})"
        ))?;
        let params = rusqlite::params_from_iter(keys.iter().chain(&keys).chain(&keys));
        for row in stmt.query_map(params, |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })? {
            let (key, rank) = row?;
            note(
                key,
                if rank == 0 {
                    WordSearchField::Headword
                } else {
                    WordSearchField::Root
                },
            );
        }
        let mut stmt = db.prepare(&format!(
            "SELECT DISTINCT se.key FROM data.surface s \
             JOIN data.surface_entry se ON se.surface_id = s.surface_id \
             WHERE s.cons IN ({in_list})"
        ))?;
        for key in stmt.query_map(rusqlite::params_from_iter(&keys), |row| {
            row.get::<_, String>(0)
        })? {
            note(key?, WordSearchField::Form);
        }
        Ok(closest_first(self.hebrew_matches(found)?, None))
    }

    /// BDB entries by their English: BDB's own gloss, the senses TBESH gives
    /// the forms filed under them, and the reader's English for those forms
    /// in the text (`the shepherds`), each ranked by how closely it says
    /// `english` and an entry by its closest.
    fn search_hebrew_glosses(&self, english: &str) -> rusqlite::Result<Vec<WordSearchMatch>> {
        let db = self.conn();
        let mut ranks: HashMap<String, u8> = HashMap::new();
        let mut rank_rows = |sql: &str| -> rusqlite::Result<()> {
            let mut stmt = db.prepare_cached(sql)?;
            for row in stmt.query_map([english], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })? {
                let (key, gloss) = row?;
                if let Some(rank) = gloss_rank(&gloss, english) {
                    let best = ranks.entry(key).or_insert(rank);
                    *best = (*best).min(rank);
                }
            }
            Ok(())
        };
        rank_rows("SELECT key, gloss FROM data.lexicon_entry WHERE gloss LIKE '%' || ?1 || '%'")?;
        if self.has_table("sense")? {
            rank_rows(
                "SELECT DISTINCT se.key, s.gloss FROM data.sense s \
                 JOIN data.surface_sense ss ON ss.sense_id = s.sense_id \
                 JOIN data.surface_entry se ON se.surface_id = ss.surface_id \
                 JOIN data.lexicon_entry e ON e.key = se.key AND e.norm = s.word \
                 WHERE s.gloss LIKE '%' || ?1 || '%'",
            )?;
        }
        if self.has_table("reader_gloss")? {
            rank_rows(
                "SELECT DISTINCT se.key, g.text FROM data.reader_gloss g \
                 JOIN data.word w ON w.gloss_id = g.gloss_id \
                 JOIN data.word_info wi ON wi.info_id = w.info_id \
                 JOIN data.surface_entry se ON se.surface_id = w.surface_id \
                 JOIN data.lexicon_entry e ON e.key = se.key AND e.root = wi.root \
                 WHERE g.text LIKE '%' || ?1 || '%'",
            )?;
        }
        let found = ranks
            .keys()
            .map(|key| (key.clone(), WordSearchField::Gloss))
            .collect();
        Ok(closest_first(self.hebrew_matches(found)?, Some(&ranks)))
    }

    /// The BDB entries `found`, each with its forms in the text, the closest
    /// matched and most used first.
    ///
    /// An entry keeps only the forms its root's occurrence list holds
    /// ([`Bible::hebrew_root_occurrences_detailed`]): a homograph BDB also
    /// files under it, from another root, is neither counted nor opened, as
    /// that list would not show it. An entry with no such form is left out.
    fn hebrew_matches(
        &self,
        found: HashMap<String, WordSearchField>,
    ) -> rusqlite::Result<Vec<WordSearchMatch>> {
        let mut forms_of = self.conn().prepare_cached(&format!(
            "SELECT s.text, s.occurrences FROM data.surface_entry se \
             JOIN data.surface s ON s.surface_id = se.surface_id \
             WHERE se.key = ?2 \
               AND (s.surface_id IN (SELECT surface_id FROM data.root_surface \
                                     WHERE lexeme = ?1 AND sources & 1) \
                    OR s.surface_id IN ({LEXICON_ROOT_SURFACES})) \
             ORDER BY s.occurrences DESC, s.text"
        ))?;
        let mut root_of = self.conn().prepare_cached(
            "SELECT wi.root FROM data.surface_entry se \
             JOIN data.surface s ON s.surface_id = se.surface_id \
             JOIN data.word_info wi ON wi.surface_id = s.surface_id \
             WHERE se.key = ?1 AND wi.root != '' \
             GROUP BY wi.root ORDER BY sum(s.occurrences) DESC, wi.root LIMIT 1",
        )?;
        let mut out = Vec::new();
        for (key, matched) in found {
            let Some(entry) = self.hebrew_bdb_by_id(&key)? else {
                continue;
            };
            // BDB files some words under no root (בֵּן); they are found under
            // the one the text's analysis gives their forms.
            let root = if entry.root.is_empty() {
                match root_of
                    .query_row([&key], |row| row.get::<_, String>(0))
                    .optional()?
                {
                    Some(root) => root,
                    None => continue,
                }
            } else {
                entry.root
            };
            let forms: Vec<(String, u32)> = forms_of
                .query_map([&root, &key], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            let occurrences: u32 = forms.iter().map(|(_, n)| n).sum();
            if occurrences == 0 {
                continue;
            }
            let own = crate::normalize_surface(&entry.headword);
            let word = forms
                .iter()
                .find(|(form, _)| *form == own)
                .unwrap_or(&forms[0])
                .0
                .clone();
            out.push(WordSearchMatch {
                language: WordSearchLanguage::Hebrew,
                matched,
                headword: normalize_hebrew_combining(&entry.headword),
                root,
                gloss: entry.gloss,
                part_of_speech: entry.pos,
                word,
                forms: forms.into_iter().map(|(form, _)| form).collect(),
                lexeme: String::new(),
                occurrences,
                key,
            });
        }
        Ok(out)
    }

    // --- The Peshitta -----------------------------------------------------

    fn search_aramaic(&self, query: &str) -> rusqlite::Result<Vec<WordSearchMatch>> {
        // SEDRA spells with no final letters and one ש, as `bare_letters` does.
        let key = bare_letters(query);
        if key.is_empty() || !self.has_sedra()? {
            return Ok(Vec::new());
        }
        let mut stmt = self.conn().prepare_cached(
            "SELECT lexeme_id, 0 FROM data.syriac_lexeme WHERE lexeme = ?1 \
             UNION ALL SELECT l.lexeme_id, 2 FROM data.syriac_lexeme l \
               JOIN data.syriac_root r ON r.root_id = l.root_id WHERE r.root = ?1 \
             UNION ALL SELECT DISTINCT CAST(lexeme_id AS INTEGER), 1 FROM data.syriac_word WHERE word = ?1",
        )?;
        let mut found: HashMap<i64, WordSearchField> = HashMap::new();
        for row in stmt.query_map([&key], |row| {
            Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, i64>(1)?))
        })? {
            let (Some(id), rank) = row? else { continue };
            let field = match rank {
                0 => WordSearchField::Headword,
                1 => WordSearchField::Form,
                _ => WordSearchField::Root,
            };
            found
                .entry(id)
                .and_modify(|f| *f = (*f).min(field))
                .or_insert(field);
        }
        Ok(closest_first(self.aramaic_matches(found)?, None))
    }

    fn search_aramaic_glosses(&self, english: &str) -> rusqlite::Result<Vec<WordSearchMatch>> {
        if !self.has_sedra()? {
            return Ok(Vec::new());
        }
        let mut stmt = self.conn().prepare_cached(
            "SELECT CAST(lexeme_id AS INTEGER), meaning FROM data.syriac_gloss \
             WHERE meaning LIKE '%' || ?1 || '%'",
        )?;
        let mut found = HashMap::new();
        let mut ranks: HashMap<String, u8> = HashMap::new();
        for row in stmt.query_map([english], |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, Option<String>>(1)?,
            ))
        })? {
            let (Some(id), Some(meaning)) = row? else {
                continue;
            };
            if let Some(rank) = gloss_rank(&meaning, english) {
                let best = ranks.entry(id.to_string()).or_insert(rank);
                *best = (*best).min(rank);
                found.insert(id, WordSearchField::Gloss);
            }
        }
        Ok(closest_first(self.aramaic_matches(found)?, Some(&ranks)))
    }

    /// The SEDRA lexemes `found`, each with its commonest form, the closest
    /// matched and most used first.
    fn aramaic_matches(
        &self,
        found: HashMap<i64, WordSearchField>,
    ) -> rusqlite::Result<Vec<WordSearchMatch>> {
        let db = self.conn();
        let category = self.sedra_category_column("l")?;
        let mut lexeme_of = db.prepare_cached(&format!(
            "SELECT l.lexeme, coalesce(r.root, ''), {category} FROM data.syriac_lexeme l \
             LEFT JOIN data.syriac_root r ON r.root_id = l.root_id WHERE l.lexeme_id = ?1"
        ))?;
        let mut forms_of = db.prepare_cached(
            "SELECT w.vocalised, count(*) AS n FROM data.syriac_word w \
             JOIN data.nt_word o ON o.word_id = w.word_id \
             WHERE w.lexeme_id = ?1 GROUP BY w.word_id ORDER BY n DESC, w.vocalised",
        )?;
        let mut meanings_of = db.prepare_cached(
            "SELECT meaning FROM data.syriac_gloss WHERE lexeme_id = ?1 \
             AND meaning IS NOT NULL AND meaning != '' ORDER BY gloss_id",
        )?;
        let mut out = Vec::new();
        for (id, matched) in found {
            let forms: Vec<(String, u32)> = forms_of
                .query_map([id], |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                        row.get(1)?,
                    ))
                })?
                .collect::<rusqlite::Result<_>>()?;
            let occurrences: u32 = forms.iter().map(|(_, n)| n).sum();
            if occurrences == 0 {
                continue;
            }
            let (lexeme, root, category): (Option<String>, String, Option<i64>) =
                lexeme_of.query_row([id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
            let lexeme = display(&lexeme.unwrap_or_default());
            let meanings: Vec<String> = meanings_of
                .query_map([id], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            let mut seen = HashSet::new();
            let gloss = meanings
                .into_iter()
                .filter(|m| seen.insert(m.clone()))
                .take(4)
                .collect::<Vec<_>>()
                .join("; ");
            out.push(WordSearchMatch {
                language: WordSearchLanguage::Aramaic,
                matched,
                headword: lexeme.clone(),
                root: display(&root),
                gloss,
                part_of_speech: category
                    .and_then(decode_category)
                    .unwrap_or_default()
                    .to_string(),
                word: display(&forms[0].0),
                forms: Vec::new(),
                lexeme,
                occurrences,
                key: id.to_string(),
            });
        }
        Ok(out)
    }

    fn has_sedra(&self) -> rusqlite::Result<bool> {
        self.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM data.sqlite_master \
             WHERE type = 'table' AND name = 'syriac_lexeme')",
            [],
            |row| row.get(0),
        )
    }

    // --- Greek ------------------------------------------------------------

    fn search_greek(&self, query: &str) -> rusqlite::Result<Vec<WordSearchMatch>> {
        let key = greek_key(query);
        if key.is_empty() || !self.has_greek()? {
            return Ok(Vec::new());
        }
        let db = self.conn();
        let mut found: HashMap<String, WordSearchField> = HashMap::new();
        let mut note = |lemma: String, field: WordSearchField| {
            found
                .entry(lemma)
                .and_modify(|f| *f = (*f).min(field))
                .or_insert(field);
        };
        // Lemmas, then the forms of the text, are few enough to fold here: the
        // database keeps no accentless spelling to look them up by.
        let prefix = key.chars().count() >= 3;
        let mut stmt = db.prepare_cached("SELECT lemma FROM data.greek_lemma")?;
        for lemma in stmt.query_map([], |row| row.get::<_, String>(0))? {
            let lemma = lemma?;
            let folded = greek_key(&lemma);
            if folded == key {
                note(lemma, WordSearchField::Headword);
            } else if prefix && folded.starts_with(&key) {
                note(lemma, WordSearchField::Prefix);
            }
        }
        let mut stmt = db.prepare_cached(
            "SELECT DISTINCT w.word, l.lemma FROM data.greek_word w \
             JOIN data.greek_lemma l ON l.lemma_id = w.lemma_id",
        )?;
        for row in stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })? {
            let (word, lemma) = row?;
            if greek_key(&word) == key {
                note(lemma, WordSearchField::Form);
            }
        }
        if self.has_table("greek_root")? {
            let mut stmt = db.prepare_cached("SELECT lemma, root FROM data.greek_root")?;
            for row in stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })? {
                let (lemma, root) = row?;
                if greek_key(&root) == key {
                    note(lemma, WordSearchField::Root);
                }
            }
        }
        Ok(closest_first(self.greek_matches(found)?, None))
    }

    fn search_greek_glosses(&self, english: &str) -> rusqlite::Result<Vec<WordSearchMatch>> {
        if !self.has_greek()? {
            return Ok(Vec::new());
        }
        let mut stmt = self.conn().prepare_cached(
            "SELECT lemma, gloss FROM data.greek_lemma WHERE gloss LIKE '%' || ?1 || '%'",
        )?;
        let mut found = HashMap::new();
        let mut ranks = HashMap::new();
        for row in stmt.query_map([english], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })? {
            let (lemma, gloss) = row?;
            if let Some(rank) = gloss_rank(&gloss, english) {
                ranks.insert(lemma.clone(), rank);
                found.insert(lemma, WordSearchField::Gloss);
            }
        }
        Ok(closest_first(self.greek_matches(found)?, Some(&ranks)))
    }

    /// The Greek dictionary forms `found`, the closest matched and most used
    /// first.
    fn greek_matches(
        &self,
        found: HashMap<String, WordSearchField>,
    ) -> rusqlite::Result<Vec<WordSearchMatch>> {
        let mut count_of = self.conn().prepare_cached(
            "SELECT l.gloss, count(w.lemma_id) FROM data.greek_lemma l \
             LEFT JOIN data.greek_word w ON w.lemma_id = l.lemma_id \
             WHERE l.lemma = ?1 GROUP BY l.lemma_id ORDER BY 2 DESC LIMIT 1",
        )?;
        let mut out = Vec::new();
        for (lemma, matched) in found {
            let Some((gloss, occurrences)) = count_of
                .query_map([&lemma], |row| Ok((row.get::<_, String>(0)?, row.get(1)?)))?
                .next()
                .transpose()?
            else {
                continue;
            };
            if occurrences == 0 {
                continue;
            }
            let root = self
                .greek_roots(&lemma)?
                .into_iter()
                .next()
                .unwrap_or_default();
            out.push(WordSearchMatch {
                language: WordSearchLanguage::Greek,
                matched,
                headword: lemma.clone(),
                root,
                gloss,
                part_of_speech: String::new(),
                word: lemma.clone(),
                forms: Vec::new(),
                lexeme: String::new(),
                occurrences,
                key: lemma,
            });
        }
        Ok(out)
    }
}

/// The script a query is typed in, by its first letter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Script {
    Hebrew,
    Syriac,
    Greek,
    Latin,
}

impl Script {
    fn of(query: &str) -> Option<Self> {
        query.chars().find_map(|c| match c as u32 {
            0x05D0..=0x05EA => Some(Script::Hebrew),
            0x0710..=0x072F | 0x074D..=0x074F => Some(Script::Syriac),
            0x0370..=0x03FF | 0x1F00..=0x1FFF => Some(Script::Greek),
            _ if c.is_ascii_alphabetic() => Some(Script::Latin),
            _ => None,
        })
    }
}

fn is_hebrew_point(c: char) -> bool {
    matches!(c as u32, 0x05B0..=0x05BC | 0x05C1 | 0x05C2 | 0x05C7)
}

/// The folded Hebrew `key` with each bare ש, up to [`SHIN_VARIANTS`] of
/// them, tried as a sin too: a reader typing without points means either.
fn shin_variants(key: &str) -> Vec<String> {
    let chars: Vec<char> = key.chars().collect();
    let bare: Vec<usize> = (0..chars.len())
        .filter(|&i| chars[i] == 'ש' && chars.get(i + 1) != Some(&SIN_DOT))
        .take(SHIN_VARIANTS)
        .collect();
    (0..1usize << bare.len())
        .map(|mask| {
            let mut out = String::with_capacity(key.len() + bare.len() * 2);
            for (i, &c) in chars.iter().enumerate() {
                out.push(c);
                if let Some(bit) = bare.iter().position(|&b| b == i)
                    && mask & (1 << bit) != 0
                {
                    out.push(SIN_DOT);
                }
            }
            out
        })
        .collect()
}

fn placeholders(n: usize) -> String {
    vec!["?"; n].join(", ")
}

/// A SEDRA spelling as the reader shows it.
fn display(text: &str) -> String {
    crate::transliterate::hebrew_display(text)
}

/// The closest matches first — by what matched, then for gloss matches by
/// how closely the gloss says the query (`gloss_ranks`, keyed by
/// [`WordSearchMatch::key`]; see [`gloss_rank`]) — then the most used, then
/// by spelling so the order is stable; at most [`PER_LANGUAGE`].
/// Whether a match is a person, place or other name.
fn is_name(m: &WordSearchMatch) -> bool {
    match m.language {
        WordSearchLanguage::Hebrew => m.part_of_speech.starts_with("n.pr"),
        WordSearchLanguage::Aramaic => m.part_of_speech == "Proper noun",
        WordSearchLanguage::Greek => m.headword.chars().next().is_some_and(char::is_uppercase),
    }
}

fn closest_first(
    mut matches: Vec<WordSearchMatch>,
    gloss_ranks: Option<&HashMap<String, u8>>,
) -> Vec<WordSearchMatch> {
    // A name says what it means in its gloss ("son of; god; Hadad"), so it
    // follows the words that mean what was asked for.
    let rank = |m: &WordSearchMatch| {
        gloss_ranks.map_or(0, |ranks| {
            let rank = ranks.get(&m.key).copied().unwrap_or(u8::MAX);
            rank.saturating_add(if is_name(m) { 3 } else { 0 })
        })
    };
    matches.sort_by(|a, b| {
        a.matched
            .cmp(&b.matched)
            .then(rank(a).cmp(&rank(b)))
            .then(b.occurrences.cmp(&a.occurrences))
            .then_with(|| a.headword.cmp(&b.headword))
            .then_with(|| a.key.cmp(&b.key))
    });
    // BDB's homographs of one root (רָעָה "pasture", "associate with") have
    // the same forms in the text, which no list of occurrences can tell
    // apart, so they are one match, glossed with each sense.
    let mut kept: Vec<WordSearchMatch> = Vec::with_capacity(matches.len());
    for m in matches {
        let twin = kept.iter_mut().find(|k| {
            !m.forms.is_empty()
                && k.language == m.language
                && k.root == m.root
                && k.forms == m.forms
        });
        match twin {
            Some(twin) => {
                if !twin.gloss.split(" · ").any(|g| g == m.gloss) {
                    twin.gloss = format!("{} · {}", twin.gloss, m.gloss);
                }
            }
            None => kept.push(m),
        }
    }
    kept.truncate(PER_LANGUAGE);
    kept
}

/// English as a gloss is compared: lower case, words separated by single
/// spaces, with nothing but letters, digits and apostrophes in them.
fn english_key(text: &str) -> String {
    text.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// How closely `gloss` says `english` (an [`english_key`]): 0 when one of
/// its senses is just that ("love", or "to love" for a verb), 1 when it has
/// the words in a row, 2 when a word of it begins with them ("loved" for
/// "love"), and `None` otherwise.
fn gloss_rank(gloss: &str, english: &str) -> Option<u8> {
    let senses: Vec<String> = gloss
        .split([';', ',', '/', ':'])
        .map(english_key)
        .filter(|s| !s.is_empty())
        .collect();
    let bare = |s: &str| {
        s.strip_prefix("to ")
            .or_else(|| s.strip_prefix("a "))
            .or_else(|| s.strip_prefix("an "))
            .or_else(|| s.strip_prefix("the "))
            .unwrap_or(s)
            .to_string()
    };
    if senses.iter().any(|s| *s == english || bare(s) == english) {
        return Some(0);
    }
    let whole = english_key(gloss);
    let padded = format!(" {whole} ");
    if padded.contains(&format!(" {english} ")) {
        return Some(1);
    }
    if english.chars().count() >= 3 && padded.contains(&format!(" {english}")) {
        return Some(2);
    }
    None
}

/// Greek as a search compares it: lower case, with no accents, breathings or
/// iota subscripts, a final sigma as any other, and nothing but letters.
pub fn greek_key(text: &str) -> String {
    text.chars().filter_map(greek_base).collect()
}

/// The bare lower-case letter of a Greek letter, accented or not; `None` for
/// anything else.
fn greek_base(c: char) -> Option<char> {
    let n = c as u32;
    let base = match n {
        0x0370..=0x03FF => GREEK_BASIC.chars().nth((n - 0x0370) as usize)?,
        0x1F00..=0x1FFF => GREEK_EXTENDED.chars().nth((n - 0x1F00) as usize)?,
        _ => return None,
    };
    match base {
        '_' => None,
        'ς' => Some('σ'),
        other => Some(other),
    }
}

/// The base letter of each code point of the Greek and Coptic block
/// (U+0370…U+03FF), and of Greek Extended (U+1F00…U+1FFF): `_` where there
/// is none. Generated from Unicode's decompositions.
const GREEK_BASIC: &str = "______________________α_εηι_ο_υωιαβγδεζηθικλμνξοπρ_στυφχψωιυαεηιυαβγδεζηθικλμνξοπρςστυφχψωιυουω_____________________________________θ___________";
const GREEK_EXTENDED: &str = "ααααααααααααααααεεεεεε__εεεεεε__ηηηηηηηηηηηηηηηηιιιιιιιιιιιιιιιιοοοοοο__οοοοοο__υυυυυυυυ_υ_υ_υ_υωωωωωωωωωωωωωωωωααεεηηιιοουυωω__ααααααααααααααααηηηηηηηηηηηηηηηηωωωωωωωωωωωωωωωωααααα_ααααααα_ι___ηηη_ηηεεηηη___ιιιι__ιιιιιι____υυυυρρυυυυυυρ_____ωωω_ωωοοωωω___";

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn bible() -> Option<Bible> {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("haqor.db").exists() {
            eprintln!("skipping: data/haqor.db not generated in this checkout");
            return None;
        }
        Some(Bible::open(&data).unwrap())
    }

    fn find<'a>(
        matches: &'a [WordSearchMatch],
        language: WordSearchLanguage,
        headword: &str,
    ) -> Option<&'a WordSearchMatch> {
        matches
            .iter()
            .find(|m| m.language == language && m.headword == headword)
    }

    #[test]
    fn the_fold_tables_cover_their_blocks() {
        assert_eq!(GREEK_BASIC.chars().count(), 0x90);
        assert_eq!(GREEK_EXTENDED.chars().count(), 0x100);
    }

    #[test]
    fn greek_is_compared_without_accents_or_case() {
        assert_eq!(greek_key("ἀγαπάω"), "αγαπαω");
        assert_eq!(greek_key("Ἰησοῦς"), "ιησουσ");
        assert_eq!(greek_key("ᾠδῇ,"), "ωδη");
        assert_eq!(greek_key("λόγος"), greek_key("ΛΟΓΟΣ"));
        assert_eq!(greek_key("love"), "");
    }

    #[test]
    fn an_unpointed_shin_is_tried_as_a_sin_too() {
        let keys = shin_variants("שמש");
        assert_eq!(keys.len(), 4);
        assert!(keys.contains(&"שמש".to_string()));
        assert!(keys.contains(&format!("ש{SIN_DOT}מש")));
        assert_eq!(shin_variants(&format!("ש{SIN_DOT}מח")).len(), 1);
    }

    #[test]
    fn a_gloss_ranks_by_how_closely_it_says_the_query() {
        assert_eq!(gloss_rank("love", "love"), Some(0));
        assert_eq!(gloss_rank("to love", "love"), Some(0));
        assert_eq!(gloss_rank("goodness, kindness", "kindness"), Some(0));
        assert_eq!(gloss_rank("love of God", "love"), Some(1));
        assert_eq!(gloss_rank("beloved; loved one", "loved"), Some(1));
        assert_eq!(gloss_rank("lovely", "love"), Some(2));
        assert_eq!(gloss_rank("glove", "love"), None);
        assert_eq!(gloss_rank("lo", "lo"), Some(0));
        assert_eq!(gloss_rank("long", "lo"), None);
    }

    #[test]
    fn a_hebrew_root_finds_its_words_and_the_peshitta_its_cognates() {
        let Some(bible) = bible() else { return };
        let matches = bible.search_words("אהב").unwrap();
        let verb = find(&matches, WordSearchLanguage::Hebrew, "אָהֵב").unwrap();
        assert_eq!(verb.matched, WordSearchField::Headword);
        assert_eq!(verb.root, "אהב");
        assert!(verb.occurrences > 150, "{}", verb.occurrences);
        assert!(verb.forms.len() > 20);
        let noun = find(&matches, WordSearchLanguage::Hebrew, "אַהֲבָה").unwrap();
        assert_eq!(noun.matched, WordSearchField::Root);
        assert_eq!(noun.word, "אַהֲבָה");
        // The Peshitta has no אהב; it has שמע.
        assert!(
            !matches
                .iter()
                .any(|m| m.language == WordSearchLanguage::Aramaic)
        );
        let matches = bible.search_words("שמע").unwrap();
        let aramaic = matches
            .iter()
            .find(|m| m.language == WordSearchLanguage::Aramaic)
            .unwrap();
        assert_eq!(aramaic.root, "שמע");
        assert!(aramaic.occurrences > 50, "{aramaic:?}");
    }

    #[test]
    fn an_inflected_form_finds_its_dictionary_word() {
        let Some(bible) = bible() else { return };
        let matches = bible.search_words("וַיֹּאמֶר").unwrap();
        let first = &matches[0];
        assert_eq!(first.language, WordSearchLanguage::Hebrew);
        assert_eq!(first.root, "אמר");
        assert_eq!(first.matched, WordSearchField::Headword);
        assert!(first.forms.iter().any(|f| f == "וַיֹּאמֶר"));
    }

    #[test]
    fn an_unpointed_sin_root_is_found() {
        let Some(bible) = bible() else { return };
        let matches = bible.search_words("שמח").unwrap();
        assert!(
            matches
                .iter()
                .any(|m| m.language == WordSearchLanguage::Hebrew
                    && m.root == format!("ש{SIN_DOT}מח")),
            "{matches:?}"
        );
    }

    #[test]
    fn greek_is_found_without_its_accents() {
        let Some(bible) = bible() else { return };
        if !bible.has_greek().unwrap() {
            return;
        }
        let matches = bible.search_words("αγαπη").unwrap();
        let agape = &matches[0];
        assert_eq!(agape.headword, "ἀγάπη");
        assert_eq!(agape.matched, WordSearchField::Headword);
        assert!(agape.occurrences > 100, "{}", agape.occurrences);
        // A form of the text finds its dictionary form.
        let matches = bible.search_words("ἠγάπησεν").unwrap();
        assert!(find(&matches, WordSearchLanguage::Greek, "ἀγαπάω").is_some());
    }

    #[test]
    fn english_finds_each_language_s_words_for_it() {
        let Some(bible) = bible() else { return };
        let matches = bible.search_words("Love").unwrap();
        for language in [
            WordSearchLanguage::Hebrew,
            WordSearchLanguage::Greek,
            WordSearchLanguage::Aramaic,
        ] {
            let first = matches.iter().find(|m| m.language == language);
            let first = first.unwrap_or_else(|| panic!("no {language:?} match"));
            assert_eq!(first.matched, WordSearchField::Gloss);
            assert_eq!(gloss_rank(&first.gloss, "love"), Some(0), "{first:?}");
        }
        assert!(find(&matches, WordSearchLanguage::Hebrew, "אָהֵב").is_some());
        assert!(find(&matches, WordSearchLanguage::Greek, "ἀγαπάω").is_some());
    }

    #[test]
    fn a_syriac_word_finds_its_lexeme() {
        let Some(bible) = bible() else { return };
        let matches = bible.search_words("ܚܘܒܐ").unwrap();
        let first = &matches[0];
        assert_eq!(first.language, WordSearchLanguage::Aramaic);
        assert_eq!(first.matched, WordSearchField::Headword);
        assert!(first.gloss.contains("love"), "{first:?}");
        assert!(!first.word.is_empty());
        assert_eq!(first.lexeme, first.headword);
    }

    #[test]
    fn nothing_searchable_finds_nothing() {
        let Some(bible) = bible() else { return };
        assert!(bible.search_words("").unwrap().is_empty());
        assert!(bible.search_words("  ,; ").unwrap().is_empty());
        assert!(bible.search_words("123").unwrap().is_empty());
    }
}
