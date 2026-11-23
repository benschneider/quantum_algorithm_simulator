use crate::{messages::Message, state::AppState};

pub fn handle_gate_palette_message(
    state: &mut AppState,
    message: Message,
    _messages: &mut Vec<Message>,
) {
    match message {
        Message::SelectGate(gate_id) => {
            if state.ui_state.selected_gate == Some(gate_id.clone()) {
                state.ui_state.selected_gate = None;
                state.ui_state.placement_mode = crate::state::ui_state::PlacementMode::Idle;
            } else {
                state.ui_state.selected_gate = Some(gate_id.clone());
                state.ui_state.placement_mode = crate::state::ui_state::PlacementMode::Placing;
            }
        }
        Message::OpenGateEditor(gate_id) => {
            state.ui_state.selected_gate_for_editing = None;
            state.ui_state.palette_gate_for_editing = Some(gate_id);
            state.ui_state.show_gate_editor_window = true;
        }
        Message::OpenCustomGateEditor(quantsim_core::core::types::Gate::Custom) => {
            let editor = &mut state.custom_gate_editor_state;
            editor.matrix_str = state
                .circuit_state
                .palette_custom_matrix
                .map(|z| (z.re.to_string(), z.im.to_string()));
            editor.error_message = None;
            editor.is_open = true;
        }
        Message::UpdateCustomGateEditorValue {
            row,
            col,
            real,
            imag,
        } => {
            let editor = &mut state.custom_gate_editor_state;
            if let Some(cell) = editor.matrix_str.get_mut((row, col)) {
                *cell = (real, imag);
                editor.error_message = None;
            }
        }
        Message::MakeCustomGateMatrixUnitary => {
            state.custom_gate_editor_state.make_unitary();
        }
        Message::SaveCustomGateMatrix if state.custom_gate_editor_state.is_open => {
            let editor = &mut state.custom_gate_editor_state;
            match editor.parse_and_validate_matrix() {
                Ok(matrix) => {
                    state.circuit_state.palette_custom_matrix = matrix;
                    *editor = Default::default();
                }
                Err(error) => editor.error_message = Some(error),
            }
        }
        Message::CloseCustomGateEditor => {
            state.custom_gate_editor_state = crate::state::CustomGateEditorState::default();
        }
        _ => {}
    }
}
