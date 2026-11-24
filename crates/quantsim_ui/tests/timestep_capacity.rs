use egui::{Context, Event, FullOutput, PointerButton, Pos2, RawInput, Rect, Shape};
use quantsim_core::types::Gate;
use quantsim_ui::{
    components::{circuit_controls::circuit_controls, circuit_grid::circuit_grid},
    messages::Message,
    QCSimApp,
};
use serde_json::json;

fn render(ctx: &Context, app: &mut QCSimApp, events: Vec<Event>) -> FullOutput {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1200.0, 250.0))),
        events,
        ..Default::default()
    };
    let frame = ctx.run(input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            circuit_controls(ui, &mut app.state, &mut app.messages);
            egui::ScrollArea::horizontal()
                .stick_to_right(true)
                .show(ui, |ui| {
                    let step = app.state.ui_state.current_timestep;
                    circuit_grid(&mut app.state, ui, &mut app.messages, step);
                });
        });
    });
    for message in app.drain_messages() {
        app.handle_message(message);
    }
    frame
}

fn click(ctx: &Context, app: &mut QCSimApp, label: &str) {
    let frame = render(ctx, app, vec![]);
    let pos = frame
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            Shape::Text(text) if text.galley.text() == label => {
                Some(text.visual_bounding_rect().center())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing circuit control {label}"));
    let mut events = vec![Event::PointerMoved(pos)];
    events.extend([true, false].map(|pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }));
    render(ctx, app, events);
}

fn ram_button(ctx: &Context, app: &mut QCSimApp) -> String {
    render(ctx, app, vec![])
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            Shape::Text(text) if text.galley.text().starts_with("Est. peak RAM") => {
                Some(text.galley.text().to_owned())
            }
            _ => None,
        })
        .expect("RAM estimate button")
}

#[test]
fn editable_ram_warning_persists_and_allows_run_above_threshold() {
    let ctx = Context::default();
    let mut app = QCSimApp::default();
    let circuit = r#"{"num_qubits":16,"steps":[[{"id":"X","qubits":[15]}]]}"#;
    app.handle_message(Message::UpdateCircuitFromJson(circuit.into()));
    let estimate = ram_button(&ctx, &mut app);
    assert!(!estimate.contains("above"));
    click(&ctx, &mut app, &estimate);
    let default = if cfg!(target_arch = "wasm32") {
        "64 MiB"
    } else {
        "128 MiB"
    };
    click(&ctx, &mut app, default);
    render(&ctx, &mut app, vec![Event::Text("1".into())]);
    assert!(ram_button(&ctx, &mut app).contains("above 1 MiB"));

    let saved = ctx.data(|data| serde_json::to_string(data).unwrap());
    let restored = Context::default();
    restored.data_mut(|data| *data = serde_json::from_str(&saved).unwrap());
    assert!(ram_button(&restored, &mut app).contains("above 1 MiB"));
    click(&restored, &mut app, "Run ▶");
    assert_eq!(app.state.ui_state.error_message, None);
    let state = app.state.simulation_state.quantum_state.as_ref().unwrap();
    assert_eq!(state.state_vector[1 << 15].re, 1.0);
}

#[test]
fn edit_to_one_thousand_steps_and_scrub_the_final_gate() {
    let ctx = Context::default();
    let mut app = QCSimApp::default();
    click(&ctx, &mut app, "10");
    render(&ctx, &mut app, vec![Event::Text("1000".into())]);
    render(&ctx, &mut app, vec![]);
    assert_eq!(app.state.circuit_state.circuit.steps.len(), 1000);

    app.handle_message(Message::PlaceGate(Gate::X, 0, 999));
    click(&ctx, &mut app, "X");
    assert_eq!(app.state.ui_state.selected_gate_for_editing, Some((0, 999)));
    assert!(app.state.ui_state.show_gate_editor_window);
    for (button, step, amplitude) in [
        ("Run ▶", 1000, 1.0),
        ("Previous step", 999, 0.0),
        ("Next step", 1000, 1.0),
    ] {
        click(&ctx, &mut app, button);
        assert_eq!(app.state.ui_state.current_timestep, step);
        assert_eq!(app.state.ui_state.error_message, None);
        let state = app.state.simulation_state.quantum_state.as_ref().unwrap();
        assert_eq!(state.state_vector[1].re, amplitude);
    }
}

#[test]
fn imported_timeline_above_the_entry_limit_retains_its_final_gate() {
    let mut steps = vec![json!([]); 1001];
    steps[1000] = json!([{"id":"X","qubits":[0]}]);
    let mut app = QCSimApp::default();
    app.handle_message(Message::UpdateCircuitFromJson(
        json!({"num_qubits":1, "steps":steps}).to_string(),
    ));
    let ctx = Context::default();
    click(&ctx, &mut app, "Run ▶");
    assert_eq!(app.state.circuit_state.circuit.steps.len(), 1001);
    assert_eq!(app.state.ui_state.current_timestep, 1001);
    let state = app.state.simulation_state.quantum_state.as_ref().unwrap();
    assert_eq!(state.state_vector[1].re, 1.0);
}
