use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use quantsim_core::{
    circuit::{Circuit, QubitLayout, QubitRole},
    core::types::Operation,
};

/// Show physical links and the operations in the selected gate column.
pub(super) fn draw_map(
    ui: &mut Ui,
    circuit: &Circuit,
    layout: &QubitLayout,
    timestep: usize,
    selected: &mut Option<usize>,
) {
    if layout.positions.is_empty() {
        return;
    }
    let width = ui.available_width();
    let (size, centers) = map_geometry(&layout.positions, width);
    let operations = timestep
        .checked_sub(1)
        .and_then(|column| circuit.steps.get(column))
        .map(Vec::as_slice)
        .unwrap_or_default();
    let accent = if ui.visuals().dark_mode {
        Color32::from_rgb(205, 149, 244)
    } else {
        Color32::from_rgb(114, 48, 155)
    };

    egui::ScrollArea::new([size.x > width + 0.5, size.y > 310.0])
        .id_salt("layout_map")
        .max_height(310.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let (canvas, _) = ui.allocate_exact_size(size, Sense::hover());
            let centers: Vec<_> = centers.iter().map(|offset| canvas.min + *offset).collect();
            for &[a, b] in &layout.links {
                let stroke = if *selected == Some(a) || *selected == Some(b) {
                    Stroke::new(3.0_f32, accent)
                } else {
                    Stroke::new(1.5_f32, ui.visuals().widgets.noninteractive.fg_stroke.color)
                };
                ui.painter().line_segment([centers[a], centers[b]], stroke);
            }
            for (wire, &center) in centers.iter().enumerate() {
                let role = circuit.qubit_roles[wire];
                let name = site_name(circuit, wire);
                let rect = Rect::from_center_size(center, Vec2::splat(38.0));
                let hit = ui.interact(rect.expand(3.0), ui.id().with(wire), Sense::click());
                let is_selected = *selected == Some(wire);
                let description = match role {
                    Some(QubitRole::Ancilla) => format!("Ancilla {name} (q{wire})"),
                    Some(QubitRole::Data) => format!("Data qubit q{wire}"),
                    None => format!("Qubit q{wire}"),
                };
                hit.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Button,
                        true,
                        is_selected,
                        &description,
                    )
                });
                if hit.clicked() {
                    *selected = Some(wire);
                }
                let color = match role {
                    Some(QubitRole::Ancilla) => accent,
                    Some(QubitRole::Data) if ui.visuals().dark_mode => {
                        Color32::from_rgb(139, 199, 234)
                    }
                    Some(QubitRole::Data) => Color32::from_rgb(31, 103, 148),
                    None => ui.visuals().text_color(),
                };
                let is_ancilla = role == Some(QubitRole::Ancilla);
                let fill = if hit.hovered() {
                    ui.visuals().widgets.hovered.bg_fill
                } else {
                    ui.visuals().extreme_bg_color
                };
                if is_selected || hit.has_focus() {
                    draw_site(
                        ui,
                        rect.expand(4.0),
                        is_ancilla,
                        Color32::TRANSPARENT,
                        Stroke::new(1.5_f32, color),
                    );
                }
                draw_site(ui, rect, is_ancilla, fill, Stroke::new(2.0_f32, color));
                ui.painter().text(
                    center,
                    Align2::CENTER_CENTER,
                    &name,
                    FontId::monospace(14.0),
                    ui.visuals().text_color(),
                );
                if is_ancilla {
                    let label = ui.painter().layout_no_wrap(
                        format!("q{wire}"),
                        FontId::monospace(12.0),
                        ui.visuals().text_color(),
                    );
                    let rect =
                        Align2::CENTER_TOP.anchor_size(center + vec2(0.0, 24.0), label.size());
                    // Vertical links can run directly behind the wire ID on a lattice.
                    ui.painter().rect_filled(
                        rect.expand2(vec2(2.0, 0.0)),
                        2.0,
                        ui.visuals().panel_fill,
                    );
                    ui.painter()
                        .galley(rect.min, label, ui.visuals().text_color());
                }

                let operation = operations
                    .iter()
                    .find(|op| op.qubits.contains(&(wire as u32)));
                if let Some(op) = operation {
                    draw_badge(ui, center, &gate_badge(op, wire));
                }
                hit.on_hover_ui(|ui| {
                    ui.strong(description);
                    let [x, y] = layout.positions[wire];
                    ui.label(format!("Position ({x}, {y})"));
                    if let Some(op) = operation {
                        if let Some(name) = &op.name {
                            ui.label(name);
                        }
                        let wires = op
                            .qubits
                            .iter()
                            .map(|q| format!("q{q}"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        ui.label(format!("{} on {wires}", op.id));
                    }
                });
            }
        });

    if let Some(wire) = selected.filter(|&wire| wire < layout.positions.len()) {
        let name = if circuit.qubit_roles[wire] == Some(QubitRole::Ancilla) {
            format!("{} (q{wire})", site_name(circuit, wire))
        } else {
            format!("q{wire}")
        };
        let neighbors = linked_wires(layout, wire);
        if neighbors.is_empty() {
            ui.label(format!("{name} has no physical links."));
        } else {
            let neighbors = neighbors
                .iter()
                .map(|q| format!("q{q}"))
                .collect::<Vec<_>>()
                .join(", ");
            ui.label(format!("{name} connects to {neighbors}."));
        }
    } else {
        ui.weak("Select a qubit to inspect its physical links.");
    }
}

