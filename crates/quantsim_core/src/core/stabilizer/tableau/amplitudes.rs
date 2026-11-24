use super::{Pauli, StabilizerState};
use num_complex::Complex64;

/// A read-only index of the amplitudes of a pure stabilizer state. The tableau
/// encodes the state up to global phase, so the amplitude of the lowest
/// supported basis index is chosen to be positive real. Every other amplitude
/// has its physical phase relative to this reference.
#[derive(Debug, Clone)]
pub struct StabilizerAmplitudes {
    wires: usize,
    reference: u64,
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
        let rank = echelon(&mut rows, (0..wires).rev(), |row| row.x);
        let constraints = &mut rows[rank..];
        echelon(constraints, 0..wires, |row| row.z);

        // For a diagonal row, (-1)^(z·b) must match its sign. Each equation
        // determines its lowest bit from higher bits. Back-substitution with
        // free bits set to zero gives the smallest supported basis index.
        let mut reference = 0u64;
        for row in constraints.iter().rev() {
            debug_assert!(row.x == 0 && row.z != 0 && row.phase % 2 == 0);
            let parity = (reference & row.z).count_ones() & 1;
            if parity != u32::from(row.phase / 2) {
                reference |= 1 << row.z.trailing_zeros();
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
    component: impl Fn(&Pauli) -> u64,
) -> usize {
    let mut rank = 0;
    for wire in wires {
        let mask = 1u64 << wire;
        let Some(offset) = rows[rank..]
            .iter()
            .position(|row| component(row) & mask != 0)
        else {
            continue;
        };
        rows.swap(rank, rank + offset);
        let pivot = rows[rank];
        rank += 1;
        for row in &mut rows[rank..] {
            if component(row) & mask != 0 {
                row.multiply(pivot);
            }
        }
    }
    rank
}

impl StabilizerAmplitudes {
    /// The smallest computational-basis index with a nonzero amplitude.
    pub fn reference_index(&self) -> u64 {
        self.reference
    }

    /// Origin and power-of-two spacing for a basis-index plot. Bits below the
    /// stride are fixed throughout the support. A singleton uses stride one.
    pub fn basis_slice(&self) -> (u64, u64) {
        let bit = self
            .translations
            .iter()
            .map(|row| row.x.trailing_zeros())
            .min();
        (self.reference, bit.map_or(1, |bit| 1u64 << bit))
    }

    /// Amplitude at a computational-basis index, or `None` outside the
    /// register. A valid outcome outside the state's support returns zero.
    /// Bit zero is q0. Each query visits at most one packed row per wire.
    pub fn amplitude(&self, index: u64) -> Option<Complex64> {
        if self.wires < 64 && index >> self.wires != 0 {
            return None;
        }
        let mut from = self.reference;
        let mut phase = 0u8;
        for row in &self.translations {
            let pivot = 1u64 << row.x.ilog2();
            if (from ^ index) & pivot != 0 {
                // i^p X^x Z^z maps |b> to i^p (-1)^(z·b) |b xor x>.
                let odd = ((row.z & from).count_ones() & 1) as u8;
                phase = (phase + row.phase + 2 * odd) & 3;
                from ^= row.x;
            }
        }
        if from != index {
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
}
