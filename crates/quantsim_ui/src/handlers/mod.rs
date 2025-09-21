pub mod circuit_controls_handler;
pub mod circuit_grid_handler;
pub mod gate_palette_handler;
pub mod info_panel_handler;
pub mod initial_state_editor_handler;
pub mod json_editor_handler;
pub mod menu_bar_handler;
pub mod new_circuit_handler;
pub mod simulation_handler;

use crate::{messages::Message, state::AppState};

/// The `handle_message` function is the central message handler for the
/// application. It takes a message and the current application state, and
/// then delegates the message to the appropriate handler based on the
/// message type.
///
/// This function is the core of the application's logic. It is where all
/// state changes are initiated.
pub fn handle_message(state: &mut AppState, message: Message, messages: &mut Vec<Message>) {
    match message {
        Message::NewCircuit => new_circuit_handler::new_circuit_handler(state),
        Message::LoadTemplate(_)
        | Message::SaveCircuit
        | Message::ToggleInfoWindow
        | Message::ToggleAboutWindow
        | Message::ToggleTutorialWindow => {
            menu_bar_handler::handle_menu_bar_message(state, message, messages)
        }
        Message::RunSimulation
        | Message::SelectTimestep(_)
        | Message::SimulationResultsPageChanged(_) => {
            simulation_handler::handle_simulation_message(state, message, messages)
        }
        Message::ChangeQubits(_)
        | Message::ChangeTimesteps(_)
        | Message::CircuitDimensionsChanged { .. } => {
            circuit_controls_handler::handle_circuit_controls_message(state, message, messages)
        }
        Message::SelectGate(_)
        | Message::OpenGateEditor(_)
        | Message::OpenCustomGateEditor(_)
        | Message::UpdateCustomGateEditorValue { .. }
        | Message::SaveCustomGateMatrix
        | Message::CloseCustomGateEditor => {
            gate_palette_handler::handle_gate_palette_message(state, message, messages)
        }
        Message::PlaceGate(_, _, _)
        | Message::PlaceMultiQubitGate(_, _, _)
        | Message::MoveGate(_, _, _, _)
        | Message::DeleteGate(_, _)
        | Message::SelectGateForEditing(_, _) => {
            circuit_grid_handler::handle_circuit_grid_message(state, message, messages)
        }
        Message::UpdateGateAngle(_)
        | Message::UpdateGateControl(_, _)
        | Message::UpdateCustomGate(_, _) => {
            info_panel_handler::handle_info_panel_message(state, message, messages)
        }
        Message::UpdateJsonFromCircuit
        | Message::UpdateCircuitFromJson(_)
        | Message::CopyJsonToClipboard
        | Message::FormatJson => {
            json_editor_handler::handle_json_editor_message(state, message, messages)
        }
        Message::ToggleInitialStateEditor
        | Message::ApplyInitialStateFromEditor(_)
        | Message::ResetInitialState
        | Message::InitialStateEditorPageChanged(_) => {
            initial_state_editor_handler::handle_initial_state_editor_message(
                state, message, messages,
            )
        }
    }
}
