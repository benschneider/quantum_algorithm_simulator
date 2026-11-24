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

/// Order of the sites on either side of a Schmidt cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CutOrder {
    /// One site per wire, ordered q0, q1, ...
    WireOrder,
    /// One site per column, pairing wire j with wire j + (number of wires / 2).
    PairedColumns,
}

impl CutOrder {
    /// Number of sites under this ordering. A two-row ladder needs an even width.
    pub fn site_count(self, wires: usize) -> Result<usize, String> {
        match self {
            Self::WireOrder => Ok(wires),
            Self::PairedColumns if wires % 2 == 0 => Ok(wires / 2),
            Self::PairedColumns => {
                Err("The two-row MPS layout requires an even number of wires.".into())
            }
        }
    }

    /// Whether a gate has valid operands on both sides of this interior cut.
    pub fn crosses(self, qubits: &[u32], wires: usize, left_sites: usize) -> bool {
        let Ok(sites) = self.site_count(wires) else {
            return false;
        };
        if !(1..sites).contains(&left_sites) {
            return false;
        }
        let (mut left, mut right) = (false, false);
        for &q in qubits {
            let wire = q as usize;
            if wire >= wires {
                return false;
            }
            // For wire order, sites == wires; for paired columns this folds the two rows.
            if wire % sites < left_sites {
                left = true;
            } else {
                right = true;
            }
        }
        left && right
    }
}

/// Structural rank bound at one cut after a selected circuit step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CutBound {
    /// Upper bound on the state's Schmidt rank across this cut.
    pub rank: BigUint,
    /// Indices in the selected step's operation list that cross this cut, in authored order.
    pub crossing_gates: Vec<usize>,
}

/// Inspect a product-input prefix. `step` counts applied layers (zero selects the
/// input) and `left_sites` counts sites to the left of an interior cut.
/// Wire order admits arbitrary two-wire connections. Paired columns use the
/// nearest-neighbor ladder update and neighboring-rank limits from [`circuit_bound`].
pub fn cut_bound(
    circuit: &Circuit,
    step: usize,
    order: CutOrder,
    left_sites: usize,
) -> Result<CutBound, String> {
    let sites = order.site_count(circuit.num_qubits)?;
    if !(1..sites).contains(&left_sites) {
        return Err(format!(
            "Cut after {left_sites} sites must leave a site on each side."
        ));
    }
    let prefix = checked_prefix(circuit, step)?;
    let exponent = match order {
        CutOrder::WireOrder => {
            let physical = left_sites.min(sites - left_sites);
            let mut rank = 0usize;
            let mut occupied = HashSet::new();
            for (i, layer) in prefix.iter().enumerate() {
                visit_pairs(circuit, i, layer, &mut occupied, |[a, b], growth| {
                    if (a < left_sites) != (b < left_sites) {
                        rank = (rank + growth).min(physical);
                    }
                    Ok(())
                })?;
            }
            rank
        }
        CutOrder::PairedColumns => column_rank_exponents(circuit, prefix, sites)?[left_sites],
    };
    let crossing_gates = prefix
        .last()
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, op)| {
            order
                .crosses(&op.qubits, circuit.num_qubits, left_sites)
                .then_some(index)
        })
        .collect();
    Ok(CutBound {
        rank: BigUint::from(1u8) << exponent,
        crossing_gates,
    })
}

fn checked_prefix(circuit: &Circuit, step_count: usize) -> Result<&[Vec<Operation>], String> {
    circuit
        .validate_compact_input("MPS rank estimation")
        .map_err(|e| e.message)?;
    circuit.steps.get(..step_count).ok_or_else(|| {
        format!(
            "Step {step_count} exceeds the circuit's {} steps.",
            circuit.steps.len()
        )
    })
}

