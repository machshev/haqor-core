//! Which Greek words the Peshitta's words render, found from the two texts
//! themselves.
//!
//! No open edition aligns the Peshitta to the Greek word by word, but both
//! are here verse by verse: each Peshitta word tagged with its SEDRA lexeme,
//! each TR word with its dictionary form. Over the 7,957 verses they share, a
//! lexeme and the dictionary forms it stands beside again and again are
//! equivalents. IBM Model 1 is trained in each direction, a word free to
//! render nothing (Greek's article and καί have no Syriac word of their own:
//! SEDRA's words carry their proclitics), and each word of each verse
//! shares itself among the words of the other text as the trained model
//! expects. A pair is kept where both directions agree it is linked often
//! enough to count, and is a large enough part of either word's renderings.

use std::collections::HashMap;

use anyhow::Result;
use log::info;
use rusqlite::{Connection, params};

/// Rounds of expectation and maximisation; Model 1 has settled by then.
const ITERATIONS: usize = 10;

/// What a pair must reach to be kept: links, in expected words; and, unless
/// it is either word's commonest equivalent, a large part of one word's
/// renderings and more than a sliver of the other's. A word as common as
/// ܗܘܐ "be" stands beside δέ often enough to be linked to it a little, and
/// δέ to it, but neither is much of the other's renderings; ܐܝܬ "there is"
/// is εἰμί half the time, if only a fifth of εἰμί is it.
const MIN_LINKS: f64 = 2.0;
const MAJOR_SHARE: f64 = 0.4;
const MINOR_SHARE: f64 = 0.05;

/// The proclitics a Peshitta word is written with, before its lexeme: ܘ
/// "and", ܕ "of, that", ܠ "to", ܒ "in". SEDRA files the word under its
/// lexeme alone, so each is given a word of its own here, for Greek's καί,
/// articles and prepositions to be linked to rather than to the lexeme.
/// They are the letters before where the word best matches its lexeme:
/// ܕܕܘܝܕ is ܕ and ܕܘܝܕ, but ܒܪܗ, its lexeme ܒܪܐ, has none.
fn proclitics<'a>(word: &'a str, lexeme: &str) -> impl Iterator<Item = char> + 'a {
    let letters: Vec<char> = word.chars().collect();
    let lexeme: Vec<char> = lexeme.chars().collect();
    let matched = |at: usize| {
        letters[at..]
            .iter()
            .zip(&lexeme)
            .take_while(|(a, b)| a == b)
            .count()
    };
    let mut best = (matched(0), 0);
    for at in 1..=3.min(letters.len().saturating_sub(1)) {
        if !matches!(letters[at - 1], 'ו' | 'ד' | 'ל' | 'ב') {
            break;
        }
        let m = matched(at);
        if m > best.0 {
            best = (m, at);
        }
    }
    word.chars().take(best.1)
}

/// The pseudo-lexeme a proclitic stands as, above every real one.
fn proclitic_id(letter: char) -> u32 {
    PROCLITIC + letter as u32 % 64
}

/// Where the proclitics' ids begin; [`NONE`] is above them all.
const PROCLITIC: u32 = u32::MAX - 128;

/// Where the pronoun suffixes' ids begin, below the proclitics'.
const SUFFIX: u32 = u32::MAX - 256;

/// The verses both texts have: each one's Peshitta lexemes and TR
/// dictionary forms, as ids, with the dictionary forms' spellings.
struct Corpus {
    verses: Vec<(Vec<u32>, Vec<u32>)>,
    lemmas: Vec<String>,
}

