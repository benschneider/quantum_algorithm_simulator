//! Native publication benchmark. Each measurement or correctness run is a fresh process.
#[path = "sycamore_mps/budget.rs"]
mod budget;
#[path = "audit_benchmark/heap.rs"]
mod heap;
#[path = "sycamore_mps/layers.rs"]
mod layers;
#[path = "audit_benchmark/mps.rs"]
#[allow(dead_code)]
mod mps;
#[path = "sycamore_mps/planning.rs"]
mod planning;
#[path = "audit_benchmark/reference.rs"]
mod reference;
#[path = "audit_benchmark/workloads.rs"]
mod workloads;

use mps::ColumnMps;
use num_complex::{Complex32, Complex64 as C};
use num_traits::ToPrimitive;
use quantsim_core::{
    circuit::{Circuit, RunOptions},
    grouped::GroupedState,
    product_state::ProductState,
    stabilizer::{StabilizerAmplitudes, StabilizerState},
    types::{Gate, Operation},
    BasisIndex,
};
use serde_json::{json, Value};
use std::{error::Error, time::Instant};
use workloads::Workload;

enum State {
    Product(ProductState),
    Tableau(StabilizerState),
    Grouped(GroupedState),
    Dense(Vec<Complex32>),
    Mps(ColumnMps),
}

fn simulate(
    workload: &Workload,
    backend: &str,
    cap: usize,
    tensor_budget: Option<usize>,
) -> Result<(State, Value), Box<dyn Error>> {
    let circuit = &workload.circuit;
    let depth = circuit.steps.len();
    let n = circuit.num_qubits;
    let compact_error = |e: quantsim_core::circuit::ValidationError| {
        format!(
            "column {} gate {}: {}",
            e.step + 1,
            e.op_index + 1,
            e.message
        )
    };
    let (state, model) = match backend {
        "product" => (
            State::Product(circuit.run_product_prefix(depth).map_err(compact_error)?),
            json!({"estimated_peak_state_bytes":32*n,"largest_dense_subsystem":0}),
        ),
        "tableau" => {
            let state = circuit
                .run_stabilizer_prefix(depth)
                .map_err(compact_error)?;
            let bytes = 2 * n * (2 * 8 * n.div_ceil(64) + 1);
            (
                State::Tableau(state),
                json!({"estimated_peak_state_bytes":bytes,"largest_dense_subsystem":0}),
            )
        }
        "grouped" => {
            let state = circuit.run_grouped_prefix(depth).map_err(compact_error)?;
            let model = json!({
                "estimated_peak_state_bytes":16*workload.estimated_group_entries,
                "largest_dense_subsystem":if workload.estimated_group_width == 1 {0} else {workload.estimated_group_width},
                "final_largest_factor":state.largest_group_size(),
            });
            (State::Grouped(state), model)
        }
        "dense" => {
            let result = circuit.run(&RunOptions::default());
            if !result.diagnostics.is_empty() {
                return Err(format!("{:?}", result.diagnostics).into());
            }
            // Returning the probability vector is within the measured API
            // call. Its ownership can be dropped before correctness work.
            (
                State::Dense(result.final_state_vector.ok_or("missing state")?),
                json!({"estimated_peak_state_bytes":12*(1usize<<n),"largest_dense_subsystem":n,"final_largest_factor":n}),
            )
        }
        "mps" => {
            let cols = n / 2;
            let mut state = ColumnMps::with_bond_cap(cols, cap);
            let mut bytes = state.state_bytes();
            let mut bond = 1;
            for (i, column) in circuit.steps.iter().enumerate() {
                for op in column {
                    let mut matrix = reference::matrix(op);
                    if i >= workload.forward_depth {
                        matrix = matrix.adjoint();
                    }
                    match *op.qubits.as_slice() {
                        [q] => {
                            let q = q as usize;
                            state.apply_one(q % cols, q / cols, &matrix);
                        }
                        [a, b] if a as usize % cols == b as usize % cols => {
                            state.apply_rung(a as usize % cols, &matrix)
                        }
                        [a, b] if b == a + 1 && a as usize / cols == b as usize / cols => {
                            state.apply_leg(a as usize % cols, a as usize / cols, &matrix)?;
                        }
                        _ => return Err("invalid ladder gate".into()),
                    }
                    bytes = bytes.max(state.state_bytes());
                    bond = bond.max(state.max_bond());
                }
            }
            let model = json!({
                "estimated_peak_state_bytes":bytes,"largest_dense_subsystem":0,
                "peak_bond":bond, "discarded_weight_sum":state.discarded_weight_sum(),
            });
            (State::Mps(state), model)
        }
        "mps-paired" | "mps-adaptive" => paired_ladder(workload, cap, tensor_budget)?,
        _ => return Err(format!("unknown backend {backend}").into()),
    };
    Ok((state, model))
}

