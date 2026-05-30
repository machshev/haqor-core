//! Noun inflection from a supplied stem.
//!
//! Unlike verbs, a Hebrew noun's *pattern* (mishqal) can't be derived from
//! its 3-letter root alone — the same root often yields multiple unrelated
//! nouns (כָּתַב → כָּתָב "writing", מִכְתָּב "letter"). So the noun generator
//! takes a fully-pointed singular absolute stem and conjugates it across
//! state (absolute/construct), number (singular/plural/dual), and pronominal
//! suffixes.
//!
//! Stem-vowel reductions inside a noun are governed by the same propretonic
//! reduction rules as verbs: the vowel two syllables before the stress
//! reduces to sheva, the pretonic open syllable stays long.

use super::hebrew::{self, Cons, Vowel, letter};
use super::{Gender, Number, Person};

/// Possessive pronominal suffixes attached to nouns/prepositions.
const PRON_SUFFIXES: &[(Person, Gender, Number)] = &[
    (Person::First, Gender::Common, Number::Singular), // -î
    (Person::Second, Gender::Masculine, Number::Singular), // -ḵā
    (Person::Second, Gender::Feminine, Number::Singular), // -ēḵ
    (Person::Third, Gender::Masculine, Number::Singular), // -ô
    (Person::Third, Gender::Feminine, Number::Singular), // -āh
    (Person::First, Gender::Common, Number::Plural),   // -ēnû
    (Person::Second, Gender::Masculine, Number::Plural), // -ḵem
    (Person::Second, Gender::Feminine, Number::Plural), // -ḵen
    (Person::Third, Gender::Masculine, Number::Plural), // -ām
    (Person::Third, Gender::Feminine, Number::Plural), // -ān
];

/// What kind of noun stem we're inflecting. Most Biblical Hebrew masculine
/// nouns add -îm in the plural and pronominal suffixes attach to the plain
/// stem; feminine nouns ending in -â / -t have their own pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NounStemKind {
    /// Masculine, no overt ending (e.g. דָּבָר, סוּס).
    Masculine,
    /// Feminine ending in qamats-he, e.g. תּוֹרָה.
    FeminineHe,
    /// Feminine ending in -t (segolate), e.g. בַּת.
    FeminineT,
}

/// A noun stem supplied by the caller: the singular absolute form parsed
/// into our `Cons` representation, plus its inflection class.
#[derive(Debug, Clone)]
pub struct NounStem {
    pub absolute_singular: Vec<Cons>,
    pub kind: NounStemKind,
}

impl NounStem {
    /// Build a masculine noun stem from a raw Hebrew Unicode string.
    /// The string should be the singular absolute form, fully pointed.
    pub fn masculine(text: &str) -> Self {
        NounStem {
            absolute_singular: hebrew::parse_pointed(text),
            kind: NounStemKind::Masculine,
        }
    }

    pub fn feminine_he(text: &str) -> Self {
        NounStem {
            absolute_singular: hebrew::parse_pointed(text),
            kind: NounStemKind::FeminineHe,
        }
    }
}

/// One inflected noun form.
#[derive(Debug, Clone)]
pub struct NounInflection {
    pub label: String,
    pub text: String,
}

/// Generate the inflectional paradigm of a noun stem.
pub fn inflect_noun(stem: &NounStem) -> Vec<NounInflection> {
    let mut out = vec![
        NounInflection {
            label: "Singular Absolute".to_string(),
            text: hebrew::render(&stem.absolute_singular),
        },
        NounInflection {
            label: "Singular Construct".to_string(),
            text: hebrew::render(&singular_construct(stem)),
        },
        NounInflection {
            label: "Plural Absolute".to_string(),
            text: hebrew::render(&plural_absolute(stem)),
        },
        NounInflection {
            label: "Plural Construct".to_string(),
            text: hebrew::render(&plural_construct(stem)),
        },
    ];
    if matches!(stem.kind, NounStemKind::Masculine) {
        // Dual mostly survives in body parts & paired items (יָדַיִם, רַגְלַיִם).
        out.push(NounInflection {
            label: "Dual Absolute".to_string(),
            text: hebrew::render(&dual_absolute(stem)),
        });
    }

    // Pronominal suffixes (singular base).
    for &(p, g, n) in PRON_SUFFIXES {
        let label = format!(
            "Sg + {}{}{}",
            pgn_letters(p, g, n).0,
            pgn_letters(p, g, n).1,
            pgn_letters(p, g, n).2
        );
        out.push(NounInflection {
            label,
            text: hebrew::render(&with_pron_suffix(stem, false, p, g, n)),
        });
    }
    // Pronominal suffixes (plural base).
    for &(p, g, n) in PRON_SUFFIXES {
        let label = format!(
            "Pl + {}{}{}",
            pgn_letters(p, g, n).0,
            pgn_letters(p, g, n).1,
            pgn_letters(p, g, n).2
        );
        out.push(NounInflection {
            label,
            text: hebrew::render(&with_pron_suffix(stem, true, p, g, n)),
        });
    }
    out
}

