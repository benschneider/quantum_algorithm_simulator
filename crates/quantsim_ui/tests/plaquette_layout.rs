use quantsim_core::circuit::QubitRole::{Ancilla, Data};
use quantsim_ui::{messages::Message, QCSimApp};

#[test]
fn plaquette_checks_its_four_neighbors_and_flags_at_the_final_coupling() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate("plaquette_z_check.json".into()));
    let circuit = &app.state.circuit_state.circuit;
    assert_eq!(
        circuit.qubit_roles,
        [
            Some(Data),
            Some(Data),
            Some(Data),
            Some(Data),
            Some(Ancilla)
        ]
    );
    let layout = circuit.layout.as_ref().unwrap();
    let center = layout.positions[4];
    assert_eq!(layout.links.len(), 4);
    for wire in 0..4 {
        let [x, y] = layout.positions[wire];
        assert_eq!(x.abs_diff(center[0]) + y.abs_diff(center[1]), 1);
        assert!(layout.links.contains(&[wire, 4]) || layout.links.contains(&[4, wire]));
        assert_eq!(
            circuit.steps[wire + 1][0].qubits.as_slice(),
            &[wire as u32, 4]
        );
    }

    for (step, expected_basis_state) in [(0, 0b00000), (1, 0b01000), (4, 0b01000), (5, 0b11000)] {
        app.handle_message(Message::SelectTimestep(step));
        assert_eq!(app.state.ui_state.error_message, None, "step {step}");
        let state = app.state.simulation_state.quantum_state.as_ref().unwrap();
        assert_eq!(
            state.probabilities()[expected_basis_state],
            1.0,
            "step {step}"
        );
    }
}
