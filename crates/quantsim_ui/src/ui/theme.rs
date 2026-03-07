//! Theming for the egui application.
//!
//! This module provides functions for applying custom themes to the `egui` context.
//! It is used to ensure a consistent and visually appealing user interface.

fn dark_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_gray(220));
    visuals.widgets.inactive.bg_fill = egui::Color32::from_gray(40);
    visuals.selection.bg_fill = egui::Color32::from_rgb(0, 116, 166);
    visuals
}

fn light_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::light();
    visuals.override_text_color = Some(egui::Color32::from_rgb(28, 32, 38));
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(248, 250, 252);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(236, 240, 245);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(226, 232, 240);
    visuals.panel_fill = egui::Color32::from_rgb(252, 253, 255);
    visuals.selection.bg_fill = egui::Color32::from_rgb(104, 176, 232);
    visuals
}

/// Applies theme configuration to the `egui` context and follows the system theme.
pub fn apply_theme(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::System);
    ctx.set_visuals_of(egui::Theme::Dark, dark_visuals());
    ctx.set_visuals_of(egui::Theme::Light, light_visuals());
}
