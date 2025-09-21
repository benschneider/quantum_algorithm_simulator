use quantsim_ui::{app::QCSimApp, messages::Message};

#[test]
fn test_load_circuit_updates_app_state() {
    let mut app = QCSimApp {
        state: Default::default(),
        messages: Vec::new(),
    };

    let message = Message::LoadTemplate("bell.json".to_string());
    app.handle_message(message);

    assert_eq!(app.state.circuit_state.circuit.num_qubits, 2);
    assert_eq!(app.state.circuit_state.circuit.steps.len(), 2);
}
