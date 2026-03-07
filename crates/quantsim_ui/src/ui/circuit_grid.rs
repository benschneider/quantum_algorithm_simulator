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
    /// rendering.
    pub cells: Vec<GridCell>,
}

/// Represents a single cell on the circuit grid.
/// Each cell has a pre-calculated `Rect` for its position and size,
/// making the rendering loop simpler.
#[derive(Clone, Debug)]
pub enum GridCell {
    /// A label for a qubit, e.g., "q0:".
    QubitLabel { rect: Rect, label: String },
    /// A visual indicator for the current time step in the simulation.
    TimeIndicator { rect: Rect, color: Color32 },
    /// A quantum gate.
    Gate(GateCell),
    /// An empty cell, representing a point where a gate could be placed.
    Empty { rect: Rect, row: usize, col: usize },
}

/// Represents a quantum gate on the circuit grid.
#[derive(Clone, Debug)]
pub struct GateCell {
    /// A unique identifier for the gate instance.
    pub id: Gate,
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
    let num_qubits = circuit.num_qubits;
    let num_steps = num_timesteps;

    let (cell_width, cell_height) = (60.0, 30.0);
    let required_width = cell_width * num_steps as f32;
    let required_height = cell_height * (num_qubits + 1) as f32;

    let (response, _painter) = ui.allocate_painter(
        egui::Vec2::new(required_width, required_height),
        egui::Sense::hover(),
    );
    let canvas_rect = response.rect;

    let mut max_label_width = 0.0;
    for row in 0..num_qubits {
        let label = format!("q{}:", row);
        let label_width = ui.fonts(|f| f.glyph_width(&egui::FontId::proportional(14.0), 'm'))
            * label.len() as f32;
        if label_width > max_label_width {
            max_label_width = label_width;
        }
    }
    max_label_width += 10.0;

    let grid_rect = egui::Rect::from_min_size(
        canvas_rect.min + egui::vec2(max_label_width, 0.0),
        canvas_rect.size() - egui::vec2(max_label_width, 0.0),
    );

    let mut cells = Vec::new();

    for row in 0..num_qubits {
        let label_pos =
            canvas_rect.min + egui::vec2(5.0, (row + 1) as f32 * cell_height + cell_height / 2.0);
        cells.push(GridCell::QubitLabel {
            rect: Rect::from_center_size(label_pos, egui::vec2(max_label_width, cell_height)),
            label: format!("q{}:", row),
        });
    }

    let mut column_maps = Vec::new();
    for col in 0..num_steps {
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
        column_maps.push(multi_gate_map);
    }

    if current_timestep > 0 {
        let col = current_timestep - 1;
        let indicator_rect = egui::Rect::from_min_size(
            grid_rect.min + egui::vec2(col as f32 * cell_width, 0.0),
            egui::vec2(cell_width, cell_height),
        )
        .shrink(2.0);
        cells.push(GridCell::TimeIndicator {
            rect: indicator_rect,
            color: Color32::DARK_BLUE,
        });
    }

    for row in 0..num_qubits {
        for (col, column_map) in column_maps.iter().enumerate().take(num_steps) {
            let gate_rect = egui::Rect::from_min_size(
                grid_rect.min + egui::vec2(col as f32 * cell_width, (row + 1) as f32 * cell_height),
                egui::vec2(cell_width, cell_height),
            )
            .shrink(2.0);

            let gate_op = circuit.steps[col]
                .iter()
                .find(|op| op.qubits.contains(&(row as u32)));

            if let Some(op) = gate_op {
                let gate_group = if op.qubits.len() > 1 {
                    column_map.get(&row).copied()
                } else {
                    None
                };
                let color = choose_gate_fill(ui, gate_group);
                let text_color = choose_text_color(color);
                let (full_name, description) = if let Some(meta) = circuit.registry.get_meta(&op.id)
                {
                    (meta.label.clone(), meta.description.clone())
                } else {
                    (op.id.to_string(), "Unknown gate".to_string())
                };
                let instance_id = if op.qubits.len() > 1 {
                    column_map.get(&row).copied()
                } else {
                    None
                };
                cells.push(GridCell::Gate(GateCell {
                    id: op.id.clone(),
                    rect: gate_rect,
                    color,
                    text_color,
                    qubits: op.qubits.iter().map(|&q| q as usize).collect(),
                    row,
                    col,
                    full_name,
                    description,
                    params: op.params.to_vec(),
                    instance_id,
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
    for col in 0..=num_steps {
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
