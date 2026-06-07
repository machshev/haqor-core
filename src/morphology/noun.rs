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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NounStemKind {
    /// Masculine, no overt ending (e.g. דָּבָר, סוּס).
    Masculine,
    /// Feminine ending in qamats-he, e.g. תּוֹרָה.
    FeminineHe,
    /// Feminine ending in -t (segolate), e.g. בַּת.
    FeminineT,
    /// Segolate: a historically monosyllabic CVCC base (malk-, sipr-, qudš-)
    /// surfacing with a penultimate-stress helping segol (מֶלֶךְ, סֵפֶר, קֹדֶשׁ).
    /// The original base vowel is read back off the first syllable: segol/patah
    /// → a-class, tsere → i-class, holam/qamats-qatan → u-class.
    Segolate,
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

    pub fn feminine_t(text: &str) -> Self {
        NounStem {
            absolute_singular: hebrew::parse_pointed(text),
            kind: NounStemKind::FeminineT,
        }
    }

    /// Build a segolate stem from its singular absolute form (e.g. מֶלֶךְ). The
    /// base vowel class is recovered from the pointing at inflection time.
    pub fn segolate(text: &str) -> Self {
        NounStem {
            absolute_singular: hebrew::parse_pointed(text),
            kind: NounStemKind::Segolate,
        }
    }

    /// Build a stem from a pointed singular-absolute headword, guessing the
    /// inflection class from the spelling: a final qamats-he is the feminine -â
    /// pattern; a three-radical penultimate-stress shape is segolate; anything
    /// else is treated as a sound masculine. The guess only affects which
    /// inflected forms are generated — a wrong guess loses recall but, because
    /// downstream matching is exact, never produces a spurious analysis.
    pub fn classify(text: &str) -> Self {
        let seq = hebrew::parse_pointed(text);
        NounStem {
            kind: classify_kind(&seq),
            absolute_singular: seq,
        }
    }

    /// For a segolate stem, the same stem with its first vowel forced to each of
    /// the three base classes — a (segol), i (tsere), u (holam). The segolate
    /// base vowel can't be read off the absolute pointing (קֶרֶב looks a-class
    /// but is i-class: קִרְבְּךָ), so we emit all three paradigms; exact-match
    /// downstream keeps only the spellings that actually occur, so the wrong two
    /// classes cost nothing. Non-segolates return just themselves.
    pub fn class_variants(self) -> Vec<NounStem> {
        use Vowel::*;
        if self.kind != NounStemKind::Segolate || self.absolute_singular.is_empty() {
            return vec![self];
        }
        // a-class surfaces as segol, or patah next to a guttural (naʕar נַעַר,
        // paḥad פַּחַד); i-class as tsere; u-class as holam. Emit all four so the
        // real one matches and a guttural a-class isn't lost.
        [Segol, Patah, Tsere, Holam]
            .into_iter()
            .map(|v| {
                let mut s = self.clone();
                s.absolute_singular[0].vowel = Some(v);
                s
            })
            .collect()
    }
}

