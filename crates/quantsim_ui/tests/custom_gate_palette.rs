use nalgebra::Complex;
use quantsim_core::core::types::Gate;
use quantsim_ui::{app::QCSimApp, messages::Message};

fn set_cell(app: &mut QCSimApp, row: usize, col: usize, real: &str, imag: &str) {
    app.handle_message(Message::UpdateCustomGateEditorValue {
        row,
        col,
        real: real.into(),
        imag: imag.into(),
    });
}

fn amplitude_after(app: &mut QCSimApp, step: usize, index: usize) -> Complex<f32> {
    app.handle_message(Message::SelectTimestep(step));
    app.state
        .simulation_state
        .quantum_state
        .as_ref()
        .unwrap()
        .state_vector[index]
}

#[test]
fn custom_gates_keep_their_placed_matrices_across_palette_changes_and_json_reload() {
    let mut app = QCSimApp::default();
    // An identity from an older circuit used no explicit matrix.
    app.handle_message(Message::UpdateCircuitFromJson(
        r#"{"num_qubits":1,"steps":[[{"id":"X","qubits":[0]}],[{"id":"Custom","qubits":[0]}],[],[]]}"#.into(),
    ));
    app.handle_message(Message::OpenCustomGateEditor(Gate::Custom));
    set_cell(&mut app, 1, 1, "0", "1");
    app.handle_message(Message::SaveCustomGateMatrix);
    assert!(!app.state.custom_gate_editor_state.is_open);
    app.handle_message(Message::PlaceGate(Gate::Custom, 0, 2));

    app.handle_message(Message::OpenCustomGateEditor(Gate::Custom));
    assert_eq!(
        app.state.custom_gate_editor_state.matrix_str[(1, 1)],
        ("0".into(), "1".into())
    );
    set_cell(&mut app, 1, 1, "-1", "0");
    app.handle_message(Message::SaveCustomGateMatrix);
    app.handle_message(Message::PlaceGate(Gate::Custom, 0, 3));
    assert_eq!(amplitude_after(&mut app, 2, 1), Complex::new(1.0, 0.0));
    assert_eq!(amplitude_after(&mut app, 3, 1), Complex::new(0.0, 1.0));
    assert_eq!(amplitude_after(&mut app, 4, 1), Complex::new(0.0, -1.0));

    app.handle_message(Message::UpdateJsonFromCircuit);
    let exported = app.state.ui_state.circuit_json_string.clone();
    let mut reloaded = QCSimApp::default();
    reloaded.handle_message(Message::UpdateCircuitFromJson(exported));
    assert_eq!(
        amplitude_after(&mut reloaded, 4, 1),
        Complex::new(0.0, -1.0)
    );

    app.handle_message(Message::NewCircuit);
    app.handle_message(Message::OpenCustomGateEditor(Gate::Custom));
    assert_eq!(
        app.state.custom_gate_editor_state.matrix_str[(1, 1)],
        ("-1".into(), "0".into())
    );
}

#[test]
fn rejected_and_cancelled_drafts_preserve_the_saved_palette_gate() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::OpenCustomGateEditor(Gate::Custom));
    for invalid in ["oops", "NaN", "2"] {
        set_cell(&mut app, 1, 1, invalid, "0");
        app.handle_message(Message::SaveCustomGateMatrix);
        assert!(app.state.custom_gate_editor_state.is_open);
        assert!(app.state.custom_gate_editor_state.error_message.is_some());
        assert_eq!(
            app.state.custom_gate_editor_state.matrix_str[(1, 1)].0,
            invalid
        );
    }
    set_cell(&mut app, 1, 1, "-1", "0");
    assert!(app.state.custom_gate_editor_state.error_message.is_none());
    app.handle_message(Message::SaveCustomGateMatrix);

    app.handle_message(Message::OpenCustomGateEditor(Gate::Custom));
    set_cell(&mut app, 1, 1, "1", "0");
    app.handle_message(Message::CloseCustomGateEditor);
    app.handle_message(Message::OpenCustomGateEditor(Gate::Custom));
    assert_eq!(
        app.state.custom_gate_editor_state.matrix_str[(1, 1)].0,
        "-1"
    );
}
