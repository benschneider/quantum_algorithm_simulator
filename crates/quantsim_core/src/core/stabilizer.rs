//! Exact computational-basis readout of Clifford circuits on variable-width registers.
//! Uses the [Aaronson–Gottesman tableau](https://arxiv.org/abs/quant-ph/0406196).

mod tableau;

use super::circuit::{Circuit, ValidationError};
use super::types::{Gate, Operation, QubitPrep};
pub use tableau::{StabilizerAmplitudes, StabilizerState};

impl Circuit {
    /// Checks input and gates for the selected prefix without constructing a tableau.
    pub fn validate_stabilizer_prefix(&self, step_count: usize) -> Result<(), ValidationError> {
        self.validate_compact_input("Stabilizer")?;
        self.visit_compact_operations(step_count, "Stabilizer", |op| {
            CliffordOperation::decode(op)
                .map(|_| ())
                .map_err(|reason| format!("Gate {} {reason}", op.id))
        })
    }

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
    /// let ones = (1_u128 << 50) - 1;
    /// assert!(state.sample(32, 42).iter().all(|&bits| bits == 0 || bits == ones));
    /// ```
    pub fn run_stabilizer_prefix(
        &self,
        step_count: usize,
    ) -> Result<StabilizerState, ValidationError> {
        self.validate_stabilizer_prefix(step_count)?;
        let mut state =
            StabilizerState::zero(self.num_qubits).map_err(|message| ValidationError {
                step: 0,
                op_index: 0,
                message,
            })?;
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

        for op in self.steps.iter().take(step_count).flatten() {
            CliffordOperation::decode(op)
                .expect("validated Clifford operation")
                .apply(&mut state);
        }
        Ok(state)
    }
}

enum CliffordOperation {
    Single(fn(&mut StabilizerState, usize), usize),
    Cx(usize, usize),
}

impl CliffordOperation {
    fn decode(op: &Operation) -> Result<Self, &'static str> {
        if op.id == Gate::CX {
            return match op.qubits.as_slice() {
                &[control, target] => Ok(Self::Cx(control as usize, target as usize)),
                _ => Err("requires two qubits (control, target)."),
            };
        }
        let apply = match op.id {
            Gate::H => StabilizerState::h,
            Gate::SqrtZ => StabilizerState::s,
            Gate::X => StabilizerState::x,
            Gate::Y => StabilizerState::y,
            Gate::Z => StabilizerState::z,
            _ => return Err("is unsupported by the stabilizer simulator."),
        };
        match op.qubits.as_slice() {
            &[wire] => Ok(Self::Single(apply, wire as usize)),
            _ => Err("requires one qubit."),
        }
    }

    fn apply(self, state: &mut StabilizerState) {
        match self {
            Self::Single(apply, wire) => apply(state, wire),
            Self::Cx(control, target) => state.cx(control, target),
        }
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

        assert!(Circuit::new(0).run_stabilizer_prefix(0).is_err());
    }
}
