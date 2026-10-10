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
