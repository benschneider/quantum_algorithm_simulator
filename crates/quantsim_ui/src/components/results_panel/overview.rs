use crate::state::simulation_state::{SimulationState, COMPACT_SHOTS};
use egui::{Color32, Id, Sense, Stroke, Ui};
use egui_plot::{Bar, BarChart, GridMark, Plot};

#[derive(Clone, Copy)]
struct PinnedRange {
    wires: usize,
    first_bin: usize,
    bin_count: usize,
}

pub(super) fn show(ui: &mut Ui, simulation: &SimulationState) {
    let bins = simulation.probability_bins();
    if bins.is_empty() {
        return;
    }
    let group_size = bins.len().div_ceil(if ui.available_width() >= 600.0 {
        32
    } else if ui.available_width() >= 320.0 {
        16
    } else {
        8
    });
    let count = bins.len() / group_size;
    let prefix_bits = count.ilog2() as usize;
    let wires = simulation.wire_one_probabilities.len();
    let axis = if prefix_bits == wires {
        "Basis state".to_owned()
    } else {
        format!("Leading {prefix_bits} bits of basis state")
    };
    // Leave room for the full binary label centered on the last bar.
    let edge_padding = if count >= 16 { 1.25 } else { 0.6 };
    let (bars, ceiling) = bars_and_ceiling(bins, group_size, ui.visuals().hyperlink_color);
    let plot = Plot::new("results_probability_ranges")
        .height(144.0)
        .default_x_bounds(-edge_padding, (count - 1) as f64 + edge_padding)
        .default_y_bounds(0.0, ceiling)
        .reset()
        .sense(Sense::click())
        .allow_drag(false)
        .allow_zoom(false)
        .allow_axis_zoom_drag(false)
        .allow_boxed_zoom(false)
        .allow_double_click_reset(false)
        .allow_scroll(false)
        .show_x(false)
        .show_y(false)
        .x_axis_label(axis)
        .x_grid_spacer(|_| {
            (0..count)
                .filter(|&i| count <= 4 || i == 0 || i == count / 2 || i == count - 1)
                .map(|i| GridMark {
                    value: i as f64,
                    step_size: (count / 2).max(1) as f64,
                })
                .collect()
        })
        .x_axis_formatter(|mark, _| format!("{:0prefix_bits$b}", mark.value as usize))
        .y_grid_spacer(|_| {
            [0.0, ceiling / 2.0, ceiling]
                .map(|value| GridMark {
                    value,
                    step_size: ceiling / 2.0,
                })
                .into()
        })
        .y_axis_formatter(|mark, _| format!("{:.0}%", mark.value))
        .show(ui, |plot_ui| {
            plot_ui.bar_chart(BarChart::new("Probability", bars).allow_hover(false))
        });

    // Use the bounded prefix number throughout, even for very wide registers.
    let hovered = plot
        .response
        .interact_pointer_pos()
        .or_else(|| plot.response.hover_pos())
        .filter(|&pos| plot.transform.frame().contains(pos))
        .map(|pos| {
            plot.transform
                .value_from_position(pos)
                .x
                .round()
                .clamp(0.0, (count - 1) as f64) as usize
        });
    let selection_id = Id::new("results_inspected_probability_range");
    let clicked_index = hovered.filter(|_| plot.response.clicked());
    let pinned_index = ui.data_mut(|data| {
        if let Some(index) = clicked_index {
            data.insert_temp(
                selection_id,
                PinnedRange {
                    wires,
                    first_bin: index * group_size,
                    bin_count: group_size,
                },
            );
        }
        let pinned = data.get_temp::<PinnedRange>(selection_id)?;
        // A coarser bar contains the whole selection. At a finer resolution
        // no single bar represents the original range.
        if pinned.wires != wires || group_size < pinned.bin_count {
            data.remove::<PinnedRange>(selection_id);
            return None;
        }
        Some(pinned.first_bin / group_size)
    });
    if let Some(index) = hovered.or(pinned_index) {
        let frame = plot.transform.frame();
        let x = plot.transform.position_from_point_x(index as f64);
        ui.painter()
            .vline(x, frame.y_range(), ui.visuals().widgets.hovered.fg_stroke);
        let begin = index * group_size;
        let mass: f64 = bins[begin..begin + group_size].iter().sum();
        let label = range_label(index, prefix_bits, wires);
        let percent = mass * 100.0;
        let value = if percent > 0.0 && percent < 0.01 {
            format!("{percent:.2e}%")
        } else {
            format!("{percent:.2}%")
        };
        if simulation.compact_samples.is_some() {
            let hits = (mass * f64::from(COMPACT_SHOTS)).round() as u32;
            ui.label(format!("{label} · {value} · {hits} shots"));
        } else {
            ui.label(format!("{label} · {value}"));
        }
    } else {
        ui.small("Select a bar to inspect its probability.");
    }
    ui.separator();
}

fn bars_and_ceiling(bins: &[f64], group_size: usize, fill: Color32) -> (Vec<Bar>, f64) {
    let mut peak = 0.0_f64;
    let bars = bins
        .chunks_exact(group_size)
        .enumerate()
        .map(|(i, group)| {
            let percent = group.iter().sum::<f64>() * 100.0;
            peak = peak.max(percent);
            Bar::new(i as f64, percent)
                .width(0.78)
                .fill(fill)
                .stroke(Stroke::NONE)
        })
        .collect();
    let ceiling = if peak <= 10.0 {
        10.0
    } else if peak <= 20.0 {
        20.0
    } else {
        100.0
    };
    (bars, ceiling)
}

fn range_label(index: usize, prefix_bits: usize, wires: usize) -> String {
    if wires == prefix_bits {
        format!("Basis index {index}")
    } else if wires <= 32 {
        let suffix_bits = wires - prefix_bits;
        let start = (index as u64) << suffix_bits;
        let end = (((index + 1) as u64) << suffix_bits) - 1;
        format!("Indices {start}–{end}")
    } else {
        format!(
            "Prefix {index:0prefix_bits$b} · {} remaining bits",
            wires - prefix_bits
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_uses_the_largest_displayed_probability() {
        let uniform = [1.0 / 32.0; 32];
        let mut ghz = [0.0; 32];
        ghz[0] = 0.5;
        ghz[31] = 0.5;
        for (group_size, expected) in [(1, 10.0), (2, 10.0), (4, 20.0)] {
            let (bars, ceiling) = bars_and_ceiling(&uniform, group_size, Color32::WHITE);
            assert_eq!(bars.len(), 32 / group_size);
            assert_eq!(ceiling, expected);
            assert_eq!(bars_and_ceiling(&ghz, group_size, Color32::WHITE).1, 100.0);
        }
    }
}
