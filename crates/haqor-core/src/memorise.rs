//! Learning passages of Scripture by heart, in order, a line at a time.
//!
//! The learner picks a *passage* — a range of verses, usually a chapter —
//! and shapes it: where each verse breaks into *lines* (the pauses they would
//! make reading it aloud) and where the passage breaks into *sections* of a
//! few verses. Nothing is shaped for them — working out what a verse says,
//! and so where it pauses, is the first step in remembering it — and a
//! section is learnt only once every verse in it is shaped. Learning can
//! start as soon as the first section is (see [`Bible::memory_layout`]).
//!
//! Learning never jumps about, and builds from the bottom up. Within a
//! section, verse by verse, starting from the first line of the first verse:
//!
//! 1. each line of the verse is **read**, then **recalled** with every word
//!    hidden, then recalled **together with every line before it** in the
//!    verse — so the verse builds up line by line, ending with it whole.
//!    While a line is learnt, the lines before it are shown as its context;
//! 2. from the second verse on, the section is recalled **from its first
//!    verse through this one**.
//!
//! When a section is complete and joins sections learnt before it, the
//! **passage so far** — every section learnt, from the beginning — is
//! recited end to end before the next section is begun, and again on a
//! growing interval after that.
//!
//! There is no read-through of a whole section first: shaping the passage
//! into lines and sections has already given the learner its overview.
//!
//! A card shows its text either whole or entirely hidden — never a scatter of
//! gaps — and always with the line before it as the cue to carry on from.
//!
//! A verse that has been through its steps is *learnt* and joins day-scale
//! spaced repetition (a verse-sized SM-2: 1 day, 3 days, then ease-scaled).
//! Reviews are recited a **section at a time**, in order, whenever any verse
//! in it falls due. A verse forgotten at review goes back into learning,
//! from recalling it whole.
//!
//! Every answer earns XP (a log in `progress.memory_review`), from which the
//! level, daily goal, streak, achievements and the dashboard's graphs are all
//! derived. Verse state and shape are keyed by the verse, not the passage: a
//! verse shared by two passages is learnt once, and deleting a passage keeps
//! what was learnt. Everything merges across devices (see
//! [`crate::progress_sync`]).

use rusqlite::{Connection, OptionalExtension, params};

use crate::bible::{Bible, pack_ref};
use crate::tutor::Grade;

const SECONDS_PER_DAY: i64 = 86_400;

/// Verse-sized SM-2 ease bounds.
const DEFAULT_EASE: f64 = 2.5;
const MIN_EASE: f64 = 1.3;
/// Spacing is capped so a long-known passage still comes round a few times a
/// year.
const MAX_INTERVAL_DAYS: i64 = 180;
/// An interval at or beyond which a verse counts as *mature*.
pub const MATURE_DAYS: i64 = 21;

/// `memory_verse.stage` of a verse forgotten at review: it re-enters its
/// steps at recalling the whole verse.
const RELEARN_STAGE: u8 = 255;

/// The passage-so-far run's spacing bounds, in days.
const RUN_MIN_DAYS: i64 = 3;
const RUN_MAX_DAYS: i64 = 30;

pub const DEFAULT_NEW_PER_DAY: i64 = 3;
pub const DEFAULT_DAILY_GOAL_XP: i64 = 100;

/// XP for completing a passage — every verse learnt — for the first time.
const PASSAGE_BONUS_XP: i64 = 100;
/// XP for a verse's first graduation from its learning steps.
const GRADUATION_BONUS_XP: i64 = 20;
/// XP for a card that is only read.
const READ_XP: i64 = 5;

/// Ways to bring more senses to a line, one shown per read card.
const SENSE_PROMPTS: [&str; 8] = [
    "Picture the scene: where are you standing, what do you see, who is speaking?",
    "Say it aloud with feeling. Which words carry the weight?",
    "Give each line a gesture, and use the same one every time you recite it.",
    "Walk as you recite, a line every few steps.",
    "Imagine telling this to someone who has never heard it.",
    "Listen to the sounds: which letters or words repeat?",
    "Read it, close your eyes, and see the words on the page.",
    "Act it out: what would you do with your hands, your face, your voice?",
];

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
    /// 0 not started, 1 early in its steps, 2 late in its steps (or being
    /// relearnt), 3 learnt (< 7 days), 4 established (< 21 days), 5 mature.
    pub strength: u8,
    pub due: bool,
    /// This verse begins a section.
    pub section_start: bool,
}

/// A passage with its progress, for the passage list and heatmap.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryPassageSummary {
    pub passage: MemoryPassage,
    pub verses: Vec<MemoryVerseState>,
    pub learnt: i64,
    pub mature: i64,
    pub due: i64,
    /// Overall mastery 0..=100: learning steps count a little, and a learnt
    /// verse counts fully once it reaches maturity.
    pub mastery_pct: i64,
    pub last_studied_epoch: i64,
    /// The next verse to start sits in a section not yet shaped: learning
    /// goes on once the learner shapes it.
    pub needs_shaping: bool,
}

/// One word of a card.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryWord {
    /// The word as it reads: cantillation removed, a maqaf-bound group kept
    /// together as one word.
    pub text: String,
    /// The word's learner gloss, when the verse's glosses align with its
    /// words; empty otherwise.
    pub gloss: String,
    pub translit: String,
}

/// One line of a verse on a card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySegment {
    pub chapter: u8,
    pub verse: u8,
    /// 0-based line within the verse, and how many lines the verse has.
    pub line: usize,
    pub line_count: usize,
    pub words: Vec<MemoryWord>,
}

/// What a card asks of the learner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPurpose {
    /// Read one line (text shown).
    Read,
    /// Recall one line, or a verse's lines so far (text hidden).
    Recall,
    /// Recall the section so far, through this verse (text hidden).
    Chain,
    /// Recite a section's learnt verses for review (text hidden).
    Review,
    /// Recite every learnt verse of the passage (text hidden).
    Run,
}

impl MemoryPurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            MemoryPurpose::Read => "read",
            MemoryPurpose::Recall => "recall",
            MemoryPurpose::Chain => "chain",
            MemoryPurpose::Review => "review",
            MemoryPurpose::Run => "run",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "read" => MemoryPurpose::Read,
            "recall" => MemoryPurpose::Recall,
            "chain" => MemoryPurpose::Chain,
            "review" => MemoryPurpose::Review,
            "run" => MemoryPurpose::Run,
            _ => return None,
        })
    }

    /// Whether the card's text is hidden, to be recited.
    pub fn hidden(self) -> bool {
        self != MemoryPurpose::Read
    }

    /// Whether the card is a step of one verse's learning script.
    fn is_learning(self) -> bool {
        matches!(
            self,
            MemoryPurpose::Read | MemoryPurpose::Recall | MemoryPurpose::Chain
        )
    }

    /// Code stored in `memory_review.stage`.
    fn code(self) -> i64 {
        match self {
            MemoryPurpose::Read => 11,
            MemoryPurpose::Recall => 12,
            MemoryPurpose::Chain => 13,
            MemoryPurpose::Review => 14,
            MemoryPurpose::Run => 15,
        }
    }
}

/// One card to work through.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryCard {
    /// The passage it is being practised as part of (empty for "all").
    pub passage_id: String,
    pub book: u8,
    pub purpose: MemoryPurpose,
    /// A short heading ("Verse 23:4 · line 2 of 3").
    pub title: String,
    /// What to do, including a prompt to engage the senses on read cards.
    pub prompt: String,
    /// The lines to read or recite, in order.
    pub segments: Vec<MemorySegment>,
    /// The line before the first segment, to carry on from; empty at the
    /// start of the passage.
    pub cue: String,
    /// For a learning step, the verse it advances and its step (0-based) out
    /// of `step_count`; zero otherwise.
    pub target_chapter: u8,
    pub target_verse: u8,
    pub step: usize,
    pub step_count: usize,
    /// First time the target verse is met.
    pub is_new: bool,
    /// The target (or first) verse's position in the passage (1-based) and
    /// the passage's length; its section (1-based) and the section count.
    pub position: usize,
    pub total: usize,
    pub section: usize,
    pub section_count: usize,
}

/// What the scheduler offers next.
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryItem {
    Card(MemoryCard),
    /// Nothing due and no new verse allowed today. `next_due_epoch` is when
    /// the next review falls (0 if nothing is scheduled); `can_learn_more`
    /// says whether an unstarted verse remains that the learner could choose
    /// to start anyway; `shape_passage_id` names a passage whose next verse
    /// waits for its section to be shaped (else empty).
    Done {
        next_due_epoch: i64,
        can_learn_more: bool,
        shape_passage_id: String,
    },
    /// The scope holds no passages (or no verses) at all.
    Empty,
}

/// How one verse of a recited card went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryVerseGrade {
    pub chapter: u8,
    pub verse: u8,
    pub grade: Grade,
}

/// What an answer earned, for the celebration after it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryReviewOutcome {
    pub xp: i64,
    /// The answer graduated the target verse for the first time.
    pub first_graduation: bool,
    /// The answer completed the target verse's section.
    pub section_completed: bool,
    /// Passages this answer completed (every verse learnt) for the first time.
    pub completed_passages: Vec<String>,
    /// Verses of a recited card that were forgotten and go back to learning.
    pub relearn: i64,
    /// Days until the target verse (or the run) comes back; 0 while learning.
    pub interval_days: i64,
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