/// Guess a stem class from its pointed singular-absolute `Cons` sequence.
fn classify_kind(seq: &[Cons]) -> NounStemKind {
    use Vowel::*;
    // Feminine -â: qamats on the penult, a bare he at the end.
    if let [.., prev, last] = seq
        && last.letter == letter::HE
        && last.vowel.is_none()
        && prev.vowel == Some(Qamats)
    {
        return NounStemKind::FeminineHe;
    }
    // Segolate: exactly three radicals, a short vowel under the first and a
    // helping segol/patah under the second, the third bare.
    if let [c1, c2, c3] = seq {
        let v1_ok = matches!(c1.vowel, Some(Segol | Tsere | Holam | Patah | QamatsQatan));
        let v2_ok = matches!(c2.vowel, Some(Segol | Patah | HatafPatah | HatafSegol));
        // The final radical is bare, or carries the orthographic silent sheva
        // that final letters (esp. final kaf, מֶלֶךְ / דֶּרֶךְ) conventionally take.
        let v3_ok = matches!(c3.vowel, None | Some(Sheva));
        if v1_ok && v2_ok && v3_ok {
            return NounStemKind::Segolate;
        }
    }
    NounStemKind::Masculine
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
        // Archaic 3ms suffix -ēhû (e.g. לְמִינֵהוּ).
        if (p, g, n) == (Person::Third, Gender::Masculine, Number::Singular) {
            let mut base = match stem.kind {
                NounStemKind::Segolate => segolate_singular_base(stem),
                NounStemKind::FeminineHe => {
                    let mut out = stem.absolute_singular.clone();
                    if let Some(last) = out.last()
                        && last.letter == letter::HE
                    {
                        out.pop();
                        if let Some(prev) = out.last_mut() {
                            prev.vowel = Some(Vowel::Qamats);
                        }
                        out.push(Cons::new(letter::TAV));
                    }
                    out
                }
                _ => {
                    let mut out = stem.absolute_singular.clone();
                    reduce_heavy_masculine(&mut out);
                    out
                }
            };
            if let Some(last) = base.last_mut() {
                last.vowel = Some(Vowel::Tsere);
            }
            base.push(Cons::new(letter::HE));
            base.push(Cons::new(letter::VAV).with_dagesh());
            out.push(NounInflection {
                label: "Sg + 3ms (archaic)".to_string(),
                text: hebrew::render(&base),
            });
        }
    }
    // Geminate-origin monosyllabic nouns (בַּד, כֵּן, עֵת, חֹק, כֹּל…) double their
    // final consonant before a pronominal suffix and shorten the stem vowel to
    // its short a/i/u counterpart: baddô בַּדּוֹ, kannô כַּנּוֹ, ʕittô עִתּוֹ,
    // ḥuqqô חֻקּוֹ, lᵊḇaddᵊḵā. The base-vowel class is lexical and we don't mark
    // it, so emit all three short-vowel variants as generate-and-test alternants;
    // only the attested one matches a surface, the rest match nothing. Restricted
    // to a true CvC monosyllable whose final radical can take a forte dagesh
    // (excludes the gutturals and resh).
    if stem.kind == NounStemKind::Masculine
        && let [c1, c2] = stem.absolute_singular.as_slice()
        && c1.vowel.is_some()
        && c2.vowel.is_none()
        && !c2.dagesh
        && !hebrew::is_guttural(c2.letter)
        && c2.letter != letter::RESH
    {
        for short in [Vowel::Patah, Vowel::Hiriq, Vowel::Qubuts] {
            let gem_base = |connector: Option<Vowel>| {
                let mut c1c = c1.clone();
                c1c.vowel = Some(short);
                let mut c2c = c2.clone();
                c2c.dagesh = true;
                c2c.vowel = connector;
                vec![c1c, c2c]
            };
            for &(p, g, n) in PRON_SUFFIXES {
                let mut base = gem_base(None);
                append_pron_suffix(&mut base, false, p, g, n);
                out.push(NounInflection {
                    label: format!(
                        "Sg + {}{}{} (geminate)",
                        pgn_letters(p, g, n).0,
                        pgn_letters(p, g, n).1,
                        pgn_letters(p, g, n).2
                    ),
                    text: hebrew::render(&base),
                });
            }
            // The heavy 2nd-person suffixes take a segol connector on the doubled
            // radical rather than the bare sheva: lᵊḇaddᵊḵā beside lᵊḇaddɛḵā
            // (לְבַדֶּךָ), baddᵊḵem beside baddɛḵem.
            let mut k2ms = gem_base(Some(Vowel::Segol));
            k2ms.push(Cons::new(letter::KAF).with_vowel(Vowel::Qamats));
            out.push(NounInflection {
                label: "Sg + 2ms (geminate)".to_string(),
                text: hebrew::render(&k2ms),
            });
            let mut k2mp = gem_base(Some(Vowel::Segol));
            k2mp.push(Cons::new(letter::KAF).with_vowel(Vowel::Segol));
            k2mp.push(Cons::new(letter::MEM));
            out.push(NounInflection {
                label: "Sg + 2mp (geminate)".to_string(),
                text: hebrew::render(&k2mp),
            });
        }
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
    // Feminine -ôt plurals keep the -ōt marker and attach the suffix after the
    // tav. Both ending sets occur: the plural set (מִצְותָיו -āw, מִצְותֶיךָ) and
    // the singular set fused onto -ōt (מִשְׁפְּחֹתָם -ām, לְצִבְאֹתָם). Emit both.
    if matches!(stem.kind, NounStemKind::FeminineHe | NounStemKind::FeminineT) {
        for &(p, g, n) in PRON_SUFFIXES {
            let label = format!(
                "Pl + {}{}{}",
                pgn_letters(p, g, n).0,
                pgn_letters(p, g, n).1,
                pgn_letters(p, g, n).2
            );
            for use_plural_set in [true, false] {
                let mut base = feminine_plural_suffix_base(stem);
                append_pron_suffix(&mut base, use_plural_set, p, g, n);
                out.push(NounInflection {
                    label: label.clone(),
                    text: hebrew::render(&base),
                });
            }
        }
    }

    // Segolates commonly take a segol "helping" vowel before the 2nd-person
    // suffixes instead of the silent sheva of the contracted base — עַבְדֶּךָ
    // beside עַבְדְּךָ, נַפְשֶׁךָ, חַסְדֶּךָ. Emit those as extra singular-base
    // forms (the sheva variants are already produced by the loop above).
    if stem.kind == NounStemKind::Segolate {
        use Vowel::*;
        let variants: [(&str, Gender, Number, &[(char, Option<Vowel>)]); 3] = [
            ("Sg + 2ms", Gender::Masculine, Number::Singular, &[(letter::KAF, Some(Qamats))]),
            ("Sg + 2mp", Gender::Masculine, Number::Plural, &[(letter::KAF, Some(Segol)), (letter::MEM, None)]),
            ("Sg + 2fp", Gender::Feminine, Number::Plural, &[(letter::KAF, Some(Segol)), (letter::NUN, None)]),
        ];
        for (label, _g, _n, suffix) in variants {
            let mut base = segolate_singular_base(stem);
            if let Some(last) = base.last_mut() {
                last.vowel = Some(Segol);
            }
            for &(ltr, v) in suffix {
                let mut c = Cons::new(ltr);
                c.vowel = v;
                base.push(c);
            }
            out.push(NounInflection {
                label: label.to_string(),
                text: hebrew::render(&base),
            });
        }
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
        // Segolate construct singular = absolute (מֶלֶךְ → מֶלֶךְ).
        NounStemKind::Segolate => stem.absolute_singular.clone(),
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

/// Reduction before a stress-bearing ending (plural -îm/-ê or a pronominal
/// suffix), for the masculine/sound pattern. Normally the propretonic v1
/// reduces (דָּבָר → דְּבָרִים). But when v1 is an **unchangeable holam** — the
/// qōṭēl / active-participle noun pattern ʔōyēḇ, šōp̄ēṭ, kōhēn — v1 stays and the
/// pretonic v2 (tsere/qamats) reduces instead: ʔōyēḇ → ʔōyᵊḇîm / ʔōyᵊḇāw
/// (אֹיֵב → אֹיְבִים / אֹיְבָיו), šōp̄ēṭ → šōp̄ᵊṭîm. Only for the heavy endings, so
/// the singular construct (which keeps the tsere: שֹׁפֵט) is unaffected.
fn reduce_heavy_masculine(seq: &mut [Cons]) {
    use Vowel::*;
    if seq.len() < 3 {
        return;
    }
    match seq[0].vowel {
        Some(Qamats) | Some(Tsere) => seq[0].vowel = Some(Sheva),
        Some(Holam) if matches!(seq[1].vowel, Some(Tsere) | Some(Qamats)) => {
            seq[1].vowel = Some(Sheva);
        }
        _ => {}
    }
}

fn plural_absolute(stem: &NounStem) -> Vec<Cons> {
    use Vowel::*;
    let mut out = stem.absolute_singular.clone();
    match stem.kind {
        NounStemKind::Masculine => {
            // Reduce before the heavy -îm ending, then add it.
            reduce_heavy_masculine(&mut out);
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
        // Segolate plural absolute is the qətālîm pattern for every base class:
        // sheva (hataf-qamats for u-class), qamats, then -îm (מְלָכִים, קֳדָשִׁים).
        NounStemKind::Segolate => return segolate_plural_absolute(stem),
    }
    out
}

fn plural_construct(stem: &NounStem) -> Vec<Cons> {
    use Vowel::*;
    match stem.kind {
        NounStemKind::Segolate => segolate_plural_construct(stem),
        NounStemKind::Masculine => {
            // -ê: tsere + yod, no mem. Both vowels reduce (dīḇrê-style; we
            // simplify to dəḇərê-style propretonic-only reduction here).
            let mut out = stem.absolute_singular.clone();
            reduce_heavy_masculine(&mut out);
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

/// Suffix base for a feminine -ôt plural noun: the -ōt marker is retained and
/// the pronominal suffix attaches after the tav (מִשְׁפְּחֹתָם, מִצְותָיו). The
/// pretonic vowel before the holam reduces, since the stress shifts to the
/// suffix. The tav is left vowelless for the suffix ending to set.
fn feminine_plural_suffix_base(stem: &NounStem) -> Vec<Cons> {
    use Vowel::*;
    let mut base = plural_absolute(stem); // … ō + tav
    let n = base.len();
    // Reduce the pretonic vowel: the consonant before the holam-bearing one
    // (which is two before the final tav). Keep the very first radical intact;
    // reduce_propretonic handles that slot.
    if n >= 3
        && let Some(i) = n.checked_sub(3)
        && i >= 1
        && matches!(base[i].vowel, Some(Qamats) | Some(Tsere))
    {
        set_seg_vowel(&mut base[i], Sheva);
    }
    reduce_propretonic(&mut base);
    if let Some(last) = base.last_mut() {
        last.vowel = None;
    }
    base
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
    // The base is everything up to (but not including) the final radical's
    // suffix vowel; the per-suffix match below sets that vowel and appends the
    // suffix consonants. Each stem class builds its base differently.
    let mut out = match stem.kind {
        // Segolates restore their original CVCC base under the suffix: the
        // singular base (malk-, malkî) restores the historic vowel and silent
        // sheva, the plural base (məlāk-, məlākay) is the qətāl- shape. The heavy
        // plural suffixes (-ḵem/-ḵen/-hem/-hen) attach instead to the plural
        // *construct* base (ʔaḏnê- → אַדְנֵיהֶם, niskê- → נִסְכֵּיהֶם), not the
        // longer absolute base the light suffixes use (ʔăḏānɛ́ḵā).
        NounStemKind::Segolate => {
            return {
                let mut base = if !plural {
                    segolate_singular_base(stem)
                } else if is_heavy_suffix(p, n) {
                    let mut b = segolate_plural_construct(stem); // …ê + yod mater
                    b.pop(); // drop the yod; append_pron_suffix re-sets the ending
                    if let Some(last) = b.last_mut() {
                        last.vowel = None;
                    }
                    b
                } else {
                    segolate_plural_base(stem)
                };
                append_pron_suffix(&mut base, plural, p, g, n);
                base
            };
        }
        // For feminine -â stems, restore the -t connector before any suffix.
        NounStemKind::FeminineHe => {
            let mut out = stem.absolute_singular.clone();
            if let Some(last) = out.last()
                && last.letter == letter::HE
            {
                out.pop();
                if let Some(prev) = out.last_mut() {
                    prev.vowel = Some(Qamats);
                }
                out.push(Cons::new(letter::TAV));
            }
            out
        }
        _ => {
            // Reduction before the suffix (which carries the stress): v1
            // propretonic, or the qōṭēl v2 when v1 is an unchangeable holam.
            let mut out = stem.absolute_singular.clone();
            reduce_heavy_masculine(&mut out);
            out
        }
    };

    append_pron_suffix(&mut out, plural, p, g, n);
    out
}

/// The "heavy" plural pronominal suffixes — 2mp -ḵem, 2fp -ḵen, 3mp -hem,
/// 3fp -hen — whose extra consonant shifts stress so they attach to the
/// construct base rather than the longer light-suffix base.
fn is_heavy_suffix(p: Person, n: Number) -> bool {
    n == Number::Plural && matches!(p, Person::Second | Person::Third)
}

/// Attach the pronominal-suffix vowel + consonants to `out`, whose last `Cons`
/// is the noun's final radical (or the -t connector for feminine -â stems). The
/// suffix endings are shared across stem classes; only the base preceding them
/// differs (built by the caller).
fn append_pron_suffix(out: &mut Vec<Cons>, plural: bool, p: Person, g: Gender, n: Number) {
    use Vowel::*;
    // The final radical's vowel varies by which suffix follows; we set that
    // vowel and the suffix consonants together in one match.
    let set_last_vowel = |out: &mut Vec<Cons>, v: Vowel| {
        if let Some(last) = out.last_mut() {
            last.vowel = Some(v);
        }
    };

    if plural {
        match (p, g, n) {
            (Person::First, _, Number::Singular) => {
                // -ay: patah on final radical, then bare yod.
                set_last_vowel(out, Patah);
                out.push(Cons::new(letter::YOD));
            }
            (Person::Second, Gender::Masculine, Number::Singular) => {
                // -ɛ́ḵā: the plural "your (ms)" suffix is segol-yod, not tsere
                // (דְּבָרֶיךָ, עֵינֶיךָ) — tsere appears only on the heavier
                // -ênû/-êḵem/-êhem suffixes below.
                set_last_vowel(out, Segol);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::KAF).with_vowel(Qamats));
            }
            (Person::Second, Gender::Feminine, Number::Singular) => {
                // -ayiḵ
                set_last_vowel(out, Patah);
                out.push(Cons::new(letter::YOD).with_vowel(Hiriq));
                out.push(Cons::new(letter::KAF));
            }
            (Person::Third, Gender::Masculine, Number::Singular) => {
                // -āw: final radical takes qamats, then yod-vav.
                set_last_vowel(out, Qamats);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::VAV));
            }
            (Person::Third, Gender::Feminine, Number::Singular) => {
                // -ɛ́hā: like the 2ms above, the plural "her" suffix takes segol-yod
                // (סוּסֶיהָ, מִגְרָשֶׁיהָ), not tsere.
                set_last_vowel(out, Segol);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::HE).with_vowel(Qamats));
            }
            (Person::First, _, Number::Plural) => {
                // -ênû: tsere on final radical, yod, nun, shureq vav.
                set_last_vowel(out, Tsere);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::NUN));
                out.push(Cons::new(letter::VAV).with_dagesh());
            }
            (Person::Second, Gender::Masculine, Number::Plural) => {
                // -êḵem
                set_last_vowel(out, Tsere);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::KAF).with_vowel(Segol));
                out.push(Cons::new(letter::MEM));
            }
            (Person::Second, Gender::Feminine, Number::Plural) => {
                // -êḵen
                set_last_vowel(out, Tsere);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::KAF).with_vowel(Segol));
                out.push(Cons::new(letter::NUN));
            }
            (Person::Third, Gender::Masculine, Number::Plural) => {
                // -êhem
                set_last_vowel(out, Tsere);
                out.push(Cons::new(letter::YOD));
                out.push(Cons::new(letter::HE).with_vowel(Segol));
                out.push(Cons::new(letter::MEM));
            }
            (Person::Third, Gender::Feminine, Number::Plural) => {
                // -êhen
                set_last_vowel(out, Tsere);
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
                set_last_vowel(out, Hiriq);
                out.push(Cons::new(letter::YOD));
            }
            (Person::Second, Gender::Masculine, Number::Singular) => {
                // -ḵā
                set_last_vowel(out, Sheva);
                out.push(Cons::new(letter::KAF).with_vowel(Qamats));
            }
            (Person::Second, Gender::Feminine, Number::Singular) => {
                // -ēḵ
                set_last_vowel(out, Tsere);
                out.push(Cons::new(letter::KAF));
            }
            (Person::Third, Gender::Masculine, Number::Singular) => {
                // -ô written holam-male: the suffix vowel sits on the mater vav
                // and the preceding radical stays bare (סוּסוֹ, נַפְשׁוֹ, דְּבָרוֹ).
                out.push(Cons::new(letter::VAV).with_vowel(Holam));
            }
            (Person::Third, Gender::Feminine, Number::Singular) => {
                // -āh (with mappiq)
                set_last_vowel(out, Qamats);
                out.push(Cons::new(letter::HE).with_dagesh());
            }
            (Person::First, _, Number::Plural) => {
                // -ēnû
                set_last_vowel(out, Tsere);
                out.push(Cons::new(letter::NUN));
                out.push(Cons::new(letter::VAV).with_dagesh());
            }
            (Person::Second, Gender::Masculine, Number::Plural) => {
                // -ḵem
                set_last_vowel(out, Sheva);
                out.push(Cons::new(letter::KAF).with_vowel(Segol));
                out.push(Cons::new(letter::MEM));
            }
            (Person::Second, Gender::Feminine, Number::Plural) => {
                set_last_vowel(out, Sheva);
                out.push(Cons::new(letter::KAF).with_vowel(Segol));
                out.push(Cons::new(letter::NUN));
            }
            (Person::Third, Gender::Masculine, Number::Plural) => {
                // -ām
                set_last_vowel(out, Qamats);
                out.push(Cons::new(letter::MEM));
            }
            (Person::Third, Gender::Feminine, Number::Plural) => {
                set_last_vowel(out, Qamats);
                out.push(Cons::new(letter::NUN));
            }
            _ => {}
        }
    }
}

