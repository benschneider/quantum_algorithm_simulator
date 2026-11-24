use num_complex::Complex64;
use quantsim_core::circuit::Circuit;
use quantsim_core::types::{Gate, Operation, QubitPrep};
use quantsim_core::CircuitBuilder;

#[test]
fn amplitudes_and_marginals_match_dense_evolution_at_each_prefix() {
    let mut builder = CircuitBuilder::new(5);
    builder.h(4).h(1).x(2).next_step();
    builder
        .cx(4, 0)
        .add_gate(Gate::CY, &[1, 3], &[])
        .next_step();
    builder
        .rz(0, 0.37)
        .add_gate(Gate::Ry, &[3], &[-0.81])
        .next_step();
    builder.swap(4, 3).next_step();
    builder.add_gate(Gate::CZPow, &[0, 4], &[0.25]).next_step();
    builder.crz(3, 1, 0.51).next_step();
    builder.ccnot(3, 0, 2).next_step();
    builder.add_gate(Gate::CXPow, &[2, 1], &[-0.7]).next_step();
    builder.add_gate(Gate::SYC, &[4, 1], &[]).next_step();
    builder.swap(0, 2).next_step();
    let circuit = builder.build();
    for prefix in 0..=circuit.steps.len() {
        let groups = circuit.run_grouped_prefix(prefix).unwrap();
        let dense = circuit.run_prefix(prefix, &Default::default());
        assert!(dense.diagnostics.is_empty());
        let vector = dense.final_state_vector.unwrap();
        for (index, expected) in vector.iter().enumerate() {
            let expected = Complex64::new(expected.re.into(), expected.im.into());
            let actual = groups.amplitude(index as u128).unwrap();
            assert!(
                (actual - expected).norm() < 2e-6,
                "prefix {prefix}, index {index}: {actual} vs {expected}"
            );
        }
        for wire in 0..circuit.num_qubits {
            let expected: f32 = vector
                .iter()
                .enumerate()
                .filter(|(index, _)| index & (1 << wire) != 0)
                .map(|(_, z)| z.norm_sqr())
                .sum();
            let actual = groups.probability_one(wire).unwrap();
            assert!(
                (actual - expected).abs() < 2e-6,
                "prefix {prefix}, wire {wire}"
            );
        }
        assert!(groups.amplitude(32).is_none());
        assert!(groups.probability_one(5).is_none());
    }
}

#[test]
fn controlled_y_uses_the_authored_control_for_definite_and_entangled_inputs() {
    for (control, target) in [(0, 2), (2, 0)] {
        for prep in [QubitPrep::One, QubitPrep::Plus] {
            let mut circuit = Circuit::new(3);
            circuit.initial_preps[control as usize] = Some(prep);
            circuit.steps.push(vec![Operation::new(
                Gate::CY,
                vec![control, target],
                vec![],
            )]);
            let groups = circuit.run_grouped_prefix(1).unwrap();
            let dense = circuit.run(&Default::default()).final_state_vector.unwrap();
            let half = std::f64::consts::FRAC_1_SQRT_2;
            for (index, z) in dense.iter().enumerate() {
                let expected = match (prep, index) {
                    (QubitPrep::One, 5) => Complex64::I,
                    (QubitPrep::Plus, 0) => Complex64::from(half),
                    (QubitPrep::Plus, 5) => Complex64::I * half,
                    _ => Complex64::ZERO,
                };
                let dense_value = Complex64::new(z.re.into(), z.im.into());
                assert!(
                    (dense_value - expected).norm() < 1e-6,
                    "dense q{control} → q{target}, {prep:?}, {index}"
                );
                assert!(
                    (groups.amplitude(index as u128).unwrap() - expected).norm() < 1e-6,
                    "grouped q{control} → q{target}, {prep:?}, {index}"
                );
            }
        }
    }
}

#[test]
fn swapped_factors_sample_correlated_bits_and_find_the_lowest_physical_index() {
    let mut builder = CircuitBuilder::new(128);
    builder
        .add_gate(Gate::Ry, &[127], &[std::f32::consts::FRAC_PI_3])
        .h(65)
        .next_step();
    builder.cx(127, 64).cx(65, 63).next_step();
    builder.y(64).next_step();
    builder.swap(64, 65);
    let circuit = builder.build();
    let groups = circuit.run_grouped_prefix(circuit.steps.len()).unwrap();

    // q65 and q127 now have opposite values. q63 and q64 are a separate Bell pair.
    let bit = |wire: u32| 1u128 << wire;
    let bell = bit(63) | bit(64);
    let outcomes = [bit(65), bit(65) | bell, bit(127), bit(127) | bell];
    assert_eq!(groups.largest_group_size(), 2);
    assert_eq!(groups.basis_slice(), (bit(65), bit(63)));
    assert!((groups.reference_phase() - Complex64::I).norm() < 1e-12);
    let probabilities: [f64; 4] = [0.375, 0.375, 0.125, 0.125];
    for (index, p) in outcomes.into_iter().zip(probabilities) {
        let sign = if index & bit(127) == 0 { 1.0 } else { -1.0 };
        let expected = Complex64::I * (sign * p.sqrt());
        assert!((groups.amplitude(index).unwrap() - expected).norm() < 1e-7);
    }
    assert_eq!(groups.amplitude(u128::MAX), Some(Complex64::ZERO));
    let shots = groups.sample(4096, 200);
    assert_eq!(shots.len(), 4096);
    assert_eq!(shots, groups.sample(4096, 200));
    assert!(groups.sample(0, 200).is_empty());
    let mut counts = [0; 4];
    for shot in shots {
        counts[outcomes
            .iter()
            .position(|&index| index == shot)
            .expect("joint support")] += 1;
    }
    for (count, p) in counts.into_iter().zip(probabilities) {
        assert!((count as f64 / 4096.0 - p).abs() < 0.04);
    }
}

