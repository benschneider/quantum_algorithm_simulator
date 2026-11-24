use crate::components::{
    circuit_controls, circuit_grid, gate_palette, json_editor_panel, layout_panel,
};
use crate::messages::Message;
use crate::state::{ui_state::LeftPanelTab, AppState};

/// The `central_panel` module is responsible for rendering the main content of
/// the application. This includes the circuit grid, the JSON editor, and the
/// controls for switching between them.
///
/// The central panel is the largest and most complex part of the UI. It is
/// composed of several smaller components, each of which is responsible for
/// a specific part of the UI.
pub fn render(state: &mut AppState, ctx: &egui::Context, messages: &mut Vec<Message>) {
    let compact_palette = ctx.available_rect().width() < 720.0;
    if !compact_palette {
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
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            if compact_palette {
                ui.menu_button("Gates", |ui| {
                    let before = messages.len();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        gate_palette::gate_palette(state, ui, messages);
                    });
                    if messages.len() != before || state.ui_state.qft_draft.is_some() {
                        ui.close();
                    }
                });
            }
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
        if let Some(error) = &state.ui_state.error_message {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        match state.ui_state.active_central_tab {
            crate::state::ui_state::CentralPanelTab::Circuit => {
                egui::ScrollArea::vertical()
                    .id_salt("circuit_workspace")
                    .show(ui, |ui| {
                        egui::ScrollArea::horizontal()
                            .id_salt("circuit_columns")
                            .show(ui, |ui| {
                                circuit_grid::circuit_grid(
                                    state,
                                    ui,
                                    messages,
                                    state.ui_state.current_timestep,
                                );
                            });
                        ui.add_space(12.0);
                        layout_panel::show(ui, state, messages);
                    });
            }
            crate::state::ui_state::CentralPanelTab::JsonEditor => {
                egui::ScrollArea::both().show(ui, |ui| {
                    json_editor_panel::json_editor_panel(state, ui, messages);
                });
            }
        }
    });
    gate_palette::show_qft_builder(ctx, state, messages);
}
