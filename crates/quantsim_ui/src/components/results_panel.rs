use crate::messages::Message;
use crate::state::AppState;
use egui_plot::{Bar, BarChart, GridMark, Plot};
use quantsim_core::{circuit::QubitRole, core::engine::QuantumState};

pub fn results_panel(ui: &mut egui::Ui, state: &mut AppState) -> Option<Message> {
    let circuit = &state.circuit_state.circuit;
    let simulation = &mut state.simulation_state;
    let step = state.ui_state.current_timestep;
    let Some(quantum_state) = &simulation.quantum_state else {
        ui.label(format!("No readout for step {step}."));
        return ui
            .button(format!("Read step {step}"))
            .clicked()
            .then_some(Message::SelectTimestep(step));
    };
    if step == 0 {
        ui.heading("Step 0 · Initial state");
    } else if let Some(section) = circuit.section_at(step) {
        ui.heading(format!("After step {step} · {}", section.label));
    } else {
        ui.heading(format!("After step {step}"));
    }

    ui.strong("Wire readout");
    ui.small("P(1) is the probability of measuring 1 in the computational basis.");
    egui::Grid::new("wire_readout")
        .striped(true)
        .show(ui, |ui| {
            for title in ["Wire", "Role", "P(1)"] {
                ui.strong(title);
            }
            ui.end_row();
            for (wire, probability) in simulation.wire_one_probabilities.iter().enumerate() {
                let role = match circuit.qubit_roles.get(wire).copied().flatten() {
                    Some(QubitRole::Data) => "Data",
                    Some(QubitRole::Ancilla) => "Ancilla",
                    None => "Unlabeled",
                };
                let label = egui::RichText::new(format!("q{wire}")).monospace();
                if state.ui_state.selected_layout_wire == Some(wire) {
                    ui.label(label.strong().background_color(ui.visuals().selection.bg_fill))
                        .on_hover_text("Selected in Qubit layout");
                } else {
                    ui.label(label);
                }
                ui.label(role);
                ui.monospace(format!("{:.2}%", probability * 100.0));
                ui.end_row();
            }
        });

    ui.separator();
    ui.strong("Joint state");
    ui.small("Each ket shows q0 at the right. Amplitudes below the display threshold are hidden.");
    let pagination = &mut simulation.pagination;
    let message = ui
        .horizontal(|ui| pagination.render_controls(ui, &mut simulation.page_input_text))
        .inner
        .map(|msg| match msg {
            Message::InitialStateEditorPageChanged(page) => {
                Message::SimulationResultsPageChanged(page)
            }
            other => other,
        });

    let page =
        &simulation.populated_indices[pagination.get_start_index()..pagination.get_end_index()];
    for &index in page {
        let amp = quantum_state.state_vector[index];
        let ket = bitstring(index, quantum_state.num_qubits);
        ui.monospace(format!(
            "{ket}  {:+.4}{:+.4}i  p={}",
            amp.re,
            amp.im,
            format_probability(amp.norm_sqr())
        ));
    }

    ui.separator();
    ui.strong("Joint probabilities");
    ui.small("Bar color shows phase. Hover over a state for its amplitude and phase.");
    probability_plot(ui, quantum_state, page);
    message
}

fn probability_plot(
    ui: &mut egui::Ui,
    state: &QuantumState,
    page: &[usize],
) -> egui_plot::PlotResponse<()> {
    // Each populated state gets one column so widely separated basis indices are legible.
    let bars = page
        .iter()
        .enumerate()
        .map(|(column, &index)| {
            let amp = state.state_vector[index];
            let percent = amp.norm_sqr() * 100.0;
            let phase = amp.arg();
            let ket = bitstring(index, state.num_qubits);
            // Zero phase is blue. Opposite phases lie half a turn apart on the color wheel.
            let hue = (0.58 + phase / std::f32::consts::TAU).rem_euclid(1.0);
            Bar::new(column as f64, f64::from(percent))
                .name(format!(
                    "{ket} (index {index})\nProbability {}%\nAmplitude {:+.4}{:+.4}i\nPhase {phase:.4} rad",
                    format_probability(percent), amp.re, amp.im
                ))
                .width(0.85)
                .fill(egui::ecolor::Hsva::new(hue, 0.65, 0.95, 1.0))
        })
        .collect();
    let chart = BarChart::new("Joint probability", bars)
        .element_formatter(Box::new(|bar, _| bar.name.clone()));

    // Keep horizontal zoom while inspecting the same basis states, and fit new ones on arrival.
    let page_id = egui::Id::new((state.num_qubits, page));
    let last_page_id = ui.make_persistent_id("joint_probability_page");
    let page_changed = ui.data_mut(|data| {
        let previous = data.get_temp_mut_or_insert_with(last_page_id, || page_id);
        std::mem::replace(previous, page_id) != page_id
    });
    let plot = Plot::new("joint_probability_percent");
    let plot = if page_changed { plot.reset() } else { plot };
    plot.view_aspect(1.6)
        .x_axis_label("Basis state index")
        .show_x(false)
        .x_axis_formatter(|mark, _| {
            if mark.value >= 0.0 && mark.value.fract() == 0.0 {
                if let Some(index) = page.get(mark.value as usize) {
                    return index.to_string();
                }
            }
            String::new()
        })
        .y_axis_label("Probability")
        .default_y_bounds(0.0, 100.0)
        .y_grid_spacer(|_| {
            [0.0, 25.0, 50.0, 75.0, 100.0]
                .into_iter()
                .map(|value| GridMark {
                    value,
                    step_size: 25.0,
                })
                .collect()
        })
        .y_axis_formatter(|mark, _| format!("{:.0}%", mark.value))
        .allow_zoom([true, false])
        .allow_axis_zoom_drag([true, false])
        .allow_drag([true, false])
        .allow_scroll([true, false])
        .allow_boxed_zoom(false)
        .show(ui, |plot_ui| plot_ui.bar_chart(chart))
}

