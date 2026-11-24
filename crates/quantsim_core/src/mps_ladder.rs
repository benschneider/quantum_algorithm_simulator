//! Rank estimates for two-row MPS circuits and the native seed-53 echo experiment.

pub use crate::core::mps_bound::{circuit_bound, BondBound};

// Share the fixture, ideal gates and evolution kernels with the native benchmark.
#[cfg(feature = "mps-ladder")]
#[path = "../examples/sycamore_mps/layers.rs"]
mod layers;
#[cfg(feature = "mps-ladder")]
#[allow(dead_code)]
#[path = "../examples/audit_benchmark/mps.rs"]
pub(super) mod mps;
#[cfg(feature = "mps-ladder")]
#[allow(dead_code)]
#[path = "../examples/audit_benchmark/reference.rs"]
mod reference;
#[allow(dead_code)]
#[path = "../examples/audit_benchmark/workloads.rs"]
pub(super) mod workloads;

/// Infer sufficient caps from the eight-round forward fixture. Its exact adjoint
/// retraces the same states, so the forward caps also bound the unperturbed echo.
pub fn benchmark_bound() -> BondBound {
    let fixture = workloads::build("mps_ladder", 256, "forward");
    circuit_bound(&fixture.circuit).expect("the benchmark is a valid two-row ladder")
}

/// Execute the 256-wire ladder and its adjoint from the all-zero input, with
/// a uniform maximum Schmidt rank. Return the raw squared all-zero amplitude.
/// Numerical roundoff can place a near-unit result slightly above one.
#[cfg(feature = "mps-ladder")]
pub fn run_echo(cap: usize) -> Result<f64, &'static str> {
    echo(256, cap)
}

#[cfg(feature = "mps-ladder")]
fn echo(wires: usize, cap: usize) -> Result<f64, &'static str> {
    if cap == 0 {
        return Err("bond cap must be positive");
    }
    let workload = workloads::build("mps_ladder", wires, "reverse");
    let columns = wires / 2;
    let mut state = mps::ColumnMps::with_bond_cap(columns, cap);
    for (i, layer) in workload.circuit.steps.iter().enumerate() {
        layers::apply_layer(
            &mut state,
            columns,
            layer,
            |op| {
                let matrix = reference::matrix(op);
                if i < workload.forward_depth {
                    matrix
                } else {
                    matrix.adjoint()
                }
            },
            |_| {},
        )?;
    }
    let probability = state.zero_amplitude().norm_sqr();
    let norm_error = (state.norm_squared_at_center() - 1.0).abs();
    if !probability.is_finite() || !norm_error.is_finite() || norm_error > 2e-10 {
        return Err("MPS echo failed the benchmark norm check");
    }
    Ok(probability)
}

#[cfg(all(test, feature = "mps-ladder"))]
mod tests {
    use super::*;
    use crate::{
        circuit::Circuit,
        types::{Gate, Operation},
    };
    use num_bigint::BigUint;

    #[test]
    fn the_selected_cap_controls_the_paired_echo() {
        assert!(run_echo(0).is_err());
        assert!(echo(8, 2).unwrap() < 0.9);
        assert!((echo(8, 16).unwrap() - 1.0).abs() < 2e-10);
    }

    #[test]
    fn prefix_bounds_cover_the_measured_ranks_after_every_paired_update() {
        let columns = 5usize;
        let width = 2 * columns;
        for offset in 0..3 {
            let mut circuit = Circuit::new(width);
            let mut state = mps::ColumnMps::new(columns);
            for round in 0..8 {
                let singles = (0..width)
                    .map(|q| {
                        let gate =
                            [Gate::SqrtX, Gate::H, Gate::SqrtY][(q + round + offset) % 3].clone();
                        Operation::new(gate, vec![q as u32], vec![])
                    })
                    .collect();
                let mut legs = Vec::new();
                for row in 0..2 {
                    // Aligned rows exercise pair fusion; staggered rows cross
                    // adjacent cuts in the same layer.
                    let parity = (round + row * (offset % 2)) % 2;
                    for col in (parity..columns - 1).step_by(2) {
                        let a = (row * columns + col) as u32;
                        let gate =
                            [Gate::SYC, Gate::CX, Gate::SWAP][(col + row + round) % 3].clone();
                        legs.push(Operation::new(gate, vec![a, a + 1], vec![]));
                    }
                }
                if offset == 2 {
                    legs.reverse();
                }
                let rungs = (round % 2..columns)
                    .step_by(2)
                    .map(|col| {
                        Operation::new(Gate::SYC, vec![col as u32, (col + columns) as u32], vec![])
                    })
                    .collect();
                for layer in [singles, legs, rungs] {
                    circuit.steps.push(layer);
                    let cut_caps = circuit_bound(&circuit).unwrap().cut_caps;
                    layers::apply_layer(
                        &mut state,
                        columns,
                        circuit.steps.last().unwrap(),
                        reference::matrix,
                        |s| {
                            for (observed, upper) in s.bonds().into_iter().zip(&cut_caps) {
                                assert!(&BigUint::from(observed) <= upper);
                            }
                        },
                    )
                    .unwrap();
                }
            }
        }
    }
}
