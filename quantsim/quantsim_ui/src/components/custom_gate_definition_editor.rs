use crate::state::custom_gate_editor_state::CustomGateEditorTab;
use crate::{messages::Message, state::AppState};
use egui::{Align, Grid, Layout, TextEdit, Ui};
use nalgebra::DMatrix;

// Helper function to draw the real part of the matrix grid
fn draw_matrix_grid_real(
    ui: &mut Ui,
    current_matrix_str: &DMatrix<(String, String)>,
) -> Vec<(usize, usize, String)> {
    let (rows, cols) = current_matrix_str.shape();
    let mut changes = Vec::new();

    Grid::new("custom_gate_matrix_real_grid")
        .num_columns(cols)
        .spacing(egui::vec2(5.0, 2.0))
        .show(ui, |ui| {
            for r in 0..rows {
                for c in 0..cols {
                    let (real_str, _) = current_matrix_str[(r, c)].clone();
                    let mut real_s = real_str;

                    ui.add(
                        TextEdit::singleline(&mut real_s)
                            .desired_width(30.0)
                            .hint_text("Real"),
                    );

                    if real_s != current_matrix_str[(r, c)].0 {
                        changes.push((r, c, real_s));
                    }
                }
                ui.end_row();
            }
        });
    changes
}

// Helper function to draw the imaginary part of the matrix grid
fn draw_matrix_grid_imaginary(
    ui: &mut Ui,
    current_matrix_str: &DMatrix<(String, String)>,
) -> Vec<(usize, usize, String)> {
    let (rows, cols) = current_matrix_str.shape();
    let mut changes = Vec::new();

    Grid::new("custom_gate_matrix_imaginary_grid")
        .num_columns(cols)
        .spacing(egui::vec2(5.0, 2.0))
        .show(ui, |ui| {
            for r in 0..rows {
                for c in 0..cols {
                    let (_, imag_str) = current_matrix_str[(r, c)].clone();
                    let mut imag_s = imag_str;

                    ui.add(
                        TextEdit::singleline(&mut imag_s)
                            .desired_width(30.0)
                            .hint_text("Imag"),
                    );

                    if imag_s != current_matrix_str[(r, c)].1 {
                        changes.push((r, c, imag_s));
                    }
                }
                ui.end_row();
            }
        });
    changes
}

// Helper function to draw the action buttons
fn draw_buttons(ui: &mut Ui, messages: &mut Vec<Message>, error_message: &Option<String>) -> bool {
    let mut should_make_unitary = false;
    ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
        if ui.button("Cancel").clicked() {
            messages.push(Message::CloseCustomGateEditor);
        }

        if ui.button("Make Unitary").clicked() {
            should_make_unitary = true;
        }

        let save_button = ui.add_enabled(error_message.is_none(), egui::Button::new("Save"));
        if save_button.clicked() {
            messages.push(Message::SaveCustomGateMatrix);
        }
    });
    should_make_unitary
}

/// Renders the custom gate definition editor, which allows the user to define
/// a custom gate by specifying its matrix representation.
///
/// # Arguments
///
/// * `state` - The application state.
/// * `ctx` - The `egui` context.
/// * `messages` - A vector of messages to be sent to the application.
pub fn custom_gate_definition_editor(
    state: &mut AppState,
    ctx: &egui::Context,
    messages: &mut Vec<Message>,
) {
    if !state.custom_gate_editor_state.is_open {
        return;
    }

    let gate_id_option = state.custom_gate_editor_state.gate_id.clone();
    let mut is_open = state.custom_gate_editor_state.is_open;
    let mut selected_tab = state.custom_gate_editor_state.selected_tab;
    let error_message = state.custom_gate_editor_state.error_message.clone();
    let matrix_str = state.custom_gate_editor_state.matrix_str.clone();

    let mut real_changes: Vec<(usize, usize, String)> = Vec::new();
    let mut imag_changes: Vec<(usize, usize, String)> = Vec::new();
    let mut should_make_unitary = false;

    let window_title = if let Some(gate_id) = &gate_id_option {
        format!("Edit Gate: {}", gate_id)
    } else {
        "New Custom Gate".to_string()
    };

    egui::Window::new(window_title)
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut selected_tab, CustomGateEditorTab::Real, "Real Part");
                    ui.selectable_value(
                        &mut selected_tab,
                        CustomGateEditorTab::Imaginary,
                        "Imaginary Part",
                    );
                });

                //ui.add_space(8.0);

                match selected_tab {
                    CustomGateEditorTab::Real => {
                        ui.label("Real Part of Custom Gate Matrix:");
                        real_changes = draw_matrix_grid_real(ui, &matrix_str);
                    }
                    CustomGateEditorTab::Imaginary => {
                        ui.label("Imaginary Part of Custom Gate Matrix:");
                        imag_changes = draw_matrix_grid_imaginary(ui, &matrix_str);
                    }
                }

                if let Some(error) = &error_message {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::RED, error);
                }

                ui.add_space(10.0);
                should_make_unitary = draw_buttons(ui, messages, &error_message);
            });
        });

    state.custom_gate_editor_state.is_open = is_open;
    state.custom_gate_editor_state.selected_tab = selected_tab;

    for (r, c, new_val) in real_changes {
        state.custom_gate_editor_state.matrix_str[(r, c)].0 = new_val.clone();
        messages.push(Message::UpdateCustomGateEditorValue {
            row: r,
            col: c,
            real: new_val,
            imag: state.custom_gate_editor_state.matrix_str[(r, c)].1.clone(),
        });
    }

    for (r, c, new_val) in imag_changes {
        state.custom_gate_editor_state.matrix_str[(r, c)].1 = new_val.clone();
        messages.push(Message::UpdateCustomGateEditorValue {
            row: r,
            col: c,
            real: state.custom_gate_editor_state.matrix_str[(r, c)].0.clone(),
            imag: new_val,
        });
    }

    if should_make_unitary {
        state.custom_gate_editor_state.make_unitary();
    }

    if !state.custom_gate_editor_state.is_open {
        messages.push(Message::CloseCustomGateEditor);
    }
}
