//! Accuracy harness: score the reverse-parser against OSHB gold tags.
//!
//! [`crate::morphology::parse_word`] generates candidate verb analyses by
//! generate-and-test. This module measures how good those analyses are by
//! checking them against an independent gold standard — the OpenScriptures
//! Hebrew Bible (OSHB / morphhb), a CC BY 4.0 tagging of the Westminster
//! Leningrad Codex where every word carries a lemma (Strong's number) and an
//! OSHM morphology code.
//!
//! Crucially we do **not** copy the gold tags: we run our own parser on OSHB's
//! surface text, derive a (binyan, form, pgn) analysis ourselves, and only then
//! compare to the gold code. So this reports the accuracy of the *generator*,
//! using the lexicon purely as a scorer — which is exactly the disambiguation
//! signal a lexeme inventory would later provide.
//!
//! OSHM verb codes look like `HVqp3ms`: a language letter (`H` Hebrew / `A`
//! Aramaic), then `/`-separated morphemes. The verb morpheme is `V` + a stem
//! letter (`q`=qal, `N`=niphal, `p`=piel, `P`=pual, `h`=hiphil, `H`=hophal,
//! `t`=hithpael) + a conjugation letter (`p`=perfect, `q`=sequential perfect,
//! `i`=imperfect, `w`=wayyiqtol, `h`=cohortative, `j`=jussive, `v`=imperative,
//! `r`/`s`=participle act./pass., `a`/`c`=infinitive abs./constr.) + PGN.
//!
//! Only Hebrew (`H`) verbs are scored; Aramaic and non-verb tokens are out of
//! the parser's scope and are excluded from the denominator (and reported
//! separately).

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};
use quick_xml::Reader;
use quick_xml::events::Event;
use rayon::prelude::*;
use rusqlite::Connection;

use super::hebrew_db::normalize_surface;
use crate::morphology::{Binyan, Form, parse_word};

/// OT books are 1..=39 in Haqor numbering; NT starts at 40.
const NT_FIRST_BOOK: u8 = 40;

/// One OSHB gold verb token: the surface (cantillation-normalised) plus the
/// analysis OSHB assigns it.
struct Gold {
    surface: String,
    binyan: Binyan,
    form: Form,
    /// PGN rendered the same way [`crate::morphology::Pgn::label`] renders ours
    /// (e.g. `3ms`, `ms` for participles, empty for infinitives), so the two
    /// can be string-compared.
    pgn: String,
}

/// Map an OSHM stem letter to a binyan. Returns `None` for the rarer stems the
/// generator does not model (polel, pilpel, …); those tokens are excluded.
fn map_stem(c: char) -> Option<Binyan> {
    Some(match c {
        'q' => Binyan::Qal,
        'N' => Binyan::Niphal,
        'p' => Binyan::Piel,
        'P' => Binyan::Pual,
        'h' => Binyan::Hiphil,
        'H' => Binyan::Hophal,
        't' => Binyan::Hithpael,
        _ => return None,
    })
}

/// Map an OSHM conjugation letter to a [`Form`]. Sequential perfect (`q`,
/// weqatal) folds to [`Form::Perfect`]: our parser surfaces it as a perfect
/// with a vav proclitic, not a distinct form.
fn map_conj(c: char) -> Option<Form> {
    Some(match c {
        'p' | 'q' => Form::Perfect,
        'i' => Form::Imperfect,
        'w' => Form::Wayyiqtol,
        'h' => Form::Cohortative,
        'j' => Form::Jussive,
        'v' => Form::Imperative,
        'r' => Form::ParticipleActive,
        's' => Form::ParticiplePassive,
        'a' => Form::InfinitiveAbsolute,
        'c' => Form::InfinitiveConstruct,
        _ => return None,
    })
}

