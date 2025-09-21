//! # quantsim_core
//!
//! `quantsim_core` is a quantum computing simulation library written in Rust. It provides
//! tools to build, simulate, and analyze quantum circuits.
//!
//! ## Features
//!
//! - **Quantum Circuit Construction**: Build quantum circuits using a variety of
//!   pre-defined quantum gates.
//! - **Statevector Simulation**: Simulate the behavior of quantum circuits and
//!   observe the evolution of the quantum state.
//! - **Measurement and Probabilities**: Perform measurements on qubits and
//!   calculate the probabilities of different outcomes.
//!
//! ## Example
//!
//! Here is a simple example of how to create and simulate a Bell state:
//!
//! ```rust
//! use quantsim_core::core::builder::CircuitBuilder;
//! // Create a circuit for 2 qubits.
//! let mut builder = CircuitBuilder::new(2);
//!
//! // Add a Hadamard gate to qubit 0.
//! builder.h(0);
//!
//! // Add a CNOT gate with control qubit 0 and target qubit 1.
//! builder.cx(0, 1);
//!
//! // Build the circuit.
//! let circuit = builder.build();
//!
//! // Create a simulator engine.
//! let mut engine = quantsim_core::core::engine::Simulator::new(circuit);
//!
//! // Run the simulation.
//! let result = engine.run();
//!
//! // Print the final quantum state.
//! println!("Final state vector: {:?}", result.final_state_vector);
//!
//! // Print the measurement probabilities.
//! for (state_idx, &probability) in result.final_probabilities.iter().enumerate() {
//!     println!("State |{}> has probability: {:.2}%", state_idx, probability * 100.0);
//! }
//! ```
pub mod core;
pub mod circuits;

// Re-export the core modules for easier access.
/// Re-export of the `CircuitBuilder` for easy access.
pub use crate::core::builder::CircuitBuilder;
/// Re-export of the `circuit` module.
pub use crate::core::circuit;
/// Re-export of the `engine` module.
pub use crate::core::engine;
/// Re-export of the `gates` module.
pub use crate::core::gates;
/// Re-export of the `types` module.
pub use crate::core::types;