/// Inspect authored gates on a product input, with the first half of the wires
/// forming the top row and the second half the bottom. A site is a whole column.
/// The upper bound can exceed the actual ranks for a specific input or gate sequence.
/// Arbitrary-precision outputs preserve the bound beyond addressable machine sizes.
pub fn circuit_bound(circuit: &Circuit) -> Result<BondBound, String> {
    let columns = CutOrder::PairedColumns.site_count(circuit.num_qubits)?;
    let prefix = checked_prefix(circuit, circuit.steps.len())?;
    let ranks = column_rank_exponents(circuit, prefix, columns)?;
    // Rank bounds grow monotonically from one. The final capacities therefore
    // also bound every earlier update, including a partly executed layer.
    let one = BigUint::from(1u8);
    let cut_caps: Vec<_> = ranks[1..ranks.len() - 1]
        .iter()
        .map(|&exp| &one << exp)
        .collect();
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

fn column_rank_exponents(
    circuit: &Circuit,
    prefix: &[Vec<Operation>],
    columns: usize,
) -> Result<Vec<usize>, String> {
    // All structural rank multipliers and dimensions are powers of two. Work
    // in exponents through the scan; include a rank-one boundary at each end.
    let mut ranks = vec![0usize; columns + 1];
    let mut growth = HashMap::<usize, usize>::new();
    let mut occupied = HashSet::new();
    for (step, layer) in prefix.iter().enumerate() {
        visit_pairs(circuit, step, layer, &mut occupied, |[a, b], exponent| {
            if a % columns == b % columns {
                return Ok(()); // Rung gates stay inside a site.
            }
            if a / columns != b / columns || a.abs_diff(b) != 1 {
                return Err("the two wires must be neighbors on one row or in one column".into());
            }
            let cut = 1 + a.min(b) % columns;
            *growth.entry(cut).or_default() += exponent;
            Ok(())
        })?;
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
    Ok(ranks)
}

/// Visit each valid two-wire gate with its log2 operator rank bound.
fn visit_pairs(
    circuit: &Circuit,
    step: usize,
    layer: &[Operation],
    occupied: &mut HashSet<u32>,
    mut visit: impl FnMut([usize; 2], usize) -> Result<(), String>,
) -> Result<(), String> {
    occupied.clear();
    for op in layer {
        for &q in &op.qubits {
            if q as usize >= circuit.num_qubits {
                return Err(format!("Step {} uses q{q} outside the register.", step + 1));
            }
            if !occupied.insert(q) {
                return Err(format!("Step {} uses q{q} more than once.", step + 1));
            }
        }
        let at_gate = |message| format!("Step {}, {}: {message}", step + 1, op.id);
        if let Some((pair, exponent)) = gate_pair(circuit, op).map_err(at_gate)? {
            visit(pair, exponent).map_err(at_gate)?;
        }
    }
    Ok(())
}

fn gate_pair(circuit: &Circuit, op: &Operation) -> Result<Option<([usize; 2], usize)>, String> {
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
    let controlled = op.matrix.is_none()
        && matches!(
            op.id,
            Gate::CX | Gate::CY | Gate::CZ | Gate::CRz | Gate::CXPow | Gate::CYPow | Gate::CZPow
        );
    Ok(Some((
        [first as usize, second as usize],
        if controlled { 1 } else { 2 },
    )))
}

#[cfg(test)]
mod tests {
    use super::CutOrder::{PairedColumns, WireOrder};
    use super::*;
    use crate::types::QubitPrep;

    fn op(gate: Gate, wires: &[u32]) -> Operation {
        Operation::new(gate, wires.to_vec(), vec![])
    }

    #[test]
    fn selected_cut_tracks_the_prefix_and_its_crossing_gate_indices() {
        let mut circuit = Circuit::new(8);
        circuit.steps = vec![
            vec![op(Gate::H, &[1])],
            vec![op(Gate::Z, &[7]), op(Gate::CX, &[1, 2])],
            vec![
                op(Gate::CX, &[3, 7]),
                op(Gate::SWAP, &[6, 5]),
                op(Gate::CZ, &[2, 1]),
            ],
            vec![],
            vec![op(Gate::CX, &[0, 3])], // Unsupported connection in a later step.
        ];
        let selected = |rank: u8, gates: &[usize]| CutBound {
            rank: BigUint::from(rank),
            crossing_gates: gates.to_vec(),
        };
        assert_eq!(
            cut_bound(&circuit, 0, PairedColumns, 2).unwrap(),
            selected(1, &[])
        );
        assert_eq!(
            cut_bound(&circuit, 1, PairedColumns, 2).unwrap(),
            selected(1, &[])
        );
        assert_eq!(
            cut_bound(&circuit, 2, PairedColumns, 2).unwrap(),
            selected(2, &[1])
        );
        assert_eq!(
            cut_bound(&circuit, 3, PairedColumns, 2).unwrap(),
            selected(4, &[1, 2])
        );
        assert_eq!(
            cut_bound(&circuit, 4, PairedColumns, 2).unwrap(),
            selected(4, &[])
        );
        assert_eq!(
            cut_bound(&circuit, 3, PairedColumns, 1).unwrap(),
            selected(1, &[])
        );
        assert!(cut_bound(&circuit, 5, PairedColumns, 2)
            .unwrap_err()
            .contains("Step 5"));

        // A bare CNOT and one with a superposed control have the same structural bound.
        circuit.steps[0].clear();
        assert_eq!(
            cut_bound(&circuit, 2, PairedColumns, 2).unwrap(),
            selected(2, &[1])
        );
    }

    #[test]
    fn wire_order_covers_distant_gates_and_limits_growth_to_the_smaller_side() {
        let mut circuit = Circuit::new(5);
        circuit.steps = vec![
            vec![op(Gate::H, &[4])],
            vec![op(Gate::CX, &[4, 0])],
            vec![op(Gate::SYC, &[3, 1]), op(Gate::CX, &[0, 4])],
            vec![op(Gate::CCNOT, &[0, 1, 4])],
        ];
        assert_eq!(
            cut_bound(&circuit, 1, WireOrder, 2).unwrap().rank,
            BigUint::from(1u8)
        );
        let cx = cut_bound(&circuit, 2, WireOrder, 2).unwrap();
        assert_eq!(cx.rank, BigUint::from(2u8));
        assert_eq!(cx.crossing_gates, [0]);
        let middle = cut_bound(&circuit, 3, WireOrder, 2).unwrap();
        assert_eq!(middle.rank, BigUint::from(4u8));
        assert_eq!(middle.crossing_gates, [0, 1]);
        let edge = cut_bound(&circuit, 3, WireOrder, 4).unwrap();
        assert_eq!(edge.rank, BigUint::from(2u8));
        assert_eq!(edge.crossing_gates, [1]);
        assert!(cut_bound(&circuit, 4, WireOrder, 2)
            .unwrap_err()
            .contains("Step 4"));
    }

    #[test]
    fn crossing_geometry_is_available_even_when_the_bound_is_unsupported() {
        let wires = 8;
        assert!(WireOrder.crosses(&[0, 4], wires, 2));
        assert!(!PairedColumns.crosses(&[0, 4], wires, 2));
        assert!(PairedColumns.crosses(&[4, 5, 6], wires, 2));
        assert!(!PairedColumns.crosses(&[0, 6, 8], wires, 2));
        assert!(!PairedColumns.crosses(&[0, 6], 7, 2));
        assert!(!WireOrder.crosses(&[0, 1], 2, 2));
    }

    #[test]
    fn selected_cut_requires_a_valid_prefix_and_an_interior_column_boundary() {
        let circuit = Circuit::new(6);
        for cut in [0, 3, usize::MAX] {
            assert!(cut_bound(&circuit, 0, PairedColumns, cut).is_err());
        }
        assert!(cut_bound(&Circuit::new(2), 0, PairedColumns, 1).is_err());
        assert!(cut_bound(&circuit, 1, PairedColumns, 1).is_err());
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
