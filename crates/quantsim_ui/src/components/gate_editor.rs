use crate::messages::Message;
use crate::state::AppState;
use egui::{DragValue, Frame, RichText, ScrollArea, Stroke};
use nalgebra::{Complex, DMatrix, Matrix2};
use quantsim_core::core::formatters::format_matrix;
use quantsim_core::core::gates::Gate;
use quantsim_core::core::types::{GateMatrix, Operation, Param};

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
        Gate::MOVE => "Logical MOVE gate.\n\nAction:\n- |00> -> |00>\n- |01> -> -i|10>\n- |10> -> -i|01>\n- |11> -> |11>\n\nConvention:\n- This is an idealized logical state-transfer abstraction with a -i transfer phase.\n- It is not a calibrated physical Jaynes-Cummings MOVE pulse.\n- Basis ordering is big-endian: |00>, |01>, |10>, |11>.",
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

fn render_semantics_panel(ui: &mut egui::Ui, gate: &Gate) {
    Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .stroke(Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.label(RichText::new("Gate Semantics").strong());
            ui.add_space(6.0);

            for section in gate_semantics_text(gate).split("\n\n") {
                let trimmed = section.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if let Some((title, body)) = trimmed.split_once(":\n") {
                    if title.starts_with("Matrix") {
                        continue;
                    }
                    ui.label(RichText::new(title).strong());
                    for line in body.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        if let Some(item) = line.strip_prefix("- ") {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("•").strong());
                                ui.label(item);
                            });
                        } else {
                            ui.label(line);
                        }
                    }
                } else {
                    ui.label(trimmed);
                }
                ui.add_space(8.0);
            }
        });
}

fn render_matrix_panel(ui: &mut egui::Ui, matrix: &DMatrix<nalgebra::Complex<f32>>) {
    let formatted = format_matrix(&GateMatrix::BigEndian(matrix.clone()).to_sparse_matrix());
    Frame::group(ui.style())
        .fill(ui.visuals().extreme_bg_color)
        .stroke(Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.label(RichText::new("Gate Matrix").strong());
            ui.small("Big-endian basis ordering");
            ui.add_space(6.0);
            ui.monospace(formatted);
        });
}

fn ideal_zero_density() -> Matrix2<Complex<f64>> {
    Matrix2::new(
        Complex::new(1.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
    )
}

fn plus_density() -> Matrix2<Complex<f64>> {
    let half = Complex::new(0.5, 0.0);
    Matrix2::new(half, half, half, half)
}

fn reference_preview_state(gate: &Gate) -> (&'static str, Matrix2<Complex<f64>>) {
    match gate {
        Gate::Z
        | Gate::SqrtZ
        | Gate::Rz
        | Gate::CRz
        | Gate::ZPow
        | Gate::CZPow
        | Gate::CCZPow => ("|+>", plus_density()),
        _ => ("|0>", ideal_zero_density()),
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
            ui.label(&gate_meta.description);
            ui.separator();
            let q_u32: Vec<u32> = op.qubits.iter().map(|&x| x).collect();
            let evaluated_matrix = state
                .circuit_state
                .circuit
                .registry
                .eval(&op.id, &op.params, &q_u32)
                .map(|matrix| matrix.to_dmatrix());

            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    if state.ui_state.selected_gate_for_editing.is_some() {
                        ui.label(RichText::new(format!("Applied to qubits: {:?}", op.qubits)).italics());
                        ui.add_space(6.0);
                    }

                    if !op.params.is_empty() {
                        Frame::group(ui.style())
                            .fill(ui.visuals().faint_bg_color)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new("Parameters").strong());
                                ui.add_space(4.0);
                                parametric_gate_editor(ui, &mut op, messages);
                            });
                        ui.add_space(8.0);
                    } else if matches!(gate_meta.id, Gate::Custom) {
                        ui.label("Custom gate editing is handled by the dedicated custom gate editor.");
                        ui.add_space(8.0);
                    }

                    if gate_meta.arity == quantsim_core::core::types::Arity::OneQ {
                        if let Some(matrix_d) = evaluated_matrix.as_ref() {
                            let matrix = nalgebra::Matrix2::new(
                                matrix_d[(0, 0)],
                                matrix_d[(0, 1)],
                                matrix_d[(1, 0)],
                                matrix_d[(1, 1)],
                            );
                            let matrix_f64 =
                                matrix.map(|c| nalgebra::Complex::new(c.re as f64, c.im as f64));
                            let (reference_label, reference_density) =
                                reference_preview_state(&gate_meta.id);
                            ui.label(RichText::new("Bloch Sphere").strong());
                            ui.small(format!(
                                "Preview shown from the reference input state {}.",
                                reference_label
                            ));
                            state.ui_state.bloch_sphere.draw(
                                ui,
                                &matrix_f64,
                                state.ui_state.bloch_sphere_animation_time,
                                reference_density,
                            );
                        }
                    } else if gate_meta.arity == quantsim_core::core::types::Arity::TwoQ {
                        ui.label(RichText::new("Control Qubits").strong());
                        for i in 0..state.circuit_state.num_qubits {
                            let mut is_control =
                                op.qubits.contains(&(i as u32)) && op.qubits[0] == i as u32;
                            if ui
                                .checkbox(&mut is_control, format!("Qubit {}", i))
                                .changed()
                            {
                                log::info!("Updating control qubit {} to {}", i, is_control);
                                messages.push(Message::UpdateGateControl(i, is_control));
                            }
                        }
                    } else if op.params.is_empty() && !matches!(gate_meta.id, Gate::Custom) {
                        ui.label("This gate has no editable parameters.");
                    }
                });

                columns[1].vertical(|ui| {
                    ScrollArea::vertical()
                        .id_salt("gate_editor_right")
                        .show(ui, |ui| {
                            render_semantics_panel(ui, &gate_meta.id);
                            ui.add_space(8.0);
                            if let Some(matrix_d) = evaluated_matrix.as_ref() {
                                render_matrix_panel(ui, matrix_d);
                            }
                        });
                });
            });
        }
    } else {
        ui.label("No gate selected or found.");
    }
}
