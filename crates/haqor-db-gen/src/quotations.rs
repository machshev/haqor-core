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
//! The result is the `quotation` table: one row per (OT verse, NT verse) pair,
//! `quote_id` doubling as the global rank (1 = strongest).

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail};
use haqor_core::transliterate::lookup_key;
use log::info;
use rusqlite::{Connection, params};

use crate::runtime_db::pack_ref;

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS quotation;
CREATE TABLE quotation(
    quote_id     INTEGER PRIMARY KEY,
    ot_ref       INTEGER NOT NULL,
    nt_ref       INTEGER NOT NULL,
    score        REAL    NOT NULL,
    matched      INTEGER NOT NULL,
    ot_positions TEXT    NOT NULL,
    nt_positions TEXT    NOT NULL
);
CREATE INDEX idx_quotation_ot ON quotation(ot_ref);
CREATE INDEX idx_quotation_nt ON quotation(nt_ref);
";

/// The matcher's tuning knobs. [`Default`] is what `gen-runtime` ships: a
/// deliberately loose set, echoes and allusions included, since a reader filters
/// by score at run time and a missed quotation cannot be recovered there.
/// Tuned 2026-09-26 by sweep: gap 0.3 ranks the known quotations best, and
/// min score 6 with 12 per NT verse keeps 37 of 55 at ~60k pairs.
///
/// `db gen-quotes --set NAME=VALUE` / `--sweep NAME=V1,V2,…` try others
/// against the known quotations without a rebuild (see [`MatcherParams::set`]
/// for the names).
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
    /// Minimum local-alignment score for a pair to be stored.
    pub min_score: f32,
    /// Minimum number of aligned word pairs for a pair to be stored.
    pub min_matched: usize,
    /// Minimum number of distinct keys of at least [`Self::strong_weight`]
    /// among the aligned words.
    pub min_strong: usize,
    /// What counts as a strong key for [`Self::min_strong`].
    pub strong_weight: f32,
    /// At most this many OT verses are kept per NT verse.
    pub max_per_nt_verse: usize,
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
            max_per_nt_verse: 12,
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
        "max_per_nt_verse",
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
            "max_per_nt_verse" => self.max_per_nt_verse = count()?,
            _ => bail!(
                "unknown matcher parameter {name:?}; known: {}",
                Self::NAMES.join(", ")
            ),
        }
        Ok(())
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

/// (NT verse, OT verse it quotes).
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
        let reference: i64 = row.get(0)?;
        let position: i64 = row.get(1)?;
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
        let mut word: Vec<u32> = forms
            .into_iter()
            .chain(defectives)
            .map(|k| keys.intern(k))
            .collect();
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
    Ok(verses)
}

fn load_nt(db: &Connection, keys: &Keys) -> Result<Vec<Verse>> {
    let mut stmt = db.prepare(
        "SELECT n.ref, n.ord, COALESCE(r.root, '')
         FROM nt_word n
         JOIN syriac_word w ON w.word_id = n.word_id
         LEFT JOIN syriac_lexeme l ON l.lexeme_id = w.lexeme_id
         LEFT JOIN syriac_root r ON r.root_id = l.root_id
         ORDER BY n.ref, n.ord",
    )?;
    let mut rows = stmt.query([])?;
    let mut cache: HashMap<String, Vec<u32>> = HashMap::new();
    let mut verses: Vec<Verse> = Vec::new();
    while let Some(row) = rows.next()? {
        let reference: i64 = row.get(0)?;
        let root = normalise_key(&row.get::<_, String>(2)?);
        let word = cache
            .entry(root.clone())
            .or_insert_with(|| {
                let mut ids: Vec<u32> = hebrew_candidates(&root)
                    .iter()
                    .filter_map(|k| keys.get(k))
                    .collect();
                ids.sort_unstable();
                ids.dedup();
                ids
            })
            .clone();
        if verses.last().is_none_or(|v| v.reference != reference) {
            verses.push(Verse {
                reference,
                words: Vec::new(),
            });
        }
        verses.last_mut().unwrap().words.push((row.get(1)?, word));
    }
    Ok(verses)
}

/// Inverse document frequency of every key over a testament's verses.
fn idf(verses: &[Verse], n_keys: usize) -> Vec<f32> {
    let mut df = vec![0u32; n_keys];
    for verse in verses {
        let distinct: HashSet<u32> = verse
            .words
            .iter()
            .flat_map(|(_, k)| k.iter().copied())
            .collect();
        for k in distinct {
            df[k as usize] += 1;
        }
    }
    let n = verses.len() as f32;
    df.into_iter()
        .map(|d| if d == 0 { 0.0 } else { (n / d as f32).ln() })
        .collect()
}

/// One aligned verse pair.
pub struct Alignment {
    pub score: f32,
    pub pairs: Vec<(i64, i64)>,
    /// Distinct keys of at least [`MatcherParams::strong_weight`] among the
    /// aligned words.
    pub strong: usize,
}

