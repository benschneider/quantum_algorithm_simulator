use crate::state::simulation_state::{SimulationState, Waveform, WAVEFORM_POINTS};
use egui::{Color32, Ui};
use egui_plot::{GridMark, Line, LineStyle, Plot, PlotResponse, Points};
use quantsim_core::BasisIndex;

fn colors(ui: &Ui) -> (Color32, Color32) {
    if ui.visuals().dark_mode {
        (
            Color32::from_rgb(100, 203, 249),
            Color32::from_rgb(255, 181, 96),
        )
    } else {
        (
            Color32::from_rgb(14, 95, 158),
            Color32::from_rgb(160, 73, 7),
        )
    }
}

pub(super) fn legend(ui: &mut Ui) {
    let (real, imaginary) = colors(ui);
    ui.colored_label(real, "Real");
    ui.colored_label(imaginary, "Imaginary")
        .on_hover_text("Dashed line");
}

pub(super) fn show(ui: &mut Ui, simulation: &mut SimulationState) {
    let Some(wave) = simulation.waveform() else {
        return;
    };
    if wave.points.is_empty() {
        return;
    }

    let (real, imaginary) = colors(ui);
    plot(ui, wave, real, imaginary);

    let selection = ui
        .horizontal_wrapped(|ui| {
            let end = sample_index(wave, wave.points.len() - 1);
            let unit_stride = wave.stride == BasisIndex::from(1u8);
            ui.small(format!("Indices {}–{end}", wave.first));
            if !unit_stride {
                ui.small(format!("Stride {}", wave.stride));
            }
            if wave.last_index >= WAVEFORM_POINTS.into()
                || wave.first != BasisIndex::ZERO
                || !unit_stride
            {
                return window_controls(ui, wave);
            }
            None
        })
        .inner;
    if let Some(reference) = &wave.phase_reference {
        ui.small(format!("Phase reference at index {reference}"))
            .on_hover_text("The first occupied basis state's amplitude is drawn positive and real. Other amplitudes show phase relative to that state.");
    }
    match selection {
        Some(Selection::Window(first, stride)) => simulation.set_waveform_window(first, stride),
        Some(Selection::Follow) => simulation.reset_waveform_window(),
        None => {}
    }
    ui.separator();
}

// The state only supplies points whose indices fit in the register. Keep the
// exact basis index out of floating-point plot space, including on wasm32.
fn sample_index(wave: &Waveform, offset: usize) -> BasisIndex {
    &wave.first + &wave.stride * offset
}

fn plot(ui: &mut Ui, wave: &Waveform, real: Color32, imaginary: Color32) -> PlotResponse<()> {
    // Normalizing plot coordinates also avoids egui_plot's minimum axis span
    // flattening a 50-qubit Fourier mode. The tick labels remain in amplitude units.
    let peak = wave
        .points
        .iter()
        .map(|z| z.re.abs().max(z.im.abs()))
        .fold(0.0, f64::max);
    let scale = if peak == 0.0 { 1.0 } else { peak };
    let last_offset = wave.points.len() - 1;
    let direct_indices = wave.first == BasisIndex::ZERO && wave.stride == BasisIndex::from(1u8);
    let response = Plot::new("complex_amplitude_wave")
        .height(144.0)
        .reset()
        .show_x(false)
        .show_y(false)
        .default_x_bounds(-0.25, last_offset as f64 + 0.25)
        .default_y_bounds(-1.1, 1.1)
        .allow_drag(false)
        .allow_zoom(false)
        .allow_scroll(false)
        .allow_axis_zoom_drag(false)
        .allow_boxed_zoom(false)
        .allow_double_click_reset(false)
        .x_axis_label(if direct_indices {
            "Basis index"
        } else {
            "Position in window"
        })
        .x_grid_spacer(|_| {
            let spacing = wave.points.len().div_ceil(4).max(1);
            (0..=last_offset)
                .step_by(spacing)
                .map(|offset| GridMark {
                    value: offset as f64,
                    step_size: spacing as f64,
                })
                .collect()
        })
        .x_axis_formatter(|mark, _| {
            let x = mark.value;
            if (0.0..=last_offset as f64).contains(&x) && x.fract() == 0.0 {
                format!("{x:.0}")
            } else {
                String::new()
            }
        })
        .y_grid_spacer(|_| {
            [-1.0, 0.0, 1.0]
                .into_iter()
                .map(|value| GridMark {
                    value,
                    step_size: 1.0,
                })
                .collect()
        })
        .y_axis_formatter(|mark, _| {
            let value = mark.value * scale;
            if value == 0.0 {
                "0".into()
            } else if value.abs() < 0.01 {
                format!("{value:.1e}")
            } else {
                format!("{value:.2}")
            }
        })
        .show(ui, |plot_ui| {
            let (real_points, imaginary_points): (Vec<_>, Vec<_>) = wave
                .points
                .iter()
                .enumerate()
                .map(|(i, z)| ([i as f64, z.re / scale], [i as f64, z.im / scale]))
                .unzip();
            for (name, color, style, points) in [
                ("Real", real, LineStyle::Solid, real_points),
                (
                    "Imaginary",
                    imaginary,
                    LineStyle::dashed_dense(),
                    imaginary_points,
                ),
            ] {
                if last_offset == 0 {
                    plot_ui.points(Points::new(name, points).color(color).radius(3.0_f32));
                } else {
                    plot_ui.line(
                        Line::new(name, points)
                            .color(color)
                            .width(1.8_f32)
                            .style(style),
                    );
                }
            }
        });
    if let Some(pointer) = response.response.hover_pos() {
        let offset = response
            .transform
            .value_from_position(pointer)
            .x
            .round()
            .clamp(0.0, last_offset as f64) as usize;
        let x = response.transform.position_from_point_x(offset as f64);
        ui.painter().vline(
            x,
            response.response.rect.y_range(),
            ui.visuals().widgets.hovered.fg_stroke,
        );
        let z = wave.points[offset];
        ui.horizontal_wrapped(|ui| {
            ui.small(format!("Index {}", sample_index(wave, offset)));
            ui.small(format!("Real {:+.5e}", z.re));
            ui.small(format!("Imaginary {:+.5e}", z.im));
        });
    }
    response
}

