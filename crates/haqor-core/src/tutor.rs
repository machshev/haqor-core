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
//!    `letter_identity`); a vowel-less vav with a dagesh is the shureq vowel
//!    (וּ → "u"), likewise taught as its own glyph; a dagesh elsewhere is pure
//!    gemination and isn't taught. The five final forms (ך ם ן ף ץ) are the same
//!    *sound* as their medial base but a distinct *shape* the reader must
//!    recognise, so each is drilled as its own glyph (see `decompose_glyphs`);
//!    they don't count toward the alphabet gate (see `Bible::all_letters_known`).
//!    The first final form a learner meets is gated behind a one-time gradeless
//!    card explaining the concept (see [`crate::tutor::StudyItem::ExplainFinalForms`]).
//! 2. **Word meaning** — once all a word's glyphs are known (so the learner can
//!    already sound it out), drill what the word means.
//!
//! Verse-punctuating reading marks (sof pasuq, maqaf) carry no sound of their
//! own, so they are shown once with an explanation the first time a verse
//! needs them and never drilled with spaced repetition (see
//! [`crate::tutor::StudyItem::ExplainMark`]).
//!
//! Reviews are scheduled with a compact SM-2 with short in-session learning
//! steps (so recall actually happens within a sitting, not only the next day),
//! persisted in a writable `progress.db` (attached by
//! [`crate::bible::Bible::attach_progress`]). Static selection runs over
//! `hebrew.db`'s `verse_word` / `verse_stats` tables.

use log::debug;
use rusqlite::{Connection, OptionalExtension, params};

use crate::bible::{Bible, HebrewWord};

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

/// `progress.concepts_seen` key for the one-time final-forms explanation card
/// (see [`StudyItem::ExplainFinalForms`]). Kept out of [`crate::grammar`]'s
/// concept list — it's a script concept, not a grammar rule, so it must not
/// join the grammar unlock ordering.
const FINAL_FORMS_CONCEPT: &str = "final_forms";

/// The one-time language-intro deck, shown in this order before anything else
/// is taught: how Hebrew is read (right to left), what the alphabet is (22
/// letters, all consonants, no capitals), and what the vowel points are.
/// Script concepts like [`FINAL_FORMS_CONCEPT`] — recorded once in
/// `progress.concepts_seen`, kept out of the grammar unlock ordering; the
/// teaching content is presentation and lives in the app, keyed by these keys
/// (see [`StudyItem::ExplainIntro`]).
const INTRO_CONCEPTS: [&str; 3] = ["intro_rtl", "intro_alphabet", "intro_vowels"];

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
    /// A word's grammatical *form* — a "which form is this?" drill, tracked
    /// separately from its meaning ([`Track::Word`]) in `form_srs`.
    Form,
    /// A pronominal ending (־ִי "me", ־וֹ "him") drilled highlighted on a
    /// rotating known host word, tracked per person-gender-number key in
    /// `suffix_srs` (see [`crate::pronoun_suffix`]).
    Suffix,
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

/// A teachable glyph: a single consonant (final forms taught as their own
/// distinct glyph) or a niqqud point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GlyphCard {
    pub glyph: String,
    /// True for a base consonant, false for a vowel/dagesh/sin-shin point.
    pub is_consonant: bool,
    /// For a vowel point, an already-learnt consonant to display it on (so it is
    /// taught as a sounded-out syllable). None for consonants and reading marks.
    pub host: Option<String>,
    /// The voiced reading of the taught syllable (`host` + `glyph`, e.g. "bah")
    /// when the card has a host; empty for consonants and reading marks, which
    /// quiz by name (see [`crate::romanize::voiced_syllable`]).
    pub voiced: String,
    /// Other already-introduced glyphs of the same kind, offered as wrong
    /// answers when this card is quizzed multiple-choice. Empty on `New*` cards
    /// and whenever too few peers exist for a quiz (the app then self-grades).
    pub distractors: Vec<String>,
    /// Aligned with `distractors` on a vowel card: each syllable's voiced
    /// reading ("re", "bᵉ"); empty for consonants and reading marks.
    pub voiced_distractors: Vec<String>,
}

/// A word to learn or review. Words teach only meaning — by the time a word card
/// appears, all its glyphs are known so the learner can already sound it out.
#[derive(Debug, Clone)]
pub struct WordCard {
    pub surface_id: i64,
    pub surface: String,
    pub occurrences: i64,
    /// The surface's voiced reading ("bereshit"), shown under the Hebrew so
    /// the learner can check how they sounded it out
    /// (see [`crate::romanize::romanize`]).
    pub translit: String,
    /// The learner meaning of *this surface* — what the meaning quiz tests.
    /// The specific inflected form rendered in English where the parse
    /// supports one ("and to the house"; see `crate::bible::inflected_gloss`),
    /// the lexeme's base sense otherwise.
    pub gloss: String,
    /// The lexeme's base sense (BDB gloss, "house") when it differs from the
    /// form-specific `gloss` — a secondary "root meaning" line on the answer
    /// side. Empty when `gloss` is already the base sense, or for a curated
    /// word / function word / proper noun.
    pub root_gloss: String,
    /// Optional composition/teaching note for a curated word ("לְ (to) + ־וֹ
    /// (him)"), empty otherwise. See [`crate::vocab_gloss`].
    pub note: String,
    pub root: String,
    pub morph: String,
    /// Other plausible glosses offered as wrong answers when the meaning is
    /// quizzed multiple-choice. Empty when too few exist for a quiz (the app then
    /// self-grades).
    pub distractors: Vec<String>,
}

/// A learner-facing gloss correction made from the app's tutor admin mode.
/// These live in the writable progress database so they work offline and are
/// carried by the ordinary progress snapshot sync. `haqor-admin pull` promotes
/// reviewed corrections into the checked-in lexical overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossOverride {
    pub surface: String,
    pub gloss: String,
    pub note: String,
    pub updated_epoch: i64,
}

/// A mobile correction for the root/header gloss shown by word information.
/// Like tutor gloss corrections, these remain in the writable progress DB
/// until `haqor-admin pull` promotes them into `lexicon_entries`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexiconEntryOverride {
    pub surface: String,
    pub root: String,
    pub gloss: String,
    pub updated_epoch: i64,
}

/// A bug report or idea captured from an admin-only app control. Reports live
/// in the writable progress database so they can be recorded offline, carried
/// by normal progress sync, and downloaded later with `haqor-admin`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueReport {
    pub id: String,
    pub report_type: String,
    pub note: String,
    pub context_json: String,
    pub created_epoch: i64,
    pub updated_epoch: i64,
}

/// Counts for learner gloss corrections that are still active on this device.
/// `redundant` entries produce exactly the same learner-facing card as the
/// current bundled lexical data and can therefore be pruned safely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlossOverrideStats {
    pub total: i64,
    pub redundant: i64,
}

/// Result of pruning redundant learner gloss corrections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlossOverrideOptimization {
    pub removed: i64,
    pub stats: GlossOverrideStats,
}

/// A pronominal-ending drill: the ending shown on a host word the learner
/// already knows, with the ending's span highlighted (the app renders `stem`
/// plain and `suffix` in red, the way a new vowel is taught on its host
/// consonant). The quiz asks which pronoun the ending stands for; reviews
/// rotate the host across known suffixed words so the ending — not one
/// word's shape — is what's being learnt.
#[derive(Debug, Clone)]
pub struct SuffixCard {
    /// Person-gender-number key ("1cs", "3ms") — the [`Track::Suffix`]
    /// grading key, recorded in `progress.suffix_srs`.
    pub key: String,
    /// The pronoun the ending stands for ("me", "him") — the quiz answer.
    pub meaning: String,
    /// The host word carrying the ending; `surface == stem + suffix`.
    pub surface: String,
    /// The host's voiced reading (see [`crate::romanize::romanize`]).
    pub translit: String,
    pub stem: String,
    pub suffix: String,
    /// The host's learner gloss ("to me"), for the answer side. Empty when
    /// no curated or bridge gloss exists.
    pub gloss: String,
    /// Other endings' meanings as wrong answers (introduced endings first,
    /// topped up from the inventory). Empty when too few exist for a quiz.
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
    /// Aligned with `words`: true where the word is a proper name, so the app
    /// can render names distinctly — a name is sounded out, not translated.
    pub names: Vec<bool>,
}

/// A grammar concept shown with a short explanation, illustrated by a familiar
/// word that exercises it. Like [`StudyItem::ExplainMark`] it carries no grade —
/// the app acknowledges it and asks for the next item — and is shown at most
/// once (tracked in `progress.concepts_seen`). Content comes from
/// [`crate::grammar`].
#[derive(Debug, Clone)]
pub struct GrammarCard {
    /// Stable concept key (recorded once seen).
    pub concept: String,
    pub title: String,
    pub explanation: String,
    /// A compact formula ("וַ + imperfect → \"and he …\""), empty when none.
    pub formula: String,
    pub examples: Vec<String>,
    /// A word illustrating this concept — the learner's most familiar (or the
    /// corpus's most frequent) example, not necessarily the word about to be
    /// introduced (see `Bible::grammar_example_surface`).
    pub example: WordCard,
}

/// An explanation card the learner has already been shown, for the app's
/// reference page (see [`Bible::seen_concepts`]). `kind` says how to render
/// it: `"intro"` and `"final_forms"` cards keep their teaching content in the
/// app (keyed by `key`), a `"mark"` card's `key` is the reading-mark glyph
/// itself, and a `"grammar"` card carries its content from [`crate::grammar`]
/// in the remaining fields (empty for the other kinds).
#[derive(Debug, Clone)]
pub struct SeenConcept {
    pub kind: String,
    pub key: String,
    pub title: String,
    pub explanation: String,
    pub formula: String,
    pub examples: Vec<String>,
}

/// The next thing for the learner to do.
#[derive(Debug, Clone)]
pub enum StudyItem {
    NewGlyph(GlyphCard),
    ReviewGlyph(GlyphCard),
    NewWord(WordCard),
    ReviewWord(WordCard),
    /// A "which form is this?" drill for a word whose meaning is already known:
    /// the correct answer is the form's inflected gloss, the distractors are
    /// other inflections of the same word. Graded on [`Track::Form`].
    NewFormDrill(WordCard),
    ReviewFormDrill(WordCard),
    /// A pronominal ending drilled highlighted on a known host word (see
    /// [`SuffixCard`]). Introduced once the ending's concept card has been
    /// shown and a host word has graduated; graded on [`Track::Suffix`].
    NewSuffixDrill(SuffixCard),
    ReviewSuffixDrill(SuffixCard),
    /// A reading mark (sof pasuq, maqaf) shown with an explanation. Carries no
    /// grade — the app just acknowledges it and asks for the next item, like
    /// [`StudyItem::ReadVerse`]. Never revisited once shown.
    ExplainMark(GlyphCard),
    /// A grammar concept the next word exercises, shown once before that word's
    /// meaning card. Gradeless, like [`StudyItem::ExplainMark`].
    ExplainGrammar(GrammarCard),
    /// The final-forms concept — five letters (ך ם ן ף ץ) take a different
    /// shape at the end of a word — shown once, gating the first final-form
    /// glyph the learner meets (the card carries that glyph; the glyph itself
    /// is introduced on the next call). Gradeless, like
    /// [`StudyItem::ExplainMark`]; recorded in `progress.concepts_seen` under
    /// `FINAL_FORMS_CONCEPT`.
    ExplainFinalForms(GlyphCard),
    /// One card of the language-intro deck (reading direction, the alphabet,
    /// the vowel points), carrying its `INTRO_CONCEPTS` key; the content is
    /// presentation and lives in the app. Shown once each, before anything
    /// else is taught. Gradeless, like [`StudyItem::ExplainMark`]; recorded in
    /// `progress.concepts_seen`.
    ExplainIntro(String),
    ReadVerse(VerseCard),
    Done,
}

/// Headline progress counters for a status header. All counts mean *graduated*
/// (out of the in-session learning steps), so letters, vowels and words are on
/// the same standard — cards still being drilled are visible in [`TutorStats`]'s
/// seen/learning split, not here.
#[derive(Debug, Clone, Copy, Default)]
pub struct TutorProgress {
    /// Distinct base consonants graduated (begadkefat/shin dot-pairs folded to
    /// one leading codepoint; final forms kept as their own glyphs).
    pub letters_known: i64,
    /// Every base-consonant shape there is to learn (`LETTER_GLYPH_TOTAL`).
    pub letters_total: i64,
    /// Vowel points graduated (sheva through holam, qubuts, qamats qatan).
    pub vowels_known: i64,
    /// Every vowel-point glyph there is to learn (`VOWEL_GLYPH_TOTAL`).
    pub vowels_total: i64,
    /// Grammar rules taught so far (a [`crate::grammar::GrammarConcept`] card
    /// shown), out of `grammar_total`.
    pub grammar_known: i64,
    /// Every teachable grammar concept ([`crate::grammar::concept_count`]).
    pub grammar_total: i64,
    pub words_known: i64,
    /// Verses whose every required grammar rule is currently unlocked. This is
    /// inclusive of `verses_readable`, so a stacked progress bar can render the
    /// additional grammar-unlocked share as `verses_grammar_unlocked -
    /// verses_readable`.
    pub verses_grammar_unlocked: i64,
    /// Verses that have actually been presented for reading.
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
    /// Base consonants introduced, still in learning, and graduated (folded by
    /// leading codepoint; `letters_mature` matches
    /// [`TutorProgress::letters_known`]).
    pub letters_seen: i64,
    pub letters_learning: i64,
    pub letters_mature: i64,
    /// Vowel points introduced, still in learning, and graduated.
    pub vowels_seen: i64,
    pub vowels_learning: i64,
    pub vowels_mature: i64,
    /// Word meanings introduced, still in learning, and graduated.
    pub words_seen: i64,
    pub words_learning: i64,
    pub words_mature: i64,
    /// Grammar rules taught so far (gradeless, so there's no learning/mature
    /// split — a concept is either shown or not) and the total teachable.
    pub grammar_seen: i64,
    pub grammar_total: i64,
    /// Cards whose next review is now due (`due_epoch <= now`); every glyph
    /// (letters, vowels, marks) and word counts here.
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

/// User-configurable curriculum pacing, persisted per field in `progress.meta`
/// under `setting.*` keys (see [`Bible::tutor_settings`]); an unset key falls back
/// to the field default here. A [`Bible::reset_tutor`] clears `meta`, restoring
/// defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TutorSettings {
    /// Max glyphs (consonants/vowels) still in the in-session learning steps
    /// before new-glyph introduction pauses to let them consolidate — the
    /// alphabet ramp speed. Larger is faster (more new letters at once).
    pub letters_per_batch: u8,
    /// Max word meanings still in learning before new-word introduction pauses —
    /// the vocabulary ramp speed.
    pub words_per_batch: u8,
    /// Restrict introducible words to the grammar rules unlocked so far, so
    /// complexity expands one rule at a time (see `Bible::unlocked_concepts`).
    /// When false, every form is available immediately (the original behaviour).
    pub grammar_gating: bool,
    /// Priority given to high-frequency vocabulary when choosing the next verse.
    pub vocab_priority: u8,
    /// Grammar-expansion priority. Higher values unlock rules after fewer
    /// graduated words.
    pub grammar_priority: u8,
    /// Priority given to finishing nearly-readable verses when choosing the
    /// next target, rather than using a verse merely as a carrier for the most
    /// frequent available word.
    pub verse_priority: u8,
    /// Letters↔words balance, `0..=100` — the share of *new-material*
    /// introductions spent teaching new letters rather than the meaning of a
    /// word already spelt with letters the learner knows. Lower is more
    /// word-forward (read sooner with the letters you have); at `0` a new letter
    /// is introduced only when no already-readable word is left to learn. See
    /// `Bible::next_introduction`.
    pub letters_ratio: u8,
}

impl Default for TutorSettings {
    fn default() -> Self {
        TutorSettings {
            letters_per_batch: 3,
            words_per_batch: 8,
            grammar_gating: true,
            vocab_priority: 75,
            grammar_priority: 25,
            verse_priority: 25,
            letters_ratio: 30,
        }
    }
}

impl TutorSettings {
    /// Graduated words needed to unlock each successive grammar rule. A high
    /// grammar priority expands the frontier quickly; a low one leaves more
    /// space for vocabulary consolidation. The range deliberately matches the
    /// former vocabulary↔grammar balance (3..=30 words) for smooth migration.
    fn words_per_concept(&self) -> i64 {
        30 - self.grammar_priority as i64 * 27 / 100
    }
}

/// Vocab keys fully learnt (meaning graduated) — the "known" vocabulary for
/// verse coverage, folded through [`crate::vocab_gloss::vocab_key`] so a
/// dagesh/mark-order spelling twin of a graduated word (שָׁם→שָּׁם, בֶּן→בֶן)
/// counts as known instead of being dealt as a duplicate card. A subquery
/// reused across selection joins; join it through `surface_meta.vkey`.
const DONE_SURFACES: &str = "SELECT DISTINCT sm_done.vkey AS vkey \
     FROM progress.word_srs ws_done \
     JOIN progress.surface_meta sm_done ON sm_done.surface_id = ws_done.surface_id \
     WHERE ws_done.interval_days >= 1";

/// Roots the learner already knows — those with at least one fully-learnt
/// (meaning-graduated) surface, resolved through the [`Bible::ensure_surface_meta`]
/// cache. A new *form* of one of these roots is the cheapest thing to teach next,
/// so verse and word selection prefer them. Proper names are excluded: a name's
/// bridged root is usually a spurious homograph (אֶזְבָּי resolves to אות), so
/// having met the name says nothing about the root. A subquery reused across
/// joins.
const KNOWN_ROOTS: &str = "SELECT DISTINCT sm.root FROM progress.surface_meta sm \
     JOIN progress.word_srs ws ON ws.surface_id = sm.surface_id \
     WHERE ws.interval_days >= 1 AND sm.root <> '' AND sm.is_name = 0";

/// Roots whose simplest attested Qal member has been learnt.  This is the
/// family gate: an imperfect or derived stem must never be the learner's first
/// encounter with a verbal root when a simpler Qal form exists in Scripture.
const KNOWN_QAL_ROOTS: &str = "SELECT DISTINCT sm.root FROM progress.surface_meta sm \
     JOIN progress.word_srs ws ON ws.surface_id = sm.surface_id \
     WHERE ws.interval_days >= 1 AND sm.root <> '' AND sm.is_name = 0 \
       AND sm.is_qal = 1 AND sm.family_base = 1";

/// SQL predicate saying that a surface respects root-family order.  Roots
/// without an attested Qal are unaffected; otherwise only the simplest Qal
/// member is available until one such member graduates.
const FAMILY_READY: &str = "(sm.root = '' OR sm.is_name = 1 OR sm.form_tier < 5 \
      OR sm.family_base = 1 OR kqr.root IS NOT NULL)";

/// SQL predicate saying that a proclitic-bearing surface may only be introduced
/// after its bare lexical form has been introduced. Hebrew learners should meet עִם before וְעִם,
/// בַּיִת before לַבַּיִת, and so on. If the stripped form is not itself an
/// attested corpus surface there is nothing to teach separately, so the
/// prefixed form remains eligible.
const LEXICAL_BASE_READY: &str =
    "(sm.base_surface_id = sm.surface_id OR base_ws.surface_id IS NOT NULL)";

/// Bumped whenever [`form_tier`], the primary-root resolution, or the cached
/// `surface_meta` columns change, so a stale [`Bible::ensure_surface_meta`] cache
/// from an older build is rebuilt. 2 added `concept_rank`; 3 added `glyph_mask`;
/// 4 added the `weqatal` concept, changing `concept_rank` for vav-prefixed
/// perfect verbs; 5 made the shureq (וּ) its own teachable glyph, changing
/// `glyph_mask`; 6 folded the corpus's dotless shins (יִשָּׂשכָר, a few
/// scribal anomalies) into שׁ, changing `glyph_mask`; 7 added the standalone
/// `preposition` concept and the curated surface-concept table (function-word
/// prepositions and misparsed construct forms now carry a `concept_rank`);
/// 8 stopped spurious verb readings shadowing the definite article
/// ([`Bible::hebrew_word_info`]), dropping article+noun words like הָאָרֶץ
/// from verb-concept ranks to the article's rank; 9 added the
/// `object-marker` concept ahead of every other rank and curated the אֶת
/// family into the surface-concept table; 10 stopped the lexicon bridge
/// serving BDB cross-reference stubs, changing the primary root for the
/// re-bridged function words (אֵלָי no longer resolves to the stub's אלח);
/// 11 gave the noun bridge the same stub-filtered ranked lookup plus the
/// curated overrides (סוּס resolves to "horse"/root סוס, not the preceding
/// bird entry's root סוכ); 12 added the `prep-suffix` concept (pronoun
/// endings on prepositions) and curated every suffixed-preposition family
/// behind it — including pausal twins like אֵלָי, which previously missed
/// the table entirely and were introduced ungated; 13 dropped BDB
/// root-header stubs from the noun bridge (זָהָב carded "(√ of following;
/// meaning dubious…)" instead of "gold"); 14 curated הוּא/הַהוּא and the
/// ketiv הִוא (BDB's epicene "he; she; it" carded for the plain 3ms
/// pronoun); 15 added the `is_name` column (proper names stop inheriting
/// their bridged root's corpus frequency in verse/word ordering) and curated
/// the famous names the bridge mis-glossed (מֹשֶׁה "draw", שְׁלֹמֹה
/// "garment"), changing those surfaces' primary root; 16 curated the
/// plural-tantum nouns מַיִם/שָׁמַיִם/פָּנִים (BDB files them under
/// shortened consonant groups, so they carded blank or as a junk verb) and
/// ranked implausibly-peeled noun analyses last (הַשָּׁ+מָיִם no longer
/// shadows הַ+שָׁמַיִם); 17 flagged names by the resolved BDB lexeme's `pos`
/// (`n.pr*` / `adj.gent`) — most name entries carry only an etymology gloss
/// ("God hides"), so the gloss-text sniff missed ~40% of names (and every
/// gentilic), letting genealogy verses rank as easy and one-off names card
/// as vocabulary — and let a curated vocabulary gloss override the bridge
/// (בְּנֵי "sons of" bridges to BDB's *Bani* and was wrongly name-flagged);
/// 18 recovered the grammar cell of opaque-labelled irregular/gold noun forms
/// (a pronominal-suffix or plural tail on אֲבֹתָם/שְׁמוֹ/אֲנָשִׁים now gates
/// and glosses like any suffixed noun), folded final-form proclitic letters so
/// מֵאֶרֶץ's mem prefix classifies as prep-min, and sniffed the conjunctive
/// vav on otherwise-unclassifiable surfaces (וָמַעְלָה) — closing the hole
/// that let hundreds of suffixed rare forms rank as grammar-free beginner
/// vocabulary;
/// 19 made the noun bridge prefer the BDB lexeme whose pointed headword
/// matches the stem exactly (BDB files the verb before its derived nouns, so
/// segolates like חֹדֶשׁ/מֶלֶךְ carded the verb's gloss) and curated
/// חֹדֶשׁ "month; new moon";
/// 20 curated בְּרִית "covenant" (the bridge resolved it to BDB's
/// *Baal-berith* n.pr entry, so the ordinary noun carded "(a name)");
/// 21 gated אֲשֶׁר behind the relative-word concept card;
/// 22 taught the pronominal-ending inventory the ־ִי connecting-vowel
/// spellings the irregular kinship nouns join with (אָבִינוּ, אָחִיךְ,
/// פִּיהֶם), so their suffix cell recovers — they gloss as "our father"
/// instead of "father" and gate behind `suffix-possessive` instead of
/// ranking as grammar-free beginner vocabulary;
/// 23 broke the noun bridge's exact-headword tie in favour of non-verb
/// lexemes (hollow/stative verbs share the derived noun's pointing, so
/// הָאוֹר carded "the be" from אוֹר "be; become light").
/// 26 makes `family_base` select the simplest attested form when a verbal root
/// has no Qal surface, instead of opening every derived form at once.
/// 27 adds the lexical-base cache, gating proclitic-bearing vocabulary until
/// its bare form has been introduced.
/// 28 classifies the frozen לִקְרַאת family as verbal for Qal-first
/// curriculum ordering while retaining its lexicalized gloss.
/// 29 recovers feminine-plural possessive cells such as עֲלִילוֹתָיו, so they
/// gate behind possessive-suffix grammar and rank as complex noun forms.
const SURFACE_META_VERSION: i64 = 29;

/// Bumped when the meaning of the materialised readability columns changes.
/// Version 2 makes `verse_progress.unknown_words` count distinct vocabulary
/// keys rather than word tokens, matching the completion term used by target
/// selection and allowing that hot query to reuse the cached value.
const READABILITY_PROGRESS_VERSION: i64 = 2;

/// `concept_mask` sentinel for a surface the tutor cannot teach: no parse
/// gloss, no curated gloss, and not a name — its card would be blank. The bit
/// sits above every real concept bit and is never unlocked, so such a word is
/// never introduced and (in normal mode) a verse containing one is never a
/// target; the fix is data work (curation or analysis coverage), after which
/// the next [`SURFACE_META_VERSION`] rebuild lets the word back in.
const UNTEACHABLE_MASK: i64 = 1 << 62;

/// Distinct base consonants (final forms are drilled separately but don't count
/// here — see [`Bible::all_letters_known`]; begadkefat/shin dot-pairs counted
/// once by their base letter) that must be graduated before the alphabet counts
/// as "known" — no grammar rule unlocks until then. See
/// [`Bible::all_letters_known`].
const ALPHABET_CONSONANT_TARGET: i64 = 22;

/// Distinct vowel points that must be graduated (alongside the consonants)
/// before the alphabet counts as "known". Ten of the twelve niqqud, tolerating
/// a couple of rare ones (qamats qatan, a rarely-seen hataf) that a learner may
/// never meet, so grammar isn't gated behind a glyph that never appears.
const ALPHABET_VOWEL_TARGET: i64 = 10;

/// Every base-consonant shape the tutor drills, base letters plus final forms
/// (22 + 5) — the denominator for a "letters known" progress fraction, as
/// opposed to [`ALPHABET_CONSONANT_TARGET`] which gates grammar unlocking.
const LETTER_GLYPH_TOTAL: i64 = 27;

/// Every vowel-point glyph the tutor drills (ten common niqqud plus qubuts) —
/// the denominator for a "vowels known" progress fraction, as opposed to
/// [`ALPHABET_VOWEL_TARGET`] which gates grammar unlocking. Qamats qatan
/// (U+05C7) is a proper vowel (`is_vowel_point`) but the WLC encoding writes it as
/// plain qamats — zero corpus surfaces contain it, so counting it in the
/// denominator left the fraction permanently stuck at 11/12.
const VOWEL_GLYPH_TOTAL: i64 = 11;

/// A word's frequency for curriculum ordering. The simplest attested verbal
/// form that opens a root family uses the root's corpus frequency: choosing a
/// common family opener unlocks useful related surfaces. Every subsequent form
/// — and every nominal surface — must earn its place by its own frequency. This
/// prevents a rare noun or inflection from borrowing a large (or spuriously
/// bridged) root count: נְכֹאת occurs twice and therefore ranks as 2, not as 50
/// through its polluted נכא root. `form_tier >= 5` identifies verbs; roots with
/// no attested Qal still give their simplest attested verbal form the opener
/// role assigned by `family_base`. Assumes `sm`, `r` and `s` are joined.
const WORD_FREQ: &str = "CASE WHEN sm.family_base = 1 AND sm.form_tier >= 5 \
     THEN COALESCE(r.n_occurrences, s.occurrences) ELSE s.occurrences END";

/// Not a proper name — the words that count as real vocabulary. A verse with
/// no unknown real word teaches nothing and sorts last in targeting
/// ([`Bible::next_target_verse_excluding`]); the letter phase never
/// introduces names at all; and a teaching pin (dropped unread) never deals
/// its names ([`Bible::unfinished_words`]). Names are only ever show-once
/// freebies met while finishing a verse that's actually being read. Assumes
/// `sm` is joined.
const NOT_NAME: &str = "COALESCE(sm.is_name, 0) = 0";

/// Vocabulary ordering after grammar and root-family gates have filtered the
/// candidates: the most frequent root first, then its simplest attested form.
/// This directly maximises how many verse tokens each graduation can clear.
fn word_order() -> String {
    format!(
        "ORDER BY {WORD_FREQ} DESC, \
         COALESCE(sm.form_tier, 0) ASC, \
         s.occurrences DESC"
    )
}

/// A word's desirability while the alphabet is still being learnt: its
/// curriculum frequency ([`WORD_FREQ`]) discounted by a factor of 4 for every
/// *new* letter it would introduce (`?N` is the seen-glyph mask). Frequency
/// stays the dominant signal — a 500× word carrying one new letter still beats
/// a 2× name spelled with known letters — while the discount keeps the learner
/// on words within easy reach and feeds the alphabet a couple of letters at a
/// time. (A shift past 63 yields 0 in SQLite, so all-new long words simply
/// score 0 and sort last.)
fn letter_learning_score(seen_mask_param: &str) -> String {
    format!("({WORD_FREQ} >> (2 * popcount(sm.glyph_mask & ~{seen_mask_param})))")
}

/// A verse's calibration difficulty: its rarest word's OT occurrence count.
const DIFFICULTY: &str = "MIN(s.occurrences)";
/// Excludes Biblical Aramaic verses from a `verse_word`/`surface` grouping —
/// reused by every calibration query.
const NOT_ARAMAIC: &str = "SUM(CASE WHEN s.language = 'aramaic' THEN 1 ELSE 0 END) = 0";

/// The learner-facing gloss for an uncurated proper name — the card says
/// "this word is somebody's name" rather than serving BDB's citation
/// ("n.pr.m. father of one of David's men") as if it were a meaning. Also the
/// sentinel [`Bible::next_introduction`] matches to seed such cards known
/// after a single showing instead of drilling them.
const NAME_GLOSS: &str = "(a name)";

/// SM-2 state for a card seeded as already-known by onboarding calibration
/// (see [`Bible::seed_known_alphabet`], [`Bible::seed_known_vocab`]) rather
/// than actually drilled: graduated, but due for a retention check in two
/// weeks rather than treated as permanently mastered.
fn seeded_known_srs() -> Srs {
    Srs {
        ease: DEFAULT_EASE,
        interval_days: 14,
        reps: 3,
        lapses: 0,
    }
}

