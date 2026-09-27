//! App-facing Bible access and Hebrew learning core.

/// Version of this crate, so an app can report which core it is running
/// without hard-coding a number that drifts from `Cargo.toml`. Shown in the
/// app's About view alongside the data build from [`bible::Bible::data_version`].
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Biblical Hebrew verb/noun paradigm generator (algorithmic, not DB-backed).
pub use haqor_morphology as morphology;

/// Utilities for interacting with Bible resources.
pub mod bible;

/// Grammar concepts and learner-facing teaching content.
pub mod grammar;

/// Hand-maintained lexicon and learner-gloss overlays.
pub mod lexicon_overlay;

/// Pronominal-ending inventory and stem/suffix splitting.
pub mod pronoun_suffix;

/// Safe snapshot and merge support for synchronising learner progress.
pub mod progress_sync;

/// Learner-facing romanization of pointed Hebrew.
pub mod romanize;

/// Build-time resolution of a surface to word info, searching the generation
/// databases. `gen-runtime` precomputes its output into `haqor.db`.
pub mod resolve;

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

    /// A word's consonants, finals folded and a sin kept apart from a shin —
    /// the key roots and lexemes are filed under, the key the pointing-blind
    /// rung of the lexicon bridge matches on, and what `surface.cons` stores
    /// so the runtime can match a name against its entry without a SQL
    /// function.
    pub fn fold_consonants(word: &str) -> String {
        crate::bible::fold_consonants(word)
    }

    /// [`fold_consonants`] with sin and shin both a bare ש, as Syriac spells
    /// them: the key shared with SEDRA.
    pub fn bare_letters(word: &str) -> String {
        crate::bible::bare_letters(word)
    }

    /// How many consonants a [`fold_consonants`] key spells, a sin counting
    /// once.
    pub fn key_len(key: &str) -> usize {
        crate::bible::key_letters(key).count()
    }

    /// The root key for a root spelled with the bare letters `bare` (as the
    /// morphology generator knows it), read off a pointed `surface` of it: a
    /// sin wherever the surface's ש are all sins. `None` when the surface
    /// does not decide — no ש, or both kinds.
    pub fn root_key_from_surface(bare: &str, surface: &str) -> Option<String> {
        crate::bible::root_key_from_surface(bare, surface)
    }

    /// The connection the generation databases are attached to, so the
    /// curation stage can copy between schemas in SQL instead of ferrying
    /// every row through Rust.
    pub fn connection(bible: &crate::bible::Bible) -> &Connection {
        bible.conn()
    }
}
