use crate::messages::Message;
use crate::state::AppState;
use crate::state::ui_state::{DraggedItem, PlacementMode};
use std::collections::BTreeMap;
use crate::prelude::*;

/// The `gate_palette` module is responsible for rendering the gate palette,
/// which displays a list of available gates that can be added to the circuit.
///
/// This module also handles click an place operations for placing gates on the
/// circuit grid, and it dispatches messages to the message handlers when a
/// gate is selected or dragged.
pub fn draw(state: &mut AppState, ui: &mut egui::Ui, messages: &mut Vec<Message>) {
    let mut categories: BTreeMap<String, Vec<(Gate, GateMeta)>> =
        BTreeMap::new();
    for (id, meta) in state.circuit_state.circuit.registry.iter() {
        let category = match meta.id {
            Gate::H | Gate::X | Gate::Y | Gate::Z | Gate::SqrtX | Gate::SqrtY | Gate::SqrtZ => "Pauli Gates",
            Gate::CX | Gate::SWAP | Gate::MOVE | Gate::CZ | Gate::CY | Gate::CCNOT | Gate::CCZ => "Control Gates",
            Gate::Rz | Gate::Rx | Gate::Ry | Gate::CRz | Gate::CXPow | Gate::CYPow | Gate::CZPow => "Parametric Gates",
            Gate::XPow | Gate::YPow | Gate::ZPow | Gate::CCZPow => "EigenGates",
            Gate::Custom => "Custom Gates",
        };
        categories
            .entry(category.to_string())
            .or_default()
            .push((id.clone(), meta.clone()));
    }

    ui.heading("Gate Palette");

    for (category, gates) in categories {
        ui.collapsing(category.clone(), |ui| {
            for (id, gate) in gates {
                let is_selected = match &state.ui_state.placement_mode {
                    PlacementMode::Placing => state.ui_state.selected_gate == Some(id.clone()),
                    _ => false,
                };

                if category == "Custom Gates" {
                    ui.horizontal(|ui| {
                        let label = ui.selectable_label(is_selected, &gate.label);
                        let response = label; // Capture the response for drag_started

                        if response.drag_started() {
                            let payload = DraggedItem::Gate(id.clone());
                            egui::DragAndDrop::set_payload(ui.ctx(), payload);
                        }

                        if response.clicked() {
                            messages.push(Message::SelectGate(id.clone()));
                        }
                        if response.secondary_clicked() {
                            messages.push(Message::OpenGateEditor(id.clone()));
                        }
                        if ui.button("Edit").clicked() {
                            messages.push(Message::OpenCustomGateEditor(id.clone()));
                        }
                    });
                } else {
                    let label = ui.selectable_label(is_selected, &gate.label);
                    let response = label; // Capture the response for drag_started

                    if response.drag_started() {
                        let payload = DraggedItem::Gate(id.clone());
                        egui::DragAndDrop::set_payload(ui.ctx(), payload);
                    }

                    if response.clicked() {
                        messages.push(Message::SelectGate(id.clone()));
                    }
                    if response.secondary_clicked() {
                        messages.push(Message::OpenGateEditor(id.clone()));
                    }
                }
            }
        });
    }
}
