//! # Circuit
//!
//! This module defines the `Circuit` structure, which represents a quantum
//! circuit, and the associated components for running simulations and handling
//! results.

use super::engine::QuantumState;
use super::gates::registry::GateRegistry;
use super::types::{Gate, GateMatrix, Operation, QubitPrep};
use crate::core::endian_utils;
use nalgebra::Complex;
use rand::distributions::{Distribution, WeightedIndex};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A data-transfer object (DTO) for serializing and deserializing a `Circuit`.
///
/// This struct is used to represent the circuit in a format that can be easily
/// converted to and from JSON. It is separate from the main `Circuit` struct
/// to provide a stable serialization format and to decouple the internal
/// representation of the circuit from its external representation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CircuitData {
    pub num_qubits: usize,
    pub steps: Vec<Vec<Operation>>,
    #[serde(default)]
    pub initial_preps: Vec<Option<QubitPrep>>,
    #[serde(default)]
    pub initial_state: Option<Vec<Complex<f32>>>,
}

impl From<Circuit> for CircuitData {
    fn from(circuit: Circuit) -> Self {
        Self {
            num_qubits: circuit.num_qubits,
            steps: circuit.steps,
            initial_preps: circuit.initial_preps,
            initial_state: circuit.initial_state,
        }
    }
}

/// Represents a quantum circuit.
#[derive(Clone, Debug)]
pub struct Circuit {
    /// The number of qubits in the circuit.
    pub num_qubits: usize,
    /// The operations in the circuit, organized into timesteps.
    pub steps: Vec<Vec<Operation>>,
    /// The initial preparation for each qubit.
    pub initial_preps: Vec<Option<QubitPrep>>,
    /// The gate registry for the circuit.
    pub registry: GateRegistry,
    /// The initial state vector of the circuit, if provided.
    pub initial_state: Option<Vec<nalgebra::Complex<f32>>>,
}

/// Options for running a circuit simulation.
#[derive(Default)]
pub struct RunOptions {
    /// The number of times to measure the circuit's output.
    pub shots: Option<u32>,
    /// Whether to take a snapshot of the state vector after each step.
    pub snapshot_state_per_step: bool,
    /// The seed for the random number generator used for measurements.
    pub rng_seed: Option<u64>,
}

/// A snapshot of the circuit's state at a specific step.
pub struct StepSnapshot {
    /// The index of the step.
    pub step_index: usize,
    /// The measurement probabilities at this step.
    pub probabilities: Vec<f32>,
    /// The state vector at this step, if requested.
    pub state: Option<Vec<nalgebra::Complex<f32>>>,
}

/// The result of a circuit simulation.
pub struct RunResult {
    /// The final measurement probabilities.
    pub final_probabilities: Vec<f32>,
    /// The final state vector, if requested.
    pub final_state_vector: Option<Vec<nalgebra::Complex<f32>>>,
    /// The snapshots of the circuit's state at each step, if requested.
    pub snapshots: Vec<StepSnapshot>,
    /// The measurement outcomes for each shot.
    pub measurements: Option<Vec<Vec<u8>>>,
    /// Any validation errors that occurred.
    pub diagnostics: Vec<ValidationError>,
}

/// An error that occurred during circuit validation.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// The step in which the error occurred.
    pub step: usize,
    /// The index of the operation in the step.
    pub op_index: usize,
    /// The error message.
    pub message: String,
}

impl Circuit {
    /// Creates a new, empty `Circuit` with the given number of qubits.
    pub fn new(num_qubits: usize) -> Self {
        Self {
            num_qubits,
            steps: Vec::new(),
            initial_preps: vec![None; num_qubits],
            registry: GateRegistry::default(),
            initial_state: None,
        }
    }

    /// Updates the circuit's state from a `CircuitData` object.
    ///
    /// This method consumes a `CircuitData` object and updates the fields of the
    /// `Circuit` instance. It is used to load a circuit from a serialized
    /// format, ensuring that the internal state of the circuit is consistent.
    ///
    /// # Arguments
    ///
    /// * `data` - The `CircuitData` object to update the circuit from.
    pub fn update_from_data(&mut self, data: CircuitData) {
        self.num_qubits = data.num_qubits;
        self.steps = data.steps;
        self.initial_state = data.initial_state;

        // Ensure initial_preps is the correct size, as the data from JSON
        // might have a different number of qubits.
        let mut new_preps = vec![None; self.num_qubits];
        for (i, prep) in data.initial_preps.into_iter().enumerate() {
            if i < self.num_qubits {
                new_preps[i] = prep;
            }
        }
        self.initial_preps = new_preps;
    }

