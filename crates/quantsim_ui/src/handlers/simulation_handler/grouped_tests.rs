use crate::state::simulation_state::{COMPACT_SHOTS, MAX_SAMPLE_ROWS};
use crate::{messages::Message, QCSimApp};
use nalgebra::Complex;
use quantsim_core::circuit::forward_qft_steps;
use quantsim_core::types::{Gate, Operation};
use serde_json::json;

#[test]
fn run_and_scrub_a_64_wire_qft_with_an_entangling_suffix() {
    let mut steps = vec![Vec::new(); 5];
    steps[4].push(Operation::new(Gate::X, vec![55], vec![]));
    steps.extend(forward_qft_steps(0, 64).unwrap());
    // Four columns of a partial transform on q13–q54 act on the QFT output.
    let partial = forward_qft_steps(13, 42).unwrap();
    steps.extend(partial.into_iter().skip(2).take(4));
    steps.resize_with(218, Vec::new);

    let mut app = QCSimApp::default();
    app.handle_message(Message::UpdateCircuitFromJson(
        json!({"num_qubits":64,"steps":steps}).to_string(),
    ));
    app.handle_message(Message::SelectTimestep(133));
    assert_eq!(app.state.ui_state.error_message, None);
    app.state.simulation_state.set_waveform_window(0, 16);
    app.handle_message(Message::RunSimulation);
    assert_eq!(app.state.ui_state.error_message, None);
    assert_eq!(app.state.ui_state.current_timestep, 218);
    let readout = &app.state.simulation_state;
    assert!(readout.quantum_state.is_none() && readout.populated_indices.is_empty());
    // q52's Hadamard converts the q54-controlled π/4 phase into a probability.
    let expected_one = (1.0 - std::f64::consts::FRAC_1_SQRT_2) / 4.0;
    for (wire, &p) in readout.wire_one_probabilities.iter().enumerate() {
        let expected = match wire {
            52 => expected_one,
            53 => 0.0,
            _ => 0.5,
        };
        assert!((p - expected).abs() < 1e-6, "wire {wire}: {p}");
    }
    let samples = readout.compact_samples.as_ref().unwrap();
    assert_eq!(samples.rows.len(), MAX_SAMPLE_ROWS);
    assert!(samples.distinct <= COMPACT_SHOTS as usize);
    for &(bits, _) in &samples.rows {
        assert_eq!(bits & (1 << 53), 0);
        assert!(bits & (1 << 52) == 0 || bits & (1 << 54) != 0);
    }

    // In this window q49–q54 are zero, so only the Hadamards change the
    // scale. The phase of the low nine bits remains the Fourier mode of q55.
    let scale = 2.0_f64.powi(-32);
    for (step, magnitude) in [
        (218, 2.0 * scale),
        (133, scale),
        (134, 2.0_f64.sqrt() * scale),
    ] {
        app.handle_message(Message::SelectTimestep(step));
        assert_eq!(app.state.ui_state.error_message, None);
        let wave = app.state.simulation_state.waveform().unwrap();
        assert_eq!((wave.first, wave.stride, wave.points.len()), (0, 16, 64));
        assert_eq!(wave.phase_reference, Some(0));
        for (j, &amplitude) in wave.points.iter().enumerate() {
            let expected = Complex::from_polar(magnitude, std::f64::consts::TAU * j as f64 / 32.0);
            assert!(
                (amplitude - expected).norm() < magnitude * 1e-6,
                "step {step}, index {}",
                j * 16
            );
        }
    }
}
