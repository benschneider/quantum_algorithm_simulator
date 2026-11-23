use crate::{messages::Message, state::AppState};
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
            if step == state.ui_state.current_timestep
                && state.simulation_state.quantum_state.is_some()
            {
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
    state.ui_state.current_timestep = step_count;
    match simulate(circuit, step_count) {
        Ok((quantum_state, probabilities)) => {
            state
                .simulation_state
                .set_readout(quantum_state, &probabilities, timestep.is_none());
            state.ui_state.error_message = None;
        }
        Err(error) => {
            state.simulation_state = Default::default();
            state.ui_state.error_message = Some(format!("Simulation failed: {error}"));
        }
    }
}

fn simulate(circuit: &Circuit, step_count: usize) -> Result<(QuantumState, Vec<f32>), String> {
    let result = circuit.run_prefix(step_count, &RunOptions::default());
    if let Some(error) = result.diagnostics.into_iter().next() {
        return Err(error.message);
    }

    let state_vector = result
        .final_state_vector
        .ok_or("No state vector was produced.")?;
    let state = QuantumState::from_owned_vector(circuit.num_qubits, state_vector)?;
    Ok((state, result.final_probabilities))
}

#[cfg(test)]
mod tests {
    use crate::{app::QCSimApp, messages::Message};
    use quantsim_core::{circuit::QubitRole, core::types::Gate};

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
