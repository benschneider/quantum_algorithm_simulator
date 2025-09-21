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
fn test_load_bell_circuit() {
    let _ = env_logger::try_init();
    let mut app = QCSimApp::default();

    let bell_json = include_str!("../quantsim_core/circuits/bell.json");
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

    let grover_json = include_str!("../quantsim_core/circuits/grover_algorithm.json");
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

    let deutsch_json = include_str!("../quantsim_core/circuits/deutsch_algorithm.json");
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

    let teleportation_json = include_str!("../quantsim_core/circuits/quantum_teleportation.json");
    app.state.ui_state.circuit_json_string = teleportation_json.to_string();

    app.handle_message(Message::UpdateCircuitFromJson(
        teleportation_json.to_string(),
    ));
    process_messages(&mut app);

    let circuit = &app.state.circuit_state.circuit;

    assert_eq!(circuit.num_qubits, 3);
    assert_eq!(circuit.steps.len(), 6);

    assert_eq!(circuit.steps[0][0].id, Gate::H);
    assert_eq!(circuit.steps[1][0].id, Gate::CX);
    assert_eq!(circuit.steps[2][0].id, Gate::XPow);
    assert_eq!(circuit.steps[3][0].id, Gate::YPow);
    assert_eq!(circuit.steps[4][0].id, Gate::CX);
    assert_eq!(circuit.steps[5][0].id, Gate::H);
}

#[test]
fn test_load_bell_test_complex_circuit() {
    let _ = env_logger::try_init();
    let mut app = QCSimApp::default();

    let bell_test_complex_json = include_str!("../quantsim_core/circuits/bell_test_complex.json");
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
    assert_eq!(circuit.steps.len(), 3);

    let step0 = &circuit.steps[0];
    assert_eq!(step0.len(), 2);
    assert_eq!(step0[0].id, Gate::H);
    assert_eq!(step0[1].id, Gate::CX);

    let step1 = &circuit.steps[1];
    assert_eq!(step1.len(), 2);
    assert_eq!(step1[0].id, Gate::H);
    assert_eq!(step1[1].id, Gate::H);

    let step2 = &circuit.steps[2];
    assert_eq!(step2.len(), 3);
    assert_eq!(step2[0].id, Gate::XPow);
    assert_eq!(step2[1].id, Gate::CXPow);
    assert_eq!(step2[2].id, Gate::CXPow);
}

#[test]
fn test_load_grover_circuit() {
    let _ = env_logger::try_init();
    let mut app = QCSimApp::default();

    let grover_json = include_str!("../quantsim_core/circuits/grover.json");
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
