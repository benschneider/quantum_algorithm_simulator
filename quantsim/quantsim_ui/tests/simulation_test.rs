use nalgebra::{Complex, DMatrix};
use quantsim_core::core::gates::Gate;
use quantsim_core::core::types::GateMatrix;
use quantsim_ui::{messages::Message, simulation, QCSimApp};
use log::debug;

// Helper function for clean state vector logging
fn debug_state_vector(state: &Option<Vec<Complex<f32>>>, num_qubits: usize, label: &str) {
    if let Some(state_vec) = state {
        let dvector = nalgebra::DVector::from_iterator(state_vec.len(), state_vec.iter().cloned());
        debug!("{}: {}", label, quantsim_core::core::formatters::get_pretty_print_state_vector(&dvector, num_qubits));
    } else {
        debug!("{}: default |00...0⟩", label);
    }
}


#[test]
fn test_bell_circuit_simulation() {
    env_logger::builder().is_test(true).try_init().ok();
    let mut app = QCSimApp::default();

    let bell_json = include_str!("../circuits/bell.json");
    app.state.ui_state.circuit_json_string = bell_json.to_string();

    app.handle_message(Message::UpdateCircuitFromJson(bell_json.to_string()));

    let mut circuit = app.state.circuit_state.circuit.clone();

    // Simulate the circuit and assert the final state
    let mut sim = simulation::Simulation::new();
    sim.run_simulation(&mut circuit);

    let expected_state_vector = vec![
        Complex::new(1.0 / 2.0_f32.sqrt(), 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(1.0 / 2.0_f32.sqrt(), 0.0),
    ];

    // Use a small epsilon for floating point comparisons
    let epsilon = 1e-5_f32; // Changed to f32
    for i in 0..expected_state_vector.len() {
        let actual_re = sim.state_vector.as_ref().unwrap()[i].re;
        let actual_im = sim.state_vector.as_ref().unwrap()[i].im;
        let expected_re = expected_state_vector[i].re;
        let expected_im = expected_state_vector[i].im;

        assert!(
            (actual_re - expected_re).abs() < epsilon,
            "Real part mismatch at index {}, expected {}, got {}",
            i,
            expected_re,
            actual_re
        );
        assert!(
            (actual_im - expected_im).abs() < epsilon,
            "Imaginary part mismatch at index {}, expected {}, got {}",
            i,
            expected_im,
            actual_im
        );
    }
}

#[test]
fn test_deutsch_algorithm_simulation() {
    env_logger::builder().is_test(true).try_init().ok();
    let mut app = QCSimApp::default();

    let deutsch_json = include_str!("../circuits/deutsch_algorithm.json");
    app.state.ui_state.circuit_json_string = deutsch_json.to_string();

    app.handle_message(Message::UpdateCircuitFromJson(deutsch_json.to_string()));

    let mut circuit = app.state.circuit_state.circuit.clone();
    circuit.initial_preps = vec![
        Some(quantsim_core::core::types::QubitPrep::Plus),
        Some(quantsim_core::core::types::QubitPrep::Minus),
    ];
    // Simulate the circuit and assert the final state
    let mut sim = simulation::Simulation::new();
    sim.run_simulation(&mut circuit);

    // For a balanced function, the probability of measuring the first qubit as 1 should be 1.0
    let prob_q0_one = sim.get_qubit_probabilities(0).unwrap();
    let epsilon = 1e-5_f32; // Changed to f32
    assert!(
        (prob_q0_one - 1.0).abs() < epsilon,
        "Probability of measuring q0 as 1 is not close to 1.0 we got {}",
        prob_q0_one
    );
}

#[test]
fn test_quantum_teleportation_simulation() {
    env_logger::builder().is_test(true).try_init().ok();
    let mut app = QCSimApp::default();

    let teleportation_json = include_str!("../circuits/quantum_teleportation.json");
    app.state.ui_state.circuit_json_string = teleportation_json.to_string();

    app.handle_message(Message::UpdateCircuitFromJson(
        teleportation_json.to_string(),
    ));

    let mut circuit = app.state.circuit_state.circuit.clone();

    // First, let's get the state of the message qubit (q0) before teleportation.
    // The message is created in the second step of the circuit.
    let mut message_circuit = circuit.clone();
    message_circuit.steps = circuit.steps[0..2].to_vec();
    let mut message_sim = simulation::Simulation::new();
    message_sim.run_simulation(&mut message_circuit);
    let message_state = message_sim.get_qubit_probabilities(0).unwrap();

    // Now, run the full teleportation circuit.
    let mut full_sim = simulation::Simulation::new();
    full_sim.run_simulation(&mut circuit);

    // The state of the third qubit (q2) should now match the original message state.
    // Note: This is a simplified check. A full check would involve comparing the
    // density matrices of the qubits. For this test, we'll compare the probability
    // of being in the |1> state.
    let teleported_state = full_sim.get_qubit_probabilities(2).unwrap();

    let epsilon = 1e-5_f32; // Changed to f32
    assert!(
        (message_state - teleported_state).abs() < epsilon,
        "Teleported state does not match original message state, expected {}, got {}",
        message_state,
        teleported_state
    );
}

#[test]
fn test_grover_algorithm_simulation() {
    env_logger::builder().is_test(true).try_init().ok();

    let mut app = QCSimApp::default();

    let grover_json = include_str!("../circuits/grover_algorithm.json");
    app.state.ui_state.circuit_json_string = grover_json.to_string();

    app.handle_message(Message::UpdateCircuitFromJson(grover_json.to_string()));

    let mut circuit = app.state.circuit_state.circuit.clone();
    debug!("Grover Circuit: {:#?}", circuit);

    // Simulate the circuit and assert the final state
    let mut sim = simulation::Simulation::new();
    debug_state_vector(&circuit.initial_state, circuit.num_qubits, "Initial state");
    sim.run_simulation(&mut circuit);
    debug_state_vector(&sim.state_vector, circuit.num_qubits, "Final state");

    let state_vector = sim.state_vector.as_ref().unwrap();
    let state_vec = nalgebra::DVector::from_iterator(state_vector.len(), state_vector.iter().cloned());
    debug!("Grover Final State Vector: {}", quantsim_core::core::formatters::get_pretty_print_state_vector(&state_vec, circuit.num_qubits));

    // Iterate through snapshots and print state vector at each step
    for (i, snapshot) in sim.snapshots.iter().enumerate() {
        let state_vec = nalgebra::DVector::from_iterator(snapshot.state.as_ref().unwrap().len(), snapshot.state.as_ref().unwrap().iter().cloned());
        debug!(
            "Grover Step {}: State Vector: {}",
            i,
            quantsim_core::core::formatters::get_pretty_print_state_vector(&state_vec, circuit.num_qubits)
        );
    }

    // In Grover's algorithm, the amplitude of the marked state is amplified.
    // For the given circuit, the marked state is |10> for the first two qubits.
    // In the 3-qubit state vector, this corresponds to the |100> state (index 4).
    let marked_state_index = 4; // Corresponds to |100>
    let marked_state_amplitude = state_vector[marked_state_index].norm_sqr();
    let other_states_amplitude_sum =
        state_vector
            .iter()
            .enumerate()
            .fold(0.0_f32, |acc, (i, c)| {
                // Changed to f32 literal
                if i != marked_state_index {
                    acc + c.norm_sqr()
                } else {
                    acc
                }
            });

    // The marked state should have a higher or equal probability than any other state.
    assert!(
        marked_state_amplitude >= other_states_amplitude_sum,
        "Marked state amplitude is not dominant, expected > {}, got {}",
        other_states_amplitude_sum,
        marked_state_amplitude
    );
}

#[test]
fn test_cx_gate_on_superposition() {
    env_logger::builder().is_test(true).try_init().ok();
    use nalgebra::Complex;
    use quantsim_core::core::circuit::Circuit;
    use quantsim_core::core::types::Operation;

    let num_qubits = 2;
    let mut circuit = Circuit::new(num_qubits);
    circuit.steps.push(vec![Operation {
        id: Gate::H,
        name: None,
        params: vec![].into(),
        qubits: vec![0].into(),
        matrix: None,
    }]);
    circuit.steps.push(vec![Operation {
        id: Gate::CX,
        name: None,
        params: vec![].into(),
        qubits: vec![0, 1].into(),
        matrix: None,
    }]);

    let mut sim = simulation::Simulation::new();
    debug_state_vector(&circuit.initial_state, circuit.num_qubits, "Initial state");
    sim.run_simulation(&mut circuit);
    debug_state_vector(&sim.state_vector, circuit.num_qubits, "Final state");

    let expected_state_vector = vec![
        Complex::new(1.0 / 2.0_f32.sqrt(), 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(1.0 / 2.0_f32.sqrt(), 0.0),
    ];

    let epsilon = 1e-5_f32;
    let state_vector = sim.state_vector.as_ref().unwrap();

    for i in 0..expected_state_vector.len() {
        let actual_re = state_vector[i].re;
        let actual_im = state_vector[i].im;
        let expected_re = expected_state_vector[i].re;
        let expected_im = expected_state_vector[i].im;

        assert!(
            (actual_re - expected_re).abs() < epsilon,
            "Real part mismatch at index {}, expected {}, got {}",
            i,
            expected_re,
            actual_re
        );
        assert!(
            (actual_im - expected_im).abs() < epsilon,
            "Imaginary part mismatch at index {}, expected {}, got {}",
            i,
            expected_im,
            actual_im
        );
    }
}

#[test]
fn test_grover_oracle() {
    env_logger::builder().is_test(true).try_init().ok();
    use nalgebra::Complex;
    use quantsim_core::core::circuit::Circuit;
    use quantsim_core::core::types::Operation;

    let num_qubits = 3;
    let mut circuit = Circuit::new(num_qubits);

    // This test verifies that a Big-Endian custom gate matrix is correctly
    // converted to Little-Endian by the circuit's preparation step.
    //
    // 1. We define an oracle in Big-Endian format that targets state |100> (index 4).
    // 2. The `Circuit::run` method internally converts this to a Little-Endian matrix.
    //    Due to bit-reversal of qubit ordering, the Little-Endian matrix will target
    //    state |001> (index 1).
    // 3. Therefore, we initialize the simulator in the |001> state.
    // 4. We expect the final state to be -|001>, confirming the conversion and
    //    application of the gate were successful.
    circuit.initial_state = Some(vec![
        Complex::new(0.0, 0.0),
        Complex::new(1.0, 0.0),  // Initial state |001> (Little Endian)
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
    ]);

    // Create a Big-Endian oracle matrix that marks the |100> state (index 4).
    let mut oracle_matrix = DMatrix::identity(8, 8);
    oracle_matrix[(4, 4)] = Complex::new(-1.0, 0.0);

    circuit.steps.push(vec![Operation {
        id: Gate::Custom,
        name: Some("ORACLE".to_string()),
        params: vec![].into(),
        qubits: vec![0, 1, 2].into(),
        matrix: Some(GateMatrix::BigEndian(oracle_matrix.clone())),
    }]);

    let mut sim = simulation::Simulation::new();
    debug_state_vector(&circuit.initial_state, circuit.num_qubits, "Initial state");
    debug!("Oracle matrix (Big Endian):\n{}", oracle_matrix);
    sim.run_simulation(&mut circuit);
    debug_state_vector(&sim.state_vector, circuit.num_qubits, "Final state");

    // The expected state is -|001> because the Big-Endian oracle for |100> (idx 4)
    // is converted to a Little-Endian gate for |001> (idx 1).
    let expected_state_vector = vec![
        Complex::new(0.0, 0.0),
        Complex::new(-1.0, 0.0), // Expected final state -|001>
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
    ];

    let epsilon = 1e-5_f32;
    let state_vector = sim.state_vector.as_ref().unwrap();

    for i in 0..expected_state_vector.len() {
        let actual_re = state_vector[i].re;
        let actual_im = state_vector[i].im;
        let expected_re = expected_state_vector[i].re;
        let expected_im = expected_state_vector[i].im;

        assert!(
            (actual_re - expected_re).abs() < epsilon,
            "Real part mismatch at index {}, expected {}, got {}",
            i,
            expected_re,
            actual_re
        );
        assert!(
            (actual_im - expected_im).abs() < epsilon,
            "Imaginary part mismatch at index {}, expected {}, got {}",
            i,
            expected_im,
            actual_im
        );
    }
}
