use crate::state::AppState;

pub fn new_circuit_handler(state: &mut AppState) {
    let active_lesson = state.ui_state.active_tutorial_step;
    let tutorial_open = state.ui_state.show_tutorial_window;
    let palette_custom_matrix = state.circuit_state.palette_custom_matrix.clone();
    *state = AppState::default();
    state.circuit_state.palette_custom_matrix = palette_custom_matrix;
    state.ui_state.active_tutorial_step = active_lesson;
    state.ui_state.show_tutorial_window = tutorial_open;
    state.ui_state.show_welcome_screen = false;
}
