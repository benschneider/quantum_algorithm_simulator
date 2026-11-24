//! A tensor product of small state vectors. A multi-wire gate joins only
//! the independent factors it acts on, after checking the required width.
use super::circuit::{Circuit, ValidationError};
use super::compact::gate_parameter;
use super::engine::OtherQubitStateIterator;
use super::gates::registry::GateRegistry;
use super::product_state::{z_eigenvalue, Wire};
use super::types::{Gate, GateMatrix, Operation, QubitPrep};
use num_complex::Complex64;
use smallvec::SmallVec;

mod readout;
mod split;

/// Maximum number of wires in any one dense factor.
pub const MAX_GROUP_QUBITS: usize = 8;

#[derive(Debug, Clone)]
enum FactorState {
    Single(Wire),
    Group(Vec<Complex64>),
}

#[derive(Debug, Clone)]
struct Factor {
    // wires[i] is the physical wire for bit i of this factor's amplitude index.
    wires: Vec<u32>,
    state: FactorState,
}

impl Factor {
    fn amplitudes(&self) -> &[Complex64] {
        match &self.state {
            FactorState::Single(wire) => &wire.amplitudes,
            FactorState::Group(values) => values,
        }
    }

    fn local_bit(&self, wire: u32) -> usize {
        self.wires
            .iter()
            .position(|&w| w == wire)
            .expect("factor contains operand")
    }

    fn apply_matrix(&mut self, op: &Operation, registry: &GateRegistry) -> Result<(), String> {
        let Some(GateMatrix::BigEndian(matrix)) = registry.eval(&op.id, &op.params, &op.qubits)
        else {
            return Err(format!("Could not evaluate built-in gate {}.", op.id));
        };
        // Registry matrices use the lowest-numbered physical operand as their
        // most significant local bit, including reversed controls and targets.
        let mut sorted = op.qubits.clone();
        sorted.sort_unstable();
        let bits: SmallVec<[usize; 3]> = sorted
            .iter()
            .rev()
            .map(|&wire| self.local_bit(wire))
            .collect();
        let dimension = 1 << bits.len();
        let mut offsets = [0; 8];
        for (index, offset) in offsets[..dimension].iter_mut().enumerate() {
            for (i, &local_bit) in bits.iter().enumerate() {
                *offset |= ((index >> i) & 1) << local_bit;
            }
        }
        let FactorState::Group(values) = &mut self.state else {
            unreachable!("one-wire operations retain their singleton state")
        };
        let mut input = [Complex64::ZERO; 8];
        for base in OtherQubitStateIterator::new(self.wires.len(), &bits) {
            for local in 0..dimension {
                input[local] = values[base | offsets[local]];
            }
            for row in 0..dimension {
                values[base | offsets[row]] = (0..dimension)
                    .map(|col| {
                        let z = matrix[(row, col)];
                        Complex64::new(f64::from(z.re), f64::from(z.im)) * input[col]
                    })
                    .sum();
            }
        }
        // The registry stores unitary coefficients as f32. Normalize the
        // complete factor, preserving relative amplitudes across subspaces.
        let norm = values.iter().map(Complex64::norm_sqr).sum::<f64>().sqrt();
        if norm == 0.0 || !norm.is_finite() {
            return Err(format!("Gate {} produced invalid amplitudes.", op.id));
        }
        for value in values {
            *value /= norm;
        }
        Ok(())
    }
}

/// A pure state whose independent factors partition the physical wires.
#[derive(Debug, Clone)]
pub struct GroupedState {
    num_qubits: usize,
    factors: Vec<Factor>,
}

impl Circuit {
    /// Executes up to `step_count` columns using bounded, independent factors.
    /// A gate requiring a factor wider than the limit returns its authored
    /// location. Separable single wires can leave a factor after a gate.
    pub fn run_grouped_prefix(&self, step_count: usize) -> Result<GroupedState, ValidationError> {
        self.validate_compact_input("Grouped")?;
        let mut state = GroupedState {
            num_qubits: self.num_qubits,
            factors: (0..self.num_qubits)
                .map(|i| Factor {
                    wires: vec![i as u32],
                    state: FactorState::Single(Wire::new(
                        self.initial_preps
                            .get(i)
                            .copied()
                            .flatten()
                            .unwrap_or(QubitPrep::Zero),
                    )),
                })
                .collect(),
        };
        self.visit_compact_operations(step_count, "Grouped", |op| state.apply(op, &self.registry))?;
        Ok(state)
    }
}

