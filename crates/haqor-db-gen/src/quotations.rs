//! Find NT quotations of the OT directly across the Hebrew/Aramaic boundary.
//!
//! The Peshitta NT and the Hebrew OT share most of their Semitic roots, so the
//! two can be compared root-against-root without going through a translation.
//! Every word becomes a small set of normalised root *keys*; an NT word's keys
//! are carried into Hebrew spelling first (regular sound correspondences, III-
//! weak endings, plene names, and a short table of common non-cognate pairs).
//!
//! Scoring is two-stage. An inverted index over OT keys proposes candidate
//! verse pairs that share several rare keys; each candidate is then scored by a
//! local alignment (Smith–Waterman) whose match weight is the rarity of the
//! shared key and which pays a small cost per skipped word. The local
//! alignment finds a quotation embedded in narrative ("and Jesus cried with a
//! loud voice, saying: …") without the rest of the verse diluting it, and
//! rewards the words appearing in the same order.
//!
//! The same matcher links each testament to itself — the OT's parallel
//! passages and internal quotations (Kings/Chronicles, Psalm 18/2 Samuel 22,
//! Isaiah 2/Micah 4), the NT's synoptic parallels and repeated quotations —
//! with both sides in one language, so an OT pair compares Hebrew keys and an
//! NT pair the Peshitta's own roots.
//!
//! The result is the `quotation` table: one row per verse pair, the earlier
//! verse (in corpus order) first, `quote_id` doubling as the global rank
//! (1 = strongest).

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail};
use haqor_core::transliterate::lookup_key;
use log::info;
use rusqlite::{Connection, params};

use crate::runtime_db::pack_ref;

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS quotation;
CREATE TABLE quotation(
    quote_id    INTEGER PRIMARY KEY,
    a_ref       INTEGER NOT NULL,
    b_ref       INTEGER NOT NULL,
    score       REAL    NOT NULL,
    matched     INTEGER NOT NULL,
    a_positions TEXT    NOT NULL,
    b_positions TEXT    NOT NULL,
    CHECK (a_ref < b_ref)
);
CREATE INDEX idx_quotation_a ON quotation(a_ref);
CREATE INDEX idx_quotation_b ON quotation(b_ref);
";

/// The matcher's tuning knobs for one [`LinkKind`]; [`Matcher`] holds the set
/// for each. [`Default`] is what `gen-runtime` ships for OT/NT pairs: a
/// deliberately loose set, echoes and allusions included, since a reader filters
/// by score at run time and a missed quotation cannot be recovered there.
/// Tuned 2026-09-26 by sweep: gap 0.3 ranks the known quotations best, and
/// min score 6 with 12 per NT verse keeps 37 of 55 at ~60k pairs.
///
/// `db gen-quotes --set NAME=VALUE` / `--sweep NAME=V1,V2,…` try others
/// against the known quotations without a rebuild (see [`Matcher::set`] for
/// the names).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatcherParams {
    /// Keys rarer than this (in weight) are ignored when proposing candidates;
    /// they still count once the pair is aligned.
    pub candidate_min_weight: f32,
    /// A pair needs this many distinct shared candidate keys to be aligned.
    pub candidate_min_shared: usize,
    /// Keys below this weight never contribute a match (particles, pronouns).
    pub match_min_weight: f32,
    /// Cost of each word the alignment skips on either side.
    pub gap_penalty: f32,
    /// Added to a match that directly continues the previous one on both sides.
    pub contiguity_bonus: f32,
    /// Both matches of a contiguous run must weigh at least this for the run to
    /// earn [`Self::contiguity_bonus`]: a particle between two coincidental
    /// matches does not make them a phrase.
    pub contiguity_min_weight: f32,
    /// Multiplies the alignment score before it is compared or stored, so the
    /// kinds share one scale: a same-language pair matches every word of a
    /// quotation and scores about twice what the same quotation would across
    /// Hebrew and Aramaic.
    pub score_scale: f32,
    /// Minimum (scaled) local-alignment score for a pair to be stored.
    pub min_score: f32,
    /// Minimum number of aligned word pairs for a pair to be stored.
    pub min_matched: usize,
    /// Minimum number of distinct keys of at least [`Self::strong_weight`]
    /// among the aligned words.
    pub min_strong: usize,
    /// What counts as a strong key for [`Self::min_strong`].
    pub strong_weight: f32,
    /// At most this many earlier verses are kept per verse (for an OT/NT pair,
    /// OT verses per NT verse).
    pub max_per_verse: usize,
}

impl Default for MatcherParams {
    fn default() -> Self {
        MatcherParams {
            candidate_min_weight: 1.5,
            candidate_min_shared: 2,
            match_min_weight: 0.5,
            gap_penalty: 0.3,
            contiguity_bonus: 1.0,
            contiguity_min_weight: 0.0,
            min_score: 6.0,
            min_matched: 3,
            min_strong: 0,
            strong_weight: 3.0,
            max_per_verse: 12,
            score_scale: 1.0,
        }
    }
}

impl MatcherParams {
    pub const NAMES: &[&str] = &[
        "candidate_min_weight",
        "candidate_min_shared",
        "match_min_weight",
        "gap_penalty",
        "contiguity_bonus",
        "contiguity_min_weight",
        "min_score",
        "min_matched",
        "min_strong",
        "strong_weight",
        "max_per_verse",
        "score_scale",
    ];

    /// Set one knob by its field name.
    pub fn set(&mut self, name: &str, value: &str) -> Result<()> {
        let float = || -> Result<f32> {
            value
                .parse()
                .with_context(|| format!("{name}: bad number {value:?}"))
        };
        let count = || -> Result<usize> {
            value
                .parse()
                .with_context(|| format!("{name}: bad count {value:?}"))
        };
        match name {
            "candidate_min_weight" => self.candidate_min_weight = float()?,
            "candidate_min_shared" => self.candidate_min_shared = count()?,
            "match_min_weight" => self.match_min_weight = float()?,
            "gap_penalty" => self.gap_penalty = float()?,
            "contiguity_bonus" => self.contiguity_bonus = float()?,
            "contiguity_min_weight" => self.contiguity_min_weight = float()?,
            "min_score" => self.min_score = float()?,
            "min_matched" => self.min_matched = count()?,
            "min_strong" => self.min_strong = count()?,
            "strong_weight" => self.strong_weight = float()?,
            "max_per_verse" => self.max_per_verse = count()?,
            "score_scale" => self.score_scale = float()?,
            _ => bail!(
                "unknown matcher parameter {name:?}; known: {}",
                Self::NAMES.join(", ")
            ),
        }
        Ok(())
    }
}

/// The [`MatcherParams`] of each [`LinkKind`]: `cross` for OT/NT pairs,
/// `within` for OT/OT and NT/NT pairs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matcher {
    pub cross: MatcherParams,
    pub within: MatcherParams,
}

