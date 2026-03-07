use crate::state::{AppState, ui_state::InfoPanelTab};
use egui::Context;
use egui_commonmark::CommonMarkViewer;

pub fn render(state: &mut AppState, ctx: &Context) {
    if state.ui_state.show_info_window {
        let mut open = state.ui_state.show_info_window;
        egui::Window::new("Info").open(&mut open).show(ctx, |ui| {
            ui.horizontal(|ui| {
                //ui.selectable_value(&mut state.ui_state.active_info_tab, InfoPanelTab::About, "About");
                ui.selectable_value(
                    &mut state.ui_state.active_info_tab,
                    InfoPanelTab::GateReference,
                    "Gate Reference",
                );
            });
            ui.separator();

            let content = match state.ui_state.active_info_tab {
                //InfoPanelTab::About => include_str!("../../../../docs/about.md"),
                InfoPanelTab::GateReference => include_str!("../../assets/docs/gate_reference.md"), // Relative path to the gate reference markdown file
            };

            egui::ScrollArea::vertical().show(ui, |ui| {
                use egui_commonmark::CommonMarkCache;
                let mut cache = CommonMarkCache::default();
                CommonMarkViewer::new().show(ui, &mut cache, content);
            });
        });
        state.ui_state.show_info_window = open;
    }

    if state.ui_state.show_about_window {
        let mut open = state.ui_state.show_about_window;
        egui::Window::new("About")
            .vscroll(true)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.heading("quantsim_ui");
                ui.separator();
                ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                ui.label("Author: Ben Schneider");
            });
        state.ui_state.show_about_window = open;
    }
}
