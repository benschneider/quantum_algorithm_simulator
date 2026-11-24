use super::{Factor, FactorState, GroupedState};
use crate::BasisIndex;
use num_complex::Complex64;
use rand::{rngs::StdRng, Rng, SeedableRng};

/// Visit the physical wires set in a factor-local basis index.
fn set_wires(wires: &[u32], mut local: usize, mut set: impl FnMut(u32)) {
    while local != 0 {
        let bit = local.trailing_zeros() as usize;
        set(wires[bit]);
        local &= local - 1;
    }
}

impl Factor {
    /// Return the local index of the lowest occupied physical outcome, and
    /// a local mask of the bits varying over the support. SWAP may permute
    /// the physical wires into a different order from the tensor's local bits.
    fn support(&self) -> (usize, usize) {
        let mut occupied = self
            .amplitudes()
            .iter()
            .enumerate()
            .filter_map(|(i, &a)| (a != Complex64::ZERO).then_some(i));
        let mut minimum = occupied.next().expect("normalized factor has support");
        let mut varying = 0;
        for local in occupied {
            let diff = minimum ^ local;
            varying |= diff;
            if minimum != 0 {
                // Physical order is decided by the highest differing wire.
                let high_bit = (0..self.wires.len())
                    .filter(|&bit| diff & (1 << bit) != 0)
                    .max_by_key(|&bit| self.wires[bit])
                    .unwrap();
                if local & (1 << high_bit) == 0 {
                    minimum = local;
                }
            }
            if minimum == 0 && varying == self.amplitudes().len() - 1 {
                break;
            }
        }
        (minimum, varying)
    }
}

impl GroupedState {
    /// Physical wires stored in the same factor as `wire`, in local amplitude-bit order.
    /// A single entry means the simulator represents this wire independently.
    pub fn group_wires(&self, wire: usize) -> Option<&[u32]> {
        (wire < self.num_qubits).then(|| {
            self.factors[self.factor_index(wire as u32)]
                .wires
                .as_slice()
        })
    }

    /// The largest factor width used for this state, including single-wire factors.
    pub fn largest_group_size(&self) -> usize {
        self.factors
            .iter()
            .map(|factor| factor.wires.len())
            .max()
            .unwrap_or(0)
    }

    /// Number of complex amplitude slots retained across all factors.
    pub fn stored_amplitudes(&self) -> usize {
        self.factors
            .iter()
            .map(|f| match &f.state {
                FactorState::Single(_) => 2,
                FactorState::Group(values) => values.capacity(),
            })
            .sum()
    }

    /// Complex amplitude at a global computational-basis index. Bit 0 selects
    /// q0; the higher bits of a u128 index in a wider register are zero.
    pub fn amplitude(&self, index: u128) -> Option<Complex64> {
        if self.num_qubits < u128::BITS as usize && index >> self.num_qubits != 0 {
            return None;
        }
        Some(self.amplitude_from_bits(|q| q < u128::BITS && index & (1u128 << q) != 0))
    }

    /// Computational-basis amplitude at an arbitrary-width index.
    pub fn amplitude_wide(&self, index: &BasisIndex) -> Option<Complex64> {
        (index.bits() <= self.num_qubits as u64)
            .then(|| self.amplitude_from_bits(|q| index.bit(u64::from(q))))
    }