pub(super) fn site_name(circuit: &Circuit, wire: usize) -> String {
    if circuit.qubit_roles[wire] == Some(QubitRole::Ancilla) {
        let ancilla = circuit.qubit_roles[..wire]
            .iter()
            .filter(|&&r| r == Some(QubitRole::Ancilla))
            .count();
        format!("A{ancilla}")
    } else {
        format!("q{wire}")
    }
}

fn linked_wires(layout: &QubitLayout, wire: usize) -> Vec<usize> {
    let mut neighbors = Vec::new();
    for &[a, b] in &layout.links {
        if a == wire {
            neighbors.push(b);
        } else if b == wire {
            neighbors.push(a);
        }
    }
    neighbors.sort_unstable();
    neighbors
}

fn gate_badge(op: &Operation, wire: usize) -> String {
    if let Some((_, target, action)) = op.controlled_action() {
        return if wire == target as usize {
            format!("{action} tgt")
        } else {
            "Ctrl".to_owned()
        };
    }
    op.id.to_string()
}

fn draw_badge(ui: &Ui, center: Pos2, label: &str) {
    let visuals = ui.visuals();
    let ink = if visuals.dark_mode {
        Color32::from_rgb(251, 201, 102)
    } else {
        Color32::from_rgb(127, 69, 0)
    };
    let painter = ui.painter();
    let text = painter.layout_no_wrap(label.to_owned(), FontId::monospace(12.0), ink);
    let rect = Rect::from_center_size(center - vec2(0.0, 35.0), text.size() + vec2(8.0, 3.0));
    painter.rect(
        rect,
        3.0,
        visuals.extreme_bg_color,
        Stroke::new(1.0_f32, ink),
        StrokeKind::Inside,
    );
    painter.galley(rect.center() - text.size() / 2.0, text, ink);
}

fn draw_site(ui: &Ui, rect: Rect, square: bool, fill: Color32, outline: Stroke) {
    if square {
        ui.painter()
            .rect(rect, 3.0, fill, outline, StrokeKind::Inside);
    } else {
        ui.painter()
            .circle(rect.center(), rect.width() / 2.0, fill, outline);
    }
}