/// Both testaments tokenised and weighted: everything the matcher reads,
/// loaded once so a sweep can try many [`MatcherParams`] against it.
pub struct Corpus {
    ot: Vec<Verse>,
    nt: Vec<Verse>,
    weight: Vec<f32>,
    names: Vec<String>,
}

impl Corpus {
    pub fn load(db: &Connection) -> Result<Self> {
        let mut keys = Keys::default();
        let ot = load_ot(db, &mut keys)?;
        let nt = load_nt(db, &keys)?;
        let n_keys = keys.ids.len();
        let (idf_ot, idf_nt) = (idf(&ot, n_keys), idf(&nt, n_keys));
        let weight = idf_ot.iter().zip(&idf_nt).map(|(a, b)| a.min(*b)).collect();
        let mut names = vec![String::new(); n_keys];
        for (key, id) in keys.ids {
            names[id as usize] = key;
        }
        info!(
            "Quotations: {} OT verses, {} NT verses, {n_keys} root keys",
            ot.len(),
            nt.len()
        );
        Ok(Corpus {
            ot,
            nt,
            weight,
            names,
        })
    }

    /// The best shared key of an OT and an NT word, if it may match at all.
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
    fn align(&self, ot: &Verse, nt: &Verse, params: &MatcherParams) -> Alignment {
        let (m, n) = (ot.words.len(), nt.words.len());
        let w = n + 1;
        let mut h = vec![0.0f32; (m + 1) * w];
        // The key a cell's match was made on, where the cell is a match.
        let mut matched: Vec<Option<u32>> = vec![None; (m + 1) * w];
        let (mut best, mut best_at) = (0.0f32, (0, 0));
        for i in 1..=m {
            for j in 1..=n {
                let key = self.shared_key(&ot.words[i - 1].1, &nt.words[j - 1].1, params);
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
                pairs.push((ot.words[i - 1].0, nt.words[j - 1].0));
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
            score: best,
            pairs,
            strong: strong.len(),
        }
    }

    /// Every stored-quality pair under `params`, strongest first.
    pub fn find(&self, params: &MatcherParams) -> Vec<Found> {
        let weight = &self.weight;
        let mut postings: Vec<Vec<u32>> = vec![Vec::new(); weight.len()];
        for (v, verse) in self.ot.iter().enumerate() {
            for k in distinct_keys(verse) {
                if weight[k as usize] >= params.candidate_min_weight {
                    postings[k as usize].push(v as u32);
                }
            }
        }

        let mut found: Vec<Found> = Vec::new();
        let mut shared = vec![0u16; self.ot.len()];
        let mut touched: Vec<u32> = Vec::new();
        for verse in &self.nt {
            for k in distinct_keys(verse) {
                for &v in &postings[k as usize] {
                    if shared[v as usize] == 0 {
                        touched.push(v);
                    }
                    shared[v as usize] += 1;
                }
            }
            let mut here: Vec<Found> = Vec::new();
            for &v in &touched {
                if shared[v as usize] as usize >= params.candidate_min_shared {
                    let ot = &self.ot[v as usize];
                    let alignment = self.align(ot, verse, params);
                    if alignment.score >= params.min_score
                        && alignment.pairs.len() >= params.min_matched
                        && alignment.strong >= params.min_strong
                    {
                        here.push(Found {
                            ot_ref: ot.reference,
                            nt_ref: verse.reference,
                            alignment,
                        });
                    }
                }
                shared[v as usize] = 0;
            }
            touched.clear();
            here.sort_by(|a, b| b.alignment.score.total_cmp(&a.alignment.score));
            here.truncate(params.max_per_nt_verse);
            found.extend(here);
        }
        found.sort_by(|a, b| {
            b.alignment
                .score
                .total_cmp(&a.alignment.score)
                .then(a.nt_ref.cmp(&b.nt_ref))
                .then(a.ot_ref.cmp(&b.ot_ref))
        });
        found
    }

    fn verse(&self, reference: i64) -> Option<&Verse> {
        let verses = if reference >> 16 >= 40 {
            &self.nt
        } else {
            &self.ot
        };
        verses.iter().find(|v| v.reference == reference)
    }
}

fn distinct_keys(verse: &Verse) -> HashSet<u32> {
    verse
        .words
        .iter()
        .flat_map(|(_, k)| k.iter().copied())
        .collect()
}

/// A quotation the matcher found, before it is stored.
pub struct Found {
    pub ot_ref: i64,
    pub nt_ref: i64,
    pub alignment: Alignment,
}

/// Replace the `quotation` table with `found`, ranked in the order given.
fn write_quotations(db: &Connection, found: &[Found]) -> Result<()> {
    let tx = db.unchecked_transaction()?;
    tx.execute_batch(SCHEMA)?;
    {
        let mut insert = tx.prepare(
            "INSERT INTO quotation(quote_id, ot_ref, nt_ref, score, matched, ot_positions,
                                   nt_positions) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        let join = |xs: &mut dyn Iterator<Item = i64>| {
            xs.map(|x| x.to_string()).collect::<Vec<_>>().join(" ")
        };
        for (rank, f) in found.iter().enumerate() {
            let pairs = &f.alignment.pairs;
            insert.execute(params![
                rank as i64 + 1,
                f.ot_ref,
                f.nt_ref,
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
    let found = Corpus::load(db)?.find(&MatcherParams::default());
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

/// How a parameter set scores against the curated lists.
pub struct Evaluation {
    /// Verse pairs stored.
    pub pairs: usize,
    /// Rank of each [`KNOWN_QUOTATIONS`] pair, `None` when missed.
    pub known: Vec<(KnownQuotation, Option<usize>)>,
    /// [`KNOWN_FALSE`] pairs that were stored anyway.
    pub false_found: Vec<KnownQuotation>,
}

impl Evaluation {
    pub fn of(found: &[Found]) -> Self {
        let rank: HashMap<(i64, i64), usize> = found
            .iter()
            .enumerate()
            .map(|(i, f)| ((f.nt_ref, f.ot_ref), i + 1))
            .collect();
        Evaluation {
            pairs: found.len(),
            known: KNOWN_QUOTATIONS
                .iter()
                .map(|&(nt, ot)| ((nt, ot), rank.get(&(pack(nt), pack(ot))).copied()))
                .collect(),
            false_found: KNOWN_FALSE
                .iter()
                .copied()
                .filter(|&(nt, ot)| rank.contains_key(&(pack(nt), pack(ot))))
                .collect(),
        }
    }

    pub fn known_found(&self) -> usize {
        self.known.iter().filter(|(_, r)| r.is_some()).count()
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
            "{:>7}  {:>7}  {:>8}  {:>8}  {:>7}  {:>6}",
            "pairs", "known", "top-500", "top-2000", "median", "false"
        )
    }

    fn summary(&self) -> String {
        format!(
            "{:>7}  {:>3}/{:<3}  {:>8}  {:>8}  {:>7}  {:>3}/{:<2}",
            self.pairs,
            self.known_found(),
            self.known.len(),
            self.known_within(500),
            self.known_within(2000),
            self.median_rank().map_or("-".into(), |r| r.to_string()),
            self.false_found.len(),
            KNOWN_FALSE.len(),
        )
    }
}

/// Options of `db gen-quotes`.
#[derive(Debug, Default)]
pub struct GenQuotesOptions {
    /// `NAME=VALUE` overrides of the shipped [`MatcherParams`].
    pub set: Vec<String>,
    /// `NAME=V1,V2,…`: evaluate each value in turn instead of building.
    pub sweep: Option<String>,
    /// Evaluate without writing the table.
    pub dry_run: bool,
    /// How many of the top-ranked pairs to print.
    pub top: usize,
}

fn params_from(set: &[String]) -> Result<MatcherParams> {
    let mut params = MatcherParams::default();
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
    for (rank, f) in found.iter().take(options.top).enumerate() {
        println!(
            "{:>6}  {:>6.2}  {:>2}  {:<22} {}",
            rank + 1,
            f.alignment.score,
            f.alignment.pairs.len(),
            ref_label(f.nt_ref),
            ref_label(f.ot_ref),
        );
    }
    let evaluation = Evaluation::of(&found);
    let label = |r: Reference| ref_label(pack(r));
    println!("\nKnown quotations:");
    for ((nt, ot), rank) in &evaluation.known {
        let rank = rank.map_or("-".into(), |r| r.to_string());
        println!("  {rank:>6}  {:<22} {}", label(*nt), label(*ot));
    }
    for (nt, ot) in &evaluation.false_found {
        println!(
            "  known false positive kept: {} / {}",
            label(*nt),
            label(*ot)
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
        println!(
            "Wrote {} quotation pairs to {}",
            found.len(),
            path.display()
        );
    }
    Ok(())
}

/// Print how one NT/OT verse pair tokenises and aligns under the shipped (or
/// overridden) parameters — the tuning aid for a pair the table gets wrong.
pub fn explain_pair(
    path: &std::path::Path,
    nt_ref: i64,
    ot_ref: i64,
    set: &[String],
) -> Result<()> {
    let db = Connection::open(path)?;
    let corpus = Corpus::load(&db)?;
    let params = params_from(set)?;
    let (Some(ot), Some(nt)) = (corpus.verse(ot_ref), corpus.verse(nt_ref)) else {
        bail!("verse not found");
    };
    for (label, verse) in [("NT", nt), ("OT", ot)] {
        println!("{label} {}", ref_label(verse.reference));
        for (pos, ks) in &verse.words {
            let shown: Vec<String> = ks
                .iter()
                .map(|&k| {
                    format!(
                        "{}({:.1})",
                        corpus.names[k as usize], corpus.weight[k as usize]
                    )
                })
                .collect();
            println!("  {pos:>3} {}", shown.join(" "));
        }
    }
    let a = corpus.align(ot, nt, &params);
    let stored = a.score >= params.min_score
        && a.pairs.len() >= params.min_matched
        && a.strong >= params.min_strong;
    println!(
        "score {:.2}, {} strong keys, pairs {:?} — {}",
        a.score,
        a.strong,
        a.pairs,
        if stored { "stored" } else { "not stored" }
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
