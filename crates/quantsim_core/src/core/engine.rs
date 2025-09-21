//! # State Vector and Qubit Ordering Conventions
//!
//! This engine uses a state vector to represent the quantum state. The ordering
//! of the basis states in this vector follows a little-endian convention.
//!
//! ## Little-Endian State Vector Indexing
//!
//! For an `n`-qubit system, the state is a vector of 2<sup>n</sup> complex amplitudes.
//! The index of each amplitude corresponds to the integer value of a bitstring
//! `|q_{n-1}q_{n-2}...q_1q_0>`, where `q_i` is the state (`0` or `1`) of the `i`-th qubit.
//!
//! *   **Qubit 0 is the Least Significant Bit (LSB).**
//! *   **Qubit `n-1` is the Most Significant Bit (MSB).**
//!
//! ### Example: 2-Qubit System
//!
//! The state vector `[c₀, c₁, c₂, c₃]` maps to basis states as follows:
//! *   `c₀`: Amplitude for `|00⟩` (q₁=0, q₀=0)
//! *   `c₁`: Amplitude for `|01⟩` (q₁=0, q₀=1)
//! *   `c₂`: Amplitude for `|10⟩` (q₁=1, q₀=0)
//! *   `c₃`: Amplitude for `|11⟩` (q₁=1, q₀=1)
//!
//! # Engine
//!
//! This module provides the `QuantumState` struct, which represents the state
//! of a quantum system and the `Simulator` which is responsible for applying
//! operations to the state.
use super::circuit::Circuit;
use super::formatters;
use super::types::{GateMatrix, Operation};
use nalgebra::{Complex, DVector};
use nalgebra_sparse::CsrMatrix;
use rayon::prelude::*;

/// Represents the state of a quantum system.
#[derive(Debug, Clone)]
pub struct QuantumState {
    /// The number of qubits in the system.
    pub num_qubits: usize,
    /// The state vector of the system.
    pub state_vector: DVector<Complex<f32>>,
}

/// An iterator that produces the sequence of `other_q_state` values.
///
/// Each value represents the integer value of the basis states for the qubits
/// that a gate does *not* act upon.
pub struct OtherQubitStateIterator {
    other_qubits: Vec<usize>,
    current_iter: usize,
    max_iter: usize,
}

impl OtherQubitStateIterator {
    /// Creates a new iterator for the given system and gate qubits.
    pub fn new(n_qubits: usize, gate_qubits: &[usize]) -> Self {
        let mut other_qubits = Vec::with_capacity(n_qubits.saturating_sub(gate_qubits.len()));
        for i in 0..n_qubits {
            if !gate_qubits.contains(&i) {
                other_qubits.push(i);
            }
        }
        let max_iter = 1 << other_qubits.len();
        Self {
            other_qubits,
            current_iter: 0,
            max_iter,
        }
    }
}

impl Iterator for OtherQubitStateIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_iter >= self.max_iter {
            return None;
        }

        let i_other = self.current_iter;
        // This fold reconstructs the global index part for the non-gate qubits.
        let other_q_state = self.other_qubits.iter().enumerate().fold(0, |acc, (i, &q)| {
            if (i_other >> i) & 1 == 1 {
                acc | (1 << q)
            } else {
                acc
            }
        });

        self.current_iter += 1;
        Some(other_q_state)
    }
}

impl QuantumState {
    /// Creates a new `QuantumState` in the |0...0⟩ state.
    pub fn new(num_qubits: usize) -> Self {
        let dim = 1 << num_qubits;
        let mut state_vector = DVector::from_element(dim, Complex::new(0.0, 0.0));
        state_vector[0] = Complex::new(1.0, 0.0);
        Self {
            num_qubits,
            state_vector,
        }
    }

    /// Creates a new `QuantumState` from a given state vector.
    ///
    /// # Errors
    ///
    /// Returns an error if the state vector is not of the correct dimension or
    /// is not normalized.
    pub fn from_vector(
        num_qubits: usize,
        state_vector: &[Complex<f32>],
    ) -> Result<Self, &'static str> {
        let expected_dim = 1 << num_qubits;
        if state_vector.len() != expected_dim {
            return Err("Invalid state vector dimension.");
        }

        let norm_sq: f32 = state_vector.par_iter().map(|c| c.norm_sqr()).sum();
        if (norm_sq - 1.0).abs() > 1e-6 {
            return Err("State vector is not normalized.");
        }

