use nalgebra::DMatrix;
use num_complex::Complex64 as C;
use num_traits::ToPrimitive;
use quantsim_core::{
    types::{Gate, Operation, Param},
    BasisIndex,
};
use std::f64::consts::{FRAC_1_SQRT_2, PI, TAU};

use crate::workloads::{product_phase, qft_input, Workload};

pub fn matrix(op: &Operation) -> DMatrix<C> {
    let (z, o, h) = (C::ZERO, C::ONE, C::from(FRAC_1_SQRT_2));
    let p = C::new(0.5, 0.5);
    let phase = || {
        let Param::Scalar(x) = op.params[0];
        C::from_polar(1.0, PI * f64::from(x))
    };
    match op.id {
        Gate::H => DMatrix::from_row_slice(2, 2, &[h, h, h, -h]),
        Gate::Z => DMatrix::from_row_slice(2, 2, &[o, z, z, -o]),
        Gate::ZPow => DMatrix::from_row_slice(2, 2, &[o, z, z, phase()]),
        Gate::SqrtX => DMatrix::from_row_slice(2, 2, &[p, p.conj(), p.conj(), p]),
        Gate::SqrtY => DMatrix::from_row_slice(2, 2, &[p, -p, p, p]),
        Gate::CX | Gate::SWAP => {
            let mut m = DMatrix::identity(4, 4);
            let first = if op.id == Gate::CX { 2 } else { 1 };
            m.swap_rows(first, first + 1);
            m
        }
        Gate::CZPow => {
            let mut m = DMatrix::identity(4, 4);
            m[(3, 3)] = phase();
            m
        }
        Gate::SYC => {
            let mut m = DMatrix::zeros(4, 4);
            m[(0, 0)] = o;
            m[(1, 2)] = -C::I;
            m[(2, 1)] = -C::I;
            m[(3, 3)] = C::from_polar(1.0, -PI / 6.0);
            m
        }
        _ => panic!("missing independent matrix for {:?}", op.id),
    }
}

pub fn dense(workload: &Workload, name: &str) -> Vec<C> {
    let n = workload.circuit.num_qubits;
    assert!(n <= 16);
    let mut result = vec![C::ZERO; 1 << n];
    let basis = if name == "qft" {
        qft_input(n).to_usize().unwrap()
    } else {
        0
    };
    result[basis] = C::ONE;
    for (step, column) in workload.circuit.steps.iter().enumerate() {
        for op in column {
            let mut m = matrix(op);
            if name == "mps_ladder" && step >= workload.forward_depth {
                m = m.adjoint();
            }
            match *op.qubits.as_slice() {
                [a] => update(&mut result, [0, 1 << a], &m),
                [a, b] => update(&mut result, [0, 1 << b, 1 << a, (1 << a) | (1 << b)], &m),
                _ => unreachable!(),
            }
        }
    }
    result
}

fn update<const K: usize>(state: &mut [C], offsets: [usize; K], m: &DMatrix<C>) {
    let mask = offsets[K - 1];
    for base in 0..state.len() {
        if base & mask != 0 {
            continue;
        }
        let input = offsets.map(|offset| state[base | offset]);
        for (row, offset) in offsets.iter().enumerate() {
            state[base | offset] = (0..K).map(|col| m[(row, col)] * input[col]).sum();
        }
    }
}

pub fn analytical(name: &str, workload: &Workload, basis: &BasisIndex) -> C {
    let n = workload.circuit.num_qubits;
    if workload.inverse {
        return C::from(f64::from(basis == &workload.target));
    }
    let bit = |q: usize| basis.bit(q as u64);
    let all = |q: usize, width: usize| (q..q + width).all(|i| bit(i) == bit(q));
    let mut turns_in_pi = 0.0;
    let rank = match name {
        "product" => {
            turns_in_pi = (0..n)
                .filter(|&q| bit(q))
                .map(|q| f64::from(product_phase(q)))
                .sum();
            n
        }
        "qft" => {
            let residue = (qft_input(n) * basis) % (BasisIndex::from(1u8) << n);
            // Scale after extracting the significand. Computing 2^(-1024)
            // via powi can lose the subnormal scale by overflowing its reciprocal.
            let shift = residue.bits().saturating_sub(53);
            let significand = (residue >> shift).to_u64().unwrap() as f64;
            let turns = significand * (shift as f64 - n as f64).exp2();
            return C::from_polar(2f64.powi(-(n as i32 / 2)), TAU * turns);
        }
        "ghz" => {
            if !all(0, n) {
                return C::ZERO;
            }
            1
        }
        "groups" => {
            for q in (0..n).step_by(4) {
                if !all(q, (n - q).min(4)) {
                    return C::ZERO;
                }
                turns_in_pi += if bit(q) { 0.25 } else { 0.0 };
            }
            n.div_ceil(4)
        }
        "non_clifford" => {
            let half = n / 2;
            for q in 0..half {
                if bit(q) != bit(q + half) {
                    return C::ZERO;
                }
                if bit(q) {
                    turns_in_pi += 0.25;
                    if q > 0 && bit(q - 1) {
                        turns_in_pi += 0.25;
                    }
                }
            }
            half
        }
        _ => unreachable!(),
    };
    C::from_polar((-0.5 * rank as f64).exp2(), PI * (turns_in_pi % 2.0))
}

pub fn probes(n: usize, target: &BasisIndex) -> Vec<BasisIndex> {
    let one = BasisIndex::from(1u8);
    let mut result = vec![BasisIndex::default(), target.clone(), (&one << n) - &one];
    for q in 0..n {
        result.push(&one << q);
    }
    for q in (0..n).step_by(4) {
        result.push(((BasisIndex::from(1u8) << (n - q).min(4)) - &one) << q);
    }
    for low in [1u32, 3, 5, 23, 173, 321, 1827] {
        let half = n / 2;
        let mask = BasisIndex::from(low) % (&one << half);
        result.push(&mask | (&mask << half));
    }
    result
}
