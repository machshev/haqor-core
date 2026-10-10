//! Read STEP Bible's TAGNT (Translators Amalgamated Greek NT, CC BY 4.0) as
//! the Textus Receptus: Scrivener's 1894 edition, the Greek behind the King
//! James Version.
//!
//! TAGNT amalgamates the major editions word by word: each word says which
//! editions have it (`NA28+NA27+…+TR+Byz`), where an edition spells it
//! otherwise (`+TR: Δαβὶδ`), and where one has it elsewhere in the verse
//! (`TR«3`). Its spelling and punctuation are Nestle-Aland's where that
//! edition has the word. Every word is tagged with a disambiguated Strong's
//! number, its grammar, its dictionary form and gloss, an English rendering
//! in context, and the TIPNR person or place it names.
//!
//! The TR is the words TAGNT gives the TR, spelled as the TR spells them. For
//! their order, which TAGNT's displacements give less dependably than its
//! words, the verse is lined up against Dr Maurice Robinson's public-domain
//! transcription of Scrivener (`greektext-scrivener`): unaccented and untagged,
//! but in the edition's own order. A verse whose words the two do not agree
//! on keeps TAGNT's order, displaced as TAGNT says.
//!
//! Verses are numbered as the KJV numbers them, which TAGNT gives in square
//! brackets where it differs from its own (NRSV) numbering.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use log::info;
use rusqlite::{Connection, params};

use crate::stepbible::name_key;

/// The published TAGNT files, in canonical order, by their path in the
/// STEPBible-Data repository.
pub(crate) const TAGNT_FILES: [&str; 2] = [
    "Translators Amalgamated OT+NT/TAGNT Mat-Jhn - Translators Amalgamated Greek NT - STEPBible.org CC-BY.txt",
    "Translators Amalgamated OT+NT/TAGNT Act-Rev - Translators Amalgamated Greek NT - STEPBible.org CC-BY.txt",
];

/// Where the fetch script puts Robinson's Scrivener files beside STEP
/// Bible's, and their names, in canonical order.
pub(crate) const SCRIVENER_DIR: &str = "greektext-scrivener/textonly";
const SCRIVENER_BOOKS: [&str; 27] = [
    "MT", "MR", "LU", "JOH", "AC", "RO", "1CO", "2CO", "GA", "EPH", "PHP", "COL", "1TH", "2TH",
    "1TI", "2TI", "TIT", "PHM", "HEB", "JAS", "1PE", "2PE", "1JO", "2JO", "3JO", "JUDE", "RE",
];

/// A verse: Haqor's book (40–66), chapter and verse.
pub(crate) type Verse = (u8, u8, u8);

/// A word of TAGNT the TR has, as the TR has it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TrWord {
    /// The verse as the KJV numbers it.
    pub verse: Verse,
    /// The word as the TR spells it, with the punctuation TAGNT follows.
    pub greek: String,
    /// The English TAGNT renders it by in context.
    pub english: String,
    /// The disambiguated Strong's number: a join key, never written out.
    pub strong: String,
    /// Its grammar, in Robinson's codes as TAGNT extends them (`N-GSM-P`).
    pub grammar: String,
    /// Its dictionary form and that form's gloss.
    pub lemma: String,
    pub gloss: String,
    /// The TIPNR person or place it names, by [`name_key`].
    pub name: Option<String>,
    /// How far the TR moves it from where TAGNT lists it: forward (after
    /// later words) when positive.
    pub displacement: i32,
}

/// The verse a TAGNT reference means in the KJV's numbering:
/// `Mat.17.15[17.14]#03=NKO` is Matthew 17:14, `Act.19.41(19.40)#01=NKO`
/// (bracketed for Nestle-Aland) Acts 19:41.
fn kjv_verse(field: &str) -> Option<Verse> {
    let reference = field.split('#').next()?;
    let (nrsv, kjv) = match reference.split_once('[') {
        Some((nrsv, rest)) => (nrsv, Some(rest.split(']').next()?)),
        None => (reference, None),
    };
    let nrsv = nrsv.split(['(', '{']).next()?;
    let (book, chapter, verse) = crate::journeys::parse_ref(nrsv)?;
    match kjv {
        Some(kjv) => {
            let (chapter, verse) = kjv.split_once('.')?;
            Some((book, chapter.parse().ok()?, verse.parse().ok()?))
        }
        None => Some((book, chapter, verse)),
    }
}

/// Greek in its canonical (NFC) form: an acute written as oxia (U+1F71 ά) is
/// the tonos it is canonically equivalent to (U+03AC ά), as TAGNT writes some
/// and not others.
pub(crate) fn canonical(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\u{1F71}' => '\u{03AC}',
            '\u{1F73}' => '\u{03AD}',
            '\u{1F75}' => '\u{03AE}',
            '\u{1F77}' => '\u{03AF}',
            '\u{1F79}' => '\u{03CC}',
            '\u{1F7B}' => '\u{03CD}',
            '\u{1F7D}' => '\u{03CE}',
            '\u{1FBB}' => '\u{0386}',
            '\u{1FC9}' => '\u{0388}',
            '\u{1FCB}' => '\u{0389}',
            '\u{1FDB}' => '\u{038A}',
            '\u{1FF9}' => '\u{038C}',
            '\u{1FEB}' => '\u{038E}',
            '\u{1FFB}' => '\u{038F}',
            '\u{1FD3}' => '\u{0390}',
            '\u{1FE3}' => '\u{03B0}',
            other => other,
        })
        .collect()
}

/// A word and the punctuation after it: `Ἀβραάμ.` is `Ἀβραάμ` and `.`.
fn split_punctuation(word: &str) -> (&str, &str) {
    let core = word.trim_end_matches([',', '.', '·', ';', '¶', ' ']);
    (core, &word[core.len()..])
}

/// The TR's spelling of a word, from TAGNT's spelling variants
/// (`Tyn+WH: Δαυεὶδ ; +TR: Δαβὶδ ;`), if it differs from the main one.
fn tr_spelling(variants: &str) -> Option<&str> {
    variants.split(';').find_map(|variant| {
        let (editions, word) = variant.split_once(':')?;
        editions
            .split('+')
            .any(|e| e.trim() == "TR")
            .then(|| word.trim())
            .filter(|w| !w.is_empty())
    })
}

