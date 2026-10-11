//! The Greek New Testament: the Textus Receptus (Scrivener 1894), the Greek
//! behind the KJV, from STEP Bible's TAGNT, as `haqor.db` stores it beside
//! the Peshitta.

/// A word of the Greek text, with what TAGNT tags it with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GreekWord {
    /// The word's place in its verse, from 0.
    pub position: u32,
    /// The word, accented, with the punctuation after it.
    pub text: String,
    /// Its English in this verse; empty where it has none.
    pub english: String,
    /// Its dictionary form and that form's gloss; empty for the few words
    /// with no tags.
    pub lemma: String,
    pub gloss: String,
    /// Its grammar, in Robinson's codes as TAGNT extends them: `N-NSF`,
    /// `V-AAI-3S`, `N-GSM-P` for a proper name.
    pub grammar: String,
    /// The person or place it names, by `name_entity` id, as
    /// [`crate::bible::Bible::name_entity`] takes it.
    pub name: Option<u32>,
}

/// Where a word of the Greek text stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GreekOccurrence {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
    pub position: u32,
}

/// An entry of a Greek lexicon: Abbott-Smith's (`abbott-smith`) or Thayer's
/// (`thayer`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GreekLexiconEntry {
    pub source: String,
    pub headword: String,
    /// Its gloss, empty where the lexicon gives none apart from the entry.
    pub gloss: String,
    /// The entry as span JSON: `senses`, each with its `definition` spans
    /// and the senses within it, and `forms`, `etymology`, `lxx` and
    /// `related` spans beside them.
    pub body: String,
}

/// A Greek word a Peshitta lexeme renders, found by aligning the two texts:
/// its dictionary form and gloss, and how many of their words are linked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GreekEquivalent {
    pub lemma: String,
    pub gloss: String,
    pub links: u32,
}

/// A Peshitta lexeme a Greek dictionary form is rendered by: the lexeme in
/// SEDRA's Hebrew letters and its first meaning, the commonest word of it in
/// the text (vocalised, as the reader shows it), and how many words are
/// linked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyriacEquivalent {
    pub lexeme_id: i64,
    pub lexeme: String,
    pub meaning: String,
    pub word: String,
    pub links: u32,
}

/// A word of the Greek text built on a root: where it stands, its dictionary
/// form and its grammar code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GreekRootOccurrence {
    pub occurrence: GreekOccurrence,
    pub lemma: String,
    pub grammar: String,
}

/// A dictionary form of the TR built on a root, with its gloss and how many
/// words of the text it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GreekRootLemma {
    pub lemma: String,
    pub gloss: String,
    pub occurrences: u32,
}

/// A verse of the Greek text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GreekVerse {
    pub verse: u8,
    pub words: Vec<GreekWord>,
}

