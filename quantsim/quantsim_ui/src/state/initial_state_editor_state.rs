use nalgebra::Complex;

use super::pagination::PaginationState;

/// Represents the state of the initial state editor.
#[derive(Clone, Debug, PartialEq)]
pub struct InitialStateEditorState {
    pub state_vector: Vec<Complex<f32>>,
    pub num_qubits: usize,
    pub pagination: PaginationState,
    pub page_input_text: String,
}

impl InitialStateEditorState {
    /// Creates a new `InitialStateEditorState` with a default state vector
    /// for the given number of qubits.
    pub fn new(num_qubits: usize) -> Self {
        let mut vector = vec![Complex::new(0.0, 0.0); 1 << num_qubits];
        if !vector.is_empty() {
            vector[0] = Complex::new(1.0, 0.0);
        }
        let total_amplitudes = vector.len();
        let entries_per_page = 20; // As specified in the instructions
        let pagination = PaginationState::new(total_amplitudes, entries_per_page);
        let page_input_text = pagination.current_page.to_string(); // Capture before move
        Self {
            state_vector: vector,
            num_qubits,
            pagination,
            page_input_text,
        }
    }
}

impl Default for InitialStateEditorState {
    fn default() -> Self {
        Self::new(1) // Simply call new, which now handles page_input_text initialization
    }
}
