use super::circuit::{Circuit, ValidationError};
use super::gates::registry::GateRegistry;
use super::types::{Operation, Param};

/// Maximum circuit width supported by the compact simulators' packed bitstrings.
pub const MAX_COMPACT_QUBITS: usize = u128::BITS as usize;

pub(super) fn gate_parameter(op: &Operation, registry: &GateRegistry) -> Result<f64, String> {
    let arity = registry
        .get_meta(&op.id)
        .ok_or_else(|| format!("Unknown gate {}.", op.id))?
        .arity
        .as_usize();
    if op.qubits.len() != arity {
        return Err(format!("Gate {} requires {arity} qubits.", op.id));
    }
    if op.params.iter().any(|Param::Scalar(v)| !v.is_finite()) {
        return Err(format!("Gate {} requires finite parameters.", op.id));
    }
    Ok(op
        .params
        .first()
        .map_or(0.0, |Param::Scalar(v)| f64::from(*v)))
}

impl Circuit {
    pub(super) fn validate_compact_input(&self, backend: &str) -> Result<(), ValidationError> {
        let error = |message| ValidationError {
            step: 0,
            op_index: 0,
            message,
        };
        if !(1..=MAX_COMPACT_QUBITS).contains(&self.num_qubits) {
            return Err(error(format!(
                "{backend} simulation supports 1–{MAX_COMPACT_QUBITS} qubits. This circuit has {}.",
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
            let mut occupied = 0u128;
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
                    let mask = 1u128 << wire;
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
