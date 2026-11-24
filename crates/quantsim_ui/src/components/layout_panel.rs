use super::{
    qubit_layout::{draw_map, site_name},
    spacetime_stack::draw_stack,
};
use crate::{
    messages::Message,
    state::{
        ui_state::{CentralPanelTab, LayoutView, StackCamera},
        AppState, SimulationState,
    },
};
use quantsim_core::circuit::{Circuit, QubitLayout};

pub(crate) fn show(ui: &mut egui::Ui, state: &mut AppState, messages: &mut Vec<Message>) {
    let circuit = &state.circuit_state.circuit;
    let view = &mut state.ui_state;
    let step = view.current_timestep;

    let mut width = ui
        .available_rect_before_wrap()
        .intersect(ui.clip_rect())
        .width();
    if view.show_results_window && ui.ctx().screen_rect().width() >= 920.0 {
        width = (width - 380.0).max(280.0).min(width);
    }
    let frame = egui::Frame::group(ui.style());
    if view.active_central_tab != CentralPanelTab::Layout {
        width = width.min(660.0);
    }
    let inner_width = (width - frame.total_margin().sum().x).max(1.0);
    frame.show(ui, |ui| {
        ui.set_width(inner_width);
        ui.horizontal_wrapped(|ui| {
            ui.heading("Qubit layout");
            let caption = match (step, circuit.section_at(step)) {
                (0, _) => "Step 0 · Initial state".to_owned(),
                (_, Some(section)) => format!("Step {step} · {}", section.label),
                _ => format!("Step {step}"),
            };
            ui.label(caption);
        });
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut view.layout_view, LayoutView::Plane, "2D layout");
            ui.selectable_value(&mut view.layout_view, LayoutView::Stack, "3D stack");
            if view.layout_draft.is_none() && ui.button("Edit layout").clicked() {
                view.layout_draft = Some(
                    circuit
                        .layout
                        .clone()
                        .unwrap_or_else(|| wire_order(circuit.num_qubits)),
                );
                view.selected_layout_wire.get_or_insert(0);
            }
        });
        ui.separator();
        let fallback;
        let saved = if let Some(layout) = &circuit.layout {
            layout
        } else {
            fallback = wire_order(circuit.num_qubits);
            &fallback
        };
        // A wire can temporarily share a site while its coordinates are being edited.
        // Draw the saved arrangement until every site in the preview is distinct.
        let validation = view
            .layout_draft
            .as_ref()
            .and_then(|draft| draft.validate(circuit.num_qubits).err());
        let visible = view
            .layout_draft
            .as_ref()
            .filter(|_| validation.is_none())
            .unwrap_or(saved);
        match view.layout_view {
            LayoutView::Plane => {
                draw_map(ui, circuit, visible, step, &mut view.selected_layout_wire)
            }
            LayoutView::Stack => {
                if let Some((wire, step)) = draw_stack(
                    ui,
                    circuit,
                    visible,
                    step,
                    view.selected_layout_wire,
                    &mut view.stack_camera,
                    view.active_central_tab == CentralPanelTab::Layout,
                ) {
                    messages.push(Message::InspectLayoutSite { wire, step });
                }
                stack_inspector(
                    ui,
                    circuit,
                    step,
                    view.selected_layout_wire,
                    &state.simulation_state,
                    messages,
                );
                egui::CollapsingHeader::new("Camera")
                    .show(ui, |ui| camera_controls(ui, &mut view.stack_camera));
            }
        }
        if let Some(draft) = &mut view.layout_draft {
            ui.separator();
            let wire = view.selected_layout_wire.get_or_insert(0);
            edit_layout(ui, draft, wire);
            // Revalidate after this frame's edits so Save responds to the current inputs.
            let error = draft.validate(circuit.num_qubits).err();
            if let Some(reason) = &error {
                ui.colored_label(ui.visuals().error_fg_color, reason);
            }
            let mut cancel = false;
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(error.is_none(), egui::Button::new("Save layout"))
                    .clicked()
                {
                    messages.push(Message::SaveLayout);
                }
                cancel = ui.button("Cancel").clicked();
            });
            if cancel {
                view.layout_draft = None;
            }
        } else {
            if circuit.layout.is_none() {
                ui.weak("Shown in wire order. Edit layout to assign positions and links.");
            }
            if let Some(section) = circuit.sections.iter().find(|section| section.end > step) {
                let caption = format!("Read after {} (step {})", section.label, section.end);
                if ui.small_button(caption).clicked() {
                    messages.push(Message::SelectTimestep(section.end));
                }
            }
        }
    });
}

