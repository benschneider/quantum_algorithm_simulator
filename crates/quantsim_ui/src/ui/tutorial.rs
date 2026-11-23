use crate::messages::Message;
use crate::state::ui_state::{CentralPanelTab, TutorialStep};
use crate::state::AppState;
use egui_commonmark::CommonMarkViewer;

use TutorialStep::*;

const CHAPTERS: &[(TutorialStep, &str, &str)] = &[
    (
        TutorialIntro,
        "First circuit",
        include_str!("../../assets/docs/tutorials/tutorial_intro.md"),
    ),
    (
        SingleQubitGatesIntroduction,
        "Single-qubit gates",
        include_str!("../../assets/docs/tutorials/single_qubit_gates_introduction.md"),
    ),
    (
        MultiQubitGates,
        "Controlled gates",
        include_str!("../../assets/docs/tutorials/multi_qubit_gates.md"),
    ),
    (
        SingleQubitGatesEntangledStates,
        "Entanglement",
        include_str!("../../assets/docs/tutorials/single_qubit_gates_entangled_states.md"),
    ),
    (
        Measurements,
        "Readout",
        include_str!("../../assets/docs/tutorials/measurements.md"),
    ),
    (
        Simulations,
        "Scrub a circuit",
        include_str!("../../assets/docs/tutorials/simulations.md"),
    ),
    (
        Analytics,
        "Compare probabilities",
        include_str!("../../assets/docs/tutorials/analytics.md"),
    ),
    (
        InitialStates,
        "Initial states",
        include_str!("../../assets/docs/tutorials/initial_states.md"),
    ),
    (
        CustomGates,
        "Custom gates",
        include_str!("../../assets/docs/tutorials/custom_gates.md"),
    ),
    (
        TemplateGuide,
        "Template guide",
        include_str!("../../assets/docs/tutorials/template_guide.md"),
    ),
    (
        ErrorCorrection,
        "Error correction",
        include_str!("../../assets/docs/tutorials/error_correction.md"),
    ),
    (
        Conclusion,
        "Try it yourself",
        include_str!("../../assets/docs/tutorials/conclusion.md"),
    ),
];

pub fn render(state: &mut AppState, ctx: &egui::Context, messages: &mut Vec<Message>) {
    let mut open = state.ui_state.show_tutorial_window;
    if !open {
        return;
    }
    let bounds = ctx.screen_rect().shrink(8.0);
    let size = bounds.size().min(egui::vec2(540.0, 345.0));
    let top = (bounds.height() - size.y - 44.0).clamp(0.0, 282.0);
    egui::Window::new("Tutorial")
        .default_pos(bounds.min + egui::vec2(0.0, top))
        .default_size(size)
        .min_width(200.0)
        .open(&mut open)
        .show(ctx, |ui| contents(state, ui, messages));
    state.ui_state.show_tutorial_window = open;
}

pub(super) fn contents(state: &mut AppState, ui: &mut egui::Ui, messages: &mut Vec<Message>) {
    let ui_state = &mut state.ui_state;
    let mut index = CHAPTERS
        .iter()
        .position(|chapter| chapter.0 == ui_state.active_tutorial_step)
        .expect("every tutorial step has a chapter");
    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("tutorial_chapter")
            .width(175.0)
            .height(280.0)
            .show_index(ui, &mut index, CHAPTERS.len(), |i| {
                format!("{}. {}", i + 1, CHAPTERS[i].1)
            })
            .on_hover_text("Choose a chapter");
        if ui
            .add_enabled(index > 0, egui::Button::new("Back"))
            .clicked()
        {
            index -= 1;
        }
        if ui
            .add_enabled(index + 1 < CHAPTERS.len(), egui::Button::new("Next"))
            .clicked()
        {
            index += 1;
        }
        if CHAPTERS[index].0 == TutorialIntro
            && ui
                .button("Load Bell example")
                .on_hover_text("Replace the current circuit with the two-wire Bell example")
                .clicked()
        {
            ui_state.active_central_tab = CentralPanelTab::Circuit;
            messages.push(Message::LoadTemplate("bell.json".into()));
        }
    });
    ui.separator();

    let (step, _, content) = CHAPTERS[index];
    ui_state.active_tutorial_step = step;
    egui::ScrollArea::vertical()
        .id_salt(("tutorial_text", index))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            CommonMarkViewer::new().show(ui, &mut ui_state.tutorial_cache, content);
            if step == SingleQubitGatesEntangledStates {
                ui.collapsing("Worked example on the middle wire", |ui| {
                    CommonMarkViewer::new().show(
                        ui,
                        &mut ui_state.tutorial_cache,
                        include_str!("../../assets/docs/tutorials/middle_wire_example.md"),
                    );
                    let width = ui.available_width();
                    // Both source charts are 1120 by 480 pixels.
                    let chart_size = egui::vec2(width, width * 480.0 / 1120.0);
                    for (image, description) in [
                        (
                            egui::include_image!(
                                "../../assets/docs/tutorials/t1/prob_before_H_B.png"
                            ),
                            "Joint probabilities before H on q1",
                        ),
                        (
                            egui::include_image!(
                                "../../assets/docs/tutorials/t1/prob_after_H_B.png"
                            ),
                            "Joint probabilities after H on q1",
                        ),
                    ] {
                        ui.add(
                            egui::Image::new(image)
                                .fit_to_exact_size(chart_size)
                                .alt_text(description),
                        );
                    }
                });
            }
        });
}
