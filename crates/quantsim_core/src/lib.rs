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
//! - **JSON Serialization**: Save and load circuits in JSON format.
//! - **Parallel Execution**: Efficient simulation using Rayon for parallel processing.
//! - **Extensible Gate System**: Add custom gates with matrix definitions.
//!
//! ## Quick Start
//!
//! Apply a Hadamard gate to q0, then use q0 to control a flip on q1.
//! The resulting Bell pair gives equal probabilities for `|00⟩` and `|11⟩`.
//!
//! ```rust
//! use quantsim_core::{circuit::RunOptions, CircuitBuilder};
//!
//! let mut builder = CircuitBuilder::new(2);
//! builder.h(0).next_step().cx(0, 1);
//! let result = builder.build().run(&RunOptions::default());
//! assert!(result.diagnostics.is_empty());
//! assert!((result.final_probabilities[0b00] - 0.5).abs() < 1e-6);
//! assert!((result.final_probabilities[0b11] - 0.5).abs() < 1e-6);
//! ```
//!
//! ## Circuit Construction
//!
//! Circuits are built by adding operations to the `steps` vector. Each step contains
//! operations that can be executed in parallel (on disjoint sets of qubits).
//!
//! ```rust
//! use quantsim_core::{circuit::Circuit, types::{Gate, Operation}};
//!
//! let mut circuit = Circuit::new(3);
//!
//! // Add gates to different qubits in the same timestep
//! circuit.steps.push(vec![
//!     Operation::new(Gate::H, vec![0], vec![]),  // Hadamard on qubit 0
//!     Operation::new(Gate::X, vec![1], vec![]),  // Pauli-X on qubit 1
//!     Operation::new(Gate::Y, vec![2], vec![]),  // Pauli-Y on qubit 2
//! ]);
//!
//! // Add a 2-qubit gate in the next timestep
//! circuit.steps.push(vec![
//!     Operation::new(Gate::CX, vec![0, 1], vec![]), // CNOT: control=0, target=1
//! ]);
//! assert_eq!(circuit.steps.len(), 2);
//! assert_eq!(circuit.steps[0].len(), 3);
//! ```
//!
//! ## JSON Serialization
//!
//! Circuit JSON can include an optional qubit layout. The simulator uses its
//! sites and links for the 2D layout and repeats those positions at each
//! timestep in the 3D stack. Gate operands still determine which wires interact.
//!
//! ```rust
//! use quantsim_core::circuit::{Circuit, CircuitData, QubitLayout};
//!
//! let mut circuit = Circuit::new(2);
//! circuit.layout = Some(QubitLayout::line(2));
//! let data = CircuitData::from(circuit);
//!
//! // Serialize to JSON
//! let json = serde_json::to_string(&data).unwrap();
//!
//! // Deserialize from JSON
//! let loaded_data: CircuitData = serde_json::from_str(&json).unwrap();
//! let mut loaded_circuit = Circuit::new(loaded_data.num_qubits);
//! loaded_circuit.update_from_data(loaded_data).unwrap();
//! assert_eq!(loaded_circuit.layout.unwrap().links, [[0, 1]]);
//! ```
//!
//! ## Advanced Features
//!
//! ### Custom Gates
//!
//! Supply a unitary matrix to the builder. Here a custom Pauli-Z gate between
//! two Hadamards flips q0 from `|0⟩` to `|1⟩`.
//!
//! ```rust
//! use nalgebra::{Complex, DMatrix};
//! use quantsim_core::{circuit::RunOptions, CircuitBuilder};
//!
//! let one = Complex::new(1.0, 0.0);
//! let zero = Complex::new(0.0, 0.0);
//! let z_matrix = DMatrix::from_row_slice(2, 2, &[one, zero, zero, -one]);
//!
//! let mut builder = CircuitBuilder::new(1);
//! builder.h(0).next_step();
//! builder.add_custom_gate("My Z", z_matrix, &[0]).next_step();
//! builder.h(0);
//! let result = builder.build().run(&RunOptions::default());
//! assert!((result.final_probabilities[1] - 1.0).abs() < 1e-6);
//! ```
//!
//! ### Measurement Sampling
//!
//! Sample a Bell pair. Each shot records one bit per wire in q0, q1 order.
//!
//! ```rust
//! use quantsim_core::{circuit::RunOptions, circuits::load_template};
//!
//! let circuit = load_template("bell").unwrap();
//! let options = RunOptions {
//!     shots: Some(1000),
//!     rng_seed: Some(42),
//!     ..Default::default()
//! };
//!
//! let result = circuit.run(&options);
//! let shots = result.measurements.unwrap();
//! assert_eq!(shots.len(), 1000);
//! assert!(shots.iter().all(|shot| matches!(shot.as_slice(), [0, 0] | [1, 1])));
//! ```
//!
//! ### Loading Built-in Templates
//!
//! ```rust
//! use quantsim_core::circuits;
//!
//! // Load a built-in circuit template
//! let bell_circuit = circuits::load_template("bell").unwrap();
//!
//! // Get list of all available templates
//! let template_names = circuits::get_circuit_names();
//! println!("Available templates: {:?}", template_names);
//! ```
//!
//! ### State Snapshots
//!
//! Capture the state after every executed step. In the five-step plaquette
//! template, q3 is already flipped when the last CNOT copies its bit to q4.
//! Snapshot index 4 holds the readout shown in the simulator at step 5.
//!
//! ```rust
//! use quantsim_core::{circuit::RunOptions, circuits::load_template};
//!
//! let circuit = load_template("plaquette_z_check").unwrap();
//! let options = RunOptions {
//!     snapshot_state_per_step: true,
//!     ..Default::default()
//! };
//!
//! let result = circuit.run(&options);
//! assert_eq!(result.snapshots.len(), 5);
//! // Basis indices are written in q4 q3 q2 q1 q0 order.
//! assert_eq!(result.snapshots[3].probabilities[0b01000], 1.0);
//! assert_eq!(result.snapshots[4].probabilities[0b11000], 1.0);
//! ```
pub mod core;
pub mod circuits;

// Re-export the core modules for easier access.
/// Re-export of the `CircuitBuilder` for easy access.
pub use crate::core::builder::CircuitBuilder;
/// Arbitrary-width computational-basis index or joint outcome, with q0 at bit zero.
pub use num_bigint::BigUint as BasisIndex;
/// Re-export of the `circuit` module.
pub use crate::core::circuit;
/// Re-export of the `engine` module.
pub use crate::core::engine;
pub use crate::core::grouped;
pub use crate::core::product_state;
pub use crate::core::stabilizer;
/// Re-export of the `gates` module.
pub use crate::core::gates;
/// Re-export of the `types` module.
pub use crate::core::types;
