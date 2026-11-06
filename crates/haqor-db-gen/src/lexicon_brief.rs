//! The evidence for writing a Haqor lexicon entry (ADR 7): `db lexicon-rank`
//! lists the roots by how often they occur, and `db lexicon-brief <root>`
//! gathers what the text says about one of them, as Markdown to write an entry
//! from.
//!
//! A brief holds every word BDB files under the root, its tokens split by
//! stem, how the English renders each, examples spread across the canon, the
//! New Testament's quotations of verses using it, the Peshitta's cognate
//! root, and the other lexicons' text. Those lexicons are leads to evidence,
//! never something an entry cites.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result};
use haqor_core::bible::Bible;
use haqor_core::translation::TranslationSpan;
use rusqlite::Connection;

/// A packed reference's book, chapter and verse.
fn unpack(reference: i64) -> (u8, u8, u8) {
    (
        (reference >> 16) as u8,
        ((reference >> 8) & 255) as u8,
        (reference & 255) as u8,
    )
}

/// The first New Testament book's packed reference.
const NT_START: i64 = 40 << 16;

/// A reference as the lexicon writes it (`Gen.2.7`).
fn written(reference: i64) -> String {
    let (book, chapter, verse) = unpack(reference);
    let abbreviation = crate::journeys::BOOKS
        .iter()
        .enumerate()
        .find(|(i, _)| crate::tsk::book_of_key(i + 1) == Some(book))
        .map_or("?", |(_, b)| *b);
    format!("{abbreviation}.{chapter}.{verse}")
}

/// One token of a word.
struct Token {
    reference: i64,
    position: u16,
    surface: String,
    stem: String,
    aramaic: bool,
    /// Other BDB words the same spelling may be.
    also: Vec<String>,
    /// The root the token's own analysis gives, and whether it is a verb.
    analysed_root: String,
    verb: bool,
}

/// A BDB word under the root. BDB files many verbs as the root's own entry
/// (`kind` 'root'), so only a bare root heading, with no part of speech, is
/// left out.
struct Word {
    key: String,
    word: String,
    pos: String,
    gloss: String,
    tokens: Vec<Token>,
    /// Tokens filed under the word by spelling whose own analysis is a verb of
    /// another root (Hos 4:7 אָמִיר is מוּר 'exchange', not אָמִיר 'top').
    elsewhere: Vec<Token>,
}

/// Whether two root keys are one root, the weak ו and י aside (the
/// morphology may spell a hollow root בוא where BDB has ביא).
fn same_root(a: &str, b: &str) -> bool {
    let strong = |s: &str| -> String {
        haqor_core::data_support::bare_letters(s)
            .chars()
            .filter(|c| *c != 'ו' && *c != 'י')
            .collect()
    };
    strong(a) == strong(b)
}

fn words_of(db: &Connection, root: &str) -> Result<Vec<Word>> {
    // Roots BDB has verbs under: a token analysed as one of these, not the
    // brief's root, belongs to that verb. A root only the morphology spells
    // (הוי for היה) has none, and decides nothing.
    let verb_roots: std::collections::HashSet<String> = db
        .prepare("SELECT DISTINCT root FROM data.lexicon_entry WHERE pos LIKE 'vb%'")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let mut words: Vec<Word> = Vec::new();
    let mut stmt = db.prepare(
        "SELECT key, coalesce(word, ''), coalesce(pos, ''), coalesce(gloss, '') \
         FROM data.lexicon_entry WHERE root = ?1 AND (kind IS NOT 'root' OR coalesce(pos, '') <> '') ORDER BY key",
    )?;
    for row in stmt.query_map([root], |r| {
        Ok(Word {
            key: r.get(0)?,
            word: r.get(1)?,
            pos: r.get(2)?,
            gloss: r.get(3)?,
            tokens: Vec::new(),
            elsewhere: Vec::new(),
        })
    })? {
        words.push(row?);
    }
    let mut stmt = db.prepare(
        "SELECT w.ref, w.position, s.text, coalesce(c.form, ''), s.language IS NOT NULL, \
                (SELECT group_concat(o.key, ' ') FROM data.surface_entry o \
                 WHERE o.surface_id = w.surface_id AND o.key <> ?1), \
                coalesce(i.root, ''), c.part_of_speech = 'Verb' \
         FROM data.surface_entry se \
         JOIN data.word w ON w.surface_id = se.surface_id \
         JOIN data.surface s ON s.surface_id = w.surface_id \
         LEFT JOIN data.word_info i ON i.info_id = w.info_id \
         LEFT JOIN data.morph_cell c ON c.cell_id = i.cell_id \
         WHERE se.key = ?1 AND w.ref < ?2 ORDER BY w.ref, w.position",
    )?;
    for word in &mut words {
        for token in stmt.query_map(rusqlite::params![word.key, NT_START], |r| {
            Ok(Token {
                reference: r.get(0)?,
                position: r.get(1)?,
                surface: r.get(2)?,
                stem: r.get(3)?,
                aramaic: r.get(4)?,
                also: r
                    .get::<_, Option<String>>(5)?
                    .map(|s| s.split(' ').map(str::to_string).collect())
                    .unwrap_or_default(),
                analysed_root: r.get(6)?,
                verb: r.get::<_, Option<bool>>(7)?.unwrap_or(false),
            })
        })? {
            let token = token?;
            if token.verb
                && !same_root(&token.analysed_root, root)
                && verb_roots.contains(&token.analysed_root)
            {
                word.elsewhere.push(token);
            } else {
                word.tokens.push(token);
            }
        }
    }
    words.sort_by_key(|w| std::cmp::Reverse(w.tokens.len()));
    Ok(words)
}

