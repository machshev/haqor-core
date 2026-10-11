//! Build the prophecies of the Old Testament and where they are fulfilled.
//!
//! A prophecy is an OT passage; its fulfilments, if the Bible records any, are
//! the passages that take it up — NT quotations and allusions, types, and
//! events narrated in either testament. Many prophecies have none: a kingdom
//! prophecy still awaiting the return of Christ, or the fall of Tyre, which
//! history records but the Bible does not.
//!
//! Two sources feed the tables:
//!
//! - `data/prophecies.json`, Haqor's own curation (see `data/PROPHECIES.md`):
//!   the prophecies with their titles, themes and fulfilments, the OT passages
//!   each theme covers, and the found links rejected on review.
//! - Detection: an NT verse with a citation formula ("that it might be
//!   fulfilled which was spoken by the prophet", "it is written", "the
//!   scripture") is linked to the OT verse it quotes, by the `quotation`
//!   table's root alignment and the Treasury of Scripture Knowledge's
//!   references in it and the two verses after it. A found link inside a
//!   curated prophecy is added to it (or confirms the curated fulfilment it
//!   overlaps); one outside any, after a fulfilment formula, starts a
//!   prophecy of its own, untitled, for review.
//!
//! The result is the `prophecy` and `prophecy_fulfilment` tables.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use log::info;
use rusqlite::{Connection, params};
use serde_json::Value;

use crate::runtime_db::pack_ref;
use crate::tsk::Versification;

/// The curation file in the data directory.
pub const PROPHECIES: &str = "prophecies.json";

/// The themes a prophecy may have: the six of the promises concerning Christ,
/// then three of prophecy in general.
pub const THEMES: &[&str] = &[
    "seed-of-woman",
    "abrahamic",
    "davidic",
    "servant",
    "kingdom",
    "priest",
    "israel",
    "nations",
    "empires",
];

/// When a prophecy is fulfilled, in order: in its own day or the history
/// that followed before Christ, in his first coming, in the judgment of AD 70,
/// at his return, and at the end of the thousand years. A prophecy may be
/// fulfilled at several: first for those it was spoken to, then echoed
/// forward.
pub const STAGES: &[&str] = &[
    "history",
    "first-advent",
    "ad70",
    "return",
    "millennium-end",
];

/// The stage of a fulfilling passage the curation does not name: an OT
/// passage's is history, Revelation's the return, the rest of the NT's
/// Christ's first coming.
fn default_stage(passage: Span) -> &'static str {
    match passage.0 >> 16 {
        book if book < 40 => "history",
        66 => "return",
        _ => "first-advent",
    }
}

