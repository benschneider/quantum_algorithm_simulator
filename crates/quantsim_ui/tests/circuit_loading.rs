use quantsim_core::circuits;
use quantsim_core::core::gates::Gate;
use quantsim_ui::{messages::Message, QCSimApp};

/// Processes all pending messages in the app's queue until it's empty.
///
/// This is a test helper to simulate the egui update loop, ensuring that
/// all cascaded messages are handled before assertions are made.
fn process_messages(app: &mut QCSimApp) {
    // In a real egui app, this is handled by the update loop. For testing,
    // we need to manually drain the message queue. We loop because handlers
    // can generate new messages.
    while !app.messages.is_empty() {
        let messages_to_process: Vec<Message> = app.drain_messages();
        for msg in messages_to_process {
            app.handle_message(msg);
        }
    }
}

#[test]
fn shorter_json_can_render_on_the_load_frame() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate("qft4.json".into()));
    app.handle_message(Message::RunSimulation);
    app.handle_message(Message::SelectGateForEditing(0, 12));
    let bell = circuits::get_circuit("bell.json").unwrap();
    app.dispatch(Message::UpdateCircuitFromJson(bell));

    let ctx = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();
    let _ = ctx.run(Default::default(), |ctx| {
        eframe::App::update(&mut app, ctx, &mut frame);
    });

    assert_eq!(app.state.circuit_state.num_timesteps, 2);
    assert_eq!(app.state.initial_state_editor_state.num_qubits, 2);
    assert_eq!(app.state.ui_state.current_timestep, 0);
    assert!(app.state.ui_state.selected_gate_for_editing.is_none());
    assert!(app.state.simulation_state.quantum_state.is_none());
    app.handle_message(Message::RunSimulation);
    let probabilities = app
        .state
        .simulation_state
        .quantum_state
        .unwrap()
        .probabilities();
    assert!((probabilities[0] - 0.5).abs() < 1e-6);
    assert!((probabilities[3] - 0.5).abs() < 1e-6);
}

#[test]
fn template_replaces_the_json_draft_and_previous_results() {
    let mut app = QCSimApp::default();
    let json = r#"{"num_qubits":1,"steps":[[]],"initial_state":[[0,0],[1,0]]}"#;
    app.handle_message(Message::UpdateCircuitFromJson(json.into()));
    assert_eq!(app.state.initial_state_editor_state.state_vector[1].re, 1.0);
    app.handle_message(Message::RunSimulation);

    app.handle_message(Message::LoadTemplate("bell.json".into()));

    assert_eq!(
        app.state.ui_state.circuit_json_string,
        circuits::get_circuit("bell.json").unwrap()
    );
    assert_eq!(app.state.ui_state.current_timestep, 0);
    assert!(app.state.simulation_state.quantum_state.is_none());
    assert_eq!(app.state.initial_state_editor_state.state_vector.len(), 4);
    assert_eq!(app.state.initial_state_editor_state.state_vector[0].re, 1.0);
}

#[test]
fn rejected_json_keeps_the_last_working_circuit_and_results() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate("bell.json".into()));
    app.handle_message(Message::RunSimulation);

    for invalid in [
        "{",
        r#"{"num_qubits":129,"steps":[]}"#,
        r#"{"num_qubits":2,"steps":[],"initial_state":[[1,0]]}"#,
    ] {
        app.handle_message(Message::UpdateCircuitFromJson(invalid.into()));
        assert!(app.state.ui_state.error_message.is_some());
        assert_eq!(app.state.circuit_state.circuit.steps[1][0].id, Gate::CX);
        assert!(app.state.simulation_state.quantum_state.is_some());
    }
    app.handle_message(Message::LoadTemplate("ghz.json".into()));
    assert!(app.state.ui_state.error_message.is_none());
    assert_eq!(app.state.circuit_state.num_qubits, 3);
}

