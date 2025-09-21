use crate::{messages::Message, state::AppState};

pub fn handle_gate_palette_message(
    state: &mut AppState,
    message: Message,
    messages: &mut Vec<Message>,
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
            state.ui_state.palette_gate_for_editing = Some(gate_id.clone());
            state.ui_state.show_gate_editor_window = true;
        }
        Message::OpenCustomGateEditor(gate_id) => {
            let gate = state.circuit_state.circuit.registry.get_meta(&gate_id);
            if let Some(_gate) = gate {
                let matrix = state
                    .circuit_state
                    .circuit
                    .registry
                    .eval(&gate_id, &[], &[])
                    .unwrap();
                let matrix_d = matrix.to_dmatrix();
                let (rows, cols) = matrix_d.shape();
                let mut matrix_str =
                    nalgebra::DMatrix::from_element(rows, cols, (String::new(), String::new()));
                for r in 0..rows {
                    for c in 0..cols {
                        matrix_str[(r, c)] = (
                            matrix_d[(r, c)].re.to_string(),
                            matrix_d[(r, c)].im.to_string(),
                        );
                    }
                }

                state.custom_gate_editor_state.gate_id = Some(gate_id.clone());
                state.custom_gate_editor_state.matrix_str = matrix_str;
                state.custom_gate_editor_state.error_message = None;
                state.custom_gate_editor_state.is_open = true;
            }
        }
        Message::UpdateCustomGateEditorValue {
            row,
            col,
            real,
            imag,
        } => {
            state.custom_gate_editor_state.matrix_str[(row, col)] = (real, imag);
        }
        Message::SaveCustomGateMatrix => {
            let editor_state = &mut state.custom_gate_editor_state;
            if let Some(_gate_id) = &editor_state.gate_id {
                let (rows, cols) = editor_state.matrix_str.shape();
                let mut new_matrix =
                    nalgebra::DMatrix::from_element(rows, cols, nalgebra::Complex::new(0.0, 0.0));
                let mut parse_error = false;
                for r in 0..rows {
                    for c in 0..cols {
                        let (real_str, imag_str) = &editor_state.matrix_str[(r, c)];
                        let real = real_str.parse::<f32>();
                        let imag = imag_str.parse::<f32>();
                        if real.is_err() || imag.is_err() {
                            parse_error = true;
                            break;
                        }
                        new_matrix[(r, c)] = nalgebra::Complex::new(real.unwrap(), imag.unwrap());
                    }
                    if parse_error {
                        break;
                    }
                }

                if parse_error {
                    editor_state.error_message = Some("Invalid number format".to_string());
                } else {
                    let (is_unitary, error) = quantsim_core::core::endian_utils::is_unitary(&new_matrix);
                    if !is_unitary {
                        editor_state.error_message =
                            Some(format!("Matrix is not unitary. Error: {}", error));
                    } else {
                        // state
                        //     .circuit_state
                        //     .circuit
                        //     .registry
                        //     .update_gate_matrix(gate_id, new_matrix);
                        messages.push(Message::CloseCustomGateEditor);
                    }
                }
            }
        }
        Message::CloseCustomGateEditor => {
            state.custom_gate_editor_state = crate::state::CustomGateEditorState::default();
        }
        _ => {}
    }
}
