use crate::{messages::Message, state::AppState};
use std::time::Duration;

const STEP_SECONDS: f64 = 0.6;
const SECTION_READING_SECONDS: f64 = 8.0;

pub(super) fn toggle(state: &mut AppState, now: f64) {
    if state.ui_state.next_playback_step_at.take().is_some() {
        return;
    }
    let last = state.circuit_state.circuit.steps.len();
    if last > 0 {
        let current = state.ui_state.current_timestep;
        let start = if current >= last || state.ui_state.error_message.is_some() {
            0
        } else {
            current
        };
        show_step(state, start, now);
    }
}

pub(crate) fn advance(state: &mut AppState, now: f64) -> Option<Duration> {
    let due = state.ui_state.next_playback_step_at?;
    if !state.ui_state.show_results_window {
        state.ui_state.next_playback_step_at = None;
        return None;
    }
    if now >= due {
        // Advance once even if a background tab has missed several deadlines.
        show_step(state, state.ui_state.current_timestep + 1, now);
    }
    let next = state.ui_state.next_playback_step_at?;
    Some(Duration::from_secs_f64(next - now))
}

fn show_step(state: &mut AppState, step: usize, now: f64) {
    super::simulation_handler::handle_simulation_message(
        state,
        Message::SelectTimestep(step),
        &mut [],
    );
    let circuit = &state.circuit_state.circuit;
    let starts_description = circuit
        .section_at(step)
        .is_some_and(|section| section.start == step && !section.description.trim().is_empty());
    let delay = if starts_description {
        SECTION_READING_SECONDS
    } else {
        STEP_SECONDS
    };
    state.ui_state.next_playback_step_at = (step < circuit.steps.len()
        && state.ui_state.error_message.is_none())
    .then_some(now + delay);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QCSimApp;
    use quantsim_core::types::Gate;

    #[test]
    fn playback_gives_reading_time_once_on_entering_a_described_section() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::UpdateCircuitFromJson(
            r#"{"num_qubits":1,"steps":[[],[],[]],"sections":[{"start":1,"end":2,"label":"Preparation","description":"Follow the first two columns."}]}"#.into(),
        ));
        app.handle_message(Message::SelectTimestep(0));
        app.handle_message(Message::TogglePlayback(0.0));
        assert_eq!(advance(&mut app.state, 1.0), Some(Duration::from_secs(8)));
        advance(&mut app.state, 8.9);
        assert_eq!(app.state.ui_state.current_timestep, 1);
        advance(&mut app.state, 9.0);
        assert_eq!(app.state.ui_state.current_timestep, 2);
        assert_eq!(advance(&mut app.state, 9.6), None);
        assert_eq!(app.state.ui_state.current_timestep, 3);
    }

    #[test]
    fn playback_rewinds_advances_one_prefix_per_tick_and_stops_at_the_end() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::LoadTemplate("bell.json".into()));
        app.handle_message(Message::RunSimulation);
        app.state.ui_state.show_results_window = false;
        app.handle_message(Message::TogglePlayback(1.0));
        assert!(app.state.ui_state.show_results_window);
        assert_eq!(app.state.ui_state.current_timestep, 0);
        assert_eq!(
            app.state.simulation_state.wire_one_probabilities,
            [0.0, 0.0]
        );
        assert!(advance(&mut app.state, 1.2).is_some());
        assert_eq!(app.state.ui_state.current_timestep, 0);

        assert!(advance(&mut app.state, 8.0).is_some());
        assert_eq!(app.state.ui_state.current_timestep, 1);
        assert!((app.state.simulation_state.wire_one_probabilities[0] - 0.5).abs() < 1e-6);
        assert_eq!(app.state.simulation_state.wire_one_probabilities[1], 0.0);
        app.handle_message(Message::TogglePlayback(8.1));
        assert_eq!(advance(&mut app.state, 12.0), None);
        app.handle_message(Message::TogglePlayback(13.0));
        assert_eq!(app.state.ui_state.current_timestep, 1);
        assert_eq!(advance(&mut app.state, 14.0), None);
        assert_eq!(app.state.ui_state.current_timestep, 2);
        assert!((app.state.simulation_state.wire_one_probabilities[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn scrubbing_editing_running_and_an_unsupported_gate_stop_the_clock() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::LoadTemplate("bell.json".into()));
        for action in [
            Message::SelectTimestep(1),
            Message::PlaceGate(Gate::X, 1, 0),
            Message::RunSimulation,
            Message::LoadTemplate("bell.json".into()),
        ] {
            app.handle_message(Message::TogglePlayback(0.0));
            assert!(app.state.ui_state.next_playback_step_at.is_some());
            app.handle_message(action);
            assert_eq!(advance(&mut app.state, 2.0), None);
        }

        app.handle_message(Message::UpdateCircuitFromJson(
            r#"{"num_qubits":22,"steps":[[{"id":"H","qubits":[0]}],[],[]]}"#.into(),
        ));
        app.handle_message(Message::PlaceGate(Gate::Custom, 0, 1));
        app.handle_message(Message::TogglePlayback(0.0));
        assert!(advance(&mut app.state, 1.0).is_some());
        assert_eq!(advance(&mut app.state, 2.0), None);
        assert_eq!(app.state.ui_state.current_timestep, 2);
        assert!(app
            .state
            .ui_state
            .error_message
            .as_ref()
            .unwrap()
            .contains("explicit matrix"));
        app.handle_message(Message::TogglePlayback(3.0));
        assert_eq!(app.state.ui_state.current_timestep, 0);
        assert!(app.state.ui_state.error_message.is_none());
        app.state.ui_state.show_results_window = false;
        assert_eq!(advance(&mut app.state, 4.0), None);
        assert_eq!(app.state.ui_state.current_timestep, 0);
    }
}