impl Default for Matcher {
    /// Within one testament every word of a quotation matches, so pairs are
    /// scaled to the cross-testament scale, where the known quotations have a
    /// median of about 16 and the known parallels about 30 before scaling.
    /// Sampled 2026-09-26: below 7 scaled nearly every pair shares only a
    /// formula ("and the LORD spoke to Moses, saying") or a few common roots;
    /// from 7 to 10 formulas and genuine phrase echoes (Jeremiah's "rising
    /// early and sending", John's "I go to him who sent me") mix, which the
    /// reader's strength filter can leave out. The floor of 7 keeps 30 of the
    /// 32 known parallels (the misses are four-word sayings) at ~54k pairs.
    /// Candidates need a third shared key: with no language boundary, two are
    /// nearly any pair of verses.
    fn default() -> Self {
        let cross = MatcherParams::default();
        Matcher {
            cross,
            within: MatcherParams {
                candidate_min_shared: 3,
                score_scale: 0.5,
                min_score: 7.0,
                ..cross
            },
        }
    }
}

impl Matcher {
    /// Set one knob by name: `cross.NAME` or `within.NAME` for one set, a bare
    /// [`MatcherParams::NAMES`] name for both.
    pub fn set(&mut self, name: &str, value: &str) -> Result<()> {
        if let Some(name) = name.strip_prefix("cross.") {
            self.cross.set(name, value)
        } else if let Some(name) = name.strip_prefix("within.") {
            self.within.set(name, value)
        } else {
            self.cross.set(name, value)?;
            self.within.set(name, value)
        }
    }

    pub fn params(&self, kind: LinkKind) -> &MatcherParams {
        match kind {
            LinkKind::OtNt => &self.cross,
            LinkKind::OtOt | LinkKind::NtNt => &self.within,
        }
    }
}

/// Common Aramaic words whose Hebrew equivalent is a different root or an
/// irregular spelling. The Peshitta's quotations use these where the Hebrew
/// does, so without them the most-quoted verbs and nouns would never align.
/// Both sides are SEDRA / lexicon spellings; they go through [`normalise_key`]
/// before use, so a geminate like מלל is looked up as מל.
const EQUIVALENTS: &[(&str, &[&str])] = &[
    ("בר", &["בנ"]),           // son
    ("ברת", &["בת"]),          // daughter
    ("יהב", &["נתנ"]),         // give
    ("אזל", &["הלכ"]),         // go
    ("אתא", &["בוא"]),         // come
    ("עלל", &["בוא"]),         // enter
    ("נפק", &["יצא"]),         // go out
    ("חזא", &["ראה"]),         // see
    ("שבק", &["עזב"]),         // leave, forsake
    ("עבד", &["עשה"]),         // do, make
    ("מלל", &["דבר"]),         // speak
    ("סגד", &["שחה", "חוה"]),  // bow down
    ("קטל", &["הרג"]),         // kill
    ("בעא", &["בקש"]),         // seek
    ("רחמ", &["אהב"]),         // love
    ("מרא", &["אדנ", "יהוה"]), // lord, and the Peshitta's rendering of the divine name
    ("אנש", &["איש"]),         // man
    ("טלא", &["נער"]),         // youth
    ("טליא", &["נער"]),
    ("סגא", &["רבה"]),         // be many
    ("זבנא", &["עת"]),         // time
    ("ארח", &["דרכ"]),         // way
    ("שמיא", &["שמימ"]),       // heaven
    ("מיא", &["מימ"]),         // water
    ("אית", &["יש"]),          // there is
    ("אשכח", &["מצא"]),        // find
    ("המנ", &["אמנ"]),         // believe
    ("סהד", &["עוד"]),         // witness
    ("אפא", &["פנה"]),         // face
    ("פומא", &["פה"]),         // mouth
    ("טורא", &["הר"]),         // mountain
    ("נמוסא", &["תורה"]),      // law
    ("עדמא", &["עד"]),         // until
    ("השא", &["עתה"]),         // now
    ("הידינ", &["אז"]),        // then
    ("פארא", &["פרי"]),        // fruit
    ("קעא", &["צעק", "זעק"]),  // cry out
    ("טעא", &["תעה"]),         // wander, err
    ("דמר", &["תמה"]),         // marvel
    ("כאפא", &["צור", "סלע"]), // rock
    ("חלפ", &["תחת"]),         // instead of
    ("אנתתא", &["אשה"]),       // woman
    ("איסראיל", &["ישראל"]),   // Israel
    ("אורשלמ", &["ירושלמ"]),   // Jerusalem
    ("מושא", &["משה"]),        // Moses
    ("פלח", &["עבד"]),         // serve
    ("פלג", &["חלק"]),         // divide
    ("פסא", &["גורל"]),        // lot
    ("קרב", &["רע"]),          // neighbour (קריבא)
    ("נחתא", &["בגד"]),        // garment
    ("ערק", &["נוס", "ברח"]),  // flee
    ("שדר", &["שלח"]),         // send
    ("שקל", &["לקח", "נשא"]),  // take
    ("נסב", &["לקח"]),         // take
    ("רגז", &["כעס"]),         // anger
];

/// Regular Aramaic → Hebrew consonant correspondences (the proto-Semitic
/// interdentals and emphatics): gold דהב/זהב, snow תלג/שלג, return תוב/שוב,
/// earth ארע/ארץ, righteous זדק/צדק, hate סנא/שׂנא.
const SHIFTS: &[(char, char)] = &[
    ('ד', 'ז'),
    ('ת', 'ש'),
    ('ט', 'צ'),
    ('ע', 'צ'),
    ('ז', 'צ'),
    ('ס', 'ש'),
];

/// (book, chapter, verse).
pub type Reference = (u8, u8, u8);

/// (later verse, earlier verse): for an OT/NT pair, (NT verse, OT verse it
/// quotes).
pub type KnownQuotation = (Reference, Reference);

