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
                let (text, attested) = generate_one(root, binyan, form, pgn);
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
                // Pausal twin of the Piel/Hithpael vocalic-suffix imperfect:
                // the reduced theme sheva restores to tsere in pause
                // (yᵉḏabbᵊrû → yᵉḏabbērû, יְדַבֵּרוּ).
                let piel_pausal = (matches!(binyan, Binyan::Piel | Binyan::Hithpael)
                    && matches!(form, Form::Imperfect | Form::Wayyiqtol)
                    && imperfect_suffix_kind(pgn) == Suffix::Vocalic)
                .then(|| piel_pausal_variant(&text))
                .flatten();
                // Pausal twin of the Qal vocalic-suffix imperfect: the reduced
                // theme sheva restores to the 3ms theme vowel in pause
                // (yēlᵊḵû → yēlēḵû יֵלֵכוּ, yippᵊlû → yippōlû יִפֹּלוּ).
                let qal_pausal = (binyan == Binyan::Qal
                    && form == Form::Imperfect
                    && imperfect_suffix_kind(pgn) == Suffix::Vocalic)
                .then(|| {
                    let zero = Pgn::new(Person::Third, Gender::Masculine, Number::Singular);
                    let (zero_text, _) = generate_one(root, binyan, form, zero);
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
                // Pronominal object-suffixed forms for the 3ms host (computed
                // from &text before it is moved into the base VerbForm below).
                let object_suffixed: Vec<(Pgn, String)> = if pgn
                    == Pgn::new(Person::Third, Gender::Masculine, Number::Singular)
                {
                    match (binyan, form) {
                        (Binyan::Qal, Form::Perfect) => qal_perfect_object_suffixes(root),
                        (_, Form::Imperfect | Form::Jussive | Form::Wayyiqtol) => {
                            imperfect_object_suffixes(&text, root)
                        }
                        _ => Vec::new(),
                    }
                } else if form == Form::InfinitiveConstruct {
                    inf_construct_object_suffixes(root, binyan, &text)
                } else {
                    Vec::new()
                };
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
                    paragogic,
                    hiphil_apoc,
                    hiphil_plene,
                    piel_pausal,
                    qal_pausal,
                    guttural_imperative_pausal,
                    guttural_perfect_patah,
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
                // Object-suffixed forms (deduped by surface+suffix).
                let mut osuf_seen = std::collections::HashSet::new();
                for (obj, t) in object_suffixed {
                    if osuf_seen.insert((obj, t.clone())) {
                        forms.push(VerbForm {
                            binyan,
                            form,
                            pgn,
                            text: t,
                            attested,
                            object_suffix: Some(obj),
                        });
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
                    let (base, attested_fs) = generate_one(root, binyan, form, fs);
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
                    let (base, _) = generate_one(root, binyan, form, pgn);
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
                            object_suffix: None,
                        });
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
                    let seq = build_strong(root, binyan, form, pgn);
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

    // Hollow shureq-class plural wayyiqtol: the long û (mater vav) shortens to
    // qubuts on C1 under stress retraction (wayyāšuḇû → וַיָּשֻׁבוּ, not the long
    // וַיָּשׁוּבוּ). Only the vocalic-suffix forms have the mater to shorten; the
    // singular short stem is already handled via the jussive base above.
    if !short
        && root.has(Gizra::Hollow)
        && hollow_class(root) == HollowClass::Shureq
        && imperfect_suffix_kind(pgn) == Suffix::Vocalic
        && let Some(c1) = radical_idx(&seq, 1)
        && seq.get(c1 + 1).is_some_and(|c| c.letter == letter::VAV && c.role == Role::Mater)
    {
        seq[c1].vowel = Some(Vowel::Qubuts);
        seq.remove(c1 + 1);
    }

    let prefix_is_aleph = seq
        .first()
        .map(|c| c.letter == letter::ALEF)
        .unwrap_or(false);
    if let Some(first) = seq.first_mut() {
        // The consecutive vav doubles the preformative consonant — except when
        // that consonant carries a vocal sheva, which cannot bear a forte. This
        // is the Piel/Pual prefix yᵉ- (wayᵉḇāreḵ → וַיְבָרֶךְ, not וַיְּבָרֶךְ);
        // Hithpael's yiṯ- keeps its hiriq and so still doubles. חיה's wayyiqtol
        // likewise keeps a bare sheva-prefix (וַיְחִי).
        let sheva_prefix = first.vowel == Some(Vowel::Sheva);
        first.dagesh = !prefix_is_aleph && !is_chayah(root) && !sheva_prefix;
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

/// Qal roots whose imperfect theme vowel is /a/ (patah) rather than the default
/// /o/, without a guttural to condition it phonologically — a lexical property.
/// שכב → yiškab. (Guttural-conditioned a-theme is handled by [`apply_guttural`].)
const QAL_A_THEME: [[char; 3]; 1] = [[letter::SHIN, letter::KAF, letter::BET]];

fn build_imperfect(root: &Root, binyan: Binyan, pgn: Pgn, _jussive: bool) -> Vec<Cons> {
    use Vowel::*;
    let mut prefix = imperfect_prefix(binyan);
    // Qal a-theme: a quiescent middle aleph (yišʔal) or a lexically a-class root
    // (yiškab) takes patah where the default paradigm would place holam.
    if binyan == Binyan::Qal
        && prefix.v2_long == Holam
        && (root.ayin() == letter::ALEF
            || QAL_A_THEME.contains(&[root.pe(), root.ayin(), root.lamed()]))
    {
        prefix.v2_long = Patah;
    }
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
    {
        if let Some(i) = radical_idx(&seq, 1) {
            seq[i].vowel = Some(Vowel::Sheva);
            if i > 0 && seq[i - 1].vowel == Some(Vowel::Patah) {
                seq[i - 1].vowel = Some(Vowel::Hiriq);
            }
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

    // חיה "to live" — its apocopated jussive 3ms (the base for the very common
    // wayyiqtol וַיְחִי) is irregular: the he drops, the C2 yod becomes a
    // hiriq-mater, C1 ḥet takes hiriq, and the prefix reduces to a vocal sheva —
    // yəḥî (יְחִי). apply_lamed_he has already apocopated to yiḥy (יִחְי); rewrite
    // the prefix and C1 vowels.
    if is_chayah(root)
        && binyan == Binyan::Qal
        && form == Form::Jussive
        && pgn == Pgn::new(Person::Third, Gender::Masculine, Number::Singular)
    {
        if let Some(c1) = radical_idx(&seq, 1) {
            if c1 > 0 {
                seq[c1 - 1].vowel = Some(Vowel::Sheva);
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
    {
        if let Some(c1) = radical_idx(&seq, 1)
            && c1 > 0
            && seq[c1 - 1].vowel == Some(Vowel::Patah)
        {
            seq[c1 - 1].vowel = Some(Vowel::Hiriq);
            attested = true;
        }
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
    if is_mut(root) && binyan == Binyan::Qal && form == Form::Perfect {
        if let Some(c1) = radical_idx(&seq, 1)
            && seq[c1].vowel == Some(Vowel::Qamats)
        {
            seq[c1].vowel = Some(Vowel::Tsere);
            attested = true;
        }
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

/// לקח "to take" — its C1 lamed assimilates like a I-nun verb in the Qal.
fn is_laqah(root: &Root) -> bool {
    root.pe() == letter::LAMED && root.ayin() == letter::QOF && root.lamed() == letter::HET
}

/// ראה "to see" — doubly weak (II-aleph, III-he) with an irregular apocopated
/// jussive/wayyiqtol 3ms (וַיַּרְא).
fn is_raah(root: &Root) -> bool {
    root.pe() == letter::RESH && root.ayin() == letter::ALEF && root.lamed() == letter::HE
}

/// חיה "to live" — III-he with an irregular apocopated jussive/wayyiqtol 3ms
/// (יְחִי / וַיְחִי) whose consecutive vav adds no forte to the sheva-prefix.
fn is_chayah(root: &Root) -> bool {
    root.pe() == letter::HET && root.ayin() == letter::YOD && root.lamed() == letter::HE
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
            if perfect_suffix_kind(pgn) == Suffix::Consonantal && root.lamed() != letter::ALEF {
                if let Some(c1_idx) = radical_idx(seq, 1) {
                    if seq[c1_idx].vowel == Some(Vowel::Qamats) {
                        seq[c1_idx].vowel = Some(Vowel::Patah);
                    }
                }
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
                if let Some(i) = c3_idx {
                    seq.remove(i);
                }
                if let Some(i) = c2_idx {
                    seq[i].vowel = None;
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
    // Hiphil consonantal-suffix perfect of a III-aleph root: the theme tsere is
    // already in place (hēḇē-, hôṣē-); the quiescent alef just drops its sheva
    // and the begedkefet suffix loses its dagesh lene — hēḇēʔtî (הֵבֵאתִי),
    // hôṣēʔtî (הוֹצֵאתִי).
    if let (Binyan::Hiphil, Form::Perfect) = (binyan, form)
        && perfect_suffix_kind(pgn) == Suffix::Consonantal
    {
        if let Some(c3) = radical_idx(seq, 3) {
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
            Form::Imperfect | Form::Jussive | Form::Cohortative | Form::Wayyiqtol | Form::Imperative
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
        let is_1cs = _pgn.person == Some(Person::First)
            && _pgn.number == Some(Number::Singular);
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
    if !matches!(root.pe(), letter::HET | letter::AYIN | letter::HE) {
        return false;
    }
    // Hiphil/Niphal perfect with a guttural C1: the hi-/ni- prefix hiriq lowers
    // to segol and the guttural takes a hataf-segol opening the next syllable —
    // heḥĕzîq (הֶחֱזִיק), heʕĕmîd (הֶעֱמִיד), neḥĕzaq (נֶחֱזַק). (The Hiphil
    // imperfect instead patterns with the Qal: yaḥăzîq.)
    if matches!((binyan, form), (Binyan::Hiphil | Binyan::Niphal, Form::Perfect)) {
        let mut changed = false;
        if let Some(first) = seq.first_mut()
            && first.vowel == Some(Vowel::Hiriq)
        {
            first.vowel = Some(Vowel::Segol);
            changed = true;
        }
        if let Some(i) = radical_idx(seq, 1)
            && seq[i].vowel == Some(Vowel::Sheva)
        {
            seq[i].vowel = Some(Vowel::HatafSegol);
            changed = true;
        }
        return changed;
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
    if matches!(c3, letter::HE | letter::ALEF | letter::VAV | letter::YOD)
        || matches!(c1, letter::VAV | letter::YOD | letter::NUN)
    {
        return Vec::new();
    }
    // Build qᵊṭāC3- with an optional C3 link vowel (None leaves C3 vowelless,
    // for the holam-vav 3ms ô written on the mater), append the tail, then let
    // apply_guttural fix a guttural C1/C2 (sheva → hataf).
    let build = |c2v: Vowel, link: Option<Vowel>, tail: &[Cons]| -> String {
        let mut c3c = Cons::radical(c3, 3);
        c3c.vowel = link;
        let mut seq = vec![Cons::radical(c1, 1).with_vowel(Sheva), Cons::radical(c2, 2).with_vowel(c2v), c3c];
        seq.extend_from_slice(tail);
        apply_guttural(&mut seq, root);
        hebrew::render(&seq)
    };
    let mut out = Vec::new();
    // Light suffixes attach to the qᵊṭāl- (C2 qamats) grade.
    out.push((OBJ_1CS, build(Qamats, Some(Patah), &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)])));
    out.push((OBJ_2MS, build(Qamats, Some(Sheva), &[ocv(letter::KAF, Qamats)])));
    out.push((OBJ_2FS, build(Qamats, Some(Tsere), &[ocv(letter::KAF, Sheva)])));
    // 3ms ô — holam carried on the vav mater (qᵊṭālô קְטָלוֹ).
    out.push((OBJ_3MS, build(Qamats, None, &[Cons::new(letter::VAV).with_vowel(Holam)])));
    out.push((OBJ_3FS, build(Qamats, Some(Qamats), &[Cons::new(letter::HE).with_dagesh()])));
    out.push((OBJ_1CP, build(Qamats, Some(Qamats), &[Cons::new(letter::NUN), oshureq()])));
    out.push((OBJ_3MP, build(Qamats, Some(Qamats), &[Cons::new(letter::MEM)])));
    out.push((OBJ_3FP, build(Qamats, Some(Qamats), &[Cons::new(letter::NUN)])));
    // Heavy 2mp/2fp attach to the qᵊṭal- (C2 patah) grade.
    out.push((OBJ_2MP, build(Patah, Some(Sheva), &[ocv(letter::KAF, Segol), Cons::new(letter::MEM)])));
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
    // The bare 3ms ends in a true final consonant (C3 of yiqṭōl, the nun of
    // wayyittēn). It is either vowelless or carries the Masoretic silent sheva
    // that `render` writes under a final kaf (יְבָרֵךְ). A real vowel or a mater
    // final (HE/ALEF, or a vowelless VAV/YOD) means a weak/derived shape we
    // don't model here.
    if matches!(last.vowel, Some(v) if v != Vowel::Sheva)
        || matches!(last.letter, letter::HE | letter::ALEF)
        || matches!(last.letter, letter::VAV | letter::YOD)
    {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut emit = |obj: Pgn, link: Vowel, tail: &[Cons]| {
        for theme in [Sheva, Segol] {
            let mut s = seq.clone();
            s[n - 2].vowel = Some(theme);
            s[n - 1].vowel = Some(link);
            s.extend_from_slice(tail);
            out.push((obj, hebrew::render(&s)));
        }
    };
    // 1cs: plain -ēnî and energic -ennî.
    emit(OBJ_1CS, Tsere, &[ocv(letter::NUN, Hiriq), Cons::new(letter::YOD)]);
    emit(OBJ_1CS, Segol, &[Cons::new(letter::NUN).with_dagesh().with_vowel(Hiriq), Cons::new(letter::YOD)]);
    // 2ms -ᵊḵā, 2fs -ēḵ.
    emit(OBJ_2MS, Sheva, &[ocv(letter::KAF, Qamats)]);
    emit(OBJ_2FS, Sheva, &[ocv(letter::KAF, Sheva)]);
    // 3ms: plain -ēhû and energic -ennû.
    emit(OBJ_3MS, Tsere, &[Cons::new(letter::HE), oshureq()]);
    emit(OBJ_3MS, Segol, &[Cons::new(letter::NUN).with_dagesh(), oshureq()]);
    // 3fs -ehā.
    emit(OBJ_3FS, Segol, &[ocv(letter::HE, Qamats)]);
    // 1cp -ēnû.
    emit(OBJ_1CP, Tsere, &[Cons::new(letter::NUN), oshureq()]);
    // 3mp -ēm.
    emit(OBJ_3MP, Tsere, &[Cons::new(letter::MEM)]);
    // 2mp -ᵊḵem.
    emit(OBJ_2MP, Sheva, &[ocv(letter::KAF, Segol), Cons::new(letter::MEM)]);
    out
}

/// The nominal/possessive suffix set, as (object PGN, linking vowel on the
/// final stem consonant, tail consonants). An infinitive construct takes these
/// — "his reigning" mālḵô (מָלְכוֹ), "to possess it" — exactly as a noun does,
/// not the verbal -anî/-ēhû set.
fn nominal_suffix_tails() -> Vec<(Pgn, Option<Vowel>, Vec<Cons>)> {
    use Vowel::*;
    vec![
        (OBJ_1CS, Some(Hiriq), vec![Cons::new(letter::YOD)]),                          // -î
        (OBJ_2MS, Some(Sheva), vec![ocv(letter::KAF, Qamats)]),                        // -ᵊḵā
        (OBJ_2FS, Some(Tsere), vec![ocv(letter::KAF, Sheva)]),                         // -ēḵ
        (OBJ_3MS, None, vec![Cons::new(letter::VAV).with_vowel(Holam)]),               // -ô (holam on vav)
        (OBJ_3FS, Some(Qamats), vec![Cons::new(letter::HE).with_dagesh()]),            // -āh
        (OBJ_1CP, Some(Tsere), vec![Cons::new(letter::NUN), oshureq()]),               // -ēnû
        (OBJ_2MP, Some(Sheva), vec![ocv(letter::KAF, Segol), Cons::new(letter::MEM)]), // -ᵊḵem
        (OBJ_3MP, Some(Qamats), vec![Cons::new(letter::MEM)]),                         // -ām
        (OBJ_3FP, Some(Qamats), vec![Cons::new(letter::NUN)]),                         // -ān
    ]
}

/// Infinitive construct with a pronominal suffix ("in his reigning" בְּמָלְכוֹ,
/// "his begetting" הוֹלִידוֹ). The Qal infinitive shifts to the qoṭl- grade
/// before a suffix (mᵊlōḵ → mālḵ-) — built here from the radicals (C1 qamets-
/// hatuf, C2 silent sheva), trying both the regular and qatan qamats glyphs.
/// Other binyanim keep their bare infinitive shape and simply take the suffix
/// on C3 (haqṭîl → haqṭîl-ô). Strong/true-final-consonant shapes only.
fn inf_construct_object_suffixes(root: &Root, binyan: Binyan, base_text: &str) -> Vec<(Pgn, String)> {
    use Vowel::*;
    let mut out = Vec::new();
    if binyan == Binyan::Qal {
        let (c1, c2, c3) = (root.pe(), root.ayin(), root.lamed());
        if matches!(c3, letter::HE | letter::ALEF | letter::VAV | letter::YOD)
            || matches!(c1, letter::VAV | letter::YOD | letter::NUN)
        {
            return out;
        }
        for (obj, link, tail) in nominal_suffix_tails() {
            for c1v in [Qamats, QamatsQatan] {
                let mut c3c = Cons::radical(c3, 3);
                c3c.vowel = link;
                let mut seq = vec![Cons::radical(c1, 1).with_vowel(c1v), Cons::radical(c2, 2).with_vowel(Sheva), c3c];
                seq.extend(tail.clone());
                apply_guttural(&mut seq, root);
                out.push((obj, hebrew::render(&seq)));
            }
        }
    } else {
        let seq = hebrew::parse_pointed(base_text);
        let n = seq.len();
        if n < 2 {
            return out;
        }
        let last = seq[n - 1];
        if matches!(last.vowel, Some(v) if v != Vowel::Sheva)
            || matches!(last.letter, letter::HE | letter::ALEF)
            || matches!(last.letter, letter::VAV | letter::YOD)
        {
            return out;
        }
        for (obj, link, tail) in nominal_suffix_tails() {
            let mut s = seq.clone();
            s[n - 1].vowel = link;
            s.extend(tail);
            out.push((obj, hebrew::render(&s)));
        }
    }
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
    if last.vowel.is_some()
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
fn hiphil_apocope_variant(text: &str) -> Option<String> {
    let mut seq = hebrew::parse_pointed(text);
    // The theme î is a vowelless yod mater preceded by a hiriq-bearing
    // consonant and followed by a closing C3 (…rî-b). The prefix yod carries
    // its own vowel, and a word-final plene yod has nothing after it, so both
    // are excluded.
    let i = seq.iter().position(|c| {
        c.letter == letter::YOD && c.vowel.is_none() && !c.dagesh
    })?;
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
                Some(
                    Vowel::Holam
                        | Vowel::Tsere
                        | Vowel::Patah
                        | Vowel::Qamats
                        | Vowel::Segol
                )
            )
        {
            theme = Some((i, zseq[i].vowel.unwrap()));
        }
    }
    let (i, v) = theme?;
    vseq[i].vowel = Some(v);
    Some(hebrew::render(&vseq))
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
}
