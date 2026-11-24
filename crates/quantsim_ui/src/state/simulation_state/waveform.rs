use super::{CompactAmplitudes, SimulationState};
use nalgebra::Complex;

pub(crate) const WAVEFORM_POINTS: usize = 64;

/// Point j is the amplitude of |first + j * stride⟩. Only this window is materialized.
#[derive(Debug)]
pub(crate) struct Waveform {
    pub first: u64,
    pub stride: u64,
    pub last_index: u64,
    pub points: Vec<Complex<f64>>,
    pub phase_reference: Option<u64>,
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
            assert_eq!((wave.first, wave.stride, wave.points.len()), (0, 1, 64));
            let occupied_indices = 1_u64 << active_wires;
            let magnitude = 1.0 / (occupied_indices as f64).sqrt();
            for (index, amp) in wave.points.iter().enumerate() {
                let angle = std::f64::consts::TAU * (index & phase_bits) as f64 / 32.0;
                let expected = if (index as u64) < occupied_indices {
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
                app.state.simulation_state.set_waveform_window(0, 1);
                let wave = app.state.simulation_state.waveform().unwrap();
                assert_eq!(wave.phase_reference, Some(1));
                let plus = std::f64::consts::FRAC_1_SQRT_2;
                assert!((wave.points[1] - plus).norm() < 1e-6);
                assert!((wave.points[3] - Complex::new(0.0, plus)).norm() < 1e-6);
            }
        }
    }

    #[test]
    fn a_manual_window_survives_scrubbing_and_handles_the_last_64_bit_index() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::ChangeQubits(64));
        app.handle_message(Message::SelectTimestep(0));
        app.state
            .simulation_state
            .set_waveform_window(1, u64::MAX - 1);
        let wave = app.state.simulation_state.waveform().unwrap();
        assert_eq!(wave.points, [Complex::ZERO; 2]);
        assert_eq!(wave.first + wave.stride, u64::MAX);
        app.handle_message(Message::SelectTimestep(1));
        let wave = app.state.simulation_state.waveform().unwrap();
        assert_eq!(
            (wave.first, wave.stride, wave.last_index),
            (1, u64::MAX - 1, u64::MAX)
        );
        app.state.simulation_state.reset_waveform_window();
        assert_eq!(app.state.simulation_state.waveform().unwrap().first, 0);
    }
}

impl SimulationState {
    pub(crate) fn waveform(&self) -> Option<&Waveform> {
        self.waveform.as_ref()
    }

    pub(crate) fn set_waveform_window(&mut self, first: u64, stride: u64) {
        self.manual_window = Some((first, stride.max(1)));
        self.refresh_waveform();
    }

    pub(crate) fn reset_waveform_window(&mut self) {
        self.manual_window = None;
        self.refresh_waveform();
    }

    pub(crate) fn replace_compact_readout(&mut self, mut next: Self) {
        if let Some((first, stride)) = self.manual_window {
            next.set_waveform_window(first, stride);
        }
        *self = next;
    }

    pub(super) fn refresh_waveform(&mut self) {
        let wires = self.wire_one_probabilities.len();
        if !(1..=64).contains(&wires) {
            self.waveform = None;
            return;
        }
        let last_index = u64::MAX >> (64 - wires);
        let default_window = if last_index < WAVEFORM_POINTS as u64 {
            (0, 1)
        } else {
            self.auto_window
        };
        let (first, stride) = self.manual_window.unwrap_or(default_window);
        let first = first.min(last_index);
        // Divide before adding one so the inclusive 64-wire domain fits in a u64.
        let count = ((last_index - first) / stride).min(WAVEFORM_POINTS as u64 - 1) + 1;
        let indices = (0..count).map(|j| first + j * stride);
        let (points, phase_reference) = match &self.compact_amplitudes {
            Some(CompactAmplitudes::Product(state)) => {
                let phase = state.reference_phase().conj();
                let points = indices
                    .map(|k| state.amplitude(k).expect("index within register") * phase)
                    .collect();
                (points, Some(self.auto_window.0))
            }
            Some(CompactAmplitudes::Stabilizer(view)) => (
                indices
                    .map(|k| view.amplitude(k).expect("index within register"))
                    .collect(),
                Some(view.reference_index()),
            ),
            None => {
                let state = self.quantum_state.as_ref().expect("dense readout state");
                let first = self.auto_window.0;
                let reference = state.state_vector[first as usize];
                let reference = Complex::new(f64::from(reference.re), f64::from(reference.im));
                let phase = if reference == Complex::ZERO {
                    Complex::ONE
                } else {
                    reference.conj() / reference.norm()
                };
                let points = indices
                    .map(|k| {
                        let a = state.state_vector[k as usize];
                        Complex::new(f64::from(a.re), f64::from(a.im)) * phase
                    })
                    .collect();
                (points, (reference != Complex::ZERO).then_some(first))
            }
        };
        self.waveform = Some(Waveform {
            first,
            stride,
            last_index,
            points,
            phase_reference,
        });
    }
}