/// A well-known NT quotation of the OT: (NT book, chapter, verse), (OT book,
/// chapter, verse). OT numbering is the Hebrew text's; NT is the Peshitta's.
pub const KNOWN_QUOTATIONS: &[KnownQuotation] = &[
    ((40, 1, 23), (12, 7, 14)),    // Mt 1:23 / Isa 7:14
    ((40, 2, 6), (20, 5, 1)),      // Mt 2:6 / Mic 5:1
    ((40, 2, 15), (15, 11, 1)),    // Mt 2:15 / Hos 11:1
    ((40, 2, 18), (13, 31, 15)),   // Mt 2:18 / Jer 31:15
    ((40, 3, 3), (12, 40, 3)),     // Mt 3:3 / Isa 40:3
    ((40, 4, 4), (5, 8, 3)),       // Mt 4:4 / Deut 8:3
    ((40, 4, 6), (27, 91, 11)),    // Mt 4:6 / Ps 91:11
    ((40, 4, 7), (5, 6, 16)),      // Mt 4:7 / Deut 6:16
    ((40, 4, 10), (5, 6, 13)),     // Mt 4:10 / Deut 6:13
    ((40, 5, 38), (2, 21, 24)),    // Mt 5:38 / Ex 21:24
    ((40, 5, 43), (3, 19, 18)),    // Mt 5:43 / Lev 19:18
    ((40, 21, 5), (25, 9, 9)),     // Mt 21:5 / Zech 9:9
    ((40, 21, 9), (27, 118, 26)),  // Mt 21:9 / Ps 118:26
    ((40, 21, 42), (27, 118, 22)), // Mt 21:42 / Ps 118:22
    ((40, 22, 37), (5, 6, 5)),     // Mt 22:37 / Deut 6:5
    ((40, 22, 44), (27, 110, 1)),  // Mt 22:44 / Ps 110:1
    ((40, 13, 14), (12, 6, 9)),    // Mt 13:14 / Isa 6:9
    ((41, 12, 29), (5, 6, 4)),     // Mk 12:29 / Deut 6:4
    ((41, 15, 34), (27, 22, 2)),   // Mk 15:34 / Ps 22:2
    ((42, 4, 18), (12, 61, 1)),    // Lk 4:18 / Isa 61:1
    ((43, 19, 24), (27, 22, 19)),  // Jn 19:24 / Ps 22:19
    ((44, 2, 17), (16, 3, 1)),     // Acts 2:17 / Joel 3:1
    ((45, 1, 17), (22, 2, 4)),     // Rom 1:17 / Hab 2:4
    ((45, 4, 3), (1, 15, 6)),      // Rom 4:3 / Gen 15:6
    ((45, 10, 15), (12, 52, 7)),   // Rom 10:15 / Isa 52:7
    ((48, 3, 13), (5, 21, 23)),    // Gal 3:13 / Deut 21:23
    ((58, 1, 5), (27, 2, 7)),      // Heb 1:5 / Ps 2:7
    ((60, 1, 24), (12, 40, 6)),    // 1Pet 1:24 / Isa 40:6
    ((40, 4, 15), (12, 8, 23)),    // Mt 4:15 / Isa 8:23
    ((40, 12, 18), (12, 42, 1)),   // Mt 12:18 / Isa 42:1
    ((40, 12, 20), (12, 42, 3)),   // Mt 12:20 / Isa 42:3
    ((40, 13, 35), (27, 78, 2)),   // Mt 13:35 / Ps 78:2
    ((40, 15, 8), (12, 29, 13)),   // Mt 15:8 / Isa 29:13
    ((40, 19, 5), (1, 2, 24)),     // Mt 19:5 / Gen 2:24
    ((40, 21, 13), (12, 56, 7)),   // Mt 21:13 / Isa 56:7
    ((40, 26, 31), (25, 13, 7)),   // Mt 26:31 / Zech 13:7
    ((40, 27, 46), (27, 22, 2)),   // Mt 27:46 / Ps 22:2
    ((42, 23, 46), (27, 31, 6)),   // Lk 23:46 / Ps 31:6
    ((43, 1, 23), (12, 40, 3)),    // Jn 1:23 / Isa 40:3
    ((43, 2, 17), (27, 69, 10)),   // Jn 2:17 / Ps 69:10
    ((43, 12, 38), (12, 53, 1)),   // Jn 12:38 / Isa 53:1
    ((43, 13, 18), (27, 41, 10)),  // Jn 13:18 / Ps 41:10
    ((44, 2, 25), (27, 16, 8)),    // Acts 2:25 / Ps 16:8
    ((44, 2, 34), (27, 110, 1)),   // Acts 2:34 / Ps 110:1
    ((44, 8, 32), (12, 53, 7)),    // Acts 8:32 / Isa 53:7
    ((45, 9, 29), (12, 1, 9)),     // Rom 9:29 / Isa 1:9
    ((45, 11, 8), (5, 29, 3)),     // Rom 11:8 / Deut 29:3
    ((46, 15, 54), (12, 25, 8)),   // 1Cor 15:54 / Isa 25:8
    ((48, 4, 27), (12, 54, 1)),    // Gal 4:27 / Isa 54:1
    ((58, 3, 10), (27, 95, 10)),   // Heb 3:10 / Ps 95:10
    ((58, 8, 10), (13, 31, 33)),   // Heb 8:10 / Jer 31:33
    ((58, 11, 12), (1, 22, 17)),   // Heb 11:12 / Gen 22:17
    ((59, 2, 23), (1, 15, 6)),     // Jas 2:23 / Gen 15:6
    ((60, 2, 6), (12, 28, 16)),    // 1Pet 2:6 / Isa 28:16
    ((60, 2, 22), (12, 53, 9)),    // 1Pet 2:22 / Isa 53:9
];

/// Pairs the matcher has stored that share only coincidences of spelling —
/// no quotation or allusion. Each was checked with `gen-quotes --explain`;
/// a tuning change should not bring them back.
pub const KNOWN_FALSE: &[KnownQuotation] = &[
    // אֲכַלְכֵּל "sustain" / כוילא "ark" (both כול), טַף "little ones" / טופנא
    // "flood" (both טפ), bridged by אֶת.
    ((42, 17, 27), (1, 50, 21)), // Lk 17:27 / Gen 50:21
];

/// Well-known parallels within one testament: (later verse, earlier verse), in
/// corpus order. Parallel accounts, repeated oracles, psalms that exist twice,
/// and the NT's synoptic parallels and shared quotations.
pub const KNOWN_PARALLELS: &[KnownQuotation] = &[
    ((27, 18, 3), (9, 22, 2)),    // Ps 18:3 / 2Sam 22:2
    ((20, 4, 1), (12, 2, 2)),     // Mic 4:1 / Isa 2:2
    ((27, 53, 2), (27, 14, 1)),   // Ps 53:2 / Ps 14:1
    ((12, 36, 1), (11, 18, 13)),  // Isa 36:1 / 2Kgs 18:13
    ((39, 36, 23), (36, 1, 2)),   // 2Chr 36:23 / Ezra 1:2
    ((18, 1, 1), (13, 49, 14)),   // Obad 1:1 / Jer 49:14
    ((5, 5, 16), (2, 20, 12)),    // Deut 5:16 / Ex 20:12
    ((19, 4, 2), (2, 34, 6)),     // Jonah 4:2 / Ex 34:6
    ((17, 1, 2), (16, 4, 16)),    // Amos 1:2 / Joel 4:16
    ((38, 16, 34), (27, 106, 1)), // 1Chr 16:34 / Ps 106:1
    ((38, 17, 12), (9, 7, 13)),   // 1Chr 17:12 / 2Sam 7:13
    ((13, 51, 15), (13, 10, 12)), // Jer 51:15 / Jer 10:12
    ((27, 108, 2), (27, 57, 8)),  // Ps 108:2 / Ps 57:8
    ((27, 70, 2), (27, 40, 14)),  // Ps 70:2 / Ps 40:14
    ((39, 18, 16), (10, 22, 17)), // 2Chr 18:16 / 1Kgs 22:17
    ((22, 2, 14), (12, 11, 9)),   // Hab 2:14 / Isa 11:9
    ((26, 3, 23), (16, 3, 4)),    // Mal 3:23 / Joel 3:4
    ((41, 1, 3), (40, 3, 3)),     // Mk 1:3 / Mt 3:3
    ((42, 3, 4), (40, 3, 3)),     // Lk 3:4 / Mt 3:3
    ((42, 11, 2), (40, 6, 9)),    // Lk 11:2 / Mt 6:9
    ((41, 12, 30), (40, 22, 37)), // Mk 12:30 / Mt 22:37
    ((41, 15, 34), (40, 27, 46)), // Mk 15:34 / Mt 27:46
    ((41, 8, 34), (40, 16, 24)),  // Mk 8:34 / Mt 16:24
    ((41, 14, 22), (40, 26, 26)), // Mk 14:22 / Mt 26:26
    ((46, 11, 24), (42, 22, 19)), // 1Cor 11:24 / Lk 22:19
    ((51, 4, 7), (49, 6, 21)),    // Col 4:7 / Eph 6:21
    ((65, 1, 18), (61, 3, 3)),    // Jude 1:18 / 2Pet 3:3
    ((48, 3, 6), (45, 4, 3)),     // Gal 3:6 / Rom 4:3
    ((48, 5, 14), (45, 13, 9)),   // Gal 5:14 / Rom 13:9
    ((59, 2, 8), (40, 22, 39)),   // Jas 2:8 / Mt 22:39
    ((58, 4, 7), (58, 3, 15)),    // Heb 4:7 / Heb 3:15
    ((66, 21, 6), (66, 1, 8)),    // Rev 21:6 / Rev 1:8
];