/// Whether the TR has a word, from its editions field, and how far it moves
/// it: `TR`, `TR»1` (one word later), `TR«3` (earlier).
fn tr_edition(editions: &str) -> Option<i32> {
    editions.split('+').find_map(|e| {
        let rest = e.trim().strip_prefix("TR")?;
        if rest.is_empty() {
            return Some(0);
        }
        let (sign, count) = if let Some(n) = rest.strip_prefix('»') {
            (1, n)
        } else {
            (-1, rest.strip_prefix('«')?)
        };
        Some(sign * count.parse::<i32>().ok()?)
    })
}

/// TBESG's dictionary forms and glosses, by disambiguated Strong's number:
/// what a TR reading TAGNT gives only as a variant is looked up in.
pub(crate) type Lexicon = HashMap<String, (String, String)>;

/// Read TBESG: each row's disambiguated number (`G2424G =`), dictionary form
/// and gloss. Its Abbott-Smith definitions are not read.
pub(crate) fn read_tbesg(text: &str) -> Lexicon {
    let mut lexicon = Lexicon::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.trim_end_matches('\r').split('\t').collect();
        if fields.len() < 7 || !fields[0].starts_with('G') {
            continue;
        }
        let Some(strong) = fields[1].split_whitespace().next() else {
            continue;
        };
        lexicon
            .entry(strong.to_string())
            .or_insert_with(|| (fields[3].trim().to_string(), fields[6].trim().to_string()));
    }
    lexicon
}

/// A number's entry: exactly, or failing that the first of the numbers it
/// was disambiguated into (`G0300` as `G0300G`).
fn look_up<'a>(lexicon: &'a Lexicon, strong: &str) -> Option<&'a (String, String)> {
    lexicon.get(strong).or_else(|| {
        let mut keys: Vec<&String> = lexicon
            .keys()
            .filter(|k| k.starts_with(strong) && k.len() == strong.len() + 1)
            .collect();
        keys.sort();
        lexicon.get(*keys.first()?)
    })
}

/// How alike a word and a dictionary form look: the letters their
/// transliterations begin with in common.
fn likeness(word: &str, lemma: &str) -> usize {
    robinson(word)
        .chars()
        .zip(robinson(lemma).chars())
        .take_while(|(a, b)| a == b)
        .count()
}

/// The TR's reading where TAGNT gives it as a meaning variant of another
/// edition's word: `Ἀμών (t=Amōn) Amon - G0300=N-ASM-P in: TR+Byz`, one of
/// several separated by `¦`. A reading of several words lists their tags in
/// the lexicon's order, not the words', so each word takes the tag whose
/// dictionary form it looks most like. The reading's English goes on its first
/// word.
fn tr_variant(field: &str, verse: Verse, lexicon: &Lexicon) -> Option<Vec<TrWord>> {
    field.split('¦').find_map(|variant| {
        let (reading, editions) = variant.rsplit_once(" in: ")?;
        let displacement = tr_edition(editions)?;
        let (greek, rest) = reading.split_once(" (")?;
        let (_, rest) = rest.split_once(") ")?;
        let (english, tags) = rest.rsplit_once(" - ")?;
        let words: Vec<&str> = greek.split_whitespace().collect();
        let tags: Vec<(String, String)> = tags
            .split(" + ")
            .map(|tag| {
                let (strong, grammar) = tag.trim().split_once('=')?;
                Some((strong.to_string(), grammar.to_string()))
            })
            .collect::<Option<_>>()?;
        let lemma = |strong: &str| look_up(lexicon, strong).cloned().unwrap_or_default();
        let assigned: Vec<usize> = if words.len() == tags.len() && words.len() > 1 {
            best_assignment(&words, &tags, |s| lemma(s).0)
        } else if tags.len() == 1 {
            vec![0; words.len()]
        } else {
            // Unequal counts: in the order given, the last tag repeated.
            (0..words.len()).map(|i| i.min(tags.len() - 1)).collect()
        };
        let out = words
            .iter()
            .zip(assigned)
            .enumerate()
            .map(|(i, (word, tag))| {
                let (strong, grammar) = &tags[tag];
                let (lemma, gloss) = lemma(strong);
                TrWord {
                    verse,
                    greek: word.to_string(),
                    english: if i == 0 {
                        english.trim().to_string()
                    } else {
                        String::new()
                    },
                    strong: strong.clone(),
                    grammar: grammar.clone(),
                    lemma,
                    gloss,
                    name: None,
                    displacement,
                }
            })
            .collect();
        Some(out)
    })
}

/// The tag each word takes, as indexes into `tags`: the assignment whose
/// words look most like their dictionary forms, the given order winning a
/// tie. Readings are a few words long, so every assignment is tried.
fn best_assignment(
    words: &[&str],
    tags: &[(String, String)],
    lemma: impl Fn(&str) -> String,
) -> Vec<usize> {
    let lemmas: Vec<String> = tags.iter().map(|(s, _)| lemma(s)).collect();
    let n = words.len();
    if n > 6 {
        return (0..n).collect();
    }
    let mut best = ((0..n).collect::<Vec<_>>(), 0usize);
    best.1 = (0..n).map(|i| likeness(words[i], &lemmas[i])).sum();
    let mut permutation: Vec<usize> = (0..n).collect();
    // Heap's algorithm.
    let mut counters = vec![0; n];
    let mut i = 0;
    while i < n {
        if counters[i] < i {
            if i % 2 == 0 {
                permutation.swap(0, i);
            } else {
                permutation.swap(counters[i], i);
            }
            let score = (0..n)
                .map(|w| likeness(words[w], &lemmas[permutation[w]]))
                .sum();
            if score > best.1 {
                best = (permutation.clone(), score);
            }
            counters[i] += 1;
            i = 0;
        } else {
            counters[i] = 0;
            i += 1;
        }
    }
    best.0
}

