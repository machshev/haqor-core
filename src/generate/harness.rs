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
use super::lexicon_db::load_root_inventory;
use super::prefilter::Prefilter;
use crate::morphology::{
    Binyan, Form, ReverseIndex, parse_word_disambiguated, parse_word_indexed,
};

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
    /// Gold says this is a verb, but the lexical pre-filter would exclude it
    /// from parsing (a false exclusion — the cost of the pre-filter).
    prefilter_excluded: usize,
    /// False exclusions attributed to the function-word list.
    prefilter_excluded_function: usize,
    /// False exclusions attributed to the proper-noun list.
    prefilter_excluded_proper: usize,
    /// Verb-aware (refined) false exclusions: function words plus proper nouns
    /// with no plausible verb reading.
    prefilter_refined_excluded: usize,
    /// Refined false exclusions still attributed to the proper-noun list.
    prefilter_refined_excluded_proper: usize,
    /// Proper-noun matches rescued by the verb-aware rule (had a plausible verb
    /// reading, so no longer excluded) that are gold verbs — recall clawed back.
    prefilter_rescued_proper: usize,
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
        self.prefilter_excluded += o.prefilter_excluded;
        self.prefilter_excluded_function += o.prefilter_excluded_function;
        self.prefilter_excluded_proper += o.prefilter_excluded_proper;
        self.prefilter_refined_excluded += o.prefilter_refined_excluded;
        self.prefilter_refined_excluded_proper += o.prefilter_refined_excluded_proper;
        self.prefilter_rescued_proper += o.prefilter_rescued_proper;
        self
    }
}

/// Score the parser against OSHB gold tags and print a report.
///
/// `morphhb_dir` is the cloned morphhb repo (expects a `wlc/` subdir).
/// `bible_db`, if given, enables the alignment metric. `lexicon_db`, if given,
/// loads the real-root inventory and restricts the parser's candidate roots to
/// it (so the report measures the filtered parser). `limit` caps the number of
/// gold verb tokens scored (0 = all) for quick sampling.
pub fn parse_eval(
    morphhb_dir: &Path,
    bible_db: Option<&Path>,
    lexicon_db: Option<&Path>,
    soft: bool,
    limit: usize,
) -> Result<()> {
    let gold = collect_gold(morphhb_dir)?;
    let gold = match limit {
        0 => &gold[..],
        n => &gold[..n.min(gold.len())],
    };

    let surfaces = match bible_db {
        Some(p) => Some(bible_surfaces(p)?),
        None => None,
    };

    let roots = match lexicon_db {
        Some(p) => Some(load_root_inventory(p)?),
        None => None,
    };
    let prefilter = match lexicon_db {
        Some(p) => Some(Prefilter::load(p)?),
        None => None,
    };
    if let Some(set) = roots.as_ref() {
        let mode = if soft {
            "disambiguate-only (keep lone/all-out-of-inventory parses)"
        } else {
            "hard prune"
        };
        println!("(root filter on: {} roots, {mode})\n", set.len());
    }

    // Build the reverse index once so each gold token is a hash lookup rather
    // than per-token generate-and-test (the same speedup the DB build uses).
    let index = ReverseIndex::build();

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
            let matches = if soft {
                // Soft (disambiguate-only) keeps the per-surface path: parse
                // unrestricted, then drop out-of-inventory roots only when that
                // leaves a parse. Rarely used; not on the hot default path.
                parse_word_disambiguated(&g.surface, roots.as_ref())
            } else {
                parse_word_indexed(&g.surface, &index, roots.as_ref())
            };
            // A plausible verb reading: at least one fully-modelled (attested)
            // candidate — the same signal the generate-and-test parser trusts.
            let has_plausible = matches.iter().any(|m| m.attested);

            if let Some(pf) = prefilter.as_ref() {
                // Lexical (pure) classification — the cost of the un-refined filter.
                if let Some(class) = pf.classify(&g.surface) {
                    s.prefilter_excluded = 1;
                    match class {
                        "function" => s.prefilter_excluded_function = 1,
                        "proper" => s.prefilter_excluded_proper = 1,
                        _ => {}
                    }
                }
                // Verb-aware (refined) classification — proper nouns with a
                // plausible verb reading yield to the parser.
                match pf.exclude(&g.surface, has_plausible) {
                    Some(class) => {
                        s.prefilter_refined_excluded = 1;
                        if class == "proper" {
                            s.prefilter_refined_excluded_proper = 1;
                        }
                    }
                    None if pf.classify(&g.surface) == Some("proper") => {
                        // Was a proper-noun match, now rescued by the verb rule.
                        s.prefilter_rescued_proper = 1;
                    }
                    None => {}
                }
            }
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

    print_report(&score, surfaces.is_some(), prefilter.is_some());
    Ok(())
}

