use super::json_editor_handler::load_circuit_json;
use crate::{messages::Message, state::AppState};

pub fn handle_menu_bar_message(state: &mut AppState, message: Message, _messages: &mut [Message]) {
    match message {
        Message::LoadTemplate(template_name) => {
            if let Some((_, json)) = state
                .template_circuits
                .iter()
                .find(|(name, _)| name == &template_name)
            {
                load_circuit_json(state, json.clone());
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
        }
        _ => {
            // Handle other menu bar messages if needed
        }
    }
}
