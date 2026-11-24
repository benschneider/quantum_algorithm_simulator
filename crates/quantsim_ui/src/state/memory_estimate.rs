use super::simulation_state::{COMPACT_SHOTS, MAX_SAMPLE_ROWS, WAVEFORM_POINTS};
use super::{AppState, CircuitState};
use nalgebra::Complex;
use quantsim_core::grouped::MAX_GROUP_QUBITS;
use quantsim_core::types::{GateMatrix, Operation};
use std::mem::size_of;

const AMPLITUDE: u128 = size_of::<Complex<f32>>() as u128;
const INDEX: u128 = size_of::<usize>() as u128;
const PACKED_SHOT: u128 = size_of::<u128>() as u128;
const SAMPLE_ROW: u128 = size_of::<(u128, u32)>() as u128;

/// A conservative allowance for the major buffers of a full UI Run.
/// Use wide byte counts on wasm32 as well, and only inspect sizes of existing allocations.
pub(crate) fn estimated_peak_bytes(state: &AppState) -> Option<u64> {
    let circuit = &state.circuit_state.circuit;
    let readout = &state.simulation_state;
    if !(1..=CircuitState::MAX_QUBITS).contains(&circuit.num_qubits) {
        return None;
    }

    // Include retained buffers that overlap the next run.
    let mut bytes = readout.populated_indices.capacity() as u128 * INDEX;
    let wave_point = size_of::<Complex<f64>>() as u128;
    bytes += WAVEFORM_POINTS as u128 * wave_point;
    if let Some(wave) = readout.waveform() {
        bytes += wave.points.capacity() as u128 * wave_point;
    }
    if let Some(samples) = &readout.compact_samples {
        bytes += samples.rows.capacity() as u128 * SAMPLE_ROW;
        // The old compact state remains available for graph navigation until
        // the next run finishes, even if the circuit has since been resized.
        bytes += compact_state_bytes(readout.wire_one_probabilities.len());
    }
    bytes += (circuit.num_qubits as u128 + readout.wire_one_probabilities.capacity() as u128)
        * size_of::<f64>() as u128;
    for previous in readout.quantum_state.iter().chain(&readout.snapshot_states) {
        bytes += previous.state_vector.data.as_vec().capacity() as u128 * AMPLITUDE;
    }
    bytes += state.initial_state_editor_state.state_vector.capacity() as u128 * AMPLITUDE;
    if let Some(input) = &circuit.initial_state {
        bytes += input.capacity() as u128 * AMPLITUDE;
    }

    if circuit.num_qubits > CircuitState::MAX_DENSE_QUBITS {
        // Two tableaus, each with two rows per wire. Allow for X/Z masks and padded phase.
        bytes += 4 * circuit.num_qubits as u128 * 3 * PACKED_SHOT;
        bytes += compact_state_bytes(circuit.num_qubits);
        // Merging factors, applying a gate and sampling each use at most a
        // few group-sized buffers. Gate matrices are at most 8 by 8.
        bytes += (1_u128 << MAX_GROUP_QUBITS) * 4 * wave_point;
        bytes += 8 * 8 * (AMPLITUDE + wave_point);
        // Packed shots and temporary counts, including vector growth during grouping.
        bytes += u128::from(COMPACT_SHOTS) * (PACKED_SHOT + 2 * SAMPLE_ROW);
        bytes += MAX_SAMPLE_ROWS as u128 * SAMPLE_ROW;
        return u64::try_from(bytes).ok();
    }

    // The dense readout can list every outcome. Allow for both old and new
    // storage while its index vector grows.
    let outcomes = u128::from(1u64.checked_shl(circuit.num_qubits.try_into().ok()?)?);
    bytes += outcomes * (AMPLITUDE + size_of::<f32>() as u128 + 2 * INDEX);
    if circuit.initial_state.is_none() {
        // A wire preparation can synthesize up to two single-qubit gates.
        bytes += circuit.initial_preps.iter().flatten().count() as u128 * 2 * gate_bytes(2, 4);
    }

    for op in circuit.steps.iter().flatten() {
        let (rows, entries) = match &op.matrix {
            Some(GateMatrix::BigEndian(m)) => (m.nrows(), m.data.as_vec().capacity()),
            Some(GateMatrix::LittleEndian(m)) => (m.nrows(), m.nnz()),
            None => {
                let rows = 1usize << circuit.registry.get_meta(&op.id)?.arity.as_usize();
                (rows, rows * rows)
            }
        };
        bytes += gate_bytes(rows as u128, entries as u128);
    }
    u64::try_from(bytes).ok()
}

