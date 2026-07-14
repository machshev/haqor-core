//! # Haqor Core
//!
//! A library for interacting with bible resources. Providing low level access
//! to the bible in it's original languages as well as translations. This
//! library is intended to provide the core functionality behind the `Haqor`
//! app, developed in [Flutter](https://flutter.dev/) to provide a cross
//! platform GUI. The interface between the Rust backend and the Flutter
//! frontend could be something like [Rinf](https://github.com/cunarist/rinf)
//! or just directly using FFI.
//!
//! Other options are available for the GUI, an interesting contender is
//! [slint](https://slint.dev). Ideally the GUI would be pure Rust framework.

/// utilities for interacting with bible resources
pub mod bible;

/// Biblical Hebrew verb/noun paradigm generator (algorithmic, not DB-backed).
pub use haqor_morphology as morphology;

/// Generate Haqor data tables from original source texts (Rust port of the
/// bible-modules pipeline).
pub mod generate;

/// Lossless SEDRA→Hebrew transliteration and Hebrew↔Syriac conversion.
pub mod transliterate;

/// Learner-facing romanization of pointed Hebrew ("how it sounds") — voices
/// the tutor's cards in the core so the app stays presentation-only.
pub mod romanize;

/// Spaced-repetition reading tutor: curriculum selection over the OT corpus
/// plus SM-2 review scheduling persisted in a writable `progress.db`.
pub mod tutor;

/// Curated learner glosses for high-frequency surfaces, keyed dagesh- and
/// combining-order-insensitively — the tutor's meaning override, held in the
/// core so the app stays presentation-only.
pub mod vocab_gloss;

/// Hand-maintained lexicon and learner-gloss overlays, merged by `gen-lexicon`.
pub mod lexicon_overlay;

/// Loopback-only web editor for the hand-maintained lexical overlay.
pub mod overlay_admin;

/// Grammar concepts the tutor teaches (prefixes, conjugations, binyanim,
/// construct, suffixes) with their teaching content, held in the core.
pub mod grammar;

/// Pronominal-ending inventory and stem/suffix splitting for the tutor's
/// suffix drill (the ending shown highlighted on a known host word).
pub mod pronoun_suffix;