/// A chapter's English, verse by verse.
type ChapterEnglish = Vec<(u8, Vec<TranslationSpan>)>;

/// The English of chapters, fetched once each.
struct English<'a> {
    bible: &'a Bible,
    chapters: HashMap<(u8, u8), ChapterEnglish>,
}

impl English<'_> {
    fn verse(&mut self, reference: i64) -> Result<&[TranslationSpan]> {
        let (book, chapter, verse) = unpack(reference);
        if !self.chapters.contains_key(&(book, chapter)) {
            let spans = self.bible.chapter_translation(book, chapter)?;
            self.chapters.insert((book, chapter), spans);
        }
        Ok(self.chapters[&(book, chapter)]
            .iter()
            .find(|(v, _)| *v == verse)
            .map_or(&[][..], |(_, spans)| spans))
    }

    /// The verse's English with the words rendering the token in bold, and
    /// those words alone.
    fn rendering(&mut self, token: &Token) -> Result<(String, String)> {
        let (_, chapter, verse) = unpack(token.reference);
        let mut line = String::new();
        let mut rendered = Vec::new();
        for span in self.verse(token.reference)? {
            let renders = span
                .words
                .iter()
                .any(|w| w.chapter == chapter && w.verse == verse && w.position == token.position);
            if renders {
                rendered.push(span.text.trim().to_string());
                let _ = write!(line, "**{}**", span.text.trim());
                if span.text.ends_with(' ') {
                    line.push(' ');
                }
            } else {
                line.push_str(&span.text);
            }
        }
        Ok((
            line.trim().replace("****", ""),
            rendered.join(" ").to_lowercase(),
        ))
    }
}

/// `n` items spread evenly through `items`, so examples cover the canon.
fn spread<T>(items: &[T], n: usize) -> Vec<&T> {
    if items.len() <= n {
        return items.iter().collect();
    }
    (0..n).map(|i| &items[i * items.len() / n]).collect()
}

