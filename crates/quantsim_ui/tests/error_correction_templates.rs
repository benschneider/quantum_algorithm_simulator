use nalgebra::Complex;
use quantsim_core::{
    circuit::{
        CircuitData,
        QubitRole::{Ancilla, Data},
    },
    types::Operation,
};
use quantsim_ui::{messages::Message, QCSimApp};

struct Lesson {
    name: &'static str,
    phase: bool,
    fault: usize,
    syndrome: usize,
    corrected: usize,
    decoded: usize,
}

const LESSONS: [Lesson; 2] = [
    Lesson {
        name: "bit",
        phase: false,
        fault: 5,
        syndrome: 7,
        corrected: 11,
        decoded: 13,
    },
    Lesson {
        name: "phase",
        phase: true,
        fault: 6,
        syndrome: 10,
        corrected: 14,
        decoded: 17,
    },
];

fn assert_state(app: &QCSimApp, expected: impl Fn(usize) -> Complex<f32>) {
    let result = app.state.simulation_state.quantum_state.as_ref().unwrap();
    assert_eq!(result.state_vector.len(), 32);
    for (index, &actual) in result.state_vector.iter().enumerate() {
        let wanted = expected(index);
        assert!(
            (actual - wanted).norm() < 2e-6,
            "step {} |{index:05b}>: {actual} != {wanted}",
            app.state.ui_state.current_timestep
        );
    }
}

fn assert_encoded(app: &QCSimApp, phase: bool, syndrome: usize, fault_mask: usize) {
    let alpha = Complex::new(3.0_f32.sqrt() / 2.0, 0.0);
    let beta = Complex::new(0.0, 0.5);
    assert_state(app, |index| {
        if index >> 3 != syndrome {
            return Complex::new(0.0, 0.0);
        }
        let data = index & 0b111;
        if phase {
            // Encoded state alpha|+++> + beta|--->, with any data Z faults.
            let encoded = if data.count_ones() % 2 == 0 {
                alpha + beta
            } else {
                alpha - beta
            };
            let amplitude = encoded / 8.0_f32.sqrt();
            if (data & fault_mask).count_ones() % 2 == 0 {
                amplitude
            } else {
                -amplitude
            }
        } else {
            match data ^ fault_mask {
                0b000 => alpha,
                0b111 => beta,
                _ => Complex::new(0.0, 0.0),
            }
        }
    });
}

fn assert_decoded(app: &QCSimApp, syndrome: usize, logical_flipped: bool) {
    // q0 and q2 return to zero; q3 and q4 retain the two syndrome bits.
    let first = (syndrome << 3) | (usize::from(logical_flipped) << 1);
    assert_state(app, |index| {
        if index == first {
            Complex::new(3.0_f32.sqrt() / 2.0, 0.0)
        } else if index == (first ^ 0b10) {
            Complex::new(0.0, 0.5)
        } else {
            Complex::new(0.0, 0.0)
        }
    });
}

#[test]
fn menu_templates_extract_parity_recover_the_block_and_decode() {
    let mut app = QCSimApp::default();
    for lesson in LESSONS {
        for (suffix, fault_mask, logical_flipped) in
            [("", 0b010, false), ("_two_faults", 0b101, true)]
        {
            let template = format!("error_correction_{}_flip{suffix}.json", lesson.name);
            assert!(app
                .state
                .template_circuits
                .iter()
                .any(|(name, _)| name == &template));
            app.handle_message(Message::LoadTemplate(template));
            let circuit = &app.state.circuit_state.circuit;
            assert_eq!(
                circuit.qubit_roles,
                [
                    Some(Data),
                    Some(Data),
                    Some(Data),
                    Some(Ancilla),
                    Some(Ancilla)
                ]
            );
            assert_eq!(circuit.steps.len(), lesson.decoded);
            let coupling = circuit.section_at(lesson.syndrome).unwrap();
            assert_eq!(
                coupling.label,
                if lesson.phase {
                    "X coupling"
                } else {
                    "Z coupling"
                }
            );
            assert_eq!(
                (coupling.start, coupling.end),
                (lesson.fault + 1, lesson.syndrome)
            );

            app.handle_message(Message::SelectTimestep(2));
            assert_decoded(&app, 0, false);
            app.handle_message(Message::SelectTimestep(lesson.fault - 1));
            assert_encoded(&app, lesson.phase, 0, 0);
            app.handle_message(Message::SelectTimestep(lesson.fault));
            assert_encoded(&app, lesson.phase, 0, fault_mask);
            app.handle_message(Message::SelectTimestep(lesson.syndrome));
            assert_encoded(&app, lesson.phase, 0b11, fault_mask);
            app.handle_message(Message::SelectTimestep(lesson.corrected));
            assert_encoded(
                &app,
                lesson.phase,
                0b11,
                if logical_flipped { 0b111 } else { 0 },
            );
            app.handle_message(Message::RunSimulation);
            assert_decoded(&app, 0b11, logical_flipped);
        }
    }
}

#[test]
fn each_single_data_fault_has_its_own_syndrome_and_recovers_the_logical_phase() {
    for lesson in LESSONS {
        let mut app = QCSimApp::default();
        app.handle_message(Message::LoadTemplate(format!(
            "error_correction_{}_flip.json",
            lesson.name
        )));
        let mut data: CircuitData =
            serde_json::from_str(&app.state.ui_state.circuit_json_string).unwrap();
        let fault = data.steps[lesson.fault - 1][0].id.clone();
        for (wire, syndrome) in [
            (None, 0b00),
            (Some(0), 0b01),
            (Some(1), 0b11),
            (Some(2), 0b10),
        ] {
            data.steps[lesson.fault - 1] = wire
                .into_iter()
                .map(|q| Operation::new(fault.clone(), vec![q], vec![]))
                .collect();
            app.handle_message(Message::UpdateCircuitFromJson(
                serde_json::to_string(&data).unwrap(),
            ));
            app.handle_message(Message::SelectTimestep(lesson.syndrome));
            assert_encoded(&app, lesson.phase, syndrome, wire.map_or(0, |q| 1 << q));
            app.handle_message(Message::SelectTimestep(lesson.corrected));
            assert_encoded(&app, lesson.phase, syndrome, 0);
            app.handle_message(Message::RunSimulation);
            assert_decoded(&app, syndrome, false);
        }
    }
}