/// Parse the gold analysis out of an OSHM morph code, given the surface text.
/// Returns `None` for anything outside the parser's scope (Aramaic, non-verbs,
/// unmodelled stems).
fn parse_gold(morph: &str, surface: String) -> Option<Gold> {
    let body = morph.strip_prefix('H')?; // Hebrew only; Aramaic (`A`) excluded.
    for seg in body.split('/') {
        let mut chars = seg.chars();
        if chars.next() != Some('V') {
            continue;
        }
        let stem = chars.next()?;
        let conj = chars.next()?;
        let rest: Vec<char> = chars.collect();
        let binyan = map_stem(stem)?;
        let form = map_conj(conj)?;
        let pgn: String = match conj {
            // Participles: gender + number + state — drop the state letter so
            // the label matches our person-less participle PGN.
            'r' | 's' => rest.iter().take(2).collect(),
            // Infinitives carry no PGN.
            'a' | 'c' => String::new(),
            // Finite forms: person + gender + number, e.g. `3ms`.
            _ => rest.iter().collect(),
        };
        return Some(Gold {
            surface,
            binyan,
            form,
            pgn,
        });
    }
    None
}

/// Collect every Hebrew gold verb token from the morphhb WLC OSIS files.
fn collect_gold(morphhb_dir: &Path) -> Result<Vec<Gold>> {
    let wlc = morphhb_dir.join("wlc");
    let mut out = Vec::new();
    let mut paths: Vec<_> = std::fs::read_dir(&wlc)
        .with_context(|| format!("reading {}", wlc.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "xml"))
        .collect();
    paths.sort();

    for path in paths {
        collect_book(&path, &mut out)?;
    }
    Ok(out)
}

