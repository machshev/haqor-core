//! Spaced-repetition reading tutor.
//!
//! A single never-ending study flow that teaches the learner to *read* the
//! Hebrew Bible, lazily introducing only what the next verse requires.
//!
//! The curriculum, per target word, is layered so the learner always builds on
//! what they can already read:
//! 1. **Glyphs** — introduce each unseen consonant/niqqud point, then drill it
//!    (a vowel as a nonsense syllable on a known consonant, e.g. בַ → "ba") with
//!    SM-2 until *known*.
//! 2. **Word reading** — once all the word's glyphs are known, drill reading the
//!    whole word (its vocalisation) until known.
//! 3. **Word meaning** — only then drill what the word means.
//!
//! Reviews are scheduled with a compact SM-2 with short in-session learning
//! steps (so recall actually happens within a sitting, not only the next day),
//! persisted in a writable `progress.db` (attached by
//! [`crate::bible::Bible::attach_progress`]). Static selection runs over
//! `hebrew.db`'s `verse_word` / `verse_stats` tables.

use rusqlite::{Connection, OptionalExtension, params};

use crate::bible::Bible;

/// SM-2 ease bounds.
const DEFAULT_EASE: f64 = 2.5;
const MIN_EASE: f64 = 1.3;

/// In-session learning steps (minutes) a card passes through before it
/// graduates to day-scale intervals. Two short steps mean a newly-taught item
/// comes back for recall within the same sitting.
const LEARN_STEPS_MIN: [i64; 2] = [1, 10];

const SECONDS_PER_DAY: i64 = 86_400;
const SECONDS_PER_MIN: i64 = 60;

/// Reading marks that punctuate verses but never appear inside a word surface:
/// the sof pasuq (verse-ending "full stop") and the maqaf (joins short words).
/// Taught from the verse itself, sof-pasuq first.
const READING_MARKS: [char; 2] = ['\u{05C3}', '\u{05BE}'];

/// Gutturals — the only consonants a hataf (reduced) vowel sits under.
const GUTTURALS: [&str; 4] = ["א", "ה", "ח", "ע"];
/// Clear, common consonants preferred when a vowel is shown in isolation; any
/// consonant is grammatical for an ordinary (non-hataf) vowel.
const CLEAR_HOSTS: [&str; 6] = ["מ", "ל", "נ", "ר", "ת", "ב"];

/// How the learner rated a card, mapped onto SM-2 behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    Again,
    Hard,
    Good,
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

/// The two things learned about a word, in order: how to *read* it (its
/// vocalisation), then what it *means*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordAspect {
    Read,
    Mean,
}

impl WordAspect {
    fn as_str(self) -> &'static str {
        match self {
            WordAspect::Read => "read",
            WordAspect::Mean => "mean",
        }
    }
}

/// Which review track a card belongs to. Words split into their two aspects so
/// each is scheduled independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Track {
    Glyph,
    WordRead,
    WordMean,
}

/// Mutable SM-2 state for one card. `interval_days == 0` means the card is still
/// in the short in-session learning steps; once it graduates, `interval_days` is
/// the day-scale spacing.
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
    /// Apply a grade. Successful recalls walk through [`LEARN_STEPS_MIN`] (still
    /// `interval_days == 0`), then graduate to 1-day, 6-day, then ease-scaled
    /// spacing. A lapse drops back into learning.
    fn graded(self, grade: Grade) -> Srs {
        let mut s = self;
        let steps = LEARN_STEPS_MIN.len() as i64;
        match grade {
            Grade::Again => {
                s.ease = (s.ease - 0.20).max(MIN_EASE);
                s.reps = 0;
                s.lapses += 1;
                s.interval_days = 0;
            }
            Grade::Hard => {
                s.ease = (s.ease - 0.15).max(MIN_EASE);
                if self.interval_days > 0 {
                    s.interval_days = ((self.interval_days as f64 * 1.2).round() as i64).max(1);
                }
                // While still in learning, Hard repeats the current step.
            }
            Grade::Good => {
                s.reps = self.reps + 1;
                if s.reps <= steps {
                    s.interval_days = 0; // still in the learning steps
                } else {
                    s.interval_days = match self.interval_days {
                        0 => 1,
                        1 => 6,
                        n => (n as f64 * self.ease).round() as i64,
                    };
                }
            }
            Grade::Easy => {
                s.ease = self.ease + 0.15;
                s.reps = (self.reps + 1).max(steps + 1); // jump past the learning steps
                s.interval_days = match self.interval_days {
                    0 => 4,
                    1 => 6,
                    n => (n as f64 * self.ease * 1.3).round() as i64,
                };
            }
        }
        s
    }

    /// Graduated past the in-session learning steps (i.e. genuinely "known").
    fn graduated(&self) -> bool {
        self.interval_days >= 1
    }

    /// Epoch-second due time after grading at `now`: a learning-step offset in
    /// minutes while learning, else the day-scale interval.
    fn due_at(&self, now: i64) -> i64 {
        if self.interval_days > 0 {
            now + self.interval_days * SECONDS_PER_DAY
        } else {
            let idx = (self.reps.max(1) - 1).clamp(0, LEARN_STEPS_MIN.len() as i64 - 1) as usize;
            now + LEARN_STEPS_MIN[idx] * SECONDS_PER_MIN
        }
    }
}