/// The stages of a prophecy: those the curation names for it and those of
/// its fulfilments, in [`STAGES`]' order, space separated.
fn stages_of(p: &Prophecy) -> String {
    STAGES
        .iter()
        .copied()
        .filter(|stage| {
            p.stages.iter().any(|s| s == stage) || p.fulfilments.iter().any(|f| f.stage == *stage)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// How a passage fulfils a prophecy.
pub const KINDS: &[&str] = &["quotation", "allusion", "type", "event"];

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS prophecy;
DROP TABLE IF EXISTS prophecy_fulfilment;
DROP TABLE IF EXISTS prophecy_pattern;
DROP TABLE IF EXISTS prophecy_pattern_member;
CREATE TABLE prophecy(
    prophecy_id INTEGER PRIMARY KEY,
    ot_ref      INTEGER NOT NULL,
    ot_end      INTEGER NOT NULL,
    title       TEXT    NOT NULL,
    theme       TEXT,
    messianic   INTEGER NOT NULL,
    stages      TEXT    NOT NULL,
    curated     INTEGER NOT NULL,
    note        TEXT    NOT NULL
);
CREATE INDEX idx_prophecy_ot ON prophecy(ot_ref);
CREATE TABLE prophecy_fulfilment(
    prophecy_id INTEGER NOT NULL,
    ord         INTEGER NOT NULL,
    ref         INTEGER NOT NULL,
    end_ref     INTEGER NOT NULL,
    ot_ref      INTEGER NOT NULL,
    ot_end      INTEGER NOT NULL,
    kind        TEXT    NOT NULL,
    stage       TEXT    NOT NULL,
    formula     TEXT,
    reviewed    INTEGER NOT NULL,
    confidence  TEXT,
    note        TEXT    NOT NULL,
    PRIMARY KEY(prophecy_id, ord)
) WITHOUT ROWID;
CREATE INDEX idx_prophecy_fulfilment_ref ON prophecy_fulfilment(ref);
CREATE TABLE prophecy_pattern(
    pattern_id INTEGER PRIMARY KEY,
    title      TEXT NOT NULL,
    note       TEXT NOT NULL
);
CREATE TABLE prophecy_pattern_member(
    pattern_id  INTEGER NOT NULL,
    ord         INTEGER NOT NULL,
    prophecy_id INTEGER NOT NULL,
    stage       TEXT    NOT NULL,
    label       TEXT    NOT NULL,
    PRIMARY KEY(pattern_id, ord)
) WITHOUT ROWID;
CREATE INDEX idx_prophecy_pattern_member ON prophecy_pattern_member(prophecy_id);
";

/// An inclusive run of packed verse refs.
type Span = (i64, i64);

fn overlaps(a: Span, b: Span) -> bool {
    a.0 <= b.1 && b.0 <= a.1
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.0 <= inner.0 && inner.1 <= outer.1
}

/// A passage as the curation file writes it, in STEP Bible's book
/// abbreviations and the English Bible's numbering: `Gen.3.15`,
/// `Isa.52.13-15`, `Isa.52.13-53.12`. OT verses are renumbered onto the
/// Hebrew text.
fn parse_span(text: &str, versification: &Versification) -> Option<Span> {
    let (start, end) = match text.split_once('-') {
        Some((start, end)) => (start, Some(end)),
        None => (text, None),
    };
    let (book, chapter, verse) = crate::journeys::parse_ref(start)?;
    let (last_chapter, last_verse) = match end.map(|e| e.split('.').collect::<Vec<_>>()) {
        None => (chapter, verse),
        Some(parts) if parts.len() == 1 => (chapter, parts[0].parse().ok()?),
        Some(parts) if parts.len() == 2 => (parts[0].parse().ok()?, parts[1].parse().ok()?),
        Some(_) => return None,
    };
    let first = versification.first((book, chapter, verse));
    let last = versification.last((book, last_chapter, last_verse));
    let pack = |v: (u8, u8, u8)| pack_ref(v.0.into(), v.1.into(), v.2.into());
    let span = (pack(first), pack(last));
    (span.0 <= span.1).then_some(span)
}

/// One fulfilment row before it is written.
#[derive(Debug, Clone)]
struct Fulfilment {
    span: Span,
    /// The verses of the prophecy it takes up.
    ot: Span,
    kind: String,
    /// One of [`STAGES`].
    stage: String,
    formula: Option<String>,
    reviewed: bool,
    confidence: Option<&'static str>,
    note: String,
}

#[derive(Debug)]
struct Prophecy {
    span: Span,
    title: String,
    theme: Option<String>,
    messianic: bool,
    /// The stages the curation names beyond its fulfilments': where the
    /// Bible records no fulfilment, or not all of them.
    stages: Vec<String>,
    curated: bool,
    note: String,
    fulfilments: Vec<Fulfilment>,
}

/// What a build found, for the log and the tests.
#[derive(Debug, Default)]
pub struct ProphecySummary {
    pub curated: usize,
    /// Prophecies started from a found link alone.
    pub found: usize,
    pub fulfilments: usize,
    /// Found links that confirmed a curated fulfilment.
    pub confirmed: usize,
    /// Found links added to a curated prophecy.
    pub added: usize,
}

/// A series of prophecies repeating one pattern, like the invasions of the
/// land from the north: each member a prophecy (by its passage) at a stage.
#[derive(Debug)]
struct Pattern {
    title: String,
    note: String,
    members: Vec<(Span, String, String)>,
}

/// The parsed curation file.
struct Curation {
    themes: Vec<(Span, String)>,
    prophecies: Vec<Prophecy>,
    patterns: Vec<Pattern>,
    /// (NT verse, OT verse) pairs a found link is dropped for.
    rejected: HashSet<(i64, i64)>,
}

/// Reads the curation file's fields, collecting every fault instead of
/// stopping at the first.
struct Checker<'a> {
    versification: &'a Versification,
    verses: &'a HashSet<i64>,
    faults: Vec<String>,
}

impl Checker<'_> {
    /// A passage of the corpus.
    fn span(&mut self, text: Option<&str>, what: &str) -> Option<Span> {
        let Some(text) = text else {
            self.faults.push(format!("{what}: no passage"));
            return None;
        };
        match parse_span(text, self.versification) {
            Some(s) if self.verses.contains(&s.0) && self.verses.contains(&s.1) => Some(s),
            Some(_) => {
                self.faults.push(format!("{what}: {text} is outside the corpus"));
                None
            }
            None => {
                self.faults.push(format!("{what}: cannot read {text:?}"));
                None
            }
        }
    }

    /// One of `allowed`, or nothing when the field is absent.
    fn one_of(&mut self, value: Option<&str>, allowed: &[&str], what: &str) -> Option<String> {
        let value = value?;
        if allowed.contains(&value) {
            Some(value.to_string())
        } else {
            self.faults.push(format!("{what}: unknown {value:?}"));
            None
        }
    }
}

fn read_curation(path: &Path, versification: &Versification, verses: &HashSet<i64>) -> Result<Curation> {
    let json: Value = serde_json::from_str(
        &std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
    )
    .with_context(|| format!("parsing {}", path.display()))?;
    let mut check = Checker {
        versification,
        verses,
        faults: Vec::new(),
    };

    let mut themes = Vec::new();
    for (theme, passages) in json["themes"].as_object().into_iter().flatten() {
        let Some(theme) = check.one_of(Some(theme), THEMES, "themes") else {
            continue;
        };
        for passage in passages.as_array().into_iter().flatten() {
            if let Some(s) = check.span(passage.as_str(), &format!("theme {theme}")) {
                themes.push((s, theme.clone()));
            }
        }
    }

    let mut prophecies = Vec::new();
    for entry in json["prophecies"].as_array().into_iter().flatten() {
        let what = format!("prophecy {}", entry["ot"].as_str().unwrap_or("?"));
        let Some(ot) = check.span(entry["ot"].as_str(), &what) else {
            continue;
        };
        let title = entry["title"].as_str().unwrap_or("").to_string();
        if title.is_empty() {
            check.faults.push(format!("{what}: no title"));
        }
        let theme = check.one_of(entry["theme"].as_str(), THEMES, &what);
        let mut stages = Vec::new();
        for stage in entry["stages"].as_array().into_iter().flatten() {
            stages.extend(check.one_of(Some(stage.as_str().unwrap_or("")), STAGES, &what));
        }
        let Some(messianic) = entry["messianic"].as_bool() else {
            check.faults.push(format!("{what}: no messianic flag"));
            continue;
        };
        let mut fulfilments = Vec::new();
        for f in entry["fulfilments"].as_array().into_iter().flatten() {
            let at = format!("{what}, fulfilment {}", f["ref"].as_str().unwrap_or("?"));
            let Some(passage) = check.span(f["ref"].as_str(), &at) else {
                continue;
            };
            let taken = match f["ot"].as_str() {
                None => Some(ot),
                Some(text) => check.span(Some(text), &at),
            };
            let Some(taken) = taken else { continue };
            if !contains(ot, taken) {
                check.faults.push(format!("{at}: its OT verses are outside the prophecy"));
            }
            let Some(kind) = check.one_of(Some(f["kind"].as_str().unwrap_or("")), KINDS, &at) else {
                continue;
            };
            let stage = match f["stage"].as_str() {
                None => default_stage(passage).to_string(),
                Some(stage) => match check.one_of(Some(stage), STAGES, &at) {
                    Some(stage) => stage,
                    None => continue,
                },
            };
            fulfilments.push(Fulfilment {
                span: passage,
                ot: taken,
                kind,
                stage,
                formula: None,
                reviewed: true,
                confidence: None,
                note: f["note"].as_str().unwrap_or("").to_string(),
            });
        }
        prophecies.push(Prophecy {
            span: ot,
            title,
            theme,
            messianic,
            stages,
            curated: true,
            note: entry["note"].as_str().unwrap_or("").to_string(),
            fulfilments,
        });
    }

    let mut patterns = Vec::new();
    for entry in json["patterns"].as_array().into_iter().flatten() {
        let title = entry["title"].as_str().unwrap_or("").to_string();
        let what = format!("pattern {title:?}");
        if title.is_empty() {
            check.faults.push(format!("{what}: no title"));
        }
        let mut members = Vec::new();
        for m in entry["members"].as_array().into_iter().flatten() {
            let at = format!("{what}, member {}", m["ot"].as_str().unwrap_or("?"));
            let Some(ot) = check.span(m["ot"].as_str(), &at) else {
                continue;
            };
            if !prophecies.iter().any(|p| p.span == ot) {
                check.faults.push(format!("{at}: no prophecy has that passage"));
                continue;
            }
            let Some(stage) = check.one_of(Some(m["stage"].as_str().unwrap_or("")), STAGES, &at)
            else {
                continue;
            };
            let label = m["label"].as_str().unwrap_or("").to_string();
            if label.is_empty() {
                check.faults.push(format!("{at}: no label"));
            }
            members.push((ot, stage, label));
        }
        patterns.push(Pattern {
            title,
            note: entry["note"].as_str().unwrap_or("").to_string(),
            members,
        });
    }

    let mut rejected = HashSet::new();
    for r in json["rejected"].as_array().into_iter().flatten() {
        let nt = check.span(r["ref"].as_str(), "rejected");
        let ot = check.span(r["ot"].as_str(), "rejected");
        if let (Some(nt), Some(ot)) = (nt, ot) {
            rejected.insert((nt.0, ot.0));
        }
    }
    if !check.faults.is_empty() {
        bail!(
            "{} has {} faults:\n  {}",
            path.display(),
            check.faults.len(),
            check.faults.join("\n  ")
        );
    }
    Ok(Curation {
        themes,
        prophecies,
        patterns,
        rejected,
    })
}

/// What a citation formula says of the passage it introduces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Formula {
    /// "That it might be fulfilled", "then was fulfilled".
    Fulfilled,
    /// "That which was spoken by the prophet".
    Spoken,
    /// "It is written".
    Written,
    /// "The scripture".
    Scripture,
}

