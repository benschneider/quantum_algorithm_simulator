// crates/qcsim-egui/src/components/initial_state_editor.rs

use crate::messages::Message;
use crate::state::{AppState, InitialStateEditorState};
use egui::{Align, Button, Color32, Layout, RichText, Window};
use nalgebra::Complex;

/// Renders the initial state editor, which allows the user to define the
/// initial state of the qubits.
///
/// # Arguments
///
/// * `state` - The application state.
/// * `ctx` - The `egui` context.
///
/// # Returns
///
/// An `Option<Message>` that contains a message to be sent to the application.
pub fn initial_state_editor(state: &mut AppState, ctx: &egui::Context) -> Option<Message> {
    let mut message = None;

    if state.ui_state.show_initial_state_editor {
        Window::new("Set Initial State")
            .open(&mut state.ui_state.show_initial_state_editor)
            .collapsible(false)
            .resizable(true)
            .scroll(true)
            .show(ctx, |ui| {
                // Check if the number of qubits has changed
                if state.initial_state_editor_state.num_qubits != state.circuit_state.num_qubits {
                    // If so, create a new state with the correct number of qubits
                    state.initial_state_editor_state =
                        InitialStateEditorState::new(state.circuit_state.num_qubits);
                }

                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("Enter the initial state vector amplitudes (unnormalized):")
                            .strong(),
                    );
                });

                // Grid for real and imaginary parts
                let start_index = state
                    .initial_state_editor_state
                    .pagination
                    .get_start_index();
                let end_index = state.initial_state_editor_state.pagination.get_end_index();

                egui::Grid::new("initial_state_grid")
                    .num_columns(3)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Amplitude");
                        ui.label("Real");
                        ui.label("Imaginary");
                        ui.end_row();

                        for i in start_index..end_index {
                            let num_qubits = state.circuit_state.num_qubits;
                            ui.label(format!("|{:0width$b}>   {i}", i, width = num_qubits));

                            let mut re_str = state.initial_state_editor_state.state_vector[i]
                                .re
                                .to_string();
                            let mut im_str = state.initial_state_editor_state.state_vector[i]
                                .im
                                .to_string();

                            let re_response = ui.add(
                                egui::TextEdit::singleline(&mut re_str)
                                    .id(egui::Id::new("re_").with(i))
                                    .desired_width(50.0),
                            );
                            let im_response = ui.add(
                                egui::TextEdit::singleline(&mut im_str)
                                    .id(egui::Id::new("im_").with(i))
                                    .desired_width(50.0),
                            );

                            if re_response.changed() {
                                if let Ok(re) = re_str.parse() {
                                    state.initial_state_editor_state.state_vector[i].re = re;
                                }
                            }
                            if im_response.changed() {
                                if let Ok(im) = im_str.parse() {
                                    state.initial_state_editor_state.state_vector[i].im = im;
                                }
                            }

                            ui.end_row();
                        }
                    });

                ui.separator();

                ui.horizontal(|ui| {
                    if let Some(msg) = state
                        .initial_state_editor_state
                        .pagination
                        .render_controls(ui, &mut state.initial_state_editor_state.page_input_text)
                    {
                        message = Some(msg);
                    }

                    if ui.button("Apply").clicked() {
                        let state_vector: Vec<Complex<f64>> = state
                            .initial_state_editor_state
                            .state_vector
                            .iter()
                            .map(|c| Complex::new(c.re as f64, c.im as f64))
                            .collect();
                        message = Some(Message::ApplyInitialStateFromEditor(state_vector));
                    }
                    if ui.button("Cancel").clicked() {
                        message = Some(Message::ToggleInitialStateEditor);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(Button::new(
                                RichText::new("Reset to |0>").color(Color32::LIGHT_RED),
                            ))
                            .clicked()
                        {
                            message = Some(Message::ResetInitialState);
                        }
                    });
                });
            });
    }
    message
}