fn collect_book(path: &Path, out: &mut Vec<Gold>) -> Result<()> {
    let mut reader = Reader::from_file(path)?;
    let mut buf = Vec::new();
    let mut in_word = false;
    let mut morph = String::new();
    let mut text = String::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if e.name().as_ref() == b"w" => {
                in_word = true;
                text.clear();
                morph.clear();
                if let Some(a) = e.try_get_attribute("morph")? {
                    morph = a.decode_and_unescape_value(reader.decoder())?.into_owned();
                }
            }
            Event::Text(t) if in_word => {
                let frag = t.unescape()?;
                // Keep Hebrew fragments, drop any nested note text (mirrors the
                // UXLC parser's `> "z"` filter).
                if frag.as_ref() > "z" {
                    text.push_str(frag.as_ref());
                }
            }
            Event::End(e) if e.name().as_ref() == b"w" => {
                in_word = false;
                if !morph.is_empty() {
                    let surface = normalize_surface(&text);
                    if !surface.is_empty()
                        && let Some(g) = parse_gold(&morph, surface)
                    {
                        out.push(g);
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

/// Load the set of distinct cantillation-normalised OT surfaces present in
/// `bible.db`, for the alignment check (how transferable a morphhb-derived
/// result is to our own text).
fn bible_surfaces(bible_db: &Path) -> Result<HashSet<String>> {
    let db =
        Connection::open(bible_db).with_context(|| format!("opening {}", bible_db.display()))?;
    let mut stmt = db.prepare(&format!(
        "SELECT words FROM bible WHERE book < {NT_FIRST_BOOK}"
    ))?;
    let mut set = HashSet::new();
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    for row in rows {
        for tok in row?.split(|c: char| c.is_whitespace() || c == '\u{05BE}') {
            let norm = normalize_surface(tok);
            if !norm.is_empty() {
                set.insert(norm);
            }
        }
    }
    Ok(set)
}

#[derive(Default)]
struct Score {
    tokens: usize,
    parsed: usize,
    /// A candidate matched the gold (binyan, form, pgn).
    recall_full: usize,
    /// A candidate matched the gold (binyan, form), ignoring PGN.
    recall_binyan_form: usize,
    /// Exactly one candidate, and it matched the gold fully.
    unambiguous_correct: usize,
    /// Gold present among candidates, but >1 candidate (a lexicon could pick).
    correct_but_ambiguous: usize,
    /// In bible.db's surface set (transferable to our own text).
    aligned: usize,
}

impl Score {
    fn merge(mut self, o: Score) -> Score {
        self.tokens += o.tokens;
        self.parsed += o.parsed;
        self.recall_full += o.recall_full;
        self.recall_binyan_form += o.recall_binyan_form;
        self.unambiguous_correct += o.unambiguous_correct;
        self.correct_but_ambiguous += o.correct_but_ambiguous;
        self.aligned += o.aligned;
        self
    }
}

/// Score the parser against OSHB gold tags and print a report.
///
/// `morphhb_dir` is the cloned morphhb repo (expects a `wlc/` subdir).
/// `bible_db`, if given, enables the alignment metric. `limit` caps the number
/// of gold verb tokens scored (0 = all) for quick sampling.
pub fn parse_eval(morphhb_dir: &Path, bible_db: Option<&Path>, limit: usize) -> Result<()> {
    let gold = collect_gold(morphhb_dir)?;
    let gold = match limit {
        0 => &gold[..],
        n => &gold[..n.min(gold.len())],
    };

    let surfaces = match bible_db {
        Some(p) => Some(bible_surfaces(p)?),
        None => None,
    };

    let score = gold
        .par_iter()
        .map(|g| {
            let mut s = Score {
                tokens: 1,
                ..Default::default()
            };
            if surfaces
                .as_ref()
                .is_some_and(|set| set.contains(&g.surface))
            {
                s.aligned = 1;
            }
            let matches = parse_word(&g.surface);
            if matches.is_empty() {
                return s;
            }
            s.parsed = 1;

            let full = matches
                .iter()
                .any(|m| m.binyan == g.binyan && m.form == g.form && m.pgn.label() == g.pgn);
            let bf = matches
                .iter()
                .any(|m| m.binyan == g.binyan && m.form == g.form);
            if full {
                s.recall_full = 1;
                if matches.len() == 1 {
                    s.unambiguous_correct = 1;
                } else {
                    s.correct_but_ambiguous = 1;
                }
            }
            if bf {
                s.recall_binyan_form = 1;
            }
            s
        })
        .reduce(Score::default, Score::merge);

    print_report(&score, surfaces.is_some());
    Ok(())
}

fn pct(n: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        100.0 * n as f64 / total as f64
    }
}

fn print_report(s: &Score, aligned: bool) {
    let t = s.tokens;
    println!("Parser vs OSHB gold (Hebrew verb tokens)");
    println!("  gold verb tokens:        {t}");
    println!(
        "  parsed (>=1 candidate):  {:>7}  ({:.1}%)",
        s.parsed,
        pct(s.parsed, t)
    );
    println!();
    println!("  recall (gold analysis among our candidates):");
    println!(
        "    binyan+form+pgn:       {:>7}  ({:.1}% of tokens, {:.1}% of parsed)",
        s.recall_full,
        pct(s.recall_full, t),
        pct(s.recall_full, s.parsed)
    );
    println!(
        "    binyan+form only:      {:>7}  ({:.1}% of tokens)",
        s.recall_binyan_form,
        pct(s.recall_binyan_form, t)
    );
    println!();
    println!("  of the fully-correct tokens:");
    println!(
        "    unambiguous (1 cand.): {:>7}  ({:.1}%)",
        s.unambiguous_correct,
        pct(s.unambiguous_correct, s.recall_full)
    );
    println!(
        "    ambiguous (lexicon could pick): {:>7}  ({:.1}%)",
        s.correct_but_ambiguous,
        pct(s.correct_but_ambiguous, s.recall_full)
    );
    if aligned {
        println!();
        println!(
            "  alignment: {:>7}  ({:.1}%) of gold surfaces are present in bible.db",
            s.aligned,
            pct(s.aligned, t)
        );
    }
}
