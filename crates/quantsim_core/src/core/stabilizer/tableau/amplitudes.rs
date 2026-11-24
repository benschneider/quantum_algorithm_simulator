use super::{parity, Pauli, StabilizerState};
use crate::BasisIndex;
use num_complex::Complex64;
use num_traits::{One, Zero};
use rand::{rngs::StdRng, Rng, SeedableRng};

/// A read-only index of the amplitudes of a pure stabilizer state. The tableau
/// encodes the state up to global phase, so the amplitude of the lowest
/// supported basis index is chosen to be positive real. Every other amplitude
/// has its physical phase relative to this reference.
#[derive(Debug, Clone)]
pub struct StabilizerAmplitudes {
    wires: usize,
    reference: BasisIndex,
    magnitude: f64,
    translations: Vec<Pauli>,
}

impl StabilizerState {
    /// Indexes the tableau once for repeated amplitude queries. Construction
    /// uses O(n²) packed row operations and O(n) rows of storage for n wires.
    pub fn amplitude_view(&self) -> StabilizerAmplitudes {
        let wires = self.rows.len() / 2;
        let mut rows = self.rows[wires..].to_vec();
        // Each independent X mask translates between two supported outcomes.
        // Eliminating X leaves diagonal stabilizers that constrain the support.
        let rank = echelon(&mut rows, (0..wires).rev(), |row| &row.x);
        let constraints = &mut rows[rank..];
        echelon(constraints, 0..wires, |row| &row.z);

        // For a diagonal row, (-1)^(z·b) must match its sign. Each equation
        // determines its lowest bit from higher bits. Back-substitution with
        // free bits set to zero gives the smallest supported basis index.
        let mut reference = BasisIndex::zero();
        for row in constraints.iter().rev() {
            debug_assert!(row.x.is_zero() && !row.z.is_zero() && row.phase % 2 == 0);
            if parity(&reference, &row.z) != row.phase / 2 {
                reference.set_bit(
                    row.z.trailing_zeros().expect("independent constraint"),
                    true,
                );
            }
        }
        rows.truncate(rank);
        StabilizerAmplitudes {
            wires,
            reference,
            magnitude: (-0.5 * rank as f64).exp2(),
            translations: rows,
        }
    }
}

// Eliminate below each pivot by multiplying commuting stabilizers. Keeping
// full Pauli rows here preserves all signs and phases under the row operations.
fn echelon(
    rows: &mut [Pauli],
    wires: impl Iterator<Item = usize>,
    component: impl Fn(&Pauli) -> &BasisIndex,
) -> usize {
    let mut rank = 0;
    for wire in wires {
        if rank == rows.len() {
            break;
        }
        let wire = wire as u64;
        let Some(offset) = rows[rank..].iter().position(|row| component(row).bit(wire)) else {
            continue;
        };
        rows.swap(rank, rank + offset);
        let (pivot, rest) = rows[rank..].split_first_mut().unwrap();
        for row in rest {
            if component(row).bit(wire) {
                row.multiply(pivot);
            }
        }
        rank += 1;
    }
    rank
}

impl StabilizerAmplitudes {
    /// The smallest supported index.
    ///
    /// # Panics
    /// If the index exceeds u128. [`Self::reference_index_wide`] supports the full register.
    pub fn reference_index(&self) -> u128 {
        u128::try_from(&self.reference).expect("reference exceeds u128; use reference_index_wide")
    }

    /// The smallest computational-basis index with a nonzero amplitude.
    pub fn reference_index_wide(&self) -> BasisIndex {
        self.reference.clone()
    }

    /// Origin and power-of-two spacing for a plot.
    ///
    /// # Panics
    /// If either value exceeds u128. [`Self::basis_slice_wide`] supports the full register.
    pub fn basis_slice(&self) -> (u128, u128) {
        let (first, stride) = self.basis_slice_wide();
        (
            u128::try_from(first).expect("origin exceeds u128; use basis_slice_wide"),
            u128::try_from(stride).expect("stride exceeds u128; use basis_slice_wide"),
        )
    }

    /// Bits below the stride are fixed throughout the support. A singleton uses stride one.
    pub fn basis_slice_wide(&self) -> (BasisIndex, BasisIndex) {
        let low = self
            .translations
            .iter()
            .filter_map(|row| row.x.trailing_zeros())
            .min();
        (
            self.reference.clone(),
            BasisIndex::one() << low.unwrap_or(0),
        )
    }

    /// Amplitude at a u128 address, including low addresses on wider registers.
    pub fn amplitude(&self, index: u128) -> Option<Complex64> {
        self.amplitude_wide(&BasisIndex::from(index))
    }

    /// Amplitude at a computational-basis index, or `None` outside the register.
    /// A valid outcome outside the state's support returns zero. Bit zero is q0.
    pub fn amplitude_wide(&self, index: &BasisIndex) -> Option<Complex64> {
        if index.bits() > self.wires as u64 {
            return None;
        }
        let mut from = self.reference.clone();
        let mut phase = 0u8;
        for row in &self.translations {
            let pivot = row.x.bits() - 1;
            if from.bit(pivot) != index.bit(pivot) {
                // i^p X^x Z^z maps |b> to i^p (-1)^(z·b) |b xor x>.
                phase = (phase + row.phase + 2 * parity(&row.z, &from)) & 3;
                from ^= &row.x;
            }
        }
        if &from != index {
            return Some(Complex64::ZERO);
        }
        let m = self.magnitude;
        Some(match phase {
            0 => Complex64::new(m, 0.0),
            1 => Complex64::new(0.0, m),
            2 => Complex64::new(-m, 0.0),
            _ => Complex64::new(0.0, -m),
        })
    }

    /// Each translation pairs supported outcomes of equal probability. A wire
    /// varied by any translation is therefore unbiased; every other wire is fixed.
    pub fn probability_one(&self, wire: usize) -> Option<f32> {
        if wire >= self.wires {
            return None;
        }
        let wire = wire as u64;
        Some(if self.translations.iter().any(|row| row.x.bit(wire)) {
            0.5
        } else {
            f32::from(u8::from(self.reference.bit(wire)))
        })
    }

    /// Sample the support of this indexed state, preserving all wires in each shot.
    pub fn sample_wide(&self, shots: u32, seed: u64) -> Vec<BasisIndex> {
        self.sample_indices(seed).take(shots as usize).collect()
    }

    /// Low-to-high pivots are precisely the wires whose conditional readout is
    /// random. Draw these physical bits in order, as for successive Z measurements.
    pub(super) fn sample_indices(&self, seed: u64) -> impl Iterator<Item = BasisIndex> + '_ {
        let mut rows = self.translations.clone();
        echelon(&mut rows, 0..self.wires, |row| &row.x);
        let pivots: Vec<_> = rows
            .into_iter()
            .map(|row| {
                (
                    row.x.trailing_zeros().expect("independent translation"),
                    row.x,
                )
            })
            .collect();
        let mut rng = StdRng::seed_from_u64(seed);
        std::iter::repeat_with(move || {
            let mut outcome = self.reference.clone();
            for (wire, translation) in &pivots {
                if outcome.bit(*wire) != rng.gen::<bool>() {
                    outcome ^= translation;
                }
            }
            outcome
        })
    }
}
