use crate::messages::Message;
use crate::state::AppState;
use egui::DragValue;
use quantsim_core::core::gates::Gate;
use quantsim_core::core::types::{Operation, Param};

fn gate_semantics_text(gate: &Gate) -> &'static str {
    match gate {
        Gate::H => "Hadamard gate.\n\nMatrix in basis |0>, |1>:\n    (1/sqrt(2)) * [[1, 1], [1, -1]]\n\nAction:\n- |0> -> (|0> + |1>) / sqrt(2)\n- |1> -> (|0> - |1>) / sqrt(2)\n\nConvention:\n- Single-qubit basis ordering is big-endian: |0>, |1>.",
        Gate::X => "Pauli-X gate.\n\nMatrix in basis |0>, |1>:\n    [[0, 1], [1, 0]]\n\nAction:\n- |0> -> |1>\n- |1> -> |0>",
        Gate::Y => "Pauli-Y gate.\n\nMatrix in basis |0>, |1>:\n    [[0, -i], [i, 0]]\n\nAction:\n- |0> -> i|1>\n- |1> -> -i|0>\n\nConvention:\n- Relative phase matters; probabilities alone do not fully describe the result.",
        Gate::Z => "Pauli-Z gate.\n\nMatrix in basis |0>, |1>:\n    [[1, 0], [0, -1]]\n\nAction:\n- |0> -> |0>\n- |1> -> -|1>",
        Gate::SqrtX => "Square-root X gate.\n\nA principal square root of X such that (SqrtX)^2 = X up to numerical tolerance.\n\nConvention:\n- Acts on the big-endian single-qubit basis |0>, |1>.",
        Gate::SqrtY => "Square-root Y gate.\n\nA principal square root of Y such that (SqrtY)^2 = Y up to numerical tolerance.\n\nConvention:\n- Acts on the big-endian single-qubit basis |0>, |1>.",
        Gate::SqrtZ => "Square-root Z gate (the S gate).\n\nMatrix in basis |0>, |1>:\n    [[1, 0], [0, i]]\n\nAction:\n- |0> -> |0>\n- |1> -> i|1>",
        Gate::Rx => "Rotation about the X axis.\n\nParameter:\n- theta in radians\n\nMatrix:\n    Rx(theta) = cos(theta/2) I - i sin(theta/2) X\n\nAction examples:\n- Rx(pi) |0> = -i|1>\n- Rx(pi/2) |0> = (|0> - i|1>) / sqrt(2)\n\nConvention:\n- Matches the standard physics convention.\n- Single-qubit basis ordering is |0>, |1>.",
        Gate::Ry => "Rotation about the Y axis.\n\nParameter:\n- theta in radians\n\nMatrix:\n    Ry(theta) = cos(theta/2) I - i sin(theta/2) Y\n\nAction examples:\n- Ry(pi) |0> = |1>\n- Ry(pi/2) |0> = (|0> + |1>) / sqrt(2)\n\nConvention:\n- Matches the standard physics convention.\n- Single-qubit basis ordering is |0>, |1>.",
        Gate::Rz => "Rotation about the Z axis.\n\nParameter:\n- theta in radians\n\nMatrix:\n    Rz(theta) = diag(exp(-i theta/2), exp(+i theta/2))\n\nAction:\n- |0> -> exp(-i theta/2) |0>\n- |1> -> exp(+i theta/2) |1>\n\nConvention:\n- Matches the standard physics convention.\n- Single-qubit basis ordering is |0>, |1>.",
        Gate::CX => "Controlled-X gate (CNOT).\n\nMatrix in big-endian computational basis |00>, |01>, |10>, |11>:\n    [[1, 0, 0, 0],\n     [0, 1, 0, 0],\n     [0, 0, 0, 1],\n     [0, 0, 1, 0]]\n\nAction:\n- |00> -> |00>\n- |01> -> |01>\n- |10> -> |11>\n- |11> -> |10>\n\nConvention:\n- Qubit list is interpreted as [control, target].\n- Basis ordering shown here is big-endian.",
        Gate::CY => "Controlled-Y gate.\n\nMatrix in big-endian computational basis |00>, |01>, |10>, |11>:\n    block-diag(I, Y)\n\nAction:\n- Leaves |00>, |01> unchanged.\n- Applies Y to the target when the control is |1>.\n\nConvention:\n- Qubit list is interpreted as [control, target].\n- Basis ordering shown here is big-endian.",
        Gate::CZ => "Controlled-Z gate.\n\nMatrix in big-endian computational basis |00>, |01>, |10>, |11>:\n    diag(1, 1, 1, -1)\n\nAction:\n- |00> -> |00>\n- |01> -> |01>\n- |10> -> |10>\n- |11> -> -|11>\n\nConvention:\n- First qubit is the control / most significant qubit.\n- Basis ordering is big-endian.",
        Gate::SWAP => "SWAP gate.\n\nMatrix in big-endian computational basis |00>, |01>, |10>, |11>:\n    [[1, 0, 0, 0],\n     [0, 0, 1, 0],\n     [0, 1, 0, 0],\n     [0, 0, 0, 1]]\n\nAction:\n- |01> <-> |10>\n- |00> and |11> are unchanged.",
        Gate::CCNOT => "Toffoli gate (CCNOT).\n\nAction:\n- Flips the target qubit iff both control qubits are |1>.\n\nConvention:\n- Qubit list is interpreted as [control1, control2, target].",
        Gate::CCZ => "Controlled-controlled-Z gate.\n\nAction:\n- Applies a phase of -1 only to |111>.\n\nConvention:\n- Qubit list is interpreted as [control1, control2, target-like ordering].\n- In big-endian basis, only the |111> amplitude changes sign.",
        Gate::CRz => "Controlled phase gate used by this simulator.\n\nParameter:\n- theta in radians\n\nMatrix in big-endian computational basis |00>, |01>, |10>, |11>:\n    diag(1, 1, 1, exp(i theta))\n\nAction:\n- Only the |11> amplitude acquires phase exp(i theta).\n\nConvention:\n- This is a controlled phase form, not the block-diagonal controlled-Rz(theta) matrix.",
        Gate::XPow => "Cirq-style XPow exponent gate.\n\nParameter:\n- t, an exponent, not a physical angle\n\nDefinition:\n- XPow(t) = exp(i pi t / 2) Rx(pi t)\n\nConvention:\n- Matches Cirq up to global phase when compared to Rx(pi t).",
        Gate::YPow => "Cirq-style YPow exponent gate.\n\nParameter:\n- t, an exponent, not a physical angle\n\nDefinition:\n- YPow(t) = exp(i pi t / 2) Ry(pi t)\n\nConvention:\n- Matches Cirq up to global phase when compared to Ry(pi t).",
        Gate::ZPow => "Cirq-style ZPow exponent gate.\n\nParameter:\n- t, an exponent, not a physical angle\n\nMatrix:\n    ZPow(t) = diag(1, exp(i pi t))\n\nConvention:\n- Matches Cirq exactly in the big-endian single-qubit basis.",
        Gate::CXPow => "Cirq-style controlled XPow exponent gate.\n\nParameter:\n- t, an exponent, not a physical angle\n\nDefinition:\n- Controlled block is XPow(t)\n- CXPow(1) = CX\n\nConvention:\n- Matches the intended full-gate exponent semantics.\n- Any comparison to Rx-based forms may differ only by global phase.",
        Gate::CYPow => "Cirq-style controlled YPow exponent gate.\n\nParameter:\n- t, an exponent, not a physical angle\n\nDefinition:\n- Controlled block is YPow(t)\n- CYPow(1) = CY\n\nConvention:\n- Any comparison to Ry-based forms may differ only by global phase.",
        Gate::CZPow => "Cirq-style controlled ZPow exponent gate.\n\nParameter:\n- t, an exponent, not a physical angle\n\nMatrix in big-endian basis:\n    diag(1, 1, 1, exp(i pi t))\n\nAction:\n- Only the |11> amplitude acquires phase exp(i pi t)\n- CZPow(1) = CZ",
        Gate::CCZPow => "Cirq-style controlled-controlled ZPow exponent gate.\n\nParameter:\n- t, an exponent, not a physical angle\n\nAction:\n- Only the |111> amplitude acquires phase exp(i pi t).",
        Gate::Custom => "Custom user-defined gate.\n\nConvention:\n- The supplied matrix is interpreted in big-endian basis order.\n- The matrix should be unitary for physical simulation results.",
    }
}

