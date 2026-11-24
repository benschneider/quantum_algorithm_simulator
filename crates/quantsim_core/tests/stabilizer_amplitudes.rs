use num_complex::Complex64;
use quantsim_core::circuit::Circuit;
use quantsim_core::types::{Gate, Operation, QubitPrep};
use rand::{rngs::StdRng, Rng, SeedableRng};

fn gate(id: Gate, wires: &[u32]) -> Operation {
    Operation::new(id, wires.to_vec(), vec![])
}

#[test]
fn all_indexed_amplitudes_match_dense_up_to_the_reference_phase() {
    let mut rng = StdRng::seed_from_u64(9517);
    for wires in 1..=6 {
        for _ in 0..4 {
            let mut circuit = Circuit::new(wires);
            let preps = [
                QubitPrep::Zero,
                QubitPrep::One,
                QubitPrep::Plus,
                QubitPrep::Minus,
            ];
            for prep in &mut circuit.initial_preps {
                *prep = Some(preps[rng.gen_range(0..preps.len())]);
            }
            let gates = [Gate::H, Gate::SqrtZ, Gate::X, Gate::Y, Gate::Z, Gate::CX];
            for _ in 0..30 {
                let q = rng.gen_range(0..wires as u32);
                let choice = rng.gen_range(0..if wires == 1 { 5 } else { 6 });
                let op = if gates[choice] == Gate::CX {
                    let t = (q + rng.gen_range(1..wires as u32)) % wires as u32;
                    gate(Gate::CX, &[q, t])
                } else {
                    gate(gates[choice].clone(), &[q])
                };
                circuit.steps.push(vec![op]);
            }
            for step in 0..=circuit.steps.len() {
                let dense = circuit.run_prefix(step, &Default::default());
                assert!(dense.diagnostics.is_empty());
                let state = dense.final_state_vector.unwrap();
                let view = circuit
                    .run_stabilizer_prefix(step)
                    .unwrap()
                    .amplitude_view();
                let reference = state.iter().position(|z| z.norm() > 1e-5).unwrap();
                assert_eq!(view.reference_index(), reference as u128);
                let phase = state[reference].conj() / state[reference].norm();
                let (origin, stride) = view.basis_slice();
                assert_eq!(origin, reference as u128);
                for (index, z) in state.iter().enumerate() {
                    let expected = *z * phase;
                    let expected = Complex64::new(expected.re as f64, expected.im as f64);
                    let actual = view.amplitude(index as u128).unwrap();
                    assert!(
                        (actual - expected).norm() < 1e-5,
                        "step {step}, index {index}"
                    );
                    if actual != Complex64::ZERO {
                        assert_eq!((index as u128 - origin) % stride, 0);
                    }
                }
                assert_eq!(view.amplitude(1u128 << wires), None);
            }
        }
    }
}

#[test]
fn imaginary_ghz_and_a_high_bit_slice_are_addressable() {
    let half = std::f64::consts::FRAC_1_SQRT_2;
    for (wires, ones) in [(64, u128::from(u64::MAX)), (128, u128::MAX)] {
        let high = wires as u32 - 1;
        let mut circuit = Circuit::new(wires);
        circuit.initial_preps[3] = Some(QubitPrep::One);
        circuit.steps.push(vec![gate(Gate::H, &[high])]);
        let pole = circuit.run_stabilizer_prefix(0).unwrap().amplitude_view();
        assert_eq!(pole.basis_slice(), (8, 1));
        assert_eq!(pole.amplitude(8), Some(Complex64::ONE));
        let pair = circuit.run_stabilizer_prefix(1).unwrap().amplitude_view();
        assert_eq!(pair.basis_slice(), (8, 1u128 << high));
        for q in 0..high {
            circuit.steps.push(vec![gate(Gate::CX, &[high, q])]);
        }
        circuit.steps.push(vec![gate(Gate::SqrtZ, &[high])]);
        let view = circuit
            .run_stabilizer_prefix(circuit.steps.len())
            .unwrap()
            .amplitude_view();
        assert_eq!(view.basis_slice(), (8, 1));
        assert!((view.amplitude(8).unwrap() - half).norm() < 1e-15);
        assert!((view.amplitude(ones ^ 8).unwrap() - Complex64::I * half).norm() < 1e-15);
        assert_eq!(view.amplitude(ones), Some(Complex64::ZERO));
    }
}

#[test]
fn full_rank_normalization_works_at_64_and_128_wires() {
    for (wires, ones) in [(64, u128::from(u64::MAX)), (128, u128::MAX)] {
        let mut circuit = Circuit::new(wires);
        circuit.initial_preps.fill(Some(QubitPrep::Minus));
        let view = circuit.run_stabilizer_prefix(0).unwrap().amplitude_view();
        let magnitude = 2.0_f64.powi(-(wires as i32) / 2);
        for index in [0, 1, 1 << (wires - 1), ones] {
            let sign = if index.count_ones() % 2 == 0 {
                1.0
            } else {
                -1.0
            };
            assert_eq!(
                view.amplitude(index),
                Some(Complex64::from(sign * magnitude))
            );
        }
    }
}
