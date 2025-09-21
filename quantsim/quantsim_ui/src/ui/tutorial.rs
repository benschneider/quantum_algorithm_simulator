use crate::state::{AppState, ui_state::TutorialStep};
use egui::{Context, Vec2};
use egui_commonmark::CommonMarkViewer;

pub fn render(state: &mut AppState, ctx: &Context) {
    if state.ui_state.show_tutorial_window {
        let mut open = state.ui_state.show_tutorial_window;
        egui::Window::new("Tutorial")
            .vscroll(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.heading("qcsim Tutorial");

                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut state.ui_state.active_tutorial_step,
                        TutorialStep::TutorialIntro,
                        "Introduction",
                    );
                    ui.selectable_value(
                        &mut state.ui_state.active_tutorial_step,
                        TutorialStep::SingleQubitGatesEntangledStates,
                        "Single Qubit Gates (Entangled)",
                    );
                    ui.selectable_value(
                        &mut state.ui_state.active_tutorial_step,
                        TutorialStep::SingleQubitGatesIntroduction,
                        "Single Qubit Gates (Intro)",
                    );
                    ui.selectable_value(
                        &mut state.ui_state.active_tutorial_step,
                        TutorialStep::MultiQubitGates,
                        "Multi Qubit Gates",
                    );
                    ui.selectable_value(
                        &mut state.ui_state.active_tutorial_step,
                        TutorialStep::Measurements,
                        "Measurements",
                    );
                    ui.selectable_value(
                        &mut state.ui_state.active_tutorial_step,
                        TutorialStep::CustomGates,
                        "Custom Gates",
                    );
                    ui.selectable_value(
                        &mut state.ui_state.active_tutorial_step,
                        TutorialStep::InitialStates,
                        "Initial States",
                    );
                    ui.selectable_value(
                        &mut state.ui_state.active_tutorial_step,
                        TutorialStep::Simulations,
                        "Simulations",
                    );
                    ui.selectable_value(
                        &mut state.ui_state.active_tutorial_step,
                        TutorialStep::Analytics,
                        "Analytics",
                    );
                    ui.selectable_value(
                        &mut state.ui_state.active_tutorial_step,
                        TutorialStep::Conclusion,
                        "Conclusion",
                    );
                });

                ui.separator();

                let content = match state.ui_state.active_tutorial_step {
                    TutorialStep::TutorialIntro => {
                        include_str!("../../assets/docs/tutorials/tutorial_intro.md")
                    }
                    TutorialStep::SingleQubitGatesEntangledStates => include_str!(
                        "../../assets/docs/tutorials/single_qubit_gates_entangled_states.md"
                    ),
                    TutorialStep::SingleQubitGatesIntroduction => include_str!(
                        "../../assets/docs/tutorials/single_qubit_gates_introduction.md"
                    ),
                    TutorialStep::MultiQubitGates => {
                        include_str!("../../assets/docs/tutorials/multi_qubit_gates.md")
                    }
                    TutorialStep::Measurements => {
                        include_str!("../../assets/docs/tutorials/measurements.md")
                    }
                    TutorialStep::CustomGates => {
                        include_str!("../../assets/docs/tutorials/custom_gates.md")
                    }
                    TutorialStep::InitialStates => {
                        include_str!("../../assets/docs/tutorials/initial_states.md")
                    }
                    TutorialStep::Simulations => {
                        include_str!("../../assets/docs/tutorials/simulations.md")
                    }
                    TutorialStep::Analytics => {
                        include_str!("../../assets/docs/tutorials/analytics.md")
                    }
                    TutorialStep::Conclusion => {
                        include_str!("../../assets/docs/tutorials/conclusion.md")
                    }
                };

                if has_figures_for_step(state.ui_state.active_tutorial_step) {
                    egui::SidePanel::right("tutorial_figures_panel")
                        .resizable(true)
                        .default_width(400.0)
                        .show_inside(ui, |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                show_tutorial_figures(ui, state.ui_state.active_tutorial_step);
                            });
                        });
                }

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        use egui_commonmark::CommonMarkCache;
                        let mut cache = CommonMarkCache::default();
                        CommonMarkViewer::new().show(ui, &mut cache, content);
                    });
                });
            });
        state.ui_state.show_tutorial_window = open;
    }
}

fn has_figures_for_step(step: TutorialStep) -> bool {
    matches!(
        step,
        TutorialStep::SingleQubitGatesEntangledStates | TutorialStep::TutorialIntro
    )
}

