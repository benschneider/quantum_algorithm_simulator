use crate::messages::Message;
use crate::state::memory_estimate::estimated_peak_bytes;
use crate::state::ui_state::{CentralPanelTab, SectionDraft};
use crate::state::AppState;
use egui::widgets::DragValue;

/// Renders the circuit controls, which include the "Run" button, qubit and
/// timestep controls, and the timestep slider.
///
/// # Arguments
///
/// * `ui` - The `egui` user interface.
/// * `state` - The application state.
/// * `messages` - A vector of messages to be sent to the application.
pub fn circuit_controls(ui: &mut egui::Ui, state: &mut AppState, messages: &mut Vec<Message>) {
    let circuit_view = state.ui_state.active_central_tab == CentralPanelTab::Circuit;
    ui.horizontal_wrapped(|ui| {
        if ui.button("Run ▶").clicked() {
            messages.push(Message::RunSimulation);
        }
        show_memory_estimate(ui, state);

        ui.separator();

        let mut num_qubits = state.circuit_state.num_qubits;
        ui.label("Qubits:");
        if ui
            .add(DragValue::new(&mut num_qubits).range(1..=usize::MAX))
            .changed()
        {
            messages.push(Message::ChangeQubits(num_qubits));
        }

        let mut num_timesteps = state.circuit_state.num_timesteps;
        ui.label("Timesteps:");
        if ui
            .add(
                DragValue::new(&mut num_timesteps)
                    .range(1..=1000)
                    .clamp_existing_to_range(false),
            )
            .changed()
        {
            messages.push(Message::ChangeTimesteps(num_timesteps));
        }

        let mut current_timestep = state.ui_state.current_timestep;
        if ui
            .add(
                egui::Slider::new(&mut current_timestep, 0..=state.circuit_state.num_timesteps)
                    .text("Timestep"),
            )
            .changed()
        {
            messages.push(Message::SelectTimestep(current_timestep));
        }
        let last_step = state.circuit_state.circuit.steps.len();
        if ui
            .add_enabled(current_timestep > 0, egui::Button::new("Previous step"))
            .clicked()
        {
            messages.push(Message::SelectTimestep(current_timestep - 1));
        }
        let playback_label = if state.ui_state.next_playback_step_at.is_some() {
            "Pause"
        } else {
            "Play"
        };
        if ui
            .add_enabled(last_step > 0, egui::Button::new(playback_label))
            .on_hover_text("Advance one timestep every two seconds, or pause playback")
            .clicked()
        {
            messages.push(Message::TogglePlayback(ui.input(|i| i.time)));
        }
        if ui
            .add_enabled(current_timestep < last_step, egui::Button::new("Next step"))
            .clicked()
        {
            messages.push(Message::SelectTimestep(current_timestep + 1));
        }
        if circuit_view
            && ui
                .add_enabled(last_step > 0, egui::Button::new("Sections"))
                .clicked()
        {
            let step = current_timestep.clamp(1, last_step);
            let existing = state.circuit_state.circuit.section_at(step);
            state.ui_state.section_editor = Some(SectionDraft::new(step, existing));
        }
    });

    if circuit_view {
        super::section_editor::show(ui.ctx(), state, messages);
    }
}

const MIB: u64 = 1 << 20;
const DEFAULT_WARNING_MIB: u64 = if cfg!(target_arch = "wasm32") {
    64
} else {
    128
};

fn ram_label(bytes: Option<u64>, warning_mib: u64) -> (String, bool) {
    let Some(bytes) = bytes else {
        return ("Peak RAM estimate unavailable".into(), false);
    };
    let mut label = if bytes == u64::MAX {
        "RAM upper bound ≥ 16 EiB".into()
    } else if bytes < MIB {
        format!("Est. peak RAM ≤ {} KiB", bytes.div_ceil(1024))
    } else if bytes < 1 << 30 {
        format!("Est. peak RAM ≤ {} MiB", bytes.div_ceil(MIB))
    } else {
        let order = bytes.ilog2() / 10;
        let unit = ["GiB", "TiB", "PiB", "EiB"][(order - 3) as usize];
        let tenths = (10 * u128::from(bytes)).div_ceil(1u128 << (10 * order));
        format!("Est. peak RAM ≤ {}.{} {unit}", tenths / 10, tenths % 10)
    };
    let above = u128::from(bytes) > u128::from(warning_mib) * u128::from(MIB);
    if above {
        label.push_str(&format!(" · above {warning_mib} MiB warning level"));
    }
    (label, above)
}

