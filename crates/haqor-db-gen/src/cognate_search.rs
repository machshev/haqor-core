//! Candidate cognates for a Hebrew root from the vendored dictionaries of the
//! other Semitic languages (ADR 7), for `db lexicon-brief`.
//!
//! Two kinds of evidence are gathered, kept apart because they differ in
//! weight:
//!
//! - **Stated links.** Wiktionary entries that name one of the root's words as
//!   a cognate, a borrowing or an inheritance, and Proto-Semitic
//!   reconstructions descending into it. Someone has asserted the connection;
//!   the entry still cites the cognate's own dictionary, not Wiktionary.
//! - **Sound-correspondence matches.** Dictionary roots spelled as the Hebrew
//!   root's Proto-Semitic values would be ([`crate::cognates`]): Lane's and
//!   Wiktionary's Arabic roots, and Oracc's Akkadian lemmas by their
//!   consonants. These are only candidates; most are judged by their glosses.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::io::BufRead;
use std::path::Path;

use anyhow::{Context, Result};
use haqor_core::data_support::bare_letters;
use serde_json::Value;

use crate::cognates::{Lang, cognate_keys};

/// Each JSON line of a vendored file, or nothing when the file is absent (a
/// checkout without that source).
fn lines(path: &Path) -> Result<Vec<Value>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    std::io::BufReader::new(file)
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect()
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or_default()
}

fn strings(v: &Value, key: &str) -> Vec<String> {
    v[key]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The first `n` characters of `s`, marked when cut.
fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n).collect::<String>())
    }
}

/// Wiktionary's language names for the codes the extract keeps.
fn lang_name(code: &str) -> &str {
    match code {
        "he" => "Hebrew",
        "akk" => "Akkadian",
        "uga" => "Ugaritic",
        "phn" => "Phoenician",
        "xpu" => "Punic",
        "arc" => "Aramaic",
        "oar" => "Old Aramaic",
        "syc" => "Classical Syriac",
        "jpa" => "Jewish Palestinian Aramaic",
        "tmr" => "Jewish Babylonian Aramaic",
        "sam" => "Samaritan Aramaic",
        "smp" => "Samaritan Hebrew",
        "myz" => "Classical Mandaic",
        "ar" => "Arabic",
        "gez" => "Ge'ez",
        "xsa" => "Sabaean",
        "am" => "Amharic",
        "tig" => "Tigre",
        "ti" => "Tigrinya",
        "egy" => "Egyptian",
        "cop" => "Coptic",
        "sux" => "Sumerian",
        "grc" => "Ancient Greek",
        "sem-pro" => "Proto-Semitic",
        "sem-wes-pro" => "Proto-West Semitic",
        "afa-pro" => "Proto-Afroasiatic",
        other => other,
    }
}

/// A Wiktionary entry's one-line summary.
fn wiktionary_line(v: &Value) -> String {
    let roman = text(v, "roman");
    let roman = if roman.is_empty() {
        String::new()
    } else {
        format!(" ({roman})")
    };
    format!(
        "{} {}{roman}, {}: {} [Wiktionary: {}]",
        lang_name(text(v, "lang")),
        text(v, "word"),
        text(v, "pos"),
        clip(&strings(v, "glosses").join(" | "), 240),
        text(v, "word"),
    )
}

/// An Akkadian citation form's consonants as the correspondences spell them:
/// vowels and the glottal stop gone, doubled letters single.
fn akkadian_skeleton(cf: &str) -> String {
    let mut out: Vec<char> = Vec::new();
    for c in cf.chars() {
        if "aeiuāēīūâêîûʾ'-".contains(c) {
            continue;
        }
        if out.last() != Some(&c) {
            out.push(c);
        }
    }
    out.into_iter().collect()
}

/// The keys an Akkadian lemma may match a Hebrew root by: its spelling, with
/// doubled letters single, and without its weak w and y.
fn akkadian_keys(root: &str) -> HashSet<String> {
    let mut keys = HashSet::new();
    for key in cognate_keys(root, Lang::Akkadian) {
        let single = akkadian_skeleton(&key);
        let weak: String = single.chars().filter(|c| *c != 'w' && *c != 'y').collect();
        keys.insert(single);
        if !weak.is_empty() {
            keys.insert(weak);
        }
    }
    keys
}

