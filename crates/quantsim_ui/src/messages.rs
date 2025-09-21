use quantsim_core::core::gates::Gate;

#[derive(Debug, Clone)]
pub enum Message {
    // Top Panel Messages
    LoadTemplate(String),
    SaveCircuit,
    NewCircuit,
    RunSimulation,

    // Circuit Control Messages
    ChangeQubits(usize),
    ChangeTimesteps(usize),
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
    UpdateGateControl(usize, bool),
    UpdateCustomGate(Gate, nalgebra::DMatrix<nalgebra::Complex<f32>>),

    // Custom Gate Definition Editor Messages
    OpenCustomGateEditor(Gate),
    UpdateCustomGateEditorValue {
        row: usize,
        col: usize,
        real: String,
        imag: String,
    },
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

    // Initial State Editor Messages
    ToggleInitialStateEditor,
    ApplyInitialStateFromEditor(Vec<nalgebra::Complex<f64>>),
    ResetInitialState,
    InitialStateEditorPageChanged(usize),
    SimulationResultsPageChanged(usize),
}