/// Every word of a TAGNT file the TR has, in TAGNT's order.
pub(crate) fn parse(text: &str, lexicon: &Lexicon) -> Result<Vec<TrWord>> {
    let mut words = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.trim_end_matches('\r').split('\t').collect();
        let field = |n: usize| fields.get(n).copied().unwrap_or_default().trim();
        // Word rows only: `Mat.1.1#01=NKO`.
        if !field(0).contains('#') || !field(0).contains('=') || field(0).starts_with('#') {
            continue;
        }
        // `Jesus»Jesus|Jesus@Mat.1.1`: after `»`, a name's TIPNR key.
        let name = field(9)
            .split_once('»')
            .map(|(_, key)| key)
            .filter(|key| key.contains('@'))
            .map(name_key);
        let Some(displacement) = tr_edition(field(5)) else {
            // Another edition's word, which the TR may read otherwise.
            let verse = kjv_verse(field(0))
                .with_context(|| format!("an unreadable reference {}", field(0)))?;
            let Some(mut reading) = tr_variant(field(6), verse, lexicon) else {
                continue;
            };
            // The word's name is the reading's only where it is one word
            // standing for the other.
            if let [word] = reading.as_mut_slice() {
                word.name = name;
            }
            // The other edition's punctuation, after the reading.
            let punctuation = split_punctuation(field(1).split(" (").next().unwrap_or_default()).1;
            if let Some(last) = reading.last_mut() {
                last.greek.push_str(punctuation.replace('¶', "").trim());
            }
            words.extend(reading);
            continue;
        };
        let verse =
            kjv_verse(field(0)).with_context(|| format!("an unreadable reference {}", field(0)))?;
        // `Ἀβραάμ. (Abraam)`: the word, then its transliteration.
        let main = field(1).split(" (").next().unwrap_or_default();
        let (core, punctuation) = split_punctuation(main);
        let greek = match tr_spelling(field(7)) {
            Some(spelling) => format!("{}{punctuation}", split_punctuation(spelling).0),
            None => format!("{core}{punctuation}"),
        };
        // `H1732|G1138«G1138=N-GSM-P`: the word's own number is the last
        // before its grammar.
        let Some((tags, grammar)) = field(3).rsplit_once('=') else {
            bail!("{}: no grammar in {:?}", field(0), field(3));
        };
        let strong = tags
            .rsplit(['«', '|'])
            .next()
            .unwrap_or(tags)
            .trim()
            .to_string();
        let (lemma, gloss) = field(4).split_once('=').unwrap_or((field(4), ""));
        words.push(TrWord {
            verse,
            greek: greek.replace('¶', "").trim().to_string(),
            english: field(2).to_string(),
            strong,
            grammar: grammar.trim().to_string(),
            lemma: lemma.trim().to_string(),
            gloss: gloss.trim().to_string(),
            name,
            displacement,
        });
    }
    Ok(words)
}

/// The base letter of a Greek letter, its accents, breathings and iota
/// subscript taken off; `None` for a mark standing alone.
fn base_letter(c: char) -> Option<char> {
    let n = c as u32;
    if (0x0300..=0x036F).contains(&n) {
        return None;
    }
    let base = match n {
        0x0386 => 'Α',
        0x0388 => 'Ε',
        0x0389 => 'Η',
        0x038A | 0x03AA => 'Ι',
        0x038C => 'Ο',
        0x038E | 0x03AB => 'Υ',
        0x038F => 'Ω',
        0x03AC => 'α',
        0x03AD => 'ε',
        0x03AE => 'η',
        0x03AF | 0x0390 | 0x03CA => 'ι',
        0x03CC => 'ο',
        0x03CD | 0x03B0 | 0x03CB => 'υ',
        0x03CE => 'ω',
        0x1F00..=0x1FFF => {
            match n {
                0x1F00..=0x1F0F | 0x1F70 | 0x1F71 | 0x1F80..=0x1F8F | 0x1FB0..=0x1FBC => 'α',
                0x1F10..=0x1F1D | 0x1F72 | 0x1F73 | 0x1FC8 | 0x1FC9 => 'ε',
                0x1F20..=0x1F2F | 0x1F74 | 0x1F75 | 0x1F90..=0x1F9F | 0x1FC2..=0x1FC7 => 'η',
                0x1FCA..=0x1FCC => 'η',
                0x1F30..=0x1F3F | 0x1F76 | 0x1F77 | 0x1FD0..=0x1FDB => 'ι',
                0x1F40..=0x1F4D | 0x1F78 | 0x1F79 | 0x1FF8 | 0x1FF9 => 'ο',
                0x1F50..=0x1F5F | 0x1F7A | 0x1F7B | 0x1FE0..=0x1FE3 | 0x1FE6..=0x1FEB => 'υ',
                0x1FE4 | 0x1FE5 | 0x1FEC => 'ρ',
                0x1F60..=0x1F6F | 0x1F7C | 0x1F7D | 0x1FA0..=0x1FAF | 0x1FF2..=0x1FF7 => 'ω',
                0x1FFA..=0x1FFC => 'ω',
                // Breathings, accents and the elision mark written alone.
                _ => return None,
            }
        }
        _ => c,
    };
    Some(base)
}

/// A Greek word in Robinson's transliteration, as his Scrivener files write
/// it: unaccented, lower case, `y` for θ, `c` for χ, `q` for ψ and `v` for a
/// final ς. Anything not a Greek letter is dropped.
pub(crate) fn robinson(word: &str) -> String {
    word.chars()
        .filter_map(base_letter)
        .flat_map(char::to_lowercase)
        .filter_map(|c| {
            Some(match c {
                'α' => 'a',
                'β' => 'b',
                'γ' => 'g',
                'δ' => 'd',
                'ε' => 'e',
                'ζ' => 'z',
                'η' => 'h',
                'θ' => 'y',
                'ι' => 'i',
                'κ' => 'k',
                'λ' => 'l',
                'μ' => 'm',
                'ν' => 'n',
                'ξ' => 'x',
                'ο' => 'o',
                'π' => 'p',
                'ρ' => 'r',
                'σ' => 's',
                'ς' => 'v',
                'τ' => 't',
                'υ' => 'u',
                'φ' => 'f',
                'χ' => 'c',
                'ψ' => 'q',
                'ω' => 'w',
                _ => return None,
            })
        })
        .collect()
}

/// Robinson's Scrivener: each verse's words, in his transliteration. A book
/// is its chapter-and-verse markers (`1:2`) and the words after each; a
/// bracketed title heads the first.
pub(crate) fn read_scrivener(dir: &Path) -> Result<HashMap<Verse, Vec<String>>> {
    let mut verses: HashMap<Verse, Vec<String>> = HashMap::new();
    for (index, name) in SCRIVENER_BOOKS.iter().enumerate() {
        let path = dir.join(format!("{name}.SCV"));
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let book = 40 + index as u8;
        let mut current = None;
        let mut title = false;
        for token in text.split_whitespace() {
            if token.starts_with('[') {
                title = true;
            }
            if title {
                title = !token.ends_with(']');
                continue;
            }
            if let Some((chapter, verse)) = token.split_once(':') {
                current = Some((
                    book,
                    chapter
                        .parse()
                        .with_context(|| format!("{name}: {token}"))?,
                    verse.parse().with_context(|| format!("{name}: {token}"))?,
                ));
                continue;
            }
            let verse = current.with_context(|| format!("{name}: a word before any verse"))?;
            verses.entry(verse).or_default().push(token.to_string());
        }
    }
    Ok(verses)
}

