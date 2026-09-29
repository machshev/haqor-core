//! Learning passages of Scripture by heart, a verse at a time.
//!
//! The learner picks a *passage* — a range of verses, usually a chapter — and
//! the scheduler walks its verses in order. Each verse climbs a ladder of
//! fading cues before it is ever asked for from memory alone:
//!
//! 0. **Read** — the whole verse with transliteration and glosses, read aloud;
//! 1. **Light cloze** — about a third of the words blanked;
//! 2. **Heavy cloze** — about two thirds blanked;
//! 3. **First letters** — every word reduced to its first letter;
//! 4. **Recall** — nothing but the reference and the end of the verse before.
//!
//! Passing recall graduates the verse onto day-scale spaced repetition
//! (a verse-sized SM-2: 1 day, 3 days, then ease-scaled). A forgotten verse
//! drops back to the heavy cloze and climbs again. New verses are rationed —
//! a few a day, and never more than [`MAX_VERSES_IN_LEARNING`] half-learnt at
//! once — so reviews of what is already known are never crowded out.
//!
//! Every answer earns XP (a log in `progress.memory_review`), from which the
//! level, daily goal, streak, achievements and the dashboard's graphs are all
//! derived. Nothing is stored that cannot be recomputed from the log and the
//! verse states, so two devices merging their progress (see
//! [`crate::progress_sync`]) agree on every number afterwards.
//!
//! Verse state is keyed by the verse, not the passage: a verse shared by two
//! passages is learnt once, and deleting a passage keeps what was learnt.

use rusqlite::{Connection, OptionalExtension, params};

use crate::bible::{Bible, pack_ref};
use crate::tutor::Grade;

const SECONDS_PER_DAY: i64 = 86_400;

/// Cue-fading stages; see the module documentation.
pub const STAGE_READ: u8 = 0;
pub const STAGE_CLOZE_LIGHT: u8 = 1;
pub const STAGE_CLOZE_HEAVY: u8 = 2;
pub const STAGE_INITIALS: u8 = 3;
pub const STAGE_RECALL: u8 = 4;

/// Verse-sized SM-2 ease bounds.
const DEFAULT_EASE: f64 = 2.5;
const MIN_EASE: f64 = 1.3;
/// Spacing is capped so a long-known passage still comes round a few times a
/// year.
const MAX_INTERVAL_DAYS: i64 = 180;
/// An interval at or beyond which a verse counts as *mature*.
pub const MATURE_DAYS: i64 = 21;

/// Seconds before a verse still climbing the cue ladder comes back. Short
/// enough that the ladder is climbed within a sitting, long enough that
/// another verse can be interleaved between rungs.
const LEARNING_DELAY_SECS: i64 = 45;
/// A verse forgotten at review comes back after a minute.
const RELEARN_DELAY_SECS: i64 = 60;

/// Half-learnt verses allowed at once before a new verse is started.
pub const MAX_VERSES_IN_LEARNING: i64 = 2;

pub const DEFAULT_NEW_PER_DAY: i64 = 3;
pub const DEFAULT_DAILY_GOAL_XP: i64 = 100;

/// XP for completing a passage — every verse learnt — for the first time.
const PASSAGE_BONUS_XP: i64 = 100;
/// XP for a verse's first graduation from the cue ladder.
const GRADUATION_BONUS_XP: i64 = 20;

/// Words of the previous verse shown as the cue for the next.
const CUE_WORDS: usize = 3;

/// A target passage: an inclusive range of verses within one book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryPassage {
    pub id: String,
    pub book: u8,
    pub start_chapter: u8,
    pub start_verse: u8,
    pub end_chapter: u8,
    pub end_verse: u8,
    /// The learner's own name for it; empty means "use the reference".
    pub title: String,
    pub created_epoch: i64,
    pub updated_epoch: i64,
}

/// How well one verse of a passage is known, for the passage heatmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryVerseState {
    pub chapter: u8,
    pub verse: u8,
    /// 0 not started, 1 early cue ladder (read/cloze), 2 late cue ladder
    /// (first letters/recall), 3 learnt (< 7 days), 4 established (< 21 days),
    /// 5 mature.
    pub strength: u8,
    pub due: bool,
}

/// A passage with its progress, for the passage list and heatmap.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryPassageSummary {
    pub passage: MemoryPassage,
    pub verses: Vec<MemoryVerseState>,
    pub learnt: i64,
    pub mature: i64,
    pub due: i64,
    /// Overall mastery 0..=100: learning stages count a little, and a learnt
    /// verse counts fully once it reaches maturity.
    pub mastery_pct: i64,
    pub last_studied_epoch: i64,
}

/// One word of a drill card.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryWord {
    /// The word as it reads (cantillation removed, punctuation kept).
    pub text: String,
    /// Whether the learner has to supply it.
    pub hidden: bool,
    /// What is still shown of a hidden word: its first letter at the
    /// first-letters stage, else empty.
    pub hint: String,
    /// The word's learner gloss, when the verse's glosses align with its
    /// words; empty otherwise.
    pub gloss: String,
    pub translit: String,
}

/// One verse to practise.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryCard {
    /// The passage it is being practised as part of (empty for "all").
    pub passage_id: String,
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
    /// Its rung on the cue ladder (see the `STAGE_*` constants).
    pub stage: u8,
    pub words: Vec<MemoryWord>,
    /// The last few words of the previous verse, to chain from; empty for the
    /// first verse of a chapter.
    pub cue: String,
    /// The verse's glosses in reading order.
    pub translation: String,
    /// First time this verse is shown.
    pub is_new: bool,
    /// Already learnt and being reviewed (rather than climbing the ladder).
    pub is_review: bool,
    /// Position within the passage, 1-based, and the passage's length.
    pub position: i64,
    pub total: i64,
    /// Cards still due in this session's scope after this one.
    pub due_remaining: i64,
}

/// What the scheduler offers next.
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryItem {
    Card(MemoryCard),
    /// Nothing due and no new verse allowed today. `next_due_epoch` is when
    /// the next review falls (0 if nothing is scheduled); `can_learn_more`
    /// says whether an unstarted verse remains that the learner could choose
    /// to start anyway.
    Done {
        next_due_epoch: i64,
        can_learn_more: bool,
    },
    /// The scope holds no passages (or no verses) at all.
    Empty,
}

/// What an answer earned, for the celebration after it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryReviewOutcome {
    pub xp: i64,
    pub stage_before: u8,
    pub stage_after: u8,
    pub interval_days: i64,
    /// The answer graduated this verse off the cue ladder for the first time.
    pub first_graduation: bool,
    /// Passages this answer completed (every verse learnt) for the first time.
    pub completed_passages: Vec<String>,
    pub total_xp: i64,
    pub level_before: i64,
    pub level_after: i64,
    pub today_xp: i64,
    pub daily_goal_xp: i64,
    /// This answer took today's XP past the daily goal.
    pub goal_reached_now: bool,
    pub streak_days: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemorySettings {
    pub new_per_day: i64,
    pub daily_goal_xp: i64,
}

impl Default for MemorySettings {
    fn default() -> Self {
        MemorySettings {
            new_per_day: DEFAULT_NEW_PER_DAY,
            daily_goal_xp: DEFAULT_DAILY_GOAL_XP,
        }
    }
}