// Preserve the coordinate aspect ratio. Use scrolling when fitting the full
// arrangement would make adjacent labels or gate badges overlap.
fn map_geometry(positions: &[[i16; 2]], width: f32) -> (Vec2, Vec<Vec2>) {
    let coords: Vec<_> = positions
        .iter()
        .map(|&[x, y]| pos2(f32::from(x), f32::from(y)))
        .collect();
    let bounds = Rect::from_points(&coords);
    let padding = vec2(35.0, 48.0);
    let span = bounds.size();
    let available = vec2((width - padding.x * 2.0).max(1.0), 204.0);
    let fit = (available.x / span.x).min(available.y / span.y);
    let mut min_scale: f32 = 0.0;
    for (i, &point) in coords.iter().enumerate() {
        for &other in &coords[..i] {
            let delta = (other - point).abs();
            min_scale = min_scale.max((58.0 / delta.x).min(91.0 / delta.y));
        }
    }
    let scale = if coords.len() == 1 {
        1.0
    } else {
        fit.max(min_scale)
    };
    let extent = span * scale;
    let size = vec2(
        width.max(extent.x + padding.x * 2.0),
        extent.y + padding.y * 2.0,
    );
    let origin = (size - extent) * 0.5;
    let centers = coords
        .iter()
        .map(|&p| origin + (p - bounds.min) * scale)
        .collect();
    (size, centers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_fault_badges_follow_the_template_wires_and_selected_column() {
        for (template, fault_step, gate) in [
            ("error_correction_bit_flip_two_faults", 5, "X"),
            ("error_correction_phase_flip_two_faults", 6, "Z"),
        ] {
            let circuit = quantsim_core::circuits::load_template(template).unwrap();
            for step in [0, fault_step, fault_step + 1] {
                let ctx = egui::Context::default();
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(720.0, 360.0))),
                    ..Default::default()
                };
                let output = ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        draw_map(
                            ui,
                            &circuit,
                            circuit.layout.as_ref().unwrap(),
                            step,
                            &mut Some(3),
                        );
                    });
                });
                let labels: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|clipped| match &clipped.shape {
                        egui::Shape::Text(text) => Some((
                            text.galley.job.text.as_str(),
                            text.pos.x + text.galley.size().x * 0.5,
                        )),
                        _ => None,
                    })
                    .collect();
                assert!(labels
                    .iter()
                    .any(|(s, _)| *s == "A0 (q3) connects to q0, q1."));
                let x = |label| labels.iter().find(|(s, _)| *s == label).unwrap().1;
                assert!(
                    x("q0") < x("A0")
                        && x("A0") < x("q1")
                        && x("q1") < x("A1")
                        && x("A1") < x("q2")
                );
                let badges: Vec<_> = labels
                    .iter()
                    .filter(|(s, _)| *s == gate)
                    .map(|(_, x)| *x)
                    .collect();
                if step == fault_step {
                    assert_eq!(badges.len(), 2);
                    assert!((badges[0] - x("q0")).abs() < 1.0);
                    assert!((badges[1] - x("q2")).abs() < 1.0);
                } else {
                    assert!(badges.is_empty());
                }
            }
        }
    }

    #[test]
    fn clicking_an_ancilla_selects_its_incident_links() {
        let circuit = quantsim_core::circuits::load_template("error_correction_bit_flip").unwrap();
        let layout = circuit.layout.as_ref().unwrap();
        let mut selected = Some(3);
        let ctx = egui::Context::default();
        let mut frame = |events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(640.0, 320.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        draw_map(ui, &circuit, layout, 0, &mut selected);
                    });
                },
            )
        };
        let initial = frame(vec![]);
        let pos = initial
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.job.text == "A1" => {
                    Some(text.pos + text.galley.size() * 0.5)
                }
                _ => None,
            })
            .unwrap();
        for pressed in [true, false] {
            frame(vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
        }
        let after = frame(vec![]);
        assert_eq!(selected, Some(4));
        assert!(after.shapes.iter().any(|clipped| matches!(&clipped.shape,
            egui::Shape::Text(text) if text.galley.job.text == "A1 (q4) connects to q1, q2.")));
        let highlighted = after.shapes.iter().filter(|clipped| matches!(&clipped.shape,
            egui::Shape::LineSegment { points, stroke } if stroke.width == 3.0 && points.iter().any(|p| (p.x - pos.x).abs() < 1.0))).count();
        assert_eq!(highlighted, 2);
    }

    #[test]
    fn dense_arrangements_keep_legible_spacing_on_a_narrow_viewport() {
        for layout in [
            QubitLayout::line(1),
            QubitLayout::line(21),
            QubitLayout::ring(21),
            QubitLayout::grid(21),
        ] {
            let (size, centers) = map_geometry(&layout.positions, 300.0);
            assert!(size.is_finite());
            for (i, center) in centers.iter().enumerate() {
                assert!(Rect::from_min_size(Pos2::ZERO, size).contains(center.to_pos2()));
                for other in &centers[..i] {
                    let separation = (*center - *other).abs();
                    assert!(separation.x >= 57.9 || separation.y >= 90.9);
                }
            }
        }
    }
}