fn paired_ladder(
    workload: &Workload,
    cap: usize,
    tensor_budget: Option<usize>,
) -> Result<(State, Value), Box<dyn Error>> {
    let circuit = &workload.circuit;
    let columns = circuit.num_qubits / 2;
    let start = Instant::now();
    let plan = tensor_budget
        .map(|bytes| {
            planning::calibrate(
                columns,
                cap,
                bytes,
                &circuit.steps[..workload.forward_depth],
                reference::matrix,
            )
        })
        .transpose()?;
    let calibration_ns = if plan.is_some() {
        start.elapsed().as_nanos() as u64
    } else {
        0
    };
    let start = Instant::now();
    let fixed_caps;
    let caps = match &plan {
        Some(p) => &p.caps,
        None => {
            fixed_caps = budget::base_caps(columns, cap)?;
            &fixed_caps
        }
    };
    let mut state = ColumnMps::with_cut_caps(columns, caps)?;
    let mut retained = state.state_bytes();
    let mut peak_bond = 1;
    for (i, step) in circuit.steps.iter().enumerate() {
        layers::apply_layer(
            &mut state,
            columns,
            step,
            |op| {
                let matrix = reference::matrix(op);
                if i < workload.forward_depth {
                    matrix
                } else {
                    matrix.adjoint()
                }
            },
            |s| {
                retained = retained.max(s.state_bytes());
                peak_bond = peak_bond.max(s.max_bond());
            },
        )?;
    }
    let evolution_ns = start.elapsed().as_nanos() as u64;
    let peak = plan
        .as_ref()
        .map_or(retained, |p| retained.max(p.pilot_peak_tensor_bytes));
    let model = json!({
        "estimated_peak_state_bytes":peak, "largest_dense_subsystem":0, "peak_bond":peak_bond,
        "tensor_budget_bytes":tensor_budget, "planned_tensor_bound_bytes":budget::tensor_bound(caps),
        "calibration_ns":calibration_ns, "evolution_ns":evolution_ns, "cut_caps":caps,
        "calibration_loss":plan.as_ref().map(|p| &p.calibration_loss),
        "discarded_weight_sum":state.discarded_weight_sum(),
        "discarded_weight_by_cut":state.discarded_weights()
    });
    Ok((State::Mps(state), model))
}

