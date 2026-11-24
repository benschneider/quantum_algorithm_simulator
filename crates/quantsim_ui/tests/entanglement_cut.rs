#![cfg(not(target_arch = "wasm32"))]

use egui::{Context, Event, FullOutput, PointerButton, Pos2, RawInput, Rect, Shape};
use quantsim_ui::{messages::Message, QCSimApp};

fn frame(app: &mut QCSimApp, ctx: &Context, events: Vec<Event>) -> FullOutput {
    ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 900.0))),
            events,
            ..Default::default()
        },
        |ctx| eframe::App::update(app, ctx, &mut eframe::Frame::_new_kittest()),
    )
}

fn label_centers<'a>(frame: &'a FullOutput, expected: &'a str) -> impl Iterator<Item = Pos2> + 'a {
    frame.shapes.iter().rev().filter_map(move |shape| {
        let Shape::Text(text) = &shape.shape else {
            return None;
        };
        let center = text.visual_bounding_rect().center();
        (text.galley.text() == expected && shape.clip_rect.contains(center)).then_some(center)
    })
}

fn label(frame: &FullOutput, expected: &str) -> Pos2 {
    label_centers(frame, expected)
        .next()
        .unwrap_or_else(|| panic!("Missing visible label {expected:?}"))
}

fn assert_measured_rank(frame: &FullOutput, rank: &str) {
    let row = label(frame, "Schmidt rank");
    let gate_column = label(frame, "Gate bound");
    assert!(
        label_centers(frame, rank).any(|pos| pos.x > gate_column.x && (pos.y - row.y).abs() < 2.0),
        "Expected measured rank {rank}"
    );
}

fn click(app: &mut QCSimApp, ctx: &Context, pos: Pos2, button: PointerButton) -> FullOutput {
    let mut events = vec![Event::PointerMoved(pos)];
    events.extend([true, false].map(|pressed| Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }));
    frame(app, ctx, events);
    frame(app, ctx, vec![]); // Apply messages dispatched by the grid.
    frame(app, ctx, vec![])
}

#[test]
fn step_and_edit_a_bell_pair_while_inspecting_its_cut() {
    let mut app = QCSimApp::default();
    app.state.ui_state.show_welcome_screen = false;
    app.state.ui_state.show_results_window = true;
    app.handle_message(Message::UpdateCircuitFromJson(
        r#"{
        "num_qubits": 4,
        "steps": [[{"id":"H","qubits":[1]}],[{"id":"X","qubits":[3]},{"id":"CX","qubits":[1,2]}]]
    }"#
        .into(),
    ));
    let ctx = Context::default();
    ctx.style_mut(|style| style.animation_time = 0.0);
    frame(&mut app, &ctx, vec![]);
    let initial = frame(&mut app, &ctx, vec![]);
    let inspect = label(&initial, "Entanglement cut");
    let input = click(&mut app, &ctx, inspect, PointerButton::Primary);
    for text in ["Gate bound", "Dense state", "≤ 1", "Read step 0"] {
        label(&input, text);
    }

    let step = label(&input, "2");
    let bell = click(&mut app, &ctx, step, PointerButton::Primary);
    for text in ["Crossing gates at step 2", "≤ 2", "1.0000"] {
        label(&bell, text);
    }
    assert_measured_rank(&bell, "2");
    let outlines: Vec<_> = bell
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Rect(rect)
                if rect.stroke.color == ctx.style().visuals.hyperlink_color
                    && rect.stroke.width > 2.0 =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .collect();
    assert_eq!(outlines.len(), 2);
    for gate in ["Ctrl", "X tgt"] {
        assert!(
            label_centers(&bell, gate).any(|pos| outlines.iter().any(|rect| rect.contains(pos))),
            "Missing outlined {gate}"
        );
    }

    let control_y = outlines
        .iter()
        .map(|rect| rect.center().y)
        .fold(f32::INFINITY, f32::min);
    let prep = label_centers(&bell, "H")
        .find(|pos| (pos.y - control_y).abs() < 2.0)
        .expect("q1 preparation");
    let edited = click(&mut app, &ctx, prep, PointerButton::Secondary);
    let product = click(
        &mut app,
        &ctx,
        label(&edited, "Read step 2"),
        PointerButton::Primary,
    );
    for text in ["Crossing gates at step 2", "≤ 2", "0.0000"] {
        label(&product, text);
    }
    assert_measured_rank(&product, "1");
    let neighbor = click(&mut app, &ctx, label(&product, "‹"), PointerButton::Primary);
    for text in ["No gates cross this cut at this step.", "≤ 1"] {
        label(&neighbor, text);
    }
}
