use crate::{
    messages::Message,
    state::{AppState, CircuitState},
};

pub fn handle_initial_state_editor_message(
    state: &mut AppState,
    message: Message,
    _messages: &mut Vec<Message>,
) {
    match message {
        Message::ToggleInitialStateEditor => {
            state.ui_state.show_initial_state_editor = !state.ui_state.show_initial_state_editor;
            if state.ui_state.show_initial_state_editor {
                state.initial_state_editor_state =
                    crate::state::InitialStateEditorState::from_circuit(
                        &state.circuit_state.circuit,
                    );
            }
        }
        Message::ApplyInitialStateFromEditor(state_vector) => {
            let Some(expected_len) =
                CircuitState::dense_amplitude_count(state.circuit_state.circuit.num_qubits)
            else {
                state.ui_state.error_message = Some(
                    "Set wide-circuit input wires with initial_preps in the circuit JSON.".into(),
                );
                return;
            };
            if state_vector.len() == expected_len {
                state.circuit_state.circuit.initial_state = Some(
                    state_vector
                        .iter()
                        .map(|c| nalgebra::Complex::new(c.re as f32, c.im as f32))
                        .collect(),
                );
                state.simulation_state = Default::default();
            } else {
                log::error!(
                    "Invalid state vector length. Expected {}, got {}.",
                    expected_len,
                    state_vector.len()
                );
            }
            state.ui_state.show_initial_state_editor = false;
        }
        Message::ResetInitialState => {
            if state.circuit_state.circuit.initial_state.take().is_some() {
                state.simulation_state = Default::default();
            }
            state.initial_state_editor_state =
                crate::state::InitialStateEditorState::new(state.circuit_state.circuit.num_qubits);
            state.ui_state.show_initial_state_editor = false;
        }
        Message::InitialStateEditorPageChanged(page_number) => {
            state
                .initial_state_editor_state
                .pagination
                .set_page(page_number);
            state.initial_state_editor_state.page_input_text = state
                .initial_state_editor_state
                .pagination
                .current_page
                .to_string();
        }
        _ => {}
    }
}
