use crate::{messages::Message, state::AppState};
//use quantsim_core::core::circuit::CircuitData;
use quantsim_core::core::circuit::Circuit;

pub fn handle_menu_bar_message(state: &mut AppState, message: Message, _messages: &mut [Message]) {
    match message {
        Message::LoadTemplate(template_name) => {
            if let Some((_, json)) = state
                .template_circuits
                .iter()
                .find(|(name, _)| name == &template_name)
            {
                if let Ok(data) = serde_json::from_str::<quantsim_core::core::circuit::CircuitData>(json) {
                    let mut new_circuit = Circuit::new(data.num_qubits);
                    new_circuit.update_from_data(data);
                    state.circuit_state.circuit = new_circuit;
                    state.circuit_state.num_qubits = state.circuit_state.circuit.num_qubits;
                    state.circuit_state.num_timesteps = state.circuit_state.circuit.steps.len();
                }
            }
        }
        Message::SaveCircuit => {
            // Logic to save the current circuit
        }
        Message::ToggleInfoWindow => {
            state.ui_state.show_info_window = !state.ui_state.show_info_window;
        }
        Message::ToggleAboutWindow => {
            state.ui_state.show_about_window = !state.ui_state.show_about_window;
        }
        Message::ToggleTutorialWindow => {
            state.ui_state.show_tutorial_window = true;
            state.ui_state.active_tutorial_step =
                crate::state::ui_state::TutorialStep::SingleQubitGatesEntangledStates;
        }
        _ => {
            // Handle other menu bar messages if needed
        }
    }
}