/// A day of memorisation activity, for the dashboard's graphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MemoryDay {
    /// Local day number (days since the epoch in the learner's time zone).
    pub day: i64,
    pub xp: i64,
    pub reviews: i64,
    /// Verses learnt by the end of this day (first graduations, cumulative).
    pub learnt_total: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryAchievement {
    pub key: String,
    pub title: String,
    pub description: String,
    pub progress: i64,
    pub target: i64,
    pub earned: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MemoryStats {
    pub total_xp: i64,
    /// 1-based level.
    pub level: i64,
    /// XP gained within the current level, and the span of the level.
    pub level_xp: i64,
    pub level_span: i64,
    pub today_xp: i64,
    pub daily_goal_xp: i64,
    pub new_per_day: i64,
    pub streak_days: i64,
    pub best_streak_days: i64,
    /// Days the daily goal has been met.
    pub goal_days: i64,
    pub verses_learnt: i64,
    pub verses_mature: i64,
    pub verses_learning: i64,
    pub verses_total: i64,
    pub due_now: i64,
    pub passages_completed: i64,
    pub passages_total: i64,
    pub reviews_total: i64,
    /// Share of answers not forgotten, 0..=100.
    pub accuracy_pct: i64,
    /// The last `history_days` days, oldest first, today last.
    pub history: Vec<MemoryDay>,
    /// Reviews falling due on each of the coming days; index 0 is today and
    /// includes everything overdue.
    pub forecast: Vec<i64>,
    pub achievements: Vec<MemoryAchievement>,
}

/// Mutable per-verse scheduling state.
#[derive(Debug, Clone, Copy, PartialEq)]
struct VerseSrs {
    stage: u8,
    ease: f64,
    interval_days: i64,
    reps: i64,
    lapses: i64,
}

impl Default for VerseSrs {
    fn default() -> Self {
        VerseSrs {
            stage: STAGE_READ,
            ease: DEFAULT_EASE,
            interval_days: 0,
            reps: 0,
            lapses: 0,
        }
    }
}

impl VerseSrs {
    fn learnt(&self) -> bool {
        self.interval_days >= 1
    }

    /// Apply a grade at `now`, returning the new state and its due time.
    ///
    /// On the ladder, Good climbs one rung and Easy two (never skipping
    /// recall itself), Hard repeats the rung, Again steps down one. Passing
    /// recall graduates. Once learnt, the verse is spaced SM-2 style; a lapse
    /// sends it back to the heavy cloze.
    ///
    /// `early` marks a run-through answer for a verse that was not due yet: a
    /// success then leaves its schedule alone (reviewing ahead must not
    /// inflate the spacing), while trouble still brings it forward.
    fn graded(self, grade: Grade, now: i64, due_epoch: i64, early: bool) -> (VerseSrs, i64) {
        let mut s = self;
        if !self.learnt() {
            match grade {
                Grade::Again => s.stage = self.stage.saturating_sub(1).max(STAGE_CLOZE_LIGHT),
                Grade::Hard => s.stage = self.stage.max(STAGE_CLOZE_LIGHT),
                Grade::Good | Grade::Easy if self.stage >= STAGE_RECALL => {
                    s.stage = STAGE_RECALL;
                    s.reps = self.reps + 1;
                    s.interval_days = if grade == Grade::Easy { 3 } else { 1 };
                    if grade == Grade::Easy {
                        s.ease = self.ease + 0.15;
                    }
                    return (s, now + s.interval_days * SECONDS_PER_DAY);
                }
                Grade::Good => s.stage = self.stage + 1,
                Grade::Easy => s.stage = (self.stage + 2).min(STAGE_RECALL),
            }
            return (s, now + LEARNING_DELAY_SECS);
        }
        match grade {
            Grade::Again => {
                s.ease = (self.ease - 0.20).max(MIN_EASE);
                s.lapses = self.lapses + 1;
                s.interval_days = 0;
                s.stage = STAGE_CLOZE_HEAVY;
                return (s, now + RELEARN_DELAY_SECS);
            }
            _ if early && grade != Grade::Hard => return (s, due_epoch),
            Grade::Hard => {
                s.ease = (self.ease - 0.15).max(MIN_EASE);
                if early {
                    // Struggled on a verse not yet due: bring it forward to
                    // tomorrow without shrinking its spacing.
                    return (s, due_epoch.min(now + SECONDS_PER_DAY));
                }
                s.interval_days = ((self.interval_days as f64 * 1.2).round() as i64).max(1);
            }
            Grade::Good => {
                s.interval_days = match self.interval_days {
                    1 => 3,
                    n => (n as f64 * self.ease).round() as i64,
                };
            }
            Grade::Easy => {
                s.ease = self.ease + 0.15;
                s.interval_days =
                    ((self.interval_days as f64 * self.ease * 1.3).round() as i64).max(4);
            }
        }
        s.reps = self.reps + 1;
        s.interval_days = s.interval_days.clamp(1, MAX_INTERVAL_DAYS);
        (s, now + s.interval_days * SECONDS_PER_DAY)
    }
}

/// Heatmap strength of a verse; see [`MemoryVerseState::strength`].
fn strength(state: Option<(VerseSrs, i64)>) -> u8 {
    match state {
        None => 0,
        Some((s, _)) if !s.learnt() => {
            if s.stage >= STAGE_INITIALS {
                2
            } else {
                1
            }
        }
        Some((s, _)) if s.interval_days >= MATURE_DAYS => 5,
        Some((s, _)) if s.interval_days >= 7 => 4,
        Some(_) => 3,
    }
}

/// A verse's contribution to passage mastery, 0.0..=1.0.
fn mastery(state: Option<(VerseSrs, i64)>) -> f64 {
    match state {
        None => 0.0,
        Some((s, _)) if !s.learnt() => f64::from(s.stage) * 0.1,
        Some((s, _)) => 0.5 + 0.5 * (s.interval_days as f64 / MATURE_DAYS as f64).min(1.0),
    }
}

/// XP for one answer at `stage`.
fn answer_xp(stage: u8, grade: Grade) -> i64 {
    if stage == STAGE_READ {
        return 5;
    }
    let base = match grade {
        Grade::Again => 2,
        Grade::Hard => 6,
        Grade::Good => 10,
        Grade::Easy => 12,
    };
    let recall_bonus = if stage >= STAGE_RECALL && matches!(grade, Grade::Good | Grade::Easy) {
        5
    } else {
        0
    };
    base + recall_bonus
}

/// Cumulative XP needed to reach 1-based `level`: 0, 100, 300, 600, 1000, …
pub fn level_threshold(level: i64) -> i64 {
    let l = (level - 1).max(0);
    50 * l * (l + 1)
}

/// The 1-based level `xp` has reached.
pub fn level_for_xp(xp: i64) -> i64 {
    let mut level = 1;
    while level_threshold(level + 1) <= xp {
        level += 1;
    }
    level
}

/// Local day number of `epoch` for a learner `utc_offset` seconds east of UTC.
fn local_day(epoch: i64, utc_offset: i64) -> i64 {
    (epoch + utc_offset).div_euclid(SECONDS_PER_DAY)
}

fn is_cantillation(c: char) -> bool {
    // Accents, plus meteg: pronunciation aids for chanting that only clutter
    // a verse being learnt by heart.
    matches!(c, '\u{0591}'..='\u{05AF}' | '\u{05BD}')
}

fn is_letter(c: char) -> bool {
    c.is_alphabetic()
}

/// Split a verse's display text into words, dropping cantillation. Tokens
/// that carry no letter (a paseq, a lone sof pasuq) are joined to the word
/// before them rather than being drilled as words.
pub fn memory_words(text: &str) -> Vec<String> {
    let plain: String = text.chars().filter(|&c| !is_cantillation(c)).collect();
    let mut words: Vec<String> = Vec::new();
    for token in plain.split_whitespace() {
        if !token.chars().any(is_letter)
            && let Some(last) = words.last_mut()
        {
            last.push(' ');
            last.push_str(token);
            continue;
        }
        words.push(token.to_string());
    }
    words
}

/// The first letter of a word, bare of its points — the first-letters cue.
fn first_letter(word: &str) -> String {
    word.chars()
        .find(|&c| is_letter(c))
        .map(String::from)
        .unwrap_or_default()
}

/// A small deterministic mixer, so which words a cloze blanks changes from
/// one showing to the next without storing anything.
fn mix(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^ (x >> 33)
}

/// Which of `n` words to hide at `stage`, varied by `seed`.
fn hidden_mask(n: usize, stage: u8, seed: u64) -> Vec<bool> {
    if n == 0 {
        return Vec::new();
    }
    let share = match stage {
        STAGE_READ => return vec![false; n],
        STAGE_CLOZE_LIGHT => 1.0 / 3.0,
        STAGE_CLOZE_HEAVY => 2.0 / 3.0,
        _ => return vec![true; n],
    };
    let hide = ((n as f64 * share).round() as usize).clamp(1, n);
    let mut order: Vec<(u64, usize)> = (0..n)
        .map(|i| {
            (
                mix(seed ^ (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)),
                i,
            )
        })
        .collect();
    order.sort_unstable();
    let mut mask = vec![false; n];
    for &(_, i) in order.iter().take(hide) {
        mask[i] = true;
    }
    mask
}

/// Create the memorisation tables in the attached `progress` schema.
/// Idempotent; called from [`crate::tutor::init_progress_schema`] so the sync
/// server's canonical database has them too.
pub fn init_memory_schema(db: &Connection) -> rusqlite::Result<()> {
    ensure_memory_tables(db, "progress")
}

/// [`init_memory_schema`] for any attached schema — the merge also runs it
/// over an incoming snapshot from an app that predates memorisation.
pub(crate) fn ensure_memory_tables(db: &Connection, schema: &str) -> rusqlite::Result<()> {
    db.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {schema}.memory_passage(
            id            TEXT    PRIMARY KEY,
            book          INTEGER NOT NULL,
            start_chapter INTEGER NOT NULL,
            start_verse   INTEGER NOT NULL,
            end_chapter   INTEGER NOT NULL,
            end_verse     INTEGER NOT NULL,
            title         TEXT    NOT NULL DEFAULT '',
            created_epoch INTEGER NOT NULL,
            updated_epoch INTEGER NOT NULL,
            deleted       INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS {schema}.memory_verse(
            book              INTEGER NOT NULL,
            chapter           INTEGER NOT NULL,
            verse             INTEGER NOT NULL,
            stage             INTEGER NOT NULL,
            ease              REAL    NOT NULL,
            interval_days     INTEGER NOT NULL,
            due_epoch         INTEGER NOT NULL,
            reps              INTEGER NOT NULL,
            lapses            INTEGER NOT NULL,
            introduced_epoch  INTEGER NOT NULL,
            last_review_epoch INTEGER NOT NULL,
            last_grade        INTEGER NOT NULL,
            updated_epoch     INTEGER NOT NULL,
            PRIMARY KEY (book, chapter, verse)
         );
         CREATE TABLE IF NOT EXISTS {schema}.memory_review(
            epoch      INTEGER NOT NULL,
            day        INTEGER NOT NULL,
            book       INTEGER NOT NULL,
            chapter    INTEGER NOT NULL,
            verse      INTEGER NOT NULL,
            stage      INTEGER NOT NULL,
            grade      INTEGER NOT NULL,
            xp         INTEGER NOT NULL,
            graduated  INTEGER NOT NULL DEFAULT 0,
            passage_id TEXT    NOT NULL DEFAULT ''
         );
         CREATE INDEX IF NOT EXISTS {schema}.idx_memory_review_day ON memory_review(day);
         CREATE TABLE IF NOT EXISTS {schema}.memory_settings(
            id            INTEGER PRIMARY KEY CHECK (id = 1),
            new_per_day   INTEGER NOT NULL,
            daily_goal_xp INTEGER NOT NULL,
            updated_epoch INTEGER NOT NULL
         );"
    ))
}