fn read_corpus(db: &Connection) -> Result<Corpus> {
    let mut syriac: HashMap<i64, Vec<u32>> = HashMap::new();
    let mut stmt = db.prepare(
        "SELECT n.ref, w.lexeme_id, w.word, l.lexeme, w.suffix_person, w.suffix_number \
         FROM nt_word n \
         JOIN syriac_word w ON w.word_id = n.word_id \
         JOIN syriac_lexeme l ON l.lexeme_id = w.lexeme_id \
         ORDER BY n.ref, n.ord",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let words = syriac.entry(row.get(0)?).or_default();
        let word: String = row.get(2)?;
        let lexeme: String = row.get(3)?;
        words.extend(proclitics(&word, &lexeme).map(proclitic_id));
        words.push(row.get(1)?);
        // A pronoun suffix (ܠܗ "to him") is a word of its own for Greek's
        // pronouns to be linked to, as a proclitic is.
        let person: Option<u32> = row.get(4)?;
        let number: Option<u32> = row.get(5)?;
        if let Some(person) = person.filter(|p| *p != 0) {
            words.push(SUFFIX + person * 4 + number.unwrap_or(0));
        }
    }
    let mut ids: HashMap<String, u32> = HashMap::new();
    let mut lemmas = Vec::new();
    let mut greek: HashMap<i64, Vec<u32>> = HashMap::new();
    let mut stmt = db.prepare(
        "SELECT w.ref, l.lemma FROM greek_word w \
         JOIN greek_lemma l ON l.lemma_id = w.lemma_id \
         WHERE l.lemma NOT IN ('', 'ὁ') ORDER BY w.ref, w.position",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let lemma: String = row.get(1)?;
        let id = *ids.entry(lemma.clone()).or_insert_with(|| {
            lemmas.push(lemma);
            lemmas.len() as u32 - 1
        });
        greek.entry(row.get(0)?).or_default().push(id);
    }
    let mut refs: Vec<i64> = syriac
        .keys()
        .filter(|r| greek.contains_key(r))
        .copied()
        .collect();
    refs.sort_unstable();
    let verses = refs
        .into_iter()
        .map(|r| (syriac.remove(&r).unwrap(), greek.remove(&r).unwrap()))
        .collect();
    Ok(Corpus { verses, lemmas })
}

/// The id standing for no word, which a word may render.
const NONE: u32 = u32::MAX;

/// Model 1's probabilities that a source word renders a target one,
/// `t[(source, target)]`, trained on verse pairs of sources and targets.
fn train(pairs: &[(&[u32], &[u32])]) -> HashMap<(u32, u32), f64> {
    let mut t: HashMap<(u32, u32), f64> = HashMap::new();
    for (sources, targets) in pairs {
        for &target in *targets {
            for &source in sources.iter().chain([&NONE]) {
                t.insert((source, target), 1.0);
            }
        }
    }
    for _ in 0..ITERATIONS {
        let mut counts: HashMap<(u32, u32), f64> = HashMap::with_capacity(t.len());
        let mut totals: HashMap<u32, f64> = HashMap::new();
        for (sources, targets) in pairs {
            for &target in *targets {
                let norm: f64 = sources
                    .iter()
                    .chain([&NONE])
                    .map(|&s| t[&(s, target)])
                    .sum();
                for &source in sources.iter().chain([&NONE]) {
                    let share = t[&(source, target)] / norm;
                    *counts.entry((source, target)).or_default() += share;
                    *totals.entry(source).or_default() += share;
                }
            }
        }
        for ((source, target), count) in counts {
            t.insert((source, target), count / totals[&source]);
        }
    }
    t
}

/// How many times each source is expected to render each target over the
/// verse pairs, under a trained model.
fn links(pairs: &[(&[u32], &[u32])], t: &HashMap<(u32, u32), f64>) -> HashMap<(u32, u32), f64> {
    let mut out: HashMap<(u32, u32), f64> = HashMap::new();
    for (sources, targets) in pairs {
        for &target in *targets {
            let norm: f64 = sources
                .iter()
                .chain([&NONE])
                .map(|&s| t[&(s, target)])
                .sum();
            for &source in *sources {
                *out.entry((source, target)).or_default() += t[&(source, target)] / norm;
            }
        }
    }
    out
}