/// A tokenised verse: its packed ref and one key set per word position.
struct Verse {
    reference: i64,
    words: Vec<(i64, Vec<u32>)>,
}

/// Interned root keys.
#[derive(Default)]
struct Keys {
    ids: HashMap<String, u32>,
}

impl Keys {
    fn intern(&mut self, key: String) -> u32 {
        let next = self.ids.len() as u32;
        *self.ids.entry(key).or_insert(next)
    }

    fn get(&self, key: &str) -> Option<u32> {
        self.ids.get(key).copied()
    }

    /// Each key's spelling, by id.
    fn names(&self) -> Vec<String> {
        let mut names = vec![String::new(); self.ids.len()];
        for (key, &id) in &self.ids {
            names[id as usize] = key.clone();
        }
        names
    }
}

/// Fold a root or headword to the key both sides compare on: final letters to
/// medial, pointing dropped, and a geminate root collapsed to its biliteral
/// base (Hebrew עמם, Aramaic עמ).
pub fn normalise_key(root: &str) -> String {
    let letters: Vec<char> = lookup_key(root)
        .chars()
        .filter(|c| ('\u{05D0}'..='\u{05EA}').contains(c))
        .collect();
    let n = letters.len();
    if n == 3 && letters[1] == letters[2] {
        return letters[..2].iter().collect();
    }
    letters.into_iter().collect()
}

/// Hebrew spellings an Aramaic key may correspond to (itself included).
fn hebrew_candidates(key: &str) -> Vec<String> {
    let chars: Vec<char> = key.chars().collect();
    let mut stems = vec![chars.clone()];
    // Nouns SEDRA lists in the emphatic state (ספרא, מלאכא, עלמא).
    if chars.len() >= 3 && chars.last() == Some(&'א') {
        stems.push(chars[..chars.len() - 1].to_vec());
    }
    let mut more = Vec::new();
    for stem in &stems {
        let n = stem.len();
        let last = stem.last().copied();
        let with_last = |c: char| {
            let mut v = stem.clone();
            v[n - 1] = c;
            v
        };
        // III-weak: Aramaic final aleph/yod where Hebrew writes he (בנא / בנה).
        if n >= 3 && (last == Some('א') || last == Some('י')) {
            more.push(with_last('ה'));
        }
        // Feminine -ta where Hebrew has -ah (שנתא / שנה).
        if n >= 3 && last == Some('ת') {
            more.push(with_last('ה'));
        }
        // Plurals lexicalised with Aramaic -in where Hebrew has -im (מצרין / מצרים).
        if n >= 4 && last == Some('נ') {
            more.push(with_last('מ'));
        }
    }
    stems.extend(more);

    let mut out: Vec<String> = Vec::new();
    for stem in &stems {
        let spelled: String = stem.iter().collect();
        if let Some((_, hebrew)) = EQUIVALENTS
            .iter()
            .find(|(aramaic, _)| normalise_key(aramaic) == spelled)
        {
            out.extend(hebrew.iter().map(|h| normalise_key(h)));
        }
        out.push(spelled);
        for (i, &c) in stem.iter().enumerate() {
            for &(from, to) in SHIFTS {
                if c == from {
                    let mut v = stem.clone();
                    v[i] = to;
                    out.push(v.into_iter().collect());
                }
            }
        }
    }
    let defectives: Vec<String> = out.iter().filter_map(|k| defective(k)).collect();
    out.extend(defectives);
    out.sort();
    out.dedup();
    out
}

/// The key with its inner vowel letters dropped (קול / קל, ישראיל / ישראל,
/// דויד / דוד), when that leaves at least two letters. Both testaments carry
/// it as a second key, so plene and defective spellings of one word meet.
fn defective(key: &str) -> Option<String> {
    let chars: Vec<char> = key.chars().collect();
    let n = chars.len();
    if n < 3 {
        return None;
    }
    let out: String = chars
        .iter()
        .enumerate()
        .filter(|&(i, &c)| i == 0 || i == n - 1 || (c != 'ו' && c != 'י'))
        .map(|(_, &c)| c)
        .collect();
    (out.chars().count() >= 2 && out != key).then_some(out)
}

/// Push a word's key set onto the verse it belongs to, starting a new verse
/// when the reference changes.
fn push_word(verses: &mut Vec<Verse>, reference: i64, position: i64, mut word: Vec<u32>) {
    word.sort_unstable();
    word.dedup();
    if verses.last().is_none_or(|v| v.reference != reference) {
        verses.push(Verse {
            reference,
            words: Vec::new(),
        });
    }
    verses.last_mut().unwrap().words.push((position, word));
}

