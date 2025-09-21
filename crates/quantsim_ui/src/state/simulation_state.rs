use quantsim_core::core::engine::QuantumState;

use super::pagination::PaginationState;

/// Represents the state of the simulation.
#[derive(Debug)]
pub struct SimulationState {
    pub quantum_state: Option<QuantumState>,
    /// A list of quantum states at each timestep.
    pub snapshot_states: Vec<QuantumState>,
    pub pagination: PaginationState,
    pub page_input_text: String,
}

impl Default for SimulationState {
    fn default() -> Self {
        Self {
            quantum_state: Default::default(),
            snapshot_states: Default::default(),
            pagination: PaginationState::new(0, 20), // sensible defaults for pagination
            page_input_text: String::new(),
        }
    }
}