fn pgn_letters(p: Person, g: Gender, n: Number) -> (&'static str, &'static str, &'static str) {
    let p = match p {
        Person::First => "1",
        Person::Second => "2",
        Person::Third => "3",
    };
    let g = match g {
        Gender::Masculine => "m",
        Gender::Feminine => "f",
        Gender::Common => "c",
    };
    let n = match n {
        Number::Singular => "s",
        Number::Plural => "p",
        Number::Dual => "d",
    };
    (p, g, n)
}

fn singular_construct(stem: &NounStem) -> Vec<Cons> {
    use Vowel::*;
    // Masculine: construct = absolute, with possible vowel reduction in the
    // first syllable (e.g. דָּבָר → דְּבַר). We apply propretonic reduction:
    // the first vowel (if qamats/tsere in an open syllable) → sheva, and
    // the pretonic open syllable's qamats becomes patah.
    match stem.kind {
        NounStemKind::Masculine => {
            let mut out = stem.absolute_singular.clone();
            reduce_construct_masculine(&mut out);
            out
        }
        NounStemKind::FeminineHe => {
            // Replace final he with tav: tôrâ → tôraṯ.
            let mut out = stem.absolute_singular.clone();
            if let Some(last) = out.last()
                && last.letter == letter::HE
            {
                out.pop();
                if let Some(prev) = out.last_mut() {
                    prev.vowel = Some(Patah);
                }
                out.push(Cons::new(letter::TAV));
            }
            out
        }
        NounStemKind::FeminineT => stem.absolute_singular.clone(),
    }
}

/// Construct-state reduction: both propretonic v1 and pretonic v2 shorten
/// (דָּבָר → דְּבַר).
fn reduce_construct_masculine(seq: &mut [Cons]) {
    use Vowel::*;
    reduce_propretonic(seq);
    if seq.len() >= 2
        && let Some(v) = seq[1].vowel
        && v == Qamats
    {
        seq[1].vowel = Some(Patah);
    }
}

/// Plural-absolute / suffixed reduction: only the propretonic v1 reduces;
/// v2 stays long because the new stress falls on the suffix (דָּבָר →
/// דְּבָרִים).
fn reduce_propretonic(seq: &mut [Cons]) {
    use Vowel::*;
    if seq.len() >= 3
        && let Some(v) = seq[0].vowel
        && matches!(v, Qamats | Tsere)
    {
        seq[0].vowel = Some(Sheva);
    }
}

fn plural_absolute(stem: &NounStem) -> Vec<Cons> {
    use Vowel::*;
    let mut out = stem.absolute_singular.clone();
    match stem.kind {
        NounStemKind::Masculine => {
            // Reduce first vowel propretonically, then add -îm.
            reduce_propretonic(&mut out);
            // Last consonant gets hiriq, then yod (mater), then mem.
            if let Some(last) = out.last_mut() {
                last.vowel = Some(Hiriq);
            }
            out.push(Cons::new(letter::YOD));
            out.push(Cons::new(letter::MEM));
        }
        NounStemKind::FeminineHe => {
            // Drop the final he and add -ôt.
            if let Some(last) = out.last()
                && last.letter == letter::HE
            {
                out.pop();
            }
            if let Some(last) = out.last_mut() {
                last.vowel = Some(Holam);
            }
            out.push(Cons::new(letter::TAV));
        }
        NounStemKind::FeminineT => {
            if let Some(last) = out.last_mut() {
                last.vowel = Some(Holam);
            }
            out.push(Cons::new(letter::TAV));
        }
    }
    out
}

