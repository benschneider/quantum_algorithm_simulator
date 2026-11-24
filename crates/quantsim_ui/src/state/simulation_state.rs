use quantsim_core::core::engine::QuantumState;
use quantsim_core::grouped::GroupedState;
use quantsim_core::product_state::ProductState;
use quantsim_core::stabilizer::{StabilizerAmplitudes, StabilizerState};
use quantsim_core::BasisIndex;

use super::pagination::PaginationState;
mod waveform;
pub(crate) use waveform::{Waveform, WAVEFORM_POINTS};

#[derive(Debug)]
enum CompactAmplitudes {
    Product(ProductState),
    Grouped(GroupedState),
    Stabilizer(StabilizerAmplitudes),
}

pub(crate) const COMPACT_SHOTS: u32 = 1024;
const SAMPLE_SEED: u64 = 42;
pub(crate) const MAX_SAMPLE_ROWS: usize = 16;

#[derive(Debug)]
pub(crate) struct CompactSamples {
    /// Most frequent sampled bitstrings, with q0 in the least significant bit.
    pub(crate) rows: Vec<(BasisIndex, u32)>,
    pub(crate) distinct: usize,
}

impl CompactSamples {
    fn count<T: Ord + Into<BasisIndex>>(mut outcomes: Vec<T>) -> Self {
        outcomes.sort_unstable();
        let mut counts: Vec<(T, u32)> = Vec::new();
        for outcome in outcomes {
            if let Some((_, count)) = counts.last_mut().filter(|(bits, _)| *bits == outcome) {
                *count += 1;
            } else {
                counts.push((outcome, 1));
            }
        }
        counts.sort_unstable_by(|(a, ca), (b, cb)| cb.cmp(ca).then_with(|| a.cmp(b)));
        let distinct = counts.len();
        let rows = counts
            .into_iter()
            .take(MAX_SAMPLE_ROWS)
            .map(|(bits, count)| (bits.into(), count))
            .collect();
        Self { rows, distinct }
    }
}

/// Represents the state of the simulation.
#[derive(Debug)]
pub struct SimulationState {
    pub quantum_state: Option<QuantumState>,
    pub(crate) compact_samples: Option<CompactSamples>,
    compact_amplitudes: Option<CompactAmplitudes>,
    waveform: Option<Waveform>,
    auto_window: (BasisIndex, BasisIndex),
    manual_window: Option<(BasisIndex, BasisIndex)>,
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
            compact_amplitudes: None,
            waveform: None,
            auto_window: (BasisIndex::ZERO, 1u8.into()),
            manual_window: None,
            wire_one_probabilities: Vec::new(),
            populated_indices: Vec::new(),
            snapshot_states: Default::default(),
            pagination: PaginationState::new(0, 20), // sensible defaults for pagination
            page_input_text: String::new(),
        }
    }
}

impl SimulationState {
    pub(crate) fn group_wires(&self, wire: usize) -> Option<&[u32]> {
        match self.compact_amplitudes.as_ref()? {
            CompactAmplitudes::Grouped(state) => state.group_wires(wire),
            _ => None,
        }
    }

    pub(crate) fn retained_group_amplitudes(&self) -> Option<usize> {
        match self.compact_amplitudes.as_ref()? {
            CompactAmplitudes::Grouped(state) => Some(state.stored_amplitudes()),
            _ => None,
        }
    }

    pub(crate) fn retained_stabilizer(&self) -> bool {
        matches!(
            self.compact_amplitudes,
            Some(CompactAmplitudes::Stabilizer(_))
        )
    }

    pub(crate) fn has_readout(&self) -> bool {
        self.quantum_state.is_some() || self.compact_samples.is_some()
    }

    pub(crate) fn from_stabilizer(state: &StabilizerState, wires: usize) -> Self {
        Self::from_samples(
            (0..wires).map(|wire| state.probability_one(wire)),
            |shots, seed| state.sample(shots, seed),
            |shots, seed| state.sample_wide(shots, seed),
        )
        .with_amplitudes(CompactAmplitudes::Stabilizer(state.amplitude_view()))
    }

    pub(crate) fn from_product(state: ProductState, wires: usize) -> Self {
        Self::from_samples(
            (0..wires).map(|wire| state.probability_one(wire)),
            |shots, seed| state.sample(shots, seed),
            |shots, seed| state.sample_wide(shots, seed),
        )
        .with_amplitudes(CompactAmplitudes::Product(state))
    }

    pub(crate) fn from_grouped(state: GroupedState, wires: usize) -> Self {
        Self::from_samples(
            (0..wires).map(|wire| state.probability_one(wire)),
            |shots, seed| state.sample(shots, seed),
            |shots, seed| state.sample_wide(shots, seed),
        )
        .with_amplitudes(CompactAmplitudes::Grouped(state))
    }

    fn with_amplitudes(mut self, amplitudes: CompactAmplitudes) -> Self {
        self.auto_window = match &amplitudes {
            CompactAmplitudes::Stabilizer(view) => view.basis_slice_wide(),
            CompactAmplitudes::Product(state) => state.basis_slice_wide(),
            CompactAmplitudes::Grouped(state) => state.basis_slice_wide(),
        };
        self.compact_amplitudes = Some(amplitudes);
        self.refresh_waveform();
        self
    }

    fn from_samples(
        probabilities: impl Iterator<Item = Option<f32>>,
        packed_samples: impl FnOnce(u32, u64) -> Vec<u128>,
        wide_samples: impl FnOnce(u32, u64) -> Vec<BasisIndex>,
    ) -> Self {
        let wire_one_probabilities: Vec<_> = probabilities
            .map(|p| f64::from(p.expect("validated circuit wire")))
            .collect();
        // Keep individual shots inline when they fit. Only the displayed rows
        // need arbitrary-width formatting after the packed shots are counted.
        let samples = if wire_one_probabilities.len() <= u128::BITS as usize {
            CompactSamples::count(packed_samples(COMPACT_SHOTS, SAMPLE_SEED))
        } else {
            CompactSamples::count(wide_samples(COMPACT_SHOTS, SAMPLE_SEED))
        };
        Self {
            wire_one_probabilities,
            compact_samples: Some(samples),
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
        self.compact_amplitudes = None;
        self.wire_one_probabilities = vec![0.0; quantum_state.num_qubits];
        self.populated_indices.clear();
        let mut first_nonzero = None;
        let mut varying_bits = 0;
        for (index, (amp, &probability)) in quantum_state
            .state_vector
            .iter()
            .zip(probabilities)
            .enumerate()
        {
            if amp.re != 0.0 || amp.im != 0.0 {
                let index = index as u128;
                varying_bits |= *first_nonzero.get_or_insert(index) ^ index;
            }
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
        self.auto_window = (
            first_nonzero.unwrap_or(0).into(),
            1u128
                .checked_shl(varying_bits.trailing_zeros())
                .unwrap_or(1)
                .into(),
        );
        self.refresh_waveform();
    }
}
