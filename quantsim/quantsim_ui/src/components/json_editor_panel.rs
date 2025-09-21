use crate::messages::Message;
use crate::state::AppState;

/// Renders the JSON editor panel, which allows the user to edit the circuit
/// in JSON format.
///
/// # Arguments
///
/// * `state` - The application state.
/// * `ui` - The `egui` user interface.
/// * `messages` - A vector of messages to be sent to the application.
pub fn json_editor_panel(state: &mut AppState, ui: &mut egui::Ui, messages: &mut Vec<Message>) {
    ui.heading("JSON Editor");

    ui.add_space(10.0);

    let mut json_string = state.ui_state.circuit_json_string.clone();
    ui.add(
        egui::TextEdit::multiline(&mut json_string)
            .code_editor()
            .desired_width(f32::INFINITY),
    );
    state.ui_state.circuit_json_string = json_string;

    ui.add_space(10.0);

    ui.horizontal(|ui| {
        if ui.button("Apply Changes").clicked() {
            messages.push(Message::UpdateCircuitFromJson(
                state.ui_state.circuit_json_string.clone(),
            ));
        }

        if ui.button("Copy to Clipboard").clicked() {
            ui.ctx()
                .copy_text(state.ui_state.circuit_json_string.clone());
        }

        if ui.button("Format").clicked() {
            if let Ok(json) =
                serde_json::from_str::<serde_json::Value>(&state.ui_state.circuit_json_string)
            {
                if let Ok(pretty_json) = serde_json::to_string_pretty(&json) {
                    state.ui_state.circuit_json_string = pretty_json;
                }
            }
        }
    });
}
