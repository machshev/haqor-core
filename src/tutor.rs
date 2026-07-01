//! Spaced-repetition reading tutor.
//!
//! A single never-ending study flow that teaches the learner to *read* the
//! Hebrew Bible, lazily introducing only what the next verse requires.
//!
//! The curriculum, per target word, is layered so the learner always builds on
//! what they can already read:
//! 1. **Glyphs** — introduce each unseen consonant/niqqud point, then drill it
//!    with SM-2 until *known*. Vowels are drilled as **random nonsense syllables**
//!    (the vowel on a random already-known consonant, e.g. בַ → "ba"), quizzed
//!    against other random known syllables — so vocalisation is learnt from the
//!    letters themselves, never from reading whole real words. Bet/pe with a
//!    following dagesh (vet→bet, fe→pe) and shin with a following shin/sin dot
//!    (sh/s) change *sound*, so each pair is taught as two distinct letters
//!    rather than a base consonant plus a separately-drilled mark (see
//!    [`letter_identity`]); a dagesh elsewhere is pure gemination and isn't
//!    taught as its own glyph.
//! 2. **Word meaning** — once all a word's glyphs are known (so the learner can
//!    already sound it out), drill what the word means.
//!
//! Verse-punctuating reading marks (sof pasuq, maqaf) carry no sound of their
//! own, so they are shown once with an explanation the first time a verse
//! needs them and never drilled with spaced repetition (see
//! [`StudyItem::ExplainMark`]).
//!
//! Reviews are scheduled with a compact SM-2 with short in-session learning
//! steps (so recall actually happens within a sitting, not only the next day),
//! persisted in a writable `progress.db` (attached by
//! [`crate::bible::Bible::attach_progress`]). Static selection runs over
//! `hebrew.db`'s `verse_word` / `verse_stats` tables.

use rusqlite::{Connection, OptionalExtension, params};

use crate::bible::Bible;

/// A due glyph candidate `(glyph, due_epoch)`.
type GlyphRow = Option<(String, i64)>;
/// A due word candidate `(surface, due_epoch)`.
type WordRow = Option<(String, i64)>;

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

/// Consonants whose modern transliteration is a silent onset (aleph, ayin) —
/// never used as a syllable host, so a taught or quizzed syllable always sounds
/// a consonant instead of collapsing to a bare vowel.
const SILENT_HOSTS: [&str; 2] = ["א", "ע"];
/// Gutturals that carry a hataf (reduced) vowel *and* have an audible onset
/// (aleph and ayin are silent) — the hosts used to voice a hataf as a full
/// syllable.
const AUDIBLE_GUTTURALS: [&str; 2] = ["ה", "ח"];
/// Clear, common consonants preferred when a vowel is shown in isolation; any
/// audible consonant is grammatical for an ordinary (non-hataf) vowel.
const CLEAR_HOSTS: [&str; 6] = ["מ", "ל", "נ", "ר", "ת", "ב"];

/// A consonant whose transliteration is silent, so it must not host a drill
/// syllable (which would then read as just the vowel).
fn is_silent_host(cons: &str) -> bool {
    SILENT_HOSTS.contains(&cons)
}

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

    /// Map a self-assessed confidence (`0..=100`, from the grading slider) onto
    /// an SM-2 grade. For a multiple-choice answer pass `correct`: a wrong pick
    /// is always [`Grade::Again`] regardless of confidence (you didn't know it),
    /// while a correct pick is graded purely on confidence — so a lucky guess
    /// rated low still lapses rather than counting as known.
    pub fn from_confidence(confidence: u8, correct: Option<bool>) -> Grade {
        if correct == Some(false) {
            return Grade::Again;
        }
        match confidence {
            0..=24 => Grade::Again,
            25..=54 => Grade::Hard,
            55..=84 => Grade::Good,
            _ => Grade::Easy,
        }
    }
}

/// Which review track a card belongs to: an individual glyph (consonant or
/// vowel) or a whole word's meaning. Reading is never a word-level track —
/// vocalisation is learnt at the glyph/syllable level. Reading marks (sof
/// pasuq, maqaf) are neither — they are explained once, never drilled (see
/// [`StudyItem::ExplainMark`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Track {
    Glyph,
    Word,
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
    /// Other already-introduced glyphs of the same kind, offered as wrong
    /// answers when this card is quizzed multiple-choice. Empty on `New*` cards
    /// and whenever too few peers exist for a quiz (the app then self-grades).
    pub distractors: Vec<String>,
}

/// A word to learn or review. Words teach only meaning — by the time a word card
/// appears, all its glyphs are known so the learner can already sound it out.
#[derive(Debug, Clone)]
pub struct WordCard {
    pub surface_id: i64,
    pub surface: String,
    pub occurrences: i64,
    pub gloss: String,
    pub root: String,
    pub morph: String,
    /// Other plausible glosses offered as wrong answers when the meaning is
    /// quizzed multiple-choice. Empty when too few exist for a quiz (the app then
    /// self-grades).
    pub distractors: Vec<String>,
}

/// A fully-learnt verse offered to read, with other now-readable passages.
#[derive(Debug, Clone)]
pub struct VerseCard {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
    pub examples: Vec<(u8, u8, u8)>,
    /// The verse's words in reading order, as `word_srs` surface keys — lets
    /// the app let the learner flag which ones they misread, demoting just
    /// those (see [`Bible::verse_words`]).
    pub words: Vec<String>,
}