fn stack_inspector(
    ui: &mut egui::Ui,
    circuit: &Circuit,
    step: usize,
    selected: Option<usize>,
    simulation: &SimulationState,
    messages: &mut Vec<Message>,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Inspect");
        let label = |wire| {
            let name = site_name(circuit, wire);
            if circuit.qubit_roles[wire] == Some(quantsim_core::circuit::QubitRole::Ancilla) {
                format!("{name} (q{wire})")
            } else {
                name
            }
        };
        egui::ComboBox::from_id_salt("stack_site")
            .selected_text(selected.map_or_else(|| "Select wire".into(), label))
            .show_ui(ui, |ui| {
                for wire in 0..circuit.num_qubits {
                    if ui
                        .selectable_label(selected == Some(wire), label(wire))
                        .clicked()
                    {
                        messages.push(Message::InspectLayoutSite { wire, step });
                    }
                }
            });
        if let Some(wire) = selected {
            let operation = step
                .checked_sub(1)
                .and_then(|i| circuit.steps.get(i))
                .and_then(|ops| ops.iter().find(|op| op.qubits.contains(&(wire as u32))));
            if let Some(op) = operation {
                let wires = op
                    .qubits
                    .iter()
                    .map(|q| format!("q{q}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let caption = format!("{} ({wires})", op.id);
                ui.label(match &op.name {
                    Some(name) => format!("{caption} · {name}"),
                    None => caption,
                });
            } else if step == 0 {
                ui.label("Initial state");
            } else {
                ui.label("Idle at this step");
            }
            if let Some(group) = simulation.group_wires(wire) {
                if group.len() == 1 {
                    ui.label("Independent");
                } else {
                    ui.label(format!("{}-qubit group", group.len()))
                        .on_hover_ui(|ui| {
                            let mut wires = group.to_vec();
                            wires.sort_unstable();
                            ui.label(
                                wires
                                    .iter()
                                    .map(|q| format!("q{q}"))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            );
                        });
                }
            }
        }
    });
}

fn camera_controls(ui: &mut egui::Ui, camera: &mut StackCamera) {
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::Slider::new(&mut camera.yaw, -180.0..=180.0)
                .text("Orbit")
                .suffix("°")
                .step_by(1.0),
        );
        ui.add(
            egui::Slider::new(&mut camera.pitch, 0.0..=75.0)
                .text("Tilt")
                .suffix("°")
                .step_by(1.0),
        );
        if ui.small_button("Reset view").clicked() {
            *camera = StackCamera::default();
        }
    });
}

fn wire_order(count: usize) -> QubitLayout {
    QubitLayout {
        positions: (0..count).map(|wire| [wire as i16, 0]).collect(),
        links: Vec::new(),
    }
}

fn edit_layout(ui: &mut egui::Ui, draft: &mut QubitLayout, wire: &mut usize) {
    let count = draft.positions.len();
    ui.horizontal_wrapped(|ui| {
        ui.label("Arrange as");
        if ui.button("Line").clicked() {
            *draft = QubitLayout::line(count);
        }
        if ui.button("Grid").clicked() {
            *draft = QubitLayout::grid(count);
        }
        if ui.button("Ring").clicked() {
            *draft = QubitLayout::ring(count);
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Site");
        egui::ComboBox::from_id_salt("layout_wire")
            .width(60.0)
            .selected_text(format!("q{wire}"))
            .show_ui(ui, |ui| {
                for index in 0..count {
                    ui.selectable_value(wire, index, format!("q{index}"));
                }
            });
        if let Some([x, y]) = draft.positions.get_mut(*wire) {
            ui.label("X");
            ui.add(egui::DragValue::new(x));
            ui.label("Y");
            ui.add(egui::DragValue::new(y));
        }
    });
    if count > 1 {
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Links from q{wire}"));
            for other in (0..count).filter(|other| other != wire) {
                let existing = draft
                    .links
                    .iter()
                    .position(|edge| *edge == [*wire, other] || *edge == [other, *wire]);
                let mut linked = existing.is_some();
                if ui.checkbox(&mut linked, format!("q{other}")).changed() {
                    if let Some(index) = existing {
                        draft.links.remove(index);
                    } else {
                        draft.links.push([*wire, other]);
                    }
                }
            }
        });
    }
}