impl GroupedState {
    fn factor_index(&self, wire: u32) -> usize {
        self.factors
            .iter()
            .position(|f| f.wires.contains(&wire))
            .expect("validated wire")
    }

    fn singleton(&self, wire: u32) -> Option<&Wire> {
        match &self.factors[self.factor_index(wire)].state {
            FactorState::Single(state) => Some(state),
            FactorState::Group(_) => None,
        }
    }

    fn join(&mut self, operands: &[u32], gate: &Gate) -> Result<usize, String> {
        let mut indices: SmallVec<[usize; 3]> =
            operands.iter().map(|&q| self.factor_index(q)).collect();
        indices.sort_unstable();
        indices.dedup();
        if indices.len() == 1 {
            return Ok(indices[0]);
        }
        let width: usize = indices.iter().map(|&i| self.factors[i].wires.len()).sum();
        if width > MAX_GROUP_QUBITS {
            return Err(format!(
                "Gate {gate} on {operands:?} would require a {width}-qubit group. The limit is {MAX_GROUP_QUBITS}."
            ));
        }
        let mut wires = Vec::with_capacity(width);
        let mut values = vec![Complex64::ONE; 1 << width];
        // Concatenate the local wire lists. Each source amplitude depends on
        // just its contiguous field of bits in the new factor.
        for &index in indices.iter().rev() {
            let factor = self.factors.remove(index);
            let source = factor.amplitudes();
            for (basis, value) in values.iter_mut().enumerate() {
                *value *= source[(basis >> wires.len()) & (source.len() - 1)];
            }
            wires.extend(factor.wires);
        }
        self.factors.push(Factor {
            wires,
            state: FactorState::Group(values),
        });
        Ok(self.factors.len() - 1)
    }

    fn apply_z_power(
        &mut self,
        operands: &[u32],
        exponent: f64,
        gate: &Gate,
    ) -> Result<(), String> {
        self.apply_phase(
            operands,
            z_eigenvalue(exponent),
            exponent % 0.5 == 0.0,
            gate,
        )
    }

    fn apply_phase(
        &mut self,
        operands: &[u32],
        phase: Complex64,
        is_clifford: bool,
        gate: &Gate,
    ) -> Result<(), String> {
        if phase == Complex64::ONE {
            return Ok(());
        }
        // Restrict the all-ones phase to wires that can vary. This lets wide
        // basis-input QFTs run without forming a dense group at every CZPow.
        let mut active = SmallVec::<[u32; 3]>::new();
        for &wire in operands {
            match self.singleton(wire).and_then(Wire::basis_bit) {
                Some(false) => return Ok(()),
                Some(true) => {}
                None => active.push(wire),
            }
        }
        if active.is_empty() {
            // All operands are one. Retain the global phase on any one factor.
            active.push(operands[0]);
        }
        let index = self.join(&active, gate)?;
        let factor = &mut self.factors[index];
        let mask = active
            .iter()
            .fold(0, |m, &q| m | (1 << factor.local_bit(q)));
        match &mut factor.state {
            FactorState::Single(wire) => wire.multiply_one_phase(phase, is_clifford),
            FactorState::Group(values) => {
                for (i, value) in values.iter_mut().enumerate() {
                    if i & mask == mask {
                        *value *= phase;
                    }
                }
                if active.len() > 1 {
                    self.split_singletons(index);
                }
            }
        }
        Ok(())
    }

