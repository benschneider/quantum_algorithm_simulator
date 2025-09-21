use crate::state::AppState;

pub fn new_circuit_handler(state: &mut AppState) {
    *state = AppState::default();
}