impl Formula {
    fn as_str(self) -> &'static str {
        match self {
            Formula::Fulfilled => "fulfilled",
            Formula::Spoken => "spoken",
            Formula::Written => "written",
            Formula::Scripture => "scripture",
        }
    }

    /// Whether the formula itself calls the passage fulfilled, so that a found
    /// link may start a prophecy of its own.
    fn fulfils(self) -> bool {
        self == Formula::Fulfilled
    }
}

/// The tense, voice and mood of a TAGNT verb code (`V-APS-3S`: aorist
/// passive subjunctive).
fn verb_parse(code: &str) -> Option<(char, char, char)> {
    let mut chars = code.strip_prefix("V-")?.chars().skip_while(|c| *c == '2');
    Some((chars.next()?, chars.next()?, chars.next()?))
}

/// The speakers a formula may name, by the gloss of their lemma, with the
/// books their words are found in (Haqor's numbering).
const SPEAKERS: &[(&str, &[i64])] = &[
    ("Isaiah", &[12]),
    ("Jeremiah", &[13]),
    ("Hosea", &[15]),
    ("Joel", &[16]),
    ("Daniel", &[35]),
    ("David", &[27, 8, 9]),
    ("Moses", &[1, 2, 3, 4, 5]),
];

/// An NT verse with a citation formula, and the books a named speaker narrows
/// its source to.
#[derive(Debug, Default)]
struct FormulaVerse {
    verse: i64,
    formulas: Vec<Formula>,
    books: Option<&'static [i64]>,
    /// Names a prophet ("by the prophet", "the prophets").
    prophet: bool,
}

