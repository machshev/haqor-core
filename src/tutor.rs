//! Spaced-repetition reading tutor.
//!
//! A single never-ending study flow that teaches the learner to *read* the
//! Hebrew Bible, lazily introducing only what the next verse requires. Two
//! review tracks — **glyphs** (consonants and niqqud points) and **words**
//! (surface forms) — are scheduled with a compact SM-2 algorithm and persisted
//! in a writable `progress.db` (attached by [`crate::bible::Bible::attach_progress`]).
//!
//! Curriculum selection runs over the static corpus tables built into
//! `hebrew.db` ([`crate::generate::hebrew_db`]):
//! - `verse_word(book, chapter, verse, position, surface_id)` — each verse's
//!   ordered surfaces.
//! - `verse_stats(…, word_count, distinct_count, min_occ, sum_occ)` — per-verse
//!   difficulty.
//!
//! The engine always picks the lowest-effort next step toward reading a new
//! verse: due reviews first; otherwise the not-yet-readable verse needing the
//! fewest new words (tie-broken by those words being the most common), within
//! it the most common still-unknown word, introducing any of its unseen glyphs
//! before drilling the word itself. When a verse's every word is known it is
//! shown to read, with example passages that are now fully readable too.

use rusqlite::{Connection, OptionalExtension, params};

use crate::bible::Bible;

/// A word counts as "known" (and so unlocks the verses it appears in) once it
/// has been recalled successfully at least once. A lapse resets `reps` to 0, so
/// a forgotten word re-locks its verses until it is re-learned.
const KNOWN_REPS: i64 = 1;

/// SM-2 ease bounds.
const DEFAULT_EASE: f64 = 2.5;
const MIN_EASE: f64 = 1.3;

/// Reading marks that punctuate verses but never appear inside a word surface:
/// the sof pasuq (the verse-ending "full stop") and the maqaf (joins short
/// words into one reading unit). They are taught from the verse itself — the
/// first time the learner completes a verse containing one not yet seen — since
/// tokenisation splits on the maqaf and normalisation drops both. Ordered
/// sof-pasuq-first as every verse ends in one.
const READING_MARKS: [char; 2] = ['\u{05C3}', '\u{05BE}'];

const SECONDS_PER_DAY: i64 = 86_400;

/// How the learner rated a card, mapped onto SM-2 behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    /// Failed — reset, re-drill immediately.
    Again,
    /// Recalled with difficulty — small interval, lower ease.
    Hard,
    /// Recalled correctly — standard SM-2 growth.
    Good,
    /// Recalled easily — larger interval, higher ease.
    Easy,
}

impl Grade {
    /// Decode the 0..=3 grade carried over the signal layer.
    pub fn from_i64(n: i64) -> Option<Grade> {
        match n {
            0 => Some(Grade::Again),
            1 => Some(Grade::Hard),
            2 => Some(Grade::Good),
            3 => Some(Grade::Easy),
            _ => None,
        }
    }
}

/// Which review track a card belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Track {
    Glyph,
    Word,
}

impl Track {
    fn table(self) -> &'static str {
        match self {
            Track::Glyph => "progress.glyph_srs",
            Track::Word => "progress.word_srs",
        }
    }
    fn key_col(self) -> &'static str {
        match self {
            Track::Glyph => "glyph",
            Track::Word => "surface",
        }
    }
}

/// Mutable SM-2 state for one card. `interval_days` of 0 means "not graduated"
/// (new or just-lapsed): such a card is due immediately.
#[derive(Debug, Clone, Copy)]
struct Srs {
    ease: f64,
    interval_days: i64,
    reps: i64,
    lapses: i64,
}

impl Default for Srs {
    fn default() -> Self {
        Srs {
            ease: DEFAULT_EASE,
            interval_days: 0,
            reps: 0,
            lapses: 0,
        }
    }
}

