use nalgebra::Complex;
use nalgebra::DVector;
use quantsim_core::core::circuit::Circuit;
use quantsim_core::core::gates::Gate;
use quantsim_core::core::types::{Operation, Param};
use quantsim_ui::app::QCSimApp;
use quantsim_ui::messages::Message;

const TOLERANCE: f32 = 1e-4;

fn run_test_simulation(circuit: Circuit, expected_state: DVector<Complex<f32>>) {
    let mut app = QCSimApp::default();

    app.state.circuit_state.circuit = circuit;
    app.dispatch(Message::RunSimulation);

    // The egui update loop is not run in tests, so we manually process messages.
    let messages: Vec<Message> = app.messages.drain(..).collect();
    for message in messages {
        app.handle_message(message);
    }

    let final_state = app
        .state
        .simulation_state
        .quantum_state
        .as_ref()
        .unwrap()
        .state_vector
        .clone();

    for (actual, expected) in final_state.iter().zip(expected_state.iter()) {
        assert!((actual.re - expected.re).abs() < TOLERANCE);
        assert!((actual.im - expected.im).abs() < TOLERANCE);
    }
}

#[test]
fn test_hadamard_gate() {
    let mut circuit = Circuit::new(1);
    circuit
        .steps
        .push(vec![Operation::new(Gate::H, vec![0], vec![])]);

    let expected_state = DVector::from_vec(vec![
        Complex::new(1.0 / 2.0f32.sqrt(), 0.0),
        Complex::new(1.0 / 2.0f32.sqrt(), 0.0),
    ]);

    run_test_simulation(circuit, expected_state);
}

#[test]
fn test_cnot_gate() {
    let mut circuit = Circuit::new(2);
    circuit
        .steps
        .push(vec![Operation::new(Gate::X, vec![0], vec![])]);
    circuit
        .steps
        .push(vec![Operation::new(Gate::CX, vec![0, 1], vec![])]);

    let expected_state = DVector::from_vec(vec![
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(1.0, 0.0),
    ]);

    run_test_simulation(circuit, expected_state);
}

#[test]
fn test_pauli_z_gate() {
    let mut circuit = Circuit::new(1);
    circuit
        .steps
        .push(vec![Operation::new(Gate::X, vec![0], vec![])]);
    circuit
        .steps
        .push(vec![Operation::new(Gate::Z, vec![0], vec![])]);

    let expected_state = DVector::from_vec(vec![Complex::new(0.0, 0.0), Complex::new(-1.0, 0.0)]);

    run_test_simulation(circuit, expected_state);
}

#[test]
fn test_sqrt_y_gate() {
    let mut circuit = Circuit::new(1);
    circuit.steps.push(vec![Operation {
        id: Gate::SqrtY,
        name: None,
        params: vec![].into(),
        qubits: vec![0].into(),
        matrix: None,
    }]);

    let expected_state = DVector::from_vec(vec![Complex::new(0.5, 0.5), Complex::new(0.5, 0.5)]);

    run_test_simulation(circuit, expected_state);
}

#[test]
fn test_sqrt_z_gate() {
    let mut circuit = Circuit::new(1);
    circuit.steps.push(vec![Operation {
        id: Gate::SqrtZ,
        name: None,
        params: vec![].into(),
        qubits: vec![0].into(),
        matrix: None,
    }]);

    let expected_state = DVector::from_vec(vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)]);

    run_test_simulation(circuit, expected_state);
}

#[test]
fn test_cy_gate() {
    let mut circuit = Circuit::new(2);
    circuit
        .steps
        .push(vec![Operation::new(Gate::X, vec![0], vec![])]);
    circuit
        .steps
        .push(vec![Operation::new(Gate::CY, vec![0, 1], vec![])]);

    let expected_state = DVector::from_vec(vec![
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 1.0),
    ]);

    run_test_simulation(circuit, expected_state);
}

#[test]
fn test_y_pow_gate() {
    let mut circuit = Circuit::new(1);
    circuit.steps.push(vec![Operation {
        id: Gate::YPow,
        name: None,
        params: vec![Param::Scalar(0.5)].into(),
        qubits: vec![0].into(),
        matrix: None,
    }]);

    let expected_state = DVector::from_vec(vec![Complex::new(0.5, 0.5), Complex::new(0.5, 0.5)]);

    run_test_simulation(circuit, expected_state);
}
