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

pub use haqor_runtime::{
    bible, grammar, lexicon_overlay, pronoun_suffix, romanize, transliterate, tutor, vocab_gloss,
};

/// Biblical Hebrew verb/noun paradigm generator (algorithmic, not DB-backed).
pub use haqor_morphology as morphology;

/// Generate Haqor data tables from original source texts (Rust port of the
/// bible-modules pipeline).
pub mod generate;

/// Loopback-only web editor for the hand-maintained lexical overlay.
pub mod overlay_admin;