/// A verse's words in TAGNT's order, displaced as TAGNT says: each moved
/// word is taken out and set back in as many places later, or earlier, as it
/// moves (an earlier one counting itself). Words moved to the same place keep
/// TAGNT's order.
fn displaced(words: &[&TrWord]) -> Vec<usize> {
    let mut keyed: Vec<(f64, usize)> = words
        .iter()
        .enumerate()
        .map(|(i, word)| match word.displacement {
            0 => (i as f64, i),
            d => (i as f64 + d as f64 + 0.5, i),
        })
        .collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    keyed.into_iter().map(|(_, i)| i).collect()
}

/// Levenshtein distance between two transliterations.
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substituted = previous + usize::from(ca != cb);
            previous = row[j + 1];
            row[j + 1] = substituted.min(previous + 1).min(row[j] + 1);
        }
    }
    row[b.len()]
}

/// Whether two transliterations are the same word spelled a little
/// differently: Ναζαρέτ and Ναζαρέθ, ἀλλά and ἀλλ᾽, εἶδε and εἶδεν.
pub(crate) fn alike(a: &str, b: &str) -> bool {
    let longer = a.len().max(b.len());
    longer >= 3 && distance(a, b) <= (longer / 4).max(1)
}

/// How one step of a chapter's alignment pairs TAGNT's words with
/// Scrivener's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    /// One word each, spelled the same or alike.
    Pair,
    /// A word of TAGNT's that Scrivener lacks, or the reverse.
    SkipTagnt,
    SkipScrivener,
    /// Two words of TAGNT's written as one by Scrivener (μή γε, μήγε), in
    /// either order, or one written as two (ἵνατί, ἵνα τί): TAGNT's division
    /// is kept.
    TwoForOne,
    OneForTwo,
}

/// The steps lining up a chapter's words, TAGNT's (`t`) against
/// Scrivener's (`s`), both transliterated: the alignment pairing the most
/// words, a word spelled the same counting for more than one spelled alike.
fn align(t: &[String], s: &[String]) -> Vec<Step> {
    let (n, m) = (t.len(), s.len());
    let width = m + 1;
    let mut score = vec![0i32; (n + 1) * width];
    let mut step = vec![Step::SkipTagnt; (n + 1) * width];
    for i in 0..=n {
        for j in 0..=m {
            if i == 0 && j == 0 {
                continue;
            }
            let mut best = (i32::MIN, Step::SkipTagnt);
            let mut consider = |value: i32, s: Step| {
                if value > best.0 {
                    best = (value, s);
                }
            };
            if i > 0 && j > 0 {
                let gain = if t[i - 1] == s[j - 1] {
                    3
                } else if alike(&t[i - 1], &s[j - 1]) {
                    2
                } else {
                    i32::MIN
                };
                if gain > 0 {
                    consider(score[(i - 1) * width + j - 1] + gain, Step::Pair);
                }
            }
            if i > 1
                && j > 0
                && (format!("{}{}", t[i - 2], t[i - 1]) == s[j - 1]
                    || format!("{}{}", t[i - 1], t[i - 2]) == s[j - 1])
            {
                consider(score[(i - 2) * width + j - 1] + 3, Step::TwoForOne);
            }
            if i > 0 && j > 1 && format!("{}{}", s[j - 2], s[j - 1]) == t[i - 1] {
                consider(score[(i - 1) * width + j - 2] + 3, Step::OneForTwo);
            }
            if i > 0 {
                consider(score[(i - 1) * width + j], Step::SkipTagnt);
            }
            if j > 0 {
                consider(score[i * width + j - 1], Step::SkipScrivener);
            }
            score[i * width + j] = best.0;
            step[i * width + j] = best.1;
        }
    }
    let mut steps = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        let s = step[i * width + j];
        steps.push(s);
        match s {
            Step::Pair => (i, j) = (i - 1, j - 1),
            Step::SkipTagnt => i -= 1,
            Step::SkipScrivener => j -= 1,
            Step::TwoForOne => (i, j) = (i - 2, j - 1),
            Step::OneForTwo => (i, j) = (i - 1, j - 2),
        }
    }
    steps.reverse();
    steps
}

/// Greek letters for Robinson's transliteration, unaccented: a word of
/// Scrivener's no edition TAGNT amalgamates spells as he does.
fn greek_letters(word: &str) -> String {
    let letters: Vec<char> = word
        .chars()
        .map(|c| match c {
            'a' => 'α',
            'b' => 'β',
            'g' => 'γ',
            'd' => 'δ',
            'e' => 'ε',
            'z' => 'ζ',
            'h' => 'η',
            'y' => 'θ',
            'i' => 'ι',
            'k' => 'κ',
            'l' => 'λ',
            'm' => 'μ',
            'n' => 'ν',
            'x' => 'ξ',
            'o' => 'ο',
            'p' => 'π',
            'r' => 'ρ',
            's' => 'σ',
            'v' => 'ς',
            't' => 'τ',
            'u' => 'υ',
            'f' => 'φ',
            'c' => 'χ',
            'q' => 'ψ',
            'w' => 'ω',
            other => other,
        })
        .collect();
    letters.into_iter().collect()
}

/// How the TR's text was put together, for the log.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Assembly {
    /// Scrivener's words, and of those: taken from TAGNT spelled the same,
    /// or spelled alike, and of those how many respelled as Scrivener spells
    /// them (the rest keep TAGNT's spelling); found by their spelling
    /// elsewhere in the TR; and left as Scrivener writes them, unaccented and
    /// untagged.
    pub words: usize,
    pub same: usize,
    pub alike: usize,
    pub respelled: usize,
    pub elsewhere: usize,
    pub untagged: usize,
    /// TAGNT's TR words Scrivener does not have, which are left out, and
    /// those kept where Scrivener divides the words otherwise.
    pub dropped: usize,
    pub divided: usize,
    /// Where words were left untagged or dropped, with the word.
    pub untagged_at: Vec<(Verse, String)>,
    pub dropped_at: Vec<(Verse, String)>,
}

