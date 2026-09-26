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

use anyhow::Result;
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

/// Keys rarer than this (in weight) are ignored when proposing candidates;
/// they still count once the pair is aligned.
const CANDIDATE_MIN_WEIGHT: f32 = 1.5;
/// A pair needs this many distinct shared candidate keys to be aligned at all.
const CANDIDATE_MIN_SHARED: usize = 2;
/// Keys below this weight never contribute a match (particles, pronouns).
const MATCH_MIN_WEIGHT: f32 = 0.5;
/// Cost of each word the alignment skips on either side.
const GAP_PENALTY: f32 = 0.6;
/// Added to a match that directly continues the previous one on both sides.
const CONTIGUITY_BONUS: f32 = 1.0;
/// Minimum local-alignment score for a pair to be stored.
const MIN_SCORE: f32 = 7.0;
/// Minimum number of aligned word pairs for a pair to be stored.
const MIN_MATCHED: usize = 3;
/// At most this many OT verses are kept per NT verse.
const MAX_PER_NT_VERSE: usize = 8;

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
}

/// Smith–Waterman local alignment over word key sets.
fn align(ot: &Verse, nt: &Verse, weight: &[f32]) -> Alignment {
    let (m, n) = (ot.words.len(), nt.words.len());
    let match_weight = |i: usize, j: usize| -> f32 {
        let (a, b) = (&ot.words[i].1, &nt.words[j].1);
        let mut best = 0.0f32;
        let (mut x, mut y) = (0, 0);
        while x < a.len() && y < b.len() {
            match a[x].cmp(&b[y]) {
                std::cmp::Ordering::Less => x += 1,
                std::cmp::Ordering::Greater => y += 1,
                std::cmp::Ordering::Equal => {
                    best = best.max(weight[a[x] as usize]);
                    x += 1;
                    y += 1;
                }
            }
        }
        if best >= MATCH_MIN_WEIGHT { best } else { 0.0 }
    };
    let w = n + 1;
    let mut h = vec![0.0f32; (m + 1) * w];
    let mut matched = vec![false; (m + 1) * w];
    let (mut best, mut best_at) = (0.0f32, (0, 0));
    for i in 1..=m {
        for j in 1..=n {
            let s = match_weight(i - 1, j - 1);
            // A match directly continuing the previous one is a phrase, not a
            // coincidence of scattered words.
            let run = if matched[(i - 1) * w + j - 1] {
                CONTIGUITY_BONUS
            } else {
                0.0
            };
            let diag = if s > 0.0 {
                h[(i - 1) * w + j - 1] + s + run
            } else {
                0.0
            };
            let up = h[(i - 1) * w + j] - GAP_PENALTY;
            let left = h[i * w + j - 1] - GAP_PENALTY;
            let cell = diag.max(up).max(left).max(0.0);
            h[i * w + j] = cell;
            matched[i * w + j] = s > 0.0 && cell == diag;
            if cell > best {
                best = cell;
                best_at = (i, j);
            }
        }
    }
    let mut pairs = Vec::new();
    let (mut i, mut j) = best_at;
    while i > 0 && j > 0 && h[i * w + j] > 0.0 {
        if matched[i * w + j] {
            pairs.push((ot.words[i - 1].0, nt.words[j - 1].0));
            i -= 1;
            j -= 1;
        } else if h[(i - 1) * w + j] - GAP_PENALTY == h[i * w + j] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    pairs.reverse();
    Alignment { score: best, pairs }
}

/// A stored quotation before it is ranked.
struct Found {
    ot_ref: i64,
    nt_ref: i64,
    alignment: Alignment,
}

/// Rebuild the `quotation` table of a runtime database in place. Returns the
/// number of verse pairs written.
pub fn build_quotations(db: &Connection) -> Result<usize> {
    let mut keys = Keys::default();
    let ot = load_ot(db, &mut keys)?;
    let nt = load_nt(db, &keys)?;
    let n_keys = keys.ids.len();
    let (idf_ot, idf_nt) = (idf(&ot, n_keys), idf(&nt, n_keys));
    let weight: Vec<f32> = idf_ot.iter().zip(&idf_nt).map(|(a, b)| a.min(*b)).collect();
    info!(
        "Quotations: {} OT verses, {} NT verses, {n_keys} root keys",
        ot.len(),
        nt.len()
    );

    let mut postings: Vec<Vec<u32>> = vec![Vec::new(); n_keys];
    for (v, verse) in ot.iter().enumerate() {
        let distinct: HashSet<u32> = verse
            .words
            .iter()
            .flat_map(|(_, k)| k.iter().copied())
            .collect();
        for k in distinct {
            if weight[k as usize] >= CANDIDATE_MIN_WEIGHT {
                postings[k as usize].push(v as u32);
            }
        }
    }

    let mut found: Vec<Found> = Vec::new();
    let mut shared = vec![0u16; ot.len()];
    let mut touched: Vec<u32> = Vec::new();
    for verse in &nt {
        let distinct: HashSet<u32> = verse
            .words
            .iter()
            .flat_map(|(_, k)| k.iter().copied())
            .collect();
        for k in distinct {
            for &v in &postings[k as usize] {
                if shared[v as usize] == 0 {
                    touched.push(v);
                }
                shared[v as usize] += 1;
            }
        }
        let mut here: Vec<Found> = Vec::new();
        for &v in &touched {
            if shared[v as usize] as usize >= CANDIDATE_MIN_SHARED {
                let alignment = align(&ot[v as usize], verse, &weight);
                if alignment.score >= MIN_SCORE && alignment.pairs.len() >= MIN_MATCHED {
                    here.push(Found {
                        ot_ref: ot[v as usize].reference,
                        nt_ref: verse.reference,
                        alignment,
                    });
                }
            }
            shared[v as usize] = 0;
        }
        touched.clear();
        here.sort_by(|a, b| b.alignment.score.total_cmp(&a.alignment.score));
        here.truncate(MAX_PER_NT_VERSE);
        found.extend(here);
    }
    found.sort_by(|a, b| {
        b.alignment
            .score
            .total_cmp(&a.alignment.score)
            .then(a.nt_ref.cmp(&b.nt_ref))
            .then(a.ot_ref.cmp(&b.ot_ref))
    });

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
                (f.alignment.score * 100.0).round() / 100.0,
                pairs.len() as i64,
                join(&mut pairs.iter().map(|p| p.0)),
                join(&mut pairs.iter().map(|p| p.1)),
            ])?;
        }
    }
    tx.commit()?;
    info!("Quotations: wrote {} verse pairs", found.len());
    Ok(found.len())
}