impl GreekVerse {
    /// The verse as running text.
    pub fn text(&self) -> String {
        self.words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// A grammar code in words: `V-AAI-3S` is "verb, aorist active indicative,
/// 3rd person singular", `N-GSM-P` "noun, genitive singular masculine, a
/// person's name". Robinson's codes as TAGNT extends them; a part it does
/// not know is left as it is written.
pub fn describe_grammar(code: &str) -> String {
    let mut parts = code.split('-');
    let Some(head) = parts.next() else {
        return String::new();
    };
    let rest: Vec<&str> = parts.collect();
    let (name, parsing, suffixes): (&str, Option<&str>, &[&str]) = match head {
        "N" => ("noun", rest.first().copied(), rest.get(1..).unwrap_or(&[])),
        "A" => (
            "adjective",
            rest.first().copied(),
            rest.get(1..).unwrap_or(&[]),
        ),
        "T" => ("article", rest.first().copied(), &[]),
        "V" => ("verb", rest.first().copied(), &[]),
        "P" => ("personal pronoun", rest.first().copied(), &[]),
        "R" => ("relative pronoun", rest.first().copied(), &[]),
        "C" => ("reciprocal pronoun", rest.first().copied(), &[]),
        "D" => ("demonstrative pronoun", rest.first().copied(), &[]),
        "F" => ("reflexive pronoun", rest.first().copied(), &[]),
        "I" => ("interrogative pronoun", rest.first().copied(), &[]),
        "K" => ("correlative pronoun", rest.first().copied(), &[]),
        "Q" => (
            "correlative or interrogative pronoun",
            rest.first().copied(),
            &[],
        ),
        "S" => ("possessive pronoun", rest.first().copied(), &[]),
        "X" => ("indefinite pronoun", rest.first().copied(), &[]),
        "ADV" => ("adverb", None, &rest[..]),
        "CONJ" => ("conjunction", None, &rest[..]),
        "COND" => ("conditional particle", None, &rest[..]),
        "PRT" => ("particle", None, &rest[..]),
        "PREP" => ("preposition", None, &rest[..]),
        "INJ" => ("interjection", None, &rest[..]),
        "ARAM" => ("Aramaic word", None, &rest[..]),
        "HEB" => ("Hebrew word", None, &rest[..]),
        _ => return code.to_string(),
    };
    let mut out = vec![name.to_string()];
    if let Some(parsing) = parsing {
        let described = if head == "V" {
            describe_verb(parsing, rest.get(1..).unwrap_or(&[]))
        } else if head == "S" {
            describe_possessive(parsing)
        } else {
            describe_nominal(parsing)
        };
        out.extend(described);
    }
    out.extend(
        suffixes
            .iter()
            .filter_map(|s| describe_suffix(head, s))
            .map(str::to_string),
    );
    out.join(", ")
}

fn case(c: char) -> Option<&'static str> {
    Some(match c {
        'N' => "nominative",
        'G' => "genitive",
        'D' => "dative",
        'A' => "accusative",
        'V' => "vocative",
        _ => return None,
    })
}

fn number(c: char) -> Option<&'static str> {
    Some(match c {
        'S' => "singular",
        'P' => "plural",
        _ => return None,
    })
}

fn gender(c: char) -> Option<&'static str> {
    Some(match c {
        'M' => "masculine",
        'F' => "feminine",
        'N' => "neuter",
        _ => return None,
    })
}

fn person(c: char) -> Option<&'static str> {
    Some(match c {
        '1' => "1st person",
        '2' => "2nd person",
        '3' => "3rd person",
        _ => return None,
    })
}

/// Case, number and gender (`GSM`), after a person where a pronoun has one
/// (`1GS`, `3GSM`); `NUI`, an indeclinable numeral.
fn describe_nominal(parsing: &str) -> Vec<String> {
    if parsing == "NUI" {
        return vec!["indeclinable numeral".into()];
    }
    let mut chars: Vec<char> = parsing.chars().collect();
    let mut out = Vec::new();
    if let Some(p) = chars.first().and_then(|&c| person(c)) {
        out.push(p.to_string());
        chars.remove(0);
    }
    let words: Vec<&str> = chars
        .iter()
        .enumerate()
        .filter_map(|(i, &c)| match i {
            0 => case(c),
            1 => number(c),
            _ => gender(c),
        })
        .collect();
    if words.len() == chars.len() {
        if !words.is_empty() {
            out.push(words.join(" "));
        }
    } else {
        out.push(parsing.to_string());
    }
    out
}

/// A possessive pronoun: whose (`1S`, of me), then the case, number and
/// gender it agrees with.
fn describe_possessive(parsing: &str) -> Vec<String> {
    let mut chars = parsing.chars();
    let (Some(p), Some(n)) = (chars.next().and_then(person), chars.next().and_then(number)) else {
        return vec![parsing.to_string()];
    };
    let mut out = vec![format!("{p} {n} possessor")];
    out.extend(describe_nominal(chars.as_str()));
    out
}

fn tense(c: char) -> Option<&'static str> {
    Some(match c {
        'P' => "present",
        'I' => "imperfect",
        'F' => "future",
        'A' => "aorist",
        'R' => "perfect",
        'L' => "pluperfect",
        _ => return None,
    })
}

fn voice(c: char) -> Option<&'static str> {
    Some(match c {
        'A' => "active",
        'M' => "middle",
        'P' => "passive",
        'E' => "middle or passive",
        'D' => "middle deponent",
        'O' => "passive deponent",
        'N' => "middle or passive deponent",
        _ => return None,
    })
}

