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
                let (text, attested) = generate_one(root, binyan, form, pgn);
                forms.push(VerbForm {
                    binyan,
                    form,
                    pgn,
                    text,
                    attested,
                });
            }
        }
    }
    Paradigm {
        root: root.clone(),
        forms,
    }
}

const FORMS_FOR_PARADIGM: &[Form] = &[
    Form::Perfect,
    Form::Imperfect,
    Form::Imperative,
    Form::Cohortative,
    Form::Jussive,
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
        Form::Imperfect => IMPERFECT_PGNS,
        Form::Imperative => IMPERATIVE_PGNS,
        Form::Cohortative => COHORTATIVE_PGNS,
        Form::Jussive => JUSSIVE_PGNS,
        Form::InfinitiveConstruct | Form::InfinitiveAbsolute => {
            const NONE: &[Pgn] = &[Pgn::none()];
            NONE
        }
        Form::ParticipleActive | Form::ParticiplePassive => PARTICIPLE_PGNS,
    }
}

fn generate_one(root: &Root, binyan: Binyan, form: Form, pgn: Pgn) -> (String, bool) {
    let seq = build_strong(root, binyan, form, pgn);
    let (seq, attested) = apply_gizra(seq, root, binyan, form, pgn);
    (hebrew::render(&seq), attested)
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
fn build_strong(root: &Root, binyan: Binyan, form: Form, pgn: Pgn) -> Vec<Cons> {
    match form {
        Form::Perfect => build_perfect(root, binyan, pgn),
        Form::Imperfect => build_imperfect(root, binyan, pgn, false),
        Form::Cohortative => build_cohortative(root, binyan, pgn),
        Form::Jussive => build_imperfect(root, binyan, pgn, true),
        Form::Imperative => build_imperative(root, binyan, pgn),
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

fn build_imperfect(root: &Root, binyan: Binyan, pgn: Pgn, _jussive: bool) -> Vec<Cons> {
    use Vowel::*;
    let prefix = imperfect_prefix(binyan);
    let suffix = imperfect_suffix_kind(pgn);

    let mut prefix_vowel = prefix.vowel;
    // 1cs in Qal: ʔeqṭōl — prefix vowel is segol, not hiriq (alef can't take
    // hiriq + closed syllable in Qal). Apply for binyanim that use hiriq.
    if pgn.person == Some(Person::First)
        && pgn.number == Some(Number::Singular)
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
    out.push(c2);

    // ---- Optional mater after C2 (Hiphil) ----
    if matches!(suffix, Suffix::Zero)
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

fn build_imperative(root: &Root, binyan: Binyan, pgn: Pgn) -> Vec<Cons> {
    // Imperative = Imperfect 2nd-person with the prefix stripped, and v1
    // restored to a stable open-syllable vowel. Strong Qal: qəṭōl, qiṭlî,
    // qiṭlû, qəṭōlnâ.
    use Vowel::*;
    let prefix = imperfect_prefix(binyan);

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
        Binyan::Niphal => Sheva,   // hiqqāṭēl — C1 sheva (silent under dagesh)
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
    out.push(c2);

    if matches!(suffix, Suffix::Zero)
        && let Some(m) = prefix.mater_after_c2_long
    {
        // Hiphil imperative ms is haqṭēl with TSERE, no mater yod.
        // Skip mater for Hiphil here.
        if binyan != Binyan::Hiphil {
            out.push(Cons::new(m));
        }
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
            rad(root.ayin(), 2).with_vowel(Tsere),
            rad(root.lamed(), 3),
        ],
        Binyan::Hophal => vec![
            Cons::new(letter::HE).with_vowel(QamatsQatan),
            rad(root.pe(), 1).with_vowel(Sheva),
            rad(root.ayin(), 2).with_vowel(Tsere),
            rad(root.lamed(), 3),
        ],
    }
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
    if root.has(Gizra::PeNun) {
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

    // I-Aleph: in Qal Imperfect, prefix vowel becomes holam (yōʔkal).
    if root.has(Gizra::PeAleph) {
        attested |= apply_pe_aleph(&mut seq, root, binyan, form, pgn);
    }

    // Guttural rules: forbid dagesh on guttural radicals, convert vocal
    // sheva under a guttural to a hataf vowel. Applied last so they catch
    // fixes introduced by other gizra rules. Always runs (not gated on the
    // root having a guttural radical) so prefix alefs in 1cs Imperfect get
    // their hataf-patah even for strong roots: אֲקַטֵּל, not אְקַטֵּל.
    let _ = apply_guttural(&mut seq, root);

    (seq, attested)
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
    // C1 is the radical-1 slot — uniquely identified by its Role tag, so we
    // don't confuse it with the 1cp prefix nun or the 3fp/2fp -nâ suffix nun.
    let _ = root;
    let c1_idx = radical_idx(seq, 1);
    match (binyan, form) {
        (Binyan::Qal, Form::Imperfect)
        | (Binyan::Qal, Form::Cohortative)
        | (Binyan::Qal, Form::Jussive)
        | (Binyan::Niphal, Form::Perfect)
        | (Binyan::Hiphil, _)
        | (Binyan::Hophal, _) => {
            if let Some(idx) = c1_idx
                && idx + 1 < seq.len()
            {
                seq.remove(idx);
                seq[idx].dagesh = true;
                changed = true;
            }
        }
        (Binyan::Qal, Form::Imperative) | (Binyan::Qal, Form::InfinitiveConstruct) => {
            // Drop the initial nun-sheva. Strong form was nun-sheva, C2, C3.
            if let Some(idx) = c1_idx {
                seq.remove(idx);
                changed = true;
            }
        }
        _ => {}
    }
    changed
}

fn apply_pe_yod(seq: &mut Vec<Cons>, root: &Root, binyan: Binyan, form: Form, _pgn: Pgn) -> bool {
    // I-Yod is the most complex weak class. In Qal Imperfect of originally
    // I-Vav verbs (yāšab, yāḏaʿ, etc.), the yod drops and the prefix vowel
    // becomes tsere: yēšēb. In Niphal & Hiphil the original vav reappears.
    //
    // First-pass implementation: in Qal Imperfect, drop the yod and turn
    // the prefix vowel into tsere; in Hiphil, replace the C1 yod with a
    // mater vav (hôšîaʿ-style).
    let _ = root;
    let mut changed = false;
    match (binyan, form) {
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
                changed = true;
            }
        }
        (Binyan::Hiphil, _) => {
            // Hiphil of original I-Vav: the vav reappears as a mater after
            // the he-prefix (hôšîb < *haw-šīb). Replace the C1 yod with a
            // silent vav-mater and shift the prefix vowel to holam.
            if let Some(idx) = radical_idx(seq, 1) {
                if idx > 0 {
                    seq[idx - 1].vowel = Some(Vowel::Holam);
                }
                seq[idx] = Cons::mater(letter::VAV);
                changed = true;
            }
        }
        _ => {}
    }
    changed
}

fn apply_hollow(seq: &mut Vec<Cons>, root: &Root, binyan: Binyan, form: Form, _pgn: Pgn) -> bool {
    // Hollow verbs lose their middle radical in almost every form. The
    // characteristic vowel takes its place: Qal Perfect qām (kāmū), Qal
    // Imperfect yāqûm, Hiphil hēqîm.
    //
    // First-pass: handle Qal Perfect 3ms (replace C1-C2-C3 with C1-qamats,
    // C3) and Qal Imperfect 3ms (replace yi-C1(sheva)-C2(holam)-C3 with
    // yā-C1, vav(shureq), C3).
    let mut changed = false;
    let _ = root;
    match (binyan, form) {
        (Binyan::Qal, Form::Perfect) => {
            // qām: drop the middle radical, keep C1-qamats and C3.
            if let Some(idx) = radical_idx(seq, 2) {
                seq.remove(idx);
                changed = true;
            }
        }
        (Binyan::Qal, Form::Imperfect)
        | (Binyan::Qal, Form::Cohortative)
        | (Binyan::Qal, Form::Jussive)
        | (Binyan::Qal, Form::Imperative)
        | (Binyan::Qal, Form::InfinitiveConstruct) => {
            // yāqûm: prefix vowel → qamats, C1 loses vowel, C2 becomes
            // a vav-shureq (mater for long u).
            if let Some(c1_idx) = radical_idx(seq, 1) {
                if c1_idx > 0 {
                    seq[c1_idx - 1].vowel = Some(Vowel::Qamats);
                }
                seq[c1_idx].vowel = None;
            }
            if let Some(c2_idx) = radical_idx(seq, 2) {
                seq[c2_idx] = Cons::mater(letter::VAV);
                seq[c2_idx].dagesh = true;
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
        _ => {}
    }
    changed
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
            // yāsōḇ: drop C3, add dagesh to (the remaining) C2, prefix vowel
            // → qamats, C1 loses its vowel.
            if let Some(c3_idx) = radical_idx(seq, 3) {
                seq.remove(c3_idx);
                changed = true;
            }
            if let Some(c1_idx) = radical_idx(seq, 1) {
                if c1_idx > 0 {
                    seq[c1_idx - 1].vowel = Some(Vowel::Qamats);
                }
                seq[c1_idx].vowel = None;
            }
            if let Some(c2_idx) = radical_idx(seq, 2) {
                seq[c2_idx].dagesh = true;
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

    // C2 is the radical-2 slot regardless of where the (possibly-replaced)
    // C3 he ends up. Looking it up by role beats counting from the end —
    // we may have appended a tav-suffix that shifts positions.
    let c2_idx = radical_idx(seq, 2);
    let c3_idx = radical_idx(seq, 3);
    match (binyan, form, pgn.person, pgn.gender, pgn.number) {
        (
            Binyan::Qal,
            Form::Perfect,
            Some(Person::Third),
            Some(Gender::Masculine),
            Some(Number::Singular),
        ) => {
            // bānâ: change C2 vowel from patah to qamats.
            if let Some(i) = c2_idx {
                seq[i].vowel = Some(Qamats);
            }
            changed = true;
        }
        (
            Binyan::Qal,
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
        (
            Binyan::Qal,
            Form::Imperfect,
            Some(Person::Third),
            Some(Gender::Masculine),
            Some(Number::Singular),
        ) => {
            // yibnê: change C2 vowel from holam to segol.
            if let Some(i) = c2_idx {
                seq[i].vowel = Some(Segol);
            }
            changed = true;
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
        (
            Binyan::Qal,
            Form::Jussive,
            Some(Person::Third),
            Some(Gender::Masculine),
            Some(Number::Singular),
        ) => {
            // Apocopated: yiḇen — drop the final he.
            if let Some(i) = c3_idx {
                seq.remove(i);
                if let Some(i2) = c2_idx {
                    // Indices shift if c3 was after c2 — c2 stays put.
                    let _ = i2;
                    if let Some(j) = radical_idx(seq, 2) {
                        seq[j].vowel = Some(Segol);
                    }
                }
            }
            changed = true;
        }
        (Binyan::Qal, Form::InfinitiveConstruct, _, _, _) => {
            // bənôt: replace C3 he with tav, C2 takes holam.
            if let Some(i) = c3_idx {
                seq[i] = Cons::new(letter::TAV);
            }
            if let Some(i) = c2_idx {
                seq[i].vowel = Some(Holam);
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
    changed
}

fn apply_pe_aleph(seq: &mut [Cons], root: &Root, binyan: Binyan, form: Form, _pgn: Pgn) -> bool {
    // Pe-Aleph in Qal Imperfect: yōʔkal (יֹאכַל). Prefix vowel becomes
    // holam, the alef quiesces and takes no vowel, C2 takes patah.
    if binyan != Binyan::Qal || !matches!(form, Form::Imperfect | Form::Cohortative | Form::Jussive)
    {
        return false;
    }
    use Vowel::*;
    let _ = root;
    let mut changed = false;
    // C1 alef quiesces. Prefix vowel → holam. C2 takes patah.
    if let Some(c1_idx) = radical_idx(seq, 1) {
        if c1_idx > 0 {
            seq[c1_idx - 1].vowel = Some(Holam);
            changed = true;
        }
        seq[c1_idx].vowel = None;
    }
    if let Some(c2_idx) = radical_idx(seq, 2) {
        seq[c2_idx].vowel = Some(Patah);
    }
    changed
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
    for c in seq.iter_mut() {
        if c.dagesh && hebrew::rejects_dagesh(c.letter) {
            c.dagesh = false;
            changed = true;
        }
        if let Some(Vowel::Sheva) = c.vowel
            && hebrew::is_guttural(c.letter)
        {
            // Heuristic: turn into hataf-patah, except after a long
            // vowel which still allows a silent sheva — but Biblical
            // Hebrew normally substitutes hataf there too in
            // word-initial / consonant-cluster contexts.
            c.vowel = Some(Vowel::HatafPatah);
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
}
