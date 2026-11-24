use quantsim_core::circuit::Circuit;
use quantsim_core::circuits::load_template;
use quantsim_core::types::{Gate, Operation, Param, QubitPrep};
use quantsim_core::CircuitBuilder;

fn one(gate: Gate, wire: u32) -> Operation {
    Operation::new(gate, vec![wire], vec![])
}

#[test]
fn changing_a_phase_on_the_full_50_wire_qft_changes_the_interference() {
    let mut qft = load_template("qft_50").unwrap();
    assert_eq!(qft.num_qubits, 50);
    let output = qft.run_product_prefix(qft.steps.len()).unwrap();
    for wire in 0..50 {
        assert!((output.probability_one(wire).unwrap() - 0.5).abs() < 1e-6);
    }

    // Check phases across the entire register on another basis input.
    let input = 0x2a9d_7210_9357b_u64;
    let mut probe = qft.clone();
    for (wire, prep) in probe.initial_preps.iter_mut().enumerate() {
        *prep = (input & (1 << wire) != 0).then_some(QubitPrep::One);
    }
    probe.steps.push((0..50).map(|q| one(Gate::H, q)).collect());
    let result = probe.run_product_prefix(probe.steps.len()).unwrap();
    for wire in 0..50 {
        let period = 1u64 << (50 - wire);
        let angle = std::f64::consts::TAU * (input % period) as f64 / period as f64;
        let expected = (1.0 - angle.cos()) / 2.0;
        assert!((f64::from(result.probability_one(wire).unwrap()) - expected).abs() < 1e-5);
    }

    // The tutorial adds H on q49 before the last SWAP sends its readout to q0.
    let before_swap = qft.steps.len() - 2;
    qft.steps[before_swap].push(one(Gate::H, 49));
    for (power, expected) in [(0.5, 0.5), (1.0 / 3.0, 0.25), (0.0, 0.0)] {
        let phase = qft
            .steps
            .iter_mut()
            .flatten()
            .find(|op| op.id == Gate::CZPow && op.qubits.as_slice() == [48, 49])
            .unwrap();
        phase.params[0] = Param::Scalar(power);
        let result = qft.run_product_prefix(qft.steps.len()).unwrap();
        assert!((result.probability_one(0).unwrap() - expected).abs() < 1e-6);
    }
}

#[test]
fn edited_qft4_matches_dense_x_and_y_readouts_on_every_basis_input() {
    let mut qft = load_template("qft4").unwrap();
    qft.steps[0].clear(); // Replace the template's input with each basis state below.
    qft.steps[2][0].params[0] = Param::Scalar(-0.375);
    qft.steps[3][0].params[0] = Param::Scalar(0.2);
    for input in 0..16 {
        for (wire, prep) in qft.initial_preps.iter_mut().enumerate() {
            *prep = Some(if input & (1 << wire) == 0 {
                QubitPrep::Zero
            } else {
                QubitPrep::One
            });
        }
        for probe_y in [false, true] {
            let mut circuit = qft.clone();
            if probe_y {
                circuit
                    .steps
                    .push((0..4).map(|q| one(Gate::SqrtZ, q)).collect());
            }
            circuit
                .steps
                .push((0..4).map(|q| one(Gate::H, q)).collect());
            let dense = circuit.run(&Default::default());
            assert!(dense.diagnostics.is_empty(), "{:?}", dense.diagnostics);
            let product = circuit.run_product_prefix(circuit.steps.len()).unwrap();
            for (outcome, expected) in dense.final_probabilities.iter().enumerate() {
                let mut probability = 1.0;
                for wire in 0..4 {
                    let p = product.probability_one(wire).unwrap();
                    probability *= if outcome & (1 << wire) == 0 {
                        1.0 - p
                    } else {
                        p
                    };
                }
                assert!(
                    (probability - expected).abs() < 1e-5,
                    "input={input}, probe_y={probe_y}, outcome={outcome}: {probability} vs {expected}"
                );
            }
        }
    }
}