    pub fn prepare_for_execution(&mut self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let registry = &self.registry;
        let num_qubits = self.num_qubits;

        for (i, step) in self.steps.iter_mut().enumerate() {
            let mut used_qubits = HashSet::new();
            for (j, op) in step.iter_mut().enumerate() {
                prepare_operation(i, j, op, &mut used_qubits, &mut errors, num_qubits, registry);
            }
        }
        errors
    }

    /// Runs the circuit simulation.
    ///
    /// This is the main entry point for executing a circuit. It follows a
    /// "run-time preparation" model to ensure correctness and to keep the original
    /// `Circuit` definition immutable during the run.
    ///
    /// ## Architectural Overview:
    ///
    /// 1.  **Build Run Steps**: A temporary, runnable sequence of operations is
    ///     created by `build_run_steps`. This makes the `run` method side-effect-free
    ///     with respect to the circuit's definition.
    ///
    /// 2.  **Prepare and Validate**: The sequence is passed to
    ///     `prepare_and_validate_steps`, which acts as a "compiler". It validates
    ///     each operation and converts all gate matrices into the `LittleEndian`
    ///     sparse format required by the engine.
    ///
    /// 3.  **Initialize State**: The `QuantumState` is created, either from an
    ///     initial vector or the default |0...0> state.
    ///
    /// 4.  **Execute**: The prepared steps are applied to the state.
    ///
    /// 5.  **Measure**: The final state is measured if shots are requested.
    pub fn run(&self, opts: &RunOptions) -> RunResult {
        // 1. Build the full sequence of operations for this run.
        let mut run_steps = self.build_run_steps();

        // 2. Prepare and validate the steps.
        let diagnostics = self.prepare_and_validate_steps(&mut run_steps);
        if !diagnostics.is_empty() {
            return RunResult {
                diagnostics,
                ..Default::default()
            };
        }

        // 3. Initialize the quantum state.
        let mut state = match self.initialize_state() {
            Ok(s) => s,
            Err(e) => return e,
        };

        // 4. Execute the simulation.
        let (final_state, snapshots) = self.execute_steps(&mut state, &run_steps, opts);

        // 5. Perform measurements.
        let measurements = self.perform_measurements(&final_state, opts);

        RunResult {
            final_probabilities: final_state.probabilities(),
            final_state_vector: Some(final_state.state_vector.as_slice().to_vec()),
            snapshots,
            measurements,
            diagnostics: vec![],
        }
    }

    /// Builds the complete list of operational steps for a simulation run.
    ///
    /// This function creates a temporary, side-effect-free list of operations.
    /// It first converts any `initial_preps` into concrete gate operations,
    /// then appends a clone of the main circuit steps. This ensures that the
    /// original `Circuit` definition is not mutated during a run.
    fn build_run_steps(&self) -> Vec<Vec<Operation>> {
        let mut run_steps: Vec<Vec<Operation>> = Vec::new();
        if self.initial_state.is_none() {
            let mut prep_ops_step1 = Vec::new();
            let mut prep_ops_step2 = Vec::new();
            for (q, prep) in self.initial_preps.iter().enumerate() {
                if let Some(p) = prep {
                    match p {
                        QubitPrep::One => prep_ops_step1.push(Operation::new(Gate::X, vec![q as u32], vec![])),
                        QubitPrep::Plus => prep_ops_step1.push(Operation::new(Gate::H, vec![q as u32], vec![])),
                        QubitPrep::Minus => {
                            prep_ops_step1.push(Operation::new(Gate::X, vec![q as u32], vec![]));
                            prep_ops_step2.push(Operation::new(Gate::H, vec![q as u32], vec![]));
                        }
                        QubitPrep::Zero => {}
                    }
                }
            }
            if !prep_ops_step1.is_empty() {
                run_steps.push(prep_ops_step1);
            }
            if !prep_ops_step2.is_empty() {
                run_steps.push(prep_ops_step2);
            }
        }
        run_steps.append(&mut self.steps.clone());
        run_steps
    }