    fn apply(&mut self, op: &Operation, registry: &GateRegistry) -> Result<(), String> {
        let param = gate_parameter(op, registry)?;
        match op.id {
            Gate::Custom => {
                return Err("Gate Custom is unsupported by the grouped simulator.".into())
            }
            Gate::SWAP => {
                let (a, b) = (op.qubits[0], op.qubits[1]);
                for wire in self.factors.iter_mut().flat_map(|factor| &mut factor.wires) {
                    if *wire == a {
                        *wire = b;
                    } else if *wire == b {
                        *wire = a;
                    }
                }
                return Ok(());
            }
            Gate::ZPow | Gate::CZPow | Gate::CCZPow => {
                return self.apply_z_power(&op.qubits, param, &op.id)
            }
            Gate::CZ | Gate::CCZ => return self.apply_z_power(&op.qubits, 1.0, &op.id),
            Gate::CRz => {
                // In this registry CRz applies exp(i theta) only to |11>.
                return self.apply_phase(
                    &op.qubits,
                    Complex64::from_polar(1.0, param),
                    false,
                    &op.id,
                );
            }
            Gate::CXPow | Gate::CYPow if param % 2.0 == 0.0 => return Ok(()),
            _ => {}
        }
        if let Some((controls, target, action)) = op.controlled_action() {
            let mut active = SmallVec::<[u32; 2]>::new();
            for &control in controls {
                match self.singleton(control).and_then(Wire::basis_bit) {
                    Some(false) => return Ok(()),
                    Some(true) => {}
                    None => active.push(control),
                }
            }
            if active.len() < controls.len() {
                let gate = if active.is_empty() { action } else { Gate::CX }; // CCNOT with one fixed control
                active.push(target);
                let reduced = Operation {
                    id: gate,
                    qubits: active,
                    params: op.params.clone(),
                    matrix: None,
                    name: None,
                };
                return self.apply(&reduced, registry);
            }
            if matches!(action, Gate::X | Gate::XPow) {
                if let Some(minus) = self.singleton(target).and_then(Wire::x_minus) {
                    return if minus {
                        self.apply_z_power(
                            controls,
                            if action == Gate::X { 1.0 } else { param },
                            &op.id,
                        )
                    } else {
                        Ok(())
                    };
                }
            }
        }
        let index = self.join(&op.qubits, &op.id)?;
        let factor = &mut self.factors[index];
        match &mut factor.state {
            FactorState::Single(wire) => wire.apply_single(op, registry, param),
            FactorState::Group(_) => {
                factor.apply_matrix(op, registry)?;
                if op.qubits.len() > 1 {
                    self.split_singletons(index);
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_on_fixed_operand_and_identity_powers_fit_beside_a_full_group() {
        let mut circuit = Circuit::new(9);
        circuit.initial_preps[..8].fill(Some(QubitPrep::Plus));
        for wire in 1..8 {
            circuit
                .steps
                .push(vec![Operation::new(Gate::CZPow, vec![0, wire], vec![0.25])]);
        }
        circuit
            .steps
            .push(vec![Operation::new(Gate::CRz, vec![0, 8], vec![-0.5])]);
        circuit
            .steps
            .push(vec![Operation::new(Gate::X, vec![8], vec![])]);
        circuit
            .steps
            .push(vec![Operation::new(Gate::CRz, vec![8, 0], vec![0.375])]);
        let state = circuit.run_grouped_prefix(circuit.steps.len()).unwrap();
        assert_eq!(state.largest_group_size(), 8);
        assert!((state.amplitude(256).unwrap() - 1.0 / 16.0).norm() < 1e-12);
        let expected = Complex64::from_polar(1.0 / 16.0, 0.375);
        assert!((state.amplitude(257).unwrap() - expected).norm() < 1e-12);

        circuit
            .steps
            .push(vec![Operation::new(Gate::H, vec![8], vec![])]);
        circuit
            .steps
            .push(vec![Operation::new(Gate::SqrtZ, vec![8], vec![])]);
        let before = circuit.run_grouped_prefix(circuit.steps.len()).unwrap();
        for gate in [Gate::CXPow, Gate::CYPow, Gate::CRz] {
            circuit
                .steps
                .push(vec![Operation::new(gate, vec![0, 8], vec![0.0])]);
        }
        let after = circuit.run_grouped_prefix(circuit.steps.len()).unwrap();
        assert_eq!(after.largest_group_size(), 8);
        for i in 0..512 {
            assert_eq!(after.amplitude(i), before.amplitude(i));
        }
    }
}
