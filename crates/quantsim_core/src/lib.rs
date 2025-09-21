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
//! Here is a simple example of how to create and simulate a Bell state:
//!
//! ```rust
//! use quantsim_core::{Circuit, RunOptions};
//!
//! // Create a circuit for 2 qubits
//! let mut circuit = Circuit::new(2);
//!
//! // Add a Hadamard gate to qubit 0
//! circuit.steps.push(vec![
//!     quantsim_core::core::types::Operation::new(
//!         quantsim_core::core::types::Gate::H,
//!         vec![0],
//!         vec![]
//!     )
//! ]);
//!
//! // Add a CNOT gate with control qubit 0 and target qubit 1
//! circuit.steps.push(vec![
//!     quantsim_core::core::types::Operation::new(
//!         quantsim_core::core::types::Gate::CX,
//!         vec![0, 1],
//!         vec![]
//!     )
//! ]);
//!
//! // Run the simulation
//! let options = RunOptions::default();
//! let result = circuit.run(&options);
//!
//! // Print the measurement probabilities
//! for (i, &prob) in result.final_probabilities.iter().enumerate() {
//!     println!("State |{:b}> has probability: {:.3}", i, prob);
//! }
//! ```
//!
//! ## Circuit Construction
//!
//! Circuits are built by adding operations to the `steps` vector. Each step contains
//! operations that can be executed in parallel (on disjoint sets of qubits).
//!
//! ```rust
//! use quantsim_core::{Circuit, core::types::{Operation, Gate}};
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
//! ```
//!
//! ## JSON Serialization
//!
//! Circuits can be serialized to/from JSON for persistence and sharing:
//!
//! ```rust
//! use quantsim_core::{Circuit, circuit::CircuitData};
//!
//! let circuit = Circuit::new(2);
//! let data: CircuitData = circuit.clone().into();
//!
//! // Serialize to JSON
//! let json = serde_json::to_string(&data).unwrap();
//!
//! // Deserialize from JSON
//! let loaded_data: CircuitData = serde_json::from_str(&json).unwrap();
//! let mut loaded_circuit = Circuit::new(loaded_data.num_qubits);
//! loaded_circuit.update_from_data(loaded_data);
//! ```
//!
//! ## Advanced Features
//!
//! ### Custom Gates
//!
//! Define your own quantum gates using unitary matrices:
//!
//! ```rust
//! use quantsim_core::core::types::{Operation, GateMatrix};
//! use nalgebra::Complex;
//!
//! // Define a custom 2x2 unitary matrix (Pauli-Z gate)
//! let z_matrix = vec![
//!     vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
//!     vec![Complex::new(0.0, 0.0), Complex::new(-1.0, 0.0)],
//! ];
//!
//! let mut circuit = Circuit::new(1);
//! circuit.steps.push(vec![
//!     Operation {
//!         id: "Z".to_string(),
//!         qubits: vec![0],
//!         params: vec![],
//!         matrix: Some(GateMatrix::BigEndian(z_matrix)),
//!     }
//! ]);
//! ```
//!
//! ### Measurement Sampling
//!
//! Perform multiple measurement shots to get statistical results:
//!
//! ```rust
//! use quantsim_core::RunOptions;
//!
//! let options = RunOptions {
//!     shots: Some(1000),  // Perform 1000 measurements
//!     ..Default::default()
//! };
//!
//! let result = circuit.run(&options);
//! if let Some(measurements) = result.measurements {
//!     println!("Got {} measurement outcomes", measurements.len());
//!     // Each measurement is a vector of bits (one per qubit)
//! }
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
//! Capture the quantum state at each timestep:
//!
//! ```rust
//! let options = RunOptions {
//!     snapshot_state_per_step: true,
//!     ..Default::default()
//! };
//!
//! let result = circuit.run(&options);
//! for snapshot in &result.snapshots {
//!     println!("After step {}: probabilities = {:?}", snapshot.step_index, snapshot.probabilities);
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