/// The TR: Scrivener's words, in his order and verses, each taking its
/// accents and tags from the word of TAGNT's it lines up with. TAGNT's words
/// are lined up a chapter at a time, displaced as TAGNT says, so a verse the
/// two divide differently still pairs; a word one has moved further than
/// that is paired within its verse afterwards. A word of Scrivener's TAGNT
/// lacks takes the tags of the same spelling elsewhere in the TR.
pub(crate) fn tr_text(
    words: Vec<TrWord>,
    scrivener: &HashMap<Verse, Vec<String>>,
) -> (Vec<TrWord>, Assembly) {
    // The most usual word of each spelling, for a word TAGNT lacks.
    // Each spelling's forms (word, number, grammar): how many there are and
    // the first.
    type Forms = HashMap<(String, String, String), (usize, usize)>;
    let mut spellings: HashMap<String, Forms> = HashMap::new();
    for (index, word) in words.iter().enumerate() {
        let key = (
            split_punctuation(&word.greek).0.to_string(),
            word.strong.clone(),
            word.grammar.clone(),
        );
        spellings
            .entry(robinson(&word.greek))
            .or_default()
            .entry(key)
            .or_insert((0, index))
            .0 += 1;
    }
    let elsewhere: HashMap<String, usize> = spellings
        .into_iter()
        .filter_map(|(spelling, forms)| {
            let (_, (_, index)) = forms
                .into_iter()
                .max_by(|a, b| a.1.0.cmp(&b.1.0).then(b.1.1.cmp(&a.1.1)))?;
            Some((spelling, index))
        })
        .collect();

    // TAGNT's words by chapter, each verse displaced.
    let mut by_verse: HashMap<Verse, Vec<usize>> = HashMap::new();
    for (index, word) in words.iter().enumerate() {
        by_verse.entry(word.verse).or_default().push(index);
    }
    let mut chapters: HashMap<(u8, u8), Vec<usize>> = HashMap::new();
    let mut verses: Vec<&Verse> = by_verse.keys().collect();
    verses.sort();
    for verse in verses {
        let list: Vec<&TrWord> = by_verse[verse].iter().map(|&i| &words[i]).collect();
        let order = displaced(&list);
        chapters
            .entry((verse.0, verse.1))
            .or_default()
            .extend(order.into_iter().map(|i| by_verse[verse][i]));
    }
    let mut scrivener_chapters: HashMap<(u8, u8), Vec<(Verse, &String)>> = HashMap::new();
    let mut scrivener_verses: Vec<&Verse> = scrivener.keys().collect();
    scrivener_verses.sort();
    for verse in scrivener_verses {
        let chapter = scrivener_chapters.entry((verse.0, verse.1)).or_default();
        chapter.extend(scrivener[verse].iter().map(|w| (*verse, w)));
    }

    let mut assembly = Assembly::default();
    let mut out = Vec::new();
    let mut keys: Vec<&(u8, u8)> = scrivener_chapters.keys().collect();
    keys.sort();
    for key in keys {
        let s = &scrivener_chapters[key];
        let tagnt = chapters.get(key).cloned().unwrap_or_default();
        let t: Vec<String> = tagnt.iter().map(|&i| robinson(&words[i].greek)).collect();
        let spelled: Vec<String> = s.iter().map(|(_, w)| (*w).clone()).collect();
        // For each of Scrivener's words, the TAGNT words standing for it.
        let mut taken: Vec<Vec<usize>> = vec![Vec::new(); s.len()];
        let mut same = vec![false; s.len()];
        let mut paired_t = vec![false; t.len()];
        let (mut i, mut j) = (0, 0);
        for step in align(&t, &spelled) {
            match step {
                Step::Pair => {
                    taken[j].push(tagnt[i]);
                    same[j] = t[i] == spelled[j];
                    paired_t[i] = true;
                    i += 1;
                    j += 1;
                }
                Step::SkipTagnt => i += 1,
                Step::SkipScrivener => j += 1,
                Step::TwoForOne => {
                    // In the order that spells Scrivener's word.
                    if format!("{}{}", t[i], t[i + 1]) == spelled[j] {
                        taken[j].extend([tagnt[i], tagnt[i + 1]]);
                    } else {
                        taken[j].extend([tagnt[i + 1], tagnt[i]]);
                    }
                    same[j] = true;
                    paired_t[i] = true;
                    paired_t[i + 1] = true;
                    assembly.divided += 2;
                    i += 2;
                    j += 1;
                }
                Step::OneForTwo => {
                    taken[j].push(tagnt[i]);
                    same[j] = true;
                    // The second of Scrivener's two is in TAGNT's one.
                    taken[j + 1].push(usize::MAX);
                    paired_t[i] = true;
                    assembly.divided += 1;
                    i += 1;
                    j += 2;
                }
            }
        }
        // Words moved further than the alignment can follow: paired within
        // their verse, spelled the same, then alike.
        for exact in [true, false] {
            for j in 0..s.len() {
                if !taken[j].is_empty() {
                    continue;
                }
                let found = (0..t.len()).find(|&i| {
                    !paired_t[i]
                        && words[tagnt[i]].verse == s[j].0
                        && if exact {
                            t[i] == spelled[j]
                        } else {
                            alike(&t[i], &spelled[j])
                        }
                });
                if let Some(i) = found {
                    taken[j].push(tagnt[i]);
                    same[j] = exact;
                    paired_t[i] = true;
                }
            }
        }
        for (i, paired) in paired_t.iter().enumerate() {
            if !paired {
                assembly.dropped += 1;
                let word = &words[tagnt[i]];
                assembly.dropped_at.push((word.verse, word.greek.clone()));
            }
        }
        // The chapter's words, each with the TAGNT word it came from, if one.
        let mut chapter: Vec<(TrWord, Option<usize>)> = Vec::new();
        for (j, (verse, spelling)) in s.iter().enumerate() {
            assembly.words += 1;
            let word = |index: usize| TrWord {
                verse: *verse,
                displacement: 0,
                ..words[index].clone()
            };
            match taken[j].as_slice() {
                [usize::MAX] => {}
                [] => match elsewhere.get(*spelling) {
                    Some(&index) => {
                        assembly.elsewhere += 1;
                        chapter.push((
                            TrWord {
                                greek: split_punctuation(&words[index].greek).0.to_string(),
                                name: None,
                                ..word(index)
                            },
                            None,
                        ));
                    }
                    None => {
                        assembly.untagged += 1;
                        assembly.untagged_at.push((*verse, (*spelling).clone()));
                        chapter.push((
                            TrWord {
                                verse: *verse,
                                greek: greek_letters(spelling),
                                english: String::new(),
                                strong: String::new(),
                                grammar: String::new(),
                                lemma: String::new(),
                                gloss: String::new(),
                                name: None,
                                displacement: 0,
                            },
                            None,
                        ));
                    }
                },
                [index] if !same[j] => {
                    // Spelled alike: as Scrivener spells it, where the TR has
                    // his spelling elsewhere or the two differ only in the
                    // letters they end with; the rendering and name are this
                    // verse's.
                    assembly.alike += 1;
                    let tagged = word(*index);
                    let (core, punctuation) = split_punctuation(&tagged.greek);
                    let respelled = match elsewhere.get(*spelling) {
                        Some(&other) => Some(TrWord {
                            greek: format!(
                                "{}{punctuation}",
                                split_punctuation(&words[other].greek).0
                            ),
                            english: tagged.english.clone(),
                            name: tagged.name.clone(),
                            ..word(other)
                        }),
                        None => respell_ending(core, spelling).map(|greek| TrWord {
                            greek: format!("{greek}{punctuation}"),
                            ..tagged.clone()
                        }),
                    };
                    if respelled.is_some() {
                        assembly.respelled += 1;
                    }
                    chapter.push((respelled.unwrap_or(tagged), Some(*index)));
                }
                indexes => {
                    assembly.same += 1;
                    for &index in indexes {
                        chapter.push((word(index), Some(index)));
                    }
                }
            }
        }
        punctuate(&mut chapter, &words);
        out.extend(chapter.into_iter().map(|(word, _)| word));
    }
    (out, assembly)
}