fn load_ot(db: &Connection, keys: &mut Keys) -> Result<Vec<Verse>> {
    // Headword consonants for each surface, with the root the entry files it
    // under: they fill in the many particles and nouns (שֵׁם, עִם, בֵּן) whose
    // resolved rendering carries no root.
    let mut entries: HashMap<i64, Vec<(String, String)>> = HashMap::new();
    let mut stmt = db.prepare(
        "SELECT se.surface_id, le.root, le.cons FROM surface_entry se
         JOIN lexicon_entry le ON le.key = se.key WHERE le.cons IS NOT NULL",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let cons = normalise_key(&row.get::<_, String>(2)?);
        if (2..=5).contains(&cons.chars().count()) {
            entries
                .entry(row.get(0)?)
                .or_default()
                .push((normalise_key(&row.get::<_, String>(1)?), cons));
        }
    }

    let mut stmt = db.prepare(
        "SELECT w.ref, w.position, w.surface_id, COALESCE(wi.root, '')
         FROM word w LEFT JOIN word_info wi ON wi.info_id = w.info_id
         WHERE w.ref < (40 << 16) ORDER BY w.ref, w.position",
    )?;
    let mut rows = stmt.query([])?;
    let mut verses: Vec<Verse> = Vec::new();
    while let Some(row) = rows.next()? {
        let root = normalise_key(&row.get::<_, String>(3)?);
        let mut forms: Vec<String> = Vec::new();
        if !root.is_empty() {
            forms.push(root.clone());
        }
        for (entry_root, cons) in entries.get(&row.get(2)?).into_iter().flatten() {
            if entry_root.is_empty() || *entry_root == root {
                forms.push(cons.clone());
            }
        }
        let defectives: Vec<String> = forms.iter().filter_map(|k| defective(k)).collect();
        let word = forms
            .into_iter()
            .chain(defectives)
            .map(|k| keys.intern(k))
            .collect();
        push_word(&mut verses, row.get(0)?, row.get(1)?, word);
    }
    Ok(verses)
}

/// The NT twice over: in the OT's Hebrew keys (`hebrew`, for OT/NT pairs) and
/// in its own Syriac roots (interned into `native`, for NT/NT pairs).
fn load_nt(db: &Connection, hebrew: &Keys, native: &mut Keys) -> Result<(Vec<Verse>, Vec<Verse>)> {
    let mut stmt = db.prepare(
        "SELECT n.ref, n.ord, COALESCE(r.root, '')
         FROM nt_word n
         JOIN syriac_word w ON w.word_id = n.word_id
         LEFT JOIN syriac_lexeme l ON l.lexeme_id = w.lexeme_id
         LEFT JOIN syriac_root r ON r.root_id = l.root_id
         ORDER BY n.ref, n.ord",
    )?;
    let mut rows = stmt.query([])?;
    let mut cache: HashMap<String, (Vec<u32>, Vec<u32>)> = HashMap::new();
    let (mut in_hebrew, mut in_syriac) = (Vec::new(), Vec::new());
    while let Some(row) = rows.next()? {
        let (reference, position): (i64, i64) = (row.get(0)?, row.get(1)?);
        let root = normalise_key(&row.get::<_, String>(2)?);
        let (as_hebrew, as_syriac) = cache
            .entry(root.clone())
            .or_insert_with(|| {
                let as_hebrew = hebrew_candidates(&root)
                    .iter()
                    .filter_map(|k| hebrew.get(k))
                    .collect();
                let as_syriac = if root.is_empty() {
                    Vec::new()
                } else {
                    defective(&root)
                        .into_iter()
                        .chain([root.clone()])
                        .map(|k| native.intern(k))
                        .collect()
                };
                (as_hebrew, as_syriac)
            })
            .clone();
        push_word(&mut in_hebrew, reference, position, as_hebrew);
        push_word(&mut in_syriac, reference, position, as_syriac);
    }
    Ok((in_hebrew, in_syriac))
}

/// Inverse document frequency of every key over a testament's verses.
fn idf(verses: &[Verse], n_keys: usize) -> Vec<f32> {
    let mut df = vec![0u32; n_keys];
    for verse in verses {
        for k in distinct_keys(verse) {
            df[k as usize] += 1;
        }
    }
    let n = verses.len() as f32;
    df.into_iter()
        .map(|d| if d == 0 { 0.0 } else { (n / d as f32).ln() })
        .collect()
}

/// Which testaments a link joins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkKind {
    /// An NT verse quoting or echoing an OT verse.
    OtNt,
    /// Two OT verses: parallel passages, repeated oracles and formulas.
    OtOt,
    /// Two NT verses: synoptic parallels, shared quotations.
    NtNt,
}

impl LinkKind {
    pub const ALL: [LinkKind; 3] = [LinkKind::OtNt, LinkKind::OtOt, LinkKind::NtNt];

    /// The kind of the link between two packed refs.
    pub fn of(a: i64, b: i64) -> Self {
        match (a >> 16 >= 40, b >> 16 >= 40) {
            (false, false) => LinkKind::OtOt,
            (true, true) => LinkKind::NtNt,
            _ => LinkKind::OtNt,
        }
    }

    fn label(self) -> &'static str {
        match self {
            LinkKind::OtNt => "OT-NT",
            LinkKind::OtOt => "OT-OT",
            LinkKind::NtNt => "NT-NT",
        }
    }
}

/// One aligned verse pair.
pub struct Alignment {
    pub score: f32,
    /// Aligned word positions, (earlier verse, later verse).
    pub pairs: Vec<(i64, i64)>,
    /// Distinct keys of at least [`MatcherParams::strong_weight`] among the
    /// aligned words.
    pub strong: usize,
}

/// The two sides one [`LinkKind`] compares, and the weight of each key they
/// share.
struct Sides<'a> {
    earlier: &'a [Verse],
    later: &'a [Verse],
    weight: &'a [f32],
    names: &'a [String],
    /// Both sides are the same testament: a verse is only paired with the
    /// verses before it, and never with its own chapter.
    same: bool,
}

