use crate::{messages::Message, state::AppState};

pub fn handle_info_panel_message(
    state: &mut AppState,
    message: Message,
    _messages: &mut [Message],
) {
    match message {
        Message::UpdateGateAngle(new_angle) => {
            if let Some((row, col)) = state.ui_state.selected_gate_for_editing {
                let circuit = &mut state.circuit_state.circuit;
                if let Some(op) = circuit
                    .steps
                    .get_mut(col)
                    .and_then(|step| step.iter_mut().find(|op| op.qubits.contains(&(row as u32))))
                {
                    let parametric = circuit.registry.get_meta(&op.id).is_some_and(|meta| meta.is_parametric);
                    if op.matrix.is_some() || !parametric {
                        return;
                    }
                    let value = quantsim_core::core::types::Param::Scalar(new_angle as f32);
                    match op.params.first_mut() {
                        Some(param) if *param == value => return,
                        Some(param) => *param = value,
                        None => op.params.push(value),
                    }
                    super::simulation_handler::invalidate_edited_gate(state, col);
                }
            }
        }
        Message::SwapControlAndTarget => {
            if let Some((row, col)) = state.ui_state.selected_gate_for_editing {
                if let Some(op) = state
                    .circuit_state
                    .circuit
                    .steps
                    .get_mut(col)
                    .and_then(|step| step.iter_mut().find(|op| op.qubits.contains(&(row as u32))))
                {
                    if matches!(op.controlled_action(), Some(([_], _, _))) {
                        op.qubits.swap(0, 1);
                        super::simulation_handler::invalidate_edited_gate(state, col);
                    }
                }
            }
        }
        _ => {
            //log::warn!("Unhandled message in info panel handler: {:?}", message);
        }
    }
}
