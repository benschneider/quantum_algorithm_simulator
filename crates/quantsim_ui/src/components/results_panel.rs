use crate::messages::Message;
use crate::state::simulation_state::{CompactSamples, COMPACT_SHOTS};
use crate::state::AppState;
use egui_plot::{Bar, BarChart, GridMark, Plot};
use quantsim_core::{circuit::QubitRole, core::engine::QuantumState, BasisIndex};

mod overview;
mod waveform;

pub fn results_panel(ui: &mut egui::Ui, state: &mut AppState) -> Option<Message> {
    let circuit = &state.circuit_state.circuit;
    let simulation = &mut state.simulation_state;
    let step = state.ui_state.current_timestep;
    if !simulation.has_readout() {
        ui.label(format!("No readout for step {step}."));
        return ui
            .button(format!("Read step {step}"))
            .clicked()
            .then_some(Message::SelectTimestep(step));
    }
    if step == 0 {
        ui.heading("Step 0 · Initial state");
    } else if let Some(section) = circuit.section_at(step) {
        ui.heading(format!("After step {step} · {}", section.label));
        let description = section.description.trim();
        if !description.is_empty() {
            ui.add(egui::Label::new(description).wrap());
        }
    } else {
        ui.heading(format!("After step {step}"));
    }

    // Results moves between a window and a bottom panel on resize.
    let mode_id = egui::Id::new("results_probability_overview");
    let previous_mode = ui.data(|data| data.get_temp::<bool>(mode_id).unwrap_or(false));
    let mut show_overview = previous_mode;
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(&mut show_overview, false, "Amplitude waveform");
        ui.selectable_value(&mut show_overview, true, "Overview");
        if show_overview {
            if simulation.compact_samples.is_some() {
                ui.label(format!("Sampled · {COMPACT_SHOTS} shots"));
            } else {
                ui.label("Exact probabilities");
            }
        } else {
            waveform::legend(ui);
        }
    });
    if previous_mode != show_overview {
        ui.data_mut(|data| data.insert_temp(mode_id, show_overview));
    }
    if show_overview {
        overview::show(ui, simulation);
    } else {
        waveform::show(ui, simulation);
    }
    if let Some(samples) = &simulation.compact_samples {
        show_joint_samples(ui, samples, circuit.num_qubits);
        ui.separator();
    }

    ui.strong("Wire readout");
    ui.small("P(1) is the exact probability of measuring 1 in the computational basis.");
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
                    ui.label(
                        label
                            .strong()
                            .background_color(ui.visuals().selection.bg_fill),
                    )
                    .on_hover_text("Selected in Qubit layout");
                } else {
                    ui.label(label);
                }
                ui.label(role);
                ui.monospace(format!("{:.2}%", probability * 100.0));
                ui.end_row();
            }
        });

    let quantum_state = simulation.quantum_state.as_ref()?;
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

fn show_joint_samples(ui: &mut egui::Ui, samples: &CompactSamples, width: usize) {
    ui.strong("Joint samples");
    let shown = samples.rows.len();
    if samples.distinct > shown {
        ui.small(format!(
            "{COMPACT_SHOTS} shots · {shown} most frequent of {} observed outcomes",
            samples.distinct
        ));
    } else {
        let noun = if shown == 1 { "outcome" } else { "outcomes" };
        ui.small(format!("{COMPACT_SHOTS} shots · {shown} observed {noun}"));
    }

    egui::Grid::new("joint_samples")
        .max_col_width((ui.available_width() - 60.0).max(64.0))
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Shots");
            ui.strong(format!("Outcome (q{} … q0)", width - 1));
            ui.end_row();
            for (outcome, count) in &samples.rows {
                ui.monospace(count.to_string());
                ui.add(
                    egui::Label::new(egui::RichText::new(grouped_bits(outcome, width)).monospace())
                        .wrap(),
                );
                ui.end_row();
            }
            if samples.distinct > shown {
                let displayed_shots: u32 = samples.rows.iter().map(|&(_, count)| count).sum();
                ui.monospace((COMPACT_SHOTS - displayed_shots).to_string());
                ui.label(format!(
                    "Other {} observed outcomes",
                    samples.distinct - shown
                ));
                ui.end_row();
            }
        });
}

