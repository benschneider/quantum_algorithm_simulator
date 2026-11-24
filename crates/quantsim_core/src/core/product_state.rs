//! Computational-basis readout of separable circuit prefixes on 1–128 wires.
use super::circuit::{Circuit, ValidationError};
use super::compact::{gate_parameter, MAX_COMPACT_QUBITS};
use super::gates::registry::GateRegistry;
use super::types::{Gate, GateMatrix, Operation, QubitPrep};
use num_complex::Complex64;
use rand::{rngs::StdRng, Rng, SeedableRng};

#[derive(Debug, Clone, Copy)]
pub(super) struct Wire {
    pub(super) amplitudes: [Complex64; 2],
    // Only use amplitude identities to admit a controlled gate when the wire
    // is known by construction to be a Pauli eigenstate. A general rotation
    // can leave a tiny component that floating-point arithmetic rounds away.
    pauli_eigenstate: bool,
}

impl Wire {
    pub(super) fn from_factor(amplitudes: [Complex64; 2]) -> Self {
        // A numerical factorization supplies amplitudes, not a gate-history
        // certificate for simplifying later controlled operations.
        Self {
            amplitudes,
            pauli_eigenstate: false,
        }
    }

    pub(super) fn new(prep: QubitPrep) -> Self {
        let half = std::f64::consts::FRAC_1_SQRT_2;
        let pair = match prep {
            QubitPrep::Zero => [1.0, 0.0],
            QubitPrep::One => [0.0, 1.0],
            QubitPrep::Plus => [half, half],
            QubitPrep::Minus => [half, -half],
        };
        Self {
            amplitudes: pair.map(Complex64::from),
            pauli_eigenstate: true,
        }
    }

    pub(super) fn basis_bit(&self) -> Option<bool> {
        if !self.pauli_eigenstate {
            return None;
        }
        let [a, b] = self.amplitudes;
        if b == Complex64::ZERO {
            Some(false)
        } else if a == Complex64::ZERO {
            Some(true)
        } else {
            None
        }
    }

    // Returns the sign bit of the eigenvalue of X, when known.
    pub(super) fn x_minus(&self) -> Option<bool> {
        if !self.pauli_eigenstate {
            return None;
        }
        let [a, b] = self.amplitudes;
        if a == b {
            Some(false)
        } else if a == -b {
            Some(true)
        } else {
            None
        }
    }

    pub(super) fn z_phase(&mut self, exponent: f64) {
        self.multiply_one_phase(z_eigenvalue(exponent), exponent % 0.5 == 0.0);
    }

    pub(super) fn multiply_one_phase(&mut self, phase: Complex64, is_clifford: bool) {
        if !is_clifford {
            // A definite basis bit stays definite even when |1> gains phase.
            self.pauli_eigenstate = self.basis_bit().is_some();
        }
        self.amplitudes[1] *= phase;
    }

    pub(super) fn apply_single(
        &mut self,
        op: &Operation,
        registry: &GateRegistry,
        param: f64,
    ) -> Result<(), String> {
        if op.id == Gate::ZPow {
            self.z_phase(param);
            return Ok(());
        }
        let [a, b] = self.amplitudes;
        // Rotation about the wire's own axis preserves its eigenstate certificate.
        // Its eigenvalue still contributes phase to the joint amplitude.
        let same_axis = match op.id {
            Gate::Rx | Gate::XPow => self.x_minus().is_some(),
            Gate::Ry | Gate::YPow => {
                self.pauli_eigenstate && (b == Complex64::I * a || b == -Complex64::I * a)
            }
            Gate::Rz => self.basis_bit().is_some(),
            _ => false,
        };
        let zero_rotation = param == 0.0
            && matches!(
                op.id,
                Gate::Rx | Gate::Ry | Gate::Rz | Gate::XPow | Gate::YPow
            );
        if zero_rotation {
            return Ok(());
        }
        self.pauli_eigenstate &= same_axis
            || matches!(
                op.id,
                Gate::H | Gate::X | Gate::Y | Gate::Z | Gate::SqrtX | Gate::SqrtY | Gate::SqrtZ
            );
        // All built-in one-wire gates are unitary and are registered in big-endian form.
        let Some(GateMatrix::BigEndian(m)) = registry.eval(&op.id, &op.params, &op.qubits) else {
            return Err(format!("Could not evaluate single-qubit gate {}.", op.id));
        };
        let element = |r, c| {
            let z: num_complex::Complex32 = m[(r, c)];
            Complex64::new(z.re as f64, z.im as f64)
        };
        let next = [
            element(0, 0) * a + element(0, 1) * b,
            element(1, 0) * a + element(1, 1) * b,
        ];
        let norm = (next[0].norm_sqr() + next[1].norm_sqr()).sqrt();
        if !norm.is_finite() || norm == 0.0 {
            return Err(format!("Gate {} produced an invalid amplitude.", op.id));
        }
        self.amplitudes = next.map(|z| z / norm);
        Ok(())
    }

    fn probability_one(&self) -> f64 {
        let [a, b] = self.amplitudes;
        b.norm_sqr() / (a.norm_sqr() + b.norm_sqr())
    }
}

pub(super) fn z_eigenvalue(exponent: f64) -> Complex64 {
    match exponent % 2.0 {
        0.0 => Complex64::ONE,
        1.0 | -1.0 => -Complex64::ONE,
        0.5 | -1.5 => Complex64::I,
        -0.5 | 1.5 => -Complex64::I,
        turns => Complex64::from_polar(1.0, std::f64::consts::PI * turns),
    }
}

