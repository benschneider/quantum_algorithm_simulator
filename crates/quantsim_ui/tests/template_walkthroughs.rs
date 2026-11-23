use nalgebra::Complex;
use quantsim_ui::{messages::Message, QCSimApp};

#[path = "template_checks/entanglement.rs"]
mod entanglement;
#[path = "template_checks/fourier.rs"]
mod fourier;
#[path = "template_checks/search.rs"]
mod search;

fn load(stem: &str) -> QCSimApp {
    let mut app = QCSimApp::default();
    app.handle_message(Message::LoadTemplate(format!("{stem}.json")));
    assert_eq!(app.state.ui_state.error_message, None, "{stem}");
    app
}

fn at_step(app: &mut QCSimApp, step: usize) -> &[Complex<f32>] {
    app.handle_message(Message::SelectTimestep(step));
    assert_eq!(app.state.ui_state.current_timestep, step);
    assert_eq!(app.state.ui_state.error_message, None, "step {step}");
    app.state
        .simulation_state
        .quantum_state
        .as_ref()
        .unwrap()
        .state_vector
        .as_slice()
}