/// Merge the memorisation tables of the attached `sync` schema into
/// `progress`. Passages, verse states and settings keep the most recently
/// updated copy (a deleted passage is a tombstone that wins by recency);
/// the answer log is unioned.
pub(crate) fn merge_memory(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch(
        "INSERT INTO progress.memory_passage(
             id, book, start_chapter, start_verse, end_chapter, end_verse,
             title, created_epoch, updated_epoch, deleted)
         SELECT id, book, start_chapter, start_verse, end_chapter, end_verse,
                title, created_epoch, updated_epoch, deleted
         FROM sync.memory_passage WHERE true
         ON CONFLICT(id) DO UPDATE SET
            book=excluded.book, start_chapter=excluded.start_chapter,
            start_verse=excluded.start_verse, end_chapter=excluded.end_chapter,
            end_verse=excluded.end_verse, title=excluded.title,
            created_epoch=MIN(progress.memory_passage.created_epoch, excluded.created_epoch),
            updated_epoch=excluded.updated_epoch, deleted=excluded.deleted
         WHERE excluded.updated_epoch > progress.memory_passage.updated_epoch
            OR (excluded.updated_epoch = progress.memory_passage.updated_epoch
                AND excluded.deleted > progress.memory_passage.deleted);

         INSERT INTO progress.memory_verse(
             book, chapter, verse, stage, ease, interval_days, due_epoch, reps,
             lapses, introduced_epoch, last_review_epoch, last_grade, updated_epoch)
         SELECT book, chapter, verse, stage, ease, interval_days, due_epoch, reps,
                lapses, introduced_epoch, last_review_epoch, last_grade, updated_epoch
         FROM sync.memory_verse WHERE true
         ON CONFLICT(book, chapter, verse) DO UPDATE SET
            stage=excluded.stage, ease=excluded.ease,
            interval_days=excluded.interval_days, due_epoch=excluded.due_epoch,
            reps=excluded.reps, lapses=excluded.lapses,
            introduced_epoch=MIN(progress.memory_verse.introduced_epoch,
                                 excluded.introduced_epoch),
            last_review_epoch=excluded.last_review_epoch,
            last_grade=excluded.last_grade, updated_epoch=excluded.updated_epoch
         WHERE excluded.updated_epoch > progress.memory_verse.updated_epoch
            OR (excluded.updated_epoch = progress.memory_verse.updated_epoch
                AND excluded.reps > progress.memory_verse.reps);

         INSERT INTO progress.memory_review(
             epoch, day, book, chapter, verse, stage, grade, xp, graduated, passage_id)
         SELECT r.epoch, r.day, r.book, r.chapter, r.verse, r.stage, r.grade, r.xp,
                r.graduated, r.passage_id
         FROM sync.memory_review r
         WHERE NOT EXISTS (
            SELECT 1 FROM progress.memory_review p
            WHERE p.epoch = r.epoch AND p.book = r.book AND p.chapter = r.chapter
              AND p.verse = r.verse AND p.stage = r.stage AND p.grade = r.grade);

         INSERT INTO progress.memory_settings(id, new_per_day, daily_goal_xp, updated_epoch)
         SELECT id, new_per_day, daily_goal_xp, updated_epoch
         FROM sync.memory_settings WHERE true
         ON CONFLICT(id) DO UPDATE SET
            new_per_day=excluded.new_per_day, daily_goal_xp=excluded.daily_goal_xp,
            updated_epoch=excluded.updated_epoch
         WHERE excluded.updated_epoch > progress.memory_settings.updated_epoch;",
    )
}

/// SQL predicate: the verse `v` (an alias with book/chapter/verse columns)
/// lies inside passage `p`.
const IN_PASSAGE: &str = "((v.book << 16) | (v.chapter << 8) | v.verse) BETWEEN \
     ((p.book << 16) | (p.start_chapter << 8) | p.start_verse) AND \
     ((p.book << 16) | (p.end_chapter << 8) | p.end_verse)";

fn passage_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryPassage> {
    Ok(MemoryPassage {
        id: r.get(0)?,
        book: r.get(1)?,
        start_chapter: r.get(2)?,
        start_verse: r.get(3)?,
        end_chapter: r.get(4)?,
        end_verse: r.get(5)?,
        title: r.get(6)?,
        created_epoch: r.get(7)?,
        updated_epoch: r.get(8)?,
    })
}

const PASSAGE_COLUMNS: &str = "id, book, start_chapter, start_verse, end_chapter, end_verse, \
     title, created_epoch, updated_epoch";

impl Bible {
    // --- passages ------------------------------------------------------------

