use quantsim_ui::{
    messages::Message,
    state::ui_state::{CentralPanelTab, LayoutView},
    QCSimApp,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    f32::consts::FRAC_1_SQRT_2,
};

fn load_layout(stem: &str, row_widths: &[usize], edges: &[[usize; 2]]) -> QCSimApp {
    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate(format!("{stem}.json")));
    assert_eq!(app.state.ui_state.error_message, None);
    assert_eq!(
        app.state.ui_state.active_central_tab,
        CentralPanelTab::Layout
    );
    assert_eq!(app.state.ui_state.layout_view, LayoutView::Plane);

    let circuit = &app.state.circuit_state.circuit;
    let layout = circuit.layout.as_ref().unwrap();
    let mut rows = BTreeMap::new();
    for &[_, y] in &layout.positions {
        *rows.entry(y).or_insert(0) += 1;
    }
    assert_eq!(rows.into_values().collect::<Vec<_>>(), row_widths);
    let undirected = |[a, b]: [usize; 2]| [a.min(b), a.max(b)];
    let edges: BTreeSet<_> = edges.iter().copied().map(undirected).collect();
    let links: BTreeSet<_> = layout.links.iter().copied().map(undirected).collect();
    assert_eq!(links, edges);

    let mut used_links = BTreeSet::new();
    for layer in &circuit.steps {
        let mut busy = BTreeSet::new();
        for op in layer {
            for &qubit in &op.qubits {
                assert!(busy.insert(qubit), "two gates on q{qubit} in one layer");
            }
            if let &[a, b] = op.qubits.as_slice() {
                used_links.insert(undirected([a as usize, b as usize]));
            }
        }
    }
    assert_eq!(used_links, links);
    app
}

fn assert_amplitudes(app: &QCSimApp, expected: &[(usize, f32)]) {
    assert_eq!(app.state.ui_state.error_message, None);
    let state = app.state.simulation_state.quantum_state.as_ref().unwrap();
    for (index, actual) in state.state_vector.iter().enumerate() {
        let amplitude = expected.iter().find(|(basis, _)| *basis == index);
        let expected = amplitude.map_or(0.0, |(_, value)| *value);
        assert!(
            (actual - expected).norm() < 1e-6,
            "basis {index}: {actual}, expected {expected}"
        );
    }
}

#[test]
fn the_ring_recombines_the_two_paths_at_a_phase_selected_exit() {
    let mut app = load_layout(
        "ring_interferometer_8",
        &[2, 2, 2, 2],
        &[
            [0, 1],
            [1, 2],
            [2, 3],
            [3, 4],
            [4, 5],
            [5, 6],
            [6, 7],
            [7, 0],
        ],
    );
    let a = FRAC_1_SQRT_2;
    for (step, left, right) in [(4, 0, 1), (5, 7, 2), (6, 6, 3), (7, 5, 4)] {
        app.handle_message(Message::SelectTimestep(step));
        assert_amplitudes(&app, &[(1 << left, -a), (1 << right, a)]);
    }
    app.handle_message(Message::SelectTimestep(8));
    assert_amplitudes(&app, &[(1 << 5, -a), (1 << 4, -a)]);
    app.handle_message(Message::RunSimulation);
    assert_amplitudes(&app, &[(1 << 4, -1.0)]);

    app.handle_message(Message::DeleteGate(4, 7));
    app.handle_message(Message::RunSimulation);
    assert_amplitudes(&app, &[(1 << 5, 1.0)]);
}

#[test]
fn the_tree_grows_a_ghz_state_across_both_branches() {
    let mut app = load_layout(
        "tree_ghz_7",
        &[1, 2, 4],
        &[[0, 1], [0, 2], [1, 3], [1, 4], [2, 5], [2, 6]],
    );
    let a = FRAC_1_SQRT_2;
    for (step, entangled) in [
        (1, 0b1),
        (2, 0b11),
        (3, 0b1111),
        (4, 0b11_1111),
        (5, 0b111_1111),
    ] {
        app.handle_message(Message::SelectTimestep(step));
        assert_amplitudes(&app, &[(0, a), (entangled, a)]);
    }

    app.handle_message(Message::DeleteGate(2, 4));
    app.handle_message(Message::RunSimulation);
    assert_amplitudes(&app, &[(0, a), (0b11_1111, a)]);
}
