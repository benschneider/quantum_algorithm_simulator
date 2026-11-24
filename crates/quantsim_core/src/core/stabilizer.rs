//! Exact computational-basis readout of Clifford circuits on 1–64 wires.
//! Uses the [Aaronson–Gottesman tableau](https://arxiv.org/abs/quant-ph/0406196).

mod tableau;

use super::circuit::{Circuit, ValidationError};
use super::types::{Gate, Operation, QubitPrep};
pub use tableau::{StabilizerAmplitudes, StabilizerState};

impl Circuit {
    /// Simulates the prepared input and the first `step_count` authored columns
    /// in a packed tableau. A count past the last column runs the whole circuit.
    ///
    /// Supports the four per-wire preparations and the built-in H, SqrtZ (S),
    /// CX, X, Y and Z gates. An explicit initial vector or gate matrix requires
    /// a different simulator. Only operations in the requested prefix are checked.
    /// Operation diagnostics use zero-based authored column and operation indices.
    ///
    /// ```
    /// use quantsim_core::CircuitBuilder;
    ///
    /// let mut builder = CircuitBuilder::new(50);
    /// builder.h(0);
    /// for target in 1..50 {
    ///     builder.next_step().cx(0, target);
    /// }
    /// let circuit = builder.build();
    /// let state = circuit.run_stabilizer_prefix(circuit.steps.len()).unwrap();
    /// assert_eq!(state.probability_one(49), Some(0.5));
    /// let ones = (1_u64 << 50) - 1;
    /// assert!(state.sample(32, 42).iter().all(|&bits| bits == 0 || bits == ones));
    /// ```
    pub fn run_stabilizer_prefix(
        &self,
        step_count: usize,
    ) -> Result<StabilizerState, ValidationError> {
        self.validate_compact_input("Stabilizer")?;
        let mut state = StabilizerState::zero(self.num_qubits);
        for (wire, &prep) in self.initial_preps.iter().enumerate() {
            let Some(prep) = prep else { continue };
            match prep {
                QubitPrep::Zero => {}
                QubitPrep::One => state.x(wire),
                QubitPrep::Plus => state.h(wire),
                QubitPrep::Minus => {
                    state.x(wire);
                    state.h(wire);
                }
            }
        }

        self.visit_compact_operations(step_count, "Stabilizer", |op| state.apply_operation(op))?;
        Ok(state)
    }
}

impl StabilizerState {
    fn apply_operation(&mut self, op: &Operation) -> Result<(), String> {
        match (&op.id, op.qubits.as_slice()) {
            (Gate::H, &[q]) => self.h(q as usize),
            (Gate::SqrtZ, &[q]) => self.s(q as usize),
            (Gate::X, &[q]) => self.x(q as usize),
            (Gate::Y, &[q]) => self.y(q as usize),
            (Gate::Z, &[q]) => self.z(q as usize),
            (Gate::CX, &[control, target]) => self.cx(control as usize, target as usize),
            (Gate::H | Gate::SqrtZ | Gate::X | Gate::Y | Gate::Z, _) => {
                return Err(format!("Gate {} requires one qubit.", op.id))
            }
            (Gate::CX, _) => return Err("Gate CX requires two qubits (control, target).".into()),
            _ => {
                return Err(format!(
                    "Gate {} is unsupported by the stabilizer simulator.",
                    op.id
                ))
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_selected_columns_require_stabilizer_gates() {
        let mut circuit = Circuit::new(2);
        circuit.initial_preps[1] = Some(QubitPrep::One);
        let h = Operation::new(Gate::H, vec![0], vec![]);
        circuit.steps = vec![
            vec![h.clone()],
            vec![Operation::new(Gate::Rz, vec![1], vec![0.3])],
        ];

        assert_eq!(
            circuit.run_stabilizer_prefix(0).unwrap().sample(1, 1),
            [0b10]
        );
        assert_eq!(
            circuit.run_stabilizer_prefix(1).unwrap().probability_one(0),
            Some(0.5)
        );
        let error = circuit.run_stabilizer_prefix(usize::MAX).unwrap_err();
        assert_eq!((error.step, error.op_index), (1, 0));
        assert!(error.message.contains("Rz"));

        // Even a gate named H can carry an authored matrix for a different operation.
        let mut overridden = h;
        overridden.matrix = circuit.registry.eval(&Gate::X, &[], &[0]);
        circuit.steps[1] = vec![Operation::new(Gate::Z, vec![1], vec![]), overridden];
        let error = circuit.run_stabilizer_prefix(2).unwrap_err();
        assert_eq!((error.step, error.op_index), (1, 1));
        assert!(error.message.contains("explicit matrix"));
    }

    #[test]
    fn invalid_wires_and_unsupported_inputs_return_diagnostics() {
        let gate = |id, wires| Operation::new(id, wires, vec![]);
        let mut circuit = Circuit::new(2);
        for (column, index, reason) in [
            (vec![gate(Gate::X, vec![2])], 0, "out of bounds"),
            (vec![gate(Gate::H, vec![])], 0, "one qubit"),
            (vec![gate(Gate::CX, vec![0])], 0, "two qubits"),
            (vec![gate(Gate::CX, vec![0, 0])], 0, "more than once"),
            (
                vec![gate(Gate::X, vec![1]), gate(Gate::CX, vec![0, 1])],
                1,
                "more than once",
            ),
        ] {
            circuit.steps = vec![column];
            let error = circuit.run_stabilizer_prefix(1).unwrap_err();
            assert_eq!((error.step, error.op_index), (0, index));
            assert!(error.message.contains(reason), "{}", error.message);
        }

        circuit.initial_preps.push(Some(QubitPrep::Plus));
        assert!(circuit
            .run_stabilizer_prefix(0)
            .unwrap_err()
            .message
            .contains("out of bounds"));
        circuit.initial_preps.pop();
        circuit.initial_state = Some(vec![1.0.into(), 0.0.into(), 0.0.into(), 0.0.into()]);
        assert!(circuit
            .run_stabilizer_prefix(0)
            .unwrap_err()
            .message
            .contains("explicit initial state"));

        for width in [0, 65] {
            assert!(Circuit::new(width)
                .run_stabilizer_prefix(0)
                .unwrap_err()
                .message
                .contains("1–64"));
        }
    }
}
