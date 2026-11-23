use quantsim_core::circuit::{Circuit, CircuitSection};
use quantsim_core::core::types::{Gate, Operation};
use quantsim_ui::ui::circuit_grid::{generate_circuit_grid, GridCell};

#[test]
fn section_header_spans_its_numbered_columns_and_aligns_with_gates() {
    let mut circuit = Circuit::new(2);
    circuit.steps.resize(4, vec![]);
    circuit
        .set_section(
            None,
            CircuitSection {
                label: "X coupling".into(),
                start: 2,
                end: 3,
            },
        )
        .unwrap();

    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let grid = generate_circuit_grid(&circuit, ui, 4, 2);
            let headers = grid
                .cells
                .iter()
                .filter_map(|cell| match cell {
                    GridCell::Timestep { step, rect } => Some((*step, *rect)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                headers.iter().map(|&(step, _)| step).collect::<Vec<_>>(),
                [1, 2, 3, 4]
            );
            let section = grid
                .cells
                .iter()
                .find_map(|cell| match cell {
                    GridCell::Section { rect, index: 0 } => Some(*rect),
                    _ => None,
                })
                .unwrap();
            assert_eq!(section.left(), headers[1].1.left());
            assert_eq!(section.right(), headers[2].1.right());
            assert!(section.bottom() < headers[1].1.top());
            for cell in &grid.cells {
                if let GridCell::Empty { rect, col, .. } = cell {
                    assert_eq!(rect.center().x, headers[*col].1.center().x);
                }
            }
        });
    });
}

#[test]
fn directional_gates_label_their_actual_wires_and_parallel_pairs() {
    let mut circuit = Circuit::new(4);
    circuit.steps = vec![
        vec![
            Operation::new(Gate::CX, vec![2, 0], vec![]),
            Operation::new(Gate::CY, vec![1, 3], vec![]),
        ],
        vec![Operation::new(Gate::CCNOT, vec![2, 3, 1], vec![])],
        vec![Operation::new(Gate::SWAP, vec![0, 1], vec![])],
    ];
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let grid = generate_circuit_grid(&circuit, ui, 3, 0);
            let column = |col| {
                grid.cells
                    .iter()
                    .filter_map(|cell| match cell {
                        GridCell::Gate(gate) if gate.col == col => Some(gate.label.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(column(0), ["X tgt·1", "Ctrl·2", "Ctrl·1", "Y tgt·2"]);
            assert_eq!(column(1), ["X tgt", "Ctrl", "Ctrl"]);
            assert_eq!(column(2), ["SWAP", "SWAP"]);
        });
    });
}