/// The next thing for the learner to do.
#[derive(Debug, Clone)]
pub enum StudyItem {
    NewGlyph(GlyphCard),
    ReviewGlyph(GlyphCard),
    NewWord(WordCard),
    ReviewWord(WordCard),
    /// A reading mark (sof pasuq, maqaf) shown with an explanation. Carries no
    /// grade — the app just acknowledges it and asks for the next item, like
    /// [`StudyItem::ReadVerse`]. Never revisited once shown.
    ExplainMark(GlyphCard),
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

/// Richer spaced-repetition statistics for the tutor stats view. Cheap indexed
/// counts over `progress.db`, computed on demand (not attached to every card
/// like [`TutorProgress`]). A card is *learning* while `interval_days == 0` (in
/// the short in-session steps) and *mature* once it graduates to day-scale
/// spacing; *seen* is every introduced card.
#[derive(Debug, Clone, Copy, Default)]
pub struct TutorStats {
    /// Letters/vowels/marks introduced, still in learning, and graduated.
    pub glyphs_seen: i64,
    pub glyphs_learning: i64,
    pub glyphs_mature: i64,
    /// Word meanings introduced, still in learning, and graduated.
    pub words_seen: i64,
    pub words_learning: i64,
    pub words_mature: i64,
    /// Cards whose next review is now due (`due_epoch <= now`).
    pub glyphs_due: i64,
    pub words_due: i64,
    /// Card answers logged today (UTC day) and over all time.
    pub reviews_today: i64,
    pub reviews_total: i64,
    /// Consecutive days (ending today or yesterday) with at least one review.
    pub streak_days: i64,
    /// Share of answers recalled (not "Again"), 0..=100; 0 when no reviews yet.
    pub accuracy_pct: i64,
    /// Verses every word of which is now known, out of the whole corpus.
    pub verses_readable: i64,
    pub total_verses: i64,
}

/// Surface-ids fully learnt (meaning graduated) — the "known" vocabulary for
/// verse coverage. A subquery reused across selection joins.
const DONE_SURFACES: &str = "SELECT surface_id FROM progress.word_srs \
     WHERE interval_days >= 1";

/// Create the `progress.db` tables if they do not yet exist. Idempotent. A
/// `word_srs` carrying the old per-aspect `aspect` column (from when reading and
/// meaning were separate word tracks) is dropped and rebuilt — word progress
/// resets once, glyph progress is kept.
pub fn init_progress_schema(db: &Connection) -> rusqlite::Result<()> {
    let word_sql: Option<String> = db
        .query_row(
            "SELECT sql FROM progress.sqlite_master WHERE type='table' AND name='word_srs'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(sql) = word_sql
        && sql.contains("aspect")
    {
        db.execute_batch("DROP TABLE progress.word_srs")?;
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
         );
         CREATE TABLE IF NOT EXISTS progress.reviews(
            epoch INTEGER NOT NULL,
            day   INTEGER NOT NULL,
            track TEXT    NOT NULL,
            grade INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS progress.idx_reviews_day ON reviews(day);
         CREATE TABLE IF NOT EXISTS progress.marks_seen(
            mark             TEXT PRIMARY KEY,
            introduced_epoch INTEGER NOT NULL
         );",
    )?;

    // Reading marks used to be drilled as ordinary glyphs before they were
    // switched to a one-time explanation (see `ExplainMark`). A leftover
    // `glyph_srs` row from that era makes a mark permanently eligible for
    // `next_review`'s pull-forward rotation, so it resurfaces as a quiz card
    // forever. Purge any such rows; harmless (and a no-op) once cleaned.
    for mark in READING_MARKS {
        db.execute(
            "DELETE FROM progress.glyph_srs WHERE glyph = ?1",
            params![mark.to_string()],
        )?;
    }

    // `letter_identity` used to stop scanning at the first mark after a
    // shin, so a geminated shin (dagesh *then* shin/sin dot, e.g.
    // אַשּׁוּר/הַשּׁוֹפָר) was mistaught as a bare, dotless שׁ instead of
    // folding the dot in. Purge that stale key; a correctly-dotted "שׁ"/"שׂ"
    // row is unaffected and gets re-introduced normally if missing.
    db.execute(
        "DELETE FROM progress.glyph_srs WHERE glyph = ?1",
        params![SHIN.to_string()],
    )?;
    Ok(())
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
    matches!(vowel as u32, 0x05B1..=0x05B3)
}

/// Bet and pe, whose dagesh changes the *sound* (vet→bet, fe→pe) rather than
/// just marking gemination — taught as two distinct letters, not a base
/// consonant plus a separately-drilled dagesh mark.
const DAGESH_LETTERS: [char; 2] = ['\u{05D1}', '\u{05E4}'];

/// Shin, whose following shin-dot or sin-dot picks between two distinct sounds
/// (sh / s) — taught as two distinct letters, not a base consonant plus a
/// separately-drilled dot.
const SHIN: char = '\u{05E9}';

fn is_dagesh(c: char) -> bool {
    c as u32 == 0x05BC
}

fn is_shin_sin_dot(c: char) -> bool {
    matches!(c as u32, 0x05C1 | 0x05C2)
}

/// The glyph identity of consonant `letter` given the mark cluster
/// immediately following it in `rest` (vowel points, a dagesh, and a
/// shin/sin dot, in any combination): for bet/pe a dagesh, or for shin a
/// shin/sin dot, changes the sound, so it is folded into the letter itself
/// and the pair is taught as one atomic glyph rather than a letter plus a
/// separately-drilled mark.
///
/// The source text's *Unicode canonical* combining order places a
/// consonant's vowel *before* its dagesh/shin-sin-dot (vowel points have a
/// lower combining class), not after as the traditional transliteration
/// order would suggest — e.g. הַשָּׁמַיִם encodes שׁ as shin, qamats, dagesh,
/// shin-dot. So this scans the whole run of marks attached to `letter`
/// (stopping at the next base consonant) rather than assuming the
/// identity-changing mark sits immediately next, and separately reports
/// which of those marks were vowel points so callers can still teach them.
/// Returns the glyph key, the vowel points found in the cluster (in
/// surface order), and how many of `rest`'s chars were consumed into it.
fn letter_cluster(letter: char, rest: &[char]) -> (String, Vec<char>, usize) {
    let mut vowels = Vec::new();
    let mut dagesh = None;
    let mut dot = None;
    let mut consumed = 0;
    for &c in rest {
        if is_vowel_point(c) {
            vowels.push(c);
        } else if is_dagesh(c) && dagesh.is_none() {
            dagesh = Some(c);
        } else if is_shin_sin_dot(c) && dot.is_none() {
            dot = Some(c);
        } else {
            break;
        }
        consumed += 1;
    }
    let key = if DAGESH_LETTERS.contains(&letter) {
        dagesh.map_or_else(|| letter.to_string(), |m| format!("{letter}{m}"))
    } else if letter == SHIN {
        dot.map_or_else(|| letter.to_string(), |m| format!("{letter}{m}"))
    } else {
        letter.to_string()
    };
    (key, vowels, consumed)
}

/// Preferred consonants that can legitimately carry `vowel`.
fn valid_host_prefs(vowel: char) -> &'static [&'static str] {
    if is_hataf(vowel) {
        &AUDIBLE_GUTTURALS
    } else {
        &CLEAR_HOSTS
    }
}

/// The consonant `vowel` sits on in `surface`: the base letter whose mark
/// cluster contains that vowel occurrence, with a dagesh/shin-sin-dot in the
/// same cluster folded into its identity (see [`letter_cluster`]).
fn contextual_host(surface: &str, vowel: char) -> Option<String> {
    let chars: Vec<char> = surface.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if is_consonant(c) {
            let (key, vowels, consumed) = letter_cluster(fold_final(c), &chars[i + 1..]);
            if vowels.contains(&vowel) {
                return Some(key);
            }
            i += consumed;
        }
        i += 1;
    }
    None
}

/// The consonant to teach before `vowel` when no valid host is learnt yet. A
/// silent contextual consonant (aleph/ayin) is skipped so the taught host voices
/// a full syllable.
fn host_to_teach(surface: &str, vowel: char) -> String {
    contextual_host(surface, vowel)
        .filter(|c| !is_silent_host(c))
        .unwrap_or_else(|| valid_host_prefs(vowel)[0].to_string())
}

/// The glyph SRS keys a graded card touches. A single-codepoint key (a lone
/// consonant, vowel, or reading mark) is graded as-is; a multi-codepoint
/// syllable key (`"<consonant><vowel>"`) grades every glyph in it — with a
/// consonant's dagesh/shin-sin-dot folded into it (see [`letter_cluster`])
/// rather than split out as its own glyph — so reading the syllable credits
/// its consonant *and* its vowel.
fn split_glyph_key(key: &str) -> Vec<String> {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 1 {
        return vec![key.to_string()];
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = fold_final(chars[i]);
        if is_consonant(c) {
            let (tok, vowels, consumed) = letter_cluster(c, &chars[i + 1..]);
            out.push(tok);
            out.extend(vowels.into_iter().map(|v| v.to_string()));
            i += 1 + consumed;
        } else {
            out.push(c.to_string());
            i += 1;
        }
    }
    out
}

