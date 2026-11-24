use num_complex::Complex64;
use quantsim_core::circuit::Circuit;
use quantsim_core::circuits::load_template;
use quantsim_core::types::{Gate, Operation, QubitPrep};

#[test]
fn amplitudes_match_dense_prefixes_including_gate_phases() {
    use Gate::*;
    let op =
        |gate, wires: &[u32], params: &[f32]| Operation::new(gate, wires.to_vec(), params.to_vec());
    let mut circuit = Circuit::new(4);
    circuit.initial_preps = [
        QubitPrep::One,
        QubitPrep::Minus,
        QubitPrep::Plus,
        QubitPrep::One,
    ]
    .map(Some)
    .to_vec();
    circuit.steps = vec![
        // q2 is now a +Y eigenstate. The first rotation on each wire changes
        // a phase that its computational-basis probability cannot reveal.
        vec![op(SqrtZ, &[2], &[])],
        vec![
            op(Rz, &[0], &[0.2]),
            op(Rx, &[1], &[0.8]),
            op(Ry, &[2], &[-0.7]),
        ],
        vec![op(ZPow, &[0], &[0.375]), op(XPow, &[1], &[-0.3])],
        vec![op(Z, &[2], &[])],
        vec![op(YPow, &[2], &[0.125])],
        vec![op(CZPow, &[0, 3], &[-0.8])],
        // Test relative phase kickback on q2 from the |-⟩ target q1.
        vec![op(CX, &[2, 1], &[])],
        vec![op(SWAP, &[3, 2], &[]), op(Y, &[0], &[])],
        vec![op(H, &[0], &[]), op(SqrtY, &[3], &[])],
    ];
    for step in 0..=circuit.steps.len() {
        let product = circuit.run_product_prefix(step).unwrap();
        let dense = circuit.run_prefix(step, &Default::default());
        assert!(dense.diagnostics.is_empty());
        for (index, expected) in dense.final_state_vector.unwrap().into_iter().enumerate() {
            let actual = product.amplitude(index as u128).unwrap();
            assert!(
                (actual - Complex64::new(expected.re.into(), expected.im.into())).norm() < 1e-6,
                "step {step}, index {index}: {actual} vs {expected}"
            );
        }
        assert_eq!(product.amplitude(16), None);
    }
}

#[test]
fn selected_amplitudes_match_the_50_wire_fourier_transform() {
    let mut qft = load_template("qft_50").unwrap();
    let input = (1u128 << 45) | 3;
    qft.steps[0].clear(); // Supply the probe input directly.
    for (wire, prep) in qft.initial_preps.iter_mut().enumerate() {
        *prep = (input & (1 << wire) != 0).then_some(QubitPrep::One);
    }
    let state = qft.run_product_prefix(qft.steps.len()).unwrap();
    let size = 1u128 << 50;
    let magnitude = 2f64.powi(-25);
    let indices = (0..64).chain([1 << 32, 1 << 49, size - 1]);
    for index in indices {
        // Only the low 50 bits of the phase numerator enter the Fourier kernel.
        let phase = std::f64::consts::TAU * (input.wrapping_mul(index) % size) as f64 / size as f64;
        let expected = Complex64::from_polar(magnitude, phase);
        assert!((state.amplitude(index).unwrap() - expected).norm() < magnitude * 1e-12);
    }
    assert_eq!(state.amplitude(size), None);
    assert_eq!(state.basis_slice(), (0, 1));
}

#[test]
fn sparse_slices_use_every_address_bit() {
    for width in [64, 65, 128] {
        let mut circuit = Circuit::new(width);
        circuit.initial_preps.fill(Some(QubitPrep::One));
        let initial = circuit.run_product_prefix(0).unwrap();
        let last = u128::MAX >> (128 - width);
        assert_eq!(initial.amplitude(last), Some(Complex64::ONE));
        assert_eq!(initial.basis_slice(), (last, 1));
        assert_eq!(initial.sample(4, 42), [last; 4]);
        if let Some(outside) = last.checked_add(1) {
            assert_eq!(initial.amplitude(outside), None);
        }

        let top = (width - 1) as u32;
        circuit.steps = vec![vec![Operation::new(Gate::H, vec![top], vec![])]];
        let state = circuit.run_product_prefix(1).unwrap();
        let high = 1u128 << top;
        assert_eq!(state.basis_slice(), (high - 1, high));
        let a = std::f64::consts::FRAC_1_SQRT_2;
        assert!((state.amplitude(high - 1).unwrap() - a).norm() < 1e-15);
        assert!((state.amplitude(last).unwrap() + a).norm() < 1e-15);
        assert_eq!(state.amplitude(0), Some(Complex64::ZERO));
    }
}

#[test]
fn reference_phase_survives_joint_amplitude_underflow() {
    let mut circuit = Circuit::new(64);
    circuit.initial_preps[63] = Some(QubitPrep::One);
    circuit.steps = vec![
        (0..63)
            .map(|q| Operation::new(Gate::Ry, vec![q], vec![1e-7]))
            .collect(),
        (0..63)
            .map(|q| Operation::new(Gate::X, vec![q], vec![]))
            .collect(),
        vec![
            Operation::new(Gate::Rz, vec![0], vec![std::f32::consts::FRAC_PI_2]),
            Operation::new(Gate::ZPow, vec![63], vec![0.5]),
        ],
    ];
    let state = circuit.run_product_prefix(circuit.steps.len()).unwrap();
    let (first, _) = state.basis_slice();
    assert_eq!(first, 1u128 << 63);
    assert_eq!(state.amplitude(first), Some(Complex64::ZERO));
    let expected = Complex64::from_polar(1.0, std::f64::consts::FRAC_PI_4);
    assert!((state.reference_phase() - expected).norm() < 1e-7);
}
