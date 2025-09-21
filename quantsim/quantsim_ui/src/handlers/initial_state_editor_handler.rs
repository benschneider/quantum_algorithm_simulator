use crate::{messages::Message, state::AppState};

pub fn handle_initial_state_editor_message(
    state: &mut AppState,
    message: Message,
    _messages: &mut Vec<Message>,
) {
    match message {
        Message::ToggleInitialStateEditor => {
            state.ui_state.show_initial_state_editor = !state.ui_state.show_initial_state_editor;
            if state.ui_state.show_initial_state_editor {
                state.initial_state_editor_state = crate::state::InitialStateEditorState::new(
                    state.circuit_state.circuit.num_qubits,
                );
                if let Some(initial_state) = &state.circuit_state.circuit.initial_state {
                    state.initial_state_editor_state.state_vector = initial_state
                        .iter()
                        .map(|c| nalgebra::Complex::new(c.re, c.im))
                        .collect();
                }
            }
        }
        Message::ApplyInitialStateFromEditor(state_vector) => {
            let expected_len = 1 << state.circuit_state.circuit.num_qubits;
            if state_vector.len() == expected_len {
                state.circuit_state.circuit.initial_state = Some(
                    state_vector
                        .iter()
                        .map(|c| nalgebra::Complex::new(c.re as f32, c.im as f32))
                        .collect(),
                );
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
            state.circuit_state.circuit.initial_state = None;
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
