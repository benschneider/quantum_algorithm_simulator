use crate::{messages::Message, state::AppState};

pub fn handle_circuit_controls_message(
    state: &mut AppState,
    message: Message,
    _messages: &mut Vec<Message>,
) {
    match message {
        Message::ChangeQubits(new_qubits) => {
            resize_wires(state, new_qubits);
            state
                .circuit_state
                .circuit
                .steps
                .resize(state.circuit_state.num_timesteps, Vec::new());
        }
        Message::SetQubitRole { row, role } => {
            if let Some(wire) = state.circuit_state.circuit.qubit_roles.get_mut(row) {
                *wire = role;
            }
        }
        Message::ChangeTimesteps(new_timesteps) => {
            resize_steps(state, new_timesteps);
        }
        Message::SaveLayout => {
            if let Some(draft) = &state.ui_state.layout_draft {
                if draft.validate(state.circuit_state.num_qubits).is_ok() {
                    state.circuit_state.circuit.layout = state.ui_state.layout_draft.take();
                }
            }
        }
        Message::SaveSection => {
            if let Some(draft) = &mut state.ui_state.section_editor {
                match state
                    .circuit_state
                    .circuit
                    .set_section(draft.original_start, draft.section.clone())
                {
                    Ok(()) => state.ui_state.section_editor = None,
                    Err(error) => draft.error = Some(error),
                }
            }
        }
        Message::DeleteSection => {
            if let Some(start) = state
                .ui_state
                .section_editor
                .take()
                .and_then(|draft| draft.original_start)
            {
                state.circuit_state.circuit.remove_section(start);
            }
        }
        Message::CircuitDimensionsChanged {
            num_qubits,
            num_timesteps,
        } => {
            resize_wires(state, num_qubits);
            resize_steps(state, num_timesteps);
            state
                .circuit_state
                .circuit
                .initial_preps
                .resize(num_qubits, None);
            state.initial_state_editor_state =
                crate::state::InitialStateEditorState::new(num_qubits);
        }
        _ => {}
    }
}

fn resize_steps(state: &mut AppState, count: usize) {
    if count < state.circuit_state.num_timesteps {
        state.ui_state.section_editor = None;
    }
    state.circuit_state.num_timesteps = count;
    let circuit = &mut state.circuit_state.circuit;
    circuit.steps.resize(count, Vec::new());
    circuit.trim_sections();
    if state.ui_state.current_timestep > count {
        state.ui_state.current_timestep = count;
        state.simulation_state = Default::default();
    }
}

fn resize_wires(state: &mut AppState, count: usize) {
    if count != state.circuit_state.num_qubits {
        state.simulation_state = Default::default();
        state.ui_state.current_timestep = 0;
        state.ui_state.layout_draft = None;
        state.ui_state.selected_layout_wire = state
            .ui_state
            .selected_layout_wire
            .filter(|&wire| wire < count);
        if let Some(layout) = &mut state.circuit_state.circuit.layout {
            layout.resize(count);
        }
    }
    state.circuit_state.num_qubits = count;
    state.circuit_state.circuit.num_qubits = count;
    state.circuit_state.circuit.qubit_roles.resize(count, None);
}
