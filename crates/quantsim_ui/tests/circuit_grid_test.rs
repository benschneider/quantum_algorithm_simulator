use quantsim_core::circuit::{Circuit, CircuitSection};
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
