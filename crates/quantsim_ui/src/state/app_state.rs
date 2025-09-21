use super::{
    CircuitState, CustomGateEditorState, InitialStateEditorState, SimulationState, UIState,
};
use quantsim_core::circuits;

/// The `AppState` struct is the top-level container for all application
/// state. It is the single source of truth for the application, and it is
/// passed to all UI and event handler functions.
///
/// The `AppState` is composed of several sub-states, each of which is
/// responsible for a specific part of the application's state. This
/// separation of concerns makes the application easier to reason about and
/// maintain.
#[derive(Debug)]
pub struct AppState {
    pub circuit_state: CircuitState,
    /// The state of the user interface.
    pub ui_state: UIState,
    /// The state of the simulation.
    pub simulation_state: SimulationState,
    /// The state of the initial state editor.
    pub initial_state_editor_state: InitialStateEditorState,
    /// The state of the custom gate editor.
    pub custom_gate_editor_state: CustomGateEditorState,
    /// A list of template circuits that can be loaded.
    pub template_circuits: Vec<(String, String)>,
}

impl AppState {
    /// Creates a new `AppState` with default values.
    pub fn new() -> Self {
        // Load the built-in circuit templates from the `quantsim_core` library.
        // `get_circuit_names` provides the list of available template filenames,
        // and `get_circuit` retrieves the JSON content for each one.
        // This populates the dropdown menu in the UI for loading example circuits.
        let templates = circuits::get_circuit_names()
            .into_iter()
            .map(|name| {
                let content = circuits::get_circuit(&name).unwrap_or_default();
                (name, content)
            })
            .collect();

        let circuit_state = CircuitState::default();
        let num_qubits = circuit_state.circuit.num_qubits;

        Self {
            circuit_state,
            ui_state: Default::default(),
            simulation_state: Default::default(),
            initial_state_editor_state: InitialStateEditorState::new(num_qubits),
            custom_gate_editor_state: CustomGateEditorState::new(),
            template_circuits: templates,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
