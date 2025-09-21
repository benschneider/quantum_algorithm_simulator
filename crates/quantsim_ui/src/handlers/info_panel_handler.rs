use crate::{messages::Message, state::AppState};

pub fn handle_info_panel_message(
    state: &mut AppState,
    message: Message,
    _messages: &mut [Message],
) {
    match message {
        Message::UpdateGateAngle(new_angle) => {
            if let Some((row, col)) = state.ui_state.selected_gate_for_editing {
                if let Some(op) = state
                    .circuit_state
                    .circuit
                    .steps
                    .get_mut(col)
                    .and_then(|step| step.iter_mut().find(|op| op.qubits.contains(&(row as u32))))
                {
                    if let Some(param) = op.params.get_mut(0) {
                        *param = quantsim_core::core::types::Param::Scalar(new_angle as f32);
                    }
                }
            }
        }
        Message::UpdateGateControl(gate_pos, is_control) => {
            if let Some((row, col)) = state.ui_state.selected_gate_for_editing {
                if let Some(op) = state
                    .circuit_state
                    .circuit
                    .steps
                    .get_mut(col)
                    .and_then(|step| step.iter_mut().find(|op| op.qubits.contains(&(row as u32))))
                {
                    if is_control {
                        if !op.qubits.contains(&(gate_pos as u32)) {
                            op.qubits.push(gate_pos as u32);
                        }
                    } else {
                        op.qubits.retain(|q| *q != gate_pos as u32);
                    }
                }
            }
        }
        Message::UpdateCustomGate(_gate_id, _new_matrix) => {
            todo!()
            // state
            //     .circuit_state
            //     .circuit
            //     .registry
            //     .update_gate_matrix(&gate_id, new_matrix);
        }
        _ => {
            //log::warn!("Unhandled message in info panel handler: {:?}", message);
        }
    }
}