/// A segolate's underlying base-vowel class.
#[derive(Clone, Copy, PartialEq)]
enum SegClass {
    A,
    I,
    U,
}

/// Recover a segolate's base-vowel class from the pointing of its first
/// syllable: tsere → i, holam/qamats-qatan → u, otherwise a.
fn seg_class(stem: &NounStem) -> SegClass {
    match stem.absolute_singular.first().and_then(|c| c.vowel) {
        Some(Vowel::Tsere) => SegClass::I,
        Some(Vowel::Holam) | Some(Vowel::QamatsQatan) => SegClass::U,
        _ => SegClass::A,
    }
}

/// The short vowel restored under the first radical of the original closed base
/// (malk-/sipr-/qudš-): a → patah, i → hiriq, u → qamats-qatan.
fn seg_restored_v1(class: SegClass) -> Vowel {
    match class {
        SegClass::A => Vowel::Patah,
        SegClass::I => Vowel::Hiriq,
        SegClass::U => Vowel::QamatsQatan,
    }
}

/// בגדכפת letters take a dagesh lene when they begin a syllable after a silent
/// sheva (e.g. the kaf of מַלְכִּי / מַלְכֵי).
fn is_begedkefet(c: char) -> bool {
    matches!(
        c,
        letter::BET | letter::GIMEL | letter::DALET | letter::KAF | letter::PE | letter::TAV
    )
}

