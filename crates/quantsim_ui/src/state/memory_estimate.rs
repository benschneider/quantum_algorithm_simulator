use super::simulation_state::{COMPACT_SHOTS, MAX_SAMPLE_ROWS, WAVEFORM_POINTS};
use super::{AppState, CircuitState};
use nalgebra::Complex;
use quantsim_core::circuit::Circuit;
use quantsim_core::types::{GateMatrix, Operation};
use quantsim_core::BasisIndex;
use std::mem::size_of;

mod groups;

const AMPLITUDE: u128 = size_of::<Complex<f32>>() as u128;
const INDEX: u128 = size_of::<usize>() as u128;
const COMPLEX64: u128 = size_of::<Complex<f64>>() as u128;
const SATURATED: u128 = u64::MAX as u128;

/// A conservative allowance for the major buffers of a full UI Run.
/// Work in wide byte counts on wasm32 too. `Some(u64::MAX)` also represents
/// estimates beyond the displayable byte count. `None` means unknown.
pub(crate) fn estimated_peak_bytes(state: &AppState) -> Option<u64> {
    let circuit = &state.circuit_state.circuit;
    let readout = &state.simulation_state;
    if circuit.num_qubits == 0 {
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
        let wires = readout.wire_one_probabilities.len();
        bytes += samples.rows.capacity() as u128
            * (size_of::<(BasisIndex, u32)>() as u128 + digits(wires));
        // The previous state still exists while the next run is being built, including
        // after a gate edit. Use its stored factors rather than the new circuit's links.
        bytes += match readout.retained_group_amplitudes() {
            Some(amplitudes) => amplitudes as u128 * COMPLEX64 + wires as u128 * 128,
            None if readout.retained_stabilizer() => tableau_bytes(wires, 1),
            None => wires as u128 * 64, // One product-state amplitude pair plus metadata.
        };
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
        bytes += wide_run_bytes(circuit);
        return Some(bytes.min(SATURATED) as u64);
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
    Some(bytes.min(SATURATED) as u64)
}

// Two times the packed limb count allows for Vec capacity growth in a BigUint.
fn digits(wires: usize) -> u128 {
    2 * wires.div_ceil(64).max(2) as u128 * 8
}

fn tableau_bytes(wires: usize, rows_per_wire: u128) -> u128 {
    let row = 2 * (size_of::<BasisIndex>() as u128 + digits(wires)) + INDEX;
    (wires as u128)
        .saturating_mul(rows_per_wire * row)
        .min(SATURATED)
}

fn wide_run_bytes(circuit: &Circuit) -> u128 {
    let wires = circuit.num_qubits;
    let packed = size_of::<BasisIndex>() as u128 + digits(wires);
    let row = size_of::<(BasisIndex, u32)>() as u128 + digits(wires);
    let shots = u128::from(COMPACT_SHOTS);
    // Packed outcomes, growing counts, temporary grouped sample draws, and displayed rows.
    let samples = shots * (packed + 2 * row + 32) + MAX_SAMPLE_ROWS as u128 * row;
    let authored: u128 = circuit
        .steps
        .iter()
        .map(|ops| {
            ops.capacity() as u128 * size_of::<Operation>() as u128
                + ops
                    .iter()
                    .filter_map(|op| op.matrix.as_ref())
                    .map(|m| match m {
                        GateMatrix::BigEndian(m) => m.data.as_vec().capacity() as u128 * AMPLITUDE,
                        GateMatrix::LittleEndian(m) => {
                            m.nnz() as u128 * (AMPLITUDE + INDEX) + (m.nrows() as u128 + 1) * INDEX
                        }
                    })
                    .sum::<u128>()
        })
        .sum();
    // Unsupported gate definitions are rejected before the tableau is allocated.
    let evolution = if circuit
        .validate_stabilizer_prefix(circuit.steps.len())
        .is_ok()
    {
        // Two rows per wire plus elimination/sampling scratch and the amplitude index.
        tableau_bytes(wires, 4) + wires as u128 * (packed + INDEX)
    } else {
        let amplitudes = groups::amplitude_bound(circuit);
        // Merges and rank-one splits can overlap the input and output vectors.
        // Two full sets of the final upper-bound partition cover these overlaps.
        2 * amplitudes * COMPLEX64 + wires as u128 * 128 + 8 * 8 * (AMPLITUDE + COMPLEX64)
    };
    authored + evolution + samples
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
        for qubits in [22, 50, 64, 65, 128, 129, 256] {
            state.circuit_state.circuit.num_qubits = qubits;
            let peak = estimated_peak_bytes(&state).unwrap();
            assert!((13_192..1024 * 1024).contains(&peak));
        }
        state.circuit_state.circuit.num_qubits = 0;
        assert_eq!(estimated_peak_bytes(&state), None);
        #[cfg(target_pointer_width = "64")]
        {
            state.circuit_state.circuit.num_qubits = usize::MAX;
            assert_eq!(estimated_peak_bytes(&state), Some(u64::MAX));
        }
    }

    #[test]
    fn same_width_estimate_follows_the_clifford_or_grouped_execution_path() {
        let mut state = AppState::new();
        let mut circuit = Circuit::new(35);
        circuit
            .steps
            .push(vec![Operation::new(Gate::H, vec![0], vec![])]);
        for wire in 1..35 {
            circuit
                .steps
                .push(vec![Operation::new(Gate::CX, vec![wire - 1, wire], vec![])]);
        }
        state.circuit_state.circuit = circuit;
        assert!(estimated_peak_bytes(&state).unwrap() < 1024 * 1024);
        // A non-Clifford phase requires replay via the grouped simulator. The
        // GHZ factor now grows across the same 35 wires instead of using a tableau.
        let circuit = &mut state.circuit_state.circuit;
        circuit
            .steps
            .push(vec![Operation::new(Gate::ZPow, vec![0], vec![0.25])]);
        assert!(estimated_peak_bytes(&state).unwrap() >= (1u64 << 35) * 16);
    }

    #[test]
    fn sparse_groups_follow_swaps_and_qft_basis_controls() {
        let op = |gate, wires: &[u32]| Operation::new(gate, wires.to_vec(), vec![]);
        let mut circuit = Circuit::new(256);
        circuit.initial_preps[0] = Some(quantsim_core::types::QubitPrep::Plus);
        circuit.steps = vec![
            vec![op(Gate::CX, &[0, 200])],
            vec![op(Gate::SWAP, &[200, 201])],
            vec![op(Gate::CX, &[201, 202])],
        ];
        assert_eq!(groups::amplitude_bound(&circuit), 253 * 2 + 8);
        let qft = quantsim_core::circuits::load_template("fourier_wave_50").unwrap();
        assert_eq!(groups::amplitude_bound(&qft), 2 * 50);
    }

    #[test]
    fn disconnected_phase_pairs_do_not_grow_into_one_factor_and_huge_estimates_saturate() {
        use quantsim_core::types::QubitPrep;
        let mut state = AppState::new();
        let mut circuit = Circuit::new(80);
        circuit.initial_preps.fill(Some(QubitPrep::Plus));
        let phase = |a, b| Operation::new(Gate::CZPow, vec![a, b], vec![0.25]);
        circuit
            .steps
            .push((0..80).step_by(2).map(|i| phase(i, i + 1)).collect());
        assert_eq!(groups::amplitude_bound(&circuit), 40 * 4);
        circuit
            .steps
            .push((1..79).step_by(2).map(|i| phase(i, i + 1)).collect());
        state.circuit_state.circuit = circuit;
        assert_eq!(estimated_peak_bytes(&state), Some(u64::MAX));
    }

    #[test]
    fn wide_estimate_includes_group_storage_on_first_run_and_rerun() {
        let mut app = QCSimApp::default();
        // A controlled quarter turn on two |+⟩ wires needs the grouped readout.
        app.handle_message(Message::UpdateCircuitFromJson(
            r#"{"num_qubits":64,"steps":[[{"id":"H","qubits":[0]},{"id":"H","qubits":[63]}],[{"id":"CZPow","qubits":[0,63],"params":[0.25]}]]}"#.into(),
        ));
        let before = estimated_peak_bytes(&app.state).unwrap();
        assert!(before > 0);
        app.handle_message(Message::RunSimulation);
        assert!(app.state.ui_state.error_message.is_none());
        let after = estimated_peak_bytes(&app.state).unwrap();
        assert!(after >= before + 64 * size_of::<Complex<f64>>() as u64);
    }
}
