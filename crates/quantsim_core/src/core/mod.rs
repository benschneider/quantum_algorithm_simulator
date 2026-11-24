//! # Core
//!
//! The `core` module contains the fundamental building blocks for quantum
//! circuit simulation. This includes components for building circuits, defining
//! quantum gates, and running simulations.
//!
//! ## Modules
//!
//! - `builder`: Provides a `CircuitBuilder` for constructing quantum circuits.
//! - `circuit`: Defines the `Circuit` structure and related components for
//!   representing quantum circuits.
//! - `engine`: Contains the `Simulator` for running quantum circuit simulations.
//! - `gates`: Defines the standard quantum gates and their properties.
//! - `types`: Contains common data structures and type definitions used
//!   throughout the library.

/// Provides a `CircuitBuilder` for constructing quantum circuits.
pub mod builder;
/// Defines the `Circuit` structure and related components.
pub mod circuit;
/// Contains the `Simulator` for running quantum circuit simulations.
pub mod engine;
mod compact;
pub(crate) mod mps_bound;
/// Compact simulation of independent groups of entangled wires.
pub mod grouped;
/// Product-state simulation of separable circuit prefixes.
pub mod product_state;
/// Packed stabilizer simulation of Clifford circuit prefixes.
pub mod stabilizer;
/// Pretty-printing formatters for matrices and state vectors.
pub mod formatters;
/// Defines the standard quantum gates and their properties.
pub mod gates;
/// Contains gate matrix construction logic.
/// Contains common data structures and type definitions.
pub mod types;
pub mod endian_utils;
