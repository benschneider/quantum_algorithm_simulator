//! # Gates
//!
//! This module defines the quantum gates used in the `qcsim` library. It
//! includes a `GateRegistry` for managing and accessing gate definitions,
//! as well as implementations for various standard and parametric gates.

pub mod matrix_builders;
pub mod parametric;
pub mod registry;
#[cfg(test)]
mod tests;
pub use crate::core::endian_utils::*;
pub use crate::core::types::Gate;