fn mood(c: char) -> Option<&'static str> {
    Some(match c {
        'I' => "indicative",
        'S' => "subjunctive",
        'O' => "optative",
        'M' => "imperative",
        'N' => "infinitive",
        'P' => "participle",
        _ => return None,
    })
}

/// A grammar code's parts, one a dimension, for filtering words by: the part
/// of speech as [`describe_grammar`] names it, then each of the rest in
/// words, empty where the code has none (a noun has no tense, an infinitive
/// no person). A second aorist is an aorist, and a possessive pronoun's
/// case, number and gender are those it agrees in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GreekParse {
    pub part_of_speech: String,
    pub tense: String,
    pub voice: String,
    pub mood: String,
    pub case: String,
    pub person: String,
    pub number: String,
    pub gender: String,
}

/// [`GreekParse`] for a grammar code: `V-AAI-3S` is a verb, aorist, active,
/// indicative, 3rd person, singular.
pub fn parse_grammar(code: &str) -> GreekParse {
    let mut out = GreekParse {
        part_of_speech: describe_grammar(code)
            .split(", ")
            .next()
            .unwrap_or_default()
            .to_string(),
        ..GreekParse::default()
    };
    let mut parts = code.split('-');
    let head = parts.next().unwrap_or_default();
    let rest: Vec<&str> = parts.collect();
    let set = |slot: &mut String, value: Option<&str>| {
        if let Some(value) = value {
            *slot = value.to_string();
        }
    };
    // Case, number and gender, after a person where the code gives one.
    let nominal = |out: &mut GreekParse, parsing: &str| {
        let mut chars: Vec<char> = parsing.chars().collect();
        if let Some(p) = chars.first().and_then(|&c| person(c)) {
            out.person = p.to_string();
            chars.remove(0);
        }
        set(&mut out.case, chars.first().and_then(|&c| case(c)));
        set(&mut out.number, chars.get(1).and_then(|&c| number(c)));
        set(&mut out.gender, chars.get(2).and_then(|&c| gender(c)));
    };
    match head {
        "V" => {
            let Some(parsing) = rest.first() else {
                return out;
            };
            let mut chars = parsing.trim_start_matches('2').chars();
            set(&mut out.tense, chars.next().and_then(tense));
            set(&mut out.voice, chars.next().and_then(voice));
            set(&mut out.mood, chars.next().and_then(mood));
            for part in rest.iter().skip(1) {
                let mut chars = part.chars();
                match chars.next() {
                    Some(c) if person(c).is_some() => {
                        set(&mut out.person, person(c));
                        set(&mut out.number, chars.next().and_then(number));
                    }
                    Some(c) if part.len() == 3 && case(c).is_some() => nominal(&mut out, part),
                    _ => {}
                }
            }
        }
        "S" => {
            // Whose (`1S`), then what it agrees with.
            if let Some(parsing) = rest.first() {
                nominal(&mut out, parsing.get(2..).unwrap_or_default());
            }
        }
        "N" | "A" | "T" | "P" | "R" | "C" | "D" | "F" | "I" | "K" | "Q" | "X" => {
            if let Some(parsing) = rest.first().filter(|p| **p != "NUI") {
                nominal(&mut out, parsing);
            }
        }
        _ => {}
    }
    out
}

/// Tense, voice and mood (`AAI`, with a `2` for a second tense), then
/// person and number (`3S`) or, for a participle, case, number and gender.
fn describe_verb(parsing: &str, rest: &[&str]) -> Vec<String> {
    let (second, tvm) = match parsing.strip_prefix('2') {
        Some(tvm) => (true, tvm),
        None => (false, parsing),
    };
    let mut chars = tvm.chars();
    let tense = chars.next().and_then(tense);
    let voice = chars.next().and_then(voice);
    let mood = chars.next().and_then(mood);
    let mut out = Vec::new();
    match (tense, voice, mood) {
        (Some(t), Some(v), Some(m)) if chars.next().is_none() => {
            let t = if second {
                format!("second {t}")
            } else {
                t.to_string()
            };
            out.push(format!("{t} {v} {m}"));
        }
        _ => {
            out.push(parsing.to_string());
            return out;
        }
    }
    for part in rest {
        if let Some(suffix) = describe_suffix("V", part) {
            out.push(suffix.to_string());
            continue;
        }
        let mut chars = part.chars();
        let first = chars.clone().next();
        match first.and_then(person) {
            Some(p) => match chars.nth(1).and_then(number) {
                Some(n) => out.push(format!("{p} {n}")),
                None => out.push(part.to_string()),
            },
            None if part.len() == 3 && case(first.unwrap_or(' ')).is_some() => {
                out.extend(describe_nominal(part))
            }
            None => out.push(part.to_string()),
        }
    }
    out
}

