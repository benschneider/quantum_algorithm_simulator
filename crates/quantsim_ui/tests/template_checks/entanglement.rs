use super::{at_step, load};
use nalgebra::Complex;
use quantsim_ui::messages::Message;
use std::f32::consts::FRAC_1_SQRT_2;

#[test]
fn bell_and_ghz_extend_the_correlated_pair_one_wire_at_a_time() {
    for (stem, occupied) in [("bell", &[1, 3][..]), ("ghz", &[1, 3, 7][..])] {
        let mut app = load(stem);
        for (column, &second_basis_state) in occupied.iter().enumerate() {
            let state = at_step(&mut app, column + 1);
            for (i, &amplitude) in state.iter().enumerate() {
                let expected = if i == 0 || i == second_basis_state {
                    FRAC_1_SQRT_2
                } else {
                    0.0
                };
                assert!(
                    (amplitude - expected).norm() < 1e-6,
                    "{stem} step {} |{i:b}>",
                    column + 1
                );
            }
        }
    }
}

#[test]
fn complex_bell_settings_predict_the_data_pair_correlations() {
    let mut app = load("bell_test_complex");
    let before_rotations = at_step(&mut app, 3);
    for (index, value) in before_rotations.iter().enumerate() {
        let data = index & 3;
        let expected = if data == 0 || data == 3 { 0.125 } else { 0.0 };
        assert!((value.norm_sqr() - expected).abs() < 1e-6);
    }

    let output = at_step(&mut app, 5);
    // A slice fixes the setting wires q3 q2; q1 q0 = 00 and 11 agree.
    for (settings, data) in output.chunks_exact(4).enumerate() {
        let total: f32 = data.iter().map(|a| a.norm_sqr()).sum();
        assert!((total - 0.25).abs() < 1e-6);
        let expected_correlation = if settings == 3 {
            -FRAC_1_SQRT_2
        } else {
            FRAC_1_SQRT_2
        };
        let expected_agreement = (1.0 + expected_correlation) / 2.0;
        let agreement = (data[0].norm_sqr() + data[3].norm_sqr()) / total;
        assert!(
            (agreement - expected_agreement).abs() < 1e-6,
            "settings {settings:02b}: probability of matching outcomes {agreement}, expected {expected_agreement}"
        );
    }
}

#[test]
fn teleportation_restores_both_complex_amplitudes_on_the_receiver() {
    // Use the bundled input, then change q0's preparation to |1> and repeat.
    let mut app = load("quantum_teleportation");
    let high = 3.0_f32.sqrt() / 2.0;
    for (input, alpha, beta_im) in [(0, high, 0.5), (1, -0.5, high)] {
        if input == 1 {
            let mut initial = vec![Complex::new(0.0, 0.0); 8];
            initial[1] = Complex::new(1.0, 0.0);
            app.handle_message(Message::ApplyInitialStateFromEditor(initial));
        }
        let beta = Complex::new(0.0, beta_im);
        let prepared = at_step(&mut app, 4);
        // Only q1 q2 = 00 or 11 are occupied in the shared Bell pair.
        for start in [0, 6] {
            assert!((prepared[start] - alpha * FRAC_1_SQRT_2).norm() < 1e-6);
            assert!((prepared[start + 1] - beta * FRAC_1_SQRT_2).norm() < 1e-6);
        }
        let uncorrected = at_step(&mut app, 6);
        let receiver_one: f32 = uncorrected[4..].iter().map(|a| a.norm_sqr()).sum();
        assert!((receiver_one - 0.5).abs() < 1e-6);

        for step in [7, 8] {
            let state = at_step(&mut app, step);
            // Each sender result q1 q0 has weight 1/4. Step 8 removes
            // the remaining phase sign from branches where q0 is 1.
            for sender in 0..4 {
                let sign = if step == 7 && sender & 1 == 1 {
                    -1.0
                } else {
                    1.0
                };
                assert!((state[sender] - alpha / 2.0).norm() < 1e-6);
                assert!((state[4 + sender] - beta * sign / 2.0).norm() < 1e-6);
            }
        }
    }
}
