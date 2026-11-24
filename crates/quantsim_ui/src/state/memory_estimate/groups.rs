//! An upper bound on grouped storage from gate operands and definite input bits.
//! The estimator never allocates an amplitude vector or assumes a group will split.
use quantsim_core::{
    circuit::Circuit,
    types::{Gate, Operation, Param, QubitPrep},
};
use smallvec::SmallVec;
use std::collections::HashMap;

struct Node {
    root: usize,
    size: usize,
    // A known bit is always in a singleton factor.
    bit: Option<bool>,
}

#[derive(Default)]
struct Groups {
    // SWAP moves the labels, leaving the factors themselves intact.
    wires: HashMap<u32, usize>,
    nodes: Vec<Node>,
}

impl Groups {
    fn node(&mut self, wire: u32, circuit: &Circuit) -> usize {
        *self.wires.entry(wire).or_insert_with(|| {
            let id = self.nodes.len();
            let bit = match circuit.initial_preps.get(wire as usize).copied().flatten() {
                Some(QubitPrep::Plus | QubitPrep::Minus) => None,
                Some(QubitPrep::One) => Some(true),
                Some(QubitPrep::Zero) | None => Some(false),
            };
            self.nodes.push(Node {
                root: id,
                size: 1,
                bit,
            });
            id
        })
    }

    fn root(&mut self, mut i: usize) -> usize {
        while self.nodes[i].root != i {
            self.nodes[i].root = self.nodes[self.nodes[i].root].root;
            i = self.nodes[i].root;
        }
        i
    }

    fn join(&mut self, operands: &[usize]) {
        let Some((&first, rest)) = operands.split_first() else {
            return;
        };
        let mut root = self.root(first);
        self.nodes[first].bit = None;
        for &id in rest {
            self.nodes[id].bit = None;
            let mut other = self.root(id);
            if other == root {
                continue;
            }
            if self.nodes[root].size < self.nodes[other].size {
                std::mem::swap(&mut root, &mut other);
            }
            self.nodes[root].size += self.nodes[other].size;
            self.nodes[other].root = root;
        }
    }

    fn gate(&mut self, op: &Operation, circuit: &Circuit) {
        let ids: SmallVec<[usize; 3]> = op
            .qubits
            .iter()
            .map(|&wire| self.node(wire, circuit))
            .collect();
        let angle = op.params.first().map_or(0.0, |Param::Scalar(x)| *x);
        if let [a, b] = ids.as_slice() {
            if op.id == Gate::SWAP {
                self.wires.insert(op.qubits[0], *b);
                self.wires.insert(op.qubits[1], *a);
                return;
            }
        }
        if let [id] = ids.as_slice() {
            let bit = &mut self.nodes[*id].bit;
            match op.id {
                Gate::Z | Gate::SqrtZ | Gate::ZPow | Gate::Rz => {}
                Gate::X | Gate::Y => *bit = bit.map(|b| !b),
                Gate::Rx | Gate::Ry | Gate::XPow | Gate::YPow if angle == 0.0 => {}
                _ => *bit = None,
            }
            return;
        }

        if matches!(
            op.id,
            Gate::CZ | Gate::CCZ | Gate::CZPow | Gate::CCZPow | Gate::CRz
        ) {
            // The grouped executor drops inactive controls on an all-ones phase.
            if ids.iter().any(|&id| self.nodes[id].bit == Some(false))
                || matches!(op.id, Gate::CZPow | Gate::CCZPow) && angle % 2.0 == 0.0
                || op.id == Gate::CRz && angle == 0.0
            {
                return;
            }
            let active: SmallVec<[usize; 3]> = ids
                .into_iter()
                .filter(|&id| self.nodes[id].bit.is_none())
                .collect();
            self.join(&active);
        } else if let Some((_, _, action)) = op.controlled_action() {
            let (&target, controls) = ids.split_last().expect("controlled gate has a target");
            if controls.iter().any(|&id| self.nodes[id].bit == Some(false))
                || matches!(action, Gate::XPow | Gate::YPow) && angle % 2.0 == 0.0
            {
                return;
            }
            let mut active: SmallVec<[usize; 3]> = controls
                .iter()
                .copied()
                .filter(|&id| self.nodes[id].bit.is_none())
                .collect();
            if active.is_empty() && matches!(action, Gate::X | Gate::Y) {
                self.nodes[target].bit = self.nodes[target].bit.map(|b| !b);
            } else {
                active.push(target);
                self.join(&active);
            }
        } else {
            self.join(&ids);
        }
    }
}

pub(super) fn amplitude_bound(circuit: &Circuit) -> u128 {
    let mut groups = Groups::default();
    for op in circuit.steps.iter().flatten() {
        // Compact executors stop at an unsupported definition or malformed gate.
        if op.matrix.is_some()
            || op.id == Gate::Custom
            || op.qubits.iter().any(|&q| q as usize >= circuit.num_qubits)
            || !circuit
                .registry
                .get_meta(&op.id)
                .is_some_and(|m| m.arity.as_usize() == op.qubits.len())
        {
            break;
        }
        groups.gate(op, circuit);
    }
    let mut amplitudes = (circuit.num_qubits - groups.nodes.len()) as u128 * 2;
    for (i, node) in groups.nodes.iter().enumerate() {
        if i == node.root {
            if node.size >= u64::BITS as usize {
                return super::SATURATED;
            }
            amplitudes += 1u128 << node.size;
        }
    }
    amplitudes.min(super::SATURATED)
}
