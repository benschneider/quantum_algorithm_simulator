use super::{Factor, GroupedState};
use crate::MAX_COMPACT_QUBITS;
use num_complex::Complex64;
use rand::{rngs::StdRng, Rng, SeedableRng};

// A SWAP can leave local amplitude bits in a different order from physical wires.
fn local_index(wires: &[u32], physical: u128) -> usize {
    wires.iter().enumerate().fold(0, |index, (bit, wire)| {
        index | (((physical >> wire) & 1) as usize) << bit
    })
}

fn physical_index(wires: &[u32], local: usize) -> u128 {
    wires.iter().enumerate().fold(0, |index, (bit, wire)| {
        index | (((local >> bit) & 1) as u128) << wire
    })
}

impl Factor {
    fn occupied(&self) -> impl Iterator<Item = (u128, Complex64)> + '_ {
        self.amplitudes()
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, a)| *a != Complex64::ZERO)
            .map(|(local, a)| (physical_index(&self.wires, local), a))
    }
}

impl GroupedState {
    /// The largest factor width used for this state, including single-wire factors.
    pub fn largest_group_size(&self) -> usize {
        self.factors
            .iter()
            .map(|factor| factor.wires.len())
            .max()
            .unwrap_or(0)
    }

    /// Complex amplitude at a global computational-basis index (q0 is the low bit).
    pub fn amplitude(&self, index: u128) -> Option<Complex64> {
        if self.num_qubits < MAX_COMPACT_QUBITS && index >> self.num_qubits != 0 {
            return None;
        }
        Some(
            self.factors
                .iter()
                .map(|factor| factor.amplitudes()[local_index(&factor.wires, index)])
                .product(),
        )
    }

    /// Probability of measuring one on a single physical wire.
    pub fn probability_one(&self, wire: usize) -> Option<f32> {
        if wire >= self.num_qubits {
            return None;
        }
        self.factors.iter().find_map(|factor| {
            let bit = factor.wires.iter().position(|&q| q as usize == wire)?;
            let mut one = 0.0;
            let mut total = 0.0;
            for (index, a) in factor.amplitudes().iter().enumerate() {
                let p = a.norm_sqr();
                total += p;
                if index & (1 << bit) != 0 {
                    one += p;
                }
            }
            Some((one / total) as f32)
        })
    }

    /// Smallest occupied physical index and spacing set by the lowest varying wire.
    /// A state with a single occupied index uses spacing one.
    pub fn basis_slice(&self) -> (u128, u128) {
        let mut first = 0;
        let mut varying = 0_u128;
        for factor in &self.factors {
            let mut occupied = factor.occupied();
            let (anchor, _) = occupied.next().expect("normalized factor has support");
            let mut minimum = anchor;
            for (bits, _) in occupied {
                minimum = minimum.min(bits);
                varying |= anchor ^ bits;
            }
            first |= minimum;
        }
        (
            first,
            1_u128.checked_shl(varying.trailing_zeros()).unwrap_or(1),
        )
    }

    /// Unit phase of the smallest occupied index, even if its joint amplitude underflows.
    pub fn reference_phase(&self) -> Complex64 {
        let mut phase = Complex64::ONE;
        for factor in &self.factors {
            let (_, a) = factor
                .occupied()
                .min_by_key(|&(bits, _)| bits)
                .expect("normalized factor has support");
            let scaled = a / a.re.abs().max(a.im.abs());
            phase *= scaled / scaled.norm();
        }
        phase / phase.norm()
    }

    /// Reproducible joint measurements. Each shot draws one complete outcome per factor,
    /// keeping entangled wire correlations intact.
    pub fn sample(&self, shots: u32, seed: u64) -> Vec<u128> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut samples = vec![0; shots as usize];
        for factor in &self.factors {
            let mut cumulative = Vec::with_capacity(factor.amplitudes().len());
            let mut total = 0.0;
            for (bits, a) in factor.occupied() {
                let weight = a.norm_sqr();
                if weight > 0.0 {
                    total += weight;
                    cumulative.push((total, bits));
                }
            }
            // Searching only the prefix lets the last outcome absorb roundoff at the endpoint.
            let prefix = &cumulative[..cumulative.len() - 1];
            for bits in &mut samples {
                let draw = rng.gen::<f64>() * total;
                let choice = prefix.partition_point(|&(end, _)| end <= draw);
                *bits |= cumulative[choice].1;
            }
        }
        samples
    }
}