fn formula_verses(db: &Connection) -> Result<Vec<FormulaVerse>> {
    let mut stmt = db.prepare(
        "SELECT w.ref, l.lemma, l.gloss, g.code FROM greek_word w \
         JOIN greek_lemma l USING(lemma_id) JOIN greek_grammar g USING(grammar_id) \
         ORDER BY w.ref, w.position",
    )?;
    let mut verses: BTreeMap<i64, FormulaVerse> = BTreeMap::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let verse: i64 = row.get(0)?;
        let lemma: String = row.get(1)?;
        let gloss: String = row.get(2)?;
        let code: String = row.get(3)?;
        let parse = verb_parse(&code);
        let passive = parse.is_some_and(|(_, voice, _)| voice == 'P');
        let formula = match lemma.as_str() {
            "πληρόω" | "ἀναπληρόω" | "τελέω" | "τελειόω" if passive => Some(Formula::Fulfilled),
            "γράφω" if parse.is_some_and(|(tense, voice, _)| (tense, voice) == ('R', 'P')) => {
                Some(Formula::Written)
            }
            "γραφή" => Some(Formula::Scripture),
            // "That which was spoken" (τὸ ῥηθέν, τὸ εἰρημένον), not "it was
            // said" (ἐρρέθη), which also introduces the Law's sayings.
            l if l.contains("ἐρῶ") && passive && parse.is_some_and(|(_, _, mood)| mood == 'P') => {
                Some(Formula::Spoken)
            }
            _ => None,
        };
        let speaker = SPEAKERS.iter().find(|(name, _)| gloss == *name).map(|(_, b)| *b);
        let prophet = lemma == "προφήτης";
        if formula.is_none() && speaker.is_none() && !prophet {
            continue;
        }
        let entry = verses.entry(verse).or_insert(FormulaVerse {
            verse,
            ..FormulaVerse::default()
        });
        if let Some(f) = formula
            && !entry.formulas.contains(&f)
        {
            entry.formulas.push(f);
        }
        if speaker.is_some() {
            entry.books = speaker;
        }
        entry.prophet |= prophet;
    }
    Ok(verses
        .into_values()
        .filter_map(|mut v| {
            // Something fulfilled is a citation only with what was: the
            // scripture, a prophet's word, what was spoken or written — not a
            // time, joy, or the plagues.
            let cited = v.prophet || v.books.is_some() || v.formulas.iter().any(|f| !f.fulfils());
            if !cited {
                v.formulas.retain(|f| !f.fulfils());
            }
            // A speaker or prophet named without a formula ("David saith")
            // narrows nothing on its own: most mentions quote no one.
            if v.formulas.is_empty() {
                return None;
            }
            v.formulas.sort();
            Some(v)
        })
        .collect())
}

