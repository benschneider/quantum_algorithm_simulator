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
use nalgebra_sparse::ops::{serial::spmm_csr_dense, Op};
use nalgebra_sparse::CsrMatrix;
use rayon::prelude::*;

const MIN_SUBSPACE_PROBABILITY: f32 = 1e-12;

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
    other_mask: usize,
    next_index: Option<usize>,
}

impl OtherQubitStateIterator {
    /// Creates a new iterator for the given system and gate qubits.
    pub fn new(n_qubits: usize, gate_qubits: &[usize]) -> Self {
        let mut other_mask = 0;
        for q in 0..n_qubits {
            if !gate_qubits.contains(&q) {
                other_mask |= 1 << q;
            }
        }
        Self {
            other_mask,
            next_index: Some(0),
        }
    }
}

impl Iterator for OtherQubitStateIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.next_index?;
        // Let the carry skip the gate bits, then clear those bits again.
        self.next_index = (index != self.other_mask)
            .then(|| (index | !self.other_mask).wrapping_add(1) & self.other_mask);
        Some(index)
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
    /// Single-qubit gates use disjoint amplitude pairs. The general case uses
    /// the gather-apply-scatter algorithm below and supports per-subspace logs.
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

        if let [qubit] = sorted_qubits.as_slice() {
            if u.nrows() == 2 && u.ncols() == 2 && !log::log_enabled!(log::Level::Debug) {
                self.apply_one_qubit_gate(u, *qubit);
                return;
            }
        }

        let mut next_state_vector = DVector::from_element(1 << n_qubits, Complex::new(0.0, 0.0));

        let mut subspace = GateSubspace::new(self, u, &sorted_qubits);
        for other_q_state in OtherQubitStateIterator::new(n_qubits, &sorted_qubits) {
            subspace.transform(&self.state_vector, other_q_state, &mut next_state_vector);
        }
        self.state_vector = next_state_vector;
    }

    /// Within each block, the amplitudes in a pair differ only in the target bit.
    /// Copy both inputs before replacing either one.
    fn apply_one_qubit_gate(&mut self, matrix: &CsrMatrix<Complex<f32>>, qubit: usize) {
        let rows = [matrix.row(0), matrix.row(1)];
        let stride = 1 << qubit;
        for chunk in self
            .state_vector
            .as_mut_slice()
            .chunks_exact_mut(2 * stride)
        {
            let (low, high) = chunk.split_at_mut(stride);
            for (low, high) in low.iter_mut().zip(high) {
                let input = [*low, *high];
                for (target, row) in [low, high].into_iter().zip(&rows) {
                    let mut amp = Complex::new(0.0, 0.0);
                    for (&column, &coefficient) in row.col_indices().iter().zip(row.values()) {
                        amp += coefficient * input[column];
                    }
                    // The sparse kernel drops nonfinite outputs during scattering.
                    *target = if amp.is_finite() && amp.norm_sqr() > MIN_SUBSPACE_PROBABILITY {
                        amp
                    } else {
                        Complex::default()
                    };
                }
            }
        }
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
        GateSubspace::new(self, u, qubits).transform(
            &self.state_vector,
            other_q_state,
            next_state_vector,
        );
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

/// Reuse the local basis offsets and matrix-vector workspace for every
/// assignment of the non-gate qubits.
struct GateSubspace<'a> {
    matrix: &'a CsrMatrix<Complex<f32>>,
    offsets: Vec<usize>,
    input: DVector<Complex<f32>>,
    output: DVector<Complex<f32>>,
}

impl<'a> GateSubspace<'a> {
    fn new(state: &QuantumState, matrix: &'a CsrMatrix<Complex<f32>>, qubits: &[usize]) -> Self {
        let dim = 1 << qubits.len();
        Self {
            matrix,
            offsets: (0..dim)
                .map(|local| state._reconstruct_global_output_index(0, local, qubits))
                .collect(),
            input: DVector::zeros(dim),
            output: DVector::zeros(dim),
        }
    }

