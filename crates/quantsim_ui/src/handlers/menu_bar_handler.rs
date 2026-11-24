use super::json_editor_handler::load_circuit_json;
use crate::state::ui_state::{CentralPanelTab, LayoutView, StackCamera};
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
                let ui = &mut state.ui_state;
                if ui.error_message.is_none() {
                    match template_name.as_str() {
                        "sycamore_echo_53.json"
                        | "ring_interferometer_8.json"
                        | "tree_ghz_7.json" => {
                            ui.active_central_tab = CentralPanelTab::Layout;
                            ui.layout_view = LayoutView::Plane;
                        }
                        "sycamore_53.json" => {
                            ui.active_central_tab = CentralPanelTab::Layout;
                            ui.layout_view = LayoutView::Stack;
                            ui.stack_camera = StackCamera {
                                yaw: -32.0,
                                pitch: 45.0,
                            };
                        }
                        _ if ui.active_central_tab == CentralPanelTab::Layout
                            && state.circuit_state.circuit.layout.is_none() =>
                        {
                            ui.active_central_tab = CentralPanelTab::Circuit;
                        }
                        _ => {}
                    }
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
        }
        _ => {
            // Handle other menu bar messages if needed
        }
    }
}

#[cfg(test)]
mod showcase_tests;
