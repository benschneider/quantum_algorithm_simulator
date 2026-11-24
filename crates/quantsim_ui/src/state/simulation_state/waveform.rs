use super::{CompactAmplitudes, SimulationState};
use nalgebra::Complex;
use quantsim_core::BasisIndex;

pub(crate) const WAVEFORM_POINTS: usize = 64;

/// Point j is the amplitude of |first + j * stride⟩. Only this window is materialized.
#[derive(Debug)]
pub(crate) struct Waveform {
    pub first: BasisIndex,
    pub stride: BasisIndex,
    pub last_index: BasisIndex,
    pub points: Vec<Complex<f64>>,
    pub phase_reference: Option<BasisIndex>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{messages::Message, QCSimApp};

    #[test]
    fn fourier_lesson_prepares_the_register_then_builds_the_wave_one_phase_bit_at_a_time() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::LoadTemplate("fourier_wave_50.json".into()));
        // Number of wires in superposition, then the index bits contributing phase.
        let stages = [
            (1, 0),
            (2, 0),
            (5, 0),
            (6, 0),
            (50, 0),
            (50, 16),
            (50, 24),
            (50, 28),
            (50, 30),
            (50, 31),
        ];
        let circuit = &app.state.circuit_state.circuit;
        assert_eq!(circuit.steps.len(), stages.len());
        assert_eq!(circuit.steps.iter().map(Vec::len).sum::<usize>(), 55);
        assert_eq!(circuit.sections.len(), stages.len());
        for (index, section) in circuit.sections.iter().enumerate() {
            assert_eq!((section.start, section.end), (index + 1, index + 1));
            assert!(!section.description.trim().is_empty());
        }
        for (i, (active_wires, phase_bits)) in stages.into_iter().enumerate() {
            let step = i + 1;
            app.handle_message(Message::SelectTimestep(step));
            let readout = &app.state.simulation_state;
            assert_eq!(readout.wire_one_probabilities.len(), 50);
            for (wire, p) in readout.wire_one_probabilities.iter().enumerate() {
                let expected = if wire < active_wires { 0.5 } else { 0.0 };
                assert!((p - expected).abs() < 1e-6, "step {step}, wire {wire}");
            }
            let wave = readout.waveform().unwrap();
            assert_eq!(
                (&wave.first, &wave.stride, wave.points.len()),
                (&BasisIndex::ZERO, &1u8.into(), 64)
            );
            let occupied_indices = 1_u128 << active_wires;
            let magnitude = 1.0 / (occupied_indices as f64).sqrt();
            for (index, amp) in wave.points.iter().enumerate() {
                let angle = std::f64::consts::TAU * (index & phase_bits) as f64 / 32.0;
                let expected = if (index as u128) < occupied_indices {
                    Complex::from_polar(magnitude, angle)
                } else {
                    Complex::ZERO
                };
                assert!(
                    (*amp - expected).norm() < 1e-13,
                    "step {step}, index {index}"
                );
            }
        }
    }

    #[test]
    fn changing_executor_at_an_identity_gate_preserves_the_drawn_phase() {
        for wires in [2, 50] {
            let mut app = QCSimApp::default();
            app.handle_message(Message::UpdateCircuitFromJson(
                serde_json::json!({
                    "num_qubits": wires,
                    "steps": [
                        [{"id":"Y","qubits":[0]}, {"id":"H","qubits":[1]}],
                        [{"id":"SqrtZ","qubits":[1]}],
                        [{"id":"Rz","qubits":[1],"params":[0]}]
                    ]
                })
                .to_string(),
            ));
            // The 50-wire run switches from the tableau to the product backend at step 3.
            for step in [2, 3] {
                app.handle_message(Message::SelectTimestep(step));
                app.state
                    .simulation_state
                    .set_waveform_window(0u8.into(), 1u8.into());
                let wave = app.state.simulation_state.waveform().unwrap();
                assert_eq!(wave.phase_reference, Some(1u8.into()));
                let plus = std::f64::consts::FRAC_1_SQRT_2;
                assert!((wave.points[1] - plus).norm() < 1e-6);
                assert!((wave.points[3] - Complex::new(0.0, plus)).norm() < 1e-6);
            }
        }
    }

    #[test]
    fn a_manual_window_survives_scrubbing_and_handles_the_last_basis_index() {
        for (wires, last_index) in [(64, u128::from(u64::MAX)), (128, u128::MAX)] {
            let mut app = QCSimApp::default();
            app.handle_message(Message::ChangeQubits(wires));
            app.handle_message(Message::SelectTimestep(0));
            app.state
                .simulation_state
                .set_waveform_window(1u8.into(), (last_index - 1).into());
            let wave = app.state.simulation_state.waveform().unwrap();
            assert_eq!(wave.points, [Complex::ZERO; 2]);
            assert_eq!(&wave.first + &wave.stride, last_index.into());
            app.handle_message(Message::SelectTimestep(1));
            let wave = app.state.simulation_state.waveform().unwrap();
            assert_eq!(
                (&wave.first, &wave.stride, &wave.last_index),
                (&1u8.into(), &(last_index - 1).into(), &last_index.into())
            );
            app.state.simulation_state.reset_waveform_window();
            assert_eq!(
                app.state.simulation_state.waveform().unwrap().first,
                BasisIndex::ZERO
            );
        }
    }
}