fn show_tutorial_figures(ui: &mut egui::Ui, step: TutorialStep) {
    ui.heading("Figures");

    match step {
        TutorialStep::SingleQubitGatesEntangledStates => {
            //ui.label("Pairing for B");
            //ui.add(egui::Image::new(egui::include_image!("../../../assets/docs/tutorials/t1/pairing_B_diagram.png")).fit_to_exact_size(Vec2::new(400.0, 200.0)));
            ui.label("Probabilities BEFORE H on B");
            ui.add(
                egui::Image::new(egui::include_image!(
                    "../../assets/docs/tutorials/t1/prob_before_H_B.png"
                ))
                .fit_to_exact_size(Vec2::new(400.0, 200.0)),
            );
            ui.label("Probabilities AFTER H on B");
            ui.add(
                egui::Image::new(egui::include_image!(
                    "../../assets/docs/tutorials/t1/prob_after_H_B.png"
                ))
                .fit_to_exact_size(Vec2::new(400.0, 200.0)),
            );
            ui.label("Bell correlations");
            ui.add(
                egui::Image::new(egui::include_image!(
                    "../../assets/docs/tutorials/t1/bell_correlation.png"
                ))
                .fit_to_exact_size(Vec2::new(400.0, 200.0)),
            );
            ui.label("Bell collapse tree");
            ui.add(
                egui::Image::new(egui::include_image!(
                    "../../assets/docs/tutorials/t1/bell_collapse_tree.png"
                ))
                .fit_to_exact_size(Vec2::new(400.0, 300.0)),
            );
        }
        TutorialStep::TutorialIntro => {
            ui.label("Top Panel Overview");
            ui.add(
                egui::Image::new(egui::include_image!(
                    "../../assets/docs/tutorials/tutorial_intro_figures/top_panel2.png"
                ))
                .fit_to_exact_size(Vec2::new(200.0, 200.0)),
            );
            ui.separator();
            ui.label("Gate Palette");
            ui.add(
                egui::Image::new(egui::include_image!(
                    "../../assets/docs/tutorials/tutorial_intro_figures/gate_palette2.png"
                ))
                .fit_to_exact_size(Vec2::new(400.0, 200.0)),
            );
            ui.separator();
            ui.label("Circuit Grid and Gate Placement");
            ui.add(
                egui::Image::new(egui::include_image!(
                    "../../assets/docs/tutorials/tutorial_intro_figures/circuit_grid2.png"
                ))
                .fit_to_exact_size(Vec2::new(400.0, 400.0)),
            );
            ui.separator();
            ui.label("Simulation Results Panel");
            ui.add(egui::Image::new(egui::include_image!("../../assets/docs/tutorials/tutorial_intro_figures/simulation_results_panel2.png")).fit_to_exact_size(Vec2::new(400.0, 400.0)));
            ui.separator();
            ui.label("Step-by-step Gate Placement");
            ui.add(egui::Image::new(egui::include_image!("../../assets/docs/tutorials/tutorial_intro_figures/step_by_step_gate_placement2.png")).fit_to_exact_size(Vec2::new(400.0, 400.0)));
            ui.separator();
            ui.label("JSON Editor Interface and Copy/Paste Buttons");
            ui.add(
                egui::Image::new(egui::include_image!(
                    "../../assets/docs/tutorials/tutorial_intro_figures/json_editor2.png"
                ))
                .fit_to_exact_size(Vec2::new(400.0, 400.0)),
            );
            ui.separator();
            ui.label("Time Scrubber in Action");
            ui.add(
                egui::Image::new(egui::include_image!(
                    "../../assets/docs/tutorials/tutorial_intro_figures/time_scrubber2.png"
                ))
                .fit_to_exact_size(Vec2::new(200.0, 200.0)),
            );
            ui.separator();
            ui.label("Initial State Editor");
            ui.add(
                egui::Image::new(egui::include_image!(
                    "../../assets/docs/tutorials/tutorial_intro_figures/initial_state_editor2.png"
                ))
                .fit_to_exact_size(Vec2::new(400.0, 400.0)),
            );
            ui.separator();
            ui.label("Gate Editor with Bloch Sphere");
            ui.add(egui::Image::new(egui::include_image!("../../assets/docs/tutorials/tutorial_intro_figures/gate_editor_bloch_sphere2.png")).fit_to_exact_size(Vec2::new(400.0, 400.0)));
        }
        // Other
        _ => {}
    }
}
