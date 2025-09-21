use quantsim_core::core::circuit::Circuit;
use smallvec::smallvec;
use quantsim_core::core::types::Gate;

/// The `CircuitState` struct contains all the state related to the quantum
/// circuit itself. This includes the circuit definition, the number of qubits,
/// and the number of timesteps.
///
/// This struct also provides methods for manipulating the circuit, such as
/// placing, moving, and deleting gates. These methods are called by the
/// message handlers in response to user actions.
#[derive(Debug)]
pub struct CircuitState {
    pub circuit: Circuit,
    pub num_qubits: usize,
    pub num_timesteps: usize,
}

impl CircuitState {
    /// Places a gate on the circuit grid.
    ///
    /// # Arguments
    ///
    /// * `gate_id` - The ID of the gate to place.
    /// * `row` - The row (qubit) to place the gate on.
    /// * `col` - The column (timestep) to place the gate on.
    pub fn place_gate(&mut self, gate_id: Gate, row: usize, col: usize) {
        log::debug!("Placing gate '{:?}' at ({}, {})", gate_id, row, col);
        if col >= self.circuit.steps.len() {
            return;
        }

        if let Some(meta) = self.circuit.registry.get_meta(&gate_id) {
            let arity = meta.arity;
            let mut qubits = smallvec![row as u32];
            match arity {
                quantsim_core::core::types::Arity::OneQ => {}
                quantsim_core::core::types::Arity::TwoQ => {
                    if row + 1 < self.num_qubits {
                        qubits.push((row + 1) as u32);
                    } else {
                        log::warn!("Not enough qubits to place a two-qubit gate at row {}", row);
                        return;
                    }
                }
                quantsim_core::core::types::Arity::ThreeQ => {
                    if row + 2 < self.num_qubits {
                        qubits.push((row + 1) as u32);
                        qubits.push((row + 2) as u32);
                    } else {
                        log::warn!(
                            "Not enough qubits to place a three-qubit gate at row {}",
                            row
                        );
                        return;
                    }
                }
            }

            // Check for collisions
            for q in &qubits {
                for op in &self.circuit.steps[col] {
                    if op.qubits.contains(&(*q as u32)) {
                        log::warn!("Collision detected at timestep {}, qubit {}", col, q);
                        return;
                    }
                }
            }

            let params = if meta.is_parametric {
                smallvec![quantsim_core::core::types::Param::Scalar(0.0)]
            } else {
                smallvec![]
            };

            let op = quantsim_core::core::types::Operation {
                id: gate_id.clone(),
                name: None,
                params,
                qubits,
                matrix: None,
            };
            self.circuit.steps[col].push(op);
        }
    }

    /// Places a multi-qubit gate on the circuit grid.
    ///
    /// # Arguments
    ///
    /// * `gate_id` - The ID of the gate to place.
    /// * `qubits` - The qubits to place the gate on.
    /// * `col` - The column (timestep) to place the gate on.
    pub fn place_multi_qubit_gate(&mut self, gate_id: Gate, qubits: Vec<usize>, col: usize) {
        log::debug!(
            "Placing multi-qubit gate '{:?}' at col {} on qubits {:?}",
            gate_id,
            col,
            qubits
        );
        if col >= self.circuit.steps.len() {
            return;
        }

        if let Some(meta) = self.circuit.registry.get_meta(&gate_id) {
            // Basic validation
            // if meta.arity as usize != qubits.len() {
            //     log::warn!(
            //         "Arity mismatch for gate '{}'. Expected {}, got {}.",
            //         gate_id,
            //         meta.arity as usize,
            //         qubits.len()
            //     );
            //     return;
            // }

            // Check for collisions
            for q in &qubits {
                for op in &self.circuit.steps[col] {
                    if op.qubits.contains(&(*q as u32)) {
                        log::warn!("Collision detected at timestep {}, qubit {}", col, q);
                        return;
                    }
                }
            }

            let params = if meta.label.contains("Gate") {
                smallvec![quantsim_core::core::types::Param::Scalar(0.0)]
            } else {
                smallvec![]
            };

            let op = quantsim_core::core::types::Operation {
                id: gate_id.clone(),
                name: None,
                params,
                qubits: qubits.into_iter().map(|q| q as u32).collect(),
                matrix: None,
            };
            self.circuit.steps[col].push(op);
        }
    }

    /// Moves a gate from one position to another on the circuit grid.
    ///
    /// # Arguments
    ///
    /// * `from_row` - The row (qubit) to move the gate from.
    /// * `from_col` - The column (timestep) to move the gate from.
    /// * `_to_row` - The row (qubit) to move the gate to (currently unused).
    /// * `to_col` - The column (timestep) to move the gate to.
    pub fn move_gate(&mut self, from_row: usize, from_col: usize, _to_row: usize, to_col: usize) {
        if from_col < self.circuit.steps.len() && to_col < self.circuit.steps.len() {
            if let Some(op) = self.delete_gate(from_row, from_col) {
                self.circuit.steps[to_col].push(op);
                // Note: This doesn't handle multi-qubit gates correctly yet.
                // It also doesn't update the row.
            }
        }
    }

    /// Deletes a gate from the circuit grid.
    ///
    /// # Arguments
    ///
    /// * `row` - The row (qubit) of the gate to delete.
    /// * `col` - The column (timestep) of the gate to delete.
    ///
    /// # Returns
    ///
    /// An `Option` containing the deleted gate, or `None` if no gate was found.
    pub fn delete_gate(&mut self, row: usize, col: usize) -> Option<quantsim_core::core::types::Operation> {
        if col < self.circuit.steps.len() {
            if let Some(index) = self.circuit.steps[col]
                .iter()
                .position(|op| op.qubits.contains(&(row as u32)))
            {
                return Some(self.circuit.steps[col].remove(index));
            }
        }
        None
    }
}

impl Default for CircuitState {
    fn default() -> Self {
        let num_qubits = 2;
        let num_timesteps = 10;
        let mut circuit = Circuit::new(num_qubits);
        circuit.steps.resize(num_timesteps, Vec::new());
        Self {
            circuit,
            num_qubits,
            num_timesteps,
        }
    }
}
