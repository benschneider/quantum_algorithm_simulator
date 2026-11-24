use crate::{
    messages::Message,
    state::{AppState, CircuitState, SimulationState},
};
use quantsim_core::core::circuit::{Circuit, RunOptions};
use quantsim_core::core::engine::QuantumState;

pub(super) fn invalidate_edited_gate(state: &mut AppState, column: usize) {
    if column < state.ui_state.current_timestep {
        state.simulation_state = Default::default();
    }
}

pub fn handle_simulation_message(
    state: &mut AppState,
    message: Message,
    _messages: &mut [Message],
) {
    let timestep = match message {
        Message::RunSimulation => None,
        Message::SelectTimestep(timestep) => Some(timestep),
        Message::InspectLayoutSite { wire, step } => {
            let circuit = &state.circuit_state.circuit;
            if wire >= circuit.num_qubits || step > circuit.steps.len() {
                return;
            }
            state.ui_state.selected_layout_wire = Some(wire);
            if step == state.ui_state.current_timestep && state.simulation_state.has_readout() {
                state.ui_state.show_results_window = true;
                return;
            }
            Some(step)
        }
        Message::SimulationResultsPageChanged(page_number) => {
            state.simulation_state.pagination.set_page(page_number);
            state.simulation_state.page_input_text =
                state.simulation_state.pagination.current_page.to_string();
            return;
        }
        _ => return,
    };

    state.ui_state.show_results_window = true;
    let circuit = &state.circuit_state.circuit;
    let step_count = timestep
        .unwrap_or(circuit.steps.len())
        .min(circuit.steps.len());
    if timestep.is_some()
        && step_count == state.ui_state.current_timestep
        && state.simulation_state.compact_samples.is_some()
    {
        state.ui_state.error_message = None;
        return;
    }
    state.ui_state.current_timestep = step_count;
    match simulate(
        circuit,
        step_count,
        &mut state.simulation_state,
        timestep.is_none(),
    ) {
        Ok(()) => state.ui_state.error_message = None,
        Err(error) => {
            state.simulation_state = Default::default();
            state.ui_state.error_message = Some(format!("Simulation failed: {error}"));
        }
    }
}

fn simulate(
    circuit: &Circuit,
    step_count: usize,
    readout: &mut SimulationState,
    reset_page: bool,
) -> Result<(), String> {
    if circuit.num_qubits > CircuitState::MAX_DENSE_QUBITS {
        readout.replace_compact_readout(compact_readout(circuit, step_count)?);
        return Ok(());
    }
    let result = circuit.run_prefix(step_count, &RunOptions::default());
    if let Some(error) = result.diagnostics.into_iter().next() {
        return Err(error.message);
    }

    let state_vector = result
        .final_state_vector
        .ok_or("No state vector was produced.")?;
    let state = QuantumState::from_owned_vector(circuit.num_qubits, state_vector)?;
    readout.set_readout(state, &result.final_probabilities, reset_page);
    Ok(())
}

fn compact_readout(circuit: &Circuit, step_count: usize) -> Result<SimulationState, String> {
    let stabilizer_error = match circuit.run_stabilizer_prefix(step_count) {
        Ok(state) => return Ok(SimulationState::from_stabilizer(&state, circuit.num_qubits)),
        Err(error) => error,
    };
    let product_error = match circuit.run_product_prefix(step_count) {
        Ok(state) => return Ok(SimulationState::from_product(state, circuit.num_qubits)),
        Err(error) => error,
    };
    let grouped_error = match circuit.run_grouped_prefix(step_count) {
        Ok(state) => return Ok(SimulationState::from_grouped(state, circuit.num_qubits)),
        Err(error) => error,
    };
    // Prefer the grouped diagnostic on ties; it explains the group-size limit.
    let error = [stabilizer_error, product_error, grouped_error]
        .into_iter()
        .max_by_key(|e| (e.step, e.op_index))
        .expect("three backend diagnostics");
    // Input diagnostics share (0, 0) with the first authored gate.
    if error.step == 0 && error.op_index == 0 && circuit.run_stabilizer_prefix(0).is_err() {
        return Err(error.message);
    }
    Err(format!(
        "Step {}, gate {}: {}",
        error.step + 1,
        error.op_index + 1,
        error.message
    ))
}