/// Every string in a lexicon entry's JSON, tags stripped, in order.
fn plain_text(json: &str) -> String {
    fn walk(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::String(s) => {
                let mut text = String::new();
                let mut in_tag = false;
                for c in s.chars() {
                    match c {
                        '<' => in_tag = true,
                        '>' => in_tag = false,
                        c if !in_tag => text.push(c),
                        _ => {}
                    }
                }
                let text = text.trim();
                if !text.is_empty() {
                    out.push(text.to_string());
                }
            }
            serde_json::Value::Array(items) => items.iter().for_each(|v| walk(v, out)),
            serde_json::Value::Object(fields) => fields.values().for_each(|v| walk(v, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    match serde_json::from_str::<serde_json::Value>(json) {
        Ok(value) => walk(&value, &mut out),
        Err(_) => out.push(json.to_string()),
    }
    out.join(" ")
}

/// A pointed word without its accents, which lexicons mark differently.
fn unaccented(word: &str) -> String {
    word.chars()
        .filter(|c| !('\u{0591}'..='\u{05AF}').contains(c) && *c != '\u{05BD}')
        .collect()
}

/// The Greek lemmas the Septuagint renders each of `words` by, from MACULA's
/// alignment in `src_texts`, each with its New Testament occurrences. A
/// translation is no evidence of the Hebrew's meaning: this is the bridge
/// from the New Testament's Greek back to the Hebrew word.
fn septuagint_section(bible: &Bible, src_texts: &Path, words: &[Word]) -> Result<String> {
    use haqor_core::data_support::bare_letters;
    let mut out = String::from(
        "## Septuagint renderings (the bridge to the New Testament)\n\n\
         Not evidence of the Hebrew's meaning, which rests on scripture's use \
         and the cognates: these are the Greek words the New Testament's \
         vocabulary connects through. From MACULA Hebrew's word alignment; \
         homographs pointed alike are counted together.\n\n",
    );
    let path = src_texts.join("MACULA-LXX/lxx.tsv");
    if !path.exists() {
        out.push_str("Not vendored (run scripts/fetch-macula-lxx.sh).\n\n");
        return Ok(out);
    }
    // A lemma is matched by its pointing, so that נֶפֶשׁ and נָפַשׁ stay
    // apart; by its consonants only where those are one word's.
    let pointed: HashMap<String, &str> = words
        .iter()
        .map(|w| (unaccented(&w.word), w.word.as_str()))
        .collect();
    let mut by_letters: HashMap<String, Vec<&str>> = HashMap::new();
    for w in words {
        by_letters
            .entry(bare_letters(&w.word))
            .or_default()
            .push(w.word.as_str());
    }
    let lookup = |lemma: &str| -> Option<&str> {
        pointed.get(&unaccented(lemma)).copied().or_else(|| {
            match by_letters.get(&bare_letters(lemma)).map(Vec::as_slice) {
                Some([only]) => Some(*only),
                _ => None,
            }
        })
    };
    let mut tally: HashMap<&str, HashMap<String, usize>> = HashMap::new();
    for line in std::fs::read_to_string(&path)?.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 6 {
            continue;
        }
        if let Some(word) = lookup(fields[2]) {
            *tally
                .entry(word)
                .or_default()
                .entry(fields[5].to_string())
                .or_default() += 1;
        }
    }
    for w in words {
        let Some(greek) = tally.get(w.word.as_str()) else {
            continue;
        };
        let mut greek: Vec<_> = greek.iter().collect();
        greek.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        let listed: Vec<String> = greek
            .iter()
            .take(10)
            .map(|(lemma, n)| {
                let nt = bible
                    .greek_lemma_occurrences(lemma)
                    .map(|o| o.len())
                    .unwrap_or(0);
                format!("{lemma} ×{n} (NT ×{nt})")
            })
            .collect();
        writeln!(out, "- {}: {}", w.word, listed.join("; "))?;
    }
    writeln!(out)?;
    Ok(out)
}

/// `db lexicon-brief`: the evidence for the root `root` (a root key, as
/// `data/lexicon/README.md` describes) from the runtime database in
/// `data_dir`, with up to `examples` example verses per word and stem and
/// the New Testament quotations scoring at least `min_score` that align the
/// root's own word.
pub fn lexicon_brief(
    data_dir: &Path,
    root: &str,
    examples: usize,
    min_score: f64,
    src_texts: &Path,
) -> Result<String> {
    let bible = Bible::open(data_dir).with_context(|| format!("opening {}", data_dir.display()))?;
    let db = haqor_core::data_support::connection(&bible);
    let root = haqor_core::data_support::fold_consonants(root);
    let words = words_of(db, &root)?;
    let mut english = English {
        bible: &bible,
        chapters: HashMap::new(),
    };
    let mut out = String::new();
    let total: usize = words.iter().map(|w| w.tokens.len()).sum();
    writeln!(out, "# Brief: {root}\n")?;
    writeln!(
        out,
        "{total} Old Testament tokens across {} words. A token whose spelling \
         could be another word too is counted under each and marked *also*.\n",
        words.len()
    )?;

    writeln!(out, "## Words\n")?;
    writeln!(
        out,
        "| Word | BDB key | Part of speech | BDB gloss | Tokens |"
    )?;
    writeln!(out, "|---|---|---|---|---|")?;
    for w in &words {
        writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            w.word,
            w.key,
            w.pos,
            w.gloss,
            w.tokens.len()
        )?;
    }
    writeln!(out)?;

    let mut quoted: Vec<(i64, u16)> = Vec::new();
    for w in words
        .iter()
        .filter(|w| !w.tokens.is_empty() || !w.elsewhere.is_empty())
    {
        writeln!(out, "## {} ({}, {})\n", w.word, w.pos, w.key)?;
        if !w.elsewhere.is_empty() {
            let listed: Vec<String> = w
                .elsewhere
                .iter()
                .map(|t| {
                    format!(
                        "{} {} (root {})",
                        written(t.reference),
                        t.surface,
                        t.analysed_root
                    )
                })
                .collect();
            writeln!(
                out,
                "Filed here by spelling, but analysed as a verb of another root, so left out: {}\n",
                listed.join("; ")
            )?;
        }
        let mut stems: BTreeMap<&str, Vec<&Token>> = BTreeMap::new();
        for t in &w.tokens {
            stems.entry(t.stem.as_str()).or_default().push(t);
            quoted.push((t.reference, t.position));
        }
        let mut stems: Vec<_> = stems.into_iter().collect();
        stems.sort_by_key(|(_, tokens)| std::cmp::Reverse(tokens.len()));
        for (stem, tokens) in stems {
            let stem = if stem.is_empty() { "(no stem)" } else { stem };
            writeln!(out, "### {stem}: {} tokens\n", tokens.len())?;
            let mut renderings: HashMap<String, usize> = HashMap::new();
            for t in &tokens {
                let (_, rendered) = english.rendering(t)?;
                let rendered = if rendered.is_empty() {
                    "(unrendered)".to_string()
                } else {
                    rendered
                };
                *renderings.entry(rendered).or_default() += 1;
            }
            let mut renderings: Vec<_> = renderings.into_iter().collect();
            renderings.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let listed: Vec<String> = renderings
                .iter()
                .take(25)
                .map(|(r, n)| format!("{r} ×{n}"))
                .collect();
            writeln!(out, "Rendered: {}\n", listed.join("; "))?;
            for t in spread(&tokens, examples) {
                let (line, _) = english.rendering(t)?;
                let mut marks = String::new();
                if t.aramaic {
                    marks.push_str(" *Aramaic*");
                }
                if !t.also.is_empty() {
                    let _ = write!(marks, " *also {}*", t.also.join(", "));
                }
                writeln!(
                    out,
                    "- {} {}{marks}: {line}",
                    written(t.reference),
                    t.surface
                )?;
            }
            if tokens.len() > examples {
                let all: Vec<String> = tokens.iter().map(|t| written(t.reference)).collect();
                writeln!(out, "\nAll: {}", all.join(", "))?;
            }
            writeln!(out)?;
        }
    }

    quoted.sort_unstable();
    quoted.dedup();
    writeln!(
        out,
        "## New Testament quotations of verses using the root\n\n\
         Found by aligning the Hebrew with the Peshitta, scoring at least \
         {min_score}, with the root's word among the aligned Hebrew words. The \
         Greek is the TR's verse.\n"
    )?;
    let mut stmt = db.prepare(
        "SELECT a_ref, b_ref, score, a_positions FROM data.quotation \
         WHERE a_ref = ?1 AND b_ref >= ?2 AND score >= ?3 ORDER BY b_ref",
    )?;
    // Each New Testament verse once, with every Old Testament verse it aligns.
    let mut by_nt: BTreeMap<i64, Vec<(i64, f64)>> = BTreeMap::new();
    for &(reference, position) in &quoted {
        let rows = stmt
            .query_map(rusqlite::params![reference, NT_START, min_score], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, f64>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (a, b, score, positions) in rows {
            let aligned = positions
                .split(' ')
                .any(|p| p.parse::<u16>() == Ok(position));
            let sources = by_nt.entry(b).or_default();
            if aligned && !sources.iter().any(|(s, _)| *s == a) {
                sources.push((a, score));
            }
        }
    }
    by_nt.retain(|_, sources| !sources.is_empty());
    let mut listed: Vec<_> = by_nt.into_iter().collect();
    let best = |sources: &[(i64, f64)]| sources.iter().map(|s| s.1).fold(0.0, f64::max);
    listed.sort_by(|a, b| best(&b.1).total_cmp(&best(&a.1)).then(a.0.cmp(&b.0)));
    for (nt, mut sources) in listed.iter().cloned() {
        sources.sort_by(|a, b| b.1.total_cmp(&a.1));
        let from: Vec<String> = sources
            .iter()
            .map(|(a, score)| format!("{} {score:.1}", written(*a)))
            .collect();
        let (book, chapter, verse) = unpack(nt);
        let greek = bible
            .greek_verse_words(book, chapter, verse)
            .unwrap_or_default();
        let english: Vec<&str> = greek
            .iter()
            .map(|w| w.english.as_str())
            .filter(|e| !e.is_empty())
            .collect();
        let greek: Vec<&str> = greek.iter().map(|w| w.text.as_str()).collect();
        writeln!(
            out,
            "- {} ← {}\n  - {}\n  - {}",
            written(nt),
            from.join(", "),
            greek.join(" "),
            english.join(" ")
        )?;
    }
    if listed.is_empty() {
        writeln!(out, "None found.")?;
    }
    writeln!(out)?;

    writeln!(out, "## Peshitta cognate root\n")?;
    let sedra = bible.sedra_root_tree_by_letters(&root)?;
    if sedra.is_empty() {
        writeln!(out, "None.")?;
    }
    for lexeme in &sedra {
        writeln!(out, "- {}: {}", lexeme.lexeme, lexeme.meanings.join("; "))?;
    }
    writeln!(out)?;

    let lemmas: Vec<String> = words.iter().map(|w| w.word.clone()).collect();
    let glosses: Vec<String> = words.iter().map(|w| w.gloss.clone()).collect();
    out.push_str(&crate::cognate_search::cognate_section(
        src_texts, &root, &lemmas, &glosses,
    )?);
    out.push_str(&septuagint_section(&bible, src_texts, &words)?);

    writeln!(
        out,
        "## Other lexicons\n\nLeads only: follow each claim to its evidence and \
         cite that, never the lexicon.\n"
    )?;
    let bdb = bible.hebrew_bdb_by_root(&root)?;
    for entry in bible.root_lexicon(&root, bdb, Vec::new())? {
        writeln!(
            out,
            "### {} {}: {}\n\n{}\n",
            entry.source.as_str(),
            entry.headword,
            entry.gloss,
            plain_text(&entry.content_json)
        )?;
    }
    Ok(out)
}

/// `db lexicon-rank`: the `n` most frequent roots in the Old Testament, a
/// token whose spelling could be several words counting a share to each,
/// proper names left out (they get their own entries), with
/// the id of each root's entry in `lexicon_dir` where it has one.
pub fn lexicon_rank(data_dir: &Path, lexicon_dir: &Path, n: usize) -> Result<String> {
    let bible = Bible::open(data_dir).with_context(|| format!("opening {}", data_dir.display()))?;
    let db = haqor_core::data_support::connection(&bible);
    let lexicon = crate::haqor_lexicon::read_lexicon(lexicon_dir)?;
    let entries: HashMap<&str, &str> = lexicon
        .entries
        .iter()
        .map(|e| (e.root.as_str(), e.id.as_str()))
        .collect();
    let mut stmt = db.prepare(
        "WITH k AS (SELECT surface_id, key, \
                    1.0 / count(*) OVER (PARTITION BY surface_id) AS share \
                    FROM data.surface_entry) \
         SELECT e.root, round(sum(k.share)) AS tokens, \
                (SELECT group_concat(x.word, ' · ') FROM \
                   (SELECT word FROM data.lexicon_entry WHERE root = e.root AND (kind IS NOT 'root' OR coalesce(pos, '') <> '') AND coalesce(pos, '') NOT LIKE 'n.pr%' \
                    ORDER BY key LIMIT 4) x) \
         FROM data.word w JOIN k USING(surface_id) \
         JOIN data.lexicon_entry e ON e.key = k.key \
         WHERE w.ref < ?1 AND e.root <> '' AND coalesce(e.pos, '') NOT LIKE 'n.pr%' \
         GROUP BY e.root ORDER BY tokens DESC LIMIT ?2",
    )?;
    let mut out = String::from("| Rank | Root | Tokens | Words | Entry |\n|---|---|---|---|---|\n");
    let rows = stmt.query_map(rusqlite::params![NT_START, n as i64], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, f64>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    })?;
    for (rank, row) in rows.enumerate() {
        let (root, tokens, words) = row?;
        let entry = entries.get(root.as_str()).copied().unwrap_or("");
        writeln!(
            out,
            "| {} | {root} | {tokens:.0} | {} | {entry} |",
            rank + 1,
            words.unwrap_or_default()
        )?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_are_written_as_the_lexicon_writes_them() {
        assert_eq!(written(crate::runtime_db::pack_ref(1, 2, 7)), "Gen.2.7");
    }

    #[test]
    fn roots_match_with_their_weak_letters_aside() {
        assert!(same_root("בוא", "ביא"));
        assert!(same_root("אמר", "אמר"));
        assert!(!same_root("מור", "אמר"));
        assert!(!same_root("בנה", "בוא"));
    }

    #[test]
    fn spread_covers_the_list() {
        let items: Vec<u32> = (0..100).collect();
        let picked: Vec<u32> = spread(&items, 4).into_iter().copied().collect();
        assert_eq!(picked, vec![0, 25, 50, 75]);
    }

    #[test]
    fn plain_text_strips_markup() {
        assert_eq!(
            plain_text(r#"{"a": "<b>say</b>, speak", "b": ["tell"]}"#),
            "say, speak tell"
        );
    }
}