#[test]
fn two_eight_wire_groups_can_swap_then_a_ninth_entangled_wire_joins_one() {
    let mut builder = CircuitBuilder::new(128);
    builder.h(0).h(8).h(127).next_step();
    for target in 1..8 {
        builder.cx(0, target).cx(8, 8 + target).next_step();
    }
    builder.swap(0, 8).next_step();
    // q62 is |0⟩, so this controlled phase can execute without growing a group.
    builder.add_gate(Gate::CZPow, &[62, 1], &[0.25]).next_step();
    let mut circuit = builder.build();
    let before_last = circuit.steps.len();
    let valid = circuit.run_grouped_prefix(before_last).unwrap();
    assert_eq!(valid.largest_group_size(), 8);
    assert!((valid.amplitude(0).unwrap().norm_sqr() - 0.125).abs() < 1e-8);
    // Even a tiny nonzero phase is entangling here.
    for exponent in [0.25, 1e-11] {
        circuit.steps.push(vec![
            Operation::new(Gate::X, vec![61], vec![]),
            Operation::new(Gate::CZPow, vec![1, 127], vec![exponent]),
        ]);
        let merged = circuit.run_grouped_prefix(circuit.steps.len()).unwrap();
        assert_eq!(merged.group_wires(127).unwrap().len(), 9);
        assert_eq!(merged.group_wires(0).unwrap().len(), 8);
        let bits = (1u128 << 127) | (1u128 << 61) | 0b1_1111_1110;
        let expected =
            Complex64::from_polar(0.125_f64.sqrt(), std::f64::consts::PI * f64::from(exponent));
        assert!((merged.amplitude(bits).unwrap() - expected).norm() < 1e-12);
        assert!((merged.probability_one(127).unwrap() - 0.5).abs() < 1e-6);
        circuit.steps.pop();
    }

    // After the SWAP, q8 is the GHZ control for q1..q7. Undo its last CX
    // so the same controlled phase can join q127 without needing a ninth wire.
    circuit
        .steps
        .push(vec![Operation::new(Gate::CX, vec![8, 7], vec![])]);
    let split = circuit.run_grouped_prefix(circuit.steps.len()).unwrap();
    assert_eq!(split.group_wires(7), Some(&[7][..]));
    assert_eq!(split.group_wires(8).unwrap().len(), 7);
    circuit
        .steps
        .push(vec![Operation::new(Gate::CZPow, vec![1, 127], vec![0.25])]);
    let joined = circuit.run_grouped_prefix(circuit.steps.len()).unwrap();
    assert_eq!(joined.group_wires(127).unwrap().len(), 8);
    assert_eq!(joined.probability_one(7), Some(0.0));
}

#[test]
fn twelve_entangled_wires_match_all_dense_amplitudes() {
    let mut circuit = Circuit::new(12);
    circuit.initial_preps.fill(Some(QubitPrep::Plus));
    for wire in 1..12 {
        circuit.steps.push(vec![Operation::new(
            Gate::CZPow,
            vec![wire - 1, wire],
            vec![0.25],
        )]);
    }
    let grouped = circuit.run_grouped_prefix(circuit.steps.len()).unwrap();
    assert_eq!(grouped.largest_group_size(), 12);
    let dense = circuit.run(&Default::default());
    assert!(dense.diagnostics.is_empty());
    for (i, value) in dense.final_state_vector.unwrap().into_iter().enumerate() {
        let expected = Complex64::new(value.re.into(), value.im.into());
        assert!((grouped.amplitude(i as u128).unwrap() - expected).norm() < 2e-6);
    }
}

#[test]
fn cancelling_a_controlled_phase_releases_two_superposed_wires_with_their_phase() {
    let mut builder = CircuitBuilder::new(128);
    builder.x(127).next_step().y(127).next_step(); // -i |0>
    builder.h(127).h(62).next_step();
    builder.add_gate(Gate::SqrtZ, &[62], &[]).next_step();
    builder.cz(127, 62).next_step().cz(62, 127);
    let circuit = builder.build();
    let last = circuit.steps.len();
    let coupled = circuit.run_grouped_prefix(last - 1).unwrap();
    assert_eq!(coupled.group_wires(127).unwrap().len(), 2);
    let split = circuit.run_grouped_prefix(last).unwrap();
    assert_eq!(split.group_wires(127), Some(&[127][..]));
    assert_eq!(split.group_wires(62), Some(&[62][..]));
    assert_eq!(split.group_wires(128), None);
    for high in [0, 1u128 << 127] {
        assert!((split.amplitude(high).unwrap() + Complex64::I * 0.5).norm() < 1e-12);
        assert!((split.amplitude(high | (1u128 << 62)).unwrap() - 0.5).norm() < 1e-12);
    }
}

