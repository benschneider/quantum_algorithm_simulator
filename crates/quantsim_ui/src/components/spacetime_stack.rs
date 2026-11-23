use super::qubit_layout::site_name;
use crate::state::ui_state::StackCamera;
use egui::{
    pos2, vec2, Align2, Color32, FontId, PointerButton, Pos2, Rect, Sense, Shape, Stroke,
    StrokeKind, Ui, Vec2,
};
use quantsim_core::{
    circuit::{Circuit, QubitLayout, QubitRole},
    core::types::Operation,
};

/// Orthographic projection of a physical plane and its displacement through time.
struct Projection {
    base: Vec<(Pos2, f32)>,
    corners: [Pos2; 4],
    rise: f32,
    depth_rise: f32,
}

impl Projection {
    fn new(layout: &QubitLayout, steps: usize, camera: &StackCamera, viewport: Rect) -> Self {
        let coords: Vec<_> = layout
            .positions
            .iter()
            .map(|&[x, y]| pos2(f32::from(x), f32::from(y)))
            .collect();
        let bounds = Rect::from_points(&coords);
        let unit = bounds.width().max(bounds.height()).max(1.0);
        let (sy, cy) = camera.yaw.to_radians().sin_cos();
        let (sp, cp) = camera.pitch.to_radians().sin_cos();
        let rotate = |p: Pos2| {
            let local = (p - bounds.center()) / unit;
            let forward = local.x * sy + local.y * cy;
            (
                pos2(local.x * cy - local.y * sy, forward * sp),
                forward * cp,
            )
        };
        let plate = bounds.expand(unit * 0.075);
        let corners = [
            plate.left_top(),
            plate.right_top(),
            plate.right_bottom(),
            plate.left_bottom(),
        ]
        .map(|p| rotate(p).0);
        let height = if steps == 0 {
            0.0
        } else {
            0.26 + 0.025 * steps.min(18) as f32
        };
        let rise = vec2(0.0, -height * cp);
        let mut extents = Rect::from_points(&corners);
        extents = extents.union(extents.translate(rise));
        let space = viewport.shrink2(vec2(32.0, 23.0));
        let scale = (space.width() / extents.width().max(0.001))
            .min(space.height() / extents.height().max(0.001))
            .max(1.0);
        let place = |p: Pos2| space.center() + (p - extents.center()) * scale;
        let base = coords
            .into_iter()
            .map(|p| {
                let (p, depth) = rotate(p);
                (place(p), depth)
            })
            .collect();
        let intervals = steps.max(1) as f32;
        Self {
            base,
            corners: corners.map(place),
            rise: rise.y * scale / intervals,
            depth_rise: height * sp / intervals,
        }
    }

    fn site(&self, wire: usize, step: usize) -> (Pos2, f32) {
        let (p, depth) = self.base[wire];
        (
            p + vec2(0.0, self.rise * step as f32),
            depth + self.depth_rise * step as f32,
        )
    }

    fn plate(&self, step: usize) -> [Pos2; 4] {
        self.corners.map(|p| p + vec2(0.0, self.rise * step as f32))
    }
}

// The editor supports 100 columns. For larger imported circuits keep the full time
// extent and include the current slice and its immediate neighbors at full resolution.
fn visible_steps(end: usize, active: usize) -> Vec<usize> {
    const LIMIT: usize = 100;
    if end <= LIMIT {
        return (0..=end).collect();
    }
    let mut steps: Vec<_> = (0..=LIMIT)
        .map(|i| ((end as u128 * i as u128) / LIMIT as u128) as usize)
        .collect();
    steps.extend([
        active.saturating_sub(1),
        active,
        active.saturating_add(1).min(end),
    ]);
    steps.sort_unstable();
    steps.dedup();
    steps
}

struct Site<'a> {
    wire: usize,
    step: usize,
    pos: Pos2,
    depth: f32,
    op: Option<&'a Operation>,
}

fn pick_site<'a, 'op>(
    sites: &'a [Site<'op>],
    pointer: Pos2,
    active: usize,
) -> Option<&'a Site<'op>> {
    // The last painted glyph wins when the pointer is inside overlapping glyphs.
    // Between glyphs, use the nearest center inside the larger touch target.
    let mut closest = None;
    let mut best = 12.0_f32.powi(2);
    for site in sites.iter().rev() {
        let d = site.pos.distance_sq(pointer);
        if d <= site_radius(site, active).powi(2) {
            return Some(site);
        }
        if d < best {
            closest = Some(site);
            best = d;
        }
    }
    closest
}

fn site_radius(site: &Site<'_>, active: usize) -> f32 {
    if site.step == active {
        5.5
    } else if site.op.is_some() {
        3.2
    } else {
        2.0
    }
}

