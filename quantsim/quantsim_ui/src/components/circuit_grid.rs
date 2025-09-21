use crate::messages::Message;
use crate::state::AppState;
use crate::state::ui_state::{DraggedItem, PlacementMode};
use crate::ui::circuit_grid::{self, GridCell};

/// The `circuit_grid` module is responsible for rendering the quantum circuit
/// grid. It takes the circuit state and renders it as a grid of qubits and
/// timesteps.
///
/// This module also handles user interactions with the circuit grid, such as
/// placing, moving, and deleting gates. These interactions are translated
/// into messages that are dispatched to the message handlers.
pub fn circuit_grid(
    state: &mut AppState,
    ui: &mut egui::Ui,
    messages: &mut Vec<Message>,
    current_timestep: usize,
) {
    let circuit = &mut state.circuit_state.circuit;
    let grid = circuit_grid::generate_circuit_grid(
        circuit,
        ui,
        state.circuit_state.num_timesteps,
        current_timestep,
    );

    let response = ui.interact(grid.rect, ui.id(), egui::Sense::click());
    if response.clicked() {
        state.ui_state.placement_mode = PlacementMode::Idle;
        state.ui_state.selected_gate = None;
    }

    for cell in &grid.cells {
        match cell {
            GridCell::QubitLabel { rect, label } => {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(14.0),
                    ui.style().visuals.text_color(),
                );
            }
            GridCell::TimeIndicator { .. } => {
                // This is now handled by the gray-out overlay in generate_circuit_grid
            }
            GridCell::Gate(gate) => {
                let text = if let Some(id) = gate.instance_id {
                    format!("{}{}", gate.id, id)
                } else {
                    gate.id.to_string()
                };
                let button = egui::Button::new(text).fill(gate.color);
                let response = ui.put(gate.rect, button).on_hover_text(format!(
                    "{}\n{}\nQubits: {:?}\nParams: {:?}",
                    gate.full_name, gate.description, gate.qubits, gate.params
                ));

                if response.drag_started() {
                    let payload = DraggedItem::GridGate(gate.row, gate.col);
                    egui::DragAndDrop::set_payload(ui.ctx(), payload);
                }

                if response.clicked() {
                    state.ui_state.placement_mode = PlacementMode::Idle;
                    messages.push(Message::SelectGateForEditing(gate.qubits[0], gate.col));
                }

                if response.secondary_clicked() {
                    messages.push(Message::DeleteGate(gate.row, gate.col));
                }
            }
            GridCell::Empty { rect, row, col } => {
                let is_placing = matches!(state.ui_state.placement_mode, PlacementMode::Placing);
                let response = ui.put(*rect, egui::Button::new("").sense(egui::Sense::click()));

                if is_placing && response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    ui.painter().rect_stroke(
                        *rect,
                        0.0,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 255, 0)),
                        egui::StrokeKind::Middle,
                    );
                }

                if response.clicked() {
                    if let Some(gate) = state.ui_state.selected_gate.clone() {
                        let arity = state
                            .circuit_state
                            .circuit
                            .registry
                            .get_meta(&gate)
                            .map(|m| m.arity.as_usize())
                            .unwrap_or(1);

                        let mut new_placement_mode = None;

                        if let PlacementMode::PendingPlacement { qubits, col } =
                            &mut state.ui_state.placement_mode
                        {
                            if !qubits.contains(row) {
                                qubits.push(*row);
                            }

                            if qubits.len() == arity {
                                messages.push(Message::PlaceMultiQubitGate(
                                    gate,
                                    qubits.clone(),
                                    *col,
                                ));
                                new_placement_mode = Some(PlacementMode::Placing);
                            }
                        } else if matches!(&state.ui_state.placement_mode, PlacementMode::Placing) {
                            if arity == 1 {
                                messages.push(Message::PlaceGate(gate, *row, *col));
                                new_placement_mode = Some(PlacementMode::Placing);
                            } else {
                                new_placement_mode = Some(PlacementMode::PendingPlacement {
                                    qubits: vec![*row],
                                    col: *col,
                                });
                            }
                        }

                        if let Some(mode) = new_placement_mode {
                            state.ui_state.placement_mode = mode;
                        }
                    }
                }

                if let Some(payload) = egui::DragAndDrop::payload::<DraggedItem>(ui.ctx()) {
                    if response.hovered() {
                        match payload.as_ref() {
                            DraggedItem::Gate(gate_id) => {
                                messages.push(Message::PlaceGate(gate_id.clone(), *row, *col));
                                egui::DragAndDrop::clear_payload(ui.ctx());
                            }
                            DraggedItem::GridGate(from_row, from_col) => {
                                messages.push(Message::MoveGate(*from_row, *from_col, *row, *col));
                                egui::DragAndDrop::clear_payload(ui.ctx());
                            }
                        }
                    }
                }
            }
        }
    }

    if ui.input(|i| i.key_pressed(egui::Key::Escape)) || ui.input(|i| i.pointer.secondary_clicked())
    {
        if let PlacementMode::PendingPlacement { .. } = state.ui_state.placement_mode {
            state.ui_state.placement_mode = PlacementMode::Idle;
            state.ui_state.selected_gate = None;
        }
    }

    if ui.input(|i| i.pointer.any_released()) {
        if let Some(payload) = egui::DragAndDrop::payload::<DraggedItem>(ui.ctx()) {
            if let DraggedItem::GridGate(row, col) = payload.as_ref() {
                if !grid
                    .grid_rect
                    .contains(ui.input(|i| i.pointer.hover_pos().unwrap_or_default()))
                {
                    messages.push(Message::DeleteGate(*row, *col));
                }
            }
            egui::DragAndDrop::clear_payload(ui.ctx());
        }
    }

    if let PlacementMode::PendingPlacement { qubits, col } = &state.ui_state.placement_mode {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);

        if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
            let gate_color = egui::Color32::from_rgba_unmultiplied(100, 100, 100, 128);
            let text_color = egui::Color32::WHITE;
            let font_id = egui::FontId::proportional(14.0);

            let placed_qubit_rects: Vec<_> = qubits
                .iter()
                .filter_map(|placed_row| {
                    grid.cells.iter().find_map(|c| match c {
                        GridCell::Empty {
                            rect,
                            row,
                            col: cell_col,
                        } if row == placed_row && *cell_col == *col => Some(*rect),
                        _ => None,
                    })
                })
                .collect();

            // Draw ghost gates for already placed qubits
            for (i, rect) in placed_qubit_rects.iter().enumerate() {
                ui.painter().rect_filled(*rect, 5.0, gate_color);
                if let Some(gate_id) = &state.ui_state.selected_gate {
                    let text = format!("{}_{}", gate_id, i + 1);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        text,
                        font_id.clone(),
                        text_color,
                    );
                }
            }

            // Draw connecting lines and ghost gate for the hover position
            if let Some(last_rect) = placed_qubit_rects.last() {
                let start_pos = last_rect.center();
                ui.painter().line_segment(
                    [start_pos, hover_pos],
                    egui::Stroke::new(2.0, egui::Color32::YELLOW),
                );
            }

            let hover_cell = grid.cells.iter().find(|c| match c {
                GridCell::Empty { rect, .. } => rect.contains(hover_pos),
                _ => false,
            });

            if let Some(GridCell::Empty { rect, row, .. }) = hover_cell {
                if !qubits.contains(row) {
                    ui.painter().rect_filled(*rect, 5.0, gate_color);
                    if let Some(gate_id) = &state.ui_state.selected_gate {
                        let text = format!("{}_{}", gate_id, qubits.len() + 1);
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            text,
                            font_id,
                            text_color,
                        );
                    }
                }
            }
        }
    }
}
