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

    /// The connection the generation databases are attached to, so the
    /// curation stage can copy between schemas in SQL instead of ferrying
    /// every row through Rust.
    pub fn connection(bible: &crate::bible::Bible) -> &Connection {
        bible.conn()
    }

    /// One OSHB token tagging, as stored in `hebrew.db.oshb_primary`.
    pub struct TokenTagging<'a> {
        pub source_word: &'a str,
        pub lemma: &'a str,
        pub morph: &'a str,
    }

    /// Resolve the word-info sheet the reader would see for one surface, with
    /// the OSHB tagging of a concrete token applied when there is one.
    ///
    /// This is the resolution `gen-runtime` precomputes into `word_info` so the
    /// runtime never searches the candidate space (ADR 6). It is deliberately
    /// the *same* code the live path runs, so the generator cannot drift from
    /// it; the differential test compares the stored rows against this.
    ///
    /// The device-local `lexicon_entries` correction that the live path applies
    /// last is a no-op here — no writable progress database is attached at
    /// build time — which is exactly why that layer stays at runtime.
    pub fn resolve_word_info(
        bible: &crate::bible::Bible,
        surface_id: i64,
        norm: &str,
        tagging: Option<TokenTagging<'_>>,
    ) -> Option<crate::bible::HebrewWord> {
        crate::bible::resolve_word_info(bible, surface_id, norm, tagging)
    }
}
