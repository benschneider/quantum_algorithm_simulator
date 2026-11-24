use crate::state::SimulationState;
use egui::{Color32, Sense, Ui, Vec2};
use quantsim_core::{circuit::Circuit, types::QubitPrep};

/// Computational-basis marginals for the step shown in the layout.
#[derive(Clone, Copy)]
pub(super) enum SiteReadout<'a> {
    Measured(&'a [f64]),
    Input(&'a [Option<QubitPrep>]),
    Unavailable,
}

impl<'a> SiteReadout<'a> {
    pub(super) fn for_step(
        circuit: &'a Circuit,
        step: usize,
        readout: &'a SimulationState,
    ) -> Self {
        if readout.wire_one_probabilities.len() == circuit.num_qubits {
            Self::Measured(&readout.wire_one_probabilities)
        } else if step == 0 && circuit.initial_state.is_none() {
            Self::Input(&circuit.initial_preps)
        } else {
            Self::Unavailable
        }
    }

    pub(super) fn probability(self, wire: usize) -> Option<f64> {
        match self {
            Self::Measured(values) => values.get(wire).copied().map(|p| p.clamp(0.0, 1.0)),
            Self::Input(preps) => Some(match preps.get(wire).copied().flatten() {
                None | Some(QubitPrep::Zero) => 0.0,
                Some(QubitPrep::One) => 1.0,
                Some(QubitPrep::Plus | QubitPrep::Minus) => 0.5,
            }),
            Self::Unavailable => None,
        }
    }
}

pub(super) fn probability_color(p: f64, dark: bool) -> Color32 {
    let [zero, half, one] = if dark {
        [
            Color32::from_rgb(48, 68, 102),
            Color32::from_rgb(26, 115, 119),
            Color32::from_rgb(163, 67, 67),
        ]
    } else {
        [
            Color32::from_rgb(205, 219, 246),
            Color32::from_rgb(125, 210, 201),
            Color32::from_rgb(248, 163, 151),
        ]
    };
    if p <= 0.5 {
        zero.lerp_to_gamma(half, (p * 2.0) as f32)
    } else {
        half.lerp_to_gamma(one, (p * 2.0 - 1.0) as f32)
    }
}

pub(super) fn legend(ui: &mut Ui) {
    ui.label("P(1)")
        .on_hover_text("Probability of measuring 1 at the selected step.");
    for (p, label) in [(0.0, "0%"), (0.5, "50%"), (1.0, "100%")] {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        ui.painter().circle_filled(
            rect.center(),
            5.0,
            probability_color(p, ui.visuals().dark_mode),
        );
        ui.label(label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_colors_follow_preparations_and_invalidated_steps_stay_uncolored() {
        let mut circuit = Circuit::new(5);
        circuit.initial_preps = vec![
            None,
            Some(QubitPrep::One),
            Some(QubitPrep::Plus),
            Some(QubitPrep::Minus),
            Some(QubitPrep::Zero),
        ];
        let mut state = SimulationState::default();
        let input = SiteReadout::for_step(&circuit, 0, &state);
        assert_eq!(
            (0..5)
                .map(|q| input.probability(q).unwrap())
                .collect::<Vec<_>>(),
            [0.0, 1.0, 0.5, 0.5, 0.0]
        );
        assert_eq!(
            SiteReadout::for_step(&circuit, 1, &state).probability(0),
            None
        );
        circuit.initial_state = Some(vec![]);
        assert_eq!(
            SiteReadout::for_step(&circuit, 0, &state).probability(0),
            None
        );
        state.wire_one_probabilities = vec![0.25; 5];
        assert_eq!(
            SiteReadout::for_step(&circuit, 0, &state).probability(0),
            Some(0.25)
        );
    }
}