fn plural_construct(stem: &NounStem) -> Vec<Cons> {
    use Vowel::*;
    match stem.kind {
        NounStemKind::Masculine => {
            // -ê: tsere + yod, no mem. Both vowels reduce (dīḇrê-style; we
            // simplify to dəḇərê-style propretonic-only reduction here).
            let mut out = stem.absolute_singular.clone();
            reduce_propretonic(&mut out);
            if let Some(last) = out.last_mut() {
                last.vowel = Some(Tsere);
            }
            out.push(Cons::new(letter::YOD));
            out
        }
        NounStemKind::FeminineHe | NounStemKind::FeminineT => {
            // -ôt — same as plural absolute for feminines.
            plural_absolute(stem)
        }
    }
}

fn dual_absolute(stem: &NounStem) -> Vec<Cons> {
    use Vowel::*;
    let mut out = stem.absolute_singular.clone();
    // Dual ending -ayim: patah, yod, hiriq, mem.
    if let Some(last) = out.last_mut() {
        last.vowel = Some(Patah);
    }
    out.push(Cons::new(letter::YOD).with_vowel(Hiriq));
    out.push(Cons::new(letter::MEM));
    out
}

/// Attach a pronominal suffix to the noun stem.
///
/// `plural` selects the plural-base set (-ay, -êḵā, …): with plural nouns
/// the suffix attaches to a stem ending in -ê + yod rather than the bare
/// singular stem. The whole list of plural-base endings is:
///   1cs -ay, 2ms -êḵā, 2fs -ayiḵ, 3ms -āw, 3fs -êhā, 1cp -ênû,
///   2mp -êḵem, 2fp -êḵen, 3mp -êhem, 3fp -êhen.
fn with_pron_suffix(stem: &NounStem, plural: bool, p: Person, g: Gender, n: Number) -> Vec<Cons> {
    use Vowel::*;
    let mut out = stem.absolute_singular.clone();

    // For feminine -â stems, restore the -t connector before any suffix.
    if matches!(stem.kind, NounStemKind::FeminineHe) {
        if let Some(last) = out.last()
            && last.letter == letter::HE
        {
            out.pop();
            if let Some(prev) = out.last_mut() {
                prev.vowel = Some(Qamats);
            }
            out.push(Cons::new(letter::TAV));
        }
    } else {
        // Propretonic reduction; pretonic stays long because the suffix
        // carries the stress.
        reduce_propretonic(&mut out);
    }

    // The last `Cons` of `out` is the final radical of the noun. Its vowel
    // varies by which suffix follows. We set both that vowel and the suffix
    // consonants in one match.
    let set_last_vowel = |out: &mut Vec<Cons>, v: Vowel| {
        if let Some(last) = out.last_mut() {
            last.vowel = Some(v);
        }
    };

    if plural {
        match (p, g, n) {
            (Person::First, _, Number::Singular) => {
                // -ay: patah on final radical, then bare yod.
                set_last_vowel(&mut out, Patah);
                out.push(Cons::new(letter::YOD));
            }
            (Person::Second, Gender::Masculine, Number::Singular) => {
                // -êḵā
                set_last_vowel(&mut out, Tsere);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::KAF).with_vowel(Qamats));
            }
            (Person::Second, Gender::Feminine, Number::Singular) => {
                // -ayiḵ
                set_last_vowel(&mut out, Patah);
                out.push(Cons::new(letter::YOD).with_vowel(Hiriq));
                out.push(Cons::new(letter::KAF));
            }
            (Person::Third, Gender::Masculine, Number::Singular) => {
                // -āw: final radical takes qamats, then yod-vav.
                set_last_vowel(&mut out, Qamats);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::VAV));
            }
            (Person::Third, Gender::Feminine, Number::Singular) => {
                // -êhā
                set_last_vowel(&mut out, Tsere);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::HE).with_vowel(Qamats));
            }
            (Person::First, _, Number::Plural) => {
                // -ênû: tsere on final radical, yod, nun, shureq vav.
                set_last_vowel(&mut out, Tsere);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::NUN));
                out.push(Cons::new(letter::VAV).with_dagesh());
            }
            (Person::Second, Gender::Masculine, Number::Plural) => {
                // -êḵem
                set_last_vowel(&mut out, Tsere);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::KAF).with_vowel(Segol));
                out.push(Cons::new(letter::MEM));
            }
            (Person::Second, Gender::Feminine, Number::Plural) => {
                // -êḵen
                set_last_vowel(&mut out, Tsere);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::KAF).with_vowel(Segol));
                out.push(Cons::new(letter::NUN));
            }
            (Person::Third, Gender::Masculine, Number::Plural) => {
                // -êhem
                set_last_vowel(&mut out, Tsere);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::HE).with_vowel(Segol));
                out.push(Cons::new(letter::MEM));
            }
            (Person::Third, Gender::Feminine, Number::Plural) => {
                // -êhen
                set_last_vowel(&mut out, Tsere);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::HE).with_vowel(Segol));
                out.push(Cons::new(letter::NUN));
            }
            _ => {}
        }
    } else {
        match (p, g, n) {
            (Person::First, _, Number::Singular) => {
                // -î
                set_last_vowel(&mut out, Hiriq);
                out.push(Cons::new(letter::YOD));
            }
            (Person::Second, Gender::Masculine, Number::Singular) => {
                // -ḵā
                set_last_vowel(&mut out, Sheva);
                out.push(Cons::new(letter::KAF).with_vowel(Qamats));
            }
            (Person::Second, Gender::Feminine, Number::Singular) => {
                // -ēḵ
                set_last_vowel(&mut out, Tsere);
                out.push(Cons::new(letter::KAF));
            }
            (Person::Third, Gender::Masculine, Number::Singular) => {
                // -ô (defective; could also be holam+vav)
                set_last_vowel(&mut out, Holam);
                out.push(Cons::new(letter::VAV));
            }
            (Person::Third, Gender::Feminine, Number::Singular) => {
                // -āh (with mappiq)
                set_last_vowel(&mut out, Qamats);
                out.push(Cons::new(letter::HE).with_dagesh());
            }
            (Person::First, _, Number::Plural) => {
                // -ēnû
                set_last_vowel(&mut out, Tsere);
                out.push(Cons::new(letter::NUN));
                out.push(Cons::new(letter::VAV).with_dagesh());
            }
            (Person::Second, Gender::Masculine, Number::Plural) => {
                // -ḵem
                set_last_vowel(&mut out, Sheva);
                out.push(Cons::new(letter::KAF).with_vowel(Segol));
                out.push(Cons::new(letter::MEM));
            }
            (Person::Second, Gender::Feminine, Number::Plural) => {
                set_last_vowel(&mut out, Sheva);
                out.push(Cons::new(letter::KAF).with_vowel(Segol));
                out.push(Cons::new(letter::NUN));
            }
            (Person::Third, Gender::Masculine, Number::Plural) => {
                // -ām
                set_last_vowel(&mut out, Qamats);
                out.push(Cons::new(letter::MEM));
            }
            (Person::Third, Gender::Feminine, Number::Plural) => {
                set_last_vowel(&mut out, Qamats);
                out.push(Cons::new(letter::NUN));
            }
            _ => {}
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pointed_word() {
        // דָּבָר
        let seq = hebrew::parse_pointed("דָּבָר");
        // dalet (with dagesh + qamats), bet (with qamats), resh
        assert_eq!(seq.len(), 3);
        assert!(seq[0].dagesh);
        assert_eq!(seq[0].vowel, Some(Vowel::Qamats));
    }

    #[test]
    fn inflect_dāḇār() {
        let stem = NounStem::masculine("דָּבָר");
        let forms = inflect_noun(&stem);
        // Plural absolute should be דְּבָרִים — dalet+dagesh+sheva, bet+qamats,
        // resh+hiriq, yod, mem (traditional combining order).
        let pl = forms.iter().find(|f| f.label == "Plural Absolute").unwrap();
        assert_eq!(
            pl.text,
            "\u{05D3}\u{05BC}\u{05B0}\u{05D1}\u{05B8}\u{05E8}\u{05B4}\u{05D9}\u{05DD}",
        );
    }
}
