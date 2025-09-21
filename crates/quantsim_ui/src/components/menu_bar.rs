use crate::messages::Message;
use crate::state::AppState;

/// Renders the menu bar, which provides access to file operations, view
/// settings, and help information.
///
/// # Arguments
///
/// * `state` - The application state.
/// * `ui` - The `egui` user interface.
///
/// # Returns
///
/// A vector of messages to be sent to the application.
pub fn menu_bar(state: &mut AppState, ui: &mut egui::Ui) -> Vec<Message> {
    let mut messages = Vec::new();
    let is_web = cfg!(target_arch = "wasm32");

    ui.menu_button("File", |ui| {
        if !is_web && ui.button("Quit").clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ui.menu_button("Load Template", |ui| {
            for (name, _) in state.template_circuits.iter() {
                let display_name = name.trim_end_matches(".json").replace('_', " ");
                if ui.button(display_name).clicked() {
                    messages.push(Message::LoadTemplate(name.to_string()));
                }
            }
        });
    });
    ui.menu_button("View", |ui| {
        ui.checkbox(&mut state.ui_state.show_gate_editor_window, "Gate Editor");
        ui.checkbox(&mut state.ui_state.show_results_window, "Results");
    });
    ui.menu_button("Settings", |ui| {
        if ui.button("Set Initial State...").clicked() {
            messages.push(Message::ToggleInitialStateEditor);
        }
    });
    if ui.button("New Circuit").clicked() {
        messages.push(Message::NewCircuit);
    }
    //if ui.button("Run").clicked() {
    //    messages.push(Message::RunSimulation);
    //}
    //if ui.button("View Analytics").clicked() {
    // This will be handled by a message
    //}
    //if ui.button("JSON Editor").clicked() {
    // This will be handled by a message
    //}
    ui.menu_button("Help", |ui| {
        if ui.button("Info").clicked() {
            messages.push(Message::ToggleInfoWindow);
        }
        if ui.button("About").clicked() {
            messages.push(Message::ToggleAboutWindow);
        }
        if ui.button("Tutorial").clicked() {
            messages.push(Message::ToggleTutorialWindow);
        }
    });
    ui.add_space(16.0);

    egui::widgets::global_theme_preference_buttons(ui);

    messages
}
