use crate::messages::Message;
use crate::state::ui_state::{CentralPanelTab, SectionDraft};
use crate::state::{AppState, CircuitState};
use egui::widgets::DragValue;

/// Renders the circuit controls, which include the "Run" button, qubit and
/// timestep controls, and the timestep slider.
///
/// # Arguments
///
/// * `ui` - The `egui` user interface.
/// * `state` - The application state.
/// * `messages` - A vector of messages to be sent to the application.
pub fn circuit_controls(ui: &mut egui::Ui, state: &mut AppState, messages: &mut Vec<Message>) {
    let circuit_view = state.ui_state.active_central_tab == CentralPanelTab::Circuit;
    ui.horizontal_wrapped(|ui| {
        if ui.button("Run ▶").clicked() {
            messages.push(Message::RunSimulation);
        }

        ui.separator();

        let mut num_qubits = state.circuit_state.num_qubits;
        ui.label("Qubits:");
        if ui
            .add(DragValue::new(&mut num_qubits).range(1..=CircuitState::MAX_QUBITS))
            .changed()
        {
            messages.push(Message::ChangeQubits(num_qubits));
        }

        let mut num_timesteps = state.circuit_state.num_timesteps;
        ui.label("Timesteps:");
        if ui
            .add(
                DragValue::new(&mut num_timesteps)
                    .range(1..=100)
                    .clamp_existing_to_range(false),
            )
            .changed()
        {
            messages.push(Message::ChangeTimesteps(num_timesteps));
        }

        let mut current_timestep = state.ui_state.current_timestep;
        if ui
            .add(
                egui::Slider::new(&mut current_timestep, 0..=state.circuit_state.num_timesteps)
                    .text("Timestep"),
            )
            .changed()
        {
            messages.push(Message::SelectTimestep(current_timestep));
        }
        let last_step = state.circuit_state.circuit.steps.len();
        if circuit_view
            && ui
                .add_enabled(last_step > 0, egui::Button::new("Sections"))
                .clicked()
        {
            let step = current_timestep.clamp(1, last_step);
            let existing = state.circuit_state.circuit.section_at(step);
            state.ui_state.section_editor = Some(SectionDraft::new(step, existing));
        }
    });

    if circuit_view {
        super::section_editor::show(ui.ctx(), state, messages);
    }
}
