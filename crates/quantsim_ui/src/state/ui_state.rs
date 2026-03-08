use quantsim_core::core::gates::Gate;

/// Represents the selected tab in the left panel.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum LeftPanelTab {
    GatePalette,
}

/// Represents the selected tab in the central panel.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum CentralPanelTab {
    Circuit,
    JsonEditor,
}

/// Represents the selected tab in the info panel.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum InfoPanelTab {
    GateReference,
}

/// Represents the current step in the tutorial.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TutorialStep {
    TutorialIntro,
    SingleQubitGatesEntangledStates,
    SingleQubitGatesIntroduction,
    MultiQubitGates,
    Measurements,
    CustomGates,
    InitialStates,
    Simulations,
    Analytics,
    Conclusion,
}

/// Represents the current step in the welcome screen.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum WelcomeStep {
    Welcome,
    License,
    TutorialOffer,
}

/// Represents the current placement mode for gates on the circuit grid.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum PlacementMode {
    Idle,
    Placing,
    Moving,
    PendingPlacement { qubits: Vec<usize>, col: usize },
}

impl Default for PlacementMode {
    fn default() -> Self {
        Self::Idle
    }
}

/// Represents an item being dragged on the circuit grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DraggedItem {
    Gate(Gate),
    GridGate(usize, usize),
}

use nalgebra::{Complex, DMatrix};

/// The `UIState` struct contains all the state related to the user interface.
/// This includes the state of the various panels, windows, and editors, as
/// well as any other UI-specific state.
///
/// This struct is used by the UI components to render themselves, and it is
/// updated by the message handlers in response to user actions.
#[derive(Debug)]
pub struct UIState {
    pub placement_mode: PlacementMode,
    pub active_left_tab: LeftPanelTab,
    pub active_central_tab: CentralPanelTab,
    pub active_info_tab: InfoPanelTab,
    pub active_tutorial_step: TutorialStep,
    pub active_welcome_step: WelcomeStep,
    pub selected_gate: Option<Gate>,
    pub selected_gate_for_editing: Option<(usize, usize)>,
    pub palette_gate_for_editing: Option<Gate>,
    pub circuit_json_string: String,
    pub bloch_sphere_animation_time: f64,
    pub bloch_sphere: crate::components::bloch_sphere_view::BlochSphere,
    pub show_initial_state_editor: bool,
    pub show_gate_editor_window: bool,
    pub show_results_window: bool,
    pub show_info_window: bool,
    pub show_about_window: bool,
    pub show_tutorial_window: bool,
    pub show_welcome_screen: bool,
    pub current_timestep: usize,
    pub gate_editor_matrix: DMatrix<Complex<f32>>,
    pub is_unitary: bool,
    pub unitarity_error: f32,
    pub error_message: Option<String>,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            placement_mode: PlacementMode::Idle,
            active_left_tab: LeftPanelTab::GatePalette,
            active_central_tab: CentralPanelTab::Circuit,
            active_info_tab: InfoPanelTab::GateReference,
            active_tutorial_step: TutorialStep::TutorialIntro,
            active_welcome_step: WelcomeStep::Welcome,
            selected_gate: None,
            selected_gate_for_editing: None,
            palette_gate_for_editing: None,
            circuit_json_string: String::new(),
            bloch_sphere_animation_time: 0.0,
            bloch_sphere: crate::components::bloch_sphere_view::BlochSphere::new(),
            show_initial_state_editor: false,
            show_gate_editor_window: false,
            show_results_window: false,
            show_info_window: false,
            show_about_window: false,
            show_tutorial_window: false,
            show_welcome_screen: true,
            current_timestep: 0,
            gate_editor_matrix: DMatrix::from_element(0, 0, Complex::new(0.0, 0.0)),
            is_unitary: true,
            unitarity_error: 0.0,
            error_message: None,
        }
    }
}
