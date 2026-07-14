//! App-facing Bible access and Hebrew learning runtime.

/// Utilities for interacting with Bible resources.
pub mod bible;

/// Grammar concepts and learner-facing teaching content.
pub mod grammar;

/// Hand-maintained lexicon and learner-gloss overlays.
pub mod lexicon_overlay;

/// Pronominal-ending inventory and stem/suffix splitting.
pub mod pronoun_suffix;

/// Learner-facing romanization of pointed Hebrew.
pub mod romanize;

mod surface;
pub use surface::normalize_surface;

/// Lossless SEDRA-to-Hebrew and Hebrew-to-Syriac conversion.
pub mod transliterate;

/// Spaced-repetition reading tutor.
pub mod tutor;

/// Curated learner glosses for high-frequency surfaces.
pub mod vocab_gloss;

/// Narrow bridge used by the generated-data crate.
#[doc(hidden)]
pub mod data_support {
    use rusqlite::Connection;

    pub fn decode_pgn(pgn: &str) -> (Option<String>, Option<String>, Option<String>) {
        crate::bible::decode_pgn(pgn)
    }

    pub fn decode_noun_label(label: &str) -> (Option<String>, Option<String>) {
        crate::bible::decode_noun_label(label)
    }

    pub fn lexicon_fallback(db: &Connection, surface: &str) -> Option<(String, String, String)> {
        crate::bible::lexicon_fallback(db, surface)
    }
}
