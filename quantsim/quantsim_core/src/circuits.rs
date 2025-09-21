//! This module provides access to the built-in circuit templates.
//!
//! The circuit templates are embedded directly into the `quantsim_core` library
//! at compile time using the `include_dir` crate. This approach ensures that
//! the templates are always available to any consumer of this library (like
//! `quantsim_ui` or integration tests) without needing to worry about relative
//! file paths or a separate distribution of the circuit files.
//!
//! The main components are:
//! - `CIRCUITS_DIR`: A static `Dir` instance that holds the contents of the
//!   `quantsim/quantsim_core/circuits` directory.
//! - `get_circuit_names()`: A function to retrieve the filenames of all
//!   available templates.
//! - `get_circuit()`: A function to retrieve the JSON content of a specific
//!   template by its filename.

use include_dir::{include_dir, Dir};

static CIRCUITS_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/circuits");

/// Retrieves the names of all available circuit templates.
///
/// # Returns
///
/// A vector of strings, where each string is the filename of a circuit template.
pub fn get_circuit_names() -> Vec<String> {
    CIRCUITS_DIR
        .files()
        .filter_map(|f| f.path().file_name())
        .filter_map(|s| s.to_str())
        .map(|s| s.to_string())
        .collect()
}

/// Retrieves the JSON content of a specific circuit template by its filename.
///
/// # Arguments
///
/// * `name` - The filename of the circuit template to retrieve (e.g., "bell.json").
///
/// # Returns
///
/// An `Option` containing the JSON string of the circuit if found, otherwise `None`.
pub fn get_circuit(name: &str) -> Option<String> {
    CIRCUITS_DIR
        .get_file(name)
        .and_then(|f| f.contents_utf8())
        .map(|s| s.to_string())
}