use crate::{messages::Message, state::AppState};
use egui::Ui;
use quantsim_core::{core::schmidt::MAX_SCHMIDT_QUBITS, mps_ladder::CutOrder};

pub(super) fn show(ui: &mut Ui, state: &mut AppState) -> Option<Message> {
    let circuit = &state.circuit_state.circuit;
    let wires = circuit.num_qubits;
    if wires < 2 {
        ui.label("A cut needs at least two qubits.");
        return None;
    }
    let view = &mut state.ui_state;
    if wires >= 4 && wires % 2 == 0 {
        ui.horizontal_wrapped(|ui| {
            ui.label("Site order");
            let previous = view.cut_order;
            ui.selectable_value(&mut view.cut_order, CutOrder::WireOrder, "Wire order");
            ui.selectable_value(
                &mut view.cut_order,
                CutOrder::PairedColumns,
                "Paired columns",
            )
            .on_hover_text("Each site holds one wire from each half of the register.");
            if previous != view.cut_order {
                view.cut_left_sites = None;
            }
        });
    } else {
        view.cut_order = CutOrder::WireOrder;
    }
    let order = view.cut_order;
    let sites = order.site_count(wires).expect("valid order for this width");
    let left = view.cut_left_sites.get_or_insert(sites / 2);
    *left = (*left).clamp(1, sites - 1);
    ui.horizontal_wrapped(|ui| {
        let paired = order == CutOrder::PairedColumns;
        ui.label(if paired {
            "Cut after column"
        } else {
            "Cut after wire"
        });
        if ui
            .add_enabled(*left > 1, egui::Button::new("‹"))
            .on_hover_text("Move cut left")
            .clicked()
        {
            *left -= 1;
        }
        let offset = usize::from(!paired);
        let mut value = *left - offset;
        ui.add(
            egui::DragValue::new(&mut value)
                .range((1 - offset)..=(sites - 1 - offset))
                .prefix(if paired { "" } else { "q" }),
        );
        *left = value + offset;
        if ui
            .add_enabled(*left < sites - 1, egui::Button::new("›"))
            .on_hover_text("Move cut right")
            .clicked()
        {
            *left += 1;
        }
    });
    let left = *left;
    let side = |start: usize, end: usize| {
        let top = wire_range(start, end);
        if order == CutOrder::PairedColumns {
            format!("{top}, {}", wire_range(start + sites, end + sites))
        } else {
            top
        }
    };
    ui.horizontal_wrapped(|ui| {
        ui.monospace(side(0, left - 1));
        ui.separator();
        ui.monospace(side(left, sites - 1));
    });

    let step = view.current_timestep;
    let inspection = state
        .simulation_state
        .inspect_cut(circuit, step, order, left);
    ui.separator();
    egui::Grid::new("entanglement_cut_metrics").show(ui, |ui| {
        ui.label("");
        ui.strong("Gate bound")
            .on_hover_text("Upper limits from the product input and the gates through this step.");
        ui.strong("Dense state")
            .on_hover_text("Computed from the simulated amplitudes on the two sides of this cut.");
        ui.end_row();
        ui.label("Schmidt rank")
            .on_hover_text("How many product terms the state needs across this cut. Rank one means the two sides are separable.");
        match &inspection.rank_bound {
            Ok(rank) if rank.bits() > 20 => ui.label(format!("≤ 2^{}", rank.bits() - 1)),
            Ok(rank) => ui.label(format!("≤ {rank}")),
            Err(_) => ui.label("—"),
        };
        let measured = inspection.dense.as_ref().and_then(|r| r.as_ref().ok());
        ui.label(measured.map_or_else(|| "—".into(), |r| r.rank.to_string()))
            .on_hover_text("Numerical rank counts normalized Schmidt coefficients above 0.000001.");
        ui.end_row();
        ui.label("Entropy (bits)")
            .on_hover_text("Entanglement between the two sides. Equally weighted Schmidt terms give the largest entropy for a given rank.");
        ui.label(
            inspection
                .rank_bound
                .as_ref()
                .map_or_else(|_| "—".into(), |r| format!("≤ {}", r.bits() - 1)),
        );
        ui.label(measured.map_or_else(|| "—".into(), |r| format!("{:.4}", r.entropy_bits)));
        ui.end_row();
    });
    if let Err(reason) = &inspection.rank_bound {
        ui.label(format!("Gate bound unavailable. {reason}"));
    }
    if let Some(Err(reason)) = &inspection.dense {
        ui.label(format!("Dense calculation unavailable. {reason}"));
    }
    if wires > MAX_SCHMIDT_QUBITS {
        ui.small(format!(
            "Dense cut readout is available through {MAX_SCHMIDT_QUBITS} qubits."
        ));
    }
    ui.separator();
    if inspection.crossing_gates.is_empty() {
        ui.label("No gates cross this cut at this step.");
    } else {
        ui.label(format!("Crossing gates at step {step}"))
            .on_hover_text("Outlined in the circuit grid.");
        for &index in inspection.crossing_gates.iter().take(4) {
            let op = &circuit.steps[step - 1][index];
            let operands: Vec<_> = op.qubits.iter().map(|q| format!("q{q}")).collect();
            ui.colored_label(
                ui.visuals().hyperlink_color,
                format!("{}  {}", op.id, operands.join(" · ")),
            );
        }
        if inspection.crossing_gates.len() > 4 {
            ui.small(format!(
                "{} more in the circuit grid",
                inspection.crossing_gates.len() - 4
            ));
        }
    }
    if wires <= MAX_SCHMIDT_QUBITS && !state.simulation_state.has_readout() {
        return ui
            .button(format!("Read step {step}"))
            .clicked()
            .then_some(Message::SelectTimestep(step));
    }
    None
}

fn wire_range(first: usize, last: usize) -> String {
    if first == last {
        format!("q{first}")
    } else {
        format!("q{first}–q{last}")
    }
}