fn verify(workload: &Workload, state: &State, name: &str, backend: &str, cap: usize) -> Value {
    let n = workload.circuit.num_qubits;
    let tableau: Option<StabilizerAmplitudes> = match state {
        State::Tableau(state) => Some(state.amplitude_view()),
        _ => None,
    };
    let mps_vector = match state {
        State::Mps(state) if n <= 16 => Some(state.statevector()),
        _ => None,
    };
    let amplitude = |index: &BasisIndex| -> C {
        match state {
            State::Product(s) => s.amplitude_wide(index).unwrap(),
            State::Tableau(_) => tableau.as_ref().unwrap().amplitude_wide(index).unwrap(),
            State::Grouped(s) => s.amplitude_wide(index).unwrap(),
            State::Dense(s) => {
                let a = s[index.to_usize().unwrap()];
                C::new(f64::from(a.re), f64::from(a.im))
            }
            State::Mps(s) => match &mps_vector {
                Some(v) => v[index.to_usize().unwrap()],
                None => {
                    assert_eq!(index.bits(), 0);
                    s.zero_amplitude()
                }
            },
        }
    };
    let mut out = json!({"validation_status":"pass"});
    let mut pass = true;
    let tolerance = match backend {
        "dense" => 3e-5,
        "grouped" => 2e-7,
        _ => 2e-10,
    };
    if n <= 16 {
        let expected = reference::dense(workload, name);
        let actual: Vec<_> = (0..expected.len())
            .map(|i| amplitude(&BasisIndex::from(i)))
            .collect();
        let reference_norm = expected.iter().map(C::norm_sqr).sum::<f64>();
        let norm = actual.iter().map(C::norm_sqr).sum::<f64>();
        let inner: C = expected
            .iter()
            .zip(&actual)
            .map(|(e, a)| e.conj() * a)
            .sum();
        let phase = if inner.norm() > 0.0 {
            inner / inner.norm()
        } else {
            C::ONE
        };
        let errors = expected
            .iter()
            .zip(&actual)
            .map(|(e, a)| (a - e * phase).norm());
        let max = errors.clone().fold(0.0, f64::max);
        let l2 = errors.map(|e| e * e).sum::<f64>().sqrt();
        out["dense_reference"] = json!("independent_f64");
        let projection = inner / reference_norm;
        let orthogonal_weight = expected
            .iter()
            .zip(&actual)
            .map(|(e, a)| (a - e * projection).norm_sqr())
            .sum::<f64>();
        out["fidelity"] = json!(1.0 - orthogonal_weight / norm);
        out["norm_error"] = json!((norm - 1.0).abs());
        out["max_abs_amplitude_error"] = json!(max);
        out["l2_amplitude_error"] = json!(l2);
        pass &= (matches!(state, State::Mps(_)) && cap != 0 || l2 <= tolerance)
            && (norm - 1.0).abs() <= tolerance;
    }
    if name != "mps_ladder" {
        let probes = reference::probes(n, &workload.target);
        let scale = if workload.inverse {
            1.0
        } else {
            reference::analytical(name, workload, &BasisIndex::default()).norm()
        };
        let mut max: f64 = 0.0;
        // A tableau fixes a global phase convention. For basis-state
        // returns, the target probability is invariant under global phase.
        for i in &probes {
            let a = amplitude(i);
            let e = reference::analytical(name, workload, i);
            let error = if workload.inverse {
                (a.norm() - e.norm()).abs()
            } else {
                (a - e).norm()
            };
            pass &= error.is_finite();
            max = max.max(error / scale);
        }
        out["analytical_probe_count"] = json!(probes.len());
        out["max_scaled_analytical_error"] = json!(max);
        pass &= max <= tolerance;
        if let State::Grouped(grouped) = state {
            let observed_peak = if workload.inverse {
                workload
                    .circuit
                    .run_grouped_prefix(workload.forward_depth)
                    .unwrap()
                    .largest_group_size()
            } else {
                grouped.largest_group_size()
            };
            out["validated_peak_factor_width"] = json!(observed_peak);
            pass &= observed_peak == workload.estimated_group_width;
        }
    } else if n > 16 && !workload.inverse {
        out["validation_status"] = json!("norm_only");
    }
    if let State::Mps(s) = state {
        let norm = s.norm_squared_at_center();
        out["norm_error"] = json!((norm - 1.0).abs());
        pass &= (norm - 1.0).abs() <= tolerance;
    }

    if workload.inverse {
        let p = amplitude(&workload.target).norm_sqr();
        out["return_probability"] = json!(p);
        if cap == 0 {
            pass &= (1.0 - p).abs() <= tolerance;
        }
    }
    if !pass {
        out["validation_status"] = json!("failed");
    } else if matches!(state, State::Mps(_)) && cap != 0 && out["validation_status"] == "pass" {
        out["validation_status"] = json!("approximation_measured");
    }
    out
}

