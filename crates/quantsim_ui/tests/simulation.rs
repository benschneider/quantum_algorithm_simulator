use nalgebra::Complex;
use nalgebra::DVector;
use quantsim_core::core::circuit::Circuit;
use quantsim_core::core::gates::Gate;
use quantsim_core::core::types::{Operation, Param, QubitPrep};
use quantsim_ui::app::QCSimApp;
use quantsim_ui::messages::Message;

const TOLERANCE: f32 = 1e-4;

#[test]
fn scrubber_counts_visible_steps_from_the_prepared_input() {
    let mut app = QCSimApp::default();
    let mut circuit = Circuit::new(1);
    circuit.initial_preps = vec![Some(QubitPrep::Minus)];
    circuit.steps = vec![
        vec![Operation::new(Gate::H, vec![0], vec![])],
        vec![Operation::new(Gate::Z, vec![0], vec![])],
    ];
    app.state.circuit_state.circuit = circuit;

    let s = std::f32::consts::FRAC_1_SQRT_2;
    let re = |value| Complex::new(value, 0.0);
    let im = |value| Complex::new(0.0, value);
    let preparations = [
        (
            None,
            [[re(s), re(-s)], [re(0.0), re(1.0)], [re(0.0), re(-1.0)]],
        ),
        (
            Some(vec![im(1.0), im(0.0)]),
            [[im(1.0), im(0.0)], [im(s), im(s)], [im(s), im(-s)]],
        ),
    ];

    for (initial_state, expected) in preparations {
        app.state.circuit_state.circuit.initial_state = initial_state;
        for step in [0, 1, 2, 99] {
            app.handle_message(Message::SelectTimestep(step));
            let selected = step.min(2);
            assert_eq!(app.state.ui_state.current_timestep, selected);
            let state = app.state.simulation_state.quantum_state.as_ref().unwrap();
            for (amplitude, expected) in state.state_vector.iter().zip(expected[selected]) {
                assert!(
                    (*amplitude - expected).norm() < TOLERANCE,
                    "visible step {step}"
                );
            }
        }
    }
}

#[test]
fn scrubbing_a_valid_prefix_skips_a_later_invalid_gate() {
    let mut app = QCSimApp::default();
    let mut circuit = Circuit::new(1);
    circuit.steps = vec![
        vec![Operation::new(Gate::X, vec![0], vec![])],
        vec![Operation::new(Gate::X, vec![1], vec![])],
    ];
    app.state.circuit_state.circuit = circuit;
    app.handle_message(Message::SelectTimestep(1));

    let state = app.state.simulation_state.quantum_state.as_ref().unwrap();
    assert_eq!(state.state_vector[1], Complex::new(1.0, 0.0));
    app.handle_message(Message::RunSimulation);
    assert!(app
        .state
        .ui_state
        .error_message
        .as_deref()
        .unwrap()
        .contains("out of bounds"));
}

#[test]
fn scrubbing_preserves_the_results_page_until_the_state_has_fewer_entries() {
    let mut app = QCSimApp::default();
    let mut circuit = Circuit::new(5);
    circuit.steps = vec![
        vec![Operation::new(Gate::X, vec![0], vec![])],
        (0..5)
            .map(|qubit| Operation::new(Gate::H, vec![qubit], vec![]))
            .collect(),
        vec![Operation::new(Gate::Z, vec![0], vec![])],
    ];
    app.state.circuit_state.circuit = circuit;

    let ctx = egui::Context::default();
    let render_results = |app: &mut QCSimApp| {
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                quantsim_ui::components::results_panel::results_panel(ui, &mut app.state);
            });
        });
    };

    app.handle_message(Message::RunSimulation);
    render_results(&mut app);
    app.handle_message(Message::SimulationResultsPageChanged(2));
    app.handle_message(Message::SelectTimestep(2));
    render_results(&mut app);
    assert_eq!(app.state.simulation_state.pagination.current_page, 2);
    assert_eq!(app.state.simulation_state.page_input_text, "2");

    app.handle_message(Message::SelectTimestep(1));
    render_results(&mut app);
    assert_eq!(app.state.simulation_state.pagination.current_page, 1);
    assert_eq!(app.state.simulation_state.page_input_text, "1");
    assert_eq!(app.state.simulation_state.pagination.total_entries, 1);
}

#[test]
fn invalid_circuit_reports_diagnostics_on_run_and_scrub() {
    let json = r#"{"num_qubits":1,"steps":[[{"id":"X","qubits":[1]}]]}"#;
    for message in [Message::RunSimulation, Message::SelectTimestep(1)] {
        let mut app = QCSimApp::default();
        app.handle_message(Message::RunSimulation);
        assert!(app.state.simulation_state.quantum_state.is_some());

        let data = serde_json::from_str(json).unwrap();
        app.state.circuit_state.circuit.update_from_data(data);
        app.handle_message(message);

        assert!(app.state.simulation_state.quantum_state.is_none());
        assert!(app
            .state
            .ui_state
            .error_message
            .as_deref()
            .unwrap()
            .contains("out of bounds"));

        app.state.circuit_state.circuit.steps.clear();
        app.handle_message(Message::RunSimulation);
        assert!(app.state.ui_state.error_message.is_none());
        assert!(app.state.simulation_state.quantum_state.is_some());
    }
}

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
