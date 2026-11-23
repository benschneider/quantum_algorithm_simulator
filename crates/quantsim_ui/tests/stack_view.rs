use egui::{Event, FullOutput, PointerButton, Pos2, RawInput, Rect, Shape};
use quantsim_core::circuit::QubitLayout;
use quantsim_ui::{messages::Message, state::ui_state::LayoutView, QCSimApp};
use serde_json::json;

fn painted_label(output: &FullOutput, label: &str) -> Pos2 {
    output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Text(text) if text.galley.text() == label => {
                let center = text.visual_bounding_rect().center();
                clipped.clip_rect.contains(center).then_some(center)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing visible label {label}"))
}

fn assert_stack_opens_without_running(wires: usize, steps: usize) {
    let operations: Vec<_> = (0..wires)
        .map(|wire| json!({"id": "H", "qubits": [wire]}))
        .collect();
    let circuit = json!({
        "num_qubits": wires,
        "layout": QubitLayout::grid(wires),
        "steps": vec![operations; steps],
    });
    let mut app = QCSimApp::default();
    app.state.ui_state.show_welcome_screen = false;
    app.dispatch(Message::UpdateCircuitFromJson(circuit.to_string()));
    let ctx = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();
    let mut render = |events| {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1440.0, 1500.0))),
            events,
            ..Default::default()
        };
        ctx.run(input, |ctx| eframe::App::update(&mut app, ctx, &mut frame))
    };
    let button = painted_label(&render(Vec::new()), "3D stack");
    let click = |pressed| Event::PointerButton {
        pos: button,
        button: PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    render(vec![Event::PointerMoved(button), click(true)]);
    render(vec![click(false)]);
    let painted = render(Vec::new());
    painted_label(&painted, "t0");
    if steps > 0 {
        painted_label(&painted, &format!("t{steps}"));
    }
    assert_eq!(app.state.circuit_state.num_timesteps, steps);
    assert_eq!(app.state.ui_state.layout_view, LayoutView::Stack);
    assert_eq!(app.state.ui_state.current_timestep, 0);
    assert_eq!(app.state.ui_state.error_message, None);
    assert!(!app.state.ui_state.show_results_window);
    assert!(app.state.simulation_state.quantum_state.is_none());
    assert!(app.messages.is_empty());
}

#[test]
fn empty_timeline_has_only_the_initial_slice() {
    assert_stack_opens_without_running(1, 0);
}

#[test]
fn maximum_editor_dimensions_paint_without_running_the_circuit() {
    assert_stack_opens_without_running(21, 100);
}
