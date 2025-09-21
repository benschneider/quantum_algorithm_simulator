use quantsim_core::core::gates::Gate;
use quantsim_core::core::types::Param;
use quantsim_ui::messages::Message;
use quantsim_ui::prelude::*;

#[test]
fn test_gate_editor_content_is_correct() {
    let mut app = QCSimApp::default();

    // Place a parametric gate
    app.handle_message(Message::SelectGate(Gate::Rx));
    app.handle_message(Message::PlaceGate(Gate::Rx, 0, 0));

    // Select the gate for editing
    app.handle_message(Message::SelectGateForEditing(0, 0));

    // Check that the gate editor shows the correct angle
    let op = &app.state.circuit_state.circuit.steps[0][0];
    assert_eq!(op.params[0], Param::Scalar(0.0));
}