/// Decompose a (normalized) surface into its distinct teachable glyphs in
/// first-seen order: consonants (finals folded, with a dagesh/shin-sin-dot
/// folded into begadkefat/shin letters — see [`letter_cluster`]) and vowel
/// points. A dagesh or shin/sin dot not folded into a letter this way is a
/// gemination/orthographic mark that doesn't change the sound and is not
/// taught as its own glyph.
fn decompose_glyphs(surface: &str) -> Vec<GlyphCard> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let chars: Vec<char> = surface.chars().map(fold_final).collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if is_consonant(c) {
            let (key, vowels, consumed) = letter_cluster(c, &chars[i + 1..]);
            if seen.insert(key.clone()) {
                out.push(GlyphCard {
                    glyph: key,
                    is_consonant: true,
                    host: None,
                    distractors: Vec::new(),
                });
            }
            for v in vowels {
                let vk = v.to_string();
                if seen.insert(vk.clone()) {
                    out.push(GlyphCard {
                        glyph: vk,
                        is_consonant: false,
                        host: None,
                        distractors: Vec::new(),
                    });
                }
            }
            i += 1 + consumed;
            continue;
        }
        if is_vowel_point(c) {
            let key = c.to_string();
            if seen.insert(key.clone()) {
                out.push(GlyphCard {
                    glyph: key,
                    is_consonant: false,
                    host: None,
                    distractors: Vec::new(),
                });
            }
        }
        i += 1;
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

    fn word_srs(&self, surface: &str) -> rusqlite::Result<Option<Srs>> {
        self.conn()
            .query_row(
                "SELECT ease, interval_days, reps, lapses FROM progress.word_srs \
                 WHERE surface = ?1",
                params![surface],
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
        // Prefer the consonant the vowel actually sits on in the word, but only
        // if it voices a syllable (not silent aleph/ayin).
        if let Some(ctx) = contextual_host(surface, vowel)
            && !is_silent_host(&ctx)
            && self.glyph_known(&ctx)?
        {
            return Ok(Some(ctx));
        }
        for g in valid_host_prefs(vowel) {
            if self.glyph_known(g)? {
                return Ok(Some(g.to_string()));
            }
        }
        if is_hataf(vowel) {
            return Ok(None);
        }
        // Any known audible consonant (aleph/ayin excluded).
        self.conn()
            .query_row(
                "SELECT glyph FROM progress.glyph_srs \
                 WHERE unicode(glyph) BETWEEN 1488 AND 1514 \
                   AND glyph NOT IN ('א','ע') LIMIT 1",
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
                distractors: Vec::new(),
            })),
        }
    }

    fn review_glyph_card(&self, glyph: String) -> rusqlite::Result<GlyphCard> {
        let ch = glyph.chars().next();
        // A vowel is drilled as a random nonsense syllable: it sits on a random
        // already-known (valid) consonant, quizzed against other random known
        // syllables. Consonants and marks quiz by name against same-kind peers.
        match ch {
            Some(c) if is_vowel_point(c) => {
                let host = self.random_vowel_host(c)?;
                let distractors = match &host {
                    Some(h) => self.syllable_distractors(h, c)?,
                    None => Vec::new(),
                };
                Ok(GlyphCard {
                    is_consonant: false,
                    glyph,
                    host,
                    distractors,
                })
            }
            _ => {
                let distractors = self.glyph_distractors(&glyph)?;
                Ok(GlyphCard {
                    is_consonant: ch.is_some_and(is_consonant),
                    glyph,
                    host: None,
                    distractors,
                })
            }
        }
    }

    /// A random already-known *audible* consonant that can legitimately carry
    /// `vowel` (audible gutturals ה/ח only for a hataf; aleph/ayin excluded as
    /// silent), for showing the vowel as a random full syllable. Falls back to
    /// the deterministic host picker if no random host qualifies.
    fn random_vowel_host(&self, vowel: char) -> rusqlite::Result<Option<String>> {
        let sql = if is_hataf(vowel) {
            "SELECT glyph FROM progress.glyph_srs \
             WHERE glyph IN ('ה','ח') ORDER BY RANDOM() LIMIT 1"
        } else {
            "SELECT glyph FROM progress.glyph_srs \
             WHERE unicode(glyph) BETWEEN 1488 AND 1514 \
               AND glyph NOT IN ('א','ע') ORDER BY RANDOM() LIMIT 1"
        };
        match self.conn().query_row(sql, [], |r| r.get(0)).optional()? {
            Some(h) => Ok(Some(h)),
            None => self.known_vowel_host("", vowel),
        }
    }

    /// Up to `WANT` random nonsense syllables built from already-known *audible*
    /// consonants and vowels, each a two-char `"<consonant><vowel>"` string, as
    /// wrong answers for a vowel's multiple-choice reading quiz. Silent hosts
    /// (aleph/ayin) are excluded so every option is a full syllable; a hataf
    /// vowel is only paired with an audible guttural (ה/ח); the exact
    /// `host`+`vowel` combo is excluded. The app transliterates and dedups, so a
    /// few extra are returned for margin.
    fn syllable_distractors(&self, host: &str, vowel: char) -> rusqlite::Result<Vec<String>> {
        const WANT: usize = 6;
        let mut out = Vec::new();
        // c is a known audible consonant (aleph/ayin excluded). v is a proper
        // vowel point (sheva..holam=1456..1465, qubuts=1467, qamats-qatan=1479) —
        // never a dagesh/sin-shin dot/mark that may also be in glyph_srs. A hataf
        // (1457..1459) is only paired with an audible guttural (ה/ח).
        let mut stmt = self.conn().prepare(
            "SELECT c.glyph || v.glyph \
             FROM progress.glyph_srs c \
             JOIN progress.glyph_srs v \
             WHERE unicode(c.glyph) BETWEEN 1488 AND 1514 \
               AND c.glyph NOT IN ('א','ע') \
               AND (unicode(v.glyph) BETWEEN 1456 AND 1465 \
                    OR unicode(v.glyph) IN (1467, 1479)) \
               AND NOT (unicode(v.glyph) BETWEEN 1457 AND 1459 \
                        AND c.glyph NOT IN ('ה','ח')) \
               AND NOT (c.glyph = ?1 AND v.glyph = ?2) \
             ORDER BY RANDOM() LIMIT ?3",
        )?;
        let rows = stmt.query_map(
            params![host, vowel.to_string(), WANT as i64],
            |r| r.get::<_, String>(0),
        )?;
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Up to three other already-introduced glyphs of the *same kind*
    /// (consonant / vowel point / reading mark) as `glyph`, for a
    /// multiple-choice quiz. Most-recently-introduced first; the app shuffles.
    fn glyph_distractors(&self, glyph: &str) -> rusqlite::Result<Vec<String>> {
        const WANT: usize = 3;
        let Some(ch) = glyph.chars().next() else {
            return Ok(Vec::new());
        };
        let cons = is_consonant(ch);
        let vowel = is_vowel_point(ch);
        let mut out = Vec::new();
        let mut stmt = self.conn().prepare(
            "SELECT glyph FROM progress.glyph_srs WHERE glyph != ?1 \
             ORDER BY introduced_epoch DESC",
        )?;
        let rows = stmt.query_map(params![glyph], |r| r.get::<_, String>(0))?;
        for row in rows {
            if out.len() >= WANT {
                break;
            }
            let g = row?;
            let Some(gc) = g.chars().next() else { continue };
            let same = if cons {
                is_consonant(gc)
            } else if vowel {
                is_vowel_point(gc)
            } else {
                !is_consonant(gc) && !is_vowel_point(gc)
            };
            if same {
                out.push(g);
            }
        }
        Ok(out)
    }

    /// Up to three plausible *other* glosses for a multiple-choice meaning quiz:
    /// meanings the learner has already studied first (familiar, so genuinely
    /// confusable), topped up with the most frequent words' glosses. Deduplicated
    /// against `gloss` and each other; the app adds the right answer and shuffles.
    fn meaning_distractors(&self, surface: &str, gloss: &str) -> rusqlite::Result<Vec<String>> {
        const WANT: usize = 3;
        let mut out: Vec<String> = Vec::new();
        if gloss.trim().is_empty() {
            return Ok(out);
        }
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        seen.insert(gloss.trim().to_lowercase());

        let mut candidates: Vec<String> = Vec::new();
        {
            let mut stmt = self.conn().prepare(
                "SELECT s.text FROM progress.word_srs ws \
                 JOIN hebrewdb.surface s ON s.surface_id = ws.surface_id \
                 WHERE s.text != ?1 \
                 ORDER BY ws.introduced_epoch DESC LIMIT 60",
            )?;
            let rows = stmt.query_map(params![surface], |r| r.get::<_, String>(0))?;
            for row in rows {
                candidates.push(row?);
            }
        }
        {
            let mut stmt = self.conn().prepare(
                "SELECT text FROM hebrewdb.surface \
                 WHERE text != ?1 AND n_candidates > 0 \
                 ORDER BY occurrences DESC LIMIT 80",
            )?;
            let rows = stmt.query_map(params![surface], |r| r.get::<_, String>(0))?;
            for row in rows {
                candidates.push(row?);
            }
        }

        for cand in candidates {
            if out.len() >= WANT {
                break;
            }
            if let Some(w) = self.hebrew_word_info(&cand) {
                let g = w.gloss.trim().to_string();
                if g.is_empty() {
                    continue;
                }
                if seen.insert(g.to_lowercase()) {
                    out.push(g);
                }
            }
        }
        Ok(out)
    }

    // --- card builders -------------------------------------------------------

    /// Build a meaning word card for `surface`, resolving gloss/root/morph.
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

        let distractors = self.meaning_distractors(surface, &gloss)?;

        Ok(Some(WordCard {
            surface_id,
            surface: surface.to_string(),
            occurrences,
            gloss,
            root,
            morph,
            distractors,
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

    /// Every not-fully-learnt word in a verse, most common first. Unlike
    /// [`Self::first_unfinished_word`] (used only to test verse completion),
    /// this backs [`Self::next_introduction`]'s search for *something* left to
    /// teach — so a word already mid-learning doesn't block introducing a
    /// different word in the same verse.
    fn unfinished_words(&self, b: u8, c: u8, v: u8) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT s.text
             FROM hebrewdb.verse_word vw
             JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
             LEFT JOIN ({DONE_SURFACES}) done ON done.surface_id = vw.surface_id
             WHERE vw.book = ?1 AND vw.chapter = ?2 AND vw.verse = ?3
               AND done.surface_id IS NULL
             ORDER BY s.occurrences DESC"
        ))?;
        stmt.query_map(params![b, c, v], |r| r.get(0))?.collect()
    }

    /// The next thing to *introduce* (teach) toward the target verse: unseen
    /// glyphs, then — once a word's glyphs are all known (so it can be sounded
    /// out) — that word's meaning. Tries every not-fully-learnt word in the
    /// verse, most common first, rather than stopping at the first one that
    /// happens to already be mid-learning — otherwise the active card pool
    /// never grows past whichever word is currently graduating (e.g. a
    /// frequent word like the divine name, which always sorts first) and the
    /// learner just keeps re-drilling it. Returns None only when every word is
    /// either graduated or already fully introduced and mid-learning — the
    /// remaining work is graduating cards already in learning (handled by
    /// pulling a learning review forward).
    fn next_introduction(&self, b: u8, c: u8, v: u8) -> rusqlite::Result<Option<StudyItem>> {
        for surface in self.unfinished_words(b, c, v)? {
            // 1. Introduce unseen glyphs.
            for g in decompose_glyphs(&surface) {
                if !self.glyph_known(&g.glyph)? {
                    return Ok(Some(self.new_glyph_item(&surface, &g)?));
                }
            }
            // 2. Drill this word's glyphs to "known" before the word itself;
            // try the next word instead of giving up.
            if !self.all_glyphs_graduated(&surface)? {
                continue;
            }
            // 3. Word meaning (reading is already covered by the glyph/syllable
            // drill). Already introduced (in learning or graduated) — try the
            // next word instead of giving up.
            if self.word_srs(&surface)?.is_none() {
                return Ok(self.word_card(&surface)?.map(StudyItem::NewWord));
            }
        }
        Ok(None)
    }

    /// The next review card: the most-overdue introduced card (`pull_forward`
    /// false), or — to keep the session moving when nothing is strictly due —
    /// the longest-waiting still-in-learning card (`pull_forward` true).
    fn next_review(&self, now: i64, pull_forward: bool) -> rusqlite::Result<Option<StudyItem>> {
        // No `reps > 0` guard on either query: a lapse (`Grade::Again`) resets
        // `reps` to 0 on a card that's very much still in the table and due
        // for a re-drill, so filtering on it stranded freshly-lapsed cards —
        // never due, never pulled forward, never re-introduced (a row already
        // exists) — permanently.
        //
        // Pull-forward orders by `introduced_epoch`, not `due_epoch`: a card
        // repeatedly graded Again/Hard keeps resetting to the *shortest*
        // learning step (`Srs::due_at`'s `reps == 0` case), so ordering by
        // due_epoch would let it perpetually cut back to the front ahead of
        // siblings that have made real progress (and so sit at a later,
        // farther-out step) — starving them of the reviews they need to
        // graduate and freezing the whole verse on the one stuck card.
        // `introduced_epoch` is set once and never bumped by a re-grade, so it
        // round-robins fairly by first-introduced order, while still
        // eventually returning the stuck card once it's the only one left in
        // the learning pool.
        let cond = if pull_forward {
            "interval_days = 0"
        } else {
            "due_epoch <= ?1"
        };
        let order_col = if pull_forward {
            "introduced_epoch"
        } else {
            "due_epoch"
        };
        let gsql = format!(
            "SELECT glyph, {order_col} FROM progress.glyph_srs WHERE {cond} \
             ORDER BY {order_col} ASC LIMIT 1"
        );
        let wsql = format!(
            "SELECT surface, {order_col} FROM progress.word_srs WHERE {cond} \
             ORDER BY {order_col} ASC LIMIT 1"
        );

        let gmap = |r: &rusqlite::Row| Ok((r.get(0)?, r.get(1)?));
        let wmap = |r: &rusqlite::Row| Ok((r.get(0)?, r.get(1)?));
        let (glyph, word): (GlyphRow, WordRow) = if pull_forward {
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
            (Some((_, wd)), Some((_, gd))) => wd <= gd,
            (Some(_), None) => true,
            _ => false,
        };
        if word_wins {
            let (surface, _) = word.expect("word_wins implies a word");
            return Ok(self.word_card(&surface)?.map(StudyItem::ReviewWord));
        }
        if let Some((g, _)) = glyph {
            return Ok(Some(StudyItem::ReviewGlyph(self.review_glyph_card(g)?)));
        }
        Ok(None)
    }

    /// Whether a reading mark has already been shown (tracked in
    /// `progress.marks_seen`, distinct from `glyph_srs`, since reading marks
    /// are never drilled).
    fn mark_seen(&self, mark: &str) -> rusqlite::Result<bool> {
        Ok(self
            .conn()
            .query_row(
                "SELECT 1 FROM progress.marks_seen WHERE mark = ?1",
                params![mark],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    fn next_unseen_reading_mark(&self, b: u8, c: u8, v: u8) -> rusqlite::Result<Option<GlyphCard>> {
        let text = self.get(b, c, v)?;
        for mark in READING_MARKS {
            if !text.contains(mark) {
                continue;
            }
            let key = mark.to_string();
            if !self.mark_seen(&key)? {
                return Ok(Some(GlyphCard {
                    glyph: key,
                    is_consonant: false,
                    host: None,
                    distractors: Vec::new(),
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
        // Verse fully learnt: explain any unseen reading marks, then read it.
        // Recorded as seen immediately (never drilled), mirroring how the
        // verse itself is marked readable below.
        if let Some(mark) = self.next_unseen_reading_mark(b, c, v)? {
            self.conn().execute(
                "INSERT INTO progress.marks_seen(mark, introduced_epoch) VALUES (?1, ?2) \
                 ON CONFLICT(mark) DO NOTHING",
                params![mark.glyph, now],
            )?;
            return Ok(StudyItem::ExplainMark(mark));
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
        let words = self.verse_words(b, c, v)?;
        Ok(StudyItem::ReadVerse(VerseCard {
            book: b,
            chapter: c,
            verse: v,
            examples,
            words,
        }))
    }

    /// Record a graded review and return the next item. `track` selects the glyph
    /// store or the word store; `key` is a surface (word) or a glyph. A glyph key
    /// may be a whole syllable (`"<consonant><vowel>"`): reading it correctly
    /// demonstrates every glyph in it, so **each** glyph is graded, not just the
    /// drilled vowel.
    pub fn submit_review(
        &self,
        track: Track,
        key: &str,
        grade: Grade,
        now: i64,
    ) -> rusqlite::Result<StudyItem> {
        let grade_i = grade as i64;

        match track {
            Track::Glyph => {
                for glyph in split_glyph_key(key) {
                    // Reading marks are explained once via `ExplainMark`, never
                    // drilled — guard against a client mistakenly grading one
                    // (or a stale key) from ever re-entering `glyph_srs`.
                    if glyph.chars().count() == 1
                        && READING_MARKS.contains(&glyph.chars().next().unwrap())
                    {
                        continue;
                    }
                    let next = self.glyph_srs(&glyph)?.unwrap_or_default().graded(grade);
                    self.conn().execute(
                        "INSERT INTO progress.glyph_srs(glyph, ease, interval_days, due_epoch, \
                            reps, lapses, introduced_epoch, last_grade) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                         ON CONFLICT(glyph) DO UPDATE SET ease=excluded.ease, \
                            interval_days=excluded.interval_days, due_epoch=excluded.due_epoch, \
                            reps=excluded.reps, lapses=excluded.lapses, last_grade=excluded.last_grade",
                        params![
                            glyph,
                            next.ease,
                            next.interval_days,
                            next.due_at(now),
                            next.reps,
                            next.lapses,
                            now,
                            grade_i
                        ],
                    )?;
                }
            }
            Track::Word => {
                let next = self.word_srs(key)?.unwrap_or_default().graded(grade);
                let due = next.due_at(now);
                let surface_id: i64 = self.conn().query_row(
                    "SELECT surface_id FROM hebrewdb.surface WHERE text = ?1",
                    params![key],
                    |r| r.get(0),
                )?;
                self.conn().execute(
                    "INSERT INTO progress.word_srs(surface, surface_id, ease, \
                        interval_days, due_epoch, reps, lapses, introduced_epoch, last_grade) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
                     ON CONFLICT(surface) DO UPDATE SET ease=excluded.ease, \
                        interval_days=excluded.interval_days, due_epoch=excluded.due_epoch, \
                        reps=excluded.reps, lapses=excluded.lapses, last_grade=excluded.last_grade",
                    params![
                        key,
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
        // Log one review event per card answer (a syllable card grades several
        // glyphs but is a single answer) for streak / activity / accuracy stats.
        let track_str = match track {
            Track::Glyph => "glyph",
            Track::Word => "word",
        };
        self.conn().execute(
            "INSERT INTO progress.reviews(epoch, day, track, grade) VALUES (?1, ?2, ?3, ?4)",
            params![now, now.div_euclid(SECONDS_PER_DAY), track_str, grade_i],
        )?;
        self.next_study_item(now)
    }

    /// A verse's words in reading order, as `word_srs` surface keys — so the
    /// app can offer them for the learner to flag ones they misread.
    pub fn verse_words(&self, b: u8, c: u8, v: u8) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self.conn().prepare(
            "SELECT s.text
             FROM hebrewdb.verse_word vw
             JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
             WHERE vw.book = ?1 AND vw.chapter = ?2 AND vw.verse = ?3
             ORDER BY vw.position",
        )?;
        stmt.query_map(params![b, c, v], |r| r.get(0))?.collect()
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

    /// Richer SRS statistics for the stats view: learning/mature splits, cards
    /// due, activity (reviews today/total), streak, accuracy, and reading
    /// coverage. All cheap indexed counts over `progress.db`.
    pub fn tutor_stats(&self, now: i64) -> rusqlite::Result<TutorStats> {
        let conn = self.conn();
        let count = |sql: &str| -> rusqlite::Result<i64> { conn.query_row(sql, [], |r| r.get(0)) };

        let glyphs_seen = count("SELECT COUNT(*) FROM progress.glyph_srs")?;
        let glyphs_mature =
            count("SELECT COUNT(*) FROM progress.glyph_srs WHERE interval_days >= 1")?;
        let words_seen = count("SELECT COUNT(*) FROM progress.word_srs")?;
        let words_mature =
            count("SELECT COUNT(*) FROM progress.word_srs WHERE interval_days >= 1")?;

        let glyphs_due = conn.query_row(
            "SELECT COUNT(*) FROM progress.glyph_srs WHERE due_epoch <= ?1",
            params![now],
            |r| r.get(0),
        )?;
        let words_due = conn.query_row(
            "SELECT COUNT(*) FROM progress.word_srs WHERE due_epoch <= ?1",
            params![now],
            |r| r.get(0),
        )?;

        let day_now = now.div_euclid(SECONDS_PER_DAY);
        let reviews_today = conn.query_row(
            "SELECT COUNT(*) FROM progress.reviews WHERE day = ?1",
            params![day_now],
            |r| r.get(0),
        )?;
        let reviews_total = count("SELECT COUNT(*) FROM progress.reviews")?;
        let recalled = count("SELECT COUNT(*) FROM progress.reviews WHERE grade > 0")?;
        let accuracy_pct = if reviews_total > 0 {
            recalled * 100 / reviews_total
        } else {
            0
        };

        let verses_readable =
            count("SELECT COUNT(*) FROM progress.verse_progress WHERE state = 'readable'")?;
        let total_verses = count("SELECT COUNT(*) FROM hebrewdb.verse_stats")?;

        Ok(TutorStats {
            glyphs_seen,
            glyphs_learning: glyphs_seen - glyphs_mature,
            glyphs_mature,
            words_seen,
            words_learning: words_seen - words_mature,
            words_mature,
            glyphs_due,
            words_due,
            reviews_today,
            reviews_total,
            streak_days: self.review_streak(day_now)?,
            accuracy_pct,
            verses_readable,
            total_verses,
        })
    }

    /// Consecutive review days ending on `day_now` (or `day_now - 1`, so the
    /// streak is not shown as broken until a whole day is missed).
    fn review_streak(&self, day_now: i64) -> rusqlite::Result<i64> {
        let mut stmt = self
            .conn()
            .prepare("SELECT DISTINCT day FROM progress.reviews ORDER BY day DESC")?;
        let days: Vec<i64> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        // Anchor on today if studied today, else yesterday — a still-alive streak
        // that just hasn't been continued yet today.
        let mut expected = match days.first() {
            Some(&d) if d == day_now => day_now,
            _ => day_now - 1,
        };
        let mut streak = 0;
        for d in days {
            if d == expected {
                streak += 1;
                expected -= 1;
            } else if d < expected {
                break;
            }
        }
        Ok(streak)
    }

    /// Wipe all tutor progress.
    pub fn reset_tutor(&self) -> rusqlite::Result<()> {
        self.conn().execute_batch(
            "DELETE FROM progress.glyph_srs;
             DELETE FROM progress.word_srs;
             DELETE FROM progress.verse_progress;
             DELETE FROM progress.meta;
             DELETE FROM progress.reviews;
             DELETE FROM progress.marks_seen;",
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
    fn confidence_maps_to_grades_and_quiz_gates() {
        use Grade::*;
        // Self-grade slider buckets.
        assert_eq!(Grade::from_confidence(0, None), Again);
        assert_eq!(Grade::from_confidence(24, None), Again);
        assert_eq!(Grade::from_confidence(25, None), Hard);
        assert_eq!(Grade::from_confidence(54, None), Hard);
        assert_eq!(Grade::from_confidence(55, None), Good);
        assert_eq!(Grade::from_confidence(84, None), Good);
        assert_eq!(Grade::from_confidence(85, None), Easy);
        assert_eq!(Grade::from_confidence(100, None), Easy);
        // A wrong multiple-choice pick always lapses, however confident.
        assert_eq!(Grade::from_confidence(100, Some(false)), Again);
        // A correct pick is graded on confidence — a low-confidence (lucky) hit
        // still lapses rather than counting as known.
        assert_eq!(Grade::from_confidence(90, Some(true)), Easy);
        assert_eq!(Grade::from_confidence(10, Some(true)), Again);
    }

    #[test]
    fn vowel_review_builds_random_syllable_distractors() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        // Seed audible consonants, audible gutturals (for hataf), plus a silent
        // guttural (ע) and vowels incl. a hataf.
        let now = 1_700_000_000;
        for g in ["מ", "ל", "ר", "ה", "ח", "ע", "ַ", "ֶ", "ֲ"] {
            bible.submit_review(Track::Glyph, g, Grade::Easy, now)?;
        }

        // A non-hataf vowel: distractor syllables are consonant+vowel pairs, on an
        // audible consonant (never silent aleph/ayin), none equal to the correct
        // combo, and never a bare glyph.
        let card = bible.review_glyph_card("ַ".to_string())?;
        let host = card.host.clone().expect("vowel gets a host");
        assert!(!is_silent_host(&host), "host voices a syllable: {host}");
        assert!(!card.distractors.is_empty(), "should offer syllables");
        for d in &card.distractors {
            let cps: Vec<char> = d.chars().collect();
            assert_eq!(cps.len(), 2, "syllable is consonant+vowel: {d:?}");
            assert!(is_consonant(cps[0]) && !is_silent_host(&cps[0].to_string()));
            assert!(is_vowel_point(cps[1]));
            assert_ne!(*d, format!("{host}ַ"), "excludes the correct syllable");
        }

        // Distractors are random syllables; whenever one uses a hataf vowel it is
        // paired only with an audible guttural (ה/ח), and never a silent host.
        let hataf = bible.syllable_distractors("ה", 'ֲ')?;
        assert!(!hataf.is_empty(), "hataf card should still offer syllables");
        for d in &hataf {
            let cps: Vec<char> = d.chars().collect();
            assert!(!is_silent_host(&cps[0].to_string()));
            if is_hataf(cps[1]) {
                assert!(AUDIBLE_GUTTURALS.contains(&cps[0].to_string().as_str()));
            }
        }
        Ok(())
    }

    #[test]
    fn tutor_stats_track_activity_streak_and_accuracy() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let day = SECONDS_PER_DAY;
        // Day 0: two answers within the same UTC day — one recalled, one lapse.
        bible.submit_review(Track::Glyph, "מ", Grade::Good, 0)?;
        bible.submit_review(Track::Glyph, "ל", Grade::Again, 100)?;
        // Days 1 and 2: one answer each — a 3-day streak ending "today" (day 2).
        bible.submit_review(Track::Glyph, "מ", Grade::Good, day)?;
        bible.submit_review(Track::Glyph, "מ", Grade::Good, 2 * day)?;

        let s = bible.tutor_stats(2 * day)?;
        assert_eq!(s.reviews_total, 4);
        assert_eq!(s.reviews_today, 1, "only the day-2 answer counts as today");
        assert_eq!(s.streak_days, 3, "days 0, 1 and 2 are consecutive");
        assert_eq!(s.accuracy_pct, 75, "3 of 4 answers recalled");
        assert_eq!(s.glyphs_seen, 2, "two distinct glyphs introduced");

        // A whole missed day breaks the streak: from day 4, day 2 is stale.
        assert_eq!(bible.tutor_stats(4 * day)?.streak_days, 0);
        // Studying "yesterday" (day 3) keeps the run 0..=3 alive today (day 4),
        // even though today has no review yet — a 4-day streak.
        bible.submit_review(Track::Glyph, "ל", Grade::Good, 3 * day)?;
        assert_eq!(bible.tutor_stats(4 * day)?.streak_days, 4);
        Ok(())
    }

    #[test]
    fn grading_a_syllable_credits_every_glyph() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        // Reading the syllable מַ correctly credits BOTH the consonant and vowel.
        let now = 1_700_000_000;
        bible.submit_review(Track::Glyph, "מַ", Grade::Good, now)?;
        let m = bible.glyph_srs("מ")?.expect("consonant credited");
        let a = bible.glyph_srs("ַ")?.expect("vowel credited");
        assert_eq!(m.reps, 1);
        assert_eq!(a.reps, 1);

        // A lone glyph key still grades just that glyph.
        assert_eq!(split_glyph_key("ל"), vec!["ל".to_string()]);
        // Final forms fold when a syllable is split.
        assert_eq!(split_glyph_key("ךַ"), vec!["כ".to_string(), "ַ".to_string()]);
        Ok(())
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

    #[test]
    fn dagesh_and_shin_sin_dot_fold_into_letter_identity() {
        // Traditional combining order: letter → dagesh → shin/sin dot → vowel
        // (see morphology/hebrew.rs).
        const BET: char = '\u{05D1}';
        const GIMEL: char = '\u{05D2}';
        const NUN: char = '\u{05E0}';
        const RESH: char = '\u{05E8}';
        const ALEF: char = '\u{05D0}';
        const SHIN: char = '\u{05E9}';
        const MEM: char = '\u{05DE}';
        const DAGESH: char = '\u{05BC}';
        const SHIN_DOT: char = '\u{05C1}';
        const SIN_DOT: char = '\u{05C2}';
        const QAMATS: char = '\u{05B8}';
        const PATAH: char = '\u{05B7}';

        // בּ (bet, plosive) is taught as a letter distinct from bare ב (vet).
        let bet = decompose_glyphs(&format!("{BET}{DAGESH}{QAMATS}{RESH}{QAMATS}{ALEF}"));
        let cons: Vec<&str> = bet
            .iter()
            .filter(|c| c.is_consonant)
            .map(|c| c.glyph.as_str())
            .collect();
        assert_eq!(cons, vec![format!("{BET}{DAGESH}"), RESH.to_string(), ALEF.to_string()]);

        // ש with a sin-dot is taught as sin, distinct from ש with a shin-dot.
        let sin = decompose_glyphs(&format!("{SHIN}{SIN_DOT}{QAMATS}{MEM}"));
        assert!(sin
            .iter()
            .any(|c| c.is_consonant && c.glyph == format!("{SHIN}{SIN_DOT}")));
        let shin = decompose_glyphs(&format!("{SHIN}{SHIN_DOT}{QAMATS}{MEM}"));
        assert!(shin
            .iter()
            .any(|c| c.is_consonant && c.glyph == format!("{SHIN}{SHIN_DOT}")));

        // A geminated shin (dagesh forte, e.g. the assimilated definite
        // article in אַשּׁוּר/הַשּׁוֹפָר) carries *both* a dagesh and a shin/sin
        // dot, in that order — the dagesh must not stop the scan from finding
        // the dot, or the doubled letter is mistaught as a dotless bare שׁ.
        let geminated =
            decompose_glyphs(&format!("{ALEF}{PATAH}{SHIN}{DAGESH}{SHIN_DOT}{QAMATS}{RESH}"));
        let cons: Vec<&str> = geminated
            .iter()
            .filter(|c| c.is_consonant)
            .map(|c| c.glyph.as_str())
            .collect();
        assert_eq!(
            cons,
            vec![ALEF.to_string(), format!("{SHIN}{SHIN_DOT}"), RESH.to_string()]
        );

        // Real Bible text puts a consonant's vowel *before* its
        // dagesh/shin-sin-dot (Unicode canonical combining order sorts vowel
        // points ahead of the dagesh/dot classes) — the opposite of the
        // traditional transliteration order used above. E.g. הַשָּׁמַיִם
        // ("the heavens") encodes its שׁ as shin, qamats, dagesh, shin-dot.
        let real_order = decompose_glyphs(&format!(
            "{}{PATAH}{SHIN}{QAMATS}{DAGESH}{SHIN_DOT}{MEM}{PATAH}{RESH}",
            ALEF
        ));
        let cons: Vec<&str> = real_order
            .iter()
            .filter(|c| c.is_consonant)
            .map(|c| c.glyph.as_str())
            .collect();
        assert_eq!(
            cons,
            vec![ALEF.to_string(), format!("{SHIN}{SHIN_DOT}"), MEM.to_string(), RESH.to_string()],
            "vowel-before-dagesh/dot ordering must still fold the shin/sin dot in"
        );
        assert!(
            real_order.iter().any(|c| !c.is_consonant && c.glyph == QAMATS.to_string()),
            "the vowel sitting between the letter and its dot is still taught"
        );

        // A dagesh on a non-begadkefat letter (pure gemination) isn't taught as
        // its own glyph, and doesn't change the host letter's identity.
        let gem = decompose_glyphs(&format!("{GIMEL}{DAGESH}{PATAH}{NUN}")); // dagesh chazak
        let cons: Vec<&str> = gem
            .iter()
            .filter(|c| c.is_consonant)
            .map(|c| c.glyph.as_str())
            .collect();
        assert_eq!(cons, vec![GIMEL.to_string(), NUN.to_string()]);
        assert!(
            !gem.iter().any(|c| c.glyph == DAGESH.to_string()),
            "dagesh is never taught as a standalone glyph"
        );

        // Grading the compound consonant credits it as one atomic glyph, not a
        // bare letter plus a separate dagesh/dot glyph.
        let bet_dagesh = format!("{BET}{DAGESH}");
        assert_eq!(split_glyph_key(&bet_dagesh), vec![bet_dagesh.clone()]);
        let sin_letter = format!("{SHIN}{SIN_DOT}");
        assert_eq!(split_glyph_key(&sin_letter), vec![sin_letter.clone()]);
        // A compound consonant fronting a syllable still splits off its vowel.
        assert_eq!(
            split_glyph_key(&format!("{bet_dagesh}{PATAH}")),
            vec![bet_dagesh, PATAH.to_string()]
        );
    }

    /// End-to-end against the in-repo data DBs: cold start should walk
    /// glyph → syllable drill → word meaning and eventually read the first verse,
    /// driven entirely by grading Good (pull-forward graduates the learning steps
    /// at a fixed `now`).
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
        let mut saw_word = false;
        let mut saw_mark = false;
        for _ in 0..4000 {
            item = match item {
                StudyItem::NewGlyph(g) | StudyItem::ReviewGlyph(g) => {
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                }
                StudyItem::NewWord(w) | StudyItem::ReviewWord(w) => {
                    saw_word = true;
                    bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
                }
                StudyItem::ExplainMark(_) => {
                    // Gradeless, like ReadVerse: acknowledged just by asking
                    // for the next item.
                    saw_mark = true;
                    bible.next_study_item(now)?
                }
                StudyItem::ReadVerse(_) => {
                    saw_read = true;
                    break;
                }
                StudyItem::Done => break,
            };
        }
        assert!(saw_word, "should drill word meaning via SRS");
        assert!(saw_mark, "should explain the sof pasuq before reading");
        assert!(saw_read, "should finish and read the first verse");
        Ok(())
    }

    /// Flagging a word as misread after `ReadVerse` (an "Again" grade on the
    /// `word` track) must not re-serve the same verse to read forever. Before
    /// the `next_review` fix, a lapse reset `reps` to 0, which the pull-forward
    /// query's `reps > 0` guard then excluded — so the just-demoted word could
    /// never be pulled forward for a re-drill, `next_target_verse` kept
    /// re-picking the same still-unfinished verse, and `next_study_item` fell
    /// straight through to `ReadVerse` again every single call.
    #[test]
    fn misread_word_does_not_re_serve_the_same_verse_forever() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let mut now = 1_700_000_000;
        let mut item = bible.next_study_item(now)?;
        let verse = loop {
            now += 5;
            item = match item {
                StudyItem::NewGlyph(g) | StudyItem::ReviewGlyph(g) => {
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                }
                StudyItem::NewWord(w) | StudyItem::ReviewWord(w) => {
                    bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
                }
                StudyItem::ExplainMark(_) => bible.next_study_item(now)?,
                StudyItem::ReadVerse(v) => break v,
                StudyItem::Done => panic!("ran out of curriculum before a read"),
            };
        };
        let misread = verse.words.first().cloned().expect("verse has words");

        now += 5;
        let after = bible.submit_review(Track::Word, &misread, Grade::Again, now)?;
        let same_verse = |item: &StudyItem| match item {
            StudyItem::ReadVerse(v) => {
                (v.book, v.chapter, v.verse) == (verse.book, verse.chapter, verse.verse)
            }
            _ => false,
        };
        assert!(
            !same_verse(&after),
            "flagging a word should not immediately re-serve the same verse"
        );

        // The demoted word must actually be reachable again (not stranded).
        let mut saw_misread_review = matches!(&after, StudyItem::ReviewWord(w) if w.surface == misread);
        let mut item = after;
        for _ in 0..500 {
            if saw_misread_review {
                break;
            }
            assert!(
                !same_verse(&item),
                "verse re-appeared before the misread word was ever reviewed"
            );
            now += 5;
            item = match item {
                StudyItem::NewGlyph(g) | StudyItem::ReviewGlyph(g) => {
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                }
                StudyItem::NewWord(w) | StudyItem::ReviewWord(w) => {
                    saw_misread_review |= w.surface == misread;
                    bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
                }
                StudyItem::ExplainMark(_) => bible.next_study_item(now)?,
                StudyItem::ReadVerse(_) | StudyItem::Done => break,
            };
        }
        assert!(
            saw_misread_review,
            "the misread word should be pulled forward for review, not stranded"
        );
        Ok(())
    }

    /// A word that never graduates (graded `Hard` forever, so it stays at
    /// `interval_days == 0`) must not block introducing a *different* word in
    /// the same verse. Before this was fixed, `next_introduction` only ever
    /// looked at the single most-common not-fully-learnt word — so once that
    /// word (often a very frequent one, sorting first) was introduced but not
    /// yet graduated, nothing else in the verse was ever introduced, and the
    /// learner just kept re-drilling the same one or two cards forever.
    #[test]
    fn stuck_word_does_not_block_introducing_other_words() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        // A real session has wall-clock time passing between answers, which
        // is what lets `introduced_epoch` order pull-forward fairly (see
        // `next_review`); a frozen `now` makes every row's `introduced_epoch`
        // identical and defeats that entirely, so advance it a little each
        // card, like a learner actually answering at a steady pace.
        let mut now = 1_700_000_000;
        let mut item = bible.next_study_item(now)?;
        let mut new_words = std::collections::HashSet::new();
        let mut stuck: Option<String> = None;
        for _ in 0..2000 {
            if new_words.len() >= 2 {
                break;
            }
            now += 5;
            item = match item {
                StudyItem::NewGlyph(g) | StudyItem::ReviewGlyph(g) => {
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                }
                StudyItem::NewWord(w) => {
                    new_words.insert(w.surface.clone());
                    if stuck.is_none() {
                        stuck = Some(w.surface.clone());
                    }
                    // Hard, while still in the learning steps, repeats the
                    // current step forever (see `Srs::graded`) — this word
                    // never graduates.
                    bible.submit_review(Track::Word, &w.surface, Grade::Hard, now)?
                }
                StudyItem::ReviewWord(w) => {
                    let grade = if stuck.as_deref() == Some(w.surface.as_str()) {
                        Grade::Hard
                    } else {
                        Grade::Good
                    };
                    bible.submit_review(Track::Word, &w.surface, grade, now)?
                }
                StudyItem::ExplainMark(_) => bible.next_study_item(now)?,
                StudyItem::ReadVerse(_) | StudyItem::Done => break,
            };
        }
        assert!(
            new_words.len() >= 2,
            "a second word should be introduced while the first is stuck mid-learning"
        );
        Ok(())
    }

    #[test]
    fn reading_mark_is_explained_once_and_never_drilled() -> rusqlite::Result<()> {
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
        let mut mark_count = 0;
        for _ in 0..4000 {
            item = match item {
                StudyItem::NewGlyph(g) | StudyItem::ReviewGlyph(g) => {
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                }
                StudyItem::NewWord(w) | StudyItem::ReviewWord(w) => {
                    bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
                }
                StudyItem::ExplainMark(_) => {
                    mark_count += 1;
                    bible.next_study_item(now)?
                }
                StudyItem::ReadVerse(_) | StudyItem::Done => break,
            };
        }
        assert_eq!(mark_count, 1, "sof pasuq is explained exactly once");
        // Never entered the drilled-glyph store, so it never comes up for
        // review.
        assert!(!bible.glyph_known("\u{05C3}")?);
        Ok(())
    }

    /// Before marks were switched to a one-time explanation, they were drilled
    /// like ordinary glyphs, so some existing `progress.db` files still carry a
    /// leftover `glyph_srs` row for one. Without a cleanup, that stale row makes
    /// the mark permanently eligible for `next_review`'s pull-forward rotation —
    /// it never graduates cleanly and keeps resurfacing as a quiz card forever,
    /// crowding out real progression. `init_progress_schema` must purge it.
    #[test]
    fn stale_reading_mark_glyph_row_is_purged_on_init() -> rusqlite::Result<()> {
        let db = Connection::open_in_memory()?;
        db.execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(&db)?;
        db.execute(
            "INSERT INTO progress.glyph_srs(glyph, ease, interval_days, due_epoch, \
                reps, lapses, introduced_epoch, last_grade) \
             VALUES ('\u{05C3}', 2.5, 0, 0, 1, 0, 0, 2)",
            [],
        )?;
        // Re-running init (as happens on every app start) must remove it.
        init_progress_schema(&db)?;
        let count: i64 = db.query_row(
            "SELECT COUNT(*) FROM progress.glyph_srs WHERE glyph = '\u{05C3}'",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(count, 0, "stale sof-pasuq glyph_srs row must be purged");
        Ok(())
    }

    /// Grading a reading mark via `Track::Glyph` (e.g. a client that mistakenly
    /// treats an `ExplainMark` card as gradable) must not resurrect it in
    /// `glyph_srs`, or it would fall back into the forever-drilled state above.
    #[test]
    fn submit_review_ignores_reading_mark_glyph_keys() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        bible.submit_review(Track::Glyph, "\u{05C3}", Grade::Good, 1_700_000_000)?;
        assert!(!bible.glyph_known("\u{05C3}")?);
        Ok(())
    }
}
