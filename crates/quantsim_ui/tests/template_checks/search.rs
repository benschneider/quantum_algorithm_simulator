use super::{at_step, load};
use nalgebra::Complex;
use std::f32::consts::FRAC_1_SQRT_2;

#[test]
fn deutsch_balanced_oracle_moves_the_answer_from_phase_to_q0() {
    let mut app = load("deutsch_algorithm");
    // The ket order is q1 q0. The ancilla q1 stays in |->.
    for (step, expected) in [
        (2, [0.5, 0.5, -0.5, -0.5]),
        (3, [0.5, -0.5, -0.5, 0.5]),
        (4, [0.0, FRAC_1_SQRT_2, 0.0, -FRAC_1_SQRT_2]),
    ] {
        let state = at_step(&mut app, step);
        assert_eq!(state.len(), expected.len());
        for (&amplitude, real) in state.iter().zip(expected) {
            assert!((amplitude - Complex::new(real, 0.0)).norm() < 1e-6);
        }
    }
}

#[test]
fn grover_oracles_mark_one_state_and_each_diffusion_amplifies_it() {
    for (stem, width, marked, rounds) in [
        ("grover", 4, 0b11, &[(2, 6, 1.0)][..]),
        (
            "grover_algorithm",
            8,
            0b100,
            &[(2, 7, 25.0 / 32.0), (8, 13, 121.0 / 128.0)][..],
        ),
    ] {
        let mut app = load(stem);
        let spread = at_step(&mut app, 1);
        assert_eq!(spread.len(), width);
        for &amplitude in spread {
            let expected = Complex::new(1.0 / (width as f32).sqrt(), 0.0);
            assert!((amplitude - expected).norm() < 1e-6);
        }
        for &(oracle, diffusion, marked_probability) in rounds {
            let before = at_step(&mut app, oracle - 1).to_vec();
            let after = at_step(&mut app, oracle);
            for (index, (&original, &actual)) in before.iter().zip(after).enumerate() {
                let expected = if index == marked { -original } else { original };
                assert!((actual - expected).norm() < 1e-6, "{stem} oracle {oracle}");
            }

            let state = at_step(&mut app, diffusion);
            let other_probability = (1.0 - marked_probability) / (width - 1) as f32;
            for (index, amplitude) in state.iter().enumerate() {
                let expected = if index == marked {
                    marked_probability
                } else {
                    other_probability
                };
                assert!(
                    (amplitude.norm_sqr() - expected).abs() < 2e-6,
                    "{stem} step {diffusion}"
                );
            }
        }
    }
}