/// Markdown for the brief's cognates section: the root `root` (a root key),
/// whose words are `lemmas`, against the vendored sources under `src_texts`.
pub fn cognate_section(src_texts: &Path, root: &str, lemmas: &[String]) -> Result<String> {
    let mut out = String::new();
    let wiktionary = lines(&src_texts.join("Wiktionary/semitic.jsonl"))?;
    let lane = lines(&src_texts.join("Lane/lane.jsonl"))?;
    let oracc = lines(&src_texts.join("Oracc/glossary.jsonl"))?;

    // The root's words by their consonants, which is how other languages'
    // entries name them (pointed or not).
    let root_letters = bare_letters(root);
    let mut words: HashSet<String> = lemmas
        .iter()
        .map(|l| bare_letters(l))
        .filter(|w| !w.is_empty())
        .collect();
    // Wiktionary's own Hebrew entries for the root add its words too.
    let hebrew: Vec<&Value> = wiktionary
        .iter()
        .filter(|v| text(v, "lang") == "he")
        .filter(|v| {
            strings(v, "roots")
                .iter()
                .any(|r| bare_letters(r) == root_letters)
                || words.contains(&bare_letters(text(v, "word")))
        })
        .collect();
    for v in &hebrew {
        words.insert(bare_letters(text(v, "word")));
    }

    writeln!(
        out,
        "## Cognates\n\nCandidates only: each must be checked in its own \
         language's dictionary and cited from there (Wiktionary is a lead, not \
         a source).\n"
    )?;

    writeln!(out, "### Stated links\n")?;
    writeln!(out, "Wiktionary's Hebrew entries for the root:\n")?;
    if hebrew.is_empty() {
        writeln!(out, "- None.")?;
    }
    for v in &hebrew {
        writeln!(out, "- {}", wiktionary_line(v))?;
        let etymology = text(v, "etymology");
        if !etymology.is_empty() {
            writeln!(out, "  - Etymology: {}", clip(etymology, 600))?;
        }
    }
    writeln!(out)?;

    // Entries elsewhere that link to one of the words.
    let mut linked: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for v in wiktionary.iter().filter(|v| text(v, "lang") != "he") {
        let links = v["links"].as_array().cloned().unwrap_or_default();
        let hit = links.iter().find_map(|l| {
            let rel = l[0].as_str()?;
            (l[1].as_str()? == "he" && words.contains(&bare_letters(l[2].as_str()?)))
                .then(|| format!("{rel} {}", l[2].as_str().unwrap_or_default()))
        });
        let descends = v["descendants"].as_array().is_some_and(|d| {
            d.iter().any(|x| {
                x[0].as_str() == Some("he")
                    && x[1]
                        .as_str()
                        .is_some_and(|w| words.contains(&bare_letters(w)))
            })
        });
        if let Some(how) = hit {
            linked
                .entry(lang_name(text(v, "lang")).to_string())
                .or_default()
                .push(format!("{} — links `{how}`", wiktionary_line(v)));
        } else if descends {
            let mut set: Vec<String> = v["descendants"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|x| {
                    let roman = x[2].as_str().map(|r| format!(" ({r})")).unwrap_or_default();
                    format!(
                        "{} {}{roman}",
                        lang_name(x[0].as_str().unwrap_or_default()),
                        x[1].as_str().unwrap_or_default()
                    )
                })
                .collect();
            set.dedup();
            linked
                .entry(lang_name(text(v, "lang")).to_string())
                .or_default()
                .push(format!(
                    "{} — descendants: {}",
                    wiktionary_line(v),
                    clip(&set.join("; "), 900)
                ));
        }
    }
    writeln!(
        out,
        "Other languages' entries linking to the root's words:\n"
    )?;
    if linked.is_empty() {
        writeln!(out, "- None.")?;
    }
    for (lang, entries) in &linked {
        for e in entries {
            writeln!(out, "- {lang}: {e}")?;
        }
    }
    writeln!(out)?;

    writeln!(out, "### By sound correspondence\n")?;
    let arabic: Vec<String> = cognate_keys(root, Lang::Arabic);
    writeln!(out, "Arabic roots tried: {}\n", arabic.join(", "))?;
    let arabic_set: HashSet<&str> = arabic.iter().map(String::as_str).collect();

    writeln!(out, "Lane:\n")?;
    let mut by_root: BTreeMap<(usize, String), Vec<&Value>> = BTreeMap::new();
    for v in &lane {
        let key = text(v, "root_key");
        if let Some(rank) = arabic.iter().position(|k| k == key) {
            by_root.entry((rank, key.to_string())).or_default().push(v);
        }
    }
    if by_root.is_empty() {
        writeln!(out, "- None.")?;
    }
    for ((_, key), entries) in &by_root {
        let pages: Vec<String> = entries
            .iter()
            .map(|e| e["page"].to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        writeln!(
            out,
            "- {key} ({} entries, pp. {}):",
            entries.len(),
            pages.join(", ")
        )?;
        for e in entries.iter().take(4) {
            writeln!(
                out,
                "  - {} (p. {}): {}",
                text(e, "headword"),
                e["page"],
                clip(text(e, "text"), 400)
            )?;
        }
    }
    writeln!(out)?;

    writeln!(out, "Wiktionary Arabic:\n")?;
    let mut any = false;
    for v in wiktionary.iter().filter(|v| text(v, "lang") == "ar") {
        let roots = strings(v, "roots");
        let matched = roots.iter().any(|r| {
            let key = crate::lane::arabic_root_key(&r.replace([' ', '\u{200c}'], ""));
            arabic_set.contains(key.as_str())
        });
        if matched {
            any = true;
            writeln!(out, "- {}", wiktionary_line(v))?;
        }
    }
    if !any {
        writeln!(out, "- None.")?;
    }
    writeln!(out)?;

    let akkadian = akkadian_keys(root);
    let mut keys: Vec<&String> = akkadian.iter().collect();
    keys.sort();
    writeln!(
        out,
        "Oracc Akkadian (consonants tried: {}):\n",
        keys.iter()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )?;
    let mut any = false;
    for v in oracc.iter().filter(|v| text(v, "lang") == "akk") {
        let skeleton = akkadian_skeleton(text(v, "cf"));
        let feminine = skeleton.strip_suffix('t').map(str::to_string);
        if akkadian.contains(&skeleton) || feminine.is_some_and(|f| akkadian.contains(&f)) {
            any = true;
            let entries = v["entries"].as_array().cloned().unwrap_or_default();
            let cite = entries
                .first()
                .map(|e| {
                    format!(
                        "{}/{}",
                        e[0].as_str().unwrap_or(""),
                        e[1].as_str().unwrap_or("")
                    )
                })
                .unwrap_or_default();
            writeln!(
                out,
                "- {} [{}] {}: {} (×{}; {}) [Oracc {cite}]",
                text(v, "cf"),
                text(v, "gw"),
                text(v, "pos"),
                clip(&strings(v, "meanings").join("; "), 200),
                v["icount"],
                strings(v, "periods").join(", "),
            )?;
        }
    }
    if !any {
        writeln!(out, "- None.")?;
    }
    writeln!(out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn akkadian_skeletons_drop_vowels_and_doubling() {
        assert_eq!(akkadian_skeleton("šemû"), "šm");
        assert_eq!(akkadian_skeleton("erṣetu"), "rṣt");
        assert_eq!(akkadian_skeleton("kakkabu"), "kb");
    }

    #[test]
    fn hebrew_roots_reach_their_akkadian_spellings() {
        assert!(akkadian_keys("שמע").contains("šm"));
        assert!(akkadian_keys("ארצ").contains("rṣ"));
        // A hollow root without its weak letter: בוא, Akkadian bâʾu.
        assert!(akkadian_keys("בוא").contains("b"));
    }
}