#[test]
fn wide_imports_and_editor_actions_keep_input_storage_bounded() {
    use quantsim_core::types::QubitPrep;

    let mut app = QCSimApp::default();
    app.handle_message(Message::ToggleInitialStateEditor);
    let json = r#"{"num_qubits":64,"initial_preps":["Plus"],"steps":[[{"id":"X","qubits":[63]}],[{"id":"Rz","qubits":[0],"params":[0.1]}]]}"#;
    app.handle_message(Message::UpdateCircuitFromJson(json.into()));
    assert!(app.state.ui_state.error_message.is_none());
    assert_eq!(
        app.state.circuit_state.circuit.initial_preps[0],
        Some(QubitPrep::Plus)
    );
    assert_eq!(app.state.initial_state_editor_state.num_qubits, 64);
    assert_eq!(
        app.state.initial_state_editor_state.state_vector.capacity(),
        0
    );

    let ctx = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();
    let _ = ctx.run(Default::default(), |ctx| {
        eframe::App::update(&mut app, ctx, &mut frame);
    });
    app.handle_message(Message::ApplyInitialStateFromEditor(vec![]));
    assert!(app
        .state
        .ui_state
        .error_message
        .as_ref()
        .unwrap()
        .contains("initial_preps"));
    assert!(app.state.circuit_state.circuit.initial_state.is_none());
    app.handle_message(Message::ResetInitialState);
    app.handle_message(Message::ToggleInitialStateEditor);
    assert!(app.state.ui_state.show_initial_state_editor);
    assert_eq!(
        app.state.initial_state_editor_state.state_vector.capacity(),
        0
    );

    app.handle_message(Message::UpdateCircuitFromJson(
        r#"{"num_qubits":50,"initial_state":[[1,0]],"steps":[]}"#.into(),
    ));
    assert!(app
        .state
        .ui_state
        .error_message
        .as_ref()
        .unwrap()
        .contains("initial_preps"));
    assert_eq!(app.state.circuit_state.circuit.num_qubits, 64);
    assert_eq!(app.state.circuit_state.circuit.steps.len(), 2);
}

#[test]
fn width_changes_preserve_explicit_input_and_validate_both_dimension_messages() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::UpdateCircuitFromJson(
        r#"{"num_qubits":1,"initial_state":[[0,0],[1,0]],"steps":[[]]}"#.into(),
    ));
    for width in [0, 50, 129] {
        app.handle_message(Message::ChangeQubits(width));
        assert!(app.state.ui_state.error_message.is_some());
        assert_eq!(app.state.circuit_state.num_qubits, 1);
        assert_eq!(
            app.state
                .circuit_state
                .circuit
                .initial_state
                .as_ref()
                .unwrap()[1]
                .re,
            1.0
        );
    }
    app.handle_message(Message::CircuitDimensionsChanged {
        num_qubits: 129,
        num_timesteps: 8,
    });
    assert_eq!(app.state.circuit_state.num_timesteps, 1);

    app.handle_message(Message::ResetInitialState);
    app.handle_message(Message::ChangeQubits(50));
    assert_eq!(app.state.circuit_state.circuit.initial_preps.len(), 50);
    assert_eq!(
        app.state.initial_state_editor_state.state_vector.capacity(),
        0
    );
    app.handle_message(Message::CircuitDimensionsChanged {
        num_qubits: 128,
        num_timesteps: 2,
    });
    assert_eq!(app.state.initial_state_editor_state.num_qubits, 128);
    assert_eq!(
        app.state.initial_state_editor_state.state_vector.capacity(),
        0
    );
    app.handle_message(Message::ChangeQubits(2));
    app.handle_message(Message::ToggleInitialStateEditor);
    assert_eq!(app.state.initial_state_editor_state.state_vector.len(), 4);
}

#[test]
fn test_load_bell_circuit() {
    let _ = env_logger::try_init();
    let mut app = QCSimApp::default();

    let bell_json = circuits::get_circuit("bell.json").unwrap();
    app.state.ui_state.circuit_json_string = bell_json.to_string();

    app.handle_message(Message::UpdateCircuitFromJson(bell_json.to_string()));
    process_messages(&mut app);

    let circuit = &app.state.circuit_state.circuit;

    assert_eq!(circuit.num_qubits, 2);
    assert_eq!(circuit.steps.len(), 2);

    let step0 = &circuit.steps[0];
    assert_eq!(step0.len(), 1);
    assert_eq!(step0[0].id, Gate::H);

    let step1 = &circuit.steps[1];
    assert_eq!(step1.len(), 1);
    assert_eq!(step1[0].id, Gate::CX);
}

#[test]
fn test_load_grover_algorithm_circuit() {
    let _ = env_logger::try_init();
    let mut app = QCSimApp::default();

    let grover_json = circuits::get_circuit("grover_algorithm.json").unwrap();
    app.state.ui_state.circuit_json_string = grover_json.to_string();

    app.handle_message(Message::UpdateCircuitFromJson(grover_json.to_string()));
    process_messages(&mut app);

    let circuit = &app.state.circuit_state.circuit;

    assert_eq!(circuit.num_qubits, 3);
    assert_eq!(circuit.steps.len(), 13);

    assert_eq!(circuit.steps[0][0].id, Gate::H);
    assert_eq!(circuit.steps[1][0].id, Gate::Custom);
    assert_eq!(circuit.steps[4][0].id, Gate::CCZ);
}

