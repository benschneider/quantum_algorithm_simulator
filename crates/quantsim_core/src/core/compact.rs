use super::circuit::{Circuit, ValidationError};
use super::types::Operation;

impl Circuit {
    pub(super) fn validate_compact_input(&self, backend: &str) -> Result<(), ValidationError> {
        let error = |message| ValidationError {
            step: 0,
            op_index: 0,
            message,
        };
        if !(1..=64).contains(&self.num_qubits) {
            return Err(error(format!(
                "{backend} simulation supports 1–64 qubits. This circuit has {}.",
                self.num_qubits
            )));
        }
        if self.initial_state.is_some() {
            return Err(error(format!(
                "{backend} simulation requires a product preparation. Found an explicit initial state vector."
            )));
        }
        if let Some((wire, _)) = self
            .initial_preps
            .iter()
            .enumerate()
            .skip(self.num_qubits)
            .find(|(_, prep)| prep.is_some())
        {
            return Err(error(format!(
                "Initial preparation wire {wire} is out of bounds."
            )));
        }
        Ok(())
    }

    pub(super) fn visit_compact_operations(
        &self,
        step_count: usize,
        backend: &str,
        mut apply: impl FnMut(&Operation) -> Result<(), String>,
    ) -> Result<(), ValidationError> {
        for (step, column) in self.steps.iter().take(step_count).enumerate() {
            let mut occupied = 0u64;
            for (op_index, op) in column.iter().enumerate() {
                let error = |message| ValidationError {
                    step,
                    op_index,
                    message,
                };
                if op.matrix.is_some() {
                    return Err(error(format!(
                        "Gate {} has an explicit matrix. {backend} simulation requires a built-in definition.", op.id
                    )));
                }
                for &wire in &op.qubits {
                    if wire as usize >= self.num_qubits {
                        return Err(error(format!(
                            "Qubit {wire} is out of bounds for a {}-qubit circuit.",
                            self.num_qubits
                        )));
                    }
                    let mask = 1u64 << wire;
                    if occupied & mask != 0 {
                        return Err(error(format!(
                            "Qubit {wire} is used more than once in the same timestep."
                        )));
                    }
                    occupied |= mask;
                }
                apply(op).map_err(error)?;
            }
        }
        Ok(())
    }
}