/// Each Peshitta lexeme's Greek equivalents with how many times it renders
/// each, by dictionary form; the pairs both directions agree on.
pub(crate) fn align(db: &Connection) -> Result<Vec<(u32, String, f64)>> {
    let corpus = read_corpus(db)?;
    let forward: Vec<(&[u32], &[u32])> = corpus
        .verses
        .iter()
        .map(|(s, g)| (s.as_slice(), g.as_slice()))
        .collect();
    let backward: Vec<(&[u32], &[u32])> = corpus
        .verses
        .iter()
        .map(|(s, g)| (g.as_slice(), s.as_slice()))
        .collect();
    // Syriac to Greek: which Greek word each Syriac word renders; and the
    // reverse.
    let syriac_greek = links(&forward, &train(&forward));
    let greek_syriac = links(&backward, &train(&backward));
    let mut lexeme_total: HashMap<u32, f64> = HashMap::new();
    let mut lemma_total: HashMap<u32, f64> = HashMap::new();
    let mut combined: Vec<(u32, u32, f64)> = Vec::new();
    for (&(lexeme, lemma), &a) in &syriac_greek {
        let b = greek_syriac.get(&(lemma, lexeme)).copied().unwrap_or(0.0);
        // Both directions must see the link: the smaller is what they agree.
        let agreed = a.min(b);
        *lexeme_total.entry(lexeme).or_default() += agreed;
        *lemma_total.entry(lemma).or_default() += agreed;
        combined.push((lexeme, lemma, agreed));
    }
    // Each word's commonest equivalent, by its links.
    let mut lexeme_best: HashMap<u32, f64> = HashMap::new();
    let mut lemma_best: HashMap<u32, f64> = HashMap::new();
    for &(lexeme, lemma, n) in &combined {
        let best = lexeme_best.entry(lexeme).or_default();
        *best = best.max(n);
        let best = lemma_best.entry(lemma).or_default();
        *best = best.max(n);
    }
    let mut out: Vec<(u32, String, f64)> = combined
        .into_iter()
        .filter(|&(lexeme, lemma, n)| {
            lexeme < SUFFIX
                && n >= MIN_LINKS
                && (n >= lexeme_best[&lexeme] || n >= lemma_best[&lemma] || {
                    let (a, b) = (n / lexeme_total[&lexeme], n / lemma_total[&lemma]);
                    a.max(b) >= MAJOR_SHARE && a.min(b) >= MINOR_SHARE
                })
        })
        .map(|(lexeme, lemma, n)| (lexeme, corpus.lemmas[lemma as usize].clone(), n))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0).then(b.2.total_cmp(&a.2)));
    Ok(out)
}

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS syriac_greek;

-- The Greek words each Peshitta lexeme renders, by TR dictionary form
-- (`greek_lemma.lemma`), with how many words of the two texts are linked:
-- found by aligning their shared verses, not given by any edition.
CREATE TABLE syriac_greek(
    lexeme_id INTEGER NOT NULL,
    lemma     TEXT    NOT NULL,
    links     INTEGER NOT NULL,
    PRIMARY KEY(lexeme_id, lemma)
) WITHOUT ROWID;
CREATE INDEX idx_syriac_greek_lemma ON syriac_greek(lemma);
";

/// Rebuild `syriac_greek` in a runtime database holding both texts.
pub fn build(db: &Connection) -> Result<usize> {
    let pairs = align(db)?;
    let tx = db.unchecked_transaction()?;
    tx.execute_batch(SCHEMA)?;
    {
        let mut insert =
            tx.prepare("INSERT INTO syriac_greek(lexeme_id, lemma, links) VALUES (?1, ?2, ?3)")?;
        for (lexeme, lemma, n) in &pairs {
            insert.execute(params![lexeme, lemma, n.round() as i64])?;
        }
    }
    tx.commit()?;
    info!("Peshitta and Greek: {} pairs of equivalents", pairs.len());
    Ok(pairs.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_one_finds_the_word_a_verse_pair_shares() {
        // Syriac 1 is always beside Greek 10, Syriac 2 beside Greek 20; 30
        // is Greek's article, beside everything.
        let verses: Vec<(Vec<u32>, Vec<u32>)> = vec![
            (vec![1], vec![10, 30]),
            (vec![1, 2], vec![10, 20, 30]),
            (vec![2], vec![20, 30]),
            (vec![1], vec![10]),
            (vec![2], vec![20]),
        ];
        let pairs: Vec<(&[u32], &[u32])> = verses
            .iter()
            .map(|(s, g)| (s.as_slice(), g.as_slice()))
            .collect();
        let t = train(&pairs);
        assert!(t[&(1, 10)] > t[&(1, 30)]);
        assert!(t[&(2, 20)] > t[&(2, 30)]);
        let links = links(&pairs, &t);
        assert!(links[&(1, 10)] > 2.5);
        assert!(links[&(1, 20)] < 0.5);
    }
}