fn grouped_bits(outcome: &BasisIndex, width: usize) -> String {
    let bits = format!("{outcome:0width$b}");
    let mut grouped = String::with_capacity(bits.len() + bits.len().saturating_sub(1) / 8);
    for (i, bit) in bits.bytes().enumerate() {
        if i != 0 && (bits.len() - i).is_multiple_of(8) {
            grouped.push(' ');
        }
        grouped.push(char::from(bit));
    }
    grouped
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
    fn overview_selection_survives_new_readouts_and_resizing() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::ChangeQubits(6));
        for wire in 0..6 {
            app.handle_message(Message::PlaceGate(Gate::X, wire, 0));
        }
        app.handle_message(Message::SelectTimestep(0));
        let ctx = egui::Context::default();
        let render = |width, events, state: &mut AppState| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 850.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        // The application reparents Results at its layout breakpoint.
                        ui.push_id(width > 400.0, |ui| results_panel(ui, state));
                    });
                },
            )
        };
        let label = |output: &egui::FullOutput, needle: &str| {
            output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Text(t) if t.galley.text() == needle => {
                    Some(t.visual_bounding_rect().center())
                }
                _ => None,
            })
        };
        let click = |pos| {
            let mut events = vec![egui::Event::PointerMoved(pos)];
            events.extend([true, false].map(|pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }));
            events
        };
        render(740.0, vec![], &mut app.state);
        let initial = render(740.0, vec![], &mut app.state);
        assert!(label(&initial, "Real").is_some());
        let pos = label(&initial, "Overview").unwrap();
        render(740.0, click(pos), &mut app.state);
        let wide = render(740.0, vec![], &mut app.state);
        assert!(label(&wide, "Leading 5 bits of basis state").is_some());
        assert!(label(&wide, "Exact probabilities").is_some());
        assert!(label(&wide, "Real").is_none());
        let top = label(&wide, "00000").unwrap();
        let mid = label(&wide, "50%").unwrap();
        render(740.0, click(egui::pos2(top.x, mid.y)), &mut app.state);
        let pinned = render(740.0, vec![egui::Event::PointerGone], &mut app.state);
        assert!(label(&pinned, "Indices 0–1 · 100.00%").is_some());

        app.handle_message(Message::SelectTimestep(1));
        let medium = render(365.0, vec![], &mut app.state);
        assert!(label(&medium, "Leading 4 bits of basis state").is_some());
        let narrow = render(300.0, vec![], &mut app.state);
        assert!(label(&narrow, "Leading 3 bits of basis state").is_some());
        assert!(label(&narrow, "Indices 0–7 · 0.00%").is_some());
        let restored = render(740.0, vec![], &mut app.state);
        assert!(label(&restored, "Indices 0–1 · 0.00%").is_some());

        // A pin originating at 16 bars cannot identify which of its two
        // children to select at 32 bars. Only the right child has mass here.
        let medium = render(365.0, vec![], &mut app.state);
        let last = label(&medium, "1111").unwrap();
        let mid = label(&medium, "50%").unwrap();
        render(365.0, click(egui::pos2(last.x, mid.y)), &mut app.state);
        let pinned = render(365.0, vec![egui::Event::PointerGone], &mut app.state);
        assert!(label(&pinned, "Indices 60–63 · 100.00%").is_some());
        let wide = render(740.0, vec![], &mut app.state);
        assert!(label(&wide, "Select a bar to inspect its probability.").is_some());
        let pos = label(&wide, "Amplitude waveform").unwrap();
        render(740.0, click(pos), &mut app.state);
        let waveform = render(740.0, vec![], &mut app.state);
        assert!(label(&waveform, "Real").is_some());
        assert!(label(&waveform, "Exact probabilities").is_none());
    }

    #[test]
    fn narrow_compact_results_show_sample_counts_and_wrap_all_outcome_bits() {
        for width in [64, 128, 200] {
            check_narrow_sample_table(width);
        }
    }

    fn check_narrow_sample_table(width: usize) {
        use crate::state::SimulationState;
        use quantsim_core::{
            circuit::{Circuit, CircuitSection},
            types::Operation,
        };

        let description = "The high bit stays set while Hadamard gates spread the amplitude evenly across the low twelve bits.";
        let mut circuit = Circuit::new(width);
        let mut column = vec![Operation::new(Gate::X, vec![(width - 1) as u32], vec![])];
        column.extend((0..12).map(|wire| Operation::new(Gate::H, vec![wire], vec![])));
        circuit.steps.push(column);
        circuit
            .set_section(
                None,
                CircuitSection {
                    start: 1,
                    end: 1,
                    label: "Spread the amplitude".into(),
                    description: description.into(),
                },
            )
            .unwrap();
        let state = circuit.run_stabilizer_prefix(1).unwrap();
        let mut app = QCSimApp::default();
        app.state.circuit_state.circuit = circuit;
        app.state.ui_state.current_timestep = 1;
        app.state.simulation_state = SimulationState::from_stabilizer(&state, width);

        let ctx = egui::Context::default();
        let mut render = || {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(360.0, 3000.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| {
                            results_panel(ui, &mut app.state);
                        })
                        .inner
                },
            )
        };
        render();
        let output = render();
        let samples = app.state.simulation_state.compact_samples.as_ref().unwrap();
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text),
                _ => None,
            })
            .collect();
        let label = |needle: &str| {
            texts
                .iter()
                .find(|text| text.galley.text() == needle)
                .unwrap_or_else(|| panic!("missing label {needle}"))
        };
        let caption = label(description);
        assert!(label("After step 1 · Spread the amplitude").pos.y < caption.pos.y);
        assert!(caption.visual_bounding_rect().bottom() < label("Amplitude waveform").pos.y);
        assert!(caption.galley.rows.len() > 1);
        assert!(caption.visual_bounding_rect().right() <= 360.0);
        assert!(label("Amplitude waveform").pos.y < label("Joint samples").pos.y);
        assert!(label("Joint samples").pos.y < label("Wire readout").pos.y);
        label(&format!("Outcome (q{} … q0)", width - 1));
        label(&format!(
            "1024 shots · 16 most frequent of {} observed outcomes",
            samples.distinct
        ));
        let displayed: u32 = samples.rows.iter().map(|&(_, n)| n).sum();
        label(&(COMPACT_SHOTS - displayed).to_string());
        label(&format!(
            "Other {} observed outcomes",
            samples.distinct - 16
        ));
        for (bits, _) in &samples.rows {
            let text = label(&grouped_bits(bits, width));
            assert!(text.galley.text().starts_with("10000000 "));
            assert_eq!(
                text.galley.text().replace(' ', ""),
                format!("{bits:0width$b}")
            );
            assert!(text.galley.rows.len() > 1);
            assert!(text.visual_bounding_rect().right() <= 360.0);
        }
        assert!(texts
            .iter()
            .all(|text| !matches!(text.galley.text(), "Joint state" | "Joint probabilities")));
    }

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
                    description: String::new(),
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
                        // Exercise both the amplitude and probability plots in one scrollable result.
                        egui::vec2(600.0, 1100.0),
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