/// Score the **already-built `hebrew.db`** against OSHB gold, instead of
/// re-running the parser. `hebrew.db.analyses` *is* the committed parser output,
/// so this is a pure DB join — instant, and it measures exactly what shipped
/// (the unfiltered analyses). Each gold verb token is matched to its surface's
/// stored candidate analyses by cantillation-normalised text; a gold surface
/// absent from the DB (the ~1% OSHB/UXLC misalignment) counts as unaligned.
pub fn eval_from_db(morphhb_dir: &Path, hebrew_db: &Path, limit: usize) -> Result<()> {
    let gold = collect_gold(morphhb_dir)?;
    let gold = match limit {
        0 => &gold[..],
        n => &gold[..n.min(gold.len())],
    };

    // surface text → its stored verb analyses (binyan name, form name, pgn label).
    let db = Connection::open(hebrew_db)
        .with_context(|| format!("opening {}", hebrew_db.display()))?;
    let mut stmt = db.prepare(
        "SELECT s.text, a.binyan, a.form, a.pgn \
         FROM analyses a JOIN surface s ON s.surface_id = a.surface_id",
    )?;
    let mut by_surface: std::collections::HashMap<String, Vec<(String, String, String)>> =
        std::collections::HashMap::new();
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (text, b, f, p) = row?;
        by_surface.entry(text).or_default().push((b, f, p));
    }
    // Every surface present in the DB (whether or not it got a verb analysis),
    // for the alignment metric — distinct from "parsed".
    let mut all_surfaces = std::collections::HashSet::new();
    {
        let mut s = db.prepare("SELECT text FROM surface")?;
        let rows = s.query_map([], |r| r.get::<_, String>(0))?;
        for t in rows {
            all_surfaces.insert(t?);
        }
    }

    let score = gold
        .par_iter()
        .map(|g| {
            let mut s = Score {
                tokens: 1,
                ..Default::default()
            };
            if all_surfaces.contains(&g.surface) {
                s.aligned = 1;
            }
            let Some(cands) = by_surface.get(&g.surface).filter(|c| !c.is_empty()) else {
                return s;
            };
            s.parsed = 1;
            let (gb, gf) = (g.binyan.name(), g.form.name());
            let full = cands
                .iter()
                .any(|(b, f, p)| b == gb && f == gf && *p == g.pgn);
            let bf = cands.iter().any(|(b, f, _)| b == gb && f == gf);
            if full {
                s.recall_full = 1;
                if cands.len() == 1 {
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

    println!("Scoring hebrew.db analyses vs OSHB gold (no reparse)\n");
    print_report(&score, true, false);
    Ok(())
}

fn pct(n: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        100.0 * n as f64 / total as f64
    }
}

fn print_report(s: &Score, aligned: bool, prefilter: bool) {
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
    if prefilter {
        println!();
        println!(
            "  pre-filter false exclusions: {:>7}  ({:.2}% of gold verbs the lexical \
             pre-filter would wrongly skip)",
            s.prefilter_excluded,
            pct(s.prefilter_excluded, t)
        );
        println!(
            "    via function list: {:>7}   via proper-noun list: {:>7}",
            s.prefilter_excluded_function, s.prefilter_excluded_proper
        );
        println!();
        println!(
            "  verb-aware (proper-noun yields to plausible verb reading):"
        );
        println!(
            "    refined false exclusions: {:>7}  ({:.2}% of gold verbs)",
            s.prefilter_refined_excluded,
            pct(s.prefilter_refined_excluded, t)
        );
        println!(
            "    via function list: {:>7}   via proper-noun list: {:>7}",
            s.prefilter_excluded_function, s.prefilter_refined_excluded_proper
        );
        println!(
            "    proper-noun matches rescued: {:>7}  (recall clawed back)",
            s.prefilter_rescued_proper
        );
    }
}
