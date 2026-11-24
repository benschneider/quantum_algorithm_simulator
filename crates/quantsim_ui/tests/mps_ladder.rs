#![cfg(not(target_arch = "wasm32"))]

use egui::{Event, FullOutput, PointerButton, Pos2, RawInput, Rect, Shape};
use quantsim_ui::QCSimApp;
use std::time::{Duration, Instant};

fn frame(app: &mut QCSimApp, ctx: &egui::Context, events: Vec<Event>) -> FullOutput {
    ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 900.0))),
            events,
            ..Default::default()
        },
        |ctx| eframe::App::update(app, ctx, &mut eframe::Frame::_new_kittest()),
    )
}

fn labels(output: &FullOutput) -> impl DoubleEndedIterator<Item = (&str, Pos2)> {
    output.shapes.iter().filter_map(|clipped| {
        let Shape::Text(text) = &clipped.shape else {
            return None;
        };
        let center = text.visual_bounding_rect().center();
        clipped
            .clip_rect
            .contains(center)
            .then_some((text.galley.text(), center))
    })
}

fn click(app: &mut QCSimApp, ctx: &egui::Context, label: &str) {
    frame(app, ctx, Vec::new());
    let output = frame(app, ctx, Vec::new());
    let (_, pos) = labels(&output)
        .rev()
        .find(|(text, _)| *text == label)
        .unwrap_or_else(|| panic!("Missing control {label:?}"));
    let mut events = vec![Event::PointerMoved(pos)];
    events.extend([true, false].map(|pressed| Event::PointerButton {
        pos,
        pressed,
        button: PointerButton::Primary,
        modifiers: egui::Modifiers::NONE,
    }));
    frame(app, ctx, events);
}

#[test]
fn ladder_template_opens_the_benchmark_and_preserves_the_working_circuit() {
    use quantsim_ui::messages::Message;

    let mut app = QCSimApp::default();
    app.state.ui_state.show_welcome_screen = false;
    app.handle_message(Message::LoadTemplate("bell.json".into()));
    app.handle_message(Message::RunSimulation);
    let editor_json = app.state.ui_state.circuit_json_string.clone();
    let ctx = egui::Context::default();
    ctx.style_mut(|style| style.animation_time = 0.0);
    click(&mut app, &ctx, "File");
    click(&mut app, &ctx, "Load Template");
    click(&mut app, &ctx, "Scale experiments");
    click(&mut app, &ctx, "MPS ladder echo 256");
    frame(&mut app, &ctx, Vec::new());
    let output = frame(&mut app, &ctx, Vec::new());
    assert!(labels(&output).any(|(text, _)| text == "Selected cap 256"));
    assert!(labels(&output).any(|(text, _)| text == "Run echo"));
    assert_eq!(app.state.ui_state.circuit_json_string, editor_json);
    assert!(app.state.simulation_state.quantum_state.is_some());
}

#[test]
fn mps_ladder_opens_with_auto_and_runs_the_manual_cap() {
    let rows: Vec<(usize, f64, u64)> = include_str!("../assets/benchmarks/mps-cap-curve.csv")
        .lines()
        .skip(1)
        .map(|line| {
            let cells: Vec<_> = line.split(',').collect();
            (
                cells[0].parse().unwrap(),
                cells[1].parse().unwrap(),
                cells[5].parse().unwrap(),
            )
        })
        .collect();
    let at_target = rows.iter().copied().filter(|&(_, p, _)| p >= 0.99);
    let lowest_memory_cap = at_target
        .clone()
        .map(|(cap, _, rss)| (rss, cap))
        .min()
        .unwrap()
        .1;
    let smallest_passing_cap = at_target.map(|(cap, _, _)| cap).min().unwrap();
    let manual_cap = rows.iter().map(|&(cap, _, _)| cap).min().unwrap();
    let mut app = QCSimApp::default();
    app.state.ui_state.show_welcome_screen = false;
    let ctx = egui::Context::default();
    click(&mut app, &ctx, "Settings");
    click(&mut app, &ctx, "MPS ladder experiment");
    frame(&mut app, &ctx, Vec::new());
    let output = frame(&mut app, &ctx, Vec::new());
    assert!(labels(&output).any(|(text, _)| text == "Selected cap 256"));
    assert!(labels(&output).any(|(text, _)| text.contains("122.1") && text.contains("MiB")));
    assert!(!labels(&output).any(|(text, _)| text == "Lowest memory"));
    click(&mut app, &ctx, "Target return");
    let output = frame(&mut app, &ctx, Vec::new());
    assert!(labels(&output).any(|(text, _)| text == format!("Selected cap {lowest_memory_cap}")));
    assert!(labels(&output).any(|(text, _)| text == "Lowest memory"));
    click(&mut app, &ctx, "Smallest cap");
    let output = frame(&mut app, &ctx, Vec::new());
    assert!(labels(&output).any(|(text, _)| text == format!("Selected cap {smallest_passing_cap}")));
    assert!(labels(&output).any(|(text, _)| text == "Target return"));
    let target_text = labels(&output)
        .find_map(|(text, _)| {
            (text.strip_suffix('%')?.parse::<f64>().ok()? == 99.0).then_some(text)
        })
        .expect("editable 99 percent target");
    click(&mut app, &ctx, target_text);
    frame(&mut app, &ctx, vec![Event::Text("0".into())]);
    let output = frame(&mut app, &ctx, Vec::new());
    assert!(labels(&output).any(|(text, _)| text == format!("Selected cap {manual_cap}")));

    click(&mut app, &ctx, "Manual");
    click(&mut app, &ctx, "32");
    click(&mut app, &ctx, &manual_cap.to_string());
    let output = frame(&mut app, &ctx, Vec::new());
    assert!(labels(&output).any(|(text, _)| text == format!("Selected cap {manual_cap}")));
    let displayed_return = labels(&output)
        .find_map(|(text, _)| text.strip_prefix("Measured return "))
        .expect("measured return for the selected cap");
    click(&mut app, &ctx, "Run echo");
    let deadline = Instant::now() + Duration::from_secs(120);
    let expected = format!("Run at cap {manual_cap} · return {displayed_return} ·");
    let running = format!("Running at cap {manual_cap}…");
    loop {
        let output = frame(&mut app, &ctx, Vec::new());
        if labels(&output).any(|(text, _)| text.starts_with(&expected)) {
            break;
        }
        assert!(
            labels(&output).any(|(text, _)| text == running),
            "The desktop worker did not start at cap {manual_cap}"
        );
        assert!(
            Instant::now() < deadline,
            "Missing completed result for cap {manual_cap}"
        );
        std::thread::sleep(Duration::from_millis(30));
    }
    assert!(app.state.simulation_state.quantum_state.is_none());
}
