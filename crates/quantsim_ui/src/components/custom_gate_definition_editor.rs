use crate::state::custom_gate_editor_state::CustomGateEditorTab;
use crate::{messages::Message, state::AppState};
use egui::{Align, Grid, Layout, TextEdit};

pub fn custom_gate_definition_editor(
    state: &mut AppState,
    ctx: &egui::Context,
    messages: &mut Vec<Message>,
) {
    let editor = &mut state.custom_gate_editor_state;
    if !editor.is_open {
        return;
    }

    let mut open = true;
    egui::Window::new("Edit Custom gate")
        .open(&mut open)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label("Save sets the matrix for the next Custom gates you place.");
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut editor.selected_tab,
                    CustomGateEditorTab::Real,
                    "Real Part",
                );
                ui.selectable_value(
                    &mut editor.selected_tab,
                    CustomGateEditorTab::Imaginary,
                    "Imaginary Part",
                );
            });

            Grid::new((
                "custom_gate_matrix",
                editor.selected_tab == CustomGateEditorTab::Real,
            ))
            .num_columns(editor.matrix_str.ncols())
            .show(ui, |ui| {
                for row in 0..editor.matrix_str.nrows() {
                    for col in 0..editor.matrix_str.ncols() {
                        let (mut real, mut imag) = editor.matrix_str[(row, col)].clone();
                        let field = match editor.selected_tab {
                            CustomGateEditorTab::Real => &mut real,
                            CustomGateEditorTab::Imaginary => &mut imag,
                        };
                        if ui
                            .add(TextEdit::singleline(field).desired_width(55.0))
                            .changed()
                        {
                            messages.push(Message::UpdateCustomGateEditorValue {
                                row,
                                col,
                                real,
                                imag,
                            });
                        }
                    }
                    ui.end_row();
                }
            });

            if let Some(error) = &editor.error_message {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                if ui.button("Cancel").clicked() {
                    messages.push(Message::CloseCustomGateEditor);
                }
                if ui.button("Make Unitary").clicked() {
                    messages.push(Message::MakeCustomGateMatrixUnitary);
                }
                if ui.button("Save").clicked() {
                    messages.push(Message::SaveCustomGateMatrix);
                }
            });
        });

    if !open {
        messages.push(Message::CloseCustomGateEditor);
    }
}
