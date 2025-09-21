use log::debug;
use quantsim_core::core::gates::Gate;
use quantsim_core::core::types::Param;
use quantsim_ui::messages::Message;
use quantsim_ui::prelude::*;

#[test]
fn test_gate_selection_for_editor() {
    let mut app = QCSimApp::default();

    // Place a gate
    app.handle_message(Message::SelectGate(Gate::Rx));
    app.handle_message(Message::PlaceGate(Gate::Rx, 0, 0));
    debug!(
        "Circuit after placing gate: {:#?}",
        &app.state.circuit_state.circuit
    );

    // Select the gate for editing
    app.handle_message(Message::SelectGateForEditing(0, 0));
    debug!(
        "Gate selected for editing: {:#?}",
        &app.state.ui_state.selected_gate_for_editing
    );

    assert_eq!(app.state.ui_state.selected_gate_for_editing, Some((0, 0)));
}

#[test]
fn test_clicking_gate_exits_placement_mode() {
    let mut app = QCSimApp::default();

    // Enter placement mode
    app.handle_message(Message::SelectGate(Gate::H));
    debug!("Placement mode: {:?}", &app.state.ui_state.placement_mode);
    assert_eq!(app.state.ui_state.placement_mode, PlacementMode::Placing);

    // Click a gate, which should exit placement mode
    app.handle_message(Message::SelectGateForEditing(0, 0));
    debug!(
        "Exited placement mode: {:?}",
        &app.state.ui_state.placement_mode
    );
    assert_eq!(app.state.ui_state.placement_mode, PlacementMode::Idle);
}

#[test]
fn test_update_gate_angle() {
    let _ = env_logger::builder().is_test(true).try_init();
    let mut app = QCSimApp::default();

    // Place a parametric gate
    app.handle_message(Message::SelectGate(Gate::Rz));
    app.handle_message(Message::PlaceGate(Gate::Rz, 0, 0));
    debug!(
        "Place a parametric gate: {:#?}",
        &app.state.circuit_state.circuit
    );

    // Select the gate for editing
    app.handle_message(Message::SelectGateForEditing(0, 0));

    // Update the angle
    let new_angle = 1.23;
    app.handle_message(Message::UpdateGateAngle(new_angle));
    debug!(
        "Updated gate angle: {:#?}",
        &app.state.circuit_state.circuit
    );

    let op = &app.state.circuit_state.circuit.steps[0][0];
    assert_eq!(op.params[0], Param::Scalar(new_angle as f32));
}