impl Sides<'_> {
    /// The best shared key of two words, if it may match at all.
    fn shared_key(&self, a: &[u32], b: &[u32], params: &MatcherParams) -> Option<u32> {
        let mut best: Option<u32> = None;
        let (mut x, mut y) = (0, 0);
        while x < a.len() && y < b.len() {
            match a[x].cmp(&b[y]) {
                std::cmp::Ordering::Less => x += 1,
                std::cmp::Ordering::Greater => y += 1,
                std::cmp::Ordering::Equal => {
                    let k = a[x];
                    if best.is_none_or(|b| self.weight[k as usize] > self.weight[b as usize]) {
                        best = Some(k);
                    }
                    x += 1;
                    y += 1;
                }
            }
        }
        best.filter(|&k| self.weight[k as usize] >= params.match_min_weight)
    }

    /// Smith–Waterman local alignment over word key sets.
    fn align(&self, a: &Verse, b: &Verse, params: &MatcherParams) -> Alignment {
        let (m, n) = (a.words.len(), b.words.len());
        let w = n + 1;
        let mut h = vec![0.0f32; (m + 1) * w];
        // The key a cell's match was made on, where the cell is a match.
        let mut matched: Vec<Option<u32>> = vec![None; (m + 1) * w];
        let (mut best, mut best_at) = (0.0f32, (0, 0));
        for i in 1..=m {
            for j in 1..=n {
                let key = self.shared_key(&a.words[i - 1].1, &b.words[j - 1].1, params);
                let s = key.map_or(0.0, |k| self.weight[k as usize]);
                // A match directly continuing the previous one is a phrase, not a
                // coincidence of scattered words — unless either is a particle.
                let run = match (matched[(i - 1) * w + j - 1], key) {
                    (Some(prev), Some(_))
                        if self.weight[prev as usize] >= params.contiguity_min_weight
                            && s >= params.contiguity_min_weight =>
                    {
                        params.contiguity_bonus
                    }
                    _ => 0.0,
                };
                let diag = if key.is_some() {
                    h[(i - 1) * w + j - 1] + s + run
                } else {
                    0.0
                };
                let up = h[(i - 1) * w + j] - params.gap_penalty;
                let left = h[i * w + j - 1] - params.gap_penalty;
                let cell = diag.max(up).max(left).max(0.0);
                h[i * w + j] = cell;
                matched[i * w + j] = key.filter(|_| cell == diag);
                if cell > best {
                    best = cell;
                    best_at = (i, j);
                }
            }
        }
        let mut pairs = Vec::new();
        let mut strong = HashSet::new();
        let (mut i, mut j) = best_at;
        while i > 0 && j > 0 && h[i * w + j] > 0.0 {
            if let Some(k) = matched[i * w + j] {
                pairs.push((a.words[i - 1].0, b.words[j - 1].0));
                if self.weight[k as usize] >= params.strong_weight {
                    strong.insert(k);
                }
                i -= 1;
                j -= 1;
            } else if h[(i - 1) * w + j] - params.gap_penalty == h[i * w + j] {
                i -= 1;
            } else {
                j -= 1;
            }
        }
        pairs.reverse();
        Alignment {
            score: best * params.score_scale,
            pairs,
            strong: strong.len(),
        }
    }

    /// An upper bound of [`Self::align`]'s score for `later` against a verse
    /// with the (sorted) keys `earlier_keys`: each later word matches at most
    /// once, on at best its heaviest key the earlier verse has, and every match
    /// but the first may earn the contiguity bonus. Skipping the pairs that
    /// cannot reach the floor saves most of the alignments.
    fn bound(&self, earlier_keys: &[u32], later: &Verse, params: &MatcherParams) -> f32 {
        let (mut total, mut matches) = (0.0f32, 0usize);
        for (_, keys) in &later.words {
            let best = keys
                .iter()
                .filter(|k| earlier_keys.binary_search(k).is_ok())
                .map(|&k| self.weight[k as usize])
                .fold(0.0, f32::max);
            if best >= params.match_min_weight {
                total += best;
                matches += 1;
            }
        }
        let bonus = params.contiguity_bonus.max(0.0) * matches.saturating_sub(1) as f32;
        (total + bonus) * params.score_scale
    }

    /// Every stored-quality pair of these sides under `params`, in later-verse
    /// order.
    fn find(&self, params: &MatcherParams) -> Vec<Found> {
        let mut postings: Vec<Vec<u32>> = vec![Vec::new(); self.weight.len()];
        let mut earlier_keys: Vec<Vec<u32>> = Vec::with_capacity(self.earlier.len());
        for (v, verse) in self.earlier.iter().enumerate() {
            let mut keys: Vec<u32> = distinct_keys(verse).into_iter().collect();
            keys.sort_unstable();
            for &k in &keys {
                if self.weight[k as usize] >= params.candidate_min_weight {
                    postings[k as usize].push(v as u32);
                }
            }
            earlier_keys.push(keys);
        }

        let mut found: Vec<Found> = Vec::new();
        let mut shared = vec![0u16; self.earlier.len()];
        let mut touched: Vec<u32> = Vec::new();
        for (index, verse) in self.later.iter().enumerate() {
            let chapter = verse.reference >> 8;
            for k in distinct_keys(verse) {
                for &v in &postings[k as usize] {
                    // Verses are in reference order, so within one testament
                    // the earlier verses are the ones before this index.
                    if self.same
                        && (v as usize >= index
                            || self.earlier[v as usize].reference >> 8 == chapter)
                    {
                        continue;
                    }
                    if shared[v as usize] == 0 {
                        touched.push(v);
                    }
                    shared[v as usize] += 1;
                }
            }
            let mut here: Vec<Found> = Vec::new();
            for &v in &touched {
                if shared[v as usize] as usize >= params.candidate_min_shared
                    && self.bound(&earlier_keys[v as usize], verse, params) >= params.min_score
                {
                    let earlier = &self.earlier[v as usize];
                    let alignment = self.align(earlier, verse, params);
                    if alignment.score >= params.min_score
                        && alignment.pairs.len() >= params.min_matched
                        && alignment.strong >= params.min_strong
                    {
                        here.push(Found {
                            a_ref: earlier.reference,
                            b_ref: verse.reference,
                            alignment,
                        });
                    }
                }
                shared[v as usize] = 0;
            }
            touched.clear();
            here.sort_by(|a, b| b.alignment.score.total_cmp(&a.alignment.score));
            here.truncate(params.max_per_verse);
            found.extend(here);
        }
        found
    }

    fn verse(&self, reference: i64) -> Option<&Verse> {
        [self.earlier, self.later]
            .into_iter()
            .flatten()
            .find(|v| v.reference == reference)
    }
}

/// Both testaments tokenised and weighted: everything the matcher reads,
/// loaded once so a sweep can try many [`Matcher`]s against it.
pub struct Corpus {
    ot: Vec<Verse>,
    /// The NT in the OT's Hebrew keys.
    nt: Vec<Verse>,
    /// The NT in its own Syriac roots.
    nt_native: Vec<Verse>,
    /// Hebrew key weights for OT/NT pairs: the rarer of the two testaments'.
    cross_weight: Vec<f32>,
    ot_weight: Vec<f32>,
    nt_weight: Vec<f32>,
    hebrew_names: Vec<String>,
    syriac_names: Vec<String>,
}

impl Corpus {
    pub fn load(db: &Connection) -> Result<Self> {
        let (mut hebrew, mut syriac) = (Keys::default(), Keys::default());
        let ot = load_ot(db, &mut hebrew)?;
        let (nt, nt_native) = load_nt(db, &hebrew, &mut syriac)?;
        let n_keys = hebrew.ids.len();
        let ot_weight = idf(&ot, n_keys);
        let cross_weight = ot_weight
            .iter()
            .zip(idf(&nt, n_keys))
            .map(|(a, b)| a.min(b))
            .collect();
        let nt_weight = idf(&nt_native, syriac.ids.len());
        info!(
            "Quotations: {} OT verses, {} NT verses, {n_keys} Hebrew and {} Syriac root keys",
            ot.len(),
            nt.len(),
            syriac.ids.len()
        );
        Ok(Corpus {
            ot,
            nt,
            nt_native,
            cross_weight,
            ot_weight,
            nt_weight,
            hebrew_names: hebrew.names(),
            syriac_names: syriac.names(),
        })
    }

