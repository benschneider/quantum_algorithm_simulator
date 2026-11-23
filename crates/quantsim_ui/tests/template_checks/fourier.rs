use super::{at_step, load};
use nalgebra::Complex;
use std::f32::consts::TAU;

#[test]
fn both_qft_templates_prepare_the_advertised_input_and_apply_the_forward_transform() {
    for (name, example_input) in [("qft", 0b01), ("qft4", 0b1011)] {
        let mut app = load(name);
        let prepared = at_step(&mut app, 1);
        assert_eq!(prepared[example_input], Complex::new(1.0, 0.0));

        // Keep the complete transform, replacing only the example's X-gate preparation.
        let circuit = &mut app.state.circuit_state.circuit;
        circuit.steps[0].clear();
        let size = 1 << circuit.num_qubits;
        for input in 0..size {
            let mut state = vec![Complex::new(0.0, 0.0); size];
            state[input].re = 1.0;
            circuit.initial_state = Some(state);
            let result = circuit.run(&Default::default());
            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
            let output = result.final_state_vector.unwrap();
            assert_eq!(output.len(), size);
            for (index, amplitude) in output.iter().enumerate() {
                let angle = TAU * ((input * index) % size) as f32 / size as f32;
                let expected = Complex::from_polar(1.0 / (size as f32).sqrt(), angle);
                assert!(
                    (*amplitude - expected).norm() < 1e-5,
                    "{name}, input {input}, output {index}: expected {expected}, got {amplitude}"
                );
            }
        }
    }
}