/// What a suffix to a code adds: a proper name's kind, a degree of
/// comparison, a word's language.
fn describe_suffix(head: &str, suffix: &str) -> Option<&'static str> {
    Some(match suffix {
        "P" => "a person's name",
        "L" => "a place's name",
        "T" => "a title",
        "PG" => "a people's name",
        "LG" => "naming people from a place",
        "C" if head == "A" || head == "ADV" => "comparative",
        "C" => "contracted",
        "S" => "superlative",
        "N" => "negative",
        "I" => "interrogative",
        "ATT" => "Attic form",
        "ABB" => "abbreviated",
        "LI" => "a letter's name",
        "ARAM" => "Aramaic",
        "HEB" => "Hebrew",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_grammar_codes_into_dimensions() {
        let verb = parse_grammar("V-2AAI-3S");
        assert_eq!(
            (
                verb.part_of_speech.as_str(),
                verb.tense.as_str(),
                verb.voice.as_str(),
                verb.mood.as_str(),
                verb.person.as_str(),
                verb.number.as_str(),
            ),
            (
                "verb",
                "aorist",
                "active",
                "indicative",
                "3rd person",
                "singular"
            )
        );
        assert!(verb.case.is_empty() && verb.gender.is_empty());
        let participle = parse_grammar("V-PAP-NSM");
        assert_eq!(
            (
                participle.mood.as_str(),
                participle.case.as_str(),
                participle.gender.as_str()
            ),
            ("participle", "nominative", "masculine")
        );
        let pronoun = parse_grammar("P-1GS");
        assert_eq!(
            (
                pronoun.part_of_speech.as_str(),
                pronoun.person.as_str(),
                pronoun.case.as_str()
            ),
            ("personal pronoun", "1st person", "genitive")
        );
        let possessive = parse_grammar("S-1SNSM");
        assert_eq!(
            (possessive.case.as_str(), possessive.gender.as_str()),
            ("nominative", "masculine")
        );
        assert!(possessive.person.is_empty());
        assert_eq!(parse_grammar("N-GSM-P").number, "singular");
        assert_eq!(
            parse_grammar("CONJ"),
            GreekParse {
                part_of_speech: "conjunction".into(),
                ..GreekParse::default()
            }
        );
    }

    #[test]
    fn describes_grammar_codes() {
        assert_eq!(
            describe_grammar("V-AAI-3S"),
            "verb, aorist active indicative, 3rd person singular"
        );
        assert_eq!(
            describe_grammar("V-2AAP-NSM"),
            "verb, second aorist active participle, nominative singular masculine"
        );
        assert_eq!(
            describe_grammar("N-GSM-P"),
            "noun, genitive singular masculine, a person's name"
        );
        assert_eq!(
            describe_grammar("T-ASM"),
            "article, accusative singular masculine"
        );
        assert_eq!(
            describe_grammar("P-1GS"),
            "personal pronoun, 1st person, genitive singular"
        );
        assert_eq!(
            describe_grammar("S-1SNSM"),
            "possessive pronoun, 1st person singular possessor, nominative singular masculine"
        );
        assert_eq!(
            describe_grammar("A-NSM-C"),
            "adjective, nominative singular masculine, comparative"
        );
        assert_eq!(describe_grammar("PRT-N"), "particle, negative");
        assert_eq!(describe_grammar("CONJ"), "conjunction");
        assert_eq!(describe_grammar("A-NUI"), "adjective, indeclinable numeral");
        assert_eq!(describe_grammar("ZZZ"), "ZZZ");
    }
}
