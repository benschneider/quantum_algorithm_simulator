use super::Projection;
use egui::{
    pos2,
    text::{LayoutJob, TextWrapping},
    Color32, FontId, Pos2, Rect, Stroke, Ui,
};
use quantsim_core::circuit::{Circuit, CircuitSection};

pub(super) fn range_text(section: &CircuitSection) -> String {
    if section.start == section.end {
        format!("t{}", section.start)
    } else {
        format!("t{}–{}", section.start, section.end)
    }
}

fn axis_name(label: &str) -> String {
    // egui shapes the entire input before it elides to the column width.
    // Keep that work bounded for names imported from JSON. Hover uses the original.
    const MAX_CHARS: usize = 256;
    let mut chars = label.chars();
    let mut name = String::with_capacity(label.len().min(MAX_CHARS));
    name.extend(
        chars
            .by_ref()
            .take(MAX_CHARS)
            .map(|ch| if ch.is_whitespace() { ' ' } else { ch }),
    );
    if chars.next().is_some() {
        name.push('…');
    }
    name
}

/// Draw the exact section intervals even when the stack samples a long circuit's layers.
pub(super) fn paint<'a>(
    ui: &Ui,
    rect: Rect,
    view: &Projection,
    circuit: &'a Circuit,
    active: usize,
    accent: Color32,
    pointer: Option<Pos2>,
) -> Option<&'a CircuitSection> {
    const ROW: f32 = 28.0;
    const MARGIN: f32 = 5.0;
    let sections = &circuit.sections;
    if sections.is_empty() {
        return None;
    }
    let painter = ui.painter_at(rect);
    let muted = ui
        .visuals()
        .text_color()
        .lerp_to_gamma(ui.visuals().weak_text_color(), 0.5);
    let tick_font = FontId::monospace(10.0);
    let tick_width = painter
        .layout_no_wrap(
            format!("t{}", circuit.steps.len()),
            tick_font.clone(),
            muted,
        )
        .size()
        .x;
    let origin = view.time_origin;
    let rail_x = origin.x - tick_width - 12.0;
    let text_right = rail_x - 15.0;
    let text_width = (text_right - rect.left() - MARGIN).max(1.0);
    let y = |step| origin.y + view.rise * step as f32;
    let middle = |section: &CircuitSection| (y(section.start) + y(section.end)) * 0.5;

    // Make room for legible labels around the inspected step when there are more
    // sections than rows. Earlier sections are lower on this upward time axis.
    let count = sections
        .len()
        .min(((rect.height() - 2.0 * MARGIN) / ROW) as usize);
    if count == 0 {
        return None;
    }
    let nearby = sections.partition_point(|section| section.end < active);
    let first = nearby.saturating_sub(count / 2).min(sections.len() - count);
    let shown = &sections[first..first + count];
    let span = middle(&shown[0]) - middle(&shown[count - 1]);
    let spread = ((count - 1) as f32 * ROW - span).max(0.0) * 0.5;
    let mut next_y = rect.top() + MARGIN + ROW * 0.5;
    let mut label_hit = None;
    let mut closest_bracket = None;
    for (i, section) in shown.iter().rev().enumerate() {
        let top = y(section.end);
        let bottom = y(section.start);
        let mid = (top + bottom) * 0.5;
        let last_y = rect.bottom() - MARGIN - ROW * (count as f32 - i as f32 - 0.5);
        let label_y = (mid - spread).clamp(next_y, last_y.max(next_y));
        next_y = label_y + ROW;
        let selected = (section.start..=section.end).contains(&active);
        let color = if selected { accent } else { muted };
        let stroke = Stroke::new(if selected { 2.0_f32 } else { 1.0_f32 }, color);
        let rail = pos2(rail_x, mid);
        painter.line_segment(
            [pos2(text_right + 3.0, label_y), rail],
            Stroke::new(1.0_f32, color),
        );
        let outline = [
            pos2(rail_x + 5.0, top),
            pos2(rail_x, top),
            pos2(rail_x, bottom),
            pos2(rail_x + 5.0, bottom),
        ];
        let bracket = if section.start == section.end {
            &outline[..2]
        } else {
            &outline[..]
        };
        for edge in bracket.windows(2) {
            painter.line_segment([edge[0], edge[1]], stroke);
            if let Some(p) = pointer {
                // A zero-width or zero-height rectangle measures distance to this exact stroke.
                let distance = Rect::from_two_pos(edge[0], edge[1]).distance_sq_to_pos(p);
                if distance <= 4.0
                    && closest_bracket
                        .is_none_or(|(best, _)| distance < best || (distance == best && selected))
                {
                    closest_bracket = Some((distance, section));
                }
            }
        }

        let mut label_box = Rect::NOTHING;
        for (text, font, top) in [
            (
                axis_name(&section.label),
                FontId::proportional(12.0),
                label_y - 13.0,
            ),
            (range_text(section), tick_font.clone(), label_y + 1.0),
        ] {
            let mut job = LayoutJob::simple_singleline(text, font, color);
            job.wrap = TextWrapping::truncate_at_width(text_width);
            let galley = painter.layout_job(job);
            let pos = pos2(text_right - galley.size().x, top);
            label_box = label_box.union(Rect::from_min_size(pos, galley.size()));
            painter.galley(pos, galley, color);
        }
        if pointer.is_some_and(|p| label_box.contains(p)) {
            label_hit = Some(section);
        }
    }
    label_hit.or_else(|| closest_bracket.map(|(_, section)| section))
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{vec2, Context, RawInput, Shape};
    use quantsim_core::circuit::QubitLayout;

    #[test]
    fn closely_spaced_brackets_pick_the_nearest_step_and_preserve_the_full_name() {
        let mut circuit = Circuit::new(1);
        circuit.steps.resize(1000, vec![]);
        let custom = format!("Parity\n{}\treadout", "相".repeat(4096));
        for (step, label) in [(20, "Prepare".to_owned()), (21, custom.clone())] {
            circuit
                .set_section(
                    None,
                    CircuitSection {
                        start: step,
                        end: step,
                        label,
                        description: String::new(),
                    },
                )
                .unwrap();
        }
        let ctx = Context::default();
        let canvas = Rect::from_min_size(Pos2::ZERO, vec2(530.0, 300.0));
        let mut view = Projection::new(&QubitLayout::line(1), 1000, &Default::default(), canvas);
        let render = |view: &Projection, pointer, active| {
            let mut hovered = None;
            let output = ctx.run(
                RawInput {
                    screen_rect: Some(canvas),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        hovered = paint(ui, canvas, view, &circuit, active, Color32::BLUE, pointer)
                            .map(|section| section.label.clone());
                    });
                },
            );
            (output, hovered)
        };
        let (output, _) = render(&view, None, 20);
        let ticks: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|clip| match clip.shape {
                Shape::LineSegment { points: [a, b], .. } if a.y == b.y && a.x != b.x => {
                    Some(a.lerp(b, 0.5))
                }
                _ => None,
            })
            .collect();
        assert_eq!(ticks.len(), 2);
        let upper = *ticks.iter().min_by(|a, b| a.y.total_cmp(&b.y)).unwrap();
        assert_eq!(
            render(&view, Some(upper), 20).1.as_deref(),
            Some(custom.as_str())
        );
        let name = output
            .shapes
            .iter()
            .find_map(|clip| match &clip.shape {
                Shape::Text(text) if text.galley.text().starts_with("Parity") => Some(text),
                _ => None,
            })
            .unwrap();
        assert!(name.galley.text().starts_with("Parity "));
        assert!(name.galley.text().ends_with('…'));
        assert_eq!(name.galley.rows.len(), 1);
        assert_eq!(
            render(&view, Some(name.visual_bounding_rect().center()), 20).1,
            Some(custom.clone())
        );

        // Two intervals can project onto the same pixel on very long timelines.
        view.rise = 0.0;
        let shared = pos2(upper.x, view.time_origin.y);
        assert_eq!(render(&view, Some(shared), 21).1, Some(custom));
    }
}
