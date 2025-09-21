use crate::{messages::Message, state::AppState};
use quantsim_core::core::engine::QuantumState;

pub fn handle_simulation_message(
    state: &mut AppState,
    message: Message,
    _messages: &mut [Message],
) {
    match message {
        Message::RunSimulation => {
            state.ui_state.show_results_window = true;
            let circuit_to_run = state.circuit_state.circuit.clone();
            let result = circuit_to_run.run(&quantsim_core::core::circuit::RunOptions::default());
            let quantum_state =
                QuantumState::from_vector(circuit_to_run.num_qubits, &result.final_state_vector.unwrap())
                    .unwrap();
            state.simulation_state.quantum_state = Some(quantum_state);
            if let Some(qs) = &state.simulation_state.quantum_state {
                state.simulation_state.pagination.total_entries = qs.state_vector.len();
                state.simulation_state.pagination.total_pages =
                    (state.simulation_state.pagination.total_entries as f32
                        / state.simulation_state.pagination.entries_per_page as f32)
                        .ceil() as usize;
                state.simulation_state.pagination.set_page(1);
                state.simulation_state.page_input_text =
                    state.simulation_state.pagination.current_page.to_string();
            }
            state.ui_state.current_timestep = state.circuit_state.num_timesteps;
        }
        Message::SelectTimestep(timestep) => {
            state.ui_state.current_timestep = timestep;
            let circuit_to_run = state.circuit_state.circuit.clone();
            // We can't easily run just part of the circuit anymore, so we'll just
            // show the final state for now. A more sophisticated implementation
            // could take snapshots.
            let result = circuit_to_run.run(&quantsim_core::core::circuit::RunOptions {
                snapshot_state_per_step: true,
                ..Default::default()
            });

            let state_to_show = if let Some(snapshot) = result.snapshots.get(timestep.saturating_sub(1)) {
                snapshot.state.clone().unwrap()
            } else if timestep == 0 {
                QuantumState::new(circuit_to_run.num_qubits).state_vector.as_slice().to_vec()
            } else {
                result.final_state_vector.unwrap()
            };

            let quantum_state =
                QuantumState::from_vector(circuit_to_run.num_qubits, &state_to_show).unwrap();

            state.simulation_state.quantum_state = Some(quantum_state);
        }
        Message::SimulationResultsPageChanged(page_number) => {
            state.simulation_state.pagination.set_page(page_number);
            state.simulation_state.page_input_text =
                state.simulation_state.pagination.current_page.to_string();
        }
        _ => {
            //log::warn!("Unhandled message in simulation handler: {:?}", message);
        }
    }
}
