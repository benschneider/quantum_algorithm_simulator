use crate::messages::Message;
use crate::state::AppState;
use egui_plot::{Bar, BarChart, Corner, HLine, Plot, Points};
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
                ui.monospace(format!("q{wire}"));
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
            "{ket}  {:+.4}{:+.4}i  p={:.4}",
            amp.re,
            amp.im,
            amp.norm_sqr()
        ));
    }

    ui.separator();
    ui.strong("Joint probabilities");
    ui.small("Bars show the states on this page. Hover for amplitude and phase.");
    probability_plot(ui, quantum_state, page);
    message
}

fn probability_plot(ui: &mut egui::Ui, state: &QuantumState, page: &[usize]) {
    let max_probability = page
        .iter()
        .map(|&i| state.state_vector[i].norm_sqr())
        .fold(0.0, f32::max);
    let y_top = (1.1 * f64::from(max_probability)).clamp(f64::EPSILON, 1.0);
    let max_amp = f64::from(max_probability.sqrt()).max(f64::EPSILON);
    let has_phase = page
        .iter()
        .any(|&i| state.state_vector[i].arg().abs() > 1e-6);
    let mut bars = Vec::with_capacity(page.len());
    let mut phase_points = Vec::with_capacity(page.len());
    let mut re_points = Vec::with_capacity(page.len());
    let mut im_points = Vec::with_capacity(page.len());

    // Each populated state gets one column so widely separated basis indices are legible.
    for (column, &index) in page.iter().enumerate() {
        let amp = state.state_vector[index];
        let probability = amp.norm_sqr();
        let phase = amp.arg();
        let x = column as f64;
        let ket = bitstring(index, state.num_qubits);
        bars.push(
            Bar::new(x, f64::from(probability))
                .name(format!("{ket} (index {index})\n{:+.4}{:+.4}i\np={probability:.4}\nphase={phase:.4} rad", amp.re, amp.im))
                .width(0.85)
                .fill(phase_color(phase, has_phase))
                .stroke(egui::Stroke::new(1.0_f32, egui::Color32::WHITE)),
        );
        let phase_y = 0.5 * (1.0 + f64::from(phase) / std::f64::consts::PI) * y_top;
        phase_points.push([x, phase_y]);
        re_points.push([x, 0.5 * (1.0 + f64::from(amp.re) / max_amp) * y_top]);
        im_points.push([x, 0.5 * (1.0 + f64::from(amp.im) / max_amp) * y_top]);
    }

    let chart = BarChart::new("Probabilities", bars)
        .color(egui::Color32::TRANSPARENT)
        .element_formatter(Box::new(|bar, _| bar.name.clone()));

    Plot::new("probabilities_plot")
        .view_aspect(1.6)
        .legend(egui_plot::Legend::default().position(Corner::LeftTop))
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
        .include_y(0.0)
        .include_y(y_top)
        .allow_boxed_zoom(false)
        .show(ui, |plot_ui| {
            let midline = HLine::new("", y_top * 0.5)
                .color(egui::Color32::from_gray(96))
                .allow_hover(false);
            plot_ui.hline(midline);
            for (name, points, color) in [
                (
                    "phase (scaled)",
                    phase_points,
                    egui::Color32::from_rgb(240, 180, 80),
                ),
                (
                    "Re(amp) (scaled)",
                    re_points,
                    egui::Color32::from_rgb(160, 220, 255),
                ),
                (
                    "Im(amp) (scaled)",
                    im_points,
                    egui::Color32::from_rgb(140, 200, 180),
                ),
            ] {
                plot_ui.points(
                    Points::new(name, points)
                        .radius(2.5_f32)
                        .color(color)
                        .allow_hover(false),
                );
            }
            plot_ui.bar_chart(chart);
        });
}

fn bitstring(idx: usize, width: usize) -> String {
    format!("|{idx:0width$b}⟩")
}

fn phase_color(phase: f32, has_phase: bool) -> egui::Color32 {
    if has_phase {
        let hue = (phase + std::f32::consts::PI) / std::f32::consts::TAU;
        egui::Rgba::from_rgba_unmultiplied(hue, 0.85, 0.95, 1.0).into()
    } else {
        egui::Color32::from_rgb(80, 180, 255)
    }
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
            "24",
            "26",
        ] {
            assert!(
                labels.contains(&expected),
                "Missing {expected} in {labels:?}"
            );
        }
    }
}
