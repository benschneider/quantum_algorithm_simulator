//! Theming for the egui application.
//!
//! This module provides functions for applying custom themes to the `egui` context.
//! It is used to ensure a consistent and visually appealing user interface.

/// Applies a custom dark theme to the `egui` context.
///
/// This function sets up the `Visuals` for the dark theme, customizing colors
/// for text, widgets, and selections to create a polished and professional look.
pub fn apply_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_gray(220));
    visuals.widgets.inactive.bg_fill = egui::Color32::from_gray(40);
    visuals.selection.bg_fill = egui::Color32::from_rgb(0, 116, 166);
    ctx.set_visuals(visuals);
}