/// A pure product state. Each wire retains its relative phase through later gates.
#[derive(Debug, Clone)]
pub struct ProductState {
    wires: Vec<Wire>,
}

impl Circuit {
    /// Executes the first `step_count` columns while the state can be certified
    /// as a product of individual wires. Supports standard single-wire gates,
    /// SWAP, CX, CZ and CZPow. Controlled phases accept a known basis state
    /// on either operand or an identity phase. CX accepts a known basis-state
    /// control or a known X-eigenstate target.
    /// An unrecognized or entangling operation returns its zero-based authored location.
    pub fn run_product_prefix(&self, step_count: usize) -> Result<ProductState, ValidationError> {
        self.validate_compact_input("Product-state")?;
        let mut state = ProductState {
            wires: (0..self.num_qubits)
                .map(|wire| {
                    Wire::new(
                        self.initial_preps
                            .get(wire)
                            .copied()
                            .flatten()
                            .unwrap_or(QubitPrep::Zero),
                    )
                })
                .collect(),
        };
        self.visit_compact_operations(step_count, "Product-state", |op| {
            state.apply(op, &self.registry)
        })?;
        Ok(state)
    }
}

impl ProductState {
    fn apply(&mut self, op: &Operation, registry: &GateRegistry) -> Result<(), String> {
        let param = gate_parameter(op, registry)?;
        let entanglement_error = || {
            format!(
                "Gate {} on {:?} can create entanglement between these wires.",
                op.id, op.qubits
            )
        };
        match (&op.id, op.qubits.as_slice()) {
            (id, &[wire]) if *id != Gate::Custom => {
                self.wires[wire as usize].apply_single(op, registry, param)?
            }
            (Gate::SWAP, &[left, right]) => self.wires.swap(left as usize, right as usize),
            (Gate::CZ | Gate::CZPow, &[left, right]) => {
                let (left, right) = (left as usize, right as usize);
                let exponent = if op.id == Gate::CZ { 1.0 } else { param };
                if exponent % 2.0 == 0.0 {
                    return Ok(());
                }
                match (self.wires[left].basis_bit(), self.wires[right].basis_bit()) {
                    (Some(false), _) | (_, Some(false)) => {}
                    (Some(true), _) => self.wires[right].z_phase(exponent),
                    (_, Some(true)) => self.wires[left].z_phase(exponent),
                    _ => return Err(entanglement_error()),
                }
            }
            (Gate::CX, &[control, target]) => {
                let (control, target) = (control as usize, target as usize);
                match self.wires[control].basis_bit() {
                    Some(false) => {}
                    Some(true) => self.wires[target].amplitudes.swap(0, 1),
                    None => match self.wires[target].x_minus() {
                        Some(false) => {}
                        Some(true) => self.wires[control].z_phase(1.0),
                        None => return Err(entanglement_error()),
                    },
                }
            }
            _ => {
                return Err(format!(
                    "Gate {} is unsupported by the product-state simulator.",
                    op.id
                ))
            }
        }
        Ok(())
    }

    /// Probability of reading 1 on a wire, or `None` for an invalid wire.
    pub fn probability_one(&self, wire: usize) -> Option<f32> {
        self.wires.get(wire).map(|w| w.probability_one() as f32)
    }

    /// Returns a single computational-basis amplitude, including the phase of
    /// the authored gates. Index bit 0 selects q0. Queries use O(wires) time
    /// and constant additional space, even for a 128-wire register.
    pub fn amplitude(&self, index: u128) -> Option<Complex64> {
        if self.wires.len() < MAX_COMPACT_QUBITS && index >> self.wires.len() != 0 {
            return None;
        }
        let mut amplitude = Complex64::ONE;
        for (wire, state) in self.wires.iter().enumerate() {
            let bit = ((index >> wire) & 1) as usize;
            amplitude *= state.amplitudes[bit];
        }
        Some(amplitude)
    }

    /// Unit phase of the first occupied basis amplitude. Removing each factor's
    /// magnitude before multiplication also works when the joint amplitude underflows.
    pub fn reference_phase(&self) -> Complex64 {
        let mut phase = Complex64::ONE;
        for wire in &self.wires {
            let [zero, one] = wire.amplitudes;
            let first = if zero == Complex64::ZERO { one } else { zero };
            // Scale first so even a subnormal component has a representable norm.
            let scaled = first / first.re.abs().max(first.im.abs());
            phase *= scaled / scaled.norm();
        }
        phase / phase.norm()
    }

    /// The first occupied basis index and the spacing of the lowest varying
    /// bit. A single occupied index uses spacing 1.
    pub fn basis_slice(&self) -> (u128, u128) {
        let mut first = 0;
        let mut stride = None;
        for (wire, state) in self.wires.iter().enumerate() {
            let [zero, one] = state.amplitudes;
            let bit = 1u128 << wire;
            if zero == Complex64::ZERO {
                first |= bit;
            } else if one != Complex64::ZERO {
                stride.get_or_insert(bit);
            }
        }
        (first, stride.unwrap_or(1))
    }

    /// Draws reproducible joint measurements. Bit 0 is wire q0.
    pub fn sample(&self, shots: u32, seed: u64) -> Vec<u128> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut samples = vec![0; shots as usize];
        for (wire, state) in self.wires.iter().enumerate() {
            let p = state.probability_one();
            if p == 0.0 {
                continue;
            }
            for bits in &mut samples {
                if p == 1.0 || rng.gen::<f64>() < p {
                    *bits |= 1u128 << wire;
                }
            }
        }
        samples
    }
}