impl SimulationState {
    pub(crate) fn waveform(&self) -> Option<&Waveform> {
        self.waveform.as_ref()
    }

    pub(crate) fn set_waveform_window(&mut self, first: BasisIndex, stride: BasisIndex) {
        self.manual_window = Some((first, stride.max(1u8.into())));
        self.refresh_waveform();
    }

    pub(crate) fn reset_waveform_window(&mut self) {
        self.manual_window = None;
        self.refresh_waveform();
    }

    pub(crate) fn replace_compact_readout(&mut self, mut next: Self) {
        if let Some((first, stride)) = self.manual_window.take() {
            next.set_waveform_window(first, stride);
        }
        *self = next;
    }

    pub(super) fn refresh_waveform(&mut self) {
        let wires = self.wire_one_probabilities.len();
        if wires == 0 {
            self.waveform = None;
            return;
        }
        let last_index = (BasisIndex::from(1u8) << wires) - 1u8;
        let small_window = (BasisIndex::ZERO, BasisIndex::from(1u8));
        let default_window = if last_index < WAVEFORM_POINTS.into() {
            &small_window
        } else {
            &self.auto_window
        };
        let (first, stride) = self.manual_window.as_ref().unwrap_or(default_window);
        let first = first.min(&last_index).clone();
        let indices = std::iter::successors(Some(first.clone()), |index| Some(index + stride))
            .take(WAVEFORM_POINTS)
            .take_while(|index| index <= &last_index);
        let (points, phase_reference) = match &self.compact_amplitudes {
            Some(view) => {
                let phase = match view {
                    CompactAmplitudes::Stabilizer(_) => Complex::ONE,
                    CompactAmplitudes::Product(state) => state.reference_phase().conj(),
                    CompactAmplitudes::Grouped(state) => state.reference_phase().conj(),
                };
                let points = indices
                    .map(|k| {
                        let amplitude = match view {
                            CompactAmplitudes::Stabilizer(state) => state.amplitude_wide(&k),
                            CompactAmplitudes::Product(state) => state.amplitude_wide(&k),
                            CompactAmplitudes::Grouped(state) => state.amplitude_wide(&k),
                        };
                        amplitude.expect("index within register") * phase
                    })
                    .collect();
                (points, Some(self.auto_window.0.clone()))
            }
            None => {
                let state = self.quantum_state.as_ref().expect("dense readout state");
                let reference_index = &self.auto_window.0;
                let reference = state.state_vector
                    [usize::try_from(reference_index).expect("dense basis index")];
                let reference = Complex::new(f64::from(reference.re), f64::from(reference.im));
                let phase = if reference == Complex::ZERO {
                    Complex::ONE
                } else {
                    reference.conj() / reference.norm()
                };
                let points = indices
                    .map(|k| {
                        let a = state.state_vector[usize::try_from(&k).expect("dense basis index")];
                        Complex::new(f64::from(a.re), f64::from(a.im)) * phase
                    })
                    .collect();
                (
                    points,
                    (reference != Complex::ZERO).then(|| reference_index.clone()),
                )
            }
        };
        self.waveform = Some(Waveform {
            first,
            stride: stride.clone(),
            last_index,
            points,
            phase_reference,
        });
    }
}
