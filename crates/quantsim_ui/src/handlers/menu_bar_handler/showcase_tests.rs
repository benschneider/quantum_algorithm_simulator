use crate::messages::Message;
use crate::state::simulation_state::COMPACT_SHOTS;
use crate::state::ui_state::{CentralPanelTab, LayoutView};
use crate::QCSimApp;
use quantsim_core::types::Gate;
use quantsim_core::BasisIndex;

#[test]
fn sycamore_echo_returns_the_phase_flip_to_q23() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate("sycamore_echo_53.json".into()));
    assert_eq!(app.state.ui_state.error_message, None);
    assert_eq!(
        app.state.ui_state.active_central_tab,
        CentralPanelTab::Layout
    );
    assert_eq!(app.state.ui_state.layout_view, LayoutView::Plane);
    let circuit = &app.state.circuit_state.circuit;
    assert_eq!(circuit.num_qubits, 53);
    let layout = circuit.layout.as_ref().unwrap();
    for op in circuit.steps.iter().flatten() {
        if let [a, b] = op.qubits.as_slice() {
            let (a, b) = (*a as usize, *b as usize);
            assert!(layout.links.contains(&[a, b]) || layout.links.contains(&[b, a]));
        }
    }
    for step in [8, 9] {
        app.handle_message(Message::SelectTimestep(step));
        assert_eq!(
            app.state.simulation_state.wire_one_probabilities,
            vec![0.5; 53]
        );
        let all_ones = (BasisIndex::from(1u8) << 53usize) - 1u8;
        let samples = app.state.simulation_state.compact_samples.as_ref().unwrap();
        assert_eq!(samples.distinct, 2);
        assert!(samples
            .rows
            .iter()
            .all(|(bits, _)| *bits == BasisIndex::ZERO || *bits == all_ones));
    }
    for phase_flip in [true, false] {
        if !phase_flip {
            app.handle_message(Message::DeleteGate(33, 8));
        }
        app.handle_message(Message::RunSimulation);
        assert_eq!(app.state.ui_state.error_message, None);
        assert_eq!(app.state.ui_state.current_timestep, 17);
        let expected = BasisIndex::from(u8::from(phase_flip)) << 23usize;
        let samples = app.state.simulation_state.compact_samples.as_ref().unwrap();
        assert_eq!(samples.rows, [(expected, COMPACT_SHOTS)]);
    }
}

#[test]
fn bell_256_pairs_only_the_endpoint_bits() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate("bell_256.json".into()));
    app.handle_message(Message::RunSimulation);
    assert_eq!(app.state.ui_state.error_message, None);
    assert_eq!(app.state.ui_state.current_timestep, 2);
    let readout = &app.state.simulation_state;
    let mut expected = vec![0.0; 256];
    expected[0] = 0.5;
    expected[255] = 0.5;
    assert_eq!(readout.wire_one_probabilities, expected);
    let endpoints = (BasisIndex::from(1u8) << 255usize) + 1u8;
    let samples = readout.compact_samples.as_ref().unwrap();
    assert_eq!(samples.distinct, 2);
    assert!(samples
        .rows
        .iter()
        .all(|(bits, _)| *bits == BasisIndex::ZERO || *bits == endpoints));
}

#[test]
fn phase_echo_512_returns_to_zero_until_one_inverse_rotation_is_removed() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate("phase_echo_512.json".into()));
    app.handle_message(Message::RunSimulation);
    assert_eq!(app.state.ui_state.error_message, None);
    assert_eq!(app.state.ui_state.current_timestep, 4);
    let readout = &app.state.simulation_state;
    assert_eq!(readout.wire_one_probabilities.len(), 512);
    assert!(readout.wire_one_probabilities.iter().all(|&p| p < 1e-6));
    assert_eq!(
        readout.compact_samples.as_ref().unwrap().rows,
        [(BasisIndex::ZERO, COMPACT_SHOTS)]
    );

    app.handle_message(Message::DeleteGate(0, 2));
    app.handle_message(Message::RunSimulation);
    assert_eq!(app.state.ui_state.error_message, None);
    let probabilities = &app.state.simulation_state.wire_one_probabilities;
    let expected = std::f64::consts::FRAC_PI_8.sin().powi(2);
    assert!((probabilities[0] - expected).abs() < 1e-6);
    assert!(probabilities[1..].iter().all(|&p| p < 1e-6));
}

#[test]
fn grouped_blocks_1024_bridge_changes_correlations_with_equal_wire_marginals() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate("grouped_blocks_1024.json".into()));
    assert_eq!(app.state.circuit_state.num_timesteps, 5);
    // An extra CNOT reads the q4/q5 disagreement probability directly on q4.
    app.handle_message(Message::ChangeTimesteps(6));
    app.handle_message(Message::PlaceMultiQubitGate(Gate::CX, vec![5, 4], 5));

    for bridged in [true, false] {
        if !bridged {
            app.handle_message(Message::DeleteGate(3, 4));
        }
        app.handle_message(Message::SelectTimestep(5));
        assert_eq!(app.state.ui_state.error_message, None);
        let readout = &app.state.simulation_state;
        assert_eq!(readout.wire_one_probabilities.len(), 1024);
        for (wire, &p) in readout.wire_one_probabilities.iter().enumerate() {
            assert!((p - 0.5).abs() < 1e-6, "q{wire}: {p}");
            let size = if bridged && wire < 8 { 8 } else { 4 };
            let group = readout.group_wires(wire).unwrap();
            assert_eq!(group.len(), size);
            assert!(group.iter().all(|&q| q as usize / size == wire / size));
        }

        app.handle_message(Message::RunSimulation);
        assert_eq!(app.state.ui_state.error_message, None);
        let disagreement = app.state.simulation_state.wire_one_probabilities[4];
        let expected = if bridged { 0.5 } else { 0.0 };
        assert!((disagreement - expected).abs() < 1e-6);
    }
}
