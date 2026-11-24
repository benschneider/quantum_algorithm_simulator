//! A midpoint Z changes the exact all-zero echo amplitude to ⟨U†ZU⟩.
//! Check the full 256-wire paired MPS against a dense backward causal cone.
#[path = "sycamore_mps/dense.rs"]
mod dense;
#[path = "sycamore_mps/layers.rs"]
mod layers;
#[allow(dead_code)]
mod mps {
    include!("audit_benchmark/mps.rs");
}
#[allow(dead_code)]
#[path = "audit_benchmark/reference.rs"]
mod reference;
#[allow(dead_code)]
#[path = "audit_benchmark/workloads.rs"]
mod workloads;

use mps::ColumnMps;
use num_complex::Complex64;
use quantsim_core::types::{Gate, Operation};
use serde_json::json;
use std::error::Error;

const WIDTH: usize = 256;
const WIRE: usize = 0;
const CAP: usize = 256;
const TOLERANCE: f64 = 2e-10;

fn dense_cone_z(forward: &[Vec<Operation>], width: usize, wire: usize) -> (f64, Vec<usize>, usize) {
    let mut support = vec![false; width];
    support[wire] = true;
    let mut cone = Vec::new();
    for layer in forward.iter().rev() {
        // Gates in one authored layer act on disjoint wires.
        for op in layer.iter().rev() {
            if op.qubits.iter().any(|&q| support[q as usize]) {
                for &q in &op.qubits {
                    support[q as usize] = true;
                }
                cone.push(op);
            }
        }
    }
    let wires: Vec<_> = (0..width).filter(|&q| support[q]).collect();
    let local = |q: usize| wires.binary_search(&q).expect("wire in cone");
    // The independent dense implementation stores at most a 16-wire vector.
    let mut oracle = dense::DenseOracle::new(wires.len());
    for &op in cone.iter().rev() {
        let mut compact = op.clone();
        for q in &mut compact.qubits {
            *q = local(*q as usize) as u32;
        }
        oracle.apply(&compact);
    }
    let bit = 1 << local(wire);
    let expectation = oracle
        .amplitudes
        .iter()
        .enumerate()
        .map(|(i, a)| {
            if i & bit == 0 {
                a.norm_sqr()
            } else {
                -a.norm_sqr()
            }
        })
        .sum();
    (expectation, wires, cone.len())
}

fn main() -> Result<(), Box<dyn Error>> {
    let work = workloads::build("mps_ladder", WIDTH, "reverse");
    let (forward, reverse) = work.circuit.steps.split_at(work.forward_depth);
    let (expected, cone_wires, cone_gates) = dense_cone_z(forward, WIDTH, WIRE);
    let columns = WIDTH / 2;
    let mut state = ColumnMps::with_bond_cap(columns, CAP);
    for layer in forward {
        layers::apply_layer(&mut state, columns, layer, reference::matrix, |_| {})?;
    }
    let z = Operation::new(Gate::Z, vec![WIRE as u32], vec![]);
    state.apply_one(WIRE % columns, WIRE / columns, &reference::matrix(&z));
    for layer in reverse {
        layers::apply_layer(
            &mut state,
            columns,
            layer,
            |op| reference::matrix(op).adjoint(),
            |_| {},
        )?;
    }
    let amp = state.zero_amplitude();
    let error = (amp - Complex64::from(expected)).norm();
    let norm_error = (state.norm_squared_at_center() - 1.0).abs();
    let pass = error <= TOLERANCE && norm_error <= TOLERANCE;
    println!(
        "{}",
        json!({
            "qubits": WIDTH, "seed": workloads::LADDER_SEED, "rounds": workloads::LADDER_ROUNDS,
            "cap": CAP, "midpoint_z_wire": WIRE, "dense_cone_wires": cone_wires,
            "dense_cone_gates": cone_gates, "dense_expected_signed_amplitude": expected,
            "dense_expected_return_probability": expected * expected,
            "mps_amplitude_re": amp.re, "mps_amplitude_im": amp.im,
            "mps_return_probability": amp.norm_sqr(), "amplitude_error": error, "norm_error": norm_error,
            "cap_discarded_weight_sum": state.cap_discarded_weights().iter().sum::<f64>(),
            "absolute_tolerance": TOLERANCE, "passed": pass
        })
    );
    if !pass {
        return Err("perturbed echo differs from the dense causal-cone control".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense_cone_matches_full_perturbed_echo_at_small_widths() {
        for width in [8, 16] {
            let mut work = workloads::build("mps_ladder", width, "reverse");
            let forward = &work.circuit.steps[..work.forward_depth];
            let (expected, wires, _) = dense_cone_z(forward, width, WIRE);
            if width == 16 {
                assert!(wires.len() < width);
            }
            let z = Operation::new(Gate::Z, vec![WIRE as u32], vec![]);
            work.circuit.steps.insert(work.forward_depth, vec![z]);
            let full_echo = reference::dense(&work, "mps_ladder");
            assert!((full_echo[0] - Complex64::from(expected)).norm() < 1e-12);
        }
    }
}