/// A word TAGNT spells a little otherwise than Scrivener, respelled as he
/// does where they differ only in how they end: letters he leaves off are
/// cut (οὕτως as οὕτω, ἀλλά elided as ἀλλ᾽) and letters he adds are added
/// (εἶδε as εἶδεν). `None` where they differ otherwise.
fn respell_ending(accented: &str, scrivener: &str) -> Option<String> {
    let tagnt = robinson(accented);
    if let Some(extra) = scrivener.strip_prefix(tagnt.as_str()) {
        return (!extra.is_empty()).then(|| format!("{accented}{}", greek_letters(extra)));
    }
    let cut = tagnt.strip_prefix(scrivener)?;
    if cut.is_empty() || scrivener.is_empty() {
        return None;
    }
    // Keep the accented word up to the last of Scrivener's letters.
    let mut letters = 0;
    let mut end = 0;
    for (at, c) in accented.char_indices() {
        if base_letter(c).is_some_and(|b| !robinson(&b.to_string()).is_empty()) {
            if letters == scrivener.len() {
                break;
            }
            letters += 1;
        }
        end = at + c.len_utf8();
    }
    let kept = &accented[..end];
    // A vowel cut off is an elision.
    let elided = cut.starts_with(['a', 'e', 'h', 'i', 'o', 'u', 'w']);
    Some(if elided {
        format!("{kept}᾽")
    } else {
        kept.to_string()
    })
}

/// Punctuation for a chapter of the TR. TAGNT's follows the Nestle-Aland
/// text, so a mark stays only where the TR has the same two words either side
/// of it, and a verse's last mark goes on the TR's last word of the verse.
/// TAGNT's editorial brackets (Mark's ending, `[[…]]`) go.
fn punctuate(chapter: &mut [(TrWord, Option<usize>)], words: &[TrWord]) {
    // Whether TAGNT's word `index` ends its verse.
    let last = |index: usize| {
        words
            .get(index + 1)
            .is_none_or(|w| w.verse != words[index].verse)
    };
    let mut verse_end: HashMap<Verse, String> = HashMap::new();
    for i in 0..chapter.len() {
        let next_source = chapter
            .get(i + 1)
            .filter(|(w, _)| w.verse == chapter[i].0.verse)
            .and_then(|(_, source)| *source);
        let (word, source) = &mut chapter[i];
        let greek = word.greek.replace(['[', ']'], "");
        let (core, punctuation) = split_punctuation(&greek);
        let (core, punctuation) = (core.to_string(), punctuation.to_string());
        let shared = match *source {
            Some(index) if last(index) => {
                verse_end.insert(word.verse, punctuation.clone());
                false
            }
            Some(index) => next_source == Some(index + 1),
            None => false,
        };
        word.greek = if shared { greek } else { core };
    }
    for i in 0..chapter.len() {
        let ends = chapter
            .get(i + 1)
            .is_none_or(|(w, _)| w.verse != chapter[i].0.verse);
        if ends && let Some(mark) = verse_end.get(&chapter[i].0.verse) {
            let word = &mut chapter[i].0;
            word.greek = format!("{}{mark}", split_punctuation(&word.greek).0);
        }
    }
}

// Preparing and building: the TR written out without its Strong's numbers,
// and read into the runtime database.

/// The prepared files, in the STEPBible-Data directory of `src_texts`:
///
/// - `tr.tsv`: the TR's words, a line each in order: the verse (`Mat.1.1`),
///   the word, its English in context, the id of its dictionary form in
///   `greek_lemmas.tsv`, its grammar, and the id of the person or place it
///   names in `names.jsonl`; an id is empty where it has none.
/// - `greek_lemmas.tsv`: the dictionary forms: id, form and gloss.
pub(crate) const PREPARED_TR: &str = "tr.tsv";
pub(crate) const PREPARED_LEMMAS: &str = "greek_lemmas.tsv";

/// STEP Bible's abbreviation for a New Testament book.
fn abbreviation(book: u8) -> &'static str {
    crate::journeys::BOOKS[usize::from(book) - 1]
}

/// The prepared files' text: the TR's words, and their dictionary forms,
/// numbered in the order the text first uses them. `name_id` gives the id in
/// `names.jsonl` of a TIPNR key named in a verse.
pub(crate) fn prepared(
    words: &[TrWord],
    name_id: impl Fn(&str, Verse) -> Option<u32>,
) -> (String, String) {
    let mut lemma_ids: HashMap<&str, u32> = HashMap::new();
    let mut lemmas = String::new();
    let mut text = String::new();
    for word in words {
        let lemma = (!word.strong.is_empty()).then(|| {
            let next = lemma_ids.len() as u32 + 1;
            *lemma_ids.entry(&word.strong).or_insert_with(|| {
                lemmas.push_str(&format!(
                    "{next}\t{}\t{}\n",
                    canonical(&word.lemma),
                    word.gloss
                ));
                next
            })
        });
        let id = |id: Option<u32>| id.map(|i| i.to_string()).unwrap_or_default();
        let (book, chapter, verse) = word.verse;
        text.push_str(&format!(
            "{}.{chapter}.{verse}\t{}\t{}\t{}\t{}\t{}\n",
            abbreviation(book),
            canonical(&word.greek),
            word.english,
            id(lemma),
            word.grammar,
            id(word
                .name
                .as_deref()
                .and_then(|key| name_id(key, word.verse))),
        ));
    }
    (text, lemmas)
}

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS greek_word;
DROP TABLE IF EXISTS greek_lemma;
DROP TABLE IF EXISTS greek_grammar;
DROP TABLE IF EXISTS greek_english;