fn parametric_gate_editor(ui: &mut egui::Ui, op: &mut Operation, messages: &mut Vec<Message>) {
    if let Some(param) = op.params.get_mut(0) {
        let Param::Scalar(angle) = param;
        let mut angle_degrees = angle.to_degrees();
        ui.label("Angle (degrees):");
        if ui
            .add(DragValue::new(&mut angle_degrees).speed(1.0))
            .changed()
        {
            *angle = angle_degrees.to_radians();
            log::info!("Updating gate angle to {}", angle);
            messages.push(Message::UpdateGateAngle(*angle as f64));
        }
    }
}

/// Renders the gate editor panel, which displays information about the
/// selected gate and allows the user to edit its parameters.
///
/// # Arguments
///
/// * `state` - The application state.
/// * `ui` - The `egui` user interface.
/// * `messages` - A vector of messages to be sent to the application.
pub fn gate_editor_panel(state: &mut AppState, ui: &mut egui::Ui, messages: &mut Vec<Message>) {
    let mut op: Option<Operation> = None;

    if let Some((row, col)) = state.ui_state.selected_gate_for_editing {
        op = state
            .circuit_state
            .circuit
            .steps
            .get(col)
            .and_then(|step| step.iter().find(|op| op.qubits.contains(&(row as u32))))
            .cloned();
    } else if let Some(gate_id) = state.ui_state.palette_gate_for_editing.clone() {
        let gate_meta = state.circuit_state.circuit.registry.get_meta(&gate_id);
        if let Some(gate_meta) = gate_meta {
            let params = if gate_meta.is_parametric {
                vec![0.0]
            } else {
                vec![]
            };
            op = Some(Operation::new_for_editing(
                gate_id.clone(),
                params,
                gate_meta.arity,
            ));
        }
    }

    if let Some(mut op) = op {
        let gate_meta = state
            .circuit_state
            .circuit
            .registry
            .get_meta(&op.id)
            .cloned();
        if let Some(gate_meta) = gate_meta {
            ui.heading(gate_meta.id.to_string());
            //ui.label(&gate_meta.label);
            ui.label(&gate_meta.description);
            ui.separator();
            ui.label("Gate Semantics:");
            ui.monospace(gate_semantics_text(&gate_meta.id));
            ui.separator();

            if state.ui_state.selected_gate_for_editing.is_some() {
                ui.label(format!("Applied to qubits: {:?}", op.qubits));
            }

            if !op.params.is_empty() {
                parametric_gate_editor(ui, &mut op, messages);
            } else if matches!(gate_meta.id, Gate::Custom) {
                ui.label("Custom gate editing is handled by the dedicated custom gate editor.");
            } else {
                ui.label("This gate has no editable parameters.");
            }

            // Display Bloch sphere for 1-qubit gates
            if gate_meta.arity == quantsim_core::core::types::Arity::OneQ {
                let q_u32: Vec<u32> = op.qubits.iter().map(|&x| x).collect();
                if let Some(gate_matrix) = &state
                    .circuit_state
                    .circuit
                    .registry
                    .eval(&op.id, &op.params, &q_u32)
                {
                    let matrix_d = gate_matrix.to_dmatrix();
                    let matrix = nalgebra::Matrix2::new(
                        matrix_d[(0, 0)],
                        matrix_d[(0, 1)],
                        matrix_d[(1, 0)],
                        matrix_d[(1, 1)],
                    );
                    let matrix_f64 =
                        matrix.map(|c| nalgebra::Complex::new(c.re as f64, c.im as f64));
                    ui.separator();
                    ui.label("Bloch Sphere Visualization:");
                    ui.checkbox(
                        &mut state.ui_state.use_incoming_bloch_state,
                        "Use incoming state",
                    );
                    let initial_state = if state.ui_state.use_incoming_bloch_state {
                        if let Some((row, col)) = state.ui_state.selected_gate_for_editing {
                            let mut quantum_state = quantsim_core::core::engine::QuantumState::new(
                                state.circuit_state.num_qubits,
                            );
                            for i in 0..col {
                                for operation in &mut state.circuit_state.circuit.steps[i] {
                                    quantum_state.apply_operation(
                                        operation,
                                        &state.circuit_state.circuit.registry,
                                    );
                                }
                            }
                            let mut state_vector = nalgebra::Vector2::new(
                                nalgebra::Complex::new(0.0, 0.0),
                                nalgebra::Complex::new(0.0, 0.0),
                            );
                            state_vector[(0, 0)] = nalgebra::Complex::new(
                                quantum_state.state_vector[row].re as f64,
                                quantum_state.state_vector[row].im as f64,
                            );
                            state_vector[(1, 0)] = nalgebra::Complex::new(
                                quantum_state.state_vector[row + 1].re as f64,
                                quantum_state.state_vector[row + 1].im as f64,
                            );
                            state_vector
                        } else {
                            nalgebra::Vector2::new(
                                nalgebra::Complex::new(1.0, 0.0),
                                nalgebra::Complex::new(0.0, 0.0),
                            )
                        }
                    } else {
                        nalgebra::Vector2::new(
                            nalgebra::Complex::new(1.0, 0.0),
                            nalgebra::Complex::new(0.0, 0.0),
                        )
                    };
                    state.ui_state.bloch_sphere.draw(
                        ui,
                        &matrix_f64,
                        state.ui_state.bloch_sphere_animation_time,
                        initial_state,
                    );
                }
            }

            // Display control qubit options for 2-qubit gates
            if gate_meta.arity == quantsim_core::core::types::Arity::TwoQ {
                ui.separator();
                ui.label("Control Qubits:");
                for i in 0..state.circuit_state.num_qubits {
                    let mut is_control = op.qubits.contains(&(i as u32)) && op.qubits[0] == i as u32;
                    if ui
                        .checkbox(&mut is_control, format!("Qubit {}", i))
                        .changed()
                    {
                        log::info!("Updating control qubit {} to {}", i, is_control);
                        messages.push(Message::UpdateGateControl(i, is_control));
                    }
                }
            }

            // Display the gate matrix
            let q_u32: Vec<u32> = op.qubits.iter().map(|&x| x).collect();
            if let Some(matrix_to_display) = state
                .circuit_state
                .circuit
                .registry
                .eval(&op.id, &op.params, &q_u32)
            {
                ui.separator();
                ui.label("Gate Matrix:");
                ui.label(format!("{}", matrix_to_display));
            }
        }
    } else {
        ui.label("No gate selected or found.");
    }
}
