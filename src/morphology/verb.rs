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
                // Construct-state twin for masculine-plural participles
                // (-îm → -ê). Same (binyan, form, pgn) label; only the
                // surface differs, which is all reverse-parsing needs.
                if matches!(form, Form::ParticipleActive | Form::ParticiplePassive)
                    && pgn.gender == Some(Gender::Masculine)
                    && pgn.number == Some(Number::Plural)
                {
                    let seq = build_strong(root, binyan, form, pgn);
                    let (seq, c_attested) = apply_gizra(seq, root, binyan, form, pgn);
                    if let Some(cseq) = participle_mp_construct(&seq) {
                        forms.push(VerbForm {
                            binyan,
                            form,
                            pgn,
                            text: hebrew::render(&cseq),
                            attested: c_attested,
                        });
                    }
                }
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
        Form::Jussive => JUSSIVE_PGNS,
        Form::InfinitiveConstruct | Form::InfinitiveAbsolute => {
            const NONE: &[Pgn] = &[Pgn::none()];
            NONE
        }
        Form::ParticipleActive | Form::ParticiplePassive => PARTICIPLE_PGNS,
    }
}

fn generate_one(root: &Root, binyan: Binyan, form: Form, pgn: Pgn) -> (String, bool) {
    if form == Form::Wayyiqtol {
        return build_wayyiqtol(root, binyan, pgn);
    }
    let seq = build_strong(root, binyan, form, pgn);
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
fn build_wayyiqtol(root: &Root, binyan: Binyan, pgn: Pgn) -> (String, bool) {
    let short = JUSSIVE_PGNS.contains(&pgn);
    let base_form = if short {
        Form::Jussive
    } else {
        Form::Imperfect
    };
    let seq = build_strong(root, binyan, base_form, pgn);
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
    // to segol under the consecutive vav (וַיָּשֶׂם, וַיָּבֶן).
    if short
        && root.has(Gizra::Hollow)
        && let Some(i) = radical_idx(&seq, 1)
        && seq[i].vowel == Some(Vowel::Tsere)
    {
        seq[i].vowel = Some(Vowel::Segol);
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

    let prefix_is_aleph = seq
        .first()
        .map(|c| c.letter == letter::ALEF)
        .unwrap_or(false);
    if let Some(first) = seq.first_mut() {
        first.dagesh = !prefix_is_aleph;
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
fn build_strong(root: &Root, binyan: Binyan, form: Form, pgn: Pgn) -> Vec<Cons> {
    match form {
        Form::Perfect => build_perfect(root, binyan, pgn),
        Form::Imperfect => build_imperfect(root, binyan, pgn, false),
        Form::Cohortative => build_cohortative(root, binyan, pgn),
        Form::Jussive => build_imperfect(root, binyan, pgn, true),
        Form::Wayyiqtol => unreachable!("wayyiqtol is built via build_wayyiqtol"),
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
    // Dagesh lene: when C1 closes the prefix syllable on a silent sheva
    // (yi-ḵ-, ya-q-), a begedkefet C2 begins the next syllable and takes a
    // dagesh lene — yiḵtōḇ → יִכְתֹּב, yaškēm → … . This fires for the binyanim
    // whose C1 carries a silent sheva after a real prefix vowel (Qal, Hiphil,
    // Hophal); Piel/Pual/Hithpael double C2 with a forte instead, and Niphal
    // closes C1 with its own forte.
    let c1_silent_sheva = prefix.v1_long == Vowel::Sheva && prefix_vowel != Vowel::Sheva;
    if c1_silent_sheva && !prefix.c1_dagesh && hebrew::is_begedkefet(root.ayin()) {
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
    {
        if let Some(i) = radical_idx(&seq, 1) {
            seq[i].vowel = Some(Vowel::Sheva);
            if i > 0 && seq[i - 1].vowel == Some(Vowel::Patah) {
                seq[i - 1].vowel = Some(Vowel::Hiriq);
            }
        }
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
            if let Some(c2) = radical_idx(&seq, 2) {
                if seq[c2].vowel == Some(Vowel::Holam) {
                    seq[c2].vowel = Some(Vowel::Tsere);
                }
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
                if let Some(c2) = radical_idx(&seq, 2) {
                    if seq[c2].vowel == Some(Vowel::Holam) {
                        seq[c2].vowel = Some(Vowel::Tsere);
                    }
                }
            }
            Form::Perfect => {
                if let Some(c3) = radical_idx(&seq, 3) {
                    if seq[c3].vowel == Some(Vowel::Sheva)
                        && c3 + 1 < seq.len()
                        && matches!(seq[c3 + 1].letter, letter::TAV | letter::NUN)
                    {
                        seq.remove(c3);
                        seq[c3].dagesh = true;
                        attested = true;
                    }
                }
            }
            _ => {}
        }
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
                // Long grade (2ms/2fp) had holam under C2 — lower to patah.
                if let Some(i) = radical_idx(&seq, 2)
                    && seq[i].vowel == Some(Vowel::Holam)
                {
                    seq[i].vowel = Some(Vowel::Patah);
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

/// לקח "to take" — its C1 lamed assimilates like a I-nun verb in the Qal.
fn is_laqah(root: &Root) -> bool {
    root.pe() == letter::LAMED && root.ayin() == letter::QOF && root.lamed() == letter::HET
}

/// ראה "to see" — doubly weak (II-aleph, III-he) with an irregular apocopated
/// jussive/wayyiqtol 3ms (וַיַּרְא).
fn is_raah(root: &Root) -> bool {
    root.pe() == letter::RESH && root.ayin() == letter::ALEF && root.lamed() == letter::HE
}

/// Rewrite an I-Yod / הלך Qal infinitive construct as the segholate form with
/// a tav afformative: drop C1, give both surviving radicals segol, append tav
/// (šeḇeṯ שֶׁבֶת, leḵeṯ לֶכֶת). The guttural rules run afterwards (e.g. ידע→דַּעַת).
fn apply_iyod_segholate_infinitive(seq: &mut Vec<Cons>) {
    if let Some(idx) = radical_idx(seq, 1) {
        seq.remove(idx);
    }
    if let Some(i) = radical_idx(seq, 2) {
        seq[i].vowel = Some(Vowel::Segol);
    }
    if let Some(i) = radical_idx(seq, 3) {
        seq[i].vowel = Some(Vowel::Segol);
    }
    seq.push(Cons::new(letter::TAV));
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
    let mut changed = false;
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
                changed = true;
            }
            // The theme vowel of these original-I-Vav verbs is tsere, not the
            // strong holam: yēšēḇ, yērēḏ, yēlēḏ. The vocalic-suffix grade keeps
            // its sheva (yēšḇû). III-guttural verbs (ידע) lower the theme to
            // patah instead (yēḏaʿ), so leave the holam for apply_lamed_guttural
            // to handle; III-aleph (יצא) ends up tsere either way.
            if !root.has(Gizra::LamedGuttural) {
                if let Some(c2) = radical_idx(seq, 2) {
                    if seq[c2].vowel == Some(Vowel::Holam) {
                        seq[c2].vowel = Some(Vowel::Tsere);
                    }
                }
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
    } else if root.pe() == letter::BET && root.lamed() == letter::ALEF {
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
                        seq[i].vowel = Some(Hiriq);
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
                }
                Suffix::Vocalic => {
                    if let Some(i) = c3_idx {
                        seq.remove(i);
                    }
                    if let Some(i) = c2_idx {
                        seq[i].vowel = None;
                    }
                }
                Suffix::Zero => {}
            }
            changed = true;
        }
        (Binyan::Qal, Form::Imperfect, person, gender, number) => {
            // III-He imperfect, by suffix grade:
            //   zero suffix (3ms/3fs/2ms/1cs/1cp): C2 segol, etymological he
            //     kept — yibnê (יִבְנֶה), tihyê (תִּהְיֶה).
            //   vocalic suffix (-û 3mp/2mp, -î 2fs): the he elides and C2 loses
            //     its vowel so the suffix vowel carries the syllable — yibnû
            //     (יִבְנוּ), tibnî (תִּבְנִי).
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
            if vocalic {
                if let Some(i) = c3_idx {
                    seq.remove(i);
                }
                if let Some(i) = c2_idx {
                    seq[i].vowel = None;
                }
                changed = true;
            } else if !heavy {
                if let Some(i) = c2_idx {
                    seq[i].vowel = Some(Segol);
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
        (
            Binyan::Qal,
            Form::Jussive,
            Some(Person::Third),
            Some(Gender::Masculine),
            Some(Number::Singular),
        ) => {
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
        (Binyan::Qal, Form::InfinitiveConstruct, _, _, _) => {
            // bənôt (בְּנוֹת), ʕăśôt (עֲשׂוֹת): the etymological he becomes a tav,
            // and the linking vowel is a plene holam written on a vav mater — so
            // C2 itself carries no vowel and a vav-holam is inserted before the
            // tav.
            if let Some(i) = c2_idx {
                seq[i].vowel = None;
            }
            if let Some(i) = c3_idx {
                seq[i] = Cons::new(letter::TAV);
                seq.insert(i, Cons::new(letter::VAV).with_vowel(Holam));
            }
            changed = true;
        }
        (Binyan::Qal, Form::ParticipleActive, _, gender, number) => {
            // III-He active participle qōṭeh (qōṭ-e + etymological he):
            //   ms ʿōśeh  (עֹשֶׂה) — C2 segol, he kept.
            //   fs ʿōśâ   (עֹשָׂה) — C2 qamats, he kept; the strong builder
            //              appended a segolate -t, so drop everything past the he.
            //   mp ʿōśîm  (עֹשִׂים) — he elides, C2 hiriq + yod mater.
            //   fp ʿōśôt  (עֹשׂוֹת) — he elides, plene holam-vav before the tav.
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
    // Qal Imperfect family: the quiescent alef lengthens the thematic holam to
    // qamats — yimṣāʔ (יִמְצָא), yiqrāʔ (יִקְרָא). Only the long grade carries the
    // holam; a vocalic suffix already reduces C2 to sheva (yimṣəʔû, יִמְצְאוּ),
    // where the alef simply quiesces, so leave it untouched.
    if matches!(
        (binyan, form),
        (
            Binyan::Qal,
            Form::Imperfect | Form::Jussive | Form::Cohortative | Form::Wayyiqtol
        )
    ) && let Some(i) = radical_idx(seq, 2)
        && seq[i].vowel == Some(Holam)
    {
        seq[i].vowel = Some(Qamats);
        changed = true;
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
    // Pe-Aleph in Qal Imperfect: yōʔkal (יֹאכַל). Prefix vowel becomes
    // holam, the alef quiesces and takes no vowel, C2 takes patah.
    if !matches!(form, Form::Imperfect | Form::Cohortative | Form::Jussive) {
        return false;
    }
    let _ = root;
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
    // III-Guttural (C3 = ח/ע). In the Qal Imperfect the thematic vowel under C2
    // lowers from holam to patah because a guttural prefers an a-class vowel
    // before it: yišlaḥ (יִשְׁלַח), yišmaʕ (יִשְׁמַע). Only the long grade carries
    // the holam; a vocalic suffix already reduces C2 to sheva (yišləḥû), so we
    // leave that alone.
    if !matches!(root.lamed(), letter::HET | letter::AYIN) {
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
    if let Some(i) = radical_idx(seq, 2)
        && seq[i].vowel == Some(Vowel::Holam)
    {
        seq[i].vowel = Some(Vowel::Patah);
        return true;
    }
    false
}

/// Insert a furtive patah under a word-final guttural (ח/ע) when the preceding
/// vowel is a long, non-a vowel. The guttural must close the word vowelless;
/// the glide patah is rendered directly on it (שֹׁמֵעַ, שְׁמֹעַ). The preceding
/// vowel may sit on a consonant (tsere/holam/hiriq) or on a vav/yod mater
/// carrying holam/shureq (inf. abs. שָׁמוֹעַ).
fn apply_furtive_patah(seq: &mut Vec<Cons>) -> bool {
    let n = seq.len();
    if n < 2 {
        return false;
    }
    let last = &seq[n - 1];
    if !matches!(last.letter, letter::HET | letter::AYIN) || last.vowel.is_some() {
        return false;
    }
    let prev = &seq[n - 2];
    let prev_long = matches!(
        prev.vowel,
        Some(Vowel::Tsere | Vowel::Holam | Vowel::Hiriq)
    ) || (prev.letter == letter::VAV && prev.vowel.is_none() && prev.dagesh);
    if !prev_long {
        return false;
    }
    seq[n - 1].vowel = Some(Vowel::Patah);
    true
}

fn apply_pe_guttural(seq: &mut [Cons], root: &Root, binyan: Binyan, form: Form, pgn: Pgn) -> bool {
    // I-Guttural (C1 = ח/ע). In the Qal Imperfect family the hiriq prefix vowel
    // lowers to patah and the guttural takes a hataf-patah (added by
    // [`apply_guttural`]): yaʕămōd (יַעֲמֹד), yaḥăzōq (יַחֲזֹק), and — combined with
    // III-He — yaʕăśê (יַעֲשֶׂה). Alef has its own pattern (pe_aleph) and the
    // 1cs alef-prefix form takes segol + hataf-segol, so both are excluded.
    if !matches!(root.pe(), letter::HET | letter::AYIN) {
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
    if pgn.person == Some(Person::First) && pgn.number == Some(Number::Singular) {
        // The aleph preformative already carries segol (אֶעֱמֹד, אֶעֱשֶׂה); the
        // C1 guttural takes a matching hataf-segol rather than a plain sheva.
        if let Some(i) = radical_idx(seq, 1)
            && seq[i].vowel == Some(Vowel::Sheva)
        {
            seq[i].vowel = Some(Vowel::HatafSegol);
            return true;
        }
        return false;
    }
    if let Some(first) = seq.first_mut()
        && first.vowel == Some(Vowel::Hiriq)
    {
        first.vowel = Some(Vowel::Patah);
        // The C1 guttural opens the second syllable (yaʕă-mōḏ): give it the
        // vocal hataf-patah explicitly, since apply_guttural now keeps a sheva
        // after a short vowel silent (which is correct for a closing guttural
        // like the lamed in יָדַעְתִּי, but wrong for this opening one).
        if let Some(i) = radical_idx(seq, 1)
            && seq[i].vowel == Some(Vowel::Sheva)
        {
            seq[i].vowel = Some(Vowel::HatafPatah);
        }
        return true;
    }
    false
}

/// Whether a vowel is inherently long (an open syllable before a sheva makes
/// that sheva vocal). Hataf and the short vowels keep a following sheva silent.
fn is_long_vowel(v: Vowel) -> bool {
    matches!(v, Vowel::Qamats | Vowel::Tsere | Vowel::Holam)
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
        if seq[i].dagesh && hebrew::rejects_dagesh(seq[i].letter) {
            seq[i].dagesh = false;
            changed = true;
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
}
