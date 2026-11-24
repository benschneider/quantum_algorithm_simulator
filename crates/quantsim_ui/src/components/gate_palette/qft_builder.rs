use crate::messages::Message;
use crate::state::ui_state::{CentralPanelTab, QftDraft};
use crate::state::AppState;

pub(super) fn button(ui: &mut egui::Ui, state: &mut AppState) {
    if state.ui_state.active_central_tab == CentralPanelTab::Circuit
        && ui
            .button("Insert QFT…")
            .on_hover_text("Insert a quantum Fourier transform as editable gates")
            .clicked()
    {
        state.ui_state.qft_draft = Some(QftDraft {
            first_wire: 0,
            width: state.circuit_state.circuit.num_qubits,
            after_step: state.ui_state.current_timestep,
        });
    }
}

pub(crate) fn show(ctx: &egui::Context, state: &mut AppState, messages: &mut Vec<Message>) {
    let Some(draft) = &mut state.ui_state.qft_draft else {
        return;
    };
    let circuit = &state.circuit_state.circuit;
    let dialog = egui::Modal::new(egui::Id::new("qft_builder")).show(ctx, |ui| {
        ui.heading("Quantum Fourier transform");
        egui::Grid::new("qft_fields").show(ui, |ui| {
            ui.label("First wire");
            ui.add(
                egui::DragValue::new(&mut draft.first_wire)
                    .prefix("q")
                    .range(0..=circuit.num_qubits - 1),
            );
            ui.end_row();
            ui.label("Qubit count");
            ui.add(
                egui::DragValue::new(&mut draft.width)
                    .range(1..=circuit.num_qubits - draft.first_wire),
            );
            ui.end_row();
            ui.label("After step");
            ui.add(egui::DragValue::new(&mut draft.after_step).range(0..=circuit.steps.len()));
            ui.end_row();
        });
        let wires = if draft.width == 1 {
            format!("q{}", draft.first_wire)
        } else {
            format!(
                "q{}–q{}",
                draft.first_wire,
                draft.first_wire + draft.width - 1
            )
        };
        if draft.after_step < circuit.steps.len() {
            ui.label(format!(
                "Inserts on {wires} before step {}.",
                draft.after_step + 1
            ));
        } else {
            ui.label(format!("Appends on {wires} at the end of the circuit."));
        }
        ui.horizontal(|ui| {
            if ui.button("Insert QFT").clicked() {
                messages.push(Message::InsertQft {
                    after_step: draft.after_step,
                    first_wire: draft.first_wire,
                    width: draft.width,
                });
                return true;
            }
            ui.button("Cancel").clicked()
        })
        .inner
    });
    if dialog.inner || dialog.should_close() {
        state.ui_state.qft_draft = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QCSimApp;
    use egui::{Context, Event, FullOutput, Modifiers, PointerButton, RawInput};

    #[test]
    fn compact_palette_opens_builder_and_submits_the_chosen_wires_and_step() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::ChangeQubits(6));
        app.handle_message(Message::ChangeTimesteps(10));
        app.state.ui_state.current_timestep = 4;
        let ctx = Context::default();
        let render = |app: &mut QCSimApp, events| -> FullOutput {
            ctx.run(
                RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(600.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| crate::ui::central_panel::render(&mut app.state, ctx, &mut app.messages),
            )
        };
        let click = |app: &mut QCSimApp, label: &str| {
            render(app, vec![]);
            let pos = render(app, vec![])
                .shapes
                .iter()
                .rev()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.visual_bounding_rect().center())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Missing {label}"));
            let mut events = vec![Event::PointerMoved(pos)];
            events.extend([true, false].map(|pressed| Event::PointerButton {
                pos,
                pressed,
                button: PointerButton::Primary,
                modifiers: Modifiers::NONE,
            }));
            render(app, events);
        };

        click(&mut app, "Gates");
        click(&mut app, "Insert QFT…");
        let draft = app.state.ui_state.qft_draft.as_ref().unwrap();
        assert_eq!((draft.first_wire, draft.width, draft.after_step), (0, 6, 4));
        click(&mut app, "Cancel");
        assert!(app.state.ui_state.qft_draft.is_none() && app.messages.is_empty());

        click(&mut app, "Gates");
        click(&mut app, "Insert QFT…");
        for (field, value) in [("q0", "1"), ("5", "3"), ("4", "2")] {
            click(&mut app, field);
            render(&mut app, vec![Event::Text(value.into())]);
        }
        click(&mut app, "Insert QFT");
        assert!(app.state.ui_state.qft_draft.is_none());
        assert!(matches!(
            app.messages.as_slice(),
            [Message::InsertQft {
                after_step: 2,
                first_wire: 1,
                width: 3
            }]
        ));
    }
}