/// A teachable glyph: a single consonant (final forms folded) or a niqqud point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphCard {
    pub glyph: String,
    /// True for a base consonant, false for a vowel/dagesh/sin-shin point.
    pub is_consonant: bool,
    /// For a vowel point, an already-learnt consonant to display it on (so it is
    /// taught as a sounded-out syllable). None for consonants and reading marks.
    pub host: Option<String>,
}

/// A word to learn or review, for a particular [`WordAspect`].
#[derive(Debug, Clone)]
pub struct WordCard {
    pub surface_id: i64,
    pub surface: String,
    pub occurrences: i64,
    pub gloss: String,
    pub root: String,
    pub morph: String,
    pub aspect: WordAspect,
}

/// A fully-learnt verse offered to read, with other now-readable passages.
#[derive(Debug, Clone)]
pub struct VerseCard {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
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

/// Surface-ids fully learnt (both aspects graduated) — the "known" vocabulary
/// for verse coverage. A subquery reused across selection joins.
const DONE_SURFACES: &str = "SELECT surface_id FROM progress.word_srs \
     WHERE interval_days >= 1 GROUP BY surface_id HAVING COUNT(DISTINCT aspect) = 2";

/// Create the `progress.db` tables if they do not yet exist. Idempotent. A
/// pre-aspect `word_srs` (from before reading/meaning were split) is dropped and
/// rebuilt — word progress resets once, glyph progress is kept.
pub fn init_progress_schema(db: &Connection) -> rusqlite::Result<()> {
    let word_sql: Option<String> = db
        .query_row(
            "SELECT sql FROM progress.sqlite_master WHERE type='table' AND name='word_srs'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(sql) = word_sql {
        if !sql.contains("aspect") {
            db.execute_batch("DROP TABLE progress.word_srs")?;
        }
    }

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
            surface          TEXT    NOT NULL,
            aspect           TEXT    NOT NULL,
            surface_id       INTEGER NOT NULL,
            ease             REAL    NOT NULL,
            interval_days    INTEGER NOT NULL,
            due_epoch        INTEGER NOT NULL,
            reps             INTEGER NOT NULL,
            lapses           INTEGER NOT NULL,
            introduced_epoch INTEGER NOT NULL,
            last_grade       INTEGER NOT NULL,
            PRIMARY KEY (surface, aspect)
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

/// Fold a final-form consonant to its medial base so ך and כ are one glyph.
fn fold_final(c: char) -> char {
    match c {
        '\u{05DA}' => '\u{05DB}',
        '\u{05DD}' => '\u{05DE}',
        '\u{05DF}' => '\u{05E0}',
        '\u{05E3}' => '\u{05E4}',
        '\u{05E5}' => '\u{05E6}',
        other => other,
    }
}

fn is_consonant(c: char) -> bool {
    (0x05D0..=0x05EA).contains(&(c as u32))
}

/// A proper vowel point (sheva through holam, qubuts, qamats qatan) — taught on
/// a host consonant. Excludes dagesh and the shin/sin dots.
fn is_vowel_point(c: char) -> bool {
    matches!(c as u32, 0x05B0..=0x05B9 | 0x05BB | 0x05C7)
}

fn is_hataf(vowel: char) -> bool {
    matches!(vowel as u32, 0x05B1 | 0x05B2 | 0x05B3)
}

/// Preferred consonants that can legitimately carry `vowel`.
fn valid_host_prefs(vowel: char) -> &'static [&'static str] {
    if is_hataf(vowel) {
        &GUTTURALS
    } else {
        &CLEAR_HOSTS
    }
}

/// The consonant `vowel` sits on in `surface`: the nearest preceding base letter.
fn contextual_host(surface: &str, vowel: char) -> Option<String> {
    let mut on = None;
    for c in surface.chars() {
        if c == vowel {
            break;
        }
        if is_consonant(c) {
            on = Some(fold_final(c).to_string());
        }
    }
    on
}

/// The consonant to teach before `vowel` when no valid host is learnt yet.
fn host_to_teach(surface: &str, vowel: char) -> String {
    contextual_host(surface, vowel).unwrap_or_else(|| valid_host_prefs(vowel)[0].to_string())
}

/// Decompose a (normalized) surface into its distinct teachable glyphs in
/// first-seen order: consonants (finals folded) and niqqud points.
fn decompose_glyphs(surface: &str) -> Vec<GlyphCard> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in surface.chars() {
        let g = fold_final(c);
        let cons = is_consonant(g);
        if !cons
            && !matches!(
                g as u32,
                0x05B0..=0x05B9 | 0x05BB | 0x05BC | 0x05C1 | 0x05C2 | 0x05C7
            )
        {
            continue;
        }
        let key = g.to_string();
        if seen.insert(key.clone()) {
            out.push(GlyphCard {
                glyph: key,
                is_consonant: cons,
                host: None,
            });
        }
    }
    out
}

