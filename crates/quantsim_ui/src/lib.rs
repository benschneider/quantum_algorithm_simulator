//! # quantsim_ui
//!
//! `quantsim_ui` is a graphical user interface for `quantsim`, a quantum computing
//! simulator. It is built using the `egui` framework and provides a visual
//! way to create, edit, and simulate quantum circuits.
//!
//! ## Features
//!
//! *   **Circuit Grid:** A visual representation of a quantum circuit where you
//!     can drag and drop gates.
//! *   **Gate Palette:** A collection of standard quantum gates that can be
//!     added to the circuit.
//! *   **Initial State Editor:** A tool for defining the initial state of the
//!     qubits.
//! *   **JSON Editor:** A text-based editor for creating and editing circuits
//!     in JSON format.
//! *   **Simulation Results:** A panel that displays the results of the
//!     simulation, including the final state vector and measurement outcomes.
//!
#![warn(clippy::all, rust_2018_idioms)]

pub mod app;
pub mod components;
pub mod handlers;
pub mod messages;
pub mod prelude;
pub mod simulation;
pub mod state;
pub mod ui;
pub use app::QCSimApp;
