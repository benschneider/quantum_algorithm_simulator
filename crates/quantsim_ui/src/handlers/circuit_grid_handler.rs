use crate::{messages::Message, state::AppState};

pub fn handle_circuit_grid_message(
    state: &mut AppState,
    message: Message,
    _messages: &mut Vec<Message>,
) {
    match message {
        Message::PlaceGate(gate, row, col) => {
            state.circuit_state.place_gate(gate, row, col);
        }
        Message::PlaceMultiQubitGate(gate, qubits, col) => {
            state
                .circuit_state
                .place_multi_qubit_gate(gate, qubits, col);
        }
        Message::MoveGate(from_row, from_col, to_row, to_col) => {
            state
                .circuit_state
                .move_gate(from_row, from_col, to_row, to_col);
        }
        Message::DeleteGate(row, col) => {
            state.circuit_state.delete_gate(row, col);
        }
        Message::SelectGateForEditing(row, col) => {
            state.ui_state.selected_gate_for_editing = Some((row, col));
            state.ui_state.placement_mode = crate::state::ui_state::PlacementMode::Idle;
            state.ui_state.show_gate_editor_window = true;
        }
        _ => {} //panic!("Unhandled message in circuit grid handler: {:?}", message); }
    }
}