#[test]
fn test_load_deutsch_algorithm_circuit() {
    let _ = env_logger::try_init();
    let mut app = QCSimApp::default();

    let deutsch_json = circuits::get_circuit("deutsch_algorithm.json").unwrap();
    app.state.ui_state.circuit_json_string = deutsch_json.to_string();

    app.handle_message(Message::UpdateCircuitFromJson(deutsch_json.to_string()));
    process_messages(&mut app);

    let circuit = &app.state.circuit_state.circuit;

    assert_eq!(circuit.num_qubits, 2);
    assert_eq!(circuit.steps.len(), 4);

    assert_eq!(circuit.steps[0][0].id, Gate::X);
    assert_eq!(circuit.steps[1][0].id, Gate::H);
    assert_eq!(circuit.steps[1][1].id, Gate::H);
    assert_eq!(circuit.steps[2][0].id, Gate::CX);
    assert_eq!(circuit.steps[3][0].id, Gate::H);
}

#[test]
fn test_load_quantum_teleportation_circuit() {
    let _ = env_logger::try_init();
    let mut app = QCSimApp::default();

    let teleportation_json = circuits::get_circuit("quantum_teleportation.json").unwrap();
    app.state.ui_state.circuit_json_string = teleportation_json.to_string();

    app.handle_message(Message::UpdateCircuitFromJson(
        teleportation_json.to_string(),
    ));
    process_messages(&mut app);

    let circuit = &app.state.circuit_state.circuit;

    assert_eq!(circuit.num_qubits, 3);
    assert_eq!(circuit.steps.len(), 8);

    assert_eq!(circuit.steps[0][0].id, Gate::H);
    assert_eq!(circuit.steps[1][0].id, Gate::CX);
    assert_eq!(circuit.steps[2][0].id, Gate::Ry);
    assert_eq!(circuit.steps[3][0].id, Gate::SqrtZ);
    assert_eq!(circuit.steps[4][0].id, Gate::CX);
    assert_eq!(circuit.steps[5][0].id, Gate::H);
    assert_eq!(circuit.steps[6][0].id, Gate::CX);
    assert_eq!(circuit.steps[7][0].id, Gate::CZ);
}

#[test]
fn test_load_bell_test_complex_circuit() {
    let _ = env_logger::try_init();
    let mut app = QCSimApp::default();

    let bell_test_complex_json = circuits::get_circuit("bell_test_complex.json").unwrap();
    app.state.ui_state.circuit_json_string = bell_test_complex_json.to_string();

    app.handle_message(Message::UpdateCircuitFromJson(
        bell_test_complex_json.to_string(),
    ));
    process_messages(&mut app);

    let circuit = &app.state.circuit_state.circuit;

    println!(
        "About to assert. circuit.num_qubits = {}",
        circuit.num_qubits
    );
    assert_eq!(circuit.num_qubits, 4);
    assert_eq!(circuit.steps.len(), 5);

    let step0 = &circuit.steps[0];
    assert_eq!(step0.len(), 1);
    assert_eq!(step0[0].id, Gate::H);

    let step1 = &circuit.steps[1];
    assert_eq!(step1.len(), 1);
    assert_eq!(step1[0].id, Gate::CX);

    let step2 = &circuit.steps[2];
    assert_eq!(step2.len(), 2);
    assert_eq!(step2[0].id, Gate::H);
    assert_eq!(step2[1].id, Gate::H);

    let step3 = &circuit.steps[3];
    assert_eq!(step3.len(), 1);
    assert_eq!(step3[0].id, Gate::XPow);

    let step4 = &circuit.steps[4];
    assert_eq!(step4.len(), 2);
    assert_eq!(step4[0].id, Gate::CXPow);
    assert_eq!(step4[1].id, Gate::CXPow);
}

#[test]
fn test_load_grover_circuit() {
    let _ = env_logger::try_init();
    let mut app = QCSimApp::default();

    let grover_json = circuits::get_circuit("grover.json").unwrap();
    app.state.ui_state.circuit_json_string = grover_json.to_string();

    app.handle_message(Message::UpdateCircuitFromJson(grover_json.to_string()));
    process_messages(&mut app);

    let circuit = &app.state.circuit_state.circuit;

    assert_eq!(circuit.num_qubits, 2);
    assert_eq!(circuit.steps.len(), 6);

    let step4 = &circuit.steps[4];
    assert_eq!(step4.len(), 1);
    assert_eq!(step4[0].id, Gate::CZ);
}
