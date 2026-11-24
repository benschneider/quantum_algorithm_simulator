use crate::messages::Message;
use crate::state::simulation_state::COMPACT_SHOTS;
use crate::state::ui_state::{CentralPanelTab, LayoutView};
use crate::QCSimApp;
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