#[test]
fn entanglement_is_rejected_at_the_authored_gate_even_if_tiny() {
    let mut circuit = Circuit::new(3);
    circuit.steps = vec![
        vec![one(Gate::H, 0), one(Gate::H, 1)],
        vec![
            one(Gate::X, 2),
            Operation::new(Gate::CZPow, vec![0, 1], vec![1e-11]),
        ],
    ];
    assert!(circuit.run_product_prefix(1).is_ok());
    let err = circuit.run_product_prefix(2).err().unwrap();
    assert_eq!((err.step, err.op_index), (1, 1));

    // The H–phase–H preparation makes q0 arbitrarily close to a basis state.
    // A controlled copy of its tiny nonzero component still entangles q0 and q1.
    let mut builder = CircuitBuilder::new(3);
    builder.x(2).h(0).next_step();
    builder.add_gate(Gate::CZPow, &[2, 0], &[1e-7]).next_step();
    builder.h(0).next_step().cx(0, 1);
    let almost_basis = builder.build();
    assert!(almost_basis.run_product_prefix(3).is_ok());
    let err = almost_basis.run_product_prefix(4).err().unwrap();
    assert_eq!((err.step, err.op_index), (3, 0));
}

#[test]
fn controlled_x_on_an_x_eigenstate_retains_its_kickback_phase() {
    for (target, expected) in [(QubitPrep::Plus, 0), (QubitPrep::Minus, 3)] {
        let mut circuit = Circuit::new(2);
        circuit.initial_preps = vec![Some(QubitPrep::Plus), Some(target)];
        circuit.steps = vec![
            vec![Operation::new(Gate::CX, vec![0, 1], vec![])],
            vec![one(Gate::H, 0), one(Gate::H, 1)],
        ];
        let state = circuit.run_product_prefix(2).unwrap();
        assert_eq!(state.sample(64, 42), [expected; 64]);
    }
}

#[test]
fn invalid_inputs_and_overridden_gates_cannot_yield_a_product_readout() {
    for width in [0, 65] {
        assert!(Circuit::new(width).run_product_prefix(0).is_err());
    }
    let mut circuit = Circuit::new(2);
    circuit.initial_state = Some(vec![0.0.into(), 1.0.into(), 0.0.into(), 0.0.into()]);
    assert!(circuit.run_product_prefix(0).is_err());
    circuit.initial_state = None;
    circuit.initial_preps.push(Some(QubitPrep::Plus));
    assert!(circuit.run_product_prefix(0).is_err());
    circuit.initial_preps.pop();
    circuit.steps.push(vec![one(Gate::H, 0), one(Gate::H, 1)]);

    // This operation's named SWAP is separable here. Its authored CZ matrix entangles.
    let mut overridden = Operation::new(Gate::SWAP, vec![0, 1], vec![]);
    overridden.matrix = circuit.registry.eval(&Gate::CZ, &[], &[0, 1]);
    assert!(overridden.matrix.is_some());
    for (ops, expected_index) in [
        (vec![overridden], 0),
        (vec![one(Gate::H, 2)], 0),
        (vec![Operation::new(Gate::H, vec![], vec![])], 0),
        (vec![one(Gate::X, 1), one(Gate::Z, 1)], 1),
    ] {
        circuit.steps.push(ops);
        assert!(circuit.run_product_prefix(1).is_ok());
        let err = circuit.run_product_prefix(2).err().unwrap();
        assert_eq!((err.step, err.op_index), (1, expected_index));
        circuit.steps.pop();
    }
    // Even a phase gate on |00⟩ must validate the authored parameter.
    circuit.steps = vec![vec![Operation::new(
        Gate::CZPow,
        vec![0, 1],
        vec![f32::NAN],
    )]];
    assert!(circuit.run_product_prefix(1).is_err());
}

#[test]
fn packed_samples_preserve_the_top_bit_and_independent_biased_outcomes() {
    let mut circuit = Circuit::new(64);
    circuit.initial_preps[63] = Some(QubitPrep::One);
    circuit.initial_preps[7] = Some(QubitPrep::Plus);
    circuit.initial_preps[0] = Some(QubitPrep::Plus);
    circuit.steps = vec![
        vec![Operation::new(Gate::CZPow, vec![63, 0], vec![1.0 / 3.0])],
        vec![one(Gate::H, 0)],
    ];
    let product = circuit.run_product_prefix(2).unwrap();
    assert_eq!(product.probability_one(64), None);
    let samples = product.sample(4096, 42);
    assert_eq!(samples.len(), 4096);
    assert_eq!(samples, product.sample(4096, 42));
    let mut counts = [0u32; 4];
    for sample in samples {
        assert_eq!(sample & !0x81, 1u64 << 63);
        let low_bits = (sample & 1) | ((sample >> 6) & 2);
        counts[low_bits as usize] += 1;
    }
    for (count, expected) in counts.into_iter().zip([0.375, 0.125, 0.375, 0.125]) {
        assert!((count as f64 / 4096.0 - expected).abs() < 0.04);
    }
}
