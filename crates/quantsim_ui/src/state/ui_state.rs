use quantsim_core::{
    circuit::{CircuitSection, QubitLayout},
    core::gates::Gate,
};

#[derive(Debug)]
pub struct SectionDraft {
    pub original_start: Option<usize>,
    pub section: CircuitSection,
    pub error: Option<String>,
}

impl SectionDraft {
    pub fn new(step: usize, existing: Option<&CircuitSection>) -> Self {
        Self {
            original_start: existing.map(|section| section.start),
            section: existing.cloned().unwrap_or_else(|| CircuitSection {
                start: step,
                end: step,
                label: String::new(),
            }),
            error: None,
        }
    }
}

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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LayoutView {
    Plane,
    Stack,
}

#[derive(Debug, Clone, Copy)]
pub struct StackCamera {
    /// Horizontal rotation and elevation in degrees.
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for StackCamera {
    fn default() -> Self {
        Self {
            yaw: -32.0,
            pitch: 18.0,
        }
    }
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
    TemplateGuide,
    SingleQubitGatesEntangledStates,
    SingleQubitGatesIntroduction,
    MultiQubitGates,
    Measurements,
    CustomGates,
    InitialStates,
    Simulations,
    Analytics,
    ErrorCorrection,
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
    pub tutorial_cache: egui_commonmark::CommonMarkCache,
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
    pub selected_layout_wire: Option<usize>,
    pub layout_view: LayoutView,
    pub stack_camera: StackCamera,
    pub layout_draft: Option<QubitLayout>,
    pub section_editor: Option<SectionDraft>,
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
            tutorial_cache: Default::default(),
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
            selected_layout_wire: None,
            layout_view: LayoutView::Plane,
            stack_camera: StackCamera::default(),
            layout_draft: None,
            section_editor: None,
            gate_editor_matrix: DMatrix::from_element(0, 0, Complex::new(0.0, 0.0)),
            is_unitary: true,
            unitarity_error: 0.0,
            error_message: None,
        }
    }
}