    fn sides(&self, kind: LinkKind) -> Sides<'_> {
        match kind {
            LinkKind::OtNt => Sides {
                earlier: &self.ot,
                later: &self.nt,
                weight: &self.cross_weight,
                names: &self.hebrew_names,
                same: false,
            },
            LinkKind::OtOt => Sides {
                earlier: &self.ot,
                later: &self.ot,
                weight: &self.ot_weight,
                names: &self.hebrew_names,
                same: true,
            },
            LinkKind::NtNt => Sides {
                earlier: &self.nt_native,
                later: &self.nt_native,
                weight: &self.nt_weight,
                names: &self.syriac_names,
                same: true,
            },
        }
    }

    /// Every stored-quality pair of every kind under `matcher`, strongest first.
    pub fn find(&self, matcher: &Matcher) -> Vec<Found> {
        let mut found: Vec<Found> = LinkKind::ALL
            .into_iter()
            .flat_map(|kind| self.sides(kind).find(matcher.params(kind)))
            .collect();
        found.sort_by(|a, b| {
            b.alignment
                .score
                .total_cmp(&a.alignment.score)
                .then(a.b_ref.cmp(&b.b_ref))
                .then(a.a_ref.cmp(&b.a_ref))
        });
        found
    }
}

fn distinct_keys(verse: &Verse) -> HashSet<u32> {
    verse
        .words
        .iter()
        .flat_map(|(_, k)| k.iter().copied())
        .collect()
}

/// A link the matcher found, before it is stored: `a_ref` the earlier verse.
pub struct Found {
    pub a_ref: i64,
    pub b_ref: i64,
    pub alignment: Alignment,
}

impl Found {
    pub fn kind(&self) -> LinkKind {
        LinkKind::of(self.a_ref, self.b_ref)
    }
}