/// A link found from a formula verse.
#[derive(Debug, Clone)]
struct Found {
    /// The formula verse through the verse the quoted words were found in.
    nt: Span,
    ot: i64,
    formula: Formula,
    formulas: String,
    confidence: &'static str,
    /// Whether the roots of the two verses align, rather than the TSK alone
    /// linking them.
    aligned: bool,
}

/// Root-alignment scores below this do not count as evidence on their own:
/// the formula's own words ("by the prophet, saying") align weakly with OT
/// narrative.
const MIN_ALIGNMENT: f64 = 14.0;

fn find_links(db: &Connection) -> Result<Vec<Found>> {
    let nt = pack_ref(40, 0, 0);
    let mut aligned: HashMap<i64, Vec<(i64, f64)>> = HashMap::new();
    let mut stmt = db.prepare(&format!(
        "SELECT b_ref, a_ref, score FROM quotation WHERE a_ref < {nt} AND b_ref >= {nt}"
    ))?;
    for row in stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))? {
        let (nt_ref, ot_ref, score) = row?;
        aligned.entry(nt_ref).or_default().push((ot_ref, score));
    }
    let mut thematic: HashMap<i64, Vec<i64>> = HashMap::new();
    let mut stmt = db.prepare(&format!(
        "SELECT ref, targets FROM thematic_reference WHERE ref >= {nt}"
    ))?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))? {
        let (verse, targets) = row?;
        for target in targets.split_whitespace() {
            let (first, last) = target.split_once('-').unwrap_or((target, target));
            let (Ok(first), Ok(last)) = (first.parse::<i64>(), last.parse::<i64>()) else {
                continue;
            };
            if first < nt {
                // A span's verses, within a chapter (a verse number is 1-255).
                for r in first..=last {
                    if r & 255 != 0 {
                        thematic.entry(verse).or_default().push(r);
                    }
                }
            }
        }
    }
    let order: Vec<i64> = db
        .prepare(&format!("SELECT DISTINCT ref FROM greek_word WHERE ref >= {nt} ORDER BY ref"))?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let index: HashMap<i64, usize> = order.iter().enumerate().map(|(i, r)| (*r, i)).collect();

    let mut found = Vec::new();
    for fv in formula_verses(db)? {
        let Some(&at) = index.get(&fv.verse) else { continue };
        // The quoted words usually follow the formula: it and the next two
        // verses, within the book.
        let window = order[at..(at + 3).min(order.len())]
            .iter()
            .copied()
            .filter(|r| r >> 16 == fv.verse >> 16);
        // OT verse -> (best alignment, the NT verse it was in, TSK mentions).
        let mut candidates: HashMap<i64, (f64, i64, u32)> = HashMap::new();
        for w in window {
            for &(ot, score) in aligned.get(&w).into_iter().flatten() {
                let c = candidates.entry(ot).or_insert((0.0, w, 0));
                if score > c.0 {
                    (c.0, c.1) = (score, w);
                }
            }
            for &ot in thematic.get(&w).into_iter().flatten() {
                candidates.entry(ot).or_insert((0.0, w, 0)).2 += 1;
            }
        }
        let in_book = |ot: i64| fv.books.is_none_or(|b| b.contains(&(ot >> 16)));
        let best = candidates
            .into_iter()
            .filter(|(_, (score, _, tsk))| *score >= MIN_ALIGNMENT || *tsk > 0)
            .max_by(|(a_ot, a), (b_ot, b)| {
                let rank = |ot: i64, c: &(f64, i64, u32)| {
                    (in_book(ot), c.0 + 10.0 * f64::from(c.2.min(2)))
                };
                let (ra, rb) = (rank(*a_ot, a), rank(*b_ot, b));
                ra.0.cmp(&rb.0)
                    .then(ra.1.total_cmp(&rb.1))
                    // Ties go to the earlier verse, so a build is repeatable.
                    .then(b_ot.cmp(a_ot))
            });
        let Some((ot, (score, w, tsk))) = best else { continue };
        let confidence = if tsk > 0 && score >= 12.0 {
            "high"
        } else if (tsk > 0 && score > 0.0) || score >= 20.0 || (tsk > 0 && fv.books.is_some() && in_book(ot)) {
            "medium"
        } else {
            // A guess among the TSK's many references: left for review.
            continue;
        };
        found.push(Found {
            nt: (fv.verse, w.max(fv.verse)),
            ot,
            formula: fv.formulas[0],
            formulas: fv.formulas.iter().map(|f| f.as_str()).collect::<Vec<_>>().join("+"),
            confidence,
            aligned: score >= MIN_ALIGNMENT,
        });
    }
    Ok(found)
}

