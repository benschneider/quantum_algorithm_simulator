use egui::{Color32, Rect};
use quantsim_core::core::gates::Gate;

/// Represents the entire drawable circuit canvas.
/// This structure is an intermediate representation, generated from the
/// core `Circuit` data, and is designed to be directly consumed by the
/// rendering logic.
pub struct CircuitGrid {
    /// The bounding box of the entire grid.
    pub rect: Rect,
    /// The bounding box of the grid cells, excluding labels.
    pub grid_rect: Rect,
    /// The number of qubit rows in the grid.
    pub num_qubits: usize,
    /// The number of time-step columns in the grid.
    pub num_steps: usize,
    /// A flattened vector of all cells in the grid. The cells are stored
    /// in row-major order, but also contain their own `Rect` for flexible
    /// rendering. The interactive view retains only visible cells.
    pub cells: Vec<GridCell>,
}

impl CircuitGrid {
    pub(crate) fn qubit_cell_rect(&self, row: usize, col: usize) -> Option<Rect> {
        if row >= self.num_qubits || col >= self.num_steps {
            return None;
        }
        let width = self.grid_rect.width() / self.num_steps as f32;
        let height = self.grid_rect.height() / (self.num_qubits + 1) as f32;
        let corner = self.grid_rect.min + egui::vec2(col as f32 * width, (row + 1) as f32 * height);
        Some(Rect::from_min_size(corner, egui::vec2(width, height)).shrink(2.0))
    }
}

/// Represents a single cell on the circuit grid.
/// Each cell has a pre-calculated `Rect` for its position and size,
/// making the rendering loop simpler.
#[derive(Clone, Debug)]
pub enum GridCell {
    /// The area for a wire's index and role selector.
    QubitLabel { rect: Rect, row: usize },
    /// A named range above its gate columns.
    Section { rect: Rect, index: usize },
    /// Numbered column for inspecting the state after this step.
    Timestep { rect: Rect, step: usize },
    /// A quantum gate.
    Gate(GateCell),
    /// An empty cell, representing a point where a gate could be placed.
    Empty { rect: Rect, row: usize, col: usize },
}

impl GridCell {
    /// Widget position in the full grid, including cells outside the scroll viewport.
    pub(crate) fn widget_index(&self, sections: usize, qubits: usize, steps: usize) -> usize {
        match self {
            Self::Section { index, .. } => *index,
            Self::QubitLabel { row, .. } => sections + row,
            Self::Timestep { step, .. } => sections + qubits + step - 1,
            Self::Gate(GateCell { row, col, .. }) | Self::Empty { row, col, .. } => {
                sections + qubits + steps + row * steps + col
            }
        }
    }
}

/// Represents a quantum gate on the circuit grid.
#[derive(Clone, Debug)]
pub struct GateCell {
    /// A unique identifier for the gate instance.
    pub id: Gate,
    /// Visible cell label, including this wire's role in a directional gate.
    pub label: String,
    /// The pre-calculated bounding box for the gate.
    pub rect: Rect,
    /// The color of the gate, used for visual distinction, especially
    /// for multi-qubit gates.
    pub color: Color32,
    /// Text color chosen for contrast against the gate fill.
    pub text_color: Color32,
    /// The qubits this gate acts on.
    pub qubits: Vec<usize>,
    /// The row of the gate
    pub row: usize,
    /// The column (time step) of the gate.
    pub col: usize,
    /// The full name of the gate.
    pub full_name: String,
    /// A description of the gate's function.
    pub description: String,
    /// The parameters of the gate.
    pub params: Vec<quantsim_core::core::types::Param>,
    pub instance_id: Option<usize>,
}
use quantsim_core::core::circuit::Circuit;
use std::collections::HashMap;

fn choose_gate_fill(ui: &egui::Ui, gate_group: Option<usize>) -> Color32 {
    let visuals = ui.visuals();
    if let Some(id) = gate_group {
        let palette_light = [
            Color32::from_rgb(236, 244, 255),
            Color32::from_rgb(238, 248, 242),
            Color32::from_rgb(255, 243, 232),
            Color32::from_rgb(244, 238, 255),
        ];
        let palette_dark = [
            Color32::from_rgb(58, 87, 128),
            Color32::from_rgb(46, 104, 76),
            Color32::from_rgb(133, 86, 43),
            Color32::from_rgb(88, 67, 130),
        ];
        if visuals.dark_mode {
            palette_dark[id % palette_dark.len()]
        } else {
            palette_light[id % palette_light.len()]
        }
    } else if visuals.dark_mode {
        Color32::from_rgb(72, 78, 92)
    } else {
        Color32::from_rgb(242, 245, 249)
    }
}