/// Rank of each [`KNOWN_QUOTATIONS`] pair in a built `quotation` table (`None`
/// when it was not found), for measuring recall.
pub fn known_quotation_ranks(db: &Connection) -> Result<Vec<(KnownQuotation, Option<i64>)>> {
    let mut stmt =
        db.prepare("SELECT MIN(quote_id) FROM quotation WHERE nt_ref = ?1 AND ot_ref = ?2")?;
    KNOWN_QUOTATIONS
        .iter()
        .map(|&(nt, ot)| {
            let rank: Option<i64> = stmt.query_row(
                params![
                    pack_ref(nt.0.into(), nt.1.into(), nt.2.into()),
                    pack_ref(ot.0.into(), ot.1.into(), ot.2.into())
                ],
                |row| row.get(0),
            )?;
            Ok(((nt, ot), rank))
        })
        .collect()
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

/// `db gen-quotes`: rebuild the `quotation` table of `path` in place, print
/// the strongest `top` pairs and the recall on [`KNOWN_QUOTATIONS`].
pub fn gen_quotes(path: &std::path::Path, top: usize) -> Result<()> {
    let db = Connection::open(path)?;
    let total = build_quotations(&db)?;
    println!("Wrote {total} quotation pairs to {}", path.display());
    let mut stmt = db.prepare(
        "SELECT quote_id, score, matched, ot_ref, nt_ref FROM quotation
         ORDER BY quote_id LIMIT ?1",
    )?;
    let mut rows = stmt.query([top as i64])?;
    while let Some(row) = rows.next()? {
        println!(
            "{:>6}  {:>6.2}  {:>2}  {:<22} {}",
            row.get::<_, i64>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, i64>(2)?,
            ref_label(row.get(4)?),
            ref_label(row.get(3)?),
        );
    }
    let ranks = known_quotation_ranks(&db)?;
    let hits = ranks.iter().filter(|(_, r)| r.is_some()).count();
    println!("\nKnown quotations found: {hits}/{}", ranks.len());
    for ((nt, ot), rank) in ranks {
        let label = |(b, c, v): Reference| ref_label(pack_ref(b.into(), c.into(), v.into()));
        match rank {
            Some(r) => println!("  {:>6}  {:<22} {}", r, label(nt), label(ot)),
            None => println!("       -  {:<22} {}", label(nt), label(ot)),
        }
    }
    Ok(())
}

/// Print how one NT/OT verse pair tokenises and aligns — the tuning aid for
/// a known quotation the table misses.
pub fn explain_pair(path: &std::path::Path, nt_ref: i64, ot_ref: i64) -> Result<()> {
    let db = Connection::open(path)?;
    let mut keys = Keys::default();
    let ot = load_ot(&db, &mut keys)?;
    let nt = load_nt(&db, &keys)?;
    let n_keys = keys.ids.len();
    let (idf_ot, idf_nt) = (idf(&ot, n_keys), idf(&nt, n_keys));
    let weight: Vec<f32> = idf_ot.iter().zip(&idf_nt).map(|(a, b)| a.min(*b)).collect();
    let names: HashMap<u32, &str> = keys.ids.iter().map(|(k, &v)| (v, k.as_str())).collect();
    let find = |verses: &[Verse], r: i64| verses.iter().position(|v| v.reference == r);
    let (Some(o), Some(n)) = (find(&ot, ot_ref), find(&nt, nt_ref)) else {
        anyhow::bail!("verse not found");
    };
    for (label, verse) in [("NT", &nt[n]), ("OT", &ot[o])] {
        println!("{label} {}", ref_label(verse.reference));
        for (pos, ks) in &verse.words {
            let shown: Vec<String> = ks
                .iter()
                .map(|k| format!("{}({:.1})", names[k], weight[*k as usize]))
                .collect();
            println!("  {pos:>3} {}", shown.join(" "));
        }
    }
    let a = align(&ot[o], &nt[n], &weight);
    println!("score {:.2}, pairs {:?}", a.score, a.pairs);
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