-- The Textus Receptus (Scrivener 1894), the Greek behind the KJV, a word a
-- row, numbered as the KJV numbers its verses. `position` counts the verse's
-- words from 0; `word` is accented, with the punctuation after it;
-- `lemma_id` its dictionary form, `grammar_id` its Robinson grammar code and
-- `english_id` its English in context, each NULL where it has none;
-- `entity_id` the person or place it names (`name_entity`).
CREATE TABLE greek_word(
    ref        INTEGER NOT NULL,
    position   INTEGER NOT NULL,
    word       TEXT    NOT NULL,
    lemma_id   INTEGER,
    grammar_id INTEGER,
    english_id INTEGER,
    entity_id  INTEGER,
    PRIMARY KEY(ref, position)
) WITHOUT ROWID;
CREATE INDEX idx_greek_word_lemma ON greek_word(lemma_id);
CREATE INDEX idx_greek_word_entity ON greek_word(entity_id) WHERE entity_id IS NOT NULL;

CREATE TABLE greek_lemma(
    lemma_id INTEGER PRIMARY KEY,
    lemma    TEXT NOT NULL,
    gloss    TEXT NOT NULL
);
CREATE TABLE greek_grammar(grammar_id INTEGER PRIMARY KEY, code TEXT NOT NULL);
CREATE TABLE greek_english(english_id INTEGER PRIMARY KEY, text TEXT NOT NULL);
";

/// What [`build_greek`] wrote, for the log.
#[derive(Debug, Default)]
pub struct GreekSummary {
    pub verses: usize,
    pub words: usize,
    pub lemmas: usize,
    /// Words with no dictionary form, and words naming a person or place.
    pub untagged: usize,
    pub names: usize,
}

/// Rebuild the Greek tables of a runtime database in place from the
/// prepared files in `src_texts`.
pub fn build_greek(db: &Connection, src_texts: &Path) -> Result<GreekSummary> {
    let dir = crate::stepbible::source_dir(src_texts);
    let read = |name: &str| {
        let path = dir.join(name);
        std::fs::read_to_string(&path).with_context(|| {
            format!(
                "reading {} (run scripts/fetch-stepbible-data.sh)",
                path.display()
            )
        })
    };
    let lemmas = read(PREPARED_LEMMAS)?;
    let text = read(PREPARED_TR)?;

    let tx = db.unchecked_transaction()?;
    tx.execute_batch(SCHEMA)?;
    let mut summary = GreekSummary::default();
    {
        let mut insert =
            tx.prepare("INSERT INTO greek_lemma(lemma_id, lemma, gloss) VALUES (?1, ?2, ?3)")?;
        for line in lemmas.lines() {
            let mut fields = line.split('\t');
            let (Some(id), Some(lemma), Some(gloss)) =
                (fields.next(), fields.next(), fields.next())
            else {
                bail!("an unreadable dictionary form {line:?}");
            };
            insert.execute(params![id.parse::<i64>()?, lemma, gloss])?;
            summary.lemmas += 1;
        }
    }
    let mut grammars: HashMap<String, i64> = HashMap::new();
    let mut englishes: HashMap<String, i64> = HashMap::new();
    {
        let mut insert = tx.prepare(
            "INSERT INTO greek_word(ref, position, word, lemma_id, grammar_id, english_id, \
             entity_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        let mut previous = None;
        let mut position = 0i64;
        for line in text.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            let [reference, word, english, lemma, grammar, name] = fields[..] else {
                bail!("an unreadable TR word {line:?}");
            };
            let (book, chapter, verse) = crate::journeys::parse_ref(reference)
                .with_context(|| format!("an unreadable verse {reference:?}"))?;
            let reference = crate::runtime_db::pack_ref(book.into(), chapter.into(), verse.into());
            if previous == Some(reference) {
                position += 1;
            } else {
                previous = Some(reference);
                position = 0;
                summary.verses += 1;
            }
            let intern = |pool: &mut HashMap<String, i64>, value: &str| {
                (!value.is_empty()).then(|| {
                    let next = pool.len() as i64 + 1;
                    *pool.entry(value.to_string()).or_insert(next)
                })
            };
            let id = |value: &str| -> Result<Option<i64>> {
                Ok(if value.is_empty() {
                    None
                } else {
                    Some(value.parse()?)
                })
            };
            let lemma = id(lemma)?;
            let name = id(name)?;
            summary.words += 1;
            summary.untagged += usize::from(lemma.is_none());
            summary.names += usize::from(name.is_some());
            insert.execute(params![
                reference,
                position,
                word,
                lemma,
                intern(&mut grammars, grammar),
                intern(&mut englishes, english),
                name
            ])?;
        }
    }
    for (table, column, pool) in [
        ("greek_grammar", "grammar_id, code", &grammars),
        ("greek_english", "english_id, text", &englishes),
    ] {
        let mut insert = tx.prepare(&format!("INSERT INTO {table}({column}) VALUES (?1, ?2)"))?;
        for (value, id) in pool {
            insert.execute(params![id, value])?;
        }
    }
    tx.commit()?;
    info!(
        "Greek: {} TR words in {} verses ({} untagged, {} naming a person or place), {} \
         dictionary forms",
        summary.words, summary.verses, summary.untagged, summary.names, summary.lemmas
    );
    Ok(summary)
}