fn choose_text_color(fill: Color32) -> Color32 {
    let luminance = 0.2126 * (fill.r() as f32 / 255.0)
        + 0.7152 * (fill.g() as f32 / 255.0)
        + 0.0722 * (fill.b() as f32 / 255.0);
    if luminance > 0.55 {
        Color32::from_rgb(20, 24, 30)
    } else {
        Color32::from_rgb(245, 247, 250)
    }
}

/// Generates an intermediate representation of the circuit grid that can be
/// consumed by the rendering logic.
///
/// # Arguments
///
/// * `circuit` - The circuit to render.
/// * `ui` - The `egui` user interface.
/// * `num_timesteps` - The number of timesteps in the circuit.
/// * `current_timestep` - The currently selected timestep.
///
/// # Returns
///
/// A `CircuitGrid` struct that contains all the information needed to render
/// the circuit grid.
pub fn generate_circuit_grid(
    circuit: &Circuit,
    ui: &mut egui::Ui,
    num_timesteps: usize,
    current_timestep: usize,
) -> CircuitGrid {
    generate_grid_in_view(
        circuit,
        ui,
        num_timesteps,
        current_timestep,
        Rect::EVERYTHING,
    )
}

// Reserve the full scrollable canvas, but build widgets only for the visible rows and columns.
pub(crate) fn generate_grid_in_view(
    circuit: &Circuit,
    ui: &mut egui::Ui,
    num_timesteps: usize,
    current_timestep: usize,
    viewport: Rect,
) -> CircuitGrid {
    let num_qubits = circuit.num_qubits;
    let num_steps = num_timesteps;

    let label_width = 108.0;
    let cell_width = ((ui.available_width() - label_width) / num_steps.max(1) as f32)
        .floor()
        .clamp(48.0, 60.0);
    let cell_height = 30.0;
    let section_height = if circuit.sections.is_empty() {
        0.0
    } else {
        26.0
    };
    let required_width = label_width + cell_width * num_steps as f32;
    let required_height = section_height + cell_height * (num_qubits + 1) as f32;

    let (response, _painter) = ui.allocate_painter(
        egui::Vec2::new(required_width, required_height),
        egui::Sense::hover(),
    );
    let canvas_rect = response.rect;

    let grid_rect = egui::Rect::from_min_max(
        canvas_rect.min + egui::vec2(label_width, section_height),
        canvas_rect.max,
    );
    let columns = visible_indices(
        viewport.left(),
        viewport.right(),
        grid_rect.left(),
        cell_width,
        num_steps,
    );
    let rows = visible_indices(
        viewport.top(),
        viewport.bottom(),
        grid_rect.top() + cell_height,
        cell_height,
        num_qubits,
    );

    let mut cells = Vec::new();
    for (index, section) in circuit.sections.iter().enumerate() {
        let rect = Rect::from_min_max(
            egui::pos2(
                grid_rect.left() + (section.start - 1) as f32 * cell_width,
                canvas_rect.top(),
            ),
            egui::pos2(
                grid_rect.left() + section.end as f32 * cell_width,
                grid_rect.top(),
            ),
        );
        if viewport.intersects(rect) {
            cells.push(GridCell::Section {
                rect: rect.shrink(2.0),
                index,
            });
        }
    }

    for row in rows.clone() {
        let label_pos = egui::pos2(
            canvas_rect.left(),
            grid_rect.top() + (row + 1) as f32 * cell_height,
        );
        let rect = Rect::from_min_size(label_pos, egui::vec2(label_width, cell_height));
        if viewport.intersects(rect) {
            cells.push(GridCell::QubitLabel {
                rect: rect.shrink(2.0),
                row,
            });
        }
    }

    let mut column_maps = Vec::new();
    for col in columns.clone() {
        let rect = Rect::from_min_size(
            grid_rect.min + egui::vec2(col as f32 * cell_width, 0.0),
            egui::vec2(cell_width, cell_height),
        );
        if viewport.intersects(rect) {
            cells.push(GridCell::Timestep {
                rect: rect.shrink(2.0),
                step: col + 1,
            });
        }
        let mut multi_gate_map = HashMap::new();
        let mut multi_gate_counter = 0;
        for op in &circuit.steps[col] {
            if op.qubits.len() > 1 {
                for &qubit in &op.qubits {
                    multi_gate_map.insert(qubit as usize, multi_gate_counter);
                }
                multi_gate_counter += 1;
            }
        }
        column_maps.push((multi_gate_map, multi_gate_counter));
    }

    for row in rows {
        for (col, (column_map, group_count)) in columns.clone().zip(&column_maps) {
            let gate_rect = egui::Rect::from_min_size(
                grid_rect.min + egui::vec2(col as f32 * cell_width, (row + 1) as f32 * cell_height),
                egui::vec2(cell_width, cell_height),
            )
            .shrink(2.0);

            let gate_op = circuit.steps[col]
                .iter()
                .find(|op| op.qubits.contains(&(row as u32)));

            if let Some(op) = gate_op {
                let gate_group = column_map.get(&row).copied();
                let color = choose_gate_fill(ui, gate_group);
                let text_color = choose_text_color(color);
                let (full_name, mut description) =
                    if let Some(meta) = circuit.registry.get_meta(&op.id) {
                        (meta.label.clone(), meta.description.clone())
                    } else {
                        (op.id.to_string(), "Unknown gate".to_string())
                    };
                let mut label = if let Some((_, target, action)) = op.controlled_action() {
                    if row as u32 == target {
                        description.push_str(&format!("\nTarget wire q{row}"));
                        match action {
                            Gate::XPow => "X^t tgt".to_owned(),
                            Gate::YPow => "Y^t tgt".to_owned(),
                            _ => format!("{action} tgt"),
                        }
                    } else {
                        description.push_str(&format!("\nControl wire q{row}"));
                        "Ctrl".to_owned()
                    }
                } else {
                    op.id.to_string()
                };
                if let Some(group) = gate_group.filter(|_| *group_count > 1) {
                    label.push_str(&format!("·{}", group + 1));
                }
                cells.push(GridCell::Gate(GateCell {
                    id: op.id.clone(),
                    label,
                    rect: gate_rect,
                    color,
                    text_color,
                    qubits: op.qubits.iter().map(|&q| q as usize).collect(),
                    row,
                    col,
                    full_name: match &op.name {
                        Some(name) => format!("{name} ({full_name})"),
                        None => full_name,
                    },
                    description,
                    params: op.params.to_vec(),
                    instance_id: gate_group,
                }));
            } else {
                cells.push(GridCell::Empty {
                    rect: gate_rect,
                    row,
                    col,
                });
            }
        }
    }

    let painter = ui.painter();

    // Draw lines for qubit lanes
    for row in 0..=num_qubits {
        let y = grid_rect.min.y + row as f32 * cell_height;
        let start_pos = egui::pos2(grid_rect.min.x, y);
        let end_pos = egui::pos2(grid_rect.max.x, y);
        painter.line_segment(
            [start_pos, end_pos],
            ui.style().visuals.widgets.noninteractive.bg_stroke,
        );
    }

    // Draw lines for time steps
    for col in columns.start..=columns.end {
        let x = grid_rect.min.x + col as f32 * cell_width;
        let start_pos = egui::pos2(x, grid_rect.min.y);
        let end_pos = egui::pos2(x, grid_rect.max.y);
        painter.line_segment(
            [start_pos, end_pos],
            ui.style().visuals.widgets.noninteractive.bg_stroke,
        );
    }
    // Gray out the area of the circuit that is not part of the current step
    if current_timestep < num_steps {
        let gray_out_rect = egui::Rect::from_min_max(
            egui::pos2(
                grid_rect.min.x + current_timestep as f32 * cell_width,
                grid_rect.min.y,
            ),
            egui::pos2(
                grid_rect.min.x + num_steps as f32 * cell_width,
                grid_rect.max.y,
            ),
        );
        ui.painter().rect_filled(
            gray_out_rect,
            0.0,
            Color32::from_rgba_unmultiplied(128, 128, 128, 128),
        );
    }
    CircuitGrid {
        rect: canvas_rect,
        grid_rect,
        num_qubits,
        num_steps,
        cells,
    }
}

