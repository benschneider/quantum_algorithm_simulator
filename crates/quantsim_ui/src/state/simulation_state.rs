use quantsim_core::core::engine::QuantumState;

use super::pagination::PaginationState;

/// Represents the state of the simulation.
#[derive(Debug)]
pub struct SimulationState {
    pub quantum_state: Option<QuantumState>,
    /// Marginals in wire order (q0 first), including every basis probability.
    pub(crate) wire_one_probabilities: Vec<f64>,
    /// Basis indices shown in the joint-state table and chart.
    pub(crate) populated_indices: Vec<usize>,
    /// A list of quantum states at each timestep.
    pub snapshot_states: Vec<QuantumState>,
    pub pagination: PaginationState,
    pub page_input_text: String,
}

impl Default for SimulationState {
    fn default() -> Self {
        Self {
            quantum_state: Default::default(),
            wire_one_probabilities: Vec::new(),
            populated_indices: Vec::new(),
            snapshot_states: Default::default(),
            pagination: PaginationState::new(0, 20), // sensible defaults for pagination
            page_input_text: String::new(),
        }
    }
}

impl SimulationState {
    pub(crate) fn set_readout(
        &mut self,
        quantum_state: QuantumState,
        probabilities: &[f32],
        reset_page: bool,
    ) {
        self.wire_one_probabilities = vec![0.0; quantum_state.num_qubits];
        self.populated_indices.clear();
        for (index, (amp, &probability)) in quantum_state
            .state_vector
            .iter()
            .zip(probabilities)
            .enumerate()
        {
            if amp.re.abs() > 1e-6 || amp.im.abs() > 1e-6 {
                self.populated_indices.push(index);
            }
            if probability > 0.0 {
                let mut bits = index;
                while bits != 0 {
                    self.wire_one_probabilities[bits.trailing_zeros() as usize] +=
                        f64::from(probability);
                    bits &= bits - 1;
                }
            }
        }

        let page = if reset_page {
            1
        } else {
            self.pagination.current_page
        };
        self.pagination = PaginationState::new(
            self.populated_indices.len(),
            self.pagination.entries_per_page,
        );
        self.pagination.set_page(page);
        self.page_input_text = self.pagination.current_page.to_string();
        self.quantum_state = Some(quantum_state);
    }
}