#[cfg(test)]
mod grouped_tests;

#[cfg(test)]
mod tests {
    use crate::state::simulation_state::{COMPACT_SHOTS, MAX_SAMPLE_ROWS};
    use crate::{app::QCSimApp, messages::Message};
    use quantsim_core::{circuit::QubitRole, core::types::Gate};
    use serde_json::json;

    #[test]
    fn wide_ghz_samples_only_correlated_outcomes_at_the_selected_step() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::LoadTemplate("ghz_50.json".to_owned()));
        assert!(app.state.ui_state.error_message.is_none());
        for (step, entangled_wires) in [(6, 32), (7, 50)] {
            app.handle_message(Message::SelectTimestep(step));
            assert!(app.state.ui_state.error_message.is_none());
            let readout = &app.state.simulation_state;
            let samples = readout.compact_samples.as_ref().unwrap();
            assert_eq!(samples.distinct, 2);
            let ones = (1_u128 << entangled_wires) - 1;
            assert!(samples.rows.iter().any(|&(bits, _)| bits == 0));
            assert!(samples.rows.iter().any(|&(bits, _)| bits == ones));
            assert_eq!(samples.rows.iter().map(|r| r.1).sum::<u32>(), COMPACT_SHOTS);
            assert!(readout.quantum_state.is_none());
            assert!(readout.populated_indices.is_empty());
            assert_eq!(readout.wire_one_probabilities.len(), 50);
            assert!(readout.wire_one_probabilities[..entangled_wires]
                .iter()
                .all(|&p| p == 0.5));
            assert!(readout.wire_one_probabilities[entangled_wires..]
                .iter()
                .all(|&p| p == 0.0));
        }
    }

    #[test]
    fn a_clifford_prefix_beyond_group_capacity_reports_the_first_unsupported_gate() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::LoadTemplate("ghz_50.json".into()));
        let last = app.state.circuit_state.num_timesteps;
        app.handle_message(Message::ChangeTimesteps(last + 1));
        app.handle_message(Message::PlaceGate(Gate::ZPow, 0, last));
        app.handle_message(Message::SelectGateForEditing(0, last));
        app.handle_message(Message::UpdateGateAngle(0.25));
        app.handle_message(Message::RunSimulation);
        let error = app.state.ui_state.error_message.as_ref().unwrap();
        assert!(
            error.contains(&format!("Step {}, gate 1", last + 1)),
            "{error}"
        );
        assert!(error.contains("ZPow"), "{error}");
    }

    #[test]
    fn exceeding_group_capacity_preserves_readable_prefixes_and_bounded_rows() {
        use quantsim_core::grouped::MAX_GROUP_QUBITS;
        let independent: Vec<_> = (0..50).map(|q| json!({"id":"H","qubits":[q]})).collect();
        let mut columns = vec![independent];
        for wire in 1..=MAX_GROUP_QUBITS {
            columns.push(vec![json!({"id":"CZPow","qubits":[0,wire],"params":[0.3]})]);
        }
        let mut app = QCSimApp::default();
        app.handle_message(Message::UpdateCircuitFromJson(
            json!({"num_qubits":50,"steps":columns}).to_string(),
        ));
        app.handle_message(Message::RunSimulation);
        let error = app.state.ui_state.error_message.as_ref().unwrap();
        assert!(
            error.contains(&format!("Step {}, gate 1", MAX_GROUP_QUBITS + 1))
                && error.contains("CZPow")
                && error.contains(&format!("{}-qubit group", MAX_GROUP_QUBITS + 1))
                && error.contains(&format!("limit is {MAX_GROUP_QUBITS}")),
            "{error}"
        );
        assert!(!app.state.simulation_state.has_readout());

        app.handle_message(Message::SelectTimestep(MAX_GROUP_QUBITS));
        assert!(app.state.ui_state.error_message.is_none());
        let readout = &app.state.simulation_state;
        assert!(readout
            .wire_one_probabilities
            .iter()
            .all(|&p| (p - 0.5).abs() < 1e-6));
        let samples = readout.compact_samples.as_ref().unwrap();
        assert!(samples.distinct > MAX_SAMPLE_ROWS && samples.distinct <= COMPACT_SHOTS as usize);
        assert_eq!(samples.rows.len(), MAX_SAMPLE_ROWS);
        assert!(samples
            .rows
            .is_sorted_by_key(|&(bits, count)| (std::cmp::Reverse(count), bits)));
        assert!(samples.rows.iter().map(|r| r.1).sum::<u32>() < COMPACT_SHOTS);

        app.handle_message(Message::DeleteGate(0, 0));
        assert!(!app.state.simulation_state.has_readout());
    }

    #[test]
    fn wide_fourier_template_prepares_a_delta_then_shows_two_periods() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::LoadTemplate("qft_50.json".into()));
        for (step, input) in [(0, 0), (1, 1u128 << 45)] {
            app.handle_message(Message::SelectTimestep(step));
            assert_eq!(app.state.ui_state.error_message, None);
            let readout = &app.state.simulation_state;
            assert_eq!(
                readout.compact_samples.as_ref().unwrap().rows,
                [(input, COMPACT_SHOTS)]
            );
            let wave = readout.waveform().unwrap();
            assert_eq!((wave.first, wave.stride), (input, 1));
            assert_eq!(wave.points[0], nalgebra::Complex::ONE);
            assert!(wave.points[1..]
                .iter()
                .all(|&z| z == nalgebra::Complex::ZERO));
        }
        app.handle_message(Message::RunSimulation);
        assert_eq!(app.state.ui_state.error_message, None);
        assert_eq!(app.state.ui_state.current_timestep, 101);
        let wave = app.state.simulation_state.waveform().unwrap();
        assert_eq!((wave.first, wave.stride, wave.points.len()), (0, 1, 64));
        let amplitude = 2.0_f64.powi(-25);
        for (index, point) in wave.points.iter().enumerate() {
            let angle = std::f64::consts::TAU * index as f64 / 32.0;
            let expected = nalgebra::Complex::from_polar(amplitude, angle);
            assert!(
                (*point - expected).norm() < amplitude * 1e-6,
                "index {index}"
            );
        }
    }

    #[test]
    fn wide_fourier_phase_edit_flows_into_an_entangling_gate() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::LoadTemplate("qft_50.json".into()));
        let end = app.state.circuit_state.num_timesteps;
        app.handle_message(Message::ChangeTimesteps(end + 2));
        app.handle_message(Message::PlaceGate(Gate::H, 0, end));
        app.handle_message(Message::RunSimulation);
        assert_eq!(app.state.ui_state.error_message, None);
        let readout = &app.state.simulation_state;
        let expected = (std::f64::consts::PI / 32.0).sin().powi(2);
        assert!((readout.wire_one_probabilities[0] - expected).abs() < 1e-6);

        app.handle_message(Message::SelectGateForEditing(45, 5));
        app.handle_message(Message::UpdateGateAngle(1.0 / 3.0));
        assert!(!app.state.simulation_state.has_readout());
        app.handle_message(Message::SelectTimestep(end + 2));
        assert_eq!(app.state.ui_state.error_message, None);
        let readout = &app.state.simulation_state;
        assert!((readout.wire_one_probabilities[0] - 0.25).abs() < 1e-6);
        assert!(readout.wire_one_probabilities[1..]
            .iter()
            .all(|&p| (p - 0.5).abs() < 1e-6));
        assert!(readout.quantum_state.is_none() && readout.populated_indices.is_empty());
        let samples = readout.compact_samples.as_ref().unwrap();
        assert_eq!(samples.rows.len(), MAX_SAMPLE_ROWS);
        assert!(samples.distinct > MAX_SAMPLE_ROWS && samples.distinct <= COMPACT_SHOTS as usize);

        app.handle_message(Message::PlaceMultiQubitGate(Gate::CX, vec![49, 0], end + 1));
        app.handle_message(Message::RunSimulation);
        assert_eq!(app.state.ui_state.error_message, None);
        assert!((app.state.simulation_state.wire_one_probabilities[0] - 0.5).abs() < 1e-6);
        app.handle_message(Message::SelectTimestep(end + 1));
        assert_eq!(app.state.ui_state.error_message, None);
        assert!((app.state.simulation_state.wire_one_probabilities[0] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn selected_prefix_reports_every_wire_and_high_index_joint_states() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::ChangeQubits(5));
        for wire in [3, 4] {
            app.handle_message(Message::PlaceGate(Gate::X, wire, 0));
        }
        app.handle_message(Message::PlaceGate(Gate::H, 1, 0));
        app.handle_message(Message::PlaceGate(Gate::Z, 1, 1));
        app.handle_message(Message::SelectTimestep(2));

        assert!(app.state.ui_state.show_results_window);
        let readout = &app.state.simulation_state;
        assert_eq!(readout.populated_indices, [24, 26]);
        for (actual, expected) in readout
            .wire_one_probabilities
            .iter()
            .zip([0.0, 0.5, 0.0, 1.0, 1.0])
        {
            assert!((actual - expected).abs() < 1e-6);
        }
        assert_eq!(readout.wire_one_probabilities.len(), 5);
        let amplitudes = &readout.quantum_state.as_ref().unwrap().state_vector;
        assert!(amplitudes[24].re > 0.7 && amplitudes[26].re < -0.7);

        app.handle_message(Message::SelectTimestep(0));
        assert_eq!(app.state.simulation_state.populated_indices, [0]);
        assert_eq!(app.state.simulation_state.wire_one_probabilities, [0.0; 5]);
    }

    #[test]
    fn readout_stays_valid_until_the_selected_prefix_or_input_changes() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::SelectTimestep(1));
        app.handle_message(Message::SetQubitRole {
            row: 0,
            role: Some(QubitRole::Data),
        });
        app.handle_message(Message::PlaceGate(Gate::X, 0, 1));
        assert_eq!(app.state.simulation_state.populated_indices, [0]);

        app.handle_message(Message::MoveGate(0, 1, 0, 0));
        assert!(app.state.simulation_state.quantum_state.is_none());
        assert_eq!(app.state.ui_state.current_timestep, 1);
        app.handle_message(Message::SelectTimestep(1));
        assert_eq!(app.state.simulation_state.populated_indices, [1]);

        app.handle_message(Message::PlaceGate(Gate::X, 0, 0)); // Occupied.
        assert!(app.state.simulation_state.quantum_state.is_some());
        app.handle_message(Message::DeleteGate(0, 0));
        assert!(app.state.simulation_state.quantum_state.is_none());

        app.handle_message(Message::SelectTimestep(0));
        let mut input = vec![nalgebra::Complex::new(0.0, 0.0); 4];
        input[2].im = 1.0;
        app.handle_message(Message::ApplyInitialStateFromEditor(input));
        assert!(app.state.simulation_state.quantum_state.is_none());
        app.handle_message(Message::SelectTimestep(0));
        assert_eq!(
            app.state.simulation_state.wire_one_probabilities,
            [0.0, 1.0]
        );
        app.handle_message(Message::ResetInitialState);
        assert!(app.state.simulation_state.quantum_state.is_none());
    }
}