/// `db gen-greek`: rebuild the Greek tables of the runtime database at `path`
/// in place and re-stamp it, so syncing it to the app reinstalls it.
pub fn gen_greek(path: &Path, src_texts: &Path) -> Result<GreekSummary> {
    let db = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
    let summary = build_greek(&db, src_texts)?;
    crate::runtime_db::restamp_built(&db)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_greek_canonically() {
        assert_eq!(
            canonical("\u{1F00}\u{03B3}\u{1F71}\u{03C0}\u{03B7}"),
            "\u{1F00}\u{03B3}\u{03AC}\u{03C0}\u{03B7}"
        );
    }

    #[test]
    fn transliterates_as_robinson() {
        assert_eq!(robinson("Βίβλος"), "biblov");
        assert_eq!(robinson("γενέσεως"), "genesewv");
        assert_eq!(robinson("Ἰησοῦ"), "ihsou");
        assert_eq!(robinson("Χριστοῦ,"), "cristou");
        assert_eq!(robinson("μετ᾽"), "met");
        assert_eq!(robinson("Ματθαῖον"), "matyaion");
        assert_eq!(robinson("ψυχῇ"), "quch");
        assert_eq!(robinson("δόξα"), "doxa");
        assert_eq!(robinson("Ῥαχάβ·"), "racab");
    }

    const MATTHEW_2_3: &str = "\
Mat.2.3#01=NKO\tἈκούσας (Akousas)\thaving heard [this]\tG0191=V-AAP-NSM\tἀκούω=to hear\tNA28+NA27+Tyn+SBL+WH+Treg+TR+Byz\t\t\t\thear\t\t
Mat.2.3#02=NKO\tδὲ (de)\tnow\tG1161=CONJ\tδέ=but/and\tNA28+NA27+Tyn+SBL+WH+Treg+TR+Byz\t\t\t\tbut\t\t
Mat.2.3#03=NKO\tὁ (ho)\tthe\tG3588=T-NSM\tὁ=the/this/who\tNA28+NA27+Tyn+SBL+WH+Treg+TR+Byz\t\t\t\tthe\t\t
Mat.2.3#04=NKO\tβασιλεὺς (basileus)\tking\tG0935=N-NSM-T\tβασιλεύς=king\tNA28+NA27+Tyn+SBL+WH+Treg+TR+Byz\t\t\t\tking\t\t
Mat.2.3#05=NKO\tἩρῴδης (Hērōdēs)\tHerod\tG2264=N-NSM-P\tἩρῴδης=Herod\tNA28+NA27+Tyn+SBL+WH+Treg+TR«3+Byz«3\t\tTR+Byz: Ἡρώδης ;\t\tHerod»Herod|Herod@Mat.2.1\t\t
Mat.2.3#06=NKO\tἐταράχθη, (etarachthē)\twas troubled\tG5015=V-API-3S\tταράσσω=to trouble\tNA28+NA27+Tyn+SBL+WH+Treg+TR+Byz\t\t\t\ttrouble\t\t
Mat.2.3#07=N(k)O\tκαὶ (kai)\tand\tG2532=CONJ\tκαί=and\tNA28+NA27+Tyn+SBL+WH+Treg+Byz\t\t\t\tand\t\t
";

    #[test]
    fn reads_the_tr_words_in_its_spelling() {
        let words = parse(MATTHEW_2_3, &Lexicon::new()).unwrap();
        // The last word is not the TR's.
        assert_eq!(words.len(), 6);
        let herod = &words[4];
        assert_eq!(herod.verse, (40, 2, 3));
        assert_eq!(herod.greek, "Ἡρώδης");
        assert_eq!(herod.strong, "G2264");
        assert_eq!(herod.grammar, "N-NSM-P");
        assert_eq!(
            (herod.lemma.as_str(), herod.gloss.as_str()),
            ("Ἡρῴδης", "Herod")
        );
        assert_eq!(herod.name.as_deref(), Some("Herod@Mat.2.1"));
        assert_eq!(herod.displacement, -3);
        assert_eq!(words[5].greek, "ἐταράχθη,");
    }

    #[test]
    fn orders_a_verse_as_scrivener() {
        let words = parse(MATTHEW_2_3, &Lexicon::new()).unwrap();
        let scrivener: HashMap<Verse, Vec<String>> = [(
            (40, 2, 3),
            "akousav de hrwdhv o basileuv etaracyh"
                .split(' ')
                .map(str::to_string)
                .collect(),
        )]
        .into();
        let (text, assembly) = tr_text(words.clone(), &scrivener);
        let greek: Vec<&str> = text.iter().map(|w| w.greek.as_str()).collect();
        assert_eq!(
            greek,
            ["Ἀκούσας", "δὲ", "Ἡρώδης", "ὁ", "βασιλεὺς", "ἐταράχθη,"]
        );
        assert_eq!((assembly.words, assembly.same), (6, 6));
        // TAGNT's displacement says the same.
        let refs: Vec<&TrWord> = words.iter().collect();
        assert_eq!(displaced(&refs)[2], 4);
    }

    #[test]
    fn respells_endings_as_scrivener() {
        assert_eq!(respell_ending("οὕτως", "outw").as_deref(), Some("οὕτω"));
        assert_eq!(respell_ending("ἀλλὰ", "all").as_deref(), Some("ἀλλ᾽"));
        assert_eq!(respell_ending("εἶδε", "eiden").as_deref(), Some("εἶδεν"));
        assert_eq!(respell_ending("αὐτοῦ", "autw"), None);
    }

    #[test]
    fn lines_up_words_divided_otherwise() {
        let t = ["ei", "de", "ge", "mh", "misyon"].map(String::from);
        let s = ["ei", "de", "mhge", "misyon"].map(String::from);
        assert_eq!(
            align(&t, &s),
            [Step::Pair, Step::Pair, Step::TwoForOne, Step::Pair]
        );
        let t = ["inati", "umeiv"].map(String::from);
        let s = ["ina", "ti", "umeiv"].map(String::from);
        assert_eq!(align(&t, &s), [Step::OneForTwo, Step::Pair]);
        // Spelled alike, and a word only one has.
        let t = ["nazaret", "opwv"].map(String::from);
        let s = ["nazarey", "kai", "opwv"].map(String::from);
        assert_eq!(align(&t, &s), [Step::Pair, Step::SkipScrivener, Step::Pair]);
    }

    #[test]
    fn numbers_verses_as_the_kjv() {
        assert_eq!(kjv_verse("Mat.17.15[17.14]#03=NKO"), Some((40, 17, 14)));
        assert_eq!(kjv_verse("Act.19.41(19.40)#01=NKO"), Some((44, 19, 41)));
        assert_eq!(kjv_verse("Jhn.7.53{8.1}#01=K"), Some((43, 7, 53)));
        assert_eq!(kjv_verse("Mat.1.1#01=NKO"), Some((40, 1, 1)));
    }
}