/// Fold the found links into the curated prophecies, starting new ones for
/// fulfilment formulas outside them.
fn merge(curation: Curation, found: Vec<Found>, summary: &mut ProphecySummary) -> Vec<Prophecy> {
    let Curation {
        themes,
        mut prophecies,
        rejected,
        ..
    } = curation;
    summary.curated = prophecies.len();
    let theme_of = |ot: i64| {
        themes
            .iter()
            .find(|(span, _)| contains(*span, (ot, ot)))
            .map(|(_, theme)| theme.clone())
    };
    // The NT passages curated as fulfilling some prophecy.
    let curated: Vec<Span> = prophecies
        .iter()
        .flat_map(|p| p.fulfilments.iter().map(|f| f.span))
        .collect();
    let mut started: BTreeMap<i64, Prophecy> = BTreeMap::new();
    for link in found {
        if (link.nt.0..=link.nt.1).any(|nt| rejected.contains(&(nt, link.ot))) {
            continue;
        }
        // The narrowest curated prophecy holding the OT verse.
        let home = prophecies
            .iter_mut()
            .filter(|p| contains(p.span, (link.ot, link.ot)))
            .min_by_key(|p| p.span.1 - p.span.0);
        let fulfilment = Fulfilment {
            span: link.nt,
            ot: (link.ot, link.ot),
            kind: if link.aligned { "quotation" } else { "allusion" }.to_string(),
            stage: default_stage(link.nt).to_string(),
            formula: Some(link.formulas.clone()),
            reviewed: false,
            confidence: Some(link.confidence),
            note: String::new(),
        };
        match home {
            Some(prophecy) => {
                if let Some(curated) = prophecy
                    .fulfilments
                    .iter_mut()
                    .find(|f| f.reviewed && overlaps(f.span, link.nt))
                {
                    curated.formula.get_or_insert(link.formulas);
                    summary.confirmed += 1;
                } else if !curated.iter().any(|s| overlaps(*s, link.nt))
                    && !prophecy.fulfilments.iter().any(|f| overlaps(f.span, link.nt))
                {
                    // Curated as fulfilling another prophecy, the passage
                    // quotes that one: the found link is the TSK's guess.
                    prophecy.fulfilments.push(fulfilment);
                    summary.added += 1;
                }
            }
            // Outside any curated prophecy only a strong link after a fulfilment
            // formula stands on its own.
            None if link.formula.fulfils()
                && link.confidence == "high"
                && !curated.iter().any(|s| overlaps(*s, link.nt)) =>
            {
                let prophecy = started.entry(link.ot).or_insert_with(|| Prophecy {
                    span: (link.ot, link.ot),
                    title: String::new(),
                    theme: theme_of(link.ot),
                    // Found from the Gospels' and Acts' fulfilment formulas,
                    // which speak of Christ.
                    messianic: true,
                    stages: Vec::new(),
                    curated: false,
                    note: String::new(),
                    fulfilments: Vec::new(),
                });
                if !prophecy.fulfilments.iter().any(|f| overlaps(f.span, link.nt)) {
                    prophecy.fulfilments.push(fulfilment);
                }
            }
            None => {}
        }
    }
    summary.found = started.len();
    prophecies.extend(started.into_values());
    prophecies.sort_by_key(|p| (p.span.0, p.span.1));
    for p in &mut prophecies {
        p.fulfilments.sort_by_key(|f| (f.span.0, f.ot.0));
        summary.fulfilments += p.fulfilments.len();
    }
    prophecies
}