fn show_memory_estimate(ui: &mut egui::Ui, state: &AppState) {
    // eframe saves egui's persisted data on both web and native.
    let id = egui::Id::new("ram_warning_mib");
    let mut warning_mib = ui
        .data_mut(|data| data.get_persisted(id))
        .unwrap_or(DEFAULT_WARNING_MIB);
    let (label, above) = ram_label(estimated_peak_bytes(state), warning_mib);
    let mut text = egui::RichText::new(label);
    if above {
        text = text.color(ui.visuals().warn_fg_color).strong();
    }
    egui::ComboBox::from_id_salt("ram_warning_menu")
        .selected_text(text)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Warn above");
                if ui
                    .add(
                        DragValue::new(&mut warning_mib)
                            .range(0..=u64::MAX / MIB)
                            .suffix(" MiB"),
                    )
                    .changed()
                {
                    ui.data_mut(|data| data.insert_persisted(id, warning_mib));
                }
            });
            ui.label("You can run circuits above this level.");
            ui.separator();
            ui.collapsing("Estimate MPS cap", |ui| {
                ui.set_width((ui.ctx().screen_rect().width() - 40.0).clamp(1.0, 300.0));
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                let circuit = &state.circuit_state.circuit;
                match quantsim_core::mps_ladder::circuit_bound(circuit) {
                    Ok(bound) => {
                        let rank = &bound.uniform_cap;
                        let cap = if rank.bits() > 64 && rank.count_ones() == 1 {
                            format!("2^{}", rank.bits() - 1)
                        } else {
                            rank.to_string()
                        };
                        ui.strong(format!("Sufficient bond cap  {cap}")).on_hover_text(
                            "This upper bound avoids bond truncation in exact arithmetic.",
                        );
                        let payload = match u64::try_from(&bound.tensor_bytes) {
                            Ok(bytes) if bytes < 1024 => format!("{bytes} bytes"),
                            Ok(bytes) if bytes < MIB => format!("{} KiB", bytes.div_ceil(1024)),
                            Ok(bytes) => format!("{} MiB", bytes.div_ceil(MIB)),
                            Err(_) => "at least 16 EiB".into(),
                        };
                        ui.label(format!("Retained tensor upper bound  {payload}"));
                        ui.label(format!(
                            "Each column pairs wire j with wire j + {}.",
                            circuit.num_qubits / 2
                        ));
                        ui.label("A smaller cap may suffice for this input. Factorization needs extra workspace.");
                    }
                    Err(reason) => {
                        ui.label(reason);
                    }
                }
                ui.weak("Editor Run uses dense, stabilizer, product, or grouped simulation.");
            });
        })
        .response
        .on_hover_text(
            "Estimated upper bound for the full run’s simulation buffers. Total app memory can be higher. Click for the RAM warning and MPS cap estimate.",
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ram_warning_compares_exact_bytes_and_handles_unavailable_estimates() {
        assert!(!ram_label(Some(4 * MIB), 4).1);
        assert!(ram_label(Some(4 * MIB + 1), 4).1);
        assert!(!ram_label(Some(4 * MIB + 1), 5).1);
        assert!(ram_label(Some(u64::MAX), u64::MAX / MIB).1);
        assert!(ram_label(Some(u64::MAX), 64).0.contains("≥ 16 EiB"));
        assert!(ram_label(Some((3 << 30) + 1), 64).0.contains("≤ 3.1 GiB"));
        assert!(ram_label(None, 64).0.contains("unavailable"));
    }
}
