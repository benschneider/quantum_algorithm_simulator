use quantsim_core::circuit::Circuit;
use quantsim_core::circuits::load_template;
use quantsim_core::types::{Gate, Operation, QubitPrep};
use rand::{rngs::StdRng, Rng, SeedableRng};

fn op(gate: Gate, wires: &[u32]) -> Operation {
    Operation::new(gate, wires.to_vec(), vec![])
}

fn check_prefixes(circuit: &Circuit, label: &str) {
    for step in 0..=circuit.steps.len() {
        let dense = circuit.run_prefix(step, &Default::default());
        assert!(dense.diagnostics.is_empty(), "{label}, step {step}");
        let expected = &dense.final_probabilities;
        let state = circuit.run_stabilizer_prefix(step).unwrap();

        for wire in 0..circuit.num_qubits {
            let probability: f32 = expected
                .iter()
                .enumerate()
                .filter(|(index, _)| index & (1 << wire) != 0)
                .map(|(_, p)| p)
                .sum();
            let actual = state.probability_one(wire).unwrap();
            assert!(
                (actual - probability).abs() < 1e-5,
                "{label}, step {step}, q{wire}: {actual} vs {probability}"
            );
        }

        // An independent coin flip per wire can pass every marginal and still
        // produce impossible Bell or parity outcomes. Check the full register.
        let shots = 1024;
        let samples = state.sample(shots, 2026);
        assert_eq!(samples.len(), shots as usize);
        let mut counts = vec![0; expected.len()];
        for sample in samples {
            assert!(
                sample < expected.len() as u64,
                "{label}, step {step}, sample {sample}"
            );
            counts[sample as usize] += 1;
        }
        for (outcome, (&count, &probability)) in counts.iter().zip(expected).enumerate() {
            assert_eq!(
                count > 0,
                probability > 1e-6,
                "{label}, step {step}, outcome {outcome}"
            );
            let frequency = count as f32 / shots as f32;
            assert!(
                (frequency - probability).abs() < 0.1,
                "{label}, step {step}, outcome {outcome}: {frequency} vs {probability}"
            );
        }
    }
}

#[test]
fn bundled_circuit_prefixes_match_dense_results() {
    for name in ["bell", "ghz", "plaquette_z_check"] {
        check_prefixes(&load_template(name).unwrap(), name);
    }
}

#[test]
fn preparations_reversed_control_and_phase_interference() {
    use Gate::{SqrtZ, CX, H, X, Y, Z};
    use QubitPrep::{Minus, One, Plus, Zero};

    let mut circuit = Circuit::new(4);
    circuit.initial_preps = [Zero, One, Plus, Minus].map(Some).to_vec();
    circuit.steps = vec![
        vec![op(CX, &[1, 0]), op(H, &[2]), op(H, &[3])],
        vec![op(H, &[0]), op(SqrtZ, &[1])],
        vec![op(SqrtZ, &[0]), op(X, &[2]), op(Y, &[3])],
        vec![op(SqrtZ, &[0]), op(Z, &[1])],
        vec![op(H, &[0])],
    ];
    check_prefixes(&circuit, "phase interference");
    let first = circuit.run_stabilizer_prefix(1).unwrap();
    assert_eq!(first.sample(8, 0), [0b1011; 8]);
    let last = circuit.run_stabilizer_prefix(circuit.steps.len()).unwrap();
    assert_eq!(last.sample(8, 0), [0b0110; 8]);
}

#[test]
fn generated_clifford_circuits_match_dense_results() {
    use Gate::{SqrtZ, CX, H, X, Y, Z};
    let gates = [H, SqrtZ, CX, X, H, Y, CX, Z];
    for seed in 0..12 {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut circuit = Circuit::new(4);
        for _ in 0..20 {
            let gate = gates[rng.gen_range(0..gates.len())].clone();
            let wire = rng.gen_range(0..4);
            let operation = if gate == CX {
                let target = (wire + rng.gen_range(1..4)) % 4;
                op(gate, &[wire, target])
            } else {
                op(gate, &[wire])
            };
            circuit.steps.push(vec![operation]);
        }
        check_prefixes(&circuit, &format!("Clifford seed {seed}"));
    }
}

#[test]
fn sampling_is_seeded_and_preserves_the_state() {
    let circuit = load_template("bell").unwrap();
    let state = circuit.run_stabilizer_prefix(circuit.steps.len()).unwrap();
    let first = state.sample(64, 42);
    assert!(first.contains(&0b00) && first.contains(&0b11));
    assert_eq!(state.probability_one(0), Some(0.5));
    assert_eq!(state.probability_one(1), Some(0.5));
    assert_eq!(first, state.sample(64, 42));
    assert!(state.sample(0, 42).is_empty());
}

#[test]
fn four_bell_states_and_superdense_decoding_across_50_wires() {
    use Gate::{CX, H, X, Z};

    let high_bit = 1u64 << 49;
    for (phase, parity) in [(false, false), (false, true), (true, false), (true, true)] {
        let mut bell = Circuit::new(50);
        bell.steps = vec![vec![op(H, &[0])], vec![op(CX, &[0, 49])]];
        if phase {
            bell.steps.push(vec![op(Z, &[0])]);
        }
        if parity {
            bell.steps.push(vec![op(X, &[0])]);
        }

        // Z readout reveals the parity flip. X readout reveals the phase flip.
        let mut x_readout = bell.clone();
        x_readout.steps.push(vec![op(H, &[0]), op(H, &[49])]);
        for (basis, circuit, opposite) in [("Z", &bell, parity), ("X", &x_readout, phase)] {
            let state = circuit.run_stabilizer_prefix(circuit.steps.len()).unwrap();
            let label = format!("phase={phase}, parity={parity}, {basis} readout");
            for wire in 0..50 {
                let expected = if wire == 0 || wire == 49 { 0.5 } else { 0.0 };
                assert_eq!(
                    state.probability_one(wire),
                    Some(expected),
                    "{label}, q{wire}"
                );
            }
            let allowed = if opposite {
                [1, high_bit]
            } else {
                [0, high_bit | 1]
            };
            let samples = state.sample(64, 42);
            assert!(samples.iter().all(|s| allowed.contains(s)), "{label}");
            assert!(allowed.iter().all(|s| samples.contains(s)), "{label}");
        }

        bell.steps
            .extend([vec![op(CX, &[0, 49])], vec![op(H, &[0])]]);
        let decoded = bell.run_stabilizer_prefix(bell.steps.len()).unwrap();
        let message = u64::from(phase) | (u64::from(parity) << 49);
        assert_eq!(
            decoded.sample(16, 42),
            [message; 16],
            "phase={phase}, parity={parity}"
        );
    }
}