#[test]
fn a_weak_controlled_phase_keeps_the_pair_joined() {
    let mut builder = CircuitBuilder::new(2);
    builder.h(0).h(1).next_step();
    let exponent = 1e-25_f32;
    builder.add_gate(Gate::CZPow, &[0, 1], &[exponent]);
    let circuit = builder.build();
    let grouped = circuit.run_grouped_prefix(circuit.steps.len()).unwrap();
    assert_eq!(grouped.group_wires(0).unwrap().len(), 2);
    let expected_im = 0.5 * (std::f64::consts::PI * f64::from(exponent)).sin();
    assert!((grouped.amplitude(3).unwrap().im / expected_im - 1.0).abs() < 1e-12);
    assert_eq!(grouped.amplitude(1).unwrap().im, 0.0);
}

#[test]
fn reference_phase_survives_underflow_across_entangled_and_product_factors() {
    let mut circuit = Circuit::new(64);
    circuit.steps = vec![
        (0..62)
            .map(|wire| Operation::new(Gate::Ry, vec![wire], vec![1e-7]))
            .collect(),
        (0..62)
            .map(|wire| Operation::new(Gate::X, vec![wire], vec![]))
            .collect(),
        vec![Operation::new(Gate::H, vec![63], vec![])],
        vec![Operation::new(Gate::CX, vec![63, 62], vec![])],
        vec![Operation::new(Gate::Y, vec![62], vec![])],
    ];
    let groups = circuit.run_grouped_prefix(circuit.steps.len()).unwrap();
    let first = 1u128 << 62;
    assert_eq!(groups.basis_slice(), (first, 1));
    assert_eq!(groups.amplitude(first), Some(Complex64::ZERO));
    assert!((groups.reference_phase() - Complex64::I).norm() < 1e-8);
}

#[test]
fn high_wire_anticorrelated_factor_keeps_both_outcomes_and_their_phases() {
    use quantsim_core::BasisIndex;
    let mut builder = CircuitBuilder::new(257);
    builder
        .h(256)
        .next_step()
        .cx(256, 2)
        .next_step()
        .y(2)
        .next_step()
        .swap(2, 128);
    let circuit = builder.build();
    let state = circuit.run_grouped_prefix(circuit.steps.len()).unwrap();
    let low = BasisIndex::from(1u8) << 128usize;
    let high = BasisIndex::from(1u8) << 256usize;
    assert_eq!(state.basis_slice_wide(), (low.clone(), low.clone()));
    assert_eq!(state.probability_one(128), Some(0.5));
    assert_eq!(state.probability_one(256), Some(0.5));
    assert!((state.reference_phase() - Complex64::I).norm() < 1e-12);
    let a = Complex64::I * std::f64::consts::FRAC_1_SQRT_2;
    assert!((state.amplitude_wide(&low).unwrap() - a).norm() < 1e-12);
    assert!((state.amplitude_wide(&high).unwrap() + a).norm() < 1e-12);
    assert_eq!(state.amplitude_wide(&(&low | &high)), Some(Complex64::ZERO));
    assert_eq!(state.amplitude(0), Some(Complex64::ZERO));
    assert_eq!(
        state.amplitude_wide(&(BasisIndex::from(1u8) << 257usize)),
        None
    );
    let shots = state.sample_wide(128, 42);
    assert!(shots.contains(&low) && shots.contains(&high));
    assert!(shots.iter().all(|v| v == &low || v == &high));
}

#[test]
fn streaming_factor_samples_match_small_cdf_search_for_identical_draws() {
    use quantsim_core::BasisIndex;
    let mut builder = CircuitBuilder::new(6);
    builder.add_gate(Gate::Ry, &[0], &[0.9]).next_step();
    for wire in 1..6 {
        builder.cx(0, wire).next_step();
    }
    builder.x(0);
    let circuit = builder.build();
    let state = circuit.run_grouped_prefix(circuit.steps.len()).unwrap();
    assert_eq!(state.largest_group_size(), 6);
    // Seventeen draws scan the 64-entry factor; 128 use its CDF.
    let few = state.sample(17, 42);
    let many = state.sample(128, 42);
    assert_eq!(few, many[..17]);
    assert!(many.contains(&1) && many.contains(&62));
    assert!(many.iter().all(|&value| value == 1 || value == 62));
    assert_eq!(
        state.sample_wide(17, 42),
        few.into_iter().map(BasisIndex::from).collect::<Vec<_>>()
    );
}
