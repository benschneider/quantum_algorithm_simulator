use crate::components::{circuit_controls, circuit_grid, gate_palette, json_editor_panel};
use crate::messages::Message;
use crate::state::{AppState, ui_state::LeftPanelTab};

/// The `central_panel` module is responsible for rendering the main content of
/// the application. This includes the circuit grid, the JSON editor, and the
/// controls for switching between them.
///
/// The central panel is the largest and most complex part of the UI. It is
/// composed of several smaller components, each of which is responsible for
/// a specific part of the UI.
pub fn render(state: &mut AppState, ctx: &egui::Context, messages: &mut Vec<Message>) {
    egui::SidePanel::left("left_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut state.ui_state.active_left_tab,
                LeftPanelTab::GatePalette,
                "Gates",
            );
        });

        ui.separator();

        match state.ui_state.active_left_tab {
            LeftPanelTab::GatePalette => {
                gate_palette::gate_palette(state, ui, messages);
            }
        }
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut state.ui_state.active_central_tab,
                crate::state::ui_state::CentralPanelTab::Circuit,
                "Circuit",
            );
            if ui
                .selectable_value(
                    &mut state.ui_state.active_central_tab,
                    crate::state::ui_state::CentralPanelTab::JsonEditor,
                    "JSON",
                )
                .clicked()
            {
                messages.push(Message::UpdateJsonFromCircuit);
            }
        });
        ui.separator();

        circuit_controls::circuit_controls(ui, state, messages);
        egui::ScrollArea::both().show(ui, |ui| match state.ui_state.active_central_tab {
            crate::state::ui_state::CentralPanelTab::Circuit => {
                circuit_grid::circuit_grid(state, ui, messages, state.ui_state.current_timestep);
            }
            crate::state::ui_state::CentralPanelTab::JsonEditor => {
                json_editor_panel::json_editor_panel(state, ui, messages);
            }
        });
    });
}