impl Srs {
    /// Apply a grade, returning the updated state. Mirrors Anki's SM-2 variant:
    /// the first two successful reps use fixed 1- and 6-day steps, then the
    /// interval grows by the ease factor.
    fn graded(self, grade: Grade) -> Srs {
        let mut s = self;
        match grade {
            Grade::Again => {
                s.ease = (s.ease - 0.20).max(MIN_EASE);
                s.reps = 0;
                s.lapses += 1;
                s.interval_days = 0;
            }
            Grade::Hard => {
                s.ease = (s.ease - 0.15).max(MIN_EASE);
                s.reps += 1;
                s.interval_days = ((self.interval_days as f64 * 1.2).round() as i64).max(1);
            }
            Grade::Good => {
                s.reps += 1;
                s.interval_days = match s.reps {
                    1 => 1,
                    2 => 6,
                    _ => (self.interval_days as f64 * self.ease).round() as i64,
                };
            }
            Grade::Easy => {
                s.ease += 0.15;
                s.reps += 1;
                s.interval_days = match s.reps {
                    1 => 4,
                    _ => (self.interval_days as f64 * self.ease * 1.3).round() as i64,
                };
            }
        }
        s
    }

    /// Epoch-second due time after grading at `now`.
    fn due_at(&self, now: i64) -> i64 {
        now + self.interval_days * SECONDS_PER_DAY
    }
}

/// A teachable glyph: a single consonant (final forms folded to their base) or
/// a niqqud point. The display/teaching content (name, sound, examples) lives
/// in the Flutter layer keyed by `glyph`; the engine only tracks identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphCard {
    pub glyph: String,
    /// True for a base consonant, false for a vowel/dagesh/sin-shin point.
    pub is_consonant: bool,
}

/// A word to learn or review.
#[derive(Debug, Clone)]
pub struct WordCard {
    pub surface_id: i64,
    pub surface: String,
    pub occurrences: i64,
    pub gloss: String,
    pub root: String,
    pub morph: String,
    /// Glyphs in this word not yet introduced (empty for a review card).
    pub new_glyphs: Vec<GlyphCard>,
}

/// A fully-known verse offered for real reading, with other now-readable
/// passages sharing its vocabulary.
#[derive(Debug, Clone)]
pub struct VerseCard {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
    /// Other (book, chapter, verse) refs, fully readable, sharing a word.
    pub examples: Vec<(u8, u8, u8)>,
}

/// The next thing for the learner to do.
#[derive(Debug, Clone)]
pub enum StudyItem {
    NewGlyph(GlyphCard),
    ReviewGlyph(GlyphCard),
    NewWord(WordCard),
    ReviewWord(WordCard),
    ReadVerse(VerseCard),
    /// The whole corpus is readable — nothing left to teach.
    Done,
}

/// Headline progress counters for a status header.
#[derive(Debug, Clone, Copy, Default)]
pub struct TutorProgress {
    pub glyphs_known: i64,
    pub words_known: i64,
    pub verses_readable: i64,
    pub total_verses: i64,
}

/// Create the `progress.db` tables if they do not yet exist. Idempotent.
pub fn init_progress_schema(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS progress.glyph_srs(
            glyph            TEXT    PRIMARY KEY,
            ease             REAL    NOT NULL,
            interval_days    INTEGER NOT NULL,
            due_epoch        INTEGER NOT NULL,
            reps             INTEGER NOT NULL,
            lapses           INTEGER NOT NULL,
            introduced_epoch INTEGER NOT NULL,
            last_grade       INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS progress.word_srs(
            surface          TEXT    PRIMARY KEY,
            surface_id       INTEGER NOT NULL,
            ease             REAL    NOT NULL,
            interval_days    INTEGER NOT NULL,
            due_epoch        INTEGER NOT NULL,
            reps             INTEGER NOT NULL,
            lapses           INTEGER NOT NULL,
            introduced_epoch INTEGER NOT NULL,
            last_grade       INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS progress.idx_word_srs_id ON word_srs(surface_id);
         CREATE TABLE IF NOT EXISTS progress.verse_progress(
            book           INTEGER NOT NULL,
            chapter        INTEGER NOT NULL,
            verse          INTEGER NOT NULL,
            state          TEXT    NOT NULL,
            last_read_epoch INTEGER NOT NULL,
            PRIMARY KEY (book, chapter, verse)
         );
         CREATE TABLE IF NOT EXISTS progress.meta(
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
         );",
    )
}

/// Fold a final-form consonant to its medial base so the learner does not learn
/// ך and כ as separate glyphs. Non-final characters pass through unchanged.
fn fold_final(c: char) -> char {
    match c {
        '\u{05DA}' => '\u{05DB}', // KAF
        '\u{05DD}' => '\u{05DE}', // MEM
        '\u{05DF}' => '\u{05E0}', // NUN
        '\u{05E3}' => '\u{05E4}', // PE
        '\u{05E5}' => '\u{05E6}', // TSADE
        other => other,
    }
}

fn is_consonant(c: char) -> bool {
    (0x05D0..=0x05EA).contains(&(c as u32))
}

/// Decompose a (normalized) surface into its distinct teachable glyphs in
/// first-seen order: consonants (final forms folded) and niqqud points. The
/// surface text already contains only these characters.
fn decompose_glyphs(surface: &str) -> Vec<GlyphCard> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in surface.chars() {
        let g = fold_final(c);
        let cons = is_consonant(g);
        if !cons && !matches!(g as u32, 0x05B0..=0x05B9 | 0x05BB | 0x05BC | 0x05C1 | 0x05C2 | 0x05C7)
        {
            continue; // not a teachable glyph (shouldn't occur in a surface)
        }
        let key = g.to_string();
        if seen.insert(key.clone()) {
            out.push(GlyphCard {
                glyph: key,
                is_consonant: cons,
            });
        }
    }
    out
}