    /// Every live passage, most recently created first.
    pub fn memory_passage_list(&self) -> rusqlite::Result<Vec<MemoryPassage>> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT {PASSAGE_COLUMNS} FROM progress.memory_passage
             WHERE deleted = 0 ORDER BY created_epoch DESC, id"
        ))?;
        stmt.query_map([], passage_from_row)?.collect()
    }

    pub fn memory_passage(&self, id: &str) -> rusqlite::Result<Option<MemoryPassage>> {
        self.conn()
            .query_row(
                &format!(
                    "SELECT {PASSAGE_COLUMNS} FROM progress.memory_passage
                     WHERE id = ?1 AND deleted = 0"
                ),
                [id],
                passage_from_row,
            )
            .optional()
    }

    /// Add a passage to learn, returning it. The range is normalised (start
    /// before end) and clipped to verses that exist; the same range added
    /// twice returns the existing passage (retitled if a title is given).
    /// `None` when the range holds no verse at all.
    #[allow(clippy::too_many_arguments)]
    pub fn add_memory_passage(
        &self,
        book: u8,
        start_chapter: u8,
        start_verse: u8,
        end_chapter: u8,
        end_verse: u8,
        title: &str,
        now: i64,
    ) -> rusqlite::Result<Option<MemoryPassage>> {
        let (mut a, mut b) = (
            pack_ref(book, start_chapter, start_verse),
            pack_ref(book, end_chapter, end_verse),
        );
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        let bounds: Option<(i64, i64)> = self.conn().query_row(
            "SELECT MIN(ref), MAX(ref) FROM data.verse WHERE ref BETWEEN ?1 AND ?2",
            params![a, b],
            |r| Ok(r.get::<_, Option<i64>>(0)?.zip(r.get::<_, Option<i64>>(1)?)),
        )?;
        let Some((first, last)) = bounds else {
            return Ok(None);
        };
        let unpack = |r: i64| (((r >> 8) & 255) as u8, (r & 255) as u8);
        let ((sc, sv), (ec, ev)) = (unpack(first), unpack(last));
        let title = title.trim();
        let existing: Option<String> = self
            .conn()
            .query_row(
                "SELECT id FROM progress.memory_passage
                 WHERE deleted = 0 AND book = ?1 AND start_chapter = ?2 AND start_verse = ?3
                   AND end_chapter = ?4 AND end_verse = ?5",
                params![book, sc, sv, ec, ev],
                |r| r.get(0),
            )
            .optional()?;
        let id = match existing {
            Some(id) => {
                if !title.is_empty() {
                    self.conn().execute(
                        "UPDATE progress.memory_passage
                         SET title = ?2, updated_epoch = MAX(updated_epoch + 1, ?3)
                         WHERE id = ?1",
                        params![id, title, now],
                    )?;
                }
                id
            }
            None => {
                // Unique across devices without coordination: two phones
                // adding passages offline must not collide on merge.
                let id: String = self.conn().query_row(
                    "SELECT printf('%x-', ?1) || lower(hex(randomblob(6)))",
                    [now],
                    |r| r.get(0),
                )?;
                self.conn().execute(
                    "INSERT INTO progress.memory_passage(
                         id, book, start_chapter, start_verse, end_chapter, end_verse,
                         title, created_epoch, updated_epoch, deleted)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 0)",
                    params![id, book, sc, sv, ec, ev, title, now],
                )?;
                id
            }
        };
        self.memory_passage(&id)
    }

    /// Remove a passage. What was learnt of its verses is kept, so adding it
    /// back later carries on where it left off.
    pub fn delete_memory_passage(&self, id: &str, now: i64) -> rusqlite::Result<bool> {
        Ok(self.conn().execute(
            "UPDATE progress.memory_passage
             SET deleted = 1, updated_epoch = MAX(updated_epoch + 1, ?2)
             WHERE id = ?1 AND deleted = 0",
            params![id, now],
        )? > 0)
    }

    /// The verses of a passage, in order.
    pub fn memory_passage_verses(&self, p: &MemoryPassage) -> rusqlite::Result<Vec<(u8, u8)>> {
        let mut stmt = self.conn().prepare(
            "SELECT (ref >> 8) & 255, ref & 255 FROM data.verse
             WHERE ref BETWEEN ?1 AND ?2 ORDER BY ref",
        )?;
        stmt.query_map(
            params![
                pack_ref(p.book, p.start_chapter, p.start_verse),
                pack_ref(p.book, p.end_chapter, p.end_verse)
            ],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?
        .collect()
    }

    fn memory_verse_srs(
        &self,
        book: u8,
        chapter: u8,
        verse: u8,
    ) -> rusqlite::Result<Option<(VerseSrs, i64)>> {
        self.conn()
            .query_row(
                "SELECT stage, ease, interval_days, reps, lapses, due_epoch
                 FROM progress.memory_verse WHERE book = ?1 AND chapter = ?2 AND verse = ?3",
                params![book, chapter, verse],
                |r| {
                    Ok((
                        VerseSrs {
                            stage: r.get(0)?,
                            ease: r.get(1)?,
                            interval_days: r.get(2)?,
                            reps: r.get(3)?,
                            lapses: r.get(4)?,
                        },
                        r.get(5)?,
                    ))
                },
            )
            .optional()
    }

    /// Every live passage with its per-verse progress.
    pub fn memory_passages(&self, now: i64) -> rusqlite::Result<Vec<MemoryPassageSummary>> {
        let mut out = Vec::new();
        for passage in self.memory_passage_list()? {
            out.push(self.memory_passage_summary(passage, now)?);
        }
        Ok(out)
    }

    pub fn memory_passage_summary(
        &self,
        passage: MemoryPassage,
        now: i64,
    ) -> rusqlite::Result<MemoryPassageSummary> {
        let mut verses = Vec::new();
        let (mut learnt, mut mature, mut due, mut total) = (0, 0, 0, 0.0);
        let refs = self.memory_passage_verses(&passage)?;
        for &(chapter, verse) in &refs {
            let state = self.memory_verse_srs(passage.book, chapter, verse)?;
            let is_due = state.is_some_and(|(_, d)| d <= now);
            if let Some((s, _)) = state {
                learnt += i64::from(s.learnt());
                mature += i64::from(s.interval_days >= MATURE_DAYS);
            }
            due += i64::from(is_due);
            total += mastery(state);
            verses.push(MemoryVerseState {
                chapter,
                verse,
                strength: strength(state),
                due: is_due,
            });
        }
        let last_studied_epoch: i64 = self.conn().query_row(
            &format!(
                "SELECT COALESCE(MAX(v.last_review_epoch), 0)
                 FROM progress.memory_verse v, progress.memory_passage p
                 WHERE p.id = ?1 AND v.book = p.book AND {IN_PASSAGE}"
            ),
            [&passage.id],
            |r| r.get(0),
        )?;
        let mastery_pct = if refs.is_empty() {
            0
        } else {
            (total * 100.0 / refs.len() as f64).round() as i64
        };
        Ok(MemoryPassageSummary {
            passage,
            verses,
            learnt,
            mature,
            due,
            mastery_pct,
            last_studied_epoch,
        })
    }

    // --- settings ------------------------------------------------------------

    pub fn memory_settings(&self) -> rusqlite::Result<MemorySettings> {
        Ok(self
            .conn()
            .query_row(
                "SELECT new_per_day, daily_goal_xp FROM progress.memory_settings WHERE id = 1",
                [],
                |r| {
                    Ok(MemorySettings {
                        new_per_day: r.get(0)?,
                        daily_goal_xp: r.get(1)?,
                    })
                },
            )
            .optional()?
            .unwrap_or_default())
    }

    /// Store the settings (clamped to sensible bounds) and return them.
    pub fn set_memory_settings(
        &self,
        settings: MemorySettings,
        now: i64,
    ) -> rusqlite::Result<MemorySettings> {
        let s = MemorySettings {
            new_per_day: settings.new_per_day.clamp(1, 20),
            daily_goal_xp: settings.daily_goal_xp.clamp(20, 1000),
        };
        self.conn().execute(
            "INSERT INTO progress.memory_settings(id, new_per_day, daily_goal_xp, updated_epoch)
             VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET
                new_per_day = excluded.new_per_day, daily_goal_xp = excluded.daily_goal_xp,
                updated_epoch = MAX(progress.memory_settings.updated_epoch + 1,
                                    excluded.updated_epoch)",
            params![s.new_per_day, s.daily_goal_xp, now],
        )?;
        Ok(s)
    }

    // --- scheduling ----------------------------------------------------------

    /// The passages in scope: one passage, or every live passage when
    /// `passage_id` is empty (most recently created first, so a newly added
    /// passage is where new verses come from).
    fn memory_scope(&self, passage_id: &str) -> rusqlite::Result<Vec<MemoryPassage>> {
        if passage_id.is_empty() {
            self.memory_passage_list()
        } else {
            Ok(self.memory_passage(passage_id)?.into_iter().collect())
        }
    }

    /// The next verse to practise within `passage_id` (empty = all passages).
    ///
    /// Order: a verse on the cue ladder that is due; a learnt verse due for
    /// review (in passage order, so reviews chain); a new verse, if today's
    /// ration and the half-learnt cap allow (or `extra_new` asks for one
    /// anyway); then a verse still on the ladder even if not quite due, so a
    /// sitting never stalls mid-climb.
    pub fn next_memory_item(
        &self,
        passage_id: &str,
        extra_new: bool,
        now: i64,
        utc_offset: i64,
    ) -> rusqlite::Result<MemoryItem> {
        let scope = self.memory_scope(passage_id)?;
        let mut verses: Vec<(MemoryPassage, u8, u8, i64, i64)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for passage in &scope {
            let refs = self.memory_passage_verses(passage)?;
            let total = refs.len() as i64;
            for (i, (chapter, verse)) in refs.into_iter().enumerate() {
                if seen.insert((passage.book, chapter, verse)) {
                    verses.push((passage.clone(), chapter, verse, i as i64 + 1, total));
                }
            }
        }
        if verses.is_empty() {
            return Ok(MemoryItem::Empty);
        }

        let mut learning_due = None;
        let mut learning_soon = None;
        let mut review_due = None;
        let mut first_new = None;
        let mut in_learning = 0;
        let mut due_count = 0;
        let mut next_due = i64::MAX;
        for (idx, (p, chapter, verse, _, _)) in verses.iter().enumerate() {
            match self.memory_verse_srs(p.book, *chapter, *verse)? {
                None => {
                    first_new.get_or_insert(idx);
                }
                Some((s, due)) => {
                    let due_now = due <= now;
                    due_count += i64::from(due_now);
                    next_due = next_due.min(due);
                    if !s.learnt() {
                        in_learning += 1;
                        if due_now {
                            if learning_due.is_none_or(|(_, d)| due < d) {
                                learning_due = Some((idx, due));
                            }
                        } else if learning_soon.is_none_or(|(_, d)| due < d) {
                            learning_soon = Some((idx, due));
                        }
                    } else if due_now && review_due.is_none() {
                        review_due = Some(idx);
                    }
                }
            }
        }

        let settings = self.memory_settings()?;
        let today = local_day(now, utc_offset);
        let started_today: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM progress.memory_verse
             WHERE (introduced_epoch + ?2) / 86400 = ?1",
            params![today, utc_offset],
            |r| r.get(0),
        )?;
        let may_start = first_new.is_some()
            && (extra_new
                || (started_today < settings.new_per_day && in_learning < MAX_VERSES_IN_LEARNING));

        let chosen = learning_due
            .map(|(i, _)| i)
            .or(review_due)
            .or(if may_start { first_new } else { None })
            .or(learning_soon.map(|(i, _)| i));
        let Some(idx) = chosen else {
            return Ok(MemoryItem::Done {
                next_due_epoch: if next_due == i64::MAX { 0 } else { next_due },
                can_learn_more: first_new.is_some(),
            });
        };
        let (p, chapter, verse, position, total) = &verses[idx];
        let chosen_due = self
            .memory_verse_srs(p.book, *chapter, *verse)?
            .is_some_and(|(_, due)| due <= now);
        let mut card = self.memory_card(passage_id, p.book, *chapter, *verse, None, now)?;
        card.position = *position;
        card.total = *total;
        card.due_remaining = due_count - i64::from(chosen_due);
        Ok(MemoryItem::Card(card))
    }

    /// Build the drill card for one verse at its current stage, or at
    /// `force_stage` (a run-through asks for recall).
    pub fn memory_card(
        &self,
        passage_id: &str,
        book: u8,
        chapter: u8,
        verse: u8,
        force_stage: Option<u8>,
        now: i64,
    ) -> rusqlite::Result<MemoryCard> {
        let state = self.memory_verse_srs(book, chapter, verse)?;
        let stage = force_stage
            .unwrap_or_else(|| state.map_or(STAGE_READ, |(s, _)| s.stage))
            .min(STAGE_RECALL);
        let words = memory_words(&self.get(book, chapter, verse)?);
        let glosses: Vec<String> = self
            .verse_gloss_words(book, chapter, verse)
            .map(|pairs| pairs.into_iter().map(|(_, g)| g).collect())
            .unwrap_or_default();
        let aligned = glosses.len() == words.len();
        let (reps, lapses) = state.map_or((0, 0), |(s, _)| (s.reps, s.lapses));
        let seed = (pack_ref(book, chapter, verse) as u64)
            ^ ((reps as u64) << 40)
            ^ ((lapses as u64) << 52)
            ^ (u64::from(stage) << 60)
            ^ (now as u64 / 30);
        let mask = hidden_mask(words.len(), stage, seed);
        let words = words
            .into_iter()
            .enumerate()
            .map(|(i, text)| MemoryWord {
                hint: if mask[i] && stage == STAGE_INITIALS {
                    first_letter(&text)
                } else {
                    String::new()
                },
                hidden: mask[i],
                gloss: if aligned {
                    glosses[i].clone()
                } else {
                    String::new()
                },
                translit: if book < 40 {
                    crate::romanize::romanize(&text)
                } else {
                    String::new()
                },
                text,
            })
            .collect();
        let cue = if verse > 1 {
            self.get(book, chapter, verse - 1)
                .map(|text| {
                    let previous = memory_words(&text);
                    let from = previous.len().saturating_sub(CUE_WORDS);
                    previous[from..].join(" ")
                })
                .unwrap_or_default()
        } else {
            String::new()
        };
        let translation = glosses
            .iter()
            .map(|g| g.trim())
            .filter(|g| !g.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        Ok(MemoryCard {
            passage_id: passage_id.to_string(),
            book,
            chapter,
            verse,
            stage,
            words,
            cue,
            translation,
            is_new: state.is_none(),
            is_review: state.is_some_and(|(s, _)| s.learnt()),
            position: 0,
            total: 0,
            due_remaining: 0,
        })
    }

    /// Record an answer for a verse and reschedule it. `run_through` marks
    /// answers from reciting a whole passage in order, where a verse may not
    /// be due yet (see `VerseSrs::graded`).
    #[allow(clippy::too_many_arguments)]
    pub fn submit_memory_review(
        &self,
        passage_id: &str,
        book: u8,
        chapter: u8,
        verse: u8,
        grade: Grade,
        run_through: bool,
        now: i64,
        utc_offset: i64,
    ) -> rusqlite::Result<MemoryReviewOutcome> {
        let today = local_day(now, utc_offset);
        let settings = self.memory_settings()?;
        let total_before = self.memory_total_xp()?;
        let today_before = self.memory_day_xp(today)?;
        let completed_before = self.completed_passage_ids(now)?;

        let previous = self.memory_verse_srs(book, chapter, verse)?;
        let (before, due_before) = previous.unwrap_or((VerseSrs::default(), now));
        let early = run_through && due_before > now;
        let (after, due) = before.graded(grade, now, due_before, early);
        let ever_graduated: bool = self.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM progress.memory_review
             WHERE book = ?1 AND chapter = ?2 AND verse = ?3 AND graduated = 1)",
            params![book, chapter, verse],
            |r| r.get(0),
        )?;
        let graduated_now = !before.learnt() && after.learnt();
        let first_graduation = graduated_now && !ever_graduated;

        self.conn().execute(
            "INSERT INTO progress.memory_verse(
                 book, chapter, verse, stage, ease, interval_days, due_epoch, reps, lapses,
                 introduced_epoch, last_review_epoch, last_grade, updated_epoch)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?11, ?10)
             ON CONFLICT(book, chapter, verse) DO UPDATE SET
                stage = excluded.stage, ease = excluded.ease,
                interval_days = excluded.interval_days, due_epoch = excluded.due_epoch,
                reps = excluded.reps, lapses = excluded.lapses,
                last_review_epoch = excluded.last_review_epoch,
                last_grade = excluded.last_grade,
                updated_epoch = MAX(progress.memory_verse.updated_epoch + 1,
                                    excluded.updated_epoch)",
            params![
                book,
                chapter,
                verse,
                after.stage,
                after.ease,
                after.interval_days,
                due,
                after.reps,
                after.lapses,
                now,
                grade as i64,
            ],
        )?;

        let completed_passages: Vec<String> = if graduated_now {
            let ever_completed = self.ever_completed_passage_ids()?;
            self.completed_passage_ids(now)?
                .into_iter()
                .filter(|id| !completed_before.contains(id) && !ever_completed.contains(id))
                .collect()
        } else {
            Vec::new()
        };
        let xp = answer_xp(before.stage, grade)
            + if first_graduation {
                GRADUATION_BONUS_XP
            } else {
                0
            }
            + PASSAGE_BONUS_XP * completed_passages.len() as i64;
        // A completed passage is remembered in its review row, so completing
        // it again after a lapse does not pay the bonus twice.
        let passage_tag = completed_passages
            .first()
            .map(|id| format!("complete:{id}"))
            .unwrap_or_else(|| passage_id.to_string());
        self.conn().execute(
            "INSERT INTO progress.memory_review(
                 epoch, day, book, chapter, verse, stage, grade, xp, graduated, passage_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                now,
                today,
                book,
                chapter,
                verse,
                before.stage,
                grade as i64,
                xp,
                graduated_now,
                passage_tag
            ],
        )?;
        for id in completed_passages.iter().skip(1) {
            // Several passages completed at once (overlapping ranges): one
            // zero-XP marker row each, a second apart to stay distinct.
            self.conn().execute(
                "INSERT INTO progress.memory_review(
                     epoch, day, book, chapter, verse, stage, grade, xp, graduated, passage_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, -1, -1, 0, 0, ?6)",
                params![now, today, book, chapter, verse, format!("complete:{id}")],
            )?;
        }

        let total_xp = total_before + xp;
        let today_xp = today_before + xp;
        Ok(MemoryReviewOutcome {
            xp,
            stage_before: before.stage,
            stage_after: after.stage,
            interval_days: after.interval_days,
            first_graduation,
            completed_passages,
            total_xp,
            level_before: level_for_xp(total_before),
            level_after: level_for_xp(total_xp),
            today_xp,
            daily_goal_xp: settings.daily_goal_xp,
            goal_reached_now: today_before < settings.daily_goal_xp
                && today_xp >= settings.daily_goal_xp,
            streak_days: self.memory_streak(today)?,
        })
    }

    fn memory_total_xp(&self) -> rusqlite::Result<i64> {
        self.conn().query_row(
            "SELECT COALESCE(SUM(xp), 0) FROM progress.memory_review",
            [],
            |r| r.get(0),
        )
    }

    fn memory_day_xp(&self, day: i64) -> rusqlite::Result<i64> {
        self.conn().query_row(
            "SELECT COALESCE(SUM(xp), 0) FROM progress.memory_review WHERE day = ?1",
            [day],
            |r| r.get(0),
        )
    }

    /// Live passages every verse of which is currently learnt.
    fn completed_passage_ids(&self, now: i64) -> rusqlite::Result<Vec<String>> {
        Ok(self
            .memory_passages(now)?
            .into_iter()
            .filter(|s| !s.verses.is_empty() && s.learnt == s.verses.len() as i64)
            .map(|s| s.passage.id)
            .collect())
    }

    fn ever_completed_passage_ids(&self) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self.conn().prepare(
            "SELECT DISTINCT substr(passage_id, 10) FROM progress.memory_review
             WHERE passage_id LIKE 'complete:%'",
        )?;
        stmt.query_map([], |r| r.get(0))?.collect()
    }

    /// Distinct days with any memorisation answer, newest first.
    fn memory_days(&self) -> rusqlite::Result<Vec<i64>> {
        let mut stmt = self
            .conn()
            .prepare("SELECT DISTINCT day FROM progress.memory_review ORDER BY day DESC")?;
        stmt.query_map([], |r| r.get(0))?.collect()
    }

    /// Consecutive days practised, ending today — or yesterday, so a streak
    /// is not shown broken before today's practice has happened.
    fn memory_streak(&self, today: i64) -> rusqlite::Result<i64> {
        let days = self.memory_days()?;
        let mut expected = match days.first() {
            Some(&d) if d == today => today,
            _ => today - 1,
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

    // --- statistics ----------------------------------------------------------

    /// Everything the dashboard shows: level and XP, goal and streaks, verse
    /// counts, the last `history_days` days of activity, a `forecast_days`
    /// review forecast, and achievements.
    pub fn memory_stats(
        &self,
        now: i64,
        utc_offset: i64,
        history_days: i64,
        forecast_days: i64,
    ) -> rusqlite::Result<MemoryStats> {
        let conn = self.conn();
        let today = local_day(now, utc_offset);
        let settings = self.memory_settings()?;
        let total_xp = self.memory_total_xp()?;
        let level = level_for_xp(total_xp);
        let level_floor = level_threshold(level);
        let level_span = level_threshold(level + 1) - level_floor;

        // Verse counts over the verses of live passages (a verse in two
        // passages counts once).
        let summaries = self.memory_passages(now)?;
        let mut distinct = std::collections::HashMap::new();
        for s in &summaries {
            for v in &s.verses {
                distinct.insert((s.passage.book, v.chapter, v.verse), *v);
            }
        }
        let count = |pred: &dyn Fn(&MemoryVerseState) -> bool| -> i64 {
            distinct.values().filter(|v| pred(v)).count() as i64
        };
        let verses_learnt = count(&|v| v.strength >= 3);
        let verses_mature = count(&|v| v.strength >= 5);
        let verses_learning = count(&|v| (1..=2).contains(&v.strength));
        let due_now = count(&|v| v.due);
        let passages_completed = summaries
            .iter()
            .filter(|s| !s.verses.is_empty() && s.learnt == s.verses.len() as i64)
            .count() as i64;

        let (reviews_total, recalled): (i64, i64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(grade > 0), 0) FROM progress.memory_review
             WHERE grade >= 0",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let accuracy_pct = if reviews_total > 0 {
            recalled * 100 / reviews_total
        } else {
            0
        };

        // Daily history, with a running total of first graduations.
        let first_day = today - history_days.max(1) + 1;
        let mut history: Vec<MemoryDay> = (first_day..=today)
            .map(|day| MemoryDay {
                day,
                ..Default::default()
            })
            .collect();
        {
            let mut stmt = conn.prepare(
                "SELECT day, SUM(xp), SUM(grade >= 0) FROM progress.memory_review
                 WHERE day >= ?1 GROUP BY day",
            )?;
            let rows = stmt.query_map([first_day], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })?;
            for row in rows {
                let (day, xp, reviews) = row?;
                if let Some(h) = history.get_mut((day - first_day) as usize) {
                    h.xp = xp;
                    h.reviews = reviews;
                }
            }
            let mut stmt = conn.prepare(
                "SELECT first_day, COUNT(*) FROM (
                    SELECT MIN(day) AS first_day FROM progress.memory_review
                    WHERE graduated = 1 GROUP BY book, chapter, verse)
                 GROUP BY first_day ORDER BY first_day",
            )?;
            let firsts: Vec<(i64, i64)> = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            let mut running: i64 = firsts
                .iter()
                .filter(|(d, _)| *d < first_day)
                .map(|(_, n)| n)
                .sum();
            for h in &mut history {
                running += firsts
                    .iter()
                    .filter(|(d, _)| *d == h.day)
                    .map(|(_, n)| n)
                    .sum::<i64>();
                h.learnt_total = running;
            }
        }

        // Review forecast: due times of learnt verses in live passages, by
        // local day; anything overdue lands today.
        let mut forecast = vec![0; forecast_days.max(1) as usize];
        {
            let mut stmt = conn.prepare(&format!(
                "SELECT DISTINCT v.book, v.chapter, v.verse, v.due_epoch
                 FROM progress.memory_verse v JOIN progress.memory_passage p
                   ON p.deleted = 0 AND v.book = p.book AND {IN_PASSAGE}"
            ))?;
            let dues = stmt.query_map([], |r| r.get::<_, i64>(3))?;
            for due in dues {
                let offset = (local_day(due?, utc_offset) - today).max(0) as usize;
                if let Some(slot) = forecast.get_mut(offset) {
                    *slot += 1;
                }
            }
        }

        let days_ascending: Vec<i64> = {
            let mut d = self.memory_days()?;
            d.reverse();
            d
        };
        let mut best_streak = 0;
        let mut run = 0;
        let mut prev: Option<i64> = None;
        for &d in &days_ascending {
            run = if prev == Some(d - 1) { run + 1 } else { 1 };
            best_streak = best_streak.max(run);
            prev = Some(d);
        }
        let goal_days: i64 = conn.query_row(
            "SELECT COUNT(*) FROM (SELECT day FROM progress.memory_review
             GROUP BY day HAVING SUM(xp) >= ?1)",
            [settings.daily_goal_xp],
            |r| r.get(0),
        )?;
        let ever_learnt: i64 = conn.query_row(
            "SELECT COUNT(*) FROM (SELECT 1 FROM progress.memory_review
             WHERE graduated = 1 GROUP BY book, chapter, verse)",
            [],
            |r| r.get(0),
        )?;
        let ever_completed = self.ever_completed_passage_ids()?.len() as i64;

        let achievements = achievements(&AchievementInputs {
            verses_learnt: ever_learnt.max(verses_learnt),
            verses_mature,
            passages_completed: ever_completed.max(passages_completed),
            best_streak,
            total_xp,
            goal_days,
        });

        Ok(MemoryStats {
            total_xp,
            level,
            level_xp: total_xp - level_floor,
            level_span,
            today_xp: self.memory_day_xp(today)?,
            daily_goal_xp: settings.daily_goal_xp,
            new_per_day: settings.new_per_day,
            streak_days: self.memory_streak(today)?,
            best_streak_days: best_streak,
            goal_days,
            verses_learnt,
            verses_mature,
            verses_learning,
            verses_total: distinct.len() as i64,
            due_now,
            passages_completed,
            passages_total: summaries.len() as i64,
            reviews_total,
            accuracy_pct,
            history,
            forecast,
            achievements,
        })
    }
}

