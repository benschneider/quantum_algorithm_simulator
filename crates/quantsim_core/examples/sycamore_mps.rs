//! Measure a column MPS and compare its full complex state to two dense references.
#[path = "sycamore_mps/budget.rs"]
mod budget;
#[path = "sycamore_mps/chain.rs"]
mod chain;
#[path = "sycamore_mps/dense.rs"]
mod dense;
#[path = "sycamore_mps/layers.rs"]
mod layers;
#[path = "sycamore_mps/memory.rs"]
mod memory;
#[path = "sycamore_mps/patch.rs"]
mod patch;
#[path = "sycamore_mps/planning.rs"]
mod planning;
#[path = "sycamore_mps/readouts.rs"]
mod readouts;
#[path = "sycamore_mps/sweep.rs"]
mod sweep;

use chain::ColumnMps;
use dense::{ideal_matrix, DenseOracle};
use memory::BackendHeap;
use num_complex::Complex64;
use patch::{COLUMNS, WIRES};
use quantsim_core::{circuit::CircuitData, engine::QuantumState, types::Operation};
use std::{error::Error, fs::File, mem::size_of_val};

enum Mode {
    Uncapped,
    Sweep(Vec<usize>, sweep::Report),
    Paired(sweep::PairedPolicy),
}

fn apply_to_mps(state: &mut ColumnMps, op: &Operation) -> Result<(), &'static str> {
    let gate = ideal_matrix(&op.id);
    let columns = COLUMNS as u32;
    match *op.qubits.as_slice() {
        [q] => state.apply_one((q % columns) as usize, (q / columns) as usize, &gate),
        [a, b] if a % columns == b % columns => state.apply_rung((a % columns) as usize, &gate),
        [a, b] if a / columns == b / columns && a.abs_diff(b) == 1 => {
            state.apply_leg((a.min(b) % columns) as usize, (a / columns) as usize, &gate)?;
        }
        _ => return Err("gate is outside the Sycamore ladder"),
    }
    Ok(())
}

