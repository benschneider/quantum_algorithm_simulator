use crate::{messages::Message, state::AppState};

pub fn handle_circuit_grid_message(
    state: &mut AppState,
    message: Message,
    _messages: &mut Vec<Message>,
) {
    match message {
        Message::InsertQft {
            after_step,
            first_wire,
            width,
        } => {
            let added = match state
                .circuit_state
                .circuit
                .insert_qft(after_step, first_wire, width)
            {
                Ok(added) => added,
                Err(error) => {
                    state.ui_state.error_message = Some(error);
                    return;
                }
            };
            state.circuit_state.num_timesteps = state.circuit_state.circuit.steps.len();
            super::simulation_handler::invalidate_edited_gate(state, after_step);
            let ui = &mut state.ui_state;
            if ui.current_timestep > after_step {
                ui.current_timestep += added;
            }
            if let Some((_, column)) = &mut ui.selected_gate_for_editing {
                if *column >= after_step {
                    *column += added;
                }
            }
            ui.section_editor = None;
            ui.placement_mode = crate::state::ui_state::PlacementMode::Idle;
            ui.selected_gate = None;
            ui.error_message = None;
        }
        Message::PlaceGate(gate, row, col) => {
            if state.circuit_state.place_gate(gate, row, col) {
                super::simulation_handler::invalidate_edited_gate(state, col);
            }
        }
        Message::PlaceMultiQubitGate(gate, qubits, col) => {
            if state
                .circuit_state
                .place_multi_qubit_gate(gate, qubits, col)
            {
                super::simulation_handler::invalidate_edited_gate(state, col);
            }
        }
        Message::MoveGate(from_row, from_col, to_row, to_col) => {
            if state
                .circuit_state
                .move_gate(from_row, from_col, to_row, to_col)
            {
                super::simulation_handler::invalidate_edited_gate(state, from_col.min(to_col));
            }
        }
        Message::DeleteGate(row, col) => {
            if state.circuit_state.delete_gate(row, col).is_some() {
                super::simulation_handler::invalidate_edited_gate(state, col);
            }
        }
        Message::SelectGateForEditing(row, col) => {
            state.ui_state.selected_gate_for_editing = Some((row, col));
            state.ui_state.placement_mode = crate::state::ui_state::PlacementMode::Idle;
            state.ui_state.show_gate_editor_window = true;
        }
        _ => {} //panic!("Unhandled message in circuit grid handler: {:?}", message); }
    }
}

#[cfg(test)]
mod tests {
    use crate::{messages::Message, QCSimApp};

    #[test]
    fn inserting_a_full_width_qft_preserves_the_following_gate_and_allows_phase_edits() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::UpdateCircuitFromJson(
            r#"{"num_qubits":64,"steps":[[{"id":"X","qubits":[59]}],[{"id":"H","qubits":[0]}]]}"#
                .into(),
        ));
        app.handle_message(Message::RunSimulation);
        app.handle_message(Message::SelectGateForEditing(0, 1));
        app.handle_message(Message::InsertQft {
            after_step: 1,
            first_wire: 0,
            width: 64,
        });
        assert_eq!(app.state.ui_state.error_message, None);
        assert_eq!(app.state.circuit_state.num_timesteps, 130);
        assert_eq!(app.state.ui_state.current_timestep, 130);
        assert_eq!(app.state.ui_state.selected_gate_for_editing, Some((0, 129)));
        assert!(!app.state.simulation_state.has_readout());
        let section = app.state.circuit_state.circuit.section_at(2).unwrap();
        assert_eq!(
            (section.label.as_str(), section.start, section.end),
            ("QFT", 2, 129)
        );

        // The original trailing H makes the lowest Fourier phase measurable on q0.
        app.handle_message(Message::RunSimulation);
        assert_eq!(app.state.ui_state.error_message, None);
        let expected = (std::f64::consts::PI / 32.0).sin().powi(2);
        let actual = app.state.simulation_state.wire_one_probabilities[0];
        assert!((f64::from(actual) - expected).abs() < 1e-6);

        app.handle_message(Message::SelectGateForEditing(59, 5));
        app.handle_message(Message::UpdateGateAngle(1.0 / 3.0));
        app.handle_message(Message::RunSimulation);
        assert_eq!(app.state.ui_state.error_message, None);
        assert!((app.state.simulation_state.wire_one_probabilities[0] - 0.25).abs() < 1e-6);
    }
}
