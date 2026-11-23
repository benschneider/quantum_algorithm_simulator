use egui::{
    epaint::TextShape, Event, FullOutput, PointerButton, Pos2, RawInput, Rect, Shape, Vec2,
};
use quantsim_core::circuit::QubitLayout;
use quantsim_ui::{
    messages::Message,
    state::{ui_state::LayoutView, CircuitState},
    QCSimApp,
};
use serde_json::json;

fn visible_labels(output: &FullOutput) -> impl Iterator<Item = (Rect, &TextShape)> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Text(text)
                if clipped
                    .clip_rect
                    .contains(text.visual_bounding_rect().center()) =>
            {
                Some((clipped.clip_rect, text))
            }
            _ => None,
        })
}

fn painted_label(output: &FullOutput, label: &str) -> Pos2 {
    visible_labels(output)
        .find(|(_, text)| text.galley.text() == label)
        .map(|(_, text)| text.visual_bounding_rect().center())
        .unwrap_or_else(|| panic!("missing visible label {label}"))
}

fn render(app: &mut QCSimApp, ctx: &egui::Context, size: Vec2, events: Vec<Event>) -> FullOutput {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        events,
        ..Default::default()
    };
    ctx.run(input, |ctx| {
        eframe::App::update(app, ctx, &mut eframe::Frame::_new_kittest())
    })
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
    // The layout selector sits below the circuit grid (30 points per wire).
    let size = egui::vec2(1440.0, 1500.0 + 30.0 * wires as f32);
    let mut render = |events| render(&mut app, &ctx, size, events);
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
    assert_stack_opens_without_running(CircuitState::MAX_QUBITS, 100);
}

fn import_stack(circuit: serde_json::Value) -> QCSimApp {
    let mut app = QCSimApp::default();
    app.state.ui_state.show_welcome_screen = false;
    app.handle_message(Message::UpdateCircuitFromJson(circuit.to_string()));
    app.state.ui_state.layout_view = LayoutView::Stack;
    app
}

#[test]
fn custom_section_labels_follow_the_selected_timestep() {
    let mut app = import_stack(json!({
        "num_qubits": 2,
        "steps": [[], [], [], [], []],
        "sections": [
            {"start": 2, "end": 3, "label": "Couple"},
            {"start": 5, "end": 5, "label": "Read"},
        ],
    }));
    app.state.ui_state.stack_camera.pitch = 70.0;
    app.state.ui_state.stack_camera.yaw = 115.0;
    let ctx = egui::Context::default();
    let mut neutral = None;
    for step in 0..=5 {
        app.dispatch(Message::SelectTimestep(step));
        let output = render(&mut app, &ctx, egui::vec2(520.0, 1500.0), vec![]);
        // Grid headings repeat the section names. The range identifies the stack canvas.
        let (canvas, range) = visible_labels(&output)
            .find(|(_, text)| text.galley.text() == "t2–3")
            .expect("custom range on the time axis");
        let axis_text = |name| {
            visible_labels(&output)
                .find(|(clip, text)| *clip == canvas && text.galley.text() == name)
                .map(|(_, text)| text)
                .unwrap_or_else(|| panic!("missing axis label {name} at step {step}"))
        };
        let couple = axis_text("Couple");
        let read = axis_text("Read");
        let color = |text: &TextShape| text.galley.job.sections[0].format.color;
        let quiet = *neutral.get_or_insert(color(couple));
        assert_eq!(color(couple) != quiet, (2..=3).contains(&step));
        assert_eq!(color(range), color(couple));
        assert_eq!(color(read) != quiet, step == 5);
        assert!(read.visual_bounding_rect().bottom() < couple.visual_bounding_rect().top());
        axis_text("t5");
    }
}

#[test]
fn crowded_sampled_axis_keeps_the_inspected_section_legible() {
    let sections: Vec<_> = (0..30)
        .map(|i| json!({"start": 10*i+1, "end": 10*i+2, "label": format!("Check {i}")}))
        .collect();
    let mut app = import_stack(json!({
        "num_qubits": 1, "steps": vec![json!([]); 300], "sections": sections,
    }));
    app.state.ui_state.stack_camera.pitch = 75.0;
    app.dispatch(Message::SelectTimestep(221));
    let output = render(
        &mut app,
        &egui::Context::default(),
        egui::vec2(440.0, 1500.0),
        vec![],
    );
    let (canvas, _) = visible_labels(&output)
        .find(|(_, text)| text.galley.text() == "t221–222")
        .expect("inspected section range on a sampled timeline");
    let labels: Vec<_> = visible_labels(&output)
        .filter(|(clip, text)| *clip == canvas && text.galley.text().starts_with("Check "))
        .map(|(_, text)| text)
        .collect();
    assert!((2..sections.len()).contains(&labels.len()));
    assert!(labels.iter().any(|text| text.galley.text() == "Check 22"));
    for (i, label) in labels.iter().enumerate() {
        let rect = label.visual_bounding_rect();
        assert!(canvas.contains_rect(rect));
        assert!(labels[..i]
            .iter()
            .all(|other| !other.visual_bounding_rect().intersects(rect)));
    }
}
