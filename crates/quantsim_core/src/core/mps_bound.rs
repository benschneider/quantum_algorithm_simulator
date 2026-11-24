use super::circuit::Circuit;
use super::compact::gate_parameter;
use super::types::{Gate, GateMatrix, Operation};
use num_bigint::BigUint;
use std::collections::{HashMap, HashSet};

/// Conservative exact-arithmetic upper bounds for a two-row column MPS.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BondBound {
    /// Maximum sufficient Schmidt ranks after any update, ordered left to right.
    pub cut_caps: Vec<BigUint>,
    /// Largest cut cap, or one for a single column.
    pub uniform_cap: BigUint,
    /// Retained Complex64 tensor entries in bytes, excluding solver workspace.
    pub tensor_bytes: BigUint,
}

/// Inspect authored gates on a product input, with the first half of the wires
/// forming the top row and the second half the bottom. A site is a whole column.
/// The upper bound can exceed the actual ranks for a specific input or gate sequence.
/// Arbitrary-precision outputs preserve the bound beyond addressable machine sizes.
pub fn circuit_bound(circuit: &Circuit) -> Result<BondBound, String> {
    circuit
        .validate_compact_input("MPS rank estimation")
        .map_err(|e| e.message)?;
    let wires = circuit.num_qubits;
    if wires % 2 != 0 {
        return Err("The two-row MPS layout requires an even number of wires.".into());
    }
    let columns = wires / 2;
    // All structural rank multipliers and dimensions are powers of two. Work
    // in exponents through the scan; include a rank-one boundary at each end.
    let mut ranks = vec![0usize; columns + 1];
    let mut growth = HashMap::<usize, usize>::new();
    let mut occupied = HashSet::new();
    for (step, layer) in circuit.steps.iter().enumerate() {
        occupied.clear();
        for op in layer {
            for &q in &op.qubits {
                if q as usize >= wires {
                    return Err(format!("Step {} uses q{q} outside the register.", step + 1));
                }
                if !occupied.insert(q) {
                    return Err(format!("Step {} uses q{q} more than once.", step + 1));
                }
            }
            if let Some((cut, exponent)) = crossing(circuit, columns, op)
                .map_err(|message| format!("Step {}, {}: {message}", step + 1, op.id))?
            {
                *growth.entry(cut).or_default() += exponent;
            }
        }
        // For each cut, its crossing gates can be commuted to the front of
        // this disjoint layer. The other gates stay on one side of that cut.
        // Thus both neighboring ranks come from BEFORE the layer, even when
        // its top and bottom gates cross adjacent cuts in different orders.
        for (&j, exp) in &mut growth {
            *exp = (ranks[j] + *exp).min(2 + ranks[j - 1].min(ranks[j + 1]));
        }
        for (j, exp) in growth.drain() {
            ranks[j] = exp;
        }
    }
    // Rank bounds grow monotonically from one. The final capacities therefore
    // also bound every earlier update, including a partly executed layer.
    let one = BigUint::from(1u8);
    let cut_caps: Vec<_> = ranks[1..columns].iter().map(|&exp| &one << exp).collect();
    let uniform_cap = &one << ranks.iter().max().copied().unwrap_or(0);
    // Each site stores 4 * left_rank * right_rank complex f64 entries.
    let tensor_bytes = ranks
        .windows(2)
        .map(|pair| BigUint::from(64u8) << (pair[0] + pair[1]))
        .sum();
    Ok(BondBound {
        cut_caps,
        uniform_cap,
        tensor_bytes,
    })
}

