use quantsim_core::core::engine::QuantumState;
use quantsim_core::stabilizer::StabilizerState;
use std::cmp::Reverse;

use super::pagination::PaginationState;

pub(crate) const COMPACT_SHOTS: u32 = 1024;
pub(crate) const MAX_SAMPLE_ROWS: usize = 16;

#[derive(Debug)]
pub(crate) struct CompactSamples {
    /// Most frequent sampled bitstrings, with q0 in the least significant bit.
    pub(crate) rows: Vec<(u64, u32)>,
    pub(crate) distinct: usize,
}

/// Represents the state of the simulation.
#[derive(Debug)]
pub struct SimulationState {
    pub quantum_state: Option<QuantumState>,
    pub(crate) compact_samples: Option<CompactSamples>,
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
            compact_samples: None,
            wire_one_probabilities: Vec::new(),
            populated_indices: Vec::new(),
            snapshot_states: Default::default(),
            pagination: PaginationState::new(0, 20), // sensible defaults for pagination
            page_input_text: String::new(),
        }
    }
}

impl SimulationState {
    pub(crate) fn has_readout(&self) -> bool {
        self.quantum_state.is_some() || self.compact_samples.is_some()
    }

    pub(crate) fn from_stabilizer(state: &StabilizerState, wires: usize) -> Self {
        let wire_one_probabilities = (0..wires)
            .map(|wire| f64::from(state.probability_one(wire).expect("validated circuit wire")))
            .collect();
        let mut outcomes = state.sample(COMPACT_SHOTS, 42);
        outcomes.sort_unstable();
        let mut rows: Vec<_> = outcomes
            .chunk_by(|left, right| left == right)
            .map(|group| (group[0], group.len() as u32))
            .collect();
        rows.sort_unstable_by_key(|&(bits, count)| (Reverse(count), bits));
        let distinct = rows.len();
        rows.truncate(MAX_SAMPLE_ROWS);
        rows.shrink_to_fit();

        Self {
            wire_one_probabilities,
            compact_samples: Some(CompactSamples { rows, distinct }),
            ..Default::default()
        }
    }

    pub(crate) fn set_readout(
        &mut self,
        quantum_state: QuantumState,
        probabilities: &[f32],
        reset_page: bool,
    ) {
        self.compact_samples = None;
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
