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
                let mut color = Color32::from_rgb(100, 100, 100);
                if op.qubits.len() > 1 {
                    if let Some(&gate_id) = column_map.get(&row) {
                        color = Color32::from_rgb(
                            (gate_id as u8 * 50) % 255,
                            (gate_id as u8 * 30) % 255,
                            (gate_id as u8 * 70) % 255,
                        );
                    }
                }
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
