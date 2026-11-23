use quantsim_core::core::gates::Gate;
use quantsim_core::core::types::Param;
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

#[test]
fn controlled_gates_placed_from_the_grid_have_editable_parameters() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::PlaceGate(Gate::X, 1, 0));
    app.handle_message(Message::PlaceMultiQubitGate(Gate::CXPow, vec![1, 0], 1));
    app.handle_message(Message::PlaceMultiQubitGate(Gate::CRz, vec![0, 1], 2));
    for column in [1, 2] {
        assert_eq!(
            app.state.circuit_state.circuit.steps[column][0]
                .params
                .as_slice(),
            &[Param::Scalar(0.0)]
        );
    }
    app.handle_message(Message::SelectGateForEditing(1, 1));
    app.handle_message(Message::UpdateGateAngle(1.0)); // Full controlled X.
    app.handle_message(Message::SelectGateForEditing(0, 2));
    app.handle_message(Message::UpdateGateAngle(std::f64::consts::FRAC_PI_2));

    app.handle_message(Message::UpdateJsonFromCircuit);
    let json = app.state.ui_state.circuit_json_string.clone();
    app.handle_message(Message::UpdateCircuitFromJson(json));
    app.handle_message(Message::RunSimulation);
    let state = app.state.simulation_state.quantum_state.as_ref().unwrap();
    assert!((state.state_vector[3] - nalgebra::Complex::i()).norm() < 1e-5);
}

#[test]
fn an_old_controlled_gate_with_no_stored_parameter_can_be_edited() {
    let mut app = QCSimApp::default();
    let json = r#"{"num_qubits":2,"steps":[[{"id":"X","qubits":[0]}],[{"id":"CXPow","qubits":[0,1],"params":[]}]]}"#;
    app.handle_message(Message::UpdateCircuitFromJson(json.into()));
    app.handle_message(Message::RunSimulation);
    let state = app.state.simulation_state.quantum_state.as_ref().unwrap();
    assert!((state.state_vector[1].norm_sqr() - 1.0).abs() < 1e-5);
    app.handle_message(Message::SelectGateForEditing(0, 1));
    app.handle_message(Message::UpdateGateAngle(1.0));
    app.handle_message(Message::RunSimulation);
    let state = app.state.simulation_state.quantum_state.as_ref().unwrap();
    assert!((state.state_vector[3].norm_sqr() - 1.0).abs() < 1e-5);
}

#[test]
fn gate_editor_shows_powers_as_exponents_and_rotations_as_degrees() {
    for (gate, parameter, label, displayed) in [
        (Gate::CXPow, 0.5, "Exponent", 0.5_f32),
        (
            Gate::CRz,
            std::f64::consts::FRAC_PI_2,
            "Angle (degrees)",
            90.0,
        ),
    ] {
        let mut app = QCSimApp::default();
        app.handle_message(Message::PlaceMultiQubitGate(gate, vec![1, 0], 0));
        app.handle_message(Message::SelectGateForEditing(1, 0));
        app.handle_message(Message::UpdateGateAngle(parameter));
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                quantsim_ui::components::gate_editor::gate_editor_panel(
                    &mut app.state,
                    ui,
                    &mut Vec::new(),
                );
            });
        });
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(labels.contains(&label), "{labels:?}");
        assert!(
            labels.iter().any(|s| s.parse() == Ok(displayed)),
            "{labels:?}"
        );
    }
}