impl Bible {
    /// The single most-overdue review across both tracks at `now`, if any.
    fn due_review(&self, now: i64) -> rusqlite::Result<Option<StudyItem>> {
        // Earliest-due word and glyph; whichever is more overdue wins.
        let word: Option<(String, i64)> = self
            .conn()
            .query_row(
                "SELECT surface, due_epoch FROM progress.word_srs \
                 WHERE due_epoch <= ?1 ORDER BY due_epoch ASC LIMIT 1",
                params![now],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let glyph: Option<(String, i64)> = self
            .conn()
            .query_row(
                "SELECT glyph, due_epoch FROM progress.glyph_srs \
                 WHERE due_epoch <= ?1 ORDER BY due_epoch ASC LIMIT 1",
                params![now],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;

        // Prefer whichever track is more overdue; ties go to the word.
        let pick_word = match (&word, &glyph) {
            (Some((_, wd)), Some((_, gd))) => wd <= gd,
            (Some(_), None) => true,
            _ => false,
        };
        if pick_word {
            let s = word.expect("pick_word implies a due word").0;
            return Ok(self.word_card(&s)?.map(StudyItem::ReviewWord));
        }
        if let Some((g, _)) = glyph {
            return Ok(Some(StudyItem::ReviewGlyph(GlyphCard {
                is_consonant: g.chars().next().is_some_and(is_consonant),
                glyph: g,
            })));
        }
        Ok(None)
    }

    /// The not-yet-readable verse needing the fewest new words, tie-broken by
    /// those words being the most common. Biblical Aramaic verses are excluded
    /// (the Hebrew tutor never teaches toward them). `None` when every Hebrew
    /// verse is already fully known.
    fn next_target_verse(&self) -> rusqlite::Result<Option<(u8, u8, u8)>> {
        self.conn()
            .query_row(
                "SELECT vw.book, vw.chapter, vw.verse
                 FROM hebrewdb.verse_word vw
                 JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
                 LEFT JOIN progress.word_srs k
                        ON k.surface_id = vw.surface_id AND k.reps >= ?1
                 GROUP BY vw.book, vw.chapter, vw.verse
                 HAVING SUM(CASE WHEN s.language = 'aramaic' THEN 1 ELSE 0 END) = 0
                    AND COUNT(DISTINCT CASE WHEN k.surface_id IS NULL
                                            THEN vw.surface_id END) >= 1
                 -- Simplest = its rarest still-unknown word is as common as
                 -- possible (high reuse, easy to learn); then fewest new words.
                 ORDER BY MIN(CASE WHEN k.surface_id IS NULL
                                   THEN s.occurrences END) DESC,
                          COUNT(DISTINCT CASE WHEN k.surface_id IS NULL
                                              THEN vw.surface_id END) ASC,
                          vw.book, vw.chapter, vw.verse
                 LIMIT 1",
                params![KNOWN_REPS],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
    }

    /// The most common still-unknown word in a verse, if any remain.
    fn next_word_in_verse(&self, b: u8, c: u8, v: u8) -> rusqlite::Result<Option<WordCard>> {
        let surface: Option<String> = self
            .conn()
            .query_row(
                "SELECT s.text
                 FROM hebrewdb.verse_word vw
                 JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
                 LEFT JOIN progress.word_srs k
                        ON k.surface_id = vw.surface_id AND k.reps >= ?4
                 WHERE vw.book = ?1 AND vw.chapter = ?2 AND vw.verse = ?3
                   AND k.surface_id IS NULL
                 ORDER BY s.occurrences DESC
                 LIMIT 1",
                params![b, c, v, KNOWN_REPS],
                |r| r.get(0),
            )
            .optional()?;
        match surface {
            Some(s) => self.word_card(&s),
            None => Ok(None),
        }
    }

    /// Build a word card for a surface, resolving gloss/root/morph via the
    /// existing lexical bridge and flagging glyphs not yet introduced.
    fn word_card(&self, surface: &str) -> rusqlite::Result<Option<WordCard>> {
        let row: Option<(i64, i64)> = self
            .conn()
            .query_row(
                "SELECT surface_id, occurrences FROM hebrewdb.surface WHERE text = ?1",
                params![surface],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((surface_id, occurrences)) = row else {
            return Ok(None);
        };

        let (root, gloss, morph) = match self.hebrew_word_info(surface) {
            Some(w) => {
                let morph = [w.form.as_deref(), w.tense.as_deref(), w.person.as_deref(),
                             w.gender.as_deref(), w.number.as_deref(), w.state.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" ");
                (w.root, w.gloss, morph)
            }
            None => (String::new(), String::new(), String::new()),
        };

        // Which of the word's glyphs are not yet introduced.
        let mut new_glyphs = Vec::new();
        for g in decompose_glyphs(surface) {
            let known: bool = self.conn().query_row(
                "SELECT 1 FROM progress.glyph_srs WHERE glyph = ?1",
                params![g.glyph],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
            if !known {
                new_glyphs.push(g);
            }
        }

        Ok(Some(WordCard {
            surface_id,
            surface: surface.to_string(),
            occurrences,
            gloss,
            root,
            morph,
            new_glyphs,
        }))
    }

    fn meta_target(&self) -> rusqlite::Result<Option<(u8, u8, u8)>> {
        let v: Option<String> = self
            .conn()
            .query_row(
                "SELECT value FROM progress.meta WHERE key = 'target'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        Ok(v.and_then(|s| {
            let mut it = s.split('.').filter_map(|n| n.parse::<u8>().ok());
            Some((it.next()?, it.next()?, it.next()?))
        }))
    }

    fn set_meta_target(&self, t: Option<(u8, u8, u8)>) -> rusqlite::Result<()> {
        match t {
            Some((b, c, v)) => self.conn().execute(
                "INSERT INTO progress.meta(key, value) VALUES ('target', ?1) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![format!("{b}.{c}.{v}")],
            ),
            None => self
                .conn()
                .execute("DELETE FROM progress.meta WHERE key = 'target'", []),
        }
        .map(|_| ())
    }

    /// The heart of the flow: decide the learner's next card at time `now`
    /// (epoch seconds). Due reviews come first; otherwise we progress through
    /// the current target verse — introducing unseen glyphs, then drilling the
    /// most common unknown word — and present the verse to read once known.
    pub fn next_study_item(&self, now: i64) -> rusqlite::Result<StudyItem> {
        if let Some(review) = self.due_review(now)? {
            return Ok(review);
        }
        loop {
            match self.meta_target()? {
                None => match self.next_target_verse()? {
                    None => return Ok(StudyItem::Done),
                    Some(t) => self.set_meta_target(Some(t))?, // loop produces its first card
                },
                Some((b, c, v)) => match self.next_word_in_verse(b, c, v)? {
                    Some(word) => {
                        return Ok(match word.new_glyphs.first() {
                            Some(g) => StudyItem::NewGlyph(g.clone()),
                            None => StudyItem::NewWord(word),
                        });
                    }
                    None => {
                        // Before the reward read, teach any verse punctuation
                        // (sof pasuq / maqaf) the learner has not yet seen.
                        if let Some(mark) = self.next_unseen_reading_mark(b, c, v)? {
                            return Ok(StudyItem::NewGlyph(mark));
                        }
                        // Verse complete: record it, reward with a read, advance.
                        self.conn().execute(
                            "INSERT INTO progress.verse_progress(book, chapter, verse, state, last_read_epoch) \
                             VALUES (?1, ?2, ?3, 'readable', ?4) \
                             ON CONFLICT(book, chapter, verse) DO UPDATE SET \
                                state = 'readable', last_read_epoch = excluded.last_read_epoch",
                            params![b, c, v, now],
                        )?;
                        self.set_meta_target(None)?;
                        let examples = self.readable_examples(b, c, v, 3)?;
                        return Ok(StudyItem::ReadVerse(VerseCard {
                            book: b,
                            chapter: c,
                            verse: v,
                            examples,
                        }));
                    }
                },
            }
        }
    }

    /// Record a graded review for a glyph or word and return the next item.
    /// A glyph "intro" is just a `Good` grade on a fresh row.
    pub fn submit_review(&self, track: Track, key: &str, grade: Grade, now: i64) -> rusqlite::Result<StudyItem> {
        let prior: Option<(f64, i64, i64, i64)> = self
            .conn()
            .query_row(
                &format!(
                    "SELECT ease, interval_days, reps, lapses FROM {} WHERE {} = ?1",
                    track.table(),
                    track.key_col()
                ),
                params![key],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;

        let prev = prior
            .map(|(ease, interval_days, reps, lapses)| Srs {
                ease,
                interval_days,
                reps,
                lapses,
            })
            .unwrap_or_default();
        let introduced_new = prior.is_none();
        let next = prev.graded(grade);
        let grade_i = grade as i64;

        match track {
            Track::Word => {
                // surface_id is needed for verse-coverage joins.
                let surface_id: i64 = self.conn().query_row(
                    "SELECT surface_id FROM hebrewdb.surface WHERE text = ?1",
                    params![key],
                    |r| r.get(0),
                )?;
                self.conn().execute(
                    "INSERT INTO progress.word_srs(surface, surface_id, ease, interval_days, \
                        due_epoch, reps, lapses, introduced_epoch, last_grade) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
                     ON CONFLICT(surface) DO UPDATE SET ease=excluded.ease, \
                        interval_days=excluded.interval_days, due_epoch=excluded.due_epoch, \
                        reps=excluded.reps, lapses=excluded.lapses, last_grade=excluded.last_grade",
                    params![key, surface_id, next.ease, next.interval_days, next.due_at(now),
                            next.reps, next.lapses, now, grade_i],
                )?;
            }
            Track::Glyph => {
                self.conn().execute(
                    "INSERT INTO progress.glyph_srs(glyph, ease, interval_days, due_epoch, \
                        reps, lapses, introduced_epoch, last_grade) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                     ON CONFLICT(glyph) DO UPDATE SET ease=excluded.ease, \
                        interval_days=excluded.interval_days, due_epoch=excluded.due_epoch, \
                        reps=excluded.reps, lapses=excluded.lapses, last_grade=excluded.last_grade",
                    params![key, next.ease, next.interval_days, next.due_at(now),
                            next.reps, next.lapses, now, grade_i],
                )?;
            }
        }
        let _ = introduced_new;
        self.next_study_item(now)
    }

    /// The first [`READING_MARKS`] punctuation mark present in verse `(b,c,v)`'s
    /// display text that the learner has not yet been introduced to, if any.
    fn next_unseen_reading_mark(&self, b: u8, c: u8, v: u8) -> rusqlite::Result<Option<GlyphCard>> {
        let text = self.get(b, c, v)?;
        for mark in READING_MARKS {
            if !text.contains(mark) {
                continue;
            }
            let key = mark.to_string();
            let seen = self
                .conn()
                .query_row(
                    "SELECT 1 FROM progress.glyph_srs WHERE glyph = ?1",
                    params![key],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if !seen {
                return Ok(Some(GlyphCard {
                    glyph: key,
                    is_consonant: false,
                }));
            }
        }
        Ok(None)
    }

    /// Up to `limit` other verses sharing a word with `(b,c,v)` that are now
    /// fully readable (every word known) — example passages for reading practice.
    pub fn readable_examples(
        &self,
        b: u8,
        c: u8,
        v: u8,
        limit: i64,
    ) -> rusqlite::Result<Vec<(u8, u8, u8)>> {
        let mut stmt = self.conn().prepare(
            "SELECT DISTINCT vw2.book, vw2.chapter, vw2.verse
             FROM hebrewdb.verse_word vw1
             JOIN hebrewdb.verse_word vw2 ON vw2.surface_id = vw1.surface_id
             WHERE vw1.book = ?1 AND vw1.chapter = ?2 AND vw1.verse = ?3
               AND NOT (vw2.book = ?1 AND vw2.chapter = ?2 AND vw2.verse = ?3)
               AND NOT EXISTS (
                   SELECT 1 FROM hebrewdb.verse_word w3
                   LEFT JOIN progress.word_srs k
                          ON k.surface_id = w3.surface_id AND k.reps >= ?4
                   WHERE w3.book = vw2.book AND w3.chapter = vw2.chapter
                     AND w3.verse = vw2.verse AND k.surface_id IS NULL)
             LIMIT ?5",
        )?;
        let rows = stmt.query_map(params![b, c, v, KNOWN_REPS, limit], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;
        rows.collect()
    }

    /// Headline counters for a progress display.
    pub fn tutor_progress(&self) -> rusqlite::Result<TutorProgress> {
        let glyphs_known =
            self.conn()
                .query_row("SELECT COUNT(*) FROM progress.glyph_srs", [], |r| r.get(0))?;
        let words_known = self.conn().query_row(
            "SELECT COUNT(*) FROM progress.word_srs WHERE reps >= ?1",
            params![KNOWN_REPS],
            |r| r.get(0),
        )?;
        let verses_readable = self.conn().query_row(
            "SELECT COUNT(*) FROM progress.verse_progress WHERE state = 'readable'",
            [],
            |r| r.get(0),
        )?;
        let total_verses = self
            .conn()
            .query_row("SELECT COUNT(*) FROM hebrewdb.verse_stats", [], |r| r.get(0))?;
        Ok(TutorProgress {
            glyphs_known,
            words_known,
            verses_readable,
            total_verses,
        })
    }

    /// Wipe all tutor progress (settings included). For a dev/reset action.
    pub fn reset_tutor(&self) -> rusqlite::Result<()> {
        self.conn().execute_batch(
            "DELETE FROM progress.glyph_srs;
             DELETE FROM progress.word_srs;
             DELETE FROM progress.verse_progress;
             DELETE FROM progress.meta;",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sm2_growth_and_lapse() {
        let s = Srs::default();
        let s = s.graded(Grade::Good);
        assert_eq!(s.reps, 1);
        assert_eq!(s.interval_days, 1);
        let s = s.graded(Grade::Good);
        assert_eq!(s.reps, 2);
        assert_eq!(s.interval_days, 6);
        let s = s.graded(Grade::Good);
        assert_eq!(s.reps, 3);
        assert_eq!(s.interval_days, 15); // round(6 * 2.5)
        // A lapse resets reps and lowers ease, due immediately.
        let s = s.graded(Grade::Again);
        assert_eq!(s.reps, 0);
        assert_eq!(s.interval_days, 0);
        assert_eq!(s.lapses, 1);
        assert!(s.ease < DEFAULT_EASE);
    }

    #[test]
    fn glyph_decomposition_folds_finals_and_dedups() {
        // מֶלֶךְ — final kaf folds to medial; mem/lamed/kaf consonants + segols.
        let g = decompose_glyphs("מֶלֶךְ");
        let cons: Vec<&str> = g.iter().filter(|c| c.is_consonant).map(|c| c.glyph.as_str()).collect();
        assert_eq!(cons, vec!["מ", "ל", "כ"]); // final ך folded to כ, deduped
        assert!(g.iter().any(|c| !c.is_consonant)); // segol present
    }

    /// End-to-end against the in-repo data DBs: an empty progress.db should pick
    /// a sensible simplest first verse and walk glyph→word→read without panics.
    #[test]
    fn cold_start_flow() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        if !data.join("hebrew.db").exists() {
            return Ok(()); // data not built in this checkout; skip.
        }
        let bible = Bible::open(&data).expect("open data dbs");
        // A throwaway in-memory progress db per test run.
        bible.conn().execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let now = 1_700_000_000;
        let mut item = bible.next_study_item(now)?;
        // Cold start: nothing due, so it must be a new glyph or new word.
        assert!(matches!(item, StudyItem::NewGlyph(_) | StudyItem::NewWord(_)));
        // A target verse must have been chosen.
        assert!(bible.meta_target()?.is_some());

        // Always grading Good, chaining on submit_review's returned next item,
        // we should complete and read the first verse within a few dozen cards.
        let mut saw_read = false;
        for _ in 0..200 {
            item = match item {
                StudyItem::NewGlyph(g) | StudyItem::ReviewGlyph(g) => {
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                }
                StudyItem::NewWord(w) | StudyItem::ReviewWord(w) => {
                    bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
                }
                StudyItem::ReadVerse(_) => {
                    saw_read = true;
                    break;
                }
                StudyItem::Done => break,
            };
        }
        assert!(saw_read, "should complete and read the first verse");
        Ok(())
    }
}
