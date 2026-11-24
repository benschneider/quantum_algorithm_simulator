use crate::BasisIndex;

mod amplitudes;
pub use amplitudes::StabilizerAmplitudes;

/// Represents i^phase X^x Z^z, with all X factors ordered before all Z factors.
/// In particular Y = iXZ. This convention keeps the phase of a row product explicit.
#[derive(Clone, Debug, Default)]
struct Pauli {
    x: BasisIndex,
    z: BasisIndex,
    phase: u8,
}

fn parity(a: &BasisIndex, b: &BasisIndex) -> u8 {
    let overlap = a
        .iter_u64_digits()
        .zip(b.iter_u64_digits())
        .fold(0_u64, |acc, (x, z)| acc ^ (x & z));
    (overlap.count_ones() & 1) as u8
}

impl Pauli {
    fn multiply(&mut self, rhs: &Self) {
        // Moving rhs's X factors past our Z factors gives one minus sign per overlap.
        self.phase = (self.phase + rhs.phase + 2 * parity(&self.z, &rhs.x)) & 3;
        self.x ^= &rhs.x;
        self.z ^= &rhs.z;
    }
}

// On 32-bit targets BigUint takes ownership of base-2^32 digits. Reserve the
// identity masks fallibly so wasm can report failed allocations during setup.
fn identity_mask(wire: usize) -> Result<BasisIndex, String> {
    #[cfg(target_pointer_width = "32")]
    {
        let word = wire / 32;
        let mut digits = Vec::new();
        digits
            .try_reserve_exact(word + 1)
            .map_err(|e| format!("Cannot allocate stabilizer mask for q{wire}: {e}."))?;
        digits.resize(word, 0);
        digits.push(1u32 << (wire % 32));
        Ok(BasisIndex::new(digits))
    }
    #[cfg(not(target_pointer_width = "32"))]
    {
        Ok(BasisIndex::from(1u8) << wire)
    }
}

/// Each row stores two variable-width packed Pauli masks. The first half are
/// destabilizers paired with the stabilizers in the second half, as in the
/// Aaronson–Gottesman tableau.
#[derive(Clone, Debug)]
pub struct StabilizerState {
    rows: Vec<Pauli>,
}

impl StabilizerState {
    pub(super) fn zero(num_qubits: usize) -> Result<Self, String> {
        debug_assert!(num_qubits > 0);
        let count = num_qubits
            .checked_mul(2)
            .ok_or("Tableau row count overflow.")?;
        // The initial X and Z identities alone need two copies of masks of
        // bit lengths 1..=n, even before digit alignment and allocator overhead.
        let n = num_qubits as u128;
        let minimum_bytes =
            (n * (n + 1)).div_ceil(8) + count as u128 * std::mem::size_of::<Pauli>() as u128;
        if minimum_bytes > usize::MAX as u128 {
            return Err(
                "The initial stabilizer tableau exceeds this platform's address space.".into(),
            );
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(count).map_err(|e| {
            format!("Cannot allocate a {num_qubits}-qubit stabilizer tableau: {e}.")
        })?;
        for wire in 0..num_qubits {
            rows.push(Pauli {
                x: identity_mask(wire)?,
                ..Pauli::default()
            });
        }
        for wire in 0..num_qubits {
            rows.push(Pauli {
                z: identity_mask(wire)?,
                ..Pauli::default()
            });
        }
        Ok(Self { rows })
    }

    pub(super) fn h(&mut self, wire: usize) {
        let wire = wire as u64;
        for row in &mut self.rows {
            let x = row.x.bit(wire);
            let z = row.z.bit(wire);
            if x && z {
                row.phase ^= 2; // H exchanges X and Z, reversing XZ on this wire.
            }
            if x != z {
                row.x.set_bit(wire, z);
                row.z.set_bit(wire, x);
            }
        }
    }

    pub(super) fn s(&mut self, wire: usize) {
        let wire = wire as u64;
        for row in &mut self.rows {
            if row.x.bit(wire) {
                row.phase = (row.phase + 1) & 3; // S maps X to iXZ.
                row.z.set_bit(wire, !row.z.bit(wire));
            }
        }
    }

    pub(super) fn x(&mut self, wire: usize) {
        self.conjugate_pauli(wire, true, false);
    }

    pub(super) fn y(&mut self, wire: usize) {
        self.conjugate_pauli(wire, true, true);
    }

    pub(super) fn z(&mut self, wire: usize) {
        self.conjugate_pauli(wire, false, true);
    }

    fn conjugate_pauli(&mut self, wire: usize, x: bool, z: bool) {
        for row in &mut self.rows {
            if (z && row.x.bit(wire as u64)) ^ (x && row.z.bit(wire as u64)) {
                row.phase ^= 2;
            }
        }
    }

    pub(super) fn cx(&mut self, control: usize, target: usize) {
        let (control, target) = (control as u64, target as u64);
        for row in &mut self.rows {
            if row.x.bit(control) {
                row.x.set_bit(target, !row.x.bit(target));
            }
            if row.z.bit(target) {
                row.z.set_bit(control, !row.z.bit(control));
            }
        }
    }

    /// Exact probability of reading 1 on a wire, or `None` for an invalid wire.
    /// Inspection leaves the state intact.
    pub fn probability_one(&self, wire: usize) -> Option<f32> {
        if wire >= self.rows.len() / 2 {
            return None;
        }
        let wire = wire as u64;
        let (destabilizers, stabilizers) = self.rows.split_at(self.rows.len() / 2);
        if stabilizers.iter().any(|row| row.x.bit(wire)) {
            // A stabilizer anticommutes with Z on this wire.
            return Some(0.5);
        }
        // Otherwise ±Z belongs to the group. Its sign comes from the product
        // of stabilizers whose paired destabilizers anticommute with Z.
        let mut product = Pauli::default();
        for (destabilizer, stabilizer) in destabilizers.iter().zip(stabilizers) {
            if destabilizer.x.bit(wire) {
                product.multiply(stabilizer);
            }
        }
        debug_assert!(product.x.bits() == 0 && product.z.bit(wire) && product.z.count_ones() == 1);
        debug_assert!(product.phase == 0 || product.phase == 2);
        Some(f32::from(product.phase / 2))
    }

    /// Draw reproducible joint computational-basis samples. Bit 0 is wire q0.
    ///
    /// # Panics
    /// For registers wider than 128 qubits, use [`Self::sample_wide`].
    pub fn sample(&self, shots: u32, seed: u64) -> Vec<u128> {
        assert!(
            self.rows.len() / 2 <= u128::BITS as usize,
            "use sample_wide for registers wider than 128 wires"
        );
        self.amplitude_view()
            .sample_indices(seed)
            .take(shots as usize)
            .map(|bits| u128::try_from(bits).expect("wire count fits in u128"))
            .collect()
    }

    /// Draw reproducible joint samples with every outcome bit preserved.
    pub fn sample_wide(&self, shots: u32, seed: u64) -> Vec<BasisIndex> {
        self.amplitude_view().sample_wide(shots, seed)
    }
}