    fn amplitude_from_bits(&self, bit: impl Fn(u32) -> bool) -> Complex64 {
        self.factors
            .iter()
            .map(|factor| {
                let index = factor
                    .wires
                    .iter()
                    .enumerate()
                    .fold(0usize, |local, (i, &q)| local | (usize::from(bit(q)) << i));
                factor.amplitudes()[index]
            })
            .product()
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

    /// First occupied index and stride for registers up to 128 wires.
    /// For wider registers use [`Self::basis_slice_wide`].
    pub fn basis_slice(&self) -> (u128, u128) {
        assert!(
            self.num_qubits <= u128::BITS as usize,
            "use basis_slice_wide for this register"
        );
        let mut first = 0;
        let stride_bit = self.basis_support(|q| first |= 1u128 << q);
        (first, 1u128 << stride_bit)
    }

    /// Smallest occupied physical index and spacing set by the lowest varying
    /// wire. A single occupied index uses spacing one.
    pub fn basis_slice_wide(&self) -> (BasisIndex, BasisIndex) {
        let mut first = BasisIndex::default();
        let stride_bit = self.basis_support(|q| first.set_bit(u64::from(q), true));
        (first, BasisIndex::from(1u8) << stride_bit)
    }

    fn basis_support(&self, mut set_first: impl FnMut(u32)) -> u32 {
        let mut lowest_varying = None;
        for factor in &self.factors {
            let (minimum, varying) = factor.support();
            set_wires(&factor.wires, minimum, &mut set_first);
            set_wires(&factor.wires, varying, |q| {
                lowest_varying = Some(lowest_varying.unwrap_or(q).min(q));
            });
        }
        lowest_varying.unwrap_or(0)
    }

    /// Unit phase of the smallest occupied index, even if its joint amplitude underflows.
    pub fn reference_phase(&self) -> Complex64 {
        let mut phase = Complex64::ONE;
        for factor in &self.factors {
            let a = factor.amplitudes()[factor.support().0];
            let scaled = a / a.re.abs().max(a.im.abs());
            phase *= scaled / scaled.norm();
        }
        phase / phase.norm()
    }

    /// Reproducible joint measurements packed into u128, for registers up to 128 wires.
    /// For wider registers use [`Self::sample_wide`].
    pub fn sample(&self, shots: u32, seed: u64) -> Vec<u128> {
        assert!(
            self.num_qubits <= u128::BITS as usize,
            "use sample_wide for this register"
        );
        self.sample_bits(shots, seed, |bits, q| *bits |= 1u128 << q)
    }

    /// Reproducible joint measurements over all wires, including correlations
    /// within factors. The auxiliary sampling tables are bounded by the shot count.
    pub fn sample_wide(&self, shots: u32, seed: u64) -> Vec<BasisIndex> {
        self.sample_bits(shots, seed, |bits: &mut BasisIndex, q| {
            bits.set_bit(u64::from(q), true)
        })
    }

    fn sample_bits<T: Default + Clone>(
        &self,
        shots: u32,
        seed: u64,
        mut set: impl FnMut(&mut T, u32),
    ) -> Vec<T> {
        let mut samples = vec![T::default(); shots as usize];
        if samples.is_empty() {
            return samples;
        }
        let mut rng = StdRng::seed_from_u64(seed);
        let mut cdf = Vec::new();
        let mut draws = Vec::new();
        for factor in &self.factors {
            let amplitudes = factor.amplitudes();
            if amplitudes.len() <= samples.len() {
                // Binary search a small CDF. Reuse this table for the next factor.
                cdf.clear();
                let mut total = 0.0;
                let mut last = 0;
                cdf.extend(amplitudes.iter().enumerate().map(|(i, a)| {
                    let p = a.norm_sqr();
                    total += p;
                    if p > 0.0 {
                        last = i;
                    }
                    total
                }));
                for bits in &mut samples {
                    let draw = rng.gen::<f64>() * total;
                    let local = cdf[..last].partition_point(|&end| end <= draw);
                    set_wires(&factor.wires, local, |q| set(bits, q));
                }
            } else {
                // Large factor: locate all sorted random draws in one scan of
                // the amplitudes. A full CDF here could double the factor's RAM.
                let total: f64 = amplitudes.iter().map(Complex64::norm_sqr).sum();
                draws.clear();
                draws.extend((0..samples.len()).map(|i| (rng.gen::<f64>() * total, i)));
                draws.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
                let mut next = 0;
                let mut end = 0.0;
                let mut last = 0;
                for (local, a) in amplitudes.iter().enumerate() {
                    let p = a.norm_sqr();
                    if p == 0.0 {
                        continue;
                    }
                    end += p;
                    last = local;
                    while next < draws.len() && draws[next].0 < end {
                        let bits = &mut samples[draws[next].1];
                        set_wires(&factor.wires, local, |q| set(bits, q));
                        next += 1;
                    }
                    if next == draws.len() {
                        break;
                    }
                }
                // A draw rounded to the upper endpoint takes the last nonzero outcome.
                for &(_, i) in &draws[next..] {
                    set_wires(&factor.wires, last, |q| set(&mut samples[i], q));
                }
            }
        }
        samples
    }
}
