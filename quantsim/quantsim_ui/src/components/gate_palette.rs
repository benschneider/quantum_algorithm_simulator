use crate::messages::Message;
use crate::state::AppState;
use crate::state::ui_state::PlacementMode;
use std::collections::BTreeMap;
use quantsim_core::core::types::Gate;
use quantsim_core::core::gates::registry::GateMeta;

/// The `gate_palette` module is responsible for rendering the gate palette,
/// which displays a list of available gates that can be added to the circuit.
///
/// This module also handles click an place operations for placing gates on the
/// circuit grid, and it dispatches messages to the message handlers when a
/// gate is selected or dragged.
pub fn gate_palette(state: &mut AppState, ui: &mut egui::Ui, messages: &mut Vec<Message>) {
    let mut categories: BTreeMap<String, Vec<(Gate, GateMeta)>> =
        BTreeMap::new();
    for (id, meta) in state.circuit_state.circuit.registry.iter() {
        let category = if meta.arity.as_usize() > 1 {
            "Control Gates"
        } else if meta.is_parametric {
            "Parametric Gates"
        } else {
            match meta.id {
                Gate::H
                | Gate::X
                | Gate::Y
                | Gate::Z
                | Gate::SqrtX
                | Gate::SqrtY
                | Gate::SqrtZ => "Pauli Gates",
                Gate::Custom => "Custom Gates",
                Gate::XPow
                | Gate::YPow
                | Gate::ZPow
                | Gate::CCZPow => "EigenGates",
                _ => "Standard Gates",
            }
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
                        if label.clicked() {
                            messages.push(Message::SelectGate(id.clone()));
                        }
                        if label.secondary_clicked() {
                            messages.push(Message::OpenGateEditor(id.clone()));
                        }
                        if ui.button("Edit").clicked() {
                            messages.push(Message::OpenCustomGateEditor(id.clone()));
                        }
                    });
                } else {
                    let label = ui.selectable_label(is_selected, &gate.label);
                    if label.clicked() {
                        messages.push(Message::SelectGate(id.clone()));
                    }
                    if label.secondary_clicked() {
                        messages.push(Message::OpenGateEditor(id.clone()));
                    }
                }
            }
        });
    }
}
