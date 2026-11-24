use nalgebra::Complex;
use quantsim_core::{circuit::Circuit, circuits, types::Gate, CircuitBuilder};
use quantsim_ui::{
    messages::Message,
    state::ui_state::{CentralPanelTab, LayoutView},
    QCSimApp,
};
use std::collections::BTreeSet;

const PATCH: [u32; 8] = [21, 22, 23, 24, 31, 32, 33, 34];
// Local bits 0..3 are the upper row and 4..7 the lower row.
const MATCHINGS: [&[[u32; 2]]; 3] = [
    &[[0, 1], [2, 3], [4, 5], [6, 7]],
    &[[1, 2], [5, 6]],
    &[[0, 4], [1, 5], [2, 6], [3, 7]],
];

#[test]
fn sycamore_uses_the_53_site_chip_and_only_the_patch_couplers() {
    let circuit = circuits::load_template("sycamore_53").unwrap();
    let layout = circuit.layout.as_ref().unwrap();
    // Inclusive column bounds from the published 53-site geometry, one row per pair.
    let row_spans = [
        (5, 6),
        (4, 7),
        (4, 8),
        (2, 9),
        (1, 9),
        (0, 8),
        (1, 7),
        (2, 6),
        (3, 5),
        (4, 4),
    ];
    let positions: Vec<_> = row_spans
        .into_iter()
        .enumerate()
        .flat_map(|(row, (first, last))| (first..=last).map(move |col| [col, row as i64]))
        .collect();
    assert_eq!(circuit.num_qubits, 53);
    assert_eq!(layout.positions, positions);
    let mut neighbors = BTreeSet::new();
    for (wire, &[x, y]) in positions.iter().enumerate() {
        for (other, &[u, v]) in positions.iter().enumerate().skip(wire + 1) {
            if x.abs_diff(u) + y.abs_diff(v) == 1 {
                neighbors.insert([wire, other]);
            }
        }
    }
    assert_eq!(layout.links.len(), 86);
    let links: BTreeSet<_> = layout
        .links
        .iter()
        .map(|&[a, b]| [a.min(b), a.max(b)])
        .collect();
    assert_eq!(links, neighbors);
    assert_eq!(circuit.steps.len(), 5);
    for (layer, expected) in MATCHINGS.iter().enumerate() {
        let actual: BTreeSet<_> = circuit.steps[layer + 1]
            .iter()
            .map(|op| {
                assert_eq!(op.id, Gate::SYC);
                let [a, b] = *op.qubits.as_slice() else {
                    panic!("SYC needs two wires")
                };
                [a.min(b), a.max(b)]
            })
            .collect();
        assert_eq!(
            actual,
            expected
                .iter()
                .map(|&[a, b]| [PATCH[a as usize], PATCH[b as usize]])
                .collect()
        );
        assert_eq!(circuit.steps[layer + 1].len(), expected.len());
    }
}

// A dense eight-wire reference uses MOVE for the -i excitation exchange and
// CZPow(-1/6) for the |11> phase, independently of the registered SYC matrix.
fn dense_patch() -> Circuit {
    let mut builder = CircuitBuilder::new(8);
    builder.x(0);
    for q in 1..8 {
        builder.h(q);
    }
    builder.next_step();
    for matching in MATCHINGS {
        for &[a, b] in matching {
            builder.move_gate(a, b);
        }
        builder.next_step();
        for pair in matching {
            builder.add_gate(Gate::CZPow, pair, &[-1.0 / 6.0]);
        }
        builder.next_step();
    }
    for q in (0..8).filter(|&q| q != 6) {
        builder.h(q);
    }
    builder.build()
}

#[test]
fn five_steps_and_full_run_match_the_dense_patch() {
    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate("sycamore_53.json".into()));
    assert_eq!(
        app.state.ui_state.active_central_tab,
        CentralPanelTab::Layout
    );
    assert_eq!(app.state.ui_state.layout_view, LayoutView::Stack);
    assert_eq!(app.state.ui_state.current_timestep, 0);
    let dense = dense_patch();
    for (step, dense_steps) in [0, 1, 3, 5, 7, 8].into_iter().enumerate() {
        app.handle_message(Message::SelectTimestep(step));
        assert_eq!(app.state.ui_state.current_timestep, step);
        assert_eq!(app.state.ui_state.error_message, None, "step {step}");
        assert!(app.state.simulation_state.quantum_state.is_none());
        let grouped = app
            .state
            .circuit_state
            .circuit
            .run_grouped_prefix(step)
            .unwrap();
        assert!(grouped.largest_group_size() <= PATCH.len());
        let reference = dense.run_prefix(dense_steps, &Default::default());
        assert!(reference.diagnostics.is_empty());
        let mut expected_one = [0.0; 8];
        for (local, z) in reference.final_state_vector.unwrap().iter().enumerate() {
            for (bit, p) in expected_one.iter_mut().enumerate() {
                if local & (1 << bit) != 0 {
                    *p += f64::from(z.norm_sqr());
                }
            }
            let index = PATCH.iter().enumerate().fold(0u128, |bits, (bit, wire)| {
                bits | (((local >> bit) & 1) as u128) << wire
            });
            let actual = grouped.amplitude(index).unwrap();
            assert!(
                (actual - Complex::new(f64::from(z.re), f64::from(z.im))).norm() < 2e-6,
                "step {step}, patch basis {local}"
            );
        }
        for (wire, expected) in PATCH.into_iter().zip(expected_one) {
            let actual = grouped.probability_one(wire as usize).unwrap();
            assert!(
                (f64::from(actual) - expected).abs() < 2e-6,
                "step {step}, q{wire}"
            );
        }
        for wire in (0..53).filter(|wire| !PATCH.contains(wire)) {
            assert_eq!(grouped.probability_one(wire as usize), Some(0.0));
        }
        if step > 0 {
            let wire = [21, 22, 23, 33, 33][step - 1];
            assert_eq!(grouped.probability_one(wire), Some(1.0));
            let patch_bits = PATCH.iter().fold(0u128, |bits, q| bits | (1u128 << q));
            for shot in grouped.sample(64, 73) {
                assert_eq!(shot & !patch_bits, 0, "shot included an idle wire");
                assert_ne!(shot & (1u128 << wire), 0, "step {step}, q{wire}");
            }
        }
        if step == 3 {
            let before: BTreeSet<_> = grouped.group_wires(33).unwrap().iter().copied().collect();
            assert_eq!(before, BTreeSet::from([31, 32, 33, 34]));
        }
        if step >= 4 {
            assert_eq!(grouped.group_wires(33), Some(&[33][..]));
            let rest: BTreeSet<_> = grouped.group_wires(21).unwrap().iter().copied().collect();
            assert_eq!(rest, BTreeSet::from([21, 22, 23, 24, 31, 32, 34]));
        }
        if step == 5 {
            assert!((grouped.probability_one(31).unwrap() - 0.40849365).abs() < 1e-6);
            assert!((grouped.probability_one(21).unwrap() - 0.47099365).abs() < 1e-6);
        }
    }
    app.handle_message(Message::SelectTimestep(0));
    app.handle_message(Message::RunSimulation);
    assert_eq!(app.state.ui_state.error_message, None);
    assert_eq!(app.state.ui_state.current_timestep, 5);
    assert!(app.state.ui_state.show_results_window);
}