fn visible_indices(
    min: f32,
    max: f32,
    origin: f32,
    size: f32,
    count: usize,
) -> std::ops::Range<usize> {
    let start = ((min - origin) / size).floor() as usize;
    let end = ((max - origin) / size).ceil() as usize;
    start.min(count)..end.min(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantsim_core::core::types::Operation;

    #[test]
    fn scrolled_view_builds_only_visible_cells_with_original_coordinates() {
        let mut circuit = Circuit::new(128);
        circuit.steps.resize_with(1000, Vec::new);
        circuit.steps[999].push(Operation::new(Gate::X, vec![127], vec![]));
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let origin = ui.next_widget_position();
                let viewport = Rect::from_min_max(
                    origin + egui::vec2(108.0 + 997.0 * 48.0, 126.0 * 30.0),
                    origin + egui::vec2(108.0 + 1000.0 * 48.0, 129.0 * 30.0),
                );
                let grid = generate_grid_in_view(&circuit, ui, 1000, 1000, viewport);
                assert_eq!(
                    grid.grid_rect.size(),
                    egui::vec2(1000.0 * 48.0, 129.0 * 30.0)
                );
                assert_eq!(grid.cells.len(), 9);
                let Some(GridCell::Gate(gate)) = grid.cells.last() else {
                    panic!("Missing final gate");
                };
                assert_eq!((gate.row, gate.col, &gate.id), (127, 999, &Gate::X));
                assert_eq!(grid.qubit_cell_rect(127, 999), Some(gate.rect));
            });
        });
    }
}