fn issues() {
    let mut circuit = Circuit::new(2);
    circuit.initial_state = Some(vec![
        Complex32::ONE,
        Complex32::new(5e-7, 0.0),
        Complex32::ZERO,
        Complex32::ZERO,
    ]);
    circuit.steps = vec![vec![Operation::new(Gate::Z, vec![0], vec![])]];
    let state = circuit
        .run(&RunOptions::default())
        .final_state_vector
        .unwrap();
    let wide = workloads::build("qft", 32, "reverse");
    let product_error = wide
        .circuit
        .run_product_prefix(wide.circuit.steps.len())
        .err();
    let mut blocks = Circuit::new(4);
    blocks.steps = vec![
        vec![
            Operation::new(Gate::H, vec![0], vec![]),
            Operation::new(Gate::H, vec![2], vec![]),
        ],
        vec![
            Operation::new(Gate::CX, vec![0, 1], vec![]),
            Operation::new(Gate::CX, vec![2, 3], vec![]),
        ],
        vec![Operation::new(Gate::CZ, vec![1, 2], vec![])],
        vec![Operation::new(Gate::CZ, vec![1, 2], vec![])],
    ];
    let block_before = blocks.run_grouped_prefix(2).unwrap();
    let block_after = blocks.run_grouped_prefix(4).unwrap();
    let block_diff = (0..16)
        .map(|i| (block_after.amplitude(i).unwrap() - block_before.amplitude(i).unwrap()).norm())
        .fold(0.0_f64, f64::max);
    println!(
        "{}",
        json!({
            "two_bell_factors_before_bridge":block_before.largest_group_size(),
            "two_bell_factors_after_bridge_and_inverse":block_after.largest_group_size(),
            "two_bell_factors_max_amplitude_difference":block_diff,
            "dense_input_tiny_amplitude":5e-7,"dense_expected_tiny_amplitude_after_z":-5e-7,
            "dense_observed_tiny_amplitude_after_z":state[1].re,
            "qft32_product_inverse_error":product_error.as_ref().map(|e| &e.message),
            "qft32_product_inverse_first_rejected_column":product_error.as_ref().map(|e| e.step+1),
        })
    );
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.as_slice() == ["issues"] {
        issues();
        return Ok(());
    }
    let [mode, name, width, backend, variant, cap, extra @ ..] = args.as_slice() else {
        return Err(
            "usage: audit_benchmark <measure|verify> FAMILY QUBITS BACKEND VARIANT BOND_CAP [TENSOR_BUDGET_KiB]".into(),
        );
    };
    let n = width.parse::<usize>()?;
    let cap = cap.parse::<usize>()?;
    let tensor_budget = match (backend.as_str(), extra) {
        ("mps-adaptive", [kib]) if cap > 0 => Some(
            kib.parse::<usize>()?
                .checked_mul(1024)
                .ok_or("budget overflows usize")?,
        ),
        ("mps-adaptive", _) => {
            return Err("mps-adaptive takes a positive base cap and a tensor budget in KiB".into())
        }
        (_, []) => None,
        _ => return Err("only mps-adaptive accepts a tensor budget".into()),
    };
    if matches!(backend.as_str(), "mps" | "mps-paired" | "mps-adaptive")
        && (name != "mps_ladder"
            || n < 2
            || n % 2 != 0
            || !matches!(variant.as_str(), "forward" | "reverse"))
    {
        return Err(
            "MPS requires an even-width two-row mps_ladder and forward or reverse evolution".into(),
        );
    }
    let workload = workloads::build(name, n, variant);
    let steps = &workload.circuit.steps;
    let prep_gates = usize::from(name == "qft") * 2;
    let setup_peak = heap::peak();
    let baseline = heap::start();
    let start = Instant::now();
    let result = simulate(
        &workload,
        backend,
        if cap == 0 { usize::MAX } else { cap },
        tensor_budget,
    );
    let elapsed = start.elapsed();
    let peak = heap::peak();
    let (state, mut output) = match result {
        Ok(result) => result,
        Err(err) => {
            println!(
                "{}",
                json!({"status":"backend_error","detail":err.to_string()})
            );
            return Ok(());
        }
    };
    if mode == "verify" {
        println!("{}", verify(&workload, &state, name, backend, cap));
    } else if mode == "measure" {
        output["status"] = json!("ok");
        output["gates"] = json!(steps.iter().map(Vec::len).sum::<usize>());
        output["depth"] = json!(steps.iter().filter(|c| !c.is_empty()).count());
        output["authored_columns"] = json!(steps.len());
        output["input_prep_gates"] = json!(prep_gates);
        output["elapsed_ns"] = json!(elapsed.as_nanos() as u64);
        output["peak_live_heap_bytes"] = json!(peak);
        output["process_peak_live_heap_bytes"] = json!(peak.max(setup_peak));
        output["pre_run_live_heap_bytes"] = json!(baseline);
        output["peak_live_heap_above_baseline_bytes"] = json!(peak - baseline);
        println!("{output}");
    } else {
        return Err("expected measure or verify".into());
    }
    Ok(())
}
