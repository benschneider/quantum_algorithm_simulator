use crate::{messages::Message, state::AppState};

pub fn handle_circuit_controls_message(
    state: &mut AppState,
    message: Message,
    _messages: &mut Vec<Message>,
) {
    match message {
        Message::ChangeQubits(new_qubits) => {
            state.circuit_state.num_qubits = new_qubits;
            state.circuit_state.circuit.num_qubits = new_qubits;
            state
                .circuit_state
                .circuit
                .steps
                .resize(state.circuit_state.num_timesteps, Vec::new());
        }
        Message::ChangeTimesteps(new_timesteps) => {
            state.circuit_state.num_timesteps = new_timesteps;
            state
                .circuit_state
                .circuit
                .steps
                .resize(new_timesteps, Vec::new());
        }
        Message::CircuitDimensionsChanged {
            num_qubits,
            num_timesteps,
        } => {
            state.circuit_state.num_qubits = num_qubits;
            state.circuit_state.num_timesteps = num_timesteps;
            state.circuit_state.circuit.num_qubits = num_qubits;
            // The steps are already loaded by `update_from_data`, so we should not resize them.
            state.circuit_state.circuit.steps.resize(num_timesteps, Vec::new());
            state.circuit_state.circuit.initial_preps.resize(num_qubits, None);
            state.initial_state_editor_state =
                crate::state::InitialStateEditorState::new(num_qubits);
        }
        _ => {}
    }
}
