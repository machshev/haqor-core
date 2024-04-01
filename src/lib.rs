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
/// Library of bible resources
pub mod library;
/// Interact with external repositories of bible resources
pub mod repo;