/// One verse of a passage as the shaping page shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryLayoutVerse {
    pub chapter: u8,
    pub verse: u8,
    pub words: Vec<String>,
    /// Each word's gloss, in parallel with `words` (empty where unknown):
    /// the learner splits a verse by its sense, so must see what it says.
    pub glosses: Vec<String>,
    /// Word indexes at which a new line starts (never 0), ascending.
    pub line_starts: Vec<usize>,
    pub section_start: bool,
    /// The learner has decided this verse's lines (a verse kept whole is
    /// shaped too). There is no default: an unshaped verse cannot be learnt.
    pub shaped: bool,
    /// This verse's section is ready to learn: every verse in it is shaped
    /// and the section is closed, by the start of the next one or by the end
    /// of the passage.
    pub ready: bool,
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

/// Mutable per-verse scheduling state. While learning, `stage` is the index
/// of the verse's next step (or [`RELEARN_STAGE`]).
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
            stage: 0,
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

    /// First graduation from the learning steps.
    fn graduate(self, grade: Grade, now: i64) -> (VerseSrs, i64) {
        let mut s = self;
        s.reps = self.reps + 1;
        s.interval_days = if grade == Grade::Easy { 3 } else { 1 };
        if grade == Grade::Easy {
            s.ease = self.ease + 0.15;
        }
        (s, now + s.interval_days * SECONDS_PER_DAY)
    }

    /// Grade a learnt verse recited at `now`, SM-2 style. A lapse sends it
    /// back into learning. `early` marks a verse recited before it was due
    /// (it came along with its section): success then leaves its schedule
    /// alone — reviewing ahead must not inflate the spacing — while trouble
    /// still brings it forward.
    fn reviewed(self, grade: Grade, now: i64, due_epoch: i64, early: bool) -> (VerseSrs, i64) {
        let mut s = self;
        match grade {
            Grade::Again => {
                s.ease = (self.ease - 0.20).max(MIN_EASE);
                s.lapses = self.lapses + 1;
                s.interval_days = 0;
                s.stage = RELEARN_STAGE;
                return (s, now);
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

/// One step of a verse's learning script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// Read one line.
    Read(usize),
    /// Recall lines `from..=to` of the verse.
    Recall(usize, usize),
    /// Recall the section from its first verse through this one.
    Chain,
}

/// The learning script of a verse with `lines` lines: see the module docs.
fn verse_steps(lines: usize, first_in_section: bool) -> Vec<Step> {
    let mut steps = Vec::new();
    for k in 0..lines.max(1) {
        steps.push(Step::Read(k));
        steps.push(Step::Recall(k, k));
        if k > 0 {
            steps.push(Step::Recall(0, k));
        }
    }
    if !first_in_section {
        steps.push(Step::Chain);
    }
    steps
}

/// Where a verse forgotten at review re-enters its script: recalling the
/// whole verse.
fn relearn_step(steps: &[Step], lines: usize) -> usize {
    let whole = Step::Recall(0, lines.max(1) - 1);
    steps.iter().position(|&s| s == whole).unwrap_or(0)
}

/// Heatmap strength of a verse; see [`MemoryVerseState::strength`].
fn strength(state: Option<(VerseSrs, i64)>, step_count: usize) -> u8 {
    match state {
        None => 0,
        Some((s, _)) if !s.learnt() => {
            if s.stage == RELEARN_STAGE || usize::from(s.stage) * 2 >= step_count {
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
fn mastery(state: Option<(VerseSrs, i64)>, step_count: usize) -> f64 {
    match state {
        None => 0.0,
        Some((s, _)) if !s.learnt() => {
            let done = if s.stage == RELEARN_STAGE {
                step_count
            } else {
                usize::from(s.stage)
            };
            0.4 * done as f64 / step_count.max(1) as f64
        }
        Some((s, _)) => 0.5 + 0.5 * (s.interval_days as f64 / MATURE_DAYS as f64).min(1.0),
    }
}

/// XP for reciting one verse (or line) with `grade`; a recital from memory
/// that went well earns a bonus.
fn recital_xp(grade: Grade, review: bool) -> i64 {
    let base = match grade {
        Grade::Again => 2,
        Grade::Hard => 6,
        Grade::Good => 10,
        Grade::Easy => 12,
    };
    base + if review && matches!(grade, Grade::Good | Grade::Easy) {
        5
    } else {
        0
    }
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

const MAQAF: char = '\u{05BE}';

/// A word of a verse, before display: its text as written (accents and all)
/// and its gloss.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Unit {
    raw: String,
    gloss: String,
}

impl Unit {
    fn text(&self) -> String {
        self.raw.chars().filter(|&c| !is_cantillation(c)).collect()
    }
}

/// Split a verse's text into words, pairing each with its gloss when the
/// glosses align with the text's tokens. Tokens with no letter (a paseq, a
/// lone sof pasuq) join the word before; a maqaf joins a word to the next,
/// as they are said as one.
fn verse_units(text: &str, glosses: &[String]) -> Vec<Unit> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let aligned = glosses.len() == tokens.len();
    let mut units: Vec<Unit> = Vec::new();
    let mut joining = false;
    for (i, token) in tokens.iter().enumerate() {
        let gloss = if aligned { glosses[i].trim() } else { "" };
        let letterless = !token.chars().any(char::is_alphabetic);
        match units.last_mut() {
            Some(last) if joining || letterless => {
                if !joining {
                    last.raw.push(' ');
                }
                last.raw.push_str(token);
                if !gloss.is_empty() {
                    if !last.gloss.is_empty() {
                        last.gloss.push(' ');
                    }
                    last.gloss.push_str(gloss);
                }
            }
            _ => units.push(Unit {
                raw: token.to_string(),
                gloss: gloss.to_string(),
            }),
        }
        joining = token.ends_with(MAQAF);
    }
    units
}

/// The words of a verse as they are memorised: see `verse_units`.
pub fn memory_words(text: &str) -> Vec<String> {
    verse_units(text, &[]).iter().map(Unit::text).collect()
}

/// Parse a stored `line_starts` list, keeping only breaks inside the verse.
fn parse_line_starts(stored: &str, words: usize) -> Vec<usize> {
    let mut starts: Vec<usize> = stored
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .filter(|&i| i > 0 && i < words)
        .collect();
    starts.sort_unstable();
    starts.dedup();
    starts
}

/// A verse of a passage, shaped into lines.
#[derive(Debug, Clone)]
struct PlanVerse {
    chapter: u8,
    verse: u8,
    units: Vec<Unit>,
    /// Word ranges of its lines, in order.
    lines: Vec<std::ops::Range<usize>>,
    section_start: bool,
    /// The learner has set its lines. An unshaped verse is held as one line
    /// (so a verse already begun can carry on), but is never started.
    shaped: bool,
}

impl PlanVerse {
    fn line_text(&self, line: usize) -> String {
        self.units[self.lines[line].clone()]
            .iter()
            .map(Unit::text)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// A passage shaped into sections of lined verses.
#[derive(Debug, Clone)]
struct Plan {
    verses: Vec<PlanVerse>,
    /// Index of the first verse of each section, ascending, starting at 0.
    sections: Vec<usize>,
}

impl Plan {
    /// The section holding verse `i`, as (section index, first verse, end).
    fn section_of(&self, i: usize) -> (usize, usize, usize) {
        let s = self.sections.partition_point(|&start| start <= i) - 1;
        let end = self
            .sections
            .get(s + 1)
            .copied()
            .unwrap_or(self.verses.len());
        (s, self.sections[s], end)
    }

    /// Whether verse `i`'s section can be learnt: every verse in it is
    /// shaped. A section runs to the next section start, or to the end of the
    /// passage, so learning can begin as soon as the learner has shaped one
    /// section and marked where the next begins.
    fn ready(&self, i: usize) -> bool {
        let (_, first, end) = self.section_of(i);
        self.verses[first..end].iter().all(|v| v.shaped)
    }

    fn steps(&self, i: usize) -> Vec<Step> {
        let (_, first, _) = self.section_of(i);
        verse_steps(self.verses[i].lines.len(), i == first)
    }
}

/// The index a stored stage points at in `steps` (clamped: the verse's shape
/// may have changed since).
fn resolve_step(stage: u8, steps: &[Step], lines: usize) -> usize {
    if stage == RELEARN_STAGE {
        relearn_step(steps, lines)
    } else {
        usize::from(stage).min(steps.len().saturating_sub(1))
    }
}

/// Create the memorisation tables in the attached `progress` schema.
/// Idempotent; called from [`crate::tutor::init_progress_schema`] so the sync
/// server's canonical database has them too.
pub fn init_memory_schema(db: &Connection) -> rusqlite::Result<()> {
    ensure_memory_tables(db, "progress")?;
    // A verse caught half-way through its steps when their numbering changed
    // restarts them: the first release's cloze ladder, then the dropped
    // section preview (version 1), then the part-verse recitals of a verse of
    // three or more lines, dropped (version 2) and brought back (version 3). What was
    // already learnt, and a verse waiting to be relearnt, are kept.
    let version: Option<String> = db
        .query_row(
            "SELECT value FROM progress.meta WHERE key = 'memory.steps'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if version.as_deref() != Some("4") {
        db.execute_batch(&format!(
            "UPDATE progress.memory_verse SET stage = 0
             WHERE interval_days = 0 AND stage != {RELEARN_STAGE};
             INSERT INTO progress.meta(key, value) VALUES ('memory.steps', '4')
             ON CONFLICT(key) DO UPDATE SET value = excluded.value;"
        ))?;
    }
    Ok(())
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
         );
         CREATE TABLE IF NOT EXISTS {schema}.memory_layout(
            book          INTEGER NOT NULL,
            chapter       INTEGER NOT NULL,
            verse         INTEGER NOT NULL,
            line_starts   TEXT,
            section_start INTEGER,
            updated_epoch INTEGER NOT NULL,
            PRIMARY KEY (book, chapter, verse)
         );
         CREATE TABLE IF NOT EXISTS {schema}.memory_passage_run(
            passage_id     TEXT    PRIMARY KEY,
            due_epoch      INTEGER NOT NULL,
            interval_days  INTEGER NOT NULL,
            last_run_epoch INTEGER NOT NULL,
            updated_epoch  INTEGER NOT NULL
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
         WHERE excluded.updated_epoch > progress.memory_settings.updated_epoch;

         INSERT INTO progress.memory_layout(
             book, chapter, verse, line_starts, section_start, updated_epoch)
         SELECT book, chapter, verse, line_starts, section_start, updated_epoch
         FROM sync.memory_layout WHERE true
         ON CONFLICT(book, chapter, verse) DO UPDATE SET
            line_starts=excluded.line_starts, section_start=excluded.section_start,
            updated_epoch=excluded.updated_epoch
         WHERE excluded.updated_epoch > progress.memory_layout.updated_epoch;

         INSERT INTO progress.memory_passage_run(
             passage_id, due_epoch, interval_days, last_run_epoch, updated_epoch)
         SELECT passage_id, due_epoch, interval_days, last_run_epoch, updated_epoch
         FROM sync.memory_passage_run WHERE true
         ON CONFLICT(passage_id) DO UPDATE SET
            due_epoch=excluded.due_epoch, interval_days=excluded.interval_days,
            last_run_epoch=excluded.last_run_epoch, updated_epoch=excluded.updated_epoch
         WHERE excluded.updated_epoch > progress.memory_passage_run.updated_epoch;",
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

    // --- shape ---------------------------------------------------------------

    /// Shape a passage into sections of lined verses, from the learner's
    /// layout. Nothing is shaped for them: a verse they have not split is
    /// one unshaped line, and the passage is one section until they mark
    /// where the next begins.
    fn memory_plan(&self, p: &MemoryPassage) -> rusqlite::Result<Plan> {
        let refs = self.memory_passage_verses(p)?;
        let mut stored = std::collections::HashMap::new();
        {
            let mut stmt = self.conn().prepare(
                "SELECT chapter, verse, line_starts, section_start FROM progress.memory_layout
                 WHERE book = ?1 AND ((chapter << 8) | verse) BETWEEN ?2 AND ?3",
            )?;
            let rows = stmt.query_map(
                params![
                    p.book,
                    (i64::from(p.start_chapter) << 8) | i64::from(p.start_verse),
                    (i64::from(p.end_chapter) << 8) | i64::from(p.end_verse)
                ],
                |r| {
                    Ok((
                        (r.get::<_, u8>(0)?, r.get::<_, u8>(1)?),
                        (r.get::<_, Option<String>>(2)?, r.get::<_, Option<i64>>(3)?),
                    ))
                },
            )?;
            for row in rows {
                let (key, value) = row?;
                stored.insert(key, value);
            }
        }
        let n = refs.len();
        let mut verses = Vec::with_capacity(n);
        for (i, &(chapter, verse)) in refs.iter().enumerate() {
            let units = verse_units(&self.get(p.book, chapter, verse)?, &[]);
            let (lines_stored, section_stored) = stored
                .get(&(chapter, verse))
                .cloned()
                .unwrap_or((None, None));
            let starts = lines_stored
                .as_deref()
                .map_or_else(Vec::new, |s| parse_line_starts(s, units.len()));
            let mut bounds = vec![0];
            bounds.extend(&starts);
            bounds.push(units.len());
            let lines = bounds.windows(2).map(|w| w[0]..w[1]).collect();
            let section_start = i == 0 || section_stored.is_some_and(|s| s != 0);
            verses.push(PlanVerse {
                chapter,
                verse,
                units,
                lines,
                section_start,
                shaped: lines_stored.is_some(),
            });
        }
        let sections = verses
            .iter()
            .enumerate()
            .filter(|(_, v)| v.section_start)
            .map(|(i, _)| i)
            .collect();
        Ok(Plan { verses, sections })
    }

    /// Every verse of a passage with its words, glosses, lines and section
    /// breaks, for the shaping page.
    pub fn memory_layout(&self, passage_id: &str) -> rusqlite::Result<Vec<MemoryLayoutVerse>> {
        let Some(p) = self.memory_passage(passage_id)? else {
            return Ok(Vec::new());
        };
        let plan = self.memory_plan(&p)?;
        let mut out = Vec::with_capacity(plan.verses.len());
        for (i, v) in plan.verses.iter().enumerate() {
            let words = self.memory_card_words(p.book, v)?;
            out.push(MemoryLayoutVerse {
                chapter: v.chapter,
                verse: v.verse,
                glosses: words.iter().map(|w| w.gloss.clone()).collect(),
                words: words.into_iter().map(|w| w.text).collect(),
                line_starts: v.lines.iter().skip(1).map(|r| r.start).collect(),
                section_start: v.section_start,
                shaped: v.shaped,
                ready: plan.ready(i),
            });
        }
        Ok(out)
    }

    /// Set where a verse's lines start (word indexes, never 0); an empty
    /// list keeps it whole, and `None` leaves it unshaped again. Shaping a
    /// verse is itself a way into it: the learner decides, from what it says,
    /// where the pauses and the emphasis fall. It is also the gate to
    /// learning it — there is no default split to fall back on.
    pub fn set_memory_line_starts(
        &self,
        book: u8,
        chapter: u8,
        verse: u8,
        line_starts: Option<&[usize]>,
        now: i64,
    ) -> rusqlite::Result<()> {
        let stored = line_starts.map(|s| {
            let mut s = s.to_vec();
            s.sort_unstable();
            s.dedup();
            s.iter()
                .filter(|&&i| i > 0)
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(",")
        });
        self.conn().execute(
            "INSERT INTO progress.memory_layout(
                 book, chapter, verse, line_starts, section_start, updated_epoch)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5)
             ON CONFLICT(book, chapter, verse) DO UPDATE SET
                line_starts = excluded.line_starts,
                updated_epoch = MAX(progress.memory_layout.updated_epoch + 1,
                                    excluded.updated_epoch)",
            params![book, chapter, verse, stored, now],
        )?;
        Ok(())
    }

    /// Set whether a new section starts at a verse. The first verse of a
    /// passage always starts one; without any break the passage is a single
    /// section.
    pub fn set_memory_section_start(
        &self,
        book: u8,
        chapter: u8,
        verse: u8,
        section_start: bool,
        now: i64,
    ) -> rusqlite::Result<()> {
        self.conn().execute(
            "INSERT INTO progress.memory_layout(
                 book, chapter, verse, line_starts, section_start, updated_epoch)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5)
             ON CONFLICT(book, chapter, verse) DO UPDATE SET
                section_start = excluded.section_start,
                updated_epoch = MAX(progress.memory_layout.updated_epoch + 1,
                                    excluded.updated_epoch)",
            params![book, chapter, verse, i64::from(section_start), now],
        )?;
        Ok(())
    }

    pub fn memory_passage_summary(
        &self,
        passage: MemoryPassage,
        now: i64,
    ) -> rusqlite::Result<MemoryPassageSummary> {
        let plan = self.memory_plan(&passage)?;
        let mut verses = Vec::new();
        let (mut learnt, mut mature, mut due, mut total) = (0, 0, 0, 0.0);
        let mut needs_shaping = None;
        for (i, v) in plan.verses.iter().enumerate() {
            let state = self.memory_verse_srs(passage.book, v.chapter, v.verse)?;
            let steps = plan.steps(i).len();
            let is_due = state.is_some_and(|(s, d)| s.learnt() && d <= now);
            if needs_shaping.is_none() && !state.is_some_and(|(s, _)| s.learnt()) {
                // Learning goes in order: the first verse not learnt is the
                // next one, and a verse not yet begun waits on its shape.
                needs_shaping = Some(state.is_none() && !plan.ready(i));
            }
            if let Some((s, _)) = state {
                learnt += i64::from(s.learnt());
                mature += i64::from(s.interval_days >= MATURE_DAYS);
            }
            due += i64::from(is_due);
            total += mastery(state, steps);
            verses.push(MemoryVerseState {
                chapter: v.chapter,
                verse: v.verse,
                strength: strength(state, steps),
                due: is_due,
                section_start: v.section_start,
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
        let mastery_pct = if plan.verses.is_empty() {
            0
        } else {
            (total * 100.0 / plan.verses.len() as f64).round() as i64
        };
        Ok(MemoryPassageSummary {
            passage,
            verses,
            learnt,
            mature,
            due,
            mastery_pct,
            last_studied_epoch,
            needs_shaping: needs_shaping.unwrap_or(false),
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

    /// A card's words for one verse, with glosses and transliteration.
    fn memory_card_words(&self, book: u8, v: &PlanVerse) -> rusqlite::Result<Vec<MemoryWord>> {
        let glosses: Vec<String> = self
            .verse_gloss_words(book, v.chapter, v.verse)
            .map(|pairs| pairs.into_iter().map(|(_, g)| g).collect())
            .unwrap_or_default();
        let mut units = verse_units(&self.get(book, v.chapter, v.verse)?, &glosses);
        if units.len() != v.units.len() {
            units = v.units.clone();
        }
        Ok(units
            .iter()
            .map(|u| {
                let text = u.text();
                MemoryWord {
                    translit: if book < 40 {
                        crate::romanize::romanize(&text)
                    } else {
                        String::new()
                    },
                    gloss: u.gloss.clone(),
                    text,
                }
            })
            .collect())
    }

    /// Segments for `lines` of plan verse `i` (a line range within it).
    fn memory_segments(
        &self,
        book: u8,
        plan: &Plan,
        parts: &[(usize, std::ops::RangeInclusive<usize>)],
    ) -> rusqlite::Result<Vec<MemorySegment>> {
        let mut out = Vec::new();
        for (i, lines) in parts {
            let v = &plan.verses[*i];
            let words = self.memory_card_words(book, v)?;
            for line in lines.clone() {
                out.push(MemorySegment {
                    chapter: v.chapter,
                    verse: v.verse,
                    line,
                    line_count: v.lines.len(),
                    words: words[v.lines[line].clone()].to_vec(),
                });
            }
        }
        Ok(out)
    }

    /// The cue to carry on from into line `line` of plan verse `i`: every
    /// earlier line of the verse, one per row, so a line is never learnt out
    /// of its place in the verse; for a verse's first line, the end of the
    /// verse before.
    fn memory_cue(plan: &Plan, i: usize, line: usize) -> String {
        if line > 0 {
            (0..line)
                .map(|l| plan.verses[i].line_text(l))
                .collect::<Vec<_>>()
                .join(
                    "
",
                )
        } else if i > 0 {
            let prev = &plan.verses[i - 1];
            format!("…{}", prev.line_text(prev.lines.len() - 1))
        } else {
            String::new()
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn memory_card_shell(
        passage_id: &str,
        p: &MemoryPassage,
        plan: &Plan,
        i: usize,
        purpose: MemoryPurpose,
        title: String,
        prompt: String,
        segments: Vec<MemorySegment>,
        cue: String,
    ) -> MemoryCard {
        let (section, _, _) = plan.section_of(i);
        MemoryCard {
            passage_id: passage_id.to_string(),
            book: p.book,
            purpose,
            title,
            prompt,
            segments,
            cue,
            target_chapter: 0,
            target_verse: 0,
            step: 0,
            step_count: 0,
            is_new: false,
            position: i + 1,
            total: plan.verses.len(),
            section: section + 1,
            section_count: plan.sections.len(),
        }
    }

    /// The card for step `step` of plan verse `i`.
    fn memory_step_card(
        &self,
        passage_id: &str,
        p: &MemoryPassage,
        plan: &Plan,
        i: usize,
        step: usize,
        is_new: bool,
    ) -> rusqlite::Result<MemoryCard> {
        let steps = plan.steps(i);
        let v = &plan.verses[i];
        let n = v.lines.len();
        let (_, first, _) = plan.section_of(i);
        let at = |i: usize| {
            let v = &plan.verses[i];
            format!("{}:{}", v.chapter, v.verse)
        };
        let recite = "Recite it aloud, revealing each word as you say it — or reveal it \
                      all at the end.";
        let sense = SENSE_PROMPTS
            [(usize::from(v.chapter) * 7 + usize::from(v.verse) * 3 + step) % SENSE_PROMPTS.len()];
        let (purpose, title, prompt, segments, cue) = match steps[step] {
            Step::Read(k) => (
                MemoryPurpose::Read,
                if n > 1 {
                    format!("{} · line {} of {n}", at(i), k + 1)
                } else {
                    at(i)
                },
                format!("Read it aloud two or three times. {sense}"),
                self.memory_segments(p.book, plan, &[(i, k..=k)])?,
                Self::memory_cue(plan, i, k),
            ),
            Step::Recall(a, b) => (
                MemoryPurpose::Recall,
                match (a, b) {
                    _ if n == 1 => format!("{} from memory", at(i)),
                    (a, b) if a == b => format!("{} · line {} from memory", at(i), a + 1),
                    (0, b) if b + 1 == n => format!("{} · the whole verse", at(i)),
                    (_, b) => format!("{} · lines 1–{} together", at(i), b + 1),
                },
                recite.to_string(),
                self.memory_segments(p.book, plan, &[(i, a..=b)])?,
                Self::memory_cue(plan, i, a),
            ),
            Step::Chain => (
                MemoryPurpose::Chain,
                format!("{}–{} together", at(first), at(i)),
                "Now the section so far, from the top. Keep the flow going from one \
                 verse into the next."
                    .to_string(),
                self.memory_segments(
                    p.book,
                    plan,
                    &(first..=i)
                        .map(|j| (j, 0..=plan.verses[j].lines.len() - 1))
                        .collect::<Vec<_>>(),
                )?,
                Self::memory_cue(plan, first, 0),
            ),
        };
        let mut card = Self::memory_card_shell(
            passage_id, p, plan, i, purpose, title, prompt, segments, cue,
        );
        card.target_chapter = v.chapter;
        card.target_verse = v.verse;
        card.step = step;
        card.step_count = steps.len();
        card.is_new = is_new;
        Ok(card)
    }

    /// A recital of the learnt verses among plan verses `range`, for review
    /// (`purpose` Review) or the passage-so-far run (`purpose` Run).
    fn memory_recital_card(
        &self,
        passage_id: &str,
        p: &MemoryPassage,
        plan: &Plan,
        indexes: &[usize],
        purpose: MemoryPurpose,
    ) -> rusqlite::Result<MemoryCard> {
        let (first, last) = (indexes[0], indexes[indexes.len() - 1]);
        let at = |i: usize| {
            let v = &plan.verses[i];
            format!("{}:{}", v.chapter, v.verse)
        };
        let span = if first == last {
            at(first)
        } else {
            format!("{}–{}", at(first), at(last))
        };
        let (title, prompt) = match purpose {
            MemoryPurpose::Run => (
                format!("The passage so far · {span}"),
                "Recite everything you have learnt, from the beginning, in one flow. \
                 See each scene as you go."
                    .to_string(),
            ),
            _ => (
                format!("Review · {span}"),
                "Recite the section from memory, revealing each word as you say it.".to_string(),
            ),
        };
        let segments = self.memory_segments(
            p.book,
            plan,
            &indexes
                .iter()
                .map(|&j| (j, 0..=plan.verses[j].lines.len() - 1))
                .collect::<Vec<_>>(),
        )?;
        Ok(Self::memory_card_shell(
            passage_id,
            p,
            plan,
            first,
            purpose,
            title,
            prompt,
            segments,
            Self::memory_cue(plan, first, 0),
        ))
    }

    fn memory_run_state(&self, passage_id: &str) -> rusqlite::Result<Option<(i64, i64)>> {
        self.conn()
            .query_row(
                "SELECT due_epoch, interval_days FROM progress.memory_passage_run
                 WHERE passage_id = ?1",
                [passage_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
    }

    fn set_memory_run_state(
        &self,
        passage_id: &str,
        due_epoch: i64,
        interval_days: i64,
        last_run_epoch: i64,
        now: i64,
    ) -> rusqlite::Result<()> {
        self.conn().execute(
            "INSERT INTO progress.memory_passage_run(
                 passage_id, due_epoch, interval_days, last_run_epoch, updated_epoch)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(passage_id) DO UPDATE SET
                due_epoch = excluded.due_epoch, interval_days = excluded.interval_days,
                last_run_epoch = MAX(progress.memory_passage_run.last_run_epoch,
                                     excluded.last_run_epoch),
                updated_epoch = MAX(progress.memory_passage_run.updated_epoch + 1,
                                    excluded.updated_epoch)",
            params![passage_id, due_epoch, interval_days, last_run_epoch, now],
        )?;
        Ok(())
    }

    /// Learnt plan verses of a passage, and the sections they fall in.
    fn memory_learnt(
        &self,
        p: &MemoryPassage,
        plan: &Plan,
    ) -> rusqlite::Result<Vec<(usize, VerseSrs, i64)>> {
        let mut out = Vec::new();
        for (i, v) in plan.verses.iter().enumerate() {
            if let Some((s, due)) = self.memory_verse_srs(p.book, v.chapter, v.verse)?
                && s.learnt()
            {
                out.push((i, s, due));
            }
        }
        Ok(out)
    }

    /// The next card within `passage_id` (empty = all passages).
    ///
    /// Order: a section with a verse due for review (recited whole, in
    /// passage order); the passage-so-far run, when due; then the next step
    /// of the first verse not yet learnt — a verse is never started while an
    /// earlier one is unfinished, a new verse only within today's ration (or
    /// when `extra_new` asks for one anyway), and never one whose section the
    /// learner has not shaped yet.
    pub fn next_memory_item(
        &self,
        passage_id: &str,
        extra_new: bool,
        now: i64,
        utc_offset: i64,
    ) -> rusqlite::Result<MemoryItem> {
        let scope = self.memory_scope(passage_id)?;
        let mut shaped = Vec::new();
        for p in scope {
            let plan = self.memory_plan(&p)?;
            if !plan.verses.is_empty() {
                shaped.push((p, plan));
            }
        }
        if shaped.is_empty() {
            return Ok(MemoryItem::Empty);
        }

        let mut next_due = i64::MAX;
        for (p, plan) in &shaped {
            let learnt = self.memory_learnt(p, plan)?;
            for &(_, _, due) in &learnt {
                next_due = next_due.min(due);
            }
            for (s, &start) in plan.sections.iter().enumerate() {
                let end = plan
                    .sections
                    .get(s + 1)
                    .copied()
                    .unwrap_or(plan.verses.len());
                let in_section: Vec<_> = learnt
                    .iter()
                    .filter(|(i, _, _)| (start..end).contains(i))
                    .collect();
                if in_section.iter().any(|(_, _, due)| *due <= now) {
                    let indexes: Vec<usize> = in_section.iter().map(|(i, _, _)| *i).collect();
                    return Ok(MemoryItem::Card(self.memory_recital_card(
                        passage_id,
                        p,
                        plan,
                        &indexes,
                        MemoryPurpose::Review,
                    )?));
                }
            }
        }

        for (p, plan) in &shaped {
            let Some((due, _)) = self.memory_run_state(&p.id)? else {
                continue;
            };
            next_due = next_due.min(due);
            if due > now {
                continue;
            }
            let learnt = self.memory_learnt(p, plan)?;
            let sections: std::collections::HashSet<usize> = learnt
                .iter()
                .map(|(i, _, _)| plan.section_of(*i).0)
                .collect();
            if sections.len() >= 2 {
                let indexes: Vec<usize> = learnt.iter().map(|(i, _, _)| *i).collect();
                return Ok(MemoryItem::Card(self.memory_recital_card(
                    passage_id,
                    p,
                    plan,
                    &indexes,
                    MemoryPurpose::Run,
                )?));
            }
        }

        let settings = self.memory_settings()?;
        let started_today: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM progress.memory_verse
             WHERE (introduced_epoch + ?2) / 86400 = ?1",
            params![local_day(now, utc_offset), utc_offset],
            |r| r.get(0),
        )?;
        let mut can_learn_more = false;
        let mut shape_passage_id = String::new();
        for (p, plan) in &shaped {
            for (i, v) in plan.verses.iter().enumerate() {
                let state = self.memory_verse_srs(p.book, v.chapter, v.verse)?;
                match state {
                    Some((s, _)) if s.learnt() => continue,
                    Some((s, _)) => {
                        let steps = plan.steps(i);
                        let step = resolve_step(s.stage, &steps, v.lines.len());
                        return Ok(MemoryItem::Card(
                            self.memory_step_card(passage_id, p, plan, i, step, false)?,
                        ));
                    }
                    // Its section is not shaped yet: nothing more of this
                    // passage until the learner decides how it goes.
                    None if !plan.ready(i) => {
                        if shape_passage_id.is_empty() {
                            shape_passage_id = p.id.clone();
                        }
                        break;
                    }
                    None if extra_new || started_today < settings.new_per_day => {
                        return Ok(MemoryItem::Card(
                            self.memory_step_card(passage_id, p, plan, i, 0, true)?,
                        ));
                    }
                    None => {
                        can_learn_more = true;
                        break;
                    }
                }
            }
            if can_learn_more {
                break;
            }
        }
        Ok(MemoryItem::Done {
            next_due_epoch: if next_due == i64::MAX { 0 } else { next_due },
            can_learn_more,
            shape_passage_id,
        })
    }

    /// Recite every learnt verse of a passage now, whether due or not.
    pub fn memory_run_card(&self, passage_id: &str) -> rusqlite::Result<Option<MemoryCard>> {
        let Some(p) = self.memory_passage(passage_id)? else {
            return Ok(None);
        };
        let plan = self.memory_plan(&p)?;
        let indexes: Vec<usize> = self
            .memory_learnt(&p, &plan)?
            .iter()
            .map(|(i, _, _)| *i)
            .collect();
        if indexes.is_empty() {
            return Ok(None);
        }
        self.memory_recital_card(passage_id, &p, &plan, &indexes, MemoryPurpose::Run)
            .map(Some)
    }

    /// The passage holding a verse: `passage_id`'s, or the first live one.
    fn memory_passage_for(
        &self,
        passage_id: &str,
        book: u8,
        chapter: u8,
        verse: u8,
    ) -> rusqlite::Result<Option<MemoryPassage>> {
        let r = pack_ref(book, chapter, verse);
        Ok(self.memory_scope(passage_id)?.into_iter().find(|p| {
            p.book == book
                && (pack_ref(book, p.start_chapter, p.start_verse)
                    ..=pack_ref(book, p.end_chapter, p.end_verse))
                    .contains(&r)
        }))
    }

    #[allow(clippy::too_many_arguments)]
    fn write_memory_verse(
        &self,
        book: u8,
        chapter: u8,
        verse: u8,
        s: VerseSrs,
        due: i64,
        grade: Grade,
        now: i64,
    ) -> rusqlite::Result<()> {
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
                s.stage,
                s.ease,
                s.interval_days,
                due,
                s.reps,
                s.lapses,
                now,
                grade as i64,
            ],
        )?;
        Ok(())
    }

    /// Record how a card went and move the schedule on.
    ///
    /// For a learning card, `target` is the verse whose script it was step
    /// `step` of, and `grade` the learner's grade for it: reading moves on;
    /// a recital that went well moves on, Hard repeats it, and a forgotten one
    /// steps back to the step before. A stale step (a card answered twice) is
    /// ignored. `verses` grades each verse of a recited card: for a chain or a
    /// review they reschedule every learnt verse recited (reviewing ahead
    /// leaves a verse's spacing alone), and a forgotten one goes back to
    /// learning. A run also respaces the next passage-so-far run.
    #[allow(clippy::too_many_arguments)]
    pub fn submit_memory_recital(
        &self,
        passage_id: &str,
        book: u8,
        purpose: MemoryPurpose,
        target: Option<(u8, u8)>,
        step: usize,
        grade: Grade,
        verses: &[MemoryVerseGrade],
        now: i64,
        utc_offset: i64,
    ) -> rusqlite::Result<MemoryReviewOutcome> {
        let today = local_day(now, utc_offset);
        let settings = self.memory_settings()?;
        let total_before = self.memory_total_xp()?;
        let today_before = self.memory_day_xp(today)?;
        let completed_before = self.completed_passage_ids(now)?;
        let mut out = MemoryReviewOutcome::default();
        let mut graduated = false;
        let mut xp = 0;
        // (chapter, verse, grade code, graduated) rows for the answer log.
        let mut log: Vec<(u8, u8, i64, bool)> = Vec::new();
        let mut run_passage: Option<MemoryPassage> = None;

        // Reschedule the learnt verses a recital covered.
        let review = |this: &Self, v: &MemoryVerseGrade, log: &mut Vec<_>| {
            let Some((s, due)) = this.memory_verse_srs(book, v.chapter, v.verse)? else {
                return Ok::<_, rusqlite::Error>(None);
            };
            if !s.learnt() {
                return Ok(None);
            }
            let (after, next) = s.reviewed(v.grade, now, due, due > now);
            this.write_memory_verse(book, v.chapter, v.verse, after, next, v.grade, now)?;
            log.push((v.chapter, v.verse, v.grade as i64, false));
            Ok(Some(after))
        };

        if purpose.is_learning() {
            let Some((chapter, verse)) = target else {
                return Ok(out);
            };
            let Some(p) = self.memory_passage_for(passage_id, book, chapter, verse)? else {
                return Ok(out);
            };
            let plan = self.memory_plan(&p)?;
            let Some(i) = plan
                .verses
                .iter()
                .position(|v| (v.chapter, v.verse) == (chapter, verse))
            else {
                return Ok(out);
            };
            // The target verse moves on by how it went itself; in a chain the
            // verses before it are graded on their own below.
            let grade = verses
                .iter()
                .find(|v| (v.chapter, v.verse) == (chapter, verse))
                .map_or(grade, |v| v.grade);
            let steps = plan.steps(i);
            let lines = plan.verses[i].lines.len();
            let previous = self.memory_verse_srs(book, chapter, verse)?;
            let (before, _) = previous.unwrap_or((VerseSrs::default(), now));
            let current = if previous.is_some() {
                resolve_step(before.stage, &steps, lines)
            } else {
                0
            };
            if before.learnt() || current != step {
                return Ok(out);
            }
            let next = match purpose {
                MemoryPurpose::Read => step + 1,
                _ => match grade {
                    Grade::Again if step > 0 => step - 1,
                    Grade::Again | Grade::Hard => step,
                    Grade::Good | Grade::Easy => step + 1,
                },
            };
            let (after, due) = if next >= steps.len() {
                graduated = true;
                before.graduate(grade, now)
            } else {
                (
                    VerseSrs {
                        stage: next as u8,
                        ..before
                    },
                    now,
                )
            };
            self.write_memory_verse(book, chapter, verse, after, due, grade, now)?;
            out.interval_days = after.interval_days;
            if graduated {
                let ever: bool = self.conn().query_row(
                    "SELECT EXISTS(SELECT 1 FROM progress.memory_review
                     WHERE book = ?1 AND chapter = ?2 AND verse = ?3 AND graduated = 1)",
                    params![book, chapter, verse],
                    |r| r.get(0),
                )?;
                out.first_graduation = !ever;
            }
            let reading = !purpose.hidden();
            xp += if reading {
                READ_XP
            } else {
                recital_xp(grade, false)
            };
            log.push((
                chapter,
                verse,
                if reading { -2 } else { grade as i64 },
                graduated,
            ));
            if purpose == MemoryPurpose::Chain {
                for v in verses
                    .iter()
                    .filter(|v| (v.chapter, v.verse) != (chapter, verse))
                {
                    if let Some(after) = review(self, v, &mut log)? {
                        out.relearn += i64::from(!after.learnt());
                        xp += recital_xp(v.grade, false);
                    }
                }
            }
            if graduated {
                let (_, first, end) = plan.section_of(i);
                let learnt: Vec<usize> = self
                    .memory_learnt(&p, &plan)?
                    .iter()
                    .map(|(j, _, _)| *j)
                    .collect();
                out.section_completed = (first..end).all(|j| learnt.contains(&j));
                if out.section_completed && learnt.iter().any(|&j| j < first) {
                    // A section just joined the ones before it: recite the
                    // whole passage so far, now.
                    let interval = self
                        .memory_run_state(&p.id)?
                        .map_or(RUN_MIN_DAYS, |(_, d)| d);
                    self.set_memory_run_state(&p.id, now, interval, 0, now)?;
                }
            }
        } else {
            for v in verses {
                if let Some(after) = review(self, v, &mut log)? {
                    out.relearn += i64::from(!after.learnt());
                    xp += recital_xp(v.grade, true);
                    if after.learnt()
                        && (out.interval_days == 0 || after.interval_days < out.interval_days)
                    {
                        out.interval_days = after.interval_days;
                    }
                }
            }
            if purpose == MemoryPurpose::Run {
                run_passage = self.memory_passage(passage_id)?;
                if run_passage.is_none()
                    && let Some(v) = verses.first()
                {
                    run_passage = self.memory_passage_for("", book, v.chapter, v.verse)?;
                }
            }
        }

        if let Some(p) = run_passage {
            let previous = self
                .memory_run_state(&p.id)?
                .map_or(RUN_MIN_DAYS, |(_, d)| d);
            let interval = match grade {
                Grade::Again => 1,
                Grade::Hard => previous.max(RUN_MIN_DAYS),
                Grade::Good => (previous * 2).clamp(RUN_MIN_DAYS, RUN_MAX_DAYS),
                Grade::Easy => (previous * 3).clamp(RUN_MIN_DAYS, RUN_MAX_DAYS),
            };
            self.set_memory_run_state(&p.id, now + interval * SECONDS_PER_DAY, interval, now, now)?;
            out.interval_days = interval;
        }

        let completed_passages: Vec<String> = if graduated {
            let ever_completed = self.ever_completed_passage_ids()?;
            self.completed_passage_ids(now)?
                .into_iter()
                .filter(|id| !completed_before.contains(id) && !ever_completed.contains(id))
                .collect()
        } else {
            Vec::new()
        };
        xp += if out.first_graduation {
            GRADUATION_BONUS_XP
        } else {
            0
        } + PASSAGE_BONUS_XP * completed_passages.len() as i64;

        // One log row per verse; the card's XP rides on the first. A
        // completed passage is remembered in its row, so completing it again
        // after a lapse does not pay the bonus twice.
        for (n, (chapter, verse, grade_code, graduated)) in log.iter().enumerate() {
            let tag = if n == 0 {
                completed_passages
                    .first()
                    .map(|id| format!("complete:{id}"))
                    .unwrap_or_else(|| passage_id.to_string())
            } else {
                passage_id.to_string()
            };
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
                    purpose.code(),
                    grade_code,
                    if n == 0 { xp } else { 0 },
                    graduated,
                    tag
                ],
            )?;
        }
        if let Some((chapter, verse, _, _)) = log.first() {
            for id in completed_passages.iter().skip(1) {
                // Several passages completed at once (overlapping ranges): a
                // zero-XP marker row each.
                self.conn().execute(
                    "INSERT INTO progress.memory_review(
                         epoch, day, book, chapter, verse, stage, grade, xp, graduated,
                         passage_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, -1, -1, 0, 0, ?6)",
                    params![now, today, book, chapter, verse, format!("complete:{id}")],
                )?;
            }
        }

        let total_xp = total_before + xp;
        let today_xp = today_before + xp;
        out.xp = xp;
        out.completed_passages = completed_passages;
        out.total_xp = total_xp;
        out.level_before = level_for_xp(total_before);
        out.level_after = level_for_xp(total_xp);
        out.today_xp = today_xp;
        out.daily_goal_xp = settings.daily_goal_xp;
        out.goal_reached_now =
            today_before < settings.daily_goal_xp && today_xp >= settings.daily_goal_xp;
        out.streak_days = self.memory_streak(today)?;
        Ok(out)
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
                   ON p.deleted = 0 AND v.book = p.book AND {IN_PASSAGE}
                 WHERE v.interval_days >= 1"
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
    fn a_verse_is_read_recalled_and_chained_line_by_line() {
        use Step::*;
        assert_eq!(
            verse_steps(2, true),
            vec![Read(0), Recall(0, 0), Read(1), Recall(1, 1), Recall(0, 1)]
        );
        assert_eq!(verse_steps(1, false), vec![Read(0), Recall(0, 0), Chain]);
        // Each line on its own and then with the lines before it (the last
        // of these is the whole verse), then the section from its start
        // through this verse.
        let three = verse_steps(3, false);
        assert_eq!(
            three,
            vec![
                Read(0),
                Recall(0, 0),
                Read(1),
                Recall(1, 1),
                Recall(0, 1),
                Read(2),
                Recall(2, 2),
                Recall(0, 2),
                Chain
            ]
        );
        assert_eq!(
            relearn_step(&three, 3),
            7,
            "relearning starts at the whole verse"
        );
    }

    #[test]
    fn early_recital_success_keeps_the_schedule() {
        let now = 1_000_000;
        let learnt = VerseSrs {
            interval_days: 10,
            ..Default::default()
        };
        let due = now + 5 * SECONDS_PER_DAY;
        assert_eq!(learnt.reviewed(Grade::Good, now, due, true), (learnt, due));
        let (hard, hard_due) = learnt.reviewed(Grade::Hard, now, due, true);
        assert_eq!(hard.interval_days, 10);
        assert_eq!(hard_due, now + SECONDS_PER_DAY);
        let (again, _) = learnt.reviewed(Grade::Again, now, due, true);
        assert_eq!((again.stage, again.interval_days), (RELEARN_STAGE, 0));
        let (good, _) = learnt.reviewed(Grade::Good, now, now, false);
        assert_eq!(good.interval_days, 25);
    }

    #[test]
    fn words_keep_maqaf_groups_and_drop_cantillation() {
        let words = memory_words("וַֽיְהִי־ עֶ֥רֶב וַֽיְהִי־ בֹ֖קֶר אֱלֹהִ֤ים ׀ לָאוֹר֙ הָאָֽרֶץ׃");
        assert_eq!(
            words,
            vec!["וַיְהִי־עֶרֶב", "וַיְהִי־בֹקֶר", "אֱלֹהִים ׀", "לָאוֹר", "הָאָרֶץ׃"]
        );
        let glossed = verse_units("וַֽיְהִי־ עֶ֥רֶב", &["and was".into(), "evening".into()]);
        assert_eq!(glossed.len(), 1);
        assert_eq!(glossed[0].gloss, "and was evening");
    }

    #[test]
    fn stored_line_starts_are_clipped_to_the_verse() {
        assert_eq!(parse_line_starts("9, 3,0,3,40", 10), vec![3, 9]);
        assert!(parse_line_starts("", 10).is_empty());
    }

    #[test]
    fn a_verse_caught_mid_steps_restarts_when_the_steps_change() -> rusqlite::Result<()> {
        let db = Connection::open_in_memory()?;
        db.execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        crate::tutor::init_progress_schema(&db)?;
        // As left by version 1 of the steps: one verse mid-way, one waiting
        // to be relearnt, one learnt.
        db.execute_batch(
            "INSERT INTO progress.memory_verse(book, chapter, verse, stage, ease,
                 interval_days, due_epoch, reps, lapses, introduced_epoch,
                 last_review_epoch, last_grade, updated_epoch)
             VALUES (1, 1, 1, 3, 2.5, 0, 0, 0, 0, 0, 0, 2, 0),
                    (1, 1, 2, 255, 2.5, 0, 0, 1, 1, 0, 0, 0, 0),
                    (1, 1, 3, 5, 2.5, 4, 0, 2, 0, 0, 0, 2, 0);
             UPDATE progress.meta SET value = '1' WHERE key = 'memory.steps';",
        )?;
        init_memory_schema(&db)?;
        let stages: Vec<i64> = db
            .prepare("SELECT stage FROM progress.memory_verse ORDER BY verse")?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        assert_eq!(stages, vec![0, 255, 5]);
        Ok(())
    }

    #[test]
    fn levels_grow_by_a_widening_step() {
        assert_eq!(level_for_xp(0), 1);
        assert_eq!(level_for_xp(99), 1);
        assert_eq!(level_for_xp(100), 2);
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

    /// Answer a card with `grade` for every verse on it (`again` verses
    /// forgotten), returning the outcome.
    fn answer(
        bible: &Bible,
        card: &MemoryCard,
        grade: Grade,
        again: &[u8],
        now: i64,
    ) -> rusqlite::Result<MemoryReviewOutcome> {
        let mut verses: Vec<MemoryVerseGrade> = Vec::new();
        for s in &card.segments {
            if verses
                .last()
                .is_some_and(|v| (v.chapter, v.verse) == (s.chapter, s.verse))
            {
                continue;
            }
            verses.push(MemoryVerseGrade {
                chapter: s.chapter,
                verse: s.verse,
                grade: if again.contains(&s.verse) {
                    Grade::Again
                } else {
                    grade
                },
            });
        }
        bible.submit_memory_recital(
            &card.passage_id,
            card.book,
            card.purpose,
            (card.target_verse > 0).then_some((card.target_chapter, card.target_verse)),
            card.step,
            grade,
            &verses,
            now,
            0,
        )
    }

    fn card(item: MemoryItem) -> MemoryCard {
        match item {
            MemoryItem::Card(c) => c,
            other => panic!("expected a card, got {other:?}"),
        }
    }

    fn verses_of(card: &MemoryCard) -> Vec<u8> {
        let mut v: Vec<u8> = card.segments.iter().map(|s| s.verse).collect();
        v.dedup();
        v
    }

    /// Shape every verse of a passage as a learner might — a longer verse
    /// split in two, a short one kept whole — with sections starting at
    /// `section_starts` (verse numbers).
    fn shape_all(
        bible: &Bible,
        p: &MemoryPassage,
        section_starts: &[u8],
        now: i64,
    ) -> rusqlite::Result<()> {
        for v in bible.memory_layout(&p.id)? {
            let n = v.words.len();
            let starts: Vec<usize> = if n > 6 { vec![n / 2] } else { Vec::new() };
            bible.set_memory_line_starts(p.book, v.chapter, v.verse, Some(&starts), now)?;
            bible.set_memory_section_start(
                p.book,
                v.chapter,
                v.verse,
                section_starts.contains(&v.verse),
                now,
            )?;
        }
        Ok(())
    }

    #[test]
    fn a_psalm_is_learnt_in_order_and_chained_within_its_section() -> rusqlite::Result<()> {
        let Some(bible) = test_bible() else {
            return Ok(());
        };
        let mut now = 1_700_000_000;
        // Psalm 23 (book 27 in Tanakh order): six verses, sections 1–4, 5–6.
        let p = bible
            .add_memory_passage(27, 23, 0, 23, 255, "", now)?
            .expect("Psalm 23 exists");
        assert_eq!((p.start_verse, p.end_verse), (1, 6));
        bible.set_memory_settings(
            MemorySettings {
                new_per_day: 2,
                daily_goal_xp: 50,
            },
            now,
        )?;
        // Nothing is shaped for the learner, so nothing can be learnt yet —
        // not even a verse asked for past the ration.
        let summary = &bible.memory_passages(now)?[0];
        assert!(summary.needs_shaping);
        assert!(bible.memory_layout(&p.id)?.iter().all(|v| !v.shaped));
        for extra_new in [false, true] {
            assert_eq!(
                bible.next_memory_item(&p.id, extra_new, now, 0)?,
                MemoryItem::Done {
                    next_due_epoch: 0,
                    can_learn_more: false,
                    shape_passage_id: p.id.clone(),
                }
            );
        }
        // Shaping the first section, 1–4, and marking where the next begins
        // is enough to start; verses 5–6 stay unshaped.
        let unshaped = bible.memory_layout(&p.id)?;
        for v in &unshaped[..4] {
            let n = v.words.len();
            let starts: Vec<usize> = if n > 6 { vec![n / 2] } else { Vec::new() };
            bible.set_memory_line_starts(27, 23, v.verse, Some(&starts), now)?;
        }
        bible.set_memory_section_start(27, 23, 5, true, now)?;
        let layout = bible.memory_layout(&p.id)?;
        let ready: Vec<bool> = layout.iter().map(|v| v.ready).collect();
        assert_eq!(ready, vec![true, true, true, true, false, false]);
        assert!(layout[0].glosses.iter().any(|g| !g.is_empty()));
        assert_eq!(layout[0].glosses.len(), layout[0].words.len());
        let summary = &bible.memory_passages(now)?[0];
        assert!(!summary.needs_shaping);
        let starts: Vec<u8> = summary
            .verses
            .iter()
            .filter(|v| v.section_start)
            .map(|v| v.verse)
            .collect();
        assert_eq!(starts, vec![1, 5]);

        // Walk with Good until today's two verses are learnt.
        let mut seen = Vec::new();
        let mut last_outcome = None;
        loop {
            now += 30;
            let c = match bible.next_memory_item(&p.id, false, now, 0)? {
                MemoryItem::Card(c) => c,
                MemoryItem::Done { can_learn_more, .. } => {
                    assert!(can_learn_more);
                    break;
                }
                MemoryItem::Empty => panic!("passage has verses"),
            };
            assert!(seen.len() < 60, "runaway: {seen:?}");
            if c.purpose.hidden() {
                assert!(c.segments.iter().all(|s| !s.words.is_empty()));
            }
            if c.purpose == MemoryPurpose::Read {
                assert!(
                    c.segments
                        .iter()
                        .flat_map(|s| &s.words)
                        .all(|w| !w.translit.is_empty())
                );
            }
            seen.push((c.purpose, c.target_verse, verses_of(&c)));
            last_outcome = Some(answer(&bible, &c, Grade::Good, &[], now)?);
        }
        // Verse 1 is finished before verse 2 begins, and verse 2 ends by
        // reciting verses 1–2 together.
        // Bottom up: the very first card is the first line of verse 1.
        assert_eq!(seen[0], (MemoryPurpose::Read, 1, vec![1]));
        let first_v2 = seen
            .iter()
            .position(|s| s.1 == 2)
            .expect("verse 2 was learnt");
        assert!(seen[first_v2..].iter().all(|s| s.1 == 2));
        assert_eq!(seen.last(), Some(&(MemoryPurpose::Chain, 2, vec![1, 2])));
        assert!(seen.iter().all(|s| s.0 != MemoryPurpose::Review));
        let outcome = last_outcome.expect("answers given");
        assert!(outcome.first_graduation);
        assert_eq!(outcome.streak_days, 1);

        // A verse more today, on request: it chains 1–3.
        let c = card(bible.next_memory_item(&p.id, true, now, 0)?);
        assert_eq!(
            (c.purpose, c.target_verse, c.is_new),
            (MemoryPurpose::Read, 3, true)
        );

        let stats = bible.memory_stats(now, 0, 7, 7)?;
        assert_eq!(stats.verses_learnt, 2);
        assert_eq!(stats.forecast[1], 2, "both come back tomorrow");
        assert!(
            stats
                .achievements
                .iter()
                .any(|a| a.key == "verse_1" && a.earned)
        );

        // Tomorrow the section is reviewed whole. Forgetting verse 2 sends
        // it back to learning — from recalling it whole, then the chain.
        now += SECONDS_PER_DAY + 60;
        let review = card(bible.next_memory_item(&p.id, false, now, 0)?);
        assert_eq!(review.purpose, MemoryPurpose::Review);
        assert_eq!(verses_of(&review), vec![1, 2]);
        assert_eq!(review.cue, "");
        let outcome = answer(&bible, &review, Grade::Good, &[2], now)?;
        assert_eq!(outcome.relearn, 1);
        now += 30;
        let relearn = card(bible.next_memory_item(&p.id, false, now, 0)?);
        assert_eq!(
            (relearn.purpose, relearn.target_verse),
            (MemoryPurpose::Recall, 2)
        );
        let lines = relearn.segments[0].line_count;
        assert_eq!(relearn.segments.len(), lines, "the whole verse");
        assert!(relearn.cue.starts_with('…'), "cued by the end of verse 1");
        answer(&bible, &relearn, Grade::Good, &[], now)?;
        now += 30;
        let chain = card(bible.next_memory_item(&p.id, false, now, 0)?);
        assert_eq!(
            (chain.purpose, verses_of(&chain)),
            (MemoryPurpose::Chain, vec![1, 2])
        );
        Ok(())
    }

    #[test]
    fn lines_then_the_verse_then_the_section_then_the_passage_so_far() -> rusqlite::Result<()> {
        let Some(bible) = test_bible() else {
            return Ok(());
        };
        let mut now = 1_700_000_000;
        let p = bible
            .add_memory_passage(27, 23, 1, 23, 6, "", now)?
            .expect("Psalm 23 exists");
        // Sections 1–2, 3–4, 5–6; verse 1 in three lines (of two, two and
        // the rest of its words), verse 2 in two.
        shape_all(&bible, &p, &[3, 5], now)?;
        bible.set_memory_line_starts(27, 23, 1, Some(&[2, 4]), now)?;
        bible.set_memory_line_starts(27, 23, 2, Some(&[3]), now)?;
        let lines: Vec<usize> = bible
            .memory_layout(&p.id)?
            .iter()
            .map(|v| v.line_starts.len() + 1)
            .collect();
        assert_eq!(&lines[..2], &[3, 2]);

        // What each card recites: (purpose, [(verse, line)]).
        type Seen = (MemoryPurpose, Vec<(u8, usize)>);
        let mut seen: Vec<Seen> = Vec::new();
        loop {
            now += 30;
            let c = card(bible.next_memory_item(&p.id, true, now, 0)?);
            if c.target_verse == 5 {
                break;
            }
            // A later line of a verse is cued by every line before it.
            if c.target_verse == 1 && c.segments[0].line == 2 {
                let cue: Vec<usize> = c.cue.split('\n').map(|l| l.split(' ').count()).collect();
                assert_eq!(cue, vec![2, 2], "lines 1 and 2 of 23:1: {:?}", c.cue);
            }
            assert!(seen.len() < 80, "runaway");
            seen.push((
                c.purpose,
                c.segments.iter().map(|s| (s.verse, s.line)).collect(),
            ));
            answer(&bible, &c, Grade::Good, &[], now)?;
        }

        let whole = |v: u8| -> Vec<(u8, usize)> {
            (0..lines[usize::from(v) - 1]).map(|l| (v, l)).collect()
        };
        let verse = |v: u8, first_in_section: bool| {
            let mut out: Vec<Seen> = Vec::new();
            for l in 0..lines[usize::from(v) - 1] {
                out.push((MemoryPurpose::Read, vec![(v, l)]));
                out.push((MemoryPurpose::Recall, vec![(v, l)]));
                if l > 0 {
                    out.push((MemoryPurpose::Recall, (0..=l).map(|k| (v, k)).collect()));
                }
            }
            if !first_in_section {
                let start = if v > 2 { 3 } else { 1 };
                out.push((MemoryPurpose::Chain, (start..=v).flat_map(&whole).collect()));
            }
            out
        };
        let mut expected = Vec::new();
        expected.extend(verse(1, true));
        expected.extend(verse(2, false));
        // The first section needs no run of its own: its chain was one.
        expected.extend(verse(3, true));
        expected.extend(verse(4, false));
        // The second section joins the first: all of it, before verse 5.
        expected.push((MemoryPurpose::Run, (1..=4).flat_map(&whole).collect()));
        assert_eq!(seen, expected);
        Ok(())
    }

    #[test]
    fn a_completed_section_brings_the_passage_so_far() -> rusqlite::Result<()> {
        let Some(bible) = test_bible() else {
            return Ok(());
        };
        let mut now = 1_700_000_000;
        let p = bible
            .add_memory_passage(27, 23, 1, 23, 6, "", now)?
            .expect("Psalm 23 exists");
        // Two sections of three verses.
        shape_all(&bible, &p, &[4], now)?;
        let layout = bible.memory_layout(&p.id)?;
        assert_eq!(
            layout
                .iter()
                .filter(|v| v.section_start)
                .map(|v| v.verse)
                .collect::<Vec<_>>(),
            vec![1, 4]
        );
        let mut runs = Vec::new();
        for _ in 0..400 {
            now += 30;
            match bible.next_memory_item(&p.id, true, now, 0)? {
                MemoryItem::Card(c) => {
                    if c.purpose == MemoryPurpose::Run {
                        runs.push(verses_of(&c));
                    }
                    answer(&bible, &c, Grade::Good, &[], now)?;
                }
                MemoryItem::Done { .. } => break,
                MemoryItem::Empty => panic!("passage has verses"),
            }
            if !runs.is_empty() {
                break;
            }
        }
        assert_eq!(runs, vec![vec![1, 2, 3, 4, 5, 6]]);
        let stats = bible.memory_stats(now, 0, 7, 7)?;
        assert_eq!(stats.passages_completed, 1);
        assert!(
            stats
                .achievements
                .iter()
                .any(|a| a.key == "passage_1" && a.earned)
        );
        // The run is spaced out once recited.
        let next = bible.next_memory_item(&p.id, false, now + 60, 0)?;
        assert!(matches!(next, MemoryItem::Done { .. }), "{next:?}");
        Ok(())
    }

    #[test]
    fn a_section_is_learnt_only_once_the_learner_has_shaped_it() -> rusqlite::Result<()> {
        let Some(bible) = test_bible() else {
            return Ok(());
        };
        let mut now = 1_700_000_000;
        let p = bible
            .add_memory_passage(1, 1, 1, 1, 5, "", now)?
            .expect("Genesis 1");
        let blocked = |bible: &Bible, now| -> rusqlite::Result<bool> {
            Ok(matches!(
                bible.next_memory_item(&p.id, true, now, 0)?,
                MemoryItem::Done { shape_passage_id, .. } if shape_passage_id == p.id
            ))
        };
        // No default split: every verse starts as one unshaped line, and the
        // passage as one section.
        let before = bible.memory_layout(&p.id)?;
        assert_eq!(before[0].words.len(), 7);
        assert!(before[0].line_starts.is_empty());
        assert!(before.iter().all(|v| !v.shaped && !v.ready));
        assert_eq!(before.iter().filter(|v| v.section_start).count(), 1);
        assert!(blocked(&bible, now)?);

        // Shaping one verse is not enough while the section runs on to the
        // end of the passage; closing it at 1:3 still leaves 1:2 unshaped.
        bible.set_memory_line_starts(1, 1, 1, Some(&[2, 5]), now)?;
        bible.set_memory_section_start(1, 1, 3, true, now)?;
        assert!(blocked(&bible, now)?);
        // Keeping 1:2 whole is a decision too, and completes the section.
        bible.set_memory_line_starts(1, 1, 2, Some(&[]), now)?;
        let shaped = bible.memory_layout(&p.id)?;
        assert_eq!(shaped[0].line_starts, vec![2, 5]);
        assert!(shaped[0].shaped && shaped[1].shaped && !shaped[2].shaped);
        let ready: Vec<bool> = shaped.iter().map(|v| v.ready).collect();
        assert_eq!(ready, vec![true, true, false, false, false]);
        // The shape drives the steps: three lines to read and recall.
        let c = card(bible.next_memory_item(&p.id, false, now, 0)?);
        assert_eq!(c.purpose, MemoryPurpose::Read);
        assert_eq!(c.section_count, 2, "the section is verses 1–2");
        assert_eq!(c.title, "1:1 · line 1 of 3");
        assert_eq!(c.segments[0].words.len(), 2);

        // A verse already begun carries on (as one line) if its shape is
        // cleared; it is only starting a verse that waits on its shape.
        answer(&bible, &c, Grade::Good, &[], now)?;
        bible.set_memory_line_starts(1, 1, 1, None, now + 1)?;
        now += 30;
        let c = card(bible.next_memory_item(&p.id, false, now, 0)?);
        assert_eq!((c.target_verse, c.segments[0].line_count), (1, 1));
        // Its section is unshaped again, though: put the shape back, or 1:2
        // could not begin.
        bible.set_memory_line_starts(1, 1, 1, Some(&[3]), now)?;

        // Learning both verses of the section stops at 1:3, unshaped.
        for _ in 0..40 {
            now += 30;
            match bible.next_memory_item(&p.id, true, now, 0)? {
                MemoryItem::Card(c) => {
                    assert!(c.target_verse <= 2, "{c:?}");
                    answer(&bible, &c, Grade::Good, &[], now)?;
                }
                _ => break,
            }
        }
        assert!(blocked(&bible, now)?);
        assert!(bible.memory_passages(now)?[0].needs_shaping);
        assert_eq!(bible.memory_passages(now)?[0].learnt, 2);
        Ok(())
    }

    #[test]
    fn completing_a_passage_pays_its_bonus_once() -> rusqlite::Result<()> {
        let Some(bible) = test_bible() else {
            return Ok(());
        };
        let mut now = 1_700_000_000;
        let p = bible
            .add_memory_passage(1, 1, 1, 1, 1, "", now)?
            .expect("Genesis 1:1");
        shape_all(&bible, &p, &[], now)?;
        let mut completions = 0;
        for _ in 0..20 {
            now += 30;
            let MemoryItem::Card(c) = bible.next_memory_item(&p.id, false, now, 0)? else {
                break;
            };
            let outcome = answer(&bible, &c, Grade::Good, &[], now)?;
            completions += outcome.completed_passages.len();
            if outcome.first_graduation {
                assert!(outcome.xp >= PASSAGE_BONUS_XP + GRADUATION_BONUS_XP);
            }
        }
        assert_eq!(completions, 1);
        // A stale answer (the same step twice) changes nothing.
        now += 30 * SECONDS_PER_DAY;
        let review = card(bible.next_memory_item(&p.id, false, now, 0)?);
        answer(&bible, &review, Grade::Good, &[1], now)?;
        for _ in 0..6 {
            now += 30;
            let MemoryItem::Card(c) = bible.next_memory_item(&p.id, false, now, 0)? else {
                break;
            };
            let outcome = answer(&bible, &c, Grade::Good, &[], now)?;
            assert!(outcome.completed_passages.is_empty());
            assert!(!outcome.first_graduation);
            let again = answer(&bible, &c, Grade::Good, &[], now)?;
            assert_eq!(again.xp, 0, "a stale step is ignored");
        }
        // Deleting keeps the verse's progress for when it is added back.
        assert!(bible.delete_memory_passage(&p.id, now)?);
        assert!(bible.memory_passages(now)?.is_empty());
        let back = bible
            .add_memory_passage(1, 1, 1, 1, 1, "", now)?
            .expect("exists");
        assert_ne!(back.id, p.id);
        assert_eq!(bible.memory_passages(now)?[0].learnt, 1);
        Ok(())
    }
}
