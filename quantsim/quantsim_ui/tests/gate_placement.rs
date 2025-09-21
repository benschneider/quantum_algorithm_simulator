use quantsim_core::core::gates::Gate;
use quantsim_ui::messages::Message;
use quantsim_ui::prelude::*;
use smallvec::{smallvec, SmallVec};

#[test]
fn test_gate_placement() {
    let mut app = QCSimApp::default();

    app.handle_message(Message::SelectGate(Gate::H));

    app.handle_message(Message::PlaceGate(Gate::H, 0, 0));

    let circuit = &app.state.circuit_state.circuit;
    log::debug!("Circuit after placing gate: {:?}", circuit);
    assert_eq!(circuit.steps[0].len(), 1);
    let op = &circuit.steps[0][0];
    assert_eq!(op.id, Gate::H);
    let expected_qubits: SmallVec<[u32; 2]> = smallvec![0];
    assert_eq!(op.qubits, expected_qubits);
}

#[test]
fn test_multi_qubit_gate_placement() {
    let mut app = QCSimApp::default();

    app.handle_message(Message::SelectGate(Gate::CX));

    app.handle_message(Message::PlaceGate(Gate::CX, 0, 0));

    let circuit = &app.state.circuit_state.circuit;
    assert_eq!(circuit.steps[0].len(), 1);
    let op = &circuit.steps[0][0];
    assert_eq!(op.id, Gate::CX);
    let expected_qubits: SmallVec<[u32; 2]> = smallvec![0, 1];
    assert_eq!(op.qubits, expected_qubits);
}
