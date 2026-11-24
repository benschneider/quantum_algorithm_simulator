use crate::{
    messages::Message,
    state::{AppState, CircuitState, InitialStateEditorState, PlacementMode},
};
use quantsim_core::core::circuit::CircuitData;

pub(super) fn load_circuit_json(state: &mut AppState, json: String) {
    let data: CircuitData = match serde_json::from_str(&json) {
        Ok(data) => data,
        Err(error) => {
            state.ui_state.error_message = Some(format!("JSON parse error: {error}"));
            return;
        }
    };
    if let Some(vector) = &data.initial_state {
        let Some(expected) = CircuitState::dense_amplitude_count(data.num_qubits) else {
            state.ui_state.error_message = Some(format!(
                "For {} qubits, use initial_preps in place of an initial_state amplitude vector.",
                data.num_qubits
            ));
            return;
        };
        if vector.len() != expected {
            state.ui_state.error_message = Some(format!(
                "Expected {expected} initial amplitudes for {} qubits, found {}.",
                data.num_qubits,
                vector.len()
            ));
            return;
        }
    }

    // The next render reads both dimensions and steps in this same frame.
    let circuit_state = &mut state.circuit_state;
    if let Err(error) = circuit_state.circuit.update_from_data(data) {
        state.ui_state.error_message = Some(error);
        return;
    }
    circuit_state.num_qubits = circuit_state.circuit.num_qubits;
    circuit_state.num_timesteps = circuit_state.circuit.steps.len();
    state.initial_state_editor_state =
        InitialStateEditorState::from_circuit(&circuit_state.circuit);
    state.simulation_state = Default::default();

    let ui = &mut state.ui_state;
    ui.current_timestep = 0;
    ui.layout_draft = None;
    ui.selected_layout_wire = circuit_state
        .circuit
        .qubit_roles
        .iter()
        .position(|role| *role == Some(quantsim_core::circuit::QubitRole::Ancilla));
    ui.section_editor = None;
    ui.qft_draft = None;
    ui.selected_gate = None;
    ui.placement_mode = PlacementMode::Idle;
    if ui.selected_gate_for_editing.take().is_some() {
        ui.show_gate_editor_window = false;
    }
    ui.circuit_json_string = json;
    ui.error_message = None;
}

pub fn handle_json_editor_message(
    state: &mut AppState,
    message: Message,
    _messages: &mut Vec<Message>,
) {
    match message {
        Message::UpdateJsonFromCircuit => {
            let circuit_data: CircuitData = state.circuit_state.circuit.clone().into();
            state.ui_state.circuit_json_string =
                serde_json::to_string_pretty(&circuit_data).unwrap();
        }
        Message::UpdateCircuitFromJson(json) => load_circuit_json(state, json),
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