/// Set a radical's vowel, swapping a plain sheva for the matching hataf when the
/// radical is a guttural that can't carry a vocal sheva.
fn set_seg_vowel(c: &mut Cons, v: Vowel) {
    c.vowel = Some(if hebrew::is_guttural(c.letter) {
        v.guttural_sheva()
    } else {
        v
    });
}

/// Segolate plural absolute: the qətālîm shape for every base class
/// (מְלָכִים, סְפָרִים, קֳדָשִׁים).
fn segolate_plural_absolute(stem: &NounStem) -> Vec<Cons> {
    use Vowel::*;
    let class = seg_class(stem);
    let mut r = stem.absolute_singular.clone();
    if r.len() < 3 {
        return r;
    }
    r.truncate(3);
    let v1 = match class {
        SegClass::U => HatafQamats,
        _ => Sheva,
    };
    set_seg_vowel(&mut r[0], v1);
    r[1].vowel = Some(Qamats);
    r[1].dagesh = false;
    r[2].vowel = Some(Hiriq);
    r[2].dagesh = false;
    r.push(Cons::new(letter::YOD));
    r.push(Cons::new(letter::MEM));
    r
}

/// Segolate plural construct: original base vowel under the first radical, a
/// silent sheva under the second, then -ê (מַלְכֵי, סִפְרֵי).
fn segolate_plural_construct(stem: &NounStem) -> Vec<Cons> {
    use Vowel::*;
    let class = seg_class(stem);
    let mut r = stem.absolute_singular.clone();
    if r.len() < 3 {
        return r;
    }
    r.truncate(3);
    r[0].vowel = Some(seg_restored_v1(class));
    set_seg_vowel(&mut r[1], Sheva);
    r[1].dagesh = false;
    r[2].vowel = Some(Tsere);
    r[2].dagesh = is_begedkefet(r[2].letter);
    r.push(Cons::new(letter::YOD));
    r
}

