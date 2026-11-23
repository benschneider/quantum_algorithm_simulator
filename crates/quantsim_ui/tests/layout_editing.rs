use quantsim_core::circuit::{CircuitData, QubitLayout};
use quantsim_ui::{messages::Message, QCSimApp};

#[test]
fn saving_layout_preserves_the_selected_step_and_exports_the_edit() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate(
        "error_correction_bit_flip_two_faults.json".into(),
    ));
    assert_eq!(app.state.ui_state.selected_layout_wire, Some(3));
    app.handle_message(Message::SelectTimestep(5));
    let probabilities = app
        .state
        .simulation_state
        .quantum_state
        .as_ref()
        .unwrap()
        .probabilities();
    let mut custom = QubitLayout::ring(5);
    custom.positions[3] = [-7, 2];
    custom.links.pop();
    app.state.ui_state.layout_draft = Some(custom.clone());
    app.handle_message(Message::SaveLayout);
    assert!(app.state.ui_state.layout_draft.is_none());
    assert_eq!(app.state.ui_state.current_timestep, 5);
    assert_eq!(
        app.state
            .simulation_state
            .quantum_state
            .as_ref()
            .unwrap()
            .probabilities(),
        probabilities
    );

    app.handle_message(Message::UpdateJsonFromCircuit);
    let exported = app.state.ui_state.circuit_json_string.clone();
    assert_eq!(
        serde_json::from_str::<CircuitData>(&exported)
            .unwrap()
            .layout
            .as_ref(),
        Some(&custom)
    );
    app.handle_message(Message::LoadTemplate("bell.json".into()));
    assert!(app.state.circuit_state.circuit.layout.is_none());
    assert!(app.state.ui_state.selected_layout_wire.is_none());
    app.handle_message(Message::UpdateCircuitFromJson(exported));
    assert_eq!(
        app.state.circuit_state.circuit.layout.as_ref(),
        Some(&custom)
    );
}

#[test]
fn invalid_drafts_cannot_replace_a_working_layout_and_resize_discards_stale_edits() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate("plaquette_z_check.json".into()));
    let saved = app.state.circuit_state.circuit.layout.clone().unwrap();
    let mut invalid = saved.clone();
    invalid.positions[1] = invalid.positions[0];
    app.state.ui_state.layout_draft = Some(invalid.clone());
    app.handle_message(Message::SaveLayout);
    assert_eq!(app.state.ui_state.layout_draft, Some(invalid));
    assert_eq!(
        app.state.circuit_state.circuit.layout.as_ref(),
        Some(&saved)
    );
    assert_eq!(app.state.ui_state.selected_layout_wire, Some(4));

    app.handle_message(Message::ChangeQubits(4));
    assert!(app.state.ui_state.layout_draft.is_none());
    assert!(app.state.ui_state.selected_layout_wire.is_none());
    let remaining = app.state.circuit_state.circuit.layout.as_ref().unwrap();
    assert_eq!(remaining.positions, saved.positions[..4]);
    assert!(remaining.links.is_empty());
    app.state.ui_state.layout_draft = Some(QubitLayout::grid(4));
    app.handle_message(Message::LoadTemplate("ghz.json".into()));
    assert!(app.state.ui_state.layout_draft.is_none());
    assert!(app.state.circuit_state.circuit.layout.is_none());
}

#[test]
fn picking_a_stack_site_reads_that_exact_step_without_changing_the_layout() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate("plaquette_z_check.json".into()));
    let layout = app.state.circuit_state.circuit.layout.clone();
    for (step, expected_one) in [(5, true), (4, false), (0, false)] {
        app.handle_message(Message::InspectLayoutSite { wire: 4, step });
        assert_eq!(app.state.ui_state.selected_layout_wire, Some(4));
        assert_eq!(app.state.ui_state.current_timestep, step);
        assert!(app.state.ui_state.show_results_window);
        let probabilities = app
            .state
            .simulation_state
            .quantum_state
            .as_ref()
            .unwrap()
            .probabilities();
        let p_one: f32 = probabilities
            .iter()
            .enumerate()
            .filter(|(index, _)| index & (1 << 4) != 0)
            .map(|(_, p)| p)
            .sum();
        assert!((p_one - if expected_one { 1.0 } else { 0.0 }).abs() < 1e-5);
    }
    app.handle_message(Message::InspectLayoutSite { wire: 1, step: 0 });
    app.handle_message(Message::InspectLayoutSite { wire: 6, step: 4 });
    app.handle_message(Message::InspectLayoutSite { wire: 4, step: 6 });
    assert_eq!(app.state.ui_state.selected_layout_wire, Some(1));
    assert_eq!(app.state.ui_state.current_timestep, 0);
    assert_eq!(app.state.circuit_state.circuit.layout, layout);
}
