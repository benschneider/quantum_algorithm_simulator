use crate::{
    messages::Message,
    state::{AppState, CircuitState, InitialStateEditorState},
};

pub fn handle_circuit_controls_message(
    state: &mut AppState,
    message: Message,
    _messages: &mut Vec<Message>,
) {
    match message {
        Message::ChangeQubits(new_qubits) => {
            if !resize_wires(state, new_qubits) {
                return;
            }
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
            if !resize_wires(state, num_qubits) {
                return;
            }
            resize_steps(state, num_timesteps);
            state.initial_state_editor_state =
                InitialStateEditorState::from_circuit(&state.circuit_state.circuit);
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

fn resize_wires(state: &mut AppState, count: usize) -> bool {
    if !(1..=CircuitState::MAX_QUBITS).contains(&count) {
        state.ui_state.error_message = Some(format!(
            "Choose between 1 and {} qubits for this simulator.",
            CircuitState::MAX_QUBITS
        ));
        return false;
    }
    if count > CircuitState::MAX_DENSE_QUBITS && state.circuit_state.circuit.initial_state.is_some()
    {
        state.ui_state.error_message = Some(format!(
            "Reset the explicit input in Settings → Set Initial State before resizing to {count} qubits."
        ));
        return false;
    }
    if count != state.circuit_state.num_qubits {
        state.simulation_state = Default::default();
        state.ui_state.current_timestep = 0;
        state.ui_state.error_message = None;
        state.ui_state.layout_draft = None;
        if count > CircuitState::MAX_DENSE_QUBITS {
            state.initial_state_editor_state = InitialStateEditorState::new(count);
        }
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
    state
        .circuit_state
        .circuit
        .initial_preps
        .resize(count, None);
    true
}