/// Replace the `quotation` table with `found`, ranked in the order given.
fn write_quotations(db: &Connection, found: &[Found]) -> Result<()> {
    let tx = db.unchecked_transaction()?;
    tx.execute_batch(SCHEMA)?;
    {
        let mut insert = tx.prepare(
            "INSERT INTO quotation(quote_id, a_ref, b_ref, score, matched, a_positions,
                                   b_positions) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        let join = |xs: &mut dyn Iterator<Item = i64>| {
            xs.map(|x| x.to_string()).collect::<Vec<_>>().join(" ")
        };
        for (rank, f) in found.iter().enumerate() {
            let pairs = &f.alignment.pairs;
            insert.execute(params![
                rank as i64 + 1,
                f.a_ref,
                f.b_ref,
                // Rounded as f64: an f32 widened afterwards stores 14.6899995 for
                // 14.69, which a reader's `score >= 14.69` filter then misses.
                (f64::from(f.alignment.score) * 100.0).round() / 100.0,
                pairs.len() as i64,
                join(&mut pairs.iter().map(|p| p.0)),
                join(&mut pairs.iter().map(|p| p.1)),
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Rebuild the `quotation` table of a runtime database in place with the
/// shipped parameters. Returns the number of verse pairs written.
pub fn build_quotations(db: &Connection) -> Result<usize> {
    let found = Corpus::load(db)?.find(&Matcher::default());
    write_quotations(db, &found)?;
    info!("Quotations: wrote {} verse pairs", found.len());
    Ok(found.len())
}

const NT_BOOKS: [&str; 27] = [
    "Matthew",
    "Mark",
    "Luke",
    "John",
    "Acts",
    "Romans",
    "1Corinthians",
    "2Corinthians",
    "Galatians",
    "Ephesians",
    "Philippians",
    "Colossians",
    "1Thessalonians",
    "2Thessalonians",
    "1Timothy",
    "2Timothy",
    "Titus",
    "Philemon",
    "Hebrews",
    "James",
    "1Peter",
    "2Peter",
    "1John",
    "2John",
    "3John",
    "Jude",
    "Revelation",
];

/// Human-readable reference for a packed ref.
pub fn ref_label(reference: i64) -> String {
    let (book, chapter, verse) = (reference >> 16, (reference >> 8) & 255, reference & 255);
    let name = if book >= 40 {
        NT_BOOKS.get(book as usize - 40).copied().unwrap_or("?")
    } else {
        crate::hebrew_db::book_name(book as u8)
    };
    format!("{name} {chapter}:{verse}")
}

fn pack(r: Reference) -> i64 {
    pack_ref(r.0.into(), r.1.into(), r.2.into())
}

/// A known pair's packed refs in stored order, earlier verse first.
fn stored_pair((x, y): KnownQuotation) -> (i64, i64) {
    let (x, y) = (pack(x), pack(y));
    (x.min(y), x.max(y))
}

/// How a parameter set scores against the curated lists.
pub struct Evaluation {
    /// Verse pairs stored, of each [`LinkKind`] in [`LinkKind::ALL`] order.
    pub pairs: [usize; 3],
    /// Rank of each [`KNOWN_QUOTATIONS`] pair among the OT/NT pairs, `None`
    /// when missed.
    pub known: Vec<(KnownQuotation, Option<usize>)>,
    /// Rank of each [`KNOWN_PARALLELS`] pair among the pairs of its kind.
    pub parallels: Vec<(KnownQuotation, Option<usize>)>,
    /// [`KNOWN_FALSE`] pairs that were stored anyway.
    pub false_found: Vec<KnownQuotation>,
}

impl Evaluation {
    pub fn of(found: &[Found]) -> Self {
        // Ranks count within a kind: same-language pairs score higher, and
        // would otherwise push every OT/NT quotation down the ranking.
        let mut seen = [0usize; 3];
        let mut rank: HashMap<(i64, i64), usize> = HashMap::new();
        for f in found {
            let n = &mut seen[LinkKind::ALL.iter().position(|&k| k == f.kind()).unwrap()];
            *n += 1;
            rank.insert((f.a_ref, f.b_ref), *n);
        }
        let ranks = |list: &[KnownQuotation]| {
            list.iter()
                .map(|&pair| (pair, rank.get(&stored_pair(pair)).copied()))
                .collect()
        };
        Evaluation {
            pairs: seen,
            known: ranks(KNOWN_QUOTATIONS),
            parallels: ranks(KNOWN_PARALLELS),
            false_found: KNOWN_FALSE
                .iter()
                .copied()
                .filter(|&pair| rank.contains_key(&stored_pair(pair)))
                .collect(),
        }
    }

    pub fn known_found(&self) -> usize {
        self.known.iter().filter(|(_, r)| r.is_some()).count()
    }

    pub fn parallels_found(&self) -> usize {
        self.parallels.iter().filter(|(_, r)| r.is_some()).count()
    }

    /// Known quotations ranked within the top `n`.
    pub fn known_within(&self, n: usize) -> usize {
        self.known
            .iter()
            .filter(|(_, r)| r.is_some_and(|r| r <= n))
            .count()
    }

    /// Median rank of the known quotations that were found.
    pub fn median_rank(&self) -> Option<usize> {
        let mut ranks: Vec<usize> = self.known.iter().filter_map(|(_, r)| *r).collect();
        ranks.sort_unstable();
        ranks.get(ranks.len() / 2).copied()
    }

    fn summary_header() -> String {
        format!(
            "{:>7}  {:>7}  {:>7}  {:>7}  {:>8}  {:>8}  {:>7}  {:>9}  {:>6}",
            "OT-NT",
            "OT-OT",
            "NT-NT",
            "known",
            "top-500",
            "top-2000",
            "median",
            "parallels",
            "false"
        )
    }

    fn summary(&self) -> String {
        format!(
            "{:>7}  {:>7}  {:>7}  {:>3}/{:<3}  {:>8}  {:>8}  {:>7}  {:>5}/{:<3}  {:>3}/{:<2}",
            self.pairs[0],
            self.pairs[1],
            self.pairs[2],
            self.known_found(),
            self.known.len(),
            self.known_within(500),
            self.known_within(2000),
            self.median_rank().map_or("-".into(), |r| r.to_string()),
            self.parallels_found(),
            self.parallels.len(),
            self.false_found.len(),
            KNOWN_FALSE.len(),
        )
    }
}

/// Options of `db gen-quotes`.
#[derive(Debug, Default)]
pub struct GenQuotesOptions {
    /// `NAME=VALUE` overrides of the shipped [`Matcher`].
    pub set: Vec<String>,
    /// `NAME=V1,V2,…`: evaluate each value in turn instead of building.
    pub sweep: Option<String>,
    /// Evaluate without writing the table.
    pub dry_run: bool,
    /// How many of the top-ranked pairs of each kind to print.
    pub top: usize,
}

fn params_from(set: &[String]) -> Result<Matcher> {
    let mut params = Matcher::default();
    for assignment in set {
        let (name, value) = assignment
            .split_once('=')
            .with_context(|| format!("expected NAME=VALUE, got {assignment:?}"))?;
        params.set(name.trim(), value.trim())?;
    }
    Ok(params)
}

/// `db gen-quotes`: find the quotations with the shipped (or overridden)
/// parameters, report them against the curated lists, and rebuild the
/// `quotation` table of `path` in place unless it is a dry run or a sweep.
pub fn gen_quotes(path: &std::path::Path, options: &GenQuotesOptions) -> Result<()> {
    let db = Connection::open(path)?;
    let corpus = Corpus::load(&db)?;
    let params = params_from(&options.set)?;

    if let Some(sweep) = &options.sweep {
        let (name, values) = sweep
            .split_once('=')
            .with_context(|| format!("expected NAME=V1,V2,…, got {sweep:?}"))?;
        println!("{:>22}  {}", name, Evaluation::summary_header());
        for value in values.split(',') {
            let mut variant = params;
            variant.set(name.trim(), value.trim())?;
            let evaluation = Evaluation::of(&corpus.find(&variant));
            println!("{:>22}  {}", value.trim(), evaluation.summary());
        }
        return Ok(());
    }

    let found = corpus.find(&params);
    for kind in LinkKind::ALL {
        println!("{}:", kind.label());
        for (rank, f) in found
            .iter()
            .filter(|f| f.kind() == kind)
            .take(options.top)
            .enumerate()
        {
            println!(
                "{:>6}  {:>6.2}  {:>2}  {:<22} {}",
                rank + 1,
                f.alignment.score,
                f.alignment.pairs.len(),
                ref_label(f.b_ref),
                ref_label(f.a_ref),
            );
        }
    }
    let evaluation = Evaluation::of(&found);
    let label = |r: Reference| ref_label(pack(r));
    for (title, list) in [
        ("Known quotations", &evaluation.known),
        ("Known parallels", &evaluation.parallels),
    ] {
        println!("\n{title}:");
        for ((later, earlier), rank) in list {
            let rank = rank.map_or("-".into(), |r| r.to_string());
            println!("  {rank:>6}  {:<22} {}", label(*later), label(*earlier));
        }
    }
    for (later, earlier) in &evaluation.false_found {
        println!(
            "  known false positive kept: {} / {}",
            label(*later),
            label(*earlier)
        );
    }
    println!(
        "\n{}\n{}",
        Evaluation::summary_header(),
        evaluation.summary()
    );
    if options.dry_run {
        println!("(dry run: {} left unchanged)", path.display());
    } else {
        write_quotations(&db, &found)?;
        crate::runtime_db::restamp_built(&db)?;
        println!(
            "Wrote {} quotation pairs to {}",
            found.len(),
            path.display()
        );
    }
    Ok(())
}

/// Print how one verse pair (either order, any testaments) tokenises and
/// aligns under the shipped (or overridden) parameters — the tuning aid for a
/// pair the table gets wrong.
pub fn explain_pair(path: &std::path::Path, x_ref: i64, y_ref: i64, set: &[String]) -> Result<()> {
    let db = Connection::open(path)?;
    let corpus = Corpus::load(&db)?;
    let params = params_from(set)?;
    let (a_ref, b_ref) = (x_ref.min(y_ref), x_ref.max(y_ref));
    let kind = LinkKind::of(a_ref, b_ref);
    let sides = corpus.sides(kind);
    let (Some(a), Some(b)) = (sides.verse(a_ref), sides.verse(b_ref)) else {
        bail!("verse not found");
    };
    println!("{} pair", kind.label());
    for verse in [b, a] {
        println!("{}", ref_label(verse.reference));
        for (pos, ks) in &verse.words {
            let shown: Vec<String> = ks
                .iter()
                .map(|&k| {
                    format!(
                        "{}({:.1})",
                        sides.names[k as usize], sides.weight[k as usize]
                    )
                })
                .collect();
            println!("  {pos:>3} {}", shown.join(" "));
        }
    }
    let params = params.params(kind);
    let alignment = sides.align(a, b, params);
    let stored = alignment.score >= params.min_score
        && alignment.pairs.len() >= params.min_matched
        && alignment.strong >= params.min_strong
        && !(sides.same && a_ref >> 8 == b_ref >> 8);
    println!(
        "score {:.2}, {} strong keys, pairs {:?} — {}",
        alignment.score,
        alignment.strong,
        alignment.pairs,
        if stored {
            "stored (unless over the per-verse cap)"
        } else {
            "not stored"
        }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_fold_finals_and_geminates() {
        assert_eq!(normalise_key("עמם"), "עמ");
        assert_eq!(normalise_key("שָׁלוֹם"), "שלומ");
        assert_eq!(normalise_key("שלמ"), "שלמ");
    }

    #[test]
    fn aramaic_keys_reach_their_hebrew_cognates() {
        assert!(hebrew_candidates("דהב").contains(&"זהב".to_string()));
        assert!(hebrew_candidates("תלג").contains(&"שלג".to_string()));
        assert!(hebrew_candidates("בנא").contains(&"בנה".to_string()));
        assert!(hebrew_candidates("שנתא").contains(&"שנה".to_string()));
        assert!(hebrew_candidates("מצרינ").contains(&"מצרימ".to_string()));
        // Plene and defective spellings meet on their shared defective key.
        assert!(hebrew_candidates("דויד").contains(&defective("דוד").unwrap()));
        assert!(hebrew_candidates("איסראיל").contains(&"ישראל".to_string()));
        assert!(hebrew_candidates("שבק").contains(&"עזב".to_string()));
    }
}