/// A verse shown during onboarding's vocabulary-calibration binary search
/// (see [`Bible::calibration_probe`]): one of the corpus's distinct
/// difficulty tiers, `tier` 0 being the easiest (most common rarest-word).
#[derive(Debug, Clone)]
pub struct CalibrationProbe {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
    pub text: String,
    pub tier: u32,
    /// This verse's difficulty: its rarest word's OT occurrence count. If the
    /// learner can read this verse, every word occurring at least this often
    /// is a reasonable "already known" cutoff (see [`Bible::seed_known_vocab`]).
    pub min_occurrences: i64,
}

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

    // `surface_meta`'s columns have evolved (`concept_rank` became the
    // set-valued `concept_mask` when grammar unlocks stopped being a fixed
    // total order); drop an older schema so it is recreated (and
    // `ensure_surface_meta` repopulated) with the current columns. Cheap,
    // rebuilt on the next study call.
    let sm_sql: Option<String> = db
        .query_row(
            "SELECT sql FROM progress.sqlite_master WHERE type='table' AND name='surface_meta'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(sql) = sm_sql
        && !(sql.contains("concept_mask") && sql.contains("glyph_mask") && sql.contains("vkey"))
    {
        db.execute_batch("DROP TABLE progress.surface_meta")?;
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
            last_grade       INTEGER NOT NULL,
            updated_epoch    INTEGER NOT NULL DEFAULT 0
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
            last_grade       INTEGER NOT NULL,
            updated_epoch    INTEGER NOT NULL DEFAULT 0
         );
         CREATE INDEX IF NOT EXISTS progress.idx_word_srs_id ON word_srs(surface_id);
         CREATE TABLE IF NOT EXISTS progress.form_srs(
            surface          TEXT    PRIMARY KEY,
            surface_id       INTEGER NOT NULL,
            ease             REAL    NOT NULL,
            interval_days    INTEGER NOT NULL,
            due_epoch        INTEGER NOT NULL,
            reps             INTEGER NOT NULL,
            lapses           INTEGER NOT NULL,
            introduced_epoch INTEGER NOT NULL,
            last_grade       INTEGER NOT NULL,
            updated_epoch    INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS progress.suffix_srs(
            key              TEXT    PRIMARY KEY,
            ease             REAL    NOT NULL,
            interval_days    INTEGER NOT NULL,
            due_epoch        INTEGER NOT NULL,
            reps             INTEGER NOT NULL,
            lapses           INTEGER NOT NULL,
            introduced_epoch INTEGER NOT NULL,
            last_grade       INTEGER NOT NULL,
            updated_epoch    INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS progress.surface_progress(
            surface_id      INTEGER PRIMARY KEY,
            graduated       INTEGER NOT NULL DEFAULT 0,
            graduated_epoch INTEGER
         );
         CREATE TABLE IF NOT EXISTS progress.verse_progress(
            book               INTEGER NOT NULL,
            chapter            INTEGER NOT NULL,
            verse              INTEGER NOT NULL,
            unknown_words      INTEGER NOT NULL,
            min_root_frequency INTEGER NOT NULL,
            mean_root_frequency REAL NOT NULL,
            max_root_frequency INTEGER NOT NULL,
            last_read_epoch    INTEGER NOT NULL DEFAULT 0,
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
         );
         CREATE TABLE IF NOT EXISTS progress.surface_meta(
            surface_id   INTEGER PRIMARY KEY,
            root         TEXT    NOT NULL,
            form_tier    INTEGER NOT NULL,
            concept_mask INTEGER NOT NULL,
            glyph_mask   INTEGER NOT NULL,
            is_name      INTEGER NOT NULL,
            is_qal       INTEGER NOT NULL,
            family_base  INTEGER NOT NULL,
            vkey         TEXT    NOT NULL,
            base_vkey    TEXT    NOT NULL,
            base_surface_id INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS progress.idx_surface_meta_vkey ON surface_meta(vkey);
         CREATE INDEX IF NOT EXISTS progress.idx_surface_meta_root ON surface_meta(root);
         CREATE TABLE IF NOT EXISTS progress.verse_meta(
            book         INTEGER NOT NULL,
            chapter      INTEGER NOT NULL,
            verse        INTEGER NOT NULL,
            concept_mask INTEGER NOT NULL,
            PRIMARY KEY (book, chapter, verse)
         );
         CREATE TABLE IF NOT EXISTS progress.concepts_seen(
            concept          TEXT PRIMARY KEY,
            introduced_epoch INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS progress.concepts_unlocked(
            concept        TEXT    PRIMARY KEY,
            unlocked_epoch INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS progress.gloss_overrides(
            surface       TEXT    PRIMARY KEY,
            gloss         TEXT    NOT NULL,
            note          TEXT    NOT NULL DEFAULT '',
            updated_epoch INTEGER NOT NULL,
            deleted       INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS progress.lexicon_entry_overrides(
            surface       TEXT    PRIMARY KEY,
            root          TEXT    NOT NULL DEFAULT '',
            gloss         TEXT    NOT NULL,
            updated_epoch INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS progress.issue_reports(
            id            TEXT    PRIMARY KEY,
            report_type   TEXT    NOT NULL,
            note          TEXT    NOT NULL,
            context_json  TEXT    NOT NULL,
            created_epoch INTEGER NOT NULL,
            updated_epoch INTEGER NOT NULL,
            deleted       INTEGER NOT NULL DEFAULT 0
         );",
    )?;

    let issue_reports_has_deleted = {
        let mut stmt = db.prepare("PRAGMA progress.table_info(issue_reports)")?;
        stmt.query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter()
            .any(|name| name == "deleted")
    };
    if !issue_reports_has_deleted {
        db.execute_batch(
            "ALTER TABLE progress.issue_reports \
             ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0",
        )?;
    }

    // `verse_progress` used to be a tiny read-history table. Readability is now
    // derived from graduated surfaces, so replace that incompatible layout.
    let verse_progress_is_readability = {
        let mut stmt = db.prepare("PRAGMA progress.table_info(verse_progress)")?;
        stmt.query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter()
            .any(|name| name == "unknown_words")
    };
    if !verse_progress_is_readability {
        db.execute_batch(
            "DROP TABLE progress.verse_progress;
             CREATE TABLE progress.verse_progress(
                book INTEGER NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL,
                unknown_words INTEGER NOT NULL,
                min_root_frequency INTEGER NOT NULL,
                mean_root_frequency REAL NOT NULL,
                max_root_frequency INTEGER NOT NULL,
                last_read_epoch INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY(book, chapter, verse));",
        )?;
    }
    db.execute_batch(
        "CREATE INDEX IF NOT EXISTS progress.idx_surface_progress_graduated
            ON surface_progress(graduated);
         CREATE INDEX IF NOT EXISTS progress.idx_verse_progress_unknown
            ON verse_progress(unknown_words);",
    )?;

    // A per-card modification time makes offline LAN sync deterministic. Older
    // progress files did not record it; their zero value is resolved using the
    // review count during their first merge.
    for table in ["glyph_srs", "word_srs", "form_srs", "suffix_srs"] {
        let has_updated_epoch = {
            let mut stmt = db.prepare(&format!("PRAGMA progress.table_info({table})"))?;
            stmt.query_map([], |r| r.get::<_, String>(1))?
                .collect::<rusqlite::Result<Vec<_>>>()?
                .iter()
                .any(|name| name == "updated_epoch")
        };
        if !has_updated_epoch {
            db.execute_batch(&format!(
                "ALTER TABLE progress.{table} ADD COLUMN updated_epoch INTEGER NOT NULL DEFAULT 0"
            ))?;
        }
    }

    // Pruned gloss corrections need a synchronisable tombstone; otherwise an
    // older copy still held by the LAN server would reappear on the next merge.
    let gloss_overrides_have_deleted = {
        let mut stmt = db.prepare("PRAGMA progress.table_info(gloss_overrides)")?;
        stmt.query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter()
            .any(|name| name == "deleted")
    };
    if !gloss_overrides_have_deleted {
        db.execute(
            "ALTER TABLE progress.gloss_overrides \
             ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }

    // Older progress databases predate the root-family Qal marker.
    let has_is_qal = {
        let mut stmt = db.prepare("PRAGMA progress.table_info(surface_meta)")?;
        stmt.query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter()
            .any(|name| name == "is_qal")
    };
    if !has_is_qal {
        db.execute(
            "ALTER TABLE progress.surface_meta ADD COLUMN is_qal INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    let has_family_base = {
        let mut stmt = db.prepare("PRAGMA progress.table_info(surface_meta)")?;
        stmt.query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter()
            .any(|name| name == "family_base")
    };
    if !has_family_base {
        db.execute(
            "ALTER TABLE progress.surface_meta ADD COLUMN family_base INTEGER NOT NULL DEFAULT 1",
            [],
        )?;
    }
    let has_base_vkey = {
        let mut stmt = db.prepare("PRAGMA progress.table_info(surface_meta)")?;
        stmt.query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter()
            .any(|name| name == "base_vkey")
    };
    if !has_base_vkey {
        db.execute(
            "ALTER TABLE progress.surface_meta ADD COLUMN base_vkey TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    let has_base_surface_id = {
        let mut stmt = db.prepare("PRAGMA progress.table_info(surface_meta)")?;
        stmt.query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter()
            .any(|name| name == "base_surface_id")
    };
    if !has_base_surface_id {
        db.execute(
            "ALTER TABLE progress.surface_meta ADD COLUMN base_surface_id INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }

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
    // folding the dot in — and the corpus's genuinely dotless shins
    // (יִשָּׂשכָר, a few scribal anomalies) used to be introduced as a bare-ש
    // glyph card instead of folding into שׁ. Purge that stale key; a
    // correctly-dotted "שׁ"/"שׂ" row is unaffected and gets re-introduced
    // normally if missing.
    db.execute(
        "DELETE FROM progress.glyph_srs WHERE glyph = ?1",
        params![SHIN.to_string()],
    )?;
    Ok(())
}

fn is_consonant(c: char) -> bool {
    (0x05D0..=0x05EA).contains(&(c as u32))
}

/// One of the five final (sofit) letter forms ך ם ן ף ץ — the end-of-word
/// shape of kaf/mem/nun/pe/tsadi, drilled as its own glyph.
fn is_final_form(c: char) -> bool {
    matches!(
        c,
        '\u{05DA}' | '\u{05DD}' | '\u{05DF}' | '\u{05E3}' | '\u{05E5}'
    )
}

/// A proper vowel point (sheva through holam, qubuts, qamats qatan) — taught on
/// a host consonant. Excludes dagesh and the shin/sin dots.
fn is_vowel_point(c: char) -> bool {
    matches!(c as u32, 0x05B0..=0x05B9 | 0x05BB | 0x05C7)
}

fn is_hataf(vowel: char) -> bool {
    matches!(vowel as u32, 0x05B1..=0x05B3)
}

/// A stable bit index (0..=43) for a teachable glyph, so a surface's glyph set
/// packs into an [`i64`] bitmask (see [`surface_glyph_mask`]) for letter-aware
/// curriculum ordering. Each consonant takes its offset from aleph (final forms
/// have their own codepoints, so ך/כ land on distinct bits and are drilled as
/// separate glyphs); the begadkefat/shin dot-pairs, the vowel points and the
/// shureq take fixed slots above them. `None` for anything not taught as its
/// own glyph.
fn glyph_bit(glyph: &str) -> Option<u32> {
    let mut chars = glyph.chars();
    let first = chars.next()?;
    let second = chars.next();
    match (first, second) {
        ('\u{05D1}', Some('\u{05BC}')) => Some(27), // בּ (bet)
        ('\u{05E4}', Some('\u{05BC}')) => Some(28), // פּ (pe)
        ('\u{05E9}', Some('\u{05C1}')) => Some(29), // שׁ (shin)
        ('\u{05E9}', Some('\u{05C2}')) => Some(30), // שׂ (sin)
        ('\u{05D5}', Some('\u{05BC}')) => Some(43), // וּ (shureq)
        (c, None) if is_consonant(c) => Some(c as u32 - 0x05D0), // 0..=26
        (c, None) if is_vowel_point(c) => Some(31 + vowel_slot(c)), // 31..=42
        _ => None,
    }
}

/// Fixed 0..=11 slot for a vowel point within the [`glyph_bit`] mask.
fn vowel_slot(c: char) -> u32 {
    match c as u32 {
        0x05BB => 10,    // qubuts
        0x05C7 => 11,    // qamats qatan
        u => u - 0x05B0, // sheva..holam → 0..=9
    }
}

/// The distinct teachable glyphs of `surface` packed into a bitmask (one bit per
/// glyph, see [`glyph_bit`]) — its letter set, cached in `surface_meta` so the
/// curriculum can order by how many *new* letters a word introduces.
fn surface_glyph_mask(surface: &str) -> i64 {
    let mut mask = 0i64;
    for g in decompose_glyphs(surface) {
        if let Some(b) = glyph_bit(&g.glyph) {
            mask |= 1i64 << b;
        }
    }
    mask
}

/// Bet and pe, whose dagesh changes the *sound* (vet→bet, fe→pe) rather than
/// just marking gemination — taught as two distinct letters, not a base
/// consonant plus a separately-drilled dagesh mark.
const DAGESH_LETTERS: [char; 2] = ['\u{05D1}', '\u{05E4}'];

/// Shin, whose following shin-dot or sin-dot picks between two distinct sounds
/// (sh / s) — taught as two distinct letters, not a base consonant plus a
/// separately-drilled dot.
const SHIN: char = '\u{05E9}';

/// Vav, which when it carries a dagesh but no vowel of its own is not a
/// consonant at all but the shureq vowel (וּ → "u") — taught as its own glyph,
/// distinct from consonantal ו. A vav with both a vowel and a dagesh is an
/// ordinary geminated consonant (e.g. חַוָּה) and stays plain ו.
const VAV: char = '\u{05D5}';

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
/// separately-drilled mark. A vowel-less vav folds its dagesh in the same
/// way — that pair is the shureq vowel (וּ), not a consonant (see [`VAV`]).
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
        // A shin always carries its shin/sin dot — except the conventionally
        // silent, dotless second shin of יִשָּׂשכָר (Issachar) and a few
        // Leningrad Codex scribal omissions (אִיש Deut 24:16, שֵיבָה Isa 46:4,
        // …). A bare ש is not a distinct letter to learn, so fold it into the
        // standard שׁ rather than introduce a dotless glyph card.
        let m = dot.unwrap_or('\u{05C1}');
        format!("{letter}{m}")
    } else if letter == VAV && vowels.is_empty() {
        // A vowel-less vav with a dagesh is the shureq vowel (וּ → "u"), a
        // distinct reading taught as its own glyph — without this, a word like
        // סוּס gates only on its consonants and "vav + dagesh = u" is never
        // taught. With a vowel present the dagesh is gemination (חַוָּה).
        dagesh.map_or_else(|| letter.to_string(), |m| format!("{letter}{m}"))
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
            let (key, vowels, consumed) = letter_cluster(c, &chars[i + 1..]);
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
        let c = chars[i];
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
/// first-seen order: consonants (final forms are their own glyph, with a dagesh/shin-sin-dot
/// folded into begadkefat/shin letters and the shureq's dagesh into its vav —
/// see [`letter_cluster`]) and vowel points. A dagesh or shin/sin dot not
/// folded into a letter this way is a gemination/orthographic mark that
/// doesn't change the sound and is not taught as its own glyph.
fn decompose_glyphs(surface: &str) -> Vec<GlyphCard> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let chars: Vec<char> = surface.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if is_consonant(c) {
            let (key, vowels, consumed) = letter_cluster(c, &chars[i + 1..]);
            if seen.insert(key.clone()) {
                out.push(GlyphCard {
                    glyph: key,
                    is_consonant: true,
                    ..Default::default()
                });
            }
            for v in vowels {
                let vk = v.to_string();
                if seen.insert(vk.clone()) {
                    out.push(GlyphCard {
                        glyph: vk,
                        ..Default::default()
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
                    ..Default::default()
                });
            }
        }
        i += 1;
    }
    out
}

/// The highest [`form_tier`] value — verbs in a rare derived stem, in the
/// hardest conjugation, carrying a proclitic and an object suffix.
const TIER_MAX: u8 = 13;

/// A learner-difficulty tier for a word's grammatical *form*, independent of how
/// rare the word is. `0` is a word with nothing to parse (a closed-class
/// function word or proper noun, learnt whole); nominal inflections occupy the
/// low tiers, verb conjugations the higher ones, so that the plainest forms of a
/// root are introduced before its harder inflections. An attached proclitic
/// (article/preposition/conjunction) and a pronominal object suffix each add a
/// step; vav-consecutive is *not* added separately as it is already captured by
/// the Wayyiqtol conjugation. See [`Bible::ensure_surface_meta`], which caches
/// one tier per surface for the curriculum's ordering.
fn form_tier(w: &HebrewWord) -> u8 {
    let base = match &w.form {
        // Verb: binyan (derivation) plus conjugation.
        Some(binyan) => {
            let stem = match binyan.as_str() {
                "Qal" | "Qal passive" => 0,
                "Niphal" | "Piel" | "Pual" | "Hiphil" | "Hophal" | "Hithpael" => 2,
                _ => 3, // rare stems (Polel, Pilpel, Hishtaphel, …)
            };
            let is_3ms = w.person.as_deref() == Some("Third")
                && w.gender.as_deref() == Some("Masculine")
                && w.number.as_deref() == Some("Singular");
            let conj = match w.tense.as_deref() {
                // Perfect 3ms is the citation-like base; other PGN a touch more.
                Some("Perfect") if is_3ms => 0,
                Some("Perfect") => 1,
                Some("Participle (act.)")
                | Some("Participle (pas.)")
                | Some("Participle (pass.)")
                | Some("Participle") => 1,
                Some("Imperfect") | Some("Wayyiqtol") | Some("Jussive") | Some("Cohortative") => 2,
                Some("Imperative") | Some("Inf. Construct") | Some("Inf. Absolute") => 3,
                _ => 2,
            };
            5 + stem + conj
        }
        // Noun / adjective / other nominal: has number/state but no verb form.
        None if w.tense.is_none() && (w.number.is_some() || w.state.is_some()) => {
            match w.state.as_deref() {
                // A pronominal-suffix label from `decode_noun_label`, e.g. "Sg + 3ms".
                Some(s) if s.contains('+') => 4,
                Some("Construct") | Some("Directional") => 3,
                _ => match w.number.as_deref() {
                    Some("Plural") | Some("Dual") => 2,
                    _ => 1, // singular absolute
                },
            }
        }
        // Function word / proper noun / unresolved: nothing to parse.
        None => return 0,
    };
    let extra = w.prefix.is_some() as u8 + w.obj_suffix.is_some() as u8;
    (base + extra).min(TIER_MAX)
}

/// Frozen forms that the corpus prefilter intentionally treats as lexical
/// items, but whose curriculum ordering still belongs to a verbal family.
/// `לִקְרַאת` functions much like a preposition ("to meet; toward"), so it
/// should keep that learner-facing gloss while waiting for the plain Qal
/// citation form `קָרָא`. Its suffixed spellings belong to the same family.
fn lexicalized_verb_tier(surface: &str) -> Option<u8> {
    matches!(surface, "לִקְרַאת" | "לִקְרָאתוֹ" | "לִקְרַאתְכֶם").then_some(8)
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

    fn form_srs(&self, surface: &str) -> rusqlite::Result<Option<Srs>> {
        self.conn()
            .query_row(
                "SELECT ease, interval_days, reps, lapses FROM progress.form_srs \
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

    fn suffix_srs(&self, key: &str) -> rusqlite::Result<Option<Srs>> {
        self.conn()
            .query_row(
                "SELECT ease, interval_days, reps, lapses FROM progress.suffix_srs \
                 WHERE key = ?1",
                params![key],
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

    /// Every glyph of `surface` at least introduced (though possibly still in
    /// learning) — the word is within reach: drilling what's already open,
    /// with no further new letters, will make it readable.
    fn all_glyphs_seen(&self, surface: &str) -> rusqlite::Result<bool> {
        for g in decompose_glyphs(surface) {
            if !self.glyph_known(&g.glyph)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    // --- pacing settings & unlock frontier ----------------------------------

    /// The persisted curriculum-pacing settings, each field falling back to its
    /// [`TutorSettings::default`] when unset.
    pub fn tutor_settings(&self) -> rusqlite::Result<TutorSettings> {
        let d = TutorSettings::default();
        let get = |key: &str| -> rusqlite::Result<Option<i64>> {
            self.conn()
                .query_row(
                    "SELECT CAST(value AS INTEGER) FROM progress.meta WHERE key = ?1",
                    params![key],
                    |r| r.get(0),
                )
                .optional()
        };
        Ok(TutorSettings {
            letters_per_batch: get("setting.letters_per_batch")?
                .map_or(d.letters_per_batch, |n| n.clamp(1, 255) as u8),
            words_per_batch: get("setting.words_per_batch")?
                .map_or(d.words_per_batch, |n| n.clamp(1, 255) as u8),
            grammar_gating: get("setting.grammar_gating")?.map_or(d.grammar_gating, |n| n != 0),
            // Migrate the former two-ended vocabulary↔grammar slider without
            // rewriting the database: its value maps to vocabulary priority,
            // and its inverse preserves the exact grammar unlock cadence.
            vocab_priority: get("setting.vocab_priority")?
                .or(get("setting.vocab_ratio")?)
                .map_or(d.vocab_priority, |n| n.clamp(0, 100) as u8),
            grammar_priority: get("setting.grammar_priority")?
                .or(get("setting.vocab_ratio")?.map(|n| 100 - n))
                .map_or(d.grammar_priority, |n| n.clamp(0, 100) as u8),
            verse_priority: get("setting.verse_priority")?
                .map_or(d.verse_priority, |n| n.clamp(0, 100) as u8),
            letters_ratio: get("setting.letters_ratio")?
                .map_or(d.letters_ratio, |n| n.clamp(0, 100) as u8),
        })
    }

    /// Persist the curriculum-pacing settings (one `meta` row per field).
    pub fn set_tutor_settings(&self, s: &TutorSettings) -> rusqlite::Result<()> {
        let put = |key: &str, val: i64| -> rusqlite::Result<()> {
            self.conn().execute(
                "INSERT INTO progress.meta(key, value) VALUES (?1, ?2) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, val.to_string()],
            )?;
            Ok(())
        };
        put("setting.letters_per_batch", s.letters_per_batch as i64)?;
        put("setting.words_per_batch", s.words_per_batch as i64)?;
        put("setting.grammar_gating", s.grammar_gating as i64)?;
        put("setting.vocab_priority", s.vocab_priority as i64)?;
        put("setting.grammar_priority", s.grammar_priority as i64)?;
        put("setting.verse_priority", s.verse_priority as i64)?;
        put("setting.letters_ratio", s.letters_ratio as i64)?;
        Ok(())
    }

    /// Running counts of new-letter and new-word introductions, kept in `meta`
    /// so [`Self::next_introduction`] can hold the letter/word mix near
    /// [`TutorSettings::letters_ratio`] over time. Reset with the rest of `meta`.
    fn intro_counts(&self) -> rusqlite::Result<(i64, i64)> {
        let get = |key: &str| -> rusqlite::Result<i64> {
            Ok(self
                .conn()
                .query_row(
                    "SELECT CAST(value AS INTEGER) FROM progress.meta WHERE key = ?1",
                    params![key],
                    |r| r.get(0),
                )
                .optional()?
                .unwrap_or(0))
        };
        Ok((get("intro.letters")?, get("intro.words")?))
    }

    fn bump_intro_counter(&self, key: &str) -> rusqlite::Result<()> {
        self.conn().execute(
            "INSERT INTO progress.meta(key, value) VALUES (?1, 1) \
             ON CONFLICT(key) DO UPDATE SET value = CAST(value AS INTEGER) + 1",
            params![key],
        )?;
        Ok(())
    }

    /// Glyph cards still in the in-session learning steps (not graduated) — the
    /// count throttled by [`TutorSettings::letters_per_batch`].
    fn glyphs_in_learning(&self) -> rusqlite::Result<i64> {
        self.conn().query_row(
            "SELECT COUNT(*) FROM progress.glyph_srs WHERE interval_days = 0",
            [],
            |r| r.get(0),
        )
    }

    /// Word-meaning cards still in the in-session learning steps (not graduated)
    /// — the count throttled by [`TutorSettings::words_per_batch`].
    fn words_in_learning(&self) -> rusqlite::Result<i64> {
        self.conn().query_row(
            "SELECT COUNT(*) FROM progress.word_srs WHERE interval_days = 0",
            [],
            |r| r.get(0),
        )
    }

    /// Graduated word meanings — the "known" vocabulary size, which paces how
    /// many grammar rules have unlocked.
    fn words_mature(&self) -> rusqlite::Result<i64> {
        self.conn().query_row(
            "SELECT COUNT(*) FROM progress.word_srs WHERE interval_days >= 1",
            [],
            |r| r.get(0),
        )
    }

    /// Whether the alphabet is known: at least [`ALPHABET_CONSONANT_TARGET`]
    /// distinct base consonants *and* [`ALPHABET_VOWEL_TARGET`] vowel points have
    /// graduated. Until then no grammar rule unlocks, so the learner meets only
    /// the simplest words while building up the letters (see
    /// [`Self::unlocked_concepts`]). Consonants are counted by distinct leading
    /// codepoint so begadkefat/shin dot-pairs (`בּ`/`ב`) count once per letter.
    /// Final forms (ך ם ן ף ץ) are their own drilled glyphs but are excluded
    /// here, so the gate still means "the 22 base letters", not 22 of the 27
    /// shapes (a learner could otherwise unlock grammar without a base letter).
    fn all_letters_known(&self) -> rusqlite::Result<bool> {
        let consonants: i64 = self.conn().query_row(
            "SELECT COUNT(DISTINCT unicode(substr(glyph, 1, 1))) FROM progress.glyph_srs \
             WHERE interval_days >= 1 AND unicode(substr(glyph, 1, 1)) BETWEEN 1488 AND 1514 \
               AND unicode(substr(glyph, 1, 1)) NOT IN (1498, 1501, 1503, 1507, 1509)",
            [],
            |r| r.get(0),
        )?;
        let vowels: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM progress.glyph_srs \
             WHERE interval_days >= 1 \
               AND (unicode(glyph) BETWEEN 1456 AND 1465 OR unicode(glyph) IN (1467, 1479))",
            [],
            |r| r.get(0),
        )?;
        Ok(consonants >= ALPHABET_CONSONANT_TARGET && vowels >= ALPHABET_VOWEL_TARGET)
    }

    /// Bitmask ([`glyph_bit`]) of every glyph the learner has at least *seen*
    /// (introduced), for counting how many *new* letters a word would add in
    /// letter-aware ordering.
    fn seen_glyph_mask(&self) -> rusqlite::Result<i64> {
        let mut mask = 0i64;
        let mut stmt = self
            .conn()
            .prepare("SELECT glyph FROM progress.glyph_srs")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        for g in rows {
            if let Some(b) = glyph_bit(&g?) {
                mask |= 1i64 << b;
            }
        }
        Ok(mask)
    }

    /// The bitmask of currently unlocked grammar rules
    /// ([`crate::grammar::concept_bit`] encoding). With gating off, all of
    /// them. Otherwise **none** until the whole alphabet is known
    /// ([`Self::all_letters_known`]) — the learner stays on simple,
    /// grammar-free words while learning the letters — after which rules
    /// unlock one at a time, one per [`TutorSettings::words_per_concept`]
    /// graduated words (the first the moment letters are done).
    ///
    /// *Which* rule unlocks next is not a fixed order: concepts carry an
    /// intrinsic-complexity `bucket` (every bucket-0 rule unlocks before any
    /// bucket-1 rule), and within the current bucket the tutor picks the
    /// locked concept that completes the most verses — i.e. maximises the
    /// number of `progress.verse_meta` rows whose mask would fit inside the
    /// new frontier. The choice is persisted in `progress.concepts_unlocked`,
    /// so it is stable across sessions and survives re-derivations.
    fn unlocked_concepts(&self, s: &TutorSettings, now: i64) -> rusqlite::Result<i64> {
        let total = crate::grammar::concept_count() as i64;
        if !s.grammar_gating {
            return Ok(crate::grammar::all_concepts_mask());
        }
        let mut target = 0;
        if self.all_letters_known()? {
            target = 1 + self.words_mature()? / s.words_per_concept();
        }
        let target = target.min(total);

        // The persisted set (ignoring keys from an older concept inventory).
        let mut mask = 0i64;
        {
            let mut stmt = self
                .conn()
                .prepare("SELECT concept FROM progress.concepts_unlocked")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            for key in rows {
                if let Some(bit) = crate::grammar::concept_bit(&key?) {
                    mask |= bit;
                }
            }
        }
        while (mask.count_ones() as i64) < target {
            let Some(next) = self.next_concept_to_unlock(mask)? else {
                break;
            };
            debug!("unlocked_concepts: unlocking [{next}] (mask={mask:#x})");
            self.conn().execute(
                "INSERT INTO progress.concepts_unlocked(concept, unlocked_epoch) \
                 VALUES (?1, ?2) ON CONFLICT(concept) DO NOTHING",
                params![next, now],
            )?;
            mask |= crate::grammar::concept_bit(next).unwrap_or(0);
        }
        Ok(mask)
    }

    /// The locked concept to unlock next: from the lowest bucket that still
    /// has locked concepts, the one that moves the most verses closest to
    /// completable — ties resolved by inventory order. `None` when everything
    /// is unlocked.
    ///
    /// "Closest" is a proximity-weighted coverage score, `Σ 1/(1 + missing)`
    /// over all verses, where `missing` is how many locked concepts the verse
    /// would still need under `unlocked | candidate`. Counting only outright
    /// completions would starve the verb concepts forever: a narrative verse
    /// needs wayyiqtol *and* construct *and* a suffix rule before it
    /// completes, so no verb ever won a completions-only vote and the flow
    /// stayed stuck on verbless list-verses (genealogies, censuses). Under
    /// the weighted score, appearing in thousands of nearly-ready narrative
    /// verses is worth more than finishing a handful of name lists.
    fn next_concept_to_unlock(&self, unlocked: i64) -> rusqlite::Result<Option<&'static str>> {
        let locked: Vec<&'static crate::grammar::GrammarConcept> = crate::grammar::concepts()
            .iter()
            .filter(|c| crate::grammar::concept_bit(c.key).is_some_and(|b| unlocked & b == 0))
            .collect();
        let Some(bucket) = locked.iter().map(|c| c.bucket).min() else {
            return Ok(None);
        };
        let mut best: Option<(&'static str, f64)> = None;
        for c in locked.iter().filter(|c| c.bucket == bucket) {
            let bit = crate::grammar::concept_bit(c.key).unwrap_or(0);
            let frontier = unlocked | bit;
            // Verses without the candidate's bit contribute identically for
            // every candidate, so they cancel out of the comparison.
            let gain: f64 = self.conn().query_row(
                "SELECT COALESCE(SUM(1.0 / (1 + popcount(concept_mask & ~?1))), 0) \
                 FROM progress.verse_meta",
                params![frontier],
                |r| r.get(0),
            )?;
            if best.is_none_or(|(_, g)| gain > g) {
                best = Some((c.key, gain));
            }
        }
        Ok(best.map(|(k, _)| k))
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
        // Any known audible consonant (aleph/ayin excluded; the shureq glyph
        // 'וּ' leads with a vav codepoint but is a vowel, never a host).
        self.conn()
            .query_row(
                "SELECT glyph FROM progress.glyph_srs \
                 WHERE unicode(glyph) BETWEEN 1488 AND 1514 \
                   AND glyph NOT IN ('א','ע','וּ') LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
    }

    /// Build a NewGlyph item, showing a vowel on a learnt valid host (teaching a
    /// host consonant first if none is learnt yet). The very first final-form
    /// consonant is preceded by the one-time final-forms concept card instead
    /// (see [`Self::final_form_gated`]).
    fn new_glyph_item(
        &self,
        surface: &str,
        g: &GlyphCard,
        now: i64,
    ) -> rusqlite::Result<StudyItem> {
        let ch = g.glyph.chars().next().unwrap_or(' ');
        if !is_vowel_point(ch) {
            return self.final_form_gated(g.clone(), now);
        }
        match self.known_vowel_host(surface, ch)? {
            Some(host) => Ok(StudyItem::NewGlyph(GlyphCard {
                voiced: crate::romanize::voiced_syllable(&host, ch),
                host: Some(host),
                ..g.clone()
            })),
            None => self.final_form_gated(
                GlyphCard {
                    glyph: host_to_teach(surface, ch),
                    is_consonant: true,
                    ..Default::default()
                },
                now,
            ),
        }
    }

    /// Wrap a new consonant card in the final-forms gate: the first time the
    /// glyph to introduce is a final form, return the one-time
    /// [`StudyItem::ExplainFinalForms`] card instead, marking the concept seen
    /// immediately (like a grammar concept) — the glyph itself is introduced on
    /// the next call.
    fn final_form_gated(&self, g: GlyphCard, now: i64) -> rusqlite::Result<StudyItem> {
        if g.glyph.chars().next().is_some_and(is_final_form)
            && !self.concept_seen(FINAL_FORMS_CONCEPT)?
        {
            self.conn().execute(
                "INSERT INTO progress.concepts_seen(concept, introduced_epoch) VALUES (?1, ?2) \
                 ON CONFLICT(concept) DO NOTHING",
                params![FINAL_FORMS_CONCEPT, now],
            )?;
            return Ok(StudyItem::ExplainFinalForms(g));
        }
        Ok(StudyItem::NewGlyph(g))
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
                    voiced: host
                        .as_deref()
                        .map(|h| crate::romanize::voiced_syllable(h, c))
                        .unwrap_or_default(),
                    voiced_distractors: distractors
                        .iter()
                        .map(|s| crate::romanize::voiced_syllable_str(s))
                        .collect(),
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
                    distractors,
                    ..Default::default()
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
               AND glyph NOT IN ('א','ע','וּ') ORDER BY RANDOM() LIMIT 1"
        };
        match self.conn().query_row(sql, [], |r| r.get(0)).optional()? {
            Some(h) => Ok(Some(h)),
            None => self.known_vowel_host("", vowel),
        }
    }

    /// Not-yet-introduced glyphs that pass `keep`, in roughly the order the
    /// curriculum will introduce them — decomposed from the most frequent
    /// surfaces still containing an unseen glyph, the same ordering as
    /// [`Self::alphabet_frontier_glyph`]. This is the *upcoming* pool that tops
    /// distractors up while too few peers have been introduced to quiz against.
    fn upcoming_glyphs(
        &self,
        want: usize,
        keep: impl Fn(&str) -> bool,
    ) -> rusqlite::Result<Vec<String>> {
        self.ensure_surface_meta()?;
        let seen = self.seen_glyph_mask()?;
        let mut out: Vec<String> = Vec::new();
        let mut stmt = self.conn().prepare(
            "SELECT s.text FROM hebrewdb.surface s \
             JOIN progress.surface_meta sm ON sm.surface_id = s.surface_id \
             WHERE popcount(sm.glyph_mask & ~?1) > 0 \
               AND COALESCE(s.language, '') <> 'aramaic' \
             ORDER BY s.occurrences DESC LIMIT 60",
        )?;
        let rows = stmt.query_map(params![seen], |r| r.get::<_, String>(0))?;
        'rows: for row in rows {
            for g in decompose_glyphs(&row?) {
                if out.len() >= want {
                    break 'rows;
                }
                if !keep(&g.glyph) || out.contains(&g.glyph) || self.glyph_known(&g.glyph)? {
                    continue;
                }
                out.push(g.glyph);
            }
        }
        Ok(out)
    }

    /// Up to `WANT` random nonsense syllables built from already-known *audible*
    /// consonants and vowels, each a two-char `"<consonant><vowel>"` string, as
    /// wrong answers for a vowel's multiple-choice reading quiz. Silent hosts
    /// (aleph/ayin) are excluded so every option is a full syllable; a hataf
    /// vowel is only paired with an audible guttural (ה/ח); the exact
    /// `host`+`vowel` combo is excluded. The app transliterates and dedups, so a
    /// few extra are returned for margin. Early on, when too few glyphs are
    /// known to fill the pool, syllables built from *upcoming* glyphs (see
    /// [`Self::upcoming_glyphs`]) top it up.
    fn syllable_distractors(&self, host: &str, vowel: char) -> rusqlite::Result<Vec<String>> {
        const WANT: usize = 6;
        let mut out = Vec::new();
        // c is a known audible consonant (aleph/ayin excluded, as is the shureq
        // glyph 'וּ' — a vowel that only *leads* with a vav codepoint). v is a
        // proper vowel point (sheva..holam=1456..1465, qubuts=1467,
        // qamats-qatan=1479) — never a dagesh/sin-shin dot/mark that may also be
        // in glyph_srs. A hataf (1457..1459) is only paired with an audible
        // guttural (ה/ח).
        let mut stmt = self.conn().prepare(
            "SELECT c.glyph || v.glyph \
             FROM progress.glyph_srs c \
             JOIN progress.glyph_srs v \
             WHERE unicode(c.glyph) BETWEEN 1488 AND 1514 \
               AND c.glyph NOT IN ('א','ע','וּ') \
               AND (unicode(v.glyph) BETWEEN 1456 AND 1465 \
                    OR unicode(v.glyph) IN (1467, 1479)) \
               AND NOT (unicode(v.glyph) BETWEEN 1457 AND 1459 \
                        AND c.glyph NOT IN ('ה','ח')) \
               AND NOT (c.glyph = ?1 AND v.glyph = ?2) \
             ORDER BY RANDOM() LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![host, vowel.to_string(), WANT as i64], |r| {
            r.get::<_, String>(0)
        })?;
        for row in rows {
            out.push(row?);
        }
        if out.len() >= WANT {
            return Ok(out);
        }

        // A fresh start: too few glyphs are known to build enough syllables
        // from them alone. Extend the pools with upcoming glyphs — the
        // known-glyph syllables already in `out` stay first, so familiar
        // material is still preferred.
        // The shureq glyph 'וּ' leads with a consonant codepoint but is a vowel,
        // never a syllable host.
        let audible =
            |g: &str| g.chars().next().is_some_and(is_consonant) && !is_silent_host(g) && g != "וּ";
        let proper_vowel = |g: &str| {
            let mut cs = g.chars();
            cs.next().is_some_and(is_vowel_point) && cs.next().is_none()
        };
        let mut cons: Vec<String> = Vec::new();
        let mut vows: Vec<String> = Vec::new();
        {
            let mut stmt = self
                .conn()
                .prepare("SELECT glyph FROM progress.glyph_srs")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            for row in rows {
                let g = row?;
                if audible(&g) {
                    cons.push(g);
                } else if proper_vowel(&g) {
                    vows.push(g);
                }
            }
        }
        for g in self.upcoming_glyphs(12, |g| audible(g) || proper_vowel(g))? {
            if audible(&g) {
                cons.push(g);
            } else {
                vows.push(g);
            }
        }
        // Walk the consonant×vowel grid diagonally so consecutive fills vary
        // both components rather than repeating one vowel across hosts.
        for offset in 0..vows.len() {
            for (ci, c) in cons.iter().enumerate() {
                if out.len() >= WANT {
                    return Ok(out);
                }
                let v = &vows[(ci + offset) % vows.len()];
                let vc = v.chars().next().expect("vowel glyph nonempty");
                if is_hataf(vc) && !AUDIBLE_GUTTURALS.contains(&c.as_str()) {
                    continue;
                }
                if c == host && vc == vowel {
                    continue;
                }
                let syl = format!("{c}{v}");
                if !out.contains(&syl) {
                    out.push(syl);
                }
            }
        }
        Ok(out)
    }

    /// Up to three other glyphs of the *same kind* (consonant / vowel point /
    /// reading mark) as `glyph`, for a multiple-choice quiz. Already-introduced
    /// glyphs first (most recent first — familiar, so genuinely confusable),
    /// topped up with *upcoming* glyphs while too few peers have been
    /// introduced (the first letters of a fresh start). The app shuffles.
    fn glyph_distractors(&self, glyph: &str) -> rusqlite::Result<Vec<String>> {
        const WANT: usize = 3;
        let Some(ch) = glyph.chars().next() else {
            return Ok(Vec::new());
        };
        let cons = is_consonant(ch);
        let vowel = is_vowel_point(ch);
        let same_kind = |g: &str| {
            let Some(gc) = g.chars().next() else {
                return false;
            };
            if cons {
                is_consonant(gc)
            } else if vowel {
                is_vowel_point(gc)
            } else {
                !is_consonant(gc) && !is_vowel_point(gc)
            }
        };
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
            if same_kind(&g) {
                out.push(g);
            }
        }
        if out.len() < WANT {
            // `upcoming_glyphs` skips introduced glyphs, so nothing here can
            // duplicate `out` or the reviewed `glyph` itself.
            out.extend(self.upcoming_glyphs(WANT - out.len(), same_kind)?);
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
            // Prefer a curated gloss (as the correct answer does), else the
            // candidate's own form-level rendering — the correct answer is
            // form-specific ("and to the house"), so lexeme-sense options
            // ("son") would give it away by register alone.
            let (g, curated) = match crate::vocab_gloss::curated_gloss(&cand) {
                Some(c) => (c.gloss.trim().to_string(), true),
                None => match self.hebrew_word_info(&cand) {
                    // A name's bridged gloss is an etymology ("my father is
                    // rescue"), not a meaning — never offer one.
                    Some(w) if w.is_name => continue,
                    Some(w) => (crate::bible::inflected_gloss(&w).trim().to_string(), false),
                    None => continue,
                },
            };
            // BDB name citations ("n.pr.m. father of one of David's men")
            // aren't meanings — never offer one as a wrong answer.
            if g.is_empty() || crate::bible::is_name_gloss(&g) {
                continue;
            }
            // Options render like the answer does: curated glosses verbatim,
            // bridged ones trimmed to one sense.
            let g = if curated {
                g
            } else {
                crate::bible::leading_sense(&g)
            };
            if seen.insert(g.to_lowercase()) {
                out.push(g);
            }
        }
        Ok(out)
    }

    // --- card builders -------------------------------------------------------

    /// Store one admin bug report or idea in the synchronised progress
    /// database. The opaque JSON object keeps app/card-specific diagnostics
    /// extensible without requiring a database migration for every new field.
    pub fn save_issue_report(
        &self,
        id: &str,
        report_type: &str,
        note: &str,
        context_json: &str,
        created_epoch: i64,
        updated_epoch: i64,
    ) -> rusqlite::Result<()> {
        let id = id.trim();
        let report_type = report_type.trim();
        let note = note.trim();
        let context_json = context_json.trim();
        let valid_context = serde_json::from_str::<serde_json::Value>(context_json)
            .is_ok_and(|value| value.is_object());
        if id.is_empty()
            || !matches!(report_type, "bug" | "idea")
            || note.is_empty()
            || !valid_context
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "issue id, type, note, and JSON object context must be valid".to_string(),
            ));
        }
        self.conn().execute(
            "INSERT INTO progress.issue_reports(
                 id, report_type, note, context_json, created_epoch, updated_epoch, deleted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)
             ON CONFLICT(id) DO UPDATE SET
                report_type=excluded.report_type, note=excluded.note,
                context_json=excluded.context_json,
                created_epoch=MIN(
                    excluded.created_epoch,
                    progress.issue_reports.created_epoch
                ),
                updated_epoch=MAX(
                    excluded.updated_epoch,
                    progress.issue_reports.updated_epoch + 1
                ),
                deleted=0",
            params![
                id,
                report_type,
                note,
                context_json,
                created_epoch,
                updated_epoch
            ],
        )?;
        Ok(())
    }

    /// Store a tutor-only learner gloss correction. It deliberately does not
    /// alter the bundled lexical data: review it later with `haqor-admin pull`
    /// before committing it to `lexicon_overrides.json`.
    pub fn set_tutor_gloss_override(
        &self,
        surface: &str,
        gloss: &str,
        note: &str,
        updated_epoch: i64,
    ) -> rusqlite::Result<()> {
        let surface = surface.trim();
        let gloss = gloss.trim();
        if surface.is_empty() || gloss.is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "surface and gloss must not be empty".to_string(),
            ));
        }
        self.conn().execute(
            "INSERT INTO progress.gloss_overrides(
                 surface, gloss, note, updated_epoch, deleted)
             VALUES (?1, ?2, ?3, ?4, 0)
             ON CONFLICT(surface) DO UPDATE SET
                gloss=excluded.gloss, note=excluded.note,
                updated_epoch=MAX(
                    excluded.updated_epoch,
                    progress.gloss_overrides.updated_epoch + 1
                ),
                deleted=0",
            params![surface, gloss, note.trim(), updated_epoch],
        )?;
        Ok(())
    }

    /// Store a mobile correction for the word-info root and header gloss.
    pub fn set_lexicon_entry_override(
        &self,
        surface: &str,
        root: &str,
        gloss: &str,
        updated_epoch: i64,
    ) -> rusqlite::Result<()> {
        let surface = surface.trim();
        let root = root.trim();
        let gloss = gloss.trim();
        if surface.is_empty() || gloss.is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "surface and gloss must not be empty".to_string(),
            ));
        }
        self.conn().execute(
            "INSERT INTO progress.lexicon_entry_overrides(
                 surface, root, gloss, updated_epoch)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(surface) DO UPDATE SET
                root=excluded.root, gloss=excluded.gloss,
                updated_epoch=MAX(
                    excluded.updated_epoch,
                    progress.lexicon_entry_overrides.updated_epoch + 1
                )",
            params![surface, root, gloss, updated_epoch],
        )?;
        self.cache_runtime_lexicon_entry(surface, root, gloss);
        Ok(())
    }

    /// Return the active mobile word-info correction for this exact surface.
    pub fn lexicon_entry_override(
        &self,
        surface: &str,
    ) -> rusqlite::Result<Option<(String, String)>> {
        Ok(self.runtime_lexicon_entry(surface))
    }

    fn tutor_gloss_override(&self, surface: &str) -> rusqlite::Result<Option<(String, String)>> {
        self.conn()
            .query_row(
                "SELECT gloss, note FROM progress.gloss_overrides
                 WHERE surface = ?1 AND deleted = 0",
                params![surface],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
    }

    fn redundant_tutor_gloss_override_surfaces(&self) -> rusqlite::Result<Vec<String>> {
        let overrides = {
            let mut statement = self.conn().prepare(
                "SELECT surface, gloss, note FROM progress.gloss_overrides
                 WHERE deleted = 0 ORDER BY surface",
            )?;
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };

        let mut redundant = Vec::new();
        for (surface, gloss, note) in overrides {
            let Some(card) = self.word_card_with_tutor_override(&surface, false)? else {
                continue;
            };
            if card.gloss == gloss && card.root_gloss.is_empty() && card.note == note {
                redundant.push(surface);
            }
        }
        Ok(redundant)
    }

    /// Count active local learner corrections and those now fully represented
    /// by the bundled lexical data.
    pub fn tutor_gloss_override_stats(&self) -> rusqlite::Result<GlossOverrideStats> {
        let total = self.conn().query_row(
            "SELECT COUNT(*) FROM progress.gloss_overrides WHERE deleted = 0",
            [],
            |row| row.get(0),
        )?;
        let redundant = self.redundant_tutor_gloss_override_surfaces()?.len() as i64;
        Ok(GlossOverrideStats { total, redundant })
    }

    /// Hide corrections that no longer change the generated word card. Rows
    /// become timestamped tombstones so the deletion converges through normal
    /// progress sync instead of being resurrected by the server.
    pub fn optimize_tutor_gloss_overrides(
        &self,
        updated_epoch: i64,
    ) -> rusqlite::Result<GlossOverrideOptimization> {
        let redundant = self.redundant_tutor_gloss_override_surfaces()?;
        let transaction = self.conn().unchecked_transaction()?;
        for surface in &redundant {
            transaction.execute(
                "UPDATE progress.gloss_overrides
                 SET gloss = '', note = '',
                     updated_epoch = MAX(updated_epoch + 1, ?2), deleted = 1
                 WHERE surface = ?1 AND deleted = 0",
                params![surface, updated_epoch],
            )?;
        }
        transaction.commit()?;
        let stats = self.tutor_gloss_override_stats()?;
        Ok(GlossOverrideOptimization {
            removed: redundant.len() as i64,
            stats,
        })
    }

    /// Build a meaning word card for `surface`, resolving gloss/root/morph.
    fn word_card(&self, surface: &str) -> rusqlite::Result<Option<WordCard>> {
        self.word_card_with_tutor_override(surface, true)
    }

    fn word_card_with_tutor_override(
        &self,
        surface: &str,
        apply_tutor_override: bool,
    ) -> rusqlite::Result<Option<WordCard>> {
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
        // The cached name classification (see `ensure_surface_meta`) knows
        // signals the resolver alone can't see — the pre-filter's `proper`
        // class catches names BDB never resolves (עִדּוֹא, which would
        // otherwise card with a blank gloss) or resolves without a `pos`
        // marker (אֱלִישָׁמָע "God has heard"). Absent row (the cache hasn't
        // been built, e.g. in isolated tests) just means no extra signal.
        let meta_name: bool = self
            .conn()
            .query_row(
                "SELECT is_name FROM progress.surface_meta WHERE surface_id = ?1",
                params![surface_id],
                |r| Ok(r.get::<_, i64>(0)? != 0),
            )
            .optional()?
            .unwrap_or(false);

        let (mut root, gloss, inflected, mut morph, resolved_name) =
            match self.hebrew_word_info(surface) {
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
                    let inflected = crate::bible::inflected_gloss(&w);
                    (w.root, w.gloss, inflected, morph, w.is_name)
                }
                None => (
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    false,
                ),
            };

        // A curated gloss (held in the core) is the final learner meaning where
        // one exists — it overrides the automatic bridge and supplies the
        // composition note. Its gloss is already form-specific, so no separate
        // root-meaning line is shown. A proclitic-prefixed curated name composes
        // the same way ("to Jacob" — the bridge would serve the homograph root
        // "to heel"). An uncurated proper name (a BDB `n.pr` citation) has no
        // meaning to learn: it cards as [`NAME_GLOSS`] with the citation kept
        // as the note, and drops the spurious bridged root/morph — the
        // scheduler seeds such cards known after one showing.
        let (gloss, root_gloss, note, curated) = match crate::vocab_gloss::curated_gloss(surface) {
            Some(c) => (
                c.gloss.to_string(),
                String::new(),
                c.note.unwrap_or_default().to_string(),
                true,
            ),
            None => match crate::bible::prefixed_name_gloss(surface) {
                Some((g, note)) => (g, String::new(), note, false),
                // A curated famous name is exempt from the pos/meta signals:
                // it is in `CURATED_NAMES` precisely because its BDB gloss is
                // already usable ("Aaron", "Esau") — serve that, not
                // "(a name)". A gloss that embeds the raw BDB citation still
                // falls through (it is not a meaning for anyone).
                None if (!crate::vocab_gloss::curated_name(surface)
                    && (meta_name || resolved_name))
                    || crate::bible::is_name_gloss(&gloss) =>
                {
                    let note = crate::bible::name_description(&gloss);
                    root.clear();
                    morph.clear();
                    (NAME_GLOSS.to_string(), String::new(), note, false)
                }
                // The surface's own meaning is what the learner is reading, so
                // the inflected rendering headlines (and is quizzed); the
                // lexeme sense demotes to a "root meaning" line.
                None if !inflected.is_empty()
                    && inflected.to_lowercase() != gloss.to_lowercase() =>
                {
                    (inflected, gloss, String::new(), false)
                }
                None => (gloss, String::new(), String::new(), false),
            },
        };

        // A bridged card headlines a single sense — the full multi-sense
        // entry belongs to the lexicon view, not a quiz answer ("who", not
        // "who; which; that"). A curated gloss is served verbatim: it was
        // hand-written as the final learner meaning (כִּי is "for, because,
        // that, when", not "for"), and its card has no root-meaning line to
        // carry the trimmed senses.
        let gloss = if curated {
            gloss
        } else {
            crate::bible::leading_sense(&gloss)
        };

        // A mobile admin correction is the last learner-facing layer. Keep it
        // separate from the generated lexicon until it has been reviewed and
        // pulled back into the JSON overlay.
        let tutor_override = if apply_tutor_override {
            self.tutor_gloss_override(surface)?
        } else {
            None
        };
        let (gloss, root_gloss, note) = match tutor_override {
            Some((gloss, note)) => (gloss, String::new(), note),
            None => (gloss, root_gloss, note),
        };

        // A name card is reveal-and-self-grade — quizzing "(a name)" against
        // real glosses is a giveaway, so it gets no distractors.
        let distractors = if gloss == NAME_GLOSS {
            Vec::new()
        } else {
            self.meaning_distractors(surface, &gloss)?
        };

        Ok(Some(WordCard {
            surface_id,
            surface: surface.to_string(),
            occurrences,
            translit: crate::romanize::romanize(surface),
            gloss,
            root_gloss,
            note,
            root,
            morph,
            distractors,
        }))
    }

    /// Build a form-drill card for `surface`: the quiz answer is the form's
    /// inflected gloss ("and he said"), the distractors are other inflections of
    /// the same word ("and she said", "and they said"). Falls back to
    /// reveal-and-self-grade when too few contrasting forms exist. Returns `None`
    /// if the surface has no usable parse.
    fn form_card(&self, surface: &str) -> rusqlite::Result<Option<WordCard>> {
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
        let Some(w) = self.hebrew_word_info(surface) else {
            return Ok(None);
        };
        let inflected = crate::bible::inflected_gloss(&w);
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
        // The quiz answer is the inflected form; contrasting inflections are the
        // wrong options. `gloss` carries the answer (as the meaning quiz does),
        // `root_gloss` is cleared so the app shows no redundant line.
        Ok(Some(WordCard {
            surface_id,
            surface: surface.to_string(),
            occurrences,
            translit: crate::romanize::romanize(surface),
            gloss: inflected,
            root_gloss: String::new(),
            note: String::new(),
            root: w.root.clone(),
            morph,
            distractors: crate::bible::form_distractors(&w),
        }))
    }

    // --- selection -----------------------------------------------------------

    /// Populate the `surface_meta` cache — one `(primary root, form tier)` per
    /// non-Aramaic surface, resolved through [`Bible::hebrew_word_info`] — used to
    /// order the curriculum by root frequency, root familiarity and form
    /// simplicity. Idempotent and guarded by a version stamp in `meta`, so it runs
    /// once (a corpus-wide pass over ~50k surfaces) and is a cheap no-op
    /// thereafter; a bump to [`SURFACE_META_VERSION`] (or a swapped-in newer
    /// `hebrew.db`) triggers a one-time rebuild.
    fn ensure_surface_meta(&self) -> rusqlite::Result<()> {
        let stamp: Option<i64> = self
            .conn()
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM progress.meta WHERE key = 'surface_meta_v'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let expected = self.surface_count()?;
        // Rebuild when the version changed or the row count no longer matches the
        // corpus (a swapped-in db): a partial/stale cache would mis-order words.
        let current: i64 =
            self.conn()
                .query_row("SELECT COUNT(*) FROM progress.surface_meta", [], |r| {
                    r.get(0)
                })?;
        if stamp == Some(SURFACE_META_VERSION) && current == expected {
            return Ok(());
        }

        let surfaces: Vec<(i64, String, Option<String>)> = {
            let mut stmt = self.conn().prepare(
                "SELECT surface_id, text, lexical_class FROM hebrewdb.surface \
                 WHERE language IS NULL",
            )?;
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<rusqlite::Result<_>>()?
        };

        self.conn().execute_batch("BEGIN")?;
        self.conn()
            .execute("DELETE FROM progress.surface_meta", [])?;
        let mut concept_masks: std::collections::HashMap<i64, i64> =
            std::collections::HashMap::new();
        {
            let mut ins = self.conn().prepare(
                "INSERT INTO progress.surface_meta(surface_id, root, form_tier, concept_mask, \
                    glyph_mask, is_name, is_qal, family_base, vkey, base_vkey, base_surface_id) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9, ?1)",
            )?;
            for (surface_id, text, lexical_class) in surfaces {
                // `surface.text` is already the normalised form the resolver uses.
                let mask = surface_glyph_mask(&text);
                let parsed = self.hebrew_word_by_surface_id(surface_id, text.clone());
                let cmask = crate::grammar::concept_mask_for_surface(&text, parsed.as_ref());
                let lexicalized_verb_tier = lexicalized_verb_tier(&text);
                let (root, tier) = match &parsed {
                    Some(w) => (
                        w.root.clone(),
                        lexicalized_verb_tier.unwrap_or_else(|| form_tier(w)) as i64,
                    ),
                    None => (String::new(), 0),
                };
                let is_qal = lexicalized_verb_tier.is_some()
                    || parsed
                        .as_ref()
                        .and_then(|w| w.form.as_deref())
                        .is_some_and(|form| form == "Qal" || form == "Qal passive");
                // A proper name, by any of the available signals: the noun
                // bridge flags lexemes BDB marks `n.pr*` / `adj.gent` in the
                // `pos` column (`w.is_name` — most name entries carry a bare
                // etymology gloss like "God hides", so the marker never
                // reaches the gloss text); glosses that *do* embed the
                // citation (the build-time bridge table) are caught by the
                // text sniff; proclitic-prefixed curated names (לְיַעֲקֹב)
                // classify through the same composition the card builder uses;
                // and the lexical pre-filter's `proper` class covers the names
                // BDB never resolves (עִדּוֹא cards blank) or whose entry
                // carries no `pos` at all (אֱלִישָׁמָע "God has heard") —
                // vetoed when the surface exactly heads a real vocabulary
                // article (זָהָב is in the proper list via Di-zahab but *is*
                // "gold"). A curated gloss overrides everything, exactly as it
                // does on the card: curated names are names (מֹשֶׁה), curated
                // vocabulary is not (בְּנֵי "sons of" bridges to BDB's *Bani*).
                let is_name = if crate::vocab_gloss::curated_name(&text) {
                    true
                } else if crate::vocab_gloss::curated_gloss(&text).is_some() {
                    false
                } else {
                    parsed
                        .as_ref()
                        .is_some_and(|w| w.is_name || crate::bible::is_name_gloss(&w.gloss))
                        || crate::bible::prefixed_name_gloss(&text).is_some()
                        || (lexical_class.as_deref() == Some("proper")
                            && !self.bdb_exact_vocab_match(
                                &text,
                                parsed.as_ref().and_then(|w| w.prefix.as_deref()),
                            ))
                };
                // A word whose card would show a blank gloss (no parse sense,
                // no curated gloss, not a name) is marked unteachable rather
                // than fed to the learner ranked by raw frequency.
                let blank = parsed.as_ref().is_none_or(|w| w.gloss.trim().is_empty())
                    && crate::vocab_gloss::curated_gloss(&text).is_none()
                    && crate::bible::prefixed_name_gloss(&text).is_none()
                    && !is_name;
                let cmask = if blank { UNTEACHABLE_MASK } else { cmask };
                concept_masks.insert(surface_id, cmask);
                let vkey = crate::vocab_gloss::vocab_key(&text);
                let base_vkey = parsed
                    .as_ref()
                    .and_then(|w| w.prefix.as_deref())
                    .and_then(|prefix| crate::bible::strip_proclitic(&text, prefix))
                    .map(|base| crate::vocab_gloss::vocab_key(&base))
                    .unwrap_or_else(|| vkey.clone());
                ins.execute(params![
                    surface_id,
                    root,
                    tier,
                    cmask,
                    mask,
                    is_name as i64,
                    is_qal as i64,
                    vkey,
                    base_vkey
                ])?;
            }
        }
        // A mechanically stripped spelling is only a separate learning step
        // when that bare form is actually attested. Normalising unattested
        // bases back to the surface's own key makes the hot selection query a
        // simple indexed join rather than a correlated existence check.
        self.conn().execute(
            "UPDATE progress.surface_meta AS sm SET base_vkey = vkey \
             WHERE base_vkey <> vkey AND NOT EXISTS ( \
                 SELECT 1 FROM progress.surface_meta base \
                 WHERE base.vkey = sm.base_vkey)",
            [],
        )?;
        self.conn().execute(
            "UPDATE progress.surface_meta AS sm SET base_surface_id = ( \
                 SELECT MIN(base.surface_id) FROM progress.surface_meta base \
                 WHERE base.vkey = sm.base_vkey)",
            [],
        )?;
        // For a verbal family with an attested Qal, only its simplest Qal tier
        // is initially eligible. Without an attested Qal, the simplest form in
        // the family becomes the base instead.
        self.conn().execute(
            "UPDATE progress.surface_meta AS sm SET family_base = CASE \
               WHEN sm.root = '' THEN 1 \
               WHEN EXISTS (SELECT 1 FROM progress.surface_meta q \
                            WHERE q.root = sm.root AND q.is_qal = 1) THEN \
                    CASE WHEN sm.is_qal = 1 AND sm.form_tier = \
                    (SELECT MIN(q.form_tier) FROM progress.surface_meta q \
                     WHERE q.root = sm.root AND q.is_qal = 1) THEN 1 ELSE 0 END \
               WHEN sm.form_tier = (SELECT MIN(q.form_tier) FROM progress.surface_meta q \
                                    WHERE q.root = sm.root) THEN 1 ELSE 0 END",
            [],
        )?;
        // Per-verse concept masks (the OR of every word's mask), for the
        // unlock chooser: "how many verses would concept X newly complete?"
        // is a static corpus property, so it is precomputed here. Verses with
        // an Aramaic word (whose surfaces carry no meta row) are skipped —
        // they are never study targets.
        self.conn().execute("DELETE FROM progress.verse_meta", [])?;
        {
            let verse_masks = {
                let mut stmt = self.conn().prepare(
                    "SELECT vw.book, vw.chapter, vw.verse, vw.surface_id \
                     FROM hebrewdb.verse_word vw ORDER BY vw.book, vw.chapter, vw.verse",
                )?;
                let mut acc: std::collections::HashMap<(u8, u8, u8), Option<i64>> =
                    std::collections::HashMap::new();
                let rows = stmt.query_map([], |r| {
                    Ok((
                        r.get::<_, u8>(0)?,
                        r.get::<_, u8>(1)?,
                        r.get::<_, u8>(2)?,
                        r.get::<_, i64>(3)?,
                    ))
                })?;
                for row in rows {
                    let (b, c, v, sid) = row?;
                    let entry = acc.entry((b, c, v)).or_insert(Some(0));
                    *entry = match (entry.as_ref(), concept_masks.get(&sid)) {
                        (Some(m), Some(w)) => Some(m | w),
                        _ => None, // an Aramaic word poisons the verse
                    };
                }
                acc
            };
            let mut ins = self.conn().prepare(
                "INSERT INTO progress.verse_meta(book, chapter, verse, concept_mask) \
                 VALUES (?1, ?2, ?3, ?4)",
            )?;
            for ((b, c, v), mask) in verse_masks {
                if let Some(m) = mask {
                    ins.execute(params![b, c, v, m])?;
                }
            }
        }
        self.conn().execute(
            "INSERT INTO progress.meta(key, value) VALUES ('surface_meta_v', ?1) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![SURFACE_META_VERSION.to_string()],
        )?;
        self.conn().execute_batch("COMMIT")?;
        Ok(())
    }

    /// Materialise learner readability in `progress.db`. One row exists for
    /// every Hebrew surface and every Scripture verse; this makes coverage a
    /// concrete property of graduated vocabulary rather than a read-history
    /// or a heuristic baked into the generated database.
    fn ensure_readability_progress(&self) -> rusqlite::Result<()> {
        let stamp: Option<i64> = self
            .conn()
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM progress.meta
                 WHERE key = 'readability_progress_v'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let expected: i64 =
            self.conn()
                .query_row("SELECT COUNT(*) FROM hebrewdb.surface", [], |r| r.get(0))?;
        let actual: i64 =
            self.conn()
                .query_row("SELECT COUNT(*) FROM progress.surface_progress", [], |r| {
                    r.get(0)
                })?;
        let materialized_done: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM progress.surface_progress WHERE graduated = 1",
            [],
            |r| r.get(0),
        )?;
        let actual_done: i64 = self.conn().query_row(
            &format!(
                "SELECT COUNT(*) FROM progress.surface_meta sm JOIN ({DONE_SURFACES}) done ON done.vkey = sm.vkey"
            ),
            [],
            |r| r.get(0),
        )?;
        if stamp == Some(READABILITY_PROGRESS_VERSION)
            && actual == expected
            && materialized_done == actual_done
        {
            return Ok(());
        }
        self.rebuild_readability_progress()
    }

    fn rebuild_readability_progress(&self) -> rusqlite::Result<()> {
        self.conn().execute_batch("BEGIN IMMEDIATE")?;
        let result = self.conn().execute_batch(&format!(
            "DELETE FROM progress.surface_progress;
             INSERT INTO progress.surface_progress(surface_id, graduated, graduated_epoch)
             SELECT s.surface_id, CASE WHEN done.vkey IS NULL THEN 0 ELSE 1 END,
                    CASE WHEN done.vkey IS NULL THEN NULL ELSE 0 END
             FROM hebrewdb.surface s
             LEFT JOIN progress.surface_meta sm ON sm.surface_id = s.surface_id
             LEFT JOIN ({DONE_SURFACES}) done ON done.vkey = sm.vkey
             ;
             DELETE FROM progress.verse_progress;
             INSERT INTO progress.verse_progress(
                book, chapter, verse, unknown_words, min_root_frequency,
                mean_root_frequency, max_root_frequency)
             SELECT vw.book, vw.chapter, vw.verse,
                    COUNT(DISTINCT CASE WHEN sp.graduated = 0 THEN
                       COALESCE(sm.vkey, '#' || vw.surface_id) END),
                    COALESCE(MIN(CASE WHEN sp.graduated = 0 THEN
                       CASE WHEN sm.is_name = 1 THEN s.occurrences
                            ELSE COALESCE(r.n_occurrences, s.occurrences) END END), 0),
                    COALESCE(AVG(CASE WHEN sp.graduated = 0 THEN
                       CASE WHEN sm.is_name = 1 THEN s.occurrences
                            ELSE COALESCE(r.n_occurrences, s.occurrences) END END), 0),
                    COALESCE(MAX(CASE WHEN sp.graduated = 0 THEN
                       CASE WHEN sm.is_name = 1 THEN s.occurrences
                            ELSE COALESCE(r.n_occurrences, s.occurrences) END END), 0)
             FROM hebrewdb.verse_word vw
             JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
             LEFT JOIN progress.surface_meta sm ON sm.surface_id = vw.surface_id
             JOIN progress.surface_progress sp ON sp.surface_id = vw.surface_id
             LEFT JOIN hebrewdb.roots r ON r.root = sm.root
             GROUP BY vw.book, vw.chapter, vw.verse;
             INSERT INTO progress.meta(key, value)
             VALUES ('readability_progress_v', '{READABILITY_PROGRESS_VERSION}')
             ON CONFLICT(key) DO UPDATE SET value = excluded.value;"
        ));
        match result {
            Ok(()) => self.conn().execute_batch("COMMIT"),
            Err(e) => {
                let _ = self.conn().execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    fn record_surface_graduation(&self, surface_id: i64, now: i64) -> rusqlite::Result<()> {
        let changed = self.conn().execute(
            "UPDATE progress.surface_progress SET graduated = 1, graduated_epoch = ?2
             WHERE surface_id IN (
               SELECT twin.surface_id FROM progress.surface_meta source
               JOIN progress.surface_meta twin ON twin.vkey = source.vkey
               WHERE source.surface_id = ?1) AND graduated = 0",
            params![surface_id, now],
        )?;
        if changed > 0 {
            // Recalculate only verses containing this vocabulary key. This is
            // synchronous with graduation, but avoids rescanning Scripture on
            // every successful card.
            self.conn().execute_batch(&format!(
                "INSERT OR REPLACE INTO progress.verse_progress(
                    book, chapter, verse, unknown_words, min_root_frequency,
                    mean_root_frequency, max_root_frequency, last_read_epoch)
                 SELECT vw.book, vw.chapter, vw.verse,
                        COUNT(DISTINCT CASE WHEN sp.graduated = 0 THEN
                           COALESCE(sm.vkey, '#' || vw.surface_id) END),
                        COALESCE(MIN(CASE WHEN sp.graduated = 0 THEN
                           CASE WHEN sm.is_name = 1 THEN s.occurrences
                                ELSE COALESCE(r.n_occurrences, s.occurrences) END END), 0),
                        COALESCE(AVG(CASE WHEN sp.graduated = 0 THEN
                           CASE WHEN sm.is_name = 1 THEN s.occurrences
                                ELSE COALESCE(r.n_occurrences, s.occurrences) END END), 0),
                        COALESCE(MAX(CASE WHEN sp.graduated = 0 THEN
                           CASE WHEN sm.is_name = 1 THEN s.occurrences
                                ELSE COALESCE(r.n_occurrences, s.occurrences) END END), 0),
                        COALESCE(old.last_read_epoch, 0)
                 FROM hebrewdb.verse_word vw
                 JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
                 JOIN progress.surface_meta sm ON sm.surface_id = vw.surface_id
                 JOIN progress.surface_progress sp ON sp.surface_id = vw.surface_id
                 LEFT JOIN hebrewdb.roots r ON r.root = sm.root
                 LEFT JOIN progress.verse_progress old ON old.book = vw.book
                    AND old.chapter = vw.chapter AND old.verse = vw.verse
                 WHERE (vw.book, vw.chapter, vw.verse) IN (
                    SELECT DISTINCT hit.book, hit.chapter, hit.verse
                    FROM hebrewdb.verse_word hit
                    JOIN progress.surface_meta hit_sm ON hit_sm.surface_id = hit.surface_id
                    WHERE hit_sm.vkey = (
                       SELECT vkey FROM progress.surface_meta WHERE surface_id = {surface_id}))
                 GROUP BY vw.book, vw.chapter, vw.verse;"
            ))?;
        }
        Ok(())
    }

    fn surface_count(&self) -> rusqlite::Result<i64> {
        self.conn().query_row(
            "SELECT COUNT(*) FROM hebrewdb.surface WHERE language IS NULL",
            [],
            |r| r.get(0),
        )
    }

    /// The next verse to work toward.
    ///
    /// In normal mode every unknown word must be inside the paced grammar and
    /// root-family frontiers. A verse with an unknown unteachable word (blank card,
    /// [`UNTEACHABLE_MASK`]) can never be finished and is excluded outright.
    /// Verses order by their unlock score: rarest unknown word as common
    /// as possible, discounted 4× per locked rule needed — so the tutor
    /// reaches for the fewest unlocks that keep the vocabulary prime — then
    /// by fewest missing rules, fewest new roots, simplest form, fewest new
    /// words.
    ///
    /// In `letter_learning` mode (the alphabet isn't known yet, so only
    /// grammar-free words are introducible) it drops the completability
    /// requirement — a verse may still contain not-yet-teachable grammar words —
    /// and instead rates each verse by its best introducible word under
    /// [`letter_learning_score`]: frequency discounted 4× per new letter. So
    /// the learner meets common words first, absorbing a couple of new letters
    /// at a time, and the curriculum never digs into one-off names just because
    /// they're short and spelled with known letters. Biblical Aramaic verses
    /// excluded.
    fn next_target_verse(
        &self,
        unlocked: i64,
        seen_mask: i64,
        letter_learning: bool,
    ) -> rusqlite::Result<Option<(u8, u8, u8)>> {
        self.next_target_verse_excluding(None, unlocked, seen_mask, letter_learning)
    }

    /// Same as [`Self::next_target_verse`], but skipping `exclude` — used to
    /// find a *second* verse to interleave new material from when the pinned
    /// target has nothing left to introduce right now (see
    /// [`Self::next_study_item`]'s stall fallback).
    fn next_target_verse_excluding(
        &self,
        exclude: Option<(u8, u8, u8)>,
        unlocked: i64,
        seen_mask: i64,
        letter_learning: bool,
    ) -> rusqlite::Result<Option<(u8, u8, u8)>> {
        // The word that qualifies a verse for targeting. In letter-learning
        // mode it must be *introducible right now* — every concept it
        // exercises inside the unlocked mask (?1), and never a name: the
        // letter phase has no completable-verse freebies, so a name card
        // there is pure noise (a pinned royal chronicle used to deal six of
        // them in a row) — and the gate must agree with
        // [`Self::unfinished_words`] or a chosen verse would have nothing to
        // introduce and targeting would spin.
        let intro = if letter_learning {
            format!(
                "sp.graduated = 0 AND (COALESCE(sm.concept_mask, 0) & ~?1) = 0 \
                 AND (COALESCE(sm.concept_mask, 0) & {UNTEACHABLE_MASK}) = 0 \
                 AND {FAMILY_READY} AND {LEXICAL_BASE_READY} AND {NOT_NAME}"
            )
        } else {
            format!(
                "sp.graduated = 0 AND (COALESCE(sm.concept_mask, 0) & ~?1) = 0 \
                 AND (COALESCE(sm.concept_mask, 0) & {UNTEACHABLE_MASK}) = 0 \
                 AND {FAMILY_READY} AND {LEXICAL_BASE_READY}"
            )
        };
        let settings = self.tutor_settings()?;
        let order = if letter_learning {
            // Rate the verse by its single *best* introducible word — the one
            // the tutor will actually teach next ([`letter_learning_score`]) —
            // so verse choice and word choice agree; MAX, not MIN, because an
            // unrelated rare word elsewhere in the verse shouldn't sink the
            // verse carrying the best next word.
            let score = letter_learning_score("?2");
            format!(
                "MAX({score}) DESC, \
                 vw.book, vw.chapter, vw.verse"
            )
        } else {
            // Balance globally useful vocabulary against completing a verse.
            // Cap the frequency contribution at 10,000; inverse unknown count
            // uses a 0..1,000 completion bonus so balanced settings still avoid
            // short name lists while a high verse priority can win deliberately.
            format!(
                "(MIN(MAX({WORD_FREQ}), 10000) * {} \
                  + (1000 / MAX(MAX(vp.unknown_words), 1)) * {}) DESC, \
             MAX({WORD_FREQ}) DESC, \
             MIN(COALESCE(sm.form_tier, 0)) ASC, \
             vw.book, vw.chapter, vw.verse",
                settings.vocab_priority, settings.verse_priority
            )
        };
        // Placeholder numbers for the exclude triple must slot in after
        // whichever of ?1 (unlocked, always present) / ?2 (seen_mask, only
        // while letter-learning) are already bound.
        let exclude_base = if letter_learning { 3 } else { 2 };
        let exclude_where = if exclude.is_some() {
            format!(
                "AND NOT (vw.book = ?{b} AND vw.chapter = ?{c} AND vw.verse = ?{v})",
                b = exclude_base,
                c = exclude_base + 1,
                v = exclude_base + 2
            )
        } else {
            String::new()
        };
        // The exclusion path is specifically the post-lapse interleave: it
        // needs a verse containing genuinely fresh material, not merely a
        // second occurrence of the word that was just failed.
        let (fresh_join, fresh_where) = if exclude.is_some() {
            (
                "LEFT JOIN progress.word_srs current_ws
                   ON current_ws.surface_id = vw.surface_id",
                "AND current_ws.surface_id IS NULL",
            )
        } else {
            ("", "")
        };
        let sql = format!(
            "SELECT vw.book, vw.chapter, vw.verse
             FROM hebrewdb.verse_word vw
             JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
             JOIN progress.surface_meta sm ON sm.surface_id = vw.surface_id
             JOIN progress.surface_progress sp ON sp.surface_id = vw.surface_id
             JOIN progress.verse_meta vm ON vm.book = vw.book
                AND vm.chapter = vw.chapter AND vm.verse = vw.verse
             JOIN progress.verse_progress vp ON vp.book = vw.book
                AND vp.chapter = vw.chapter AND vp.verse = vw.verse
             LEFT JOIN progress.word_srs base_ws ON base_ws.surface_id = sm.base_surface_id
             {fresh_join}
             LEFT JOIN hebrewdb.roots r ON r.root = sm.root
             LEFT JOIN ({KNOWN_QAL_ROOTS}) kqr ON kqr.root = sm.root
             WHERE {intro}
             {fresh_where}
             {exclude_where}
             GROUP BY vw.book, vw.chapter, vw.verse
             HAVING MAX(CASE WHEN {NOT_NAME} THEN 1 ELSE 0 END) = 1
                OR NOT EXISTS (
                    SELECT 1 FROM hebrewdb.verse_word locked_vw
                    JOIN progress.surface_meta locked_sm
                      ON locked_sm.surface_id = locked_vw.surface_id
                    JOIN progress.surface_progress locked_sp
                      ON locked_sp.surface_id = locked_vw.surface_id
                    WHERE locked_vw.book = vw.book
                      AND locked_vw.chapter = vw.chapter
                      AND locked_vw.verse = vw.verse
                      AND locked_sp.graduated = 0
                      AND (COALESCE(locked_sm.concept_mask, 0) & ~?1) != 0)
             ORDER BY {order}
             LIMIT 1"
        );
        // ?1 = unlocked (always); ?2 = seen mask (only referenced when learning);
        // ?3..?5 = excluded book/chapter/verse (only when excluding).
        let mut p: Vec<&dyn rusqlite::ToSql> = vec![&unlocked];
        if letter_learning {
            p.push(&seen_mask);
        }
        let (eb, ec, ev);
        if let Some((b, c, v)) = &exclude {
            eb = *b;
            ec = *c;
            ev = *v;
            p.push(&eb);
            p.push(&ec);
            p.push(&ev);
        }
        self.conn()
            .query_row(&sql, p.as_slice(), |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .optional()
    }

    /// The next not-fully-learnt word in a verse to introduce, if any remain,
    /// in [`word_order`] (known-root, then simplest form, then common root).
    fn first_unfinished_word(&self, b: u8, c: u8, v: u8) -> rusqlite::Result<Option<String>> {
        let word_order = word_order();
        self.conn()
            .query_row(
                &format!(
                    "SELECT s.text
                     FROM hebrewdb.verse_word vw
                     JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
                     LEFT JOIN progress.surface_meta sm ON sm.surface_id = vw.surface_id
                     JOIN progress.surface_progress sp ON sp.surface_id = vw.surface_id
                     LEFT JOIN hebrewdb.roots r ON r.root = sm.root
                     LEFT JOIN ({KNOWN_ROOTS}) kr ON kr.root = sm.root
                     WHERE vw.book = ?1 AND vw.chapter = ?2 AND vw.verse = ?3
                       AND sp.graduated = 0
                     {word_order}
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

    /// The *introducible* not-fully-learnt words of a verse — unknown and not
    /// behind a still-locked grammar rule (`concept_mask` inside `unlocked`) — that
    /// [`Self::next_introduction`] may teach. Unlike [`Self::first_unfinished_word`]
    /// (which sees every unknown word, so verse completion stays honest), a
    /// locked-grammar word is excluded here so it isn't taught before its rule
    /// unlocks, and a proper name only qualifies when `include_names` is set —
    /// reading the pinned verse genuinely needs its names, but a letter-phase
    /// teaching pin gets dropped unread, so dealing its names would be pure
    /// noise. Ordered semantically
    /// ([`word_order`]) normally, or — while learning the alphabet — by
    /// [`letter_learning_score`] (frequency discounted 4× per new letter), so
    /// common words come first and new letters arrive a couple at a time.
    fn unfinished_words(
        &self,
        (b, c, v): (u8, u8, u8),
        unlocked: i64,
        seen_mask: i64,
        letter_learning: bool,
        include_names: bool,
    ) -> rusqlite::Result<Vec<String>> {
        let order = if letter_learning {
            let score = letter_learning_score("?5");
            format!("ORDER BY {score} DESC, s.occurrences DESC")
        } else {
            word_order()
        };
        let name_gate = if include_names {
            String::new()
        } else {
            format!("AND {NOT_NAME}")
        };
        let sql = format!(
            "SELECT s.text
             FROM hebrewdb.verse_word vw
             JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
             LEFT JOIN progress.surface_meta sm ON sm.surface_id = vw.surface_id
             JOIN progress.surface_progress sp ON sp.surface_id = vw.surface_id
             LEFT JOIN progress.word_srs base_ws ON base_ws.surface_id = sm.base_surface_id
             LEFT JOIN hebrewdb.roots r ON r.root = sm.root
             LEFT JOIN ({KNOWN_ROOTS}) kr ON kr.root = sm.root
             LEFT JOIN ({KNOWN_QAL_ROOTS}) kqr ON kqr.root = sm.root
             WHERE vw.book = ?1 AND vw.chapter = ?2 AND vw.verse = ?3
               AND sp.graduated = 0
               AND (COALESCE(sm.concept_mask, 0) & ~?4) = 0
               AND {FAMILY_READY}
               AND {LEXICAL_BASE_READY}
               {name_gate}
             {order}"
        );
        let mut stmt = self.conn().prepare(&sql)?;
        let mut p: Vec<&dyn rusqlite::ToSql> = vec![&b, &c, &v, &unlocked];
        if letter_learning {
            p.push(&seen_mask);
        }
        stmt.query_map(p.as_slice(), |r| r.get(0))?.collect()
    }

    /// Whether the verse can actually be finished and read under the current
    /// grammar frontier — no unknown word behind a still-locked rule. True
    /// for every normal-mode pin once the target's concepts have
    /// run (unless it holds an unteachable word); false for letter-phase
    /// teaching pins, which get dropped unread: a completable pin is being
    /// *read* (its names must be dealt), a teaching pin mustn't deal names.
    fn verse_completable(&self, b: u8, c: u8, v: u8, unlocked: i64) -> rusqlite::Result<bool> {
        let locked: i64 = self.conn().query_row(
            "SELECT COUNT(*)
                 FROM hebrewdb.verse_word vw
                 LEFT JOIN progress.surface_meta sm ON sm.surface_id = vw.surface_id
                 JOIN progress.surface_progress sp ON sp.surface_id = vw.surface_id
                 WHERE vw.book = ?1 AND vw.chapter = ?2 AND vw.verse = ?3
                   AND sp.graduated = 0
                   AND (COALESCE(sm.concept_mask, 0) & ~?4) != 0",
            params![b, c, v, unlocked],
            |r| r.get(0),
        )?;
        Ok(locked == 0)
    }

    /// The next thing to *introduce* (teach) toward the target verse: unseen
    /// glyphs, then — once a word's glyphs are all known (so it can be sounded
    /// out) — that word's meaning. Tries every not-fully-learnt word in the
    /// verse, most common first, rather than stopping at the first one that
    /// happens to already be mid-learning — otherwise the active card pool
    /// never grows past whichever word is currently graduating (e.g. a
    /// frequent word like the divine name, which always sorts first) and the
    /// learner just keeps re-drilling it. Returns None when every word is
    /// either graduated or already fully introduced and mid-learning, or when
    /// the only candidate is a new letter held back by the focus gate (letters
    /// over their [`TutorSettings::letters_ratio`] share while a word is
    /// within reach of the glyphs already open) — either way the remaining
    /// work is graduating cards already in learning (handled by pulling a
    /// learning review forward).
    fn next_introduction(
        &self,
        (b, c, v): (u8, u8, u8),
        now: i64,
        unlocked: i64,
        seen_mask: i64,
        letter_learning: bool,
    ) -> rusqlite::Result<Option<StudyItem>> {
        let settings = self.tutor_settings()?;
        // Pacing budgets: while a batch is full, don't introduce more new
        // glyphs / words — `next_study_item` falls through to pulling an
        // in-learning card forward, consolidating what's already open before
        // adding more. Computed once; both throttle first-exposure only.
        let glyph_budget = self.glyphs_in_learning()? < settings.letters_per_batch as i64;
        let word_budget = self.words_in_learning()? < settings.words_per_batch as i64;
        // Names are dealt (as show-once freebies) only when the pinned verse
        // will actually be read — completable right now — never from a
        // teaching pin that gets dropped unread.
        let include_names = !letter_learning && self.verse_completable(b, c, v, unlocked)?;
        let surfaces = self.unfinished_words(
            (b, c, v),
            unlocked,
            seen_mask,
            letter_learning,
            include_names,
        )?;

        // The word candidate: a word already fully readable (all its glyphs are
        // known *and* graduated, so it can be sounded out) whose meaning isn't
        // learnt yet. Preferring this over introducing another new letter is
        // what stops the curriculum racing ahead through the alphabet while
        // earlier words wait — the learner reads words with the letters they
        // already have. `word_within_reach` additionally notes a word whose
        // glyphs are all *seen* but not yet graduated (or ready but over the
        // words batch): consolidating the in-learning cards would surface it,
        // so a words-forward focus holds new letters back for it (below).
        let mut ready_word: Option<String> = None;
        let mut word_within_reach = false;
        for s in &surfaces {
            if self.word_srs(s)?.is_some() {
                continue;
            }
            if self.all_glyphs_graduated(s)? {
                word_within_reach = true;
                if word_budget && ready_word.is_none() {
                    ready_word = Some(s.clone());
                }
            } else if !word_within_reach && self.all_glyphs_seen(s)? {
                word_within_reach = true;
            }
            if ready_word.is_some() {
                break;
            }
        }
        // The letter candidate: the first unseen glyph of the highest-priority
        // word still needing one (under the letters batch). When letters are
        // under their target share but the target verse has no unseen glyph
        // left, pull the next letter from the whole corpus instead — without
        // this the letters↔words focus can never actually run ahead of the
        // verse at hand (verse selection deliberately minimises new letters,
        // so in-verse candidates alone keep the mix pinned to the corpus
        // order whatever the ratio).
        let mut new_letter: Option<(String, GlyphCard)> = None;
        if glyph_budget {
            'outer: for s in &surfaces {
                for g in decompose_glyphs(s) {
                    if !self.glyph_known(&g.glyph)? {
                        new_letter = Some((s.clone(), g));
                        break 'outer;
                    }
                }
            }
            if new_letter.is_none() && self.prefer_new_letter(&settings)? {
                new_letter = self.alphabet_frontier_glyph(unlocked)?;
            }
        }

        // Choose between them by the letters↔words ratio. A ready word with no
        // letter available is always taken; a letter with no ready word is
        // taken only while letters are within their target share, or while no
        // word is even within reach (so the letter is the only route to one) —
        // otherwise returning None lets `next_study_item` pull an in-learning
        // card forward, graduating the glyphs that surface the reachable word
        // instead of racing ahead through the alphabet. Without that gate the
        // ratio is a dead letter: it would only break the rare same-call tie,
        // and the loser was introduced on the very next slot anyway.
        let do_letter = match (ready_word.is_some(), new_letter.is_some()) {
            (true, true) => self.prefer_new_letter(&settings)?,
            (false, true) => self.prefer_new_letter(&settings)? || !word_within_reach,
            (true, false) => false,
            (false, false) => {
                debug!(
                    "next_introduction: no candidate (glyph_budget={glyph_budget} \
                     word_budget={word_budget})"
                );
                return Ok(None);
            }
        };
        debug!(
            "next_introduction: ready_word={ready_word:?} new_letter={:?} do_letter={do_letter}",
            new_letter.as_ref().map(|(s, g)| (s, &g.glyph))
        );

        if do_letter {
            let (surface, g) = new_letter.expect("letter candidate present");
            self.bump_intro_counter("intro.letters")?;
            return Ok(Some(self.new_glyph_item(&surface, &g, now)?));
        }
        let Some(surface) = ready_word else {
            // A letter was available but held back by the focus gate: hand
            // the turn to consolidation instead of introducing.
            debug!("next_introduction: letter deferred by focus gate");
            return Ok(None);
        };
        // Introduce any unseen grammar concept the word exercises first (one
        // gradeless card, not counted as a word introduction), then its meaning.
        if let Some(card) = self.next_grammar_card(&surface, now)? {
            return Ok(Some(card));
        }
        self.bump_intro_counter("intro.words")?;
        let Some(card) = self.word_card(&surface)? else {
            return Ok(None);
        };
        // An uncurated proper name has no meaning to drill — show it once and
        // seed it known, so it counts toward verse readability without ever
        // becoming a review card (the learner meets it highlighted in verses
        // instead). Grading the shown card updates this seeded row, so a
        // learner who answers "no idea" still pulls the name back into
        // learning.
        if card.gloss == NAME_GLOSS {
            let seeded = seeded_known_srs();
            self.conn().execute(
                "INSERT INTO progress.word_srs(surface, surface_id, ease, interval_days, \
                    due_epoch, reps, lapses, introduced_epoch, last_grade) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 2) \
                 ON CONFLICT(surface) DO NOTHING",
                params![
                    surface,
                    card.surface_id,
                    seeded.ease,
                    seeded.interval_days,
                    seeded.due_at(now),
                    seeded.reps,
                    seeded.lapses,
                    now
                ],
            )?;
        }
        Ok(Some(StudyItem::NewWord(card)))
    }

    /// Whether the next introduction should be a new letter rather than a
    /// ready word's meaning, keeping the running letter share near
    /// [`TutorSettings::letters_ratio`]: introduce a letter while letters are
    /// under their target share of introductions so far.
    fn prefer_new_letter(&self, s: &TutorSettings) -> rusqlite::Result<bool> {
        let (letters, words) = self.intro_counts()?;
        Ok(letters * 100 < s.letters_ratio as i64 * (letters + words + 1))
    }

    /// The most frequent teachable surface (not Aramaic, not behind a locked
    /// grammar rule) that still contains a never-seen glyph, paired with that
    /// glyph — the alphabet frontier used to introduce letters faster than the
    /// target verse alone needs when the focus setting is letters-forward.
    fn alphabet_frontier_glyph(
        &self,
        unlocked: i64,
    ) -> rusqlite::Result<Option<(String, GlyphCard)>> {
        let seen = self.seen_glyph_mask()?;
        let surface: Option<String> = self
            .conn()
            .query_row(
                "SELECT s.text FROM hebrewdb.surface s
                 JOIN progress.surface_meta sm ON sm.surface_id = s.surface_id
                 WHERE (COALESCE(sm.concept_mask, 0) & ~?1) = 0
                   AND popcount(sm.glyph_mask & ~?2) > 0
                   AND COALESCE(s.language, '') <> 'aramaic'
                 ORDER BY s.occurrences DESC LIMIT 1",
                params![unlocked, seen],
                |r| r.get(0),
            )
            .optional()?;
        let Some(s) = surface else {
            return Ok(None);
        };
        for g in decompose_glyphs(&s) {
            if !self.glyph_known(&g.glyph)? {
                return Ok(Some((s, g)));
            }
        }
        Ok(None)
    }

    /// Whether a grammar concept has already been introduced.
    fn concept_seen(&self, key: &str) -> rusqlite::Result<bool> {
        Ok(self
            .conn()
            .query_row(
                "SELECT 1 FROM progress.concepts_seen WHERE concept = ?1",
                params![key],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    /// How many [`crate::grammar`] concepts have been introduced so far, for
    /// the progress displays (a plain query over `progress.concepts_seen`
    /// restricted to grammar keys, rather than `crate::grammar::concept_count()`
    /// calls to [`Self::concept_seen`]).
    fn grammar_concepts_seen(&self) -> rusqlite::Result<i64> {
        let keys: Vec<&str> = crate::grammar::concepts().iter().map(|c| c.key).collect();
        let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        self.conn().query_row(
            &format!(
                "SELECT COUNT(*) FROM progress.concepts_seen WHERE concept IN ({placeholders})"
            ),
            rusqlite::params_from_iter(keys.iter()),
            |r| r.get(0),
        )
    }

    /// The surface used to illustrate a grammar card: a word the learner has
    /// already studied that exercises the concept as its hardest rule, else
    /// the most frequent such word in the corpus — never a proper name —
    /// falling back to the triggering word itself. A rare name like
    /// הַכַּרְמְלִי makes a poor first example of the definite article when
    /// הָאָרֶץ exists.
    fn grammar_example_surface(&self, key: &str, trigger: &str) -> rusqlite::Result<String> {
        let Some(rank) = crate::grammar::concept_index(key) else {
            return Ok(trigger.to_string());
        };
        // "Exercises the concept as its hardest rule": the concept's bit is
        // set and no higher-indexed bit is (indexes follow inventory order,
        // which still encodes rough difficulty for display purposes).
        for studied_only in [true, false] {
            let studied = if studied_only {
                "JOIN progress.word_srs ws ON ws.surface_id = sm.surface_id"
            } else {
                ""
            };
            let found: Option<String> = self
                .conn()
                .query_row(
                    &format!(
                        "SELECT s.text FROM progress.surface_meta sm \
                         JOIN hebrewdb.surface s ON s.surface_id = sm.surface_id \
                         {studied} \
                         WHERE (sm.concept_mask & ?1) != 0 AND (sm.concept_mask >> (?2 + 1)) = 0 \
                           AND sm.is_name = 0 \
                         ORDER BY s.occurrences DESC LIMIT 1"
                    ),
                    params![1i64 << rank, rank],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(s) = found {
                return Ok(s);
            }
        }
        Ok(trigger.to_string())
    }

    /// The first grammar concept `surface` exercises that has not yet been
    /// shown, as an [`StudyItem::ExplainGrammar`] card — marking it seen so it
    /// is shown at most once. `None` when the word introduces no new concept.
    /// Curated function words (prepositions, suffixed prepositions) and
    /// misparsed construct forms classify through
    /// [`crate::grammar::concepts_for_surface`] even without a parse. The
    /// illustrating word is the most familiar example of the concept
    /// ([`Self::grammar_example_surface`]), not necessarily `surface` itself.
    fn next_grammar_card(&self, surface: &str, now: i64) -> rusqlite::Result<Option<StudyItem>> {
        let w = self.hebrew_word_info(surface);
        for key in crate::grammar::concepts_for_surface(surface, w.as_ref()) {
            if self.concept_seen(key)? {
                continue;
            }
            let Some(c) = crate::grammar::concept(key) else {
                continue;
            };
            let example_surface = self.grammar_example_surface(key, surface)?;
            let Some(example) = self
                .word_card(&example_surface)?
                .or(self.word_card(surface)?)
            else {
                return Ok(None);
            };
            self.conn().execute(
                "INSERT INTO progress.concepts_seen(concept, introduced_epoch) VALUES (?1, ?2) \
                 ON CONFLICT(concept) DO NOTHING",
                params![key, now],
            )?;
            return Ok(Some(StudyItem::ExplainGrammar(GrammarCard {
                concept: key.to_string(),
                title: c.title.to_string(),
                explanation: c.explanation.to_string(),
                formula: c.formula.unwrap_or_default().to_string(),
                examples: c.examples.iter().map(|s| s.to_string()).collect(),
                example,
            })));
        }
        Ok(None)
    }

    /// A form drill to introduce for a word of the target verse whose meaning is
    /// already known (graduated) and whose form is worth drilling
    /// (`form_tier >= 2`), not yet started as a form drill. Simplest form first,
    /// then most common. Never a proper name: a name card is *seeded* graduated
    /// after one showing, which would otherwise make it instantly form-drillable
    /// — quizzing inflections of its junk bridged gloss ("the adj.gent.s").
    /// `None` once every drillable word has one. The `form_srs` row is created
    /// when the card is first graded (like a word).
    fn next_form_introduction(
        &self,
        b: u8,
        c: u8,
        v: u8,
        _now: i64,
    ) -> rusqlite::Result<Option<StudyItem>> {
        let surface: Option<String> = self
            .conn()
            .query_row(
                "SELECT s.text
                 FROM hebrewdb.verse_word vw
                 JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
                 JOIN progress.word_srs ws
                   ON ws.surface_id = vw.surface_id AND ws.interval_days >= 1
                 JOIN progress.surface_meta sm ON sm.surface_id = vw.surface_id
                 LEFT JOIN progress.form_srs fs ON fs.surface_id = vw.surface_id
                 WHERE vw.book = ?1 AND vw.chapter = ?2 AND vw.verse = ?3
                   AND sm.form_tier >= 2 AND fs.surface_id IS NULL
                   AND sm.is_name = 0
                 ORDER BY sm.form_tier ASC, s.occurrences DESC
                 LIMIT 1",
                params![b, c, v],
                |r| r.get(0),
            )
            .optional()?;
        let Some(surface) = surface else {
            return Ok(None);
        };
        Ok(self.form_card(&surface)?.map(StudyItem::NewFormDrill))
    }

    // --- pronominal-ending drills --------------------------------------------

    /// A known host word for the ending `key`: an introduced (optionally
    /// graduated) `word_srs` surface that is a curated suffixed function word
    /// carrying that ending. `random` rotates the host between reviews so the
    /// ending is what gets learnt; otherwise the most frequent host is chosen
    /// (the introduction card, deterministic).
    fn suffix_host(
        &self,
        key: &str,
        graduated_only: bool,
        random: bool,
    ) -> rusqlite::Result<Option<String>> {
        let cond = if graduated_only {
            "ws.interval_days >= 1"
        } else {
            "1=1"
        };
        let order = if random {
            "RANDOM()"
        } else {
            "s.occurrences DESC"
        };
        let mut stmt = self.conn().prepare(&format!(
            "SELECT ws.surface FROM progress.word_srs ws \
             JOIN hebrewdb.surface s ON s.surface_id = ws.surface_id \
             WHERE {cond} ORDER BY {order}"
        ))?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        for row in rows {
            let surface = row?;
            if !crate::grammar::pronoun_suffix_host(&surface) {
                continue;
            }
            if let Some(split) = crate::pronoun_suffix::split_pronoun_suffix(&surface)
                && split.key == key
            {
                return Ok(Some(surface));
            }
        }
        Ok(None)
    }

    /// Build the drill card for ending `key` on `host`. `None` when the host
    /// doesn't actually carry the ending (stale state).
    fn suffix_card(&self, key: &str, host: &str) -> rusqlite::Result<Option<SuffixCard>> {
        let Some(split) = crate::pronoun_suffix::split_pronoun_suffix(host) else {
            return Ok(None);
        };
        if split.key != key {
            return Ok(None);
        }
        // The host's learner gloss, as its word card would show it.
        let gloss = match crate::vocab_gloss::curated_gloss(host) {
            Some(c) => c.gloss.to_string(),
            None => self
                .hebrew_word_info(host)
                .map(|w| w.gloss.trim().to_string())
                .unwrap_or_default(),
        };
        Ok(Some(SuffixCard {
            key: key.to_string(),
            meaning: split.meaning.to_string(),
            surface: host.to_string(),
            translit: crate::romanize::romanize(host),
            stem: split.stem,
            suffix: split.suffix,
            gloss,
            distractors: self.suffix_distractors(key)?,
        }))
    }

    /// Other endings' meanings as wrong answers: already-introduced endings
    /// first (in introduction order), topped up from the inventory (teaching
    /// order) so early drills still fill a multiple-choice card — the same
    /// top-up glyph distractors use.
    fn suffix_distractors(&self, key: &str) -> rusqlite::Result<Vec<String>> {
        const WANT: usize = 3;
        let own = crate::pronoun_suffix::pronoun_suffix(key).map(|p| p.meaning);
        let mut out: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<&str> = own.into_iter().collect();
        let introduced: Vec<String> = {
            let mut stmt = self
                .conn()
                .prepare("SELECT key FROM progress.suffix_srs ORDER BY introduced_epoch")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            rows.collect::<Result<_, _>>()?
        };
        let inventory = crate::pronoun_suffix::PRONOUN_SUFFIXES
            .iter()
            .map(|p| p.key.to_string());
        for k in introduced.into_iter().chain(inventory) {
            if out.len() >= WANT {
                break;
            }
            let Some(p) = crate::pronoun_suffix::pronoun_suffix(&k) else {
                continue;
            };
            if seen.insert(p.meaning) {
                out.push(p.meaning.to_string());
            }
        }
        Ok(out)
    }

    /// A new pronominal-ending drill, once its concept has been taught: the
    /// first inventory ending (teaching order) with no `suffix_srs` row yet
    /// and a graduated host word to show it on. The row is created when the
    /// card is first graded, like a form drill.
    fn next_suffix_introduction(&self, _now: i64) -> rusqlite::Result<Option<StudyItem>> {
        // The pronoun-ending idea is explained by the prep-suffix card (or,
        // for the אֹתוֹ family, the object-marker card) before any drilling.
        if !self.concept_seen("prep-suffix")? && !self.concept_seen("object-marker")? {
            return Ok(None);
        }
        for p in crate::pronoun_suffix::PRONOUN_SUFFIXES {
            if self.suffix_srs(p.key)?.is_some() {
                continue;
            }
            if let Some(host) = self.suffix_host(p.key, true, false)?
                && let Some(card) = self.suffix_card(p.key, &host)?
            {
                return Ok(Some(StudyItem::NewSuffixDrill(card)));
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
        // The most-due candidate from each of the three review stores.
        let pick = |table: &str, key_col: &str| -> rusqlite::Result<Option<(String, i64)>> {
            let sql = format!(
                "SELECT {key_col}, {order_col} FROM progress.{table} WHERE {cond} \
                 ORDER BY {order_col} ASC LIMIT 1"
            );
            let map = |r: &rusqlite::Row| Ok((r.get(0)?, r.get(1)?));
            if pull_forward {
                self.conn().query_row(&sql, [], map).optional()
            } else {
                self.conn().query_row(&sql, params![now], map).optional()
            }
        };
        let glyph: GlyphRow = pick("glyph_srs", "glyph")?;
        let word: WordRow = pick("word_srs", "surface")?;
        let form: WordRow = pick("form_srs", "surface")?;
        let suffix: WordRow = pick("suffix_srs", "key")?;

        // Whichever is most due wins; a smaller `order_col` is more due. Ties
        // break word → form → suffix → glyph (word meaning first).
        let due_of = |r: &Option<(String, i64)>| r.as_ref().map(|(_, d)| *d);
        let best = [
            due_of(&word),
            due_of(&form),
            due_of(&suffix),
            due_of(&glyph),
        ]
        .into_iter()
        .flatten()
        .min();
        let Some(best) = best else {
            debug!("next_review: pull_forward={pull_forward} -> nothing due");
            return Ok(None);
        };
        if due_of(&word) == Some(best) {
            let (surface, _) = word.expect("min came from word");
            debug!("next_review: pull_forward={pull_forward} -> word {surface:?} (due={best})");
            // Periodically put a due word back into authentic context, but
            // only in a verse whose every surface has graduated. Recording
            // the presentation immediately means the next request returns
            // the actual word review instead of repeating the verse.
            if !pull_forward {
                let reviews: i64 = self.conn().query_row(
                    "SELECT COUNT(*) FROM progress.reviews WHERE track = 'word'",
                    [],
                    |r| r.get(0),
                )?;
                let shown_at: Option<i64> = self.conn().query_row(
                    "SELECT CAST(value AS INTEGER) FROM progress.meta WHERE key = 'reading_review_at'",
                    [], |r| r.get(0),
                ).optional()?;
                if reviews > 0 && reviews % 10 == 0 && shown_at != Some(reviews) {
                    self.ensure_surface_meta()?;
                    self.ensure_readability_progress()?;
                    let verse: Option<(u8, u8, u8)> = self
                        .conn()
                        .query_row(
                            "SELECT vp.book, vp.chapter, vp.verse
                         FROM progress.verse_progress vp
                         JOIN hebrewdb.verse_word vw ON vw.book = vp.book
                            AND vw.chapter = vp.chapter AND vw.verse = vp.verse
                         JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
                         WHERE vp.unknown_words = 0 AND s.text = ?1
                         ORDER BY vp.last_read_epoch, vp.book, vp.chapter, vp.verse LIMIT 1",
                            params![surface],
                            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                        )
                        .optional()?;
                    if let Some((b, c, v)) = verse {
                        self.conn().execute(
                            "UPDATE progress.verse_progress SET last_read_epoch = ?4
                             WHERE book = ?1 AND chapter = ?2 AND verse = ?3",
                            params![b, c, v, now],
                        )?;
                        self.conn().execute(
                            "INSERT INTO progress.meta(key, value) VALUES ('reading_review_at', ?1)
                             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                            params![reviews.to_string()],
                        )?;
                        let examples = self.readable_examples(b, c, v, 3)?;
                        let (words, names) = self.verse_words_flagged(b, c, v)?.into_iter().unzip();
                        return Ok(Some(StudyItem::ReadVerse(VerseCard {
                            book: b,
                            chapter: c,
                            verse: v,
                            examples,
                            words,
                            names,
                        })));
                    }
                }
            }
            return Ok(self.word_card(&surface)?.map(StudyItem::ReviewWord));
        }
        if due_of(&form) == Some(best) {
            let (surface, _) = form.expect("min came from form");
            debug!("next_review: pull_forward={pull_forward} -> form {surface:?} (due={best})");
            return Ok(self.form_card(&surface)?.map(StudyItem::ReviewFormDrill));
        }
        if due_of(&suffix) == Some(best) {
            let (key, _) = suffix.expect("min came from suffix");
            debug!("next_review: pull_forward={pull_forward} -> suffix {key:?} (due={best})");
            // A random known host keeps the drill about the ending, not one
            // word's shape. Every introduced ending had a host word; if none
            // remains (stale state from a partial reset), drop the orphan row
            // — it re-introduces itself once a host graduates again.
            if let Some(host) = self.suffix_host(&key, false, true)?
                && let Some(card) = self.suffix_card(&key, &host)?
            {
                return Ok(Some(StudyItem::ReviewSuffixDrill(card)));
            }
            debug!("next_review: no host renders suffix {key:?}; dropping the row");
            self.conn().execute(
                "DELETE FROM progress.suffix_srs WHERE key = ?1",
                params![key],
            )?;
            return self.next_review(now, pull_forward);
        }
        let (g, _) = glyph.expect("min came from glyph");
        debug!("next_review: pull_forward={pull_forward} -> glyph {g:?} (due={best})");
        Ok(Some(StudyItem::ReviewGlyph(self.review_glyph_card(g)?)))
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
                    ..Default::default()
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
        self.next_study_item_impl(now, false)
    }

    /// Implements [`Self::next_study_item`]. `interleave_on_stall` additionally
    /// tries a second verse for fresh material when the pinned target has
    /// nothing left to introduce (see the call site in [`Self::submit_review`]);
    /// it costs an extra whole-corpus query, so it's only worth paying right
    /// after a lapse — the case that used to hand the learner straight back
    /// the card they just got wrong, seconds before it would naturally recur
    /// via `due_epoch`. Cheap, no-op internal recursive calls stay at `false`.
    fn next_study_item_impl(
        &self,
        now: i64,
        interleave_on_stall: bool,
    ) -> rusqlite::Result<StudyItem> {
        debug!("next_study_item: now={now}");
        // The language-intro deck comes before everything, even due reviews:
        // three one-time cards a brand-new learner needs before the first
        // glyph (an existing learner sees each at most once too — the deck
        // also backfills the reference page).
        for key in INTRO_CONCEPTS {
            if !self.concept_seen(key)? {
                self.conn().execute(
                    "INSERT INTO progress.concepts_seen(concept, introduced_epoch) VALUES (?1, ?2) \
                     ON CONFLICT(concept) DO NOTHING",
                    params![key, now],
                )?;
                debug!("next_study_item: intro card {key}");
                return Ok(StudyItem::ExplainIntro(key.to_string()));
            }
        }
        if let Some(review) = self.next_review(now, false)? {
            debug!("next_study_item: due review -> {review:?}");
            return Ok(review);
        }
        // A pronominal ending whose concept card has been shown gets its
        // drill introduced as soon as a host word graduates — consolidation
        // on known material, ahead of anything new.
        if let Some(item) = self.next_suffix_introduction(now)? {
            debug!("next_study_item: suffix drill introduction -> {item:?}");
            return Ok(item);
        }
        // Curriculum ordering (below) needs the per-surface root/form-tier cache;
        // build it once here (cheap version-stamp check thereafter).
        self.ensure_surface_meta()?;
        self.ensure_readability_progress()?;
        let settings = self.tutor_settings()?;
        let unlocked = self.unlocked_concepts(&settings, now)?;
        // Letter-learning phase: the alphabet isn't known yet, so no grammar rule
        // is unlocked — the curriculum stays on the simplest grammar-free words
        // (ordered by fewest new letters). `seen_mask` counts how many letters a
        // word would add. Once gating is off, every word is available and this is
        // just the normal semantic ordering.
        let letter_learning = settings.grammar_gating && unlocked == 0;
        let seen_mask = if letter_learning {
            self.seen_glyph_mask()?
        } else {
            0
        };
        debug!(
            "next_study_item: unlocked={unlocked:#x} ({} concepts) \
             letter_learning={letter_learning} seen_mask={seen_mask:#x}",
            unlocked.count_ones()
        );
        let target = match self.meta_target()? {
            Some(t) => {
                debug!("next_study_item: resuming meta_target={t:?}");
                t
            }
            None => match self.next_target_verse(unlocked, seen_mask, letter_learning)? {
                Some(t) => {
                    debug!("next_study_item: new target verse={t:?}");
                    self.set_meta_target(Some(t))?;
                    t
                }
                None => {
                    // Nothing is currently eligible inside the paced grammar
                    // frontier. Consolidate open cards; a later graduation can
                    // advance the frontier according to `words_per_concept`.
                    // Never force a grammar unlock merely to obtain a verse.
                    debug!("next_study_item: no eligible target; consolidating");
                    return Ok(self.next_review(now, true)?.unwrap_or(StudyItem::Done));
                }
            },
        };
        let (b, c, v) = target;

        // Target selection only admits verses inside the paced grammar
        // frontier.  In particular, pinning a verse must not force-unlock a
        // rule and bypass the learner's grammar-priority pacing setting.

        if let Some(item) =
            self.next_introduction((b, c, v), now, unlocked, seen_mask, letter_learning)?
        {
            debug!("next_study_item: introducing {item:?}");
            return Ok(item);
        }
        if !self.verse_done(b, c, v)? {
            // The verse may now be waiting solely on another root family's
            // Qal base. Release the pin so targeting can go teach that base;
            // once it graduates this verse becomes eligible again.
            if self
                .unfinished_words((b, c, v), unlocked, seen_mask, letter_learning, true)?
                .is_empty()
            {
                self.set_meta_target(None)?;
                return self.next_study_item_impl(now, interleave_on_stall);
            }
            // The pinned verse has nothing new to teach right now (every
            // remaining word is either locked or already mid-learning — e.g.
            // during letter-learning only one word's glyphs may be fully
            // known at a time). Rather than immediately re-drilling a card
            // the learner just answered seconds ago (pull-forward below
            // ignores `due_epoch` and would otherwise hand back the very
            // card just graded, since it's the only thing in the learning
            // pool), look for a second verse with fresh material to
            // interleave — the just-graded card naturally resurfaces once
            // it's actually due, via the check at the top of this function.
            if interleave_on_stall
                && let Some((ab, ac, av)) = self.next_target_verse_excluding(
                    Some((b, c, v)),
                    unlocked,
                    seen_mask,
                    letter_learning,
                )?
                && let Some(item) =
                    self.next_introduction((ab, ac, av), now, unlocked, seen_mask, letter_learning)?
            {
                debug!(
                    "next_study_item: verse {b}/{c}/{v} has nothing new; \
                             interleaving from {ab}/{ac}/{av} -> {item:?}"
                );
                return Ok(item);
            }
            // Words mid-learning: drill a learning card toward graduation.
            if let Some(review) = self.next_review(now, true)? {
                debug!(
                    "next_study_item: verse {b}/{c}/{v} not done; pull-forward review -> {review:?}"
                );
                return Ok(review);
            }
            // Nothing left to introduce or drill for this verse, yet it isn't
            // fully learnt — its remaining words are behind the letter-phase
            // grammar lock, or unteachable (a stale pin from before the
            // unteachable gate). Don't mark it readable; drop the target and
            // move to a verse we can make progress on.
            debug!(
                "next_study_item: verse {b}/{c}/{v} stuck behind locked grammar; dropping target"
            );
            self.set_meta_target(None)?;
            return self.next_study_item_impl(now, interleave_on_stall);
        }
        // Meanings known: introduce a form drill for each drillable word once
        // (its grammatical form, over and above its meaning) before reading.
        if let Some(form) = self.next_form_introduction(b, c, v, now)? {
            debug!("next_study_item: form drill introduction -> {form:?}");
            return Ok(form);
        }
        // Verse fully learnt: explain any unseen reading marks, then read it.
        // Recorded as seen immediately (never drilled), mirroring how the
        // verse itself is marked readable below.
        if let Some(mark) = self.next_unseen_reading_mark(b, c, v)? {
            debug!("next_study_item: explaining unseen reading mark {mark:?}");
            self.conn().execute(
                "INSERT INTO progress.marks_seen(mark, introduced_epoch) VALUES (?1, ?2) \
                 ON CONFLICT(mark) DO NOTHING",
                params![mark.glyph, now],
            )?;
            return Ok(StudyItem::ExplainMark(mark));
        }
        debug!("next_study_item: verse {b}/{c}/{v} fully learnt -> offering reading review");
        self.conn().execute(
            "UPDATE progress.verse_progress SET last_read_epoch = ?4
             WHERE book = ?1 AND chapter = ?2 AND verse = ?3 AND unknown_words = 0",
            params![b, c, v, now],
        )?;
        self.set_meta_target(None)?;
        let examples = self.readable_examples(b, c, v, 3)?;
        let (words, names) = self.verse_words_flagged(b, c, v)?.into_iter().unzip();
        Ok(StudyItem::ReadVerse(VerseCard {
            book: b,
            chapter: c,
            verse: v,
            examples,
            words,
            names,
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
        debug!("submit_review: track={track:?} key={key:?} grade={grade:?} now={now}");
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
                            reps, lapses, introduced_epoch, last_grade, updated_epoch) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
                         ON CONFLICT(glyph) DO UPDATE SET ease=excluded.ease, \
                            interval_days=excluded.interval_days, due_epoch=excluded.due_epoch, \
                            reps=excluded.reps, lapses=excluded.lapses, last_grade=excluded.last_grade, \
                            updated_epoch=excluded.updated_epoch",
                        params![
                            glyph,
                            next.ease,
                            next.interval_days,
                            next.due_at(now),
                            next.reps,
                            next.lapses,
                            now,
                            grade_i,
                            now
                        ],
                    )?;
                }
            }
            Track::Word => {
                let previous = self.word_srs(key)?.unwrap_or_default();
                let next = previous.graded(grade);
                let due = next.due_at(now);
                let surface_id: i64 = self.conn().query_row(
                    "SELECT surface_id FROM hebrewdb.surface WHERE text = ?1",
                    params![key],
                    |r| r.get(0),
                )?;
                let graduating = !previous.graduated() && next.graduated();
                if graduating {
                    // Bring an old or freshly-created progress database up to
                    // date before the SRS row starts reporting this surface as
                    // graduated. Doing this after the write makes the cache
                    // consistency check see a mismatch and rebuild readability
                    // for the entire corpus; once the baseline is current,
                    // `record_surface_graduation` only touches this vocabulary
                    // key and the verses that contain it.
                    self.ensure_surface_meta()?;
                    self.ensure_readability_progress()?;
                }
                self.conn().execute(
                    "INSERT INTO progress.word_srs(surface, surface_id, ease, \
                        interval_days, due_epoch, reps, lapses, introduced_epoch, last_grade, updated_epoch) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
                     ON CONFLICT(surface) DO UPDATE SET ease=excluded.ease, \
                        interval_days=excluded.interval_days, due_epoch=excluded.due_epoch, \
                        reps=excluded.reps, lapses=excluded.lapses, last_grade=excluded.last_grade, \
                        updated_epoch=excluded.updated_epoch",
                    params![
                        key,
                        surface_id,
                        next.ease,
                        next.interval_days,
                        due,
                        next.reps,
                        next.lapses,
                        now,
                        grade_i,
                        now
                    ],
                )?;
                if graduating {
                    self.record_surface_graduation(surface_id, now)?;
                    // A curriculum pin belonged to the old verse-first model.
                    // Release it whenever its word graduates so the next word
                    // is selected globally by root frequency.
                    self.set_meta_target(None)?;
                }
            }
            Track::Form => {
                let next = self.form_srs(key)?.unwrap_or_default().graded(grade);
                let due = next.due_at(now);
                let surface_id: i64 = self.conn().query_row(
                    "SELECT surface_id FROM hebrewdb.surface WHERE text = ?1",
                    params![key],
                    |r| r.get(0),
                )?;
                self.conn().execute(
                    "INSERT INTO progress.form_srs(surface, surface_id, ease, \
                        interval_days, due_epoch, reps, lapses, introduced_epoch, last_grade, updated_epoch) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
                     ON CONFLICT(surface) DO UPDATE SET ease=excluded.ease, \
                        interval_days=excluded.interval_days, due_epoch=excluded.due_epoch, \
                        reps=excluded.reps, lapses=excluded.lapses, last_grade=excluded.last_grade, \
                        updated_epoch=excluded.updated_epoch",
                    params![
                        key,
                        surface_id,
                        next.ease,
                        next.interval_days,
                        due,
                        next.reps,
                        next.lapses,
                        now,
                        grade_i,
                        now
                    ],
                )?;
            }
            Track::Suffix => {
                // Only inventory endings are drilled — ignore a stale or
                // foreign key rather than let it haunt the review rotation
                // (the same guard reading-mark glyph keys get).
                if crate::pronoun_suffix::pronoun_suffix(key).is_some() {
                    let next = self.suffix_srs(key)?.unwrap_or_default().graded(grade);
                    self.conn().execute(
                        "INSERT INTO progress.suffix_srs(key, ease, interval_days, due_epoch, \
                            reps, lapses, introduced_epoch, last_grade, updated_epoch) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
                         ON CONFLICT(key) DO UPDATE SET ease=excluded.ease, \
                            interval_days=excluded.interval_days, due_epoch=excluded.due_epoch, \
                            reps=excluded.reps, lapses=excluded.lapses, last_grade=excluded.last_grade, \
                            updated_epoch=excluded.updated_epoch",
                        params![
                            key,
                            next.ease,
                            next.interval_days,
                            next.due_at(now),
                            next.reps,
                            next.lapses,
                            now,
                            grade_i,
                            now
                        ],
                    )?;
                }
            }
        }
        // Log one review event per card answer (a syllable card grades several
        // glyphs but is a single answer) for streak / activity / accuracy stats.
        let track_str = match track {
            Track::Glyph => "glyph",
            Track::Word => "word",
            Track::Form => "form",
            Track::Suffix => "suffix",
        };
        self.conn().execute(
            "INSERT INTO progress.reviews(epoch, day, track, grade) VALUES (?1, ?2, ?3, ?4)",
            params![now, now.div_euclid(SECONDS_PER_DAY), track_str, grade_i],
        )?;
        // A lapse justifies the extra whole-corpus query in
        // `next_study_item_impl`'s stall fallback: without it, a just-failed
        // card that's the only thing in the learning pool gets handed straight
        // back before its `due_epoch`, drilling the same word over and over
        // instead of resting it and teaching something new in the meantime.
        self.next_study_item_impl(now, grade == Grade::Again)
    }

    /// A verse's words in reading order, as `word_srs` surface keys — so the
    /// app can offer them for the learner to flag ones they misread.
    pub fn verse_words(&self, b: u8, c: u8, v: u8) -> rusqlite::Result<Vec<String>> {
        Ok(self
            .verse_words_flagged(b, c, v)?
            .into_iter()
            .map(|(w, _)| w)
            .collect())
    }

    /// [`Self::verse_words`] with each word's proper-name flag (from the
    /// `surface_meta` cache), for rendering names distinctly in the verse.
    pub fn verse_words_flagged(
        &self,
        b: u8,
        c: u8,
        v: u8,
    ) -> rusqlite::Result<Vec<(String, bool)>> {
        let mut stmt = self.conn().prepare(
            "SELECT s.text, COALESCE(sm.is_name, 0)
             FROM hebrewdb.verse_word vw
             JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
             LEFT JOIN progress.surface_meta sm ON sm.surface_id = vw.surface_id
             WHERE vw.book = ?1 AND vw.chapter = ?2 AND vw.verse = ?3
             ORDER BY vw.position",
        )?;
        stmt.query_map(params![b, c, v], |r| {
            Ok((r.get(0)?, r.get::<_, i64>(1)? != 0))
        })?
        .collect()
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
        let mut stmt = self.conn().prepare(
            "SELECT DISTINCT vw2.book, vw2.chapter, vw2.verse
             FROM hebrewdb.verse_word vw1
             JOIN hebrewdb.verse_word vw2 ON vw2.surface_id = vw1.surface_id
             WHERE vw1.book = ?1 AND vw1.chapter = ?2 AND vw1.verse = ?3
               AND NOT (vw2.book = ?1 AND vw2.chapter = ?2 AND vw2.verse = ?3)
               AND NOT EXISTS (
                   SELECT 1 FROM hebrewdb.verse_word w3
                   JOIN progress.surface_progress sp3 ON sp3.surface_id = w3.surface_id
                   WHERE w3.book = vw2.book AND w3.chapter = vw2.chapter
                      AND w3.verse = vw2.verse AND sp3.graduated = 0)
             LIMIT ?4",
        )?;
        let rows = stmt.query_map(params![b, c, v, limit], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;
        rows.collect()
    }

    /// Headline counters for a progress display.
    pub fn tutor_progress(&self) -> rusqlite::Result<TutorProgress> {
        self.ensure_surface_meta()?;
        self.ensure_readability_progress()?;
        // Consonants folded by leading codepoint (so בּ/ב count once); vowel
        // points counted individually. Mirrors the classification in
        // `all_letters_known`; dagesh/dots/marks fall into neither bucket.
        // Graduated rows only, matching `words_known` ([`DONE_SURFACES`]) —
        // counting glyphs at first sight next to graduated-only words made the
        // alphabet look like it was racing ahead of the vocabulary.
        let letters_known = self.conn().query_row(
            "SELECT COUNT(DISTINCT unicode(substr(glyph, 1, 1))) FROM progress.glyph_srs \
             WHERE interval_days >= 1 AND unicode(substr(glyph, 1, 1)) BETWEEN 1488 AND 1514",
            [],
            |r| r.get(0),
        )?;
        let vowels_known = self.conn().query_row(
            "SELECT COUNT(*) FROM progress.glyph_srs \
             WHERE interval_days >= 1 \
               AND (unicode(glyph) BETWEEN 1456 AND 1465 OR unicode(glyph) IN (1467, 1479))",
            [],
            |r| r.get(0),
        )?;
        let words_known = self.conn().query_row(
            &format!("SELECT COUNT(*) FROM ({DONE_SURFACES})"),
            [],
            |r| r.get(0),
        )?;
        let grammar_known = self.grammar_concepts_seen()?;
        let verses_grammar_unlocked = self.verses_grammar_unlocked()?;
        let verses_readable = self.conn().query_row(
            "SELECT COUNT(*) FROM progress.verse_progress WHERE unknown_words = 0",
            [],
            |r| r.get(0),
        )?;
        let total_verses =
            self.conn()
                .query_row("SELECT COUNT(*) FROM progress.verse_progress", [], |r| {
                    r.get(0)
                })?;
        Ok(TutorProgress {
            letters_known,
            letters_total: LETTER_GLYPH_TOTAL,
            vowels_known,
            vowels_total: VOWEL_GLYPH_TOTAL,
            grammar_known,
            grammar_total: crate::grammar::concept_count() as i64,
            words_known,
            verses_grammar_unlocked,
            verses_readable,
            total_verses,
        })
    }

    /// Number of verses covered by the current grammar frontier. The generated
    /// `verse_stats` table has one boolean column per grammar concept; a verse
    /// is covered when none of its required columns belongs to a locked rule.
    fn verses_grammar_unlocked(&self) -> rusqlite::Result<i64> {
        let settings = self.tutor_settings()?;
        if !settings.grammar_gating {
            return self
                .conn()
                .query_row("SELECT COUNT(*) FROM hebrewdb.verse_stats", [], |r| {
                    r.get(0)
                });
        }

        let mut unlocked = 0i64;
        let mut stmt = self
            .conn()
            .prepare("SELECT concept FROM progress.concepts_unlocked")?;
        for key in stmt.query_map([], |r| r.get::<_, String>(0))? {
            if let Some(bit) = crate::grammar::concept_bit(&key?) {
                unlocked |= bit;
            }
        }

        let locked_requirements = crate::grammar::concepts()
            .iter()
            .filter(|concept| {
                crate::grammar::concept_bit(concept.key).is_some_and(|bit| unlocked & bit == 0)
            })
            .map(|concept| format!("{} = 0", concept.key.replace('-', "_")))
            .collect::<Vec<_>>();
        let where_clause = if locked_requirements.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", locked_requirements.join(" AND "))
        };
        self.conn().query_row(
            &format!("SELECT COUNT(*) FROM hebrewdb.verse_stats{where_clause}"),
            [],
            |r| r.get(0),
        )
    }

    /// Every explanation card already shown, for the app's reference page:
    /// the intro deck (in deck order), the final-forms card, reading marks (in
    /// the order met) and grammar concepts (in teaching order). Only cards the
    /// learner has actually seen are listed — the page grows as the tutor
    /// unlocks them.
    pub fn seen_concepts(&self) -> rusqlite::Result<Vec<SeenConcept>> {
        let script = |kind: &str, key: &str| SeenConcept {
            kind: kind.to_string(),
            key: key.to_string(),
            title: String::new(),
            explanation: String::new(),
            formula: String::new(),
            examples: Vec::new(),
        };
        let mut out = Vec::new();
        for key in INTRO_CONCEPTS {
            if self.concept_seen(key)? {
                out.push(script("intro", key));
            }
        }
        if self.concept_seen(FINAL_FORMS_CONCEPT)? {
            out.push(script("final_forms", FINAL_FORMS_CONCEPT));
        }
        let mut stmt = self
            .conn()
            .prepare("SELECT mark FROM progress.marks_seen ORDER BY introduced_epoch")?;
        let marks: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        out.extend(marks.iter().map(|m| script("mark", m)));
        for c in crate::grammar::concepts() {
            if self.concept_seen(c.key)? {
                out.push(SeenConcept {
                    kind: "grammar".to_string(),
                    key: c.key.to_string(),
                    title: c.title.to_string(),
                    explanation: c.explanation.to_string(),
                    formula: c.formula.unwrap_or_default().to_string(),
                    examples: c.examples.iter().map(|s| s.to_string()).collect(),
                });
            }
        }
        Ok(out)
    }

    /// Richer SRS statistics for the stats view: learning/mature splits, cards
    /// due, activity (reviews today/total), streak, accuracy, and reading
    /// coverage. All cheap indexed counts over `progress.db`.
    pub fn tutor_stats(&self, now: i64) -> rusqlite::Result<TutorStats> {
        self.ensure_surface_meta()?;
        self.ensure_readability_progress()?;
        let conn = self.conn();
        let count = |sql: &str| -> rusqlite::Result<i64> { conn.query_row(sql, [], |r| r.get(0)) };

        // Letters folded by leading codepoint (בּ/ב count once); vowel points
        // counted individually. Mature rows are a subset of seen, so both
        // distinct/plain counts keep `learning = seen - mature` non-negative.
        const LETTER: &str = "unicode(substr(glyph, 1, 1)) BETWEEN 1488 AND 1514";
        const VOWEL: &str =
            "(unicode(glyph) BETWEEN 1456 AND 1465 OR unicode(glyph) IN (1467, 1479))";
        let letters_seen = count(&format!(
            "SELECT COUNT(DISTINCT unicode(substr(glyph, 1, 1))) FROM progress.glyph_srs \
             WHERE {LETTER}"
        ))?;
        let letters_mature = count(&format!(
            "SELECT COUNT(DISTINCT unicode(substr(glyph, 1, 1))) FROM progress.glyph_srs \
             WHERE {LETTER} AND interval_days >= 1"
        ))?;
        let vowels_seen = count(&format!(
            "SELECT COUNT(*) FROM progress.glyph_srs WHERE {VOWEL}"
        ))?;
        let vowels_mature = count(&format!(
            "SELECT COUNT(*) FROM progress.glyph_srs WHERE {VOWEL} AND interval_days >= 1"
        ))?;
        let words_seen = count("SELECT COUNT(*) FROM progress.word_srs")?;
        let words_mature =
            count("SELECT COUNT(*) FROM progress.word_srs WHERE interval_days >= 1")?;
        let grammar_seen = self.grammar_concepts_seen()?;

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
            count("SELECT COUNT(*) FROM progress.verse_progress WHERE unknown_words = 0")?;
        let total_verses = count("SELECT COUNT(*) FROM progress.verse_progress")?;

        Ok(TutorStats {
            letters_seen,
            letters_learning: letters_seen - letters_mature,
            letters_mature,
            vowels_seen,
            vowels_learning: vowels_seen - vowels_mature,
            vowels_mature,
            words_seen,
            words_learning: words_seen - words_mature,
            words_mature,
            grammar_seen,
            grammar_total: crate::grammar::concept_count() as i64,
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

    // --- onboarding calibration ----------------------------------------------
    //
    // A brand-new learner otherwise has to grind through every glyph and every
    // common word one SM-2 card at a time before reaching anything actually
    // new to them. Onboarding (driven from the app, offered only while
    // `progress.db` is still empty — see [`Self::needs_onboarding`]) lets them
    // self-report already knowing the alphabet ([`Self::seed_known_alphabet`])
    // and then calibrate a vocabulary baseline against real verses via binary
    // search ([`Self::calibration_probe`], [`Self::seed_known_vocab`]) instead.

    /// Whether the learner has no progress at all yet — the gate for offering
    /// onboarding calibration instead of the ordinary cold-start curriculum.
    /// Only checked once: any glyph or word progress (including calibration's
    /// own seeding) means onboarding has already happened or been skipped.
    pub fn needs_onboarding(&self) -> rusqlite::Result<bool> {
        let glyphs: i64 =
            self.conn()
                .query_row("SELECT COUNT(*) FROM progress.glyph_srs", [], |r| r.get(0))?;
        if glyphs > 0 {
            return Ok(false);
        }
        let words: i64 =
            self.conn()
                .query_row("SELECT COUNT(*) FROM progress.word_srs", [], |r| r.get(0))?;
        Ok(words == 0)
    }

    /// The number of distinct verse-difficulty tiers — the domain for
    /// [`Self::calibration_probe`]'s binary search. A verse's difficulty is
    /// its rarest word's occurrence count; raw vocabulary rank is *not* a
    /// usable search domain here, because Biblical Hebrew's frequency tail is
    /// dominated by hapax legomena (~58% of the ~51k distinct surfaces occur
    /// exactly once), so the bottom half of the rank space collapses to one
    /// giant plateau of identical thresholds — searching it would just keep
    /// re-serving the same verse. The distinct difficulty *values* are a much
    /// smaller, well-behaved set (~80), one probe per genuinely distinguishable
    /// step.
    pub fn calibration_tier_count(&self) -> rusqlite::Result<u32> {
        self.conn().query_row(
            &format!(
                "SELECT COUNT(*) FROM (SELECT DISTINCT {DIFFICULTY} AS diff \
                 FROM hebrewdb.verse_word vw \
                 JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id \
                 GROUP BY vw.book, vw.chapter, vw.verse \
                 HAVING {NOT_ARAMAIC})"
            ),
            [],
            |r| r.get(0),
        )
    }

    /// Mark every glyph the curriculum would ever teach — every consonant and
    /// vowel point `decompose_glyphs` produces across the whole non-Aramaic
    /// corpus — as already graduated. For a learner who self-reports already
    /// knowing the alphabet, so onboarding doesn't re-teach it letter by
    /// letter. The script explanation cards (the intro deck and final forms)
    /// are marked seen too — they explain how to read, which this learner
    /// already can — which also lands them on the reference page. Existing
    /// progress (glyphs already introduced) is left alone.
    pub fn seed_known_alphabet(&self, now: i64) -> rusqlite::Result<()> {
        for key in INTRO_CONCEPTS.iter().copied().chain([FINAL_FORMS_CONCEPT]) {
            self.conn().execute(
                "INSERT INTO progress.concepts_seen(concept, introduced_epoch) VALUES (?1, ?2) \
                 ON CONFLICT(concept) DO NOTHING",
                params![key, now],
            )?;
        }
        let mut glyphs = std::collections::HashSet::new();
        {
            let mut stmt = self
                .conn()
                .prepare("SELECT DISTINCT text FROM hebrewdb.surface WHERE language IS NULL")?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let text: String = row.get(0)?;
                for g in decompose_glyphs(&text) {
                    glyphs.insert(g.glyph);
                }
            }
        }
        let seeded = seeded_known_srs();
        for glyph in glyphs {
            self.conn().execute(
                "INSERT INTO progress.glyph_srs(glyph, ease, interval_days, due_epoch, \
                    reps, lapses, introduced_epoch, last_grade) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 2) \
                 ON CONFLICT(glyph) DO NOTHING",
                params![
                    glyph,
                    seeded.ease,
                    seeded.interval_days,
                    seeded.due_at(now),
                    seeded.reps,
                    seeded.lapses,
                    now
                ],
            )?;
        }
        Ok(())
    }

    /// A representative verse for the `tier`'th distinct difficulty tier
    /// (0 = easiest — the most common rarest-word — counting up toward the
    /// rarest), for the vocabulary-calibration binary search over
    /// [`Self::calibration_tier_count`] tiers. `None` once `tier` runs past
    /// the tier count.
    pub fn calibration_probe(&self, tier: u32) -> rusqlite::Result<Option<CalibrationProbe>> {
        let min_occurrences: Option<i64> = self
            .conn()
            .query_row(
                &format!(
                    "SELECT diff FROM (SELECT DISTINCT {DIFFICULTY} AS diff \
                     FROM hebrewdb.verse_word vw \
                     JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id \
                     GROUP BY vw.book, vw.chapter, vw.verse \
                     HAVING {NOT_ARAMAIC}) \
                     ORDER BY diff DESC LIMIT 1 OFFSET ?1"
                ),
                params![tier],
                |r| r.get(0),
            )
            .optional()?;
        let Some(min_occurrences) = min_occurrences else {
            return Ok(None);
        };

        // Any one verse at this exact difficulty; lowest reference first for
        // a deterministic pick (many verses can tie on the same difficulty).
        let found: Option<(u8, u8, u8)> = self
            .conn()
            .query_row(
                &format!(
                    "SELECT vw.book, vw.chapter, vw.verse
                     FROM hebrewdb.verse_word vw
                     JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id
                     GROUP BY vw.book, vw.chapter, vw.verse
                     HAVING {NOT_ARAMAIC} AND {DIFFICULTY} = ?1
                     ORDER BY vw.book, vw.chapter, vw.verse
                     LIMIT 1"
                ),
                params![min_occurrences],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some((b, c, v)) = found else {
            return Ok(None);
        };
        let text = self.get(b, c, v)?;
        Ok(Some(CalibrationProbe {
            book: b,
            chapter: c,
            verse: v,
            text,
            tier,
            min_occurrences,
        }))
    }

    /// Finish vocabulary calibration: mark every (non-Aramaic) word occurring
    /// at least `min_occurrences` times as already known (graduated), so the
    /// ordinary curriculum starts introducing new vocabulary from that
    /// frequency boundary instead of from scratch. A no-op for
    /// `min_occurrences <= 0` (nothing confirmed known).
    pub fn seed_known_vocab(&self, min_occurrences: i64, now: i64) -> rusqlite::Result<()> {
        if min_occurrences <= 0 {
            return Ok(());
        }
        let seeded = seeded_known_srs();
        let due = seeded.due_at(now);
        let mut stmt = self.conn().prepare(
            "SELECT text, surface_id FROM hebrewdb.surface \
             WHERE language IS NULL AND occurrences >= ?1",
        )?;
        let rows: Vec<(String, i64)> = stmt
            .query_map(params![min_occurrences], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (surface, surface_id) in rows {
            self.conn().execute(
                "INSERT INTO progress.word_srs(surface, surface_id, ease, interval_days, \
                    due_epoch, reps, lapses, introduced_epoch, last_grade) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 2) \
                 ON CONFLICT(surface) DO NOTHING",
                params![
                    surface,
                    surface_id,
                    seeded.ease,
                    seeded.interval_days,
                    due,
                    seeded.reps,
                    seeded.lapses,
                    now
                ],
            )?;
        }
        Ok(())
    }

    /// Wipe all tutor progress. Leaves `surface_meta` (and its `surface_meta_v`
    /// stamp in `meta`) alone: that cache is derived purely from `hebrew.db`,
    /// not learner progress, and clearing it would force the ~50k-surface
    /// `ensure_surface_meta` rebuild to redo its one-time scan on every reset.
    pub fn reset_tutor(&self) -> rusqlite::Result<()> {
        self.conn().execute_batch(
            "DELETE FROM progress.glyph_srs;
             DELETE FROM progress.word_srs;
             DELETE FROM progress.surface_progress;
             DELETE FROM progress.verse_progress;
             DELETE FROM progress.meta WHERE key != 'surface_meta_v';
             DELETE FROM progress.form_srs;
             DELETE FROM progress.suffix_srs;
             DELETE FROM progress.reviews;
             DELETE FROM progress.marks_seen;
             DELETE FROM progress.concepts_seen;
             DELETE FROM progress.concepts_unlocked;",
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

    /// Root-frequency verse selection must open the curriculum on useful
    /// vocabulary — not on genealogy lists of rare proper names. Rootlessness
    /// itself is not the right proxy: indispensable function words such as
    /// אֶת, אֲשֶׁר, כִּי, אֶל and לֹא legitimately have no lexical root.
    #[test]
    fn cold_start_opens_on_content_words_not_genealogy() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;
        bible.seed_known_alphabet(1_700_000_000)?; // skip glyph teaching
        // This exercises the difficulty ordering, not the grammar schedule —
        // keep every form available so verbs count toward the rooted tally.
        bible.set_tutor_settings(&TutorSettings {
            grammar_gating: false,
            ..Default::default()
        })?;

        let mut now = 1_700_000_000;
        let mut item = bible.next_study_item(now)?;
        let mut total = 0;
        let mut introduced = Vec::new();
        let mut proper_names = 0;
        for _ in 0..4000 {
            if total >= 12 {
                break;
            }
            now += 5;
            item = match item {
                StudyItem::NewWord(w) => {
                    total += 1;
                    let is_name: bool = bible.conn().query_row(
                        "SELECT is_name FROM progress.surface_meta WHERE surface_id = ?1",
                        params![w.surface_id],
                        |r| r.get(0),
                    )?;
                    proper_names += usize::from(is_name);
                    introduced.push((w.surface.clone(), w.root.clone(), is_name));
                    bible.submit_review(Track::Word, &w.surface, Grade::Easy, now)?
                }
                StudyItem::ReviewWord(w) => {
                    bible.submit_review(Track::Word, &w.surface, Grade::Easy, now)?
                }
                StudyItem::NewFormDrill(w) | StudyItem::ReviewFormDrill(w) => {
                    bible.submit_review(Track::Form, &w.surface, Grade::Easy, now)?
                }
                StudyItem::NewSuffixDrill(s) | StudyItem::ReviewSuffixDrill(s) => {
                    bible.submit_review(Track::Suffix, &s.key, Grade::Easy, now)?
                }
                StudyItem::NewGlyph(g) | StudyItem::ReviewGlyph(g) => {
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Easy, now)?
                }
                StudyItem::ExplainMark(_)
                | StudyItem::ExplainGrammar(_)
                | StudyItem::ExplainFinalForms(_)
                | StudyItem::ExplainIntro(_)
                | StudyItem::ReadVerse(_) => bible.next_study_item(now)?,
                StudyItem::Done => break,
            };
        }
        assert_eq!(total, 12, "should introduce a dozen words quickly");
        assert_eq!(
            proper_names, 0,
            "early vocabulary should not contain proper-name cards: {introduced:?}"
        );
        Ok(())
    }

    /// Root frequency chooses which verbal family to open; it must not inflate
    /// rare nouns or later forms once that choice has been made.
    #[test]
    fn root_frequency_only_boosts_verbal_family_openers() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;
        bible.ensure_surface_meta()?;

        let frequency = |surface: &str| -> rusqlite::Result<(i64, i64)> {
            bible.conn().query_row(
                &format!(
                    "SELECT s.occurrences, {WORD_FREQ} \
                     FROM hebrewdb.surface s \
                     JOIN progress.surface_meta sm ON sm.surface_id = s.surface_id \
                     LEFT JOIN hebrewdb.roots r ON r.root = sm.root \
                     WHERE s.text = ?1"
                ),
                params![surface],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
        };

        let (surface_frequency, curriculum_frequency) = frequency("נְכֹאת")?;
        assert_eq!(surface_frequency, 2);
        assert_eq!(curriculum_frequency, surface_frequency);

        let (surface, surface_frequency, root_frequency): (String, i64, i64) =
            bible.conn().query_row(
                "SELECT s.text, s.occurrences, r.n_occurrences \
                 FROM hebrewdb.surface s \
                 JOIN progress.surface_meta sm ON sm.surface_id = s.surface_id \
                 JOIN hebrewdb.roots r ON r.root = sm.root \
                 WHERE sm.family_base = 1 AND sm.form_tier >= 5 \
                   AND r.n_occurrences > s.occurrences \
                 ORDER BY r.n_occurrences DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
        let (_, curriculum_frequency) = frequency(&surface)?;
        assert_eq!(curriculum_frequency, root_frequency);
        assert!(curriculum_frequency > surface_frequency);

        let (later_qal, later_frequency): (String, i64) = bible.conn().query_row(
            "SELECT s.text, s.occurrences \
             FROM hebrewdb.surface s \
             JOIN progress.surface_meta sm ON sm.surface_id = s.surface_id \
             JOIN hebrewdb.roots r ON r.root = sm.root \
             WHERE sm.is_qal = 1 AND sm.family_base = 0 \
               AND r.n_occurrences > s.occurrences \
             ORDER BY r.n_occurrences DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let (_, curriculum_frequency) = frequency(&later_qal)?;
        assert_eq!(curriculum_frequency, later_frequency);
        Ok(())
    }

    /// The corpus treats לִקְרַאת as a frozen preposition for glossing, but
    /// curriculum ordering must still hold it behind קָרָא, the Qal family base.
    #[test]
    fn lexicalized_liqrat_waits_for_qal_base() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;
        bible.ensure_surface_meta()?;

        let (liqrat_root, tier, is_qal, family_base): (String, i64, bool, bool) =
            bible.conn().query_row(
                "SELECT sm.root, sm.form_tier, sm.is_qal, sm.family_base \
                 FROM progress.surface_meta sm \
                 JOIN hebrewdb.surface s ON s.surface_id = sm.surface_id \
                 WHERE s.text = 'לִקְרַאת'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?;
        assert_eq!(liqrat_root, "קרא");
        assert!(
            tier >= 5,
            "lexicalized verb must pass through the family gate"
        );
        assert!(is_qal);
        assert!(!family_base, "the infinitive must not open the family");

        let ready = || -> rusqlite::Result<bool> {
            bible.conn().query_row(
                &format!(
                    "SELECT {FAMILY_READY} FROM progress.surface_meta sm \
                     JOIN hebrewdb.surface s ON s.surface_id = sm.surface_id \
                     LEFT JOIN ({KNOWN_QAL_ROOTS}) kqr ON kqr.root = sm.root \
                     WHERE s.text = 'לִקְרַאת'"
                ),
                [],
                |r| r.get(0),
            )
        };
        assert!(!ready()?, "לִקְרַאת must initially be held back");

        let qara_id: i64 = bible.conn().query_row(
            "SELECT surface_id FROM hebrewdb.surface WHERE text = 'קָרָא'",
            [],
            |r| r.get(0),
        )?;
        bible.conn().execute(
            "INSERT INTO progress.word_srs(surface, surface_id, ease, interval_days, \
                due_epoch, reps, lapses, introduced_epoch, last_grade) \
             VALUES ('קָרָא', ?1, 2.5, 1, 0, 3, 0, 0, 2)",
            params![qara_id],
        )?;
        assert!(ready()?, "learning קָרָא must unlock לִקְרַאת");
        Ok(())
    }

    /// Proper names card as names, not as BDB citations or homograph senses:
    /// an uncurated name shows "(a name)" with the citation as its note (no
    /// spurious root, no quiz); a curated famous name shows its English name;
    /// a proclitic-prefixed curated name composes ("to Jacob", not "to heel").
    #[test]
    fn name_cards_show_names_not_bdb_citations() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        // אֶזְבָּי — a one-occurrence name bridged to the spurious root אות
        // with gloss "n.pr.m. father of one of David's men".
        let card = bible.word_card("אֶזְבָּי")?.expect("Ezbai has a surface");
        assert_eq!(card.gloss, NAME_GLOSS);
        assert!(
            card.note.contains("father of one of David"),
            "citation kept as note, got {:?}",
            card.note
        );
        assert!(
            card.root.is_empty(),
            "spurious root hidden: {:?}",
            card.root
        );
        assert!(card.distractors.is_empty(), "name cards self-grade");

        // אֶלְיַחְבָּא — a name whose BDB entry carries `n.pr.m` only in the
        // `pos` column; the gloss is the bare etymology "God hides", so the
        // gloss-text sniff alone carded it as vocabulary (root אלה).
        let card = bible.word_card("אֶלְיַחְבָּא")?.expect("Eliahba exists");
        assert_eq!(card.gloss, NAME_GLOSS);
        assert!(
            card.root.is_empty(),
            "spurious root hidden: {:?}",
            card.root
        );
        // הַשַּׁעַלְבֹנִי — a gentilic (`adj.gent`) whose gloss is the bare
        // abbreviation; it carded as "the adj.gent.".
        let card = bible.word_card("הַשַּׁעַלְבֹנִי")?.expect("gentilic exists");
        assert_eq!(card.gloss, NAME_GLOSS);
        assert!(!card.note.contains("adj.gent"), "note: {:?}", card.note);

        // Famous names the bridge mis-glosses are curated.
        for (surface, gloss) in [
            ("מֹשֶׁה", "Moses"),
            ("אַבְרָהָם", "Abraham"),
            ("שְׁלֹמֹה", "Solomon"),
            ("נֹחַ", "Noah"),
        ] {
            let card = bible.word_card(surface)?.expect("surface exists");
            assert_eq!(card.gloss, gloss, "curated name gloss for {surface}");
        }
        // A curated name still quizzes (its gloss is a real answer), and no
        // BDB citation leaks into the options.
        let card = bible.word_card("אַבְרָהָם")?.unwrap();
        assert!(!card.distractors.is_empty(), "curated names quiz normally");
        assert!(card.distractors.iter().all(|d| !d.contains("n.pr")));

        // Proclitic-prefixed curated names compose mechanically.
        let card = bible.word_card("לְיַעֲקֹב")?.expect("surface exists");
        assert_eq!(card.gloss, "to Jacob");
        Ok(())
    }

    /// The surface_meta cache flags proper names — BDB `n.pr` citations and
    /// the curated famous names — and verse words carry the flag out to the
    /// app so names render distinctly.
    #[test]
    fn surface_meta_flags_proper_names() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;
        bible.ensure_surface_meta()?;

        let flag = |s: &str| -> bool {
            bible
                .conn()
                .query_row(
                    "SELECT sm.is_name FROM progress.surface_meta sm \
                     JOIN hebrewdb.surface su ON su.surface_id = sm.surface_id \
                     WHERE su.text = ?1",
                    params![s],
                    |r| r.get::<_, i64>(0),
                )
                .expect("surface in meta")
                != 0
        };
        // The genealogy names that used to be scheduled early (their bridged
        // roots are common homographs), plus a curated famous name, plus the
        // pos-only-marked names and a gentilic the gloss sniff used to miss
        // (the Ezra 7:5 genealogy carded them as vocabulary and left them
        // uncoloured in the verse).
        for name in [
            "אֶזְבָּי",
            "חֶצְרוֹ",
            "נַעֲרַי",
            "מֹשֶׁה",
            "לְיַעֲקֹב",
            "אֶלְיַחְבָּא",
            "הַשַּׁעַלְבֹנִי",
            "אֲבִישׁוּעַ",
            "אֶלְעָזָר",
            "פִּינְחָס",
            "אַהֲרֹן",
            // Names BDB never resolves (blank card without the pre-filter's
            // `proper` class) or resolves with an empty `pos`.
            "עִדּוֹא",
            "חִלְקִיָּה",
            "אֱלִישָׁמָע",
        ] {
            assert!(flag(name), "{name} should be flagged as a name");
        }
        // Homograph collisions with rare place-names must stay vocabulary:
        // זָהָב "gold" (Di-zahab), אֶלֶף "thousand" (the city Eleph), בְּנֵי
        // "sons of" (Bene-jaakan) are all in the pre-filter's proper list —
        // as is הָרֹאשׁ "the chief" (via *Rosh* son of Benjamin), whose
        // vocabulary reading only shows after peeling the article.
        for word in ["דָּבָר", "אָדָם", "הָאָרֶץ", "זָהָב", "אֶלֶף", "בְּנֵי", "הָרֹאשׁ"]
        {
            assert!(!flag(word), "{word} should not be flagged as a name");
        }

        // A curated famous name keeps its usable BDB gloss on the card — the
        // name flag must not degrade it to "(a name)".
        let card = bible.word_card("אַהֲרֹן")?.expect("Aaron has a surface");
        assert_eq!(card.gloss, "Aaron");

        // The verse card carries the flags, aligned with its words.
        let (b, c, v): (u8, u8, u8) = bible.conn().query_row(
            "SELECT vw.book, vw.chapter, vw.verse FROM hebrewdb.verse_word vw \
             JOIN hebrewdb.surface s ON s.surface_id = vw.surface_id \
             WHERE s.text = 'אֶזְבָּי' LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let words = bible.verse_words_flagged(b, c, v)?;
        assert!(
            words.iter().any(|(w, is_name)| w == "אֶזְבָּי" && *is_name),
            "verse words should flag the name: {words:?}"
        );

        // A grammar card triggered by an obscure word illustrates with a
        // frequent non-name example instead (the article card that fired on
        // הַכַּרְמְלִי used to show "the the Carmelite").
        let example = bible.grammar_example_surface("article", "הַכַּרְמְלִי")?;
        assert_ne!(example, "הַכַּרְמְלִי");
        let card = bible.word_card(&example)?.expect("example has a card");
        eprintln!("article example: {example} -> {}", card.gloss);
        assert!(!card.gloss.is_empty());
        Ok(())
    }

    /// Attached proclitics are grammar layered onto vocabulary, not separate
    /// lexical starting points: the learner must meet עִם before וְעִם.
    #[test]
    fn prefixed_word_waits_for_bare_lexical_form() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;
        bible.ensure_surface_meta()?;

        let (bare_id, bare_key): (i64, String) = bible.conn().query_row(
            "SELECT sm.surface_id, sm.vkey FROM progress.surface_meta sm \
             JOIN hebrewdb.surface s ON s.surface_id = sm.surface_id \
             WHERE s.text = 'עִם'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let (prefixed_key, base_key): (String, String) = bible.conn().query_row(
            "SELECT sm.vkey, sm.base_vkey FROM progress.surface_meta sm \
             JOIN hebrewdb.surface s ON s.surface_id = sm.surface_id \
             WHERE s.text = 'וְעִם'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        assert_ne!(prefixed_key, bare_key);
        assert_eq!(base_key, bare_key);

        let ready = || -> rusqlite::Result<bool> {
            bible.conn().query_row(
                &format!(
                    "SELECT {LEXICAL_BASE_READY} FROM progress.surface_meta sm \
                     JOIN hebrewdb.surface s ON s.surface_id = sm.surface_id \
                     LEFT JOIN progress.word_srs base_ws ON base_ws.surface_id = sm.base_surface_id \
                     WHERE s.text = 'וְעִם'"
                ),
                [],
                |r| r.get(0),
            )
        };
        assert!(!ready()?, "prefixed form must initially be held back");

        bible.conn().execute(
            "INSERT INTO progress.word_srs(surface, surface_id, ease, interval_days, \
                due_epoch, reps, lapses, introduced_epoch, last_grade) \
             VALUES ('עִם', ?1, 2.5, 1, 0, 1, 0, 0, 2)",
            params![bare_id],
        )?;
        assert!(
            ready()?,
            "introducing the bare form unlocks its prefixed form"
        );
        Ok(())
    }

    /// Grammar concepts are introduced as gradeless cards before the words that
    /// use them, each at most once, and never block reaching a read.
    #[test]
    fn grammar_concepts_explained_once_before_words() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;
        bible.seed_known_alphabet(1_700_000_000)?; // focus on words/grammar
        // Exercise the concept-card mechanism with every rule available, rather
        // than waiting on the one-rule-at-a-time schedule.
        bible.set_tutor_settings(&TutorSettings {
            grammar_gating: false,
            ..Default::default()
        })?;

        let mut now = 1_700_000_000;
        let mut item = bible.next_study_item(now)?;
        let mut concepts: Vec<String> = Vec::new();
        let mut saw_read = false;
        for _ in 0..6000 {
            now += 5;
            item = match item {
                StudyItem::NewGlyph(g) | StudyItem::ReviewGlyph(g) => {
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                }
                StudyItem::NewWord(w) | StudyItem::ReviewWord(w) => {
                    bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
                }
                StudyItem::NewFormDrill(w) | StudyItem::ReviewFormDrill(w) => {
                    bible.submit_review(Track::Form, &w.surface, Grade::Good, now)?
                }
                StudyItem::NewSuffixDrill(s) | StudyItem::ReviewSuffixDrill(s) => {
                    bible.submit_review(Track::Suffix, &s.key, Grade::Good, now)?
                }
                StudyItem::ExplainGrammar(card) => {
                    // Content is populated and illustrated by the pending word.
                    assert!(!card.title.is_empty() && !card.explanation.is_empty());
                    assert!(!card.example.surface.is_empty());
                    concepts.push(card.concept.clone());
                    bible.next_study_item(now)?
                }
                StudyItem::ExplainMark(_)
                | StudyItem::ExplainFinalForms(_)
                | StudyItem::ExplainIntro(_) => bible.next_study_item(now)?,
                StudyItem::ReadVerse(_) => {
                    saw_read = true;
                    break;
                }
                StudyItem::Done => break,
            };
        }
        assert!(saw_read, "grammar cards must not block reaching a read");
        assert!(
            !concepts.is_empty(),
            "some grammar concept should be taught"
        );
        let mut seen = std::collections::HashSet::new();
        for c in &concepts {
            assert!(
                seen.insert(c.clone()),
                "concept {c:?} explained more than once"
            );
        }
        // Every grammar card shown is now on the reference list, with content,
        // in teaching (CONCEPTS) order.
        let reference = bible.seen_concepts()?;
        let grammar: Vec<&SeenConcept> = reference.iter().filter(|c| c.kind == "grammar").collect();
        assert_eq!(grammar.len(), concepts.len());
        for c in &grammar {
            assert!(seen.contains(&c.key), "unshown concept {:?} listed", c.key);
            assert!(!c.title.is_empty() && !c.explanation.is_empty());
        }
        let order: Vec<usize> = grammar
            .iter()
            .map(|c| {
                crate::grammar::concepts()
                    .iter()
                    .position(|g| g.key == c.key)
                    .unwrap()
            })
            .collect();
        assert!(order.is_sorted(), "reference not in teaching order");
        Ok(())
    }

    /// The three language-intro cards are the very first thing a fresh
    /// progress DB serves — in deck order, each exactly once — and they then
    /// appear on the [`Bible::seen_concepts`] reference list.
    #[test]
    fn intro_deck_served_first_and_once() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let now = 1_700_000_000;
        assert!(bible.seen_concepts()?.is_empty(), "nothing unlocked yet");
        for key in INTRO_CONCEPTS {
            match bible.next_study_item(now)? {
                StudyItem::ExplainIntro(k) => assert_eq!(k, key, "deck order"),
                other => panic!("expected intro card {key:?}, got {other:?}"),
            }
        }
        assert!(
            !matches!(bible.next_study_item(now)?, StudyItem::ExplainIntro(_)),
            "intro deck must not repeat"
        );
        let reference = bible.seen_concepts()?;
        let intro: Vec<&str> = reference
            .iter()
            .filter(|c| c.kind == "intro")
            .map(|c| c.key.as_str())
            .collect();
        assert_eq!(intro, INTRO_CONCEPTS);
        Ok(())
    }

    /// A suffixed preposition whose BDB consonant group holds only a
    /// cross-reference stub must not surface a card glossed with that stub.
    /// אֵלָי (pausal "to me") once carded as gloss "see אוּלַי", root אלח —
    /// the stub for אֻלַי, an unrelated lexeme sharing the א־ל־י skeleton.
    #[test]
    fn word_card_never_shows_cross_reference_stub() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let card = bible
            .word_card("אֵלָי")?
            .expect("pausal אֵלָי is a corpus surface");
        assert_eq!(card.gloss, "to me");
        assert_ne!(card.root, "אלח");
        // The distractor pool must not smuggle a stub in either.
        for d in &card.distractors {
            assert!(!d.starts_with("see "), "stub distractor {d:?}");
        }
        Ok(())
    }

    /// A meaning card headlines (and quizzes) the meaning of the surface
    /// being shown — proclitics and all — with the lexeme's base sense
    /// demoted to the `root_gloss` line: וְלַבָּיִת is "and to the house",
    /// not its root's "house".
    #[test]
    fn word_card_headlines_the_surface_meaning() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let card = bible
            .word_card("וְלַבָּיִת")?
            .expect("וְלַבָּיִת is a corpus surface");
        assert_eq!(card.gloss, "and to the house");
        assert_eq!(card.root_gloss, "house");

        // A bare form whose rendering adds nothing keeps the base sense as
        // the answer with no redundant root line.
        let card = bible.word_card("בַּיִת")?.expect("בַּיִת is a surface");
        assert_eq!(card.root_gloss, "");

        // A proclitic on a function word composes too — the primary analysis
        // curates אֲשֶׁר as "that", and the vav must remain in the result.
        let card = bible.word_card("וַאֲשֶׁר")?.expect("וַאֲשֶׁר is a surface");
        assert_eq!(card.gloss, "and that");
        assert_eq!(card.root_gloss, "that");

        // A curated gloss is the final learner meaning, served verbatim —
        // trimming it to one sense would lose the rest entirely (curated
        // cards carry no root-meaning line): אֵין keeps "there is not,
        // without", כִּי keeps all four of its senses.
        let card = bible.word_card("אֵין")?.expect("אֵין is a surface");
        assert_eq!(card.gloss, "there is not, without");
        let card = bible.word_card("כִּי")?.expect("כִּי is a surface");
        assert_eq!(card.gloss, "for, because, that, when");
        Ok(())
    }

    #[test]
    fn tutor_gloss_override_stats_and_optimization_use_upstream_card() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        // This correction has since landed in the checked-in overlay exactly,
        // while the second still changes what the learner sees.
        bible.set_tutor_gloss_override("כִּי", "for, because, that, when", "", 10)?;
        bible.set_tutor_gloss_override("בַּיִת", "dwelling", "", 20)?;
        assert_eq!(
            bible.tutor_gloss_override_stats()?,
            GlossOverrideStats {
                total: 2,
                redundant: 1,
            }
        );

        let result = bible.optimize_tutor_gloss_overrides(30)?;
        assert_eq!(result.removed, 1);
        assert_eq!(
            result.stats,
            GlossOverrideStats {
                total: 1,
                redundant: 0,
            }
        );
        assert_eq!(
            bible.word_card("כִּי")?.expect("כִּי is a surface").gloss,
            "for, because, that, when"
        );
        assert_eq!(
            bible.word_card("בַּיִת")?.expect("בַּיִת is a surface").gloss,
            "dwelling"
        );
        assert_eq!(
            bible.conn().query_row(
                "SELECT deleted FROM progress.gloss_overrides WHERE surface = 'כִּי'",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            1
        );

        // A later edit reactivates a tombstoned correction even if this
        // device's wall clock is behind the deletion timestamp.
        bible.set_tutor_gloss_override("כִּי", "because", "", 5)?;
        assert_eq!(
            bible.word_card("כִּי")?.expect("כִּי is a surface").gloss,
            "because"
        );
        Ok(())
    }

    #[test]
    fn mobile_lexicon_entry_override_updates_tutor_card_before_word_gloss_override()
    -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        bible.set_lexicon_entry_override("בָּרָא", "יצר", "fashion", 1)?;
        let card = bible.word_card("בָּרָא")?.expect("בָּרָא is a surface");
        assert_eq!(card.root, "יצר");
        assert_eq!(card.gloss, "he fashioned");
        assert_eq!(card.root_gloss, "fashion");

        // The learner-facing word-gloss layer remains the final override.
        bible.set_tutor_gloss_override("בָּרָא", "create afresh", "", 2)?;
        let card = bible.word_card("בָּרָא")?.expect("בָּרָא is a surface");
        assert_eq!(card.gloss, "create afresh");
        assert!(card.root_gloss.is_empty());
        Ok(())
    }

    #[test]
    fn conjunctive_imperfect_cards_and_teaches_its_form() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let surface = "וְיִבְחָר";
        let info = bible
            .hebrew_word_info(surface)
            .expect("conjunctive imperfect has a verb analysis");
        assert_eq!(info.tense.as_deref(), Some("Imperfect"));
        assert_eq!(crate::bible::inflected_gloss(&info), "and he will choose");
        assert!(crate::grammar::concepts_for(&info).contains(&"imperfect"));

        let card = bible.word_card(surface)?.expect("surface has a word card");
        assert_eq!(card.gloss, "and he will choose");
        assert_eq!(card.root_gloss, "choose");
        Ok(())
    }

    /// A segolate noun cards its own lexeme, not the verb BDB files first in
    /// the consonant group: מֶלֶךְ is "king", not "possess, own exclusively";
    /// חֹדֶשׁ is the curated "month", not "renew" (nor BDB's etymological
    /// headline "newness").
    #[test]
    fn word_card_segolate_noun_beats_verb_group_order() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let card = bible.word_card("מֶלֶךְ")?.expect("מֶלֶךְ is a surface");
        assert_eq!(card.gloss, "king", "got {card:?}");

        let card = bible.word_card("חֹדֶשׁ")?.expect("חֹדֶשׁ is a surface");
        assert_eq!(card.gloss, "month", "got {card:?}");
        Ok(())
    }

    /// Once a word's meaning is known, its grammatical form is drilled too: a
    /// gradeless-free "which form?" card whose answer is the inflected gloss and
    /// whose options contrast other inflections, graded on its own `form_srs`
    /// track and never shown for words with nothing to drill (tier < 2).
    #[test]
    fn form_is_drilled_after_meaning() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;
        bible.seed_known_alphabet(1_700_000_000)?;
        // Form drills need words with a drillable form (verbs, plural/construct
        // nouns); make every grammar rule available rather than waiting on the
        // unlock schedule.
        bible.set_tutor_settings(&TutorSettings {
            grammar_gating: false,
            ..Default::default()
        })?;

        let mut now = 1_700_000_000;
        let mut item = bible.next_study_item(now)?;
        let mut drilled = 0;
        for _ in 0..8000 {
            if drilled >= 3 {
                break;
            }
            now += 5;
            item = match item {
                StudyItem::NewGlyph(g) | StudyItem::ReviewGlyph(g) => {
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Easy, now)?
                }
                StudyItem::NewWord(w) | StudyItem::ReviewWord(w) => {
                    bible.submit_review(Track::Word, &w.surface, Grade::Easy, now)?
                }
                StudyItem::NewFormDrill(w) => {
                    // The answer is the inflected form; a real grammar tier.
                    assert!(!w.gloss.is_empty(), "form drill has an inflected answer");
                    assert!(
                        bible.form_srs(&w.surface)?.is_none(),
                        "row created on grade"
                    );
                    drilled += 1;
                    let after = bible.submit_review(Track::Form, &w.surface, Grade::Good, now)?;
                    // Grading created the form_srs row.
                    assert!(bible.form_srs(&w.surface)?.is_some());
                    after
                }
                StudyItem::ReviewFormDrill(w) => {
                    bible.submit_review(Track::Form, &w.surface, Grade::Good, now)?
                }
                StudyItem::NewSuffixDrill(s) | StudyItem::ReviewSuffixDrill(s) => {
                    bible.submit_review(Track::Suffix, &s.key, Grade::Good, now)?
                }
                StudyItem::ExplainMark(_)
                | StudyItem::ExplainGrammar(_)
                | StudyItem::ExplainFinalForms(_)
                | StudyItem::ExplainIntro(_) => bible.next_study_item(now)?,
                StudyItem::ReadVerse(_) => bible.next_study_item(now)?,
                StudyItem::Done => break,
            };
        }
        assert!(
            drilled >= 3,
            "form drills should appear once meanings are known"
        );
        Ok(())
    }

    #[test]
    fn form_tier_orders_forms_by_simplicity() {
        let verb = |binyan: &str, tense: &str, pgn: (&str, &str, &str)| HebrewWord {
            form: Some(binyan.to_string()),
            tense: Some(tense.to_string()),
            person: (!pgn.0.is_empty()).then(|| pgn.0.to_string()),
            gender: (!pgn.1.is_empty()).then(|| pgn.1.to_string()),
            number: (!pgn.2.is_empty()).then(|| pgn.2.to_string()),
            ..Default::default()
        };
        let noun = |number: Option<&str>, state: Option<&str>| HebrewWord {
            number: number.map(str::to_string),
            state: state.map(str::to_string),
            ..Default::default()
        };

        // A function word / proper noun has nothing to parse.
        assert_eq!(form_tier(&HebrewWord::default()), 0);

        // Nouns: singular absolute < plural < construct < suffixed.
        let sg = form_tier(&noun(Some("Singular"), Some("Absolute")));
        let pl = form_tier(&noun(Some("Plural"), Some("Absolute")));
        let cs = form_tier(&noun(Some("Singular"), Some("Construct")));
        let sfx = form_tier(&noun(None, Some("Sg + 3ms")));
        assert!(sg < pl && pl < cs && cs < sfx, "{sg} {pl} {cs} {sfx}");

        // Every noun is simpler than every verb.
        let qal_perf_3ms = form_tier(&verb("Qal", "Perfect", ("Third", "Masculine", "Singular")));
        assert!(sfx < qal_perf_3ms, "nouns rank below verbs");

        // Verbs: Qal perfect 3ms (citation-like) < other Qal PGN < imperfect
        // < imperative; a derived stem outranks Qal; a suffix/prefix add a step.
        let qal_perf_2ms = form_tier(&verb("Qal", "Perfect", ("Second", "Masculine", "Singular")));
        let qal_impf = form_tier(&verb(
            "Qal",
            "Imperfect",
            ("Third", "Masculine", "Singular"),
        ));
        let qal_impv = form_tier(&verb(
            "Qal",
            "Imperative",
            ("Second", "Masculine", "Singular"),
        ));
        let piel_perf = form_tier(&verb("Piel", "Perfect", ("Third", "Masculine", "Singular")));
        assert!(qal_perf_3ms < qal_perf_2ms, "3ms is the base perfect");
        assert!(qal_perf_2ms < qal_impf && qal_impf < qal_impv);
        assert!(qal_perf_3ms < piel_perf, "derived stem is harder than Qal");

        let mut with_suffix = verb("Qal", "Perfect", ("Third", "Masculine", "Singular"));
        with_suffix.obj_suffix = Some("3ms".to_string());
        assert!(
            form_tier(&with_suffix) > qal_perf_3ms,
            "object suffix adds a step"
        );
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
    fn grammar_priority_paces_concept_unlock_spacing() {
        // Higher grammar priority unlocks rules after fewer words.
        let mut s = TutorSettings {
            grammar_priority: 100,
            ..TutorSettings::default()
        };
        let fast = s.words_per_concept();
        s.grammar_priority = 0;
        let slow = s.words_per_concept();
        assert!(fast >= 1 && fast < slow, "fast {fast} slow {slow}");
    }

    fn open_with_progress() -> Option<Bible> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return None;
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")
            .unwrap();
        init_progress_schema(bible.conn()).unwrap();
        Some(bible)
    }

    #[test]
    fn settings_default_and_round_trip() -> rusqlite::Result<()> {
        let Some(bible) = open_with_progress() else {
            return Ok(());
        };
        // Unset → defaults.
        assert_eq!(bible.tutor_settings()?, TutorSettings::default());
        bible.conn().execute(
            "INSERT INTO progress.meta(key, value) VALUES ('setting.vocab_ratio', '80')",
            [],
        )?;
        let migrated = bible.tutor_settings()?;
        assert_eq!(migrated.vocab_priority, 80);
        assert_eq!(migrated.grammar_priority, 20);
        assert_eq!(
            migrated.verse_priority,
            TutorSettings::default().verse_priority
        );
        bible.conn().execute(
            "DELETE FROM progress.meta WHERE key = 'setting.vocab_ratio'",
            [],
        )?;
        let s = TutorSettings {
            letters_per_batch: 1,
            words_per_batch: 4,
            grammar_gating: false,
            vocab_priority: 10,
            grammar_priority: 90,
            verse_priority: 80,
            letters_ratio: 55,
        };
        bible.set_tutor_settings(&s)?;
        assert_eq!(bible.tutor_settings()?, s);
        // A reset clears meta, restoring defaults.
        bible.reset_tutor()?;
        assert_eq!(bible.tutor_settings()?, TutorSettings::default());
        Ok(())
    }

    #[test]
    fn progress_counts_verses_inside_grammar_frontier() -> rusqlite::Result<()> {
        let Some(bible) = open_with_progress() else {
            return Ok(());
        };
        for concept in crate::grammar::concepts()
            .iter()
            .filter(|concept| concept.key != "article")
        {
            bible.conn().execute(
                "INSERT INTO progress.concepts_unlocked(concept, unlocked_epoch) VALUES (?1, 0)",
                params![concept.key],
            )?;
        }

        let without_article: i64 = bible.conn().query_row(
            "SELECT COUNT(*) FROM hebrewdb.verse_stats WHERE article = 0",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(
            bible.tutor_progress()?.verses_grammar_unlocked,
            without_article
        );

        bible.set_tutor_settings(&TutorSettings {
            grammar_gating: false,
            ..TutorSettings::default()
        })?;
        let progress = bible.tutor_progress()?;
        assert_eq!(progress.verses_grammar_unlocked, progress.total_verses);
        Ok(())
    }

    #[test]
    fn grammar_frontier_expands_with_vocab_when_gated() -> rusqlite::Result<()> {
        let Some(bible) = open_with_progress() else {
            return Ok(());
        };
        let now = 1_700_000_000;
        let total = crate::grammar::concept_count() as i64;

        // Gating off → every rule available immediately.
        let off = TutorSettings {
            grammar_gating: false,
            ..Default::default()
        };
        assert_eq!(
            bible.unlocked_concepts(&off, now)?.count_ones() as i64,
            total
        );

        // Gating on, alphabet not known → no grammar rule at all (the learner
        // stays on grammar-free words while learning the letters).
        let s = TutorSettings::default();
        assert_eq!(bible.unlocked_concepts(&s, now)?, 0);

        // Alphabet known but no vocabulary → exactly one rule (chosen from
        // bucket 0 by verse coverage) unlocks.
        bible.seed_known_alphabet(now)?;
        let first = bible.unlocked_concepts(&s, now)?;
        assert_eq!(first.count_ones(), 1);

        // Graduating one `words_per_concept` batch of vocabulary unlocks exactly
        // one more rule — and the earlier unlock is kept (the set only grows).
        let per = s.words_per_concept();
        for i in 0..per {
            bible.conn().execute(
                "INSERT INTO progress.word_srs(surface, surface_id, ease, interval_days, \
                    due_epoch, reps, lapses, introduced_epoch, last_grade) \
                 VALUES (?1, ?2, 2.5, 1, 0, 3, 0, 0, 2)",
                params![format!("seed{i}"), 1_000_000 + i],
            )?;
        }
        let second = bible.unlocked_concepts(&s, now)?;
        assert_eq!(second.count_ones(), 2);
        assert_eq!(second & first, first, "unlocked set only grows");
        Ok(())
    }

    #[test]
    fn word_graduation_updates_readability_incrementally() -> rusqlite::Result<()> {
        let Some(bible) = open_with_progress() else {
            return Ok(());
        };
        bible.ensure_surface_meta()?;

        let (known_surface, known_id, known_vkey): (String, i64, String) = bible.conn().query_row(
            "SELECT s.text, s.surface_id, sm.vkey
                 FROM hebrewdb.surface s
                 JOIN progress.surface_meta sm ON sm.surface_id = s.surface_id
                 WHERE s.language IS NULL AND sm.is_name = 0
                 ORDER BY s.occurrences DESC
                 LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let (new_surface, new_id): (String, i64) = bible.conn().query_row(
            "SELECT s.text, s.surface_id
             FROM hebrewdb.surface s
             JOIN progress.surface_meta sm ON sm.surface_id = s.surface_id
             WHERE s.language IS NULL AND sm.is_name = 0 AND sm.vkey <> ?1
             ORDER BY s.occurrences DESC
             LIMIT 1",
            params![known_vkey],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;

        bible.conn().execute(
            "INSERT INTO progress.word_srs(surface, surface_id, ease, interval_days,
                due_epoch, reps, lapses, introduced_epoch, last_grade)
             VALUES (?1, ?2, 2.5, 1, 0, 3, 0, 0, 2)",
            params![known_surface, known_id],
        )?;
        bible.ensure_readability_progress()?;

        const SENTINEL_EPOCH: i64 = 123_456;
        bible.conn().execute(
            "UPDATE progress.surface_progress SET graduated_epoch = ?2
             WHERE surface_id IN (
                SELECT twin.surface_id FROM progress.surface_meta source
                JOIN progress.surface_meta twin ON twin.vkey = source.vkey
                WHERE source.surface_id = ?1)",
            params![known_id, SENTINEL_EPOCH],
        )?;

        let now = 1_700_000_000;
        for offset in 0..3 {
            let _ = bible.submit_review(Track::Word, &new_surface, Grade::Good, now + offset)?;
        }

        let preserved: i64 = bible.conn().query_row(
            "SELECT graduated_epoch FROM progress.surface_progress WHERE surface_id = ?1",
            params![known_id],
            |r| r.get(0),
        )?;
        assert_eq!(preserved, SENTINEL_EPOCH);
        let graduated_at: i64 = bible.conn().query_row(
            "SELECT graduated_epoch FROM progress.surface_progress WHERE surface_id = ?1",
            params![new_id],
            |r| r.get(0),
        )?;
        assert_eq!(graduated_at, now + 2);
        Ok(())
    }

    #[test]
    fn target_does_not_pin_a_locked_verse_for_its_name_alone() -> rusqlite::Result<()> {
        let Some(bible) = open_with_progress() else {
            return Ok(());
        };
        let now = 1_700_000_000;
        bible.seed_known_alphabet(now)?;
        bible.ensure_surface_meta()?;
        bible.conn().execute_batch(
            "INSERT INTO progress.concepts_unlocked(concept, unlocked_epoch)
                 VALUES ('conj-ve', 0);
             INSERT INTO progress.word_srs(surface, surface_id, ease, interval_days,
                    due_epoch, reps, lapses, introduced_epoch, last_grade)
                 SELECT text, surface_id, 2.5, 1, 0, 3, 0, 0, 3
                 FROM hebrewdb.surface WHERE text IN ('כִּי', 'לֹא', 'וְלֹא');",
        )?;
        bible.ensure_readability_progress()?;
        bible.set_tutor_settings(&TutorSettings {
            vocab_priority: 25,
            grammar_priority: 25,
            verse_priority: 75,
            ..TutorSettings::default()
        })?;
        let settings = bible.tutor_settings()?;
        let unlocked = bible.unlocked_concepts(&settings, now)?;

        // Numbers 21:31 has Israel (a name with no locked grammar) alongside
        // ordinary words behind locked rules. It used to win the target query
        // for the name, then `next_introduction` correctly refused to teach the
        // name from an incompletable verse. Dropping and reselecting that same
        // pin recursed forever.
        assert_ne!(
            bible.next_target_verse(unlocked, 0, false)?,
            Some((4, 21, 31))
        );
        Ok(())
    }

    #[test]
    fn grammar_locked_until_letters_known() -> rusqlite::Result<()> {
        let Some(bible) = open_with_progress() else {
            return Ok(());
        };
        let s = TutorSettings::default();
        let now = 1_700_000_000;
        // No letters yet → nothing unlocked.
        assert_eq!(bible.unlocked_concepts(&s, now)?, 0);
        // A partial alphabet (some consonants, no vowels) is still not "known".
        for g in ["א", "ל", "מ", "ר", "ב", "ן"] {
            bible.submit_review(Track::Glyph, g, Grade::Easy, now)?;
        }
        assert_eq!(
            bible.unlocked_concepts(&s, now)?,
            0,
            "grammar stays locked mid-alphabet"
        );
        Ok(())
    }

    #[test]
    fn letters_batch_throttles_new_glyphs() -> rusqlite::Result<()> {
        let Some(bible) = open_with_progress() else {
            return Ok(());
        };
        let s = TutorSettings {
            letters_per_batch: 2,
            ..Default::default()
        };
        bible.set_tutor_settings(&s)?;

        let mut now = 1_700_000_000;
        let mut item = bible.next_study_item(now)?;
        let mut max_in_learning = 0i64;
        let mut taught_glyphs = 0;
        for _ in 0..500 {
            max_in_learning = max_in_learning.max(bible.glyphs_in_learning()?);
            now += 5;
            item = match item {
                StudyItem::NewGlyph(g) => {
                    taught_glyphs += 1;
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                }
                StudyItem::ReviewGlyph(g) => {
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                }
                StudyItem::NewWord(w) | StudyItem::ReviewWord(w) => {
                    bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
                }
                StudyItem::NewFormDrill(w) | StudyItem::ReviewFormDrill(w) => {
                    bible.submit_review(Track::Form, &w.surface, Grade::Good, now)?
                }
                StudyItem::NewSuffixDrill(s) | StudyItem::ReviewSuffixDrill(s) => {
                    bible.submit_review(Track::Suffix, &s.key, Grade::Good, now)?
                }
                StudyItem::ExplainMark(_)
                | StudyItem::ExplainGrammar(_)
                | StudyItem::ExplainFinalForms(_)
                | StudyItem::ExplainIntro(_)
                | StudyItem::ReadVerse(_) => bible.next_study_item(now)?,
                StudyItem::Done => break,
            };
        }
        assert!(taught_glyphs >= 3, "should have taught several letters");
        assert!(
            max_in_learning <= 2,
            "letters batch (2) exceeded: {max_in_learning} glyphs in learning at once"
        );
        Ok(())
    }

    #[test]
    fn grammar_gate_only_introduces_unlocked_forms() -> rusqlite::Result<()> {
        let Some(bible) = open_with_progress() else {
            return Ok(());
        };
        let now0 = 1_700_000_000;
        bible.seed_known_alphabet(now0)?; // isolate grammar pacing from letters
        let s = TutorSettings::default(); // gating on, vocab-forward
        bible.set_tutor_settings(&s)?;

        let mut now = now0;
        let mut item = bible.next_study_item(now)?;
        let mut checked = 0;
        for _ in 0..800 {
            if checked >= 15 {
                break;
            }
            now += 5;
            item = match item {
                StudyItem::NewWord(w) => {
                    // At introduction, every grammar rule the word needs must be
                    // within the current unlock frontier.
                    let unlocked = bible.unlocked_concepts(&s, now)?;
                    let mask: i64 = bible.conn().query_row(
                        "SELECT concept_mask FROM progress.surface_meta WHERE surface_id = ?1",
                        params![w.surface_id],
                        |r| r.get(0),
                    )?;
                    assert!(
                        mask & !unlocked == 0,
                        "introduced a locked-rule word (mask {mask:#x} outside frontier {unlocked:#x})"
                    );
                    checked += 1;
                    bible.submit_review(Track::Word, &w.surface, Grade::Easy, now)?
                }
                StudyItem::ReviewWord(w) => {
                    bible.submit_review(Track::Word, &w.surface, Grade::Easy, now)?
                }
                StudyItem::NewGlyph(g) | StudyItem::ReviewGlyph(g) => {
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Easy, now)?
                }
                StudyItem::NewFormDrill(w) | StudyItem::ReviewFormDrill(w) => {
                    bible.submit_review(Track::Form, &w.surface, Grade::Easy, now)?
                }
                StudyItem::NewSuffixDrill(s) | StudyItem::ReviewSuffixDrill(s) => {
                    bible.submit_review(Track::Suffix, &s.key, Grade::Easy, now)?
                }
                StudyItem::ExplainMark(_)
                | StudyItem::ExplainGrammar(_)
                | StudyItem::ExplainFinalForms(_)
                | StudyItem::ExplainIntro(_)
                | StudyItem::ReadVerse(_) => bible.next_study_item(now)?,
                StudyItem::Done => break,
            };
        }
        assert!(checked >= 5, "should introduce several words to check");
        Ok(())
    }

    #[test]
    fn letters_ratio_bounds_are_word_or_letter_forward() -> rusqlite::Result<()> {
        let Some(bible) = open_with_progress() else {
            return Ok(());
        };
        // With no introductions logged yet: letters-forward wants a letter,
        // word-forward wants a word.
        assert!(bible.prefer_new_letter(&TutorSettings {
            letters_ratio: 100,
            ..Default::default()
        })?);
        assert!(!bible.prefer_new_letter(&TutorSettings {
            letters_ratio: 0,
            ..Default::default()
        })?);
        Ok(())
    }

    #[test]
    fn letters_ratio_prefers_words_over_racing_through_letters() -> rusqlite::Result<()> {
        let Some(bible) = open_with_progress() else {
            return Ok(());
        };
        // How many brand-new letters are introduced before the first word's
        // meaning is reached, under a given letters↔words ratio.
        let glyphs_before_first_word = |ratio: u8| -> rusqlite::Result<i64> {
            bible.reset_tutor()?;
            bible.set_tutor_settings(&TutorSettings {
                letters_ratio: ratio,
                ..Default::default()
            })?;
            let mut now = 1_700_000_000;
            let mut item = bible.next_study_item(now)?;
            let mut new_glyphs = 0;
            for _ in 0..3000 {
                now += 5;
                item = match item {
                    StudyItem::NewGlyph(g) => {
                        new_glyphs += 1;
                        bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                    }
                    StudyItem::ReviewGlyph(g) => {
                        bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                    }
                    StudyItem::NewWord(_) => break, // reached the first word meaning
                    StudyItem::ReviewWord(w) => {
                        bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
                    }
                    StudyItem::NewFormDrill(w) | StudyItem::ReviewFormDrill(w) => {
                        bible.submit_review(Track::Form, &w.surface, Grade::Good, now)?
                    }
                    StudyItem::NewSuffixDrill(s) | StudyItem::ReviewSuffixDrill(s) => {
                        bible.submit_review(Track::Suffix, &s.key, Grade::Good, now)?
                    }
                    _ => bible.next_study_item(now)?,
                };
            }
            Ok(new_glyphs)
        };
        let word_forward = glyphs_before_first_word(0)?;
        let letter_forward = glyphs_before_first_word(100)?;
        assert!(
            word_forward >= 1,
            "some letters must precede the very first word"
        );
        assert!(
            word_forward < letter_forward,
            "word-forward should reach a word with fewer new letters \
             ({word_forward}) than letter-forward ({letter_forward})"
        );
        Ok(())
    }

    #[test]
    fn letters_ratio_shifts_the_introduction_mix() -> rusqlite::Result<()> {
        let Some(bible) = open_with_progress() else {
            return Ok(());
        };
        // Long-run letter/word *totals* converge whatever the ratio (each item
        // needs the same reviews and the corpus fixes which letters a word
        // needs), so measure what the learner actually sees: how far the
        // alphabet has raced ahead by the time a fixed number of words is
        // known. Words-forward must hold back letters; letters-forward must
        // press on with them.
        let letters_at_five_words = |ratio: u8| -> rusqlite::Result<i64> {
            bible.reset_tutor()?;
            bible.set_tutor_settings(&TutorSettings {
                letters_ratio: ratio,
                ..Default::default()
            })?;
            let mut now = 1_700_000_000;
            let mut item = bible.next_study_item(now)?;
            for _ in 0..2000 {
                if bible.tutor_progress()?.words_known >= 5 {
                    return bible.conn().query_row(
                        "SELECT COUNT(*) FROM progress.glyph_srs",
                        [],
                        |r| r.get(0),
                    );
                }
                now += 5;
                item = match item {
                    StudyItem::NewGlyph(g) | StudyItem::ReviewGlyph(g) => {
                        bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                    }
                    StudyItem::NewWord(w) | StudyItem::ReviewWord(w) => {
                        bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
                    }
                    StudyItem::NewFormDrill(w) | StudyItem::ReviewFormDrill(w) => {
                        bible.submit_review(Track::Form, &w.surface, Grade::Good, now)?
                    }
                    StudyItem::NewSuffixDrill(s) | StudyItem::ReviewSuffixDrill(s) => {
                        bible.submit_review(Track::Suffix, &s.key, Grade::Good, now)?
                    }
                    StudyItem::ExplainMark(_)
                    | StudyItem::ExplainGrammar(_)
                    | StudyItem::ExplainFinalForms(_)
                    | StudyItem::ExplainIntro(_)
                    | StudyItem::ReadVerse(_) => bible.next_study_item(now)?,
                    StudyItem::Done => break,
                };
            }
            panic!("ratio {ratio}: never reached 5 known words (flow stalled?)");
        };
        let word_forward = letters_at_five_words(0)?;
        let letter_forward = letters_at_five_words(100)?;
        eprintln!(
            "glyphs seen at 5 words: words-forward={word_forward} letters-forward={letter_forward}"
        );
        assert!(
            word_forward < letter_forward,
            "words-forward should know 5 words with fewer glyphs seen \
             ({word_forward}) than letters-forward ({letter_forward})"
        );
        Ok(())
    }

    /// A fresh start has almost no introduced peers to quiz against: the first
    /// letter's review must still offer a full multiple-choice pool, topped up
    /// with *upcoming* glyphs (never-introduced, same kind), and a vowel's
    /// syllable quiz likewise builds from upcoming consonants and vowels.
    #[test]
    fn early_reviews_fall_back_on_upcoming_glyph_distractors() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        // Only one consonant introduced — no peers at all.
        let now = 1_700_000_000;
        bible.submit_review(Track::Glyph, "ס", Grade::Good, now)?;
        let card = bible.review_glyph_card("ס".to_string())?;
        assert_eq!(card.distractors.len(), 3, "topped up from upcoming glyphs");
        for d in &card.distractors {
            assert_ne!(d, "ס");
            assert!(
                d.chars().next().is_some_and(is_consonant),
                "same kind: {d:?}"
            );
            assert!(
                !bible.glyph_known(d)?,
                "upcoming = not yet introduced: {d:?}"
            );
        }

        // One consonant and one vowel known: the syllable pool can't be filled
        // from known glyphs alone, so upcoming ones complete it.
        bible.submit_review(Track::Glyph, "ֶ", Grade::Good, now)?;
        let card = bible.review_glyph_card("ֶ".to_string())?;
        let host = card.host.clone().expect("vowel gets a host");
        assert!(card.distractors.len() >= 3, "enough syllables for a quiz");
        for d in &card.distractors {
            let cps: Vec<char> = d.chars().collect();
            assert!(is_consonant(cps[0]) && !is_silent_host(&cps[0].to_string()));
            assert!(is_vowel_point(*cps.last().unwrap()));
            assert_ne!(*d, format!("{host}ֶ"), "excludes the correct syllable");
        }
        Ok(())
    }

    #[test]
    fn vowel_review_builds_random_syllable_distractors() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
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
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
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
        assert_eq!(s.letters_seen, 2, "two distinct consonants introduced");

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
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
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
        // Final forms are their own glyph — no longer folded to the medial base.
        assert_eq!(split_glyph_key("ךַ"), vec!["ך".to_string(), "ַ".to_string()]);
        Ok(())
    }

    #[test]
    fn glyph_decomposition_keeps_finals_distinct_and_dedups() {
        // מֶלֶךְ ends in a final kaf, taught as its own glyph (not folded to כ).
        let g = decompose_glyphs("מֶלֶךְ");
        let cons: Vec<&str> = g
            .iter()
            .filter(|c| c.is_consonant)
            .map(|c| c.glyph.as_str())
            .collect();
        assert_eq!(cons, vec!["מ", "ל", "ך"]);
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
        assert_eq!(
            cons,
            vec![format!("{BET}{DAGESH}"), RESH.to_string(), ALEF.to_string()]
        );

        // ש with a sin-dot is taught as sin, distinct from ש with a shin-dot.
        let sin = decompose_glyphs(&format!("{SHIN}{SIN_DOT}{QAMATS}{MEM}"));
        assert!(
            sin.iter()
                .any(|c| c.is_consonant && c.glyph == format!("{SHIN}{SIN_DOT}"))
        );
        let shin = decompose_glyphs(&format!("{SHIN}{SHIN_DOT}{QAMATS}{MEM}"));
        assert!(
            shin.iter()
                .any(|c| c.is_consonant && c.glyph == format!("{SHIN}{SHIN_DOT}"))
        );

        // A genuinely dotless shin (the silent second shin of יִשָּׂשכָר, or a
        // Leningrad scribal omission like אִיש in Deut 24:16) folds into the
        // standard שׁ — a bare ש is never introduced as its own glyph.
        const YOD: char = '\u{05D9}';
        const KAF: char = '\u{05DB}';
        const HIRIQ: char = '\u{05B4}';
        let issachar = decompose_glyphs(&format!(
            "{YOD}{HIRIQ}{SHIN}{QAMATS}{DAGESH}{SIN_DOT}{SHIN}{KAF}{QAMATS}{RESH}"
        ));
        let cons: Vec<&str> = issachar
            .iter()
            .filter(|c| c.is_consonant)
            .map(|c| c.glyph.as_str())
            .collect();
        assert_eq!(
            cons,
            vec![
                YOD.to_string(),
                format!("{SHIN}{SIN_DOT}"),
                format!("{SHIN}{SHIN_DOT}"),
                KAF.to_string(),
                RESH.to_string()
            ],
            "dotless shin folds to שׁ, never a bare ש glyph"
        );

        // A geminated shin (dagesh forte, e.g. the assimilated definite
        // article in אַשּׁוּר/הַשּׁוֹפָר) carries *both* a dagesh and a shin/sin
        // dot, in that order — the dagesh must not stop the scan from finding
        // the dot, or the doubled letter is mistaught as a dotless bare שׁ.
        let geminated = decompose_glyphs(&format!(
            "{ALEF}{PATAH}{SHIN}{DAGESH}{SHIN_DOT}{QAMATS}{RESH}"
        ));
        let cons: Vec<&str> = geminated
            .iter()
            .filter(|c| c.is_consonant)
            .map(|c| c.glyph.as_str())
            .collect();
        assert_eq!(
            cons,
            vec![
                ALEF.to_string(),
                format!("{SHIN}{SHIN_DOT}"),
                RESH.to_string()
            ]
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
            vec![
                ALEF.to_string(),
                format!("{SHIN}{SHIN_DOT}"),
                MEM.to_string(),
                RESH.to_string()
            ],
            "vowel-before-dagesh/dot ordering must still fold the shin/sin dot in"
        );
        assert!(
            real_order
                .iter()
                .any(|c| !c.is_consonant && c.glyph == QAMATS.to_string()),
            "the vowel sitting between the letter and its dot is still taught"
        );

        // A vowel-less vav with a dagesh is the shureq vowel (וּ → "u") — its
        // own glyph, so a word like סוּס gates on knowing the shureq, not just
        // its consonants.
        const VAV: char = '\u{05D5}';
        const SAMEKH: char = '\u{05E1}';
        let sus = decompose_glyphs(&format!("{SAMEKH}{VAV}{DAGESH}{SAMEKH}")); // סוּס
        let glyphs: Vec<&str> = sus.iter().map(|c| c.glyph.as_str()).collect();
        assert_eq!(glyphs, vec![SAMEKH.to_string(), format!("{VAV}{DAGESH}")]);
        // …while a vav with a vowel of its own is a geminated consonant
        // (dagesh chazak, e.g. חַוָּה) and stays plain ו.
        const HET: char = '\u{05D7}';
        const HE: char = '\u{05D4}';
        let gem_vav = decompose_glyphs(&format!("{HET}{PATAH}{VAV}{QAMATS}{DAGESH}{HE}{QAMATS}"));
        assert!(gem_vav.iter().any(|c| c.glyph == VAV.to_string()));
        assert!(!gem_vav.iter().any(|c| c.glyph == format!("{VAV}{DAGESH}")));
        // Grading the shureq credits it as one atomic glyph.
        assert_eq!(
            split_glyph_key(&format!("{VAV}{DAGESH}")),
            vec![format!("{VAV}{DAGESH}")]
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
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let now = 1_700_000_000;
        // The one-time language-intro deck comes first, then teaching begins.
        for key in INTRO_CONCEPTS {
            match bible.next_study_item(now)? {
                StudyItem::ExplainIntro(k) => assert_eq!(k, key),
                other => panic!("expected intro card {key:?}, got {other:?}"),
            }
        }
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
                StudyItem::NewFormDrill(w) | StudyItem::ReviewFormDrill(w) => {
                    bible.submit_review(Track::Form, &w.surface, Grade::Good, now)?
                }
                StudyItem::NewSuffixDrill(s) | StudyItem::ReviewSuffixDrill(s) => {
                    bible.submit_review(Track::Suffix, &s.key, Grade::Good, now)?
                }
                StudyItem::ExplainMark(_) => {
                    // Gradeless, like ReadVerse: acknowledged just by asking
                    // for the next item.
                    saw_mark = true;
                    bible.next_study_item(now)?
                }
                StudyItem::ExplainGrammar(_)
                | StudyItem::ExplainFinalForms(_)
                | StudyItem::ExplainIntro(_) => bible.next_study_item(now)?,
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
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
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
                StudyItem::NewFormDrill(w) | StudyItem::ReviewFormDrill(w) => {
                    bible.submit_review(Track::Form, &w.surface, Grade::Good, now)?
                }
                StudyItem::NewSuffixDrill(s) | StudyItem::ReviewSuffixDrill(s) => {
                    bible.submit_review(Track::Suffix, &s.key, Grade::Good, now)?
                }
                StudyItem::ExplainMark(_)
                | StudyItem::ExplainGrammar(_)
                | StudyItem::ExplainFinalForms(_)
                | StudyItem::ExplainIntro(_) => bible.next_study_item(now)?,
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
        let mut saw_misread_review =
            matches!(&after, StudyItem::ReviewWord(w) if w.surface == misread);
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
                StudyItem::NewFormDrill(w) | StudyItem::ReviewFormDrill(w) => {
                    bible.submit_review(Track::Form, &w.surface, Grade::Good, now)?
                }
                StudyItem::NewSuffixDrill(s) | StudyItem::ReviewSuffixDrill(s) => {
                    bible.submit_review(Track::Suffix, &s.key, Grade::Good, now)?
                }
                StudyItem::ExplainMark(_)
                | StudyItem::ExplainGrammar(_)
                | StudyItem::ExplainFinalForms(_)
                | StudyItem::ExplainIntro(_) => bible.next_study_item(now)?,
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
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
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
                StudyItem::NewFormDrill(w) | StudyItem::ReviewFormDrill(w) => {
                    bible.submit_review(Track::Form, &w.surface, Grade::Good, now)?
                }
                StudyItem::NewSuffixDrill(s) | StudyItem::ReviewSuffixDrill(s) => {
                    bible.submit_review(Track::Suffix, &s.key, Grade::Good, now)?
                }
                StudyItem::ExplainMark(_)
                | StudyItem::ExplainGrammar(_)
                | StudyItem::ExplainFinalForms(_)
                | StudyItem::ExplainIntro(_) => bible.next_study_item(now)?,
                StudyItem::ReadVerse(_) | StudyItem::Done => break,
            };
        }
        assert!(
            new_words.len() >= 2,
            "a second word should be introduced while the first is stuck mid-learning"
        );
        Ok(())
    }

    /// A lapsed word (`Grade::Again`) isn't due again for a full learning step
    /// ([`LEARN_STEPS_MIN`]), so it shouldn't be handed straight back as the
    /// very next card — that drills the same word back-to-back instead of
    /// resting it and teaching something else in the meantime. Regression
    /// test for the `next_study_item_impl` stall fallback.
    ///
    /// Constructs the narrow-pool scenario directly (via `progress.word_srs`)
    /// rather than relying on natural pacing to reach it: the pinned target
    /// verse's other words are all pre-graduated ("done"), leaving exactly one
    /// unfinished word, which is freshly introduced and then failed. With no
    /// other candidate in *this* verse, the old code fell straight through to
    /// pull-forward, which ignores `due_epoch` and so handed the same word
    /// straight back.
    #[test]
    fn lapsed_word_is_not_immediately_re_served() -> rusqlite::Result<()> {
        let Some(bible) = open_with_progress() else {
            return Ok(());
        };
        bible.seed_known_alphabet(1_700_000_000)?; // isolate word pacing from letters
        bible.set_tutor_settings(&TutorSettings {
            grammar_gating: false, // pin a verse without waiting on grammar unlocks
            ..Default::default()
        })?;

        let now0 = 1_700_000_000;
        bible.ensure_surface_meta()?;
        bible.ensure_readability_progress()?;
        let all_grammar = crate::grammar::all_concepts_mask();
        let (b, c, v) = bible
            .next_target_verse(all_grammar, 0, false)?
            .expect("a target verse exists");
        let words = bible.unfinished_words((b, c, v), all_grammar, 0, false, true)?;
        assert!(
            !words.is_empty(),
            "target verse should have unfinished words"
        );
        let (last, rest) = words.split_last().expect("at least one word");

        // Graduate every other word in the verse outright.
        for surface in rest {
            let surface_id: i64 = bible.conn().query_row(
                "SELECT surface_id FROM hebrewdb.surface WHERE text = ?1",
                params![surface],
                |r| r.get(0),
            )?;
            bible.conn().execute(
                "INSERT INTO progress.word_srs(surface, surface_id, ease, interval_days, \
                    due_epoch, reps, lapses, introduced_epoch, last_grade) \
                 VALUES (?1, ?2, 2.5, 10, ?3, 3, 0, ?4, 3)",
                params![surface, surface_id, now0 + 10 * SECONDS_PER_DAY, now0],
            )?;
        }
        // The test seeds SRS rows directly, bypassing `submit_review`'s
        // incremental surface/verse cache update. Refresh the derived state so
        // selection sees those words as graduated, as it would in production.
        bible.ensure_readability_progress()?;
        bible.set_meta_target(Some((b, c, v)))?;

        // Reach the one remaining word and fail it.
        let mut now = now0;
        let mut item = bible.next_study_item(now)?;
        let mut hops = 0;
        let failed_surface = loop {
            hops += 1;
            assert!(hops < 20, "should reach the remaining word quickly");
            match item {
                StudyItem::NewWord(w) if w.surface == *last => break w.surface,
                StudyItem::ExplainGrammar(_)
                | StudyItem::ExplainMark(_)
                | StudyItem::ExplainFinalForms(_)
                | StudyItem::ExplainIntro(_) => {
                    item = bible.next_study_item(now)?;
                }
                other => panic!("unexpected item before the target word: {other:?}"),
            }
        };
        now += 5;
        let next = bible.submit_review(Track::Word, &failed_surface, Grade::Again, now)?;
        match next {
            StudyItem::ReviewWord(w) | StudyItem::NewWord(w) => assert_ne!(
                w.surface, failed_surface,
                "a just-lapsed word shouldn't be re-served before its due time"
            ),
            _ => {}
        }
        Ok(())
    }

    #[test]
    fn reading_mark_is_explained_once_and_never_drilled() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        // Walk well past the first readable verse, acknowledging reads and
        // collecting every explained mark: which reading marks a verse needs
        // depends on verse selection, so the invariant under test is that each
        // distinct mark is explained *at most once* (never re-explained, never
        // drilled), not that exactly one appears.
        let now = 1_700_000_000;
        let mut item = bible.next_study_item(now)?;
        let mut explained: Vec<String> = Vec::new();
        let mut reads = 0;
        for _ in 0..8000 {
            item = match item {
                StudyItem::NewGlyph(g) | StudyItem::ReviewGlyph(g) => {
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                }
                StudyItem::NewWord(w) | StudyItem::ReviewWord(w) => {
                    bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
                }
                StudyItem::NewFormDrill(w) | StudyItem::ReviewFormDrill(w) => {
                    bible.submit_review(Track::Form, &w.surface, Grade::Good, now)?
                }
                StudyItem::NewSuffixDrill(s) | StudyItem::ReviewSuffixDrill(s) => {
                    bible.submit_review(Track::Suffix, &s.key, Grade::Good, now)?
                }
                StudyItem::ExplainMark(g) => {
                    explained.push(g.glyph.clone());
                    bible.next_study_item(now)?
                }
                StudyItem::ExplainGrammar(_)
                | StudyItem::ExplainFinalForms(_)
                | StudyItem::ExplainIntro(_) => bible.next_study_item(now)?,
                StudyItem::ReadVerse(_) => {
                    reads += 1;
                    if reads >= 3 {
                        break;
                    }
                    bible.next_study_item(now)?
                }
                StudyItem::Done => break,
            };
        }
        assert!(
            !explained.is_empty(),
            "some reading mark should be explained"
        );
        let mut seen = std::collections::HashSet::new();
        for mark in &explained {
            assert!(
                seen.insert(mark.clone()),
                "reading mark {mark:?} was explained more than once"
            );
        }
        // Never entered the drilled-glyph store, so it never comes up for review.
        for mark in READING_MARKS {
            assert!(!bible.glyph_known(&mark.to_string())?);
        }
        Ok(())
    }

    /// Function words never reach the reverse-parser, but curated preposition
    /// surfaces still issue their grammar card (once) through the surface-
    /// concept table: עַל explains the standalone-preposition concept, לוֹ the
    /// inseparable לְ.
    #[test]
    fn function_word_prepositions_issue_grammar_cards() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let now = 1_700_000_000;
        // עַל resolves only through the gloss bridge — no morphology for
        // concepts_for to classify; the card must come from the surface table.
        let w = bible.hebrew_word_info("עַל").expect("bridge gloss");
        assert!(w.form.is_none() && w.tense.is_none() && w.state.is_none());
        match bible.next_grammar_card("עַל", now)? {
            Some(StudyItem::ExplainGrammar(card)) => {
                assert_eq!(card.concept, "preposition");
                assert_eq!(card.example.surface, "עַל");
            }
            other => panic!("expected the preposition card for עַל, got {other:?}"),
        }
        // Shown at most once.
        assert!(bible.next_grammar_card("אֶל", now)?.is_none());

        // A suffixed inseparable preposition explains its prefix card first,
        // then the pronoun-ending card on the next call — both gate it.
        match bible.next_grammar_card("לוֹ", now)? {
            Some(StudyItem::ExplainGrammar(card)) => assert_eq!(card.concept, "prep-le"),
            other => panic!("expected the prep-le card for לוֹ, got {other:?}"),
        }
        match bible.next_grammar_card("לוֹ", now)? {
            Some(StudyItem::ExplainGrammar(card)) => assert_eq!(card.concept, "prep-suffix"),
            other => panic!("expected the prep-suffix card for לוֹ, got {other:?}"),
        }
        // Once seen, the whole suffixed family shares the card: the pausal
        // form אֵלָי (a distinct vocab_key from אֵלַי) introduces nothing new
        // beyond it — the preposition card was issued for עַל above.
        assert!(bible.next_grammar_card("אֵלָי", now)?.is_none());
        Ok(())
    }

    /// The first final-form glyph (ך ם ן ף ץ) is gated behind the one-time
    /// final-forms concept card: the explanation appears before any final form
    /// is introduced, exactly once, and the finals themselves are still drilled
    /// as their own glyphs afterwards.
    #[test]
    fn final_forms_explained_once_before_first_final_glyph() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
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
        let mut explains = 0;
        let mut finals_introduced = 0;
        for _ in 0..4000 {
            item = match item {
                StudyItem::NewGlyph(g) | StudyItem::ReviewGlyph(g) => {
                    if g.glyph.chars().next().is_some_and(is_final_form) {
                        assert_eq!(
                            explains, 1,
                            "final form {:?} introduced without the concept card",
                            g.glyph
                        );
                        finals_introduced += 1;
                    }
                    bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
                }
                StudyItem::ExplainFinalForms(g) => {
                    explains += 1;
                    assert!(
                        g.glyph.chars().next().is_some_and(is_final_form),
                        "concept card should carry the final form about to be taught, got {:?}",
                        g.glyph
                    );
                    assert_eq!(finals_introduced, 0, "explanation must come first");
                    bible.next_study_item(now)?
                }
                StudyItem::NewWord(w) | StudyItem::ReviewWord(w) => {
                    bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
                }
                StudyItem::NewFormDrill(w) | StudyItem::ReviewFormDrill(w) => {
                    bible.submit_review(Track::Form, &w.surface, Grade::Good, now)?
                }
                StudyItem::NewSuffixDrill(s) | StudyItem::ReviewSuffixDrill(s) => {
                    bible.submit_review(Track::Suffix, &s.key, Grade::Good, now)?
                }
                StudyItem::ExplainMark(_)
                | StudyItem::ExplainGrammar(_)
                | StudyItem::ExplainIntro(_) => bible.next_study_item(now)?,
                StudyItem::ReadVerse(_) => bible.next_study_item(now)?,
                StudyItem::Done => break,
            };
            if finals_introduced >= 2 {
                break;
            }
        }
        assert_eq!(explains, 1, "final-forms card should be shown exactly once");
        assert!(
            finals_introduced >= 1,
            "a final-form glyph should still be introduced after the card"
        );
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
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
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

    #[test]
    fn needs_onboarding_only_before_any_progress() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        assert!(bible.needs_onboarding()?);
        bible.submit_review(Track::Glyph, "מ", Grade::Good, 1_700_000_000)?;
        assert!(!bible.needs_onboarding()?);
        Ok(())
    }

    /// Self-reporting a known alphabet must graduate every glyph the ordinary
    /// curriculum would otherwise teach one at a time, so the learner never sees
    /// a glyph card — the first cards are a word's grammar concept(s) and then
    /// the word itself, never a `NewGlyph`/`ReviewGlyph`.
    #[test]
    fn seed_known_alphabet_skips_glyph_teaching() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let mut now = 1_700_000_000;
        bible.seed_known_alphabet(now)?;
        assert!(bible.glyph_known("א")?, "aleph should be seeded");
        assert!(bible.all_glyphs_graduated("בְּרֵאשִׁית")?);

        // A learner who already reads Hebrew skips the how-to-read cards
        // entirely, but they still land on the reference page.
        for key in INTRO_CONCEPTS.iter().copied().chain([FINAL_FORMS_CONCEPT]) {
            assert!(bible.concept_seen(key)?, "{key} should be seeded as seen");
        }

        // The first card is a grammar concept or the word meaning — neither a
        // glyph nor a how-to-read intro is ever taught. Advance through any
        // leading grammar cards to the word.
        let mut item = bible.next_study_item(now)?;
        for _ in 0..30 {
            match item {
                StudyItem::NewWord(_) => return Ok(()),
                StudyItem::ExplainGrammar(_) => {
                    now += 5;
                    item = bible.next_study_item(now)?;
                }
                StudyItem::ExplainIntro(_) => {
                    panic!("a seeded alphabet must never show an intro card, got {item:?}")
                }
                StudyItem::NewGlyph(_) | StudyItem::ReviewGlyph(_) => {
                    panic!("a seeded alphabet must never teach a glyph, got {item:?}")
                }
                other => panic!("expected a word or grammar card, got {other:?}"),
            }
        }
        panic!("did not reach a word after seeding the alphabet");
    }

    /// Calibration probes return progressively easier (more common
    /// rarest-word) verses as `tier` shrinks toward 0, and harder ones as it
    /// grows — the property the app's binary search relies on. Critically,
    /// every tier must be genuinely distinct (see
    /// [`Bible::calibration_tier_count`]'s doc comment for why raw vocabulary
    /// rank fails this): neighbouring tiers never probe the same verse.
    #[test]
    fn calibration_probe_difficulty_tracks_tier_with_no_plateau() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let count = bible.calibration_tier_count()?;
        assert!(count > 1, "corpus should have multiple difficulty tiers");

        let mut prev = bible.calibration_probe(0)?.expect("tier 0 should resolve");
        for tier in 1..count {
            let probe = bible
                .calibration_probe(tier)?
                .unwrap_or_else(|| panic!("tier {tier} should resolve"));
            assert!(
                probe.min_occurrences < prev.min_occurrences,
                "tier {tier} must be strictly harder than the previous tier"
            );
            prev = probe;
        }
        // Past the last tier there is nothing left to calibrate.
        assert!(bible.calibration_probe(count)?.is_none());
        Ok(())
    }

    /// Finishing calibration seeds every word at least as common as the
    /// confirmed threshold as already known, and nothing rarer.
    #[test]
    fn seed_known_vocab_marks_words_at_or_above_the_threshold_known() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let now = 1_700_000_000;
        // A mid-corpus probe's exact threshold — real data, not a made-up cutoff.
        let probe = bible
            .calibration_probe(5)?
            .expect("tier 5 should resolve on real data");
        let threshold = probe.min_occurrences;
        bible.seed_known_vocab(threshold, now)?;

        // Done-ness folds spelling twins through `surface_meta.vkey`, so the
        // expected count is distinct keys, not raw surfaces — and the cache
        // must exist for the fold to see the seeded rows.
        bible.ensure_surface_meta()?;
        let known: i64 = bible.conn().query_row(
            &format!("SELECT COUNT(*) FROM ({DONE_SURFACES})"),
            [],
            |r| r.get(0),
        )?;
        let expected: i64 = bible.conn().query_row(
            "SELECT COUNT(DISTINCT sm.vkey) FROM progress.surface_meta sm \
             JOIN hebrewdb.surface s ON s.surface_id = sm.surface_id \
             WHERE s.occurrences >= ?1",
            params![threshold],
            |r| r.get(0),
        )?;
        assert_eq!(known, expected);
        assert!(known > 0);

        // The single rarest word in the corpus must not be seeded.
        let rarest = bible
            .conn()
            .query_row(
                "SELECT text FROM hebrewdb.surface WHERE language IS NULL \
                 ORDER BY occurrences ASC LIMIT 1",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        if let Some(rarest) = rarest {
            assert!(bible.word_srs(&rarest)?.is_none());
        }
        Ok(())
    }

    /// A no-op cutoff (nothing ever confirmed readable) must not seed anything.
    #[test]
    fn seed_known_vocab_with_zero_threshold_seeds_nothing() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        bible.seed_known_vocab(0, 1_700_000_000)?;
        let known: i64 = bible.conn().query_row(
            &format!("SELECT COUNT(*) FROM ({DONE_SURFACES})"),
            [],
            |r| r.get(0),
        )?;
        assert_eq!(known, 0);
        Ok(())
    }

    /// Article-prefixed nouns are gated behind the definite-article concept
    /// card — not the spurious verb readings the reverse-parser also carries
    /// for them (הַמֶּלֶךְ as a he-peeled imperative of הלך, הַיּוֹם as a
    /// Piel infinitive of *הימ). Each must classify as exactly ["article"]
    /// (the article's own early rank, not a verb concept's) and issue the
    /// article card, once, before being introduced as a word.
    #[test]
    fn article_words_gate_behind_the_article_card() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        let now = 1_700_000_000;
        for s in ["הַמֶּלֶךְ", "הָאָרֶץ", "הַשָּׁמַיִם", "הָעָם", "הַיּוֹם", "הַדָּבָר"]
        {
            let w = bible.hebrew_word_info(s);
            // The chosen reading is the article+noun one, not a verb.
            let parsed = w.as_ref().unwrap_or_else(|| panic!("{s} should parse"));
            assert!(
                parsed.form.is_none() && parsed.tense.is_none(),
                "{s} should read as article + noun, got verb {:?} {:?}",
                parsed.form,
                parsed.tense
            );
            let concepts = crate::grammar::concepts_for_surface(s, w.as_ref());
            assert_eq!(concepts, vec!["article"], "concepts for {s}");
            let article_rank = crate::grammar::concepts()
                .iter()
                .position(|c| c.key == "article")
                .expect("article concept exists") as i64;
            assert_eq!(
                crate::grammar::concept_rank_for_surface(s, w.as_ref()),
                article_rank,
                "rank for {s}"
            );
            match bible.next_grammar_card(s, now)? {
                Some(StudyItem::ExplainGrammar(card)) => {
                    assert_eq!(card.concept, "article", "card for {s}");
                    assert_eq!(card.example.surface, s);
                }
                other => panic!("expected the article card for {s}, got {other:?}"),
            }
            // Shown at most once; reset so each surface is tested independently.
            assert!(bible.next_grammar_card(s, now)?.is_none());
            bible
                .conn()
                .execute("DELETE FROM progress.concepts_seen", [])?;
        }

        // Article + participle is real Hebrew — the participle keeps its verb
        // reading (and its participle concept) rather than flattening to a
        // noun: הַיֹּשֵׁב "the one dwelling".
        let w = bible.hebrew_word_info("הַיֹּשֵׁב").expect("participle parses");
        assert!(
            w.tense.as_deref().is_some_and(|t| t.contains("Participle")),
            "הַיֹּשֵׁב should keep its participle reading, got {w:?}"
        );

        // A hataf-patah he is the interrogative, not the article — the verb
        // reading is genuine and must survive even though a noun reading
        // resolves (הֲתֵלֵךְ "will you go?", not תֵּל "your mound").
        let w = bible
            .hebrew_word_info("הֲתֵלֵךְ")
            .expect("interrogative parses");
        assert_eq!(
            (w.root.as_str(), w.tense.as_deref()),
            ("הלכ", Some("Imperfect")),
            "הֲתֵלֵךְ should keep its interrogative verb reading, got {w:?}"
        );
        Ok(())
    }

    /// The direct-object marker אֶת — the most common word in the Bible — is a
    /// function word: it must be gated behind (and issue) the object-marker
    /// concept card rather than being introduced as an ordinary vocabulary
    /// word, and its whole family (אֵת, וְאֶת, suffixed אֹתוֹ/אֹתָם/אֶתְכֶם)
    /// shares the one card.
    #[test]
    fn object_marker_gates_behind_its_grammar_card() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;

        // Every family member classifies to the object-marker concept (so none
        // of them is rank −1 / ungated any more), through vocab_key matching
        // even without a parse.
        for s in ["אֶת", "אֵת", "אֹתוֹ", "אוֹתָם", "אֶתְכֶם", "אֹתָהּ"]
        {
            assert_eq!(
                crate::grammar::concepts_for_surface(s, None),
                vec!["object-marker"],
                "concepts for {s}"
            );
        }
        // The vav-prefixed forms note the conjunction first, then the marker.
        assert_eq!(
            crate::grammar::concepts_for_surface("וְאֶת", None),
            vec!["conj-ve", "object-marker"]
        );

        // אֶת is the very first concept in teaching order — unlocked the
        // moment the alphabet is done.
        let w = bible.hebrew_word_info("אֶת");
        assert_eq!(
            crate::grammar::concept_rank_for_surface("אֶת", w.as_ref()),
            0,
            "the object marker should be the first-ranked concept"
        );

        // The card is issued once for the first family member met, then never
        // again — the rest introduce as plain words.
        let now = 1_700_000_000;
        match bible.next_grammar_card("אֶת", now)? {
            Some(StudyItem::ExplainGrammar(card)) => {
                assert_eq!(card.concept, "object-marker");
                assert_eq!(card.example.surface, "אֶת");
            }
            other => panic!("expected the object-marker card for אֶת, got {other:?}"),
        }
        assert!(bible.next_grammar_card("אֹתוֹ", now)?.is_none());
        assert!(bible.next_grammar_card("אֵת", now)?.is_none());

        // The אִתּ־ "with" forms are the (suffixed) preposition, not the
        // object marker.
        assert_eq!(
            crate::grammar::concepts_for_surface("אִתְּכֶם", None),
            vec!["preposition", "prep-suffix"]
        );
        Ok(())
    }

    /// Pronominal endings are drilled highlighted on known host words: once
    /// the prep-suffix concept has been shown and a suffixed word has
    /// graduated, the ending gets its own SRS card (stem + suffix split for
    /// the app's red highlight), reviews rotate hosts, and nothing is
    /// introduced before the concept card.
    #[test]
    fn pronoun_endings_drill_on_known_hosts() -> rusqlite::Result<()> {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("hebrew.db").exists() {
            return Ok(());
        }
        let bible = Bible::open(&data).expect("open data dbs");
        bible
            .conn()
            .execute_batch("ATTACH DATABASE ':memory:' AS progress")?;
        init_progress_schema(bible.conn())?;
        let now = 1_700_000_000;

        // Graduate two suffixed hosts: לוֹ (3ms) and אֵלַי (1cs).
        for surface in ["לוֹ", "אֵלַי"] {
            let id: i64 = bible.conn().query_row(
                "SELECT surface_id FROM hebrewdb.surface WHERE text = ?1",
                params![surface],
                |r| r.get(0),
            )?;
            bible.conn().execute(
                "INSERT INTO progress.word_srs(surface, surface_id, ease, interval_days, \
                    due_epoch, reps, lapses, introduced_epoch, last_grade) \
                 VALUES (?1, ?2, 2.5, 6, ?3, 3, 0, ?4, 2)",
                params![surface, id, now + 999_999, now],
            )?;
        }

        // No drill before the concept card has been shown.
        assert!(bible.next_suffix_introduction(now)?.is_none());
        bible.conn().execute(
            "INSERT INTO progress.concepts_seen(concept, introduced_epoch) VALUES ('prep-suffix', ?1)",
            params![now],
        )?;

        // 3ms leads the inventory; its only graduated host is לוֹ. The card
        // splits for the highlight and quizzes the pronoun.
        let card = match bible.next_suffix_introduction(now)? {
            Some(StudyItem::NewSuffixDrill(c)) => c,
            other => panic!("expected a new suffix drill, got {other:?}"),
        };
        assert_eq!(card.key, "3ms");
        assert_eq!(card.surface, "לוֹ");
        assert_eq!(format!("{}{}", card.stem, card.suffix), card.surface);
        assert_eq!(card.meaning, "him");
        assert_eq!(card.distractors.len(), 3);
        assert!(!card.distractors.contains(&"him".to_string()));

        // Grading creates the row; the next introduction moves to 1cs.
        bible.submit_review(Track::Suffix, "3ms", Grade::Good, now)?;
        assert!(bible.suffix_srs("3ms")?.is_some());
        let card = match bible.next_suffix_introduction(now)? {
            Some(StudyItem::NewSuffixDrill(c)) => c,
            other => panic!("expected the 1cs drill next, got {other:?}"),
        };
        assert_eq!((card.key.as_str(), card.surface.as_str()), ("1cs", "אֵלַי"));
        assert_eq!(card.stem, "אֵל");

        // A due ending comes back as a review on a known host.
        bible
            .conn()
            .execute("UPDATE progress.suffix_srs SET due_epoch = 0", [])?;
        match bible.next_review(now, false)? {
            Some(StudyItem::ReviewSuffixDrill(c)) => {
                assert_eq!(c.key, "3ms");
                assert_eq!(c.surface, "לוֹ");
            }
            other => panic!("expected a suffix review, got {other:?}"),
        }
        Ok(())
    }
}
