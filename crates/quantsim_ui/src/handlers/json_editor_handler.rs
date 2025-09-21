use crate::{messages::Message, state::AppState};
use quantsim_core::core::circuit::CircuitData;

pub fn handle_json_editor_message(
    state: &mut AppState,
    message: Message,
    messages: &mut Vec<Message>,
) {
    match message {
        Message::UpdateJsonFromCircuit => {
            let circuit_data: CircuitData = state.circuit_state.circuit.clone().into();
            state.ui_state.circuit_json_string =
                serde_json::to_string_pretty(&circuit_data).unwrap();
        }
        Message::UpdateCircuitFromJson(json) => {
            match serde_json::from_str::<CircuitData>(&json) {
                Ok(data) => {
                    let new_num_qubits = data.num_qubits;
                    let new_num_timesteps = data.steps.len();
                    log::debug!(
                        "JSON parsed. num_qubits: {}, num_timesteps: {}",
                        new_num_qubits,
                        new_num_timesteps
                    );

                    // Update the underlying circuit struct
                    state.circuit_state.circuit.update_from_data(data);
                    log::debug!(
                        "Circuit updated. circuit.num_qubits is now: {}",
                        state.circuit_state.circuit.num_qubits
                    );

                    // Dispatch a message to notify the rest of the app about the dimension change.
                    messages.push(Message::CircuitDimensionsChanged {
                        num_qubits: new_num_qubits,
                        num_timesteps: new_num_timesteps,
                    });
                    log::debug!("Dispatched CircuitDimensionsChanged message.");
                }
                Err(e) => {
                    log::error!("Failed to parse circuit from JSON: {}", e);
                    state.ui_state.error_message = Some(format!("JSON parse error: {}", e));
                }
            }
        }
        Message::CopyJsonToClipboard => {
            // Logic to copy JSON to clipboard
        }
        Message::FormatJson => {
            if let Ok(parsed_json) =
                serde_json::from_str::<serde_json::Value>(&state.ui_state.circuit_json_string)
            {
                if let Ok(formatted_json) = serde_json::to_string_pretty(&parsed_json) {
                    state.ui_state.circuit_json_string = formatted_json;
                }
            }
        }
        _ => {}
    }
}