    /// Iterates through simulation steps, preparing and validating each operation.
    ///
    /// This is the "compilation" phase of the simulation. It calls `prepare_operation`
    /// for every operation to ensure it is valid and its matrix is in the
    /// engine-ready `LittleEndian` sparse format.
    fn prepare_and_validate_steps(&self, steps: &mut [Vec<Operation>]) -> Vec<ValidationError> {
        let mut diagnostics = Vec::new();
        for (i, step) in steps.iter_mut().enumerate() {
            let mut used_qubits = HashSet::new();
            for (j, op) in step.iter_mut().enumerate() {
                prepare_operation(i, j, op, &mut used_qubits, &mut diagnostics, self.num_qubits, &self.registry);
            }
        }
        diagnostics
    }

    /// Initializes the `QuantumState` for the simulation.
    ///
    /// If an `initial_state` vector is provided, it's used. Otherwise, a default
    /// |0...0> state is created, which will then be acted upon by any initial
    /// preparation steps.
    fn initialize_state(&self) -> Result<QuantumState, RunResult> {
        match self.initial_state.as_ref() {
            Some(state_vector) => QuantumState::from_vector(self.num_qubits, state_vector).map_err(|msg| RunResult {
                diagnostics: vec![ValidationError { step: 0, op_index: 0, message: msg.to_string() }],
                ..Default::default()
            }),
            None => Ok(QuantumState::new(self.num_qubits)),
        }
    }

    /// Executes the main simulation loop over the prepared steps.
    ///
    /// This function iterates through the `run_steps`, applies each operation to the
    /// `QuantumState`, and collects snapshots if requested.
    fn execute_steps<'a>(&self, state: &'a mut QuantumState, run_steps: &[Vec<Operation>], opts: &RunOptions) -> (&'a QuantumState, Vec<StepSnapshot>) {
        let mut snapshots = Vec::new();
        for (i, step) in run_steps.iter().enumerate() {
            for op in step.iter() {
                log::debug!("=== STEP {}: Applying {} on qubits {:?} ===", i, op.id, op.qubits);
                log::debug!("State IN:\n{}", state.get_pretty_print_state_vector());
                state.apply_operation(op, &self.registry);
                log::debug!("State OUT:\n{}", state.get_pretty_print_state_vector());
            }
            if opts.snapshot_state_per_step {
                snapshots.push(StepSnapshot {
                    step_index: i,
                    probabilities: state.probabilities(),
                    state: Some(state.state_vector.as_slice().to_vec()),
                });
            }
        }
        (state, snapshots)
    }

    /// Performs measurements on the final quantum state if `shots` are requested.
    fn perform_measurements(&self, final_state: &QuantumState, opts: &RunOptions) -> Option<Vec<Vec<u8>>> {
        if let Some(shots) = opts.shots {
            let dist = WeightedIndex::new(final_state.probabilities()).unwrap();
            let results = if let Some(seed) = opts.rng_seed {
                let mut rng = StdRng::seed_from_u64(seed);
                (0..shots)
                    .map(|_| {
                        let outcome = dist.sample(&mut rng);
                        (0..self.num_qubits).map(|i| ((outcome >> i) & 1) as u8).collect()
                    })
                    .collect()
            } else {
                (0..shots)
                    .into_par_iter()
                    .map_init(StdRng::from_entropy, |rng, _| {
                        let outcome = dist.sample(rng);
                        (0..self.num_qubits).map(|i| ((outcome >> i) & 1) as u8).collect()
                    })
                    .collect()
            };
            Some(results)
        } else {
            None
        }
    }
}

impl Default for RunResult {
    fn default() -> Self {
        Self {
            final_probabilities: vec![],
            final_state_vector: None,
            snapshots: vec![],
            measurements: None,
            diagnostics: vec![],
        }
    }
}


