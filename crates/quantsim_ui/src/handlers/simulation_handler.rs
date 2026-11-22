use crate::{
    messages::Message,
    state::{AppState, PaginationState},
};
use quantsim_core::core::circuit::{Circuit, RunOptions};
use quantsim_core::core::engine::QuantumState;

pub fn handle_simulation_message(
    state: &mut AppState,
    message: Message,
    _messages: &mut [Message],
) {
    let timestep = match message {
        Message::RunSimulation => {
            state.ui_state.show_results_window = true;
            None
        }
        Message::SelectTimestep(timestep) => Some(timestep),
        Message::SimulationResultsPageChanged(page_number) => {
            state.simulation_state.pagination.set_page(page_number);
            state.simulation_state.page_input_text =
                state.simulation_state.pagination.current_page.to_string();
            return;
        }
        _ => return,
    };

    let circuit = &state.circuit_state.circuit;
    let step_count = timestep
        .unwrap_or(circuit.steps.len())
        .min(circuit.steps.len());
    match simulate(circuit, step_count) {
        Ok(quantum_state) => {
            let simulation = &mut state.simulation_state;
            if timestep.is_none() || simulation.quantum_state.is_none() {
                simulation.pagination = PaginationState::new(
                    quantum_state.state_vector.len(),
                    simulation.pagination.entries_per_page,
                );
                simulation.page_input_text = simulation.pagination.current_page.to_string();
            }
            simulation.quantum_state = Some(quantum_state);
            state.ui_state.current_timestep = step_count;
            state.ui_state.error_message = None;
        }
        Err(error) => {
            state.simulation_state = Default::default();
            state.ui_state.current_timestep = 0;
            state.ui_state.error_message = Some(format!("Simulation failed: {error}"));
        }
    }
}

fn simulate(circuit: &Circuit, step_count: usize) -> Result<QuantumState, String> {
    let result = circuit.run_prefix(step_count, &RunOptions::default());
    if let Some(error) = result.diagnostics.into_iter().next() {
        return Err(error.message);
    }

    let state_vector = result
        .final_state_vector
        .as_deref()
        .ok_or("No state vector was produced.")?;
    QuantumState::from_vector(circuit.num_qubits, state_vector).map_err(str::to_owned)
}
