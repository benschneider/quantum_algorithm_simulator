use rand::{rngs::StdRng, Rng, SeedableRng};

/// Represents i^phase X^x Z^z, with all X factors ordered before all Z factors.
/// In particular Y = iXZ. This convention keeps the phase of a row product explicit.
#[derive(Clone, Copy, Debug, Default)]
struct Pauli {
    x: u64,
    z: u64,
    phase: u8,
}

impl Pauli {
    fn multiply(&mut self, rhs: Self) {
        // Moving rhs's X factors past our Z factors gives one minus sign per overlap.
        let odd_swaps = (self.z & rhs.x).count_ones() & 1;
        self.phase = (self.phase + rhs.phase + 2 * odd_swaps as u8) & 3;
        self.x ^= rhs.x;
        self.z ^= rhs.z;
    }
}

/// A pure stabilizer state on at most 64 wires. Each row stores two packed Pauli
/// masks. The first half are destabilizers paired with the stabilizers in the
/// second half, as in the Aaronson–Gottesman tableau.
#[derive(Clone, Debug)]
pub struct StabilizerState {
    rows: Vec<Pauli>,
}

impl StabilizerState {
    pub(super) fn zero(num_qubits: usize) -> Self {
        debug_assert!((1..=64).contains(&num_qubits));
        let mut rows = Vec::with_capacity(2 * num_qubits);
        for wire in 0..num_qubits {
            rows.push(Pauli {
                x: 1 << wire,
                ..Pauli::default()
            });
        }
        for wire in 0..num_qubits {
            rows.push(Pauli {
                z: 1 << wire,
                ..Pauli::default()
            });
        }
        Self { rows }
    }

    pub(super) fn h(&mut self, wire: usize) {
        let mask = 1 << wire;
        for row in &mut self.rows {
            if row.x & row.z & mask != 0 {
                row.phase ^= 2; // H exchanges X and Z, reversing XZ on this wire.
            }
            let swap = (row.x ^ row.z) & mask;
            row.x ^= swap;
            row.z ^= swap;
        }
    }

    pub(super) fn s(&mut self, wire: usize) {
        let mask = 1 << wire;
        for row in &mut self.rows {
            if row.x & mask != 0 {
                row.phase = (row.phase + 1) & 3; // S maps X to iXZ.
                row.z ^= mask;
            }
        }
    }

    pub(super) fn x(&mut self, wire: usize) {
        self.conjugate_pauli(1 << wire, 0);
    }

    pub(super) fn y(&mut self, wire: usize) {
        self.conjugate_pauli(1 << wire, 1 << wire);
    }

    pub(super) fn z(&mut self, wire: usize) {
        self.conjugate_pauli(0, 1 << wire);
    }

    fn conjugate_pauli(&mut self, x: u64, z: u64) {
        for row in &mut self.rows {
            if ((row.x & z) ^ (row.z & x)).count_ones() & 1 != 0 {
                row.phase ^= 2;
            }
        }
    }

    pub(super) fn cx(&mut self, control: usize, target: usize) {
        let control = 1 << control;
        let target = 1 << target;
        for row in &mut self.rows {
            if row.x & control != 0 {
                row.x ^= target;
            }
            if row.z & target != 0 {
                row.z ^= control;
            }
        }
    }

    /// Exact probability of reading 1 on a wire, or `None` for an invalid wire.
    /// Inspection leaves the state intact.
    pub fn probability_one(&self, wire: usize) -> Option<f32> {
        if wire >= self.rows.len() / 2 {
            return None;
        }
        let mask = 1 << wire;
        Some(if self.pivot(mask).is_some() {
            0.5
        } else {
            u8::from(self.deterministic_one(mask)) as f32
        })
    }

    // A stabilizer with X on the measured wire anticommutes with Z, making its result random.
    fn pivot(&self, mask: u64) -> Option<usize> {
        self.rows[self.rows.len() / 2..]
            .iter()
            .position(|row| row.x & mask != 0)
    }

    fn deterministic_one(&self, mask: u64) -> bool {
        // With no anticommuting stabilizer, ±Z belongs to the group. The paired
        // destabilizers tell us which stabilizers multiply to it.
        let (destabilizers, stabilizers) = self.rows.split_at(self.rows.len() / 2);
        let mut product = Pauli::default();
        for (destabilizer, stabilizer) in destabilizers.iter().zip(stabilizers) {
            if destabilizer.x & mask != 0 {
                product.multiply(*stabilizer);
            }
        }
        debug_assert_eq!((product.x, product.z), (0, mask));
        debug_assert!(product.phase == 0 || product.phase == 2);
        product.phase == 2
    }

    fn measure(&mut self, mask: u64, rng: &mut StdRng) -> bool {
        let Some(pair) = self.pivot(mask) else {
            return self.deterministic_one(mask);
        };
        let pivot = self.rows.len() / 2 + pair;
        let old = self.rows[pivot];
        for (index, row) in self.rows.iter_mut().enumerate() {
            if index != pair && index != pivot && row.x & mask != 0 {
                row.multiply(old);
            }
        }
        // All other pairs now commute with Z. The old pivot becomes the
        // destabilizer paired with the measured ±Z in the new state.
        let one = rng.gen();
        self.rows[pair] = old;
        self.rows[pivot] = Pauli {
            x: 0,
            z: mask,
            phase: 2 * u8::from(one),
        };
        one
    }

    /// Draws reproducible joint computational-basis samples. Bit 0 is wire q0.
    /// Measurements within a shot share a scratch tableau to preserve correlations.
    pub fn sample(&self, shots: u32, seed: u64) -> Vec<u64> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut scratch = self.clone();
        let mut samples = Vec::with_capacity(shots as usize);
        for _ in 0..shots {
            scratch.rows.copy_from_slice(&self.rows);
            let mut outcome = 0;
            for wire in 0..self.rows.len() / 2 {
                let mask = 1 << wire;
                if scratch.measure(mask, &mut rng) {
                    outcome |= mask;
                }
            }
            samples.push(outcome);
        }
        samples
    }
}
