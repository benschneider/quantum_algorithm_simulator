use crate::messages::Message;
use crate::state::{ui_state::SectionDraft, AppState};

pub(super) fn show(ctx: &egui::Context, state: &mut AppState, messages: &mut Vec<Message>) {
    let Some(draft) = &mut state.ui_state.section_editor else {
        return;
    };
    let circuit = &state.circuit_state.circuit;
    let last_step = circuit.steps.len();
    let mut open = true;
    egui::Window::new("Circuit sections")
        .min_width(290.0)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            let selected = circuit
                .sections
                .iter()
                .find(|section| Some(section.start) == draft.original_start);
            egui::ComboBox::from_id_salt("section_to_edit")
                .selected_text(selected.map_or("New section", |section| section.label.as_str()))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(selected.is_none(), "New section")
                        .clicked()
                    {
                        *draft = SectionDraft::new(state.ui_state.current_timestep.max(1), None);
                        ui.close();
                    }
                    for section in &circuit.sections {
                        let caption =
                            format!("{}–{}  {}", section.start, section.end, section.label);
                        if ui
                            .selectable_label(Some(section.start) == draft.original_start, caption)
                            .clicked()
                        {
                            *draft = SectionDraft::new(section.start, Some(section));
                            ui.close();
                        }
                    }
                });

            egui::Grid::new("section_fields").show(ui, |ui| {
                ui.label("Label");
                ui.add(egui::TextEdit::singleline(&mut draft.section.label).desired_width(200.0));
                ui.end_row();
                ui.label("Description");
                ui.add(
                    egui::TextEdit::multiline(&mut draft.section.description)
                        .hint_text("Optional")
                        .desired_width(200.0)
                        .desired_rows(2),
                );
                ui.end_row();
                ui.label("First step");
                ui.add(egui::DragValue::new(&mut draft.section.start).range(1..=last_step));
                ui.end_row();
                ui.label("Last step");
                ui.add(egui::DragValue::new(&mut draft.section.end).range(1..=last_step));
                ui.end_row();
            });
            if let Some(error) = &draft.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    messages.push(Message::SaveSection);
                }
                if draft.original_start.is_some() && ui.button("Delete").clicked() {
                    messages.push(Message::DeleteSection);
                }
            });
        });
    if !open {
        state.ui_state.section_editor = None;
    }
}
