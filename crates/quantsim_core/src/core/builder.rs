//! # Builder
//!
//! This module provides a `CircuitBuilder` for programmatically constructing
//! quantum circuits. The builder pattern allows for a fluent and intuitive
//! way to add gates and define the structure of a circuit.

use super::circuit::Circuit;
use super::gates::registry::GateRegistry;
use super::types::{Gate, GateMatrix, Operation, QubitPrep};
use nalgebra::{Complex, DMatrix};

/// A builder for programmatically constructing a `Circuit`.
///
/// Operations are added to a pending time step. Calling `next_step()`
/// finalizes the current time step and prepares for a new one.
pub struct CircuitBuilder {
    num_qubits: usize,
    steps: Vec<Vec<Operation>>,
    initial_preps: Vec<Option<QubitPrep>>,
    current_step_ops: Vec<Operation>,
    registry: GateRegistry,
}

impl CircuitBuilder {
    /// Creates a new `CircuitBuilder` for a circuit with the specified number of qubits.
    pub fn new(num_qubits: usize) -> Self {
        Self {
            num_qubits,
            steps: Vec::new(),
            initial_preps: vec![None; num_qubits],
            current_step_ops: Vec::new(),
            registry: GateRegistry::new_with_standard_gates(),
        }
    }

    /// Adds a gate to the current time step of the circuit.
    ///
    /// # Arguments
    ///
    /// * `id` - The `Gate` enum variant to add.
    /// * `qubits` - The qubits to apply the gate to.
    /// * `params` - The parameters for the gate, if any.
    pub fn add_gate(&mut self, id: Gate, qubits: &[u32], params: &[f32]) -> &mut Self {
        let op = Operation::new(id, qubits.to_vec(), params.to_vec());
        //log::debug!("Adding operation: {:?}", op);
        self.current_step_ops.push(op);
        self
    }

    /// Adds a custom gate to the current time step.
    pub fn add_custom_gate(
        &mut self,
        name: &str,
        matrix: DMatrix<Complex<f32>>,
        qubits: &[u32],
    ) -> &mut Self {
        let mut op = Operation::new(Gate::Custom, qubits.to_vec(), vec![]);
        op.name = Some(name.to_string());
        op.matrix = Some(GateMatrix::BigEndian(matrix));
        self.current_step_ops.push(op);
        self
    }

    // --- Gate Helper Methods ---

    pub fn h(&mut self, q: u32) -> &mut Self {
        self.add_gate(Gate::H, &[q], &[])
    }

    pub fn x(&mut self, q: u32) -> &mut Self {
        self.add_gate(Gate::X, &[q], &[])
    }

    pub fn y(&mut self, q: u32) -> &mut Self {
        self.add_gate(Gate::Y, &[q], &[])
    }

    pub fn z(&mut self, q: u32) -> &mut Self {
        self.add_gate(Gate::Z, &[q], &[])
    }

    pub fn cx(&mut self, control: u32, target: u32) -> &mut Self {
        self.add_gate(Gate::CX, &[control, target], &[])
    }

    pub fn cz(&mut self, control: u32, target: u32) -> &mut Self {
        self.add_gate(Gate::CZ, &[control, target], &[])
    }
    
    /// Adds a controlled-controlled-NOT (CCNOT or Toffoli) gate.
    ///
    /// The gate flips the `target` qubit if and only if both `control1` and
    /// `control2` are in the |1> state.
    ///
    /// # Arguments
    ///
    /// * `control1` - The first control qubit.
    /// * `control2` - The second control qubit.
    /// * `target` - The target qubit.
    pub fn ccnot(&mut self, control1: u32, control2: u32, target: u32) -> &mut Self {
        self.add_gate(Gate::CCNOT, &[control1, control2, target], &[])
    }

    pub fn swap(&mut self, q1: u32, q2: u32) -> &mut Self {
        self.add_gate(Gate::SWAP, &[q1, q2], &[])
    }

    pub fn rz(&mut self, q: u32, phi: f32) -> &mut Self {
        self.add_gate(Gate::Rz, &[q], &[phi])
    }

    pub fn crz(&mut self, control: u32, target: u32, theta: f32) -> &mut Self {
        self.add_gate(Gate::CRz, &[control, target], &[theta])
    }

    pub fn set_initial_preps(&mut self, preps: Vec<Option<QubitPrep>>) -> &mut Self {
        self.initial_preps = preps;
        self
    }

    /// Finalizes the current time step and prepares for the next one.
    pub fn next_step(&mut self) -> &mut Self {
        if !self.current_step_ops.is_empty() {
            //log::debug!("Finalizing step with ops: {:?}", self.current_step_ops);
            self.steps.push(self.current_step_ops.drain(..).collect());
        }
        self.current_step_ops.clear();
        self
    }

    /// Builds the `Circuit` object, consuming the builder.
    pub fn build(mut self) -> Circuit {
        self.next_step(); // Finalize any pending operations
        Circuit {
            num_qubits: self.num_qubits,
            steps: self.steps,
            initial_preps: self.initial_preps,
            registry: self.registry,
            initial_state: None,
        }
    }
}
