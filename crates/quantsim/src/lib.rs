//! # Quantsim
//!
//! A quantum circuit simulator ecosystem providing both a core simulation library
//! and a web-based graphical user interface.
//!
//! ## Usage
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! quantsim = "0.2.2"
//! ```
//!
//! ## Re-exports
//!
//! This crate re-exports the main components of the quantsim ecosystem:
//!
//! - [`core`] - The core quantum circuit simulation library
//! - [`ui`] - The graphical user interface crate (re-exported when built for `wasm32`)

/// Core quantum circuit simulation library
pub mod core {
    pub use quantsim_core::*;
}

/// Web-based graphical user interface (available when targeting WASM)
#[cfg(target_arch = "wasm32")]
pub mod ui {
    pub use quantsim_ui::*;
}