        Ok(Self {
            num_qubits,
            state_vector: DVector::from_vec(state_vector.to_vec()),
        })
    }

    /// Applies a quantum operation to the state.
    pub fn apply_operation(&mut self, op: &Operation, _registry: &super::gates::registry::GateRegistry) {
        // By the time an operation reaches the engine, `prepare_for_execution`
        // should have been called. This guarantees:
        // 1. `op.matrix` is `Some(GateMatrix::LittleEndian(_))`.
        // 2. `op.qubits` contains the correct qubits to act upon.

        let matrix = op.matrix.as_ref().expect("Matrix not prepared for execution");
        let qubits_to_apply: Vec<usize> = op.qubits.iter().map(|&q| q as usize).collect();

        if !qubits_to_apply.is_empty() {
            match matrix {
                GateMatrix::LittleEndian(smat) => {
                    self.apply_sparse_gate(smat, &qubits_to_apply, &op.id.to_string())
                }
                GateMatrix::BigEndian(_) => {
                    // This path should be unreachable if the circuit was prepared correctly.
                    panic!(
                        "Engine Error: BigEndian matrix found for gate '{}'. All matrices must be converted to LittleEndian before execution.",
                        op.name.as_deref().unwrap_or(&op.id.to_string())
                    );
                }
            }
        }
    }

    /// Applies a sparse gate matrix to a subset of qubits in the quantum state.
    ///
    /// This is the core of the state vector simulation. When a gate acts on a
    /// subset of qubits, the state vector can be decomposed into independent
    /// subspaces. Each subspace corresponds to a fixed state of the non-gate
    /// qubits. This function iterates through these subspaces, applies the
    /// gate transformation to each one, and assembles the results into the
    /// new state vector.
    ///
    /// # Algorithm: Gather-Apply-Scatter
    /// 1.  **Decompose**: Identify the qubits the gate acts on (`gate_qubits`)
    ///     and those it doesn't (`other_qubits`).
    /// 2.  **Iterate**: Loop through all possible basis states of the `other_qubits`.
    ///     Each state defines a subspace.
    /// 3.  **Gather**: For each subspace, extract the relevant amplitudes from
    ///     the main state vector into a smaller `sub_vector`.
    /// 4.  **Apply**: Multiply the gate matrix `u` by the `sub_vector` to get
    ///     the transformed `new_sub_vector`.
    /// 5.  **Scatter**: Place the amplitudes from `new_sub_vector` into their
    ///     correct positions in a `next_state_vector`.
    /// 6.  **Update**: Replace the old state vector with the new one.
    ///
    /// # Arguments
    /// * `u` - The sparse matrix (`CsrMatrix`) representing the quantum gate.
    /// * `qubits` - A slice of qubit indices that the gate `u` acts on.
    fn apply_sparse_gate(&mut self, u: &CsrMatrix<Complex<f32>>, qubits: &[usize], op_id: &str) {
        let mut sorted_qubits = qubits.to_vec();
        sorted_qubits.sort_unstable();
        log::debug!("Apply_sparse_gate for '{}' gate on qubits: {:?} (sorted)", op_id, &sorted_qubits);
        let n_qubits = self.num_qubits;
        let n_gate_qubits = qubits.len();

        if n_gate_qubits == n_qubits {
            let next_state_vector = u * &self.state_vector;

            if log::log_enabled!(log::Level::Debug) {
                log::debug!("Transform (Full):\n{}", formatters::format_transform(u, &self.state_vector, &next_state_vector));
            }
            self.state_vector = next_state_vector;
            return;
        }

        let mut next_state_vector = DVector::from_element(1 << n_qubits, Complex::new(0.0, 0.0));

        for other_q_state in OtherQubitStateIterator::new(n_qubits, &sorted_qubits) {
            self.transform_subspace(u, &sorted_qubits, other_q_state, &mut next_state_vector);
        }
        self.state_vector = next_state_vector;
    }

    /// Applies the gate transformation to a single subspace of the state vector.
    ///
    /// This function implements the "gather-apply-scatter" method for a single
    /// fixed state of the non-gate qubits.
    ///
    /// # Arguments
    /// * `u` - The gate matrix.
    /// * `qubits` - The qubits the gate acts on.
    /// * `other_q_state` - The fixed state of the non-gate qubits, defining the subspace.
    /// * `next_state_vector` - The mutable next state vector to scatter results into.
    pub fn transform_subspace(
        &self,
        u: &CsrMatrix<Complex<f32>>,
        qubits: &[usize],
        other_q_state: usize,
        next_state_vector: &mut DVector<Complex<f32>>,
    ) {
        let n_gate_qubits = qubits.len();
        let gate_dim = 1 << n_gate_qubits;

        // Gather: Create a small vector for the subspace.
        let mut sub_vector_in = DVector::from_element(gate_dim, Complex::new(0.0, 0.0));
        for j_in in 0..gate_dim {
            let global_in_index = self._reconstruct_global_output_index(other_q_state, j_in, qubits);
            sub_vector_in[j_in] = self.state_vector[global_in_index];
        }
        log::debug!("Subspace other_q_state={}: sub_vector_in = {:?}", other_q_state, sub_vector_in);

        // Apply: Multiply the gate matrix by the subspace vector.
        let sub_vector_out = u * &sub_vector_in;
        log::debug!("Subspace other_q_state={}: sub_vector_out = {:?}", other_q_state, sub_vector_out);

        // Pretty-print the transformation for debugging.
        if log::log_enabled!(log::Level::Debug) {
            log::debug!(
                "Transform (Subspace for |...{:b}>):\n{}",
                other_q_state,
                formatters::format_transform(u, &sub_vector_in, &sub_vector_out)
            );
        }
        
        // Scatter: Add the results to the correct positions in the new state vector.
        for j_out in 0..gate_dim {
            if sub_vector_out[j_out].norm_sqr() > 1e-12 {
                let global_out_index = self._reconstruct_global_output_index(other_q_state, j_out, qubits);
                log::debug!("  Scattering j_out={} (local |{:b}>) to global index {} (|{:b}>) with amp {:?}",
                    j_out, j_out, global_out_index, global_out_index, sub_vector_out[j_out]);
                next_state_vector[global_out_index] += sub_vector_out[j_out];
            }
        }
    }

    /// Extracts the local state of the gate qubits from a global state index.
    ///
    /// This is a helper function for `apply_sparse_gate`. It maps a global state
    /// index `i_global` to a local index `j_in` that corresponds to the row of
    /// the gate matrix `u`.
    ///
    /// # Arguments
    /// * `i_global` - The global index in the 2^n state vector.
    /// * `qubits` - The sorted slice of qubit indices the gate acts on.
    ///
    /// # Returns
    /// The local index `j_in` for the gate's matrix.
    pub fn _extract_local_input_state(&self, i_global: usize, qubits: &[usize]) -> usize {
        let mut j_in = 0;
        for (pos, &q) in qubits.iter().enumerate() {
            if (i_global >> q) & 1 == 1 {
                j_in |= 1 << pos;
            }
        }
        log::trace!("Extracted local index: g_idx={} -> l_idx={} for qubits={:?}", i_global, j_in, qubits);
        j_in
    }

    /// Reconstructs the global output index from the state of the non-gate qubits
    /// and the local output state.
    ///
    /// This is a helper function for `apply_sparse_gate`. It performs the
    /// "scatter" part of the operation, calculating the final destination index in
    /// the `next_state_vector`.
    ///
    /// # Arguments
    /// * `other_q_state` - The part of the global index corresponding to non-gate qubits.
    /// * `j_out` - The local output index from the gate matrix multiplication (a column index).
    /// * `qubits` - The sorted slice of qubit indices the gate acts on.
    ///
    /// # Returns
    /// The global output index in the 2^n state vector.
    pub fn _reconstruct_global_output_index(&self, other_q_state: usize, j_out: usize, qubits: &[usize]) -> usize {
        let mut gate_q_state_out = 0;
        for (pos, &q) in qubits.iter().enumerate() {
            if (j_out >> pos) & 1 == 1 {
                gate_q_state_out |= 1 << q;
            }
        }
        let g_out = other_q_state | gate_q_state_out;
        log::trace!("Reconstructed global index: other_q={} | j_out={} -> g_out={} for qubits={:?}", other_q_state, j_out, g_out, qubits);
        g_out
    }

    /// Calculates the measurement probabilities for each computational basis state.
    pub fn probabilities(&self) -> Vec<f32> {
        self.state_vector.iter().map(|c| c.norm_sqr()).collect()
    }

    pub fn get_pretty_print_state_vector(&self) -> String {
        formatters::get_pretty_print_state_vector(&self.state_vector, self.num_qubits)
    }
}

/// The main simulator engine for running quantum circuits.
#[derive(Debug)]
pub struct Simulator {
    circuit: Circuit,
}

impl Simulator {
    /// Creates a new `Simulator` for the given circuit.
    pub fn new(circuit: Circuit) -> Self {
        Self { circuit }
    }

    /// Runs the simulation and returns the result.
    pub fn run(&mut self) -> super::circuit::RunResult {
        self.circuit.run(&super::circuit::RunOptions {
            shots: None,
            snapshot_state_per_step: false,
            rng_seed: None,
        })
    }
}