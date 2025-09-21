use crate::messages::Message;
use crate::state::AppState;
use egui_plot::Corner;
use egui_plot::{Bar, BarChart, HLine, Plot, Points};
use nalgebra::Complex;

/// The `results_panel` module is responsible for rendering the simulation
/// results. This includes the raw state vector and a probability bar chart.
///
/// This module is a read-only view of the simulation state. It does not
/// modify the application state directly, but it can dispatch messages
/// to the message handlers to trigger changes in the application state
/// (e.g., changing the pagination).
pub fn results_panel(ui: &mut egui::Ui, state: &mut AppState) -> Option<Message> {
    // Change to mutable state and return Option<Message>
    let mut message = None;

    ui.heading("Results");
    ui.separator();

    if let Some(quantum_state) = &state.simulation_state.quantum_state {
        ui.heading("Raw State Vector");

        // ui mention of sparse representation

        ui.label(
            "The state vector is represented in a sparse format, showing only non-zero amplitudes.",
        );

        // Render pagination controls
        ui.horizontal(|ui| {
            if let Some(msg) = state
                .simulation_state
                .pagination
                .render_controls(ui, &mut state.simulation_state.page_input_text)
            {
                // Adjust message type for simulation results
                if let Message::InitialStateEditorPageChanged(page_num) = msg {
                    message = Some(Message::SimulationResultsPageChanged(page_num));
                } else {
                    message = Some(msg);
                }
            }
        });

        let non_zero_states: Vec<_> = quantum_state
            .state_vector
            .iter()
            .enumerate()
            .filter(|(_, state)| state.re.abs() > 1e-6 || state.im.abs() > 1e-6)
            .collect();

        // Precompute magnitude and phase (arg) for coloring
        let (mag_phase, phases_present): (Vec<(usize, f32, f32)>, bool) = {
            let mut any_phase = false;
            let v = quantum_state
                .state_vector
                .iter()
                .enumerate()
                .filter(|(_, c)| c.re.abs() > 1e-6 || c.im.abs() > 1e-6)
                .map(|(i, c)| {
                    let mag = (c.re * c.re + c.im * c.im).sqrt();
                    let phase = c.im.atan2(c.re); // [-pi, pi]
                    if phase.abs() > 1e-6 {
                        any_phase = true;
                    }
                    (i, mag, phase)
                })
                .collect();
            (v, any_phase)
        };

        state.simulation_state.pagination.total_entries = non_zero_states.len();
        state.simulation_state.pagination.total_pages = (non_zero_states.len() as f32
            / state.simulation_state.pagination.entries_per_page as f32)
            .ceil() as usize;

        let start_index = state.simulation_state.pagination.get_start_index();
        let end_index = state.simulation_state.pagination.get_end_index();

        for (i, state) in non_zero_states
            .iter()
            .skip(start_index)
            .take(end_index - start_index)
        {
            let label = bitstring(*i, quantum_state.num_qubits);
            let prob = state.re * state.re + state.im * state.im;
            ui.label(format!(
                "{}  {:+.4}{:+.4} i, p={:.4}",
                label, state.re, state.im, prob
            ));
        }

        ui.separator();

        let probabilities = quantum_state.probabilities();
        ui.heading("Probabilities");

        ui.small("Plot is paginated; hover for labels. Legend toggles the phase/Re/Im overlays.");

        // ==== Build paginated probabilities over ALL basis states (include zeros) ====
        let total_states: usize = 1usize << quantum_state.num_qubits;

        // Temporarily compute start/end for the full space using the existing paginator
        let old_total_entries = state.simulation_state.pagination.total_entries;
        let old_total_pages = state.simulation_state.pagination.total_pages;
        state.simulation_state.pagination.total_entries = total_states;
        state.simulation_state.pagination.total_pages = (total_states as f32
            / state.simulation_state.pagination.entries_per_page as f32)
            .ceil() as usize;

        let page_start = state.simulation_state.pagination.get_start_index();
        let page_end = state.simulation_state.pagination.get_end_index();

        // Restore paginator book-keeping for any subsequent UI that expects non-zero count
        state.simulation_state.pagination.total_entries = old_total_entries;
        state.simulation_state.pagination.total_pages = old_total_pages;

        // Build bars for the current page (including zero-prob states)
        let mut bars: Vec<Bar> = Vec::with_capacity(page_end.saturating_sub(page_start));
        for i in page_start..page_end {
            let p = probabilities.get(i).copied().unwrap_or(0.0);
            // Phase for coloring (default 0.0 for exact zeros)
            let phase = mag_phase
                .iter()
                .find(|(idx, _, _)| *idx == i)
                .map(|(_, _, ph)| *ph)
                .unwrap_or(0.0);
            let color = phase_color(phase, phases_present, p);
            let name = bitstring(i, quantum_state.num_qubits);
            bars.push(
                Bar::new(i as f64, p as f64)
                    .name(name)
                    .width(0.9)
                    .fill(color)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::WHITE)),
            );
        }

        let bit_width = quantum_state.num_qubits;

        let state_vector_clone = quantum_state.state_vector.clone();
        let chart = BarChart::new("Probabilities", bars)
            .color(egui::Color32::TRANSPARENT)
            .element_formatter(Box::new(
                move |bar: &Bar, _chart: &BarChart| {
                    let idx = bar.argument as usize;
                    let p = bar.value as f32;
                    let name = bitstring(idx, bit_width);
                    if let Some(c) = state_vector_clone.get(idx) {
                        format!(
                            "{}\n{:+.4}{:+.4} i\np={:.4}",
                            name, c.re, c.im, p
                        )
                    } else {
                        format!("{}\np={:.4}", name, p)
                    }
                },
            ));

        // Auto-scale Y to the current page max (with a small headroom)
        let mut page_max = 0.0f64;
        for i in page_start..page_end {
            let p = *probabilities.get(i).unwrap_or(&0.0) as f64;
            if p > page_max {
                page_max = p;
            }
        }
        let y_top = if page_max > 0.0 {
            (page_max * 1.1).min(1.0)
        } else {
            0.1
        };

        // Faint y-gridlines (no text labels)
        let mut y_grid: Vec<HLine> = Vec::new();
        let tick_count = 5;
        for k in 0..=tick_count {
            let yv = y_top * (k as f64) / (tick_count as f64);
            y_grid.push(
                HLine::new(String::new(), yv)
                    .color(egui::Color32::from_gray(64))
                    .width(1.0),
            );
        }

        // ---- Optional overlays: phase and complex parts (legend-togglable) ----
        // We scale these into the same Y range for visualization only; hover shows true values.
        let mut phase_points: Vec<[f64; 2]> = Vec::new();
        let mut re_points: Vec<[f64; 2]> = Vec::new();
        let mut im_points: Vec<[f64; 2]> = Vec::new();

        // Find max absolute amplitude (for scaling re/im to page)
        let mut max_abs_amp: f32 = 0.0;
        for i in page_start..page_end {
            let c = *quantum_state
                .state_vector
                .get(i)
                .unwrap_or(&Complex::<f32>::new(0.0, 0.0));
            let abs = (c.re * c.re + c.im * c.im).sqrt();
            if abs > max_abs_amp {
                max_abs_amp = abs;
            }
        }
        if max_abs_amp <= 1e-9 {
            max_abs_amp = 1.0;
        }

        // Build points (with scaling)
        for i in page_start..page_end {
            let c = *quantum_state
                .state_vector
                .get(i)
                .unwrap_or(&Complex::<f32>::new(0.0, 0.0));
            let phase = c.im.atan2(c.re); // [-pi, pi]
            // Scale phase from [-pi,pi] -> [0,y_top]
            let phase_y =
                (((phase + std::f32::consts::PI) / (2.0 * std::f32::consts::PI)) as f64) * y_top;
            phase_points.push([i as f64, phase_y]);

            // Scale real/imag from [-max_abs_amp, max_abs_amp] -> [0,y_top] (center at y_top/2)
            let scale = (y_top as f32) / (2.0 * max_abs_amp);
            let re_y = (0.5f32 * y_top as f32 + c.re * scale) as f64;
            let im_y = (0.5f32 * y_top as f32 + c.im * scale) as f64;
            re_points.push([i as f64, re_y]);
            im_points.push([i as f64, im_y]);
        }

        // Midline for re/im (center at y_top/2)
        let midline = HLine::new(String::new(), y_top * 0.5)
            .color(egui::Color32::from_gray(96))
            .width(1.0);

        // Build the overlay series with names so they appear in legend (users can toggle visibility)
        let phase_overlay = Points::new("phase (scaled)", phase_points)
            .radius(2.5)
            .color(egui::Color32::from_rgb(240, 180, 80));
        let re_overlay = Points::new("Re(amp) (scaled)", re_points)
            .radius(2.5)
            .color(egui::Color32::from_rgb(160, 220, 255));
        let im_overlay = Points::new("Im(amp) (scaled)", im_points)
            .radius(2.5)
            .color(egui::Color32::from_rgb(140, 200, 180));

        let x0 = page_start as f64;
        let x1 = (page_end.saturating_sub(1)) as f64;

        Plot::new("probabilities_plot")
            .legend(egui_plot::Legend::default().position(Corner::LeftTop))
            .x_axis_label("Basis state index")
            .y_axis_label("Probability")
            .include_x(x0)
            .include_x(x1)
            .include_y(0.0)
            .include_y(y_top)
            .allow_scroll(true)
            .allow_zoom(true)
            .allow_boxed_zoom(false)
            .show(ui, |plot_ui| {
                // grid
                for h in &y_grid {
                    plot_ui.hline(h.clone());
                }
                // midline for re/im overlays
                plot_ui.hline(midline.clone());
                // overlays first (so bars are on top but users can hover points)
                plot_ui.points(phase_overlay);
                plot_ui.points(re_overlay);
                plot_ui.points(im_overlay);
                // main chart
                plot_ui.bar_chart(chart);
            });
    } else {
        ui.label("No simulation run yet.");
    }
    message
}

fn bitstring(idx: usize, width: usize) -> String {
    format!("|{:0width$b}>", idx, width = width)
}

fn phase_color(phase: f32, has_phase: bool, prob: f32) -> egui::Color32 {
    if has_phase && prob > 0.0 {
        // Map phase in [-pi, pi] to hue in [0,1], then use as RGB placeholder
        // (keeps the user's request to avoid HSV helpers while still varying color)
        let hue = (phase + std::f32::consts::PI) / (2.0 * std::f32::consts::PI);
        egui::Rgba::from_rgba_unmultiplied(hue, 0.85, 0.95, 1.0).into()
    } else {
        egui::Color32::from_rgb(80, 180, 255)
    }
}