/// The cut index includes the left boundary. Each crossing returns log2 of
/// its operator Schmidt rank upper bound.
fn crossing(
    circuit: &Circuit,
    columns: usize,
    op: &Operation,
) -> Result<Option<(usize, usize)>, String> {
    if !(1..=2).contains(&op.qubits.len()) {
        return Err("only one- and two-wire gates are supported".into());
    }
    if let Some(matrix) = &op.matrix {
        let shape = match matrix {
            GateMatrix::BigEndian(m) => (m.nrows(), m.ncols()),
            GateMatrix::LittleEndian(m) => (m.nrows(), m.ncols()),
        };
        let dim = 1 << op.qubits.len();
        if shape != (dim, dim) {
            return Err("matrix dimensions disagree with the gate operands".into());
        }
    } else {
        if op.id == Gate::Custom {
            return Err("custom gate needs an explicit matrix".into());
        }
        gate_parameter(op, &circuit.registry)?;
    }
    let [first, second] = *op.qubits.as_slice() else {
        return Ok(None);
    };
    let (a, b) = (first as usize, second as usize);
    if a % columns == b % columns {
        return Ok(None); // A vertical gate stays inside a column.
    }
    if a / columns != b / columns || a.abs_diff(b) != 1 {
        return Err("the two wires must be neighbors on one row or in one column".into());
    }
    let controlled = op.matrix.is_none()
        && matches!(
            op.id,
            Gate::CX | Gate::CY | Gate::CZ | Gate::CRz | Gate::CXPow | Gate::CYPow | Gate::CZPow
        );
    Ok(Some((
        1 + a.min(b) % columns,
        if controlled { 1 } else { 2 },
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::QubitPrep;

    fn op(gate: Gate, wires: &[u32]) -> Operation {
        Operation::new(gate, wires.to_vec(), vec![])
    }

    #[test]
    fn product_and_rung_operations_leave_all_virtual_ranks_at_one() {
        let mut circuit = Circuit::new(8);
        circuit.initial_preps[0] = Some(QubitPrep::Plus);
        circuit.initial_preps[5] = Some(QubitPrep::One);
        circuit
            .steps
            .push(vec![op(Gate::CX, &[5, 1]), op(Gate::H, &[2])]);
        let bound = circuit_bound(&circuit).unwrap();
        assert_eq!(bound.cut_caps, vec![BigUint::from(1u8); 3]);
        assert_eq!(bound.tensor_bytes, BigUint::from(256u16));
        let one_site = circuit_bound(&Circuit::new(2)).unwrap();
        assert!(one_site.cut_caps.is_empty());
        assert_eq!(one_site.uniform_cap, BigUint::from(1u8));
        assert_eq!(one_site.tensor_bytes, BigUint::from(64u8));
    }

    #[test]
    fn controlled_gates_and_explicit_overrides_have_safe_operator_ranks() {
        let mut circuit = Circuit::new(8);
        circuit.steps = vec![vec![op(Gate::CZPow, &[2, 1])]];
        assert_eq!(
            circuit_bound(&circuit).unwrap().uniform_cap,
            BigUint::from(2u8)
        );
        circuit.steps[0].push(op(Gate::CX, &[5, 6]));
        assert_eq!(
            circuit_bound(&circuit).unwrap().uniform_cap,
            BigUint::from(4u8)
        );
        circuit.steps[0].pop();
        circuit.steps[0][0].matrix = circuit.registry.eval(&Gate::SWAP, &[], &[2, 1]);
        assert_eq!(
            circuit_bound(&circuit).unwrap().uniform_cap,
            BigUint::from(4u8)
        );
    }

    #[test]
    fn mixed_adjacent_cuts_are_bounded_independently_of_gate_order_in_a_layer() {
        let mut circuit = Circuit::new(12);
        circuit.steps = vec![
            vec![op(Gate::SYC, &[1, 2]), op(Gate::SYC, &[3, 4])],
            vec![
                op(Gate::CX, &[0, 1]),
                op(Gate::SYC, &[7, 8]),
                op(Gate::SYC, &[2, 3]),
            ],
        ];
        let forward = circuit_bound(&circuit).unwrap();
        circuit.steps[1].reverse();
        assert_eq!(forward, circuit_bound(&circuit).unwrap());
    }

    #[test]
    fn bound_uses_the_forward_schedule_and_changes_with_depth() {
        let exact = crate::mps_ladder::benchmark_bound();
        assert_eq!(exact.cut_caps.len(), 127);
        assert_eq!(exact.uniform_cap, BigUint::from(256u16));
        assert_eq!(exact.tensor_bytes, BigUint::from(128_066_048u32));
        for (cut, cap) in exact.cut_caps.iter().enumerate() {
            let distance = (cut + 1).min(127 - cut);
            let physical = BigUint::from(1u8) << (2 * distance);
            let crossings = BigUint::from(if cut % 2 == 0 { 64u16 } else { 256 });
            assert_eq!(cap, &physical.min(crossings));
        }
        let mut fixture = crate::mps_ladder::workloads::build("mps_ladder", 256, "forward").circuit;
        fixture.steps.truncate(8); // First four rounds.
        assert_eq!(
            circuit_bound(&fixture).unwrap().uniform_cap,
            BigUint::from(16u8)
        );
    }

    #[test]
    fn physical_rank_and_payload_can_exceed_machine_words() {
        let mut circuit = Circuit::new(140);
        for layer in 0..70 {
            let mut gates = Vec::new();
            for row in [0, 70] {
                for column in (layer % 2..69).step_by(2) {
                    gates.push(op(Gate::SYC, &[row + column, row + column + 1]));
                }
            }
            circuit.steps.push(gates);
        }
        let bound = circuit_bound(&circuit).unwrap();
        assert_eq!(bound.uniform_cap, BigUint::from(1u8) << 70);
        assert!(usize::try_from(&bound.uniform_cap).is_err());
        assert!(usize::try_from(&bound.tensor_bytes).is_err());
    }

    #[test]
    fn unsupported_inputs_and_layers_return_a_diagnostic() {
        assert!(circuit_bound(&Circuit::new(0)).is_err());
        assert!(circuit_bound(&Circuit::new(5)).is_err());
        let mut circuit = Circuit::new(4);
        circuit.initial_state = Some(vec![nalgebra::Complex::new(1., 0.); 16]);
        assert!(circuit_bound(&circuit)
            .unwrap_err()
            .contains("explicit initial state"));
        circuit.initial_state = None;
        for layer in [
            vec![op(Gate::CCNOT, &[0, 1, 2])],
            vec![op(Gate::CX, &[0, 3])],
            vec![op(Gate::CX, &[2, 2])],
            vec![op(Gate::X, &[4])],
            vec![op(Gate::H, &[1]), op(Gate::CX, &[0, 1])],
            vec![op(Gate::Custom, &[0, 1])],
            vec![op(Gate::CZ, &[0])],
        ] {
            circuit.steps = vec![layer];
            assert!(circuit_bound(&circuit).is_err());
        }
    }
}