/// Singular suffix base for a segolate: the restored closed base (malk-, sipr-,
/// qodš-) with the final radical's vowel left for the suffix ending to set.
fn segolate_singular_base(stem: &NounStem) -> Vec<Cons> {
    use Vowel::*;
    let class = seg_class(stem);
    let mut r = stem.absolute_singular.clone();
    if r.len() < 3 {
        return r;
    }
    r.truncate(3);
    r[0].vowel = Some(seg_restored_v1(class));
    set_seg_vowel(&mut r[1], Sheva);
    r[1].dagesh = false;
    r[2].dagesh = is_begedkefet(r[2].letter);
    r[2].vowel = None;
    r
}

/// Plural suffix base for a segolate: the qətāl- shape (məlāk-) with the final
/// radical's vowel left for the suffix ending to set (məlākay, məlākênû).
fn segolate_plural_base(stem: &NounStem) -> Vec<Cons> {
    use Vowel::*;
    let class = seg_class(stem);
    let mut r = stem.absolute_singular.clone();
    if r.len() < 3 {
        return r;
    }
    r.truncate(3);
    let v1 = match class {
        SegClass::U => HatafQamats,
        _ => Sheva,
    };
    set_seg_vowel(&mut r[0], v1);
    r[1].vowel = Some(Qamats);
    r[1].dagesh = false;
    r[2].dagesh = false;
    r[2].vowel = None;
    r
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

    fn form_text<'a>(forms: &'a [NounInflection], label: &str) -> &'a str {
        forms
            .iter()
            .find(|f| f.label == label)
            .unwrap_or_else(|| panic!("no form labelled {label}"))
            .text
            .as_str()
    }

    /// Normalise a hand-typed expected string through the same parse→render
    /// round-trip the generator uses, so the comparison is mark-order agnostic.
    fn norm(s: &str) -> String {
        hebrew::render(&hebrew::parse_pointed(s))
    }

    #[test]
    fn inflect_segolate_a_class() {
        // מֶלֶךְ "king" (a-class): malk-.
        let forms = inflect_noun(&NounStem::segolate("מֶלֶךְ"));
        assert_eq!(form_text(&forms, "Singular Absolute"), norm("מֶלֶךְ"));
        assert_eq!(form_text(&forms, "Plural Absolute"), norm("מְלָכִים"));
        assert_eq!(form_text(&forms, "Plural Construct"), norm("מַלְכֵּי"));
        assert_eq!(form_text(&forms, "Sg + 1cs"), norm("מַלְכִּי"));
    }

    #[test]
    fn inflect_segolate_i_class() {
        // סֵפֶר "book" (i-class): sipr-.
        let forms = inflect_noun(&NounStem::segolate("סֵפֶר"));
        assert_eq!(form_text(&forms, "Plural Absolute"), norm("סְפָרִים"));
        assert_eq!(form_text(&forms, "Plural Construct"), norm("סִפְרֵי"));
        assert_eq!(form_text(&forms, "Sg + 1cs"), norm("סִפְרִי"));
    }

    #[test]
    fn inflect_segolate_u_class() {
        // קֹדֶשׁ "holiness" (u-class): qudš-.
        let forms = inflect_noun(&NounStem::segolate("קֹדֶשׁ"));
        assert_eq!(form_text(&forms, "Plural Absolute"), norm("קֳדָשִׁים"));
    }

    #[test]
    fn inflect_qotel_noun() {
        // אֹיֵב "enemy" (qōṭēl pattern: unchangeable holam v1, tsere v2). Under a
        // heavy ending the tsere reduces, not the holam: אֹיְבִים / אֹיְבָיו /
        // אֹיְבֵיכֶם — but the singular construct keeps the tsere.
        let forms = inflect_noun(&NounStem::masculine("אֹיֵב"));
        assert_eq!(form_text(&forms, "Singular Construct"), norm("אֹיֵב"));
        assert_eq!(form_text(&forms, "Plural Absolute"), norm("אֹיְבִים"));
        assert_eq!(form_text(&forms, "Pl + 3ms"), norm("אֹיְבָיו"));
        assert_eq!(form_text(&forms, "Pl + 2mp"), norm("אֹיְבֵיכֶם"));
        assert_eq!(form_text(&forms, "Sg + 3ms"), norm("אֹיְבוֹ"));
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
