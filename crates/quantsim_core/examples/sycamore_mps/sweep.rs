use super::{
    apply_to_mps, budget, ideal_matrix, layers, patch, planning, readouts::LocalReadouts,
    BackendHeap, ColumnMps, DenseOracle, COLUMNS, WIRES,
};
use num_complex::Complex64;
use quantsim_core::circuit::Circuit;
use std::{error::Error, mem::size_of_val, time::Instant};

/// Compare full complex states, including relative phases, in the same basis order.
fn overlap(mps: &[Complex64], dense: &[Complex64]) -> Result<(f64, f64, f64), &'static str> {
    assert_eq!(mps.len(), dense.len());
    let (mut inner, mut mps_norm_sq, mut dense_norm_sq) = (Complex64::ZERO, 0.0, 0.0);
    for (&a, &b) in mps.iter().zip(dense) {
        inner += b.conj() * a;
        mps_norm_sq += a.norm_sqr();
        dense_norm_sq += b.norm_sqr();
    }
    let fidelity = inner.norm_sqr() / (mps_norm_sq * dense_norm_sq);
    if !(0.0..=1.0 + 1e-10).contains(&fidelity)
        || (mps_norm_sq - 1.0).abs() > 1e-8
        || (dense_norm_sq - 1.0).abs() > 1e-8
    {
        return Err("nonfinite, unnormalized, or inconsistent states in fidelity comparison");
    }
    Ok((fidelity.min(1.0), mps_norm_sq, dense_norm_sq))
}

pub enum Report {
    Fidelity,
    LocalReadouts,
}

pub enum PairedPolicy {
    Uniform(usize),
    Adaptive {
        base_cap: usize,
        budget_bytes: usize,
    },
    Explicit(Vec<usize>),
}

pub fn run_paired(circuit: &Circuit, policy: PairedPolicy) -> Result<(), Box<dyn Error>> {
    let mut pilot_heap = BackendHeap::default();
    let (mut pilot_ns, mut pilot_peak_tensor_bytes, mut pilot_loss) = (0, 0, Vec::new());
    let (label, base_cap, tensor_budget, caps) = match policy {
        PairedPolicy::Uniform(cap) => ("paired", Some(cap), None, budget::base_caps(COLUMNS, cap)?),
        PairedPolicy::Explicit(caps) => ("cut-caps", None, None, caps),
        PairedPolicy::Adaptive {
            base_cap,
            budget_bytes,
        } => {
            let start = Instant::now();
            let plan = pilot_heap.measure(|| {
                planning::calibrate(COLUMNS, base_cap, budget_bytes, &circuit.steps, |op| {
                    ideal_matrix(&op.id)
                })
            })?;
            pilot_ns = start.elapsed().as_nanos();
            pilot_peak_tensor_bytes = plan.pilot_peak_tensor_bytes;
            pilot_loss = plan.calibration_loss;
            ("adaptive", Some(base_cap), Some(budget_bytes), plan.caps)
        }
    };
    let bound = budget::tensor_bound(&caps).ok_or("tensor storage bound overflows bytes")?;
    let mut heap = BackendHeap::default();
    let start = Instant::now();
    let (state, peak_tensor_bytes) = heap.measure(|| -> Result<_, &'static str> {
        let mut state = ColumnMps::with_cut_caps(COLUMNS, &caps)?;
        let mut peak = state.state_bytes();
        for ops in &circuit.steps {
            layers::apply_layer(
                &mut state,
                COLUMNS,
                ops,
                |op| ideal_matrix(&op.id),
                |s| {
                    peak = peak.max(s.state_bytes());
                },
            )?;
        }
        Ok((state, peak))
    })?;
    let evolution_ns = start.elapsed().as_nanos();
    if peak_tensor_bytes > bound
        || tensor_budget.is_some_and(|bytes| bound.max(pilot_peak_tensor_bytes) > bytes)
    {
        return Err("observed or planned tensor storage exceeds the declared bound".into());
    }
    // Reference evolution and full-state contraction are outside the MPS measurements.
    let mut dense = DenseOracle::new(WIRES);
    for op in circuit.steps.iter().flatten() {
        dense.apply(op);
    }
    let (fidelity, _, _) = overlap(&state.statevector(), &dense.amplitudes)?;
    println!("policy,qubits,rounds,base_cap,tensor_budget_bytes,cut_caps,planned_tensor_bound_bytes,pilot_peak_tensor_bytes,peak_tensor_bytes,pilot_ns,evolution_ns,pilot_peak_heap_bytes,evolution_peak_heap_bytes,pilot_cap_loss_by_cut,discarded_weight_by_cut,forward_fidelity");
    println!(
        "{label},{WIRES},{},{},{},\"{}\",{bound},{pilot_peak_tensor_bytes},{peak_tensor_bytes},{pilot_ns},{evolution_ns},{},{},\"{}\",\"{}\",{fidelity:.15}",
        circuit.sections.len(),
        base_cap.map(|cap| cap.to_string()).unwrap_or_default(),
        tensor_budget.map(|bytes| bytes.to_string()).unwrap_or_default(),
        serde_json::to_string(&caps)?,
        pilot_heap.peak,
        heap.peak,
        serde_json::to_string(&pilot_loss)?,
        serde_json::to_string(state.discarded_weights())?,
    );
    Ok(())
}