fn canvas_size(ui: &Ui) -> Vec2 {
    let width = ui.available_width().max(1.0);
    let height_limit = (ui.ctx().screen_rect().height() * 0.39).clamp(230.0, 355.0);
    vec2(width, (width * 0.65).clamp(230.0, height_limit))
}

/// Display the physical sites at successive circuit steps. Only a click returns a new readout.
pub(super) fn draw_stack(
    ui: &mut Ui,
    circuit: &Circuit,
    layout: &QubitLayout,
    timestep: usize,
    selected: Option<usize>,
    camera: &mut StackCamera,
) -> Option<(usize, usize)> {
    if layout.positions.is_empty() {
        return None;
    }
    let end = circuit.steps.len();
    let active = timestep.min(end);
    let (rect, response) = ui.allocate_exact_size(canvas_size(ui), Sense::click_and_drag());
    let dragging = response.dragged_by(PointerButton::Primary);
    if dragging {
        let delta = response.drag_delta();
        camera.yaw = (camera.yaw + delta.x * 0.55 + 180.0).rem_euclid(360.0) - 180.0;
        camera.pitch = (camera.pitch - delta.y * 0.35).clamp(0.0, 75.0);
    }
    let view = Projection::new(layout, end, camera, rect);
    let steps = visible_steps(end, active);
    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();
    let ink = visuals.text_color();
    let muted = visuals.weak_text_color();
    let physical = if visuals.dark_mode {
        Color32::from_rgb(117, 196, 231)
    } else {
        Color32::from_rgb(28, 100, 144)
    };
    let gate = if visuals.dark_mode {
        Color32::from_rgb(255, 198, 100)
    } else {
        Color32::from_rgb(152, 82, 6)
    };
    let ancilla = if visuals.dark_mode {
        Color32::from_rgb(205, 156, 234)
    } else {
        Color32::from_rgb(115, 58, 157)
    };
    let surface = visuals.extreme_bg_color;
    painter.rect_filled(rect, 4.0, surface);

    // A time axis and the selected wire's continuous trace provide an orientation
    // reference even for an arrangement with no links or no gates.
    let tick_every = end.div_ceil(5).max(1);
    for &step in &steps {
        if step != 0 && step != end && step != active && step % tick_every != 0 {
            continue;
        }
        let corners = view.plate(step);
        let focus = step == active;
        let stroke = if focus {
            Stroke::new(1.2_f32, physical.gamma_multiply(0.8))
        } else {
            Stroke::new(0.7_f32, muted.gamma_multiply(0.32))
        };
        if focus {
            painter.add(Shape::convex_polygon(
                corners.to_vec(),
                physical.gamma_multiply(0.09),
                stroke,
            ));
        } else {
            painter.add(Shape::closed_line(corners.to_vec(), stroke));
        }
        // Adjacent end/active labels can coincide in long circuits.
        let label_clear = (step.abs_diff(active) as f32 * view.rise.abs()) > 16.0;
        if focus || ((step == 0 || step == end) && label_clear) {
            let left = corners
                .into_iter()
                .min_by(|a, b| a.x.total_cmp(&b.x))
                .unwrap();
            painter.text(
                left - vec2(4.0, 0.0),
                Align2::RIGHT_CENTER,
                format!("t{step}"),
                FontId::monospace(10.0),
                if focus { ink } else { muted },
            );
        }
    }
    for wire in 0..layout.positions.len() {
        let traced = selected == Some(wire);
        let stroke = if traced {
            Stroke::new(2.0_f32, ancilla)
        } else {
            Stroke::new(0.7_f32, muted.gamma_multiply(0.45))
        };
        painter.line_segment([view.site(wire, 0).0, view.site(wire, end).0], stroke);
    }

    for &[a, b] in &layout.links {
        let line = [view.site(a, active).0, view.site(b, active).0];
        painter.extend(Shape::dashed_line(
            &line,
            Stroke::new(2.7_f32, physical),
            5.0,
            3.0,
        ));
    }
    let count = layout.positions.len();
    let mut sites = Vec::with_capacity(count * steps.len());
    for &step in &steps {
        let mut on_wire = vec![None; count];
        if let Some(ops) = step.checked_sub(1).and_then(|i| circuit.steps.get(i)) {
            for op in ops.iter().take(count) {
                for &wire in op.qubits.iter().take(count) {
                    if let Some(slot) = on_wire.get_mut(wire as usize) {
                        *slot = Some(op);
                    }
                }
                // Directional gates have one target and one or more controls. For
                // other multi-site operations draw undirected group membership.
                if let Some((&last, others)) = op.qubits.split_last() {
                    let last = last as usize;
                    if last < count {
                        let stroke = if step == active {
                            Stroke::new(2.2_f32, gate)
                        } else {
                            Stroke::new(1.1_f32, gate.gamma_multiply(0.50))
                        };
                        for &other in others.iter().take(count).filter(|&&w| (w as usize) < count) {
                            painter.line_segment(
                                [view.site(last, step).0, view.site(other as usize, step).0],
                                stroke,
                            );
                        }
                    }
                }
            }
        }
        for (wire, op) in on_wire.into_iter().enumerate() {
            let (pos, depth) = view.site(wire, step);
            sites.push(Site {
                wire,
                step,
                pos,
                depth,
                op,
            });
        }
    }
    sites.sort_by(|a, b| a.depth.total_cmp(&b.depth).then(a.step.cmp(&b.step)));
    let hovered = response
        .hover_pos()
        .filter(|_| !dragging)
        .and_then(|p| pick_site(&sites, p, active));
    let mut label_rects: Vec<Rect> = Vec::new();
    for site in &sites {
        let role = circuit.qubit_roles[site.wire];
        let role_color = if role == Some(QubitRole::Ancilla) {
            ancilla
        } else {
            physical
        };
        let color = if site.op.is_some() { gate } else { role_color };
        let chosen = site.step == active && selected == Some(site.wire);
        let near = hovered.is_some_and(|other| (other.wire, other.step) == (site.wire, site.step));
        let r = site_radius(site, active);
        let fill = if site.step == active {
            color
        } else {
            surface.lerp_to_gamma(color, if site.op.is_some() { 0.8 } else { 0.46 })
        };
        let mark = Rect::from_center_size(site.pos, Vec2::splat(2.0 * r));
        if role == Some(QubitRole::Ancilla) {
            painter.rect_filled(mark, 1.0, fill);
        } else {
            painter.circle_filled(site.pos, r, fill);
        }
        if chosen || near {
            let outline = Stroke::new(1.6_f32, if near { ink } else { ancilla });
            if role == Some(QubitRole::Ancilla) {
                painter.rect_stroke(mark.expand(2.7), 2.0, outline, StrokeKind::Inside);
            } else {
                painter.circle_stroke(site.pos, r + 2.7, outline);
            }
        }
        // A target cross is meaningful only for a gate with a known direction.
        if site.step == active
            && site
                .op
                .and_then(Operation::controlled_action)
                .is_some_and(|(_, t, _)| t as usize == site.wire)
        {
            painter.line_segment(
                [site.pos - vec2(2.8, 0.0), site.pos + vec2(2.8, 0.0)],
                Stroke::new(1.0_f32, surface),
            );
            painter.line_segment(
                [site.pos - vec2(0.0, 2.8), site.pos + vec2(0.0, 2.8)],
                Stroke::new(1.0_f32, surface),
            );
        }
    }
    // Label the selected site first so dense circuits never displace its annotation.
    for site in sites
        .iter()
        .filter(|s| s.step == active && selected == Some(s.wire))
        .chain(
            sites
                .iter()
                .rev()
                .filter(|s| s.step == active && selected != Some(s.wire)),
        )
    {
        if count > 8 && site.op.is_none() && selected != Some(site.wire) {
            continue;
        }
        let name = site_name(circuit, site.wire);
        let label = match site.op {
            Some(op) => format!("{name} · {}", op.id),
            None => name,
        };
        let text = painter.layout_no_wrap(label, FontId::monospace(11.0), ink);
        let label_rect = Rect::from_min_size(site.pos + vec2(5.0, -19.0), text.size()).expand(1.5);
        if rect.contains_rect(label_rect) && !label_rects.iter().any(|r| r.intersects(label_rect)) {
            label_rects.push(label_rect);
            painter.rect_filled(label_rect, 2.0, surface);
            painter.galley(label_rect.min + vec2(1.5, 1.5), text, ink);
        }
    }
    let picked = if response.clicked_by(PointerButton::Primary) {
        response
            .interact_pointer_pos()
            .and_then(|pos| pick_site(&sites, pos, active))
            .map(|s| (s.wire, s.step))
    } else {
        None
    };
    let response = response.on_hover_cursor(if dragging {
        egui::CursorIcon::Grabbing
    } else if hovered.is_some() {
        egui::CursorIcon::PointingHand
    } else {
        egui::CursorIcon::Grab
    });
    if let Some(site) = hovered {
        response.on_hover_ui_at_pointer(|ui| {
            ui.strong(format!("q{} · Step {}", site.wire, site.step));
            if let Some(op) = site.op {
                let wires = op
                    .qubits
                    .iter()
                    .take(count)
                    .map(|q| format!("q{q}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                ui.label(format!("{} on {wires}", op.id));
                if let Some(name) = &op.name {
                    ui.label(name);
                }
            } else if site.step == 0 {
                ui.label("Initial state");
            } else {
                ui.label("Idle");
            }
        });
    }
    ui.horizontal_wrapped(|ui| {
        ui.weak("Time runs upward");
        ui.colored_label(physical, "Dashed · physical link");
        ui.colored_label(gate, "Gold · gate");
        ui.weak("Drag to orbit");
    });
    if end > 100 {
        ui.weak(format!(
            "Showing {} slices across steps 0–{end}.",
            steps.len()
        ));
    }
    picked
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, RawInput};
    use quantsim_core::core::types::Gate;

    #[test]
    fn fitting_preserves_vertical_worldlines_and_camera_depth() {
        let camera = StackCamera::default();
        let viewport = Rect::from_min_size(Pos2::ZERO, vec2(320.0, 245.0));
        let layout = QubitLayout::grid(21);
        for yaw in [-180.0, -45.0, 90.0] {
            let view = Projection::new(&layout, 100, &StackCamera { yaw, ..camera }, viewport);
            for wire in 0..21 {
                let (start, _) = view.site(wire, 0);
                let (end, _) = view.site(wire, 100);
                assert_eq!(start.x, end.x);
                assert!(end.y < start.y);
                assert!(viewport.contains(start) && viewport.contains(end));
            }
        }
        assert_eq!(visible_steps(100, 43), (0..=100).collect::<Vec<_>>());
        let steps = visible_steps(1_000_000, 543_219);
        assert!(steps.len() <= 104);
        for step in [0, 543_218, 543_219, 543_220, 1_000_000] {
            assert!(steps.contains(&step));
        }
    }

    #[test]
    fn picks_the_front_glyph_when_projected_sites_coincide() {
        let mut layout = QubitLayout::line(2);
        layout.positions = vec![[0, -1], [0, 1]];
        let view = Projection::new(
            &layout,
            2,
            &StackCamera {
                yaw: 0.0,
                pitch: 0.0,
            },
            Rect::from_min_size(Pos2::ZERO, vec2(600.0, 320.0)),
        );
        let sites: Vec<_> = (0..2)
            .map(|wire| {
                let (pos, depth) = view.site(wire, 1);
                Site {
                    wire,
                    step: 1,
                    pos,
                    depth,
                    op: None,
                }
            })
            .collect();
        assert_eq!(sites[0].pos, sites[1].pos);
        assert!(sites[0].depth < sites[1].depth);
        assert_eq!(pick_site(&sites, sites[0].pos, 1).unwrap().wire, 1);
    }

    #[test]
    fn click_selects_wire_and_step_but_orbit_leaves_readout_alone() {
        let ctx = egui::Context::default();
        let mut camera = StackCamera::default();
        let mut circuit = Circuit::new(3);
        // Include a non-neighbor two-qubit operation and an out-of-bounds
        // operand, as can happen when a user shrinks an existing circuit.
        circuit.steps = vec![
            vec![],
            vec![Operation::new(Gate::CX, vec![2, 0], vec![])],
            vec![Operation::new(Gate::CX, vec![8, 1], vec![])],
        ];
        let layout = QubitLayout::line(3);
        let mut frame = |events: Vec<Event>| {
            let mut picked = None;
            let mut rect = Rect::NOTHING;
            let output = ctx.run(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(640.0, 480.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        rect = Rect::from_min_size(ui.next_widget_position(), canvas_size(ui));
                        picked = draw_stack(ui, &circuit, &layout, 2, Some(1), &mut camera);
                    });
                },
            );
            (picked, rect, output)
        };
        let (_, rect, drawn) = frame(vec![]);
        let view = Projection::new(&layout, 3, &StackCamera::default(), rect);
        let endpoints = [view.site(2, 2).0, view.site(0, 2).0];
        assert!(drawn.shapes.iter().any(|c| matches!(c.shape, Shape::LineSegment{points,stroke} if stroke.width == 2.2 && (points == endpoints || points == [endpoints[1],endpoints[0]]))));
        let pos = view.site(2, 1).0;
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        assert!(frame(vec![Event::PointerMoved(pos), button(pos, true)])
            .0
            .is_none());
        assert_eq!(frame(vec![button(pos, false)]).0, Some((2, 1)));

        let dest = pos + vec2(40.0, -16.0);
        assert!(frame(vec![button(pos, true)]).0.is_none());
        assert!(frame(vec![Event::PointerMoved(dest)]).0.is_none());
        assert!(frame(vec![button(dest, false)]).0.is_none());
        assert_ne!(camera.yaw, StackCamera::default().yaw);
        assert_ne!(camera.pitch, StackCamera::default().pitch);
    }
}