struct AchievementInputs {
    verses_learnt: i64,
    verses_mature: i64,
    passages_completed: i64,
    best_streak: i64,
    total_xp: i64,
    goal_days: i64,
}

/// The achievement ladder. Each is derived from the log and verse states, so
/// nothing about it needs syncing on its own.
fn achievements(i: &AchievementInputs) -> Vec<MemoryAchievement> {
    let ladder: [(&str, &str, &str, i64, i64); 15] = [
        (
            "verse_1",
            "First words",
            "Learn your first verse by heart",
            i.verses_learnt,
            1,
        ),
        (
            "verse_10",
            "Ten verses",
            "Learn 10 verses by heart",
            i.verses_learnt,
            10,
        ),
        (
            "verse_50",
            "Fifty verses",
            "Learn 50 verses by heart",
            i.verses_learnt,
            50,
        ),
        (
            "verse_100",
            "Hundredfold",
            "Learn 100 verses by heart",
            i.verses_learnt,
            100,
        ),
        (
            "verse_500",
            "Scroll keeper",
            "Learn 500 verses by heart",
            i.verses_learnt,
            500,
        ),
        (
            "passage_1",
            "Whole passage",
            "Learn every verse of a passage",
            i.passages_completed,
            1,
        ),
        (
            "passage_5",
            "Five passages",
            "Complete 5 passages",
            i.passages_completed,
            5,
        ),
        (
            "mature_10",
            "Deep roots",
            "Keep 10 verses for three weeks or more",
            i.verses_mature,
            10,
        ),
        (
            "mature_50",
            "Planted by the waters",
            "Keep 50 verses mature",
            i.verses_mature,
            50,
        ),
        (
            "streak_3",
            "Three-day cord",
            "Practise 3 days in a row",
            i.best_streak,
            3,
        ),
        (
            "streak_7",
            "Sabbath week",
            "Practise 7 days in a row",
            i.best_streak,
            7,
        ),
        (
            "streak_40",
            "Forty days",
            "Practise 40 days in a row",
            i.best_streak,
            40,
        ),
        (
            "goal_7",
            "Goal keeper",
            "Meet your daily goal on 7 days",
            i.goal_days,
            7,
        ),
        ("xp_1000", "Thousand", "Earn 1,000 XP", i.total_xp, 1000),
        (
            "xp_10000",
            "Ten thousand",
            "Earn 10,000 XP",
            i.total_xp,
            10_000,
        ),
    ];
    ladder
        .into_iter()
        .map(
            |(key, title, description, progress, target)| MemoryAchievement {
                key: key.to_string(),
                title: title.to_string(),
                description: description.to_string(),
                progress: progress.min(target),
                target,
                earned: progress >= target,
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cue_ladder_climbs_then_graduates_and_lapses_to_heavy_cloze() {
        let now = 1_000_000;
        let s = VerseSrs::default();
        let (s, due) = s.graded(Grade::Good, now, now, false);
        assert_eq!((s.stage, s.interval_days), (STAGE_CLOZE_LIGHT, 0));
        assert_eq!(due, now + LEARNING_DELAY_SECS);
        let (s, _) = s.graded(Grade::Again, now, now, false);
        assert_eq!(s.stage, STAGE_CLOZE_LIGHT, "never drops back to reading");
        let (s, _) = s.graded(Grade::Easy, now, now, false);
        assert_eq!(s.stage, STAGE_INITIALS);
        let (s, _) = s.graded(Grade::Easy, now, now, false);
        assert_eq!(s.stage, STAGE_RECALL, "Easy never skips recall");
        assert!(!s.learnt());
        let (s, due) = s.graded(Grade::Good, now, now, false);
        assert_eq!(s.interval_days, 1);
        assert_eq!(due, now + SECONDS_PER_DAY);
        let (s, _) = s.graded(Grade::Good, now, now, false);
        assert_eq!(s.interval_days, 3);
        let (s, _) = s.graded(Grade::Good, now, now, false);
        assert_eq!(s.interval_days, 8);
        let (s, due) = s.graded(Grade::Again, now, now, false);
        assert_eq!(
            (s.stage, s.interval_days, s.lapses),
            (STAGE_CLOZE_HEAVY, 0, 1)
        );
        assert_eq!(due, now + RELEARN_DELAY_SECS);
    }

    #[test]
    fn early_run_through_success_keeps_the_schedule() {
        let now = 1_000_000;
        let learnt = VerseSrs {
            stage: STAGE_RECALL,
            interval_days: 10,
            ..Default::default()
        };
        let due = now + 5 * SECONDS_PER_DAY;
        assert_eq!(learnt.graded(Grade::Good, now, due, true), (learnt, due));
        let (hard, hard_due) = learnt.graded(Grade::Hard, now, due, true);
        assert_eq!(hard.interval_days, 10);
        assert_eq!(hard_due, now + SECONDS_PER_DAY);
        let (again, _) = learnt.graded(Grade::Again, now, due, true);
        assert_eq!(again.stage, STAGE_CLOZE_HEAVY);
    }

    #[test]
    fn words_drop_cantillation_and_join_punctuation() {
        let words = memory_words("בְּרֵאשִׁ֖ית בָּרָ֣א אֱלֹהִ֑ים ׀ הָאָֽרֶץ׃");
        assert_eq!(words, vec!["בְּרֵאשִׁית", "בָּרָא", "אֱלֹהִים ׀", "הָאָרֶץ׃"]);
        assert_eq!(first_letter("בְּרֵאשִׁית"), "ב");
    }

    #[test]
    fn cloze_hides_a_growing_share_and_varies_with_the_seed() {
        let count = |stage, seed| hidden_mask(9, stage, seed).iter().filter(|&&h| h).count();
        assert_eq!(count(STAGE_READ, 1), 0);
        assert_eq!(count(STAGE_CLOZE_LIGHT, 1), 3);
        assert_eq!(count(STAGE_CLOZE_HEAVY, 1), 6);
        assert_eq!(count(STAGE_INITIALS, 1), 9);
        assert_eq!(hidden_mask(1, STAGE_CLOZE_LIGHT, 7), vec![true]);
        let masks: std::collections::HashSet<_> = (0..20)
            .map(|seed| hidden_mask(9, STAGE_CLOZE_LIGHT, seed))
            .collect();
        assert!(masks.len() > 3, "different showings blank different words");
    }

    #[test]
    fn levels_grow_by_a_widening_step() {
        assert_eq!(level_for_xp(0), 1);
        assert_eq!(level_for_xp(99), 1);
        assert_eq!(level_for_xp(100), 2);
        assert_eq!(level_for_xp(299), 2);
        assert_eq!(level_for_xp(300), 3);
        assert_eq!(level_threshold(5), 1000);
    }

    fn test_bible() -> Option<Bible> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("haqor.db").exists() {
            return None;
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible.attach_progress_in_memory().expect("attach progress");
        Some(bible)
    }

    fn card_ref(item: &MemoryItem) -> (u8, u8) {
        let MemoryItem::Card(card) = item else {
            panic!("expected a card, got {item:?}");
        };
        (card.chapter, card.verse)
    }

    #[test]
    fn a_psalm_is_learnt_verse_by_verse_with_rationed_new_verses() -> rusqlite::Result<()> {
        let Some(bible) = test_bible() else {
            return Ok(());
        };
        let mut now = 1_700_000_000;
        // Psalm 23 (book 27 in Tanakh order), six verses.
        let passage = bible
            .add_memory_passage(27, 23, 0, 23, 255, "", now)?
            .expect("Psalm 23 exists");
        assert_eq!((passage.start_verse, passage.end_verse), (1, 6));
        let again = bible.add_memory_passage(27, 23, 1, 23, 6, "The shepherd psalm", now)?;
        assert_eq!(
            again.as_ref().map(|p| p.id.as_str()),
            Some(passage.id.as_str())
        );
        assert_eq!(
            again.map(|p| p.title),
            Some("The shepherd psalm".to_string())
        );
        bible.set_memory_settings(
            MemorySettings {
                new_per_day: 2,
                daily_goal_xp: 50,
            },
            now,
        )?;

        let first = bible.next_memory_item(&passage.id, false, now, 0)?;
        assert_eq!(card_ref(&first), (23, 1));
        let MemoryItem::Card(card) = &first else {
            unreachable!()
        };
        assert!(card.is_new);
        assert_eq!(card.stage, STAGE_READ);
        assert!(card.words.iter().all(|w| !w.hidden));
        assert!(card.words.iter().all(|w| !w.translit.is_empty()));
        assert_eq!((card.position, card.total), (1, 6));

        // Answer Good to everything offered for a while; only two verses may
        // start today, and both must graduate.
        let mut graduated = std::collections::HashSet::new();
        let mut outcomes = Vec::new();
        for _ in 0..40 {
            now += 60;
            match bible.next_memory_item(&passage.id, false, now, 0)? {
                MemoryItem::Card(card) => {
                    if card.stage == STAGE_INITIALS {
                        assert!(card.words.iter().all(|w| w.hidden && !w.hint.is_empty()));
                    }
                    if card.verse == 2 && card.stage >= STAGE_CLOZE_LIGHT {
                        assert!(!card.cue.is_empty(), "verse 2 chains from verse 1");
                    }
                    let outcome = bible.submit_memory_review(
                        &passage.id,
                        card.book,
                        card.chapter,
                        card.verse,
                        Grade::Good,
                        false,
                        now,
                        0,
                    )?;
                    if outcome.first_graduation {
                        graduated.insert(card.verse);
                    }
                    outcomes.push(outcome);
                }
                MemoryItem::Done { can_learn_more, .. } => {
                    assert!(can_learn_more);
                    break;
                }
                MemoryItem::Empty => panic!("passage has verses"),
            }
        }
        assert_eq!(graduated, [1, 2].into_iter().collect());
        let last = outcomes.last().expect("answers were given");
        assert!(outcomes.iter().any(|o| o.goal_reached_now));
        assert_eq!(last.streak_days, 1);
        assert!(last.total_xp > 0);

        // Asking for more starts verse 3 today regardless of the ration.
        let MemoryItem::Card(extra) = bible.next_memory_item(&passage.id, true, now, 0)? else {
            panic!("an extra verse was asked for");
        };
        assert_eq!(extra.verse, 3);

        let stats = bible.memory_stats(now, 0, 7, 7)?;
        assert_eq!(stats.verses_learnt, 2);
        assert_eq!(stats.verses_total, 6);
        assert_eq!(stats.history.len(), 7);
        assert_eq!(stats.history.last().map(|d| d.learnt_total), Some(2));
        assert_eq!(stats.forecast.iter().sum::<i64>(), 2);
        assert_eq!(stats.forecast[1], 2, "both come back tomorrow");
        assert!(
            stats
                .achievements
                .iter()
                .any(|a| a.key == "verse_1" && a.earned)
        );
        assert_eq!(stats.streak_days, 1);

        let summary = &bible.memory_passages(now)?[0];
        assert_eq!(summary.learnt, 2);
        assert_eq!(
            summary
                .verses
                .iter()
                .map(|v| v.strength)
                .collect::<Vec<_>>(),
            vec![3, 3, 0, 0, 0, 0]
        );
        assert!(summary.mastery_pct > 0);
        Ok(())
    }

    #[test]
    fn completing_a_passage_pays_its_bonus_once() -> rusqlite::Result<()> {
        let Some(bible) = test_bible() else {
            return Ok(());
        };
        let mut now = 1_700_000_000;
        // A one-verse passage: Genesis 1:1.
        let passage = bible
            .add_memory_passage(1, 1, 1, 1, 1, "", now)?
            .expect("Genesis 1:1 exists");
        let mut completions = 0;
        for _ in 0..6 {
            now += 60;
            let outcome =
                bible.submit_memory_review(&passage.id, 1, 1, 1, Grade::Good, false, now, 0)?;
            completions += outcome.completed_passages.len();
            if outcome.first_graduation {
                assert!(outcome.xp >= PASSAGE_BONUS_XP + GRADUATION_BONUS_XP);
            }
        }
        assert_eq!(completions, 1);
        // Forget it and learn it again: no second bonus.
        now += SECONDS_PER_DAY * 30;
        bible.submit_memory_review(&passage.id, 1, 1, 1, Grade::Again, false, now, 0)?;
        for _ in 0..4 {
            now += 60;
            let outcome =
                bible.submit_memory_review(&passage.id, 1, 1, 1, Grade::Good, false, now, 0)?;
            assert!(outcome.completed_passages.is_empty());
            assert!(!outcome.first_graduation);
        }
        let stats = bible.memory_stats(now, 0, 40, 7)?;
        assert_eq!(stats.passages_completed, 1);
        assert!(
            stats
                .achievements
                .iter()
                .any(|a| a.key == "passage_1" && a.earned)
        );

        // Deleting keeps the verse's progress for when it is added back.
        assert!(bible.delete_memory_passage(&passage.id, now)?);
        assert!(bible.memory_passages(now)?.is_empty());
        let back = bible
            .add_memory_passage(1, 1, 1, 1, 1, "", now)?
            .expect("exists");
        assert_ne!(back.id, passage.id);
        assert_eq!(bible.memory_passages(now)?[0].learnt, 1);
        Ok(())
    }
}
