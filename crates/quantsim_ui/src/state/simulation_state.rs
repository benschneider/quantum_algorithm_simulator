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
const PROBABILITY_PREFIX_BITS: usize = 5;
pub(crate) const MAX_PROBABILITY_BINS: usize = 1 << PROBABILITY_PREFIX_BITS;

#[derive(Debug)]
pub(crate) struct CompactSamples {
    /// Most frequent sampled bitstrings, with q0 in the least significant bit.
    pub(crate) rows: Vec<(BasisIndex, u32)>,
    pub(crate) distinct: usize,
}

impl CompactSamples {
    fn count<T: Ord + Into<BasisIndex>>(
        mut outcomes: Vec<T>,
        bins: &mut [f64; MAX_PROBABILITY_BINS],
        bin_index: impl Fn(&T) -> usize,
    ) -> Self {
        let shots = outcomes.len() as f64;
        outcomes.sort_unstable();
        let mut counts: Vec<(T, u32)> = Vec::new();
        for outcome in outcomes {
            if let Some((_, count)) = counts.last_mut().filter(|(bits, _)| *bits == outcome) {
                *count += 1;
            } else {
                counts.push((outcome, 1));
            }
        }
        // Accumulate every observed outcome before the table drops its less common rows.
        for (outcome, count) in &counts {
            bins[bin_index(outcome)] += f64::from(*count) / shots;
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
    /// Mass of contiguous basis ranges, indexed by the most significant bits.
    probability_bins: [f64; MAX_PROBABILITY_BINS],
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
            probability_bins: [0.0; MAX_PROBABILITY_BINS],
            populated_indices: Vec::new(),
            snapshot_states: Default::default(),
            pagination: PaginationState::new(0, 20), // sensible defaults for pagination
            page_input_text: String::new(),
        }
    }
}

impl SimulationState {
    pub(crate) fn probability_bins(&self) -> &[f64] {
        if !self.has_readout() {
            return &[];
        }
        let prefix_bits = self
            .wire_one_probabilities
            .len()
            .min(PROBABILITY_PREFIX_BITS);
        &self.probability_bins[..1 << prefix_bits]
    }

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
        let wires = wire_one_probabilities.len();
        let prefix_start = wires.saturating_sub(PROBABILITY_PREFIX_BITS);
        let mut probability_bins = [0.0; MAX_PROBABILITY_BINS];
        // Keep individual shots inline when they fit. Only the displayed rows
        // need arbitrary-width formatting after the packed shots are counted.
        let samples = if wires <= u128::BITS as usize {
            CompactSamples::count(
                packed_samples(COMPACT_SHOTS, SAMPLE_SEED),
                &mut probability_bins,
                |bits| (*bits >> prefix_start) as usize,
            )
        } else {
            CompactSamples::count(
                wide_samples(COMPACT_SHOTS, SAMPLE_SEED),
                &mut probability_bins,
                |bits| {
                    // Read only the prefix; avoid cloning or formatting the full outcome.
                    (prefix_start..wires).rev().fold(0, |prefix, wire| {
                        (prefix << 1) | usize::from(bits.bit(wire as u64))
                    })
                },
            )
        };
        Self {
            wire_one_probabilities,
            probability_bins,
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
        self.probability_bins.fill(0.0);
        let prefix_start = quantum_state
            .num_qubits
            .saturating_sub(PROBABILITY_PREFIX_BITS);
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
                self.probability_bins[index >> prefix_start] += f64::from(probability);
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

#[cfg(test)]
mod probability_overview_tests {
    use super::*;

    #[test]
    fn dense_ranges_include_small_amplitudes_and_reset_for_short_registers() {
        let mut dense = QuantumState::new(6);
        for index in [0, 1, 2, 63] {
            dense.state_vector[index].re = 0.5;
        }
        dense.state_vector[9].re = 5e-7;
        let probabilities = dense.probabilities();
        let mut readout = SimulationState::default();
        assert!(readout.probability_bins().is_empty());
        readout.set_readout(dense, &probabilities, true);
        let bins = readout.probability_bins();
        assert_eq!(bins.len(), 32);
        assert_eq!((bins[0], bins[1], bins[31]), (0.5, 0.25, 0.25));
        assert!(!readout.populated_indices.contains(&9));
        assert_eq!(bins[4], f64::from(probabilities[9]));
        assert!(bins[4] > 0.0);

        for wires in [2, 0] {
            let dense = QuantumState::new(wires);
            let probabilities = dense.probabilities();
            readout.set_readout(dense, &probabilities, true);
            let bins = readout.probability_bins();
            assert_eq!(bins.len(), 1 << wires);
            assert_eq!(bins[0], 1.0);
            assert!(bins[1..].iter().all(|&p| p == 0.0));
        }
    }

    #[test]
    fn sampled_ranges_preserve_wide_prefixes_and_all_shots_before_truncation() {
        for wires in [128, 257] {
            let shift = wires - PROBABILITY_PREFIX_BITS;
            let prefix = |shot: u32| {
                if shot < COMPACT_SHOTS / 2 {
                    22
                } else {
                    shot % 32
                }
            };
            let readout = SimulationState::from_samples(
                (0..wires).map(|_| Some(0.0)),
                |shots, _| {
                    (0..shots)
                        .map(|s| (u128::from(prefix(s)) << shift) | 3)
                        .collect()
                },
                |shots, _| {
                    (0..shots)
                        .map(|s| (BasisIndex::from(prefix(s)) << shift) + 3u8)
                        .collect()
                },
            );
            let samples = readout.compact_samples.as_ref().unwrap();
            assert_eq!(samples.distinct, 32);
            assert_eq!(samples.rows.len(), MAX_SAMPLE_ROWS);
            let mut expected = [1.0 / 64.0; 32];
            expected[22] += 0.5;
            assert_eq!(readout.probability_bins(), expected);
        }
    }
}