pub fn run(circuit: &Circuit, caps: &[usize], report: Report) -> Result<(), Box<dyn Error>> {
    let mut dense_heap = BackendHeap::default();
    let mut dense = dense_heap.measure(|| DenseOracle::new(WIRES));
    let mut variants: Vec<_> = std::iter::once(None)
        .chain(caps.iter().copied().map(Some))
        .map(|cap| {
            let mut heap = BackendHeap::default();
            let state = heap.measure(|| match cap {
                Some(cap) => ColumnMps::with_bond_cap(COLUMNS, cap),
                None => ColumnMps::new(COLUMNS),
            });
            (cap, state, heap)
        })
        .collect();

    let links = match report {
        Report::Fidelity => {
            println!("round,layer,bond_cap,max_bond,mps_bytes,mps_peak_heap_bytes,dense_bytes,dense_peak_heap_bytes,fidelity,mps_norm_sq,dense_norm_sq");
            None
        }
        Report::LocalReadouts => {
            let layout = circuit.layout.as_ref().ok_or("missing patch couplers")?;
            println!("round,bond_cap,observable,wire_a,wire_b,dense,mps");
            Some(layout.links.as_slice())
        }
    };
    let rounds = circuit
        .sections
        .iter()
        .map(|section| &circuit.steps[section.start - 1..section.end]);
    for (round, steps) in std::iter::once(&[][..]).chain(rounds).enumerate() {
        if round > 0 {
            dense_heap.measure(|| {
                for op in steps.iter().flatten() {
                    dense.apply(op);
                }
            });
        }
        let readout_reference = links.map(|edges| {
            (
                edges,
                LocalReadouts::measure(&dense.amplitudes, WIRES, edges),
            )
        });
        for (cap, state, heap) in &mut variants {
            if round > 0 {
                heap.measure(|| {
                    steps
                        .iter()
                        .flatten()
                        .try_for_each(|op| apply_to_mps(state, op))
                })?;
            }
            // Dense contraction and overlaps are validation costs, outside both heap meters.
            let amplitudes = state.statevector();
            let (fidelity, mps_norm_sq, dense_norm_sq) = overlap(&amplitudes, &dense.amplitudes)?;
            let max_bond = state.bonds().into_iter().max().unwrap();
            if cap.is_none() && 1.0 - fidelity > 1e-10 {
                return Err(
                    format!("Uncapped control lost fidelity in round {round}: {fidelity}").into(),
                );
            }
            if cap.is_some_and(|limit| max_bond > limit) {
                return Err(format!(
                    "Bond dimension {max_bond} exceeded cap {cap:?} in round {round}"
                )
                .into());
            }
            let label = cap.map_or_else(|| "uncapped".to_string(), |n| n.to_string());
            if let Some((edges, exact)) = &readout_reference {
                LocalReadouts::measure(&amplitudes, WIRES, edges).write_comparison(
                    &mut std::io::stdout().lock(),
                    exact,
                    edges,
                    round,
                    &label,
                )?;
            } else {
                println!(
                    "{round},{},{label},{max_bond},{},{},{},{},{fidelity:.15},{mps_norm_sq:.15},{dense_norm_sq:.15}",
                    patch::layer(round), state.state_bytes(), heap.peak,
                    size_of_val(dense.amplitudes.as_slice()), dense_heap.peak,
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fidelity_distinguishes_relative_phase_and_ignores_global_phase() {
        let a = Complex64::from(std::f64::consts::FRAC_1_SQRT_2);
        let exact = [a, a];
        assert!(overlap(&[a, -a], &exact).unwrap().0 < 1e-14);
        assert!((overlap(&[a * Complex64::I; 2], &exact).unwrap().0 - 1.0).abs() < 1e-14);
        assert!(overlap(&[a, Complex64::ZERO], &exact).is_err());
    }

    #[test]
    fn cap_16_matches_independent_truncated_svd_at_first_loss() {
        let circuit = patch::circuit(5).unwrap();
        let mut state = ColumnMps::with_bond_cap(COLUMNS, 16);
        let mut dense = DenseOracle::new(WIRES);
        for (i, steps) in circuit.steps.chunks_exact(2).enumerate() {
            for op in steps.iter().flatten() {
                apply_to_mps(&mut state, op).unwrap();
                dense.apply(op);
            }
            let (fidelity, _, _) = overlap(&state.statevector(), &dense.amplitudes).unwrap();
            // Independent LAPACK SVD after each gate in the exported circuit.
            let expected = if i < 4 { 1.0 } else { 0.403402570692482 };
            assert!(
                (fidelity - expected).abs() < 1e-10,
                "round {}: {fidelity}",
                i + 1
            );
            assert!(state.bonds().iter().all(|&bond| bond <= 16));
        }
    }
}