impl Bible {
    // --- low-level SRS state -------------------------------------------------

    fn glyph_srs(&self, glyph: &str) -> rusqlite::Result<Option<Srs>> {
        self.conn()
            .query_row(
                "SELECT ease, interval_days, reps, lapses FROM progress.glyph_srs WHERE glyph = ?1",
                params![glyph],
                |r| {
                    Ok(Srs {
                        ease: r.get(0)?,
                        interval_days: r.get(1)?,
                        reps: r.get(2)?,
                        lapses: r.get(3)?,
                    })
                },
            )
            .optional()
    }

    fn word_srs(&self, surface: &str, aspect: WordAspect) -> rusqlite::Result<Option<Srs>> {
        self.conn()
            .query_row(
                "SELECT ease, interval_days, reps, lapses FROM progress.word_srs \
                 WHERE surface = ?1 AND aspect = ?2",
                params![surface, aspect.as_str()],
                |r| {
                    Ok(Srs {
                        ease: r.get(0)?,
                        interval_days: r.get(1)?,
                        reps: r.get(2)?,
                        lapses: r.get(3)?,
                    })
                },
            )
            .optional()
    }

    fn glyph_known(&self, glyph: &str) -> rusqlite::Result<bool> {
        Ok(self.glyph_srs(glyph)?.is_some())
    }

