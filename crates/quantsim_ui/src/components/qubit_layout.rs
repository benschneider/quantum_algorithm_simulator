use super::site_readout::{probability_color, SiteReadout};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use quantsim_core::{
    circuit::{Circuit, QubitLayout, QubitRole},
    core::types::Operation,
};
use std::collections::BTreeSet;

/// Show physical links and the operations in the selected gate column.
pub(super) fn draw_map(
    ui: &mut Ui,
    circuit: &Circuit,
    layout: &QubitLayout,
    timestep: usize,
    selected: &mut Option<usize>,
    readout: SiteReadout<'_>,
) {
    if layout.positions.is_empty() {
        return;
    }
    // The enclosing workspace scrolls. Its clip height is stable as the user scrolls;
    // available_height() inside the scroll area can be effectively unbounded.
    let viewport = vec2(
        ui.available_width().max(1.0),
        (ui.clip_rect().height() - 155.0).max(120.0),
    );
    let has_ancillas = circuit.qubit_roles.contains(&Some(QubitRole::Ancilla));
    let (natural_size, centers) = map_geometry(&layout.positions, viewport, has_ancillas);
    let fit = fit_zoom(natural_size, viewport);
    let zoom = map_zoom_controls(ui, fit);
    let content = natural_size * zoom;
    let size = vec2(content.x.max(viewport.x), content.y);
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

    egui::ScrollArea::new([size.x > viewport.x + 0.5, size.y > viewport.y + 0.5])
        .id_salt("layout_map")
        .max_height(viewport.y)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let (canvas, _) = ui.allocate_exact_size(size, Sense::hover());
            let origin = canvas.min + (size - content) * 0.5;
            let centers: Vec<_> = centers
                .iter()
                .map(|offset| origin + *offset * zoom)
                .collect();
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
                let rect = Rect::from_center_size(center, Vec2::splat(38.0 * zoom));
                let hit = ui.interact(rect.expand(3.0 * zoom), ui.id().with(wire), Sense::click());
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
                let probability = readout.probability(wire);
                let fill = probability.map_or(ui.visuals().extreme_bg_color, |p| {
                    probability_color(p, ui.visuals().dark_mode)
                });
                let label_color = match probability {
                    Some(_) if ui.visuals().dark_mode => Color32::WHITE,
                    Some(_) => Color32::from_rgb(19, 31, 42),
                    None => ui.visuals().text_color(),
                };
                if is_selected || hit.has_focus() || hit.hovered() {
                    draw_site(
                        ui,
                        rect.expand(4.0 * zoom),
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
                    FontId::monospace((14.0 * zoom).max(1.0)),
                    label_color,
                );
                if is_ancilla {
                    let label = ui.painter().layout_no_wrap(
                        format!("q{wire}"),
                        FontId::monospace((12.0 * zoom).max(1.0)),
                        ui.visuals().text_color(),
                    );
                    let rect = Align2::CENTER_TOP
                        .anchor_size(center + vec2(0.0, 24.0 * zoom), label.size());
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
                    draw_badge(ui, center, &gate_badge(op, wire), zoom);
                }
                hit.on_hover_ui(|ui| {
                    ui.strong(description);
                    if let Some(p) = probability {
                        ui.label(format!("P(1) = {:.2}%", 100.0 * p));
                    }
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

fn draw_badge(ui: &Ui, center: Pos2, label: &str, zoom: f32) {
    let visuals = ui.visuals();
    let ink = if visuals.dark_mode {
        Color32::from_rgb(251, 201, 102)
    } else {
        Color32::from_rgb(127, 69, 0)
    };
    let painter = ui.painter();
    let text = painter.layout_no_wrap(
        label.to_owned(),
        FontId::monospace((12.0 * zoom).max(1.0)),
        ink,
    );
    let rect = Rect::from_center_size(
        center - vec2(0.0, 35.0 * zoom),
        text.size() + vec2(8.0, 3.0) * zoom,
    );
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

/// Subtract the coordinate minima before projection so a large shared offset
/// does not erase small separations when converting to screen coordinates.
pub(super) fn local_coordinates(positions: &[[i64; 2]]) -> Vec<Pos2> {
    let min = positions
        .iter()
        .fold([i64::MAX; 2], |[x, y], &[a, b]| [x.min(a), y.min(b)]);
    positions
        .iter()
        .map(|&[x, y]| pos2(x.abs_diff(min[0]) as f32, y.abs_diff(min[1]) as f32))
        .collect()
}

// Give gate badges room above each site and ancilla IDs room below it.
// The closest pair determines the scale for legible, full-size labels.
fn label_scale(positions: &[[i64; 2]], has_ancillas: bool) -> f32 {
    const X: i128 = 58;
    let y_spacing = if has_ancillas { 91 } else { 68 };
    let mut sites: Vec<_> = positions
        .iter()
        .map(|&[x, y]| (i128::from(x) * y_spacing, i128::from(y) * X))
        .collect();
    sites.sort_unstable();
    sites.dedup();
    let mut closest = i128::MAX;
    let mut by_y = BTreeSet::new();
    let mut left = 0;
    for &(x, y) in &sites {
        while x - sites[left].0 >= closest {
            let (old_x, old_y) = sites[left];
            by_y.remove(&(old_y, old_x));
            left += 1;
        }
        // Only points inside this square can improve the current distance.
        let bottom = (y.saturating_sub(closest), i128::MIN);
        let top = (y.saturating_add(closest), i128::MAX);
        for &(other_y, other_x) in by_y.range(bottom..=top) {
            closest = closest.min((x - other_x).max((y - other_y).abs()));
        }
        by_y.insert((y, x));
    }
    (X * y_spacing) as f32 / closest as f32
}

// Natural canvas dimensions include labels and badges. Zoom then scales the
// entire drawing uniformly so sites stay clickable at their painted position.
fn map_geometry(positions: &[[i64; 2]], viewport: Vec2, has_ancillas: bool) -> (Vec2, Vec<Vec2>) {
    let coords = local_coordinates(positions);
    let bounds = Rect::from_points(&coords);
    let padding = vec2(35.0, 48.0);
    let span = bounds.size();
    let available = (viewport - padding * 2.0).max(Vec2::splat(1.0));
    let fit = (available.x / span.x).min(available.y / span.y);
    let scale = if span == Vec2::ZERO {
        1.0
    } else {
        fit.max(label_scale(positions, has_ancillas))
    };
    let extent = span * scale;
    let size = extent + padding * 2.0;
    let centers = coords
        .iter()
        .map(|&p| padding + (p - bounds.min) * scale)
        .collect();
    (size, centers)
}

fn fit_zoom(size: Vec2, viewport: Vec2) -> f32 {
    (viewport.x / size.x).min(viewport.y / size.y).min(1.0)
}

fn map_zoom_controls(ui: &mut Ui, fit: f32) -> f32 {
    let id = ui.make_persistent_id("map_zoom");
    let saved = ui.data(|data| data.get_temp::<f32>(id));
    let mut zoom = saved.unwrap_or(fit).max(fit);
    ui.horizontal(|ui| {
        ui.label("Zoom");
        if ui
            .add_enabled(zoom > fit, egui::Button::new("−"))
            .on_hover_text("Zoom out")
            .clicked()
        {
            zoom = (zoom / 1.4).max(fit);
        }
        if ui
            .add_enabled(zoom < 3.0, egui::Button::new("+"))
            .on_hover_text("Zoom in")
            .clicked()
        {
            // One click from a very large overview should make its labels visible.
            zoom = (zoom * 1.4).clamp(0.5, 3.0);
        }
        if ui
            .button("Fit")
            .on_hover_text("Fit the entire layout")
            .clicked()
        {
            zoom = fit;
        }
    });
    ui.data_mut(|data| {
        if zoom == fit {
            data.remove::<f32>(id);
        } else {
            data.insert_temp(id, zoom);
        }
    });
    zoom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sycamore_fits_all_sites_then_zooms_and_returns_to_fit() {
        let circuit = quantsim_core::circuits::load_template("sycamore_53").unwrap();
        let layout = circuit.layout.as_ref().unwrap();
        let mut selected = None;
        let ctx = egui::Context::default();
        let mut frame = |events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 760.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        draw_map(
                            ui,
                            &circuit,
                            layout,
                            0,
                            &mut selected,
                            SiteReadout::Unavailable,
                        );
                    });
                },
            )
        };
        let label = |frame: &egui::FullOutput, name: &str| {
            frame
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == name => Some((
                        Rect::from_min_size(text.pos, text.galley.size()),
                        shape.clip_rect,
                    )),
                    _ => None,
                })
                .unwrap()
        };
        let initial = frame(vec![]);
        for wire in 0..53 {
            let (rect, clip) = label(&initial, &format!("q{wire}"));
            assert!(clip.contains_rect(rect), "q{wire} is clipped");
        }
        // The wide chip uses the available height instead of a fixed short viewport.
        let (top, _) = label(&initial, "q0");
        let (bottom, _) = label(&initial, "q52");
        assert!(bottom.center().y - top.center().y > 400.0);
        let initial_center = label(&initial, "q23").0;
        let mut click = |position| {
            for pressed in [true, false] {
                frame(vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]);
            }
            frame(vec![])
        };
        let enlarged = click(label(&initial, "+").0.center());
        assert!(label(&enlarged, "q23").0.width() > initial_center.width() * 1.2);
        let restored = click(label(&enlarged, "Fit").0.center());
        assert_eq!(label(&restored, "q23").0, initial_center);
    }

    #[test]
    fn coincident_wires_share_a_center_without_affecting_map_spacing() {
        let (size, centers) = map_geometry(&[[0, 0], [0, 0], [1, 0]], vec2(300.0, 320.0), true);
        let (expected, distinct) = map_geometry(&[[0, 0], [1, 0]], vec2(300.0, 320.0), true);
        assert_eq!(size, expected);
        assert_eq!(centers, [distinct[0], distinct[0], distinct[1]]);

        let (size, centers) = map_geometry(&[[5, 5]; 3], vec2(300.0, 320.0), true);
        assert!(size.is_finite());
        assert_eq!(centers, [size * 0.5; 3]);
    }

    #[test]
    fn map_spacing_survives_wide_registers_and_translated_coordinates() {
        let wide = QubitLayout::wire_order(65_537);
        let (size, centers) = map_geometry(&wide.positions, vec2(300.0, 320.0), true);
        assert!(size.is_finite());
        assert_eq!(centers[65_536].x - centers[65_535].x, 58.0);

        let translated = [[i64::MAX - 1, i64::MIN], [i64::MAX, i64::MIN + 1]];
        assert_eq!(
            map_geometry(&translated, vec2(300.0, 320.0), true),
            map_geometry(&[[0, 0], [1, 1]], vec2(300.0, 320.0), true)
        );
        let extremes = [[i64::MIN, i64::MAX], [i64::MAX, i64::MIN]];
        let (size, centers) = map_geometry(&extremes, vec2(300.0, 320.0), true);
        assert!(size.is_finite() && centers.iter().all(|p| p.is_finite()));
    }

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
                            SiteReadout::Unavailable,
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
                        draw_map(
                            ui,
                            &circuit,
                            layout,
                            0,
                            &mut selected,
                            SiteReadout::Unavailable,
                        );
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
            let (size, centers) = map_geometry(&layout.positions, vec2(300.0, 320.0), true);
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
