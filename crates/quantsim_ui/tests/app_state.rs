use quantsim_ui::state::ui_state::TutorialStep;
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

#[test]
fn opening_a_lesson_and_starting_a_new_circuit_preserve_tutorial_progress() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate("ghz.json".into()));
    app.handle_message(Message::RunSimulation);
    app.handle_message(Message::ToggleTutorialWindow);
    assert_eq!(
        app.state.ui_state.active_tutorial_step,
        TutorialStep::TutorialIntro
    );
    assert_eq!(app.state.circuit_state.num_qubits, 3);
    assert!(app.state.simulation_state.quantum_state.is_some());

    app.state.ui_state.active_tutorial_step = TutorialStep::SingleQubitGatesIntroduction;
    app.state.ui_state.show_tutorial_window = false;
    app.handle_message(Message::ToggleTutorialWindow);
    assert_eq!(
        app.state.ui_state.active_tutorial_step,
        TutorialStep::SingleQubitGatesIntroduction
    );
    app.handle_message(Message::NewCircuit);
    assert_eq!(
        app.state.ui_state.active_tutorial_step,
        TutorialStep::SingleQubitGatesIntroduction
    );
    assert!(app.state.ui_state.show_tutorial_window);
    assert!(!app.state.ui_state.show_welcome_screen);
    assert!(app
        .state
        .circuit_state
        .circuit
        .steps
        .iter()
        .all(Vec::is_empty));
    assert!(app.state.simulation_state.quantum_state.is_none());
}