fn checkpoint(
    round: usize,
    mps: &ColumnMps,
    oracle: &DenseOracle,
    app: &QuantumState,
    mps_heap: &BackendHeap,
    app_heap: &BackendHeap,
) -> Result<(), Box<dyn Error>> {
    // Dense contraction is solely for validation, outside all memory measurements.
    let vector = mps.statevector();
    assert_eq!(vector.len(), 1 << WIRES);
    assert_eq!(vector.len(), oracle.amplitudes.len());
    assert_eq!(vector.len(), app.state_vector.len());
    let (mut max_f64, mut max_app, mut squared_error) = (0.0_f64, 0.0_f64, 0.0);
    for ((&actual, reference), app_amp) in vector
        .iter()
        .zip(&oracle.amplitudes)
        .zip(app.state_vector.iter())
    {
        let diff = actual - reference;
        let app_diff = actual - Complex64::new(f64::from(app_amp.re), f64::from(app_amp.im));
        if !diff.is_finite() || !app_diff.is_finite() {
            return Err(format!("Round {round} produced a nonfinite amplitude").into());
        }
        squared_error += diff.norm_sqr();
        max_f64 = max_f64.max(diff.norm());
        max_app = max_app.max(app_diff.norm());
    }
    let l2 = squared_error.sqrt();
    if l2 > 1e-8 || max_app > 2e-5 {
        return Err(format!(
            "Round {round} disagrees with dense evolution: f64 L2 {l2:e}, f32 max {max_app:e}"
        )
        .into());
    }
    let bonds = mps.bonds();
    let max_bond = bonds.iter().max().unwrap();
    let cuts = bonds
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{round},{},{cuts},{max_bond},{},{},{},{},{},{max_f64:.9e},{l2:.9e},{max_app:.9e},{:.9e}",
        patch::layer(round),
        mps.state_bytes(),
        size_of_val(oracle.amplitudes.as_slice()),
        size_of_val(app.state_vector.as_slice()),
        mps_heap.peak,
        app_heap.peak,
        mps.discarded_weight_sum()
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    const USAGE: &str = "sycamore_mps [--caps CAPS | --readouts CAPS | --paired CAP | --adaptive BASE_CAP TENSOR_BUDGET_KIB | --cut-caps CAPS] [ROUNDS [NEW_CIRCUIT.json]]
Runs the 16-wire patch for 16 rounds by default and writes CSV to stdout.
CAPS is a comma-separated list. For --cut-caps supply seven positive ranks, left to right.
The adaptive budget covers retained complex tensors, in KiB.";
    let mut args = std::env::args().skip(1);
    let mut first = args.next();
    if matches!(first.as_deref(), Some("--help" | "-h")) {
        println!("{USAGE}");
        return Ok(());
    }
    let mode = match first.as_deref() {
        Some("--paired" | "--adaptive") => {
            let cap = args
                .next()
                .ok_or(USAGE)?
                .parse::<std::num::NonZeroUsize>()?
                .get();
            let policy = if first.as_deref() == Some("--adaptive") {
                let kib = args
                    .next()
                    .ok_or(USAGE)?
                    .parse::<std::num::NonZeroUsize>()?
                    .get();
                sweep::PairedPolicy::Adaptive {
                    base_cap: cap,
                    budget_bytes: kib
                        .checked_mul(1024)
                        .ok_or("tensor budget overflows bytes")?,
                }
            } else {
                sweep::PairedPolicy::Uniform(cap)
            };
            first = args.next();
            Mode::Paired(policy)
        }
        Some(option @ ("--caps" | "--readouts" | "--cut-caps")) => {
            let values = args.next().ok_or(USAGE)?;
            let mut caps = Vec::new();
            for value in values.split(',') {
                let cap = value.parse::<std::num::NonZeroUsize>()?.get();
                if option != "--cut-caps" && caps.contains(&cap) {
                    return Err(format!("Duplicate bond cap {cap}").into());
                }
                caps.push(cap);
            }
            let mode = match option {
                "--cut-caps" => {
                    if caps.len() != COLUMNS - 1 {
                        return Err(format!(
                            "Expected {} cut caps for {COLUMNS} columns",
                            COLUMNS - 1
                        )
                        .into());
                    }
                    budget::tensor_bound(&caps).ok_or("tensor storage bound overflows bytes")?;
                    Mode::Paired(sweep::PairedPolicy::Explicit(caps))
                }
                "--readouts" => Mode::Sweep(caps, sweep::Report::LocalReadouts),
                _ => Mode::Sweep(caps, sweep::Report::Fidelity),
            };
            first = args.next();
            mode
        }
        _ => Mode::Uncapped,
    };
    let rounds = first.as_deref().unwrap_or("16").parse::<usize>()?;
    let export = args.next();
    if args.next().is_some() {
        return Err(USAGE.into());
    }
    let mut circuit = patch::circuit(rounds)?;
    if let Some(path) = export {
        serde_json::to_writer(File::create_new(path)?, &CircuitData::from(circuit.clone()))?;
    }
    let errors = circuit.prepare_for_execution();
    if !errors.is_empty() {
        return Err(format!("Invalid patch: {errors:?}").into());
    }
    match mode {
        Mode::Sweep(caps, report) => return sweep::run(&circuit, &caps, report),
        Mode::Paired(policy) => return sweep::run_paired(&circuit, policy),
        Mode::Uncapped => {}
    }
    let (mut mps_heap, mut app_heap) = (BackendHeap::default(), BackendHeap::default());
    let mut mps = mps_heap.measure(|| ColumnMps::new(COLUMNS));
    let mut app = app_heap.measure(|| QuantumState::new(WIRES));
    let mut oracle = DenseOracle::new(WIRES);
    println!("round,layer,chi_1,chi_2,chi_3,chi_4,chi_5,chi_6,chi_7,max_bond,mps_bytes,dense_f64_bytes,dense_f32_bytes,mps_peak_heap_bytes,dense_f32_peak_heap_bytes,max_amp_err_f64,l2_err_f64,max_amp_err_f32,discarded_weight_sum");
    checkpoint(0, &mps, &oracle, &app, &mps_heap, &app_heap)?;
    for (i, section) in circuit.sections.iter().enumerate() {
        let steps = &circuit.steps[section.start - 1..section.end];
        mps_heap.measure(|| {
            steps
                .iter()
                .flatten()
                .try_for_each(|op| apply_to_mps(&mut mps, op))
        })?;
        app_heap.measure(|| {
            for op in steps.iter().flatten() {
                app.apply_operation(op, &circuit.registry);
            }
        });
        for op in steps.iter().flatten() {
            oracle.apply(op);
        }
        checkpoint(i + 1, &mps, &oracle, &app, &mps_heap, &app_heap)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn four_rounds_use_every_physical_coupler_once() {
        let circuit = patch::circuit(4).unwrap();
        let layout = circuit.layout.as_ref().unwrap();
        let mut used = BTreeSet::new();
        for (steps, expected_gates) in circuit.steps.chunks_exact(2).zip([8, 4, 6, 4]) {
            assert_eq!(steps[0].len(), WIRES);
            assert_eq!(steps[1].len(), expected_gates);
            let mut active_wires = BTreeSet::new();
            for op in &steps[1] {
                let edge = [op.qubits[0] as usize, op.qubits[1] as usize];
                assert!(edge.into_iter().all(|wire| active_wires.insert(wire)));
                assert!(used.insert(edge));
            }
        }
        assert_eq!(used, layout.links.iter().copied().collect());
    }

    #[test]
    fn first_sycamore_round_reproduces_dense_amplitudes_and_cut_ranks() {
        let circuit = patch::circuit(1).unwrap();
        let mut state = ColumnMps::new(COLUMNS);
        let mut reference = DenseOracle::new(WIRES);
        for op in circuit.steps.iter().flatten() {
            apply_to_mps(&mut state, op).unwrap();
            reference.apply(op);
        }
        assert_eq!(state.bonds(), [4, 1, 4, 1, 4, 1, 4]);
        for (actual, expected) in state.statevector().iter().zip(&reference.amplitudes) {
            assert!((actual - expected).norm() < 1e-12);
        }
    }
}