enum Selection {
    Window(BasisIndex, BasisIndex),
    Follow,
}

fn window_controls(ui: &mut Ui, wave: &Waveform) -> Option<Selection> {
    let mut choice = None;
    let span = &wave.stride * WAVEFORM_POINTS;
    if ui
        .add_enabled(wave.first >= wave.stride, egui::Button::new("‹").small())
        .on_hover_text("Previous window")
        .clicked()
    {
        let previous = if wave.first >= span {
            &wave.first - &span
        } else {
            // Keep the window on the same arithmetic progression near zero.
            &wave.first % &wave.stride
        };
        choice = Some(Selection::Window(previous, wave.stride.clone()));
    }
    let next = &wave.first + span;
    if ui
        .add_enabled(next <= wave.last_index, egui::Button::new("›").small())
        .on_hover_text("Next window")
        .clicked()
    {
        choice = Some(Selection::Window(next, wave.stride.clone()));
    }

    let id = ui.make_persistent_id("waveform_range_draft");
    let button = ui.small_button("Range");
    let popup = egui::Popup::from_toggle_button_response(&button)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            let (mut first, mut stride) = ui
                .data(|d| d.get_temp::<(String, String)>(id))
                .unwrap_or_else(|| (wave.first.to_string(), wave.stride.to_string()));
            egui::Grid::new(id.with("fields")).show(ui, |ui| {
                ui.label("First index");
                ui.add(egui::TextEdit::singleline(&mut first).desired_width(148.0));
                ui.end_row();
                ui.label("Stride");
                ui.add(egui::TextEdit::singleline(&mut stride).desired_width(148.0));
                ui.end_row();
            });
            let range = first
                .trim()
                .parse::<BasisIndex>()
                .ok()
                .zip(stride.trim().parse::<BasisIndex>().ok())
                .filter(|(f, s)| *f <= wave.last_index && *s != BasisIndex::ZERO);
            ui.data_mut(|d| d.insert_temp(id, (first, stride)));
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(range.is_some(), egui::Button::new("Apply"))
                    .clicked()
                {
                    choice = range.map(|(f, s)| Selection::Window(f, s));
                    ui.close();
                }
                if ui
                    .button("Follow step")
                    .on_hover_text("Restore the automatic window for the selected circuit step")
                    .clicked()
                {
                    choice = Some(Selection::Follow);
                    ui.close();
                }
            });
        });
    if popup.is_none() {
        ui.data_mut(|d| d.remove::<(String, String)>(id));
    }
    choice
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_plot::PlotPoint;
    use nalgebra::Complex;

    #[test]
    fn typed_window_can_be_applied_and_paged_at_the_last_basis_index() {
        for (wires, stride) in [
            (64, 2u8.into()),
            (128, 2u8.into()),
            (200, BasisIndex::from(1u8) << 130),
        ] {
            check_window_controls(wires, stride);
        }
    }

    fn check_window_controls(wires: usize, stride: BasisIndex) {
        let mut circuit = quantsim_core::circuit::Circuit::new(wires);
        circuit
            .initial_preps
            .fill(Some(quantsim_core::types::QubitPrep::Plus));
        let state = circuit.run_stabilizer_prefix(0).unwrap();
        let mut sim = SimulationState::from_stabilizer(&state, wires);
        let last_index = (BasisIndex::from(1u8) << wires) - 1u8;
        let first = &last_index - &stride * WAVEFORM_POINTS;
        sim.set_waveform_window(123u8.into(), 17u8.into());

        let ctx = egui::Context::default();
        let render = |simulation: &mut SimulationState, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(350.0, 460.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| show(ui, simulation));
                },
            )
        };
        let click = |sim: &mut SimulationState, text: &str| {
            render(sim, vec![]);
            let output = render(sim, vec![]);
            let pos = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == text => {
                        Some(t.visual_bounding_rect().center())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Missing field or button {text}"));
            let mut events = vec![egui::Event::PointerMoved(pos)];
            events.extend([true, false].map(|pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }));
            render(sim, events);
        };
        let replace_text = |sim: &mut SimulationState, value: String| {
            render(
                sim,
                vec![
                    egui::Event::Key {
                        key: egui::Key::A,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::COMMAND,
                    },
                    egui::Event::Text(value),
                ],
            );
        };
        click(&mut sim, "Range");
        click(&mut sim, "123");
        replace_text(&mut sim, first.to_string());
        click(&mut sim, "17");
        replace_text(&mut sim, stride.to_string());
        click(&mut sim, "Apply");
        let wave = sim.waveform().unwrap();
        assert_eq!((&wave.first, &wave.stride), (&first, &stride));
        for (button, expected) in [("›", last_index), ("‹", first)] {
            click(&mut sim, button);
            assert_eq!(sim.waveform().unwrap().first, expected);
        }
        click(&mut sim, "Range");
        click(&mut sim, "Follow step");
        let wave = sim.waveform().unwrap();
        assert_eq!(wave.first, BasisIndex::ZERO);
        assert_eq!(wave.stride, 1u8.into());
    }

    #[test]
    fn tiny_fourier_mode_remains_visible_and_hover_preserves_the_full_basis_index() {
        for (last_index, peak) in [
            (u64::MAX.into(), 2.0_f64.powi(-25)),
            (u128::MAX.into(), 2.0_f64.powi(-64)),
            (
                (BasisIndex::from(1u8) << 200usize) - 1u8,
                2.0_f64.powi(-100),
            ),
        ] {
            check_fourier_hover(last_index, peak);
        }
    }

    fn check_fourier_hover(last_index: BasisIndex, peak: f64) {
        let wave = Waveform {
            first: &last_index - 63u8,
            stride: 1u8.into(),
            last_index: last_index.clone(),
            points: (0..64)
                .map(|i| Complex::from_polar(peak, std::f64::consts::TAU * i as f64 / 32.0))
                .collect(),
            phase_reference: None,
        };
        let ctx = egui::Context::default();
        let mut target = egui::Pos2::ZERO;
        let mut render = |events| {
            let result = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(350.0, 280.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let plot = plot(ui, &wave, Color32::LIGHT_BLUE, Color32::ORANGE);
                        target = plot
                            .transform
                            .position_from_point(&PlotPoint::new(8.0, 1.0));
                        let trough = plot
                            .transform
                            .position_from_point(&PlotPoint::new(24.0, -1.0));
                        assert!(trough.y - target.y > 70.0);
                    });
                },
            );
            (result, target)
        };
        render(vec![]);
        let (_, peak_pos) = render(vec![]);
        render(vec![egui::Event::PointerMoved(peak_pos)]);
        let (output, _) = render(vec![]);
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text()),
                _ => None,
            })
            .collect();
        assert!(
            labels.contains(&format!("{peak:.1e}").as_str()),
            "{labels:?}"
        );
        assert!(
            labels.contains(&format!("Index {}", &last_index - 55u8).as_str())
                && labels.contains(&format!("Imaginary {peak:+.5e}").as_str()),
            "{labels:?}"
        );
        assert_eq!(sample_index(&wave, 63), last_index);
    }
}
