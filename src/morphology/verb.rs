//! Verb paradigm generation.
//!
//! Builds the seven-binyan paradigm of a triliteral root. The core idea:
//!
//! 1. `Stem` describes a binyan+form abstractly — what goes around the three
//!    radicals (prefix, vowels v1/v2/v3, doubling, matres lectionis, suffix
//!    skeleton).
//! 2. For each PGN slot we pick the right vowel grade ("vocalic" vs.
//!    "consonantal" suffix — Hebrew verbs systematically reduce in one and
//!    not the other) and the affix consonants.
//! 3. Gizra-specific transformations (`apply_gizra`) then rewrite the
//!    `Vec<Cons>` to fix things like guttural-can't-double, nun-assimilates,
//!    III-He apocope, hollow loses its middle radical, etc.
//! 4. `hebrew::render` flattens the final `Vec<Cons>` to a Unicode string.

use super::hebrew::{self, Cons, Role, Vowel, letter};
use super::root::{Gizra, Root};
use super::{Gender, Number, Person};

const THREE_MS: Pgn = Pgn::new(Person::Third, Gender::Masculine, Number::Singular);

/// Helper: find a radical slot by its 1-/2-/3-position in the root.
fn radical_idx(seq: &[Cons], n: u8) -> Option<usize> {
    seq.iter().position(|c| c.role == Role::Radical(n))
}

/// Builder for a single radical slot. Tags the `Cons` with `Role::Radical(n)`.
fn rad(letter: char, n: u8) -> Cons {
    Cons::radical(letter, n)
}

/// The seven Biblical Hebrew binyanim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Binyan {
    Qal,
    Niphal,
    Piel,
    Pual,
    Hithpael,
    Hiphil,
    Hophal,
}

impl Binyan {
    pub const ALL: [Binyan; 7] = [
        Binyan::Qal,
        Binyan::Niphal,
        Binyan::Piel,
        Binyan::Pual,
        Binyan::Hithpael,
        Binyan::Hiphil,
        Binyan::Hophal,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Binyan::Qal => "Qal",
            Binyan::Niphal => "Niphal",
            Binyan::Piel => "Piel",
            Binyan::Pual => "Pual",
            Binyan::Hithpael => "Hithpael",
            Binyan::Hiphil => "Hiphil",
            Binyan::Hophal => "Hophal",
        }
    }
}

/// A verbal form (conjugation / non-finite type).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Form {
    Perfect,
    Imperfect,
    Imperative,
    Cohortative,
    Jussive,
    /// Vav-consecutive imperfect (preterite / narrative past): וַ + a forte
    /// dagesh on the prefix consonant over the short (jussive) stem.
    Wayyiqtol,
    InfinitiveConstruct,
    InfinitiveAbsolute,
    ParticipleActive,
    ParticiplePassive,
}

impl Form {
    pub fn name(self) -> &'static str {
        match self {
            Form::Perfect => "Perfect",
            Form::Imperfect => "Imperfect",
            Form::Imperative => "Imperative",
            Form::Cohortative => "Cohortative",
            Form::Jussive => "Jussive",
            Form::Wayyiqtol => "Wayyiqtol",
            Form::InfinitiveConstruct => "Inf. Construct",
            Form::InfinitiveAbsolute => "Inf. Absolute",
            Form::ParticipleActive => "Participle (act.)",
            Form::ParticiplePassive => "Participle (pas.)",
        }
    }
}

/// Person/Gender/Number slot. `None` axes mean the form doesn't inflect on
/// that axis (e.g. infinitives have all three None; participles have person
/// = None).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pgn {
    pub person: Option<Person>,
    pub gender: Option<Gender>,
    pub number: Option<Number>,
}

impl Pgn {
    pub const fn new(p: Person, g: Gender, n: Number) -> Self {
        Pgn {
            person: Some(p),
            gender: Some(g),
            number: Some(n),
        }
    }
    pub const fn gn(g: Gender, n: Number) -> Self {
        Pgn {
            person: None,
            gender: Some(g),
            number: Some(n),
        }
    }
    pub const fn none() -> Self {
        Pgn {
            person: None,
            gender: None,
            number: None,
        }
    }

    pub fn label(&self) -> String {
        let p = match self.person {
            Some(Person::First) => "1",
            Some(Person::Second) => "2",
            Some(Person::Third) => "3",
            None => "",
        };
        let g = match self.gender {
            Some(Gender::Masculine) => "m",
            Some(Gender::Feminine) => "f",
            Some(Gender::Common) => "c",
            None => "",
        };
        let n = match self.number {
            Some(Number::Singular) => "s",
            Some(Number::Plural) => "p",
            Some(Number::Dual) => "d",
            None => "",
        };
        format!("{p}{g}{n}")
    }
}

/// The PGN slots used by the Perfect (suffix) conjugation.
const PERFECT_PGNS: &[Pgn] = &[
    Pgn::new(Person::Third, Gender::Masculine, Number::Singular),
    Pgn::new(Person::Third, Gender::Feminine, Number::Singular),
    Pgn::new(Person::Second, Gender::Masculine, Number::Singular),
    Pgn::new(Person::Second, Gender::Feminine, Number::Singular),
    Pgn::new(Person::First, Gender::Common, Number::Singular),
    Pgn::new(Person::Third, Gender::Common, Number::Plural),
    Pgn::new(Person::Second, Gender::Masculine, Number::Plural),
    Pgn::new(Person::Second, Gender::Feminine, Number::Plural),
    Pgn::new(Person::First, Gender::Common, Number::Plural),
];

/// The PGN slots used by the Imperfect (prefix) conjugation.
const IMPERFECT_PGNS: &[Pgn] = &[
    Pgn::new(Person::Third, Gender::Masculine, Number::Singular),
    Pgn::new(Person::Third, Gender::Feminine, Number::Singular),
    Pgn::new(Person::Second, Gender::Masculine, Number::Singular),
    Pgn::new(Person::Second, Gender::Feminine, Number::Singular),
    Pgn::new(Person::First, Gender::Common, Number::Singular),
    Pgn::new(Person::Third, Gender::Masculine, Number::Plural),
    Pgn::new(Person::Third, Gender::Feminine, Number::Plural),
    Pgn::new(Person::Second, Gender::Masculine, Number::Plural),
    Pgn::new(Person::Second, Gender::Feminine, Number::Plural),
    Pgn::new(Person::First, Gender::Common, Number::Plural),
];

const IMPERATIVE_PGNS: &[Pgn] = &[
    Pgn::new(Person::Second, Gender::Masculine, Number::Singular),
    Pgn::new(Person::Second, Gender::Feminine, Number::Singular),
    Pgn::new(Person::Second, Gender::Masculine, Number::Plural),
    Pgn::new(Person::Second, Gender::Feminine, Number::Plural),
];

const COHORTATIVE_PGNS: &[Pgn] = &[
    Pgn::new(Person::First, Gender::Common, Number::Singular),
    Pgn::new(Person::First, Gender::Common, Number::Plural),
];

const JUSSIVE_PGNS: &[Pgn] = &[
    Pgn::new(Person::Third, Gender::Masculine, Number::Singular),
    Pgn::new(Person::Third, Gender::Feminine, Number::Singular),
    Pgn::new(Person::Second, Gender::Masculine, Number::Singular),
    Pgn::new(Person::Second, Gender::Feminine, Number::Singular),
];

/// The full jussive paradigm includes the plurals (negative commands like
/// אַל־תִּירְאוּ are tagged jussive), whose forms coincide with the imperfect.
/// [`JUSSIVE_PGNS`] stays the singular short-form set — it doubles as the
/// "build the wayyiqtol from the jussive base" test, which must not catch the
/// plurals.
const JUSSIVE_PARADIGM_PGNS: &[Pgn] = &[
    Pgn::new(Person::Third, Gender::Masculine, Number::Singular),
    Pgn::new(Person::Third, Gender::Feminine, Number::Singular),
    Pgn::new(Person::Second, Gender::Masculine, Number::Singular),
    Pgn::new(Person::Second, Gender::Feminine, Number::Singular),
    Pgn::new(Person::Third, Gender::Masculine, Number::Plural),
    Pgn::new(Person::Third, Gender::Feminine, Number::Plural),
    Pgn::new(Person::Second, Gender::Masculine, Number::Plural),
    Pgn::new(Person::Second, Gender::Feminine, Number::Plural),
];

const PARTICIPLE_PGNS: &[Pgn] = &[
    Pgn::gn(Gender::Masculine, Number::Singular),
    Pgn::gn(Gender::Feminine, Number::Singular),
    Pgn::gn(Gender::Masculine, Number::Plural),
    Pgn::gn(Gender::Feminine, Number::Plural),
];

/// One inflected verb form.
#[derive(Debug, Clone)]
pub struct VerbForm {
    pub binyan: Binyan,
    pub form: Form,
    pub pgn: Pgn,
    /// Final Hebrew Unicode string.
    pub text: String,
    /// True if this is from a fully-modelled (binyan, form, gizra)
    /// combination; false if the generator fell back to the strong-verb
    /// pattern for a class it doesn't yet specialise on.
    pub attested: bool,
    /// The pronominal object suffix attached to this form, if any (its
    /// person/gender/number). `None` for the bare conjugated form.
    pub object_suffix: Option<Pgn>,
}

/// The full paradigm of a root: every binyan × form × PGN we can generate.
#[derive(Debug, Clone)]
pub struct Paradigm {
    pub root: Root,
    pub forms: Vec<VerbForm>,
}

/// Generate the full paradigm.
pub fn generate_paradigm(root: &Root) -> Paradigm {
    let mut forms = Vec::new();
    for &binyan in &Binyan::ALL {
        for &form in FORMS_FOR_PARADIGM.iter() {
            if !binyan_has_form(binyan, form) {
                continue;
            }
            for &pgn in pgns_for_form(form) {
                let (text, attested) = generate_one(root, binyan, form, pgn, false);
                // Alternant spellings share this slot's analysis label and are
                // pushed AFTER the base form, so a first-match lookup still
                // returns the canonical spelling.
                //   - Maqaf twin: a stressed final-syllable tsere in a closed
                //     syllable shortens to segol when the word is bound to the
                //     next by maqaf (yittēn → יִתֶּן, wayyittēn → וַיִּתֶּן). The
                //     OSHB strips the maqaf, so the segol spelling surfaces bare.
                //   - Paragogic-he twin for the 2ms perfect: an archaic/full
                //     spelling appends a he mater to the -tā ending (nātattā →
                //     נָתַתָּה, yāḏaʕtā → יָדַעְתָּה).
                let maqaf = maqaf_segol_variant(&text);
                let guttural_lowered = guttural_lowered_variant(&text);
                let pausal = pausal_qamats_variant(&text);
                // I-guttural Hiphil C2 spirantization (הֶעֱבִיר, הַאֲבַדְתִּי).
                let hiphil_guttural_c2_spirant = (binyan == Binyan::Hiphil
                    && root.has(Gizra::PeGuttural))
                .then(|| hiphil_guttural_c2_spirant_variant(root, &text))
                .flatten();
                // I-yod Hiphil ê-twin (הֵיטִיב, הֵילִיל, הֵיטֵב).
                let pe_yod_hiphil_e = (binyan == Binyan::Hiphil && root.has(Gizra::PeYod))
                    .then(|| pe_yod_hiphil_e_variant(&text))
                    .flatten();
                // Plene-theme twin of the pe-yod Hiphil inf-absolute: the theme
                // tsere of hêṭēḇ הֵיטֵב is commonly written with a yod mater —
                // hêṭêḇ הֵיטֵיב.
                let pe_yod_hiphil_e_plene = (binyan == Binyan::Hiphil
                    && root.has(Gizra::PeYod)
                    && form == Form::InfinitiveAbsolute)
                .then(|| {
                    let base = pe_yod_hiphil_e.as_ref()?;
                    let mut seq = hebrew::parse_pointed(base);
                    if seq.len() >= 3 {
                        let theme = seq.len() - 2;
                        if seq[theme].vowel == Some(Vowel::Tsere) {
                            seq.insert(theme + 1, Cons::mater(letter::YOD));
                            return Some(hebrew::render(&seq));
                        }
                    }
                    None
                })
                .flatten();
                // I-guttural Qal imperfect/wayyiqtol holam plural (יַעֲמֹדוּ).
                let pe_guttural_impf_hataf = (binyan == Binyan::Qal
                    && root.has(Gizra::PeGuttural)
                    && matches!(form, Form::Imperfect | Form::Wayyiqtol)
                    && matches!(
                        (pgn.gender, pgn.number),
                        (Some(Gender::Masculine), Some(Number::Plural))
                    ))
                .then(|| pe_guttural_imperfect_holam_plural_variant(root, pgn))
                .flatten();
                // Silent-sheva twin for gutturals that close the syllable (יַחְפֹּץ,
                // יַחְפֹּצוּ) — singular and plural, derived from the hataf form.
                let pe_guttural_impf_silent = (binyan == Binyan::Qal
                    && root.has(Gizra::PeGuttural)
                    && matches!(
                        form,
                        Form::Imperfect | Form::Wayyiqtol | Form::Jussive | Form::Cohortative
                    ))
                .then(|| pe_guttural_qal_silent_twin_variant(root, &text))
                .flatten();
                let pe_guttural_impf_silent_pl = (binyan == Binyan::Qal
                    && root.has(Gizra::PeGuttural)
                    && matches!(form, Form::Imperfect | Form::Wayyiqtol)
                    && matches!(
                        (pgn.gender, pgn.number),
                        (Some(Gender::Masculine), Some(Number::Plural))
                    ))
                .then(|| pe_guttural_imperfect_holam_plural_silent_variant(root, pgn))
                .flatten();
                // Its interior-pausal grades (yeḥdālû יֶחְדָּלוּ) — the theme
                // restores under stress; the transforms compose. The o-theme
                // build above only reaches the holam-class pausal (yaḥpōṣû →
                // yaḥpōṣû); the stative a-theme qamats plural is built directly.
                let pe_guttural_impf_silent_pl_pausal: Vec<String> = pe_guttural_impf_silent_pl
                    .as_deref()
                    .map(pausal_imperfect_plural_variants)
                    .unwrap_or_default();
                let pe_guttural_impf_a_silent_pl: Vec<String> = if binyan == Binyan::Qal
                    && root.has(Gizra::PeGuttural)
                    && matches!(form, Form::Imperfect | Form::Wayyiqtol)
                    && matches!(
                        (pgn.gender, pgn.number),
                        (Some(Gender::Masculine), Some(Number::Plural))
                    ) {
                    pe_guttural_imperfect_a_plural_silent_variants(root, pgn)
                } else {
                    Default::default()
                };
                // Geminate Qal perfect a-class (סַבּוּ, רַבּוּ, סַבּוֹתָ).
                let geminate_qal_perf =
                    (binyan == Binyan::Qal && form == Form::Perfect && root.has(Gizra::Geminate))
                        .then(|| geminate_qal_perfect_variant(root, pgn))
                        .flatten();
                // Its pausal qamats grade (rabbû רָבּוּ beside רַבּוּ).
                let geminate_qal_perf_pausal = geminate_qal_perf.as_deref().and_then(|t| {
                    let mut seq = hebrew::parse_pointed(t);
                    (seq.first().and_then(|c| c.vowel) == Some(Vowel::Patah)).then(|| {
                        seq[0].vowel = Some(Vowel::Qamats);
                        hebrew::render(&seq)
                    })
                });
                // Uncontracted geminate perfect with the doubled radical on a
                // hataf (ṣālălû צָלֲלוּ beside the plain-sheva צָלְלוּ).
                let geminate_qal_perf_hataf = (binyan == Binyan::Qal
                    && form == Form::Perfect
                    && root.ayin() == root.lamed()
                    && perfect_suffix_kind(pgn) == Suffix::Vocalic)
                    .then(|| {
                        let mut seq = hebrew::parse_pointed(&text);
                        let n = seq.len();
                        let i = (1..n.saturating_sub(1)).find(|&i| {
                            seq[i].letter == root.ayin()
                                && seq[i].vowel == Some(Vowel::Sheva)
                                && seq[i + 1].letter == root.lamed()
                        })?;
                        seq[i].vowel = Some(Vowel::HatafPatah);
                        Some(hebrew::render(&seq))
                    })
                    .flatten();
                // Contracted geminate Qal imperative with the qamats stem —
                // ronnû רָנּוּ, ronnî רָנִּי (רנן).
                let geminate_qal_imv = (binyan == Binyan::Qal
                    && form == Form::Imperative
                    && root.ayin() == root.lamed()
                    && !hebrew::is_guttural(root.ayin())
                    && root.ayin() != letter::RESH)
                    .then(|| {
                        let c2 = rad(root.ayin(), 2).with_dagesh();
                        let tail: Vec<Cons> = match (pgn.gender, pgn.number) {
                            (Some(Gender::Masculine), Some(Number::Plural)) => vec![c2, oshureq()],
                            (Some(Gender::Feminine), Some(Number::Singular)) => {
                                vec![c2.with_vowel(Vowel::Hiriq), Cons::mater(letter::YOD)]
                            }
                            _ => return None,
                        };
                        let mut seq = vec![rad(root.pe(), 1).with_vowel(Vowel::Qamats)];
                        seq.extend(tail);
                        Some(hebrew::render(&seq))
                    })
                    .flatten();
                // נגש: the o-grade vocalic imperative gōšû גֹּשׁוּ beside גְּשׁוּ.
                let nagash_imv_holam =
                    (is_nagash(root) && binyan == Binyan::Qal && form == Form::Imperative)
                        .then(|| {
                            let mut seq = hebrew::parse_pointed(&text);
                            (seq.first().and_then(|c| c.vowel) == Some(Vowel::Sheva)).then(|| {
                                seq[0].vowel = Some(Vowel::Holam);
                                hebrew::render(&seq)
                            })
                        })
                        .flatten();
                // Interior pausal of the Qal/Niphal perfect (יָדָעְתָּ, שָׁמָרְתָּ,
                // יָצָאוּ, נִלְחָמוּ).
                // The C2 theme patah lengthens to qamats in pause across the
                // binyanim — qāṭāltā, hiṯhallāḵtā הִתְהַלָּכְתָּ, hiršāʿnû
                // הִרְשָׁעְנוּ.
                let pausal_perf = (form == Form::Perfect)
                    .then(|| pausal_perfect_c2_variant(root, &text))
                    .flatten();
                let paragogic = (form == Form::Perfect
                    && pgn.person == Some(Person::Second)
                    && pgn.gender == Some(Gender::Masculine)
                    && pgn.number == Some(Number::Singular))
                .then(|| format!("{text}\u{05D4}"));
                // Apocopated Hiphil short imperfect: the long hiriq-yod theme
                // collapses to tsere in the jussive and wayyiqtol (yaqrîb →
                // yaqrēb, wayyaškîm → wayyaškēm).
                let hiphil_apoc = (binyan == Binyan::Hiphil
                    && matches!(form, Form::Jussive | Form::Wayyiqtol))
                .then(|| hiphil_apocope_variant(&text))
                .flatten();
                // Plene twin of the vocalic-suffix Hiphil (yaggidû → yaggîdû).
                let hiphil_plene = (binyan == Binyan::Hiphil
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                    && imperfect_suffix_kind(pgn) == Suffix::Vocalic)
                    .then(|| hiphil_plene_variant(&text))
                    .flatten();
                // Defective twin of the vocalic-suffix Hiphil — the dominant
                // prose wayyiqtol (wayyaggidû וַיַּגִּדוּ beside וַיַּגִּידוּ) — and
                // of the zero-suffix forms, where the î-mater is also dropped
                // in attested spellings (yāḇiʾ יָבִא, yôsip̄ יוֹסִף).
                let hiphil_defective = (binyan == Binyan::Hiphil
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                    && matches!(imperfect_suffix_kind(pgn), Suffix::Vocalic | Suffix::Zero))
                .then(|| hiphil_defective_variant(&text))
                .flatten();
                // Reduced-theme grade of the defective vocalic Hiphil — the
                // hiriq drops to sheva (wayyaḏbᵊqû וַיַּדְבְּקוּ).
                let hiphil_impf_sheva = hiphil_defective.as_deref().and_then(|t| {
                    let mut seq = hebrew::parse_pointed(t);
                    let n = seq.len();
                    (n >= 3
                        && seq[n - 1].letter == letter::VAV
                        && seq[n - 1].dagesh
                        && seq[n - 1].vowel.is_none()
                        && seq[n - 3].vowel == Some(Vowel::Hiriq))
                    .then(|| {
                        seq[n - 3].vowel = Some(Vowel::Sheva);
                        hebrew::render(&seq)
                    })
                });
                // Hollow Niphal infinitive (himmôl הִמּוֹל): he-hiriq, the
                // doubled C1, the hollow ô, C3 — the strong path leaves
                // nothing usable for a hollow root.
                let hollow_niphal_inf = (binyan == Binyan::Niphal
                    && matches!(form, Form::InfinitiveAbsolute | Form::InfinitiveConstruct)
                    && root.has(Gizra::Hollow)
                    && !hebrew::is_guttural(root.pe())
                    && root.pe() != letter::RESH)
                    .then(|| {
                        hebrew::render(&[
                            Cons::new(letter::HE).with_vowel(Vowel::Hiriq),
                            rad(root.pe(), 1).with_dagesh(),
                            Cons::new(letter::VAV).with_vowel(Vowel::Holam),
                            rad(root.lamed(), 3),
                        ])
                    });
                // Pausal twin of the Piel/Hithpael vocalic-suffix imperfect:
                // the reduced theme sheva restores to tsere in pause
                // (yᵉḏabbᵊrû → yᵉḏabbērû, יְדַבֵּרוּ).
                let piel_pausal = (matches!(binyan, Binyan::Piel | Binyan::Hithpael)
                    && (matches!(form, Form::Imperfect | Form::Wayyiqtol)
                        && imperfect_suffix_kind(pgn) == Suffix::Vocalic
                        || form == Form::Cohortative))
                    .then(|| piel_pausal_variant(&text))
                    .flatten();
                // Theme twins of the zero-suffix Piel/Hithpael imperfect: the
                // tsere lengthens to qamats in pause (ʾeṯḥannān אֶתְחַנָּן) or
                // lowers to patah (tᵊʾaḥar תְּאַחַר, wayyiṯʾannap̄ וַיִּתְאַנַּף).
                let piel_pausal_zero: Vec<String> =
                    if matches!(binyan, Binyan::Piel | Binyan::Hithpael)
                        && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                        && imperfect_suffix_kind(pgn) == Suffix::Zero
                    {
                        let seq = hebrew::parse_pointed(&text);
                        let n = seq.len();
                        // The theme sits on C3 (n-2); the word-final consonant
                        // (n-1) is either bare or carries a silent sheva (final
                        // kaf, yiṯhallēḵ יִתְהַלֵּךְ) — both leave the theme final.
                        if n < 2
                            || !matches!(seq[n - 1].vowel, None | Some(Vowel::Sheva))
                            || seq[n - 2].vowel != Some(Vowel::Tsere)
                        {
                            Vec::new()
                        } else {
                            [Vowel::Qamats, Vowel::Patah]
                                .into_iter()
                                .map(|v| {
                                    let mut s = seq.clone();
                                    s[n - 2].vowel = Some(v);
                                    hebrew::render(&s)
                                })
                                .collect()
                        }
                    } else {
                        Vec::new()
                    };
                // Pausal twin of the Qal vocalic-suffix imperfect: the reduced
                // theme sheva restores to the 3ms theme vowel in pause
                // (yēlᵊḵû → yēlēḵû יֵלֵכוּ, yippᵊlû → yippōlû יִפֹּלוּ).
                let qal_pausal = (binyan == Binyan::Qal
                    && form == Form::Imperfect
                    && imperfect_suffix_kind(pgn) == Suffix::Vocalic)
                    .then(|| {
                        let zero = Pgn::new(Person::Third, Gender::Masculine, Number::Singular);
                        let (zero_text, _) = generate_one(root, binyan, form, zero, false);
                        qal_pausal_variant(&zero_text, &text)
                    })
                    .flatten();
                // Pausal twin of the I-guttural III-He Qal imperative: the
                // propretonic hataf-patah lengthens to qamats (ʕănê → ʕānê).
                let guttural_imperative_pausal = (binyan == Binyan::Qal
                    && form == Form::Imperative
                    && root.lamed() == letter::HE
                    && hebrew::is_guttural(root.pe()))
                .then(|| guttural_hataf_pausal_variant(&text))
                .flatten();
                // Patah-prefix twin of the I-guttural Niphal/Hiphil perfect
                // (neḥĕzaq נֶחֱזַק beside naʕăśâ נַעֲשָׂה).
                let guttural_perfect_patah = (matches!(binyan, Binyan::Niphal | Binyan::Hiphil)
                    && form == Form::Perfect)
                    .then(|| guttural_perfect_patah_variant(&text))
                    .flatten();
                // Silent-sheva twin of an I-guttural derived-stem perfect, where
                // the guttural's hataf is written as a plain sheva (nehĕp̄aḵ
                // נֶהֱפַּךְ → nehpaḵ נֶהְפַּךְ, heḥĕzîq → heḥzîq).
                let guttural_silent_sheva = (matches!(
                    binyan,
                    Binyan::Niphal | Binyan::Hiphil | Binyan::Hophal | Binyan::Hithpael
                ) && matches!(
                    form,
                    Form::Perfect
                        | Form::Imperative
                        | Form::Imperfect
                        | Form::Jussive
                        | Form::Wayyiqtol
                        | Form::InfinitiveConstruct
                        | Form::ParticipleActive
                        | Form::ParticiplePassive
                ))
                .then(|| guttural_silent_sheva_variant(&text))
                .flatten();
                // Its interior-pausal grade (neḥšāḇû נֶחְשָׁבוּ): the two
                // transforms compose, so chain them explicitly.
                let guttural_silent_pausal = guttural_silent_sheva
                    .as_deref()
                    .and_then(|t| pausal_perfect_c2_variant(root, t));
                // I-guttural Hiphil imperative segol-prefix twin (הֶעֱמִיקוּ,
                // הֶחֱשׁוּ): the hē- prefix attenuates to segol before the guttural
                // C1, mirroring the perfect's הֶעֱמִיק. Applied to the primary
                // imperative and to its silent-sheva grade (הֶעְמִיקוּ).
                let hiphil_iguttural_imp_segol = (binyan == Binyan::Hiphil
                    && root.has(Gizra::PeGuttural)
                    && form == Form::Imperative)
                .then(|| hiphil_iguttural_segol_prefix_variant(&text))
                .flatten();
                let hiphil_iguttural_imp_segol_silent = (binyan == Binyan::Hiphil
                    && form == Form::Imperative)
                .then_some(())
                .and(guttural_silent_sheva.as_deref())
                .and_then(hiphil_iguttural_segol_prefix_variant);
                // Hollow Qal active participle qāmÌ„ shape (רֹץ → רָץ).
                let hollow_ptcp = (binyan == Binyan::Qal
                    && form == Form::ParticipleActive
                    && root.has(Gizra::Hollow))
                .then(|| hollow_qal_participle_variant(&text))
                .flatten();
                // Hollow Hiphil active participle mēCîC shape (מֵבִיא, מְבִיאִים).
                let hollow_hiphil_ptcp = (binyan == Binyan::Hiphil
                    && form == Form::ParticipleActive
                    && root.has(Gizra::Hollow))
                .then(|| hollow_hiphil_participle_variant(root, pgn))
                .flatten();
                // Hollow Hophal participle mûC1āC3 shape (מוּקָם, מוּשָׁב).
                let hollow_hophal_ptcp = (binyan == Binyan::Hophal
                    && form == Form::ParticipleActive
                    && root.has(Gizra::Hollow))
                .then(|| hollow_hophal_participle_variant(root, pgn))
                .flatten();
                // Hollow Qal perfect heavy 2mp/2fp suffix (קַמְתֶּם, בָּאתֶם).
                let hollow_qal_perf_heavy =
                    (binyan == Binyan::Qal && form == Form::Perfect && root.has(Gizra::Hollow))
                        .then(|| hollow_qal_perfect_heavy_suffix_variant(root, pgn))
                        .flatten();
                // II-guttural derived-stem hataf twin (בָּרֲכוּ, וַיְבָרֲכוּ).
                let ayin_guttural_hataf =
                    (matches!(binyan, Binyan::Piel | Binyan::Pual | Binyan::Hithpael)
                        && root.has(Gizra::AyinGuttural))
                    .then(|| ayin_guttural_hataf_variant(root, &text))
                    .flatten();
                // II-guttural Qal imperative a-harmony twin: the vocalic-suffix
                // forms (qiṭlû/qiṭlî) have C2 close the first syllable on a
                // silent sheva (זִעְקוּ), but a guttural C2 cannot — it opens
                // with a hataf-patah and the hiriq harmonises to patah:
                // zaʿăqû זַעֲקוּ, šaʾălû שַׁאֲלוּ, baḥărû בַּחֲרוּ.
                let qal_imperative_ayin_gutt = (binyan == Binyan::Qal
                    && form == Form::Imperative
                    && root.has(Gizra::AyinGuttural)
                    && matches!(
                        (pgn.gender, pgn.number),
                        (Some(Gender::Feminine), Some(Number::Singular))
                            | (Some(Gender::Masculine), Some(Number::Plural))
                    ))
                .then(|| qal_imperative_ayin_guttural_a_variant(root, &text))
                .flatten();
                // II-guttural Piel/Hithpael perfect virtual-doubling hiriq twin
                // (נִאֲצוּ beside נֵאֲצוּ).
                let piel_perf_guttural_hiriq =
                    (matches!(binyan, Binyan::Piel | Binyan::Pual | Binyan::Hithpael)
                        && form == Form::Perfect
                        && hebrew::is_guttural(root.ayin()))
                    .then(|| piel_perfect_guttural_hiriq_variant(root, &text))
                    .flatten();
                // Pe-aleph wayyiqtol 1cp segol twin (וַנֹּאמֶר beside וַנֹּאמַר).
                let pe_aleph_wayy_1cp = (binyan == Binyan::Qal
                    && form == Form::Wayyiqtol
                    && root.has(Gizra::PeAleph)
                    && pgn == Pgn::new(Person::First, Gender::Common, Number::Plural))
                .then(|| pe_aleph_wayyiqtol_segol_variant(&text))
                .flatten();
                // III-aleph participle plural reduction (נִמְצְאִים, נִמְצְאוֹת).
                let lamed_aleph_ptcp =
                    (matches!(form, Form::ParticipleActive | Form::ParticiplePassive)
                        && root.has(Gizra::LamedAleph)
                        && pgn.number == Some(Number::Plural))
                    .then(|| lamed_aleph_participle_reduce_variant(&text))
                    .flatten();
                // PeAleph Qal Imperfect tsere variant — יֹאכֵל beside יֹאכַל.
                let pe_aleph_tsere = (binyan == Binyan::Qal
                    && root.has(Gizra::PeAleph)
                    && matches!(form, Form::Imperfect | Form::Jussive))
                .then(|| pe_aleph_imperfect_tsere_variant(&text))
                .flatten();
                // PeAleph Qal holam-contraction twin for roots outside YO_ROOTS
                // that nonetheless attest it — יֹאחֵז beside יֶאֱחֹז, וַיֹּאחֲזוּ
                // beside וַיֶּאֶחְזוּ. Skips the 1cs (double-aleph merge).
                let pe_aleph_holam = (binyan == Binyan::Qal
                    && root.has(Gizra::PeAleph)
                    && matches!(
                        form,
                        Form::Imperfect | Form::Jussive | Form::Cohortative | Form::Wayyiqtol
                    )
                    && !(pgn.person == Some(Person::First)
                        && pgn.number == Some(Number::Singular)))
                .then(|| pe_aleph_holam_variant(&text))
                .flatten();
                // Uncontracted Hiphil imperfect twin — יְהוֹשִׁיעַ beside יוֹשִׁיעַ.
                let hiphil_uncontracted = (binyan == Binyan::Hiphil
                    && matches!(
                        form,
                        Form::Imperfect | Form::Jussive | Form::Cohortative
                    ))
                .then(|| hiphil_imperfect_uncontracted_variant(&text))
                .flatten();
                // The yê- contracted twin (yêlîl יֵילִיל) is itself a host for the
                // uncontracted split — yᵊyêlîl (יְיֵלִיל) from ילל.
                let hiphil_uncontracted_e = (binyan == Binyan::Hiphil
                    && matches!(
                        form,
                        Form::Imperfect | Form::Jussive | Form::Cohortative
                    ))
                .then(|| {
                    hiphil_imperfect_uncontracted_variant(pe_yod_hiphil_e.as_deref()?)
                })
                .flatten();
                // LamedAleph Qal Perfect tsere variant — שָׂנֵאתִי beside שָׂנָאתִי.
                let lamed_aleph_tsere = (binyan == Binyan::Qal
                    && root.has(Gizra::LamedAleph)
                    && form == Form::Perfect
                    && matches!(pgn.person, Some(Person::First | Person::Second)))
                .then(|| lamed_aleph_perfect_tsere_variant(&text))
                .flatten();
                // PeGuttural Qal Imperfect segol variant — יֶחֱטָא beside יַחֲטָא.
                let pe_guttural_segol = (binyan == Binyan::Qal
                    && root.has(Gizra::PeGuttural)
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol))
                .then(|| pe_guttural_imperfect_segol_variant(&text))
                .flatten();
                // Qal a-theme (stative) imperfect alternant — yiqtal beside the
                // default yiqtōl (yiḡdal יִגְדַּל, yiqrab יִקְרַב), and likewise the
                // jussive and wayyiqtol (wayyiḡdal וַיִּגְדַּל). Statives are a
                // lexical class we don't mark, so we emit the a-theme form for
                // every Qal root and keep it only when it actually differs from
                // the default; the patah spelling differs from the holam one, so
                // a surface matches at most one and no ambiguity is added.
                let qal_a_theme = (binyan == Binyan::Qal
                    && matches!(
                        form,
                        Form::Imperfect | Form::Jussive | Form::Wayyiqtol | Form::Imperative
                    ))
                .then(|| {
                    let (a_text, _) = generate_one(root, binyan, form, pgn, true);
                    (a_text != text).then_some(a_text)
                })
                .flatten();
                // Pausal twin of the a-theme imperfect: the patah theme
                // lengthens to qamats in pause — yiḇḥar → yiḇḥār יִבְחָר.
                let qal_a_theme_pausal = (binyan == Binyan::Qal
                    && matches!(
                        form,
                        Form::Imperfect | Form::Jussive | Form::Wayyiqtol | Form::Imperative
                    ))
                .then(|| qal_a_theme.as_deref().and_then(pausal_qamats_variant))
                .flatten();
                // Qal a-theme infinitive-construct twin: the stative class
                // keeps its patah theme in the infinitive too — šᵊḵaḇ
                // (לִשְׁכַּב) beside the default šᵊḵōḇ. Like the imperfect
                // a-theme it is emitted for every root and matches only when
                // attested.
                let qal_a_theme_inf = (binyan == Binyan::Qal
                    && form == Form::InfinitiveConstruct
                    && root.lamed() != letter::HE)
                    .then(|| {
                        let mut seq = hebrew::parse_pointed(&text);
                        let n = seq.len();
                        (n >= 2
                            && seq[n - 1].vowel.is_none()
                            && seq[n - 2].vowel == Some(Vowel::Holam))
                        .then(|| {
                            seq[n - 2].vowel = Some(Vowel::Patah);
                            hebrew::render(&seq)
                        })
                    })
                    .flatten();
                // Loud (hataf-segol) twin of the pe-guttural imperfect plural:
                // the segol-prefix grade where the guttural C1 takes a
                // hataf-segol rather than the silent sheva — yeʾĕrōḇû יֶאֱרֹבוּ
                // (o-theme), yeḥĕrāḇû יֶחֱרָבוּ (pausal a-theme). Built by applying
                // the segol shift ([C-patah][guttural-hatafpatah] →
                // [C-segol][guttural-hatafsegol]) to the hataf-patah plural bases.
                let pe_guttural_loud_segol_pl: Vec<String> = if binyan == Binyan::Qal
                    && root.has(Gizra::PeGuttural)
                    && matches!(form, Form::Imperfect | Form::Wayyiqtol | Form::Jussive)
                {
                    pe_guttural_impf_hataf
                        .iter()
                        .cloned()
                        .chain(qal_a_theme_pausal.clone())
                        .filter_map(|t| pe_guttural_imperfect_segol_variant(&t))
                        .collect()
                } else {
                    Vec::new()
                };
                // Silent-sheva + pausal twins of the a-theme I-guttural
                // imperfect family: the hataf grade (ʾeḥĕḏal אֶחֱדַל) closes on
                // a silent sheva (ʾeḥdal אֶחְדַּל, וַיֶּחְדַּל) and lengthens in
                // pause (ʾeḥdāl אֶחְדָּל).
                let pe_guttural_a_silent: Vec<String> = if binyan == Binyan::Qal
                    && root.has(Gizra::PeGuttural)
                    && matches!(
                        form,
                        Form::Imperfect | Form::Jussive | Form::Wayyiqtol | Form::Cohortative
                    ) {
                    {
                        qal_a_theme
                            .iter()
                            .filter_map(|t| pe_guttural_qal_silent_twin_variant(root, t))
                            .flat_map(|s| {
                                let pausal = pausal_qamats_variant(&s);
                                // The vocalic-suffix plurals lengthen interiorly
                                // instead (yeḥdālû יֶחְדָּלוּ).
                                let plural = pausal_imperfect_plural_variants(&s);
                                std::iter::once(s).chain(pausal).chain(plural)
                            })
                            .collect()
                    }
                } else {
                    Default::default()
                };
                // Loud twin of the silent-sheva pe-guttural plural: where that
                // form closed the segol prefix on a *silent* sheva (yeḥrāḇû
                // יֶחְרָבוּ), the guttural can instead open the next syllable with
                // a hataf-segol — yeḥĕrāḇû יֶחֱרָבוּ, yeḥĕrāḏû יֶחֱרָדוּ.
                let pe_guttural_loud_from_silent: Vec<String> = pe_guttural_a_silent
                    .iter()
                    .chain(pe_guttural_impf_silent_pl_pausal.iter())
                    .chain(pe_guttural_impf_a_silent_pl.iter())
                    .filter_map(|t| guttural_segol_silent_to_hataf_variant(t))
                    .collect();
                // III-aleph Qal imperfect/imperative -nâ (3fp/2fp) segol twin
                // (תִּקְרֶאנָה, תֵּצֶאנָה, תִּשֶּׂאנָה).
                let lamed_aleph_impf_nun = (binyan == Binyan::Qal
                    && root.lamed() == letter::ALEF
                    && matches!(
                        form,
                        Form::Imperfect | Form::Jussive | Form::Wayyiqtol | Form::Imperative
                    )
                    && pgn.gender == Some(Gender::Feminine)
                    && pgn.number == Some(Number::Plural))
                .then(|| lamed_aleph_imperfect_nun_variant(&text))
                .flatten();
                // III-aleph derived-stem imperfect qamats twin: the theme tsere
                // before the quiescent aleph also surfaces as qamats —
                // yiṭṭammāʾ יִטַּמָּא, yiṯḥaṭṭāʾ יִתְחַטָּא.
                let lamed_aleph_impf_qamats =
                    (matches!(binyan, Binyan::Piel | Binyan::Pual | Binyan::Hithpael)
                        && root.lamed() == letter::ALEF
                        && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol))
                    .then(|| {
                        let mut seq = hebrew::parse_pointed(&text);
                        let n = seq.len();
                        (n >= 2
                            && seq[n - 1].letter == letter::ALEF
                            && seq[n - 1].vowel.is_none()
                            && seq[n - 2].vowel == Some(Vowel::Tsere))
                        .then(|| {
                            seq[n - 2].vowel = Some(Vowel::Qamats);
                            hebrew::render(&seq)
                        })
                    })
                    .flatten();
                // III-He Piel infinitive absolute (naqqēh וְנַקֵּה).
                let piel_inf_abs_lamed_he = (binyan == Binyan::Piel
                    && form == Form::InfinitiveAbsolute
                    && root.lamed() == letter::HE)
                    .then(|| {
                        let mut c1 = rad(root.pe(), 1).with_vowel(Vowel::Patah);
                        if hebrew::is_begedkefet(root.pe()) {
                            c1 = c1.with_dagesh();
                        }
                        Some(hebrew::render(&[
                            c1,
                            rad(root.ayin(), 2).with_dagesh().with_vowel(Vowel::Tsere),
                            Cons::new(letter::HE),
                        ]))
                    })
                    .flatten();
                // Hollow Hiphil infinitive absolute hāCēC (הָשֵׁב, הָקֵם).
                let hollow_hiphil_inf_abs = (binyan == Binyan::Hiphil
                    && form == Form::InfinitiveAbsolute
                    && root.has(Gizra::Hollow))
                .then(|| {
                    Some(hebrew::render(&[
                        Cons::new(letter::HE).with_vowel(Vowel::Qamats),
                        rad(root.pe(), 1).with_vowel(Vowel::Tsere),
                        rad(root.lamed(), 3),
                    ]))
                })
                .flatten();
                // הלך: the rare retained-he imperfect (yahălōḵ יַהֲלֹךְ) beside
                // the suppletive יֵלֵךְ.
                let halak_retained = (is_halak(root)
                    && binyan == Binyan::Qal
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                    && imperfect_suffix_kind(pgn) == Suffix::Zero)
                    .then(|| {
                        // The aleph preformative (1cs) lowers to the segol grade
                        // — ʾehĕlōḵ אֶהֱלֹךְ beside yahălōḵ יַהֲלֹךְ.
                        let (pre_v, he_v) = if prefix_letter(pgn) == letter::ALEF {
                            (Vowel::Segol, Vowel::HatafSegol)
                        } else {
                            (Vowel::Patah, Vowel::HatafPatah)
                        };
                        Some(hebrew::render(&[
                            Cons::new(prefix_letter(pgn)).with_vowel(pre_v),
                            rad(letter::HE, 1).with_vowel(he_v),
                            rad(letter::LAMED, 2).with_vowel(Vowel::Holam),
                            rad(letter::KAF, 3),
                        ]))
                    })
                    .flatten();
                // Reduced defective Hiphil participle plural: the defective
                // theme hiriq reduces — maḥṣᵊrîm מַחְצְרִים beside מַחְצִרִים.
                let hiphil_ptcp_reduced = (binyan == Binyan::Hiphil
                    && form == Form::ParticipleActive
                    && pgn.number == Some(Number::Plural))
                .then(|| {
                    let defective = hiphil_defective_variant(&text)?;
                    let mut seq = hebrew::parse_pointed(&defective);
                    let n = seq.len();
                    (n >= 5 && seq[n - 4].vowel == Some(Vowel::Hiriq)).then(|| {
                        seq[n - 4].vowel = Some(Vowel::Sheva);
                        hebrew::render(&seq)
                    })
                })
                .flatten();
                // Short 1cs wayyiqtol from the 3ms short base: the preformative
                // swaps to aleph under a qamats vav — wayyáʿan וַיַּעַן →
                // III-aleph perfect bare-tav fs: the quiescent aleph leaves the
                // -t afformative with neither a dagesh nor a silent sheva —
                // qārāʾt קָרָאת. This spells both the 2fs perfect and the archaic
                // 3fs -āṯ (wᵊqārāʾt šᵊmô וְקָרָאת שְׁמוֹ, Isa 7:14).
                let lamed_aleph_perf_bare_t: Option<String> = (root.lamed() == letter::ALEF
                    && form == Form::Perfect
                    && pgn.gender == Some(Gender::Feminine)
                    && pgn.number == Some(Number::Singular)
                    && matches!(pgn.person, Some(Person::Second | Person::Third)))
                .then(|| -> Option<String> {
                    // The 2fs surface already carries the gizra-correct stem
                    // vowels; the 3fs reuses that same stem, so derive both from
                    // the 2fs form.
                    let base = if pgn.person == Some(Person::Second) {
                        text.clone()
                    } else {
                        generate_one(
                            root,
                            binyan,
                            Form::Perfect,
                            Pgn::new(Person::Second, Gender::Feminine, Number::Singular),
                            false,
                        )
                        .0
                    };
                    let mut seq = hebrew::parse_pointed(&base);
                    let last = seq.last_mut()?;
                    if last.letter == letter::TAV && last.vowel == Some(Vowel::Sheva) {
                        last.vowel = None;
                        last.dagesh = false;
                        Some(hebrew::render(&seq))
                    } else {
                        None
                    }
                })
                .flatten();
                // wāʾáʿan וָאַעַן, וָאַעַשׂ.
                let wayyiqtol_1cs_short = (form == Form::Wayyiqtol
                    && pgn == Pgn::new(Person::First, Gender::Common, Number::Singular))
                .then(|| {
                    let (t3, _) = generate_one(
                        root,
                        binyan,
                        Form::Wayyiqtol,
                        Pgn::new(Person::Third, Gender::Masculine, Number::Singular),
                        false,
                    );
                    let mut seq = hebrew::parse_pointed(&t3);
                    (seq.len() >= 3
                        && seq[0].letter == letter::VAV
                        && seq[0].vowel == Some(Vowel::Patah)
                        && seq[1].letter == letter::YOD)
                        .then(|| {
                            seq[0].vowel = Some(Vowel::Qamats);
                            seq[1].letter = letter::ALEF;
                            seq[1].dagesh = false;
                            hebrew::render(&seq)
                        })
                })
                .flatten();
                // Hollow III-aleph Qal perfect 2fs (bāʾt בָּאת).
                let hollow_qal_perf_2fs = (binyan == Binyan::Qal
                    && form == Form::Perfect
                    && root.has(Gizra::Hollow)
                    && root.lamed() == letter::ALEF
                    && pgn == Pgn::new(Person::Second, Gender::Feminine, Number::Singular))
                .then(|| {
                    Some(hebrew::render(&[
                        rad(root.pe(), 1).with_vowel(Vowel::Qamats),
                        rad(root.lamed(), 3),
                        Cons::new(letter::TAV),
                    ]))
                })
                .flatten();
                // Geminate Hiphil imperfect family (יָחֵל, וַיָּחֵלּוּ) and the
                // linking-ô consonantal perfect (וַהֲשִׁמֹּתִי).
                let geminate_hiphil_impf = (binyan == Binyan::Hiphil
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                    && root.ayin() == root.lamed())
                .then(|| geminate_hiphil_imperfect_variant(root, pgn))
                .flatten()
                .map(|t| {
                    if form == Form::Wayyiqtol {
                        let mut seq = hebrew::parse_pointed(&t);
                        if let Some(first) = seq.first_mut() {
                            if first.letter == letter::ALEF {
                                let s = hebrew::render(&seq);
                                let mut out =
                                    vec![Cons::new(letter::VAV).with_vowel(Vowel::Qamats)];
                                out.extend(hebrew::parse_pointed(&s));
                                return hebrew::render(&out);
                            }
                            first.dagesh = true;
                        }
                        seq.insert(0, Cons::new(letter::VAV).with_vowel(Vowel::Patah));
                        hebrew::render(&seq)
                    } else {
                        t
                    }
                });
                // Geminate Hophal imperfect contracted twin (יֻכַּתּוּ).
                let geminate_hophal_impf = (binyan == Binyan::Hophal
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                    && root.ayin() == root.lamed())
                .then(|| geminate_hophal_imperfect_variant(root, pgn))
                .flatten();
                // Geminate Hophal perfect contracted twin (הוּחַד, הוּחַדָּה).
                let geminate_hophal_perf: Vec<String> = if binyan == Binyan::Hophal
                    && form == Form::Perfect
                    && root.ayin() == root.lamed()
                {
                    geminate_hophal_perfect_variant(root, pgn)
                } else {
                    Default::default()
                };
                // Geminate Hiphil wayyiqtol with the stress-retracted patah
                // preformative and pe-nun-style C1 doubling — wayyassēbbû
                // וַיַּסֵּבּוּ beside the qamats grade וַיָּסֵבּוּ.
                let geminate_hiphil_wayy = (binyan == Binyan::Hiphil
                    && form == Form::Wayyiqtol
                    && root.ayin() == root.lamed()
                    && !hebrew::is_guttural(root.pe())
                    && root.pe() != letter::RESH)
                    .then(|| {
                        let mut seq = vec![
                            Cons::new(letter::VAV).with_vowel(Vowel::Patah),
                            Cons::new(prefix_letter(pgn))
                                .with_dagesh()
                                .with_vowel(Vowel::Patah),
                            rad(root.pe(), 1).with_dagesh().with_vowel(Vowel::Tsere),
                        ];
                        match imperfect_suffix_kind(pgn) {
                            Suffix::Zero => {
                                seq.push(rad(root.lamed(), 3));
                                Some(hebrew::render(&seq))
                            }
                            Suffix::Vocalic => {
                                seq.push(rad(root.lamed(), 3).with_dagesh());
                                seq.push(oshureq());
                                Some(hebrew::render(&seq))
                            }
                            _ => None,
                        }
                    })
                    .flatten();
                let geminate_hiphil_perf_otav: Vec<String> = if binyan == Binyan::Hiphil
                    && form == Form::Perfect
                    && root.ayin() == root.lamed()
                    && matches!(
                        perfect_suffix_kind(pgn),
                        Suffix::Consonantal | Suffix::Heavy
                    ) {
                    geminate_hiphil_perfect_otav_variants(root, pgn)
                } else {
                    Default::default()
                };
                // Hollow Hophal perfect hûCaC (הוּבָא, הוּשַׁב).
                let hollow_hophal_perf: Vec<String> =
                    if binyan == Binyan::Hophal && form == Form::Perfect && root.has(Gizra::Hollow)
                    {
                        hollow_hophal_perfect_variants(root, pgn)
                    } else {
                        Default::default()
                    };
                // Niphal wayyiqtol nesiga patah twin: the final tsere lowers
                // under the retracted stress — וַתֵּעָצַר beside תֵּעָצֵר.
                let niphal_wayy_patah = (binyan == Binyan::Niphal
                    && form == Form::Wayyiqtol
                    && imperfect_suffix_kind(pgn) == Suffix::Zero)
                    .then(|| {
                        let mut seq = hebrew::parse_pointed(&text);
                        let c = seq
                            .iter_mut()
                            .rev()
                            .find(|c| c.vowel == Some(Vowel::Tsere))?;
                        c.vowel = Some(Vowel::Patah);
                        Some(hebrew::render(&seq))
                    })
                    .flatten();
                // III-He Qal infinitive absolute vav-holam twin: hāyô הָיוֹ
                // beside hāyōh הָיֹה.
                let lamed_he_inf_abs_vav = (binyan == Binyan::Qal
                    && form == Form::InfinitiveAbsolute
                    && root.lamed() == letter::HE)
                    .then(|| {
                        let mut seq = hebrew::parse_pointed(&text);
                        let n = seq.len();
                        (n >= 2
                            && seq[n - 1].letter == letter::HE
                            && seq[n - 1].vowel.is_none()
                            && seq[n - 2].vowel == Some(Vowel::Holam))
                        .then(|| {
                            seq[n - 2].vowel = None;
                            seq[n - 1] = Cons::new(letter::VAV).with_vowel(Vowel::Holam);
                            hebrew::render(&seq)
                        })
                    })
                    .flatten();
                // Pual participle qamats twin (מְאָדָּם beside מְאֻדָּם).
                let pual_ptcp_qamats = (binyan == Binyan::Pual
                    && matches!(form, Form::ParticipleActive | Form::ParticiplePassive))
                .then(|| {
                    let mut seq = hebrew::parse_pointed(&text);
                    let c = seq.iter_mut().find(|c| c.vowel == Some(Vowel::Qubuts))?;
                    c.vowel = Some(Vowel::Qamats);
                    Some(hebrew::render(&seq))
                })
                .flatten();
                // III-aleph Piel/Pual/Hithpael consonantal perfect quiescent
                // twin (קִנֵּאתִי beside the strong קִנַּאְתִּי).
                let lamed_aleph_derived_perf =
                    (matches!(binyan, Binyan::Piel | Binyan::Pual | Binyan::Hithpael)
                        && form == Form::Perfect
                        && root.lamed() == letter::ALEF
                        && perfect_suffix_kind(pgn) == Suffix::Consonantal)
                        .then(|| lamed_aleph_derived_perfect_variant(&text))
                        .flatten();
                // Paragogic-nun twin of the vocalic-suffix imperfect (tōʔmᵊrûn
                // תֹּאמְרוּן, yᵊšûḇûn). Any binyan; the long imperfect only.
                let paragogic_nun = (form == Form::Imperfect
                    && imperfect_suffix_kind(pgn) == Suffix::Vocalic)
                    .then(|| paragogic_nun_variant(&text));
                // Propretonic-reduced paragogic-nun twin of the hollow Qal
                // imperfect plural (yāšûḇû → yᵉšûḇûn יְשׁוּבוּן, yᵉqûmûn).
                let hollow_paragogic_nun = (binyan == Binyan::Qal
                    && root.has(Gizra::Hollow)
                    && form == Form::Imperfect
                    && imperfect_suffix_kind(pgn) == Suffix::Vocalic)
                    .then(|| hollow_paragogic_nun_variant(&text))
                    .flatten();
                // Theme-restored paragogic-nun twins (yišmāʕûn תִּשְׁמָעוּן,
                // tōʔḇēḏûn תֹּאבֵדוּן) — any binyan, the -û plural imperfect.
                let paragogic_nun_theme: Vec<String> =
                    if form == Form::Imperfect && imperfect_suffix_kind(pgn) == Suffix::Vocalic {
                        paragogic_nun_theme_variants(&text)
                    } else {
                        Default::default()
                    };
                // Pausal theme-restored plural (yišmāʕû יִשְׁמָעוּ), the same
                // restored grade minus the energic nun.
                let pausal_plural: Vec<String> = if matches!(form, Form::Imperfect | Form::Jussive)
                    && imperfect_suffix_kind(pgn) == Suffix::Vocalic
                {
                    pausal_imperfect_plural_variants(&text)
                } else {
                    Default::default()
                };
                // I-guttural o-theme imperfect plural with the holam reduced
                // (yaḥpōrû → yaḥpᵊrû יַחְפְּרוּ).
                let iguttural_reduced = (binyan == Binyan::Qal
                    && root.has(Gizra::PeGuttural)
                    && form == Form::Imperfect
                    && imperfect_suffix_kind(pgn) == Suffix::Vocalic)
                    .then(|| iguttural_reduced_plural_variant(&text))
                    .flatten();
                // I-vav Niphal imperfect vav-doubling twins (וַיִּוָּעַץ).
                let pe_yod_niphal_vav: Vec<String> = if binyan == Binyan::Niphal
                    && root.has(Gizra::PeYod)
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                {
                    pe_yod_niphal_vav_variants(&text)
                } else {
                    Default::default()
                };
                // I-nun-style I-yod Qal imperfect (יצק → יִצֹק).
                let pe_yod_as_pe_nun = (binyan == Binyan::Qal
                    && root.has(Gizra::PeYod)
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol))
                .then(|| pe_yod_as_pe_nun_variant(&text))
                .flatten();
                // I-aleph Qal patah-pattern imperfect/wayyiqtol (וַיַּאַסְפוּ).
                let pe_aleph_patah = (binyan == Binyan::Qal
                    && root.pe() == letter::ALEF
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol))
                .then(|| pe_aleph_patah_variant(&text))
                .flatten();
                // Pausal twin of the quiescent I-aleph wayyiqtol: the nesiga
                // segol (וַיֹּאמֶר) keeps its patah in pause — וַיֹּאמַר,
                // וַיֹּאכַל, וַתֹּאכַל.
                let pe_aleph_pausal = (binyan == Binyan::Qal
                    && root.pe() == letter::ALEF
                    && form == Form::Wayyiqtol
                    && imperfect_suffix_kind(pgn) == Suffix::Zero)
                    .then(|| pe_aleph_wayyiqtol_pausal_variant(&text))
                    .flatten();
                // Stative (e-class) Qal perfect 3ms twin: qāṭēl beside qāṭal —
                // ṭāmēʾ טָמֵא, mālēʾ מָלֵא, yārēʾ יָרֵא.
                let qal_stative_perfect = (binyan == Binyan::Qal
                    && form == Form::Perfect
                    && pgn == Pgn::new(Person::Third, Gender::Masculine, Number::Singular))
                .then(|| perfect_stative_tsere_variant(&text))
                .flatten();
                // Tsere-kept twin of the Hiphil short wayyiqtol: nesiga is not
                // universal — וַיַּגֵּד beside וַיַּגֶּד, וַיַּקְרֵב, וַיַּשְׁלֵךְ.
                let hiphil_wayyiqtol_tsere = (binyan == Binyan::Hiphil
                    && form == Form::Wayyiqtol
                    && imperfect_suffix_kind(pgn) == Suffix::Zero)
                    .then(|| final_segol_to_tsere_variant(&text))
                    .flatten();
                // Long (imperfect-shaped) twin of the III-He jussive: the
                // apocope is not obligatory — יַעֲשֶׂה, יִהְיֶה are tagged
                // jussive in context.
                let jussive_long = (form == Form::Jussive
                    && root.lamed() == letter::HE
                    && imperfect_suffix_kind(pgn) == Suffix::Zero)
                    .then(|| {
                        let (t, _) = generate_one(root, binyan, Form::Imperfect, pgn, false);
                        (t != text).then_some(t)
                    })
                    .flatten();
                // Long twin of the short-based wayyiqtol: the 1cs regularly
                // keeps the long form (וָאֶרְאֶה beside וָאֵרֶא), and the
                // III-He third persons attest it too (וַיַּכֶּה, וַיַּעֲשֶׂה).
                let wayyiqtol_long = (form == Form::Wayyiqtol
                    && (pgn.person == Some(Person::First) || root.lamed() == letter::HE)
                    && imperfect_suffix_kind(pgn) == Suffix::Zero)
                    .then(|| {
                        let (t, _) = build_wayyiqtol_long(root, binyan, pgn, false);
                        (t != text).then_some(t)
                    })
                    .flatten();
                // Cohortative-shaped 1cs/1cp wayyiqtol twin (the paragogic -â
                // under the consecutive vav): וָאֹמְרָה, וָאֶתְּנָה, וָאֶשְׁלְחָה.
                let wayyiqtol_cohortative = (form == Form::Wayyiqtol
                    && pgn.person == Some(Person::First))
                .then(|| {
                    let (t, att) = generate_one(root, binyan, Form::Cohortative, pgn, false);
                    att.then(|| {
                        let mut seq = hebrew::parse_pointed(&t);
                        let aleph = seq.first().is_some_and(|c| c.letter == letter::ALEF);
                        if !aleph && let Some(first) = seq.first_mut() {
                            first.dagesh = true;
                        }
                        seq.insert(
                            0,
                            Cons::new(letter::VAV).with_vowel(if aleph {
                                Vowel::Qamats
                            } else {
                                Vowel::Patah
                            }),
                        );
                        hebrew::render(&seq)
                    })
                })
                .flatten();
                // Paragogic-he cohortative twins built from the imperfect
                // surface — the weak stems (hollow/I-yod Hiphil, Niphal) the
                // index-based cohortative builder mangles: אַגִּידָה, וְאָשִׁיבָה,
                // אוֹדִיעָה, וְנִוָּשֵׁעָה.
                let cohortative_long: Vec<String> = if form == Form::Cohortative {
                    {
                        let (impf, _) = generate_one(root, binyan, Form::Imperfect, pgn, false);
                        let mut v = cohortative_paragogic_variants(&impf);
                        // Compose with the I-guttural silent-sheva grade of
                        // the host (ʾeʿlōzâ אֶעְלֹזָה, wᵉnaʿbᵊrâ וְנַעְבְּרָה) and
                        // the I-vav Niphal grade (ʾiwwāšēʿâ וְאִוָּשֵׁעָה).
                        for s in qal_iguttural_silent_sheva_variants(&impf)
                            .into_iter()
                            .chain(pe_yod_niphal_vav_variants(&impf).into_iter().flat_map(|s| {
                                // The 1cs hiriq grade rides along
                                // (ʾiwwāšēaʿ אִוָּשֵׁעַ beside אֶוָּשֵׁעַ).
                                let hiriq = niphal_1cs_hiriq_variant(&s);
                                std::iter::once(s).chain(hiriq)
                            }))
                        {
                            v.extend(cohortative_paragogic_variants(&s));
                        }
                        v
                    }
                } else {
                    Default::default()
                };
                // Their vav-consecutive twins on the 1st-person wayyiqtol slot
                // (וָאַעֲמִידָה, וָאִמָּלְטָה, וָאֶשְׁקֲלָה).
                let wayyiqtol_cohortative_long: Vec<String> = if form == Form::Wayyiqtol
                    && pgn.person == Some(Person::First)
                {
                    {
                        let (impf, _) = generate_one(root, binyan, Form::Imperfect, pgn, false);
                        let mut hosts = cohortative_paragogic_variants(&impf);
                        for s in qal_iguttural_silent_sheva_variants(&impf) {
                            hosts.extend(cohortative_paragogic_variants(&s));
                        }
                        hosts
                            .into_iter()
                            .map(|t| {
                                let mut seq = hebrew::parse_pointed(&t);
                                let aleph = seq.first().is_some_and(|c| c.letter == letter::ALEF);
                                if !aleph && let Some(first) = seq.first_mut() {
                                    first.dagesh = true;
                                }
                                seq.insert(
                                    0,
                                    Cons::new(letter::VAV).with_vowel(if aleph {
                                        Vowel::Qamats
                                    } else {
                                        Vowel::Patah
                                    }),
                                );
                                hebrew::render(&seq)
                            })
                            .collect()
                    }
                } else {
                    Default::default()
                };
                // Hollow Niphal participle — the ms is identical to the
                // contracted perfect 3ms (nāḵôn נָכוֹן); the inflected forms
                // reduce the prefix qamats: nᵊḵōnîm נְכֹנִים, nᵊḵônâ נְכוֹנָה,
                // nᵊḵōnôṯ (defective; plene matches via the holam collapse).
                let hollow_niphal_participle: Vec<String> = if binyan == Binyan::Niphal
                    && root.has(Gizra::Hollow)
                    && form == Form::ParticipleActive
                {
                    if pgn == Pgn::gn(Gender::Masculine, Number::Singular) {
                        hollow_niphal_perfect_variant(
                            root,
                            Pgn::new(Person::Third, Gender::Masculine, Number::Singular),
                        )
                        .map(|seq| vec![hebrew::render(&seq)])
                        .unwrap_or_default()
                    } else {
                        let c3 = rad(root.lamed(), 3);
                        let tail: Option<Vec<Cons>> =
                            if pgn == Pgn::gn(Gender::Masculine, Number::Plural) {
                                Some(vec![
                                    c3.with_vowel(Vowel::Hiriq),
                                    Cons::mater(letter::YOD),
                                    Cons::new(letter::MEM),
                                ])
                            } else if pgn == Pgn::gn(Gender::Feminine, Number::Singular) {
                                Some(vec![c3.with_vowel(Vowel::Qamats), Cons::new(letter::HE)])
                            } else if pgn == Pgn::gn(Gender::Feminine, Number::Plural) {
                                Some(vec![
                                    c3,
                                    Cons::new(letter::VAV).with_vowel(Vowel::Holam),
                                    Cons::new(letter::TAV),
                                ])
                            } else {
                                None
                            };
                        tail.map(|tail| {
                            let mut seq = vec![
                                Cons::new(letter::NUN).with_vowel(Vowel::Sheva),
                                rad(root.pe(), 1).with_vowel(Vowel::Holam),
                            ];
                            seq.extend(tail);
                            vec![hebrew::render(&seq)]
                        })
                        .unwrap_or_default()
                    }
                } else {
                    Vec::new()
                };
                // Patah twin of the hollow short wayyiqtol: the retracted
                // qamats (Qal וַיָּסָר) and the î-class nesiga segol (Hiphil
                // וַיָּסֶר) both have an attested patah grade — וַיָּסַר for
                // either binyan, וַיָּצַר.
                let hollow_wayyiqtol_patah = (matches!(binyan, Binyan::Qal | Binyan::Hiphil)
                    && root.has(Gizra::Hollow)
                    && form == Form::Wayyiqtol
                    && imperfect_suffix_kind(pgn) == Suffix::Zero)
                    .then(|| hollow_wayyiqtol_patah_variant(&text))
                    .flatten();
                // Plene -ênâ twin of the hollow Qal 3fp/2fp imperfect/wayyiqtol
                // (tᵊqûmênâ תְּקוּמֶינָה beside tāqûmnâ).
                let hollow_impf_fp_plene = (binyan == Binyan::Qal
                    && root.has(Gizra::Hollow)
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                    && pgn.gender == Some(Gender::Feminine)
                    && pgn.number == Some(Number::Plural))
                .then(|| hollow_imperfect_fp_plene_variant(&text))
                .flatten();
                // ô-class hollow Qal perfect twin: the stative holam grade —
                // ṭôḇ (טוֹב), bōšû (בֹּשׁוּ) — beside the default qām qamats. Before
                // a consonantal afformative the stem vowel is a short patah in the
                // default grade (baštî בַּשְׁתִּי), so the ô-class twin lifts that
                // patah to holam instead: bōštî בֹּשְׁתִּי, bōšnû בֹּשְׁנוּ.
                let hollow_perfect_holam = (binyan == Binyan::Qal
                    && root.has(Gizra::Hollow)
                    && form == Form::Perfect)
                    .then(|| {
                        let mut seq = hebrew::parse_pointed(&text);
                        let c = seq
                            .iter_mut()
                            .find(|c| matches!(c.vowel, Some(Vowel::Qamats | Vowel::Patah)))?;
                        c.vowel = Some(Vowel::Holam);
                        Some(hebrew::render(&seq))
                    })
                    .flatten();
                // Stative Qal active-participle ms twin: the e-class verbs use
                // the perfect-shaped qāṭēl as their participle — yārēʾ יָרֵא.
                let qal_stative_participle = (binyan == Binyan::Qal
                    && form == Form::ParticipleActive
                    && pgn == Pgn::gn(Gender::Masculine, Number::Singular)
                    && !root.has(Gizra::Hollow)
                    && root.lamed() != letter::HE)
                    .then(|| {
                        let mut c1 = rad(root.pe(), 1).with_vowel(Vowel::Qamats);
                        if hebrew::is_begedkefet(root.pe()) {
                            c1 = c1.with_dagesh();
                        }
                        Some(hebrew::render(&[
                            c1,
                            rad(root.ayin(), 2).with_vowel(Vowel::Tsere),
                            rad(root.lamed(), 3),
                        ]))
                    })
                    .flatten();
                // III-He apocopated imperative 2ms twin — Piel ṣaw צַו beside
                // the full צַוֵּה.
                let lamed_he_imp_apoc = (root.lamed() == letter::HE
                    && form == Form::Imperative
                    && pgn == Pgn::new(Person::Second, Gender::Masculine, Number::Singular))
                .then(|| lamed_he_imperative_apocope_variant(&text))
                .flatten();
                // Pausal twin of that apocopated imperative: at a major pause the
                // final closed-syllable patah lengthens to qamats — has הַס → hās
                // הָס (הסה). The bulk pausal pass only sees the base imperative
                // (הַסֵּה, which it rejects), so derive it from the apocope here.
                let lamed_he_imp_apoc_pausal =
                    lamed_he_imp_apoc.as_deref().and_then(pausal_qamats_variant);
                // I-guttural III-He apocopated imperative: the apocope closes the
                // final syllable on the bare last radical (haʕăl → haʕal הַעַל,
                // עלה Hiphil), so the C1 guttural's hataf-patah fills to a full
                // patah. Promote a hataf-patah guttural that a vowelless
                // (syllable-closing) consonant follows.
                let lamed_he_imp_apoc_guttural = lamed_he_imp_apoc.as_deref().and_then(|t| {
                    let mut seq = hebrew::parse_pointed(t);
                    let i = seq.iter().position(|c| {
                        hebrew::is_guttural(c.letter) && c.vowel == Some(Vowel::HatafPatah)
                    })?;
                    (seq.get(i + 1).is_some_and(|c| c.vowel.is_none())).then(|| {
                        seq[i].vowel = Some(Vowel::Patah);
                        hebrew::render(&seq)
                    })
                });
                // III-He Hiphil apocopated segholate imperative 2ms — herep̄
                // הֶרֶף (רפה), hereḇ הֶרֶב (רבה) beside the full הַרְפֵּה.
                let hiphil_imp_segholate = (binyan == Binyan::Hiphil
                    && root.lamed() == letter::HE
                    && form == Form::Imperative
                    && pgn == Pgn::new(Person::Second, Gender::Masculine, Number::Singular))
                .then(|| {
                    Some(hebrew::render(&[
                        Cons::new(letter::HE).with_vowel(Vowel::Segol),
                        rad(root.pe(), 1).with_vowel(Vowel::Segol),
                        rad(root.ayin(), 2),
                    ]))
                })
                .flatten();
                // Feminine-shaped Qal infinitive-construct twin: the -â
                // infinitives yirʾâ (לְיִרְאָה), ʾahăḇâ (לְאַהֲבָה).
                let qal_inf_fem: Vec<String> = if binyan == Binyan::Qal
                    && form == Form::InfinitiveConstruct
                    && root.lamed() != letter::HE
                {
                    {
                        let (v1, v2) = if hebrew::is_guttural(root.ayin()) {
                            (Vowel::Patah, Vowel::HatafPatah)
                        } else {
                            (Vowel::Hiriq, Vowel::Sheva)
                        };
                        // The o-grade twin with C1 qamats(-qatan) is also attested:
                        // ṭomʾâ לְטָמְאָה beside the i-grade yirʾâ לְיִרְאָה — and a
                        // guttural C2 can close that syllable on a plain silent
                        // sheva instead of the hataf (roḥṣâ לְרָחְצָה).
                        [(v1, v2), (Vowel::Qamats, v2), (Vowel::Qamats, Vowel::Sheva)]
                            .into_iter()
                            .flat_map(|(v1, v2)| {
                                let c1 = || {
                                    let mut c = rad(root.pe(), 1).with_vowel(v1);
                                    if hebrew::is_begedkefet(root.pe()) {
                                        c = c.with_dagesh();
                                    }
                                    c
                                };
                                // The free form ends in -â (C3 qamats + he mater);
                                // its construct swaps that for -aṯ (C3 patah + tav):
                                // ʾahăḇâ אַהֲבָה → ʾahăḇaṯ אַהֲבַת, yirʾâ → yirʾaṯ.
                                let free = hebrew::render(&[
                                    c1(),
                                    rad(root.ayin(), 2).with_vowel(v2),
                                    rad(root.lamed(), 3).with_vowel(Vowel::Qamats),
                                    Cons::new(letter::HE),
                                ]);
                                let cstr = hebrew::render(&[
                                    c1(),
                                    rad(root.ayin(), 2).with_vowel(v2),
                                    rad(root.lamed(), 3).with_vowel(Vowel::Patah),
                                    Cons::new(letter::TAV),
                                ]);
                                [free, cstr]
                            })
                            .collect()
                    }
                } else {
                    Default::default()
                };
                // Pausal twin of the o-class hollow wayyiqtol: stress
                // retraction shortened the holam to qamats (וַיָּמָת), but in
                // pause the holam survives — וַיָּמֹת, וַיָּנֹס.
                let hollow_wayyiqtol_pausal = (binyan == Binyan::Qal
                    && root.has(Gizra::Hollow)
                    && hollow_class(root) == HollowClass::Shureq
                    && form == Form::Wayyiqtol
                    && imperfect_suffix_kind(pgn) == Suffix::Zero)
                    .then(|| hollow_wayyiqtol_pausal_variant(&text))
                    .flatten();
                // III-guttural fs segolate participle patah twin: šōmaʿaṯ
                // שֹׁמַעַת beside שֹׁמֶעֶת.
                let lamed_guttural_fs_ptcp =
                    (matches!(form, Form::ParticipleActive | Form::ParticiplePassive)
                        && pgn == Pgn::gn(Gender::Feminine, Number::Singular)
                        && matches!(root.lamed(), letter::HET | letter::AYIN))
                    .then(|| {
                        let mut seq = hebrew::parse_pointed(&text);
                        let n = seq.len();
                        (n >= 3
                            && seq[n - 1].letter == letter::TAV
                            && seq[n - 2].vowel == Some(Vowel::Segol)
                            && seq[n - 3].vowel == Some(Vowel::Segol))
                        .then(|| {
                            seq[n - 2].vowel = Some(Vowel::Patah);
                            seq[n - 3].vowel = Some(Vowel::Patah);
                            hebrew::render(&seq)
                        })
                    })
                    .flatten();
                // Unreduced -â fs participle twin for the derived stems: beside
                // the propretonically-reduced miṯnakkᵊrâ מִתְנַכְּרָה the thematic
                // tsere is often retained — miṯnakkērâ מִתְנַכֵּרָה. Build the ms
                // base and append -â (C3 qamats + he) without reducing. Strong
                // roots only (a weak ms base would be mis-shaped).
                let ptcp_fs_unreduced_a = (matches!(
                    form,
                    Form::ParticipleActive | Form::ParticiplePassive
                ) && pgn == Pgn::gn(Gender::Feminine, Number::Singular)
                    && binyan != Binyan::Qal)
                .then(|| {
                    // The -â form ends [...C2-sheva][C3-qamats][he]; the C2 sheva
                    // is the propretonically reduced thematic tsere — restore it.
                    let mut seq = hebrew::parse_pointed(&text);
                    let n = seq.len();
                    (n >= 3
                        && seq[n - 1].letter == letter::HE
                        && seq[n - 1].vowel.is_none()
                        && seq[n - 2].vowel == Some(Vowel::Qamats)
                        && seq[n - 3].vowel == Some(Vowel::Sheva))
                    .then(|| {
                        seq[n - 3].vowel = Some(Vowel::Tsere);
                        hebrew::render(&seq)
                    })
                })
                .flatten();
                // III-He participle fs construct: the -â feminine (qamats-he,
                // maʕălâ מַעֲלָה, ʕōśâ עֹשָׂה) bound form replaces the he with a
                // tav and shortens the qamats to patah — maʕălaṯ מַעֲלַת,
                // ʕōśaṯ עֹשַׂת. Spans every binyan's III-He active/passive ptcp.
                let lamed_he_ptcp_fs_cstr = (root.lamed() == letter::HE
                    && matches!(form, Form::ParticipleActive | Form::ParticiplePassive)
                    && pgn == Pgn::gn(Gender::Feminine, Number::Singular))
                .then(|| lamed_he_participle_fs_construct_variant(&text))
                .flatten();
                // Hollow Qal participle twins: the stative tsere class (mēṯ
                // מֵת, mēṯîm מֵתִים) beside the default qām, plus the fs
                // qamats-he alternant (bāʾâ בָּאָה) and its -aṯ construct
                // (zāḇaṯ זָבַת).
                let hollow_participle: Vec<String> = if binyan == Binyan::Qal
                    && root.has(Gizra::Hollow)
                    && form == Form::ParticipleActive
                {
                    hollow_participle_twins(root, pgn)
                } else {
                    Default::default()
                };
                // III-He Qal doubled-final apocope wayyiqtol (וַיֵּשְׁתְּ, וַתֵּבְךְּ).
                let lamed_he_doubled_apoc = (binyan == Binyan::Qal
                    && root.lamed() == letter::HE
                    && form == Form::Wayyiqtol
                    && imperfect_suffix_kind(pgn) == Suffix::Zero)
                    .then(|| lamed_he_doubled_apocope_variant(root, &text))
                    .flatten();
                // ראה tsere-segol jussive/imperfect 3ms: beside the patah-prefix
                // apocopated yarʔ (the וַיַּרְא base, built in apply_gizra), the
                // 3ms also surfaces with the tsere prefix over a segol C1 — yēreʔ
                // יֵרֶא ("let him see/appear"), matching the non-3ms tēreʔ shape.
                let raah_apoc_tsere = (is_raah(root)
                    && binyan == Binyan::Qal
                    && matches!(form, Form::Jussive | Form::Imperfect)
                    && pgn == Pgn::new(Person::Third, Gender::Masculine, Number::Singular))
                .then(|| raah_apocopated_tsere_variant(root));
                // Defective twin of the I-yod Hiphil's ô prefix: the vav mater
                // dropped and the holam written on the preformative —
                // וַיּוֹסֶף → וַיֹּסֶף, יוֹסִיף → יֹסִיף.
                let pe_yod_hiphil_defective = (binyan == Binyan::Hiphil
                    && root.has(Gizra::PeYod)
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol))
                .then(|| holam_vav_defective_variant(&text))
                .flatten();
                // III-He Hiphil apocopated wayyiqtol/jussive (יַשְׁקֶה → וַיַּשְׁקְ).
                let lamed_he_hiphil_apoc = (binyan == Binyan::Hiphil
                    && root.lamed() == letter::HE
                    && matches!(form, Form::Wayyiqtol | Form::Jussive))
                .then(|| lamed_he_hiphil_apocope_variant(&text))
                .flatten();
                // Pausal o-theme imperative/inf-construct twin (אֱמֹר → אֱמָר),
                // and the same lengthening in the zero-suffix imperfect
                // (yaʿăḇōr → yaʿăḇār יַעֲבָר).
                let pausal_qotol = (binyan == Binyan::Qal
                    && (matches!(form, Form::Imperative | Form::InfinitiveConstruct)
                        || matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                            && imperfect_suffix_kind(pgn) == Suffix::Zero))
                    .then(|| pausal_qotol_variant(&text))
                    .flatten();
                // Reduced/construct grade of the Qal passive participle: the
                // C1 qamats drops to sheva — lᵊḇûš לְבוּשׁ (matching defective
                // לְבֻשׁ via the shureq collapse), nᵊśûʾ וּנְשׂוּא.
                let qal_pass_ptcp_reduced = (binyan == Binyan::Qal
                    && form == Form::ParticiplePassive)
                    .then(|| {
                        let mut seq = hebrew::parse_pointed(&text);
                        (seq.first().and_then(|c| c.vowel) == Some(Vowel::Qamats)).then(|| {
                            seq[0].vowel = Some(Vowel::Sheva);
                            // A I-guttural C1 cannot bear the bare vocal sheva —
                            // it takes a hataf-patah: ḥălûṣ חֲלוּץ, not חְלוּץ.
                            apply_guttural(&mut seq, root);
                            hebrew::render(&seq)
                        })
                    })
                    .flatten();
                // I-guttural Qal imperfect-family silent-sheva twin (אֶעְבְּרָה).
                let qal_iguttural_silent: Vec<String> = if binyan == Binyan::Qal
                    && matches!(
                        form,
                        Form::Imperfect | Form::Cohortative | Form::Jussive | Form::Wayyiqtol
                    )
                    && root.has(Gizra::PeGuttural)
                {
                    qal_iguttural_silent_sheva_variants(&text)
                } else {
                    Default::default()
                };
                // Nun-retained I-nun Qal imperative twin (נְטֵה, נְצֹר).
                let pe_nun_imperative_retained =
                    (binyan == Binyan::Qal && form == Form::Imperative && root.pe() == letter::NUN)
                        .then(|| pe_nun_imperative_retained_variant(&text))
                        .flatten();
                // III-guttural perfect 2fs helping-patah twin (yāḏaʕt → yāḏaʕat
                // יָדַעַתְּ, hišbaʕat, lāqaḥat): a final het/ayin can't close the
                // syllable before the -t, so a furtive helping patah surfaces.
                let lamed_guttural_perf_2fs = (form == Form::Perfect
                    && pgn == Pgn::new(Person::Second, Gender::Feminine, Number::Singular)
                    && matches!(root.lamed(), letter::HET | letter::AYIN))
                .then(|| lamed_guttural_perfect_2fs_helping_variant(&text))
                .flatten();
                // III-He Piel/Pual consonantal-suffix perfect tsere twin: the
                // linking hiriq-yod (ṣiwwîṯî צִוִּיתִי) has an attested
                // tsere-yod spelling — ṣiwwêṯî צִוֵּיתִי, ṣuwwêṯî צֻוֵּיתִי.
                let lamed_he_perf_tsere = (matches!(binyan, Binyan::Piel | Binyan::Pual)
                    && form == Form::Perfect
                    && root.lamed() == letter::HE
                    && perfect_suffix_kind(pgn) == Suffix::Consonantal)
                    .then(|| lamed_he_perfect_tsere_variant(&text))
                    .flatten();
                // The Hiphil's reverse twin: built tsere-yod (הִכֵּיתָ), the
                // hiriq-yod spelling also attested (הִכִּיתָ).
                let lamed_he_perf_hiriq = (binyan == Binyan::Hiphil
                    && form == Form::Perfect
                    && root.lamed() == letter::HE
                    && matches!(
                        perfect_suffix_kind(pgn),
                        Suffix::Consonantal | Suffix::Heavy
                    ))
                    .then(|| lamed_he_perfect_hiriq_variant(&text))
                    .flatten();
                // III-He Hophal consonantal-suffix perfect tsere twin: the
                // Hophal builds the linking vowel as hiriq-yod (hoglîṯî
                // הׇגְלִיתִי) but the tsere-yod spelling is attested too
                // (hoglêṯî), mirroring the Hiphil. Additive.
                let lamed_he_hophal_perf_tsere = (binyan == Binyan::Hophal
                    && form == Form::Perfect
                    && root.lamed() == letter::HE
                    && matches!(
                        perfect_suffix_kind(pgn),
                        Suffix::Consonantal | Suffix::Heavy
                    ))
                    .then(|| lamed_he_perfect_tsere_variant(&text))
                    .flatten();
                // I-guttural Hophal loud-preformative twin (הָחֳלֵיתִי, הָעֳמַד):
                // the guttural C1 trades its silent sheva for a hataf-qamats and
                // the prefix qamats-qatan opens to a full qamats.
                let iguttural_hophal_loud = (binyan == Binyan::Hophal
                    && root.has(Gizra::PeGuttural))
                .then(|| iguttural_hophal_loud_preformative_variant(&text))
                .flatten();
                // The loud preformative also composes with the III-He tsere
                // twin (hoḥŏlêṯî הָחֳלֵיתִי), so chain the two transforms.
                let iguttural_hophal_loud_tsere = lamed_he_hophal_perf_tsere
                    .as_deref()
                    .and_then(iguttural_hophal_loud_preformative_variant);
                // I-guttural Niphal he-prefix compensatory-lengthening twin: in
                // the he-prefixed imperative / infinitive the C1 ayin/het reject
                // the doubling and — unlike the doubling binyanim's virtual
                // doubling — the Niphal prefix lengthens its hiriq to tsere:
                // hēʕāṣēr הֵעָצֵר, hēḥāšēḇ הֵחָשֵׁב (the imperfect yēʕāṣēr already
                // shows it; aleph lengthens through apply_guttural).
                let niphal_iguttural_he_tsere = (binyan == Binyan::Niphal
                    && matches!(
                        form,
                        Form::Imperative
                            | Form::InfinitiveConstruct
                            | Form::InfinitiveAbsolute
                    )
                    && matches!(root.pe(), letter::AYIN | letter::HET))
                .then(|| {
                    let mut seq = hebrew::parse_pointed(&text);
                    (seq.len() >= 2
                        && seq[0].letter == letter::HE
                        && seq[0].vowel == Some(Vowel::Hiriq)
                        && seq[1].letter == root.pe())
                    .then(|| {
                        seq[0].vowel = Some(Vowel::Tsere);
                        hebrew::render(&seq)
                    })
                })
                .flatten();
                // Patah-theme Piel/Hithpael perfect 3ms twin (בֵּרַךְ, חִשַּׁב,
                // טִהַר, הִתְחַזַּק).
                let piel_perf_patah = (matches!(binyan, Binyan::Piel | Binyan::Hithpael)
                    && form == Form::Perfect
                    && pgn == Pgn::new(Person::Third, Gender::Masculine, Number::Singular))
                .then(|| piel_perfect_patah_theme_variant(&text))
                .flatten();
                // I-grade twin of the Piel/Hithpael perfect before a
                // consonantal/heavy afformative: the theme holds hiriq instead
                // of lowering to patah — hiṯqaddištem הִתְקַדִּשְׁתֶּם (the
                // dominant Hithpael shape), qiddištî. Lower the doubled-C2 patah
                // back to hiriq.
                let piel_perf_hiriq = (matches!(binyan, Binyan::Piel | Binyan::Hithpael)
                    && form == Form::Perfect
                    && matches!(
                        perfect_suffix_kind(pgn),
                        Suffix::Consonantal | Suffix::Heavy
                    ))
                .then(|| {
                    let mut seq = hebrew::parse_pointed(&text);
                    let c = seq
                        .iter_mut()
                        .find(|c| c.dagesh && c.vowel == Some(Vowel::Patah))?;
                    c.vowel = Some(Vowel::Hiriq);
                    Some(hebrew::render(&seq))
                })
                .flatten();
                // Segol-prefix III-He Hiphil perfect twin (הֶגְלָה).
                let hiphil_perf_segol = (binyan == Binyan::Hiphil
                    && form == Form::Perfect
                    && root.lamed() == letter::HE)
                    .then(|| hiphil_perfect_segol_prefix_variant(&text))
                    .flatten();
                // Short III-He imperfect 3fp/2fp twin (תִּהְיֶיןָ).
                let lamed_he_fp_short = (root.lamed() == letter::HE
                    && matches!(form, Form::Imperfect | Form::Jussive)
                    && pgn.number == Some(Number::Plural)
                    && pgn.gender == Some(Gender::Feminine))
                .then(|| lamed_he_imperfect_fp_short_variant(&text))
                .flatten();
                // Pausal-tsere III-He twin (תְּגַלֵּה beside תְּגַלֶּה).
                let lamed_he_pausal = (root.lamed() == letter::HE
                    && matches!(
                        form,
                        Form::Imperfect | Form::Jussive | Form::Cohortative | Form::Imperative
                    ))
                .then(|| lamed_he_pausal_tsere_variant(&text))
                .flatten();
                // He-less -nâ spelling of the fp afformative (תָּשֹׁבְןָ).
                let fp_nah_heless = (matches!(
                    form,
                    Form::Imperfect | Form::Jussive | Form::Wayyiqtol
                ) && pgn.number == Some(Number::Plural)
                    && pgn.gender == Some(Gender::Feminine))
                .then(|| fp_nah_heless_variant(&text))
                .flatten();
                // Archaic III-He retained-yod plural (יֶאֱתָיוּ, יֶהֱמָיוּן). The
                // pe-guttural segol-grade plurals (יֶהֱמוּ) are hosts too, so the
                // guttural verbs reach their attested segol vocalisation.
                let lamed_he_retained_yod: Vec<String> = if root.lamed() == letter::HE
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Imperative)
                    && pgn.number == Some(Number::Plural)
                    && pgn.gender == Some(Gender::Masculine)
                {
                    std::iter::once(text.as_str())
                        .chain(pe_guttural_loud_segol_pl.iter().map(String::as_str))
                        .chain(pe_guttural_loud_from_silent.iter().map(String::as_str))
                        .flat_map(lamed_he_retained_yod_plural_variants)
                        .collect()
                } else {
                    Vec::new()
                };
                // III-Aleph III-He-style -ōṯ/-ôṯ infinitive construct (מַלֹּאת).
                let lamed_aleph_inf_ot: Vec<String> = if root.lamed() == letter::ALEF
                    && form == Form::InfinitiveConstruct
                    && matches!(binyan, Binyan::Qal | Binyan::Piel)
                {
                    lamed_aleph_inf_ot_variants(&text)
                } else {
                    Vec::new()
                };
                // Niphal imperfect 1cs hiriq preformative twin (אִדָּרֵשׁ).
                let niphal_1cs_hiriq: Vec<String> = if binyan == Binyan::Niphal
                    && matches!(
                        form,
                        Form::Imperfect | Form::Cohortative | Form::Jussive | Form::Wayyiqtol
                    )
                    && pgn.person == Some(Person::First)
                    && pgn.number == Some(Number::Singular)
                {
                    {
                        // The long-wayyiqtol twins (וָאֶמָּלְטָה) take the hiriq
                        // grade too — the transforms compose.
                        std::iter::once(text.as_str())
                            .chain(wayyiqtol_cohortative_long.iter().map(|s| s.as_str()))
                            .filter_map(niphal_1cs_hiriq_variant)
                            .collect()
                    }
                } else {
                    Default::default()
                };
                // Hophal u-grade preformative twin (מֻפְקָד, הֻשְׁלַךְ).
                let hophal_qubuts = (binyan == Binyan::Hophal)
                    .then(|| hophal_qubuts_variant(&text))
                    .flatten();
                // I-guttural Niphal perfect i-grade twin (נִהְיָה, נִהְיְתָה).
                let niphal_pe_guttural_hiriq = (binyan == Binyan::Niphal
                    && form == Form::Perfect
                    && root.has(Gizra::PeGuttural))
                .then(|| pe_guttural_niphal_hiriq_variant(&text))
                .flatten();
                // I-guttural Niphal perfect hataf-segol twin: when a vocalic
                // afformative reduces C2 to a silent sheva, apply_guttural
                // promotes the C1 hataf-segol to a full segol (neʾesp̄û
                // נֶאֶסְפוּ). The MT keeps the hataf — neʾĕsp̄û נֶאֱסְפוּ,
                // neʾĕlāḥû נֶאֱלָחוּ — so restore it as a twin.
                let niphal_pe_guttural_hataf = (binyan == Binyan::Niphal
                    && form == Form::Perfect
                    && root.has(Gizra::PeGuttural))
                .then(|| {
                    let mut seq = hebrew::parse_pointed(&text);
                    let i = (1..seq.len().saturating_sub(1)).find(|&i| {
                        seq[i].letter == root.pe()
                            && seq[i].vowel == Some(Vowel::Segol)
                            && seq[i + 1].vowel == Some(Vowel::Sheva)
                    })?;
                    seq[i].vowel = Some(Vowel::HatafSegol);
                    Some(hebrew::render(&seq))
                })
                .flatten();
                // חיה/היה short imperfect segol-prefix twin (וַיֶּחִי).
                let chayah_segol = ((is_chayah(root) || is_hayah(root))
                    && binyan == Binyan::Qal
                    && matches!(form, Form::Jussive | Form::Wayyiqtol)
                    && imperfect_suffix_kind(pgn) == Suffix::Zero)
                    .then(|| sheva_prefix_segol_variant(&text))
                    .flatten();
                // שחה Hithpael: the defective infinitive (לְהִשְׁתַּחֲות) and the
                // pausal short wayyiqtol/jussive (וַיִּשְׁתָּחוּ).
                let shachah_inf_defective = (is_shachah(root)
                    && binyan == Binyan::Hithpael
                    && form == Form::InfinitiveConstruct)
                    .then(|| shachah_inf_defective_variant(&text))
                    .flatten();
                let shachah_pausal = (is_shachah(root)
                    && binyan == Binyan::Hithpael
                    && matches!(form, Form::Jussive | Form::Wayyiqtol)
                    && imperfect_suffix_kind(pgn) == Suffix::Zero)
                    .then(|| shachah_pausal_qamats_variant(&text))
                    .flatten();
                // Pausal segholate infinitive twin (לָשָׁבֶת).
                let segholate_inf_pausal = (binyan == Binyan::Qal
                    && form == Form::InfinitiveConstruct)
                    .then(|| segholate_inf_pausal_variant(&text))
                    .flatten();
                // Compensatory-qamats twin of the virtually-doubled Piel
                // family: before a non-doubling ה/ח/ע the theme patah may
                // lengthen instead of holding short — bāʿēr (לְבָעֵר) beside
                // the default baʿēr. (Resh/aleph already lengthen in
                // apply_guttural.)
                let piel_guttural_qamats = (matches!(binyan, Binyan::Piel | Binyan::Hithpael)
                    && matches!(
                        form,
                        Form::InfinitiveConstruct
                            | Form::Imperfect
                            | Form::Jussive
                            | Form::Wayyiqtol
                            | Form::Imperative
                    )
                    && matches!(root.ayin(), letter::HE | letter::HET | letter::AYIN))
                .then(|| {
                    let mut seq = hebrew::parse_pointed(&text);
                    let i = (1..seq.len()).find(|&i| {
                        seq[i].letter == root.ayin() && seq[i - 1].vowel == Some(Vowel::Patah)
                    })?;
                    seq[i - 1].vowel = Some(Vowel::Qamats);
                    Some(hebrew::render(&seq))
                })
                .flatten();
                // Niphal imperfect 3fp/2fp patah-theme twin: tēʾāḵalnâ
                // (תֵּאָכַלְנָה), tizzāḵarnâ (תִּזָּכַרְנָה) beside the builder's
                // tsere.
                let niphal_impf_fp_patah = (binyan == Binyan::Niphal
                    && matches!(form, Form::Imperfect | Form::Jussive)
                    && pgn.gender == Some(Gender::Feminine)
                    && pgn.number == Some(Number::Plural))
                .then(|| {
                    let mut seq = hebrew::parse_pointed(&text);
                    let n = seq.len();
                    (n >= 4
                        && seq[n - 1].letter == letter::HE
                        && seq[n - 2].letter == letter::NUN
                        && seq[n - 2].vowel == Some(Vowel::Qamats)
                        && seq[n - 3].vowel == Some(Vowel::Sheva)
                        && seq[n - 4].vowel == Some(Vowel::Tsere))
                    .then(|| {
                        seq[n - 4].vowel = Some(Vowel::Patah);
                        hebrew::render(&seq)
                    })
                })
                .flatten();
                // Niphal imperfect a-theme twin for the zero-suffix subjects:
                // yēʾāmar יֵאָמַר, yizzāḵar beside the builder's tsere.
                let niphal_impf_a = (binyan == Binyan::Niphal
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                    && imperfect_suffix_kind(pgn) == Suffix::Zero)
                    .then(|| {
                        let mut seq = hebrew::parse_pointed(&text);
                        let n = seq.len();
                        (n >= 2
                            && seq[n - 1].vowel.is_none()
                            && seq[n - 2].vowel == Some(Vowel::Tsere))
                        .then(|| {
                            seq[n - 2].vowel = Some(Vowel::Patah);
                            hebrew::render(&seq)
                        })
                    })
                    .flatten();
                // I-guttural Hiphil perfect patah-he twin: haʿălîṯā
                // (וְהַעֲלִיתָ), haʿămaḏtā beside the builder's segol grade
                // (הֶעֱלָה). Applied to the hiriq-yod twin as well, since the
                // two transforms compose (הַעֲלִיתָ needs both).
                let hiphil_perf_patah_he: Vec<String> = if binyan == Binyan::Hiphil
                    && form == Form::Perfect
                    && root.has(Gizra::PeGuttural)
                {
                    {
                        std::iter::once(text.as_str())
                            .chain(lamed_he_perf_hiriq.as_deref())
                            .filter_map(|t| {
                                let mut seq = hebrew::parse_pointed(t);
                                (seq.len() >= 2
                                    && seq[0].letter == letter::HE
                                    && seq[0].vowel == Some(Vowel::Segol))
                                .then(|| {
                                    seq[0].vowel = Some(Vowel::Patah);
                                    if seq[1].vowel == Some(Vowel::HatafSegol) {
                                        seq[1].vowel = Some(Vowel::HatafPatah);
                                    }
                                    hebrew::render(&seq)
                                })
                            })
                            .collect()
                    }
                } else {
                    Default::default()
                };
                // Segolate ־ֶת feminine-participle twin for the derived stems,
                // beside the default -â: mᵊḏabbereṯ מְדַבֶּרֶת, mušleḵeṯ
                // מֻשְׁלֶכֶת, and the Hiphil with its î dropping (maqṭeleṯ).
                let fs_et = |t: &str| -> Option<String> {
                    let mut seq = hebrew::parse_pointed(t);
                    let n = seq.len();
                    if n < 3
                        || seq[n - 1].letter != letter::HE
                        || seq[n - 2].vowel != Some(Vowel::Qamats)
                    {
                        return None;
                    }
                    seq.pop();
                    let n = seq.len();
                    seq[n - 1].vowel = Some(Vowel::Segol);
                    if n >= 3
                        && seq[n - 2].letter == letter::YOD
                        && seq[n - 2].vowel.is_none()
                        && seq[n - 3].vowel == Some(Vowel::Hiriq)
                    {
                        // Hiphil: the î mater drops and its hiriq lowers.
                        seq.remove(n - 2);
                        seq[n - 3].vowel = Some(Vowel::Segol);
                    } else if n >= 2
                        && matches!(
                            seq[n - 2].vowel,
                            Some(Vowel::Sheva) | Some(Vowel::Qamats) | Some(Vowel::Hiriq)
                        )
                    {
                        seq[n - 2].vowel = Some(Vowel::Segol);
                    } else {
                        return None;
                    }
                    seq.push(Cons::new(letter::TAV));
                    Some(hebrew::render(&seq))
                };
                let derived_fs_ptcp_et: Vec<String> = if binyan != Binyan::Qal
                    && matches!(form, Form::ParticipleActive | Form::ParticiplePassive)
                    && pgn == Pgn::gn(Gender::Feminine, Number::Singular)
                    && !root.has(Gizra::Hollow)
                    && root.ayin() != root.lamed()
                    && !matches!(root.lamed(), letter::HE | letter::ALEF)
                {
                    {
                        // The Hophal qubuts twin is a host too (mušleḵeṯ מֻשְׁלֶכֶת
                        // beside the qamats-qatan grade).
                        std::iter::once(text.as_str())
                            .chain(hophal_qubuts.as_deref())
                            .filter_map(fs_et)
                            .collect()
                    }
                } else {
                    Default::default()
                };
                // III-aleph contracted fs participle twin: the aleph quiesces
                // and the segolate pair contracts to tsere — yōṣēṯ יֹצֵאת
                // beside יֹצֶאֶת.
                let lamed_aleph_fs_ptcp = (root.lamed() == letter::ALEF
                    && matches!(form, Form::ParticipleActive | Form::ParticiplePassive)
                    && pgn == Pgn::gn(Gender::Feminine, Number::Singular))
                .then(|| {
                    let mut seq = hebrew::parse_pointed(&text);
                    let n = seq.len();
                    (n >= 3
                        && seq[n - 1].letter == letter::TAV
                        && seq[n - 2].letter == letter::ALEF
                        && seq[n - 2].vowel == Some(Vowel::Segol)
                        && seq[n - 3].vowel == Some(Vowel::Segol))
                    .then(|| {
                        seq[n - 2].vowel = None;
                        seq[n - 3].vowel = Some(Vowel::Tsere);
                        hebrew::render(&seq)
                    })
                })
                .flatten();
                // חיה: the monosyllabic Qal perfect 3ms ḥay חַי ("he lives"),
                // the geminate-style contraction beside the III-He חָיָה.
                let chayah_perf = (is_chayah(root)
                    && binyan == Binyan::Qal
                    && form == Form::Perfect
                    && pgn == Pgn::new(Person::Third, Gender::Masculine, Number::Singular))
                .then(|| {
                    hebrew::render(&[
                        rad(root.pe(), 1).with_vowel(Vowel::Patah),
                        rad(root.ayin(), 2),
                    ])
                });
                // מאן: the Piel participle haplologizes its mem prefix onto
                // the radical — māʾēn מָאֵן for the expected מְמָאֵן.
                let maen_ptcp = (binyan == Binyan::Piel
                    && form == Form::ParticipleActive
                    && pgn == Pgn::gn(Gender::Masculine, Number::Singular)
                    && root.pe() == letter::MEM
                    && root.ayin() == letter::ALEF
                    && root.lamed() == letter::NUN)
                    .then(|| {
                        hebrew::render(&[
                            rad(letter::MEM, 1).with_vowel(Vowel::Qamats),
                            rad(letter::ALEF, 2).with_vowel(Vowel::Tsere),
                            rad(letter::NUN, 3),
                        ])
                    });
                // I-guttural Niphal participle patah-prefix twin: naḥlâ
                // נַחְלָה beside the segol grade נֶחְלָה.
                let niphal_pe_guttural_ptcp_a: Vec<String> = if binyan == Binyan::Niphal
                    && matches!(form, Form::ParticipleActive | Form::ParticiplePassive)
                    && root.has(Gizra::PeGuttural)
                {
                    let mut seq = hebrew::parse_pointed(&text);
                    if seq.first().and_then(|c| c.vowel) != Some(Vowel::Segol) {
                        Vec::new()
                    } else {
                        seq[0].vowel = Some(Vowel::Patah);
                        let a = hebrew::render(&seq);
                        // The guttural's hataf also closes onto a silent sheva
                        // (naḥlâ נַחְלָה) — the transforms compose.
                        let silent = guttural_silent_sheva_variant(&a);
                        std::iter::once(a).chain(silent).collect()
                    }
                } else {
                    Vec::new()
                };
                // III-He Qal fs participle with the consonantal doubled yod —
                // pōriyyâ פֹּרִיָּה beside the contracted פֹּרָה.
                let lamed_he_fs_ptcp_iyya = (binyan == Binyan::Qal
                    && form == Form::ParticipleActive
                    && pgn == Pgn::gn(Gender::Feminine, Number::Singular)
                    && root.lamed() == letter::HE)
                    .then(|| {
                        let mut seq = vec![
                            rad(root.pe(), 1).with_vowel(Vowel::Holam),
                            rad(root.ayin(), 2).with_vowel(Vowel::Hiriq),
                            Cons::new(letter::YOD)
                                .with_dagesh()
                                .with_vowel(Vowel::Qamats),
                            Cons::new(letter::HE),
                        ];
                        if hebrew::is_begedkefet(root.pe()) {
                            seq[0] = seq[0].with_dagesh();
                        }
                        hebrew::render(&seq)
                    });
                // III-He infinitive-construct -ōh twin: the rare he-final
                // spelling ʿăśōh עֲשֹׂה beside the regular עֲשׂוֹת.
                let lamed_he_inf_oh = (root.lamed() == letter::HE
                    && binyan == Binyan::Qal
                    && form == Form::InfinitiveConstruct)
                    .then(|| {
                        let mut seq = vec![
                            rad(root.pe(), 1).with_vowel(Vowel::Sheva),
                            rad(root.ayin(), 2).with_vowel(Vowel::Holam),
                            Cons::new(letter::HE),
                        ];
                        apply_guttural(&mut seq, root);
                        hebrew::render(&seq)
                    });
                // The same -ōh he-final twin for the derived-stem III-He
                // infinitive construct (hērāʾōh הֵרָאֹה beside hērāʾôṯ הֵרָאוֹת,
                // hēʿāśōh הֵעָשֹׂה): derive it from the generated -ôṯ text by
                // dropping the vav+tav mater ending and re-pointing the radical
                // it left with a holam, then a final he.
                let lamed_he_inf_oh_derived = (root.lamed() == letter::HE
                    && matches!(
                        binyan,
                        Binyan::Niphal
                            | Binyan::Hiphil
                            | Binyan::Piel
                            | Binyan::Pual
                            | Binyan::Hithpael
                    )
                    && form == Form::InfinitiveConstruct)
                .then(|| {
                    let mut seq = hebrew::parse_pointed(&text);
                    let n = seq.len();
                    (n >= 3
                        && seq[n - 1].letter == letter::TAV
                        && seq[n - 2].letter == letter::VAV)
                        .then(|| {
                            seq.pop();
                            seq.pop();
                            if let Some(c) = seq.last_mut() {
                                c.vowel = Some(Vowel::Holam);
                            }
                            seq.push(Cons::new(letter::HE));
                            hebrew::render(&seq)
                        })
                })
                .flatten();
                // Plene tsere-yod spelling of the Hiphil infinitive absolute
                // (haškêm הַשְׁכֵּים beside הַשְׁכֵּם).
                let hiphil_inf_abs_plene = (binyan == Binyan::Hiphil
                    && form == Form::InfinitiveAbsolute)
                    .then(|| {
                        let mut seq = hebrew::parse_pointed(&text);
                        let n = seq.len();
                        (n >= 2 && seq[n - 2].vowel == Some(Vowel::Tsere)).then(|| {
                            seq.insert(n - 1, Cons::mater(letter::YOD));
                            hebrew::render(&seq)
                        })
                    })
                    .flatten();
                // 1cs aleph preformative hataf-segol twin: ʾĕzāreh אֱזָרֶה
                // beside the hataf-patah grade אֲזָרֶה.
                let aleph_prefix_hataf_segol =
                    (matches!(form, Form::Imperfect | Form::Jussive | Form::Cohortative)
                        && pgn.person == Some(Person::First)
                        && pgn.number == Some(Number::Singular))
                    .then(|| {
                        let mut seq = hebrew::parse_pointed(&text);
                        (seq.first().map(|c| (c.letter, c.vowel))
                            == Some((letter::ALEF, Some(Vowel::HatafPatah))))
                        .then(|| {
                            seq[0].vowel = Some(Vowel::HatafSegol);
                            hebrew::render(&seq)
                        })
                    })
                    .flatten();
                // ירא: the imperative keeps its yod — yᵊrāʾ יְרָא (2ms),
                // yᵊrʾû יְראוּ (2mp), yirʾî יִרְאִי (2fs).
                let yare_imperative: Vec<String> = if is_yare(root)
                    && binyan == Binyan::Qal
                    && form == Form::Imperative
                {
                    match (pgn.gender, pgn.number) {
                        (Some(Gender::Masculine), Some(Number::Singular)) => vec![hebrew::render(&[
                            rad(letter::YOD, 1).with_vowel(Vowel::Sheva),
                            rad(letter::RESH, 2).with_vowel(Vowel::Qamats),
                            rad(letter::ALEF, 3),
                        ])],
                        (Some(Gender::Masculine), Some(Number::Plural)) => vec![hebrew::render(&[
                            rad(letter::YOD, 1).with_vowel(Vowel::Sheva),
                            rad(letter::RESH, 2),
                            rad(letter::ALEF, 3),
                            Cons::new(letter::VAV).with_dagesh(),
                        ])],
                        (Some(Gender::Feminine), Some(Number::Singular)) => vec![hebrew::render(&[
                            rad(letter::YOD, 1).with_vowel(Vowel::Hiriq),
                            rad(letter::RESH, 2),
                            rad(letter::ALEF, 3).with_vowel(Vowel::Hiriq),
                            Cons::mater(letter::YOD),
                        ])],
                        _ => Vec::new(),
                    }
                } else {
                    Vec::new()
                };
                // Nun-retained I-nun imperfect twin (yinṣᵊrû יִנְצְרוּ and its
                // pausal יִנְצֹרוּ) beside the assimilated default (יִצְּרוּ).
                let pe_nun_impf_retained: Vec<String> = if binyan == Binyan::Qal
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                    && root.pe() == letter::NUN
                {
                    let mut seq = hebrew::parse_pointed(&text);
                    if seq.len() < 2 || !seq[1].dagesh || seq[1].letter != root.ayin() {
                        Vec::new()
                    } else {
                        seq[1].dagesh = false;
                        seq.insert(1, Cons::new(letter::NUN).with_vowel(Vowel::Sheva));
                        let retained = hebrew::render(&seq);
                        let pausal = pausal_imperfect_plural_variants(&retained);
                        std::iter::once(retained).chain(pausal).collect()
                    }
                } else {
                    Vec::new()
                };
                // I-yod imperative tsere twin: the reduced two-radical
                // imperative also appears with tsere — ṣēʾû צֵאוּ beside צְאוּ.
                let pe_yod_imperative_tsere = (binyan == Binyan::Qal
                    && form == Form::Imperative
                    && root.has(Gizra::PeYod))
                .then(|| {
                    let mut seq = hebrew::parse_pointed(&text);
                    (seq.len() <= 3 && seq.first().and_then(|c| c.vowel) == Some(Vowel::Sheva))
                        .then(|| {
                            seq[0].vowel = Some(Vowel::Tsere);
                            hebrew::render(&seq)
                        })
                })
                .flatten();
                // Piel infinitive absolute tsere twin: beside qaṭṭōl the
                // construct-shaped qaṭṭēl serves as the absolute (mahēr מַהֵר).
                let piel_inf_abs_tsere = (matches!(binyan, Binyan::Piel | Binyan::Hithpael)
                    && form == Form::InfinitiveAbsolute)
                    .then(|| {
                        let (t, _) =
                            generate_one(root, binyan, Form::InfinitiveConstruct, pgn, false);
                        (t != text).then_some(t)
                    })
                    .flatten();
                // Pausal-segol twin of the Piel/Hithpael inf-abs (dabbēr → dabber
                // דַּבֶּר): the theme tsere lowers to segol at a pause.
                let piel_inf_abs_segol = piel_inf_abs_tsere.as_deref().and_then(|t| {
                    let mut seq = hebrew::parse_pointed(t);
                    let i = seq.iter().rposition(|c| c.vowel == Some(Vowel::Tsere))?;
                    seq[i].vowel = Some(Vowel::Segol);
                    Some(hebrew::render(&seq))
                });
                // Niphal he-prefixed infinitive absolute (hiqqāṭōl): beside the
                // niqṭōl base the Niphal also attests the inf-construct's
                // he-prefixed shape with a holam theme — hēʾāḵōl הֵאָכֹל,
                // hinnāṯōn הִנָּתֹן. Take the inf-construct form and lower its
                // final tsere/segol theme to holam.
                let niphal_inf_abs_he = (binyan == Binyan::Niphal
                    && form == Form::InfinitiveAbsolute)
                    .then(|| {
                        let (t, _) =
                            generate_one(root, binyan, Form::InfinitiveConstruct, pgn, false);
                        let mut seq = hebrew::parse_pointed(&t);
                        let i = seq
                            .iter()
                            .rposition(|c| matches!(c.vowel, Some(Vowel::Tsere | Vowel::Segol)))?;
                        seq[i].vowel = Some(Vowel::Holam);
                        Some(hebrew::render(&seq))
                    })
                    .flatten();
                // III-He cohortative: the -â paragogic cannot attach to the -eh
                // stem, so the plain imperfect form serves as the cohortative
                // (וְנַעֲלֶה is tagged cohortative).
                let lamed_he_cohortative = (form == Form::Cohortative
                    && root.lamed() == letter::HE)
                    .then(|| {
                        let (t, _) = generate_one(root, binyan, Form::Imperfect, pgn, false);
                        (t != text).then_some(t)
                    })
                    .flatten();
                // Geminate Hiphil participle (מֵרַע, מְרֵעִים) and Niphal
                // imperfect (יִסַּב, יִסַּבּוּ) contracted twins.
                let geminate_hiphil_ptcp: Vec<String> = if binyan == Binyan::Hiphil
                    && form == Form::ParticipleActive
                    && root.ayin() == root.lamed()
                {
                    geminate_hiphil_participle_variants(root, pgn)
                } else {
                    Default::default()
                };
                // Geminate Niphal participle contracted twin (נָשַׁם, נְשַׁמּוֹת).
                let geminate_niphal_ptcp: Vec<String> = if binyan == Binyan::Niphal
                    && form == Form::ParticipleActive
                    && root.ayin() == root.lamed()
                {
                    geminate_niphal_participle_variants(root, pgn)
                } else {
                    Default::default()
                };
                // OSHB tags the contracted yiqqaṭ shape as Qal for some
                // geminates (yittammû יִתַּמּוּ), so emit it under Qal too.
                let geminate_niphal_impf: Vec<String> =
                    if matches!(binyan, Binyan::Niphal | Binyan::Qal)
                        && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                        && root.ayin() == root.lamed()
                    {
                        geminate_niphal_imperfect_variant(root, pgn)
                    } else {
                        Default::default()
                    }
                    .into_iter()
                    .map(|t| {
                        if form == Form::Wayyiqtol {
                            let mut seq = hebrew::parse_pointed(&t);
                            if let Some(first) = seq.first_mut() {
                                first.dagesh = true;
                            }
                            seq.insert(0, Cons::new(letter::VAV).with_vowel(Vowel::Patah));
                            hebrew::render(&seq)
                        } else {
                            t
                        }
                    })
                    .collect();
                // Doubly-weak hollow III-aleph Hiphil perfect (הֵבֵאתָ, וַהֲבֵאתֶם).
                let hollow_lamed_aleph_perf: Vec<String> = if binyan == Binyan::Hiphil
                    && form == Form::Perfect
                    && root.has(Gizra::Hollow)
                    && root.lamed() == letter::ALEF
                {
                    hollow_lamed_aleph_hiphil_perfect_variants(root, pgn)
                } else {
                    Default::default()
                };
                // I-yod Qal perfect heavy-afformative hiriq twin: yᵊraštem
                // יְרַשְׁתֶּם beside the attested i-grade yᵊrištem (וִירִשְׁתֶּם,
                // the וִי- conjunction sandhi peeled by the parser).
                // Fires only when the theme has reduced (C1 yod → sheva), i.e.
                // before a consonantal/heavy afformative (yᵊraštem → yᵊrištem,
                // יְרַשְׁתֶּם → יְרִשְׁתֶּם) or an object suffix on the 2ms/1cs
                // perfect (wîrištām וִירִשְׁתָּם, wîrištāh וִירִשְׁתָּהּ); the bare
                // qamats forms keep C1 qamats and the function's own guard skips
                // them.
                let pe_yod_perf_hiriq = (binyan == Binyan::Qal
                    && form == Form::Perfect
                    && root.has(Gizra::PeYod))
                .then(|| pe_yod_perfect_heavy_hiriq_variant(&text))
                .flatten();
                // Retained-yod twin of the I-yod Qal imperfect/wayyiqtol
                // (yēraš יֵרַשׁ → yîraš יִירַשׁ, wayyîraš וַיִּירַשׁ). Applied to both
                // the default theme and the a-theme alternant, since the attested
                // retained-yod forms are a-theme (yîraš, not the e-theme yîrēš).
                let pe_yod_retained: Vec<String> = if binyan == Binyan::Qal
                    && root.has(Gizra::PeYod)
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                {
                    std::iter::once(text.clone())
                        .chain(qal_a_theme.clone())
                        // The pausal a-theme too: the retained-yod stative
                        // surfaces with the lengthened qamats theme — yîḇāš
                        // (יִיבָשׁ), yîšān (יִישָׁן) — beside the contextual patah.
                        .chain(qal_a_theme_pausal.clone())
                        .flat_map(|t| {
                            // The mater spelling (וַיִּירָא) and its defective
                            // twin (וַיִּרָא — hiriq written bare).
                            pe_yod_retained_variant(&t)
                                .into_iter()
                                .chain(pe_yod_retained_defective_variant(&t))
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                // III-Guttural Qal Perfect vocalic suffix sheva variant —
                // יָדְעוּ beside יָדָעוּ, יָדְעָה beside יָדָעָה.
                let lamed_guttural_perf_sheva = (binyan == Binyan::Qal
                    && root.has(Gizra::LamedGuttural)
                    && form == Form::Perfect
                    && matches!(pgn.person, Some(Person::Third))
                    && matches!(
                        (pgn.gender, pgn.number),
                        (Some(_), Some(Number::Singular | Number::Plural))
                    )
                    && pgn != Pgn::new(Person::Third, Gender::Masculine, Number::Singular))
                .then(|| lamed_guttural_perfect_sheva_variant(&text))
                .flatten();
                // Hollow Hiphil consonantal/heavy perfect linking-ô forms
                // (hăqîmôṯî הֲקִימוֹתִי and its defective twins), which the base
                // builder leaves to the strong fallback.
                let hollow_hiphil_perf: Vec<String> =
                    if binyan == Binyan::Hiphil && form == Form::Perfect && root.has(Gizra::Hollow)
                    {
                        hollow_hiphil_otav_perfect(root, pgn)
                    } else if binyan == Binyan::Hiphil
                        && form == Form::Perfect
                        && root.ayin() == root.lamed()
                    {
                        // Geminate Hiphil perfect hēCēC (הֵחֵל, הֵחֵלּוּ, הֵחֵלָּה),
                        // with the patah-theme twin (hēmar הֵמַר, hēsabbû הֵסַבּוּ,
                        // hēsabbâ): C1's e-grade tsere lowers to a.
                        geminate_hiphil_perfect_variant(root, pgn)
                            .map(|s| {
                                let mut out = vec![hebrew::render(&s)];
                                if s.get(1).map(|c| c.vowel) == Some(Some(Vowel::Tsere)) {
                                    let mut p = s.clone();
                                    p[1].vowel = Some(Vowel::Patah);
                                    out.push(hebrew::render(&p));
                                }
                                out
                            })
                            .unwrap_or_default()
                    } else if binyan == Binyan::Niphal
                        && form == Form::InfinitiveConstruct
                        && root.ayin() == root.lamed()
                    {
                        // OSHB tags the contracted hēḥēl shape as a Niphal
                        // infinitive (הֵחֵל) — emit it under that label too.
                        geminate_hiphil_perfect_variant(
                            root,
                            Pgn::new(Person::Third, Gender::Masculine, Number::Singular),
                        )
                        .map(|s| vec![hebrew::render(&s)])
                        .unwrap_or_default()
                    } else if binyan == Binyan::Niphal
                        && form == Form::Perfect
                        && root.ayin() == root.lamed()
                    {
                        // Geminate Niphal perfect nāCaC (נָשַׁם, נָשַׁמּוּ, נָסַבּוּ).
                        geminate_niphal_perfect_variant(root, pgn)
                            .map(|s| vec![hebrew::render(&s)])
                            .unwrap_or_default()
                    } else if binyan == Binyan::Niphal
                        && form == Form::Perfect
                        && root.has(Gizra::Hollow)
                    {
                        // Hollow Niphal perfect nāCôC (נָכוֹן, נָפֹצוּ).
                        hollow_niphal_perfect_variant(root, pgn)
                            .map(|s| vec![hebrew::render(&s)])
                            .unwrap_or_default()
                    } else if binyan == Binyan::Qal && form == Form::Perfect {
                        // Stative qāṭēl/qāṭōl perfect twins (ṭāhēr טָהֵר, qāṭōn,
                        // yāḡōrtî יָגֹרְתִּי) — any subject whose theme patah
                        // survives on C2 (the vocalic suffixes reduce it away, so
                        // the transform is inert there).
                        qal_stative_perfect_variants(&text)
                    } else if binyan == Binyan::Hiphil
                        && form == Form::InfinitiveConstruct
                        && root.has(Gizra::Hollow)
                    {
                        // Hollow Hiphil inf-construct hāCîC (הָמִית, הָקִים, הָבִיא),
                        // which the strong builder + gizra pass leaves empty.
                        vec![hebrew::render(&hollow_hiphil_inf_construct(root))]
                    } else if binyan == Binyan::Qal
                        && form == Form::InfinitiveConstruct
                        && root.pe() == letter::NUN
                    {
                        // Nun-retained Qal inf-construct (נְפֹל beside the dropped פֹּל).
                        let mut v = vec![hebrew::render(&pe_nun_inf_construct_retained(root))];
                        // III-aleph (נשא): beside the tsere-on-C2 שֵׂאת the reduced
                        // grade śᵊʾēṯ שְׂאֵת is the common spelling.
                        if root.lamed() == letter::ALEF {
                            v.push(hebrew::render(&[
                                rad(root.ayin(), 2).with_vowel(Vowel::Sheva),
                                rad(root.lamed(), 3).with_vowel(Vowel::Tsere),
                                Cons::new(letter::TAV),
                            ]));
                        }
                        // נתן clips both nuns: the inf-construct is tēṯ תֵּת (לָתֵת).
                        // Emit it with and without the word-initial dagesh lene so
                        // both the bare and the proclitic-peeled surfaces match.
                        if root.ayin() == letter::TAV && root.lamed() == letter::NUN {
                            let t1 = rad(letter::TAV, 2).with_vowel(Vowel::Tsere);
                            // The tsere lowers to segol before a maqqef (לָתֶת־).
                            let t1s = rad(letter::TAV, 2).with_vowel(Vowel::Segol);
                            let t3 = Cons::new(letter::TAV);
                            v.push(hebrew::render(&[t1.with_dagesh(), t3]));
                            v.push(hebrew::render(&[t1, t3]));
                            v.push(hebrew::render(&[t1s.with_dagesh(), t3]));
                            v.push(hebrew::render(&[t1s, t3]));
                        }
                        v
                    } else if binyan == Binyan::Qal
                        && form == Form::InfinitiveConstruct
                        && root.pe() == letter::QOF
                        && root.ayin() == letter::RESH
                        && root.lamed() == letter::ALEF
                    {
                        // קרא's lexicalized -t inf-construct (liqraʾṯ לִקְרַאת
                        // "to meet"; the proclitic ל is peeled by the parser).
                        vec![hebrew::render(&[
                            rad(letter::QOF, 1).with_vowel(Vowel::Sheva),
                            rad(letter::RESH, 2).with_vowel(Vowel::Patah),
                            rad(letter::ALEF, 3),
                            Cons::new(letter::TAV),
                        ])]
                    } else if binyan == Binyan::Qal
                        && form == Form::ParticipleActive
                        && pgn == Pgn::gn(Gender::Feminine, Number::Singular)
                    {
                        // -â feminine participle (יוֹלֵדָה) beside the segolate
                        // יֹלֶדֶת, and its propretonic-reduced grade — ʾōḵlâ
                        // אֹכְלָה, gōʾălâ (the theme tsere drops to sheva / a
                        // guttural hataf).
                        let mut v = vec![hebrew::render(&qal_participle_fs_a_variant(root))];
                        let mut seq = vec![
                            rad(root.pe(), 1).with_vowel(Vowel::Holam),
                            rad(root.ayin(), 2).with_vowel(Vowel::Sheva),
                            rad(root.lamed(), 3).with_vowel(Vowel::Qamats),
                            Cons::new(letter::HE),
                        ];
                        apply_guttural(&mut seq, root);
                        v.push(hebrew::render(&seq));
                        v
                    } else if binyan == Binyan::Piel
                        && form == Form::InfinitiveConstruct
                        && root.pe() == letter::SHIN
                        && root.ayin() == letter::RESH
                        && root.lamed() == letter::TAV
                    {
                        // שרת's lexicalized Piel infinitive šārēṯ (לְשָׁרֵת): the resh
                        // refuses the doubling and the first syllable lengthens.
                        vec![hebrew::render(&[
                            rad(letter::SHIN, 1).with_vowel(Vowel::Qamats),
                            rad(letter::RESH, 2).with_vowel(Vowel::Tsere),
                            Cons::new(letter::TAV),
                        ])]
                    } else if binyan == Binyan::Hiphil
                        && form == Form::InfinitiveConstruct
                        && root.ayin() == root.lamed()
                    {
                        // Geminate Hiphil inf-construct hāCēC (הָפֵר, הָסֵב) and the
                        // a-grade a final guttural takes (הָרַע).
                        let theme = if hebrew::is_guttural(root.lamed()) {
                            Vowel::Patah
                        } else {
                            Vowel::Tsere
                        };
                        vec![hebrew::render(&[
                            Cons::new(letter::HE).with_vowel(Vowel::Qamats),
                            rad(root.pe(), 1).with_vowel(theme),
                            rad(root.lamed(), 3),
                        ])]
                    } else {
                        Vec::new()
                    };
                // Pronominal object-suffixed forms (computed from &text before it
                // is moved into the base VerbForm below).
                let mut object_suffixed: Vec<(Pgn, String)> =
                    if matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                        && imperfect_suffix_kind(pgn) == Suffix::Zero
                    {
                        // Consonantal (zero-suffix) imperfect hosts — the 3ms and the
                        // other singular subjects 1cs/2ms/3fs (and 1cp) — all take
                        // object suffixes the same way: the prefix differs but the
                        // stem reduction and linking vowel do not (ʾešmᵊrēhû
                        // אֶשְׁמְרֵהוּ, tišmᵊrennû). A III-He host elides its he and links
                        // on a tsere instead (yaʕănēhû יַעֲנֵהוּ).
                        if root.lamed() == letter::HE {
                            lamed_he_imperfect_object_suffixes(&text)
                        } else {
                            imperfect_object_suffixes(&text, root)
                        }
                    } else if pgn == Pgn::new(Person::Third, Gender::Masculine, Number::Singular) {
                        match (binyan, form) {
                            (_, Form::Perfect) if root.lamed() == letter::HE => {
                                // The segol-prefix III-He Hiphil twin (herʾâ
                                // הֶרְאָה) is a suffix host too — herʾām הֶרְאָם.
                                let mut v = lamed_he_perfect_object_suffixes(&text);
                                if let Some(base) = hiphil_perf_segol.as_ref() {
                                    v.extend(lamed_he_perfect_object_suffixes(base));
                                }
                                v
                            }
                            (Binyan::Qal, Form::Perfect) => qal_perfect_object_suffixes(root),
                            (Binyan::Piel | Binyan::Pual | Binyan::Hithpael, Form::Perfect) => {
                                let mut v = derived_perfect_object_suffixes(&text);
                                // The patah-theme twin (śimmaḥ שִׂמַּח beside שִׂמֵּחַ)
                                // is a suffix host too: śimmᵊḥām שִׂמְּחָם.
                                if let Some(base) = piel_perf_patah.as_ref() {
                                    v.extend(derived_perfect_object_suffixes(base));
                                }
                                v
                            }
                            (Binyan::Hiphil, Form::Perfect) => {
                                // The bare 3ms text and the hollow î alternants
                                // (הֵבִיא) are all suffix hosts.
                                let mut v = hiphil_perfect_object_suffixes(&text);
                                if root.has(Gizra::Hollow) {
                                    for base in hollow_hiphil_otav_perfect(root, pgn) {
                                        v.extend(hiphil_perfect_object_suffixes(&base));
                                    }
                                }
                                v
                            }
                            _ => Vec::new(),
                        }
                    } else if matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                        && matches!(
                            (pgn.person, pgn.gender, pgn.number),
                            // The vocalic-suffix plural/2fs subjects (yiqbᵊrû -û,
                            // tiqbᵊrî -î) take object suffixes on the retained subject
                            // vowel: wayyiqbᵊrûhû (וַיִּקְבְּרֻהוּ).
                            (
                                Some(Person::Third | Person::Second),
                                Some(Gender::Masculine),
                                Some(Number::Plural)
                            ) | (
                                Some(Person::Second),
                                Some(Gender::Feminine),
                                Some(Number::Singular)
                            )
                        )
                    {
                        imperfect_vocalic_object_suffixes(&text)
                    } else if form == Form::Perfect
                        && matches!(
                            (pgn.person, pgn.gender, pgn.number),
                            (
                                Some(Person::First),
                                Some(Gender::Common),
                                Some(Number::Singular | Number::Plural)
                            ) | (
                                Some(Person::Second),
                                Some(Gender::Masculine),
                                Some(Number::Singular)
                            ) | (
                                Some(Person::Third),
                                Some(Gender::Feminine),
                                Some(Number::Singular)
                            ) | (
                                Some(Person::Third),
                                Some(Gender::Common),
                                Some(Number::Plural)
                            )
                        )
                    {
                        let mut v = perfect_subject_object_suffixes(&text, pgn, root, binyan);
                        // The III-aleph perfect has a tsere-theme twin (śānēʾṯî
                        // שָׂנֵאתִי beside śānāʾṯî); its suffixed hosts carry the
                        // tsere through — śᵊnēʾṯîm שְׂנֵאתִים beside śᵊnāʾṯîm.
                        if let Some(base) = lamed_aleph_tsere.as_ref() {
                            v.extend(perfect_subject_object_suffixes(base, pgn, root, binyan));
                        }
                        // The hollow-Hiphil linking-ô perfect bases (hăḇîʾōṯî
                        // הֲבִיאֹתִי, hăqîmōṯā) are suffix hosts on the same
                        // consonantal afformatives — hăḇîʾōṯîw הֲבִיאֹתִיו.
                        if binyan == Binyan::Hiphil && root.has(Gizra::Hollow) {
                            for base in hollow_hiphil_otav_perfect(root, pgn) {
                                v.extend(perfect_subject_object_suffixes(&base, pgn, root, binyan));
                            }
                        }
                        // The 3cp -û is a vocalic host in any binyan: the suffix
                        // joins the subject vowel directly on the base surface —
                        // rāʾûḵā רָאוּךָ (III-he), ḥērᵊp̄ûnî חֵרְפוּנִי (Piel), and the
                        // contracted geminate host (sabbûnî סַבּוּנִי via its
                        // alternant below).
                        if pgn == Pgn::new(Person::Third, Gender::Common, Number::Plural) {
                            v.extend(imperfect_vocalic_object_suffixes(&text));
                            if let Some(base) = geminate_qal_perf.as_ref() {
                                v.extend(imperfect_vocalic_object_suffixes(base));
                            }
                        }
                        v
                    } else if matches!(form, Form::ParticipleActive | Form::ParticiplePassive) {
                        // The participle is the noun-like host; it takes the
                        // nominal suffix set. The hollow alternants (קָם, קָמִים)
                        // host suffixes too — qāmay קָמַי — so feed them alongside
                        // the primary surface.
                        if pgn == Pgn::gn(Gender::Masculine, Number::Singular) {
                            let mut v = participle_object_suffixes(&text);
                            for t in &hollow_participle {
                                v.extend(participle_object_suffixes(t));
                            }
                            v
                        } else if pgn == Pgn::gn(Gender::Masculine, Number::Plural) {
                            let mut v = participle_mp_object_suffixes(&text);
                            for t in &hollow_participle {
                                v.extend(participle_mp_object_suffixes(t));
                            }
                            v
                        } else if pgn == Pgn::gn(Gender::Feminine, Number::Plural) {
                            participle_fp_object_suffixes(&text)
                        } else if pgn == Pgn::gn(Gender::Feminine, Number::Singular) {
                            participle_fs_object_suffixes(&text)
                        } else {
                            Vec::new()
                        }
                    } else if form == Form::InfinitiveConstruct {
                        inf_construct_object_suffixes(root, binyan, &text)
                    } else if binyan == Binyan::Qal
                        && form == Form::Imperative
                        && pgn == Pgn::new(Person::Second, Gender::Masculine, Number::Singular)
                    {
                        qal_imperative_object_suffixes(root)
                    } else if binyan == Binyan::Hiphil
                        && form == Form::Imperative
                        && pgn == Pgn::new(Person::Second, Gender::Masculine, Number::Singular)
                    {
                        if root.lamed() == letter::HE {
                            lamed_he_hiphil_imperative_object_suffixes(&text)
                        } else {
                            hiphil_imperative_object_suffixes(&text)
                        }
                    } else if matches!(binyan, Binyan::Piel | Binyan::Hithpael)
                        && form == Form::Imperative
                        && pgn == Pgn::new(Person::Second, Gender::Masculine, Number::Singular)
                    {
                        piel_imperative_object_suffixes(&text)
                    } else if form == Form::Imperative
                        && matches!(
                            (pgn.person, pgn.gender, pgn.number),
                            // The vocalic-subject imperatives (2mp -û, 2fs -î) take
                            // object suffixes on the retained subject vowel exactly
                            // like the imperfect plural host: hallᵊlûhû (הַלְלוּהוּ),
                            // šmāʕûnî. Binyan-agnostic — operates on the surface.
                            (
                                Some(Person::Second),
                                Some(Gender::Masculine),
                                Some(Number::Plural)
                            ) | (
                                Some(Person::Second),
                                Some(Gender::Feminine),
                                Some(Number::Singular)
                            )
                        )
                    {
                        let mut v = imperfect_vocalic_object_suffixes(&text);
                        // The Qal 2mp host also appears in its theme-restored
                        // (pausal-style) grade — C1 sheva, C2 qamats — under the
                        // suffix: šᵊmāʕûnî שְׁמָעוּנִי beside שִׁמְעוּנִי.
                        if binyan == Binyan::Qal
                            && pgn == Pgn::new(Person::Second, Gender::Masculine, Number::Plural)
                            && !matches!(root.lamed(), letter::HE | letter::ALEF)
                            && !root.has(Gizra::Hollow)
                        {
                            let mut c1 = rad(root.pe(), 1).with_vowel(Vowel::Sheva);
                            if hebrew::is_begedkefet(root.pe()) {
                                c1 = c1.with_dagesh();
                            }
                            let restored = hebrew::render(&[
                                c1,
                                rad(root.ayin(), 2).with_vowel(Vowel::Qamats),
                                rad(root.lamed(), 3),
                                oshureq(),
                            ]);
                            v.extend(imperfect_vocalic_object_suffixes(&restored));
                        }
                        v
                    } else {
                        Vec::new()
                    };
                // Hollow Hiphil perfect: the object-suffix base is the linking-ô
                // stem (hăqîmôṯî הֲקִימוֹתִי), an alternant the dispatch above can't
                // see (the primary `text` is the strong fallback). Run the
                // subject-suffix builder over each otav base — it no-ops for any
                // non-1cs/2ms/3cp subject: hăšîḇōṯîm וַהֲשִׁבֹתִים.
                if binyan == Binyan::Hiphil && form == Form::Perfect && root.has(Gizra::Hollow) {
                    let mut extra = Vec::new();
                    for base in hollow_hiphil_otav_perfect(root, pgn) {
                        extra.extend(perfect_subject_object_suffixes(&base, pgn, root, binyan));
                    }
                    object_suffixed.extend(extra);
                }
                // I-aleph patah-grade wayyiqtol/imperfect + object suffix
                // (wayyaʾasrēhû וַיַּאַסְרֵהוּ): the suffix host is the patah
                // alternant, which the zero-suffix dispatch above can't see
                // (the primary `text` is the segol grade).
                if matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                    && imperfect_suffix_kind(pgn) == Suffix::Zero
                {
                    if let Some(base) = pe_aleph_patah.as_ref() {
                        object_suffixed.extend(imperfect_object_suffixes(base, root));
                    }
                    // The a-theme (stative) alternant is likewise a suffix host
                    // the dispatch can't see: its patah theme lengthens to
                    // qamats under the suffix — wayyišḥāṭēm וַיִּשְׁחָטֵם,
                    // wayyeʾĕhāḇēhû וַיֶּאֱהָבֵהוּ.
                    if let Some(base) = qal_a_theme.as_ref() {
                        object_suffixed.extend(imperfect_object_suffixes(base, root));
                    }
                    // The silent-sheva I-guttural twin closes the prefix
                    // syllable and is a suffix host of its own: wāʾehrᵊḡēhû
                    // וָאֶהְרְגֵהוּ, wayyaḥšᵊḇehā וַיַּחְשְׁבֶהָ.
                    if let Some(base) = pe_guttural_impf_silent.as_ref() {
                        object_suffixed.extend(imperfect_object_suffixes(base, root));
                    }
                    // The retained-yod I-yod twin (וַיִּירַשׁ) hosts too.
                    for base in &pe_yod_retained {
                        object_suffixed.extend(imperfect_object_suffixes(base, root));
                    }
                }
                // Vocalic-subject (-û/-î) hosts the dispatch can't see: the
                // theme-restored grade under the object suffix (yišḥāṭûm
                // וַיִּשְׁחָטוּם, yimšāḥuhû וַיִּמְשָׁחֻהוּ — the same grade pause
                // restores), and the I-aleph patah alternant (wayyaʾasrûhû
                // וַיַּאַסְרֻהוּ).
                if matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                    && imperfect_suffix_kind(pgn) == Suffix::Vocalic
                {
                    for base in pausal_imperfect_plural_variants(&text) {
                        object_suffixed.extend(imperfect_vocalic_object_suffixes(&base));
                    }
                    if let Some(base) = pe_aleph_patah.as_ref() {
                        object_suffixed.extend(imperfect_vocalic_object_suffixes(base));
                    }
                    // The silent-sheva I-guttural twin (yaḥᵊlᵊqû → yaḥlᵊqû) is
                    // a vocalic suffix host too: wayyaḥlᵊqûm וַיַּחְלְקוּם.
                    if let Some(base) = pe_guttural_impf_silent.as_ref() {
                        object_suffixed.extend(imperfect_vocalic_object_suffixes(base));
                        for b in pausal_imperfect_plural_variants(base) {
                            object_suffixed.extend(imperfect_vocalic_object_suffixes(&b));
                        }
                    }
                    // The a-theme plural alternant reduces straight to the
                    // silent-sheva grade (yaḥlᵊqû) and hosts suffixes too.
                    if let Some(base) = qal_a_theme.as_ref() {
                        object_suffixed.extend(imperfect_vocalic_object_suffixes(base));
                        for b in pausal_imperfect_plural_variants(base) {
                            object_suffixed.extend(imperfect_vocalic_object_suffixes(&b));
                        }
                    }
                    // I-guttural silent-sheva twins (וַיַּחְלְקוּ via
                    // iguttural_reduced), and retained-yod I-yod twins
                    // (וַיִּירְשׁוּ) — each with its theme-restored grade
                    // (וַיִּירָשֻׁם).
                    for base in qal_iguttural_silent
                        .iter()
                        .chain(&pe_yod_retained)
                        .chain(&iguttural_reduced)
                    {
                        object_suffixed.extend(imperfect_vocalic_object_suffixes(base));
                        for b in pausal_imperfect_plural_variants(base) {
                            object_suffixed.extend(imperfect_vocalic_object_suffixes(&b));
                        }
                    }
                }
                // Hollow Hiphil + object suffix: the suffix shifts the stress
                // off the prefix syllable, whose qamats reduces to vocal sheva
                // and drops the preformative doubling. The short wayyiqtol
                // host also restores the long î stem under the suffix:
                // wayḇîʾēnî וַיְבִיאֵנִי (beside the bare short וַיָּבֵא); the
                // vocalic subjects give wayḇîʾûhû וַיְבִיאוּהוּ and the
                // defective וַיְבִאֻהוּ.
                if binyan == Binyan::Hiphil
                    && root.has(Gizra::Hollow)
                    && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
                {
                    match imperfect_suffix_kind(pgn) {
                        // Only the short (jussive-based) hosts need the stem
                        // restored; the plain imperfect host already carries
                        // the î and reduces its prefix inside
                        // `imperfect_object_suffixes`.
                        Suffix::Zero if matches!(form, Form::Wayyiqtol | Form::Jussive) => {
                            if let Some(base) = hollow_hiphil_reduced_long_base(&text) {
                                object_suffixed.extend(imperfect_object_suffixes(&base, root));
                            }
                        }
                        Suffix::Vocalic => {
                            if let Some(base) = hollow_hiphil_reduced_prefix(&text) {
                                for b in std::iter::once(base.clone())
                                    .chain(hiphil_defective_variant(&base))
                                {
                                    object_suffixed.extend(imperfect_vocalic_object_suffixes(&b));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                forms.push(VerbForm {
                    binyan,
                    form,
                    pgn,
                    text,
                    attested,
                    object_suffix: None,
                });
                for alt in [
                    maqaf,
                    guttural_lowered,
                    pausal,
                    pausal_perf,
                    geminate_qal_perf,
                    pe_guttural_impf_hataf,
                    pe_guttural_impf_silent,
                    pe_guttural_impf_silent_pl,
                    hiphil_guttural_c2_spirant,
                    pe_yod_hiphil_e,
                    pe_yod_hiphil_e_plene,
                    paragogic,
                    hiphil_apoc,
                    hiphil_plene,
                    hiphil_defective,
                    piel_pausal,
                    qal_pausal,
                    guttural_imperative_pausal,
                    guttural_perfect_patah,
                    hollow_ptcp,
                    hollow_hiphil_ptcp,
                    hollow_hophal_ptcp,
                    hollow_qal_perf_heavy,
                    lamed_aleph_ptcp,
                    ayin_guttural_hataf,
                    qal_imperative_ayin_gutt,
                    lamed_he_ptcp_fs_cstr,
                    lamed_aleph_perf_bare_t,
                    piel_perf_guttural_hiriq,
                    pe_aleph_wayy_1cp,
                    pe_aleph_tsere,
                    pe_aleph_holam,
                    hiphil_uncontracted,
                    hiphil_uncontracted_e,
                    lamed_aleph_tsere,
                    pe_guttural_segol,
                    lamed_guttural_perf_sheva,
                    qal_a_theme,
                    guttural_silent_sheva,
                    guttural_silent_pausal,
                    paragogic_nun,
                    hollow_paragogic_nun,
                    lamed_guttural_perf_2fs,
                    pe_nun_imperative_retained,
                    pausal_qotol,
                    qal_pass_ptcp_reduced,
                    lamed_he_hiphil_apoc,
                    pe_aleph_patah,
                    pe_aleph_pausal,
                    qal_stative_perfect,
                    qal_stative_participle,
                    hollow_perfect_holam,
                    hollow_wayyiqtol_patah,
                    hollow_impf_fp_plene,
                    lamed_he_imp_apoc,
                    lamed_he_imp_apoc_guttural,
                    lamed_he_imp_apoc_pausal,
                    hiphil_imp_segholate,
                    hiphil_wayyiqtol_tsere,
                    jussive_long,
                    wayyiqtol_long,
                    wayyiqtol_cohortative,
                    hollow_wayyiqtol_pausal,
                    lamed_he_doubled_apoc,
                    raah_apoc_tsere,
                    pe_yod_as_pe_nun,
                    pe_yod_hiphil_defective,
                    pe_yod_perf_hiriq,
                    lamed_he_perf_tsere,
                    lamed_he_perf_hiriq,
                    lamed_he_hophal_perf_tsere,
                    iguttural_hophal_loud,
                    iguttural_hophal_loud_tsere,
                    niphal_iguttural_he_tsere,
                    ptcp_fs_unreduced_a,
                    hiphil_iguttural_imp_segol,
                    hiphil_iguttural_imp_segol_silent,
                    piel_perf_patah,
                    piel_perf_hiriq,
                    hiphil_perf_segol,
                    lamed_he_fp_short,
                    lamed_he_pausal,
                    lamed_aleph_derived_perf,
                    qal_a_theme_pausal,
                    lamed_aleph_impf_qamats,
                    lamed_aleph_impf_nun,
                    piel_inf_abs_lamed_he,
                    hollow_hiphil_inf_abs,
                    halak_retained,
                    hiphil_ptcp_reduced,
                    wayyiqtol_1cs_short,
                    hollow_qal_perf_2fs,
                    lamed_guttural_fs_ptcp,
                    niphal_wayy_patah,
                    lamed_he_inf_abs_vav,
                    pual_ptcp_qamats,
                    hophal_qubuts,
                    niphal_pe_guttural_hiriq,
                    niphal_pe_guttural_hataf,
                    chayah_segol,
                    shachah_inf_defective,
                    shachah_pausal,
                    segholate_inf_pausal,
                    piel_inf_abs_tsere,
                    piel_inf_abs_segol,
                    niphal_inf_abs_he,
                    lamed_he_cohortative,
                    geminate_hiphil_impf,
                    geminate_hophal_impf,
                    geminate_hiphil_wayy,
                    qal_a_theme_inf,
                    piel_guttural_qamats,
                    niphal_impf_fp_patah,
                    niphal_impf_a,
                    lamed_aleph_fs_ptcp,
                    chayah_perf,
                    maen_ptcp,
                    geminate_qal_perf_pausal,
                    geminate_qal_perf_hataf,
                    geminate_qal_imv,
                    nagash_imv_holam,
                    hiphil_impf_sheva,
                    hollow_niphal_inf,
                    lamed_he_fs_ptcp_iyya,
                    lamed_he_inf_oh,
                    lamed_he_inf_oh_derived,
                    hiphil_inf_abs_plene,
                    aleph_prefix_hataf_segol,
                    pe_yod_imperative_tsere,
                    fp_nah_heless,
                ]
                .into_iter()
                .flatten()
                {
                    forms.push(VerbForm {
                        binyan,
                        form,
                        pgn,
                        text: alt,
                        attested,
                        object_suffix: None,
                    });
                }
                for alt in pe_yod_retained
                    .into_iter()
                    .chain(paragogic_nun_theme)
                    .chain(pausal_plural)
                    .chain(iguttural_reduced)
                    .chain(qal_iguttural_silent)
                    .chain(pe_yod_niphal_vav)
                    .chain(hollow_participle.clone())
                    .chain(geminate_hiphil_ptcp)
                    .chain(geminate_niphal_ptcp)
                    .chain(derived_fs_ptcp_et)
                    .chain(hiphil_perf_patah_he)
                    .chain(hollow_niphal_participle)
                    .chain(niphal_1cs_hiriq)
                    .chain(pe_nun_impf_retained)
                    .chain(pe_guttural_impf_silent_pl_pausal)
                    .chain(pe_guttural_impf_a_silent_pl)
                    .chain(geminate_niphal_impf)
                    .chain(piel_pausal_zero)
                    .chain(niphal_pe_guttural_ptcp_a)
                    .chain(hollow_lamed_aleph_perf)
                    .chain(qal_inf_fem)
                    .chain(pe_guttural_a_silent)
                    .chain(pe_guttural_loud_segol_pl)
                    .chain(pe_guttural_loud_from_silent)
                    .chain(geminate_hiphil_perf_otav)
                    .chain(geminate_hophal_perf)
                    .chain(hollow_hophal_perf)
                    .chain(cohortative_long)
                    .chain(wayyiqtol_cohortative_long)
                    .chain(yare_imperative)
                    .chain(lamed_he_retained_yod)
                    .chain(lamed_aleph_inf_ot)
                {
                    forms.push(VerbForm {
                        binyan,
                        form,
                        pgn,
                        text: alt,
                        attested,
                        object_suffix: None,
                    });
                }
                for alt in hollow_hiphil_perf {
                    forms.push(VerbForm {
                        binyan,
                        form,
                        pgn,
                        text: alt,
                        attested,
                        object_suffix: None,
                    });
                }
                // Object-suffixed forms (deduped by surface+suffix). For a
                // geminate root the strong builder leaves the doubled radical
                // uncontracted (ḥānᵊnēnî חָנְנֵנִי); the attested form collapses
                // the Cˇ + C into a single dageshed radical (ḥonnēnî חָנֵּנִי), so
                // emit that contracted twin alongside.
                let geminate = root.ayin() == root.lamed();
                let mut osuf_seen = std::collections::HashSet::new();
                for (obj, t) in object_suffixed {
                    let contracted = geminate
                        .then(|| geminate_contract_variant(&t, root.lamed()))
                        .flatten();
                    // Defective twins of a suffixed form: a medial î mater
                    // (hiriq + yod) is often written bare — yᵉḇîʔēhû יְבִיאֵהוּ →
                    // יְבִאֵהוּ (Hiphil stem î), ṣiwwîṯîḵā צִוִּיתִיךָ → צִוִּיתִךָ
                    // (the -tî afformative). Each interior î is dropped in turn.
                    let defective = strip_hiriq_yod_mater_variants(&t);
                    // I-guttural twin: when the reduced suffixed stem closes the
                    // C1-guttural syllable (yaʕăzᵊḇēnî → yaʕazḇēnî יַעַזְבֵנִי,
                    // וַיַּעַזְבֵנִי), the hataf under the guttural fills to its
                    // matching short vowel.
                    let guttural = guttural_hataf_to_full_variant(&t);
                    // II-guttural derived-stem twin: a suffixed host that closes
                    // the guttural C2 on a silent sheva (bārᵊḵēnî בָּרְכֵנִי,
                    // pēʾrᵊḵā) instead opens it on a hataf-patah — bārăḵēnî
                    // בָּרֲכֵנִי, pēʾărᵊḵā פֵאֲרָךְ, lᵊḇārăḵô וּלְבָרֲכוֹ. The
                    // imperfect host already bakes this in; cover imperative /
                    // infinitive / perfect hosts here.
                    let ayin_hataf = (matches!(
                        binyan,
                        Binyan::Piel | Binyan::Pual | Binyan::Hithpael
                    ) && root.has(Gizra::AyinGuttural))
                    .then(|| ayin_guttural_hataf_variant(root, &t))
                    .flatten();
                    for surf in std::iter::once(t)
                        .chain(contracted)
                        .chain(defective)
                        .chain(guttural)
                        .chain(ayin_hataf)
                    {
                        if osuf_seen.insert((obj, surf.clone())) {
                            forms.push(VerbForm {
                                binyan,
                                form,
                                pgn,
                                text: surf,
                                attested,
                                object_suffix: Some(obj),
                            });
                        }
                    }
                }
                // Paragogic-he (emphatic) imperative twin for the 2ms: an -â
                // appends to the bare imperative, sharing the reduced stem of
                // the 2fs/2mp vocalic forms — tēn → tᵉnâ (תְּנָה), lēḵ → lᵉḵâ
                // (לְכָה), qᵉṭōl → qoṭlâ. Derive it from the 2fs (which already
                // carries the reduced stem) by swapping its final -î (C+hiriq,
                // yod) for -â (C+qamats, he). Same 2ms label; only surface differs.
                if form == Form::Imperative
                    && pgn == Pgn::new(Person::Second, Gender::Masculine, Number::Singular)
                {
                    let fs = Pgn::new(Person::Second, Gender::Feminine, Number::Singular);
                    let (base, attested_fs) = generate_one(root, binyan, form, fs, false);
                    let mut seq = hebrew::parse_pointed(&base);
                    let n = seq.len();
                    if n >= 2
                        && seq[n - 1].letter == letter::YOD
                        && seq[n - 2].vowel == Some(Vowel::Hiriq)
                    {
                        seq[n - 2].vowel = Some(Vowel::Qamats);
                        seq[n - 1] = Cons::new(letter::HE);
                        forms.push(VerbForm {
                            binyan,
                            form,
                            pgn,
                            text: hebrew::render(&seq),
                            attested: attested_fs,
                            object_suffix: None,
                        });
                        // The o-grade twin with C1 qamats(-qatan) — zoḵrâ
                        // זָכְרָה beside the i-grade šiḵḇâ שִׁכְבָה.
                        if binyan == Binyan::Qal
                            && seq.first().is_some_and(|c| c.vowel == Some(Vowel::Hiriq))
                        {
                            seq[0].vowel = Some(Vowel::Qamats);
                            forms.push(VerbForm {
                                binyan,
                                form,
                                pgn,
                                text: hebrew::render(&seq),
                                attested: attested_fs,
                                object_suffix: None,
                            });
                        }
                    }
                }
                // Tsere twin for the resh-final Piel/Hithpael perfect 3ms: the
                // base lowers the theme tsere to segol (dibbēr → דִּבֶּר, the
                // dominant spelling), but the tsere spelling דִּבֵּר is also
                // attested. Emit it alongside so both surfaces match.
                if matches!(binyan, Binyan::Piel | Binyan::Hithpael)
                    && form == Form::Perfect
                    && pgn == Pgn::new(Person::Third, Gender::Masculine, Number::Singular)
                    && root.lamed() == letter::RESH
                {
                    let (base, _) = generate_one(root, binyan, form, pgn, false);
                    let mut seq = hebrew::parse_pointed(&base);
                    if let Some(c) = seq.iter_mut().rev().find(|c| c.vowel == Some(Vowel::Segol)) {
                        c.vowel = Some(Vowel::Tsere);
                        forms.push(VerbForm {
                            binyan,
                            form,
                            pgn,
                            text: hebrew::render(&seq),
                            attested,
                            object_suffix: None,
                        });
                    }
                }
                // Archaic 2fs perfect twin: the older (and frequent ketiv) 2fs
                // perfect keeps the long -tî afformative, making it homographic
                // with the 1cs (ʕāśîṯî עָשִׂיתִי spells both "I did" and archaic
                // "you (f.) did"; also יָלַדְתִּי, נָתַתִּי). Emit the 1cs surface
                // under the 2fs label so the gold 2fs reading matches.
                if form == Form::Perfect
                    && pgn == Pgn::new(Person::Second, Gender::Feminine, Number::Singular)
                {
                    let cs1 = Pgn::new(Person::First, Gender::Common, Number::Singular);
                    let (base, attested_1cs) = generate_one(root, binyan, form, cs1, false);
                    forms.push(VerbForm {
                        binyan,
                        form,
                        pgn,
                        text: base,
                        attested: attested_1cs,
                        object_suffix: None,
                    });
                }
                // Construct-state twin for masculine-plural participles
                // (-îm → -ê). Same (binyan, form, pgn) label; only the
                // surface differs, which is all reverse-parsing needs.
                if matches!(form, Form::ParticipleActive | Form::ParticiplePassive)
                    && pgn.gender == Some(Gender::Masculine)
                    && pgn.number == Some(Number::Plural)
                {
                    let seq = build_strong(root, binyan, form, pgn, false);
                    let (seq, c_attested) = apply_gizra(seq, root, binyan, form, pgn);
                    if let Some(cseq) = participle_mp_construct(&seq) {
                        forms.push(VerbForm {
                            binyan,
                            form,
                            pgn,
                            text: hebrew::render(&cseq),
                            attested: c_attested,
                            object_suffix: None,
                        });
                        // Construct propretonic reduction: a C2-radical qamats
                        // (Niphal nišbārê→nišbᵊrê נִשְׁבְּרֵי, Pual mᵊlummāḏê→mᵊlummᵊḏê
                        // מְלֻמְּדֵי) reduces to a sheva; any C2 dagesh (Pual) stays.
                        let mut rseq = cseq.clone();
                        let mut reduced = false;
                        for c in rseq.iter_mut() {
                            if c.role == Role::Radical(2) && c.vowel == Some(Vowel::Qamats) {
                                c.vowel = Some(Vowel::Sheva);
                                reduced = true;
                            }
                        }
                        if reduced {
                            forms.push(VerbForm {
                                binyan,
                                form,
                                pgn,
                                text: hebrew::render(&rseq),
                                attested: c_attested,
                                object_suffix: None,
                            });
                        }
                    }
                    // The hollow alternants live outside the strong builder
                    // (Qal בָּאִים, Hiphil מְשִׁיבִים, Hophal מוּשָׁבִים); derive their
                    // mp constructs too — בָּאֵי, מְשִׁיבֵי, מוּשָׁבֵי.
                    let hollow_alts: Vec<String> = if root.has(Gizra::Hollow) {
                        match binyan {
                            Binyan::Qal => hollow_participle_twins(root, pgn)
                                .into_iter()
                                .chain(hollow_qal_participle_variant(&hebrew::render(&seq)))
                                .collect(),
                            Binyan::Hiphil if form == Form::ParticipleActive => {
                                hollow_hiphil_participle_variant(root, pgn)
                                    .into_iter()
                                    .collect()
                            }
                            Binyan::Hophal if form == Form::ParticipleActive => {
                                hollow_hophal_participle_variant(root, pgn)
                                    .into_iter()
                                    .collect()
                            }
                            _ => Vec::new(),
                        }
                    } else {
                        Vec::new()
                    };
                    for alt in hollow_alts {
                        let aseq = hebrew::parse_pointed(&alt);
                        if let Some(cseq) = participle_mp_construct(&aseq) {
                            forms.push(VerbForm {
                                binyan,
                                form,
                                pgn,
                                text: hebrew::render(&cseq),
                                attested: c_attested,
                                object_suffix: None,
                            });
                        }
                    }
                }
                // Construct-state twin for the masculine-singular III-He
                // participle: the segol before the etymological he raises to
                // tsere — ʿōśê (עֹשֵׂה), bōnê (בֹּנֵה) — the construct/bound form
                // of the segol absolute (עֹשֶׂה). Same label; only the surface
                // differs.
                if matches!(form, Form::ParticipleActive | Form::ParticiplePassive)
                    && root.lamed() == letter::HE
                    && pgn.gender == Some(Gender::Masculine)
                    && pgn.number == Some(Number::Singular)
                {
                    let seq = build_strong(root, binyan, form, pgn, false);
                    let (mut seq, c_attested) = apply_gizra(seq, root, binyan, form, pgn);
                    let n = seq.len();
                    if n >= 2
                        && seq[n - 1].letter == letter::HE
                        && seq[n - 2].vowel == Some(Vowel::Segol)
                    {
                        seq[n - 2].vowel = Some(Vowel::Tsere);
                        forms.push(VerbForm {
                            binyan,
                            form,
                            pgn,
                            text: hebrew::render(&seq),
                            attested: c_attested,
                            object_suffix: None,
                        });
                    }
                }
            }
        }
    }
    // The Masoretic text frequently omits a forte dagesh under a vocal sheva —
    // mᵊḇaqqᵊšê surfaces as מְבַקְשֵׁי, yᵊḏabbᵊrû as יְדַבְּרוּ beside יְדַבְּרוּ.
    // Emit a de-dotted twin of every form carrying such a dagesh, sharing its
    // analysis label. Additive: exact-match keeps only the spellings that occur.
    let dropped: Vec<VerbForm> = forms
        .iter()
        .filter_map(|f| {
            dagesh_sheva_variant(&f.text).map(|t| VerbForm {
                text: t,
                ..f.clone()
            })
        })
        .collect();
    forms.extend(dropped);

    // Pausal-tsere twin: at a major pause a reduced (vocal-sheva) theme before a
    // vocalic suffix lengthens to tsere — yiqṭᵊlû → yiqṭēlû, tōʔḵᵊlû → tōʔḵēlû
    // (תֹּאכְלוּ → תֹּאכֵלוּ). Add the twin for forms ending in a -û/-î suffix.
    let pausal: Vec<VerbForm> = forms
        .iter()
        .filter_map(|f| {
            pausal_tsere_variant(&f.text).map(|t| VerbForm {
                text: t,
                ..f.clone()
            })
        })
        .collect();
    forms.extend(pausal);

    // Furtive-patah twin: a word ending in a guttural ח/ע after a heterogeneous
    // (non-a) vowel takes a furtive patah under that guttural — môšîaʕ (מוֹשִׁיעַ),
    // šōmēaʕ (שֹׁמֵעַ), yašmîaʕ. The bare generator omits it; add the twin.
    let furtive: Vec<VerbForm> = forms
        .iter()
        .filter_map(|f| {
            furtive_patah_variant(&f.text).map(|t| VerbForm {
                text: t,
                ..f.clone()
            })
        })
        .collect();
    forms.extend(furtive);

    Paradigm {
        root: root.clone(),
        forms,
    }
}

/// Pausal-tsere twin: a form ending in a vocalic suffix (û = vav-shureq, or î =
/// final vowelless yod) whose theme reduced to a vocal sheva restores that sheva
/// to tsere in pause — tōʔḵᵊlû (תֹּאכְלוּ) → tōʔḵēlû (תֹּאכֵלוּ), yišmᵊrû →
/// yišmērû. Lengthens the last vocal sheva before the suffix. `None` otherwise.
fn pausal_tsere_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 3 {
        return None;
    }
    let last = &seq[n - 1];
    let vocalic_end = (last.letter == letter::VAV && last.dagesh && last.vowel.is_none())
        || (last.letter == letter::YOD && last.vowel.is_none());
    if !vocalic_end {
        return None;
    }
    for i in (1..n - 1).rev() {
        if seq[i].vowel == Some(Vowel::Sheva) {
            seq[i].vowel = Some(Vowel::Tsere);
            return Some(hebrew::render(&seq));
        }
    }
    None
}

/// Hollow Qal active-participle twin: the participle of a hollow root is the
/// qāmÌ„ shape with qamats on C1 — rāṣ (רָץ), qām (קָם), bāʔ (בָּא), šāḇ (שָׁב),
/// pl. rāṣîm (רָצִים) — but the strong qôṭēl base leaves a holam (rōṣ רֹץ).
/// Swap the first consonant's holam to qamats. Caller gates to hollow Qal
/// active participles; additive, so exact-match keeps only the real spelling.
fn hollow_qal_participle_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    if seq.first()?.vowel == Some(Vowel::Holam) {
        seq[0].vowel = Some(Vowel::Qamats);
        return Some(hebrew::render(&seq));
    }
    None
}

/// Hollow Hiphil active participle: the strong `maqṭîl` base mishandles a hollow
/// root by keeping the middle vav as a radical (mabwîʔ מַבְוִיא). The real shape
/// drops the middle radical entirely — mēCîC: mēḇîʔ (מֵבִיא), pl. mᵉḇîʔîm
/// (מְבִיאִים), fs mᵉḇîʔâ (מְבִיאָה), fp mᵉḇîʔôt (מְבִיאוֹת); likewise mēqîm קום,
/// mēšîḇ שוב. The mem carries tsere in the ms but reduces propretonically to a
/// vocal sheva once an ending shifts the stress. Built straight from the root;
/// caller gates to (Hiphil, ParticipleActive, Hollow). Additive alternant —
/// exact-match keeps only the spelling that occurs.
fn hollow_hiphil_participle_variant(root: &Root, pgn: Pgn) -> Option<String> {
    use Vowel::*;
    let (mem_vowel, c3_vowel, mut tail): (Vowel, Option<Vowel>, Vec<Cons>) =
        match (pgn.gender, pgn.number) {
            (Some(Gender::Masculine), Some(Number::Singular)) => (Tsere, None, vec![]),
            (Some(Gender::Masculine), Some(Number::Plural)) => (
                Sheva,
                Some(Hiriq),
                vec![Cons::new(letter::YOD), Cons::new(letter::MEM)],
            ),
            (Some(Gender::Feminine), Some(Number::Singular)) => {
                (Sheva, Some(Qamats), vec![Cons::new(letter::HE)])
            }
            (Some(Gender::Feminine), Some(Number::Plural)) => {
                (Sheva, Some(Holam), vec![Cons::new(letter::TAV)])
            }
            _ => return None,
        };
    let c3 = rad(root.lamed(), 3);
    let mut seq = vec![
        Cons::new(letter::MEM).with_vowel(mem_vowel),
        rad(root.pe(), 1).with_vowel(Hiriq),
        Cons::new(letter::YOD),
        match c3_vowel {
            Some(v) => c3.with_vowel(v),
            None => c3,
        },
    ];
    seq.append(&mut tail);
    Some(hebrew::render(&seq))
}

/// Hollow Hophal participle: like the Hiphil case the strong `moqṭāl` base keeps
/// the middle vav (mŏqwām מׇקְוָם); the real shape is mûC1āC3 — mûqām (מוּקָם),
/// mûšāḇ (מוּשָׁב), pl. mûqāmîm — the û (shureq) of the hollow Hophal under the
/// preformative mem, then C1 with qamats. C1's qamats is pretonic and stays
/// across the endings; the û never reduces. Caller gates to (Hophal,
/// ParticipleActive, Hollow). Additive alternant.
fn hollow_hophal_participle_variant(root: &Root, pgn: Pgn) -> Option<String> {
    use Vowel::*;
    // The stem vowel (qamats) sits on C1; C3 is the bare final radical in the ms
    // and carries the inflectional ending vowel otherwise.
    let (c3_vowel, mut tail): (Option<Vowel>, Vec<Cons>) = match (pgn.gender, pgn.number) {
        (Some(Gender::Masculine), Some(Number::Singular)) => (None, vec![]),
        (Some(Gender::Masculine), Some(Number::Plural)) => (
            Some(Hiriq),
            vec![Cons::new(letter::YOD), Cons::new(letter::MEM)],
        ),
        (Some(Gender::Feminine), Some(Number::Singular)) => {
            (Some(Qamats), vec![Cons::new(letter::HE)])
        }
        (Some(Gender::Feminine), Some(Number::Plural)) => {
            (Some(Holam), vec![Cons::new(letter::TAV)])
        }
        _ => return None,
    };
    let c3 = rad(root.lamed(), 3);
    let mut seq = vec![
        Cons::new(letter::MEM),
        Cons::new(letter::VAV).with_dagesh(),
        rad(root.pe(), 1).with_vowel(Qamats),
        match c3_vowel {
            Some(v) => c3.with_vowel(v),
            None => c3,
        },
    ];
    seq.append(&mut tail);
    Some(hebrew::render(&seq))
}

/// I-guttural Hiphil segol-prefix twin: the hē- (haC-) prefix attenuates to
/// segol before a guttural C1 — heʕĕmîqû הֶעֱמִיקוּ beside haʕămîqû, heḥĕšû הֶחֱשׁוּ.
/// The prefix patah → segol; a following guttural's hataf-patah → hataf-segol (a
/// silent sheva, where the prefix syllable closes, stays). The perfect already
/// generates this grade; this lifts it to the imperative. Additive.
fn hiphil_iguttural_segol_prefix_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    if seq.len() < 2
        || seq[0].letter != letter::HE
        || seq[0].vowel != Some(Vowel::Patah)
        || !hebrew::is_guttural(seq[1].letter)
    {
        return None;
    }
    seq[0].vowel = Some(Vowel::Segol);
    if seq[1].vowel == Some(Vowel::HatafPatah) {
        seq[1].vowel = Some(Vowel::HatafSegol);
    }
    Some(hebrew::render(&seq))
}

/// Hollow Qal perfect 2mp/2fp (קַמְתֶּם, קַמְתֶּן; III-aleph בָּאתֶם). The light
/// consonantal-suffix forms (2ms קַמְתָּ, 1cs קַמְתִּי) already generate, but the
/// heavy -תֶם/-תֶן endings come out reduced (qᵊ/bᵊ-) instead of keeping the stem
/// vowel. That vowel is identical to the 2ms's (patah for an ā-stem like קום,
/// qamats for III-aleph בוא), so we derive the form from the 2ms — swapping its
/// final -תָּ (tav+qamats) for -תֶם/-תֶן — which carries the right vowel for
/// either class for free. Caller gates to (Qal, Perfect, Hollow, 2mp|2fp).
fn hollow_qal_perfect_heavy_suffix_variant(root: &Root, pgn: Pgn) -> Option<String> {
    let tail = match (pgn.person, pgn.gender, pgn.number) {
        (Some(Person::Second), Some(Gender::Masculine), Some(Number::Plural)) => letter::MEM,
        (Some(Person::Second), Some(Gender::Feminine), Some(Number::Plural)) => letter::NUN,
        _ => return None,
    };
    let two_ms = Pgn::new(Person::Second, Gender::Masculine, Number::Singular);
    let (base, _) = generate_one(root, Binyan::Qal, Form::Perfect, two_ms, false);
    let mut seq = hebrew::parse_pointed(&base);
    // The 2ms ends in tav + qamats; re-point the tav to segol and add the
    // heavy-ending consonant (keeping the tav's dagesh lene).
    let last = seq.last_mut()?;
    if last.letter != letter::TAV {
        return None;
    }
    last.vowel = Some(Vowel::Segol);
    seq.push(Cons::new(tail));
    Some(hebrew::render(&seq))
}

/// III-aleph participle plural reduction: before a vocalic ending the final
/// aleph turns consonantal again and the thematic qamats on the preceding
/// radical reduces to a vocal sheva — niCˌāʔ → niCˌᵉʔîm: nimṣāʔ נִמְצָא →
/// nimṣᵉʔîm נִמְצְאִים, fp נִמְצְאוֹת; likewise the Niphal/Pual/Hophal participles
/// of any III-aleph root. The strong base wrongly keeps the qamats
/// (נִמְצָאִים). Caller gates to (participle, LamedAleph, plural). Additive.
fn lamed_aleph_participle_reduce_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    for i in 1..seq.len() {
        if seq[i].letter == letter::ALEF && seq[i - 1].vowel == Some(Vowel::Qamats) {
            seq[i - 1].vowel = Some(Vowel::Sheva);
            return Some(hebrew::render(&seq));
        }
    }
    None
}

/// Pe-aleph wayyiqtol 1cp segol twin (וַנֹּאמֶר). The stress-retracted wayyiqtol
/// lowers the theme to segol for most persons (3ms וַיֹּאמֶר, 2ms וַתֹּאמֶר), but
/// the generator keeps the patah on the 1cp (וַנֹּאמַר) the way it must for the
/// 1cs (וָאֹמַר, whose aleph merges). The 1cp behaves like the 2ms/3ms, so emit
/// the segol twin. Caller gates to (Qal, Wayyiqtol, PeAleph, 1cp). Additive.
fn pe_aleph_wayyiqtol_segol_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    // The theme sits on the penultimate consonant (before the final radical).
    let n = seq.len();
    if n < 2 {
        return None;
    }
    let c = &mut seq[n - 2];
    if c.vowel == Some(Vowel::Patah) {
        c.vowel = Some(Vowel::Segol);
        return Some(hebrew::render(&seq));
    }
    None
}

/// II-guttural derived-stem hataf twin: when the middle radical is a guttural or
/// resh, the vocal sheva it would take before a vocalic suffix becomes a
/// hataf-patah — bērᵊḵû → bārăḵû (בָּרֲכוּ), wayᵊḇārᵊḵû → wayᵊḇārăḵû (וַיְבָרֲכוּ).
/// The strong Piel/Pual/Hithpael base leaves a plain sheva (בָּרְכוּ). Caller
/// gates to (Piel|Pual|Hithpael, AyinGuttural); additive — re-points the C2
/// (root.ayin()) sheva, which in these stems is only ever the pre-suffixal
/// vocal one.
fn ayin_guttural_hataf_variant(root: &Root, text: &str) -> Option<String> {
    let g = root.ayin();
    let mut seq = hebrew::parse_pointed(text);
    for c in seq.iter_mut() {
        if c.letter == g && c.vowel == Some(Vowel::Sheva) {
            c.vowel = Some(Vowel::HatafPatah);
            return Some(hebrew::render(&seq));
        }
    }
    None
}

/// III-He participle fs construct: the -â feminine ending (qamats on the last
/// stem consonant + he, maʕălâ מַעֲלָה) binds as -aṯ — the he becomes a tav and
/// the qamats shortens to patah: maʕălaṯ מַעֲלַת. Returns `None` unless the form
/// ends in a vowelless he preceded by a qamats.
fn lamed_he_participle_fs_construct_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n >= 2
        && seq[n - 1].letter == letter::HE
        && seq[n - 1].vowel.is_none()
        && seq[n - 2].vowel == Some(Vowel::Qamats)
    {
        seq[n - 1] = Cons::new(letter::TAV);
        seq[n - 2].vowel = Some(Vowel::Patah);
        return Some(hebrew::render(&seq));
    }
    None
}

/// II-guttural Qal imperative a-harmony twin: the vocalic-suffix imperative
/// (qiṭlû/qiṭlî) gives C2 a silent sheva that closes the C1 syllable — but a
/// guttural C2 opens with a hataf-patah instead, and the C1 hiriq harmonises
/// to patah: ziʿqû זִעְקוּ → zaʿăqû זַעֲקוּ. Caller gates to (Qal, Imperative,
/// AyinGuttural, 2fs|2mp); additive. C1 is seq[0] and the guttural C2 seq[1]
/// (no prefix in the Qal imperative).
fn qal_imperative_ayin_guttural_a_variant(root: &Root, text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    if seq.len() >= 2
        && seq[0].letter == root.pe()
        && seq[0].vowel == Some(Vowel::Hiriq)
        && seq[1].letter == root.ayin()
        && seq[1].vowel == Some(Vowel::Sheva)
    {
        seq[0].vowel = Some(Vowel::Patah);
        seq[1].vowel = Some(Vowel::HatafPatah);
        return Some(hebrew::render(&seq));
    }
    None
}

/// II-guttural Piel/Hithpael perfect virtual-doubling twin: where the guttural
/// C2 forgoes the forte the C1 prefix may keep its short hiriq (niʾăṣû נִאֲצוּ,
/// niʾēṣ נִאֵץ) instead of compensatorily lengthening to tsere (nēʾēṣ נֵאֵץ).
/// Emit the hiriq grade beside the builder's lengthened one. The structural
/// guard (a tsere on C1 directly before the guttural C2) keeps it inert for the
/// stems and persons that never lengthen there.
fn piel_perfect_guttural_hiriq_variant(root: &Root, text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let i = (1..seq.len()).find(|&i| {
        seq[i].letter == root.ayin()
            && hebrew::is_guttural(seq[i].letter)
            && seq[i - 1].vowel == Some(Vowel::Tsere)
    })?;
    seq[i - 1].vowel = Some(Vowel::Hiriq);
    Some(hebrew::render(&seq))
}

/// Furtive-patah twin: when a form ends in a vowelless guttural ח or ע preceded
/// by a heterogeneous vowel (a long î/ô/û mater, or tsere/segol/hiriq/holam/
/// qubuts), insert a patah under the final guttural — môšîaʕ (מוֹשִׁיעַ), šōmēaʕ
/// (שֹׁמֵעַ). Returns `None` otherwise (incl. after an a-class vowel, where no
/// furtive patah arises, e.g. yišmaʕ יִשְׁמַע).
fn furtive_patah_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 2 {
        return None;
    }
    let last = seq[n - 1];
    if !matches!(last.letter, letter::HET | letter::AYIN) || last.vowel.is_some() {
        return None;
    }
    let prev = seq[n - 2];
    let triggers = match prev.vowel {
        // A vowelless yod/vav before the guttural is a long-vowel mater (î/ô/û).
        None => matches!(prev.letter, letter::YOD | letter::VAV),
        Some(v) => matches!(
            v,
            Vowel::Tsere | Vowel::Segol | Vowel::Hiriq | Vowel::Holam | Vowel::Qubuts
        ),
    };
    if !triggers {
        return None;
    }
    seq[n - 1].vowel = Some(Vowel::Patah);
    Some(hebrew::render(&seq))
}

/// Twin with the forte dagesh dropped from any consonant bearing a vocal sheva
/// (the Masoretic "dagesh omitted under sheva": מְבַקְּשֵׁי → מְבַקְשֵׁי). Returns
/// `None` when there is no such dagesh.
fn dagesh_sheva_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let mut changed = false;
    for c in seq.iter_mut() {
        if c.dagesh && c.vowel == Some(Vowel::Sheva) {
            c.dagesh = false;
            changed = true;
        }
    }
    changed.then(|| hebrew::render(&seq))
}

const FORMS_FOR_PARADIGM: &[Form] = &[
    Form::Perfect,
    Form::Imperfect,
    Form::Imperative,
    Form::Cohortative,
    Form::Jussive,
    Form::Wayyiqtol,
    Form::InfinitiveConstruct,
    Form::InfinitiveAbsolute,
    Form::ParticipleActive,
    Form::ParticiplePassive,
];

fn binyan_has_form(binyan: Binyan, form: Form) -> bool {
    // Only Qal distinguishes active and passive participle as separate forms.
    // Every other binyan has a single participle, which we put under
    // `ParticipleActive`.
    if form == Form::ParticiplePassive && binyan != Binyan::Qal {
        return false;
    }
    true
}

fn pgns_for_form(form: Form) -> &'static [Pgn] {
    match form {
        Form::Perfect => PERFECT_PGNS,
        Form::Imperfect | Form::Wayyiqtol => IMPERFECT_PGNS,
        Form::Imperative => IMPERATIVE_PGNS,
        Form::Cohortative => COHORTATIVE_PGNS,
        Form::Jussive => JUSSIVE_PARADIGM_PGNS,
        Form::InfinitiveConstruct | Form::InfinitiveAbsolute => {
            const NONE: &[Pgn] = &[Pgn::none()];
            NONE
        }
        Form::ParticipleActive | Form::ParticiplePassive => PARTICIPLE_PGNS,
    }
}

fn generate_one(
    root: &Root,
    binyan: Binyan,
    form: Form,
    pgn: Pgn,
    force_a_theme: bool,
) -> (String, bool) {
    if form == Form::Wayyiqtol {
        return build_wayyiqtol(root, binyan, pgn, force_a_theme);
    }
    let seq = build_strong(root, binyan, form, pgn, force_a_theme);
    let (seq, attested) = apply_gizra(seq, root, binyan, form, pgn);
    (hebrew::render(&seq), attested)
}

/// Build a vav-consecutive imperfect (wayyiqtol). The base is the short
/// (jussive) stem where one exists for this PGN, otherwise the ordinary
/// imperfect. We then prepend the consecutive vav — patah plus a forte dagesh
/// doubling the prefix consonant, or qamats with no dagesh before the
/// non-doubling 1cs aleph.
///
/// Stress retraction (nesiga) shortens the hollow stem vowel: the jussive's
/// holam becomes qamats (wayyāqom → וַיָּקָם), except when a quiescent III-aleph
/// preserves it (wayyāḇōʔ → וַיָּבֹא). Retracted hollow forms depend on the
/// verb's lexical vowel class, so they're flagged `attested = false`.
fn build_wayyiqtol(root: &Root, binyan: Binyan, pgn: Pgn, force_a_theme: bool) -> (String, bool) {
    let short = JUSSIVE_PGNS.contains(&pgn) || pgn.person == Some(Person::First);
    build_wayyiqtol_with(root, binyan, pgn, force_a_theme, short)
}

/// Long (imperfect-based) wayyiqtol, for the persons where [`build_wayyiqtol`]
/// would default to the short jussive base — the non-apocopated twin (וָאֶרְאֶה,
/// וַיַּכֶּה).
fn build_wayyiqtol_long(
    root: &Root,
    binyan: Binyan,
    pgn: Pgn,
    force_a_theme: bool,
) -> (String, bool) {
    build_wayyiqtol_with(root, binyan, pgn, force_a_theme, false)
}

fn build_wayyiqtol_with(
    root: &Root,
    binyan: Binyan,
    pgn: Pgn,
    force_a_theme: bool,
    short: bool,
) -> (String, bool) {
    let base_form = if short {
        Form::Jussive
    } else {
        Form::Imperfect
    };
    let seq = build_strong(root, binyan, base_form, pgn, force_a_theme);
    let (mut seq, attested) = apply_gizra(seq, root, binyan, base_form, pgn);

    if short
        && root.has(Gizra::Hollow)
        && root.lamed() != letter::ALEF
        && let Some(i) = radical_idx(&seq, 1)
        && seq[i].vowel == Some(Vowel::Holam)
    {
        seq[i].vowel = Some(Vowel::Qamats);
    }

    // î-class hollow nesiga: the short jussive's tsere (yāśēm, yāḇēn) retracts
    // to segol under the consecutive vav (וַיָּשֶׂם, וַיָּבֶן). III-aleph keeps
    // the tsere — the quiescent aleph wants the long vowel (וַיָּבֵא).
    if short
        && root.has(Gizra::Hollow)
        && root.lamed() != letter::ALEF
        && let Some(i) = radical_idx(&seq, 1)
        && seq[i].vowel == Some(Vowel::Tsere)
    {
        seq[i].vowel = Some(Vowel::Segol);
    }

    // Hiphil short-wayyiqtol nesiga: the jussive's tsere theme retracts to
    // segol under the consecutive vav — wayyaggēḏ → וַיַּגֶּד, וַיֹּסֶף,
    // וַיּוֹלֶד. A final guttural prefers the a-class instead: the tsere (and
    // its furtive patah) collapse to a plain patah — wayyôšaʿ וַיּוֹשַׁע.
    // III-aleph keeps the tsere (the quiescent aleph wants the long vowel),
    // and the hollow class is handled above.
    if short
        && binyan == Binyan::Hiphil
        && !root.has(Gizra::Hollow)
        && root.lamed() != letter::ALEF
        && let Some(i) = radical_idx(&seq, 2)
        && seq[i].vowel == Some(Vowel::Tsere)
    {
        let guttural_final = seq
            .last()
            .is_some_and(|c| matches!(c.letter, letter::HET | letter::AYIN));
        if guttural_final {
            seq[i].vowel = Some(Vowel::Patah);
            if let Some(last) = seq.last_mut() {
                last.vowel = None; // the furtive patah goes with the tsere
            }
        } else {
            seq[i].vowel = Some(Vowel::Segol);
        }
    }

    // I-Aleph nesiga: the imperfect's stressed C2 patah (yōʔmar) retracts to
    // segol when the consecutive vav pulls the stress back (wayyōʔmer →
    // וַיֹּאמֶר). Vocalic-suffix forms keep their sheva (וַיֹּאמְרוּ) and are
    // untouched.
    // The 1cs (aleph preformative) does not retract — it keeps patah: וָאֹמַר.
    if root.has(Gizra::PeAleph)
        && binyan == Binyan::Qal
        && pgn.person != Some(Person::First)
        && let Some(i) = radical_idx(&seq, 2)
        && seq[i].vowel == Some(Vowel::Patah)
    {
        seq[i].vowel = Some(Vowel::Segol);
    }

    // I-Yod / הלך nesiga: the open-prefix imperfect's tsere theme (yēšēḇ,
    // yēleḵ) retracts to segol under the consecutive vav (וַיֵּשֶׁב, וַיֵּלֶךְ).
    // Gated to this class so it never touches forms whose tsere is stable,
    // e.g. נתן's closed-prefix וַיִּתֵּן. III-aleph keeps tsere (וַיֵּצֵא).
    if (root.has(Gizra::PeYod) || is_halak(root))
        && binyan == Binyan::Qal
        && root.lamed() != letter::ALEF
        && let Some(i) = radical_idx(&seq, 2)
        && seq[i].vowel == Some(Vowel::Tsere)
    {
        seq[i].vowel = Some(Vowel::Segol);
    }

    // Hollow shureq-class plural wayyiqtol: the long û (mater vav) shortens to
    // qubuts on C1 under stress retraction (wayyāšuḇû → וַיָּשֻׁבוּ, not the long
    // וַיָּשׁוּבוּ). Only the vocalic-suffix forms have the mater to shorten; the
    // singular short stem is already handled via the jussive base above.
    if !short
        && root.has(Gizra::Hollow)
        && hollow_class(root) == HollowClass::Shureq
        && imperfect_suffix_kind(pgn) == Suffix::Vocalic
        && let Some(c1) = radical_idx(&seq, 1)
        && seq
            .get(c1 + 1)
            .is_some_and(|c| c.letter == letter::VAV && c.role == Role::Mater)
    {
        seq[c1].vowel = Some(Vowel::Qubuts);
        seq.remove(c1 + 1);
    }

    // III-strong / I-guttural stress-retraction nesiga: some Qal verbs shorten
    // the C2 holam to patah when the consecutive-vav stress is pulled back
    // (וַיַּחֲזֹק → וַיֶּחֱזַק). Gated lexically because it is not universal.
    if is_hazaq(root)
        && binyan == Binyan::Qal
        && short
        && let Some(c2) = radical_idx(&seq, 2)
        && seq[c2].vowel == Some(Vowel::Holam)
    {
        seq[c2].vowel = Some(Vowel::Patah);
    }

    let prefix_is_aleph = seq
        .first()
        .map(|c| c.letter == letter::ALEF)
        .unwrap_or(false);
    if prefix_is_aleph
        && short
        && root.has(Gizra::LamedHe)
        && let Some(first) = seq.first_mut()
        && first.vowel == Some(Vowel::Segol)
    {
        first.vowel = Some(Vowel::Tsere);
    }

    if let Some(first) = seq.first_mut() {
        // The consecutive vav doubles the preformative consonant — except a
        // *yod* carrying a vocal sheva, where the Masoretes omit the forte.
        // This is the Piel/Pual prefix yᵉ- (wayᵉḇāreḵ → וַיְבָרֶךְ, not
        // וַיְּבָרֶךְ) and חיה/היה's sheva-prefix wayyiqtol (וַיְחִי, וַיְהִי).
        // The same sheva on a non-yod preformative keeps the dagesh — the 3fs
        // וַתְּהִי, Piel וַתְּדַבֵּר. Hithpael's yiṯ- keeps its hiriq and so
        // still doubles.
        let sheva_prefix = first.vowel == Some(Vowel::Sheva) && first.letter == letter::YOD;
        first.dagesh = !prefix_is_aleph && !sheva_prefix;
    }
    let vav_vowel = if prefix_is_aleph {
        Vowel::Qamats
    } else {
        Vowel::Patah
    };
    seq.insert(0, Cons::new(letter::VAV).with_vowel(vav_vowel));

    (hebrew::render(&seq), attested && !root.has(Gizra::Hollow))
}

// ----------------------------------------------------------------------------
// Strong-verb construction
// ----------------------------------------------------------------------------

/// Which "vowel grade" the stem takes given the suffix that follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Suffix {
    /// No suffix at all (e.g. Perfect 3ms).
    Zero,
    /// Vocalic suffix beginning with a vowel — triggers v2 reduction in
    /// Perfect (kāṯlâ, kāṯəlû) and Imperfect (yiqṭəlû).
    Vocalic,
    /// Consonantal suffix (the -tā / -t / -tî / -nû family in Perfect, or
    /// -nâ in Imperfect). v3 takes sheva to close the syllable.
    Consonantal,
    /// Heavy consonantal suffix (2mp/2fp Perfect: -tem/-ten). Stress shifts
    /// onto the suffix, so v1 propretonically reduces in addition to v3.
    Heavy,
}

fn perfect_suffix_kind(pgn: Pgn) -> Suffix {
    match (pgn.person, pgn.gender, pgn.number) {
        (Some(Person::Third), Some(Gender::Masculine), Some(Number::Singular)) => Suffix::Zero,
        (Some(Person::Third), Some(Gender::Feminine), Some(Number::Singular)) => Suffix::Vocalic,
        (Some(Person::Third), _, Some(Number::Plural)) => Suffix::Vocalic,
        (Some(Person::Second), _, Some(Number::Plural)) => Suffix::Heavy,
        _ => Suffix::Consonantal,
    }
}

fn imperfect_suffix_kind(pgn: Pgn) -> Suffix {
    match (pgn.gender, pgn.number) {
        // -î (2fs), -û (3mp, 2mp)
        (Some(Gender::Feminine), Some(Number::Singular)) if pgn.person == Some(Person::Second) => {
            Suffix::Vocalic
        }
        (Some(Gender::Masculine), Some(Number::Plural)) => Suffix::Vocalic,
        // -nâ (3fp, 2fp)
        (Some(Gender::Feminine), Some(Number::Plural)) => Suffix::Consonantal,
        _ => Suffix::Zero,
    }
}

/// Vowels appearing under the three radicals in the strong verb. For each
/// binyan we record both the "long" grade (used with Zero/Consonantal/Heavy
/// suffixes) and the "short" grade (vocalic suffix → v2 reduces).
struct StemVowels {
    /// (v1, v2) for Zero / Consonantal / Heavy suffix grade.
    long: (Vowel, Vowel),
    /// (v1, v2) for Vocalic suffix grade — typically v2 = Sheva.
    vocalic: (Vowel, Vowel),
    /// In 2mp/2fp Perfect, v1 reduces propretonically to sheva (or matching
    /// hataf under gutturals). True for most binyanim; false for those
    /// whose v1 is already a short vowel that doesn't reduce (Hiphil's
    /// hi-prefix is part of the prefix, not v1).
    heavy_reduces_v1: bool,
}

/// Imperfect prefix consonants (before the C1 of the root). Per binyan, the
/// prefix uses a fixed consonant slot whose actual letter depends on PGN
/// (yod/tav/alef/nun) but whose vowel is fixed by the binyan.
#[derive(Debug, Clone, Copy)]
struct ImperfectPrefix {
    /// Vowel under the y/t/ʔ/n prefix consonant.
    vowel: Vowel,
    /// Whether the first radical takes a dagesh (Niphal: assimilated nun).
    c1_dagesh: bool,
    /// Vowel under C1 in the imperfect stem (long grade).
    v1_long: Vowel,
    /// Vowel under C2 in the imperfect stem (long grade — Zero/Consonantal).
    v2_long: Vowel,
    /// Vowel under C2 in the imperfect stem when a vocalic suffix is
    /// attached (typically Sheva — yiqṭəlû).
    v2_vocalic: Vowel,
    /// Optional matres-lectionis letter after C2 in long grade (Hiphil: yod).
    mater_after_c2_long: Option<char>,
}

/// Build the strong-verb form for (binyan, form, pgn) as a `Vec<Cons>`.
fn build_strong(
    root: &Root,
    binyan: Binyan,
    form: Form,
    pgn: Pgn,
    force_a_theme: bool,
) -> Vec<Cons> {
    match form {
        Form::Perfect => build_perfect(root, binyan, pgn),
        Form::Imperfect => build_imperfect(root, binyan, pgn, false, force_a_theme),
        Form::Cohortative => build_cohortative(root, binyan, pgn),
        Form::Jussive => build_imperfect(root, binyan, pgn, true, force_a_theme),
        Form::Wayyiqtol => unreachable!("wayyiqtol is built via build_wayyiqtol"),
        Form::Imperative => build_imperative(root, binyan, pgn, force_a_theme),
        Form::InfinitiveConstruct => build_inf_construct(root, binyan),
        Form::InfinitiveAbsolute => build_inf_absolute(root, binyan),
        Form::ParticipleActive => build_participle(root, binyan, pgn, false),
        Form::ParticiplePassive => build_participle(root, binyan, pgn, true),
    }
}

// ----- Perfect ---------------------------------------------------------------

fn perfect_vowels(binyan: Binyan) -> StemVowels {
    use Vowel::*;
    match binyan {
        // qāṭal — long a + a. Vocalic suffix: kāṯlâ → v2 = sheva.
        // 2mp/2fp: qəṭaltem — v1 reduces to sheva.
        Binyan::Qal => StemVowels {
            long: (Qamats, Patah),
            vocalic: (Qamats, Sheva),
            heavy_reduces_v1: true,
        },
        // niqṭal — silent sheva on C1, patah on C2. Vocalic suffix:
        // niqṭəlâ → v2 = sheva. 2mp/2fp: niqṭaltem — first vowel is the
        // hiriq of the n- prefix (handled separately); the stem's v1 = sheva
        // doesn't itself reduce further.
        Binyan::Niphal => StemVowels {
            long: (Sheva, Patah),
            vocalic: (Sheva, Sheva),
            heavy_reduces_v1: false,
        },
        // qiṭṭēl — hiriq + tsere with doubled C2.
        // Vocalic suffix: qiṭṭəlâ → v2 = sheva.
        Binyan::Piel => StemVowels {
            long: (Hiriq, Tsere),
            vocalic: (Hiriq, Sheva),
            heavy_reduces_v1: false,
        },
        // quṭṭal — qubuts + patah with doubled C2.
        Binyan::Pual => StemVowels {
            long: (Qubuts, Patah),
            vocalic: (Qubuts, Sheva),
            heavy_reduces_v1: false,
        },
        // hitqaṭṭēl — handled with an explicit hit- prefix.
        Binyan::Hithpael => StemVowels {
            long: (Patah, Tsere),
            vocalic: (Patah, Sheva),
            heavy_reduces_v1: false,
        },
        // hiqṭîl — long ī on C2 (with mater yod) for Zero/Vocalic suffix.
        // With consonantal/heavy suffix: hiqṭaltā / hiqṭaltem — v2 = patah,
        // no mater.
        Binyan::Hiphil => StemVowels {
            long: (Sheva, Hiriq),
            vocalic: (Sheva, Hiriq),
            heavy_reduces_v1: false,
        },
        // hoqṭal — ho-prefix (qamats-qatan on he) closes syllable on C1
        // (silent sheva), patah on C2.
        Binyan::Hophal => StemVowels {
            long: (Sheva, Patah),
            vocalic: (Sheva, Sheva),
            heavy_reduces_v1: false,
        },
    }
}

fn build_perfect(root: &Root, binyan: Binyan, pgn: Pgn) -> Vec<Cons> {
    use Vowel::*;
    let suffix = perfect_suffix_kind(pgn);
    let vowels = perfect_vowels(binyan);
    let (mut v1, v2) = match suffix {
        Suffix::Vocalic => vowels.vocalic,
        _ => vowels.long,
    };
    if suffix == Suffix::Heavy && vowels.heavy_reduces_v1 {
        v1 = Sheva;
    }

    let mut out: Vec<Cons> = Vec::new();

    // ---- Binyan-specific prefix block ----
    match binyan {
        Binyan::Niphal => {
            // ni- (consonantal): n with hiriq, the following C1 has sheva.
            out.push(Cons::new(letter::NUN).with_vowel(Hiriq));
        }
        Binyan::Hithpael => {
            // hit- (hi + t-) prefix. Sibilant metathesis & emphatic
            // assimilation are handled in apply_gizra (TODO); strong form
            // is plain hi-t-.
            out.push(Cons::new(letter::HE).with_vowel(Hiriq));
            out.push(Cons::new(letter::TAV).with_vowel(Sheva));
        }
        Binyan::Hiphil => {
            // hi- prefix.
            out.push(Cons::new(letter::HE).with_vowel(Hiriq));
            // In heavy-suffix Hiphil Perfect (hiqṭaltem) the first vowel of
            // the prefix actually reduces: hiqṭaltem → heqṭaltem? No, it
            // stays hi-. Skip.
        }
        Binyan::Hophal => {
            // ho- (qamats-qatan) prefix.
            out.push(Cons::new(letter::HE).with_vowel(QamatsQatan));
        }
        _ => {}
    }

    // ---- C1 ----
    let c1 = rad(root.pe(), 1).with_vowel(v1);
    // Niphal Perfect: ni- prefix only, no doubling on C1.
    // Hithpael: C1 takes patah in vocalic-suffix grade too (already set via vowels).
    out.push(c1);

    // ---- C2 ----
    let mut c2 = rad(root.ayin(), 2).with_vowel(v2);
    if matches!(binyan, Binyan::Piel | Binyan::Pual | Binyan::Hithpael) {
        c2 = c2.with_dagesh();
    }
    // Hiphil Perfect: with consonantal/heavy suffix v2 = patah, no mater.
    // With Zero/Vocalic suffix v2 = hiriq + mater yod.
    if binyan == Binyan::Hiphil {
        match suffix {
            Suffix::Zero | Suffix::Vocalic => {
                c2.vowel = Some(Hiriq);
            }
            Suffix::Consonantal | Suffix::Heavy => {
                c2.vowel = Some(Patah);
            }
        }
    }
    // Piel/Hithpael Perfect: the tsere lowers to patah before a consonantal or
    // heavy afformative (dibbēr → dibbartî דִּבַּרְתִּי, qiddēš → qiddaštî).
    if matches!(binyan, Binyan::Piel | Binyan::Hithpael)
        && matches!(suffix, Suffix::Consonantal | Suffix::Heavy)
    {
        c2.vowel = Some(Patah);
    }
    // Piel Perfect 3ms (Zero suffix): a final resh lowers the tsere to segol —
    // dibbēr → dibber דִּבֶּר, kippēr → kipper כִּפֶּר (Gesenius §52n). This is the
    // dominant spelling (e.g. דִּבֶּר outnumbers דִּבֵּר ~8:1 in the text).
    if binyan == Binyan::Piel && suffix == Suffix::Zero && root.lamed() == letter::RESH {
        c2.vowel = Some(Segol);
    }
    // Dagesh lene: in the prefixed binyanim the prefix syllable closes on C1's
    // silent sheva (niḵ-, hiḵ-, hoḵ-), so a begedkefet C2 begins a new syllable
    // and takes a dagesh lene — niḵtaḇ → נִכְתַּב, hiḵtîḇ → הִכְתִּיב.
    if matches!(binyan, Binyan::Niphal | Binyan::Hiphil | Binyan::Hophal)
        && v1 == Sheva
        && hebrew::is_begedkefet(root.ayin())
        && !c2.dagesh
    {
        c2 = c2.with_dagesh();
    }
    out.push(c2);

    // Hiphil mater yod after C2 in Zero/Vocalic grade:
    if binyan == Binyan::Hiphil && matches!(suffix, Suffix::Zero | Suffix::Vocalic) {
        out.push(Cons::new(letter::YOD));
    }

    // ---- C3 ----
    let v3 = match suffix {
        Suffix::Consonantal | Suffix::Heavy => Some(Sheva),
        Suffix::Vocalic => None, // C3 gets the suffix vowel attached as next Cons
        Suffix::Zero => None,
    };
    let mut c3 = rad(root.lamed(), 3);
    if let Some(v) = v3 {
        c3.vowel = Some(v);
    }
    // For Suffix::Vocalic the vowel goes on C3 since the suffix is a single
    // letter (he with qamats for 3fs; vav as shureq for 3cp). Vowel on C3
    // is the leading vowel of the suffix.
    if suffix == Suffix::Vocalic {
        match (pgn.person, pgn.gender, pgn.number) {
            (Some(Person::Third), Some(Gender::Feminine), Some(Number::Singular)) => {
                // -â: lamed with qamats, then he.
                c3.vowel = Some(Qamats);
            }
            (Some(Person::Third), _, Some(Number::Plural)) => {
                // -û: lamed with no vowel, then vav with dagesh (shureq).
            }
            _ => {}
        }
    }
    out.push(c3);

    // ---- Suffix ----
    match (pgn.person, pgn.gender, pgn.number) {
        (Some(Person::Third), Some(Gender::Masculine), Some(Number::Singular)) => {} // ø
        (Some(Person::Third), Some(Gender::Feminine), Some(Number::Singular)) => {
            out.push(Cons::new(letter::HE));
        }
        (Some(Person::Third), _, Some(Number::Plural)) => {
            // shureq
            out.push(Cons::new(letter::VAV).with_dagesh());
        }
        (Some(Person::Second), Some(Gender::Masculine), Some(Number::Singular)) => {
            // -tā
            out.push(Cons::new(letter::TAV).with_dagesh().with_vowel(Qamats));
        }
        (Some(Person::Second), Some(Gender::Feminine), Some(Number::Singular)) => {
            // -t (silent sheva)
            out.push(Cons::new(letter::TAV).with_dagesh().with_vowel(Sheva));
        }
        (Some(Person::First), _, Some(Number::Singular)) => {
            // -tî
            out.push(Cons::new(letter::TAV).with_dagesh().with_vowel(Hiriq));
            out.push(Cons::new(letter::YOD));
        }
        (Some(Person::Second), Some(Gender::Masculine), Some(Number::Plural)) => {
            // -tem
            out.push(Cons::new(letter::TAV).with_dagesh().with_vowel(Segol));
            out.push(Cons::new(letter::MEM));
        }
        (Some(Person::Second), Some(Gender::Feminine), Some(Number::Plural)) => {
            // -ten
            out.push(Cons::new(letter::TAV).with_dagesh().with_vowel(Segol));
            out.push(Cons::new(letter::NUN));
        }
        (Some(Person::First), _, Some(Number::Plural)) => {
            // -nû
            out.push(Cons::new(letter::NUN).with_vowel(Qubuts));
            // shureq written as vav with dagesh after a vowel-less context;
            // here the qubuts on nun is the "u" — actually -nû is rendered
            // as nun-shureq: nun with no vowel, vav with dagesh.
            // Convention varies; using nun + shureq-vav.
            out.pop();
            out.push(Cons::new(letter::NUN));
            out.push(Cons::new(letter::VAV).with_dagesh());
        }
        _ => {}
    }

    // III-tav coalescence: a root-final ת immediately before a t-initial
    // afformative (-tî/-tā/-t/-tem/-ten) merges with it into a single geminated
    // תּ — kāraṯ-tî → כָּרַתִּי, not כָּרַתְתִּי. The suffix tav already carries the
    // dagesh marking the gemination, so we just drop the silent-sheva C3 tav.
    if matches!(suffix, Suffix::Consonantal | Suffix::Heavy) && root.lamed() == letter::TAV {
        for i in 0..out.len().saturating_sub(1) {
            if out[i].letter == letter::TAV
                && out[i].vowel == Some(Sheva)
                && !out[i].dagesh
                && out[i + 1].letter == letter::TAV
                && out[i + 1].dagesh
            {
                out.remove(i);
                break;
            }
        }
    }

    out
}

// ----- Imperfect / Jussive ---------------------------------------------------

fn imperfect_prefix(binyan: Binyan) -> ImperfectPrefix {
    use Vowel::*;
    match binyan {
        // yiqṭōl — i-i-ō, with v1 = silent sheva (closes syllable yi-q).
        Binyan::Qal => ImperfectPrefix {
            vowel: Hiriq,
            c1_dagesh: false,
            v1_long: Sheva,
            v2_long: Holam,
            v2_vocalic: Sheva,
            mater_after_c2_long: None,
        },
        // yiqqāṭēl — assimilated n- shows as dagesh on C1. v1 = qamats.
        Binyan::Niphal => ImperfectPrefix {
            vowel: Hiriq,
            c1_dagesh: true,
            v1_long: Qamats,
            v2_long: Tsere,
            v2_vocalic: Sheva,
            mater_after_c2_long: None,
        },
        // yəqaṭṭēl
        Binyan::Piel => ImperfectPrefix {
            vowel: Sheva,
            c1_dagesh: false,
            v1_long: Patah,
            v2_long: Tsere,
            v2_vocalic: Sheva,
            mater_after_c2_long: None,
        },
        // yəquṭṭal
        Binyan::Pual => ImperfectPrefix {
            vowel: Sheva,
            c1_dagesh: false,
            v1_long: Qubuts,
            v2_long: Patah,
            v2_vocalic: Sheva,
            mater_after_c2_long: None,
        },
        // yitqaṭṭēl
        Binyan::Hithpael => ImperfectPrefix {
            vowel: Hiriq,
            c1_dagesh: false,
            v1_long: Patah,
            v2_long: Tsere,
            v2_vocalic: Sheva,
            mater_after_c2_long: None,
        },
        // yaqṭîl
        Binyan::Hiphil => ImperfectPrefix {
            vowel: Patah,
            c1_dagesh: false,
            v1_long: Sheva,
            v2_long: Hiriq,
            v2_vocalic: Hiriq,
            mater_after_c2_long: Some(letter::YOD),
        },
        // yoqṭal
        Binyan::Hophal => ImperfectPrefix {
            vowel: QamatsQatan,
            c1_dagesh: false,
            v1_long: Sheva,
            v2_long: Patah,
            v2_vocalic: Sheva,
            mater_after_c2_long: None,
        },
    }
}

fn prefix_letter(pgn: Pgn) -> char {
    match (pgn.person, pgn.gender, pgn.number) {
        (Some(Person::First), _, Some(Number::Singular)) => letter::ALEF,
        (Some(Person::First), _, Some(Number::Plural)) => letter::NUN,
        (Some(Person::Second), _, _) => letter::TAV,
        (Some(Person::Third), Some(Gender::Feminine), Some(Number::Singular)) => letter::TAV,
        (Some(Person::Third), Some(Gender::Feminine), Some(Number::Plural)) => letter::TAV,
        (Some(Person::Third), _, _) => letter::YOD,
        _ => letter::YOD,
    }
}

/// Qal roots whose imperfect theme vowel is /a/ (patah) rather than the default
/// /o/, without a guttural to condition it phonologically — a lexical property.
/// שכב → yiškab. (Guttural-conditioned a-theme is handled by [`apply_guttural`].)
const QAL_A_THEME: [[char; 3]; 2] = [
    [letter::SHIN, letter::KAF, letter::BET],
    [letter::HET, letter::ZAYIN, letter::QOF],
];

fn build_imperfect(
    root: &Root,
    binyan: Binyan,
    pgn: Pgn,
    jussive: bool,
    force_a_theme: bool,
) -> Vec<Cons> {
    use Vowel::*;
    let mut prefix = imperfect_prefix(binyan);
    // Qal a-theme: a quiescent middle aleph (yišʔal) or a lexically a-class root
    // (yiškab) takes patah where the default paradigm would place holam.
    // `force_a_theme` additionally requests it for any Qal root, so the paradigm
    // can emit the stative yiqtal (יִגְדַּל) as an alternant of the default yiqtōl
    // — statives are a lexical class we don't otherwise mark.
    if binyan == Binyan::Qal
        && prefix.v2_long == Holam
        && (root.ayin() == letter::ALEF
            || force_a_theme
            || QAL_A_THEME.contains(&[root.pe(), root.ayin(), root.lamed()]))
    {
        prefix.v2_long = Patah;
    }
    let suffix = imperfect_suffix_kind(pgn);

    // Hiphil jussive: the î theme shortens to tsere in the zero-suffix
    // persons — yaggēd (יַגֵּד), yôsēp̄ — the base for the short wayyiqtol
    // (whose nesiga lowers it further to segol: וַיַּגֶּד, וַיֹּסֶף). The
    // vocalic-suffix persons keep the long î (תַּגִּידִי).
    if jussive && binyan == Binyan::Hiphil && suffix == Suffix::Zero {
        prefix.v2_long = Tsere;
        prefix.mater_after_c2_long = None;
    }

    let mut prefix_vowel = prefix.vowel;
    // 1cs in Qal: ʔeqṭōl — prefix vowel is segol, not hiriq (alef can't take
    // hiriq + closed syllable in Qal). Apply for binyanim that use hiriq.
    let is_1cs = pgn.person == Some(Person::First) && pgn.number == Some(Number::Singular);
    if is_1cs && let Hiriq = prefix.vowel {
        // The Niphal closes its prefix syllable with a dagesh forte in C1
        // (ʾeqqāṭēl אֶקָּטֵל → segol). But when C1 rejects the dagesh (a
        // guttural or resh) the doubling compensates by lengthening the prefix
        // — and that tsere reaches the 1cs aleph too (ʾērāʾeh אֵרָאֶה, matching
        // 3ms yērāʾeh). Leave the hiriq so the downstream guttural pass
        // lengthens it uniformly instead of pre-lowering to segol.
        if !(binyan == Binyan::Niphal && hebrew::rejects_dagesh(root.pe())) {
            prefix_vowel = Segol;
        }
    }

    // Qal a-theme statives with a guttural C1 (ח/ע) take a segol preformative
    // and hataf-segol (added in apply_pe_guttural): yeḥĕzaq (יֶחֱזַק).
    if binyan == Binyan::Qal
        && prefix.v2_long == Patah
        && hebrew::is_guttural(root.pe())
        && let Hiriq = prefix.vowel
    {
        prefix_vowel = Segol;
    }

    let mut out: Vec<Cons> = Vec::new();

    // ---- Prefix letter ----
    out.push(Cons::new(prefix_letter(pgn)).with_vowel(prefix_vowel));

    // ---- Hithpael t-infix ----
    if binyan == Binyan::Hithpael {
        out.push(Cons::new(letter::TAV).with_vowel(Sheva));
    }

    // ---- C1 ----
    let mut c1 = rad(root.pe(), 1).with_vowel(prefix.v1_long);
    if prefix.c1_dagesh {
        c1 = c1.with_dagesh();
    }
    out.push(c1);

    // ---- C2 ----
    let v2 = match suffix {
        Suffix::Vocalic => prefix.v2_vocalic,
        _ => prefix.v2_long,
    };
    let mut c2 = rad(root.ayin(), 2).with_vowel(v2);
    if matches!(binyan, Binyan::Piel | Binyan::Pual | Binyan::Hithpael) {
        c2 = c2.with_dagesh();
    }
    // Dagesh lene: when C1 closes the prefix syllable on a silent sheva
    // (yi-ḵ-, ya-q-), a begedkefet C2 begins the next syllable and takes a
    // dagesh lene — yiḵtōḇ → יִכְתֹּב, yaškēm → … . This fires for the binyanim
    // whose C1 carries a silent sheva after a real prefix vowel (Qal, Hiphil,
    // Hophal); Piel/Pual/Hithpael double C2 with a forte instead, and Niphal
    // closes C1 with its own forte.
    // A guttural C1 can't carry a silent sheva — it takes a hataf and opens its
    // own syllable (ya-ʕă-ḇ-), so a begedkefet C2 stays spirantised (יַעַבְרוּ).
    let c1_silent_sheva = prefix.v1_long == Vowel::Sheva
        && prefix_vowel != Vowel::Sheva
        && !hebrew::is_guttural(root.pe());
    if c1_silent_sheva && !prefix.c1_dagesh && hebrew::is_begedkefet(root.ayin()) {
        c2 = c2.with_dagesh();
    }
    out.push(c2);

    // ---- Optional mater after C2 (Hiphil) ----
    if matches!(suffix, Suffix::Zero | Suffix::Vocalic)
        && let Some(m) = prefix.mater_after_c2_long
    {
        out.push(Cons::new(m));
    }

    // ---- C3 ----
    // PGN-specific suffix logic below sets the C3 vowel where Hebrew
    // requires one. Default per suffix kind: Consonantal -nâ → sheva on
    // C3 (e.g. tiqṭōlnâ → tiqṭōlənâ); other suffixes leave C3 bare.
    let mut c3 = rad(root.lamed(), 3);
    if suffix == Suffix::Consonantal {
        c3.vowel = Some(Sheva);
    }
    out.push(c3);

    // ---- Suffix ----
    match (pgn.person, pgn.gender, pgn.number) {
        (Some(Person::Second), Some(Gender::Feminine), Some(Number::Singular)) => {
            // -î: hiriq on C3, then yod as mater.
            out.last_mut().unwrap().vowel = Some(Hiriq);
            out.push(Cons::new(letter::YOD));
        }
        (Some(Person::Third), Some(Gender::Masculine), Some(Number::Plural))
        | (Some(Person::Second), Some(Gender::Masculine), Some(Number::Plural)) => {
            // -û: shureq (vav with dagesh) attaches to C3 which stays bare.
            out.push(Cons::new(letter::VAV).with_dagesh());
        }
        (Some(Person::Third), Some(Gender::Feminine), Some(Number::Plural))
        | (Some(Person::Second), Some(Gender::Feminine), Some(Number::Plural)) => {
            // -nâ: nun with qamats, then he.
            out.push(Cons::new(letter::NUN).with_vowel(Qamats));
            out.push(Cons::new(letter::HE));
        }
        _ => {}
    }

    out
}

// ----- Cohortative -----------------------------------------------------------

/// Paragogic-he cohortative twins built directly from the zero-suffix
/// imperfect surface, for the weak stems whose structure the index-based
/// [`build_cohortative`] mangles (hollow/I-yod Hiphil ʾāšîḇâ וְאָשִׁיבָה,
/// ʾaggîḏâ אַגִּידָה, ʾôḏîʿâ אוֹדִיעָה; Niphal ʾimmālᵊṭâ וָאִמָּלְטָה). The -â
/// joins the final consonant; the theme vowel before it is either kept (long
/// maters and pausal-style ʾeʿlōzâ אֶעְלֹזָה, wᵉnēlēḵâ וְנֵלֵכָה), reduced to
/// sheva (the standard ʾēlᵊḵâ אֵלְכָה), or to hataf-patah (ʾēlăḵâ אֵלֲכָה,
/// wāʾešqălâ וָאֶשְׁקֲלָה). Returns every grade; exact match keeps the attested
/// one. Skips III-he/aleph-final hosts (their own builders cover them).
fn cohortative_paragogic_variants(impf_text: &str) -> Vec<String> {
    use Vowel::*;
    let seq = hebrew::parse_pointed(impf_text);
    let n = seq.len();
    if n < 3 {
        return Vec::new();
    }
    let last = seq[n - 1];
    // The host must end in a true final consonant — vowelless, the silent
    // sheva `render` writes under a final kaf (אֵלֵךְ), a furtive patah on a
    // final guttural (ʾiwwāšēaʿ → ʾiwwāšēʿâ וְאִוָּשֵׁעָה), or a quiescent final
    // aleph (ʾôṣîʾ → ʾôṣîʾâ אוֹצִיאָה). A final he means a III-he stem — skip.
    if last.letter == letter::HE
        || !(last.vowel.is_none()
            || last.vowel == Some(Sheva)
            || (hebrew::is_guttural(last.letter) && last.vowel == Some(Patah)))
    {
        return Vec::new();
    }
    let mut out = Vec::new();
    // Grade 1: theme kept.
    let mut kept = seq.clone();
    kept[n - 1].vowel = Some(Qamats);
    kept.push(Cons::new(letter::HE));
    out.push(hebrew::render(&kept));
    // A patah theme lengthens to qamats in the open pretonic syllable the
    // suffix creates: nēḏaʿ → wᵉnēḏāʿâ וְנֵדָעָה.
    if seq[n - 2].vowel == Some(Patah) {
        let mut s = kept.clone();
        s[n - 2].vowel = Some(Qamats);
        out.push(hebrew::render(&s));
    }
    // Reduced grades: a reducible short theme on the consonant before C3
    // drops to sheva / hataf-patah. (A mater or vowelless slot there means a
    // long theme — already covered by grade 1.)
    if matches!(
        seq[n - 2].vowel,
        Some(Tsere | Holam | Patah | Segol | Hiriq)
    ) {
        for v in [Sheva, HatafPatah] {
            let mut s = kept.clone();
            s[n - 2].vowel = Some(v);
            out.push(hebrew::render(&s));
        }
    }
    out
}

fn build_cohortative(root: &Root, binyan: Binyan, pgn: Pgn) -> Vec<Cons> {
    // Cohortative = Imperfect 1cs/1cp with -â suffix. The stem vowels behave
    // as if a vocalic suffix is attached (v2 reduces).
    use Vowel::*;
    let mut seq = build_imperfect(
        root,
        binyan,
        Pgn::new(
            pgn.person.unwrap_or(Person::First),
            Gender::Common,
            pgn.number.unwrap_or(Number::Singular),
        ),
        false,
        false,
    );
    // The base imperfect we just built had Suffix::Zero. To turn it into a
    // vocalic-suffix stem we need to: (1) change v2 to its vocalic-grade
    // counterpart; (2) put sheva on C3; (3) append -â (he with qamats on
    // previous C3).
    let prefix = imperfect_prefix(binyan);
    // Find C2 (index = prefix consonants + C1).
    let c1_idx = match binyan {
        Binyan::Hithpael => 2, // y(prefix), t(infix), C1
        _ => 1,                // y(prefix), C1
    };
    let c2_idx = c1_idx + 1;
    if let Some(c2) = seq.get_mut(c2_idx) {
        c2.vowel = Some(prefix.v2_vocalic);
    }
    // C3 is the next slot; set vowel sheva and append -â.
    let c3_idx = c2_idx + 1;
    if let Some(c3) = seq.get_mut(c3_idx) {
        c3.vowel = Some(Sheva);
    }
    // -â suffix: qamats on C3, then a bare he.
    if let Some(c3) = seq.get_mut(c3_idx) {
        c3.vowel = Some(Qamats);
    }
    seq.push(Cons::new(letter::HE));
    seq
}

// ----- Imperative ------------------------------------------------------------

fn build_imperative(root: &Root, binyan: Binyan, pgn: Pgn, force_a_theme: bool) -> Vec<Cons> {
    // Imperative = Imperfect 2nd-person with the prefix stripped, and v1
    // restored to a stable open-syllable vowel. Strong Qal: qəṭōl, qiṭlî,
    // qiṭlû, qəṭōlnâ.
    use Vowel::*;
    let mut prefix = imperfect_prefix(binyan);
    // Qal a-theme: the imperative shares the imperfect's theme vowel, so the
    // stative/a-class verbs take patah where the default has holam — šəmaʿ
    // (שְׁמַע), ḥăzaq (חֲזַק), šəlaḥ (שְׁלַח). Mirrors build_imperfect's rule.
    if binyan == Binyan::Qal
        && prefix.v2_long == Holam
        && (root.ayin() == letter::ALEF
            || force_a_theme
            || QAL_A_THEME.contains(&[root.pe(), root.ayin(), root.lamed()]))
    {
        prefix.v2_long = Patah;
    }

    let mut out: Vec<Cons> = Vec::new();

    // Niphal imperative gets a hi- prefix (hiqqāṭēl): "be killed".
    // Hithpael imperative gets hit-.
    // Hiphil imperative: haqṭēl (ms) — short a, no yod.
    // Hophal & Pual lack imperative (passive binyanim).
    match binyan {
        Binyan::Niphal => {
            out.push(Cons::new(letter::HE).with_vowel(Hiriq));
            // C1 takes dagesh
        }
        Binyan::Hithpael => {
            out.push(Cons::new(letter::HE).with_vowel(Hiriq));
            out.push(Cons::new(letter::TAV).with_vowel(Sheva));
        }
        Binyan::Hiphil => {
            out.push(Cons::new(letter::HE).with_vowel(Patah));
        }
        _ => {}
    }

    // C1
    let v1 = match binyan {
        Binyan::Qal => Sheva,      // qəṭol
        Binyan::Niphal => Qamats,  // hiqqāṭēl — doubled C1 carries the qamats
        Binyan::Piel => Patah,     // qaṭṭēl
        Binyan::Hithpael => Patah, // hitqaṭṭēl
        Binyan::Hiphil => Sheva,   // haqṭēl
        _ => Sheva,
    };
    let mut c1 = rad(root.pe(), 1).with_vowel(v1);
    if binyan == Binyan::Niphal {
        c1 = c1.with_dagesh();
    }
    out.push(c1);

    // C2 — depends on PGN suffix
    let suffix = match (pgn.gender, pgn.number) {
        (Some(Gender::Feminine), Some(Number::Singular)) => Suffix::Vocalic, // qiṭlî
        (Some(Gender::Masculine), Some(Number::Plural)) => Suffix::Vocalic,  // qiṭlû
        (Some(Gender::Feminine), Some(Number::Plural)) => Suffix::Consonantal, // qəṭōlnâ
        _ => Suffix::Zero,                                                   // qəṭōl
    };

    // For Qal 2fs/2mp, v1 actually shifts to hiriq (qiṭlî/qiṭlû). Encode
    // that special case.
    if binyan == Binyan::Qal && suffix == Suffix::Vocalic {
        out.last_mut().unwrap().vowel = Some(Hiriq);
    }

    let v2 = match suffix {
        Suffix::Vocalic => prefix.v2_vocalic,
        _ => prefix.v2_long,
    };
    let mut c2 = rad(root.ayin(), 2).with_vowel(v2);
    if matches!(binyan, Binyan::Piel | Binyan::Pual | Binyan::Hithpael) {
        c2 = c2.with_dagesh();
    }
    // Dagesh lene on a begedkefet C2: in the Hiphil imperative the C1 sheva is
    // silent (it closes the haC- prefix syllable: haš-kēm), so a begedkefet C2
    // opens the next syllable with a dagesh lene — haškēm הַשְׁכֵּם. A guttural
    // C1 can't carry the silent sheva, so it doesn't apply there. (Qal's C1
    // sheva is vocal — šᵊmōr — and Niphal/Hithpael double or close differently.)
    if binyan == Binyan::Hiphil
        && hebrew::is_begedkefet(root.ayin())
        && !hebrew::is_guttural(root.pe())
    {
        c2 = c2.with_dagesh();
    }
    out.push(c2);

    // ---- Optional mater after C2 (Hiphil) ----
    // Hiphil imperative has a yod mater for vocalic suffixes (haqṭîlû,
    // haqṭîlî), but not for the zero-suffix ms form (haqṭēl).
    if binyan == Binyan::Hiphil
        && matches!(suffix, Suffix::Vocalic)
        && let Some(m) = prefix.mater_after_c2_long
    {
        out.push(Cons::new(m));
    }

    // C3
    let mut c3 = rad(root.lamed(), 3);
    if let Suffix::Consonantal = suffix {
        // qəṭōlnâ: lamed-sheva, nun-qamats, he. The sheva on lamed is
        // vocal here.
        c3.vowel = Some(Sheva);
    }
    // Hiphil imperative ms has tsere on C2 (haqṭēl), no v3.
    if binyan == Binyan::Hiphil
        && matches!(suffix, Suffix::Zero)
        && let Some(i) = radical_idx(&out, 2)
    {
        out[i].vowel = Some(Tsere);
    }
    out.push(c3);

    // Suffix
    match (pgn.gender, pgn.number) {
        (Some(Gender::Feminine), Some(Number::Singular)) => {
            // -î: hiriq on C3, then yod.
            out.last_mut().unwrap().vowel = Some(Hiriq);
            out.push(Cons::new(letter::YOD));
        }
        (Some(Gender::Masculine), Some(Number::Plural)) => {
            // -û: shureq attaches to bare C3.
            out.push(Cons::new(letter::VAV).with_dagesh());
        }
        (Some(Gender::Feminine), Some(Number::Plural)) => {
            out.push(Cons::new(letter::NUN).with_vowel(Qamats));
            out.push(Cons::new(letter::HE));
        }
        _ => {}
    }

    out
}

// ----- Infinitives -----------------------------------------------------------

fn build_inf_construct(root: &Root, binyan: Binyan) -> Vec<Cons> {
    // Inf. Construct usually matches the 2ms Imperative shape in form,
    // except Hiphil which is haqṭîl with mater yod.
    use Vowel::*;
    match binyan {
        Binyan::Qal => vec![
            rad(root.pe(), 1).with_vowel(Sheva),
            rad(root.ayin(), 2).with_vowel(Holam),
            rad(root.lamed(), 3),
        ],
        Binyan::Niphal => vec![
            Cons::new(letter::HE).with_vowel(Hiriq),
            rad(root.pe(), 1).with_dagesh().with_vowel(Qamats),
            rad(root.ayin(), 2).with_vowel(Tsere),
            rad(root.lamed(), 3),
        ],
        Binyan::Piel => vec![
            rad(root.pe(), 1).with_vowel(Patah),
            rad(root.ayin(), 2).with_dagesh().with_vowel(Tsere),
            rad(root.lamed(), 3),
        ],
        Binyan::Pual => vec![
            // Rare; use the same shape as Pual perfect 3ms.
            rad(root.pe(), 1).with_vowel(Qubuts),
            rad(root.ayin(), 2).with_dagesh().with_vowel(Patah),
            rad(root.lamed(), 3),
        ],
        Binyan::Hithpael => vec![
            Cons::new(letter::HE).with_vowel(Hiriq),
            Cons::new(letter::TAV).with_vowel(Sheva),
            rad(root.pe(), 1).with_vowel(Patah),
            rad(root.ayin(), 2).with_dagesh().with_vowel(Tsere),
            rad(root.lamed(), 3),
        ],
        Binyan::Hiphil => vec![
            Cons::new(letter::HE).with_vowel(Patah),
            rad(root.pe(), 1).with_vowel(Sheva),
            rad(root.ayin(), 2).with_vowel(Hiriq),
            Cons::new(letter::YOD),
            rad(root.lamed(), 3),
        ],
        Binyan::Hophal => vec![
            Cons::new(letter::HE).with_vowel(QamatsQatan),
            rad(root.pe(), 1).with_vowel(Sheva),
            rad(root.ayin(), 2).with_vowel(Patah),
            rad(root.lamed(), 3),
        ],
    }
}

fn build_inf_absolute(root: &Root, binyan: Binyan) -> Vec<Cons> {
    // Strong Qal: qāṭôl. Niphal: niqṭōl. Piel: qaṭṭōl. Pual: quṭṭōl.
    // Hithpael: hitqaṭṭōl. Hiphil: haqṭēl. Hophal: hoqṭēl.
    use Vowel::*;
    match binyan {
        Binyan::Qal => vec![
            rad(root.pe(), 1).with_vowel(Qamats),
            rad(root.ayin(), 2).with_vowel(Holam),
            rad(root.lamed(), 3),
        ],
        Binyan::Niphal => vec![
            Cons::new(letter::NUN).with_vowel(Hiriq),
            rad(root.pe(), 1).with_vowel(Sheva),
            rad(root.ayin(), 2).with_vowel(Holam),
            rad(root.lamed(), 3),
        ],
        Binyan::Piel => vec![
            rad(root.pe(), 1).with_vowel(Patah),
            rad(root.ayin(), 2).with_dagesh().with_vowel(Holam),
            rad(root.lamed(), 3),
        ],
        Binyan::Pual => vec![
            rad(root.pe(), 1).with_vowel(Qubuts),
            rad(root.ayin(), 2).with_dagesh().with_vowel(Holam),
            rad(root.lamed(), 3),
        ],
        Binyan::Hithpael => vec![
            Cons::new(letter::HE).with_vowel(Hiriq),
            Cons::new(letter::TAV).with_vowel(Sheva),
            rad(root.pe(), 1).with_vowel(Patah),
            rad(root.ayin(), 2).with_dagesh().with_vowel(Holam),
            rad(root.lamed(), 3),
        ],
        Binyan::Hiphil => vec![
            Cons::new(letter::HE).with_vowel(Patah),
            rad(root.pe(), 1).with_vowel(Sheva),
            inf_abs_c2_lene(root),
            rad(root.lamed(), 3),
        ],
        Binyan::Hophal => vec![
            Cons::new(letter::HE).with_vowel(QamatsQatan),
            rad(root.pe(), 1).with_vowel(Sheva),
            inf_abs_c2_lene(root),
            rad(root.lamed(), 3),
        ],
    }
}

/// Hiphil/Hophal inf-abs C2: C1's silent sheva closes the prefix syllable, so
/// a begedkefet C2 begins the next syllable with a dagesh lene — harbēh
/// (הַרְבֵּה), not הַרְבֵה.
fn inf_abs_c2_lene(root: &Root) -> Cons {
    let mut c2 = rad(root.ayin(), 2).with_vowel(Vowel::Tsere);
    if hebrew::is_begedkefet(root.ayin()) && !hebrew::is_guttural(root.pe()) {
        c2 = c2.with_dagesh();
    }
    c2
}

// ----- Participles -----------------------------------------------------------

fn build_participle(root: &Root, binyan: Binyan, pgn: Pgn, passive: bool) -> Vec<Cons> {
    use Vowel::*;

    // Construct the ms (base) shape per binyan.
    let mut base: Vec<Cons> = match (binyan, passive) {
        // Qal active: qôṭēl (קוֹטֵל). We use defective spelling: qof-holam,
        // tet-tsere, lamed.
        (Binyan::Qal, false) => vec![
            rad(root.pe(), 1).with_vowel(Holam),
            rad(root.ayin(), 2).with_vowel(Tsere),
            rad(root.lamed(), 3),
        ],
        // Qal passive: qāṭûl (קָטוּל). qamats, vav-shureq, lamed.
        (Binyan::Qal, true) => vec![
            rad(root.pe(), 1).with_vowel(Qamats),
            rad(root.ayin(), 2),
            Cons::new(letter::VAV).with_dagesh(),
            rad(root.lamed(), 3),
        ],
        // Niphal: niqṭāl (נִקְטָל).
        (Binyan::Niphal, _) => vec![
            Cons::new(letter::NUN).with_vowel(Hiriq),
            rad(root.pe(), 1).with_vowel(Sheva),
            rad(root.ayin(), 2).with_vowel(Qamats),
            rad(root.lamed(), 3),
        ],
        // Piel: məqaṭṭēl (מְקַטֵּל). All non-Qal participles take m- prefix.
        (Binyan::Piel, _) => vec![
            Cons::new(letter::MEM).with_vowel(Sheva),
            rad(root.pe(), 1).with_vowel(Patah),
            rad(root.ayin(), 2).with_dagesh().with_vowel(Tsere),
            rad(root.lamed(), 3),
        ],
        // Pual: məquṭṭāl (מְקֻטָּל).
        (Binyan::Pual, _) => vec![
            Cons::new(letter::MEM).with_vowel(Sheva),
            rad(root.pe(), 1).with_vowel(Qubuts),
            rad(root.ayin(), 2).with_dagesh().with_vowel(Qamats),
            rad(root.lamed(), 3),
        ],
        // Hithpael: mitqaṭṭēl (מִתְקַטֵּל).
        (Binyan::Hithpael, _) => vec![
            Cons::new(letter::MEM).with_vowel(Hiriq),
            Cons::new(letter::TAV).with_vowel(Sheva),
            rad(root.pe(), 1).with_vowel(Patah),
            rad(root.ayin(), 2).with_dagesh().with_vowel(Tsere),
            rad(root.lamed(), 3),
        ],
        // Hiphil: maqṭîl (מַקְטִיל).
        (Binyan::Hiphil, _) => vec![
            Cons::new(letter::MEM).with_vowel(Patah),
            rad(root.pe(), 1).with_vowel(Sheva),
            rad(root.ayin(), 2).with_vowel(Hiriq),
            Cons::new(letter::YOD),
            rad(root.lamed(), 3),
        ],
        // Hophal: moqṭāl (מָקְטָל).
        (Binyan::Hophal, _) => vec![
            Cons::new(letter::MEM).with_vowel(QamatsQatan),
            rad(root.pe(), 1).with_vowel(Sheva),
            rad(root.ayin(), 2).with_vowel(Qamats),
            rad(root.lamed(), 3),
        ],
    };

    // Dagesh lene on a begedkefet C2 in the prefixed binyanim: the prefix
    // syllable closes on C1's silent sheva (maq-, niq-, hoq-), so a begedkefet
    // C2 begins a new syllable and takes a dagesh lene (maqtîr → מַזְכִּיר,
    // niḵtāḇ → נִכְתָּב). The perfect and imperfect builders already apply this;
    // the participle builder didn't, leaving the C2 spirantised (מַזְכִיר).
    if matches!(binyan, Binyan::Niphal | Binyan::Hiphil | Binyan::Hophal)
        && hebrew::is_begedkefet(root.ayin())
        && base
            .iter()
            .find(|c| c.role == Role::Radical(1))
            .is_some_and(|c| c.vowel == Some(Sheva))
        && let Some(c2) = base.iter_mut().find(|c| c.role == Role::Radical(2))
    {
        c2.dagesh = true;
    }

    // Inflect for gender/number. The ms is `base` as-is.
    // fs uses -et / -â depending on binyan; we'll standardise on -ת ending
    // (segolate qoṭelet) for Qal active, -â otherwise.
    // mp adds -îm, fp adds -ôt.
    match (pgn.gender, pgn.number) {
        (Some(Gender::Masculine), Some(Number::Singular)) => base,
        (Some(Gender::Feminine), Some(Number::Singular)) => {
            // For Qal active, qôṭelet (segolate). Otherwise add qamats-he.
            if binyan == Binyan::Qal && !passive {
                // Replace the final lamed's vowel with segol, drop tsere on
                // ayin to segol, then add tav.
                if let Some(c) = base.iter_mut().rev().nth(1) {
                    c.vowel = Some(Segol);
                }
                base.push(Cons::new(letter::TAV));
                // Add segol under lamed:
                if let Some(last) = base.iter_mut().rev().nth(1) {
                    last.vowel = Some(Segol);
                }
                base
            } else {
                // Other binyanim + Qal passive: reduce the thematic vowel
                // (propretonic) and append -â.
                reduce_participle_thematic(&mut base, root);
                if let Some(last) = base.last_mut() {
                    last.vowel = Some(Qamats);
                }
                base.push(Cons::new(letter::HE));
                base
            }
        }
        (Some(Gender::Masculine), Some(Number::Plural)) => {
            // -îm: short thematic vowel reduces (Qal active tsere → sheva,
            // Piel/Hithpael tsere → sheva, Hiphil-active î stays); long
            // vowels (qamats in Niphal/Pual/Hophal) stay.
            reduce_participle_thematic(&mut base, root);
            if let Some(last) = base.last_mut() {
                last.vowel = Some(Hiriq);
            }
            base.push(Cons::new(letter::YOD));
            base.push(Cons::new(letter::MEM));
            base
        }
        (Some(Gender::Feminine), Some(Number::Plural)) => {
            // -ôt: same propretonic reduction story as mp.
            reduce_participle_thematic(&mut base, root);
            if let Some(last) = base.last_mut() {
                last.vowel = Some(Holam);
            }
            base.push(Cons::new(letter::TAV));
            base
        }
        _ => base,
    }
}

/// Convert a masculine-plural participle from absolute (-îm) to construct
/// (-ê). Absolute ends `[…, C3(Hiriq), YOD, MEM]`; construct is `[…, C3(Tsere),
/// YOD]` (יֹשְׁבִים → יֹשְׁבֵי). Returns `None` unless the trailing -îm pattern
/// matches exactly, so weak-verb endings reshaped by gizra are left alone.
fn participle_mp_construct(seq: &[Cons]) -> Option<Vec<Cons>> {
    let n = seq.len();
    if n < 3 {
        return None;
    }
    if seq[n - 1].letter != letter::MEM || seq[n - 2].letter != letter::YOD {
        return None;
    }
    let mut out = seq.to_vec();
    out.pop(); // drop the MEM
    out[n - 3].vowel = Some(Vowel::Tsere); // C3 Hiriq → Tsere
    Some(out)
}

/// Reduce a participle's thematic vowel before plural / -â suffixes.
/// Different participle shapes reduce at different positions:
/// Qal active qôṭēl → qôṭ-l-îm: short tsere on C2 reduces.
/// Qal passive qāṭûl → qəṭûl-îm: long qamats on C1 reduces (the C2 vowel
/// is the shureq vav, which stays).
/// Other binyanim with long C2 (Niphal/Pual/Hophal qamats, Hiphil î): no
/// reduction.
fn reduce_participle_thematic(base: &mut [Cons], _root: &Root) {
    // Try C2 first: if it carries a reducible short vowel (tsere/segol/patah),
    // that's our propretonic — reduce it to sheva. Otherwise fall back to
    // reducing the C1 qamats (Qal-passive shape, where C2 is silent before
    // the shureq).
    let c1 = base
        .iter()
        .position(|c| c.role == Role::Radical(1) && c.vowel == Some(Vowel::Qamats));
    let c2 = base.iter().position(|c| {
        c.role == Role::Radical(2)
            && matches!(
                c.vowel,
                Some(Vowel::Tsere) | Some(Vowel::Segol) | Some(Vowel::Patah)
            )
    });
    if let Some(i) = c2 {
        base[i].vowel = Some(Vowel::Sheva);
    } else if let Some(i) = c1 {
        base[i].vowel = Some(Vowel::Sheva);
    }
}

// ----------------------------------------------------------------------------
// Gizra (weak-verb) transformations
// ----------------------------------------------------------------------------

/// Apply the irregular-class adjustments. Returns the rewritten sequence and
/// `attested` = whether the gizra/form combination is one we explicitly
/// model. Strong roots always return `true`.
fn apply_gizra(
    seq: Vec<Cons>,
    root: &Root,
    binyan: Binyan,
    form: Form,
    pgn: Pgn,
) -> (Vec<Cons>, bool) {
    let mut seq = seq;
    let mut attested = root.is_strong();

    // I-Nun: nun assimilates as dagesh in C2 when no vowel intervenes.
    // Affects Qal Imperfect (yippōl), Imperative (pōl), Inf. Construct (pōl,
    // sometimes lit. něpōl), Niphal/Hiphil/Hophal where the nun prefix or
    // root-initial nun closes a syllable.
    // A hollow root with an initial nun (נוס, נוד, נוח) keeps the nun as a true
    // first radical (yānûs) — it never assimilates, so pe-nun must not fire.
    if root.has(Gizra::PeNun) && !root.has(Gizra::Hollow) {
        attested |= apply_pe_nun(&mut seq, root, binyan, form, pgn);
    }

    // I-Yod (originally I-Vav for most verbs).
    if root.has(Gizra::PeYod) {
        attested |= apply_pe_yod(&mut seq, root, binyan, form, pgn);
    }

    // Hollow (II-Vav/Yod): middle radical drops in many forms; characteristic
    // long vowel takes its place.
    if root.has(Gizra::Hollow) {
        attested |= apply_hollow(&mut seq, root, binyan, form, pgn);
    }

    // Geminate (II=III).
    if root.has(Gizra::Geminate) {
        attested |= apply_geminate(&mut seq, root, binyan, form, pgn);
    }

    // III-He: replace the final he with appropriate ending (-â / -ê / -ô
    // depending on form).
    if root.has(Gizra::LamedHe) {
        attested |= apply_lamed_he(&mut seq, root, binyan, form, pgn);
    }

    // III-Aleph: alef quiesces, lengthens preceding vowel.
    if root.has(Gizra::LamedAleph) {
        attested |= apply_lamed_aleph(&mut seq, root, binyan, form, pgn);
    }

    // Consonantal-he statives (גבה, נגה, תמה, כמה): the strong builder produced
    // the right skeleton (gāḇah, gāḇhû, the a-theme yiḡbah), but a word-final
    // consonantal he needs its mappiq to mark it as a true radical — gāḇah is
    // written גָּבַהּ, not the homographic III-He גָּבַה. Set the dagesh point on
    // any final vowelless he so canonical_key (which keeps the he's dagesh)
    // matches the attested mappiq spelling.
    if root.lamed() == letter::HE && crate::morphology::root::is_consonantal_he_root(root.letters)
    {
        attested = true;
        if let Some(last) = seq.last_mut()
            && last.letter == letter::HE
            && last.vowel.is_none()
        {
            last.dagesh = true;
        }
    }

    // I-Aleph: in Qal Imperfect, prefix vowel becomes holam (yōʔkal).
    if root.has(Gizra::PeAleph) {
        attested |= apply_pe_aleph(&mut seq, root, binyan, form, pgn);
    }

    // Hithpael: sibilant metathesis & emphatic assimilation.
    if binyan == Binyan::Hithpael {
        attested |= apply_hithpael_metathesis(&mut seq, root);
    }

    // III-Guttural: Qal Imperfect thematic vowel lowers to patah (yišlaḥ).
    if root.has(Gizra::LamedGuttural) {
        attested |= apply_lamed_guttural(&mut seq, root, binyan, form, pgn);
    }

    // I-Guttural (ח/ע): Qal Imperfect hiriq prefix lowers to patah (yaʕămōd).
    // Runs before apply_guttural so the latter can add the C1 hataf-patah.
    if root.has(Gizra::PeGuttural) {
        attested |= apply_pe_guttural(&mut seq, root, binyan, form, pgn);
    }

    // Guttural rules: forbid dagesh on guttural radicals, convert vocal
    // sheva under a guttural to a hataf vowel. Applied last so they catch
    // fixes introduced by other gizra rules. Always runs (not gated on the
    // root having a guttural radical) so prefix alefs in 1cs Imperfect get
    // their hataf-patah even for strong roots: אֲקַטֵּל, not אְקַטֵּל.
    let _ = apply_guttural(&mut seq, root);

    // Hithpael: the t-infix closes the prefix syllable on a silent sheva
    // (yiṯ-, hiṯ-, miṯ-), so a begedkefet C1 opens the next syllable with a
    // dagesh lene — yiṯpallēl → יִתְפַּלֵּל, hiṯdabbēr → הִתְדַּבֵּר, yiṯbārēḵ →
    // יִתְבָּרֵךְ. (Sibilant C1s metathesise the tav and never reach this branch;
    // begedkefet C1s never metathesise.)
    if binyan == Binyan::Hithpael
        && let Some(c1) = radical_idx(&seq, 1)
        && hebrew::is_begedkefet(seq[c1].letter)
        && !seq[c1].dagesh
    {
        seq[c1].dagesh = true;
    }

    // Niphal imperfect-family with a guttural/resh C1: the preformative hiriq
    // that would close on the doubled C1 (yikkātēḇ) lengthens to tsere when the
    // guttural/resh rejects that doubling — yēʿāśê (יֵעָשֶׂה), yēʾāḵēl (יֵאָכֵל),
    // yērāʔê (יֵרָאֶה). Unlike the Piel II-guttural, where the prefix vowel may
    // stay short via virtual doubling (biʕēr בִּעֵר), the Niphal preformative
    // lengthens regularly for all of א/ה/ח/ע/ר.
    if binyan == Binyan::Niphal
        && matches!(
            form,
            Form::Imperfect | Form::Jussive | Form::Wayyiqtol | Form::Cohortative
        )
        && hebrew::rejects_dagesh(root.pe())
        && let Some(c1) = radical_idx(&seq, 1)
        && c1 > 0
        && seq[c1 - 1].vowel == Some(Vowel::Hiriq)
    {
        seq[c1 - 1].vowel = Some(Vowel::Tsere);
    }

    // Furtive patah: a word-final ח/ע preceded by a long non-a vowel (tsere,
    // holam, hiriq, shureq) takes a glide patah under the guttural — šōmēaʿ
    // (שֹׁמֵעַ), šəmōaʿ (שְׁמֹעַ), yôḏēaʿ. The patah is unlike the bearing vowel,
    // so the imperfect's yišmaʕ (patah → an a-vowel) is correctly skipped.
    if matches!(root.lamed(), letter::HET | letter::AYIN) {
        attested |= apply_furtive_patah(&mut seq);
    }

    // היה / חיה: these two III-He + I-Guttural roots are lexically irregular in
    // the Qal Imperfect — unlike a regular I-guttural III-He verb (עלה→יַעֲלֶה),
    // the guttural closes the prefix syllable with a *silent* sheva and the
    // hiriq prefix is kept: yihyê (יִהְיֶה), yiḥyê (יִחְיֶה). apply_guttural has
    // just turned C1's silent sheva into a hataf, and (for חיה) apply_pe_guttural
    // lowered the prefix to patah; undo both. The 1cs alef-prefix segol (אֶהְיֶה)
    // is left as is — only a patah prefix (the pe-guttural lowering) is reverted.
    if root.ayin() == letter::YOD
        && root.lamed() == letter::HE
        && matches!(root.pe(), letter::HE | letter::HET)
        && matches!(
            (binyan, form),
            (
                Binyan::Qal,
                Form::Imperfect | Form::Jussive | Form::Cohortative
            )
        )
        && let Some(i) = radical_idx(&seq, 1)
    {
        seq[i].vowel = Some(Vowel::Sheva);
        if i > 0 && seq[i - 1].vowel == Some(Vowel::Patah) {
            seq[i - 1].vowel = Some(Vowel::Hiriq);
        }
    }

    // היה / חיה Qal Perfect heavy-suffix forms (2mp/2fp) take a hataf-segol
    // under the I-He guttural — hĕyîtem (הֱיִיתֶם), hĕyîten — where a regular
    // I-guttural verb would show the hataf-patah that apply_guttural just set
    // (ʕăśîtem). Lower it to hataf-segol for these two lexemes; the conjunctive
    // vav then surfaces as wihyîtem (וִהְיִיתֶם).
    if root.ayin() == letter::YOD
        && root.lamed() == letter::HE
        && matches!(root.pe(), letter::HE | letter::HET)
        && binyan == Binyan::Qal
        && form == Form::Perfect
        && let Some(i) = radical_idx(&seq, 1)
        && seq[i].vowel == Some(Vowel::HatafPatah)
    {
        seq[i].vowel = Some(Vowel::HatafSegol);
    }

    // הלך "to go" conjugates in the Qal like an original I-Vav (I-Yod) verb,
    // not like the I-guttural pattern its root letters suggest: the he drops
    // entirely and the prefix takes tsere (yēlēḵ, wayyēleḵ; imperative lēḵ).
    if is_halak(root) && binyan == Binyan::Qal {
        if form == Form::InfinitiveConstruct {
            // Segholate infinitive leḵeṯ (לֶכֶת), like ישב→שֶׁבֶת.
            apply_iyod_segholate_infinitive(&mut seq);
            attested = true;
        } else if matches!(
            form,
            Form::Imperfect | Form::Jussive | Form::Cohortative | Form::Imperative
        ) {
            if let Some(i) = radical_idx(&seq, 1) {
                if i > 0 {
                    seq[i - 1].vowel = Some(Vowel::Tsere);
                }
                seq.remove(i);
                attested = true;
            }
            if let Some(c2) = radical_idx(&seq, 2)
                && seq[c2].vowel == Some(Vowel::Holam)
            {
                seq[c2].vowel = Some(Vowel::Tsere);
            }
        }
    }

    // נתן "to give" is lexically irregular in the Qal: the Imperfect theme is
    // tsere (yittēn), not the I-nun default holam (yittōl), and in the Perfect
    // the final nun assimilates into a dental/nasal suffix consonant as a
    // dagesh forte (nātattî < nātan-tî, nātannû < nātan-nû).
    if is_natan(root) && binyan == Binyan::Qal {
        match form {
            Form::Imperfect | Form::Jussive | Form::Cohortative | Form::Imperative => {
                if let Some(c2) = radical_idx(&seq, 2)
                    && seq[c2].vowel == Some(Vowel::Holam)
                {
                    seq[c2].vowel = Some(Vowel::Tsere);
                }
            }
            Form::Perfect => {
                if let Some(c3) = radical_idx(&seq, 3)
                    && seq[c3].vowel == Some(Vowel::Sheva)
                    && c3 + 1 < seq.len()
                    && matches!(seq[c3 + 1].letter, letter::TAV | letter::NUN)
                {
                    seq.remove(c3);
                    seq[c3].dagesh = true;
                    attested = true;
                }
            }
            _ => {}
        }
    }

    // נגשׁ "to draw near": I-nun with an a-class patah theme in the Qal
    // imperfect (yiggaš), where the default I-nun theme is holam (yiggōš).
    if is_nagash(root)
        && binyan == Binyan::Qal
        && matches!(
            form,
            Form::Imperfect | Form::Jussive | Form::Cohortative | Form::Wayyiqtol
        )
        && let Some(c2) = radical_idx(&seq, 2)
        && seq[c2].vowel == Some(Vowel::Holam)
    {
        seq[c2].vowel = Some(Vowel::Patah);
    }

    // לקח "to take": the C1 lamed assimilates like a I-nun (yiqqaḥ, וַיִּקַּח) and
    // drops entirely in the imperative (qaḥ קַח) and the segholate infinitive
    // construct (qaḥaṯ קַחַת). The het is a guttural, so the theme is patah.
    if is_laqah(root) && binyan == Binyan::Qal {
        match form {
            Form::Imperfect | Form::Jussive | Form::Cohortative | Form::Wayyiqtol => {
                if let Some(idx) = radical_idx(&seq, 1)
                    && idx + 1 < seq.len()
                {
                    seq.remove(idx);
                    seq[idx].dagesh = true;
                    attested = true;
                }
            }
            Form::Imperative => {
                if let Some(idx) = radical_idx(&seq, 1) {
                    seq.remove(idx);
                }
                // Long grade (2ms/2fp) had holam under C2 — lower to patah, and
                // drop the furtive patah apply_guttural placed on the final het
                // under that (now-short) holam: qōḥ + furtive → qaḥ (קַח), not
                // קַחַ. Only the zero-suffix grade (qaḥ) closes on a bare het;
                // vocalic grades (qəḥî, qəḥû) keep their suffix.
                if let Some(i) = radical_idx(&seq, 2)
                    && seq[i].vowel == Some(Vowel::Holam)
                {
                    seq[i].vowel = Some(Vowel::Patah);
                    if let Some(c3) = radical_idx(&seq, 3) {
                        seq[c3].vowel = None;
                    }
                }
                attested = true;
            }
            Form::InfinitiveConstruct => {
                if let Some(idx) = radical_idx(&seq, 1) {
                    seq.remove(idx);
                }
                if let Some(i) = radical_idx(&seq, 2) {
                    seq[i].vowel = Some(Vowel::Patah);
                }
                if let Some(i) = radical_idx(&seq, 3) {
                    seq[i].vowel = Some(Vowel::Patah);
                }
                seq.push(Cons::new(letter::TAV));
                attested = true;
            }
            _ => {}
        }
    }

    // ראה "to see" is doubly weak (II-aleph + III-he). Its apocopated jussive
    // 3ms — the base for the very common wayyiqtol וַיַּרְא — is irregular: the he
    // drops, the aleph quiesces vowel-less, C1 resh closes with a silent sheva,
    // and the prefix takes patah (yar', not the regular yir'eh-derived yiren).
    if is_raah(root)
        && binyan == Binyan::Qal
        && form == Form::Jussive
        && pgn == Pgn::new(Person::Third, Gender::Masculine, Number::Singular)
    {
        if let Some(c3) = radical_idx(&seq, 3) {
            seq.remove(c3);
        }
        if let Some(c1) = radical_idx(&seq, 1) {
            if c1 > 0 {
                seq[c1 - 1].vowel = Some(Vowel::Patah);
            }
            seq[c1].vowel = Some(Vowel::Sheva);
        }
        if let Some(c2) = radical_idx(&seq, 2) {
            seq[c2].vowel = None;
        }
        attested = true;
    }

    // ראה apocopated jussive, non-3ms: unlike the 3ms yar' (patah prefix, silent
    // C1), the other persons take a tsere prefix over a segol C1 — tēreʾ
    // (וַתֵּרֶא, 3fs/2ms). apply_lamed_he has apocopated to a hiriq-prefixed
    // tirreʾ; raise the prefix hiriq to tsere.
    if is_raah(root)
        && binyan == Binyan::Qal
        && form == Form::Jussive
        && pgn != Pgn::new(Person::Third, Gender::Masculine, Number::Singular)
        && let Some(c1) = radical_idx(&seq, 1)
        && c1 > 0
        && seq[c1 - 1].vowel == Some(Vowel::Hiriq)
    {
        seq[c1 - 1].vowel = Some(Vowel::Tsere);
        attested = true;
    }

    // חיה "to live" and היה "to be" — their apocopated jussives (the base for
    // the very common wayyiqtols וַיְחִי and וַיְהִי) are irregular: the he
    // drops, the C2 yod becomes a hiriq-mater, C1 takes hiriq, and the prefix
    // reduces to a vocal sheva — yəḥî (יְחִי), yəhî (יְהִי), təhî (תְּהִי). The
    // 1cs aleph preformative takes hataf-segol instead of the bare sheva it
    // cannot carry — ʾĕhî (אֱהִי, wayyiqtol וָאֱהִי). apply_lamed_he has
    // already apocopated to yiḥy/yihy; rewrite the prefix and C1 vowels. The
    // 2fs keeps its vocalic -î suffix and is excluded (as in the apocope arm).
    if (is_chayah(root) || is_hayah(root))
        && binyan == Binyan::Qal
        && form == Form::Jussive
        && !(pgn.person == Some(Person::Second) && pgn.gender == Some(Gender::Feminine))
    {
        if let Some(c1) = radical_idx(&seq, 1) {
            if c1 > 0 {
                seq[c1 - 1].vowel = Some(if seq[c1 - 1].letter == letter::ALEF {
                    Vowel::HatafSegol
                } else {
                    Vowel::Sheva
                });
            }
            seq[c1].vowel = Some(Vowel::Hiriq);
        }
        attested = true;
    }

    // חרה "to be(come) hot/angry" — its apocopated jussive 3ms keeps the hiriq
    // prefix (yiḥar → וַיִּחַר), unlike the otherwise-parallel עלה which lowers it
    // to patah (וַיַּעַל). apply_pe_guttural has lowered the prefix to patah for
    // the guttural ḥet; restore the hiriq for this lexeme.
    if is_charah(root)
        && binyan == Binyan::Qal
        && form == Form::Jussive
        && pgn == Pgn::new(Person::Third, Gender::Masculine, Number::Singular)
        && let Some(c1) = radical_idx(&seq, 1)
        && c1 > 0
        && seq[c1 - 1].vowel == Some(Vowel::Patah)
    {
        seq[c1 - 1].vowel = Some(Vowel::Hiriq);
        attested = true;
    }

    // ירא imperfect: apply_pe_yod has elided the yod (leaving preformative +
    // resh + aleph, e.g. yēreʾ יֵרֵא), but this verb keeps the yod as a
    // hiriq-mater under a hiriq preformative: yîrāʾ (יִירָא), tîrāʾ (תִּירָא). The
    // theme vowel under the resh stays whatever the suffix grade set — qamats in
    // the zero-suffix forms (tsere → qamats), sheva in the vocalic plurals.
    if is_yare(root)
        && binyan == Binyan::Qal
        && matches!(
            form,
            Form::Imperfect | Form::Jussive | Form::Wayyiqtol | Form::Cohortative
        )
        && seq.len() >= 3
        && seq[1].letter == root.ayin()
        && seq[2].letter == letter::ALEF
    {
        if seq[1].vowel == Some(Vowel::Tsere) {
            seq[1].vowel = Some(Vowel::Qamats);
        }
        seq[0].vowel = Some(Vowel::Hiriq);
        seq.insert(1, Cons::new(letter::YOD));
        attested = true;
    }

    // יכל "to be able" — its Qal imperfect is unique: a û theme sits on the
    // preformative with the radical yod lost entirely — yûḵal (יוּכַל), tûḵal
    // (תּוּכַל), ʾûḵal (אוּכַל), plural yûḵəlû (יוּכְלוּ). Rebuild from the
    // preformative letter + shureq + C2/C3, keeping the strong build's
    // person/gender suffix tail.
    if is_yakol(root)
        && binyan == Binyan::Qal
        && matches!(form, Form::Imperfect | Form::Jussive | Form::Wayyiqtol)
    {
        let pl = seq.first().map(|c| c.letter).unwrap_or(letter::YOD);
        if let Some(c3) = radical_idx(&seq, 3) {
            let tail: Vec<Cons> = seq[c3..].to_vec();
            let mut new = vec![Cons::new(pl), Cons::new(letter::VAV).with_dagesh()];
            new.push(rad(root.ayin(), 2).with_vowel(
                if imperfect_suffix_kind(pgn) == Suffix::Vocalic {
                    Vowel::Sheva
                } else {
                    Vowel::Patah
                },
            ));
            new.extend(tail);
            seq = new;
            attested = true;
        }
    }

    // בושׁ "to be ashamed" — corrected as the ô-class hollow in apply_hollow,
    // but its preformative is tsere not qamats (yēḇôš יֵבוֹשׁ, yēḇōšû יֵבֹשׁוּ),
    // and its perfect carries holam on C1 rather than qamats (bōšû בֹשׁוּ).
    if is_bosh(root) && binyan == Binyan::Qal {
        match form {
            Form::Imperfect | Form::Jussive | Form::Wayyiqtol | Form::Cohortative => {
                if let Some(c1) = radical_idx(&seq, 1)
                    && c1 > 0
                    && matches!(seq[c1 - 1].vowel, Some(Vowel::Qamats) | Some(Vowel::Hiriq))
                {
                    seq[c1 - 1].vowel = Some(Vowel::Tsere);
                    attested = true;
                }
            }
            Form::Perfect => {
                if let Some(c1) = radical_idx(&seq, 1)
                    && seq[c1].vowel == Some(Vowel::Qamats)
                {
                    seq[c1].vowel = Some(Vowel::Holam);
                    attested = true;
                }
            }
            _ => {}
        }
    }

    // מות "to die" — a û-class hollow whose Qal perfect is stative: the theme
    // vowel is tsere, not the regular qamats (mēṯ מֵת, mēṯâ מֵתָה, mēṯû מֵתוּ).
    // Before a consonantal afformative the theme is patah (mattî מַתִּי), which
    // apply_hollow has already produced, so only the surviving qamats grade
    // (zero/vocalic) is corrected here. The imperfect (yāmûṯ) stays regular.
    if is_mut(root)
        && binyan == Binyan::Qal
        && form == Form::Perfect
        && let Some(c1) = radical_idx(&seq, 1)
        && seq[c1].vowel == Some(Vowel::Qamats)
    {
        seq[c1].vowel = Some(Vowel::Tsere);
        attested = true;
    }

    // הרה apocopated jussive/wayyiqtol — lower the III-He hiriq preformative to
    // the a-class patah this I-guttural lexeme takes (tihar → tahar תַּהַר).
    if is_harah(root)
        && binyan == Binyan::Qal
        && form == Form::Jussive
        && let Some(c1) = radical_idx(&seq, 1)
        && c1 > 0
        && seq[c1 - 1].vowel == Some(Vowel::Hiriq)
    {
        seq[c1 - 1].vowel = Some(Vowel::Patah);
        attested = true;
    }

    // בכה "to weep" — wayyiqtol 3ms וַיֵּבְךְּ: prefix takes tsere, and the
    // apocopated stem ends in a dual-sheva cluster (bᵊḵk).
    if is_bakah(root) && binyan == Binyan::Qal && form == Form::Jussive && pgn == THREE_MS {
        if let Some(c1) = radical_idx(&seq, 1) {
            if c1 > 0 && seq[c1 - 1].vowel == Some(Vowel::Hiriq) {
                seq[c1 - 1].vowel = Some(Vowel::Tsere);
            }
            seq[c1].vowel = Some(Vowel::Sheva);
        }
        if let Some(c2) = radical_idx(&seq, 2) {
            seq[c2].vowel = Some(Vowel::Sheva);
        }
        attested = true;
    }

    // שחה "to bow down" — Hithpael (hishtaḥăvâ): behaves as if root was original
    // שחו (Lamed-Vav), keeping the vav radical before suffixes.
    if is_shachah(root) && binyan == Binyan::Hithpael {
        if matches!(form, Form::Imperfect | Form::Wayyiqtol) && pgn.number == Some(Number::Plural) {
            // יִשְׁתַּחֲווּ: apply_lamed_he has elided the he; insert the vav
            // radical and give C2 (ḥet) its vocal hataf-patah.
            if let Some(c2) = radical_idx(&seq, 2) {
                seq[c2].vowel = Some(Vowel::HatafPatah);
                seq.insert(c2 + 1, Cons::new(letter::VAV));
                attested = true;
            }
        } else if form == Form::InfinitiveConstruct {
            // לְהִשְׁתַּחֲוֹת: the he becomes a vav-holam mater before the tav,
            // and C2 (ḥet) takes its vocal hataf-patah.
            if let Some(c2) = radical_idx(&seq, 2) {
                seq[c2].vowel = Some(Vowel::HatafPatah);
            }
            if let Some(c3) = radical_idx(&seq, 3) {
                seq[c3] = Cons::new(letter::VAV).with_vowel(Vowel::Holam);
                seq.push(Cons::new(letter::TAV));
                attested = true;
            }
        } else if form == Form::Jussive && imperfect_suffix_kind(pgn) == Suffix::Zero {
            // The apocopated jussive/wayyiqtol 3ms — yištáḥû (וַיִּשְׁתַּחוּ):
            // the he drops and the etymological vav radical survives as a
            // word-final shureq carrying the syllable. The derived-stem
            // apocope arm has already removed the he and bared C2; rewrite
            // the tail as ḥet + shureq.
            if let Some(c2) = radical_idx(&seq, 2) {
                seq[c2].vowel = None;
                seq.truncate(c2 + 1);
                seq.push(Cons::new(letter::VAV).with_dagesh());
                attested = true;
            }
        }
    }

    (seq, attested)
}

/// הלך "to go" — recognised by its three radicals so its lexically irregular
/// Qal conjugation (see `apply_gizra`/`build_wayyiqtol`) can be applied.
fn is_halak(root: &Root) -> bool {
    root.pe() == letter::HE && root.ayin() == letter::LAMED && root.lamed() == letter::KAF
}

/// נתן "to give" — recognised by its three radicals for its lexically
/// irregular Qal conjugation (tsere imperfect theme, perfect nun assimilation).
fn is_natan(root: &Root) -> bool {
    root.pe() == letter::NUN && root.ayin() == letter::TAV && root.lamed() == letter::NUN
}

/// נגשׁ "to draw near" — a I-nun verb whose Qal imperfect takes an a-class
/// patah theme (yiggaš יִגַּשׁ, wayyiggaš וַיִּגַּשׁ) rather than the I-nun default
/// holam (yiggōš). Lexically irregular like נתן (which takes tsere instead).
fn is_nagash(root: &Root) -> bool {
    root.pe() == letter::NUN && root.ayin() == letter::GIMEL && root.lamed() == letter::SHIN
}

/// חָזַק "to be strong / to prevail" — its Qal wayyiqtol shortens the C2
/// holam to patah under stress retraction: וַיֶּחֱזַק, not *וַיֶּחֱזֹק.
fn is_hazaq(root: &Root) -> bool {
    root.pe() == letter::HET && root.ayin() == letter::ZAYIN && root.lamed() == letter::QOF
}

/// לקח "to take" — its C1 lamed assimilates like a I-nun verb in the Qal.
fn is_laqah(root: &Root) -> bool {
    root.pe() == letter::LAMED && root.ayin() == letter::QOF && root.lamed() == letter::HET
}

/// ראה "to see" — doubly weak (II-aleph, III-he) with an irregular apocopated
/// jussive/wayyiqtol 3ms (וַיַּרְא).
fn is_raah(root: &Root) -> bool {
    root.pe() == letter::RESH && root.ayin() == letter::ALEF && root.lamed() == letter::HE
}

/// ראה tsere-segol jussive/imperfect 3ms — yēreʔ יֵרֶא: a tsere prefix yod over a
/// segol C1 resh, the C2 aleph quiescent and the III-he dropped. Built directly
/// (the canonical form is the patah-prefix yarʔ); caller gates to ראה 3ms.
fn raah_apocopated_tsere_variant(root: &Root) -> String {
    let seq = [
        Cons::new(letter::YOD).with_vowel(Vowel::Tsere),
        Cons::radical(root.pe(), 1).with_vowel(Vowel::Segol),
        Cons::radical(root.ayin(), 2),
    ];
    hebrew::render(&seq)
}

/// חיה "to live" — III-he with an irregular apocopated jussive/wayyiqtol 3ms
/// (יְחִי / וַיְחִי) whose consecutive vav adds no forte to the sheva-prefix.
fn is_chayah(root: &Root) -> bool {
    root.pe() == letter::HET && root.ayin() == letter::YOD && root.lamed() == letter::HE
}

/// היה "to be" — III-he with the same irregular apocopated jussive/wayyiqtol as
/// its twin חיה: the prefix reduces to vocal sheva and C1 takes hiriq (יְהִי /
/// וַיְהִי, תְּהִי / וַתְּהִי).
fn is_hayah(root: &Root) -> bool {
    root.pe() == letter::HE && root.ayin() == letter::YOD && root.lamed() == letter::HE
}

/// חרה "to be(come) hot/angry" — III-he, I-guttural whose apocopated jussive 3ms
/// keeps the hiriq prefix (יִחַר / וַיִּחַר) rather than lowering it to patah the
/// way the parallel עלה does (וַיַּעַל).
fn is_charah(root: &Root) -> bool {
    root.pe() == letter::HET && root.ayin() == letter::RESH && root.lamed() == letter::HE
}

/// ירא "to fear" — a I-yod, III-aleph verb whose Qal imperfect is irregular: it
/// does NOT elide the yod the way a normal I-yod verb does (yēšēḇ), but keeps it
/// as a hiriq-mater under a hiriq preformative — yîrāʾ (יִירָא), tîrāʾ (תִּירָא),
/// ʾîrāʾ (אִירָא).
fn is_yare(root: &Root) -> bool {
    root.pe() == letter::YOD && root.ayin() == letter::RESH && root.lamed() == letter::ALEF
}

/// יכל "to be able" — a I-yod root whose Qal imperfect is suppletive-looking:
/// the û theme on the preformative with the radical yod lost — yûḵal (יוּכַל).
fn is_yakol(root: &Root) -> bool {
    root.pe() == letter::YOD && root.ayin() == letter::KAF && root.lamed() == letter::LAMED
}

/// יטב "to be good/well" — a true I-Yod root (not an original I-Vav). Unlike
/// the I-Vav majority (yēšēḇ), the yod is retained as a hiriq-yod mater and the
/// theme lowers to patah: yîṭaḇ (יִיטַב), wayyîṭaḇ (וַיִּיטַב), rather than the
/// contracted tsere yēṭēḇ. Corrected in the I-Yod pass.
fn is_yatab(root: &Root) -> bool {
    root.pe() == letter::YOD && root.ayin() == letter::TET && root.lamed() == letter::BET
}

/// בושׁ "to be ashamed" — a hollow (II-vav) root with an irregular Qal: the
/// imperfect takes a tsere preformative rather than the regular û-class qamats
/// (yēḇôš יֵבוֹשׁ / yēḇōšû יֵבֹשׁוּ, not yāḇûš), and the perfect keeps a holam
/// theme vowel (bōšû בֹשׁוּ). Classified as the ô-class in `hollow_class`; the
/// vowels unique to this lexeme are corrected in the irregulars pass.
fn is_bosh(root: &Root) -> bool {
    root.pe() == letter::BET && root.ayin() == letter::VAV && root.lamed() == letter::SHIN
}

/// מות "to die" — a û-class hollow with a stative Qal perfect: the theme vowel
/// is tsere (mēṯ מֵת, mēṯû מֵתוּ) rather than the regular qamats (māṯ). Corrected
/// in the irregulars pass; the imperfect (yāmûṯ) is the regular û-class.
fn is_mut(root: &Root) -> bool {
    root.pe() == letter::MEM && root.ayin() == letter::VAV && root.lamed() == letter::TAV
}

/// הרה "to conceive" — III-He, I-guttural. Its apocopated jussive/wayyiqtol
/// takes an a-class patah preformative (wattahar וַתַּהַר), not the regular
/// III-He hiriq (wattihreh-derived תִּהַר). The III-He apocope already produces
/// tihar; the irregulars pass only lowers the preformative to patah.
fn is_harah(root: &Root) -> bool {
    root.pe() == letter::HE && root.ayin() == letter::RESH && root.lamed() == letter::HE
}

fn is_bakah(root: &Root) -> bool {
    root.pe() == letter::BET && root.ayin() == letter::KAF && root.lamed() == letter::HE
}

fn is_shachah(root: &Root) -> bool {
    root.pe() == letter::SHIN && root.ayin() == letter::HET && root.lamed() == letter::HE
}

/// Rewrite an I-Yod / הלך Qal infinitive construct as the segholate form with
/// a tav afformative: drop C1, give both surviving radicals segol, append tav
/// (šeḇeṯ שֶׁבֶת, leḵeṯ לֶכֶת). The guttural rules run afterwards (e.g. ידע→דַּעַת).
fn apply_iyod_segholate_infinitive(seq: &mut Vec<Cons>) {
    if let Some(idx) = radical_idx(seq, 1) {
        seq.remove(idx);
    }
    // A guttural C3 prefers the a-class segolate pattern — dáʿaṯ (דַּעַת,
    // ידע), not the e-class šeḇeṯ.
    let guttural_c3 = radical_idx(seq, 3).is_some_and(|i| hebrew::is_guttural(seq[i].letter));
    let v = if guttural_c3 {
        Vowel::Patah
    } else {
        Vowel::Segol
    };
    if let Some(i) = radical_idx(seq, 2) {
        seq[i].vowel = Some(v);
    }
    if let Some(i) = radical_idx(seq, 3) {
        seq[i].vowel = Some(v);
    }
    seq.push(Cons::new(letter::TAV));
}

fn apply_hithpael_metathesis(seq: &mut Vec<Cons>, _root: &Root) -> bool {
    // Sibilant metathesis: hit- + s... → his-t... (ס/ש), hiz-d... (ז), hits-t... (צ).
    // The prefix TAV immediately precedes the first radical.
    let Some(c1_idx) = radical_idx(seq, 1) else {
        return false;
    };
    if c1_idx == 0 {
        return false;
    }
    let t_idx = c1_idx - 1;
    if seq[t_idx].letter != letter::TAV || seq[t_idx].role != Role::Affix {
        return false;
    }

    let c1 = seq[c1_idx].letter;
    // Dental assimilation: the infix tav assimilates fully into a dental C1
    // (ט/ד/ת) as a forte dagesh — hiṭṭahēr הִטַּהֵר, middabbēr מִטַּהֵר-style,
    // not *hiṯṭahēr.
    if matches!(c1, letter::TET | letter::DALET | letter::TAV) {
        seq.remove(t_idx);
        seq[t_idx].dagesh = true;
        return true;
    }
    if !hebrew::is_sibilant(c1) {
        return false;
    }

    // Metathesis: swap the TAV and the sibilant radical.
    let v_tav = seq[t_idx].vowel;
    let v_c1 = seq[c1_idx].vowel;
    seq.swap(t_idx, c1_idx);

    // Vowels: C1 takes the sheva from the prefix TAV; the TAV keeps its
    // theme-grade vowel (which was C1's vowel).
    seq[t_idx].vowel = v_tav;
    seq[c1_idx].vowel = v_c1;

    // The TAV now follows a silent sheva (on the sibilant); give it a
    // dagesh lene.
    seq[c1_idx].dagesh = true;

    // Emphatic assimilation.
    match c1 {
        letter::ZAYIN => {
            // hit-z... → hiz-d... (dalet)
            seq[c1_idx].letter = letter::DALET;
        }
        letter::TSADE => {
            // hit-ts... → hits-t... (tet)
            seq[c1_idx].letter = letter::TET;
        }
        _ => {}
    }

    true
}

fn apply_pe_nun(seq: &mut Vec<Cons>, root: &Root, binyan: Binyan, form: Form, _pgn: Pgn) -> bool {
    // Qal Imperfect / Imperative / Inf. Construct: nun closes a syllable
    // before C2 and assimilates as a dagesh on C2.
    // Niphal: ni- + nC2C3 → nin- + nC2C3 — the prefix nun stays, the root
    // nun also stays (no assimilation since both are word-internal with
    // a vowel between).
    // Hiphil: hi- + nC2C3 → hîC2C2îC3 (e.g., hiqîlַ → wait, that's wrong;
    // hîqîl is the hollow pattern. Hiphil of I-nun: hippîl "fell" → hiphîl.
    // For Hiphil Perfect: nun assimilates as dagesh on C2: hipîl, where
    // C1 (nun) has been replaced by dagesh on C2.
    //
    // Strong-verb sequence for Qal Imperfect 3ms is yi-nun(sheva)-pe(holam)-lamed
    // (yinpōl). We rewrite to yip(dagesh)pōl by:
    //  - Dropping the nun consonant
    //  - Adding dagesh to C2
    //  - Possibly retaining the hiriq on the prefix (it stays hiriq, not
    //    closed by a long vowel).
    //
    // For Qal Imperative the prefix is gone in the strong form; for I-nun
    // the initial nun-sheva is dropped, yielding pōl (e.g., גַּשׁ for
    // נגשׁ). For some I-nun roots the Qal Imperative keeps the nun (נְפֹל).
    // We choose the "dropped nun" form as the typical pattern.

    let mut changed = false;
    // A guttural (or resh) C2 refuses the assimilation dagesh, so the nun stays
    // and the strong shape stands: yinhaḡ יִנְהַג, yinʿar — not *yihaḡ. The one
    // exception is the Niphal of a ḥet root, where assimilation with virtual
    // doubling is the attested shape (niḥam נִחַם, niḥamtî נִחַמְתִּי).
    if (hebrew::is_guttural(root.ayin()) || root.ayin() == letter::RESH)
        && !(binyan == Binyan::Niphal && root.ayin() == letter::HET)
    {
        return false;
    }
    // C1 is the radical-1 slot — uniquely identified by its Role tag, so we
    // don't confuse it with the 1cp prefix nun or the 3fp/2fp -nâ suffix nun.
    let _ = root;
    let c1_idx = radical_idx(seq, 1);
    match (binyan, form) {
        (Binyan::Qal, Form::Imperfect)
        | (Binyan::Qal, Form::Cohortative)
        | (Binyan::Qal, Form::Jussive)
        | (Binyan::Niphal, Form::Perfect)
        // The Niphal participle (niqṭāl) has the same C1-silent-sheva shape as
        // the perfect, so the radical nun assimilates the same way: niḇbā נִבָּא,
        // mp niḇbᵊʔîm נִבְּאִים, niggāš.
        | (Binyan::Niphal, Form::ParticipleActive)
        | (Binyan::Niphal, Form::ParticiplePassive)
        | (Binyan::Hiphil, _)
        | (Binyan::Hophal, _) => {
            if let Some(idx) = c1_idx
                && idx + 1 < seq.len()
            {
                seq.remove(idx);
                seq[idx].dagesh = true;
                // Hophal of an assimilating I-nun is u-class: the prefix's
                // qamats-qatan raises to qubuts before the doubled C2 —
                // huggaḏ הֻגַּד, yuggaḏ יֻגַּד (וַיֻּגַּד).
                if binyan == Binyan::Hophal
                    && idx > 0
                    && matches!(
                        seq[idx - 1].vowel,
                        Some(Vowel::QamatsQatan | Vowel::Qamats)
                    )
                {
                    seq[idx - 1].vowel = Some(Vowel::Qubuts);
                }
                changed = true;
            }
        }
        (Binyan::Qal, Form::Imperative) | (Binyan::Qal, Form::InfinitiveConstruct) => {
            // Drop the initial nun-sheva. Strong form was nun-sheva, C2, C3.
            if let Some(idx) = c1_idx {
                seq.remove(idx);
                changed = true;
            }
            if form == Form::InfinitiveConstruct && root.lamed() == letter::ALEF {
                // נשא → שְׂאֵת / שֵׂאת. The holam (נְשֹׂא) lowers to tsere and
                // a tav afformative is added. Both šəʾēṯ and šēʾṯ occur; we
                // pick tsere on C2 (šē-) which matches the common לָשֵׂאת.
                if let Some(c2) = radical_idx(seq, 2) {
                    seq[c2].vowel = Some(Vowel::Tsere);
                }
                if let Some(c3) = radical_idx(seq, 3) {
                    seq[c3].vowel = None;
                }
                seq.push(Cons::new(letter::TAV));
            }
        }
        _ => {}
    }
    changed
}

fn apply_pe_yod(seq: &mut Vec<Cons>, root: &Root, binyan: Binyan, form: Form, pgn: Pgn) -> bool {
    // I-Yod is the most complex weak class. In Qal Imperfect of originally
    // I-Vav verbs (yāšab, yāḏaʿ, etc.), the yod drops and the prefix vowel
    // becomes tsere: yēšēb. In Niphal & Hiphil the original vav reappears.
    //
    // First-pass implementation: in Qal Imperfect, drop the yod and turn
    // the prefix vowel into tsere; in Hiphil, replace the C1 yod with a
    // mater vav (hôšîaʿ-style).
    let mut changed = false;
    // True I-Yod stative roots (יטב) retain the yod as a hiriq-yod mater rather
    // than contracting to tsere: yîṭaḇ (יִיטַב). The prefix keeps its hiriq, C1
    // yod loses its vowel (becoming a mater), and the theme lowers to patah.
    if is_yatab(root)
        && matches!(
            (binyan, form),
            (
                Binyan::Qal,
                Form::Imperfect | Form::Jussive | Form::Cohortative | Form::Wayyiqtol
            )
        )
    {
        if let Some(c1) = radical_idx(seq, 1) {
            seq[c1].vowel = None;
        }
        if let Some(c2) = radical_idx(seq, 2) {
            seq[c2].vowel = Some(Vowel::Patah);
        }
        return true;
    }
    match (binyan, form) {
        (Binyan::Qal, Form::InfinitiveConstruct) if root.lamed() != letter::ALEF => {
            // Segholate infinitive with a tav afformative: šeḇeṯ (שֶׁבֶת),
            // redeṯ (רֶדֶת), leḵeṯ. The initial yod drops and both surviving
            // radicals take segol. (III-aleph יצא→צֵאת differs and falls to the
            // arm below.)
            apply_iyod_segholate_infinitive(seq);
            changed = true;
        }
        (Binyan::Qal, Form::Imperfect)
        | (Binyan::Qal, Form::Cohortative)
        | (Binyan::Qal, Form::Jussive)
        | (Binyan::Qal, Form::Imperative)
        | (Binyan::Qal, Form::InfinitiveConstruct) => {
            // Find C1 (the yod radical) and drop it; prefix vowel → tsere.
            if let Some(idx) = radical_idx(seq, 1) {
                if idx > 0 {
                    seq[idx - 1].vowel = Some(Vowel::Tsere);
                }
                seq.remove(idx);
                // build_imperfect put a dagesh lene on a begedkefet C2,
                // assuming C1's silent sheva closed the prefix syllable
                // (yiḏ-…). With C1 gone the prefix vowel is the long tsere
                // (yē-ḏaʿ), so the begedkefet C2 spirantises — drop the dagesh.
                if let Some(c2) = seq.get_mut(idx)
                    && hebrew::is_begedkefet(c2.letter)
                {
                    c2.dagesh = false;
                }
                changed = true;
            }
            if root.lamed() == letter::ALEF {
                // The C2 theme holam lowers to tsere for the short (zero-suffix)
                // forms (יֵצֵא, צֵא). Vocalic-suffix forms keep the sheva on C2
                // that build_imperfect already placed (יֵצְאוּ).
                if imperfect_suffix_kind(pgn) == Suffix::Zero {
                    if let Some(c2) = radical_idx(seq, 2) {
                        seq[c2].vowel = Some(Vowel::Tsere);
                    }
                    if let Some(c3) = radical_idx(seq, 3) {
                        seq[c3].vowel = None;
                    }
                }
                if form == Form::InfinitiveConstruct {
                    seq.push(Cons::new(letter::TAV));
                }
            }
            // The theme vowel of these original-I-Vav verbs is tsere, not the
            // strong holam: yēšēḇ, yērēḏ, yēlēḏ. The vocalic-suffix grade keeps
            // its sheva (yēšḇû). III-guttural verbs (ידע) lower the theme to
            // patah instead (yēḏaʿ), so leave the holam for apply_lamed_guttural
            // to handle; III-aleph (יצא) ends up tsere either way.
            if !root.has(Gizra::LamedGuttural)
                && let Some(c2) = radical_idx(seq, 2)
                && seq[c2].vowel == Some(Vowel::Holam)
            {
                seq[c2].vowel = Some(Vowel::Tsere);
            }
        }
        (Binyan::Hiphil, _) => {
            // Hiphil of original I-Vav: the vav reappears as a holam-male
            // (hôšîb < *haw-šīb). The prefix consonant takes no vowel and the
            // vav carries the holam — הוֹלִיד / יוֹלִיד, not הֹולִיד / יֹולִיד.
            if let Some(idx) = radical_idx(seq, 1) {
                if idx > 0 {
                    seq[idx - 1].vowel = None;
                }
                seq[idx] = Cons::new(letter::VAV).with_vowel(Vowel::Holam);
                // C1 is now an open vav-mater syllable (yô-/hô-), so a
                // begedkefet C2 is spirant — drop the dagesh lene the strong
                // builder added when C1 still carried a silent sheva
                // (yôḏeh יוֹדֶה, not יוֹדֶּה; hôḏû הוֹדוּ).
                if let Some(c2) = radical_idx(seq, 2) {
                    seq[c2].dagesh = false;
                }
                changed = true;
            }
        }
        (Binyan::Niphal, Form::Perfect | Form::ParticipleActive) => {
            // Niphal perfect & participle of original I-Vav: like the Hiphil,
            // the vav reappears as a holam-male after the nun preformative,
            // which loses its hiriq — nôṯar (נוֹתַר), participle nôṯār (נוֹתָר),
            // nôlaḏ (נוֹלַד), nôḏaʕ (נוֹדַע). (The imperfect doubles the vav
            // instead — yiwwāṯēr — and is left to the strong pattern.)
            if let Some(idx) = radical_idx(seq, 1) {
                if idx > 0 {
                    seq[idx - 1].vowel = None;
                }
                seq[idx] = Cons::new(letter::VAV).with_vowel(Vowel::Holam);
                // C1 is now an open vav-mater syllable (nô-), so a begedkefet C2
                // is spirant — drop the dagesh lene the strong builder added when
                // C1 still carried a silent sheva (nôṯar נוֹתַר, not נוֹתַּר).
                if let Some(c2) = radical_idx(seq, 2) {
                    seq[c2].dagesh = false;
                }
                changed = true;
            }
        }
        _ => {}
    }
    changed
}

/// The characteristic long vowel of a hollow (II-vav/yod) root's Qal: û
/// (קום→yāqûm), ô (בוא→yāḇôʾ), or î (שׂים/בין→yāśîm). C2=yod marks the î-class;
/// בוא is the common ô-class verb; everything else defaults to û.
#[derive(PartialEq, Clone, Copy)]
enum HollowClass {
    Shureq,
    Holam,
    Hiriq,
}

fn hollow_class(root: &Root) -> HollowClass {
    if root.ayin() == letter::YOD {
        HollowClass::Hiriq
    } else if (root.pe() == letter::BET && root.lamed() == letter::ALEF) || is_bosh(root) {
        HollowClass::Holam
    } else {
        HollowClass::Shureq
    }
}

fn apply_hollow(seq: &mut Vec<Cons>, root: &Root, binyan: Binyan, form: Form, pgn: Pgn) -> bool {
    // Hollow verbs lose their middle radical in almost every form. The
    // characteristic vowel takes its place: Qal Perfect qām (kāmū), Qal
    // Imperfect yāqûm, Hiphil hēqîm.
    //
    // First-pass: handle Qal Perfect 3ms (replace C1-C2-C3 with C1-qamats,
    // C3) and Qal Imperfect 3ms (replace yi-C1(sheva)-C2(holam)-C3 with
    // yā-C1, vav(shureq), C3).
    let mut changed = false;
    let class = hollow_class(root);
    match (binyan, form) {
        (Binyan::Qal, Form::Perfect) => {
            // qām: drop the middle radical, keep C1-qamats and C3.
            if let Some(idx) = radical_idx(seq, 2) {
                seq.remove(idx);
                changed = true;
            }
            // Before a consonantal afformative the theme vowel shortens to
            // patah: śamtî (שַׂמְתִּי), qamtā (קַמְתָּ), šabtî (שַׁבְתִּי). The
            // zero/vocalic forms keep qamats (qām, qāmâ, qāmû); the heavy
            // 2mp/2fp forms reduce to vocal sheva (a separate grade). III-aleph
            // hollow (בוא) keeps qamats — the quiescent aleph lengthens it
            // (bāʾtî) — and is handled by apply_lamed_aleph.
            if perfect_suffix_kind(pgn) == Suffix::Consonantal
                && root.lamed() != letter::ALEF
                && let Some(c1_idx) = radical_idx(seq, 1)
                && seq[c1_idx].vowel == Some(Vowel::Qamats)
            {
                seq[c1_idx].vowel = Some(Vowel::Patah);
            }
        }
        (Binyan::Qal, Form::Imperfect)
        | (Binyan::Qal, Form::Cohortative)
        | (Binyan::Qal, Form::Imperative)
        | (Binyan::Qal, Form::InfinitiveConstruct) => {
            // The characteristic long vowel depends on the class: û (yāqûm),
            // ô (yāḇôʾ), or î (yāśîm). Before a vocalic suffix the ô-class
            // writes defectively — holam on C1, no mater (yāḇōʾû).
            let vocalic = imperfect_suffix_kind(pgn) == Suffix::Vocalic;
            if let Some(c1_idx) = radical_idx(seq, 1) {
                if c1_idx > 0 {
                    seq[c1_idx - 1].vowel = Some(Vowel::Qamats);
                }
                seq[c1_idx].vowel = match class {
                    HollowClass::Hiriq => Some(Vowel::Hiriq),
                    HollowClass::Holam if vocalic => Some(Vowel::Holam),
                    _ => None,
                };
            }
            if let Some(c2_idx) = radical_idx(seq, 2) {
                match (class, vocalic) {
                    (HollowClass::Shureq, _) => {
                        seq[c2_idx] = Cons::mater(letter::VAV);
                        seq[c2_idx].dagesh = true;
                    }
                    (HollowClass::Holam, false) => {
                        seq[c2_idx] = Cons::mater(letter::VAV).with_vowel(Vowel::Holam);
                    }
                    (HollowClass::Holam, true) => {
                        seq.remove(c2_idx);
                    }
                    (HollowClass::Hiriq, _) => {
                        seq[c2_idx] = Cons::mater(letter::YOD);
                    }
                }
                changed = true;
            }
        }
        (Binyan::Qal, Form::Jussive) => {
            // Short (apocopated) jussive: prefix vowel → qamats, the middle
            // radical drops, and C1 takes the short theme vowel — holam for the
            // û/ô classes (yāqōm, yāḇōʾ), tsere for the î class (yāśēm).
            if let Some(c1_idx) = radical_idx(seq, 1) {
                if c1_idx > 0 {
                    seq[c1_idx - 1].vowel = Some(Vowel::Qamats);
                }
                seq[c1_idx].vowel = Some(if class == HollowClass::Hiriq {
                    Vowel::Tsere
                } else {
                    Vowel::Holam
                });
            }
            if let Some(c2_idx) = radical_idx(seq, 2) {
                seq.remove(c2_idx);
                changed = true;
            }
        }
        (Binyan::Qal, Form::ParticipleActive) => {
            // qām (קָם) — drop the middle radical from the active participle.
            if let Some(idx) = radical_idx(seq, 2) {
                seq.remove(idx);
                changed = true;
            }
        }
        (Binyan::Niphal, Form::Imperfect | Form::Jussive | Form::Wayyiqtol) => {
            // Hollow Niphal imperfect: yimmôṭ (יִמּוֹט) — the doubled C1
            // carries the ô written plene on the middle vav; the strong
            // build's C1 qamats and C2 tsere are both replaced.
            if let Some(c2) = radical_idx(seq, 2) {
                seq[c2] = Cons::new(letter::VAV).with_vowel(Vowel::Holam);
                if let Some(c1) = radical_idx(seq, 1) {
                    seq[c1].vowel = None;
                }
                changed = true;
            }
        }
        (Binyan::Qal, Form::InfinitiveAbsolute) => {
            // môṯ (מוֹת), qôm, śôm: the inf-abs holam is written plene on a
            // middle vav (a yod-class radical is re-spelt as vav) and C1 drops
            // the qamats the strong build (qāṭôl) gave it.
            if let Some(c2) = radical_idx(seq, 2) {
                seq[c2] = Cons::new(letter::VAV).with_vowel(Vowel::Holam);
                if let Some(c1) = radical_idx(seq, 1) {
                    seq[c1].vowel = None;
                }
                changed = true;
            }
        }
        (Binyan::Hiphil, Form::Perfect | Form::Imperfect) => {
            // Hollow Hiphil. Long grade (perfect zero/vocalic, all imperfect):
            // the middle radical drops and the î theme (hiriq + yod mater) sits
            // on C1 — hēqîm (הֵקִים), yāqîm (יָקִים). The prefix is tsere in the
            // perfect (hē-), qamats in the imperfect (yā-). Before a consonantal
            // afformative the theme shortens to tsere; for the III-aleph
            // subclass (בוא) the aleph then quiesces — hēḇēʔtî (הֵבֵאתִי), closed
            // off by apply_lamed_aleph. The longer -ôṯ- perfect of non-aleph
            // hollows (hăqîmôtî) is not yet modelled, so those consonantal-suffix
            // forms fall through to the strong fallback.
            let consonantal =
                form == Form::Perfect && perfect_suffix_kind(pgn) == Suffix::Consonantal;
            let aleph = root.lamed() == letter::ALEF;
            if !(consonantal && !aleph)
                && let Some(c2_idx) = radical_idx(seq, 2)
            {
                let c1_idx = radical_idx(seq, 1);
                seq.remove(c2_idx);
                if let Some(c1) = c1_idx {
                    seq[c1].vowel = Some(if consonantal {
                        Vowel::Tsere
                    } else {
                        Vowel::Hiriq
                    });
                    if c1 > 0 {
                        seq[c1 - 1].vowel = Some(if form == Form::Perfect {
                            Vowel::Tsere
                        } else {
                            Vowel::Qamats
                        });
                    }
                }
                changed = true;
            }
        }
        (Binyan::Hiphil, Form::Jussive | Form::Imperative) => {
            // Short hollow Hiphil grades. The jussive apocopates the î theme
            // to tsere — yāqēm (יָקֵם), yāšēḇ; build_wayyiqtol's nesiga then
            // lowers it to segol under the consecutive vav (וַיָּקֶם, וַיָּשֶׁב)
            // while III-aleph keeps tsere (וַיָּבֵא). The 2ms imperative is the
            // same short grade off the hā- prefix — hāqēm (הָקֵם), hāḇēʾ
            // (הָבֵא). The vocalic-subject grades (3mp/2mp/2fs) keep the long
            // î — the strong build's hiriq + yod mater survive C2's removal:
            // yāqîmû, hāqîmû (הָקִימוּ), hāḇîʾû.
            let zero = imperfect_suffix_kind(pgn) == Suffix::Zero;
            if let Some(c2_idx) = radical_idx(seq, 2) {
                let c1_idx = radical_idx(seq, 1);
                seq.remove(c2_idx);
                if let Some(c1) = c1_idx {
                    if zero {
                        seq[c1].vowel = Some(Vowel::Tsere);
                        // The strong build's î was hiriq + yod mater; the
                        // mater outlives C2's removal — drop it, the short
                        // grade is defective (יָקֵם, not יָקֵים).
                        if seq.get(c1 + 1).is_some_and(|c| {
                            c.letter == letter::YOD && c.vowel.is_none() && !c.dagesh
                        }) {
                            seq.remove(c1 + 1);
                        }
                    } else {
                        seq[c1].vowel = Some(Vowel::Hiriq);
                    }
                    if c1 > 0 {
                        seq[c1 - 1].vowel = Some(Vowel::Qamats);
                    }
                }
                changed = true;
            }
        }
        (Binyan::Hophal, Form::Perfect | Form::Imperfect) => {
            // Hophal of a hollow root is uniformly û-class regardless of the
            // root's Qal vowel: hûmat (הוּמַת), yûqam (יוּקַם), hûḇā (הוּבָא). The
            // ho-/yo- prefix's qamats-qatan becomes a shureq (vav mater), the
            // middle radical drops, and C1 carries the theme vowel — patah with
            // a zero/consonantal suffix, sheva before a vocalic one (yûmətû).
            let vocalic = imperfect_suffix_kind(pgn) == Suffix::Vocalic;
            if let Some(c1_idx) = radical_idx(seq, 1) {
                if c1_idx > 0 {
                    seq[c1_idx - 1].vowel = None;
                    let mut shureq = Cons::mater(letter::VAV);
                    shureq.dagesh = true;
                    seq.insert(c1_idx, shureq);
                }
                // c1 has shifted right by one after the insert.
                let c1_idx = radical_idx(seq, 1).unwrap();
                seq[c1_idx].vowel = Some(if vocalic { Vowel::Sheva } else { Vowel::Patah });
            }
            if let Some(c2_idx) = radical_idx(seq, 2) {
                seq.remove(c2_idx);
                changed = true;
            }
        }
        _ => {}
    }
    changed
}

/// Hollow Hiphil perfect before a consonantal/heavy afformative takes the
/// long linking-ô pattern: hăqîmôṯî (הֲקִימוֹתִי), hăḇîʔôṯ… — prefix he with a
/// hataf-patah, C1 + hiriq (+ optional yod mater), the middle radical drops,
/// C3 + holam (written plene with a vav mater or defectively), then the
/// afformative. The base `apply_hollow` branch leaves these to the strong
/// fallback (see its comment), so this is a pure additive recovery: it emits
/// the plene and defective spellings as alternants sharing the slot's label.
/// Returns the spelling variants (possibly empty for unsupported pgns).
///
/// III-aleph hollows (בוא) take the quiescent-aleph pattern (hēḇēʔṯî) handled
/// by `apply_lamed_aleph`, so they are excluded here.
fn hollow_hiphil_otav_perfect(root: &Root, pgn: Pgn) -> Vec<String> {
    use Vowel::*;
    if !root.has(Gizra::Hollow)
        || perfect_suffix_kind(pgn) != Suffix::Consonantal
            && perfect_suffix_kind(pgn) != Suffix::Heavy
    {
        return Vec::new();
    }
    // Afformative consonants + their leading vowel for the supported persons.
    let suffix: Vec<Cons> = match (pgn.person, pgn.gender, pgn.number) {
        (Some(Person::First), _, Some(Number::Singular)) => {
            vec![
                Cons::new(letter::TAV).with_vowel(Hiriq),
                Cons::new(letter::YOD),
            ]
        }
        (Some(Person::Second), Some(Gender::Masculine), Some(Number::Singular)) => {
            vec![Cons::new(letter::TAV).with_vowel(Qamats)]
        }
        (Some(Person::Second), Some(Gender::Feminine), Some(Number::Singular)) => {
            vec![Cons::new(letter::TAV).with_vowel(Sheva)]
        }
        (Some(Person::First), _, Some(Number::Plural)) => {
            vec![Cons::new(letter::NUN), Cons::new(letter::VAV).with_dagesh()]
        }
        (Some(Person::Second), Some(Gender::Masculine), Some(Number::Plural)) => {
            vec![
                Cons::new(letter::TAV).with_vowel(Segol),
                Cons::new(letter::MEM),
            ]
        }
        (Some(Person::Second), Some(Gender::Feminine), Some(Number::Plural)) => {
            vec![
                Cons::new(letter::TAV).with_vowel(Segol),
                Cons::new(letter::NUN),
            ]
        }
        _ => return Vec::new(),
    };
    let c1 = root.pe();
    let c3 = root.lamed();
    // A III-aleph C3 carries the linking holam on the aleph itself (hăḇîʾōṯî
    // הֲבִיאֹתִי) — it never spells the holam with a following vav mater, so the
    // "plene" link collapses onto the bare aleph for these roots.
    let c3_alef = c3 == letter::ALEF;
    let build = |prefix_vowel: Vowel, plene_c1: bool, plene_c3: bool| -> String {
        let mut seq: Vec<Cons> = Vec::new();
        seq.push(Cons::new(letter::HE).with_vowel(prefix_vowel));
        seq.push(Cons::new(c1).with_vowel(Hiriq));
        if plene_c1 {
            seq.push(Cons::mater(letter::YOD));
        }
        if plene_c3 && !c3_alef {
            seq.push(Cons::new(c3).with_vowel(Holam));
            seq.push(Cons::new(letter::VAV).with_vowel(Holam));
        } else {
            seq.push(Cons::new(c3).with_vowel(Holam));
        }
        seq.extend(suffix.iter().cloned());
        hebrew::render(&seq)
    };
    // Emit the four mater combinations (yod present/absent × vav present/absent).
    // Only the attested spelling matches a real surface; the rest are inert.
    // The unreduced patah-he grade also occurs (haʿîḏōṯî הַעִידֹתִי).
    let mut out = vec![
        build(HatafPatah, true, true),
        build(HatafPatah, true, false),
        build(HatafPatah, false, true),
        build(HatafPatah, false, false),
        build(Patah, true, false),
        build(Patah, false, false),
    ];
    // The tsere grade of the linking-ô perfect: hăqēmōtā (הֲקֵמֹתָ), hăšēḇōtā
    // (וַהֲשֵׁבֹתָ) — C1 takes tsere (the ē of the bare hēqîm stem) instead of
    // the î, so no yod mater, but the holam link to the afformative stays. The
    // unreduced patah-he grade pairs with it, and the link writes plene or
    // defective.
    for he_v in [HatafPatah, Patah] {
        for plene_c3 in [true, false] {
            let mut seq = vec![
                Cons::new(letter::HE).with_vowel(he_v),
                Cons::new(c1).with_vowel(Tsere),
                Cons::new(c3).with_vowel(Holam),
            ];
            if plene_c3 && !c3_alef {
                seq.push(Cons::new(letter::VAV).with_vowel(Holam));
            }
            seq.extend(suffix.iter().cloned());
            out.push(hebrew::render(&seq));
        }
    }
    // The contracted short-stem grade without the linking ô — hēqamtā
    // הֵקַמְתָּ, hēnap̄tā וְהֵנַפְתָּ (and the reduced hă- twin). The begedkefet
    // afformative tav keeps its dagesh lene after the now-closed syllable.
    for he_v in [Tsere, HatafPatah] {
        let mut seq = vec![
            Cons::new(letter::HE).with_vowel(he_v),
            Cons::new(c1).with_vowel(Patah),
            Cons::new(c3).with_vowel(Sheva),
        ];
        let mut tail = suffix.clone();
        if let Some(first) = tail.first_mut()
            && hebrew::is_begedkefet(first.letter)
        {
            first.dagesh = true;
        }
        seq.extend(tail);
        out.push(hebrew::render(&seq));
        // When C3 is the afformative's own consonant (a tav-root like מות
        // before the -tî/-tā/-tem tav afformatives), the two coalesce into a
        // single dageshed radical carrying the afformative vowel — hēmattî
        // הֵמַתִּי, hēmattā הֵמַתָּה — rather than the doubled-spelling הֵמַתְתִּי.
        if suffix.first().is_some_and(|f| f.letter == c3) {
            let mut merged = vec![
                Cons::new(letter::HE).with_vowel(he_v),
                Cons::new(c1).with_vowel(Patah),
                Cons::new(c3).with_dagesh().with_vowel(suffix[0].vowel.unwrap_or(Hiriq)),
            ];
            merged.extend(suffix.iter().skip(1).cloned());
            out.push(hebrew::render(&merged));
        }
    }
    // The 2ms perfect also takes the paragogic he (-tā → -tāh): hărîmôṯâ
    // הֲרִימוֹתָה, hēmattâ הֵמַתָּה. Mirror every 2ms form with a trailing he.
    if (pgn.person, pgn.gender, pgn.number)
        == (Some(Person::Second), Some(Gender::Masculine), Some(Number::Singular))
    {
        let with_he: Vec<String> = out.iter().map(|s| format!("{s}\u{05D4}")).collect();
        out.extend(with_he);
    }
    out
}

fn apply_geminate(seq: &mut Vec<Cons>, root: &Root, binyan: Binyan, form: Form, _pgn: Pgn) -> bool {
    // Geminate roots fuse the two identical radicals into one with dagesh
    // forte in most forms. Qal Perfect 3ms: sāḇaḇ → סָבַב (often as written).
    // Qal Imperfect 3ms: yāsōḇ — drop one C and add dagesh.
    let mut changed = false;
    let _ = root;
    match (binyan, form) {
        (Binyan::Qal, Form::Imperfect)
        | (Binyan::Qal, Form::Cohortative)
        | (Binyan::Qal, Form::Jussive)
        | (Binyan::Qal, Form::Imperative)
        | (Binyan::Qal, Form::InfinitiveConstruct) => {
            // yāsōḇ: drop C3, carry the holam theme on C1, double the (remaining)
            // C2, prefix vowel → qamats. Putting the holam on C1 and clearing C2
            // gives the canonical yā-sōḇ; crucially it also produces the vocalic
            // plural yā-sōḇ-ḇû (יָסֹבּוּ) — the strong builder reduces C2 to a
            // sheva there, which the old "C1 vowelless, holam left on C2" shape
            // mangled into יָסְבְּוּ.
            if let Some(c3_idx) = radical_idx(seq, 3) {
                seq.remove(c3_idx);
                changed = true;
            }
            if let Some(c1_idx) = radical_idx(seq, 1) {
                if c1_idx > 0 {
                    seq[c1_idx - 1].vowel = Some(Vowel::Qamats);
                }
                seq[c1_idx].vowel = Some(Vowel::Holam);
            }
            if let Some(c2_idx) = radical_idx(seq, 2) {
                // The fused radical doubles (dagesh forte) only when something
                // follows to close the syllable — the vocalic plural yāsōḇḇû
                // (יָסֹבּוּ). Word-final it cannot be doubled, so the bare forms
                // surface single and undageshed: imperative sōḇ (סֹב), inf-cstr
                // tōm (תֹּם), imperfect 3ms yāsōḇ (יָסֹב). A stray forte there
                // produced יָסֹּב, which matches no Masoretic surface.
                seq[c2_idx].dagesh = c2_idx + 1 < seq.len();
                seq[c2_idx].vowel = None;
            }
        }
        _ => {}
    }
    changed
}

fn apply_lamed_he(seq: &mut Vec<Cons>, root: &Root, binyan: Binyan, form: Form, pgn: Pgn) -> bool {
    // III-He (originally III-Yod). The final he is etymological — it's a
    // mater for various endings depending on form:
    //   Qal Perfect 3ms: bānâ  (בָּנָה) — C1-C2-â (qamats + he)
    //   Qal Perfect 3fs: bāntâ (בָּנְתָה)
    //   Qal Imperfect 3ms: yibnê (יִבְנֶה) — segol + he
    //   Qal Imperative ms: bənê (בְּנֵה) — tsere + he
    //   Inf. Const: bənôt (בְּנוֹת)
    //   Jussive 3ms: yibn (יִבֶן) — apocopated, no he
    //   Active participle ms: bōnê (בּוֹנֶה)
    //
    // First-pass: for forms where the strong builder put a "he" as C3, swap
    // the vowel pattern on C2 + he according to form. For 3fs Perfect,
    // insert a tav before the he (-tâ). For most consonantal-suffix
    // Perfects, replace the final he with yod or drop it.
    //
    // We approximate by overwriting the last 2-3 elements of `seq` with the
    // canonical III-He ending. This is enough for the common forms.
    let mut changed = false;
    if root.lamed() != letter::HE {
        return false;
    }
    use Vowel::*;

    // The plural jussives never apocopate — their forms coincide with the
    // imperfect (תִּרְאוּ, אַל־תַּעֲשׂוּ) — so route them through the imperfect
    // arm below rather than leaving the strong-form he tail untouched.
    let form = if form == Form::Jussive && pgn.number == Some(Number::Plural) {
        Form::Imperfect
    } else {
        form
    };

    // Hiphil III-He Perfect interposes the î-mater (yod) between C2 and the
    // etymological he in the Zero/Vocalic grades (build_perfect line ~1013),
    // exactly as the imperfect does — heʿĕlâ (הֶעֱלָה), heʿĕlû (הֶעֱלוּ). The
    // III-He ending attaches straight to C2, so strip that interposed yod
    // before rewriting the tail. (Consonantal/heavy grades use no mater here,
    // so there is nothing to remove.)
    if binyan == Binyan::Hiphil
        && form == Form::Perfect
        && let (Some(c2), Some(c3)) = (radical_idx(seq, 2), radical_idx(seq, 3))
    {
        for i in (c2 + 1..c3).rev() {
            seq.remove(i);
        }
    }

    // C2 is the radical-2 slot regardless of where the (possibly-replaced)
    // C3 he ends up. Looking it up by role beats counting from the end —
    // we may have appended a tav-suffix that shifts positions.
    let c2_idx = radical_idx(seq, 2);
    let c3_idx = radical_idx(seq, 3);
    match (binyan, form, pgn.person, pgn.gender, pgn.number) {
        (
            _,
            Form::Perfect,
            Some(Person::Third),
            Some(Gender::Masculine),
            Some(Number::Singular),
        ) => {
            // bānâ / ṣiwwâ (צִוָּה) / niglâ: C2 takes qamats before the final he.
            // Uniform across binyanim — build_perfect already set the binyan's
            // C1/doubling, so we only adjust the III-He ending here.
            if let Some(i) = c2_idx {
                seq[i].vowel = Some(Qamats);
            }
            changed = true;
        }
        (
            _,
            Form::Perfect,
            Some(Person::Third),
            Some(Gender::Feminine),
            Some(Number::Singular),
        ) => {
            // bāntâ — replace the C3 he with a tav-qamats; final he stays.
            // Strong gave us [...ayin-sheva, he-qamats, he]. Rewrite C3:
            if let Some(i) = c3_idx {
                let mut tav = Cons::new(letter::TAV);
                tav.vowel = Some(Qamats);
                seq[i] = tav;
            }
            changed = true;
        }
        (_, Form::Perfect, ..) => {
            // The remaining persons (3cp and all consonantal/heavy suffixes).
            // Before a consonantal suffix the etymological he becomes a yod
            // mater and C2 takes hiriq: ʿāśîtî (עָשִׂיתִי), bānîtā (בָּנִיתָ),
            // rāʾînû (רָאִינוּ). The 3cp vocalic suffix elides the he entirely:
            // ʿāśû (עָשׂוּ).
            match perfect_suffix_kind(pgn) {
                Suffix::Consonantal | Suffix::Heavy => {
                    if let Some(i) = c2_idx {
                        // Qal/Piel/Pual take hiriq before the î-mater (ʿāśîtî,
                        // ṣiwwîtî); the Hiphil and Niphal take tsere — hirbêtî
                        // (הִרְבֵּיתִי), higlêtā, niḡlêtî.
                        seq[i].vowel = Some(if matches!(binyan, Binyan::Hiphil | Binyan::Niphal) {
                            Tsere
                        } else {
                            Hiriq
                        });
                    }
                    if let Some(i) = c3_idx {
                        seq[i] = Cons::mater(letter::YOD);
                        // The suffix consonant now follows a vowel (the mater),
                        // so a begedkefet tav spirantises: ʿāśîtā (עָשִׂיתָ), not
                        // עָשִׂיתָּ.
                        if i + 1 < seq.len() {
                            seq[i + 1].dagesh = false;
                        }
                    }
                    // The 2fs ending is a word-final tav here (ʿāśît עָשִׂית,
                    // bānît בָּנִית): its silent sheva is not written in the MT,
                    // so drop it. Only the 2fs ends in a bare consonant among the
                    // consonantal/heavy suffixes (1cs -î, 2ms -ā, 1cp -û, 2mp -m).
                    if let Some(last) = seq.last_mut()
                        && last.vowel == Some(Sheva)
                    {
                        last.vowel = None;
                    }
                }
                Suffix::Vocalic => {
                    if let Some(i) = c3_idx {
                        seq.remove(i);
                    }
                    if let Some(i) = c2_idx {
                        // Regular III-He vocalic suffix drops the C2 vowel
                        // (yibnû יִבְנוּ). But if C2 is a guttural in a doubled
                        // binyan (Piel/Pual/Hithpael), it rejects the dagesh
                        // forte and takes a vocal hataf vowel instead
                        // (yištaḥăvû יִשְׁתַּחֲווּ) — so keep the sheva for
                        // apply_guttural to promote.
                        if matches!(binyan, Binyan::Piel | Binyan::Pual | Binyan::Hithpael)
                            && hebrew::is_guttural(seq[i].letter)
                        {
                            seq[i].vowel = Some(Sheva);
                        } else {
                            seq[i].vowel = None;
                        }
                    }
                }
                Suffix::Zero => {}
            }
            changed = true;
        }
        (_, Form::Imperfect, person, gender, number) => {
            // III-He imperfect, by suffix grade. The ending is uniform across
            // binyanim (build_strong has already set the stem's C1/doubling), so
            // we only rewrite the C2 + he tail:
            //   zero suffix (3ms/3fs/2ms/1cs/1cp): C2 segol, etymological he
            //     kept — yibnê (יִבְנֶה), tihyê (תִּהְיֶה), yakkê (יַכֶּה).
            //   vocalic suffix (-û 3mp/2mp, -î 2fs): the he elides and C2 loses
            //     its vowel so the suffix vowel carries the syllable — yibnû
            //     (יִבְנוּ), tibnî (תִּבְנִי), yakkû (יַכּוּ).
            //   heavy -nâ (3fp/2fp): special segol-yod ending, not yet modelled
            //     — left as the strong-form stub.
            let vocalic = matches!(
                (person, gender, number),
                (
                    Some(Person::Second),
                    Some(Gender::Feminine),
                    Some(Number::Singular)
                ) | (
                    Some(Person::Second | Person::Third),
                    Some(Gender::Masculine),
                    Some(Number::Plural)
                )
            );
            let heavy = number == Some(Number::Plural) && gender == Some(Gender::Feminine);
            // Hiphil interposes its î-mater (yod) between C2 and the he
            // (yakkîh → …); the III-He ending attaches straight to C2, so drop
            // any consonant sitting between them before rewriting the tail.
            if let (Some(c2), Some(c3)) = (c2_idx, c3_idx) {
                for i in (c2 + 1..c3).rev() {
                    seq.remove(i);
                }
            }
            let c3_idx = radical_idx(seq, 3);
            if vocalic {
                // The 2fs suffix is hiriq + yod mater, and build_strong wrote
                // that hiriq on the C3 slot — removing the he would silently
                // drop it (תַּעֲשׁי). Carry it over to C2: taʿăśî (תַּעֲשִׂי),
                // tiḇnî (תִּבְנִי). The plural suffixes (-û) are whole Cons
                // entries, so they survive the removal and C2 goes vowelless.
                let c3_vowel = c3_idx.and_then(|i| seq[i].vowel);
                if let Some(i) = c3_idx {
                    seq.remove(i);
                }
                if let Some(i) = c2_idx {
                    if matches!(binyan, Binyan::Piel | Binyan::Pual | Binyan::Hithpael)
                        && hebrew::is_guttural(seq[i].letter)
                    {
                        seq[i].vowel = Some(Sheva);
                    } else {
                        seq[i].vowel = c3_vowel;
                    }
                }
                changed = true;
            } else {
                // C2 takes segol in both the zero-suffix singular and the
                // heavy -nâ (3fp/2fp). The singular keeps the etymological he
                // (yibnê יִבְנֶה), while -nâ turns it into a yod mater before the
                // nun-qamats-he suffix: tiḇnênâ (תִּבְנֶינָה), tihyênâ (תִּהְיֶינָה).
                if let Some(i) = c2_idx {
                    seq[i].vowel = Some(Segol);
                }
                if heavy && let Some(i) = c3_idx {
                    seq[i] = Cons::mater(letter::YOD);
                }
                changed = true;
            }
        }
        (
            Binyan::Qal,
            Form::Imperative,
            Some(_),
            Some(Gender::Masculine),
            Some(Number::Singular),
        ) => {
            // bənê: C2 takes tsere.
            if let Some(i) = c2_idx {
                seq[i].vowel = Some(Tsere);
            }
            changed = true;
        }
        (_, Form::Imperative, _, Some(Gender::Masculine), Some(Number::Plural))
        | (_, Form::Imperative, _, Some(Gender::Feminine), Some(Number::Singular)) => {
            // III-He vocalic imperative (-û 2mp, -î 2fs): exactly like the
            // imperfect vocalic grade, the he elides and C2 loses its vowel so
            // the suffix vowel carries the syllable — bᵊnû בְּנוּ, ʿăśû עֲשׂוּ,
            // hôḏû הוֹדוּ, harbû הַרְבּוּ. Drop any interposed Hiphil î-mater
            // between C2 and the he first (cf. the imperfect arm).
            if let (Some(c2), Some(c3)) = (c2_idx, c3_idx) {
                for i in (c2 + 1..c3).rev() {
                    seq.remove(i);
                }
            }
            if let Some(i) = radical_idx(seq, 3) {
                // The he may itself carry the suffix vowel (the 2fs -î hiriq
                // sits on the he before its yod mater: עִשְׁהִי); hand it to C2
                // as the he elides. The 2mp -û (shureq vav) leaves the he
                // vowel-less, so C2 ends up bare, as it should — bᵊnû בְּנוּ.
                let he_vowel = seq[i].vowel.filter(|&v| v != Sheva);
                seq.remove(i);
                if let Some(j) = c2_idx {
                    seq[j].vowel = he_vowel;
                }
            }
            // The strong Qal plural imperative's C1 hiriq (qiṭlû) opens to a
            // vocal sheva once the III-He cluster collapses (bᵊnû, rᵊʿû), and
            // a begedkefet C2 after that vocal sheva spirantises.
            if binyan == Binyan::Qal {
                if let Some(i) = radical_idx(seq, 1)
                    && seq[i].vowel == Some(Hiriq) {
                        seq[i].vowel = Some(Sheva);
                    }
                if let Some(i) = c2_idx
                    && hebrew::is_begedkefet(seq[i].letter)
                {
                    seq[i].dagesh = false;
                }
            }
            changed = true;
        }
        (Binyan::Qal, Form::Jussive, person, gender, Some(Number::Singular))
            // The zero-suffix singular jussives — 3ms (yiḇen), 3fs and 2ms
            // (tiḇen) — all apocopate identically; the prefix consonant already
            // distinguishes them. The 2fs carries a vocalic -î suffix (tiḇnî)
            // and is handled by the imperfect path, so exclude it here.
            if !(person == Some(Person::Second) && gender == Some(Gender::Feminine)) =>
        {
            // Apocopated jussive yiḇen (יִבֶן): drop the final he, then break
            // the resulting C1-C2 word-final cluster with a helping vowel on C1
            // while C2 closes the syllable vowel-less. The helping vowel is a
            // segol, but a guttural C1 prefers an a-class vowel — yáʕaś
            // (וַיַּעַשׂ), yáʕal (וַיַּעַל).
            if let Some(i) = c3_idx {
                seq.remove(i);
            }
            if let Some(j) = radical_idx(seq, 2) {
                seq[j].vowel = None;
            }
            if let Some(j) = radical_idx(seq, 1) {
                seq[j].vowel = Some(if hebrew::is_guttural(seq[j].letter) {
                    Patah
                } else {
                    Segol
                });
            }
            changed = true;
        }
        (_, Form::Jussive, person, gender, Some(Number::Singular))
            // Derived-stem apocopated jussive (the short-wayyiqtol base): the
            // he drops and C2 closes the word — Piel yəṣaw (וַיְצַו), yəḵal
            // (וַיְכַל); Niphal yērāʾ (וַיֵּרָא); Hithpael yiṯgal (וַיִּתְגַּל);
            // Hiphil yáʿal (וַיַּעַל), yaḵ (וַיַּךְ). A word-final consonant
            // cannot carry the forte (yəṣaw, not yəṣaww), and a C1 left on a
            // vocal sheva/hataf takes a helping vowel — segol, or patah for a
            // guttural (yáʿal). The 2fs keeps its vocalic -î and is excluded.
            if !(person == Some(Person::Second) && gender == Some(Gender::Feminine)) =>
        {
            // Drop any interposed Hiphil î-mater between C2 and the he.
            if let (Some(c2), Some(c3)) = (c2_idx, c3_idx) {
                for i in (c2 + 1..c3).rev() {
                    seq.remove(i);
                }
            }
            if let Some(i) = radical_idx(seq, 3) {
                seq.remove(i);
            }
            if let Some(j) = radical_idx(seq, 2) {
                seq[j].vowel = None;
                seq[j].dagesh = false;
                if j > 0
                    && matches!(
                        seq[j - 1].vowel,
                        Some(Sheva | HatafPatah | HatafSegol | HatafQamats)
                    )
                {
                    let helping = if hebrew::is_guttural(seq[j - 1].letter) {
                        Patah
                    } else {
                        Segol
                    };
                    seq[j - 1].vowel = Some(helping);
                    // A segol helping vowel attenuates a preformative patah to
                    // segol too — wayyép̄en וַיֶּפֶן, וַיֶּרֶב; the guttural's
                    // patah keeps the prefix patah (וַיַּעַל).
                    if helping == Segol
                        && j >= 2
                        && seq[j - 2].vowel == Some(Patah)
                    {
                        seq[j - 2].vowel = Some(Segol);
                    }
                }
            }
            changed = true;
        }
        (_, Form::InfinitiveConstruct, _, _, _) => {
            // bənôt (בְּנוֹת), ʕăśôt (עֲשׂוֹת), and the same -ôṯ ending in the
            // derived stems — Piel ṣawwôṯ (צַוּוֹת), Hiphil haʿălôṯ
            // (הַעֲלוֹת): the etymological he becomes a tav, and the linking
            // vowel is a plene holam written on a vav mater — so C2 itself
            // carries no vowel and a vav-holam is inserted before the tav.
            // Drop any interposed Hiphil î-mater between C2 and the he first.
            if let (Some(c2), Some(c3)) = (c2_idx, c3_idx) {
                for i in (c2 + 1..c3).rev() {
                    seq.remove(i);
                }
            }
            if let Some(i) = radical_idx(seq, 2) {
                seq[i].vowel = None;
                // A begedkefet C2 opening the final syllable after a closed
                // (silent-sheva) one is plosive — harbôṯ הַרְבּוֹת.
                if hebrew::is_begedkefet(seq[i].letter)
                    && i > 0
                    && seq[i - 1].vowel == Some(Sheva)
                {
                    seq[i].dagesh = true;
                }
            }
            if let Some(i) = radical_idx(seq, 3) {
                seq[i] = Cons::new(letter::TAV);
                seq.insert(i, Cons::new(letter::VAV).with_vowel(Holam));
            }
            changed = true;
        }
        (Binyan::Qal, Form::ParticiplePassive, _, gender, number) => {
            // III-He Qal passive participle: the etymological III-yod resurfaces
            // as a consonantal yod where the strong qāṭûl builder left the mater
            // he (build_participle put the shureq vav before it):
            //   ms nāṭûy   (נָטוּי)  — C3 he → bare yod.
            //   fs nəṭûyâ  (נְטוּיָה) — C3 he (qamats) → yod-qamats; the -â he kept.
            //   mp nəṭûyîm (נְטוּיִם) — C3 he (hiriq) → yod-hiriq; the -îm mater yod drops.
            //   fp nəṭûyôt (נְטוּיוֹת) — C3 he → bare yod; holam-vav + tav follow.
            if let Some(i) = c3_idx {
                match (gender, number) {
                    (Some(Gender::Masculine), Some(Number::Singular)) => {
                        seq[i] = Cons::new(letter::YOD);
                    }
                    (Some(Gender::Feminine), Some(Number::Singular)) => {
                        seq[i] = Cons::new(letter::YOD).with_vowel(Qamats);
                    }
                    (Some(Gender::Masculine), Some(Number::Plural)) => {
                        seq[i] = Cons::new(letter::YOD).with_vowel(Hiriq);
                        if i + 1 < seq.len() && seq[i + 1].letter == letter::YOD {
                            seq.remove(i + 1);
                        }
                    }
                    (Some(Gender::Feminine), Some(Number::Plural)) => {
                        seq[i] = Cons::new(letter::YOD);
                        seq.insert(i + 1, Cons::new(letter::VAV).with_vowel(Holam));
                    }
                    _ => {}
                }
            }
            changed = true;
        }
        (_, Form::ParticipleActive, _, gender, number) => {
            // III-He active participle qōṭeh (qōṭ-e + etymological he), the same
            // ending across binyanim (the binyan's prefix/doubling is already set
            // by build_participle, so only the C2 + he tail is rewritten here —
            // Qal ʿōśeh עֹשֶׂה, Piel məṣawweh מְצַוֶּה, Hiphil maʿăleh מַעֲלֶה):
            //   ms ʿōśeh  (עֹשֶׂה) — C2 segol, he kept.
            //   fs ʿōśâ   (עֹשָׂה) — C2 qamats, he kept; the strong builder
            //              appended a segolate -t, so drop everything past the he.
            //   mp ʿōśîm  (עֹשִׂים) — he elides, C2 hiriq + yod mater.
            //   fp ʿōśôt  (עֹשׂוֹת) — he elides, plene holam-vav before the tav.
            // Hiphil interposes an î-mater (yod) between C2 and the he; drop any
            // such interposed consonant before rewriting (mirrors the imperfect
            // and perfect branches above).
            if let (Some(c2), Some(c3)) = (c2_idx, c3_idx) {
                for i in (c2 + 1..c3).rev() {
                    seq.remove(i);
                }
            }
            let c3_idx = radical_idx(seq, 3);
            match (gender, number) {
                (Some(Gender::Masculine), Some(Number::Singular)) => {
                    if let Some(i) = c2_idx {
                        seq[i].vowel = Some(Segol);
                    }
                }
                (Some(Gender::Feminine), Some(Number::Singular)) => {
                    if let Some(i) = c2_idx {
                        seq[i].vowel = Some(Qamats);
                    }
                    if let Some(i) = c3_idx {
                        seq[i].vowel = None;
                        seq.truncate(i + 1);
                    }
                }
                (Some(Gender::Masculine), Some(Number::Plural)) => {
                    if let Some(i) = c3_idx {
                        seq.remove(i);
                    }
                    if let Some(i) = c2_idx {
                        seq[i].vowel = Some(Hiriq);
                    }
                }
                (Some(Gender::Feminine), Some(Number::Plural)) => {
                    if let Some(i) = c3_idx {
                        seq[i] = Cons::new(letter::VAV).with_vowel(Holam);
                    }
                    if let Some(i) = c2_idx {
                        seq[i].vowel = None;
                    }
                }
                _ => {}
            }
            changed = true;
        }
        _ => {
            // Forms not yet specialised — leave strong-form output.
        }
    }
    changed
}

fn apply_lamed_aleph(seq: &mut [Cons], root: &Root, binyan: Binyan, form: Form, pgn: Pgn) -> bool {
    // III-Aleph: the alef quiesces. In Qal Perfect 3ms (māṣāʔ → מָצָא),
    // the patah on C2 lengthens to qamats. In forms with consonantal
    // suffix the alef vowel "carries" before -tî: māṣā(ʔ)ṯî.
    let mut changed = false;
    if root.lamed() != letter::ALEF {
        return false;
    }
    use Vowel::*;
    if let (
        Binyan::Qal,
        Form::Perfect,
        Some(Person::Third),
        Some(Gender::Masculine),
        Some(Number::Singular),
    ) = (binyan, form, pgn.person, pgn.gender, pgn.number)
    {
        // māṣāʔ: change C2's vowel from patah to qamats.
        if let Some(i) = radical_idx(seq, 2) {
            seq[i].vowel = Some(Qamats);
        }
        changed = true;
    }
    // Qal Perfect with a consonantal suffix (1cs/1cp, all 2nd person): the
    // quiescent alef loses its sheva and the C2 patah lengthens to qamats —
    // māṣāʔtî (מָצָאתִי), māṣāʔnû (מָצָאנוּ), məṣāʔtem (מְצָאתֶם). With the alef
    // now closing an open (long-vowel) syllable, the begedkefet suffix loses
    // its dagesh lene.
    if let (Binyan::Qal, Form::Perfect) = (binyan, form)
        && matches!(pgn.person, Some(Person::First | Person::Second))
    {
        if let Some(c2) = radical_idx(seq, 2)
            && seq[c2].vowel == Some(Patah)
        {
            seq[c2].vowel = Some(Qamats);
            changed = true;
        }
        if let Some(c3) = radical_idx(seq, 3) {
            seq[c3].vowel = None;
            if c3 + 1 < seq.len() && seq[c3 + 1].dagesh {
                seq[c3 + 1].dagesh = false;
            }
            changed = true;
        }
    }
    // Hiphil consonantal-suffix perfect of a III-aleph root: the theme tsere is
    // already in place (hēḇē-, hôṣē-); the quiescent alef just drops its sheva
    // and the begedkefet suffix loses its dagesh lene — hēḇēʔtî (הֵבֵאתִי),
    // hôṣēʔtî (הוֹצֵאתִי).
    if let (Binyan::Hiphil, Form::Perfect) = (binyan, form)
        && perfect_suffix_kind(pgn) == Suffix::Consonantal
        && let Some(c3) = radical_idx(seq, 3)
    {
        // The theme consonant before the alef carries tsere (hē-ḇē-, hô-ṣē-);
        // for the I-yod class the strong stem leaves it patah, so set it.
        if c3 > 0 {
            seq[c3 - 1].vowel = Some(Tsere);
        }
        seq[c3].vowel = None;
        if c3 + 1 < seq.len() && seq[c3 + 1].dagesh {
            seq[c3 + 1].dagesh = false;
        }
        changed = true;
    }
    // Qal Imperfect family + imperative: the quiescent alef lengthens the
    // thematic holam to qamats — yimṣāʔ (יִמְצָא), yiqrāʔ (יִקְרָא), and likewise
    // the ms imperative qᵉrāʔ (קְרָא), məṣāʔ (מְצָא). Only the long grade carries
    // the holam; a vocalic suffix already reduces C2 to sheva (yimṣəʔû,
    // יִמְצְאוּ), where the alef simply quiesces, so leave it untouched. The
    // infinitive construct keeps its holam (liqrōʔ לִקְרֹא) and is excluded.
    if matches!(
        (binyan, form),
        (
            Binyan::Qal,
            Form::Imperfect
                | Form::Jussive
                | Form::Cohortative
                | Form::Wayyiqtol
                | Form::Imperative
        )
    ) && let Some(i) = radical_idx(seq, 2)
        && seq[i].vowel == Some(Holam)
    {
        seq[i].vowel = Some(Qamats);
        changed = true;
    }
    // Any binyan, imperfect family + perfect: a patah immediately before a
    // word-final quiescent alef lengthens to qamats — the alef can't close the
    // syllable. Covers the hollow Hophal (yûḇāʾ יוּבָא, not יוּבַא), the
    // patah-theme derived imperfects generally, and the derived-stem perfect 3ms
    // (Pual qōrāʾ קֹרָא, not קֹרַא). A consonantal suffix moves the alef off the
    // word edge, so the guard leaves those (qōraʾtā קֹרַאְתָּ) untouched.
    if matches!(
        form,
        Form::Imperfect | Form::Jussive | Form::Cohortative | Form::Wayyiqtol | Form::Perfect
    ) && let Some(last) = seq.last()
        && last.letter == letter::ALEF
        && last.vowel.is_none()
        && seq.len() >= 2
    {
        let i = seq.len() - 2;
        if seq[i].vowel == Some(Patah) {
            seq[i].vowel = Some(Qamats);
            changed = true;
        }
    }
    changed
}

fn apply_pe_aleph(seq: &mut Vec<Cons>, root: &Root, binyan: Binyan, form: Form, _pgn: Pgn) -> bool {
    use Vowel::*;
    if binyan != Binyan::Qal {
        return false;
    }
    // Pe-Aleph imperative / infinitive construct: the word-initial alef takes a
    // hataf-segol rather than the hataf-patah apply_guttural would supply —
    // ʔĕmōr (אֱמֹר), ʔĕḵōl (אֱכֹל), ʔĕsōp̄ (אֱסֹף). Set it before apply_guttural
    // runs so it isn't overwritten.
    if matches!(form, Form::Imperative | Form::InfinitiveConstruct) {
        if let Some(c1) = radical_idx(seq, 1)
            && seq[c1].letter == letter::ALEF
            && seq[c1].vowel == Some(Sheva)
        {
            seq[c1].vowel = Some(HatafSegol);
            return true;
        }
        return false;
    }
    // Pe-Aleph in Qal Imperfect. Only five high-frequency verbs take the
    // contracted yōʔ- pattern: yōʔḵal (יֹאכַל). Every other I-aleph verb behaves
    // like a regular I-guttural with segol-class vowels — yeʔĕsōp̄ (יֶאֱסֹף): the
    // prefix hiriq lowers to segol and the aleph takes a hataf-segol.
    if !matches!(form, Form::Imperfect | Form::Cohortative | Form::Jussive) {
        return false;
    }
    const YO_ROOTS: [[char; 3]; 5] = [
        [letter::ALEF, letter::BET, letter::DALET], // אבד
        [letter::ALEF, letter::BET, letter::HE],    // אבה
        [letter::ALEF, letter::KAF, letter::LAMED], // אכל
        [letter::ALEF, letter::MEM, letter::RESH],  // אמר
        [letter::ALEF, letter::PE, letter::HE],     // אפה
    ];
    if !YO_ROOTS.contains(&root.letters) {
        // The 1cs aleph preformative already carries segol (אֶאֱסֹף); for the
        // other persons the prefix hiriq lowers to segol. In both, the C1 aleph
        // takes a hataf-segol opening the next syllable.
        let mut changed = false;
        let is_1cs = _pgn.person == Some(Person::First) && _pgn.number == Some(Number::Singular);
        if !is_1cs
            && let Some(first) = seq.first_mut()
            && first.vowel == Some(Hiriq)
        {
            first.vowel = Some(Segol);
            changed = true;
        }
        if let Some(i) = radical_idx(seq, 1)
            && seq[i].letter == letter::ALEF
            && seq[i].vowel == Some(Sheva)
        {
            seq[i].vowel = Some(HatafSegol);
            changed = true;
        }
        return changed;
    }
    let mut changed = false;
    // C2 takes patah, but only in the long grade (zero / -nâ suffixes, where
    // the strong stem has holam under C2). A vocalic suffix reduces C2 to
    // sheva (yōʔmərû, יֹאמְרוּ); don't clobber that.
    if let Some(c2_idx) = radical_idx(seq, 2)
        && seq[c2_idx].vowel == Some(Holam)
    {
        seq[c2_idx].vowel = Some(Patah);
    }
    // C1 alef quiesces. Prefix vowel → holam. C2 takes patah.
    if let Some(c1_idx) = radical_idx(seq, 1) {
        let prefix_aleph = c1_idx > 0 && seq[c1_idx - 1].letter == letter::ALEF;
        if c1_idx > 0 {
            seq[c1_idx - 1].vowel = Some(Holam);
            changed = true;
        }
        seq[c1_idx].vowel = None;
        // 1cs: the aleph preformative and the quiescent root aleph contract
        // into a single aleph (ʾeʾ-mar → ʾōmar): אֹמַר, not אֹאמַר.
        if prefix_aleph {
            seq.remove(c1_idx);
            changed = true;
        }
    }
    changed
}

fn apply_lamed_guttural(
    seq: &mut [Cons],
    root: &Root,
    binyan: Binyan,
    form: Form,
    _pgn: Pgn,
) -> bool {
    // III-Guttural (C3 = ח/ע).
    if !matches!(root.lamed(), letter::HET | letter::AYIN) {
        return false;
    }
    match (binyan, form) {
        (Binyan::Qal, Form::Perfect) => {
            // Qal perfect with a vocalic suffix (3fs 3cp): the sheva under C2
            // becomes qamats before a guttural C3 — יָדָעוּ not יָדְעוּ,
            // שָׁמָעָה not שָׁמְעָה.
            if let Some(i) = radical_idx(seq, 2)
                && seq[i].vowel == Some(Vowel::Sheva)
            {
                seq[i].vowel = Some(Vowel::Qamats);
                return true;
            }
            false
        }
        (Binyan::Qal, Form::Imperfect | Form::Jussive | Form::Cohortative | Form::Wayyiqtol) => {
            // In the Qal Imperfect the thematic vowel under C2 lowers from
            // holam to patah because a guttural prefers an a-class vowel
            // before it: yišlaḥ (יִשְׁלַח), yišmaʕ (יִשְׁמַע). Only the long
            // grade carries the holam; a vocalic suffix already reduces C2
            // to sheva (yišləḥû), so we leave that alone.
            if let Some(i) = radical_idx(seq, 2)
                && seq[i].vowel == Some(Vowel::Holam)
            {
                seq[i].vowel = Some(Vowel::Patah);
                return true;
            }
            false
        }
        _ => false,
    }
}

/// Insert a furtive patah under a word-final guttural (ח/ע) when the preceding
/// vowel is a long, non-a vowel. The guttural must close the word vowelless;
/// the glide patah is rendered directly on it (שֹׁמֵעַ, שְׁמֹעַ). The preceding
/// vowel may sit on a consonant (tsere/holam/hiriq) or on a vav/yod mater
/// carrying holam/shureq (inf. abs. שָׁמוֹעַ).
fn apply_furtive_patah(seq: &mut [Cons]) -> bool {
    let n = seq.len();
    if n < 2 {
        return false;
    }
    let last = &seq[n - 1];
    if !matches!(last.letter, letter::HET | letter::AYIN) || last.vowel.is_some() {
        return false;
    }
    let prev = &seq[n - 2];
    let prev_long = matches!(prev.vowel, Some(Vowel::Tsere | Vowel::Holam | Vowel::Hiriq))
        || (prev.letter == letter::VAV && prev.vowel.is_none() && prev.dagesh);
    if !prev_long {
        return false;
    }
    seq[n - 1].vowel = Some(Vowel::Patah);
    true
}

fn apply_pe_guttural(seq: &mut [Cons], root: &Root, binyan: Binyan, form: Form, _pgn: Pgn) -> bool {
    // I-Guttural (C1 = א/ה/ח/ע). In the Qal Imperfect family the hiriq prefix
    // vowel lowers to patah and the guttural takes a hataf-patah (added by
    // [`apply_guttural`]): yaʕămōd (יַעֲמֹד), yaḥăzōq (יַחֲזֹק).
    if !hebrew::is_guttural(root.pe()) {
        return false;
    }
    // Hiphil/Niphal perfect with a guttural C1: the hi-/ni- prefix hiriq lowers
    // to segol and the guttural takes a hataf-segol opening the next syllable —
    // heḥĕzîq (הֶחֱזִיק), heʕĕmîd (הֶעֱמִיד), neḥĕzaq (נֶחֱזַק). (The Hiphil
    // imperfect instead patterns with the Qal: yaḥăzîq.)
    if matches!(
        (binyan, form),
        (
            Binyan::Hiphil | Binyan::Niphal,
            Form::Perfect | Form::ParticipleActive | Form::ParticiplePassive
        )
    ) {
        let mut changed = false;
        if let Some(first) = seq.first_mut()
            && matches!(first.vowel, Some(Vowel::Hiriq) | Some(Vowel::Patah))
            && (binyan != Binyan::Hiphil || form == Form::Perfect)
        {
            first.vowel = Some(Vowel::Segol);
            changed = true;
        }
        // The C1 hataf echoes the preformative: the segol prefixes (he-/ne-
        // perfect, Niphal participle ne-) take hataf-segol — heḥĕzîq, neʔĕmān —
        // but the Hiphil participle keeps its patah ma- preformative, so its
        // guttural takes hataf-patah: maḥărîd (מַחֲרִיד), maʕămîd (מַעֲמִיד).
        let prefix_patah = seq.first().is_some_and(|c| c.vowel == Some(Vowel::Patah));
        if let Some(i) = radical_idx(seq, 1)
            && seq[i].vowel == Some(Vowel::Sheva)
        {
            seq[i].vowel = Some(if prefix_patah {
                Vowel::HatafPatah
            } else {
                Vowel::HatafSegol
            });
            changed = true;
            // The hataf opens the C1 syllable, so a begedkefet C2 spirantizes —
            // heʕĕḇîr הֶעֱבִיר not הֶעֱבִּיר (contrast strong hikbîd, where C1's
            // silent sheva closes the syllable and the dagesh lene stays).
            if let Some(j) = radical_idx(seq, 2)
                && seq[j].dagesh
                && hebrew::is_begedkefet(seq[j].letter)
            {
                seq[j].dagesh = false;
            }
        }
        return changed;
    }
    // Hiphil imperfect family with a guttural C1: like the Qal, the patah
    // preformative opens onto the guttural, which takes a hataf-patah —
    // yaʕămîd (יַעֲמִיד), wayyaʕămēd (וַיַּעֲמֵד), yaḥăzîq (יַחֲזִיק). The prefix
    // is already patah (see ImperfectPrefix for Hiphil), so we only voice the
    // C1 sheva. The 1cs alef-prefix carries its own segol; keep the parallel
    // hataf there too.
    if matches!(
        (binyan, form),
        (
            Binyan::Hiphil,
            Form::Imperfect
                | Form::Jussive
                | Form::Cohortative
                | Form::Wayyiqtol
                | Form::Imperative
                | Form::ParticipleActive
                // The ha- infinitives pattern the same way: haʿălôṯ
                // (הַעֲלוֹת), haʿăḇîr (לְהַעֲבִיר).
                | Form::InfinitiveConstruct
                | Form::InfinitiveAbsolute
        )
    ) {
        if let Some(i) = radical_idx(seq, 1)
            && seq[i].vowel == Some(Vowel::Sheva)
        {
            seq[i].vowel = Some(Vowel::HatafPatah);
            return true;
        }
        return false;
    }
    if !matches!(
        (binyan, form),
        (
            Binyan::Qal,
            Form::Imperfect | Form::Jussive | Form::Cohortative | Form::Wayyiqtol
        )
    ) {
        return false;
    }
    if let Some(first) = seq.first_mut() {
        if first.vowel == Some(Vowel::Hiriq) {
            first.vowel = Some(Vowel::Patah);
            if let Some(i) = radical_idx(seq, 1)
                && seq[i].vowel == Some(Vowel::Sheva)
            {
                seq[i].vowel = Some(Vowel::HatafPatah);
            }
            return true;
        } else if first.vowel == Some(Vowel::Segol) {
            // Segol preformative (1cs, or stative yeḥĕzaq): C1 takes hataf-segol.
            if let Some(i) = radical_idx(seq, 1)
                && seq[i].vowel == Some(Vowel::Sheva)
            {
                seq[i].vowel = Some(Vowel::HatafSegol);
                return true;
            }
        }
    }
    false
}

/// Whether a vowel is inherently long (an open syllable before a sheva makes
/// that sheva vocal). Hataf and the short vowels keep a following sheva silent.
fn is_long_vowel(v: Vowel) -> bool {
    matches!(v, Vowel::Qamats | Vowel::Tsere | Vowel::Holam)
}

/// Maqaf-shortening alternant: when a word ends in a closed syllable whose
/// vowel is a (defective) tsere — tav-tsere + a real final consonant, as in
/// yittēn (יִתֵּן) — that tsere shortens to segol once the word is joined to the
/// following word by maqaf (יִתֶּן־). Returns the segol spelling, or `None` when
/// the shape doesn't apply. A word-final mater (he/aleph/yod/vav) means the
/// tsere is open/plene (bənê בְּנֵה) and stable, so those are excluded.
fn maqaf_segol_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 2 {
        return None;
    }
    let last = &seq[n - 1];
    // The final consonant must close the syllable: either bare or carrying a
    // silent sheva (the latter for begedkefet finals like the kaf of וַיְבָרֵךְ).
    if !(last.vowel.is_none() || last.vowel == Some(Vowel::Sheva))
        || matches!(
            last.letter,
            letter::HE | letter::ALEF | letter::YOD | letter::VAV
        )
    {
        return None;
    }
    if seq[n - 2].vowel != Some(Vowel::Tsere) {
        return None;
    }
    seq[n - 2].vowel = Some(Vowel::Segol);
    Some(hebrew::render(&seq))
}

/// Paragogic-nun twin of a vocalic-suffix imperfect: the long/energic imperfect
/// appends a nun to the plural -û / 2fs -î ending — tōʔmᵊrûn (תֹּאמְרוּן),
/// yᵊšûḇûn (יְשׁוּבוּן), taʕăśûn (תַּעֲשׂוּן), tiqṭᵊlîn. Re-rendered so the nun takes
/// its word-final form. The -ûn spelling differs from the bare -û, so this only
/// adds recall.
fn paragogic_nun_variant(text: &str) -> String {
    let mut seq = hebrew::parse_pointed(text);
    seq.push(Cons::new(letter::NUN));
    hebrew::render(&seq)
}

/// Theme-restored paragogic-nun twins of a vocalic-suffix (-û) imperfect. The
/// bare plural reduces the theme vowel to a vocal sheva (yišmᵊʕû, yēlᵊḵû,
/// tōʔḇᵊḏû); the energic -ûn form restores it to its full grade — qamats for the
/// a-theme/guttural verbs (yišmāʕûn תִּשְׁמָעוּן, yiškāḇûn, yḥpāṣûn) or tsere for
/// the e-theme (tōʔḇēḏûn תֹּאבֵדוּן, yēlēḵûn יֵלֵכוּן). The consonant before C3
/// (which precedes the final vav-shureq) carries the restored vowel. Emits both
/// the qamats and tsere twins; only the attested grade matches a real surface.
fn paragogic_nun_theme_variants(text: &str) -> Vec<String> {
    let seq = hebrew::parse_pointed(text);
    let n = seq.len();
    // Need …[restore-consonant + Sheva][C3 vowelless][vav-shureq]. The -û plural
    // ends in a vav carrying the shureq dagesh with no vowel of its own. When the
    // restore-consonant is a guttural it bears a hataf rather than a vocal sheva
    // in the bare plural (yišʾălû יִשְׁאֲלוּ), so accept that too — the energic
    // grade restores it to qamats/tsere just the same (yišʾālûn יִשְׁאָלוּן).
    let restore_ok = seq.get(n.wrapping_sub(3)).is_some_and(|c| {
        c.vowel == Some(Vowel::Sheva)
            || (hebrew::is_guttural(c.letter)
                && matches!(
                    c.vowel,
                    Some(Vowel::HatafPatah | Vowel::HatafSegol | Vowel::HatafQamats)
                ))
    });
    if n < 3
        || !(seq[n - 1].letter == letter::VAV && seq[n - 1].dagesh && seq[n - 1].vowel.is_none())
        || seq[n - 2].vowel.is_some()
        || !restore_ok
    {
        return Vec::new();
    }
    [Vowel::Qamats, Vowel::Tsere]
        .into_iter()
        .map(|theme| {
            let mut s = seq.clone();
            s[n - 3].vowel = Some(theme);
            s.push(Cons::new(letter::NUN));
            hebrew::render(&s)
        })
        .collect()
}

/// Pausal twin of a Qal o-theme imperative / infinitive construct: in pause the
/// theme holam on C2 lengthens to qamats — ʾĕmōr אֱמֹר → ʾĕmār אֱמָר, ʾĕḵōl →
/// ʾĕḵāl (לֶאֱכָל), šᵊmōr → šᵊmār. Changes the penultimate consonant's holam to
/// qamats when the last consonant is a true final. Additive.
fn pausal_qotol_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 2 || seq[n - 2].vowel != Some(Vowel::Holam) {
        return None;
    }
    let last = &seq[n - 1];
    if !(last.vowel.is_none() || last.vowel == Some(Vowel::Sheva))
        || matches!(last.letter, letter::HE | letter::VAV | letter::YOD)
    {
        return None;
    }
    seq[n - 2].vowel = Some(Vowel::Qamats);
    Some(hebrew::render(&seq))
}

/// Reduced-plural twin of a I-guttural Qal o-theme imperfect. The strong plural
/// reduces the holam theme to a vocal sheva (yišmōr → yišmᵊrû), but the
/// I-guttural builder keeps it (yaḥpōrû יַחְפֹּרוּ); the reduced spelling yaḥpᵊrû
/// (יַחְפְּרוּ, וְיַחְפְּרוּ) also occurs. Reduces the holam on the consonant before
/// C3 to sheva. Additive.
fn iguttural_reduced_plural_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    // [prefix] C1-guttural(patah/hataf) C2(sheva) … VAV(shureq): the guttural
    // closes the prefix syllable on a silent sheva and a begedkefet C2 takes a
    // dagesh lene — yaḥap̄rû יַחַפְרוּ → yaḥpᵊrû יַחְפְּרוּ.
    if n < 4
        || !(seq[n - 1].letter == letter::VAV && seq[n - 1].dagesh && seq[n - 1].vowel.is_none())
        || !hebrew::is_guttural(seq[1].letter)
        || !matches!(
            seq[1].vowel,
            Some(Vowel::Patah | Vowel::HatafPatah | Vowel::HatafSegol)
        )
        || seq[2].vowel != Some(Vowel::Sheva)
    {
        return None;
    }
    seq[1].vowel = Some(Vowel::Sheva);
    if hebrew::is_begedkefet(seq[2].letter) {
        seq[2].dagesh = true;
    }
    Some(hebrew::render(&seq))
}

/// Pausal twin of a vocalic-suffix (-û) imperfect: in pause the theme vowel the
/// bare plural reduced to sheva is restored to its full grade on the consonant
/// before C3 — qamats (yišmāʕû יִשְׁמָעוּ, yiqrāḇû) or tsere — exactly the grade
/// the energic -ûn restores, but with no nun. Returns both twins; only the
/// attested grade matches. (Same shape test as [`paragogic_nun_theme_variants`].)
fn pausal_imperfect_plural_variants(text: &str) -> Vec<String> {
    let seq = hebrew::parse_pointed(text);
    let n = seq.len();
    // An undageshable theme consonant carries a hataf instead of the sheva
    // (yišḥăṭû יִשְׁחֲטוּ); it restores to the same full grade (יִשְׁחָטוּ).
    if n < 3
        || !(seq[n - 1].letter == letter::VAV && seq[n - 1].dagesh && seq[n - 1].vowel.is_none())
        || seq[n - 2].vowel.is_some()
        || !matches!(
            seq[n - 3].vowel,
            Some(Vowel::Sheva | Vowel::HatafPatah | Vowel::HatafSegol)
        )
    {
        return Vec::new();
    }
    [Vowel::Qamats, Vowel::Tsere, Vowel::Holam]
        .into_iter()
        .map(|theme| {
            let mut s = seq.clone();
            s[n - 3].vowel = Some(theme);
            hebrew::render(&s)
        })
        .collect()
}

/// Paragogic-nun twin of a hollow Qal vocalic-suffix imperfect, with the
/// propretonic prefix reduction the long ending forces. In the bare plural the
/// open prefix syllable is pretonic and keeps its long qamats (yāšûḇû יָשׁוּבוּ);
/// appending the stressed -ûn pushes the prefix into the propretonic position,
/// where the open-syllable qamats reduces to a vocal sheva: yᵉšûḇûn (יְשׁוּבוּן),
/// yᵉqûmûn (יְקוּמוּן). Returns the reduced-prefix form, or `None` when the
/// preformative isn't an open qamats syllable. Additive — the sheva spelling
/// differs from the plain qamats append in [`paragogic_nun_variant`].
fn hollow_paragogic_nun_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let pre = seq.first()?;
    if !matches!(
        pre.letter,
        letter::YOD | letter::TAV | letter::ALEF | letter::NUN
    ) || pre.vowel != Some(Vowel::Qamats)
    {
        return None;
    }
    seq[0].vowel = Some(Vowel::Sheva);
    seq.push(Cons::new(letter::NUN));
    Some(hebrew::render(&seq))
}

/// Retained-yod twin of a I-yod Qal imperfect/wayyiqtol. The "true" I-yod roots
/// (ירש, ירא, יבש, יקץ) do not elide the radical yod the way the I-waw class
/// does (ישב → yēšēḇ); instead the preformative takes hiriq and the radical yod
/// surfaces as a mater: yîraš (יִירַשׁ), yîrᵊšû (יִירְשׁוּ), wayyîraš (וַיִּירַשׁ).
/// The base builder produces only the elided tsere form (yēraš יֵרַשׁ), so this
/// rewrites the preformative tsere to hiriq and inserts the yod mater after it.
/// The preformative is `seq[0]`, or `seq[1]` past a wayyiqtol vav. Additive: the
/// hiriq+yod spelling differs from the tsere one, so only the attested form
/// matches.
fn pe_yod_retained_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    // Locate the preformative: skip a leading vav-consecutive.
    let p = if seq.first().is_some_and(|c| c.letter == letter::VAV) && seq.len() > 1 {
        1
    } else {
        0
    };
    let pre = seq.get(p)?;
    if !matches!(
        pre.letter,
        letter::YOD | letter::TAV | letter::ALEF | letter::NUN
    ) || pre.vowel != Some(Vowel::Tsere)
    {
        return None;
    }
    seq[p].vowel = Some(Vowel::Hiriq);
    seq.insert(p + 1, Cons::mater(letter::YOD));
    Some(hebrew::render(&seq))
}

/// Defective (mater-less) twin of the retained-yod I-yod imperfect: the
/// preformative hiriq written without the radical yod mater — wayyîrāʾ
/// וַיִּירָא beside the defective וַיִּרָא (ירא), yîraš beside יִרַשׁ. Two
/// input shapes reach here: the elided tsere base (yēraš — rewrite tsere to
/// hiriq, as in `pe_yod_retained_variant` minus the mater), and a base the
/// builder already spells retained-plene (wayyîrāʾ — strip the yod mater
/// after the hiriq preformative). Additive.
fn pe_yod_retained_defective_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let p = if seq.first().is_some_and(|c| c.letter == letter::VAV) && seq.len() > 1 {
        1
    } else {
        0
    };
    let pre = seq.get(p)?;
    if !matches!(
        pre.letter,
        letter::YOD | letter::TAV | letter::ALEF | letter::NUN
    ) {
        return None;
    }
    match pre.vowel {
        Some(Vowel::Tsere) => {
            seq[p].vowel = Some(Vowel::Hiriq);
        }
        Some(Vowel::Hiriq)
            if seq
                .get(p + 1)
                .is_some_and(|c| c.letter == letter::YOD && c.vowel.is_none() && !c.dagesh) =>
        {
            seq.remove(p + 1);
        }
        _ => return None,
    }
    Some(hebrew::render(&seq))
}

/// Tsere twin of the III-He consonantal-suffix perfect's linking vowel: the
/// hiriq-yod the base builder writes before the afformative (ṣiwwîṯî
/// צִוִּיתִי) is also attested as tsere-yod — ṣiwwêṯî (צִוֵּיתִי), ṣuwwêṯî
/// (צֻוֵּיתִי). Rewrites the hiriq that sits immediately before a yod mater +
/// tav afformative. Additive; gated to the III-He Piel/Pual consonantal
/// perfect by the caller.
fn lamed_he_perfect_tsere_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    // The linking vowel is the hiriq whose yod mater is followed by a
    // consonantal afformative — tav (-tî/-tā/-tem) or nun (1cp -nû). Skip
    // C1's own hiriq and the 1cs -tî tail.
    let i = (0..seq.len().saturating_sub(2)).find(|&i| {
        seq[i].vowel == Some(Vowel::Hiriq)
            && seq[i + 1].letter == letter::YOD
            && seq[i + 1].vowel.is_none()
            && matches!(seq[i + 2].letter, letter::TAV | letter::NUN)
    })?;
    seq[i].vowel = Some(Vowel::Tsere);
    Some(hebrew::render(&seq))
}

/// The reverse twin for the Hiphil: its III-He consonantal-suffix linking
/// vowel is built as tsere-yod (הִכֵּיתָ) but the hiriq-yod spelling is
/// attested too — hikkîṯā הִכִּיתָ, הִגְלִיתָ. Additive.
fn lamed_he_perfect_hiriq_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let i = (0..seq.len().saturating_sub(2)).find(|&i| {
        seq[i].vowel == Some(Vowel::Tsere)
            && seq[i + 1].letter == letter::YOD
            && seq[i + 1].vowel.is_none()
            && matches!(seq[i + 2].letter, letter::TAV | letter::NUN)
    })?;
    seq[i].vowel = Some(Vowel::Hiriq);
    Some(hebrew::render(&seq))
}

/// Hiriq-theme twin of the I-yod Qal perfect before a heavy afformative:
/// yᵊraštem יְרַשְׁתֶּם beside the attested i-grade yᵊrištem (וִירִשְׁתֶּם — the
/// conjunction sandhi וִי- is peeled by the parser, the bare stem matches
/// here). C2's patah → hiriq; gated to the I-yod 2mp/2fp perfect by the
/// caller. Additive.
fn pe_yod_perfect_heavy_hiriq_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    if seq.first().map(|c| (c.letter, c.vowel)) != Some((letter::YOD, Some(Vowel::Sheva)))
        || seq.get(1).and_then(|c| c.vowel) != Some(Vowel::Patah)
    {
        return None;
    }
    seq[1].vowel = Some(Vowel::Hiriq);
    Some(hebrew::render(&seq))
}

/// Reduce a hollow Hiphil imperfect-family form's prefix syllable for use as
/// an object-suffix host: the qamats preformative (yā-/wayyā-) drops to a
/// vocal sheva and loses the consecutive-vav doubling — וַיָּבִיאוּ →
/// וַיְבִיאוּ. Returns `None` when the preformative isn't a qamats syllable.
fn hollow_hiphil_reduced_prefix(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let p = if seq.first().is_some_and(|c| c.letter == letter::VAV) && seq.len() > 1 {
        1
    } else {
        0
    };
    if seq.get(p)?.vowel != Some(Vowel::Qamats) {
        return None;
    }
    seq[p].vowel = Some(Vowel::Sheva);
    seq[p].dagesh = false;
    Some(hebrew::render(&seq))
}

/// Rebuild the long î stem of a short (jussive-grade) hollow Hiphil wayyiqtol
/// and reduce its prefix, producing the object-suffix host: וַיָּבֵא →
/// וַיְבִיא (which then takes the suffix as wayḇîʾēnî וַיְבִיאֵנִי). The short
/// theme tsere (or its nesiga segol) returns to hiriq + yod mater and the
/// preformative qamats reduces as in [`hollow_hiphil_reduced_prefix`].
fn hollow_hiphil_reduced_long_base(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let p = if seq.first().is_some_and(|c| c.letter == letter::VAV) && seq.len() > 1 {
        1
    } else {
        0
    };
    if seq.get(p)?.vowel != Some(Vowel::Qamats) {
        return None;
    }
    let c1 = p + 1;
    if !matches!(seq.get(c1)?.vowel, Some(Vowel::Tsere | Vowel::Segol)) {
        return None;
    }
    seq[p].vowel = Some(Vowel::Sheva);
    seq[p].dagesh = false;
    seq[c1].vowel = Some(Vowel::Hiriq);
    seq.insert(c1 + 1, Cons::mater(letter::YOD));
    Some(hebrew::render(&seq))
}

/// Geminate Hiphil perfect hēCēC (הֵחֵל, הֵסֵב): he + C1 both tsere, the doubled
/// radical collapsing to a single C2. The strong builder spells חלל as a 3-radical
/// heqṭîl (הֶחֱלִיל); this builds the contracted form for the common subjects.
/// Before a vocalic afformative the radical doubles (dagesh): 3cp hēḥēllû הֵחֵלּוּ,
/// 3fs hēḥēllâ הֵחֵלָּה. Returns `None` for the heavy/consonantal subjects (which
/// take the hăsibbôṯ- linking-ô stem, not modelled here).
fn geminate_hiphil_perfect_variant(root: &Root, pgn: Pgn) -> Option<Vec<Cons>> {
    use Vowel::*;
    let c = root.ayin(); // == root.lamed() for a geminate root
    let he = Cons::new(letter::HE).with_vowel(Tsere);
    let c1 = rad(root.pe(), 1).with_vowel(Tsere);
    let three = |g: Gender, num: Number| pgn == Pgn::new(Person::Third, g, num);
    if three(Gender::Masculine, Number::Singular) {
        // A final guttural lowers the theme to patah: hēraʿ הֵרַע (רעע),
        // beside the default hēCēC (הֵפֵר, הֵסֵב).
        let c1 = if hebrew::is_guttural(c) {
            c1.with_vowel(Patah)
        } else {
            c1
        };
        Some(vec![he, c1, rad(c, 2)])
    } else if three(Gender::Common, Number::Plural) {
        Some(vec![he, c1, rad(c, 2).with_dagesh(), oshureq()])
    } else if three(Gender::Feminine, Number::Singular) {
        Some(vec![
            he,
            c1,
            rad(c, 2).with_dagesh().with_vowel(Qamats),
            Cons::new(letter::HE),
        ])
    } else {
        None
    }
}

/// The -â feminine of a Qal active participle (qōṭēlâ), beside the segolate
/// qōṭelet the builder makes: yôlēḏâ יוֹלֵדָה ("woman in labour"), šōmᵊrâ. Built
/// from the radicals — C1 holam, C2 tsere, C3 qamats, he. Additive.
fn qal_participle_fs_a_variant(root: &Root) -> Vec<Cons> {
    use Vowel::*;
    let mut seq = vec![
        rad(root.pe(), 1).with_vowel(Holam),
        rad(root.ayin(), 2).with_vowel(Tsere),
        rad(root.lamed(), 3).with_vowel(Qamats),
        Cons::new(letter::HE),
    ];
    apply_guttural(&mut seq, root);
    seq
}

/// Hollow Niphal perfect nāCôC (נָכוֹן, נָפוֹץ): nun-qamats prefix, the hollow ô
/// on C1, C3. The strong builder treats the vav as a consonant (נִפְוַץ); this
/// builds the contracted form for the common third-person subjects — 3cp nāp̄ôṣû
/// נָפֹצוּ, 3fs nāḵônâ. The ô is written defectively (holam on C1); the plene
/// (vav mater) spelling matches via the parser's holam collapse.
fn hollow_niphal_perfect_variant(root: &Root, pgn: Pgn) -> Option<Vec<Cons>> {
    use Vowel::*;
    let nun = Cons::new(letter::NUN).with_vowel(Qamats);
    let c1 = rad(root.pe(), 1).with_vowel(Holam);
    let c3 = rad(root.lamed(), 3);
    let three = |g: Gender, num: Number| pgn == Pgn::new(Person::Third, g, num);
    if three(Gender::Masculine, Number::Singular) {
        Some(vec![nun, c1, c3])
    } else if three(Gender::Common, Number::Plural) {
        Some(vec![nun, c1, c3, oshureq()])
    } else if three(Gender::Feminine, Number::Singular) {
        Some(vec![nun, c1, c3.with_vowel(Qamats), Cons::new(letter::HE)])
    } else {
        None
    }
}

/// Geminate Niphal perfect nāCaC (נָסַב, נָשַׁם): nun-qamats prefix, C1-patah, the
/// doubled radical collapsing to a single C2. The strong builder spells שמם as a
/// 3-radical niqṭal (נִשְׁמַם); this builds the contracted form. Before a vocalic
/// afformative the radical doubles: 3cp nāšammû נָשַׁמּוּ, 3fs nāšammâ נָשַׁמָּה.
fn geminate_niphal_perfect_variant(root: &Root, pgn: Pgn) -> Option<Vec<Cons>> {
    use Vowel::*;
    let c = root.ayin(); // == root.lamed()
    let nun = Cons::new(letter::NUN).with_vowel(Qamats);
    let c1 = rad(root.pe(), 1).with_vowel(Patah);
    let three = |g: Gender, num: Number| pgn == Pgn::new(Person::Third, g, num);
    if three(Gender::Masculine, Number::Singular) {
        Some(vec![nun, c1, rad(c, 2)])
    } else if three(Gender::Common, Number::Plural) {
        Some(vec![nun, c1, rad(c, 2).with_dagesh(), oshureq()])
    } else if three(Gender::Feminine, Number::Singular) {
        Some(vec![
            nun,
            c1,
            rad(c, 2).with_dagesh().with_vowel(Qamats),
            Cons::new(letter::HE),
        ])
    } else {
        None
    }
}

/// Geminate Niphal participle nāCaC — identical to the contracted perfect 3ms
/// in the ms (נָשַׁם, נָסַב), with the doubled radical restored before the
/// vocalic endings and the prefix qamats reducing: mp nᵊšammîm נְשַׁמִּים,
/// fs nᵊšammâ נְשַׁמָּה, fp nᵊšammôṯ נְשַׁמּוֹת (emitted defectively; the plene
/// spelling matches via the parser's holam collapse).
fn geminate_niphal_participle_variants(root: &Root, pgn: Pgn) -> Vec<String> {
    use Vowel::*;
    let c = root.ayin(); // == root.lamed()
    if hebrew::is_guttural(c) || c == letter::RESH {
        return Vec::new();
    }
    if pgn == Pgn::gn(Gender::Masculine, Number::Singular) {
        return geminate_niphal_perfect_variant(
            root,
            Pgn::new(Person::Third, Gender::Masculine, Number::Singular),
        )
        .map(|s| vec![hebrew::render(&s)])
        .unwrap_or_default();
    }
    let nun = Cons::new(letter::NUN).with_vowel(Sheva);
    let c1 = rad(root.pe(), 1).with_vowel(Patah);
    let c2 = rad(c, 2).with_dagesh();
    let tail: Vec<Cons> = if pgn == Pgn::gn(Gender::Masculine, Number::Plural) {
        vec![
            c2.with_vowel(Hiriq),
            Cons::mater(letter::YOD),
            Cons::new(letter::MEM),
        ]
    } else if pgn == Pgn::gn(Gender::Feminine, Number::Singular) {
        vec![c2.with_vowel(Qamats), Cons::new(letter::HE)]
    } else if pgn == Pgn::gn(Gender::Feminine, Number::Plural) {
        vec![c2.with_vowel(Holam), Cons::new(letter::TAV)]
    } else {
        return Vec::new();
    };
    let mut seq = vec![nun, c1];
    seq.extend(tail);
    vec![hebrew::render(&seq)]
}

/// The hollow Hiphil infinitive construct hāCîC (הָקִים, הָמִית, הָבִיא): he with
/// qamats, C1 with the long hiriq-yod î, C3. The strong builder's haqṭîl shape
/// leaves no usable form for a hollow root (the gizra pass voids it), so build it
/// from the radicals. (C2, the etymological vav/yod, is absorbed into the î.)
fn hollow_hiphil_inf_construct(root: &Root) -> Vec<Cons> {
    use Vowel::*;
    vec![
        Cons::new(letter::HE).with_vowel(Qamats),
        rad(root.pe(), 1).with_vowel(Hiriq),
        Cons::mater(letter::YOD),
        rad(root.lamed(), 3),
    ]
}

/// Doubled-final apocope of a III-He Qal wayyiqtol (שתה, בכה): the etymological
/// he drops and the surviving C2 doubles, giving a tsere-prefixed monosyllable —
/// wayyēšt וַיֵּשְׁתְּ (שתה), wattēḇk וַתֵּבְךְּ (בכה). Built from the preformative
/// (the consonant after the wayyiqtol vav) + C1 (silent sheva) + C2 (dagesh,
/// silent sheva). Returns `None` if the form isn't a vav-prefixed wayyiqtol.
fn lamed_he_doubled_apocope_variant(root: &Root, text: &str) -> Option<String> {
    use Vowel::*;
    let seq = hebrew::parse_pointed(text);
    if seq.first().map(|c| c.letter) != Some(letter::VAV) || seq.len() < 2 {
        return None;
    }
    let pre = seq[1].letter;
    if !matches!(pre, letter::YOD | letter::TAV | letter::ALEF | letter::NUN) {
        return None;
    }
    let mut out = vec![
        Cons::new(letter::VAV).with_vowel(Patah),
        {
            let mut c = Cons::new(pre).with_vowel(Tsere);
            c.dagesh = true;
            c
        },
        rad(root.pe(), 1).with_vowel(Sheva),
        {
            let mut c = rad(root.ayin(), 2).with_vowel(Sheva);
            c.dagesh = true;
            c
        },
    ];
    // A final guttural/resh C2 can't take the dagesh; drop it there.
    if hebrew::rejects_dagesh(root.ayin()) {
        out.last_mut().unwrap().dagesh = false;
    }
    Some(hebrew::render(&out))
}

/// I-nun-style twin of a I-yod Qal imperfect: a few I-yod roots (יצק "pour")
/// drop the yod the way a I-nun root does, the preformative taking hiriq and the
/// theme an o-vowel — yiṣōq יִצֹק, wayyiṣōq וַיִּצֹק. The builder gives the I-vav
/// elided e-grade (yēṣēq יֵצֵק); this rewrites the preformative tsere to hiriq
/// and the C2 tsere to holam. The preformative is `seq[0]`, or `seq[1]` past a
/// wayyiqtol vav. Additive — only the o-grade roots match.
fn pe_yod_as_pe_nun_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let p = if seq.first().is_some_and(|c| c.letter == letter::VAV) && seq.len() > 1 {
        1
    } else {
        0
    };
    if seq.get(p).and_then(|c| c.vowel) != Some(Vowel::Tsere)
        || seq.get(p + 1).and_then(|c| c.vowel) != Some(Vowel::Tsere)
    {
        return None;
    }
    seq[p].vowel = Some(Vowel::Hiriq);
    seq[p + 1].vowel = Some(Vowel::Holam);
    Some(hebrew::render(&seq))
}

/// Patah-pattern twin of a I-aleph Qal imperfect/wayyiqtol. The builder gives
/// the segol grade (yeʾĕsōp̄ וַיֶּאֱסֹף, plural yeʾĕsᵊp̄û); these verbs also take
/// the patah grade where the preformative and the aleph both carry patah and the
/// aleph closes the syllable — wayyaʾasp̄û וַיַּאַסְפוּ, wayyaʾăsōp̄. Changes the
/// preformative segol → patah and the C1 aleph's hataf → patah. The preformative
/// is `seq[0]`, or `seq[1]` past a wayyiqtol vav. Additive.
fn pe_aleph_patah_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let p = if seq.first().is_some_and(|c| c.letter == letter::VAV) && seq.len() > 1 {
        1
    } else {
        0
    };
    if seq.get(p).map(|c| c.vowel) != Some(Some(Vowel::Segol))
        || seq.get(p + 1).map(|c| c.letter) != Some(letter::ALEF)
        || !matches!(
            seq.get(p + 1).and_then(|c| c.vowel),
            Some(Vowel::HatafSegol | Vowel::HatafPatah | Vowel::Segol)
        )
    {
        return None;
    }
    seq[p].vowel = Some(Vowel::Patah);
    seq[p + 1].vowel = Some(Vowel::Patah);
    Some(hebrew::render(&seq))
}

/// Pausal twin of a quiescent I-aleph Qal wayyiqtol. build_wayyiqtol applies
/// nesiga (the stressed C2 patah retracts to segol — וַיֹּאמֶר); in pause the
/// stress stays put and the patah survives — wayyōʾmār וַיֹּאמַר, וַיֹּאכַל.
/// Matches the shape "…quiescent-aleph, C2-segol, final-C3-vowel-less" and
/// raises the segol back to patah. Additive.
fn pe_aleph_wayyiqtol_pausal_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 4 {
        return None;
    }
    // Final radical closes the word vowel-less; penultimate carries the nesiga
    // segol; the quiescent aleph sits right before it.
    if seq[n - 1].vowel.is_some()
        || seq[n - 2].vowel != Some(Vowel::Segol)
        || !(seq[n - 3].letter == letter::ALEF && seq[n - 3].vowel.is_none())
    {
        return None;
    }
    seq[n - 2].vowel = Some(Vowel::Patah);
    Some(hebrew::render(&seq))
}

/// Stative (e-class) twin of the Qal perfect 3ms: the theme patah — or, after
/// a quiescent III-aleph, qamats — raised to tsere: šāmēr beside šāmar, mālēʾ
/// (מָלֵא) beside the māṣāʾ shape, ṭāmēʾ, yārēʾ. Additive.
fn perfect_stative_tsere_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 2 || seq[n - 1].vowel.is_some() {
        return None;
    }
    let expect = if seq[n - 1].letter == letter::ALEF {
        Vowel::Qamats
    } else {
        Vowel::Patah
    };
    if seq[n - 2].vowel != Some(expect) {
        return None;
    }
    seq[n - 2].vowel = Some(Vowel::Tsere);
    Some(hebrew::render(&seq))
}

/// Tsere twin of a final-syllable nesiga segol — the stress-retracted segol
/// (וַיַּגֶּד) beside the tsere-kept spelling (וַיַּגֵּד, וַיַּקְרֵב). Additive.
fn final_segol_to_tsere_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    // A word-final kaf carries an explicit sheva (וַיַּשְׁלֶךְ); any other final
    // consonant closes the word bare.
    if n < 2
        || !matches!(seq[n - 1].vowel, None | Some(Vowel::Sheva))
        || seq[n - 2].vowel != Some(Vowel::Segol)
    {
        return None;
    }
    seq[n - 2].vowel = Some(Vowel::Tsere);
    Some(hebrew::render(&seq))
}

/// Patah twin of a hollow short wayyiqtol's retracted theme — the qamats
/// (וַיָּסָר, Qal) or nesiga segol (וַיָּסֶר, Hiphil) beside the attested patah
/// grade וַיָּסַר. Additive.
fn hollow_wayyiqtol_patah_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 4
        || seq[n - 1].vowel.is_some()
        || !matches!(seq[n - 2].vowel, Some(Vowel::Qamats | Vowel::Segol))
    {
        return None;
    }
    seq[n - 2].vowel = Some(Vowel::Patah);
    Some(hebrew::render(&seq))
}

/// Plene -ênâ twin of the hollow Qal imperfect/wayyiqtol 3fp/2fp: beside the
/// builder's tāqûmnâ (תָּקוּמְנָה) the long ending with a segol-yod surfaces —
/// tᵊqûmênâ (תְּקוּמֶינָה), tᵊp̄ûṣênâ (תְּפוּצֶינָה). The prefix reduces to a
/// sheva, C2 takes segol, and a yod mater is inserted before the -nâ suffix
/// (nun-qamats + he). `text` is the builder's …C2(sheva)-nun(qamats)-he form.
fn hollow_imperfect_fp_plene_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 4
        || seq[n - 1].letter != letter::HE
        || seq[n - 1].vowel.is_some()
        || seq[n - 2].letter != letter::NUN
        || seq[n - 2].vowel != Some(Vowel::Qamats)
    {
        return None;
    }
    // The imperfect preformative reduces to a sheva. In the wayyiqtol the
    // consecutive vav (wa-) leads, so the preformative — keeping its dagesh
    // forte — is the second consonant: watt- → wattᵊqûmênâ וַתְּקוּמֶינָה.
    let prefix = usize::from(seq[0].letter == letter::VAV && seq[0].vowel == Some(Vowel::Patah));
    seq[prefix].vowel = Some(Vowel::Sheva);
    seq[n - 3].vowel = Some(Vowel::Segol);
    seq.insert(n - 2, Cons::mater(letter::YOD));
    Some(hebrew::render(&seq))
}

/// Apocopated twin of a III-He imperative 2ms: drop the final he and bare the
/// preceding consonant — Piel ṣawwēh צַוֵּה → ṣaw צַו (a word-final consonant
/// cannot hold the forte). Additive.
fn lamed_he_imperative_apocope_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 3
        || seq[n - 1].letter != letter::HE
        || seq[n - 1].vowel.is_some()
        || !matches!(seq[n - 2].vowel, Some(Vowel::Tsere | Vowel::Segol))
    {
        return None;
    }
    seq.pop();
    let last = seq.last_mut().unwrap();
    last.vowel = None;
    last.dagesh = false;
    Some(hebrew::render(&seq))
}

/// Defective twin of an ô written plene on a vav mater: drop the vav and put
/// the holam on the preceding (vowel-less) consonant — וַיּוֹסֶף → וַיֹּסֶף,
/// יוֹסִיף → יֹסִיף. Additive.
fn holam_vav_defective_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let i = seq
        .iter()
        .position(|c| c.letter == letter::VAV && c.vowel == Some(Vowel::Holam) && !c.dagesh)?;
    if i == 0 || seq[i - 1].vowel.is_some() {
        return None;
    }
    seq[i - 1].vowel = Some(Vowel::Holam);
    seq.remove(i);
    Some(hebrew::render(&seq))
}

/// Pausal twin of the o-class hollow Qal wayyiqtol. build_wayyiqtol retracts
/// the stress and shortens the holam to qamats (wayyāmāṯ וַיָּמָת); in pause
/// the stress stays put and the holam survives — wayyāmōṯ וַיָּמֹת, וַיָּנֹס.
/// Matches "…C1-qamats, final vowel-less C3" and restores the holam. Additive.
fn hollow_wayyiqtol_pausal_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 4 || seq[n - 1].vowel.is_some() || seq[n - 2].vowel != Some(Vowel::Qamats) {
        return None;
    }
    seq[n - 2].vowel = Some(Vowel::Holam);
    Some(hebrew::render(&seq))
}

/// Hollow Qal active-participle twins, built straight from the radicals.
/// The stative tsere class — mēṯ מֵת, fs מֵתָה, mp מֵתִים, fp מֵתוֹת — beside
/// the default qamats participle, plus the fs qamats-he alternant (bāʾâ
/// בָּאָה, qāmâ) and its construct in -aṯ (zāḇaṯ זָבַת). Additive.
fn hollow_participle_twins(root: &Root, pgn: Pgn) -> Vec<String> {
    use Vowel::*;
    let c1 = |v: Vowel| {
        let mut c = rad(root.pe(), 1).with_vowel(v);
        if hebrew::is_begedkefet(root.pe()) {
            c = c.with_dagesh();
        }
        c
    };
    let c3 = rad(root.lamed(), 3);
    match (pgn.gender, pgn.number) {
        (Some(Gender::Masculine), Some(Number::Singular)) => {
            vec![hebrew::render(&[c1(Tsere), c3])]
        }
        (Some(Gender::Feminine), Some(Number::Singular)) => vec![
            hebrew::render(&[c1(Tsere), c3.with_vowel(Qamats), Cons::new(letter::HE)]),
            hebrew::render(&[c1(Qamats), c3.with_vowel(Qamats), Cons::new(letter::HE)]),
            hebrew::render(&[c1(Qamats), c3.with_vowel(Patah), Cons::new(letter::TAV)]),
        ],
        (Some(Gender::Masculine), Some(Number::Plural)) => vec![
            hebrew::render(&[
                c1(Tsere),
                c3.with_vowel(Hiriq),
                Cons::new(letter::YOD),
                Cons::new(letter::MEM),
            ]),
            // The default qamats grade (qāmîm קָמִים) too, so the suffix
            // hosts built from these twins cover qāmay קָמַי.
            hebrew::render(&[
                c1(Qamats),
                c3.with_vowel(Hiriq),
                Cons::new(letter::YOD),
                Cons::new(letter::MEM),
            ]),
        ],
        (Some(Gender::Feminine), Some(Number::Plural)) => vec![
            hebrew::render(&[
                c1(Tsere),
                c3,
                Cons::new(letter::VAV).with_vowel(Holam),
                Cons::new(letter::TAV),
            ]),
            hebrew::render(&[
                c1(Qamats),
                c3,
                Cons::new(letter::VAV).with_vowel(Holam),
                Cons::new(letter::TAV),
            ]),
        ],
        _ => Vec::new(),
    }
}

/// Vav-doubling twins of a I-vav Niphal imperfect/wayyiqtol. The generator
/// treats the radical as a yod and doubles it (yiyyāʕēṣ וַיִּיָּעֵץ), but the
/// historically I-vav roots double the vav: yiwwāʕēṣ, and before a guttural C2
/// the tsere lowers to patah (wayyiwwāʕaṣ וַיִּוָּעַץ). Finds the doubled radical
/// (a dageshed yod bearing qamats) and re-spells it as a vav; emits both the
/// tsere-kept and patah-lowered twins. Additive.
fn pe_yod_niphal_vav_variants(text: &str) -> Vec<String> {
    let seq = hebrew::parse_pointed(text);
    let Some(j) = seq
        .iter()
        .position(|c| c.letter == letter::YOD && c.dagesh && c.vowel == Some(Vowel::Qamats))
    else {
        return Vec::new();
    };
    let mut base = seq.clone();
    base[j].letter = letter::VAV;
    let mut out = vec![hebrew::render(&base)];
    // patah twin: lower a following guttural's tsere/segol theme to patah.
    if let Some(c2) = base.get_mut(j + 1)
        && hebrew::is_guttural(c2.letter)
        && matches!(c2.vowel, Some(Vowel::Tsere | Vowel::Segol))
    {
        c2.vowel = Some(Vowel::Patah);
        out.push(hebrew::render(&base));
    }
    out
}

/// Nun-retained twin of a I-nun Qal infinitive construct. The builder drops the
/// nun (and dageshes C2: nᵊp̄ōl → pōl פֹּל), but many I-nun roots keep it,
/// surfacing as the plain qᵊṭōl shape — nᵊp̄ōl נְפֹל (בִּנְפֹל), nᵊṣōr, nᵊṯōš. Built
/// straight from the radicals (C1 sheva, C2 holam, C3) so no compensatory dagesh
/// is carried. Additive.
fn pe_nun_inf_construct_retained(root: &Root) -> Vec<Cons> {
    use Vowel::*;
    let mut seq = vec![
        rad(root.pe(), 1).with_vowel(Sheva),
        rad(root.ayin(), 2).with_vowel(Holam),
        rad(root.lamed(), 3),
    ];
    apply_guttural(&mut seq, root);
    seq
}

/// Nun-retained twin of a I-nun Qal imperative. Most I-nun roots assimilate the
/// nun in the imperative (naṭṭēl → ṭēl), but several keep it (nᵊṭēh נְטֵה, nᵊṣōr
/// נְצֹר, nᵊśāʔ). The builder produces only the assimilated stem, so prepend a
/// sheva-nun to restore the strong shape. Additive: only the retaining roots
/// match the nun-initial spelling.
fn pe_nun_imperative_retained_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    if seq.first().is_some_and(|c| c.letter == letter::NUN) {
        return None; // already nun-initial
    }
    // Before a sheva-initial stem (the vocalic-suffix plurals, ṭᵊʿû) the
    // restored nun takes hiriq instead of stacking shevas — niṭʿû נִטְעוּ.
    let v = if seq.first().is_some_and(|c| c.vowel == Some(Vowel::Sheva)) {
        Vowel::Hiriq
    } else {
        Vowel::Sheva
    };
    seq.insert(0, Cons::new(letter::NUN).with_vowel(v));
    Some(hebrew::render(&seq))
}

/// Stative twin(s) of a Qal perfect 3ms: the dynamic qāṭal default (ṭāhar) gives
/// way to the qāṭēl (ṭāhēr טָהֵר, zāqēn זָקֵן, kāḇēḏ) or qāṭōl (qāṭōn קָטֹן, yāḵōl)
/// theme in the lexically-stative verbs, which we don't otherwise mark. Returns
/// the tsere and holam twins of the C2 theme patah. Additive: only the attested
/// theme matches a surface (patah/tsere/holam are distinct spellings).
fn qal_stative_perfect_variants(text: &str) -> Vec<String> {
    let seq = hebrew::parse_pointed(text);
    let n = seq.len();
    // Require the qāṭal shape (C1 qamats); the theme patah is then the first
    // patah after C1 — both in the bare 3ms (qāṭal) and before a consonantal
    // afformative (qāṭaltî, yāḡōrtî יָגֹרְתִּי). The vocalic suffixes reduce
    // the theme away, so no patah is found and nothing is emitted.
    if n < 2 || seq[0].vowel != Some(Vowel::Qamats) {
        return Vec::new();
    }
    if let Some(i) = (1..n).find(|&i| seq[i].vowel == Some(Vowel::Patah)) {
        return [Vowel::Tsere, Vowel::Holam]
            .into_iter()
            .map(|v| {
                let mut s = seq.clone();
                s[i].vowel = Some(v);
                hebrew::render(&s)
            })
            .collect();
    }
    // No theme patah: a vocalic afformative has reduced it to sheva (qāṭlû). The
    // tsere stative reduces likewise (kāḇēḏ → kāḇᵊḏû), but the qāṭōl stative keeps
    // its holam under C2 — yāḵōl → yāḵōlû (יָכֹלוּ), yāḵōlâ (יָכֹלָה). Emit the
    // holam twin on the first reduced (sheva) slot after C1.
    if let Some(i) = (1..n).find(|&i| seq[i].vowel == Some(Vowel::Sheva)) {
        let mut s = seq.clone();
        s[i].vowel = Some(Vowel::Holam);
        return vec![hebrew::render(&s)];
    }
    Vec::new()
}

/// Silent-sheva twin of a word whose C1 guttural carries a composite (hataf)
/// vowel in a closed syllable. A guttural prefers a hataf where a plain
/// consonant would take a silent sheva — the derived-stem perfect of an
/// I-guttural root (neʔĕsap̄, nehĕp̄aḵ נֶהֱפַּךְ, heḥĕzîq) — but the Masoretes
/// frequently write the plain silent sheva instead (נֶהְפַּךְ, נֶעְזַב, הֶחְזִיק).
/// Converts the first guttural hataf that is preceded by a consonant bearing a
/// full short vowel to a plain sheva. The now-silent sheva closes the syllable,
/// so a following בגדכפת consonant takes a dagesh lene (nehĕp̄aḵ → nehpaḵ →
/// nehpáḵ נֶהְפַּךְ, with the pe plosive). Additive: the twin only matches
/// surfaces spelled that way.
fn guttural_silent_sheva_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    for i in 1..seq.len() {
        // The guttural's hataf — or the full segol/patah it was promoted to
        // before a following sheva (neḥešᵊḇû נֶחֶשְׁבוּ) — closes onto a plain
        // silent sheva in the attested twin (נֶחְשְׁבוּ).
        if hebrew::is_guttural(seq[i].letter)
            && matches!(
                seq[i].vowel,
                Some(
                    Vowel::HatafSegol
                        | Vowel::HatafPatah
                        | Vowel::HatafQamats
                        | Vowel::Segol
                        | Vowel::Patah
                )
            )
            && matches!(
                seq[i - 1].vowel,
                Some(Vowel::Segol | Vowel::Patah | Vowel::Hiriq)
            )
        {
            seq[i].vowel = Some(Vowel::Sheva);
            if let Some(next) = seq.get_mut(i + 1)
                && hebrew::is_begedkefet(next.letter)
            {
                next.dagesh = true;
            }
            return Some(hebrew::render(&seq));
        }
    }
    None
}

/// Loud-preformative twin of an I-guttural Hophal: the strong base closes the
/// prefix syllable on a qamats-qatan with the guttural C1 taking a silent sheva
/// (hoḥlêṯî הׇחְלֵיתִי). A I-guttural cannot carry silent sheva comfortably, so
/// the Masoretes open the prefix on a full qamats and give the guttural a
/// hataf-qamats — hoḥŏlêṯî הָחֳלֵיתִי, hoʕŏmaḏ הָעֳמַד. Promotes seq[0]'s
/// qamats-qatan → qamats and the C1 guttural's silent sheva → hataf-qamats.
/// Additive.
fn iguttural_hophal_loud_preformative_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    if seq.len() < 2
        || seq[0].vowel != Some(Vowel::QamatsQatan)
        || !hebrew::is_guttural(seq[1].letter)
        || seq[1].vowel != Some(Vowel::Sheva)
    {
        return None;
    }
    seq[0].vowel = Some(Vowel::Qamats);
    seq[1].vowel = Some(Vowel::HatafQamats);
    Some(hebrew::render(&seq))
}

/// Silent-sheva twin of a I-guttural Qal imperfect-family form whose C1 guttural
/// opens its own syllable on a segol/hataf (ʾeʕeḇᵊrâ אֶעֶבְרָה, yaʕăḇōr): the
/// Masoretes often close the prefix syllable instead, the guttural taking a
/// silent sheva and a following בגדכפת consonant a dagesh lene — ʾeʕbᵊrâ
/// אֶעְבְּרָה. Converts the C1 guttural's segol/hataf-segol/hataf-patah (preceded
/// by the prefix vowel) to a plain sheva. Additive.
fn qal_iguttural_silent_sheva_variants(text: &str) -> Vec<String> {
    let mut seq = hebrew::parse_pointed(text);
    // C1 is the second slot (after the one-letter preformative).
    if seq.len() < 3 || !hebrew::is_guttural(seq[1].letter) {
        return Vec::new();
    }
    if !matches!(
        seq[1].vowel,
        Some(Vowel::Segol | Vowel::HatafSegol | Vowel::HatafPatah)
    ) || seq[0].vowel.is_none()
    {
        return Vec::new();
    }
    seq[1].vowel = Some(Vowel::Sheva);
    if let Some(next) = seq.get_mut(2)
        && hebrew::is_begedkefet(next.letter)
    {
        next.dagesh = true;
    }
    // The I-guttural preformative vowel varies patah (yaʕămōḏ) / segol (yeʾĕsōp̄);
    // emit the silent-sheva form with both so e.g. yehgeh יֶהְגֶּה is reached from
    // a patah-prefixed base.
    [Vowel::Patah, Vowel::Segol]
        .into_iter()
        .map(|pv| {
            let mut s = seq.clone();
            s[0].vowel = Some(pv);
            hebrew::render(&s)
        })
        .collect()
}

/// Guttural-lowered alternant of a III-guttural Piel/Hithpael whose theme tsere
/// is followed by a final ח/ע bearing a furtive patah: yᵉšallēaḥ (יְשַׁלֵּחַ) has
/// the contextual / vav-consecutive short form yᵉšallaḥ (יְשַׁלַּח, וַיְשַׁלַּח), the
/// guttural counterpart of [`maqaf_segol_variant`]. The penult tsere lowers to
/// patah and the now-redundant furtive patah on the guttural drops. Returns the
/// lowered spelling, or `None` when the shape doesn't apply.
fn guttural_lowered_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 2 {
        return None;
    }
    if !matches!(seq[n - 1].letter, letter::HET | letter::AYIN)
        || seq[n - 1].vowel != Some(Vowel::Patah)
        || seq[n - 2].vowel != Some(Vowel::Tsere)
    {
        return None;
    }
    seq[n - 2].vowel = Some(Vowel::Patah);
    seq[n - 1].vowel = None;
    Some(hebrew::render(&seq))
}

/// Patah-prefix twin of the I-guttural Niphal/Hiphil perfect. The generator
/// builds these with a segol preformative and a hataf-segol on the C1 guttural
/// (neḥĕzaq נֶחֱזַק, heʕĕmîd הֶעֱמִיד — see [`apply_pe_guttural`]), but for some
/// I-guttural roots the preformative surfaces with patah and the guttural with
/// hataf-patah: naʕăśâ (נַעֲשָׂה), the Niphal perfect of עשה. Both spellings are
/// attested across roots, so emit the patah twin alongside the segol base —
/// generate-and-test only gains matches. Returns `None` unless the word opens
/// with a segol-bearing consonant followed by a guttural bearing hataf-segol.
fn guttural_perfect_patah_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    if seq.len() < 2 {
        return None;
    }
    if seq[0].vowel != Some(Vowel::Segol)
        || seq[1].vowel != Some(Vowel::HatafSegol)
        || !hebrew::is_guttural(seq[1].letter)
    {
        return None;
    }
    seq[0].vowel = Some(Vowel::Patah);
    seq[1].vowel = Some(Vowel::HatafPatah);
    Some(hebrew::render(&seq))
}

/// PeAleph Qal Imperfect / Jussive of the yōʾ- contracted class: the C2 vowel
/// may surface as tsere instead of patah — יֹאכֵל beside יֹאכַל, יֹאכֵלוּ beside
/// יֹאכְלוּ. The pattern has an aleph (C1) with no vowel, followed by a consonant
/// with a short a-class vowel (patah or qamats) that should also get a tsere
/// variant. Returns the tsere twin or None.
/// Holam-contraction twin for a I-aleph Qal imperfect-family form of a root
/// outside [`YO_ROOTS`] but which nonetheless admits the contracted yōʔ- pattern
/// in attested usage — yeʔĕḥōz (יֶאֱחֹז) beside yōʔḥēz (יֹאחֵז), wayyeʾeḥᵊzû
/// (וַיֶּאֶחְזוּ) beside wayyōʾḥăzû (וַיֹּאחֲזוּ). The default is the regular
/// I-guttural segol class; this twin contracts it: the prefix segol/patah → holam,
/// the C1 aleph quiesces (drops its vowel), and the C2 holam of the long grade
/// shortens to tsere (יֹאחֵז). A reduced guttural C2 (sheva before a vocalic
/// afformative) reopens with hataf-patah (יֹאחֲזוּ). Skips the 1cs, whose
/// aleph preformative would merge with the C1 aleph. Additive.
/// Uncontracted (archaic) twin of a Hiphil imperfect whose preformative has
/// contracted onto the stem vowel. Two shapes:
///   • pe-vav/pe-yod holam stem — yôšîₐʿ (יוֹשִׁיעַ) → yᵊhôšîₐʿ (יְהוֹשִׁיעַ),
///     the original Hiphil he resurfacing before the holam-vav;
///   • pe-yod tsere stem — yêlîl (יֵילִיל) → yᵊyêlîl (יְיֵלִיל), the merged
///     yod-mater splitting back into a vocal-sheva preformative + tsere yod.
/// Detects a preformative consonant (vowelless, or tsere over a yod mater) and
/// rebuilds the full prefix. Additive.
fn hiphil_imperfect_uncontracted_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    if seq.len() < 2
        || !matches!(
            seq[0].letter,
            letter::YOD | letter::TAV | letter::NUN | letter::ALEF
        )
    {
        return None;
    }
    // pe-vav/pe-yod holam: vowelless preformative + holam-vav → sheva + he + vav.
    if seq[0].vowel.is_none()
        && seq[1].letter == letter::VAV
        && seq[1].vowel == Some(Vowel::Holam)
    {
        seq[0].vowel = Some(Vowel::Sheva);
        seq.insert(1, Cons::new(letter::HE));
        return Some(hebrew::render(&seq));
    }
    // pe-yod tsere: preformative tsere + vowelless yod mater → sheva + tsere yod.
    if seq[0].vowel == Some(Vowel::Tsere)
        && seq[1].letter == letter::YOD
        && seq[1].vowel.is_none()
    {
        seq[0].vowel = Some(Vowel::Sheva);
        seq[1].vowel = Some(Vowel::Tsere);
        return Some(hebrew::render(&seq));
    }
    None
}

fn pe_aleph_holam_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    // Find the C1 aleph: a non-initial aleph carrying a segol/hataf opening vowel,
    // immediately preceded by a prefix consonant with a segol or patah.
    let a = (1..seq.len()).find(|&i| {
        seq[i].letter == letter::ALEF
            && matches!(
                seq[i].vowel,
                Some(Vowel::Segol | Vowel::HatafSegol | Vowel::HatafPatah)
            )
            && matches!(seq[i - 1].vowel, Some(Vowel::Segol | Vowel::Patah))
            && seq[i - 1].letter != letter::ALEF
    })?;
    seq[a - 1].vowel = Some(Vowel::Holam);
    seq[a].vowel = None;
    if let Some(c2) = seq.get_mut(a + 1) {
        match c2.vowel {
            Some(Vowel::Holam) => c2.vowel = Some(Vowel::Tsere),
            Some(Vowel::Sheva) if hebrew::is_guttural(c2.letter) => {
                c2.vowel = Some(Vowel::HatafPatah)
            }
            _ => {}
        }
    }
    Some(hebrew::render(&seq))
}

fn pe_aleph_imperfect_tsere_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    // Find a vowelless aleph followed by a consonant with patah or qamats.
    // Replace that vowel with tsere.
    let aleph_idx = seq
        .iter()
        .position(|c| c.letter == letter::ALEF && c.vowel.is_none())?;
    let target_idx = aleph_idx + 1;
    if target_idx < seq.len() && matches!(seq[target_idx].vowel, Some(Vowel::Patah | Vowel::Qamats))
    {
        seq[target_idx].vowel = Some(Vowel::Tsere);
        Some(hebrew::render(&seq))
    } else {
        None
    }
}

/// LamedAleph Qal Perfect with a consonantal suffix: the shortened a-class vowel
/// under C2 can surface as tsere instead of qamats — שָׂנֵאתִי beside שָׂנָאתִי,
/// מָצֵאתִי beside מָצָאתִי. Returns the tsere twin or None.
fn lamed_aleph_perfect_tsere_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    // Look for: consonant + qamats + aleph(vowelless) + suffix-consonant(tav/nun).
    // Replace qamats with tsere.
    for i in 0..seq.len().saturating_sub(2) {
        if seq[i].vowel == Some(Vowel::Qamats)
            && seq[i + 1].letter == letter::ALEF
            && seq[i + 1].vowel.is_none()
            && matches!(seq[i + 2].letter, letter::TAV | letter::NUN)
        {
            seq[i].vowel = Some(Vowel::Tsere);
            return Some(hebrew::render(&seq));
        }
    }
    None
}

/// III-aleph imperfect-family / imperative -nâ (3fp/2fp): the quiescent aleph
/// takes no vowel and the radical before it carries segol — tiqreʔnâ
/// (תִּקְרֶאנָה), tēṣeʔnâ (תֵּצֶאנָה), tiśśeʔnâ (תִּשֶּׂאנָה) — beside the builder's
/// patah/qamats spellings (תִּקְרַאְנָה, תִּקְרָאֲנָה). The -nâ closes the syllable
/// after the quiescent aleph, so the long thematic vowel (holam → qamats) the
/// open imperfect would take never appears; the short segol is what surfaces.
fn lamed_aleph_imperfect_nun_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 4 {
        return None;
    }
    // Match a trailing ...C2 + aleph + nun(qamats) + he(vowelless), the -nâ
    // ending sitting on the quiescent C3 aleph.
    if seq[n - 1].letter == letter::HE
        && seq[n - 1].vowel.is_none()
        && seq[n - 2].letter == letter::NUN
        && seq[n - 2].vowel == Some(Vowel::Qamats)
        && seq[n - 3].letter == letter::ALEF
    {
        seq[n - 3].vowel = None;
        seq[n - 4].vowel = Some(Vowel::Segol);
        return Some(hebrew::render(&seq));
    }
    None
}

/// PeGuttural Qal Imperfect / Jussive / Wayyiqtol non-1cs: the prefix vowel
/// may surface as segol instead of patah, with a matching hataf-segol on C1 —
/// יֶחֱטָא beside יַחֲטָא, וַיֶּחֱזַק beside וַיַּחֲזֹק. Returns the segol twin or
/// None.
/// Loud twin of a segol-prefix form whose guttural closed the prefix syllable on
/// a silent sheva: a vowelless guttural preceded by a segol opens the next
/// syllable with a hataf-segol instead — yeḥrāḇû יֶחְרָבוּ → yeḥĕrāḇû יֶחֱרָבוּ.
/// Returns `None` when no such guttural is present. Additive.
fn guttural_segol_silent_to_hataf_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    for i in 1..seq.len() {
        // The guttural closes the prefix syllable on a silent sheva, which
        // `render` writes (Some(Sheva)) — or, word-finally, leaves bare (None).
        if hebrew::is_guttural(seq[i].letter)
            && matches!(seq[i].vowel, None | Some(Vowel::Sheva))
            && seq[i - 1].vowel == Some(Vowel::Segol)
        {
            seq[i].vowel = Some(Vowel::HatafSegol);
            return Some(hebrew::render(&seq));
        }
    }
    None
}

fn pe_guttural_imperfect_segol_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    // Find a guttural with hataf-patah, preceded by a consonant with patah.
    // Change them to segol + hataf-segol.
    for i in 1..seq.len() {
        if hebrew::is_guttural(seq[i].letter)
            && seq[i].vowel == Some(Vowel::HatafPatah)
            && seq[i - 1].vowel == Some(Vowel::Patah)
        {
            seq[i - 1].vowel = Some(Vowel::Segol);
            seq[i].vowel = Some(Vowel::HatafSegol);
            return Some(hebrew::render(&seq));
        }
    }
    None
}

/// III-Guttural Qal Perfect with a vocalic suffix (3fs, 3cp): the default
/// sheva under C2 is lengthened to qamats by apply_lamed_guttural. For some
/// roots the short form with sheva is also attested — יָדְעוּ beside יָדָעוּ,
/// יָדְעָה beside יָדָעָה. Returns the sheva twin or None.
fn lamed_guttural_perfect_sheva_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    // Look for: consonant + qamats (C2) + guttural C3 + suffix. The guttural
    // is vowelless before the shureq -û (יָדָעוּ) but itself carries the
    // suffix qamats in the 3fs -â (שָׁמָעָה → שָׁמְעָה).
    for i in 0..seq.len().saturating_sub(2) {
        if seq[i].vowel == Some(Vowel::Qamats)
            && matches!(seq[i + 1].letter, letter::HET | letter::AYIN)
            && matches!(seq[i + 1].vowel, None | Some(Vowel::Qamats))
        {
            seq[i].vowel = Some(Vowel::Sheva);
            return Some(hebrew::render(&seq));
        }
    }
    None
}

/// Full-vowel twin of a suffixed form whose C1 guttural carries a hataf that the
/// reduction should have closed: a guttural with hataf-patah/segol immediately
/// followed by a consonant bearing a vocal sheva fills to the matching short
/// vowel, closing its syllable — yaʕăzᵊḇēnî יַעֲזְבֵנִי → yaʕazḇēnî יַעַזְבֵנִי
/// (וַיַּעַזְבֵנִי), yahărgēhû → yahargēhû. Returns `None` when no such guttural is
/// present. Additive.
fn guttural_hataf_to_full_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    for i in 0..seq.len().saturating_sub(1) {
        let full = match seq[i].vowel {
            Some(Vowel::HatafPatah) => Vowel::Patah,
            Some(Vowel::HatafSegol) => Vowel::Segol,
            Some(Vowel::HatafQamats) => Vowel::QamatsQatan,
            _ => continue,
        };
        if hebrew::is_guttural(seq[i].letter) && seq[i + 1].vowel == Some(Vowel::Sheva) {
            seq[i].vowel = Some(full);
            return Some(hebrew::render(&seq));
        }
    }
    None
}

/// Defective twin of a form carrying an î mater: the first hiriq-bearing
/// consonant followed by a vowelless yod mater (the plene î of הִקְטִיל / יְבִיא)
/// drops the yod, leaving a bare hiriq — yᵉḇîʔēhû יְבִיאֵהוּ → yᵉḇiʔēhû יְבִאֵהוּ.
/// Returns `None` when no such hiriq+yod is present. Additive: the defective
/// spelling differs from the plene one.
fn strip_hiriq_yod_mater_variants(text: &str) -> Vec<String> {
    let seq = hebrew::parse_pointed(text);
    let mut out = Vec::new();
    // Require something after the yod so a word-final suffix yod (the 1cs -nî)
    // is never mistaken for a medial î mater. Each interior hiriq+yod is dropped
    // independently — a suffixed form can carry more than one î (the Hiphil/III-He
    // stem î and the -tî afformative), and either may be written defectively.
    for i in 0..seq.len().saturating_sub(2) {
        if seq[i].vowel == Some(Vowel::Hiriq)
            && seq[i + 1].letter == letter::YOD
            && seq[i + 1].vowel.is_none()
        {
            let mut s = seq.clone();
            s.remove(i + 1);
            out.push(hebrew::render(&s));
        }
    }
    out
}

/// Geminate-contraction twin of a suffixed form whose doubled radical the
/// strong builder spelled out in full: the first `[L + vocal sheva][L + vowel]`
/// pair (both the geminate radical `l`) collapses into one dageshed `L` carrying
/// the second's vowel — ḥānᵊnēnî חָנְנֵנִי → ḥonnēnî חָנֵּנִי, sabbēnî. Returns
/// `None` when no such adjacent pair is present. Additive: the contracted
/// spelling differs from the spelled-out one.
fn geminate_contract_variant(text: &str, l: char) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    for i in 0..seq.len().saturating_sub(1) {
        if seq[i].letter == l
            && seq[i].vowel == Some(Vowel::Sheva)
            && seq[i + 1].letter == l
            && seq[i + 1].vowel.is_some()
        {
            seq[i + 1].dagesh = true;
            seq.remove(i);
            return Some(hebrew::render(&seq));
        }
    }
    None
}

/// III-guttural perfect 2fs helping-patah twin: the 2fs afformative -t closes
/// the stem (yāḏaʕt יָדַעְתְּ), but a final het/ayin can't sit vowelless before
/// the tav, so a furtive helping patah surfaces on the guttural — yāḏaʕat
/// (יָדַעַתְּ), lāqaḥat, hišbaʕat. Replaces the silent sheva under the final
/// radical guttural (the consonant before the suffix tav) with patah. Additive:
/// the helping-patah spelling differs from the bare silent-sheva one.
fn lamed_guttural_perfect_2fs_helping_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 2 || seq[n - 1].letter != letter::TAV {
        return None;
    }
    let g = &mut seq[n - 2];
    if matches!(g.letter, letter::HET | letter::AYIN)
        && matches!(g.vowel, None | Some(Vowel::Sheva))
    {
        g.vowel = Some(Vowel::Patah);
        return Some(hebrew::render(&seq));
    }
    None
}

// ----- Pronominal object suffixes -------------------------------------------
//
// Finite verbs take pronominal *object* suffixes — "he sent me" (שְׁלָחַנִי),
// "I will bless you" (יְבָרֶכְךָ), "and he gave them" (וַיִּתְּנֵם). The suffix
// attaches to a "connecting" grade of the stem: stress shifts onto the ending,
// so the stem's own vowels reduce, and a linking vowel joins stem to suffix.
//
// We model the two highest-frequency hosts — the Qal Perfect 3ms and the
// Imperfect/Jussive/Wayyiqtol 3ms — for a single (3ms) subject. As with every
// reverse-parse target this is generate-and-test: an imperfectly modelled
// connecting stem simply fails to exact-match a real surface (costing recall,
// never correctness), so emitting a small spread of plausible stems is safe.
// Weak hosts whose suffixed stem is idiosyncratic (III-He עָשָׂהוּ, the geminating
// I-Nun/III-He Hiphil וַיַּכֵּהוּ) are left for a later pass.

const OBJ_1CS: Pgn = Pgn::new(Person::First, Gender::Common, Number::Singular);
const OBJ_2MS: Pgn = Pgn::new(Person::Second, Gender::Masculine, Number::Singular);
const OBJ_2FS: Pgn = Pgn::new(Person::Second, Gender::Feminine, Number::Singular);
const OBJ_3MS: Pgn = Pgn::new(Person::Third, Gender::Masculine, Number::Singular);
const OBJ_3FS: Pgn = Pgn::new(Person::Third, Gender::Feminine, Number::Singular);
const OBJ_1CP: Pgn = Pgn::new(Person::First, Gender::Common, Number::Plural);
const OBJ_2MP: Pgn = Pgn::new(Person::Second, Gender::Masculine, Number::Plural);
const OBJ_3MP: Pgn = Pgn::new(Person::Third, Gender::Masculine, Number::Plural);
const OBJ_3FP: Pgn = Pgn::new(Person::Third, Gender::Feminine, Number::Plural);

/// A vowelled consonant, for assembling suffix tails.
fn ocv(letter: char, v: Vowel) -> Cons {
    Cons::new(letter).with_vowel(v)
}
/// A shureq written as a vav mater bearing the dagesh point (וּ).
fn oshureq() -> Cons {
    Cons::new(letter::VAV).with_dagesh()
}

/// Qal Perfect 3ms with a pronominal object suffix. The connecting stem is
/// qᵊṭāl- (C1 propretonically reduced to sheva, C2 qamats), the suffix joins C3
/// with a linking vowel: qᵊṭāl-á-nî (קְטָלַנִי), qᵊṭāl-ᵊ-ḵā (קְטָלְךָ), qᵊṭāl-ô
/// (קְטָלוֹ), qᵊṭāl-ā-m (קְטָלָם). Restricted to the strong/guttural shape — a 3ms
/// ending in a true final consonant — so I-weak and III-weak roots (whose
/// suffixed perfect is irregular) are skipped.
fn qal_perfect_object_suffixes(root: &Root) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let (c1, c2, c3) = (root.pe(), root.ayin(), root.lamed());
    // The perfect keeps all three radicals even in the I-nun and I-yod classes
    // (nᵊṯānām נְתָנָם, yᵊḏāʿāh יְדָעָהּ are regular), and a quiescent III-aleph
    // simply carries the link vowel (nᵊśāʾô נְשָׂאוֹ, śᵊnēʾāh וּשְׂנֵאָהּ), so only
    // III-he/vav/yod and I-vav shapes are excluded.
    if matches!(c3, letter::HE | letter::VAV | letter::YOD) || c1 == letter::VAV {
        return Vec::new();
    }
    // Hollow: the suffix joins the contracted qām- stem directly — śāmô
    // שָׂמוֹ, śāmāh — C1 qamats, C3 + link (the etymological vav/yod is gone).
    let hollow = matches!(c2, letter::VAV | letter::YOD);
    // Build qᵊṭāC3- with an optional C3 link vowel (None leaves C3 vowelless,
    // for the holam-vav 3ms ô written on the mater), append the tail, then let
    // apply_guttural fix a guttural C1/C2 (sheva → hataf).
    let build = |c2v: Vowel, link: Option<Vowel>, tail: &[Cons]| -> String {
        let mut c3c = Cons::radical(c3, 3);
        c3c.vowel = link;
        let mut seq = if hollow {
            vec![Cons::radical(c1, 1).with_vowel(Qamats), c3c]
        } else {
            vec![
                Cons::radical(c1, 1).with_vowel(Sheva),
                Cons::radical(c2, 2).with_vowel(c2v),
                c3c,
            ]
        };
        seq.extend_from_slice(tail);
        apply_guttural(&mut seq, root);
        hebrew::render(&seq)
    };
    let mut out = Vec::new();
    // Light suffixes attach to the qᵊṭāl- (C2 qamats) grade — the statives on
    // their tsere grade instead (ʾăhēḇô אֲהֵבוֹ), so emit both.
    for c2v in [Qamats, Tsere] {
        out.push((
            OBJ_1CS,
            build(
                c2v,
                Some(Patah),
                &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
            ),
        ));
        out.push((
            OBJ_2MS,
            build(c2v, Some(Sheva), &[ocv(letter::KAF, Qamats)]),
        ));
        out.push((OBJ_2FS, build(c2v, Some(Tsere), &[ocv(letter::KAF, Sheva)])));
        // 3ms ô — holam carried on the vav mater (qᵊṭālô קְטָלוֹ).
        out.push((
            OBJ_3MS,
            build(c2v, None, &[Cons::new(letter::VAV).with_vowel(Holam)]),
        ));
        out.push((
            OBJ_3FS,
            build(c2v, Some(Qamats), &[Cons::new(letter::HE).with_dagesh()]),
        ));
        out.push((
            OBJ_1CP,
            build(c2v, Some(Qamats), &[Cons::new(letter::NUN), oshureq()]),
        ));
        out.push((OBJ_3MP, build(c2v, Some(Qamats), &[Cons::new(letter::MEM)])));
        out.push((OBJ_3FP, build(c2v, Some(Qamats), &[Cons::new(letter::NUN)])));
    }
    // Heavy 2mp/2fp attach to the qᵊṭal- (C2 patah) grade.
    out.push((
        OBJ_2MP,
        build(
            Patah,
            Some(Sheva),
            &[ocv(letter::KAF, Segol), Cons::new(letter::MEM)],
        ),
    ));
    out
}

/// Piel/Pual/Hithpael perfect 3ms with a pronominal object suffix, built from the
/// bare perfect `base_text`. Unlike the Qal (whose suffix grade qᵊṭāl- differs
/// from the base qāṭal), these stems keep their first syllable and only the theme
/// vowel on the doubled C2 reduces to a vocal sheva before the suffix: dibbēr →
/// dibbᵊrô (דִּבְּרוֹ), dibbᵊranî (דִּבְּרַנִי), bērak → bērᵊḵô. C3 then takes the
/// same perfect linking vowels + suffix consonants as the Qal. The base must end
/// in a true final consonant — III-He (ṣiwwâ) and III-guttural shapes are left to
/// other paths. Additive: a mis-modelled connecting vowel simply fails to match.
fn derived_perfect_object_suffixes(base_text: &str) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let seq = hebrew::parse_pointed(base_text);
    let n = seq.len();
    if n < 3 {
        return Vec::new();
    }
    let last = seq[n - 1];
    // A III-aleph base (ṭimmēʾ טִמֵּא): the theme reduces and the suffix joins
    // the quiescent aleph — ṭimmᵊʾô וְטִמְּאוֹ.
    if last.letter == letter::ALEF && last.vowel.is_none() && seq[n - 2].vowel == Some(Tsere) {
        let build = |link: Option<Vowel>, tail: &[Cons]| -> String {
            let mut s = seq.clone();
            s[n - 2].vowel = Some(Sheva);
            s[n - 1].vowel = link;
            s.extend_from_slice(tail);
            hebrew::render(&s)
        };
        return vec![
            (
                OBJ_1CS,
                build(
                    Some(Patah),
                    &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
                ),
            ),
            (
                OBJ_2MS,
                build(Some(HatafPatah), &[ocv(letter::KAF, Qamats)]),
            ),
            (
                OBJ_3MS,
                build(None, &[Cons::new(letter::VAV).with_vowel(Holam)]),
            ),
            (
                OBJ_3FS,
                build(Some(Qamats), &[Cons::new(letter::HE).with_dagesh()]),
            ),
            (
                OBJ_1CP,
                build(Some(Qamats), &[Cons::new(letter::NUN), oshureq()]),
            ),
            (OBJ_3MP, build(Some(Qamats), &[Cons::new(letter::MEM)])),
        ];
    }
    // A III-guttural's furtive patah (śimmēaḥ שִׂמֵּחַ) is not a real final
    // vowel; the suffix replaces it (śimmᵊḥām שִׂמְּחָם).
    let furtive = hebrew::is_guttural(last.letter) && last.vowel == Some(Patah);
    if (matches!(last.vowel, Some(v) if v != Sheva) && !furtive)
        || matches!(
            last.letter,
            letter::HE | letter::ALEF | letter::VAV | letter::YOD
        )
    {
        return Vec::new();
    }
    // Reduce the theme on C2 (seq[n-2]) — to a vocal sheva, or the hataf-patah
    // an undageshable guttural or resh takes instead (ṭihăr-ô וְטִהֲרוֹ,
    // bērăḵô בֵּרֲכוֹ) — keeping any forte dagesh, and give C3 (seq[n-1]) the
    // linking vowel, then append the suffix tail.
    let reduced_grades: &[Vowel] = if hebrew::is_guttural(seq[n - 2].letter) {
        &[HatafPatah]
    } else if seq[n - 2].letter == letter::RESH {
        &[Sheva, HatafPatah]
    } else {
        &[Sheva]
    };
    let build = |theme: Vowel, link: Option<Vowel>, tail: &[Cons]| -> String {
        let mut s = seq.clone();
        s[n - 2].vowel = Some(theme);
        s[n - 1].vowel = link;
        s.extend_from_slice(tail);
        hebrew::render(&s)
    };
    let mut out = Vec::new();
    for &reduced in reduced_grades {
        out.extend([
            (
                OBJ_1CS,
                build(
                    reduced,
                    Some(Patah),
                    &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
                ),
            ), // -anî
            (
                OBJ_2MS,
                build(reduced, Some(Sheva), &[ocv(letter::KAF, Qamats)]),
            ), // -ᵊḵā
            (
                OBJ_2FS,
                build(reduced, Some(Tsere), &[ocv(letter::KAF, Sheva)]),
            ), // -ēḵ
            (
                OBJ_3MS,
                build(reduced, None, &[Cons::new(letter::VAV).with_vowel(Holam)]),
            ), // -ô
            (
                OBJ_3FS,
                build(
                    reduced,
                    Some(Qamats),
                    &[Cons::new(letter::HE).with_dagesh()],
                ),
            ), // -āh
            (
                OBJ_1CP,
                build(reduced, Some(Qamats), &[Cons::new(letter::NUN), oshureq()]),
            ), // -ānû
            (
                OBJ_3MP,
                build(reduced, Some(Qamats), &[Cons::new(letter::MEM)]),
            ), // -ām
            (
                OBJ_3FP,
                build(reduced, Some(Qamats), &[Cons::new(letter::NUN)]),
            ), // -ān
            (
                OBJ_2MP,
                build(
                    reduced,
                    Some(Sheva),
                    &[ocv(letter::KAF, Segol), Cons::new(letter::MEM)],
                ),
            ), // -ᵊḵem
        ]);
    }
    // The sheva-link 2ms/2mp also keep a full patah theme: bēraḵḵā בֵּרַכְךָ.
    out.push((
        OBJ_2MS,
        build(Patah, Some(Sheva), &[ocv(letter::KAF, Qamats)]),
    ));
    out.push((
        OBJ_2MP,
        build(
            Patah,
            Some(Sheva),
            &[ocv(letter::KAF, Segol), Cons::new(letter::MEM)],
        ),
    ));
    out
}

/// Hiphil perfect 3ms with a pronominal object suffix, built from the bare
/// perfect `base_text`: the long î theme is retained and the suffix joins C3 —
/// hisgîrô וְהִסְגִּירוֹ, hēḇîʾanî → with the first syllable reduced
/// propretonically, hĕḇîʾanî הֱבִיאַנִי. Additive: only exact matches survive.
fn hiphil_perfect_object_suffixes(base_text: &str) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let seq = hebrew::parse_pointed(base_text);
    let n = seq.len();
    if n < 3 {
        return Vec::new();
    }
    let last = seq[n - 1];
    // C3 must be a true final consonant or a quiescent aleph after the î.
    if matches!(last.vowel, Some(v) if v != Sheva)
        || matches!(last.letter, letter::HE | letter::VAV | letter::YOD)
    {
        return Vec::new();
    }
    // Hosts: the base, plus a propretonically-reduced first syllable when it
    // carries a tsere/hiriq (הֵבִיא → הֱבִיא).
    let mut hosts = vec![seq.clone()];
    if seq[0].letter == letter::HE && matches!(seq[0].vowel, Some(Tsere)) {
        let mut reduced = seq.clone();
        reduced[0].vowel = Some(HatafSegol);
        hosts.push(reduced);
    }
    let aleph = last.letter == letter::ALEF;
    let tails: Vec<(Pgn, Option<Vowel>, Vec<Cons>)> = vec![
        (
            OBJ_1CS,
            Some(Patah),
            vec![ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
        ), // -anî
        (
            OBJ_1CS,
            Some(Qamats),
            vec![ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
        ), // -ānî (hiṣṣîlānî הִצִּילָנִי)
        (
            OBJ_2MS,
            Some(if aleph { HatafPatah } else { Sheva }),
            vec![ocv(letter::KAF, Qamats)],
        ), // -ᵊḵā
        (
            OBJ_3MS,
            None,
            vec![Cons::new(letter::VAV).with_vowel(Holam)],
        ), // -ô
        (
            OBJ_3FS,
            Some(Qamats),
            vec![Cons::new(letter::HE).with_dagesh()],
        ), // -āh
        (
            OBJ_1CP,
            Some(Qamats),
            vec![Cons::new(letter::NUN), oshureq()],
        ), // -ānû
        (OBJ_3MP, Some(Qamats), vec![Cons::new(letter::MEM)]), // -ām
        (OBJ_3FP, Some(Qamats), vec![Cons::new(letter::NUN)]), // -ān
    ];
    let mut out = Vec::new();
    for host in &hosts {
        for (obj, link, tail) in &tails {
            let mut s = host.clone();
            s[n - 1].vowel = *link;
            s.extend(tail.iter().cloned());
            out.push((*obj, hebrew::render(&s)));
        }
    }
    out
}

/// III-He perfect 3ms with a pronominal object suffix, any binyan, built from the
/// bare perfect `base_text` (…C2-qamats + etymological he mater: ʕāśâ עָשָׂה,
/// ṣiwwâ צִוָּה, heʿĕlâ הֶעֱלָה). The he mater drops and the suffix attaches to the
/// stem's -ā: the light suffixes keep that qamats (ʕāśāhû עָשָׂהוּ, ṣiwwānî,
/// ṣiwwāhû צִוָּהוּ), while the heavy -ḵā/-ḵem reduce it to a vocal sheva
/// (ṣiwwᵊḵā צִוְּךָ, ʕāśᵊḵā). Additive — only exact matches survive.
fn lamed_he_perfect_object_suffixes(base_text: &str) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let seq = hebrew::parse_pointed(base_text);
    let n = seq.len();
    if n < 3 || seq[n - 1].letter != letter::HE || seq[n - 2].vowel != Some(Qamats) {
        return Vec::new();
    }
    // Drop the he mater; the C2 (now last) carries the suffix's connecting vowel.
    let stem = seq[..n - 1].to_vec();
    let emit = |c2v: Vowel, tail: &[Cons]| -> String {
        let mut s = stem.clone();
        if let Some(c) = s.last_mut() {
            c.vowel = Some(c2v);
        }
        s.extend_from_slice(tail);
        hebrew::render(&s)
    };
    // Light suffixes: the Qal/Piel keep the stem's -ā (qamats), but the Hiphil
    // III-He links on a patah (hirʔanî הִרְאַנִי, not hirʔānî); emit both grades.
    let nun_yod = [ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)];
    let he_u = [Cons::new(letter::HE), oshureq()];
    let nun_u = [Cons::new(letter::NUN), oshureq()];
    let mem = [Cons::new(letter::MEM)];
    let mut out = Vec::new();
    for link in [Qamats, Patah] {
        out.push((OBJ_1CS, emit(link, &nun_yod))); // -ānî / -anî
        out.push((OBJ_3MS, emit(link, &he_u))); // -āhû / -ahû
        out.push((OBJ_1CP, emit(link, &nun_u))); // -ānû / -anû
        out.push((OBJ_3MP, emit(link, &mem))); // -ām / -am
    }
    out.push((OBJ_2MS, emit(Sheva, &[ocv(letter::KAF, Qamats)]))); // -ᵊḵā
    out.push((
        OBJ_2MP,
        emit(Sheva, &[ocv(letter::KAF, Segol), Cons::new(letter::MEM)]),
    )); // -ᵊḵem
    // 3fs mappiq-he -āh (ṣiwwāh צִוָּהּ) and the pausal -āḵ (ṣiwwāḵ צִוָּךְ).
    out.push((
        OBJ_3FS,
        emit(Qamats, &[Cons::new(letter::HE).with_dagesh()]),
    ));
    out.push((OBJ_2MS, emit(Qamats, &[ocv(letter::KAF, Sheva)])));
    out.push((OBJ_2FS, emit(Qamats, &[ocv(letter::KAF, Sheva)])));
    out
}

/// III-He imperfect/jussive/wayyiqtol 3ms (or other zero-suffix subject) with a
/// pronominal object suffix. The base ends in the segol + etymological he
/// (yaʕăneh יַעֲנֶה, yaʕăleh); the he elides and the suffix joins C2 with a
/// connecting tsere — yaʕănēhû יַעֲנֵהוּ, yaʕănēnî, yaʕălēhû — beside the heavier
/// 2ms/2mp on a sheva-grade C2. Mirrors [`lamed_he_perfect_object_suffixes`] but
/// for the imperfect's tsere link. Requires the segol+he ending.
fn lamed_he_imperfect_object_suffixes(base_text: &str) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let seq = hebrew::parse_pointed(base_text);
    let n = seq.len();
    if n < 3 || seq[n - 1].letter != letter::HE || seq[n - 2].vowel != Some(Segol) {
        return Vec::new();
    }
    let stem = seq[..n - 1].to_vec();
    let emit = |c2v: Vowel, tail: &[Cons]| -> String {
        let mut s = stem.clone();
        if let Some(c) = s.last_mut() {
            c.vowel = Some(c2v);
        }
        s.extend_from_slice(tail);
        hebrew::render(&s)
    };
    vec![
        (
            OBJ_1CS,
            emit(Tsere, &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)]),
        ), // -ēnî
        (OBJ_3MS, emit(Tsere, &[Cons::new(letter::HE), oshureq()])), // -ēhû
        (OBJ_3FS, emit(Segol, &[ocv(letter::HE, Qamats)])),          // -ehā
        (OBJ_1CP, emit(Tsere, &[Cons::new(letter::NUN), oshureq()])), // -ēnû
        (OBJ_3MP, emit(Tsere, &[Cons::new(letter::MEM)])),           // -ēm
        (OBJ_2MS, emit(Sheva, &[ocv(letter::KAF, Qamats)])),         // -ᵊḵā
        // The long -ḵâ spelling with the he mater (yakkᵊḵâ יַכְּכָה).
        (
            OBJ_2MS,
            emit(Sheva, &[ocv(letter::KAF, Qamats), Cons::new(letter::HE)]),
        ), // -ᵊḵâ
        (OBJ_2FS, emit(Tsere, &[ocv(letter::KAF, Sheva)])), // -ēḵ (וָאֶרְאֵךְ)
        // The energic set joins the elided-he stem on segol: yirʾennâ
        // יִרְאֶנָּה, ʾerʾennû אֶרְאֶנּוּ, ʾăṣawwekkā אֲצַוֶּךָּ.
        (
            OBJ_3MS,
            emit(Segol, &[Cons::new(letter::NUN).with_dagesh(), oshureq()]),
        ), // -ennû
        (
            OBJ_3FS,
            emit(
                Segol,
                &[
                    Cons::new(letter::NUN).with_dagesh().with_vowel(Qamats),
                    Cons::new(letter::HE),
                ],
            ),
        ), // -ennā
        (
            OBJ_2MS,
            emit(
                Segol,
                &[Cons::new(letter::KAF).with_dagesh().with_vowel(Qamats)],
            ),
        ), // -ekkā
    ]
}

/// Reduce the first stem vowel of a perfect base to sheva (propretonic
/// reduction when stress shifts onto a suffix): nāṯattî → nᵊṯattî, bāḥartî →
/// bᵊḥartî. A word-initial guttural takes hataf-patah instead of a vocal sheva.
/// Mutates `seq[0]`'s vowel in place. (Strong/most weak perfects carry the C1
/// vowel on `seq[0]`; the few prefixed shapes — none in the perfect — are not a
/// concern here.)
fn reduce_perfect_c1(seq: &mut [Cons]) {
    use Vowel::*;
    if let Some(first) = seq.first_mut()
        && matches!(first.vowel, Some(Qamats | Patah))
    {
        first.vowel = Some(if hebrew::is_guttural(first.letter) {
            HatafPatah
        } else {
            Sheva
        });
    }
}

/// Perfect with a **non-3ms subject** carrying a pronominal object suffix, built
/// additively from the bare perfect `base_text` so any weak-root reshaping in
/// the base carries through. Three productive subject endings are modelled (the
/// regular, high-frequency ones); the rare/irregular 2mp -tu and 3fs -at hosts
/// are left to a later pass:
///
///   * **1cs** -tî (תִּי): suffix joins the retained î — nᵊṯattîḵā (נְתַתִּיךָ),
///     bᵊḥartîḵā (בְּחַרְתִּיךָ), yᵊḏaʕtîm (יְדַעְתִּים), ʾăḥaztîw (אֲחַזְתִּיו).
///   * **2ms** -tā (תָּ → -ta before the suffix): ʾăhaḇtānû (אֲהַבְתָּנוּ),
///     gᵊmaltanî (גְּמַלְתַּנִי), bᵊrāʾtām (בְּרָאתָם).
///   * **3cp** -û (וּ): ʾăhēḇûḵā (אֲהֵבוּךָ), ʾăḥāzûnî (אֲחָזוּנִי) — the same
///     vocalic-subject linking as the imperfect plural.
///
/// C1 reduces propretonically (qamats/patah → sheva, or hataf-patah under a
/// guttural). As elsewhere this is generate-and-test: a mis-modelled connecting
/// vowel merely fails to match, never mis-parses.
fn perfect_subject_object_suffixes(
    base_text: &str,
    pgn: Pgn,
    root: &Root,
    binyan: Binyan,
) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let seq = hebrew::parse_pointed(base_text);
    let n = seq.len();
    if n < 3 {
        return Vec::new();
    }
    let last = seq[n - 1];
    let mut out = Vec::new();
    let is = |p: Person, g: Gender, num: Number| pgn == Pgn::new(p, g, num);

    if is(Person::First, Gender::Common, Number::Singular) {
        // 1cs base ends in -tî: tav(+dagesh, hiriq) + yod mater. The suffix
        // attaches to the retained î (the final yod), so we just append the
        // tail consonants. 3ms is -w (a bare vav on the î), 3fs -hā.
        if !(last.letter == letter::YOD && seq[n - 2].vowel == Some(Hiriq)) {
            return out;
        }
        let mut base = seq.clone();
        reduce_perfect_c1(&mut base);
        let mut push = |obj: Pgn, tail: &[Cons]| {
            let mut s = base.clone();
            s.extend_from_slice(tail);
            out.push((obj, hebrew::render(&s)));
        };
        push(OBJ_2MS, &[ocv(letter::KAF, Qamats)]);
        push(OBJ_2FS, &[ocv(letter::KAF, Sheva)]);
        push(OBJ_3MS, &[Cons::new(letter::VAV)]);
        push(OBJ_3FS, &[ocv(letter::HE, Qamats)]);
        push(OBJ_1CP, &[Cons::new(letter::NUN), oshureq()]);
        push(OBJ_3MP, &[Cons::new(letter::MEM)]);
        push(OBJ_3FP, &[Cons::new(letter::NUN)]);
    } else if is(Person::Second, Gender::Masculine, Number::Singular) {
        // 2ms base ends in -tā (tav + qamats); before a suffix the qamats
        // shortens to patah. Build from the base, retune the final vowel, then
        // append the tail. (No paragogic-he base here — that would end in HE.)
        if !(last.letter == letter::TAV && last.vowel == Some(Qamats)) {
            return out;
        }
        let mut base = seq.clone();
        reduce_perfect_c1(&mut base);
        // The -tā theme appears both shortened to patah (gᵊmaltanî) and
        // retained as qamats (ʾăhaḇtānû) before a suffix; emit both.
        let mut push = |obj: Pgn, tail: &[Cons]| {
            for tv in [Patah, Qamats] {
                let mut s = base.clone();
                if let Some(c) = s.last_mut() {
                    c.vowel = Some(tv);
                }
                s.extend_from_slice(tail);
                out.push((obj, hebrew::render(&s)));
            }
        };
        push(OBJ_1CS, &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)]);
        push(OBJ_1CP, &[Cons::new(letter::NUN), oshureq()]);
        push(OBJ_3MS, &[Cons::new(letter::HE), oshureq()]);
        push(OBJ_3FS, &[ocv(letter::HE, Qamats)]);
        push(OBJ_3MP, &[Cons::new(letter::MEM)]);
        // Contracted 3ms -tô (וַאֲסַפְתּוֹ, וְקִדַּשְׁתּוֹ) and 3fs -tāh: the -tā
        // theme elides into the suffix vowel.
        let mut s = base.clone();
        if let Some(c) = s.last_mut() {
            c.vowel = None;
        }
        s.push(Cons::new(letter::VAV).with_vowel(Holam));
        out.push((OBJ_3MS, hebrew::render(&s)));
        let mut s = base.clone();
        if let Some(c) = s.last_mut() {
            c.vowel = Some(Qamats);
        }
        s.push(Cons::new(letter::HE).with_dagesh());
        out.push((OBJ_3FS, hebrew::render(&s)));
    } else if is(Person::Third, Gender::Feminine, Number::Singular) {
        // 3fs host: the suffixed stem is qᵊṭālat- (the old -at afformative
        // restored): ʾăḵālatHû אֲכָלָתְהוּ, yᵊlāḏatḵā יְלָדַתְךָ, the contracted
        // -attû גְּנָבַתּוּ, and -āṯam וַאֲכָלָתַם. Strong/I-guttural shapes only.
        let (c1, c2, c3) = (root.pe(), root.ayin(), root.lamed());
        // Hiphil 3fs host: the heqṭîlâ base restores the -aṯ afformative before
        // a suffix — heḥĕzîqaṯhû הֶחֱזִיקַתְהוּ, hēmîṯāṯhû הֱמִיתָתְהוּ,
        // hiḏbîqāṯhû הִדְבִּיקָתְהוּ. The heqṭîl stem vowel does not shift, so we
        // derive the host straight from the 3fs text: drop its final -â (the
        // qamats sits on the last radical, then a he mater), re-point that
        // radical, and attach the afformative tav + suffix.
        if binyan == Binyan::Hiphil
            && !matches!(c3, letter::HE | letter::VAV | letter::YOD)
        {
            let mut prefix = hebrew::parse_pointed(base_text);
            if prefix.last().map(|c| c.letter) == Some(letter::HE) {
                prefix.pop();
            }
            if prefix.len() < 2 {
                return out;
            }
            // The heavy afformative+suffix shifts the stress two syllables
            // forward, reducing the hollow Hiphil's propretonic preformative
            // tsere to hataf-segol (hēmîṯ → hĕmîṯ, hēṣîq → hĕṣîq); emit both.
            let mut prefixes = vec![prefix.clone()];
            if prefix.first().map(|c| c.letter == letter::HE && c.vowel == Some(Tsere))
                == Some(true)
            {
                let mut reduced = prefix.clone();
                reduced[0].vowel = Some(HatafSegol);
                prefixes.push(reduced);
            }
            for prefix in &prefixes {
            for c3v in [Patah, Qamats] {
                let mut push = |obj: Pgn, tv: Option<Vowel>, dagesh: bool, tail: &[Cons]| {
                    let mut s = prefix.clone();
                    if let Some(c) = s.last_mut() {
                        c.vowel = Some(c3v);
                    }
                    let mut t = Cons::new(letter::TAV);
                    t.vowel = tv;
                    if dagesh {
                        t = t.with_dagesh();
                    }
                    s.push(t);
                    s.extend_from_slice(tail);
                    out.push((obj, hebrew::render(&s)));
                };
                push(OBJ_3MS, Some(Sheva), false, &[Cons::new(letter::HE), oshureq()]);
                push(OBJ_3MS, None, true, &[oshureq()]);
                push(
                    OBJ_1CS,
                    Some(Sheva),
                    false,
                    &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
                );
                push(OBJ_2MS, Some(Sheva), false, &[ocv(letter::KAF, Qamats)]);
                push(
                    OBJ_3FS,
                    Some(Qamats),
                    true,
                    &[Cons::new(letter::HE).with_dagesh()],
                );
                push(OBJ_3MP, Some(Patah), false, &[Cons::new(letter::MEM)]);
                push(OBJ_1CP, Some(Sheva), false, &[Cons::new(letter::NUN), oshureq()]);
            }
            }
            return out;
        }
        if matches!(c3, letter::HE | letter::ALEF | letter::VAV | letter::YOD)
            || matches!(c2, letter::VAV | letter::YOD)
            || c1 == letter::VAV
        {
            return out;
        }
        // C3 carries patah or qamats depending on stress; emit both.
        for c3v in [Patah, Qamats] {
            let stem = |tv: Option<Vowel>, dagesh: bool| {
                let mut t = Cons::new(letter::TAV);
                t.vowel = tv;
                if dagesh {
                    t = t.with_dagesh();
                }
                let mut s = vec![
                    Cons::radical(c1, 1).with_vowel(Sheva),
                    Cons::radical(c2, 2).with_vowel(Qamats),
                    Cons::radical(c3, 3).with_vowel(c3v),
                    t,
                ];
                apply_guttural(&mut s, root);
                s
            };
            let mut push = |obj: Pgn, tv: Option<Vowel>, dagesh: bool, tail: &[Cons]| {
                let mut s = stem(tv, dagesh);
                s.extend_from_slice(tail);
                out.push((obj, hebrew::render(&s)));
            };
            push(
                OBJ_3MS,
                Some(Sheva),
                false,
                &[Cons::new(letter::HE), oshureq()],
            );
            push(OBJ_3MS, None, true, &[oshureq()]);
            push(
                OBJ_1CS,
                Some(Sheva),
                false,
                &[
                    Cons::new(letter::NUN).with_vowel(Hiriq),
                    Cons::new(letter::YOD),
                ],
            );
            push(OBJ_2MS, Some(Sheva), false, &[ocv(letter::KAF, Qamats)]);
            push(
                OBJ_3FS,
                Some(Qamats),
                true,
                &[Cons::new(letter::HE).with_dagesh()],
            );
            push(OBJ_3MP, Some(Patah), false, &[Cons::new(letter::MEM)]);
            push(
                OBJ_1CP,
                Some(Sheva),
                false,
                &[Cons::new(letter::NUN), oshureq()],
            );
        }
    } else if is(Person::First, Gender::Common, Number::Plural) {
        // 1cp host: the -nû afformative carries the suffix on a qubuts/û —
        // dᵊrašnuhû דְרַשְׁנֻהוּ, ḥăšaḇnuhû חֲשַׁבְנֻהוּ. Strong shapes only.
        let (c1, c2, c3) = (root.pe(), root.ayin(), root.lamed());
        if matches!(c3, letter::HE | letter::ALEF | letter::VAV | letter::YOD)
            || matches!(c2, letter::VAV | letter::YOD)
            || c1 == letter::VAV
        {
            return out;
        }
        let tails: &[(Pgn, &[Cons])] = &[
            (
                OBJ_3MS,
                &[Cons::new(letter::HE), Cons::new(letter::VAV).with_dagesh()],
            ),
            (OBJ_3FS, &[Cons::new(letter::HE).with_vowel(Qamats)]),
            (OBJ_3MP, &[Cons::new(letter::MEM)]),
        ];
        for &(obj, tail) in tails {
            for nv in [Some(Qubuts), None] {
                let mut nun = Cons::new(letter::NUN);
                nun.vowel = nv;
                let mut s = vec![
                    Cons::radical(c1, 1).with_vowel(Sheva),
                    Cons::radical(c2, 2).with_vowel(Patah),
                    Cons::radical(c3, 3).with_vowel(Sheva),
                    nun,
                ];
                if nv.is_none() {
                    s.push(oshureq());
                }
                s.extend_from_slice(tail);
                apply_guttural(&mut s, root);
                out.push((obj, hebrew::render(&s)));
            }
        }
    } else if is(Person::Third, Gender::Common, Number::Plural) && binyan == Binyan::Qal {
        // 3cp base ends in -û (vav-shureq). Before a suffix the connecting stem
        // is qᵊṭāl-û (C1 sheva, C2 *qamats* restored — not the bare 3cp's
        // reduced C2): šᵊp̄āṭûnû (שְׁפָטוּנוּ), gᵊnāḇûḵā (גְּנָבוּךָ), ʕăzāḇûnî
        // (עֲזָבוּנִי). Build from the radicals like qal_perfect_object_suffixes,
        // then let apply_guttural fix a guttural C1/C2. Strong shapes only.
        if !(last.letter == letter::VAV && last.dagesh && last.vowel.is_none()) {
            return out;
        }
        let (c1, c2, c3) = (root.pe(), root.ayin(), root.lamed());
        // The Qal perfect of I-yod/I-nun roots is regular (yāḏaʕ, nāp̄al), so the
        // qᵊṭāl-û connecting stem builds straight from the radicals — only the
        // truly reshaping classes (III-weak, hollow) are excluded. A quiescent
        // III-aleph keeps the shape (mᵊṣāʾûnî מְצָאוּנִי).
        if matches!(c3, letter::HE | letter::VAV | letter::YOD)
            || matches!(c2, letter::VAV | letter::YOD)
            || c1 == letter::VAV
        {
            return out;
        }
        let tails: &[(Pgn, &[Cons])] = &[
            (
                OBJ_3MS,
                &[Cons::new(letter::HE), Cons::new(letter::VAV).with_dagesh()],
            ),
            (OBJ_3FS, &[Cons::new(letter::HE).with_vowel(Qamats)]),
            (
                OBJ_1CS,
                &[
                    Cons::new(letter::NUN).with_vowel(Hiriq),
                    Cons::new(letter::YOD),
                ],
            ),
            (OBJ_2MS, &[Cons::new(letter::KAF).with_vowel(Qamats)]),
            (
                OBJ_1CP,
                &[Cons::new(letter::NUN), Cons::new(letter::VAV).with_dagesh()],
            ),
            (OBJ_3MP, &[Cons::new(letter::MEM)]),
        ];
        for &(obj, tail) in tails {
            // -û on the C3 (qᵊṭāl-û) written plene (vav-shureq) or defectively
            // (qubuts on C3, never on a quiescent aleph); both occur. The
            // statives restore tsere instead of qamats (šᵊḵēḥûnî שְׁכֵחוּנִי,
            // ʾăhēḇûḵā אֲהֵבוּךָ); emit both grades.
            for c2v in [Qamats, Tsere] {
                for c3v in [None, Some(Qubuts)] {
                    if c3 == letter::ALEF && c3v.is_some() {
                        continue;
                    }
                    let mut seq = vec![
                        Cons::radical(c1, 1).with_vowel(Sheva),
                        Cons::radical(c2, 2).with_vowel(c2v),
                        {
                            let mut c = Cons::radical(c3, 3);
                            c.vowel = c3v;
                            c
                        },
                    ];
                    if c3v.is_none() {
                        seq.push(oshureq());
                    }
                    seq.extend_from_slice(tail);
                    apply_guttural(&mut seq, root);
                    out.push((obj, hebrew::render(&seq)));
                }
            }
        }
    }
    // Pe-yod perfect hosts reduce C1 to sheva before a suffix; the C2 theme patah
    // can then surface as hiriq — wîrištām וִירִשְׁתָּם, wîrištāh וִירִשְׁתָּהּ
    // (2ms ירש + suffix). Emit the hiriq twin of every patah-themed host.
    if root.has(Gizra::PeYod) {
        let extra: Vec<(Pgn, String)> = out
            .iter()
            .filter_map(|(p, t)| pe_yod_perfect_heavy_hiriq_variant(t).map(|h| (*p, h)))
            .collect();
        out.extend(extra);
    }
    out
}

/// Active/passive **participle (ms)** with a pronominal suffix. A participle is
/// noun-like, so it takes the nominal/possessive suffix set (-î, -ᵊḵā, -ô, …),
/// not the verbal object set. The thematic vowel before the final radical
/// reduces to (vocal) sheva and the suffix joins the last radical:
///
///   * **Strong** qōṭēl → qōṭl-: šōmerḵā (שֹׁמֶרְךָ), ʾōyḇî (אֹיְבִי),
///     ʾōyḇô (אֹיְבוֹ), bôzēhû (בּוֹזֵהוּ). The qamats/tsere theme is emitted both
///     reduced to sheva and lowered to segol (the heavy-suffix segolate twin
///     šōmerḵā), so both Masoretic pointings match.
///   * **III-He** mᵊṣawwê → mᵊṣawwᵊ-: the etymological final he is dropped and
///     the preceding radical reduces, then the suffix attaches — mᵊṣawwᵊḵā
///     (מְצַוְּךָ, Piel ptcp of צוה + 2ms). Same nominal set.
///
/// `base_text` is the bare ms participle surface for this (binyan); reverse
/// parsing only needs the right spelling, and additive emission means a
/// mis-modelled grade simply fails to match.
fn participle_object_suffixes(base_text: &str) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let seq = hebrew::parse_pointed(base_text);
    let n = seq.len();
    if n < 2 {
        return Vec::new();
    }
    let last = seq[n - 1];
    let mut out = Vec::new();

    // Two-letter hollow participles (mēṯ מֵת, qām קָם): no theme to reduce —
    // the suffix joins the final radical directly (mēṯeḵā מֵתֶךָ).
    if n == 2 {
        if last.vowel.is_some() || matches!(last.letter, letter::HE | letter::VAV | letter::YOD) {
            return out;
        }
        for (obj, link, tail) in nominal_suffix_tails() {
            let mut s = seq.clone();
            s[1].vowel = link;
            s.extend(tail);
            out.push((obj, hebrew::render(&s)));
        }
        return out;
    }

    // III-He participle (ends in a bare HE mater, the radical before it carrying
    // the theme): drop the he, reduce the now-final radical to sheva, attach the
    // nominal tail. mᵊṣawwê (מְצַוֶּה) → mᵊṣawwᵊ-ḵā (מְצַוְּךָ).
    if last.letter == letter::HE && matches!(seq[n - 2].vowel, Some(Segol | Tsere)) {
        let stem = seq[..n - 1].to_vec();
        for (obj, link, tail) in nominal_suffix_tails() {
            let mut s = stem.clone();
            // The radical takes the linking vowel; the -ô 3ms (link None) leaves
            // it on a sheva with the vav-holam mater following.
            if let Some(c) = s.last_mut() {
                c.vowel = link.or(Some(Sheva));
            }
            s.extend(tail);
            out.push((obj, hebrew::render(&s)));
        }
        // The verbal-style tails also occur on the tsere link: ʿōśēhû
        // עֹשֵׂהוּ, ʿōśēnî.
        let mut emit = |obj: Pgn, tail: &[Cons]| {
            let mut s = stem.clone();
            if let Some(c) = s.last_mut() {
                c.vowel = Some(Tsere);
            }
            s.extend_from_slice(tail);
            out.push((obj, hebrew::render(&s)));
        };
        emit(OBJ_3MS, &[Cons::new(letter::HE), oshureq()]);
        emit(OBJ_1CS, &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)]);
        return out;
    }

    // An î-stem participle (Hiphil môšîaʿ מוֹשִׁיעַ, môṣîʾ מוֹצִיא, mēqîm) carries
    // a long hiriq written with a yod mater immediately before C3. That mater
    // is fixed — it does not reduce — so the suffix joins C3 directly and the
    // yod stays a bare mater: môšîʿēḵ מוֹשִׁיעֵךְ, môšîʿēnî. A quiescent aleph C3
    // takes a hataf-patah for the sheva-link suffixes (môṣîʾăḵā מוֹצִיאֲךָ) and
    // quiesces (bare) before the -ô. Runs *before* the strong-skip below, which
    // would otherwise bail on the aleph/furtive final and never emit these.
    if seq[n - 2].letter == letter::YOD
        && seq[n - 2].vowel.is_none()
        && seq.get(n - 3).and_then(|c| c.vowel) == Some(Hiriq)
        && !matches!(last.letter, letter::HE | letter::VAV | letter::YOD)
    {
        let aleph = last.letter == letter::ALEF;
        for (obj, link, tail) in nominal_suffix_tails() {
            let link = if aleph && link == Some(Sheva) {
                Some(HatafPatah)
            } else {
                link
            };
            let mut s = seq.clone();
            s[n - 1].vowel = link;
            s.extend(tail);
            out.push((obj, hebrew::render(&s)));
        }
        return out;
    }
    // Strong participle: ends in a true final radical. Skip mater/weak finals.
    if matches!(last.vowel, Some(v) if v != Vowel::Sheva)
        || matches!(
            last.letter,
            letter::HE | letter::ALEF | letter::VAV | letter::YOD
        )
    {
        return out;
    }
    // The theme vowel sits on the consonant before C3 (seq[n-2]); reduce it to
    // sheva, and also emit the segol twin for the heavy 2ms/2fs/2mp suffixes
    // (šōmerḵā) and the hiriq twin a yod theme consonant prefers (ʾōyiḇḵā
    // אֹיִבְךָ). A guttural theme consonant takes a hataf-patah for its
    // reduced grade instead — gōʾălēḵ (גֹּאֲלֵךְ). The suffix joins C3.
    let themes: &[Vowel] = if hebrew::is_guttural(seq[n - 2].letter) {
        &[Sheva, Segol, Hiriq, HatafPatah]
    } else {
        &[Sheva, Segol, Hiriq]
    };
    for &theme in themes {
        for (obj, link, tail) in nominal_suffix_tails() {
            let mut s = seq.clone();
            s[n - 2].vowel = Some(theme);
            s[n - 1].vowel = link;
            s.extend(tail);
            out.push((obj, hebrew::render(&s)));
        }
    }
    out
}

/// Active/passive **masculine-plural participle** with a pronominal suffix.
/// Built from the bare absolute surface (qōṭlîm קֹטְלִים) via the construct
/// base (qōṭlê- קֹטְלֵי-).
fn participle_mp_object_suffixes(base_text: &str) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let mut out = Vec::new();
    let seq = hebrew::parse_pointed(base_text);
    // Convert to construct form (יֹשְׁבִים → יֹשְׁבֵי).
    let Some(construct) = participle_mp_construct(&seq) else {
        return out;
    };

    // Construct plural -ê (tsere-yod) followed by nominal suffixes:
    //   -ê-hem (3mp) → פְּקֻדֵיהֶם
    //   -ê-ḵem (2mp) → פְּקֻדֵיכֶם
    //   -ê-nu  (1cp) → פְּקֻדֵינוּ
    //   -ê-ḵā  (2ms) → פְּקֻדֶיךָ
    //   -â-yw  (3ms) → פְּקֻדָיו
    // As with the ms pass, we emit several plausible spellings.

    // -êhem (3mp): construct + he-segol-mem.
    let mut s3mp = construct.clone();
    s3mp.push(Cons::new(letter::HE).with_vowel(Segol));
    s3mp.push(Cons::new(letter::MEM));
    out.push((OBJ_3MP, hebrew::render(&s3mp)));

    // -êḵem (2mp): construct + kaf-segol-mem — ʾōyᵊḇêḵem אֹיְבֵיכֶם.
    let mut s2mp = construct.clone();
    s2mp.push(Cons::new(letter::KAF).with_vowel(Segol));
    s2mp.push(Cons::new(letter::MEM));
    out.push((OBJ_2MP, hebrew::render(&s2mp)));

    // -êhen (3fp): construct + he-segol-nun.
    let mut s3fp = construct.clone();
    s3fp.push(Cons::new(letter::HE).with_vowel(Segol));
    s3fp.push(Cons::new(letter::NUN));
    out.push((OBJ_3FP, hebrew::render(&s3fp)));

    // -eyhā (3fs): construct (reduced tsere → segol) + he-qamats — yōšᵊḇeyhā
    // יֹשְׁבֶיהָ.
    let mut s3fs = construct.clone();
    if let Some(c) = s3fs.iter_mut().rev().nth(1) {
        c.vowel = Some(Segol);
    }
    s3fs.push(Cons::new(letter::HE).with_vowel(Qamats));
    out.push((OBJ_3FS, hebrew::render(&s3fs)));

    // -ayiḵ (2fs): construct tsere lowers to patah, the yod takes hiriq, final
    // kaf — and the pausal qamats grade (rōḵᵊlāyiḵ רֹכְלָיִךְ).
    for v in [Patah, Qamats] {
        let mut s2fs = construct.clone();
        if let Some(c) = s2fs.iter_mut().rev().nth(1) {
            c.vowel = Some(v);
        }
        if let Some(c) = s2fs.last_mut() {
            c.vowel = Some(Hiriq);
        }
        s2fs.push(Cons::new(letter::KAF).with_vowel(Sheva));
        out.push((OBJ_2FS, hebrew::render(&s2fs)));
    }

    // -ênû (1cp): construct + nun-shureq.
    let mut s1cp = construct.clone();
    s1cp.push(Cons::new(letter::NUN));
    s1cp.push(Cons::new(letter::VAV).with_dagesh());
    out.push((OBJ_1CP, hebrew::render(&s1cp)));

    // -eyḵā (2ms): construct (reduced tsere → segol) + kaf-qamats.
    let mut s2ms = construct.clone();
    if let Some(c) = s2ms.iter_mut().rev().nth(1) {
        c.vowel = Some(Segol);
    }
    s2ms.push(Cons::new(letter::KAF).with_vowel(Qamats));
    out.push((OBJ_2MS, hebrew::render(&s2ms)));

    // -āyw (3ms): construct (reduced tsere-yod → qamats-yod) + vav mater.
    let mut s3ms = construct.clone();
    if let Some(c) = s3ms.iter_mut().rev().nth(1) {
        c.vowel = Some(Qamats);
    }
    s3ms.push(Cons::new(letter::VAV));
    out.push((OBJ_3MS, hebrew::render(&s3ms)));

    // -ay (1cs): the construct tsere-yod lowers to patah-yod — ʾōyᵊḇay אֹיְבַי,
    // sûsay. The final yod (already present in the construct) is the suffix.
    let mut s1cs = construct.clone();
    if let Some(c) = s1cs.iter_mut().rev().nth(1) {
        c.vowel = Some(Patah);
    }
    out.push((OBJ_1CS, hebrew::render(&s1cs)));

    // The pausal -āy grade (ṣōrᵊrāy צֹרְרָי, qāmāy קָמָי).
    let mut s1cs_pausal = construct;
    if let Some(c) = s1cs_pausal.iter_mut().rev().nth(1) {
        c.vowel = Some(Qamats);
    }
    out.push((OBJ_1CS, hebrew::render(&s1cs_pausal)));

    out
}

/// Active/passive **feminine-singular participle** with a pronominal suffix.
/// The segolate -eṯ shape shifts to its -at construct grade (sōḥereṯ →
/// sōḥart-) and takes the nominal set on a dagesh-lene tav — sōḥartēḵ
/// סֹחַרְתֵּךְ.
fn participle_fs_object_suffixes(base_text: &str) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let seq = hebrew::parse_pointed(base_text);
    let n = seq.len();
    if n < 3
        || seq[n - 1].letter != letter::TAV
        || seq[n - 1].vowel.is_some()
        || seq[n - 2].vowel != Some(Segol)
        || seq[n - 3].vowel != Some(Segol)
    {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (obj, link, tail) in nominal_suffix_tails() {
        let mut s = seq.clone();
        s[n - 3].vowel = Some(Patah);
        s[n - 2].vowel = Some(Sheva);
        s[n - 1].vowel = link;
        s[n - 1].dagesh = true;
        s.extend(tail);
        out.push((obj, hebrew::render(&s)));
    }
    out
}

/// Active/passive **feminine-plural participle** with a pronominal suffix.
/// Built from the bare absolute -ôṯ surface (נִפְלָאוֹת), whose construct is
/// identical except that the pretonic qamats reduces to vocal sheva
/// (nip̄lᵊʾôṯ-): both grades are emitted as hosts and the plural-noun suffix
/// set joins the ôṯ — nip̄lᵊʾôṯāyw נִפְלְאוֹתָיו, nip̄lᵊʾôṯeyḵā נִפְלְאוֹתֶיךָ.
fn participle_fp_object_suffixes(base_text: &str) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let seq = hebrew::parse_pointed(base_text);
    let n = seq.len();
    // Require the -ôṯ ending: a final vowelless tav after a holam (written on a
    // vav mater or directly on the previous consonant).
    if n < 3 || seq[n - 1].letter != letter::TAV || seq[n - 1].vowel.is_some() {
        return Vec::new();
    }
    let holam = (seq[n - 2].letter == letter::VAV && seq[n - 2].vowel == Some(Holam))
        || seq[n - 2].vowel == Some(Holam);
    if !holam {
        return Vec::new();
    }
    // Hosts: the absolute, plus the construct grade with the last qamats before
    // the ôṯ reduced to sheva.
    let mut hosts = vec![seq.clone()];
    if let Some(c) = seq[..n - 2].iter().rposition(|c| c.vowel == Some(Qamats)) {
        let mut reduced = seq.clone();
        reduced[c].vowel = Some(Sheva);
        hosts.push(reduced);
    }
    // Plural-noun suffix tails joining the tav.
    let tails: Vec<(Pgn, Vowel, Vec<Cons>)> = vec![
        (OBJ_1CS, Patah, vec![Cons::new(letter::YOD)]), // -ay
        (
            OBJ_2MS,
            Segol,
            vec![Cons::new(letter::YOD), ocv(letter::KAF, Qamats)],
        ), // -eyḵā
        (
            OBJ_3MS,
            Qamats,
            vec![Cons::new(letter::YOD), Cons::new(letter::VAV)],
        ), // -āyw
        (
            OBJ_3FS,
            Segol,
            vec![Cons::new(letter::YOD), ocv(letter::HE, Qamats)],
        ), // -eyhā
        (
            OBJ_1CP,
            Tsere,
            vec![Cons::new(letter::YOD), Cons::new(letter::NUN), oshureq()],
        ), // -ênû
        (
            OBJ_2MP,
            Tsere,
            vec![
                Cons::new(letter::YOD),
                ocv(letter::KAF, Segol),
                Cons::new(letter::MEM),
            ],
        ), // -êḵem
        (
            OBJ_3MP,
            Tsere,
            vec![
                Cons::new(letter::YOD),
                ocv(letter::HE, Segol),
                Cons::new(letter::MEM),
            ],
        ), // -êhem
        (OBJ_3MP, Qamats, vec![Cons::new(letter::MEM)]), // -ām
    ];
    let mut out = Vec::new();
    for host in &hosts {
        for (obj, link, tail) in &tails {
            let mut s = host.clone();
            s[n - 1].vowel = Some(*link);
            s.extend(tail.iter().cloned());
            out.push((*obj, hebrew::render(&s)));
        }
    }
    out
}

/// Imperfect-family (Imperfect / Jussive / Wayyiqtol) 3ms with a pronominal
/// object suffix, built from the bare 3ms `base_text` so weak-root prefixes and
/// gemination already in the base carry through. The theme vowel reduces and the
/// suffix joins C3: yiqṭᵊl-ḗnî / -énnî (1cs), -ᵊḵā (2ms, e.g. yᵊḇāreḵḵā
/// יְבָרֶכְךָ), -ḗhû / -énnû (3ms), -ḗm (3mp). Two theme reductions (sheva and
/// segol) are emitted per suffix and exact-match keeps the real one. Skipped
/// when the base does not end in a true final consonant (III-He/-Aleph matres).
fn imperfect_object_suffixes(base_text: &str, _root: &Root) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let seq = hebrew::parse_pointed(base_text);
    let n = seq.len();
    if n < 3 {
        return Vec::new();
    }
    let last = seq[n - 1];
    // A III-guttural verb's bare 3ms ends in that guttural carrying patah — the
    // a-theme (Qal yišmaʕ יִשְׁמַע) or the guttural-lowered Piel theme (yᵉšallaḥ
    // יְשַׁלַּח). The suffixed forms reduce that theme and the guttural takes the
    // link vowel (yišmāʕēnî יִשְׁמָעֵנִי, yᵉšallᵊḥēm יְשַׁלְּחֵם), so allow this
    // final guttural+patah through.
    let guttural_a = matches!(last.letter, letter::HET | letter::AYIN) && last.vowel == Some(Patah);
    // A Hiphil long-î host whose C3 is a quiescent aleph (the doubly-weak hollow
    // III-aleph בוא: yāḇîʔ יָבִיא): the suffix joins that aleph (yᵊḇîʔēm יְבִיאֵם),
    // so let a final aleph through when the …C(hiriq)-YOD-aleph shape is present.
    let plene_i = n >= 4
        && seq[n - 2].letter == letter::YOD
        && seq[n - 2].vowel.is_none()
        && seq[n - 3].vowel == Some(Hiriq);
    // The hollow long û/ô themes behave the same way: the mater is kept and
    // the suffix joins C3 directly, with the qamats preformative reducing —
    // yāšûr → yᵉšûrennā יְשׁוּרֶנָּה, yāšûp̄ → yᵉšûp̄ēnî יְשׁוּפֵנִי.
    let long_mater = plene_i
        || (n >= 3
            && seq[n - 2].letter == letter::VAV
            && (seq[n - 2].vowel == Some(Holam)
                || (seq[n - 2].dagesh && seq[n - 2].vowel.is_none())));
    // Otherwise the bare 3ms ends in a true final consonant (C3 of yiqṭōl, the
    // nun of wayyittēn): vowelless, or the Masoretic silent sheva `render` writes
    // under a final kaf (יְבָרֵךְ). A real vowel or a mater final (HE/ALEF, or a
    // vowelless VAV/YOD) means a weak/derived shape we don't model here.
    // A III-aleph host (yimṣāʾ יִמְצָא): the theme qamats stays and the suffix
    // joins the quiescent aleph directly — yimṣāʾḗhû וַיִּמְצָאֵהוּ.
    let lamed_aleph_host = last.letter == letter::ALEF
        && last.vowel.is_none()
        && n >= 3
        && seq[n - 2].vowel == Some(Qamats);
    if lamed_aleph_host {
        let mut out = Vec::new();
        let mut emit = |obj: Pgn, link: Vowel, tail: &[Cons]| {
            let mut s = seq.clone();
            s[n - 1].vowel = Some(link);
            s.extend_from_slice(tail);
            out.push((obj, hebrew::render(&s)));
        };
        emit(
            OBJ_1CS,
            Tsere,
            &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
        );
        emit(
            OBJ_1CS,
            Segol,
            &[
                Cons::new(letter::NUN).with_dagesh().with_vowel(Hiriq),
                Cons::new(letter::YOD),
            ],
        );
        emit(OBJ_2MS, HatafPatah, &[ocv(letter::KAF, Qamats)]);
        emit(OBJ_3MS, Tsere, &[Cons::new(letter::HE), oshureq()]);
        emit(
            OBJ_3MS,
            Segol,
            &[Cons::new(letter::NUN).with_dagesh(), oshureq()],
        );
        emit(OBJ_3FS, Segol, &[ocv(letter::HE, Qamats)]);
        // Mappiq-he 3fs: the theme qamats carries straight into the suffix
        // syllable — wayyimṣāʾāh וַיִּמְצָאָהּ.
        emit(OBJ_3FS, Qamats, &[Cons::new(letter::HE).with_dagesh()]);
        // Energic 2ms/3fs on the quiescent aleph (ʾeqrāʾekkā אֶקְרָאֶךָּ).
        emit(
            OBJ_2MS,
            Segol,
            &[Cons::new(letter::KAF).with_dagesh().with_vowel(Qamats)],
        );
        emit(
            OBJ_3FS,
            Segol,
            &[
                Cons::new(letter::NUN).with_dagesh().with_vowel(Qamats),
                Cons::new(letter::HE),
            ],
        );
        emit(OBJ_1CP, Tsere, &[Cons::new(letter::NUN), oshureq()]);
        emit(OBJ_3MP, Tsere, &[Cons::new(letter::MEM)]);
        return out;
    }
    if (matches!(last.vowel, Some(v) if v != Vowel::Sheva) && !guttural_a)
        || (matches!(last.letter, letter::HE | letter::ALEF) && !plene_i)
        || matches!(last.letter, letter::VAV | letter::YOD)
    {
        return Vec::new();
    }
    let mut out = Vec::new();
    // An a-theme base (C2 patah — the Qal III-guttural yišmaʕ, or a lexical
    // stative) lengthens that pretonic a to qamats in the open syllable before
    // the suffix (yišmāʕēnî יִשְׁמָעֵנִי), beside the sheva/segol the strong
    // reduction gives; emit all three so generate-and-test matches whichever the
    // surface uses. (Piel's C2 carries tsere, not patah, so it keeps sheva/segol.)
    let c2_patah = seq[n - 2].vowel == Some(Patah);
    let mut themes: Vec<Vowel> = if c2_patah {
        vec![Sheva, Segol, Qamats]
    } else {
        vec![Sheva, Segol]
    };
    // A I-aleph host whose quiescent aleph precedes the theme consonant
    // (tōʾḵal תֹּאכַל) reduces that theme to a hataf-patah under the suffix
    // (even on a non-guttural): tōʾḵălennû תֹּאכֲלֶנּוּ.
    if n >= 4 && seq[n - 3].letter == letter::ALEF && seq[n - 3].vowel.is_none() {
        themes.push(HatafPatah);
    }
    // An undageshable theme consonant (guttural or resh) reduces to a hataf
    // rather than a sheva: wayḇārăḵēhû וַיְבָרֲכֵהוּ.
    if hebrew::is_guttural(seq[n - 2].letter) || seq[n - 2].letter == letter::RESH {
        themes.push(HatafPatah);
    }
    // The Hiphil long hiriq-yod î theme before C3 (yaqrîḇ יַקְרִיב, yāšîḇ יָשִׁיב)
    // is *retained* under a suffix rather than reduced: yaqrîḇēhû יַקְרִיבֵהוּ,
    // yᵊšîḇennû יְשִׁיבֶנּוּ (`plene_i`, computed above). The suffix joins C3 with
    // the î kept; a qamats preformative (hollow yā-/ʾā-) reduces propretonically
    // (ʾāšîḇ → ʾăšîḇennû אֲשִׁיבֶנּוּ), the strong Hiphil's patah stays.
    let mut emit = |obj: Pgn, link: Vowel, tail: &[Cons]| {
        if long_mater {
            let mut s = seq.clone();
            if s[0].vowel == Some(Qamats) {
                s[0].vowel = Some(if hebrew::is_guttural(s[0].letter) {
                    HatafPatah
                } else {
                    Sheva
                });
            }
            // A guttural C3 can't carry the silent-sheva link of the -ḵā/-ḵem
            // suffixes; it takes a hataf-patah instead: yᵊḇîʾăḵā (יְבִיאֲךָ),
            // ʾôḏîʕăḵā (אוֹדִיעֲךָ), not יְבִיאְךָ / אוֹדִיעְךָ.
            let link = if link == Sheva && hebrew::is_guttural(s[n - 1].letter) {
                HatafPatah
            } else {
                link
            };
            s[n - 1].vowel = Some(link);
            s.extend_from_slice(tail);
            out.push((obj, hebrew::render(&s)));
            return;
        }
        // A guttural C3 can't carry the silent-sheva link of the -ḵā/-ḵem
        // suffixes; it takes a hataf-patah instead — ʾešlāḥăḵā אֶשְׁלָחֲךָ, not
        // אֶשְׁלָחְךָ.
        let link = if link == Sheva && hebrew::is_guttural(seq[n - 1].letter) {
            HatafPatah
        } else {
            link
        };
        for &theme in themes.iter() {
            let mut s = seq.clone();
            s[n - 2].vowel = Some(theme);
            s[n - 1].vowel = Some(link);
            s.extend_from_slice(tail);
            out.push((obj, hebrew::render(&s)));
        }
    };
    // 1cs: plain -ēnî and energic -ennî.
    emit(
        OBJ_1CS,
        Tsere,
        &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
    );
    emit(
        OBJ_1CS,
        Segol,
        &[
            Cons::new(letter::NUN).with_dagesh().with_vowel(Hiriq),
            Cons::new(letter::YOD),
        ],
    );
    // 2ms -ᵊḵā, the segol-link -eḵā (ʾămîṯeḵā אֲמִיתֶךָ, yišmāreḵā), the energic
    // -ekkā (ʾeʿezḇekkā אֶעֶזְבֶךָּ), 2fs -ēḵ.
    emit(OBJ_2MS, Sheva, &[ocv(letter::KAF, Qamats)]);
    emit(OBJ_2MS, Segol, &[ocv(letter::KAF, Qamats)]);
    emit(
        OBJ_2MS,
        Segol,
        &[Cons::new(letter::KAF).with_dagesh().with_vowel(Qamats)],
    );
    emit(OBJ_2FS, Sheva, &[ocv(letter::KAF, Sheva)]);
    // 2fs -ēḵ: the tsere link on C3 + a bare final kaf (yiḡʾālēḵ יִגְאָלֵךְ,
    // tōʔḵᵊlēḵ תֹּאכְלֵךְ) — the regular shape, beside the reduced -ᵊḵ above.
    emit(OBJ_2FS, Tsere, &[Cons::new(letter::KAF)]);
    // 3ms: plain -ēhû and energic -ennû.
    emit(OBJ_3MS, Tsere, &[Cons::new(letter::HE), oshureq()]);
    emit(
        OBJ_3MS,
        Segol,
        &[Cons::new(letter::NUN).with_dagesh(), oshureq()],
    );
    // 3fs: link-vowel -ehā, the mappiq-he -āh (wayyilkᵉdāh וַיִּלְכְּדָהּ), and
    // the energic -ennā (yišmᵊrennā, ʾettᵉnennā אֶתְּנֶנָּה, yᵉsîrennā).
    emit(OBJ_3FS, Segol, &[ocv(letter::HE, Qamats)]);
    emit(OBJ_3FS, Qamats, &[Cons::new(letter::HE).with_dagesh()]);
    emit(
        OBJ_3FS,
        Segol,
        &[
            Cons::new(letter::NUN).with_dagesh().with_vowel(Qamats),
            Cons::new(letter::HE),
        ],
    );
    // 1cp -ēnû.
    emit(OBJ_1CP, Tsere, &[Cons::new(letter::NUN), oshureq()]);
    // 3mp -ēm, and the archaic poetic -ēmô (tᵊšîṯēmô תְּשִׁיתֵמוֹ).
    emit(OBJ_3MP, Tsere, &[Cons::new(letter::MEM)]);
    emit(
        OBJ_3MP,
        Tsere,
        &[
            Cons::new(letter::MEM),
            Cons::new(letter::VAV).with_vowel(Holam),
        ],
    );
    // 2mp -ᵊḵem.
    emit(
        OBJ_2MP,
        Sheva,
        &[ocv(letter::KAF, Segol), Cons::new(letter::MEM)],
    );
    out
}

/// Imperfect-family verb (imperfect/jussive/wayyiqtol) with a **vocalic-suffix
/// subject** — the 3mp/2mp -û or 2fs -î plural-subject forms (yiqbᵊrû יִקְבְּרוּ)
/// — carrying a pronominal **object** suffix: "and they buried him"
/// wayyiqbᵊrûhû (וַיִּקְבְּרֻהוּ), "they will seek me" yᵊḇaqšûnî. The object
/// joins the retained subject vowel (-û or -î), which is written either plene
/// (a vav/yod mater: …רוּהוּ) or defectively (a bare qubuts/hiriq on the stem
/// consonant: …רֻהוּ); both spellings occur, so emit both. The subject PGN is
/// carried by the base form's own label, so only the object PGN varies here.
/// `base_text` is the bare 3mp/2mp/2fs surface; it must end in a vocalic mater
/// (vav-shureq for -û, or a bare yod for -î).
/// Object suffixes on a strong III-aleph plural imperfect (yimṣᵊʔû יִמְצְאוּ +
/// "him/them"). The C2 sheva lengthens to qamats and the quiescent aleph keeps
/// the subject û — plene on a vav (yimṣāʔûm יִמְצָאוּם) or defective as a qubuts
/// on the aleph (yimṣāʔuhû יִמְצָאֻהוּ). `seq` is the bare 3mp ending
/// `…C2(sheva) ALEF(∅) VAV(shureq)`; the caller has verified that shape.
fn lamed_aleph_vocalic_object_suffixes(seq: &[Cons]) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let n = seq.len();
    let tails: &[(Pgn, &[Cons])] = &[
        (
            OBJ_3MS,
            &[Cons::new(letter::HE), Cons::new(letter::VAV).with_dagesh()],
        ),
        (OBJ_3FS, &[Cons::new(letter::HE).with_vowel(Qamats)]),
        (
            OBJ_1CS,
            &[
                Cons::new(letter::NUN).with_vowel(Hiriq),
                Cons::new(letter::YOD),
            ],
        ),
        (OBJ_2MS, &[ocv(letter::KAF, Qamats)]),
        (
            OBJ_1CP,
            &[Cons::new(letter::NUN), Cons::new(letter::VAV).with_dagesh()],
        ),
        (OBJ_3MP, &[Cons::new(letter::MEM)]),
    ];
    let mut out = Vec::new();
    for &(obj, tail) in tails {
        // The derived stems keep the sheva theme under the suffix (Piel
        // wayḇaqqᵊšûm; with the forte dagesh dropped on the sheva, Piel מלא
        // gives וַיְמַלְאוּם), so emit the unrestored grade beside the Qal's
        // qamats restore.
        for theme in [Qamats, Sheva] {
            // Plene: keep the aleph + vav-shureq, append the suffix.
            let mut plene = seq.to_vec();
            plene[n - 3].vowel = Some(theme);
            plene.extend_from_slice(tail);
            out.push((obj, hebrew::render(&plene)));
            // Defective: drop the vav and put qubuts on the aleph.
            let mut defec = seq[..n - 1].to_vec();
            defec[n - 3].vowel = Some(theme);
            defec[n - 2].vowel = Some(Qubuts);
            defec.extend_from_slice(tail);
            out.push((obj, hebrew::render(&defec)));
        }
    }
    out
}

fn imperfect_vocalic_object_suffixes(base_text: &str) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let seq = hebrew::parse_pointed(base_text);
    let n = seq.len();
    if n < 3 {
        return Vec::new();
    }
    let last = seq[n - 1];
    // Identify the subject vowel from the final mater: vav-shureq (וּ, -û) or a
    // bare yod (-î). The consonant it sits on is seq[n-2].
    let (is_u, mater_vowel) = if last.letter == letter::VAV && last.dagesh && last.vowel.is_none() {
        (true, Qubuts)
    } else if last.letter == letter::YOD && last.vowel.is_none() {
        (false, Hiriq)
    } else {
        return Vec::new();
    };
    // The object-suffix tails that attach after a vocalic subject (-û/-î).
    // (Declared before the III-aleph branch, which reuses them.)
    // ── see `tails` below ──
    // The stem consonant under the subject vowel must be a true final consonant
    // (vowelless apart from the mater), i.e. a strong base — weak/III-he plural
    // bases (…וּ on a he/aleph, or a doubly-vocalic shape) are not modelled,
    // except the strong III-aleph plural (yimṣᵊʔû): the C2 sheva lengthens to
    // qamats and the quiescent aleph keeps the û (plene יִמְצָאוּם) or carries it
    // defectively (יִמְצָאֻהוּ). Handled by `lamed_aleph_vocalic_object_suffixes`.
    let stem = seq[n - 2];
    if is_u
        && stem.letter == letter::ALEF
        && stem.vowel.is_none()
        && seq[n - 3].vowel == Some(Sheva)
    {
        return lamed_aleph_vocalic_object_suffixes(&seq);
    }
    // The hollow-Hiphil long-î plural is the other quiescent-aleph exception
    // (yāḇîʾû וַיָּבִיאוּ, defective וַיְבִאוּ): the î stem is untouched and the
    // suffix joins the aleph's û exactly like the generic shape below — plene
    // וַיְבִיאוּהוּ or defective וַיְבִאֻהוּ — so let it through.
    let hollow_i_aleph = is_u
        && stem.letter == letter::ALEF
        && stem.vowel.is_none()
        && (seq[n - 3].vowel == Some(Hiriq)
            || (seq[n - 3].letter == letter::YOD
                && seq[n - 3].vowel.is_none()
                && n >= 4
                && seq[n - 4].vowel == Some(Hiriq)));
    // A quiescent aleph after a full vowel (the III-he perfect rāʾû רָאוּ, or
    // an already-restored III-aleph stem) also keeps its shape; the suffix
    // joins the û directly — rāʾûḵā רָאוּךָ.
    let aleph_after_full = is_u
        && stem.letter == letter::ALEF
        && stem.vowel.is_none()
        && matches!(seq[n - 3].vowel, Some(v) if v != Sheva);
    if (stem.vowel.is_some() || matches!(stem.letter, letter::HE | letter::ALEF))
        && !hollow_i_aleph
        && !aleph_after_full
    {
        return Vec::new();
    }
    // The object-suffix tails that attach after a vocalic subject (-û/-î).
    // The linking vowel is the subject vowel itself, already present, so each
    // tail is just the suffix consonants. 3ms -hû, 3fs -hā, 1cs -nî, 2ms -ḵā,
    // 1cp -nû, 3mp -m, 2mp -ḵem.
    let tails: &[(Pgn, &[Cons])] = &[
        (
            OBJ_3MS,
            &[Cons::new(letter::HE), Cons::new(letter::VAV).with_dagesh()],
        ),
        (OBJ_3FS, &[Cons::new(letter::HE).with_vowel(Qamats)]),
        (
            OBJ_1CS,
            &[
                Cons::new(letter::NUN).with_vowel(Hiriq),
                Cons::new(letter::YOD),
            ],
        ),
        (OBJ_2MS, &[Cons::new(letter::KAF).with_vowel(Qamats)]),
        (
            OBJ_1CP,
            &[Cons::new(letter::NUN), Cons::new(letter::VAV).with_dagesh()],
        ),
        (OBJ_3MP, &[Cons::new(letter::MEM)]),
    ];
    let mut out = Vec::new();
    for &(obj, tail) in tails {
        // Plene base: keep the mater (…רוּ) and append the suffix.
        let mut plene = seq.clone();
        plene.extend_from_slice(tail);
        out.push((obj, hebrew::render(&plene)));
        // Defective base: drop the mater and put the subject vowel directly on
        // the stem consonant (…רֻ), then append the suffix.
        let mut defec = seq.clone();
        defec.truncate(n - 1);
        if let Some(c) = defec.last_mut() {
            c.vowel = Some(mater_vowel);
        }
        defec.extend_from_slice(tail);
        out.push((obj, hebrew::render(&defec)));
    }
    // Energic (retained paragogic nun) plural variants: the -ûn imperfect keeps
    // its nun before a light object suffix instead of dropping it — yimṣāʾŭnᵊnî
    // (יִמְצָאֻנְנִי), yᵊšārᵊṯûneḵā (יְשָׁרְתוּנֶךָ). The nun links with sheva before
    // the nun-initial 1cs/1cp suffix (the doubled נְנִי/נְנוּ spelling) and with
    // segol before the 2ms/2fs kaf. Only on a -û plural subject.
    if is_u {
        let energic: &[(Pgn, Vowel, &[Cons])] = &[
            (
                OBJ_1CS,
                Sheva,
                &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
            ),
            (OBJ_1CP, Sheva, &[Cons::new(letter::NUN), oshureq()]),
            (OBJ_2MS, Segol, &[ocv(letter::KAF, Qamats)]),
            (OBJ_2FS, Segol, &[ocv(letter::KAF, Sheva)]),
        ];
        for &(obj, nv, tail) in energic {
            // Plene base (…וּן-): keep the mater, add the energic nun + suffix.
            let mut plene = seq.clone();
            plene.push(ocv(letter::NUN, nv));
            plene.extend_from_slice(tail);
            out.push((obj, hebrew::render(&plene)));
            // Defective base (…ֻן-): subject vowel on the stem consonant.
            let mut defec = seq.clone();
            defec.truncate(n - 1);
            if let Some(c) = defec.last_mut() {
                c.vowel = Some(mater_vowel);
            }
            defec.push(ocv(letter::NUN, nv));
            defec.extend_from_slice(tail);
            out.push((obj, hebrew::render(&defec)));
        }
    }
    out
}

/// The nominal/possessive suffix set, as (object PGN, linking vowel on the
/// final stem consonant, tail consonants). An infinitive construct takes these
/// — "his reigning" mālḵô (מָלְכוֹ), "to possess it" — exactly as a noun does,
/// not the verbal -anî/-ēhû set.
fn nominal_suffix_tails() -> Vec<(Pgn, Option<Vowel>, Vec<Cons>)> {
    use Vowel::*;
    vec![
        (OBJ_1CS, Some(Hiriq), vec![Cons::new(letter::YOD)]), // -î
        // The verbal-style 1cs -ēnî also rides infinitive hosts: haḵʿîsēnî
        // (לְהַכְעִיסֵנִי), šallᵊḥēnî.
        (
            OBJ_1CS,
            Some(Tsere),
            vec![ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
        ), // -ēnî
        (OBJ_2MS, Some(Sheva), vec![ocv(letter::KAF, Qamats)]), // -ᵊḵā
        // Long 2ms -ḵâ with the he mater (bôʾăḵâ בֹּאֲכָה, מַלְכָּכָה-style spellings).
        (
            OBJ_2MS,
            Some(Sheva),
            vec![ocv(letter::KAF, Qamats), Cons::new(letter::HE)],
        ), // -ᵊḵâ
        // Segol-link 2ms (šiḇteḵā שִׁבְתֶּךָ, liqrāṯeḵā לִקְרָאתֶךָ).
        (OBJ_2MS, Some(Segol), vec![ocv(letter::KAF, Qamats)]), // -eḵā
        // The -āḵ 2ms grade with a final kaf (hiššāmᵊḏāḵ הִשָּׁמְדָךְ).
        (OBJ_2MS, Some(Qamats), vec![ocv(letter::KAF, Sheva)]), // -āḵ
        (OBJ_2FS, Some(Tsere), vec![ocv(letter::KAF, Sheva)]),  // -ēḵ
        (
            OBJ_3MS,
            None,
            vec![Cons::new(letter::VAV).with_vowel(Holam)],
        ), // -ô (holam on vav)
        (
            OBJ_3FS,
            Some(Qamats),
            vec![Cons::new(letter::HE).with_dagesh()],
        ), // -āh
        (
            OBJ_1CP,
            Some(Tsere),
            vec![Cons::new(letter::NUN), oshureq()],
        ), // -ēnû
        (
            OBJ_2MP,
            Some(Sheva),
            vec![ocv(letter::KAF, Segol), Cons::new(letter::MEM)],
        ), // -ᵊḵem
        (OBJ_3MP, Some(Qamats), vec![Cons::new(letter::MEM)]),  // -ām
        (OBJ_3FP, Some(Qamats), vec![Cons::new(letter::NUN)]),  // -ān
    ]
}

/// Infinitive construct with a pronominal suffix ("in his reigning" בְּמָלְכוֹ,
/// "his begetting" הוֹלִידוֹ). The Qal infinitive shifts to the qoṭl- grade
/// before a suffix (mᵊlōḵ → mālḵ-) — built here from the radicals (C1 qamets-
/// hatuf, C2 silent sheva), trying both the regular and qatan qamats glyphs.
/// Other binyanim keep their bare infinitive shape and simply take the suffix
/// on C3 (haqṭîl → haqṭîl-ô). Strong/true-final-consonant shapes only.
fn inf_construct_object_suffixes(
    root: &Root,
    binyan: Binyan,
    base_text: &str,
) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let mut out = Vec::new();
    let (c1, c2, c3) = (root.pe(), root.ayin(), root.lamed());
    // Hollow Hiphil inf-construct (hāmîṯ הָמִית): before a suffix the qamats on
    // the he reduces propretonically to hataf-patah and the î is retained —
    // hămîṯô הֲמִיתוֹ, hămîṯām. Build from the radicals (the strong path has no
    // form to start from).
    if binyan == Binyan::Hiphil && matches!(c2, letter::VAV | letter::YOD) {
        for (obj, link, tail) in nominal_suffix_tails() {
            let mut seq = vec![
                Cons::new(letter::HE).with_vowel(HatafPatah),
                Cons::radical(c1, 1).with_vowel(Hiriq),
                Cons::mater(letter::YOD),
                {
                    let mut c = Cons::radical(c3, 3);
                    c.vowel = link;
                    c
                },
            ];
            seq.extend(tail);
            out.push((obj, hebrew::render(&seq)));
        }
        return out;
    }
    // Hollow Qal (בּוֹא, קוּם): the bare infinitive is already the suffix base;
    // the suffix joins C3 directly. Both the plene (vav mater) and defective
    // spellings occur — bᵊḇōʾô בְּבֹאוֹ beside בּוֹאוֹ — as do both word-initial
    // dagesh-lene states (the proclitic peel exposes the soft one). A guttural
    // or aleph C3 takes a hataf-patah for the sheva-link suffixes (bôʾăḵā
    // בּוֹאֲךָ) and quiesces before the -ô.
    if binyan == Binyan::Qal && matches!(c2, letter::VAV | letter::YOD) && c3 != letter::HE {
        let u_class = hollow_class(root) == HollowClass::Shureq;
        for (obj, link, tail) in nominal_suffix_tails() {
            let link = if link == Some(Sheva) && hebrew::is_guttural(c3) {
                Some(HatafPatah)
            } else {
                link
            };
            let mut c3c = Cons::radical(c3, 3);
            c3c.vowel = link;
            for first_dagesh in [true, false] {
                let mut c1c = Cons::radical(c1, 1);
                c1c.dagesh = first_dagesh;
                // Plene: C1 + vav mater carrying the theme.
                let mater = if u_class {
                    Cons::new(letter::VAV).with_dagesh()
                } else {
                    Cons::new(letter::VAV).with_vowel(Holam)
                };
                let mut plene = vec![c1c, mater, c3c];
                plene.extend(tail.clone());
                out.push((obj, hebrew::render(&plene)));
                // Defective: the theme written on C1 itself.
                let mut c1d = c1c;
                c1d.vowel = Some(if u_class { Qubuts } else { Holam });
                let mut defective = vec![c1d, c3c];
                defective.extend(tail.clone());
                out.push((obj, hebrew::render(&defective)));
            }
        }
        return out;
    }
    // Geminate Qal (תֹּם, סֹב): before a suffix the doubled radical surfaces as
    // a single dageshed consonant on a short-vowel stem — tummām תֻּמָּם,
    // subbô סֻבּוֹ; the holam grade (ḥonnô) also occurs. Both word-initial
    // dagesh-lene states are emitted for the proclitic peel.
    if binyan == Binyan::Qal && c2 == c3 && !hebrew::is_guttural(c2) && c2 != letter::RESH {
        for (obj, link, tail) in nominal_suffix_tails() {
            for v1 in [Qubuts, Holam] {
                for first_dagesh in [true, false] {
                    let mut c1c = Cons::radical(c1, 1).with_vowel(v1);
                    c1c.dagesh = first_dagesh;
                    let mut c2c = Cons::radical(c2, 2).with_dagesh();
                    c2c.vowel = link;
                    let mut seq = vec![c1c, c2c];
                    seq.extend(tail.clone());
                    out.push((obj, hebrew::render(&seq)));
                }
            }
        }
        return out;
    }
    // שרת: the Piel infinitive is the lexicalized šārēṯ (שָׁרֵת); its theme
    // reduces before a suffix — šārᵊṯô לְשָׁרְתוֹ.
    if binyan == Binyan::Piel && c1 == letter::SHIN && c2 == letter::RESH && c3 == letter::TAV {
        for (obj, link, tail) in nominal_suffix_tails() {
            let mut t = Cons::radical(c3, 3);
            t.vowel = link;
            let mut seq = vec![
                Cons::radical(c1, 1).with_vowel(Qamats),
                Cons::radical(c2, 2).with_vowel(Sheva),
                t,
            ];
            seq.extend(tail);
            out.push((obj, hebrew::render(&seq)));
        }
        return out;
    }
    // נתן: the suffix base is titt- — both nuns gone, the final one
    // assimilating into the affix tav: tittî תִּתִּי, tittô תִּתּוֹ, tittᵊḵā
    // תִּתְּךָ. Emit both word-initial dagesh-lene spellings (the proclitic
    // peel strips it: בְּתִתּוֹ).
    if binyan == Binyan::Qal && c1 == letter::NUN && c2 == letter::TAV && c3 == letter::NUN {
        for (obj, link, tail) in nominal_suffix_tails() {
            for first_dagesh in [true, false] {
                let mut t1 = rad(letter::TAV, 2).with_vowel(Hiriq);
                t1.dagesh = first_dagesh;
                let mut t2 = Cons::new(letter::TAV).with_dagesh();
                t2.vowel = link;
                let mut seq = vec![t1, t2];
                seq.extend(tail.clone());
                out.push((obj, hebrew::render(&seq)));
            }
        }
        return out;
    }
    // לקח carries its I-nun-style assimilation into the -t infinitive
    // (qaḥaṯ קַחַת); the suffix base is qaḥt- with a dagesh-lene tav —
    // qaḥtô (בְּקַחְתּוֹ), qaḥtāh (לְקַחְתָּהּ), qaḥtî.
    if binyan == Binyan::Qal && is_laqah(root) {
        for (obj, link, tail) in nominal_suffix_tails() {
            let mut t = Cons::new(letter::TAV).with_dagesh();
            t.vowel = link;
            let mut seq = vec![
                rad(letter::QOF, 2).with_vowel(Patah),
                rad(letter::HET, 3).with_vowel(Sheva),
                t,
            ];
            seq.extend(tail);
            out.push((obj, hebrew::render(&seq)));
        }
        return out;
    }
    // קרא: the lexicalized -t infinitive (liqraʾṯ לִקְרַאת) takes its suffixes
    // on a qamats grade: liqrāṯô לִקְרָאתוֹ, לִקְרָאתִי, לִקְרָאתָם. It also
    // keeps the plain qoṭl- host (qorʾî בְּקָרְאִי), so fall through to the
    // strong path after emitting these.
    if binyan == Binyan::Qal && c1 == letter::QOF && c2 == letter::RESH && c3 == letter::ALEF {
        for (obj, link, tail) in nominal_suffix_tails() {
            let mut t = Cons::new(letter::TAV);
            t.vowel = link;
            let mut seq = vec![
                rad(c1, 1).with_vowel(Sheva),
                rad(c2, 2).with_vowel(Qamats),
                rad(c3, 3),
                t,
            ];
            seq.extend(tail);
            out.push((obj, hebrew::render(&seq)));
        }
    }
    // Pe-yod -t infinitive host: the dropped-yod infinitive is a segolate
    // (šeḇeṯ שֶׁבֶת, daʕaṯ דַעַת) or a quiescent-aleph CēT (ṣêṯ צֵאת). Derive the
    // suffix base from that infinitive (base_text) rather than the radicals, so
    // the per-root ground vowel and the III-aleph quiescence carry through:
    //   segolate (C3 voweled) → the ground vowel returns to C1 (segol→hiriq,
    //     šeḇeṯ→šiḇt-; patah held, daʕaṯ→daʕt-) and C3 reduces to a sheva,
    //     giving šiḇtô שִׁבְתּוֹ, šiḇtām שִׁבְתָּם, šiḇtᵊḵā שִׁבְתְּךָ.
    //   quiescent CēT (C3 bare) → C1/C3 are untouched and the affix tav simply
    //     takes the link vowel: ṣêṯām צֵאתָם.
    // Emitted before the yod-retained qoṭl- host below so the segolate base is
    // the primary suffix form.
    if binyan == Binyan::Qal && c1 == letter::YOD {
        let base = hebrew::parse_pointed(base_text);
        if base.len() == 3 && base[2].letter == letter::TAV && base[2].vowel.is_none() {
            let segolate = base[1].vowel.is_some();
            for (obj, link, tail) in nominal_suffix_tails() {
                let mut head0 = base[0];
                let mut head1 = base[1];
                if segolate {
                    if head0.vowel == Some(Segol) {
                        head0.vowel = Some(Hiriq);
                    }
                    head1.vowel = Some(Sheva);
                }
                let mut t = Cons::new(letter::TAV);
                t.vowel = link;
                let mut seq = vec![head0, head1, t];
                seq.extend(tail);
                apply_guttural(&mut seq, root);
                out.push((obj, hebrew::render(&seq)));
            }
        }
    }
    // Pe-nun / pe-yod Qal inf-construct also has a nun/yod-RETAINED qoṭl- host
    // beside the assimilated/segholate one (gōʕô גֹּעוֹ but also nāḡʕô נָגְעוֹ;
    // šeḇtô שִׁבְתּוֹ but also yāsdô יָסְדוֹ): the full triliteral takes the same
    // qoṭl- grade as a strong root (C1 qamats, C2 sheva, C3 + link). Additive —
    // fall through to the segholate/assimilated host below.
    if binyan == Binyan::Qal
        && matches!(c1, letter::NUN | letter::YOD)
        && !matches!(c2, letter::VAV | letter::YOD)
        && !matches!(c3, letter::HE | letter::VAV | letter::YOD)
    {
        for (obj, link, tail) in nominal_suffix_tails() {
            for c1v in [Qamats, QamatsQatan, Hiriq] {
                let mut c3c = Cons::radical(c3, 3);
                c3c.vowel = link;
                let mut seq = vec![
                    Cons::radical(c1, 1).with_vowel(c1v),
                    Cons::radical(c2, 2).with_vowel(Sheva),
                    c3c,
                ];
                seq.extend(tail.clone());
                apply_guttural(&mut seq, root);
                out.push((obj, hebrew::render(&seq)));
            }
        }
    }
    // A quiescent III-aleph rides the strong host (the link vowel lands on
    // the aleph — qorʾî קָרְאִי), so only the true mater finals are excluded.
    let strong_qal = binyan == Binyan::Qal
        && !matches!(c3, letter::HE | letter::VAV | letter::YOD)
        && !matches!(c1, letter::VAV | letter::YOD | letter::NUN);
    if strong_qal {
        // Strong Qal: the base inf-construct (qᵊṭōl) reduces to the qoṭl- grade
        // before a suffix (šomrî), so build that from the radicals. The
        // i-grade (šiḇrî בְּשִׁבְרִי) is attested beside the o-grade.
        for (obj, link, tail) in nominal_suffix_tails() {
            for c1v in [Qamats, QamatsQatan, Hiriq] {
                let mut c3c = Cons::radical(c3, 3);
                c3c.vowel = link;
                let mut seq = vec![
                    Cons::radical(c1, 1).with_vowel(c1v),
                    Cons::radical(c2, 2).with_vowel(Sheva),
                    c3c,
                ];
                seq.extend(tail.clone());
                apply_guttural(&mut seq, root);
                out.push((obj, hebrew::render(&seq)));
            }
        }
        // The heavy 2mp -ḵem shifts the stress two syllables forward, so the
        // stem takes the propretonic-qamats grade qᵊṭāl- (C1 reduces to a
        // vocal sheva / hataf, C2 carries the qamats): ʾăḵālḵem אֲכָלְכֶם,
        // ʕăzāḇḵem עֲזָבְכֶם, ʾăḇāḏḵem אֲבָדְכֶם — beside the plain qoṭl- host
        // (the lighter -ḵā keeps qoṭl-, ʾoḵlᵊḵā).
        for (obj, link, tail) in nominal_suffix_tails() {
            if obj != OBJ_2MP {
                continue;
            }
            let mut c3c = Cons::radical(c3, 3);
            c3c.vowel = link;
            let mut seq = vec![
                Cons::radical(c1, 1).with_vowel(Sheva),
                Cons::radical(c2, 2).with_vowel(Qamats),
                c3c,
            ];
            seq.extend(tail.clone());
            apply_guttural(&mut seq, root);
            out.push((obj, hebrew::render(&seq)));
            // An I-aleph C1 also surfaces with hataf-segol here (ʾĕmārḵem
            // אֱמָרְכֶם beside the hataf-patah ʾăḵālḵem).
            if c1 == letter::ALEF && seq[0].vowel == Some(HatafPatah) {
                let mut s2 = seq.clone();
                s2[0].vowel = Some(HatafSegol);
                out.push((obj, hebrew::render(&s2)));
            }
        }
    } else {
        // Derived stems and the weak Qal inf-constructs (the pe-yod/pe-nun ṣēʾṯ
        // צֵאת, šeḇeṯ, tēṯ, daʕaṯ): the base form is already the suffix base, so
        // the suffix attaches straight to its final consonant (ṣêṯām בְּצֵאתָם,
        // ṣêṯᵊḵā צֵאתְךָ).
        let seq = hebrew::parse_pointed(base_text);
        let n = seq.len();
        if n < 2 {
            return out;
        }
        let last = seq[n - 1];
        // A final aleph stays as a host — the link vowel lands on it and it
        // quiesces (hôṣîʾām לְהוֹצִיאָם); he/vav/yod finals are maters and
        // can't carry the suffix.
        // A III-guttural inf-construct ends in that guttural carrying a furtive
        // patah (Piel šallēaḥ שַׁלֵּחַ, hôšîaʕ) — not a real vowel; the suffix
        // replaces it (šallᵊḥām שַׁלְּחָם), so let it through like a bare final.
        let furtive = hebrew::is_guttural(last.letter) && last.vowel == Some(Vowel::Patah);
        if (matches!(last.vowel, Some(v) if v != Vowel::Sheva) && !furtive)
            || matches!(last.letter, letter::HE | letter::VAV | letter::YOD)
        {
            return out;
        }
        // Pe-yod segholate infinitive construct (šeḇeṯ שֶׁבֶת, leḵeṯ לֶכֶת):
        // the two surviving radicals both carry segol. Before a suffix C2 goes to
        // a silent sheva and C1 either reduces to hiriq (the I-yod šibtô שִׁבְתּוֹ)
        // or keeps its segol (leḵtô לֶכְתּוֹ, ridtô); emit both grades.
        let segholate = n >= 3
            && seq[n - 1].letter == letter::TAV
            && seq[0].vowel == Some(Vowel::Segol)
            && seq[1].vowel == Some(Vowel::Segol);
        if segholate {
            for (obj, link, tail) in nominal_suffix_tails() {
                for c1v in [Vowel::Hiriq, Vowel::Segol] {
                    // After the now-silent C2 sheva the tav takes a dagesh
                    // lene (šiḇtᵊḵā שִׁבְתְּךָ); emit both spellings.
                    for dagesh in [false, true] {
                        let mut s = seq.clone();
                        s[0].vowel = Some(c1v);
                        s[1].vowel = Some(Vowel::Sheva);
                        s[n - 1].vowel = link;
                        s[n - 1].dagesh = dagesh;
                        s.extend(tail.clone());
                        out.push((obj, hebrew::render(&s)));
                    }
                }
            }
        } else {
            // The Piel theme tsere on the doubled C2 (dabbēr דַּבֵּר) reduces
            // to a vocal sheva before a vowel-initial suffix: dabbᵊrô
            // (בְּדַבְּרוֹ), qaddᵊšô (לְקַדְּשׁוֹ) — a hataf-patah under an
            // undageshable guttural C2 (naḥămô לְנַחֲמוֹ). The sheva-link
            // suffixes (-ᵊḵā, -ᵊḵem) keep the full tsere instead (dabberḵā),
            // so the reduced twin is emitted only for the vowel links.
            // The Niphal theme tsere (hiššāmēḏ הִשָּׁמֵד) reduces the same way
            // before the vowel links — hiššāmᵊḏāḵ הִשָּׁמְדָךְ.
            let piel_theme = (matches!(binyan, Binyan::Piel | Binyan::Pual | Binyan::Hithpael)
                && n >= 2
                && (seq[n - 2].dagesh || hebrew::is_guttural(seq[n - 2].letter))
                || binyan == Binyan::Niphal && n >= 2)
                && seq[n - 2].vowel == Some(Vowel::Tsere);
            let piel_reduced = if hebrew::is_guttural(seq[n - 2].letter) {
                Vowel::HatafPatah
            } else {
                Vowel::Sheva
            };
            for (obj, link, tail) in nominal_suffix_tails() {
                // A guttural/aleph host can't carry the vocal-sheva link —
                // it takes a hataf-patah (hôṣîʾăḵā הוֹצִיאֲךָ).
                let link = if link == Some(Vowel::Sheva) && hebrew::is_guttural(last.letter) {
                    Some(Vowel::HatafPatah)
                } else {
                    link
                };
                let mut s = seq.clone();
                s[n - 1].vowel = link;
                s.extend(tail.clone());
                out.push((obj, hebrew::render(&s)));
                if piel_theme && link != Some(Vowel::Sheva) {
                    let mut s = seq.clone();
                    s[n - 2].vowel = Some(piel_reduced);
                    s[n - 1].vowel = link;
                    s.extend(tail.clone());
                    out.push((obj, hebrew::render(&s)));
                    // A guttural theme also closes on a plain silent sheva
                    // (hiṯyaḥśām וְהִתְיַחְשָׂם).
                    if piel_reduced != Vowel::Sheva {
                        let mut s = seq.clone();
                        s[n - 2].vowel = Some(Vowel::Sheva);
                        s[n - 1].vowel = link;
                        s.extend(tail);
                        out.push((obj, hebrew::render(&s)));
                    }
                } else if piel_theme && link == Some(Vowel::Sheva) {
                    // Before the -ᵊḵā link the theme tsere lowers to segol
                    // instead — hiṯraggezḵā הִתְרַגֶּזְךָ.
                    let mut s = seq.clone();
                    s[n - 2].vowel = Some(Vowel::Segol);
                    s[n - 1].vowel = link;
                    s.extend(tail);
                    out.push((obj, hebrew::render(&s)));
                }
            }
        }
    }
    out
}

/// Qal imperative (2ms) with a pronominal object suffix. Like the inf.
/// construct, the suffix attaches to the qoṭl- grade (C1 qamats, C2 sheva), but
/// with the *verbal* suffix set: šomrēnî (שָׁמְרֵנִי), šomrēhû/šomrennû,
/// šomrēm. Strong/true-final-consonant shapes only.
fn qal_imperative_object_suffixes(root: &Root) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let (c1, c2, c3) = (root.pe(), root.ayin(), root.lamed());
    // A weak C1 (pe-vav/yod/nun) reshapes the bare imperative, so the strong
    // C1-sheva stem below is wrong for it — except a III-He pe-nun keeps its nun
    // in the imperative (nᵊḥê נְחֵה, not the assimilated imperfect form), so the
    // III-He branch's C1-sheva stem is correct: nᵊḥēnî נְחֵנִי. Exempt III-He.
    if matches!(c1, letter::VAV | letter::YOD | letter::NUN) && c3 != letter::HE {
        return Vec::new();
    }
    // III-He imperative + suffix: the etymological he elides and the suffix
    // attaches straight to C2, which carries the linking vowel — ʕănēnî (עֲנֵנִי),
    // bᵊnēhû (בְּנֵהוּ). C1 takes a sheva (a hataf under a guttural, via
    // apply_guttural). No C3 in the stem.
    if c3 == letter::HE {
        let mut out = Vec::new();
        let mut emit = |obj: Pgn, link: Vowel, tail: &[Cons]| {
            let mut seq = vec![
                Cons::radical(c1, 1).with_vowel(Sheva),
                Cons::radical(c2, 2).with_vowel(link),
            ];
            seq.extend_from_slice(tail);
            apply_guttural(&mut seq, root);
            out.push((obj, hebrew::render(&seq)));
        };
        emit(
            OBJ_1CS,
            Tsere,
            &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
        ); // -ēnî
        emit(
            OBJ_1CS,
            Tsere,
            &[
                Cons::new(letter::NUN).with_dagesh().with_vowel(Hiriq),
                Cons::new(letter::YOD),
            ],
        ); // -ēnnî
        emit(OBJ_3MS, Tsere, &[Cons::new(letter::HE), oshureq()]); // -ēhû
        emit(OBJ_3FS, Segol, &[ocv(letter::HE, Qamats)]); // -ehā
        emit(OBJ_1CP, Tsere, &[Cons::new(letter::NUN), oshureq()]); // -ēnû
        emit(OBJ_3MP, Tsere, &[Cons::new(letter::MEM)]); // -ēm
        return out;
    }
    // III-Aleph imperative + suffix: the quiescent aleph keeps the linking
    // vowel and C2 takes the qamats that the open pretonic syllable demands —
    // rᵊp̄āʔēnî רְפָאֵנִי, mᵊṣāʔēm. C1 reduces to a sheva (a hataf under a
    // guttural, via apply_guttural).
    if c3 == letter::ALEF {
        let mut out = Vec::new();
        let mut emit = |obj: Pgn, link: Vowel, tail: &[Cons]| {
            let mut seq = vec![
                Cons::radical(c1, 1).with_vowel(Sheva),
                Cons::radical(c2, 2).with_vowel(Qamats),
                Cons::radical(c3, 3).with_vowel(link),
            ];
            seq.extend_from_slice(tail);
            apply_guttural(&mut seq, root);
            out.push((obj, hebrew::render(&seq)));
        };
        emit(
            OBJ_1CS,
            Tsere,
            &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
        ); // -ēnî
        emit(
            OBJ_1CS,
            Segol,
            &[
                Cons::new(letter::NUN).with_dagesh().with_vowel(Hiriq),
                Cons::new(letter::YOD),
            ],
        ); // -ennî
        emit(OBJ_3MS, Tsere, &[Cons::new(letter::HE), oshureq()]); // -ēhû
        emit(OBJ_3FS, Segol, &[ocv(letter::HE, Qamats)]); // -ehā
        emit(OBJ_1CP, Tsere, &[Cons::new(letter::NUN), oshureq()]); // -ēnû
        emit(OBJ_3MP, Tsere, &[Cons::new(letter::MEM)]); // -ēm
        return out;
    }
    if matches!(c3, letter::VAV | letter::YOD) {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut emit = |obj: Pgn, c1_vowel: Vowel, c2_vowel: Vowel, link: Vowel, tail: &[Cons]| {
        let mut seq = vec![
            Cons::radical(c1, 1).with_vowel(c1_vowel),
            Cons::radical(c2, 2).with_vowel(c2_vowel),
            Cons::radical(c3, 3).with_vowel(link),
        ];
        seq.extend_from_slice(tail);
        apply_guttural(&mut seq, root);
        out.push((obj, hebrew::render(&seq)));
    };
    // Two gradings of the qoṭl- base, both emitted (each matches at most one
    // surface): the strong qāṭl- (C1 qamats, C2 sheva → šomrēnî), and the
    // qᵊṭāl- shape with C1 reduced to sheva and the qamats on C2. The latter
    // covers a C2 guttural that can't take the silent sheva (bᵊḥānēnî
    // בְּחָנֵנִי) and the III-guttural a-theme imperative (šᵊmaʕ שְׁמַע → with
    // the theme patah lengthening to qamats: šᵊmāʕēnî שְׁמָעֵנִי, šᵊlāḥēnî).
    let gradings: &[(Vowel, Vowel)] = if hebrew::is_guttural(c2) || hebrew::is_guttural(c3) {
        &[(Qamats, Sheva), (Sheva, Qamats)]
    } else {
        &[(Qamats, Sheva)]
    };
    for &(g1, g2) in gradings {
        emit(
            OBJ_1CS,
            g1,
            g2,
            Tsere,
            &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
        ); // -ēnî
        emit(
            OBJ_1CS,
            g1,
            g2,
            Segol,
            &[
                Cons::new(letter::NUN).with_dagesh().with_vowel(Hiriq),
                Cons::new(letter::YOD),
            ],
        ); // -ennî
        emit(OBJ_3MS, g1, g2, Tsere, &[Cons::new(letter::HE), oshureq()]); // -ēhû
        emit(
            OBJ_3MS,
            g1,
            g2,
            Segol,
            &[Cons::new(letter::NUN).with_dagesh(), oshureq()],
        ); // -ennû
        emit(OBJ_2MS, g1, g2, Sheva, &[ocv(letter::KAF, Qamats)]); // -ᵊḵā
        emit(OBJ_3FS, g1, g2, Segol, &[ocv(letter::HE, Qamats)]); // -ehā
        emit(OBJ_1CP, g1, g2, Tsere, &[Cons::new(letter::NUN), oshureq()]); // -ēnû
        emit(OBJ_3MP, g1, g2, Tsere, &[Cons::new(letter::MEM)]); // -ēm
    }
    out
}

/// Piel/Hithpael imperative (2ms) with a pronominal object suffix, built from
/// the bare imperative `base_text`. The theme vowel on the (doubled) C2 reduces
/// to a vocal sheva before the verbal suffix — lammᵊḏēnî לַמְּדֵנִי from לַמֵּד,
/// qaddᵊšēm — a hataf-patah under an undageshable guttural C2. A III-He base
/// (ḥayyê חַיֵּה) elides its he and the suffix joins C2's linking vowel directly:
/// ḥayyēnî חַיֵּנִי. Additive: a mis-modelled grade simply fails to match.
fn piel_imperative_object_suffixes(base_text: &str) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let seq = hebrew::parse_pointed(base_text);
    let n = seq.len();
    if n < 2 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let tails: Vec<(Pgn, Vowel, Vec<Cons>)> = vec![
        (
            OBJ_1CS,
            Tsere,
            vec![ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
        ), // -ēnî
        (
            OBJ_1CS,
            Segol,
            vec![
                Cons::new(letter::NUN).with_dagesh().with_vowel(Hiriq),
                Cons::new(letter::YOD),
            ],
        ), // -ennî
        (OBJ_3MS, Tsere, vec![Cons::new(letter::HE), oshureq()]), // -ēhû
        (
            OBJ_3MS,
            Segol,
            vec![Cons::new(letter::NUN).with_dagesh(), oshureq()],
        ), // -ennû
        (OBJ_3FS, Segol, vec![ocv(letter::HE, Qamats)]),          // -ehā
        (OBJ_1CP, Tsere, vec![Cons::new(letter::NUN), oshureq()]), // -ēnû
        (OBJ_3MP, Tsere, vec![Cons::new(letter::MEM)]),           // -ēm
        (OBJ_2MS, Sheva, vec![ocv(letter::KAF, Qamats)]),         // -ᵊḵā
    ];
    let last = seq[n - 1];
    if last.letter == letter::HE && matches!(seq[n - 2].vowel, Some(Tsere | Segol)) {
        // III-He: drop the he; C2 carries the linking vowel.
        for (obj, link, tail) in &tails {
            let mut s = seq[..n - 1].to_vec();
            if let Some(c) = s.last_mut() {
                c.vowel = Some(*link);
            }
            s.extend(tail.iter().cloned());
            out.push((*obj, hebrew::render(&s)));
        }
        return out;
    }
    // Strong shape: C3 must be a true final consonant. A III-guttural's
    // furtive patah (שַׁלֵּחַ) is not a real vowel — it drops before the
    // suffix link (šallᵊḥēnî שַׁלְּחֵנִי).
    if matches!(last.vowel, Some(v) if v != Sheva
        && !(v == Patah && hebrew::is_guttural(last.letter)))
        || matches!(
            last.letter,
            letter::HE | letter::ALEF | letter::VAV | letter::YOD
        )
    {
        return Vec::new();
    }
    let reduced = if hebrew::is_guttural(seq[n - 2].letter) {
        HatafPatah
    } else {
        Sheva
    };
    for (obj, link, tail) in &tails {
        let mut s = seq.clone();
        s[n - 2].vowel = Some(reduced);
        s[n - 1].vowel = Some(*link);
        s.extend(tail.iter().cloned());
        out.push((*obj, hebrew::render(&s)));
    }
    out
}

/// III-He Hiphil imperative (2ms) with a pronominal object suffix. The bare
/// imperative is haqṭēh (harʾēh הַרְאֵה, hôrēh הוֹרֵה, hakkēh הַכֵּה); before a
/// suffix the final he elides and the suffix attaches to the C2 tsere link —
/// hôrēnî הוֹרֵנִי, harʾēnû הַרְאֵנוּ, hakkēnî/hakkênî הַכֵּינִי. Both the
/// defective and the plene tsere-yod spellings occur. Additive.
fn lamed_he_hiphil_imperative_object_suffixes(base_text: &str) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let mut seq = hebrew::parse_pointed(base_text);
    if seq.last().map(|c| c.letter) != Some(letter::HE) || seq.len() < 3 {
        return Vec::new();
    }
    seq.pop(); // drop the final he; the C2 tsere is the link vowel
    let tails: &[(Pgn, &[Cons])] = &[
        (OBJ_1CS, &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)]),
        (OBJ_1CP, &[Cons::new(letter::NUN), oshureq()]),
        (OBJ_3MS, &[Cons::new(letter::HE), oshureq()]),
        (OBJ_3MP, &[Cons::new(letter::MEM)]),
    ];
    let mut out = Vec::new();
    for &(obj, tail) in tails {
        for plene in [false, true] {
            let mut s = seq.clone();
            if plene {
                s.push(Cons::mater(letter::YOD));
            }
            s.extend_from_slice(tail);
            out.push((obj, hebrew::render(&s)));
        }
    }
    out
}

/// Hiphil imperative (2ms) with a pronominal object suffix. Before a suffix the
/// Hiphil imperative shows its long hiriq-yod (î) theme and the final radical
/// takes a tsere linking vowel + the *verbal* suffix set: haḏrîḵēnî
/// (הַדְרִיכֵנִי), hôḏîʕēnî (הוֹדִיעֵנִי), haṣṣîlēnî (הַצִּילֵנִי), hăḇînēnî
/// (הֲבִינֵנִי). Built from the bare Hiphil imperative `base_text`, whose prefix
/// (ha-/hô-/hă-) and any I-weak/hollow reshaping already in the base carry
/// through; we only retune the theme syllable. The base must end in
/// `[C2(theme)] [C3]` where C3 is a true final consonant — so III-He/-mater
/// finals are skipped. Additive: a mis-modelled theme simply fails to match.
fn hiphil_imperative_object_suffixes(base_text: &str) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let seq = hebrew::parse_pointed(base_text);
    let n = seq.len();
    if n < 3 {
        return Vec::new();
    }
    let last = seq[n - 1];
    // A III-guttural Hiphil imperative ends in that guttural with a furtive patah
    // (hôšēaʕ הוֹשֵׁעַ); the suffixed form retunes the theme and the guttural takes
    // the link vowel (hôšîʕēnî הוֹשִׁיעֵנִי), so allow a final het/ayin+patah.
    let guttural =
        matches!(last.letter, letter::HET | letter::AYIN) && last.vowel == Some(Vowel::Patah);
    // Otherwise C3 must be a true final consonant (vowelless or the rendered
    // silent sheva under a final kaf); reject mater/weak finals.
    if (matches!(last.vowel, Some(v) if v != Vowel::Sheva) && !guttural)
        || matches!(
            last.letter,
            letter::HE | letter::ALEF | letter::VAV | letter::YOD
        )
    {
        return Vec::new();
    }
    // The theme sits on C2 (seq[n-2]); a long-Hiphil base may already carry a
    // hiriq-yod there (hôḏîʕ-), in which case seq[n-2] is the YOD mater on a
    // hiriq C2 at seq[n-3]. Detect both shapes and rebuild the stem up to C3
    // with an explicit hiriq + yod theme.
    let mut stem: Vec<Cons>;
    if seq[n - 2].letter == letter::YOD
        && seq[n - 2].vowel.is_none()
        && n >= 3
        && seq[n - 3].vowel == Some(Hiriq)
    {
        // Already plene î (…C2(hiriq) YOD C3): keep through the yod.
        stem = seq[..n - 1].to_vec();
    } else {
        // Defective theme on C2 (haḏrēḵ): set C2 to hiriq and insert a yod
        // mater before C3.
        stem = seq[..n - 1].to_vec();
        if let Some(c) = stem.last_mut() {
            c.vowel = Some(Hiriq);
        }
        stem.push(Cons::new(letter::YOD));
    }
    let mut out = Vec::new();
    let mut emit = |obj: Pgn, link: Vowel, tail: &[Cons]| {
        let mut s = stem.clone();
        s.push(last.with_vowel(link));
        s.extend_from_slice(tail);
        out.push((obj, hebrew::render(&s)));
        // The suffix moves the stress: a qamats prefix syllable (the hollow
        // hā- of הָבֵן) reduces propretonically to hataf-patah — hăḇînēnî
        // הֲבִינֵנִי.
        if s[0].vowel == Some(Qamats) {
            s[0].vowel = Some(HatafPatah);
            out.push((obj, hebrew::render(&s)));
        }
    };
    emit(
        OBJ_1CS,
        Tsere,
        &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)],
    ); // -ēnî
    emit(
        OBJ_1CS,
        Segol,
        &[
            Cons::new(letter::NUN).with_dagesh().with_vowel(Hiriq),
            Cons::new(letter::YOD),
        ],
    ); // -ennî
    emit(OBJ_3MS, Tsere, &[Cons::new(letter::HE), oshureq()]); // -ēhû
    emit(
        OBJ_3MS,
        Segol,
        &[Cons::new(letter::NUN).with_dagesh(), oshureq()],
    ); // -ennû
    emit(OBJ_2MS, Sheva, &[ocv(letter::KAF, Qamats)]); // -ᵊḵā
    emit(OBJ_3FS, Segol, &[ocv(letter::HE, Qamats)]); // -ehā
    emit(OBJ_1CP, Tsere, &[Cons::new(letter::NUN), oshureq()]); // -ēnû
    emit(OBJ_3MP, Tsere, &[Cons::new(letter::MEM)]); // -ēm
    out
}

/// Pausal alternant: a stressed patah in the final closed syllable lengthens to
/// qamats in pause — qāṭal → qāṭāl (קָטָל), yûmat → yûmāt (יוּמָת). Returns the
/// pausal spelling when `text` ends in a true consonant (closed syllable)
/// preceded by patah, else `None`. Scoped to that ending so it never perturbs
/// open-syllable or mater-final forms.
fn pausal_qamats_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 2 {
        return None;
    }
    let last = &seq[n - 1];
    // A final kaf carries the Masoretic silent sheva — still a closed syllable.
    let final_kaf_sheva = last.letter == letter::KAF && last.vowel == Some(Vowel::Sheva);
    if (last.vowel.is_some() && !final_kaf_sheva)
        || matches!(
            last.letter,
            letter::HE | letter::ALEF | letter::YOD | letter::VAV
        )
    {
        return None;
    }
    if seq[n - 2].vowel != Some(Vowel::Patah) {
        return None;
    }
    seq[n - 2].vowel = Some(Vowel::Qamats);
    Some(hebrew::render(&seq))
}

/// I-guttural Hiphil C2 spirantization twin: after the open hataf syllable the
/// preformative opens (heʕĕ-, haʕă-), so a begedkefet second radical spirantizes
/// — heʕĕḇîr הֶעֱבִיר, haʕăḇîr הַעֲבִיר, hāʔăḇadtî הַאֲבַדְתִּי — but the builder
/// leaves the dagesh lene it would take after a closed syllable (הֶעֱבִּיר).
/// Strips a dagesh on the second radical (root.ayin()). Caller gates to (Hiphil,
/// PeGuttural). Additive.
fn hiphil_guttural_c2_spirant_variant(root: &Root, text: &str) -> Option<String> {
    let c2 = root.ayin();
    let mut seq = hebrew::parse_pointed(text);
    for c in seq.iter_mut() {
        if c.letter == c2 && c.dagesh && hebrew::is_begedkefet(c.letter) {
            c.dagesh = false;
            return Some(hebrew::render(&seq));
        }
    }
    None
}

/// Plural sibling of the silent-sheva twin, built from the root (the generator's
/// primary 3mp/2mp form is the reduced sheva shape, so there is no hataf plural
/// for the transform to re-point): yaḥpōṣû יַחְפֹּצוּ, taḥpōṣû.
fn pe_guttural_imperfect_holam_plural_silent_variant(root: &Root, pgn: Pgn) -> Option<String> {
    use Vowel::*;
    if !hebrew::is_guttural(root.pe()) {
        return None;
    }
    let prefix = match (pgn.person, pgn.gender) {
        (Some(Person::Third), Some(Gender::Masculine)) => letter::YOD,
        (Some(Person::Second), Some(Gender::Masculine)) => letter::TAV,
        _ => return None,
    };
    let mut c2 = rad(root.ayin(), 2).with_vowel(Holam);
    if hebrew::is_begedkefet(c2.letter) {
        c2 = c2.with_dagesh();
    }
    let seq = vec![
        Cons::new(prefix).with_vowel(Patah),
        rad(root.pe(), 1).with_vowel(Sheva),
        c2,
        rad(root.lamed(), 3),
        Cons::new(letter::VAV).with_dagesh(),
    ];
    Some(hebrew::render(&seq))
}

/// A-theme (stative) sibling of [`pe_guttural_imperfect_holam_plural_silent_variant`]:
/// for stative I-guttural roots the imperfect theme is patah, which restores to
/// qamats under the stress of the bare -û plural — yeḥdālû יֶחְדָּלוּ, yaḥdālû,
/// and the 2mp taḥdālû. The C1 guttural closes on a silent sheva. Both prefix
/// vowels (segol / patah) are emitted, since the stative preformative varies.
/// Built from the root: the generator's 3mp/2mp is the reduced-sheva shape, so
/// there is no full-grade plural for a re-pointing transform to act on.
fn pe_guttural_imperfect_a_plural_silent_variants(root: &Root, pgn: Pgn) -> Vec<String> {
    use Vowel::*;
    if !hebrew::is_guttural(root.pe()) {
        return Vec::new();
    }
    let prefix = match (pgn.person, pgn.gender) {
        (Some(Person::Third), Some(Gender::Masculine)) => letter::YOD,
        (Some(Person::Second), Some(Gender::Masculine)) => letter::TAV,
        _ => return Vec::new(),
    };
    let mut c2 = rad(root.ayin(), 2).with_vowel(Qamats);
    if hebrew::is_begedkefet(c2.letter) {
        c2 = c2.with_dagesh();
    }
    [Segol, Patah]
        .into_iter()
        .map(|pv| {
            let seq = vec![
                Cons::new(prefix).with_vowel(pv),
                rad(root.pe(), 1).with_vowel(Sheva),
                c2,
                rad(root.lamed(), 3),
                Cons::new(letter::VAV).with_dagesh(),
            ];
            hebrew::render(&seq)
        })
        .collect()
}

/// I-guttural Qal imperfect silent-sheva twin: a guttral that closes the first
/// syllable takes a silent sheva (and a begedkefet C2 a dagesh lene) rather than
/// the composite sheva the generator emits — yaḥăp̄ōṣ יַחֲפֹץ → yaḥpōṣ יַחְפֹּץ,
/// likewise the plural יַחְפֹּצוּ. Transforms any generated form whose first
/// radical guttural carries a hataf: re-points it to a silent sheva. Caller
/// gates to (Qal, PeGuttural, imperfect family). Additive — for gutturals that
/// genuinely keep the hataf (יַעֲמֹד) the silent twin simply never matches.
fn pe_guttural_qal_silent_twin_variant(root: &Root, text: &str) -> Option<String> {
    let g = root.pe();
    if !hebrew::is_guttural(g) {
        return None;
    }
    let mut seq = hebrew::parse_pointed(text);
    for i in 0..seq.len() {
        if seq[i].letter == g
            && matches!(
                seq[i].vowel,
                Some(Vowel::HatafPatah) | Some(Vowel::HatafSegol)
            )
        {
            seq[i].vowel = Some(Vowel::Sheva);
            if let Some(c2) = seq.get_mut(i + 1)
                && hebrew::is_begedkefet(c2.letter)
            {
                c2.dagesh = true;
            }
            return Some(hebrew::render(&seq));
        }
    }
    None
}

/// Geminate Qal perfect (a-class): the two identical radicals contract to one
/// doubled C2, which the strong builder leaves as two separate radicals (רָבַב).
/// The contracted paradigm — sab סַב, sabbâ סַבָּה, sabbû סַבּוּ, and the
/// ô-linked consonantal-suffix forms sabbôṯā סַבּוֹתָ, sabbôṯî, sabbôṯem, sabbônû
/// — cascades to all a-class geminates (סבב, תמם, רבב, קלל). Built from the root;
/// caller gates to (Qal, Perfect, Geminate). Additive (an e/o-class geminate
/// just never matches the a-class spelling).
fn geminate_qal_perfect_variant(root: &Root, pgn: Pgn) -> Option<String> {
    use Vowel::*;
    let c1 = rad(root.pe(), 1).with_vowel(Patah);
    let cc = rad(root.ayin(), 2).with_dagesh();
    // ô link (vav + holam) shared by every consonantal-suffix form.
    let olink = || vec![cc, Cons::new(letter::VAV).with_vowel(Holam)];
    let seq: Vec<Cons> = match (pgn.person, pgn.gender, pgn.number) {
        (Some(Person::Third), Some(Gender::Masculine), Some(Number::Singular)) => {
            vec![c1, rad(root.ayin(), 2)]
        }
        (Some(Person::Third), Some(Gender::Feminine), Some(Number::Singular)) => {
            vec![c1, cc.with_vowel(Qamats), Cons::new(letter::HE)]
        }
        (Some(Person::Third), _, Some(Number::Plural)) => {
            vec![c1, cc, Cons::new(letter::VAV).with_dagesh()]
        }
        (Some(Person::Second), Some(Gender::Masculine), Some(Number::Singular)) => {
            let mut s = vec![c1];
            s.extend(olink());
            s.push(Cons::new(letter::TAV).with_vowel(Qamats));
            s
        }
        (Some(Person::Second), Some(Gender::Feminine), Some(Number::Singular)) => {
            let mut s = vec![c1];
            s.extend(olink());
            s.push(Cons::new(letter::TAV));
            s
        }
        (Some(Person::First), _, Some(Number::Singular)) => {
            let mut s = vec![c1];
            s.extend(olink());
            s.push(Cons::new(letter::TAV).with_vowel(Hiriq));
            s.push(Cons::new(letter::YOD));
            s
        }
        (Some(Person::Second), Some(Gender::Masculine), Some(Number::Plural)) => {
            let mut s = vec![c1];
            s.extend(olink());
            s.push(Cons::new(letter::TAV).with_vowel(Segol));
            s.push(Cons::new(letter::MEM));
            s
        }
        (Some(Person::Second), Some(Gender::Feminine), Some(Number::Plural)) => {
            let mut s = vec![c1];
            s.extend(olink());
            s.push(Cons::new(letter::TAV).with_vowel(Segol));
            s.push(Cons::new(letter::NUN));
            s
        }
        (Some(Person::First), _, Some(Number::Plural)) => {
            let mut s = vec![c1];
            s.extend(olink());
            s.push(Cons::new(letter::NUN));
            s.push(Cons::new(letter::VAV).with_dagesh());
            s
        }
        _ => return None,
    };
    Some(hebrew::render(&seq))
}

/// Pausal twin of the Qal perfect's interior theme: in pause the stress holds on
/// the second radical and its vowel lengthens to qamats — the patah of the
/// consonantal-suffix forms (qāṭaltā → qāṭāltā, yāḏaʕtā → יָדָעְתָּ, šāmartā →
/// שָׁמָרְתָּ) and the reduced sheva of the vocalic 3fs/3cp (qāṭᵊlû → qāṭālû,
/// yāṣᵊʔû → יָצָאוּ). [`pausal_qamats_variant`] only lengthens a *word-final*
/// patah, so it misses these interior cases. We re-point the middle radical
/// (root.ayin()); caller gates to (Qal, Perfect). Additive alternant.
fn pausal_perfect_c2_variant(root: &Root, text: &str) -> Option<String> {
    let c2 = root.ayin();
    let mut seq = hebrew::parse_pointed(text);
    for c in seq.iter_mut() {
        // patah (qāṭaltā), the reduced sheva of 3fs/3cp, or the hataf-patah a
        // guttural C2 takes there (nilḥămû → נִלְחָמוּ) all lengthen to qamats.
        if c.letter == c2
            && matches!(
                c.vowel,
                Some(Vowel::Patah) | Some(Vowel::Sheva) | Some(Vowel::HatafPatah)
            )
        {
            c.vowel = Some(Vowel::Qamats);
            return Some(hebrew::render(&seq));
        }
    }
    None
}

/// I-yod (true pe-yod) Hiphil ê-twin: original-yod verbs (יטב, ילל, יבש, ינק)
/// form the Hiphil with a tsere-yod preformative — hêṭîḇ הֵיטִיב, hêlîl הֵילִיל,
/// inf. abs. hêṭēḇ הֵיטֵב — where original-vav verbs (ישב, ירד) take the holam-vav
/// the generator emits for every pe-yod root (hôṭîḇ הוֹטִיב). Surface yod can't
/// tell the two apart, so we emit the ê-twin for any pe-yod Hiphil by swapping
/// the וֹ preformative mater for ֵי; additive, so the wrong twin never matches a
/// true I-vav verb. One transform covers the whole paradigm (the preformative is
/// uniform across perfect/imperfect/participle/infinitive).
fn pe_yod_hiphil_e_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    if seq.len() >= 2 && seq[1].letter == letter::VAV && seq[1].vowel == Some(Vowel::Holam) {
        seq[0].vowel = Some(Vowel::Tsere);
        seq[1] = Cons::new(letter::YOD);
        return Some(hebrew::render(&seq));
    }
    None
}

/// I-guttural Qal imperfect/wayyiqtol vocalic plural (3mp/2mp): the o-theme keeps
/// its holam before the -û ending and the first-radical guttural takes a composite
/// sheva — yaʕămōḏû יַעֲמֹדוּ, taḥăp̄ōṣû, yaʕăḇōrû — where the strong builder
/// reduces to a plain sheva (יַעַמְדוּ). Built straight from the root: prefix +
/// patah, C1 + hataf-patah, C2 + holam, C3, vav (shureq). Caller gates to (Qal,
/// PeGuttural, Imperfect|Wayyiqtol, 3mp|2mp). Additive.
fn pe_guttural_imperfect_holam_plural_variant(root: &Root, pgn: Pgn) -> Option<String> {
    use Vowel::*;
    if !hebrew::is_guttural(root.pe()) {
        return None;
    }
    let prefix = match (pgn.person, pgn.gender) {
        (Some(Person::Third), Some(Gender::Masculine)) => letter::YOD,
        (Some(Person::Second), Some(Gender::Masculine)) => letter::TAV,
        _ => return None,
    };
    let seq = vec![
        Cons::new(prefix).with_vowel(Patah),
        rad(root.pe(), 1).with_vowel(HatafPatah),
        rad(root.ayin(), 2).with_vowel(Holam),
        rad(root.lamed(), 3),
        Cons::new(letter::VAV).with_dagesh(),
    ];
    Some(hebrew::render(&seq))
}

/// Pausal twin of an I-guttural III-He Qal imperative: the propretonic
/// hataf-patah under the word-initial guttural lengthens to qamats when the
/// stress retracts in pause — ʕănê (עֲנֵה) → ʕānê (עָנֵה), ʕăśê → ʕāśê, ʕălê →
/// ʕālê. Returns `None` unless the first consonant is a guttural bearing a
/// hataf-patah.
fn guttural_hataf_pausal_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let first = seq.first_mut()?;
    if first.vowel == Some(Vowel::HatafPatah) && hebrew::is_guttural(first.letter) {
        first.vowel = Some(Vowel::Qamats);
        return Some(hebrew::render(&seq));
    }
    None
}

/// Apocopated (short) alternant of the Hiphil jussive/wayyiqtol: the long
/// theme vowel î — written as a consonant + hiriq followed by a yod mater
/// (yaqrîb יַקְרִיב, wayyaškîm וַיַּשְׁכִּים, wayyaggîd וַיַּגִּיד) — shortens to
/// tsere with the mater dropped (yaqrēb יַקְרֵב, wayyaškēm וַיַּשְׁכֵּם, wayyaggēd
/// וַיַּגֵּד). This is the dominant prose shape of the Hiphil short imperfect.
/// Returns the shortened spelling, or `None` when no such hiriq-yod theme is
/// present (e.g. the vocalic-suffix 2fs תַּקְרִבִי, where the yod is the suffix).
/// III-He Hiphil apocopated wayyiqtol/jussive: the segol/tsere + etymological he
/// drops, leaving C2 to close the word on a silent sheva — yašqeh יַשְׁקֶה →
/// wayyašq וַיַּשְׁקְ, wattašq. Requires a `…C2(tsere/segol) HE` ending.
fn lamed_he_hiphil_apocope_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 3 || seq[n - 1].letter != letter::HE {
        // apply_lamed_he now apocopates the derived-stem jussive itself,
        // breaking the final cluster with a helping vowel (yašeq). The
        // cluster-kept spelling is the attested twin for some lexemes —
        // wayyašq וַיַּשְׁקְ: both C1's helping segol and the final C2 close
        // on shevas.
        if n >= 3
            && seq[n - 1].vowel.is_none()
            && seq[n - 2].vowel == Some(Vowel::Segol)
            && !hebrew::is_guttural(seq[n - 2].letter)
        {
            seq[n - 2].vowel = Some(Vowel::Sheva);
            seq[n - 1].vowel = Some(Vowel::Sheva);
            // Undo the helping vowel's prefix attenuation: with the cluster
            // kept, the preformative carries its plain patah (וַיַּשְׁקְ).
            if n >= 4 && seq[n - 3].vowel == Some(Vowel::Segol) {
                seq[n - 3].vowel = Some(Vowel::Patah);
            }
            return Some(hebrew::render(&seq));
        }
        return None;
    }
    if matches!(seq[n - 2].vowel, Some(Vowel::Tsere | Vowel::Segol)) {
        // …C2(tsere/segol) HE  → drop the he, C2 closes on a silent sheva.
        seq.pop();
        seq.last_mut().unwrap().vowel = Some(Vowel::Sheva);
        Some(hebrew::render(&seq))
    } else if seq[n - 2].letter == letter::YOD
        && seq[n - 2].vowel.is_none()
        && seq
            .get(n - 3)
            .is_some_and(|c| c.vowel == Some(Vowel::Hiriq))
    {
        // …C2(hiriq) YOD(mater) HE  (plene î) → drop the yod mater and he, C2
        // closes on a silent sheva: wayyašqîh → wayyašq וַיַּשְׁקְ.
        seq.truncate(n - 2);
        seq.last_mut().unwrap().vowel = Some(Vowel::Sheva);
        Some(hebrew::render(&seq))
    } else {
        None
    }
}

fn hiphil_apocope_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    // The theme î is a vowelless yod mater preceded by a hiriq-bearing
    // consonant and followed by a closing C3 (…rî-b). The prefix yod carries
    // its own vowel, and a word-final plene yod has nothing after it, so both
    // are excluded.
    let i = seq
        .iter()
        .position(|c| c.letter == letter::YOD && c.vowel.is_none() && !c.dagesh)?;
    if i == 0 || i + 1 >= seq.len() || seq[i - 1].vowel != Some(Vowel::Hiriq) {
        return None;
    }
    seq[i - 1].vowel = Some(Vowel::Tsere);
    seq.remove(i);
    Some(hebrew::render(&seq))
}

/// Plene alternant of a Hiphil imperfect/jussive/wayyiqtol with a vocalic
/// suffix: the theme î is short here and the generator writes it defectively
/// (yaggidû יַגִּדוּ, yaqribû יַקְרִבוּ, taqribî תַּקְרִבִי), but the full spelling
/// retaining the yod mater is at least as common (yaggîdû יַגִּידוּ, yaqrîbû
/// יַקְרִיבוּ). Inserts a yod after the theme C2's hiriq. Returns `None` when the
/// theme is already plene (hollow Hiphil yāqîmû, where a yod already follows).
fn hiphil_plene_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    // In the Hiphil the prefix is patah and C1 a sheva, so the first plain
    // hiriq belongs to the theme C2.
    let i = seq.iter().position(|c| c.vowel == Some(Vowel::Hiriq))?;
    let next = seq.get(i + 1)?;
    if next.letter == letter::YOD {
        return None;
    }
    seq.insert(i + 1, Cons::new(letter::YOD));
    Some(hebrew::render(&seq))
}

/// Defective alternant of a plene Hiphil imperfect/jussive/wayyiqtol with a
/// vocalic suffix: the long theme î is written with a yod mater in the base
/// (yaggîdû יַגִּידוּ, yaqrîbû יַקְרִיבוּ), but the Masoretic text very commonly
/// drops the mater (yaggidû יַגִּדוּ, wayyaggidû וַיַּגִּדוּ — the dominant prose
/// wayyiqtol shape). Removes the yod mater that follows the theme C2's hiriq.
/// Returns `None` when the theme carries no such yod (already defective, or the
/// yod is the suffix as in the 2fs תַּקְרִיבִי).
fn hiphil_defective_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    // The prefix is patah and C1 a sheva, so the first plain hiriq is the theme
    // C2's; a vowelless yod immediately after it is the mater to drop.
    let i = seq.iter().position(|c| c.vowel == Some(Vowel::Hiriq))?;
    let yod = seq.get(i + 1)?;
    if yod.letter != letter::YOD || yod.vowel.is_some() || yod.dagesh {
        return None;
    }
    // Keep at least the suffix vowel beyond the dropped yod, so we never strip a
    // word-final plene yod (which would be the 2fs/1cs suffix, not the mater).
    if i + 2 >= seq.len() {
        return None;
    }
    seq.remove(i + 1);
    Some(hebrew::render(&seq))
}

/// Pausal twin of a Piel/Hithpael vocalic-suffix imperfect: the theme tsere of
/// the doubled C2 (yᵉḏabbēr) reduces to a vocal sheva before a vocalic suffix
/// (yᵉḏabbᵊrû, יְדַבְּרוּ), but in pause the stress retracts onto C2 and the full
/// tsere is restored — yᵉḏabbērû (יְדַבֵּרוּ). The C2 is the dageshed consonant
/// carrying that sheva, preceded by the patah of C1.
fn piel_pausal_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let i = seq
        .iter()
        .position(|c| c.dagesh && c.vowel == Some(Vowel::Sheva))?;
    if i == 0 || seq[i - 1].vowel != Some(Vowel::Patah) {
        return None;
    }
    seq[i].vowel = Some(Vowel::Tsere);
    Some(hebrew::render(&seq))
}

/// Pausal twin of a vocalic-suffix Qal imperfect: the theme vowel that reduced
/// to a vocal sheva before the suffix (yišmᵊrû יִשְׁמְרוּ, yēlᵊḵû יֵלְכוּ, yippᵊlû
/// יִפְּלוּ) is restored in pause (yišmōrû יִשְׁמֹרוּ, yēlēḵû יֵלֵכוּ, yippōlû
/// יִפֹּלוּ). The restored vowel is read from the 3ms (zero-suffix) form, which
/// keeps the full theme on the same consonant — so the per-root theme (holam,
/// tsere, …) is recovered without hard-coding it. `zero` and `vocalic` share a
/// prefix up to and including the theme consonant; the last consonant that is
/// full in `zero` but a vocal sheva in `vocalic` is the theme.
fn qal_pausal_variant(zero: &str, vocalic: &str) -> Option<String> {
    let zseq = hebrew::parse_pointed(zero);
    let mut vseq = hebrew::parse_pointed(vocalic);
    let mut theme = None;
    // Skip index 0: the imperfect preformative (yod/tav/aleph/nun) differs
    // between the 3ms reference and a 2nd-person target, but the radicals after
    // it align.
    for i in 1..vseq.len().min(zseq.len()) {
        if vseq[i].letter != zseq[i].letter {
            break;
        }
        if vseq[i].vowel == Some(Vowel::Sheva)
            && matches!(
                zseq[i].vowel,
                Some(Vowel::Holam | Vowel::Tsere | Vowel::Patah | Vowel::Qamats | Vowel::Segol)
            )
        {
            theme = Some((i, zseq[i].vowel.unwrap()));
        }
    }
    let (i, v) = theme?;
    vseq[i].vowel = Some(v);
    Some(hebrew::render(&vseq))
}

/// Patah-theme twin of the Piel perfect 3ms: beside the tsere theme (qiṭṭēl,
/// בֵּרֵךְ, חִשֵּׁב, טִהֵר) the a-grade is freely attested — bēraḵ בֵּרַךְ,
/// ḥiššaḇ חִשַּׁב, ṭihar טִהַר. Lowers the last tsere/segol to patah.
fn piel_perfect_patah_theme_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let c = seq
        .iter_mut()
        .rev()
        .find(|c| matches!(c.vowel, Some(Vowel::Tsere | Vowel::Segol)))?;
    c.vowel = Some(Vowel::Patah);
    Some(hebrew::render(&seq))
}

/// Segol-prefix twin of the III-He Hiphil perfect: heḡlâ הֶגְלָה beside the
/// regular hiḡlâ הִגְלָה.
fn hiphil_perfect_segol_prefix_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let first = seq.first_mut()?;
    if first.letter != letter::HE || first.vowel != Some(Vowel::Hiriq) {
        return None;
    }
    first.vowel = Some(Vowel::Segol);
    Some(hebrew::render(&seq))
}

/// Short twin of the III-He imperfect 3fp/2fp: the -еynâ ending drops its final
/// he and the nun carries the qamats — tihyeynā תִּהְיֶינָה → tihyeyn תִּהְיֶיןָ.
fn lamed_he_imperfect_fp_short_variant(text: &str) -> Option<String> {
    let seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 4
        || seq[n - 1].letter != letter::HE
        || seq[n - 1].vowel.is_some()
        || seq[n - 2].letter != letter::NUN
        || seq[n - 2].vowel != Some(Vowel::Qamats)
        || seq[n - 3].letter != letter::YOD
    {
        return None;
    }
    Some(hebrew::render(&seq[..n - 1]))
}

/// He-less spelling of the 3fp/2fp -nâ afformative: the final he is dropped and
/// the nun takes the qamats directly — tāšōḇnâ תָּשֹׁבְנָה → תָּשֹׁבְןָ. Additive.
fn fp_nah_heless_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n >= 2
        && seq[n - 1].letter == letter::HE
        && seq[n - 1].vowel.is_none()
        && seq[n - 2].letter == letter::NUN
        && seq[n - 2].vowel == Some(Vowel::Qamats)
    {
        seq.pop();
        Some(hebrew::render(&seq))
    } else {
        None
    }
}

/// Archaic III-He plural imperfect/imperative with the retained third radical:
/// the original yod resurfaces before the plural -û ending, the stem consonant
/// taking a qamats — yeʾĕṯāyû יֶאֱתָיוּ (beside the contracted יֶאֱתוּ),
/// yehĕmāyûn יֶהֱמָיוּן. Both the bare -āyû and the paragogic-nun -āyûn are
/// attested, so emit both.
fn lamed_he_retained_yod_plural_variants(text: &str) -> Vec<String> {
    let seq = hebrew::parse_pointed(text);
    let n = seq.len();
    // Locate the plural shureq vav, optionally trailed by a paragogic nun.
    let vi = if n >= 1
        && seq[n - 1].letter == letter::VAV
        && seq[n - 1].dagesh
        && seq[n - 1].vowel.is_none()
    {
        n - 1
    } else if n >= 2
        && seq[n - 1].letter == letter::NUN
        && seq[n - 1].vowel.is_none()
        && seq[n - 2].letter == letter::VAV
        && seq[n - 2].dagesh
        && seq[n - 2].vowel.is_none()
    {
        n - 2
    } else {
        return Vec::new();
    };
    // The stem consonant before the shureq must be bare (the slot the elided
    // third radical vacated); skip if it already carries a vowel or is a mater.
    if vi == 0
        || seq[vi - 1].vowel.is_some()
        || matches!(seq[vi - 1].letter, letter::VAV | letter::YOD)
    {
        return Vec::new();
    }
    let mut stem = seq[..vi].to_vec();
    stem[vi - 1].vowel = Some(Vowel::Qamats);
    stem.push(Cons::new(letter::YOD));
    stem.push(Cons::new(letter::VAV).with_dagesh());
    let bare = hebrew::render(&stem);
    stem.push(Cons::new(letter::NUN));
    let paragogic = hebrew::render(&stem);
    vec![bare, paragogic]
}

/// III-Aleph infinitive construct on the III-He pattern: מלא and its kin take
/// the -ōṯ / -ôṯ ending with C2 holding the o-grade holam — mallōʾṯ לְמַלֹּאת,
/// mallōʾôṯ לְמַלֹּאות (Piel), mᵊlōʾṯ כִּמְלֹאת (Qal). Both the bare -ṯ and the
/// mater-vav -ôṯ are attested.
fn lamed_aleph_inf_ot_variants(text: &str) -> Vec<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 2 || seq[n - 1].letter != letter::ALEF || seq[n - 1].vowel.is_some() {
        return Vec::new();
    }
    // The o-grade holam sits on C2 (already there in the Qal; the Piel's tsere
    // lowers to it).
    seq[n - 2].vowel = Some(Vowel::Holam);
    let mut with_t = seq.clone();
    with_t.push(Cons::new(letter::TAV));
    let mut with_vt = seq;
    with_vt.push(Cons::mater(letter::VAV));
    with_vt.push(Cons::new(letter::TAV));
    vec![hebrew::render(&with_t), hebrew::render(&with_vt)]
}

/// Pausal-tsere twin of a III-He form: the final segol before the etymological
/// he lengthens in pause — tᵊḡalleh תְּגַלֶּה → tᵊḡallēh תְּגַלֵּה.
fn lamed_he_pausal_tsere_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 2
        || seq[n - 1].letter != letter::HE
        || seq[n - 1].vowel.is_some()
        || seq[n - 2].vowel != Some(Vowel::Segol)
    {
        return None;
    }
    seq[n - 2].vowel = Some(Vowel::Tsere);
    Some(hebrew::render(&seq))
}

/// Hiriq twin of the Niphal imperfect 1cs preformative: ʾiddārēš אִדָּרֵשׁ beside
/// the regular ʾeddārēš אֶדָּרֵשׁ.
fn niphal_1cs_hiriq_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    // The aleph preformative may sit behind a wayyiqtol vav (וָאִמָּלְטָה).
    let i = if seq.first().map(|c| c.letter) == Some(letter::VAV) {
        1
    } else {
        0
    };
    let c = seq.get_mut(i)?;
    if c.letter != letter::ALEF || c.vowel != Some(Vowel::Segol) {
        return None;
    }
    c.vowel = Some(Vowel::Hiriq);
    Some(hebrew::render(&seq))
}

/// Qubuts twin of the Hophal preformative: the o-grade qamats-qatan
/// (מׇפְקָד, הׇשְׁלַךְ) has a freely attested u-grade — מֻפְקָד, הֻשְׁלַךְ.
fn hophal_qubuts_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let c = seq.iter_mut().find(|c| c.vowel.is_some())?;
    if c.vowel != Some(Vowel::QamatsQatan) {
        return None;
    }
    c.vowel = Some(Vowel::Qubuts);
    Some(hebrew::render(&seq))
}

/// Hiriq + silent-sheva twin of a I-guttural Niphal perfect: nehĕyâ נֶהֱיָה has
/// the attested i-grade nihyâ נִהְיָה (and 3fs נִהְיְתָה). The preformative takes
/// hiriq and the guttural closes the syllable on a plain sheva.
fn pe_guttural_niphal_hiriq_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    if seq.len() < 3
        || seq[0].letter != letter::NUN
        || !matches!(seq[0].vowel, Some(Vowel::Segol | Vowel::Patah))
        || !hebrew::is_guttural(seq[1].letter)
        || !matches!(
            seq[1].vowel,
            Some(Vowel::HatafSegol | Vowel::HatafPatah | Vowel::Sheva | Vowel::Segol)
        )
    {
        return None;
    }
    seq[0].vowel = Some(Vowel::Hiriq);
    seq[1].vowel = Some(Vowel::Sheva);
    Some(hebrew::render(&seq))
}

/// Segol twin of the חיה/היה short wayyiqtol/jussive: the sheva preformative
/// fills to segol with the forte restored — wayḥî וַיְחִי beside wayyeḥî וַיֶּחִי.
fn sheva_prefix_segol_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    if seq.len() >= 3
        && seq[0].letter == letter::VAV
        && matches!(seq[0].vowel, Some(Vowel::Patah))
        && seq[1].vowel == Some(Vowel::Sheva)
    {
        seq[1].vowel = Some(Vowel::Segol);
        seq[1].dagesh = true;
        return Some(hebrew::render(&seq));
    }
    if seq.len() >= 2 && seq[0].vowel == Some(Vowel::Sheva) {
        seq[0].vowel = Some(Vowel::Segol);
        return Some(hebrew::render(&seq));
    }
    None
}

/// Defective twin of the שחה Hithpael infinitive: the vav-holam mater before
/// the tav written as a bare vav — הִשְׁתַּחֲוֹת → הִשְׁתַּחֲות.
fn shachah_inf_defective_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 2 || seq[n - 1].letter != letter::TAV {
        return None;
    }
    let v = &mut seq[n - 2];
    if v.letter != letter::VAV || v.vowel != Some(Vowel::Holam) {
        return None;
    }
    v.vowel = None;
    Some(hebrew::render(&seq))
}

/// Pausal twin of the שחה Hithpael short wayyiqtol/jussive: the patah on the
/// infix tav lengthens — wayyištáḥû וַיִּשְׁתַּחוּ → וַיִּשְׁתָּחוּ.
fn shachah_pausal_qamats_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let i = seq
        .iter()
        .position(|c| c.letter == letter::TAV && c.vowel == Some(Vowel::Patah))?;
    if seq.get(i + 1).map(|c| c.letter) != Some(letter::HET) {
        return None;
    }
    seq[i].vowel = Some(Vowel::Qamats);
    Some(hebrew::render(&seq))
}

/// Geminate Hiphil active participle — mēCaC/mēCēC with the doubled radical
/// contracted: mēraʿ מֵרַע (רעע), plural mᵊrēCîm with the propretonic reduced
/// (mᵊrēʿîm מְרֵעִים).
fn geminate_hiphil_participle_variants(root: &Root, pgn: Pgn) -> Vec<String> {
    use Vowel::*;
    let theme = if hebrew::is_guttural(root.lamed()) || root.lamed() == letter::RESH {
        Patah
    } else {
        Tsere
    };
    if pgn == Pgn::gn(Gender::Masculine, Number::Singular) {
        vec![hebrew::render(&[
            Cons::new(letter::MEM).with_vowel(Tsere),
            rad(root.pe(), 1).with_vowel(theme),
            rad(root.lamed(), 3),
        ])]
    } else if pgn == Pgn::gn(Gender::Masculine, Number::Plural) {
        vec![hebrew::render(&[
            Cons::new(letter::MEM).with_vowel(Sheva),
            rad(root.pe(), 1).with_vowel(Tsere),
            rad(root.lamed(), 3).with_vowel(Hiriq),
            Cons::mater(letter::YOD),
            Cons::new(letter::MEM),
        ])]
    } else {
        Vec::new()
    }
}

/// Geminate Niphal imperfect with the doubled radical contracted: yissaḇ יִסַּב,
/// plural yissabbû יִסַּבּוּ (the doubling restored before the vocalic subject).
fn geminate_niphal_imperfect_variant(root: &Root, pgn: Pgn) -> Vec<String> {
    use Vowel::*;
    let pre = prefix_letter(pgn);
    // A guttural/resh C1 rejects the doubling and the prefix hiriq
    // compensates to tsere — tēḥattû תֵּחַתּוּ, with an attested qamats grade
    // (תֵּחָתּוּ) beside the patah.
    let guttural = hebrew::is_guttural(root.pe()) || root.pe() == letter::RESH;
    let (pre_v, c1_grades): (Vowel, &[Vowel]) = if guttural {
        (Tsere, &[Patah, Qamats])
    } else {
        (Hiriq, &[Patah])
    };
    let mut out = Vec::new();
    for &c1v in c1_grades {
        let mut c1 = rad(root.pe(), 1).with_vowel(c1v);
        if !guttural {
            c1 = c1.with_dagesh();
        }
        match imperfect_suffix_kind(pgn) {
            Suffix::Zero => out.push(hebrew::render(&[
                Cons::new(pre).with_vowel(pre_v),
                c1,
                rad(root.lamed(), 3),
            ])),
            Suffix::Vocalic => {
                let fem_sg =
                    pgn.gender == Some(Gender::Feminine) && pgn.number == Some(Number::Singular);
                let mut seq = vec![Cons::new(pre).with_vowel(pre_v), c1];
                if fem_sg {
                    seq.push(rad(root.lamed(), 3).with_dagesh().with_vowel(Hiriq));
                    seq.push(Cons::mater(letter::YOD));
                } else {
                    seq.push(rad(root.lamed(), 3).with_dagesh());
                    seq.push(oshureq());
                }
                out.push(hebrew::render(&seq));
            }
            _ => {}
        }
    }
    out
}

/// Geminate Hophal imperfect with the doubled radical contracted and the
/// pe-nun-style doubling of C1: yukkattû יֻכַּתּוּ (כתת), yûšad. Qubuts
/// preformative, C1 dagesh + patah, the contracted radical doubling before a
/// vocalic subject.
fn geminate_hophal_imperfect_variant(root: &Root, pgn: Pgn) -> Option<String> {
    use Vowel::*;
    let pre = prefix_letter(pgn);
    if hebrew::is_guttural(root.pe()) || root.pe() == letter::RESH {
        return None;
    }
    let c1 = rad(root.pe(), 1).with_vowel(Patah).with_dagesh();
    match imperfect_suffix_kind(pgn) {
        Suffix::Zero => Some(hebrew::render(&[
            Cons::new(pre).with_vowel(Qubuts),
            c1,
            rad(root.lamed(), 3),
        ])),
        Suffix::Vocalic => {
            let fem_sg =
                pgn.gender == Some(Gender::Feminine) && pgn.number == Some(Number::Singular);
            let mut seq = vec![Cons::new(pre).with_vowel(Qubuts), c1];
            if fem_sg {
                seq.push(rad(root.lamed(), 3).with_dagesh().with_vowel(Hiriq));
                seq.push(Cons::mater(letter::YOD));
            } else {
                seq.push(rad(root.lamed(), 3).with_dagesh());
                seq.push(oshureq());
            }
            Some(hebrew::render(&seq))
        }
        _ => None,
    }
}

/// Geminate Hophal perfect, doubled radical contracted onto a u-class prefix:
/// hûḥad הוּחַד (3ms), hûḥaddâ הוּחַדָּה (3fs), hûḥaddû (3cp). The prefix writes
/// plene (he + shureq vav) or defective (he + qubuts); C1 takes patah and the
/// contracted radical doubles (dagesh) before a vocalic afformative. Only the
/// vocalic-afformative persons (3ms/3fs/3cp) are modelled; the consonantal-
/// suffix persons take a different (ô-linking) stem left to a later pass.
/// Additive.
fn geminate_hophal_perfect_variant(root: &Root, pgn: Pgn) -> Vec<String> {
    use Vowel::*;
    let lamed = root.lamed();
    let dagesh_ok = !hebrew::rejects_dagesh(lamed);
    let tail: Vec<Cons> = match (pgn.person, pgn.gender, pgn.number) {
        // 3ms: a single radical closes the word; no afformative, no doubling.
        (Some(Person::Third), Some(Gender::Masculine), Some(Number::Singular)) => {
            vec![rad(lamed, 3)]
        }
        // 3fs -â: doubled radical + qamats, with a he mater.
        (Some(Person::Third), Some(Gender::Feminine), Some(Number::Singular)) => {
            let mut c = rad(lamed, 3).with_vowel(Qamats);
            if dagesh_ok {
                c = c.with_dagesh();
            }
            vec![c, Cons::mater(letter::HE)]
        }
        // 3cp -û: doubled radical + shureq.
        (Some(Person::Third), Some(Gender::Common), Some(Number::Plural)) => {
            let mut c = rad(lamed, 3);
            if dagesh_ok {
                c = c.with_dagesh();
            }
            vec![c, oshureq()]
        }
        _ => return Vec::new(),
    };
    let c1 = rad(root.pe(), 1).with_vowel(Patah);
    let mut out = Vec::new();
    // Plene prefix: he + shureq vav (הוּ-).
    let mut plene = vec![Cons::new(letter::HE), oshureq(), c1];
    plene.extend(tail.clone());
    out.push(hebrew::render(&plene));
    // Defective prefix: he + qubuts (הֻ-).
    let mut defec = vec![Cons::new(letter::HE).with_vowel(Qubuts), c1];
    defec.extend(tail);
    out.push(hebrew::render(&defec));
    out
}

/// Geminate Hiphil imperfect with the doubled radical contracted: yāḥēl יָחֵל,
/// plural yāḥēllû יָחֵלּוּ (the doubling restored before the vocalic subject).
fn geminate_hiphil_imperfect_variant(root: &Root, pgn: Pgn) -> Option<String> {
    use Vowel::*;
    let pre = Cons::new(prefix_letter(pgn)).with_vowel(Qamats);
    let c1 = rad(root.pe(), 1).with_vowel(Tsere);
    match imperfect_suffix_kind(pgn) {
        Suffix::Zero => Some(hebrew::render(&[pre, c1, rad(root.lamed(), 3)])),
        Suffix::Vocalic => {
            let fem_sg =
                pgn.gender == Some(Gender::Feminine) && pgn.number == Some(Number::Singular);
            let mut seq = vec![pre, c1];
            if fem_sg {
                seq.push(rad(root.lamed(), 3).with_dagesh().with_vowel(Hiriq));
                seq.push(Cons::mater(letter::YOD));
            } else {
                seq.push(rad(root.lamed(), 3).with_dagesh());
                seq.push(oshureq());
            }
            Some(hebrew::render(&seq))
        }
        _ => None,
    }
}

/// Geminate Hiphil perfect before a consonantal afformative: the contracted
/// stem takes the linking ô — hăšimmōṯî הֲשִׁמֹּתִי (and the unreduced hē-grade
/// הֲ/הִ twins).
fn geminate_hiphil_perfect_otav_variants(root: &Root, pgn: Pgn) -> Vec<String> {
    use Vowel::*;
    let tail: Vec<Cons> = match (pgn.person, pgn.gender, pgn.number) {
        (Some(Person::First), _, Some(Number::Singular)) => {
            vec![ocv(letter::TAV, Hiriq), Cons::mater(letter::YOD)]
        }
        (Some(Person::Second), Some(Gender::Masculine), Some(Number::Singular)) => {
            vec![ocv(letter::TAV, Qamats)]
        }
        (Some(Person::Second), Some(Gender::Feminine), Some(Number::Singular)) => {
            vec![Cons::new(letter::TAV)]
        }
        (Some(Person::First), _, Some(Number::Plural)) => {
            vec![Cons::new(letter::NUN), oshureq()]
        }
        (Some(Person::Second), Some(Gender::Masculine), Some(Number::Plural)) => {
            vec![ocv(letter::TAV, Segol), Cons::new(letter::MEM)]
        }
        (Some(Person::Second), Some(Gender::Feminine), Some(Number::Plural)) => {
            vec![ocv(letter::TAV, Segol), Cons::new(letter::NUN)]
        }
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    // An undoubleable contracted radical (רעע, פרר-with-resh-like classes)
    // rejects the dagesh; the C1 hiriq compensates to tsere (hărēʿōṯem
    // הֲרֵעֹתֶם) or holds short virtually — emit both grades.
    let grades: &[(Vowel, bool)] = if hebrew::rejects_dagesh(root.lamed()) {
        &[(Tsere, false), (Hiriq, false)]
    } else {
        &[(Hiriq, true)]
    };
    for he_vowel in [HatafPatah, Patah, Hiriq, Tsere] {
        for &(c1v, dagesh) in grades {
            // The linking ô on the (doubled) contracted radical writes either
            // defectively — the holam sits on C2 (haḥillōṯî הַחִלֹּתִי) — or
            // plene, with C2 bare and a vav mater carrying the holam
            // (haḥillôṯā הַחִלּוֹתָ).
            for plene in [false, true] {
                let mut c2 = rad(root.lamed(), 3);
                if dagesh {
                    c2 = c2.with_dagesh();
                }
                let mut seq = vec![
                    Cons::new(letter::HE).with_vowel(he_vowel),
                    rad(root.pe(), 1).with_vowel(c1v),
                ];
                if plene {
                    seq.push(c2);
                    seq.push(Cons::new(letter::VAV).with_vowel(Holam));
                } else {
                    c2.vowel = Some(Holam);
                    seq.push(c2);
                }
                seq.extend(tail.clone());
                out.push(hebrew::render(&seq));
            }
        }
    }
    out
}

/// Hollow Hophal perfect hûCāC (הוּבָא, הוּשַׁב): he + shureq prefix, the theme
/// on C1, the etymological vav absorbed.
fn hollow_hophal_perfect_variants(root: &Root, pgn: Pgn) -> Vec<String> {
    use Vowel::*;
    let he = Cons::new(letter::HE);
    let shureq = oshureq();
    let c3 = rad(root.lamed(), 3);
    let three = |g: Gender, num: Number| pgn == Pgn::new(Person::Third, g, num);
    let mut out = Vec::new();
    let themes = [Patah, Qamats];
    if three(Gender::Masculine, Number::Singular) {
        for theme in themes {
            out.push(hebrew::render(&[
                he,
                shureq,
                rad(root.pe(), 1).with_vowel(theme),
                c3,
            ]));
        }
    } else if three(Gender::Common, Number::Plural) {
        out.push(hebrew::render(&[
            he,
            shureq,
            rad(root.pe(), 1).with_vowel(Sheva),
            c3,
            oshureq(),
        ]));
    } else if three(Gender::Feminine, Number::Singular) {
        out.push(hebrew::render(&[
            he,
            shureq,
            rad(root.pe(), 1).with_vowel(Sheva),
            c3.with_vowel(Qamats),
            Cons::new(letter::HE),
        ]));
    }
    out
}

/// Contracted perfect of the doubly-weak hollow III-aleph Hiphil (בוא): before
/// a consonantal afformative the stem is hēḇēʾ-, the aleph quiescing so the tav
/// stays soft — hēḇēʾṯā הֵבֵאתָ, hăḇêṯem (the reduced-prefix weqatal grade)
/// וַהֲבֵאתֶם.
fn hollow_lamed_aleph_hiphil_perfect_variants(root: &Root, pgn: Pgn) -> Vec<String> {
    use Vowel::*;
    let tail: Vec<Cons> = match (pgn.person, pgn.gender, pgn.number) {
        (Some(Person::First), _, Some(Number::Singular)) => {
            vec![ocv(letter::TAV, Hiriq), Cons::mater(letter::YOD)]
        }
        (Some(Person::Second), Some(Gender::Masculine), Some(Number::Singular)) => {
            vec![ocv(letter::TAV, Qamats)]
        }
        (Some(Person::Second), Some(Gender::Feminine), Some(Number::Singular)) => {
            vec![Cons::new(letter::TAV)]
        }
        (Some(Person::First), _, Some(Number::Plural)) => {
            vec![Cons::new(letter::NUN), oshureq()]
        }
        (Some(Person::Second), Some(Gender::Masculine), Some(Number::Plural)) => {
            vec![ocv(letter::TAV, Segol), Cons::new(letter::MEM)]
        }
        (Some(Person::Second), Some(Gender::Feminine), Some(Number::Plural)) => {
            vec![ocv(letter::TAV, Segol), Cons::new(letter::NUN)]
        }
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    for he_vowel in [Tsere, HatafPatah] {
        let mut seq = vec![
            Cons::new(letter::HE).with_vowel(he_vowel),
            rad(root.pe(), 1).with_vowel(Tsere),
            rad(root.lamed(), 3),
        ];
        seq.extend(tail.clone());
        out.push(hebrew::render(&seq));
    }
    out
}

/// III-aleph derived-stem perfect with a consonantal afformative: the strong
/// builder closes the aleph on a silent sheva (qinnaʾtî קִנַּאְתִּי), but the
/// aleph quiesces — the theme patah lengthens to tsere and the now-postvocalic
/// tav loses its dagesh lene: qinnēʾṯî קִנֵּאתִי.
fn lamed_aleph_derived_perfect_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let i = seq
        .iter()
        .position(|c| c.letter == letter::ALEF && c.vowel == Some(Vowel::Sheva))?;
    if i == 0 || i + 1 >= seq.len() || seq[i - 1].vowel != Some(Vowel::Patah) {
        return None;
    }
    seq[i - 1].vowel = Some(Vowel::Tsere);
    seq[i].vowel = None;
    if seq[i + 1].letter == letter::TAV {
        seq[i + 1].dagesh = false;
    }
    Some(hebrew::render(&seq))
}

/// Pausal twin of the segholate -t infinitive construct: the first segol
/// lengthens to qamats — šeḇeṯ שֶׁבֶת → šāḇeṯ שָׁבֶת (לָשָׁבֶת).
fn segholate_inf_pausal_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    let n = seq.len();
    if n < 3
        || seq[n - 1].letter != letter::TAV
        || seq[n - 1].vowel.is_some()
        || seq[0].vowel != Some(Vowel::Segol)
        || seq[1].vowel != Some(Vowel::Segol)
    {
        return None;
    }
    seq[0].vowel = Some(Vowel::Qamats);
    Some(hebrew::render(&seq))
}

fn apply_guttural(seq: &mut [Cons], root: &Root) -> bool {
    // For each consonant slot:
    //   - If the letter is a guttural (א ה ח ע) or resh and the slot has
    //     dagesh, remove the dagesh (gutturals don't double; resh usually
    //     doesn't).
    //   - If the slot has a sheva and the letter is a guttural, swap the
    //     sheva for hataf-patah (typical) — but only when the sheva is
    //     vocal, which we approximate by "guttural is the FIRST letter of
    //     the word" or "preceded by a long vowel".
    let mut changed = false;
    for i in 0..seq.len() {
        // A word-final he with a dagesh is a mappiq (the consonantal 3fs
        // suffix -āh, לְכָדָהּ), not a doubling — leave it alone.
        if seq[i].letter == letter::HE && i == seq.len() - 1 {
            continue;
        }
        if seq[i].dagesh && hebrew::rejects_dagesh(seq[i].letter) {
            seq[i].dagesh = false;
            changed = true;
            // Compensatory lengthening: resh and aleph reject the doubling
            // outright (unlike ה/ח/ע, which keep the short vowel via "virtual"
            // doubling), so the short vowel under the preceding consonant
            // lengthens — piʕʕel → pēʕel. This realises the doubled binyanim of
            // II-resh/aleph roots: yᵉḇārēḵ (יְבָרֵךְ), bērak (בֵּרַךְ), bōrak.
            // (Resh/aleph never carry a lene dagesh, so any dagesh here is the
            // forte we just stripped.)
            if matches!(seq[i].letter, letter::RESH | letter::ALEF) && i > 0 {
                let lengthened = match seq[i - 1].vowel {
                    Some(Vowel::Patah) => Some(Vowel::Qamats),
                    Some(Vowel::Hiriq) => Some(Vowel::Tsere),
                    Some(Vowel::Qubuts) => Some(Vowel::Holam),
                    _ => None,
                };
                if let Some(v) = lengthened {
                    seq[i - 1].vowel = Some(v);
                }
            } else if matches!(seq[i].letter, letter::HE | letter::HET | letter::AYIN) {
                // Virtual doubling: if the guttural carries a sheva, it must be
                // vocal (hataf-patah) despite following a short vowel.
                if seq[i].vowel == Some(Vowel::Sheva) {
                    seq[i].vowel = Some(Vowel::HatafPatah);
                }
            }
        }
        if let Some(Vowel::Sheva) = seq[i].vowel
            && hebrew::is_guttural(seq[i].letter)
        {
            // A guttural cannot bear a vocal sheva — it takes a hataf
            // vowel instead. But a SILENT sheva (one that closes a
            // syllable) stays plain. The sheva is vocal when the
            // guttural is word-initial, follows a bare consonant or
            // another sheva (cluster), or follows a long vowel; it is
            // silent after a short vowel (e.g. יָדַעְתִּי — the ayin closes
            // the short patah syllable דַע, so the sheva is silent).
            let vocal = match seq.get(i.wrapping_sub(1)).filter(|_| i > 0) {
                None => true,
                Some(prev) => match prev.vowel {
                    None | Some(Vowel::Sheva) => true,
                    Some(v) => is_long_vowel(v),
                },
            };
            if vocal {
                seq[i].vowel = Some(Vowel::HatafPatah);
                changed = true;
            }
        }
    }
    // A hataf vowel cannot stand immediately before a consonant bearing a
    // vocal sheva: the would-be open syllable closes, promoting the hataf to
    // its full short vowel and silencing the following sheva. So the I-guttural
    // imperfect/wayyiqtol plural is yaʕam-ḏû (וַיַּעַמְדוּ), not yaʕă-mə-ḏû.
    for i in 0..seq.len().saturating_sub(1) {
        let promoted = match seq[i].vowel {
            Some(Vowel::HatafPatah) => Some(Vowel::Patah),
            Some(Vowel::HatafSegol) => Some(Vowel::Segol),
            Some(Vowel::HatafQamats) => Some(Vowel::Qamats),
            _ => None,
        };
        if let Some(v) = promoted
            && seq[i + 1].vowel == Some(Vowel::Sheva)
        {
            seq[i].vowel = Some(v);
            changed = true;
        }
    }

    let _ = root; // future use: PeGuttural-specific prefix vowel changes
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morphology::root::Root;

    /// Find the form for a specific (binyan, form, pgn) in a paradigm.
    fn pick(p: &Paradigm, binyan: Binyan, form: Form, pgn: Pgn) -> &VerbForm {
        p.forms
            .iter()
            .find(|f| f.binyan == binyan && f.form == form && f.pgn == pgn)
            .unwrap_or_else(|| panic!("missing {binyan:?} {form:?} {pgn:?}"))
    }

    const THREE_MS: Pgn = Pgn::new(Person::Third, Gender::Masculine, Number::Singular);
    const TWO_MS: Pgn = Pgn::new(Person::Second, Gender::Masculine, Number::Singular);

    #[test]
    fn qal_perfect_3ms_strong() {
        // קָטַל
        let root = Root::parse("קטל").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Qal, Form::Perfect, THREE_MS);
        assert_eq!(f.text, "\u{05E7}\u{05B8}\u{05D8}\u{05B7}\u{05DC}");
    }

    #[test]
    fn qal_imperfect_3ms_strong() {
        // יִקְטֹל
        let root = Root::parse("קטל").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Qal, Form::Imperfect, THREE_MS);
        assert_eq!(
            f.text,
            "\u{05D9}\u{05B4}\u{05E7}\u{05B0}\u{05D8}\u{05B9}\u{05DC}",
        );
    }

    #[test]
    fn niphal_perfect_3ms_strong() {
        // נִקְטַל
        let root = Root::parse("קטל").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Niphal, Form::Perfect, THREE_MS);
        assert_eq!(
            f.text,
            "\u{05E0}\u{05B4}\u{05E7}\u{05B0}\u{05D8}\u{05B7}\u{05DC}",
        );
    }

    #[test]
    fn niphal_imperfect_3ms_strong() {
        // יִקָּטֵל
        let root = Root::parse("קטל").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Niphal, Form::Imperfect, THREE_MS);
        assert_eq!(
            f.text,
            "\u{05D9}\u{05B4}\u{05E7}\u{05BC}\u{05B8}\u{05D8}\u{05B5}\u{05DC}",
        );
    }

    #[test]
    fn piel_perfect_3ms_strong() {
        // קִטֵּל — qof+hiriq, tet+dagesh+tsere, lamed
        let root = Root::parse("קטל").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Piel, Form::Perfect, THREE_MS);
        assert_eq!(f.text, "\u{05E7}\u{05B4}\u{05D8}\u{05BC}\u{05B5}\u{05DC}",);
    }

    #[test]
    fn pual_perfect_3ms_strong() {
        // קֻטַּל
        let root = Root::parse("קטל").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Pual, Form::Perfect, THREE_MS);
        assert_eq!(f.text, "\u{05E7}\u{05BB}\u{05D8}\u{05BC}\u{05B7}\u{05DC}",);
    }

    #[test]
    fn hithpael_perfect_3ms_strong() {
        // הִתְקַטֵּל
        let root = Root::parse("קטל").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Hithpael, Form::Perfect, THREE_MS);
        assert_eq!(
            f.text,
            "\u{05D4}\u{05B4}\u{05EA}\u{05B0}\u{05E7}\u{05B7}\u{05D8}\u{05BC}\u{05B5}\u{05DC}",
        );
    }

    #[test]
    fn hiphil_perfect_3ms_strong() {
        // הִקְטִיל
        let root = Root::parse("קטל").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Hiphil, Form::Perfect, THREE_MS);
        assert_eq!(
            f.text,
            "\u{05D4}\u{05B4}\u{05E7}\u{05B0}\u{05D8}\u{05B4}\u{05D9}\u{05DC}",
        );
    }

    #[test]
    fn hophal_perfect_3ms_strong() {
        // הָקְטַל — using qamats-qatan (U+05C7) for the long "o" of Hophal.
        let root = Root::parse("קטל").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Hophal, Form::Perfect, THREE_MS);
        assert_eq!(
            f.text,
            "\u{05D4}\u{05C7}\u{05E7}\u{05B0}\u{05D8}\u{05B7}\u{05DC}",
        );
    }

    #[test]
    fn hiphil_perfect_2ms_drops_mater_yod() {
        // הִקְטַלְתָּ — consonantal suffix triggers patah on C2, no mater yod.
        let root = Root::parse("קטל").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Hiphil, Form::Perfect, TWO_MS);
        assert_eq!(
            f.text,
            // he+hiriq, qof+sheva, tet+patah, lamed+sheva, tav+dagesh+qamats
            "\u{05D4}\u{05B4}\u{05E7}\u{05B0}\u{05D8}\u{05B7}\u{05DC}\u{05B0}\u{05EA}\u{05BC}\u{05B8}",
        );
    }

    #[test]
    fn pe_nun_qal_imperfect_3ms() {
        // נפל → יִפֹּל
        let root = Root::parse("נפל").unwrap();
        assert!(root.has(Gizra::PeNun));
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Qal, Form::Imperfect, THREE_MS);
        assert_eq!(
            f.text,
            // yod+hiriq, pe+dagesh+holam, lamed
            "\u{05D9}\u{05B4}\u{05E4}\u{05BC}\u{05B9}\u{05DC}",
        );
    }

    #[test]
    fn hollow_qal_perfect_3ms() {
        // קום → קָם
        let root = Root::parse("קום").unwrap();
        assert!(root.has(Gizra::Hollow));
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Qal, Form::Perfect, THREE_MS);
        assert_eq!(f.text, "\u{05E7}\u{05B8}\u{05DD}");
    }

    #[test]
    fn lamed_he_qal_perfect_3ms() {
        // בנה → בָּנָה (word-initial bet picks up dagesh lene)
        let root = Root::parse("בנה").unwrap();
        assert!(root.has(Gizra::LamedHe));
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Qal, Form::Perfect, THREE_MS);
        // bet+dagesh+qamats, nun+qamats, he
        assert_eq!(f.text, "\u{05D1}\u{05BC}\u{05B8}\u{05E0}\u{05B8}\u{05D4}",);
    }

    #[test]
    fn pe_nun_qal_imperfect_1cp() {
        // נפל → 1cp נִפֹּל (regression: prefix-nun used to clash with C1 nun).
        let root = Root::parse("נפל").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(
            &p,
            Binyan::Qal,
            Form::Imperfect,
            Pgn::new(Person::First, Gender::Common, Number::Plural),
        );
        // nun+hiriq, pe+dagesh+holam, lamed
        assert_eq!(f.text, "\u{05E0}\u{05B4}\u{05E4}\u{05BC}\u{05B9}\u{05DC}",);
    }

    #[test]
    fn hollow_qal_imperfect_3ms() {
        // קום → 3ms יָקוּם
        let root = Root::parse("קום").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Qal, Form::Imperfect, THREE_MS);
        // yod+qamats, qof, vav+dagesh, mem (final)
        assert_eq!(f.text, "\u{05D9}\u{05B8}\u{05E7}\u{05D5}\u{05BC}\u{05DD}",);
    }

    #[test]
    fn lamed_he_qal_imperfect_3ms() {
        // בנה → 3ms יִבְנֶה
        let root = Root::parse("בנה").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Qal, Form::Imperfect, THREE_MS);
        // yod+hiriq, bet+sheva, nun+segol, he
        assert_eq!(
            f.text,
            "\u{05D9}\u{05B4}\u{05D1}\u{05B0}\u{05E0}\u{05B6}\u{05D4}",
        );
    }

    #[test]
    fn lamed_he_qal_jussive_3ms_apocopated() {
        // בנה → 3ms jussive יִבֶן (apocopated: he dropped, segol on C1,
        // C2 closes the syllable).
        let root = Root::parse("בנה").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Qal, Form::Jussive, THREE_MS);
        assert_eq!(f.text, "יִבֶן");
    }

    #[test]
    fn hollow_qal_jussive_3ms_short() {
        // קום → 3ms jussive יָקֹם (short form: holam on C1, middle radical
        // dropped, no vav-shureq mater).
        let root = Root::parse("קום").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Qal, Form::Jussive, THREE_MS);
        assert_eq!(f.text, "יָקֹם");
    }

    #[test]
    fn paradigm_has_all_binyanim_all_forms() {
        let root = Root::parse("קטל").unwrap();
        let p = generate_paradigm(&root);
        for &binyan in &Binyan::ALL {
            for &form in FORMS_FOR_PARADIGM.iter() {
                if !binyan_has_form(binyan, form) {
                    continue;
                }
                let count = p
                    .forms
                    .iter()
                    .filter(|f| f.binyan == binyan && f.form == form)
                    .count();
                assert!(count > 0, "no forms for {binyan:?} {form:?}");
            }
        }
    }

    #[test]
    fn raah_wayyiqtol_1cs() {
        let root = Root::parse("ראה").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(
            &p,
            Binyan::Qal,
            Form::Wayyiqtol,
            Pgn::new(Person::First, Gender::Common, Number::Singular),
        );
        assert_eq!(f.text, "וָאֵרֶא");
    }

    #[test]
    fn chazaq_wayyiqtol_3ms() {
        let root = Root::parse("חזק").unwrap();
        let p = generate_paradigm(&root);
        let f = pick(&p, Binyan::Qal, Form::Wayyiqtol, THREE_MS);
        // וַיֶּחֱזַק (vav-patah, yod-dagesh-segol, het-hatafsegol, zayin-patah, qof)
        assert_eq!(f.text, "וַיֶּחֱזַק");
    }

    #[test]
    fn hiphil_imperative_2ms_paragogic_guttural() {
        // הַאֲזִינָה — Hiphil Imperative 2ms emphatic of אזן.
        let root = Root::parse("אזן").unwrap();
        let p = generate_paradigm(&root);
        for f in &p.forms {
            if f.binyan == Binyan::Hiphil && f.form == Form::Imperative {
                println!("{:?} {:?} {:?} {}", f.binyan, f.form, f.pgn, f.text);
            }
        }
        let f = p.forms.iter().find(|f| {
            f.binyan == Binyan::Hiphil
                && f.form == Form::Imperative
                && f.pgn == TWO_MS
                && f.text == "הַאֲזִינָה"
        });
        assert!(f.is_some(), "expected הַאֲזִינָה in paradigm");
    }

    fn find_object_suffix(
        forms: &[VerbForm],
        binyan: Binyan,
        form: Form,
        obj: Pgn,
    ) -> Option<String> {
        forms
            .iter()
            .find(|f| {
                f.binyan == binyan
                    && f.form == form
                    && f.pgn == Pgn::none()
                    && f.object_suffix == Some(obj)
            })
            .map(|f| f.text.clone())
    }

    #[test]
    fn strong_qal_inf_construct_3ms() {
        // קטל → qoṭlô (קָטְלוֹ — strong Qal inf construct shifts to qoṭl-grade).
        let root = Root::parse("קטל").unwrap();
        let p = generate_paradigm(&root);
        let text = find_object_suffix(&p.forms, Binyan::Qal, Form::InfinitiveConstruct, OBJ_3MS);
        let expected = hebrew::render(&[
            Cons::radical('ק', 1).with_vowel(Vowel::Qamats),
            Cons::radical('ט', 2).with_vowel(Vowel::Sheva),
            Cons::radical('ל', 3),
            Cons::new(letter::VAV).with_vowel(Vowel::Holam),
        ]);
        assert_eq!(text, Some(expected));
    }

    #[test]
    fn strong_qal_inf_construct_3mp() {
        let root = Root::parse("קטל").unwrap();
        let p = generate_paradigm(&root);
        let text = find_object_suffix(&p.forms, Binyan::Qal, Form::InfinitiveConstruct, OBJ_3MP);
        let expected = hebrew::render(&[
            Cons::radical('ק', 1).with_vowel(Vowel::Qamats),
            Cons::radical('ט', 2).with_vowel(Vowel::Sheva),
            Cons::radical('ל', 3).with_vowel(Vowel::Qamats),
            Cons::new(letter::MEM),
        ]);
        assert_eq!(text, Some(expected));
    }

    #[test]
    fn pe_yod_inf_construct_3mp() {
        // יצא → צֵאת → ṣêṯām (צֵאתָם — non-segholate weak Qal retains the III-aleph).
        let root = Root::parse("יצא").unwrap();
        let p = generate_paradigm(&root);
        let text = find_object_suffix(&p.forms, Binyan::Qal, Form::InfinitiveConstruct, OBJ_3MP);
        let expected = hebrew::render(&[
            Cons::new('צ').with_vowel(Vowel::Tsere),
            Cons::new(letter::ALEF),
            Cons::new('ת').with_vowel(Vowel::Qamats),
            Cons::new(letter::MEM),
        ]);
        assert_eq!(text, Some(expected));
    }

    #[test]
    fn pe_yod_segholate_inf_construct_3ms() {
        // ישב → שֶׁבֶת → šibtô (שִׁבְתּוֹ — segholate weak Qal).
        let root = Root::parse("ישב").unwrap();
        let p = generate_paradigm(&root);
        let text = find_object_suffix(&p.forms, Binyan::Qal, Form::InfinitiveConstruct, OBJ_3MS);
        let expected = hebrew::render(&[
            Cons::new('ש').with_vowel(Vowel::Hiriq),
            Cons::new('ב').with_vowel(Vowel::Sheva),
            Cons::new('ת'),
            Cons::new(letter::VAV).with_vowel(Vowel::Holam),
        ]);
        assert_eq!(text, Some(expected));
    }

    #[test]
    fn pe_yod_segholate_inf_construct_3mp() {
        // ישב → šibtām (שִׁבְתָּם).
        let root = Root::parse("ישב").unwrap();
        let p = generate_paradigm(&root);
        let text = find_object_suffix(&p.forms, Binyan::Qal, Form::InfinitiveConstruct, OBJ_3MP);
        let expected = hebrew::render(&[
            Cons::new('ש').with_vowel(Vowel::Hiriq),
            Cons::new('ב').with_vowel(Vowel::Sheva),
            Cons::new('ת').with_vowel(Vowel::Qamats),
            Cons::new(letter::MEM),
        ]);
        assert_eq!(text, Some(expected));
    }

    #[test]
    fn pe_yod_segholate_inf_construct_2ms() {
        // ישב → šibtᵊḵā (שִׁבְתְּךָ).
        let root = Root::parse("ישב").unwrap();
        let p = generate_paradigm(&root);
        let text = find_object_suffix(&p.forms, Binyan::Qal, Form::InfinitiveConstruct, OBJ_2MS);
        let expected = hebrew::render(&[
            Cons::new('ש').with_vowel(Vowel::Hiriq),
            Cons::new('ב').with_vowel(Vowel::Sheva),
            Cons::new('ת').with_vowel(Vowel::Sheva),
            Cons::new(letter::KAF).with_vowel(Vowel::Qamats),
        ]);
        assert_eq!(text, Some(expected));
    }

    /// Whether the paradigm contains some (possibly alternant) form for the
    /// given (binyan, form, pgn) whose surface equals `text`. Compared via
    /// [`canonical_key`] so source-literal mark ordering (NFC puts a vowel
    /// before the dagesh; `render` emits dagesh first) can't cause a spurious
    /// byte-level mismatch.
    fn has_text(p: &Paradigm, binyan: Binyan, form: Form, pgn: Pgn, text: &str) -> bool {
        let key = crate::morphology::parse::canonical_key(text);
        p.forms.iter().any(|f| {
            f.binyan == binyan
                && f.form == form
                && f.pgn == pgn
                && crate::morphology::parse::canonical_key(&f.text) == key
        })
    }

    const THREE_MP: Pgn = Pgn::new(Person::Third, Gender::Masculine, Number::Plural);

    #[test]
    fn niphal_iguttural_silent_sheva_takes_dagesh_lene() {
        // הפך Niphal Perfect 3ms: the silent-sheva twin of nehĕp̄aḵ closes the
        // first syllable, so the pe takes a dagesh lene → נֶהְפַּךְ.
        let root = Root::parse("הפך").unwrap();
        let p = generate_paradigm(&root);
        // נ+segol, ה+sheva, פ+dagesh+patah, ך (final kaf carries silent sheva).
        let expected = hebrew::render(&[
            Cons::new(letter::NUN).with_vowel(Vowel::Segol),
            Cons::new(letter::HE).with_vowel(Vowel::Sheva),
            {
                let mut c = Cons::new(letter::PE).with_vowel(Vowel::Patah);
                c.dagesh = true;
                c
            },
            Cons::new(letter::KAF),
        ]);
        assert!(has_text(
            &p,
            Binyan::Niphal,
            Form::Perfect,
            THREE_MS,
            &expected
        ));
    }

    #[test]
    fn pe_yod_retained_imperfect() {
        // ירש Qal Imperfect: the retained-yod twins yîraš / yîrᵊšû.
        let root = Root::parse("ירש").unwrap();
        let p = generate_paradigm(&root);
        assert!(has_text(&p, Binyan::Qal, Form::Imperfect, THREE_MS, "יִירַשׁ"));
        assert!(has_text(
            &p,
            Binyan::Qal,
            Form::Imperfect,
            THREE_MP,
            "יִירְשׁוּ"
        ));
    }

    #[test]
    fn hiphil_participle_begedkefet_c2_dagesh() {
        // זכר Hiphil participle ms: maqtîl closes the prefix on the zayin's
        // silent sheva, so the begedkefet kaf takes a dagesh lene → מַזְכִּיר.
        let root = Root::parse("זכר").unwrap();
        let p = generate_paradigm(&root);
        let expected = hebrew::render(&[
            Cons::new(letter::MEM).with_vowel(Vowel::Patah),
            Cons::new(letter::ZAYIN).with_vowel(Vowel::Sheva),
            {
                let mut c = Cons::new(letter::KAF).with_vowel(Vowel::Hiriq);
                c.dagesh = true;
                c
            },
            Cons::new(letter::YOD),
            Cons::new(letter::RESH),
        ]);
        let pgn = Pgn::gn(Gender::Masculine, Number::Singular);
        assert!(has_text(
            &p,
            Binyan::Hiphil,
            Form::ParticipleActive,
            pgn,
            &expected
        ));
    }

    #[test]
    fn lamed_guttural_perfect_2fs_helping_patah() {
        // ידע Qal Perfect 2fs: the final ayin takes a furtive helping patah
        // before the -t afformative → יָדַעַתְּ (the tav carries a dagesh).
        let root = Root::parse("ידע").unwrap();
        let p = generate_paradigm(&root);
        let pgn = Pgn::new(Person::Second, Gender::Feminine, Number::Singular);
        let expected = hebrew::render(&[
            Cons::new(letter::YOD).with_vowel(Vowel::Qamats),
            Cons::new(letter::DALET).with_vowel(Vowel::Patah),
            Cons::new(letter::AYIN).with_vowel(Vowel::Patah),
            {
                let mut c = Cons::new(letter::TAV).with_vowel(Vowel::Sheva);
                c.dagesh = true;
                c
            },
        ]);
        assert!(has_text(&p, Binyan::Qal, Form::Perfect, pgn, &expected));
    }

    #[test]
    fn pe_vav_niphal_begedkefet_c2_stays_spirant() {
        // יתר Niphal Perfect 3ms: ni+w contracts to nô-, an open vav-mater
        // syllable, so the begedkefet tav is spirant → נוֹתַר (not נוֹתַּר).
        let root = Root::parse("יתר").unwrap();
        let p = generate_paradigm(&root);
        let expected = hebrew::render(&[
            Cons::new(letter::NUN),
            Cons::new(letter::VAV).with_vowel(Vowel::Holam),
            Cons::new(letter::TAV).with_vowel(Vowel::Patah),
            Cons::new(letter::RESH),
        ]);
        assert!(has_text(
            &p,
            Binyan::Niphal,
            Form::Perfect,
            THREE_MS,
            &expected
        ));
    }

    #[test]
    fn hollow_paragogic_nun_reduces_prefix() {
        // שוב Qal Imperfect 3mp: the paragogic-nun twin reduces the propretonic
        // prefix qamats to sheva → yᵉšûḇûn (יְשׁוּבוּן).
        let root = Root::parse("שוב").unwrap();
        let p = generate_paradigm(&root);
        assert!(has_text(
            &p,
            Binyan::Qal,
            Form::Imperfect,
            THREE_MP,
            "יְשׁוּבוּן"
        ));
    }

    /// Whether any form in the paradigm (base, alternant, or object-suffixed)
    /// has the given surface text, compared via [`canonical_key`] (see
    /// [`has_text`]).
    fn any_text(p: &Paradigm, text: &str) -> bool {
        let key = crate::morphology::parse::canonical_key(text);
        p.forms
            .iter()
            .any(|f| crate::morphology::parse::canonical_key(&f.text) == key)
    }

    #[test]
    fn pe_yod_retained_defective_wayyiqtol() {
        // ירא Qal Wayyiqtol 3ms: the retained-yod twin written defectively —
        // וַיִּרָא beside the plene וַיִּירָא.
        let root = Root::parse("ירא").unwrap();
        let p = generate_paradigm(&root);
        assert!(has_text(&p, Binyan::Qal, Form::Wayyiqtol, THREE_MS, "וַיִּרָא"));
    }

    #[test]
    fn pe_yod_perfect_2mp_hiriq() {
        // ירש Qal Perfect 2mp: the i-grade twin yᵊrištem (יְרִשְׁתֶּם — surfaced
        // as the conjunction sandhi וִירִשְׁתֶּם, peeled by the parser).
        let root = Root::parse("ירש").unwrap();
        let p = generate_paradigm(&root);
        let pgn = Pgn::new(Person::Second, Gender::Masculine, Number::Plural);
        assert!(has_text(&p, Binyan::Qal, Form::Perfect, pgn, "יְרִשְׁתֶּם"));
    }

    #[test]
    fn pe_aleph_patah_wayyiqtol_object_suffix() {
        // אסר Qal wayyiqtol + 3ms object suffix on the patah-grade host:
        // wayyaʾasrēhû וַיַּאַסְרֵהוּ.
        let root = Root::parse("אסר").unwrap();
        let p = generate_paradigm(&root);
        assert!(any_text(&p, "וַיַּאַסְרֵהוּ"));
    }

    #[test]
    fn pe_aleph_energic_hataf_theme() {
        // אכל Qal imperfect + 3ms energic suffix: the theme under the suffix
        // is a hataf-patah after the quiescent aleph → תֹּאכֲלֶנּוּ.
        let root = Root::parse("אכל").unwrap();
        let p = generate_paradigm(&root);
        assert!(any_text(&p, "תֹּאכֲלֶנּוּ"));
    }

    #[test]
    fn qara_inf_construct_liqrat() {
        // קרא lexicalized -t inf-construct: קְרַאת (לִקְרַאת after the peel)
        // and the qamats suffix grade קְרָאתוֹ / קְרָאתִי.
        let root = Root::parse("קרא").unwrap();
        let p = generate_paradigm(&root);
        assert!(any_text(&p, "קְרַאת"));
        assert!(any_text(&p, "קְרָאתוֹ"));
        assert!(any_text(&p, "קְרָאתִי"));
    }

    #[test]
    fn pe_yod_hiphil_no_dagesh_after_holam_mater() {
        // ידה Hiphil: the ô mater opens C1's syllable, so the begedkefet C2
        // spirantises — yôḏeh יוֹדֶה (not יוֹדֶּה), 3mp יוֹדוּ, imv 2mp הוֹדוּ.
        let root = Root::parse("ידה").unwrap();
        let p = generate_paradigm(&root);
        assert!(any_text(&p, "יוֹדֶה"));
        assert!(any_text(&p, "יוֹדוּ"));
        assert!(any_text(&p, "הוֹדוּ"));
    }

    #[test]
    fn lamed_he_vocalic_imperative() {
        // III-He vocalic imperative: the he elides — ʿăśû עֲשׂוּ, ʿăśî עֲשִׂי.
        let root = Root::parse("עשה").unwrap();
        let p = generate_paradigm(&root);
        let two_mp = Pgn::new(Person::Second, Gender::Masculine, Number::Plural);
        let two_fs = Pgn::new(Person::Second, Gender::Feminine, Number::Singular);
        assert!(has_text(&p, Binyan::Qal, Form::Imperative, two_mp, "עֲשׂוּ"));
        assert!(has_text(&p, Binyan::Qal, Form::Imperative, two_fs, "עֲשִׂי"));
    }

    #[test]
    fn hollow_hiphil_short_wayyiqtol() {
        // Hollow Hiphil wayyiqtol 3ms apocopates: וַיָּקֶם (קום), וַיָּשֶׁב (שוב),
        // III-aleph keeps the tsere — וַיָּבֵא (בוא). Imperative 2ms הָקֵם.
        let qum = generate_paradigm(&Root::parse("קום").unwrap());
        assert!(has_text(
            &qum,
            Binyan::Hiphil,
            Form::Wayyiqtol,
            THREE_MS,
            "וַיָּקֶם"
        ));
        let two_ms = Pgn::new(Person::Second, Gender::Masculine, Number::Singular);
        assert!(has_text(
            &qum,
            Binyan::Hiphil,
            Form::Imperative,
            two_ms,
            "הָקֵם"
        ));
        let shub = generate_paradigm(&Root::parse("שוב").unwrap());
        assert!(has_text(
            &shub,
            Binyan::Hiphil,
            Form::Wayyiqtol,
            THREE_MS,
            "וַיָּשֶׁב"
        ));
        let bo = generate_paradigm(&Root::parse("בוא").unwrap());
        assert!(has_text(
            &bo,
            Binyan::Hiphil,
            Form::Wayyiqtol,
            THREE_MS,
            "וַיָּבֵא"
        ));
    }

    #[test]
    fn hollow_hiphil_bo_spelling_variants() {
        // בוא Hiphil/Hophal: the defective zero-suffix yāḇiʾ יָבִא, Hophal
        // yûḇāʾ יוּבָא, the reduced-prefix suffixed wayyiqtols וַיְבִיאֵנִי and
        // (3mp, defective) וַיְבִאֻהוּ, and the hataf suffix link יְבִיאֲךָ.
        let p = generate_paradigm(&Root::parse("בוא").unwrap());
        assert!(any_text(&p, "יָבִא"));
        assert!(any_text(&p, "יוּבָא"));
        assert!(any_text(&p, "וַיְבִיאֵנִי"));
        assert!(any_text(&p, "וַיְבִאֻהוּ"));
        assert!(any_text(&p, "יְבִיאֲךָ"));
    }

    #[test]
    fn lamed_he_perfect_tsere_yod_twin() {
        // צוה Piel/Pual perfect 1cs: the tsere-yod linking spelling beside
        // the hiriq-yod base — צִוֵּיתִי, צֻוֵּיתִי.
        let p = generate_paradigm(&Root::parse("צוה").unwrap());
        let one_cs = Pgn::new(Person::First, Gender::Common, Number::Singular);
        assert!(has_text(&p, Binyan::Piel, Form::Perfect, one_cs, "צִוֵּיתִי"));
        assert!(has_text(&p, Binyan::Pual, Form::Perfect, one_cs, "צֻוֵּיתִי"));
    }

    #[test]
    fn piel_inf_construct_reduced_suffix() {
        // Piel inf-construct + suffix reduces the theme tsere to sheva:
        // dabbᵊrô דַּבְּרוֹ (בְּדַבְּרוֹ), qaddᵊšô קַדְּשׁוֹ (לְקַדְּשׁוֹ).
        let p = generate_paradigm(&Root::parse("דבר").unwrap());
        assert!(any_text(&p, "דַּבְּרוֹ"));
        let q = generate_paradigm(&Root::parse("קדש").unwrap());
        assert!(any_text(&q, "קַדְּשׁוֹ"));
    }

    #[test]
    fn laqah_inf_construct_suffixes() {
        // לקח inf-construct qaḥaṯ קַחַת takes suffixes on the qaḥt- base:
        // qaḥtāh קַחְתָּהּ (לְקַחְתָּהּ), qaḥtô קַחְתּוֹ.
        let p = generate_paradigm(&Root::parse("לקח").unwrap());
        assert!(any_text(&p, "קַחְתָּהּ"));
        assert!(any_text(&p, "קַחְתּוֹ"));
    }

    #[test]
    fn natan_inf_construct_tet() {
        // נתן Qal inf-construct: both nuns clip → תֵּת, suffixed תִּתּוֹ / תִּתִּי.
        let root = Root::parse("נתן").unwrap();
        let p = generate_paradigm(&root);
        assert!(any_text(&p, "תֵּת"));
        assert!(any_text(&p, "תִּתּוֹ"));
        assert!(any_text(&p, "תִּתִּי"));
    }

    #[test]
    fn consonantal_he_stative_strong() {
        // גבה "be high": a consonantal-he stative, not weak III-He. It inflects
        // as a strong triliteral with a mappiq on the word-final he — gāḇah
        // גָּבַהּ (not the III-He גָּבָה), gāḇhû גָּבְהוּ, and the a-theme wayyiqtol
        // wayyiḡbah וַיִּגְבַּהּ.
        let root = Root::parse("גבה").unwrap();
        assert!(!root.has(Gizra::LamedHe), "גבה must not be classed III-He");
        let p = generate_paradigm(&root);
        assert!(has_text(&p, Binyan::Qal, Form::Perfect, THREE_MS, "גָּבַהּ"));
        let three_cp = Pgn::new(Person::Third, Gender::Common, Number::Plural);
        assert!(has_text(&p, Binyan::Qal, Form::Perfect, three_cp, "גָּבְהוּ"));
        assert!(has_text(
            &p,
            Binyan::Qal,
            Form::Wayyiqtol,
            THREE_MS,
            "וַיִּגְבַּהּ"
        ));
    }

    #[test]
    fn pe_yod_retained_imperfect_pausal_qamats() {
        // יבש / ישן Qal Imperfect: the retained-yod stative surfaces with the
        // lengthened pausal qamats theme — yîḇāš יִיבָשׁ, yîšān יִישָׁן — beside
        // the contextual patah.
        let p = generate_paradigm(&Root::parse("יבש").unwrap());
        assert!(has_text(&p, Binyan::Qal, Form::Imperfect, THREE_MS, "יִיבָשׁ"));
        let p = generate_paradigm(&Root::parse("ישן").unwrap());
        assert!(has_text(&p, Binyan::Qal, Form::Imperfect, THREE_MS, "יִישָׁן"));
    }

    #[test]
    fn raah_apocopated_tsere_jussive() {
        // ראה Qal jussive/imperfect 3ms also surfaces as the tsere-segol yēreʔ
        // יֵרֶא ("let him see/appear"), beside the patah-prefix apocope yarʔ.
        let p = generate_paradigm(&Root::parse("ראה").unwrap());
        assert!(has_text(&p, Binyan::Qal, Form::Jussive, THREE_MS, "יֵרֶא"));
        assert!(has_text(&p, Binyan::Qal, Form::Imperfect, THREE_MS, "יֵרֶא"));
    }

    #[test]
    fn imperfect_object_suffix_2ms_guttural_c3() {
        // A III-guttural C3 takes a hataf-patah link before the -ḵā suffix, not a
        // plain sheva: ʾešlāḥăḵā אֶשְׁלָחֲךָ ("I will send you"), not אֶשְׁלָחְךָ.
        let p = generate_paradigm(&Root::parse("שלח").unwrap());
        assert!(any_text(&p, "אֶשְׁלָחֲךָ"));
    }

    #[test]
    fn lamed_aleph_imperative_object_suffix() {
        // III-aleph imperative + suffix: the quiescent aleph keeps the link vowel
        // and C2 takes qamats — rᵊp̄āʔēnî רְפָאֵנִי.
        let p = generate_paradigm(&Root::parse("רפא").unwrap());
        assert!(any_text(&p, "רְפָאֵנִי"));
    }

    #[test]
    fn guttural_c2_imperative_object_suffix() {
        // C2-guttural imperative + suffix: the guttural keeps the qamats and C1
        // reduces to a sheva — bᵊḥānēnî בְּחָנֵנִי. The strong שמר shape is
        // unaffected (šomrēnî שָׁמְרֵנִי).
        let p = generate_paradigm(&Root::parse("בחן").unwrap());
        assert!(any_text(&p, "בְּחָנֵנִי"));
        let p = generate_paradigm(&Root::parse("שמר").unwrap());
        assert!(any_text(&p, "שָׁמְרֵנִי"));
    }

    #[test]
    fn imperfect_object_suffix_2fs_tsere() {
        // 2fs object suffix on the imperfect is the tsere-link -ēḵ, not only the
        // reduced -ᵊḵ: yiḡʾālēḵ יִגְאָלֵךְ ("he will redeem you[f]"), tōʔḵᵊlēḵ
        // תֹּאכְלֵךְ.
        let p = generate_paradigm(&Root::parse("גאל").unwrap());
        assert!(any_text(&p, "יִגְאָלֵךְ"));
        let p = generate_paradigm(&Root::parse("אכל").unwrap());
        assert!(any_text(&p, "תֹּאכְלֵךְ"));
    }

    #[test]
    fn pe_guttural_loud_segol_imperfect_plural() {
        // Pe-guttural Qal imperfect 3mp loud (hataf-segol) twin: the segol-prefix
        // grade where the guttural opens the next syllable — yeʾĕrōḇû יֶאֱרֹבוּ
        // (o-theme), yeḥĕrāḇû יֶחֱרָבוּ (pausal a-theme).
        let p = generate_paradigm(&Root::parse("ארב").unwrap());
        assert!(any_text(&p, "יֶאֱרֹבוּ"));
        let p = generate_paradigm(&Root::parse("חרב").unwrap());
        assert!(any_text(&p, "יֶחֱרָבוּ"));
    }

    #[test]
    fn paragogic_nun_theme_guttural_c2() {
        // שאל Qal Imperfect 3mp: the bare plural carries a hataf on the guttural
        // C2 (yišʾălû יִשְׁאֲלוּ); the energic -ûn restores the qamats theme on
        // that guttural just as it does on a vocal-sheva consonant — yišʾālûn
        // יִשְׁאָלוּן.
        let p = generate_paradigm(&Root::parse("שאל").unwrap());
        assert!(has_text(&p, Binyan::Qal, Form::Imperfect, THREE_MP, "יִשְׁאָלוּן"));
    }
}
