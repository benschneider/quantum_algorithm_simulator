use nalgebra::Complex;
use quantsim_core::core::circuit::Circuit;
use quantsim_core::core::gates::Gate;
use quantsim_core::core::types::Operation;
use quantsim_ui::simulation;

#[test]
fn test_ccnot_gate() {
    let mut circuit = Circuit::new(3);
    circuit.initial_state = Some(vec![
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(1.0, 0.0),
        Complex::new(0.0, 0.0),
    ]);
    circuit.steps.push(vec![Operation {
        id: Gate::CCNOT,
        name: None,
        params: vec![].into(),
        qubits: vec![0, 1, 2].into(),
        matrix: None,
    }]);

    let mut sim = simulation::Simulation::new();
    sim.run_simulation(&mut circuit);

    let expected_state_vector = vec![
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(1.0, 0.0),
    ];

    let epsilon = 1e-5_f32;
    let state_vector = sim.state_vector.as_ref().unwrap();

    for i in 0..expected_state_vector.len() {
        let actual_re = state_vector[i].re;
        let actual_im = state_vector[i].im;
        let expected_re = expected_state_vector[i].re;
        let expected_im = expected_state_vector[i].im;

        assert!(
            (actual_re - expected_re).abs() < epsilon,
            "Real part mismatch at index {}, expected {}, got {}",
            i,
            expected_re,
            actual_re
        );
        assert!(
            (actual_im - expected_im).abs() < epsilon,
            "Imaginary part mismatch at index {}, expected {}, got {}",
            i,
            expected_im,
            actual_im
        );
    }
}
#[test]
fn test_ccz_gate() {
    let mut circuit = Circuit::new(3);
    circuit.initial_state = Some(vec![
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(1.0, 0.0),
    ]);
    circuit.steps.push(vec![Operation {
        id: Gate::CCZ,
        name: None,
        params: vec![].into(),
        qubits: vec![0, 1, 2].into(),
        matrix: None,
    }]);

    let mut sim = simulation::Simulation::new();
    sim.run_simulation(&mut circuit);

    let expected_state_vector = vec![
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(-1.0, 0.0),
    ];

    let epsilon = 1e-5_f32;
    let state_vector = sim.state_vector.as_ref().unwrap();

    for i in 0..expected_state_vector.len() {
        let actual_re = state_vector[i].re;
        let actual_im = state_vector[i].im;
        let expected_re = expected_state_vector[i].re;
        let expected_im = expected_state_vector[i].im;

        assert!(
            (actual_re - expected_re).abs() < epsilon,
            "Real part mismatch at index {}, expected {}, got {}",
            i,
            expected_re,
            actual_re
        );
        assert!(
            (actual_im - expected_im).abs() < epsilon,
            "Imaginary part mismatch at index {}, expected {}, got {}",
            i,
            expected_im,
            actual_im
        );
    }
}
#[test]
fn test_x_on_all_qubits() {
    let mut circuit = Circuit::new(3);
    circuit.initial_state = Some(vec![
        Complex::new(1.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
    ]);
    circuit.steps.push(vec![
        Operation {
            id: Gate::X,
            name: None,
            params: vec![].into(),
            qubits: vec![0].into(),
            matrix: None,
        },
        Operation {
            id: Gate::X,
            name: None,
            params: vec![].into(),
            qubits: vec![1].into(),
            matrix: None,
        },
        Operation {
            id: Gate::X,
            name: None,
            params: vec![].into(),
            qubits: vec![2].into(),
            matrix: None,
        },
    ]);

    let mut sim = simulation::Simulation::new();
    sim.run_simulation(&mut circuit);

    let expected_state_vector = vec![
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(1.0, 0.0),
    ];

    let epsilon = 1e-5_f32;
    let state_vector = sim.state_vector.as_ref().unwrap();

    for i in 0..expected_state_vector.len() {
        let actual_re = state_vector[i].re;
        let actual_im = state_vector[i].im;
        let expected_re = expected_state_vector[i].re;
        let expected_im = expected_state_vector[i].im;

        assert!(
            (actual_re - expected_re).abs() < epsilon,
            "Real part mismatch at index {}, expected {}, got {}",
            i,
            expected_re,
            actual_re
        );
        assert!(
            (actual_im - expected_im).abs() < epsilon,
            "Imaginary part mismatch at index {}, expected {}, got {}",
            i,
            expected_im,
            actual_im
        );
    }
}