    /// Every glyph of `surface` introduced *and* graduated — the gate for
    /// learning the whole word's reading.
    fn all_glyphs_graduated(&self, surface: &str) -> rusqlite::Result<bool> {
        for g in decompose_glyphs(surface) {
            match self.glyph_srs(&g.glyph)? {
                Some(s) if s.graduated() => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    // --- host selection for vowels ------------------------------------------

    fn known_vowel_host(&self, surface: &str, vowel: char) -> rusqlite::Result<Option<String>> {
        if let Some(ctx) = contextual_host(surface, vowel) {
            if self.glyph_known(&ctx)? {
                return Ok(Some(ctx));
            }
        }
        for g in valid_host_prefs(vowel) {
            if self.glyph_known(g)? {
                return Ok(Some(g.to_string()));
            }
        }
        if is_hataf(vowel) {
            return Ok(None);
        }
        self.conn()
            .query_row(
                "SELECT glyph FROM progress.glyph_srs \
                 WHERE unicode(glyph) BETWEEN 1488 AND 1514 LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
    }

    /// Build a NewGlyph item, showing a vowel on a learnt valid host (teaching a
    /// host consonant first if none is learnt yet).
    fn new_glyph_item(&self, surface: &str, g: &GlyphCard) -> rusqlite::Result<StudyItem> {
        let ch = g.glyph.chars().next().unwrap_or(' ');
        if !is_vowel_point(ch) {
            return Ok(StudyItem::NewGlyph(g.clone()));
        }
        match self.known_vowel_host(surface, ch)? {
            Some(host) => Ok(StudyItem::NewGlyph(GlyphCard {
                host: Some(host),
                ..g.clone()
            })),
            None => Ok(StudyItem::NewGlyph(GlyphCard {
                glyph: host_to_teach(surface, ch),
                is_consonant: true,
                host: None,
            })),
        }
    }

    fn review_glyph_card(&self, glyph: String) -> rusqlite::Result<GlyphCard> {
        let ch = glyph.chars().next();
        let host = match ch {
            Some(c) if is_vowel_point(c) => self.known_vowel_host("", c)?,
            _ => None,
        };
        Ok(GlyphCard {
            is_consonant: ch.is_some_and(is_consonant),
            glyph,
            host,
        })
    }

    // --- card builders -------------------------------------------------------

    /// Build a word card for `surface` and `aspect`, resolving gloss/root/morph.
    fn word_card(&self, surface: &str, aspect: WordAspect) -> rusqlite::Result<Option<WordCard>> {
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
                let morph = [
                    w.form.as_deref(),
                    w.tense.as_deref(),
                    w.person.as_deref(),
                    w.gender.as_deref(),
                    w.number.as_deref(),
                    w.state.as_deref(),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ");
                (w.root, w.gloss, morph)
            }
            None => (String::new(), String::new(), String::new()),
        };

        Ok(Some(WordCard {
            surface_id,
            surface: surface.to_string(),
            occurrences,
            gloss,
            root,
            morph,
            aspect,
        }))
    }

    // --- selection -----------------------------------------------------------

    /// The next not-fully-learnt verse needing the fewest new words, tie-broken
    /// by those words being the most common. Biblical Aramaic verses excluded.
    fn next_target_verse(&self) -> rusqlite::Result<Option<(u8, u8, u8)>> {
        self.conn()
            .query_row(
                &format!(
                    "SELECT vw.book, vw.chapter, vw.verse
                     FROM hebrewdb.verse_word vw
                     JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
                     LEFT JOIN ({DONE_SURFACES}) done ON done.surface_id = vw.surface_id
                     GROUP BY vw.book, vw.chapter, vw.verse
                     HAVING SUM(CASE WHEN s.language = 'aramaic' THEN 1 ELSE 0 END) = 0
                        AND COUNT(DISTINCT CASE WHEN done.surface_id IS NULL
                                                THEN vw.surface_id END) >= 1
                     ORDER BY MIN(CASE WHEN done.surface_id IS NULL THEN s.occurrences END) DESC,
                              COUNT(DISTINCT CASE WHEN done.surface_id IS NULL
                                                  THEN vw.surface_id END) ASC,
                              vw.book, vw.chapter, vw.verse
                     LIMIT 1"
                ),
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
    }

    /// The most common not-fully-learnt word in a verse, if any remain.
    fn first_unfinished_word(&self, b: u8, c: u8, v: u8) -> rusqlite::Result<Option<String>> {
        self.conn()
            .query_row(
                &format!(
                    "SELECT s.text
                     FROM hebrewdb.verse_word vw
                     JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
                     LEFT JOIN ({DONE_SURFACES}) done ON done.surface_id = vw.surface_id
                     WHERE vw.book = ?1 AND vw.chapter = ?2 AND vw.verse = ?3
                       AND done.surface_id IS NULL
                     ORDER BY s.occurrences DESC
                     LIMIT 1"
                ),
                params![b, c, v],
                |r| r.get(0),
            )
            .optional()
    }

    fn verse_done(&self, b: u8, c: u8, v: u8) -> rusqlite::Result<bool> {
        Ok(self.first_unfinished_word(b, c, v)?.is_none())
    }

    /// The next thing to *introduce* (teach) toward the target verse, working one
    /// word at a time: unseen glyphs, then — once all the word's glyphs are
    /// known — the word's reading, then its meaning. Returns None when the only
    /// outstanding work is graduating cards already in learning (handled by
    /// pulling a learning review forward).
    fn next_introduction(&self, b: u8, c: u8, v: u8) -> rusqlite::Result<Option<StudyItem>> {
        let Some(surface) = self.first_unfinished_word(b, c, v)? else {
            return Ok(None);
        };
        // 1. Introduce unseen glyphs.
        for g in decompose_glyphs(&surface) {
            if !self.glyph_known(&g.glyph)? {
                return Ok(Some(self.new_glyph_item(&surface, &g)?));
            }
        }
        // 2. Drill the word's glyphs to "known" before the word itself.
        if !self.all_glyphs_graduated(&surface)? {
            return Ok(None);
        }
        // 3. Word reading, then 4. word meaning.
        for aspect in [WordAspect::Read, WordAspect::Mean] {
            match self.word_srs(&surface, aspect)? {
                None => {
                    return Ok(self.word_card(&surface, aspect)?.map(StudyItem::NewWord));
                }
                Some(s) if !s.graduated() => return Ok(None), // still being learnt
                Some(_) => {}                                 // graduated; move to the next aspect
            }
        }
        Ok(None)
    }

    /// The next review card: the most-overdue introduced card (`pull_forward`
    /// false), or — to keep the session moving when nothing is strictly due —
    /// the soonest still-in-learning card (`pull_forward` true).
    fn next_review(&self, now: i64, pull_forward: bool) -> rusqlite::Result<Option<StudyItem>> {
        // While learning, pull the soonest learning card forward (ignore due);
        // otherwise take the most-overdue introduced card.
        let cond = if pull_forward {
            "reps > 0 AND interval_days = 0"
        } else {
            "reps > 0 AND due_epoch <= ?1"
        };
        let gsql = format!(
            "SELECT glyph, due_epoch FROM progress.glyph_srs WHERE {cond} \
             ORDER BY due_epoch ASC LIMIT 1"
        );
        let wsql = format!(
            "SELECT surface, aspect, due_epoch FROM progress.word_srs WHERE {cond} \
             ORDER BY due_epoch ASC LIMIT 1"
        );

        let gmap = |r: &rusqlite::Row| Ok((r.get(0)?, r.get(1)?));
        let wmap = |r: &rusqlite::Row| Ok((r.get(0)?, r.get(1)?, r.get(2)?));
        let (glyph, word): (Option<(String, i64)>, Option<(String, String, i64)>) = if pull_forward
        {
            (
                self.conn().query_row(&gsql, [], gmap).optional()?,
                self.conn().query_row(&wsql, [], wmap).optional()?,
            )
        } else {
            (
                self.conn()
                    .query_row(&gsql, params![now], gmap)
                    .optional()?,
                self.conn()
                    .query_row(&wsql, params![now], wmap)
                    .optional()?,
            )
        };

        // Whichever is more due wins; ties go to the word.
        let word_wins = match (&word, &glyph) {
            (Some((_, _, wd)), Some((_, gd))) => wd <= gd,
            (Some(_), None) => true,
            _ => false,
        };
        if word_wins {
            let (surface, aspect, _) = word.expect("word_wins implies a word");
            let aspect = if aspect == "mean" {
                WordAspect::Mean
            } else {
                WordAspect::Read
            };
            return Ok(self.word_card(&surface, aspect)?.map(StudyItem::ReviewWord));
        }
        if let Some((g, _)) = glyph {
            return Ok(Some(StudyItem::ReviewGlyph(self.review_glyph_card(g)?)));
        }
        Ok(None)
    }

    fn next_unseen_reading_mark(&self, b: u8, c: u8, v: u8) -> rusqlite::Result<Option<GlyphCard>> {
        let text = self.get(b, c, v)?;
        for mark in READING_MARKS {
            if !text.contains(mark) {
                continue;
            }
            let key = mark.to_string();
            if !self.glyph_known(&key)? {
                return Ok(Some(GlyphCard {
                    glyph: key,
                    is_consonant: false,
                    host: None,
                }));
            }
        }
        Ok(None)
    }

    // --- meta / flow ---------------------------------------------------------

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

    /// Decide the learner's next card at `now` (epoch seconds): due reviews
    /// first; else introduce the next new thing for the target verse; else pull
    /// an in-learning card forward to keep drilling; else read the finished verse.
    pub fn next_study_item(&self, now: i64) -> rusqlite::Result<StudyItem> {
        if let Some(review) = self.next_review(now, false)? {
            return Ok(review);
        }
        loop {
            let target = match self.meta_target()? {
                Some(t) => t,
                None => match self.next_target_verse()? {
                    Some(t) => {
                        self.set_meta_target(Some(t))?;
                        t
                    }
                    None => {
                        // Nothing new to learn; keep any in-learning cards going.
                        return Ok(self.next_review(now, true)?.unwrap_or(StudyItem::Done));
                    }
                },
            };
            let (b, c, v) = target;

            if let Some(item) = self.next_introduction(b, c, v)? {
                return Ok(item);
            }
            if !self.verse_done(b, c, v)? {
                // Words mid-learning: drill a learning card toward graduation.
                if let Some(review) = self.next_review(now, true)? {
                    return Ok(review);
                }
            }
            // Verse fully learnt: teach any unseen reading marks, then read it.
            if let Some(mark) = self.next_unseen_reading_mark(b, c, v)? {
                return Ok(StudyItem::NewGlyph(mark));
            }
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
    }

    /// Record a graded review and return the next item. The `track` selects the
    /// glyph store or a word aspect; `key` is the glyph or surface.
    pub fn submit_review(
        &self,
        track: Track,
        key: &str,
        grade: Grade,
        now: i64,
    ) -> rusqlite::Result<StudyItem> {
        let prev = match track {
            Track::Glyph => self.glyph_srs(key)?,
            Track::WordRead => self.word_srs(key, WordAspect::Read)?,
            Track::WordMean => self.word_srs(key, WordAspect::Mean)?,
        }
        .unwrap_or_default();
        let next = prev.graded(grade);
        let due = next.due_at(now);
        let grade_i = grade as i64;

        match track {
            Track::Glyph => {
                self.conn().execute(
                    "INSERT INTO progress.glyph_srs(glyph, ease, interval_days, due_epoch, \
                        reps, lapses, introduced_epoch, last_grade) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                     ON CONFLICT(glyph) DO UPDATE SET ease=excluded.ease, \
                        interval_days=excluded.interval_days, due_epoch=excluded.due_epoch, \
                        reps=excluded.reps, lapses=excluded.lapses, last_grade=excluded.last_grade",
                    params![
                        key,
                        next.ease,
                        next.interval_days,
                        due,
                        next.reps,
                        next.lapses,
                        now,
                        grade_i
                    ],
                )?;
            }
            Track::WordRead | Track::WordMean => {
                let aspect = if matches!(track, Track::WordMean) {
                    WordAspect::Mean
                } else {
                    WordAspect::Read
                };
                let surface_id: i64 = self.conn().query_row(
                    "SELECT surface_id FROM hebrewdb.surface WHERE text = ?1",
                    params![key],
                    |r| r.get(0),
                )?;
                self.conn().execute(
                    "INSERT INTO progress.word_srs(surface, aspect, surface_id, ease, \
                        interval_days, due_epoch, reps, lapses, introduced_epoch, last_grade) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
                     ON CONFLICT(surface, aspect) DO UPDATE SET ease=excluded.ease, \
                        interval_days=excluded.interval_days, due_epoch=excluded.due_epoch, \
                        reps=excluded.reps, lapses=excluded.lapses, last_grade=excluded.last_grade",
                    params![
                        key,
                        aspect.as_str(),
                        surface_id,
                        next.ease,
                        next.interval_days,
                        due,
                        next.reps,
                        next.lapses,
                        now,
                        grade_i
                    ],
                )?;
            }
        }
        self.next_study_item(now)
    }

    /// Up to `limit` other verses sharing a word with `(b,c,v)` that are now
    /// fully learnt (every word known) — example passages for reading practice.
    pub fn readable_examples(
        &self,
        b: u8,
        c: u8,
        v: u8,
        limit: i64,
    ) -> rusqlite::Result<Vec<(u8, u8, u8)>> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT DISTINCT vw2.book, vw2.chapter, vw2.verse
             FROM hebrewdb.verse_word vw1
             JOIN hebrewdb.verse_word vw2 ON vw2.surface_id = vw1.surface_id
             WHERE vw1.book = ?1 AND vw1.chapter = ?2 AND vw1.verse = ?3
               AND NOT (vw2.book = ?1 AND vw2.chapter = ?2 AND vw2.verse = ?3)
               AND NOT EXISTS (
                   SELECT 1 FROM hebrewdb.verse_word w3
                   LEFT JOIN ({DONE_SURFACES}) done ON done.surface_id = w3.surface_id
                   WHERE w3.book = vw2.book AND w3.chapter = vw2.chapter
                     AND w3.verse = vw2.verse AND done.surface_id IS NULL)
             LIMIT ?4"
        ))?;
        let rows = stmt.query_map(params![b, c, v, limit], |r| {
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
            &format!("SELECT COUNT(*) FROM ({DONE_SURFACES})"),
            [],
            |r| r.get(0),
        )?;
        let verses_readable = self.conn().query_row(
            "SELECT COUNT(*) FROM progress.verse_progress WHERE state = 'readable'",
            [],
            |r| r.get(0),
        )?;
        let total_verses =
            self.conn()
                .query_row("SELECT COUNT(*) FROM hebrewdb.verse_stats", [], |r| {
                    r.get(0)
                })?;
        Ok(TutorProgress {
            glyphs_known,
            words_known,
            verses_readable,
            total_verses,
        })
    }

    /// Wipe all tutor progress.
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
    fn sm2_learning_steps_then_graduation() {
        let s = Srs::default();
        // Two in-session learning steps (interval stays 0), then graduate.
        let s = s.graded(Grade::Good);
        assert_eq!((s.reps, s.interval_days), (1, 0));
        let s = s.graded(Grade::Good);
        assert_eq!((s.reps, s.interval_days), (2, 0));
        let s = s.graded(Grade::Good);
        assert_eq!((s.reps, s.interval_days), (3, 1)); // graduated to 1 day
        assert!(s.graduated());
        let s = s.graded(Grade::Good);
        assert_eq!(s.interval_days, 6);
        // A lapse drops back into learning.
        let s = s.graded(Grade::Again);
        assert_eq!((s.reps, s.interval_days), (0, 0));
        assert_eq!(s.lapses, 1);
        assert!(!s.graduated());
    }

    #[test]
    fn glyph_decomposition_folds_finals_and_dedups() {
        let g = decompose_glyphs("מֶלֶךְ");
        let cons: Vec<&str> = g
            .iter()
            .filter(|c| c.is_consonant)
            .map(|c| c.glyph.as_str())
            .collect();
        assert_eq!(cons, vec!["מ", "ל", "כ"]);
        assert!(g.iter().any(|c| !c.is_consonant));
    }

    /// End-to-end against the in-repo data DBs: cold start should walk
    /// glyph → syllable drill → word reading → meaning and eventually read the
    /// first verse, driven entirely by grading Good (pull-forward graduates the
    /// learning steps at a fixed `now`).
    #[test]
    fn cold_start_reaches_a_read() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let now = 1_700_000_000;
        let mut item = bible.next_study_item(now)?;
        assert!(matches!(
            item,
            StudyItem::NewGlyph(_) | StudyItem::NewWord(_)
        ));
        assert!(bible.meta_target()?.is_some());

        let mut saw_read = false;
        let mut saw_word_review = false;
        for _ in 0..4000 {
            item = match item {
                StudyItem::NewGlyph(g) | StudyItem::ReviewGlyph(g) => {
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                }
                StudyItem::NewWord(w) | StudyItem::ReviewWord(w) => {
                    if matches!(w.aspect, WordAspect::Read) {
                        saw_word_review = true;
                    }
                    let track = match w.aspect {
                        WordAspect::Read => Track::WordRead,
                        WordAspect::Mean => Track::WordMean,
                    };
                    bible.submit_review(track, &w.surface, Grade::Good, now)?
                }
                StudyItem::ReadVerse(_) => {
                    saw_read = true;
                    break;
                }
                StudyItem::Done => break,
            };
        }
        assert!(saw_word_review, "should drill word reading via SRS");
        assert!(saw_read, "should finish and read the first verse");
        Ok(())
    }
}
