use quantsim_core::{circuit::QubitRole, core::gates::Gate};

#[derive(Debug, Clone)]
pub enum Message {
    // Top Panel Messages
    LoadTemplate(String),
    SaveCircuit,
    NewCircuit,
    RunSimulation,
    TogglePlayback(f64),

    // Circuit Control Messages
    ChangeQubits(usize),
    SetQubitRole { row: usize, role: Option<QubitRole> },
    ChangeTimesteps(usize),
    SaveSection,
    SaveLayout,
    DeleteSection,
    CircuitDimensionsChanged {
        num_qubits: usize,
        num_timesteps: usize,
    },

    // Gate Palette Messages
    SelectGate(Gate),
    OpenGateEditor(Gate),

    // Circuit Grid Messages
    PlaceGate(Gate, usize, usize),
    PlaceMultiQubitGate(Gate, Vec<usize>, usize),
    MoveGate(usize, usize, usize, usize),
    DeleteGate(usize, usize),
    SelectGateForEditing(usize, usize),

    // Info Panel Messages
    UpdateGateAngle(f64),
    SwapControlAndTarget,

    // Custom Gate Definition Editor Messages
    OpenCustomGateEditor(Gate),
    UpdateCustomGateEditorValue {
        row: usize,
        col: usize,
        real: String,
        imag: String,
    },
    MakeCustomGateMatrixUnitary,
    SaveCustomGateMatrix,
    CloseCustomGateEditor,

    // JSON Editor Messages
    UpdateJsonFromCircuit,
    UpdateCircuitFromJson(String),
    CopyJsonToClipboard,
    FormatJson,
    ToggleInfoWindow,
    ToggleAboutWindow,
    ToggleTutorialWindow,
    SelectTimestep(usize),
    InspectLayoutSite { wire: usize, step: usize },

    // Initial State Editor Messages
    ToggleInitialStateEditor,
    ApplyInitialStateFromEditor(Vec<nalgebra::Complex<f64>>),
    ResetInitialState,
    InitialStateEditorPageChanged(usize),
    SimulationResultsPageChanged(usize),
}
