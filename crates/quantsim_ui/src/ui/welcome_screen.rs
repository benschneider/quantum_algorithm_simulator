use crate::state::{AppState, ui_state::WelcomeStep};
use egui::Context;
use egui::{Color32, Pos2};
use egui_plot::{Bar, BarChart, Legend, Plot};

pub fn render(state: &mut AppState, ctx: &Context) {
    if state.ui_state.show_welcome_screen {
        let mut open = state.ui_state.show_welcome_screen;
        let mut close_welcome_screen_on_next_frame = false; // New flag

        egui::Window::new("Welcome to quantsim a quantum algorithm simulator!")
            .open(&mut open)
            .show(ctx, |ui| {

                egui::ScrollArea::vertical().show(ui, |ui| {
                    match state.ui_state.active_welcome_step {
                        WelcomeStep::Welcome => {
                            ui.heading("Welcome to quantsim a quantum algorithm simulator!");

                            // Animated hero: two orbs connected by a shimmering tether
                            ui.add_space(6.0);
                            let (resp, painter) = ui.allocate_painter(egui::vec2(ui.available_width(), 120.0), egui::Sense::hover());
                            let rect = resp.rect;
                            let t = ui.input(|i| i.time) as f32;
                            // Positions for two orbs (A on left, B on right)
                            let left = Pos2::new(rect.left() + 60.0, rect.center().y);
                            let right = Pos2::new(rect.right() - 60.0, rect.center().y);
                            let orb_r = 18.0;
                            // Tether: draw multiple sine-offset polylines for glow
                            let segments = 64;
                            for k in 0..3 {
                                let phase = t * (1.0 + k as f32 * 0.2) + k as f32 * 1.2;
                                let color = match k { 0 => Color32::from_rgb(120, 230, 255), 1 => Color32::from_rgb(120, 180, 255), _ => Color32::from_rgb(90, 140, 220) };
                                let mut pts: Vec<Pos2> = Vec::with_capacity(segments + 1);
                                for i in 0..=segments {
                                    let u = i as f32 / segments as f32;
                                    let x = egui::lerp(left.x..=right.x, u);
                                    let wobble = (u * std::f32::consts::TAU * 2.0 + phase).sin() * 6.0;
                                    let y = rect.center().y + wobble;
                                    pts.push(Pos2::new(x, y));
                                }
                                painter.add(egui::Shape::line(pts, egui::Stroke::new(2.0 - k as f32 * 0.4, color)));
                            }
                            // Orbs with pulsing glow
                            let glow = (t * 1.8).sin().abs();
                            let col_a: Color32 = egui::Rgba::from_rgba_unmultiplied(0.55, 0.9, 1.0, 0.6 + 0.2 * glow).into();
                            let col_b: Color32 = egui::Rgba::from_rgba_unmultiplied(0.2, 0.8, 1.0, 0.6 + 0.2 * (1.0 - glow)).into();
                            painter.circle_filled(left, orb_r, col_a);
                            painter.circle_filled(right, orb_r, col_b);
                            // keep animating
                            ui.ctx().request_repaint();
                            ui.add_space(4.0);

                            ui.label("qcsim is a data-driven quantum circuit simulator:");
                            ui.add_space(2.0);
                            egui::Grid::new("welcome_bullets").num_columns(2).spacing([6.0, 2.0]).show(ui, |ui| {
                                ui.label("•"); ui.label("Build small circuits interactively"); ui.end_row();
                                ui.label("•"); ui.label("See how gates reshape state vectors"); ui.end_row();
                                ui.label("•"); ui.label("Inspect probabilities, phases, and interference"); ui.end_row();
                            });
                            ui.add_space(6.0);
                            ui.collapsing("Quick peek: probabilities of a simple Bell state", |ui| {
                                let mut bars = Vec::new();
                                // Toy preview: |00> and |11> with prob 0.5 each (indices 0 and 3)
                                let mut push_bar = |idx: usize, p: f32, c: Color32| {
                                    bars.push(Bar::new(idx as f64, p as f64).fill(c).width(0.9));
                                };
                                push_bar(0, 0.5, Color32::from_rgb(120, 220, 255));
                                push_bar(3, 0.5, Color32::from_rgb(90, 200, 230));
                                let chart = BarChart::new("Probabilities", bars).color(Color32::TRANSPARENT);
                                Plot::new("welcome_preview_plot")
                                    .legend(Legend::default())
                                    .include_x(-0.5)
                                    .include_x(3.5)
                                    .include_y(0.0)
                                    .include_y(0.6)
                                    .x_axis_label("Basis state index")
                                    .y_axis_label("Probability")
                                    .height(160.0)
                                    .show(ui, |plot_ui| {
                                        plot_ui.bar_chart(chart);
                                    });
                            });

                            ui.add_space(8.0);
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Start tutorial").clicked() {
                                    state.ui_state.show_tutorial_window = true;
                                    state.ui_state.active_tutorial_step = crate::state::ui_state::TutorialStep::TutorialIntro;
                                    // Close welcome after opening tutorial
                                    close_welcome_screen_on_next_frame = true;
                                }
                                if ui.button("Go to main interface").clicked() {
                                    // Close on next frame via flag used below
                                    close_welcome_screen_on_next_frame = true;
                                }
                                //ui.separator();
                                //if ui.link("Open documentation website").clicked() {
                                    // If you have a docs URL, open it via `open` feature or show a copyable link.
                                //}
                            });
                            ui.add_space(6.0);
                            ui.separator();
                            ui.horizontal_wrapped(|ui| {
                                ui.label("© 2025 Ben Schneider");
                                ui.separator();
                                ui.label("Apache License 2.0 / MIT License");
                                ui.hyperlink_to("(view)", "https://www.apache.org/licenses/LICENSE-2.0");
                            });
                            ui.add_space(4.0);
                            ui.collapsing("Attribution (optional)", |ui| {
                                ui.small("Visible credit is appreciated but not required by Apache-2.0.");
                                let credit = "Includes qcsim (Apache-2.0 / MIT) by Ben Schneider — https://github.com/benschneider/quantum_algorithm_simulator";
                                ui.monospace(credit);
                                if ui.button("Copy attribution line").clicked() { ui.ctx().copy_text(credit.to_owned()); }
                            });

                        }
                        WelcomeStep::License => {}
                        WelcomeStep::TutorialOffer => {
                            // Small animated accent
                            let (resp2, painter2) = ui.allocate_painter(egui::vec2(ui.available_width(), 40.0), egui::Sense::hover());
                            let rect2 = resp2.rect; let t2 = ui.input(|i| i.time) as f32;
                            let mid = rect2.center();
                            let r = 10.0 + (t2 * 3.0).sin().abs() * 6.0;
                            painter2.circle_stroke(mid, r, egui::Stroke::new(2.0, Color32::from_rgb(120, 220, 255)));
                            ui.ctx().request_repaint();

                            ui.heading("Get Started");
                            ui.small("qcsim is distributed under the Apache License 2.0 and MIT License.");
                            ui.label("Would you like to take a quick tutorial to get started?");
                            ui.separator();
                            if ui.button("Yes, show me the tutorial").clicked() {
                                state.ui_state.show_tutorial_window = true;
                                state.ui_state.active_tutorial_step = crate::state::ui_state::TutorialStep::TutorialIntro;
                                close_welcome_screen_on_next_frame = true;
                            }
                            if ui.button("No, take me to the main interface").clicked() {
                                close_welcome_screen_on_next_frame = true;
                            }
                        }
                    }
                });
            });
        // Update the app state.
        // If any of the branches requested closing on next frame, do it here.
        if close_welcome_screen_on_next_frame {
            state.ui_state.show_welcome_screen = false;
        } else {
            state.ui_state.show_welcome_screen = open;
        }
    }
}