    fn transform(
        &mut self,
        state: &DVector<Complex<f32>>,
        base: usize,
        next: &mut DVector<Complex<f32>>,
    ) {
        for (input, &offset) in self.input.iter_mut().zip(&self.offsets) {
            *input = state[base | offset];
        }
        // spmm scales the old output by zero, even when a custom gate produced NaN.
        self.output.fill(Complex::new(0.0, 0.0));
        spmm_csr_dense(
            Complex::new(0.0, 0.0),
            &mut self.output,
            Complex::new(1.0, 0.0),
            Op::NoOp(self.matrix),
            Op::NoOp(&self.input),
        );

        if log::log_enabled!(log::Level::Debug) {
            log::debug!("Subspace other_q_state={base}: sub_vector_in = {:?}", self.input);
            log::debug!("Subspace other_q_state={base}: sub_vector_out = {:?}", self.output);
            log::debug!(
                "Transform (Subspace for |...{base:b}>):\n{}",
                formatters::format_transform(self.matrix, &self.input, &self.output)
            );
        }
        for (local, (&amp, &offset)) in self.output.iter().zip(&self.offsets).enumerate() {
            if amp.norm_sqr() > MIN_SUBSPACE_PROBABILITY {
                let global = base | offset;
                log::debug!("  Scattering j_out={local} (local |{local:b}>) to global index {global} (|{global:b}>) with amp {amp:?}");
                next[global] += amp;
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Gate;

    #[test]
    fn subspace_iterator_visits_each_assignment_in_order() {
        for width in 0..=5 {
            let dimension = 1 << width;
            for gate_mask in 0..dimension {
                let gate_qubits: Vec<_> = (0..width)
                    .filter(|q| gate_mask & (1 << q) != 0)
                    .collect();
                let expected: Vec<_> = (0..dimension).filter(|i| i & gate_mask == 0).collect();
                let actual: Vec<_> = OtherQubitStateIterator::new(width, &gate_qubits).collect();
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn one_qubit_pairs_match_sparse_scattering_at_every_target() {
        let mut input = QuantumState::new(4);
        for (i, amp) in input.state_vector.iter_mut().enumerate() {
            *amp = Complex::new(i as f32 / 32.0, (15 - i) as f32 / 64.0);
        }
        input.state_vector[0] = Complex::new(0.5e-6, 0.0);
        input.state_vector[1] = Complex::new(2e-6, 0.0);
        input.state_vector[2] = Complex::new(f32::NAN, 0.0);
        let scale = std::f32::consts::FRAC_1_SQRT_2;
        let real = Complex::new(scale, 0.0);
        let imag = Complex::new(0.0, scale);
        let dense = CsrMatrix::try_from_csr_data(
            2,
            2,
            vec![0, 2, 4],
            vec![0, 1, 0, 1],
            vec![real, imag, real, -imag],
        )
        .unwrap();
        let swap = CsrMatrix::try_from_csr_data(
            2,
            2,
            vec![0, 1, 2],
            vec![1, 0],
            vec![Complex::new(1.0, 0.0); 2],
        )
        .unwrap();

        for matrix in [dense, swap] {
            for q in 0..input.num_qubits {
                let mut expected = DVector::zeros(input.state_vector.len());
                for base in (0..expected.len()).filter(|i| i & (1 << q) == 0) {
                    input.transform_subspace(&matrix, &[q], base, &mut expected);
                }
                let mut actual = input.clone();
                actual.apply_one_qubit_gate(&matrix, q);
                assert_eq!(actual.state_vector, expected);
            }
        }
    }

    #[test]
    fn swap_nonadjacent_qubits_transforms_each_subspace_independently() {
        let mut circuit = Circuit::new(4);
        let mut input: Vec<_> = (0..16)
            .map(|i| Complex::new(i as f32, (15 - i) as f32))
            .collect();
        let norm = input.iter().map(Complex::norm_sqr).sum::<f32>().sqrt();
        input.iter_mut().for_each(|amp| *amp /= norm);
        circuit.initial_state = Some(input.clone());
        circuit.steps.push(vec![Operation::new(Gate::SWAP, vec![0, 2], vec![])]);

        let result = circuit.run(&Default::default());
        assert!(result.diagnostics.is_empty());
        let output = result.final_state_vector.unwrap();
        for (index, amplitude) in input.iter().enumerate() {
            let swapped = (index & !0b0101) | ((index & 1) << 2) | ((index & 4) >> 2);
            assert_eq!(output[swapped], *amplitude);
        }
    }
}
