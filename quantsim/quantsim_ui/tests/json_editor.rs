use quantsim_ui::{
    app::QCSimApp, messages::Message, state::app_state::AppState,
};

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