fn format_probability(value: f32) -> String {
    if value > 0.0 && value < 0.0001 {
        format!("{value:.3e}")
    } else {
        format!("{value:.4}")
    }
}

fn bitstring(idx: usize, width: usize) -> String {
    format!("|{idx:0width$b}⟩")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::QCSimApp;
    use quantsim_core::core::types::Gate;

    #[test]
    fn results_label_the_selected_section_and_sparse_plot_with_real_basis_indices() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::ChangeQubits(5));
        for wire in [3, 4] {
            app.handle_message(Message::PlaceGate(Gate::X, wire, 0));
            app.handle_message(Message::SetQubitRole {
                row: wire,
                role: Some(QubitRole::Ancilla),
            });
        }
        app.handle_message(Message::PlaceGate(Gate::H, 1, 0));
        app.state
            .circuit_state
            .circuit
            .set_section(
                None,
                quantsim_core::circuit::CircuitSection {
                    start: 1,
                    end: 1,
                    label: "Parity readout".into(),
                },
            )
            .unwrap();
        app.handle_message(Message::SelectTimestep(1));

        let ctx = egui::Context::default();
        let render = |app: &mut QCSimApp| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(600.0, 800.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        results_panel(ui, &mut app.state);
                    });
                },
            )
        };
        render(&mut app);
        let output = render(&mut app);
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        for expected in [
            "After step 1 · Parity readout",
            "q4",
            "Ancilla",
            "50.00%",
            "0%",
            "50%",
            "100%",
            "24",
            "26",
        ] {
            assert!(
                labels.contains(&expected),
                "Missing {expected} in {labels:?}"
            );
        }
    }

    #[test]
    fn probability_scale_is_fixed_across_steps_pages_and_zoom() {
        let ctx = egui::Context::default();
        let render = |state: &QuantumState, page: &[usize], events: Vec<egui::Event>| {
            let mut frame = egui::Rect::NOTHING;
            let mut x_bounds = [0.0; 2];
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 320.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let plot = probability_plot(ui, state, page);
                        let bounds = plot.transform.bounds();
                        assert_eq!((bounds.min()[1], bounds.max()[1]), (0.0, 100.0));
                        x_bounds = [bounds.min()[0], bounds.max()[0]];
                        frame = *plot.transform.frame();
                    });
                },
            );
            for &index in page {
                let height = frame.height() * state.state_vector[index].norm_sqr();
                assert!(
                    output.shapes.iter().any(|shape| {
                        matches!(&shape.shape, egui::Shape::Rect(bar)
                        if (bar.rect.bottom() - frame.bottom()).abs() < 0.1
                            && (bar.rect.height() - height).abs() < 0.1)
                    }),
                    "Expected a bar {height} points high in a {frame:?} plot"
                );
            }
            (frame, x_bounds)
        };
        let initial = QuantumState::new(2);
        let amplitudes = [0.5_f32.sqrt(), 0.5, 0.5, 0.0].map(|v| nalgebra::Complex::new(v, 0.0));
        let split = QuantumState::from_vector(2, &amplitudes).unwrap();
        let (_, single_bounds) = render(&initial, &[0], vec![]);
        let (_, pair_bounds) = render(&split, &[1, 2], vec![]);
        assert!(single_bounds[0] <= -0.425 && single_bounds[1] >= 0.425);
        assert!(pair_bounds[0] <= -0.425 && pair_bounds[1] >= 1.425);

        let (frame, _) = render(&initial, &[0], vec![]);
        let (_, zoomed) = render(
            &initial,
            &[0],
            vec![
                egui::Event::PointerMoved(frame.center()),
                egui::Event::Zoom(2.0),
            ],
        );
        assert!(zoomed[1] - zoomed[0] < single_bounds[1] - single_bounds[0]);
        assert_eq!(render(&initial, &[0], vec![]).1, zoomed);
        assert_eq!(render(&split, &[1, 2], vec![]).1, pair_bounds);
        assert_eq!(render(&initial, &[0], vec![]).1, single_bounds);
    }

    #[test]
    fn small_positive_probabilities_remain_distinguishable_from_zero() {
        assert_eq!(format_probability(0.0), "0.0000");
        assert_eq!(format_probability(0.5), "0.5000");
        assert_eq!(format_probability(1.0 / 32768.0), "3.052e-5");
    }
}
