use quantsim_core::circuit::CircuitSection;
use quantsim_ui::{app::QCSimApp, messages::Message, state::ui_state::SectionDraft};
use serde_json::json;

#[test]
fn editing_sections_preserves_the_readout_and_exports_the_saved_ranges() {
    let mut app = QCSimApp::default();
    let unannotated = json!({
        "num_qubits": 2,
        "steps": [[{"id": "H", "qubits": [0]}], [{"id": "CX", "qubits": [0, 1]}]]
    });
    app.handle_message(Message::UpdateCircuitFromJson(unannotated.to_string()));
    app.handle_message(Message::ChangeTimesteps(5));
    app.handle_message(Message::SelectTimestep(2));
    let state_vector = app
        .state
        .simulation_state
        .quantum_state
        .as_ref()
        .unwrap()
        .state_vector
        .clone();

    let mut draft = SectionDraft::new(2, None);
    draft.section.label = "Couple".into();
    app.state.ui_state.section_editor = Some(draft);
    app.handle_message(Message::SaveSection);
    assert!(app.state.ui_state.section_editor.is_none());
    let first = app
        .state
        .circuit_state
        .circuit
        .section_at(2)
        .unwrap()
        .clone();

    let mut draft = SectionDraft::new(4, None);
    draft.section.label = "Readout".into();
    app.state.ui_state.section_editor = Some(draft);
    app.handle_message(Message::SaveSection);

    let mut draft = SectionDraft::new(2, Some(&first));
    draft.section = CircuitSection {
        start: 1,
        end: 4,
        label: "Prepare Bell pair".into(),
    };
    app.state.ui_state.section_editor = Some(draft);
    app.handle_message(Message::SaveSection);
    let draft = app.state.ui_state.section_editor.as_mut().unwrap();
    assert!(draft.error.as_deref().unwrap().contains("overlap"));
    assert_eq!(app.state.circuit_state.circuit.section_at(2), Some(&first));
    draft.section.end = 2;
    app.handle_message(Message::SaveSection);
    assert!(app.state.ui_state.section_editor.is_none());
    assert_eq!(app.state.ui_state.current_timestep, 2);
    assert_eq!(
        app.state
            .simulation_state
            .quantum_state
            .as_ref()
            .unwrap()
            .state_vector,
        state_vector
    );

    app.handle_message(Message::UpdateJsonFromCircuit);
    let exported: serde_json::Value =
        serde_json::from_str(&app.state.ui_state.circuit_json_string).unwrap();
    assert_eq!(
        exported["sections"],
        json!([
            {"start": 1, "end": 2, "label": "Prepare Bell pair"},
            {"start": 4, "end": 4, "label": "Readout"}
        ])
    );
    let saved = app.state.circuit_state.circuit.section_at(2);
    app.state.ui_state.section_editor = Some(SectionDraft::new(2, saved));
    app.handle_message(Message::DeleteSection);
    assert!(app.state.circuit_state.circuit.section_at(2).is_none());
    assert_eq!(app.state.circuit_state.circuit.sections.len(), 1);
}

#[test]
fn failed_import_keeps_the_current_edit_while_resize_and_replacement_clear_it() {
    let mut app = QCSimApp::default();
    let mut data = json!({
        "num_qubits": 1,
        "steps": [[], [], [], [], [], []],
        "sections": [
            {"start": 2, "end": 4, "label": "Couple"},
            {"start": 6, "end": 6, "label": "Readout"}
        ]
    });
    app.handle_message(Message::UpdateCircuitFromJson(data.to_string()));
    app.handle_message(Message::RunSimulation);
    let saved = app.state.circuit_state.circuit.section_at(6);
    app.state.ui_state.section_editor = Some(SectionDraft::new(6, saved));
    data["sections"][1]["end"] = json!(7);
    app.handle_message(Message::UpdateCircuitFromJson(data.to_string()));
    assert!(app.state.ui_state.error_message.is_some());
    assert_eq!(
        app.state
            .ui_state
            .section_editor
            .as_ref()
            .unwrap()
            .original_start,
        Some(6)
    );
    assert_eq!(
        app.state.circuit_state.circuit.section_at(6).unwrap().end,
        6
    );
    assert_eq!(app.state.ui_state.current_timestep, 6);
    assert!(app.state.simulation_state.quantum_state.is_some());

    app.handle_message(Message::ChangeTimesteps(3));
    assert!(app.state.ui_state.section_editor.is_none());
    assert!(app.state.simulation_state.quantum_state.is_none());
    assert_eq!(app.state.ui_state.current_timestep, 3);
    assert_eq!(
        app.state.circuit_state.circuit.section_at(3).unwrap().end,
        3
    );
    app.handle_message(Message::ChangeTimesteps(6));
    assert!(app.state.circuit_state.circuit.section_at(4).is_none());
    assert!(app.state.circuit_state.circuit.section_at(6).is_none());

    app.state.ui_state.section_editor = Some(SectionDraft::new(
        3,
        app.state.circuit_state.circuit.section_at(3),
    ));
    app.handle_message(Message::LoadTemplate("bell.json".into()));
    assert!(app.state.ui_state.section_editor.is_none());
    assert!(app.state.ui_state.error_message.is_none());
    let bell = quantsim_core::circuits::load_template("bell").unwrap();
    assert_eq!(app.state.circuit_state.circuit.sections, bell.sections);
}
