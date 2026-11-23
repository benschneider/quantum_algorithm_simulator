use quantsim_ui::{app::QCSimApp, messages::Message, state::app_state::AppState};

#[test]
fn test_circuit_to_json() {
    let mut app = QCSimApp {
        state: AppState::new(),
        messages: Vec::new(),
    };
    app.handle_message(Message::LoadTemplate("bell.json".to_string()));

    app.handle_message(Message::UpdateJsonFromCircuit);

    assert!(!app.state.ui_state.circuit_json_string.is_empty());

    let circuit_data: quantsim_core::core::circuit::CircuitData =
        serde_json::from_str(&app.state.ui_state.circuit_json_string).unwrap();
    let original_circuit_data: quantsim_core::core::circuit::CircuitData =
        serde_json::from_str(include_str!("../../quantsim_core/circuits/bell.json")).unwrap();

    assert_eq!(circuit_data.num_qubits, original_circuit_data.num_qubits);
    assert_eq!(circuit_data.steps, original_circuit_data.steps);
}

#[test]
fn test_json_to_circuit() {
    let mut app = QCSimApp {
        state: AppState::new(),
        messages: Vec::new(),
    };
    let bell_json = include_str!("../../quantsim_core/circuits/bell.json");
    app.state.ui_state.circuit_json_string = bell_json.to_string();

    app.handle_message(Message::UpdateCircuitFromJson(bell_json.to_string()));

    let original_circuit_data: quantsim_core::core::circuit::CircuitData =
        serde_json::from_str(bell_json).unwrap();

    assert_eq!(
        app.state.circuit_state.circuit.num_qubits,
        original_circuit_data.num_qubits
    );
    assert_eq!(
        app.state.circuit_state.circuit.steps,
        original_circuit_data.steps
    );
}

#[test]
fn wire_annotations_survive_json_export_and_follow_wire_resizing() {
    use quantsim_core::circuit::QubitRole::{Ancilla, Data};

    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate("bell.json".into()));
    app.handle_message(Message::SetQubitRole {
        row: 0,
        role: Some(Data),
    });
    app.handle_message(Message::SetQubitRole {
        row: 1,
        role: Some(Ancilla),
    });
    app.handle_message(Message::UpdateJsonFromCircuit);
    let json = app.state.ui_state.circuit_json_string.clone();
    let exported: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        exported["qubit_roles"],
        serde_json::json!(["data", "ancilla"])
    );

    app.handle_message(Message::NewCircuit);
    assert_eq!(app.state.circuit_state.circuit.qubit_roles, [None, None]);
    app.handle_message(Message::UpdateCircuitFromJson(json));
    assert_eq!(
        app.state.circuit_state.circuit.qubit_roles,
        [Some(Data), Some(Ancilla)]
    );

    app.handle_message(Message::ChangeQubits(1));
    app.handle_message(Message::ChangeQubits(3));
    assert_eq!(
        app.state.circuit_state.circuit.qubit_roles,
        [Some(Data), None, None]
    );
    app.handle_message(Message::SetQubitRole { row: 0, role: None });
    app.handle_message(Message::UpdateJsonFromCircuit);
    let cleared: serde_json::Value =
        serde_json::from_str(&app.state.ui_state.circuit_json_string).unwrap();
    assert!(cleared.get("qubit_roles").is_none());
}
