//! Biblical Hebrew morphology generator.
//!
//! Given a triliteral root, produces verb paradigms across the seven
//! binyanim (Qal, Niphal, Piel, Pual, Hithpael, Hiphil, Hophal) and across
//! every finite/non-finite form. Detects irregular root classes (gizra) and
//! applies the standard transformations so weak verbs come out with the
//! expected pointing.
//!
//! Coverage notes:
//! - All seven binyanim, all forms (perfect, imperfect, imperative,
//!   cohortative, jussive, infinitive construct, infinitive absolute,
//!   participle) for the strong (regular) root.
//! - Gizra-specific rules for I-Guttural, II-Guttural, III-Guttural,
//!   III-Aleph, III-He, I-Nun, I-Yod, I-Aleph, Hollow, Geminate at varying
//!   levels of completeness — see `verb.rs`. Forms that haven't been
//!   modelled fall back to the strong-verb pattern and are flagged as
//!   `attested = false` in the output so callers can distinguish generated
//!   forms from stubs.
//! - Noun inflection (state + pronominal suffixes) from a supplied stem
//!   in `noun.rs`.

pub mod gold_noun;
pub mod hebrew;
pub mod irregular_noun;
pub mod irregular_verb;
pub mod noun;
pub mod noun_parse;
pub mod parse;
pub mod root;
pub mod verb;

pub use irregular_noun::{IRREGULAR_NOUNS, IrregularNoun};
pub use irregular_verb::{IRREGULAR_VERBS, IrregularVerb};
pub use noun::{NounInflection, NounStem, NounStemKind, inflect_noun};
pub use noun_parse::{NounInventory, NounMatch, parse_noun_word};
pub use parse::{
    ReverseIndex, VerbMatch, parse_word, parse_word_disambiguated, parse_word_filtered,
    parse_word_indexed,
};
pub use root::{Gizra, Root, RootError};
pub use verb::{Binyan, Form, Paradigm, Pgn, VerbForm, generate_paradigm};

/// Person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Person {
    First,
    Second,
    Third,
}

/// Grammatical gender. `Common` is used where Hebrew does not distinguish
/// (1st person, 3cp Perfect, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gender {
    Masculine,
    Feminine,
    Common,
}

/// Grammatical number. Hebrew has dual for some nouns and a handful of
/// nominal/adjectival forms, but verb paradigms only use singular/plural.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Number {
    Singular,
    Plural,
    Dual,
}