fn write(db: &Connection, prophecies: &[Prophecy], patterns: &[Pattern]) -> Result<()> {
    let tx = db.unchecked_transaction()?;
    tx.execute_batch(SCHEMA)?;
    let mut insert = tx.prepare(
        "INSERT INTO prophecy(prophecy_id, ot_ref, ot_end, title, theme, messianic, stages, \
         curated, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    let mut fulfil = tx.prepare(
        "INSERT INTO prophecy_fulfilment(prophecy_id, ord, ref, end_ref, ot_ref, ot_end, kind, \
         stage, formula, reviewed, confidence, note) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )?;
    let mut ids = HashMap::new();
    for (i, p) in prophecies.iter().enumerate() {
        let id = i as i64 + 1;
        if p.curated {
            ids.insert(p.span, id);
        }
        insert.execute(params![
            id,
            p.span.0,
            p.span.1,
            p.title,
            p.theme,
            p.messianic,
            stages_of(p),
            p.curated,
            p.note
        ])?;
        for (ord, f) in p.fulfilments.iter().enumerate() {
            fulfil.execute(params![
                id,
                ord as i64,
                f.span.0,
                f.span.1,
                f.ot.0,
                f.ot.1,
                f.kind,
                f.stage,
                f.formula,
                f.reviewed,
                f.confidence,
                f.note
            ])?;
        }
    }
    let mut pattern = tx.prepare(
        "INSERT INTO prophecy_pattern(pattern_id, title, note) VALUES (?1, ?2, ?3)",
    )?;
    let mut member = tx.prepare(
        "INSERT INTO prophecy_pattern_member(pattern_id, ord, prophecy_id, stage, label) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for (i, p) in patterns.iter().enumerate() {
        let id = i as i64 + 1;
        pattern.execute(params![id, p.title, p.note])?;
        for (ord, (span, stage, label)) in p.members.iter().enumerate() {
            // Checked when the curation was read.
            member.execute(params![id, ord as i64, ids[span], stage, label])?;
        }
    }
    drop((insert, fulfil, pattern, member));
    tx.commit()?;
    Ok(())
}

/// Build the `prophecy` and `prophecy_fulfilment` tables of the runtime
/// database `db` from `curation` (`data/prophecies.json`), with the links
/// found from its `greek_word`, `quotation` and `thematic_reference` tables,
/// so after those. Fails, naming every fault, on a curation entry that cannot
/// be read or names a passage outside the corpus.
pub fn build_prophecies(db: &Connection, curation: &Path, src_texts: &Path) -> Result<ProphecySummary> {
    let tahot = crate::stepbible::source_dir(src_texts);
    let versification = Versification::from_tahot(&tahot)
        .with_context(|| format!("reading the KJV/Hebrew verse numbering from {}", tahot.display()))?;
    let mut verses: HashSet<i64> = db
        .prepare("SELECT ref FROM verse")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    // The Greek NT has a few verses the Peshitta does not.
    verses.extend(
        db.prepare("SELECT DISTINCT ref FROM greek_word")?
            .query_map([], |row| row.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?,
    );
    let mut curation = read_curation(curation, &versification, &verses)?;
    let patterns = std::mem::take(&mut curation.patterns);
    let found = find_links(db)?;
    let mut summary = ProphecySummary::default();
    let prophecies = merge(curation, found, &mut summary);
    write(db, &prophecies, &patterns)?;
    info!(
        "Prophecies: {} curated and {} found, with {} fulfilments ({} found links confirmed \
         curated ones, {} added)",
        summary.curated, summary.found, summary.fulfilments, summary.confirmed, summary.added
    );
    Ok(summary)
}

/// `db gen-prophecy`: rebuild the prophecy tables of the runtime database at
/// `path` in place and re-stamp it, so syncing it to the app reinstalls it.
pub fn gen_prophecy(path: &Path, curation: &Path, src_texts: &Path) -> Result<ProphecySummary> {
    let db = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
    let summary = build_prophecies(&db, curation, src_texts)?;
    crate::runtime_db::restamp_built(&db)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_parse_and_renumber() {
        let mut v = Versification::default();
        // Psalm 22:18 (KJV) is 22:19 in the Hebrew text.
        v.insert((27, 22, 18), (27, 22, 19));
        assert_eq!(parse_span("Gen.3.15", &v), Some((pack_ref(1, 3, 15), pack_ref(1, 3, 15))));
        assert_eq!(
            parse_span("Isa.52.13-53.12", &v),
            Some((pack_ref(12, 52, 13), pack_ref(12, 53, 12)))
        );
        assert_eq!(parse_span("Isa.42.1-4", &v), Some((pack_ref(12, 42, 1), pack_ref(12, 42, 4))));
        assert_eq!(parse_span("Psa.22.18", &v), Some((pack_ref(27, 22, 19), pack_ref(27, 22, 19))));
        assert_eq!(parse_span("Mat.1.22-23", &v), Some((pack_ref(40, 1, 22), pack_ref(40, 1, 23))));
        assert_eq!(parse_span("Isa.42.4-1", &v), None);
        assert_eq!(parse_span("Isa.42", &v), None);
        assert_eq!(parse_span("Xyz.1.1", &v), None);
    }

    #[test]
    fn verb_codes() {
        assert_eq!(verb_parse("V-APS-3S"), Some(('A', 'P', 'S')));
        assert_eq!(verb_parse("V-2RAI-1S"), Some(('R', 'A', 'I')));
        assert_eq!(verb_parse("V-APP-NSN"), Some(('A', 'P', 'P')));
        assert_eq!(verb_parse("N-GSM"), None);
    }

    /// The curation file reads cleanly against the built database, and the
    /// found links land where the well-known fulfilment quotations are.
    #[test]
    fn complete_build() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let src_texts = root.join("src_texts");
        let database = root.join("data/haqor.db");
        if !crate::stepbible::source_dir(&src_texts).exists() || !database.exists() {
            eprintln!("skipping: TAHOT source or data/haqor.db unavailable");
            return;
        }
        // Built in memory from a copy, so the test leaves the database alone.
        let db = Connection::open_in_memory().unwrap();
        db.execute("ATTACH DATABASE ?1 AS src", [database.to_string_lossy()])
            .unwrap();
        for table in [
            "verse",
            "greek_word",
            "greek_lemma",
            "greek_grammar",
            "quotation",
            "thematic_reference",
        ] {
            db.execute_batch(&format!("CREATE TABLE {table} AS SELECT * FROM src.{table}"))
                .unwrap();
        }
        let summary = build_prophecies(&db, &root.join("data").join(PROPHECIES), &src_texts).unwrap();
        eprintln!("{summary:?}");
        assert!(summary.curated >= 100);
        assert!(summary.confirmed >= 20);

        let prophecy_of = |ot: i64| -> Option<(String, bool)> {
            db.query_row(
                "SELECT title, curated FROM prophecy WHERE ot_ref <= ?1 AND ot_end >= ?1 \
                 ORDER BY ot_end - ot_ref LIMIT 1",
                [ot],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok()
        };
        let fulfilled_at = |ot: i64, nt: i64| -> bool {
            db.query_row(
                "SELECT EXISTS(SELECT 1 FROM prophecy_fulfilment WHERE ot_ref <= ?1 AND ot_end >= ?1 \
                 AND ref <= ?2 AND end_ref >= ?2)",
                [ot, nt],
                |r| r.get(0),
            )
            .unwrap()
        };
        // Psalm 22:18 (KJV), Hebrew 22:19, parted garments: John 19:24.
        assert!(fulfilled_at(pack_ref(27, 22, 19), pack_ref(43, 19, 24)));
        // Micah 5:2 (KJV) is Hebrew 5:1.
        assert_eq!(prophecy_of(pack_ref(20, 5, 1)).unwrap().1, true);
        // Ezekiel 21:27 (KJV), "until he come whose right it is", is 21:32.
        assert!(
            prophecy_of(pack_ref(14, 21, 32))
                .unwrap()
                .0
                .contains("whose right")
        );
        // Daniel 2:44: listed though the kingdom has not yet come.
        assert!(prophecy_of(pack_ref(35, 2, 44)).is_some());
        // A prophecy's stages are its own and its fulfilments', in order:
        // Isaiah 7:14 in Ahaz's day and in Jesus.
        let stages: String = db
            .query_row(
                "SELECT stages FROM prophecy WHERE ot_ref <= ?1 AND ot_end >= ?1",
                [pack_ref(12, 7, 14)],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stages, "history first-advent");
        // Revelation's fulfilments default to the return.
        let stage: String = db
            .query_row(
                "SELECT stage FROM prophecy_fulfilment WHERE ref = ?1",
                [pack_ref(66, 19, 15)],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stage, "return");
        let members: i64 = db
            .query_row("SELECT COUNT(*) FROM prophecy_pattern_member", [], |r| r.get(0))
            .unwrap();
        assert!(members >= 4);
        // Isaiah 7:14 is found from Matthew 1:22's formula and confirms the
        // curated link.
        let formula: Option<String> = db
            .query_row(
                "SELECT formula FROM prophecy_fulfilment WHERE ref = ?1",
                [pack_ref(40, 1, 22)],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(formula.as_deref(), Some("fulfilled+spoken"));
    }
}