fn compact_state_bytes(wires: usize) -> u128 {
    // Packing the wires into maximum-width groups bounds the total number
    // of amplitudes, since two groups cost at most as much as their product.
    let groups = wires.div_ceil(MAX_GROUP_QUBITS) as u128;
    let amplitudes = groups * (1_u128 << MAX_GROUP_QUBITS);
    // Include factor metadata, wire indices and singleton state.
    amplitudes * size_of::<Complex<f64>>() as u128 + wires as u128 * 128
}

fn gate_bytes(rows: u128, entries: u128) -> u128 {
    // Count the authored, cloned and reordered dense matrices, growing sparse
    // value/index buffers, and the per-row gate workspace together.
    entries * (3 * AMPLITUDE + 2 * (AMPLITUDE + INDEX))
        + (rows + 1) * (2 * AMPLITUDE + 3 * INDEX)
        + 2 * size_of::<Operation>() as u128
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{messages::Message, QCSimApp};
    use nalgebra::DMatrix;
    use quantsim_core::types::Gate;

    #[test]
    fn counts_imported_input_and_explicit_gate_storage() {
        let mut state = AppState::new();
        let small = estimated_peak_bytes(&state).unwrap();
        assert!(small > 0 && small < 1024 * 1024);

        // Reserve extra space so this also checks retained capacity, not just length.
        let mut input = Vec::with_capacity(16);
        input.resize(4, Complex::default());
        input[0].re = 1.0;
        let input_bytes = input.capacity() as u64 * AMPLITUDE as u64;
        state.circuit_state.circuit.initial_state = Some(input);
        assert_eq!(estimated_peak_bytes(&state), Some(small + input_bytes));

        let mut op = Operation::new(Gate::Custom, vec![0, 1], vec![]);
        op.matrix = Some(GateMatrix::BigEndian(DMatrix::identity(4, 4)));
        state.circuit_state.circuit.steps.push(vec![op]);
        assert!(estimated_peak_bytes(&state).unwrap() >= small + input_bytes + 3 * 16 * 8);
    }

    #[test]
    fn estimates_cover_measured_first_run_and_rerun_at_21_qubits() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::UpdateCircuitFromJson(
            r#"{"num_qubits":21,"steps":[[{"id":"H","qubits":[0]}]]}"#.into(),
        ));
        let mib = 1024 * 1024;
        let first = estimated_peak_bytes(&app.state).unwrap();
        assert!((40 * mib..80 * mib).contains(&first));
        app.handle_message(Message::RunSimulation);
        let rerun = estimated_peak_bytes(&app.state).unwrap();
        assert!(rerun >= 56 * mib && rerun >= first + 16 * mib);
    }

    #[test]
    fn compact_estimate_scales_with_wires_and_samples_including_on_wasm32() {
        let mut state = AppState::new();
        for qubits in [22, 50, 64, 65, 128] {
            state.circuit_state.circuit.num_qubits = qubits;
            let peak = estimated_peak_bytes(&state).unwrap();
            assert!((13_192..1024 * 1024).contains(&peak));
        }
        for qubits in [0, 129, usize::MAX] {
            state.circuit_state.circuit.num_qubits = qubits;
            assert_eq!(estimated_peak_bytes(&state), None);
        }
    }

    #[test]
    fn wide_estimate_includes_group_storage_on_first_run_and_rerun() {
        let mut app = QCSimApp::default();
        // A controlled quarter turn on two |+⟩ wires needs the grouped readout.
        app.handle_message(Message::UpdateCircuitFromJson(
            r#"{"num_qubits":64,"steps":[[{"id":"H","qubits":[0]},{"id":"H","qubits":[63]}],[{"id":"CZPow","qubits":[0,63],"params":[0.25]}]]}"#.into(),
        ));
        let before = estimated_peak_bytes(&app.state).unwrap();
        assert!(u128::from(before) >= compact_state_bytes(64));
        app.handle_message(Message::RunSimulation);
        assert!(app.state.ui_state.error_message.is_none());
        let after = estimated_peak_bytes(&app.state).unwrap();
        assert!(u128::from(after - before) >= compact_state_bytes(64));
    }
}