/// Helper function to prepare a single operation for execution.
///
/// This function is called by `prepare_for_execution` for each operation in the
/// circuit. It performs the following tasks:
///
/// 1.  **Qubit Validation**: Checks if qubit indices are within the circuit's
///     bounds and ensures that a qubit is not used more than once in a single
///     timestep.
/// 2.  **Endian Conversion**: If the operation is a custom gate with a dense,
///     big-endian matrix (`GateMatrix::Dense`), it converts the matrix to a
///     sparse, little-endian format (`GateMatrix::Sparse`) as required by the
///     simulation engine.
/// 3.  **Arity Validation**: Verifies that the number of qubits the operation
///     acts on matches the required arity of the gate (either from the gate's
///     matrix or from the `GateRegistry`).
///
/// Any errors found during this process are added to the `errors` vector.
fn prepare_operation(
    step_idx: usize,
    op_idx: usize,
    op: &mut Operation,
    used_qubits: &mut HashSet<usize>,
    errors: &mut Vec<ValidationError>,
    num_qubits: usize,
    registry: &GateRegistry,
) {
    op.qubits.iter().for_each(|&q| {
        let q_usize = q as usize;
        if q_usize >= num_qubits {
            let err = ValidationError {
                step: step_idx,
                op_index: op_idx,
                message: format!(
                    "Qubit index {} is out of bounds for a {}-qubit circuit.",
                    q_usize, num_qubits
                ),
            };
            log::debug!("Validation Error: {}", err.message);
            errors.push(err);
        }
        if !used_qubits.insert(q_usize) {
            let err = ValidationError {
                step: step_idx,
                op_index: op_idx,
                message: format!("Qubit {q_usize} is used more than once in the same timestep."),
            };
            log::debug!("Validation Error: {}", err.message);
            errors.push(err);
        }
    });

    // Ensure every operation has a matrix. If it's a standard gate without
    // one, evaluate it from the registry. This guarantees that programmatic
    // gates like CCNOT are always resolved.
    if op.matrix.is_none() {
        match registry.eval(&op.id, &op.params, op.qubits.as_slice()) {
            Some(matrix) => op.matrix = Some(matrix),
            None => {
                let err = ValidationError {
                    step: step_idx,
                    op_index: op_idx,
                    message: format!("Failed to evaluate matrix for gate '{}'", op.id),
                };
                log::debug!("Validation Error: {}", err.message);
                errors.push(err);
                return; // Can't proceed with this operation
            }
        }
    }

    // Now, every operation that has a matrix should be in BigEndian format.
    // We convert it to the LittleEndian format required by the engine.
    if let Some(GateMatrix::BigEndian(dense_matrix)) = op.matrix.take() {
        match endian_utils::to_sparse_little_endian(&dense_matrix) {
            Ok(sparse_matrix) => {
                op.matrix = Some(GateMatrix::LittleEndian(sparse_matrix));
            }
            Err(e) => {
                let err = ValidationError {
                    step: step_idx,
                    op_index: op_idx,
                    message: format!("Failed to convert gate matrix to little-endian for gate '{}': {}", op.id, e),
                };
                log::debug!("Validation Error: {}", err.message);
                errors.push(err);
                return;
            }
        }
    }

    // After the above steps, the matrix MUST be LittleEndian.
    let required_qubits = match op.matrix.as_ref() {
        Some(GateMatrix::LittleEndian(m)) => (m.nrows() as f32).log2() as usize,
        Some(GateMatrix::BigEndian(_)) => {
            let err = ValidationError {
                step: step_idx,
                op_index: op_idx,
                message: "Logic error: BigEndian matrix found after conversion phase.".to_string(),
            };
            log::debug!("Validation Error: {}", err.message);
            errors.push(err);
            return;
        }
        None => {
            let err = ValidationError {
                step: step_idx,
                op_index: op_idx,
                message: format!("Matrix for gate '{}' could not be determined.", op.id),
            };
            log::debug!("Validation Error: {}", err.message);
            errors.push(err);
            return;
        }
    };

    if op.qubits.len() != required_qubits {
        let err = ValidationError {
            step: step_idx,
            op_index: op_idx,
            message: format!(
                "Gate '{}' requires {} qubits, but {} were provided.",
                op.id,
                required_qubits,
                op.qubits.len()
            ),
        };
        log::debug!("Validation Error: {}", err.message);
        errors.push(err);
    }
}
