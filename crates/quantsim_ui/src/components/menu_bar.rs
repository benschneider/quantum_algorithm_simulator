use crate::messages::Message;
use crate::state::AppState;
use egui::containers::menu::{MenuConfig, SubMenuButton};
use std::collections::BTreeMap;

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
        let config =
            MenuConfig::find(ui).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
        SubMenuButton::new("Load Template")
            .config(config)
            .ui(ui, |ui| template_picker(ui, state, &mut messages));
    });
    ui.menu_button("View", |ui| {
        ui.checkbox(&mut state.ui_state.show_gate_editor_window, "Gate Editor");
        ui.checkbox(&mut state.ui_state.show_results_window, "Results");
    });
    ui.menu_button("Settings", |ui| {
        if ui.button("Set Initial State...").clicked() {
            messages.push(Message::ToggleInitialStateEditor);
        }
        if ui.button("MPS ladder experiment").clicked() {
            state.ui_state.mps_ladder.open = true;
            ui.close();
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

const SCALE_EXPERIMENTS: &str = "Scale experiments";

fn template_category(file: &str) -> &'static str {
    match file.trim_end_matches(".json") {
        "bell" | "bell_test_complex" | "ghz" | "quantum_teleportation" => "Entanglement",
        "deutsch_algorithm" | "grover" | "grover_algorithm" => "Algorithms",
        "fourier_wave_50" | "qft" | "qft4" | "qft_50" => "Fourier transforms",
        "plaquette_z_check" => "Error correction",
        name if name.starts_with("error_correction_") => "Error correction",
        "sycamore_53"
        | "sycamore_echo_53"
        | "sycamore_patch_16"
        | "ring_interferometer_8"
        | "tree_ghz_7" => "Qubit layouts",
        "bell_256" | "ghz_50" | "grouped_blocks_1024" | "phase_echo_512" => SCALE_EXPERIMENTS,
        _ => "Other circuits",
    }
}

fn template_picker(ui: &mut egui::Ui, state: &mut AppState, messages: &mut Vec<Message>) {
    let mut groups = BTreeMap::<_, Vec<_>>::new();
    groups.entry(SCALE_EXPERIMENTS).or_default();
    for (file, _) in &state.template_circuits {
        groups
            .entry(template_category(file))
            .or_default()
            .push(file);
    }
    // The previous popup size only accounts for collapsed headings on first open.
    let screen = ui.ctx().screen_rect();
    ui.set_width((screen.width() * 0.6).min(232.0));
    ui.set_max_height(screen.height() * 0.8);
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (category, files) in groups {
            ui.collapsing(category, |ui| {
                for file in files {
                    let label = match file.as_str() {
                        "sycamore_patch_16.json" => "Sycamore 2×8 patch".into(),
                        "ring_interferometer_8.json" => "Ring interferometer (8 qubits)".into(),
                        "tree_ghz_7.json" => "Binary tree GHZ (7 qubits)".into(),
                        _ => file.trim_end_matches(".json").replace('_', " "),
                    };
                    if ui.button(label).clicked() {
                        messages.push(Message::LoadTemplate(file.clone()));
                        ui.close_kind(egui::UiKind::Menu);
                    }
                }
                if category == SCALE_EXPERIMENTS
                    && ui
                        .button("MPS ladder echo 256")
                        .on_hover_text(
                            "Explore the measured bond caps, or run the echo in the desktop app.",
                        )
                        .clicked()
                {
                    state.ui_state.mps_ladder.open = true;
                    ui.close_kind(egui::UiKind::Menu);
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_templates_have_a_subject_and_new_files_have_a_fallback() {
        for name in quantsim_core::circuits::get_circuit_names() {
            assert_ne!(template_category(&name), "Other circuits", "{name}");
        }
        assert_eq!(template_category("new_example.json"), "Other circuits");
    }
}
