use nalgebra::Complex;
use quantsim_core::{circuit::CircuitData, types::Operation};
use quantsim_ui::{messages::Message, QCSimApp};

// The middle bit is the logical output. The two outer bits hold the syndrome.
fn assert_logical_state(app: &QCSimApp, outer_bits: usize, logical_flipped: bool) {
    let result = app.state.simulation_state.quantum_state.as_ref().unwrap();
    let mut expected = [Complex::new(0.0, 0.0); 8];
    let logical_zero = if logical_flipped {
        outer_bits | 2
    } else {
        outer_bits
    };
    expected[logical_zero] = Complex::new(3.0_f32.sqrt() / 2.0, 0.0);
    expected[logical_zero ^ 2] = Complex::new(0.0, 0.5);
    assert_eq!(result.state_vector.len(), expected.len());
    for (actual, expected) in result.state_vector.iter().zip(expected) {
        assert!((*actual - expected).norm() < 2e-6, "{actual} != {expected}");
    }
}

#[test]
fn repetition_templates_show_the_same_syndrome_but_recover_only_single_faults() {
    let mut app = QCSimApp::default();
    for (template, last_step, logical_flipped_after_correction) in [
        ("error_correction_bit_flip.json", 8, false),
        ("error_correction_bit_flip_two_faults.json", 8, true),
        ("error_correction_phase_flip.json", 10, false),
        ("error_correction_phase_flip_two_faults.json", 10, true),
    ] {
        assert!(
            app.state
                .template_circuits
                .iter()
                .any(|(name, _)| name == template),
            "{template} is missing from the template menu"
        );
        app.handle_message(Message::LoadTemplate(template.into()));
        assert_eq!(app.state.circuit_state.num_timesteps, last_step);
        app.handle_message(Message::RunSimulation);
        assert_logical_state(&app, 0b101, logical_flipped_after_correction);
        app.handle_message(Message::SelectTimestep(2));
        assert_logical_state(&app, 0, false);
        app.handle_message(Message::SelectTimestep(last_step - 1));
        assert_logical_state(&app, 0b101, !logical_flipped_after_correction);
    }
}

#[test]
fn editing_the_fault_wires_changes_the_syndrome_and_logical_outcome() {
    for (template, fault_step) in [
        ("error_correction_bit_flip.json", 5),
        ("error_correction_phase_flip.json", 6),
    ] {
        let mut app = QCSimApp::default();
        app.handle_message(Message::LoadTemplate(template.into()));
        let mut data: CircuitData =
            serde_json::from_str(&app.state.ui_state.circuit_json_string).unwrap();
        let fault = data.steps[fault_step - 1][0].id.clone();
        for (wires, outer_bits, logical_flipped) in [
            (&[][..], 0b000, false),
            (&[0][..], 0b001, false),
            (&[2][..], 0b100, false),
            (&[0, 2][..], 0b101, true),
        ] {
            data.steps[fault_step - 1] = wires
                .iter()
                .map(|&wire| Operation::new(fault.clone(), vec![wire], vec![]))
                .collect();
            app.handle_message(Message::UpdateCircuitFromJson(
                serde_json::to_string(&data).unwrap(),
            ));
            app.handle_message(Message::RunSimulation);
            assert_logical_state(&app, outer_bits, logical_flipped);
        }
    }
}

#[test]
fn phase_faults_change_signs_before_any_probabilities_change() {
    let mut app = QCSimApp::default();
    for (template, fault_mask) in [
        ("error_correction_phase_flip.json", 0b010),
        ("error_correction_phase_flip_two_faults.json", 0b101),
    ] {
        app.handle_message(Message::LoadTemplate(template.into()));
        app.handle_message(Message::SelectTimestep(5));
        let encoded = app
            .state
            .simulation_state
            .quantum_state
            .as_ref()
            .unwrap()
            .state_vector
            .clone();
        app.handle_message(Message::SelectTimestep(6));
        let faulted = &app
            .state
            .simulation_state
            .quantum_state
            .as_ref()
            .unwrap()
            .state_vector;
        for (basis, &before) in encoded.iter().enumerate() {
            assert!((before.norm_sqr() - 0.125).abs() < 2e-6);
            let sign_flips = (basis & fault_mask).count_ones();
            let expected = if sign_flips % 2 == 1 { -before } else { before };
            assert!(
                (faulted[basis] - expected).norm() < 2e-6,
                "{template}, basis {basis:03b}"
            );
        }
    }
}
