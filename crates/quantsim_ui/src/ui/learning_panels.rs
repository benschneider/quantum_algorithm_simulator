use crate::components::results_panel::results_panel;
use crate::messages::Message;
use crate::state::AppState;

pub(crate) fn render(state: &mut AppState, ctx: &egui::Context, messages: &mut Vec<Message>) {
    let viewport = ctx.available_rect();
    let mut tutorial = state.ui_state.show_tutorial_window;
    let mut results = state.ui_state.show_results_window;

    // The two desktop windows need about 920 points side by side.
    if viewport.width() < 920.0 {
        let height = viewport.height();
        let both = tutorial && results;
        // Bottom panels are allocated outside-in, leaving the circuit above the lesson.
        if results {
            egui::TopBottomPanel::bottom("stacked_results")
                .exact_height(height * if both { 0.36 } else { 0.45 })
                .show(ctx, |ui| {
                    panel_header(ui, "Results", &mut results);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        messages.extend(results_panel(ui, state));
                    });
                });
        }
        if tutorial {
            egui::TopBottomPanel::bottom("stacked_tutorial")
                .exact_height(height * if both { 0.32 } else { 0.45 })
                .show(ctx, |ui| {
                    panel_header(ui, "Tutorial", &mut tutorial);
                    super::tutorial::contents(state, ui, messages);
                });
            state.ui_state.show_tutorial_window = tutorial;
        }
    } else {
        if results {
            egui::Window::new("Results")
                .default_pos(egui::pos2(viewport.right() - 380.0, viewport.top() + 280.0))
                .default_size(egui::vec2(350.0, 300.0))
                .vscroll(true)
                .open(&mut results)
                .show(ctx, |ui| messages.extend(results_panel(ui, state)));
        }
        super::tutorial::render(state, ctx, messages);
    }
    state.ui_state.show_results_window = results;
}

fn panel_header(ui: &mut egui::Ui, title: &str, open: &mut bool) {
    ui.horizontal(|ui| {
        ui.strong(title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .small_button("Close")
                .on_hover_text(format!("Close {title}"))
                .clicked()
            {
                *open = false;
            }
        });
    });
    ui.separator();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::QCSimApp;

    #[test]
    fn narrow_viewports_keep_the_circuit_lesson_and_readout_in_separate_rows() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::LoadTemplate("bell.json".into()));
        app.handle_message(Message::RunSimulation);
        app.state.ui_state.show_tutorial_window = true;
        let ctx = egui::Context::default();
        for size in [egui::vec2(780.0, 480.0), egui::vec2(390.0, 844.0)] {
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ctx| {
                    crate::ui::top_panel::render(&mut app.state, ctx);
                    render(&mut app.state, ctx, &mut app.messages);
                    let circuit = ctx.available_rect();
                    let panel = |id| {
                        egui::containers::panel::PanelState::load(ctx, egui::Id::new(id))
                            .unwrap()
                            .rect
                    };
                    let tutorial = panel("stacked_tutorial");
                    let results = panel("stacked_results");
                    assert!(circuit.bottom() <= tutorial.top());
                    assert!(tutorial.bottom() <= results.top());
                    for rect in [circuit, tutorial, results] {
                        assert!(screen.contains_rect(rect));
                        assert!(rect.width() >= size.x - 1.0 && rect.height() > size.y * 0.25);
                    }
                },
            );
        }
    }
}
