use super::{sections, Circuit, CircuitSection, Gate, Operation};

// A CZPow parameter is stored as f32. 2^-149 is its smallest positive
// value; every smaller power of two rounds to zero (the identity).
const LAST_PHASE_DISTANCE: usize = 149;

/// Builds the forward QFT on consecutive wires, with `first_wire` as the
/// least significant bit. The last column reverses the wire order for the
/// usual Fourier output ordering when the transform spans multiple wires.
/// Rotations that round to the identity at the gate parameter precision are omitted.
pub fn forward_qft_steps(first_wire: usize, width: usize) -> Result<Vec<Vec<Operation>>, String> {
    let span = width
        .checked_sub(1)
        .ok_or("A QFT needs at least one wire.")?;
    let last_wire = first_wire
        .checked_add(span)
        .filter(|&last| u32::try_from(last).is_ok())
        .ok_or("The QFT's final wire exceeds the circuit wire-index range.")?;

    let depth = if width == 1 {
        1
    } else {
        width
            .checked_mul(2)
            .ok_or("QFT depth exceeds the addressable range.")?
    };
    let mut steps = Vec::new();
    steps
        .try_reserve_exact(depth)
        .map_err(|e| format!("Cannot allocate {depth} QFT columns: {e}"))?;
    steps.resize_with(depth, Vec::new);
    for target in (first_wire..=last_wire).rev() {
        // Two columns between successive Hadamards let disjoint phase gates
        // overlap. Each control contributes its phase before its own Hadamard.
        let hadamard_step = 2 * (last_wire - target);
        steps[hadamard_step].push(Operation::new(Gate::H, vec![target as u32], vec![]));
        for control in first_wire.max(target.saturating_sub(LAST_PHASE_DISTANCE))..target {
            let distance = target - control;
            steps[hadamard_step + distance].push(Operation::new(
                Gate::CZPow,
                vec![control as u32, target as u32],
                vec![2.0_f64.powi(-(distance as i32)) as f32],
            ));
        }
    }
    if width > 1 {
        let swaps = steps.last_mut().unwrap();
        swaps
            .try_reserve_exact(width / 2)
            .map_err(|e| format!("Cannot allocate QFT swaps: {e}"))?;
        for offset in 0..width / 2 {
            swaps.push(Operation::new(
                Gate::SWAP,
                vec![(first_wire + offset) as u32, (last_wire - offset) as u32],
                vec![],
            ));
        }
    }
    Ok(steps)
}

impl Circuit {
    /// Inserts a forward QFT after `after_step` existing columns (zero prepends).
    /// Returns the number of added columns. Original columns retain their
    /// gates and section annotations on either side of the insertion.
    pub fn insert_qft(
        &mut self,
        after_step: usize,
        first_wire: usize,
        width: usize,
    ) -> Result<usize, String> {
        if after_step > self.steps.len() {
            return Err(format!(
                "Choose a step from 0 through {}.",
                self.steps.len()
            ));
        }
        if width > self.num_qubits.saturating_sub(first_wire) {
            return Err(format!(
                "The selected QFT must fit within the circuit's {} wires.",
                self.num_qubits
            ));
        }
        let steps = forward_qft_steps(first_wire, width)?;
        let mut annotations = self.sections.clone();
        annotations.sort_unstable_by_key(|section| section.start);
        sections::validate_sorted_sections(annotations.iter(), self.steps.len())?;

        let depth = steps.len();
        self.steps
            .try_reserve(depth)
            .map_err(|e| format!("Cannot insert {depth} QFT columns: {e}"))?;
        let mut insert_at = annotations.partition_point(|section| section.end <= after_step);
        if let Some(section) = annotations
            .get_mut(insert_at)
            .filter(|s| s.start <= after_step)
        {
            let mut tail = section.clone();
            tail.start = after_step + 1;
            section.end = after_step;
            insert_at += 1;
            annotations.insert(insert_at, tail);
        }
        for section in &mut annotations[insert_at..] {
            section.start += depth;
            section.end += depth;
        }
        annotations.insert(
            insert_at,
            CircuitSection {
                start: after_step + 1,
                end: after_step + depth,
                label: "QFT".into(),
                description: format!(
                    "{width}-qubit forward Fourier transform starting at q{first_wire}."
                ),
            },
        );
        self.steps.splice(after_step..after_step, steps);
        self.sections = annotations;
        Ok(depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{circuit::CircuitData, circuits::load_template, types::QubitPrep};
    use num_complex::{Complex32, Complex64};

    #[test]
    fn forward_transform_on_middle_wires_matches_the_fourier_kernel() {
        let mut circuit = Circuit::new(5);
        circuit.steps = forward_qft_steps(1, 3).unwrap();
        for input in 0..8 {
            // Keep both spectator wires in |1⟩.
            let basis = 0b10001 | (input << 1);
            for (wire, prep) in circuit.initial_preps.iter_mut().enumerate() {
                *prep = (basis & (1 << wire) != 0).then_some(QubitPrep::One);
            }
            let output = circuit.run(&Default::default());
            assert!(output.diagnostics.is_empty());
            for (index, actual) in output.final_state_vector.unwrap().into_iter().enumerate() {
                let expected = if index & 0b10001 == 0b10001 {
                    let frequency = (index >> 1) & 7;
                    let angle = std::f32::consts::TAU * ((input * frequency) % 8) as f32 / 8.0;
                    Complex32::from_polar(1.0 / 8.0_f32.sqrt(), angle)
                } else {
                    Complex32::ZERO
                };
                assert!(
                    (actual - expected).norm() < 1e-6,
                    "input {input}, index {index}"
                );
            }
        }
    }

    #[test]
    fn generated_schedule_matches_the_50_wire_example() {
        let example = load_template("qft_50").unwrap();
        assert_eq!(forward_qft_steps(0, 50).unwrap(), &example.steps[1..]);
    }

    #[test]
    fn full_width_basis_inputs_match_the_fourier_kernel() {
        let mut circuit = Circuit::new(128);
        circuit.steps = forward_qft_steps(0, 128).unwrap();
        let magnitude = 2.0_f64.powi(-64);
        for input in [1_u128 << 123, (1_u128 << 123) | 1, 1] {
            for (wire, prep) in circuit.initial_preps.iter_mut().enumerate() {
                *prep = (input & (1_u128 << wire) != 0).then_some(QubitPrep::One);
            }
            let state = circuit.run_product_prefix(circuit.steps.len()).unwrap();
            for index in (0_u128..64).chain([1 << 63, 1 << 64, (1 << 127) | 7, u128::MAX]) {
                // Reduce modulo 2^128 before conversion. For input 2^123 the
                // first 64 indices traverse exactly two periods of 32 samples.
                let turns = input.wrapping_mul(index) as f64 * 2.0_f64.powi(-128);
                let expected = Complex64::from_polar(magnitude, std::f64::consts::TAU * turns);
                let actual = state.amplitude(index).unwrap();
                assert!(
                    (actual - expected).norm() < 1e-12 * magnitude,
                    "input {input}, index {index}: {actual} vs {expected}"
                );
                if input == 1 && index == 1 {
                    // This imaginary component depends on the subnormal 2^-127
                    // controlled-phase exponent between q0 and q127.
                    assert!((actual.im - expected.im).abs() < 1e-12 * expected.im);
                }
            }
        }
    }

    #[test]
    fn generator_handles_register_boundaries() {
        let one = forward_qft_steps(u32::MAX as usize, 1).unwrap();
        assert_eq!(one, [vec![Operation::new(Gate::H, vec![u32::MAX], vec![])]]);

        for register_width in [64, 128] {
            for width in 2..=register_width {
                let first_wire = register_width - width;
                let steps = forward_qft_steps(first_wire, width).unwrap();
                assert_eq!(steps.len(), 2 * width);
                let gates = width + width * (width - 1) / 2 + width / 2;
                assert_eq!(steps.iter().map(Vec::len).sum::<usize>(), gates);
                assert_eq!(steps.last().unwrap().len(), width / 2);
                for column in steps {
                    let mut occupied = 0_u128;
                    for operation in column {
                        for wire in operation.qubits {
                            assert!((first_wire..register_width).contains(&(wire as usize)));
                            let bit = 1_u128 << wire;
                            assert_eq!(occupied & bit, 0);
                            occupied |= bit;
                        }
                    }
                }
            }
        }
        for (first, width) in [(0, 0), (u32::MAX as usize, 2), (usize::MAX, usize::MAX)] {
            assert!(forward_qft_steps(first, width).is_err());
        }
    }

    #[test]
    fn wide_blocks_use_high_wires_and_keep_all_representable_phases() {
        use crate::types::Param;

        let (first, width) = (200_usize, 151_usize);
        let last = first + width - 1;
        let steps = forward_qft_steps(first, width).unwrap();
        assert_eq!(steps.len(), 2 * width);
        assert_eq!(
            steps.iter().map(Vec::len).sum::<usize>(),
            width + width * (width - 1) / 2 - 1 + width / 2
        );
        for (i, column) in steps.iter().enumerate() {
            let mut used = std::collections::HashSet::new();
            for op in column {
                for &wire in &op.qubits {
                    assert!((first..=last).contains(&(wire as usize)));
                    assert!(used.insert(wire), "wire {wire} occurs twice in column {i}");
                }
            }
        }
        assert!(steps[149].iter().any(|op| op.qubits.as_slice()
            == [(first + 1) as u32, last as u32]
            && op.params.as_slice() == [Param::Scalar(f32::from_bits(1))]));
        assert!(steps
            .iter()
            .flatten()
            .all(|op| op.qubits.as_slice() != [first as u32, last as u32] || op.id == Gate::SWAP));

        // Input 2^(width-5) creates one phase turn per 32 local basis indices.
        let mut circuit = Circuit::new(last + 1);
        circuit.steps = steps;
        circuit.initial_preps[first + width - 5] = Some(QubitPrep::One);
        let output = circuit.run_product_prefix(circuit.steps.len()).unwrap();
        let magnitude = (-0.5 * width as f64).exp2();
        for j in [0u32, 1, 8, 31, 32, 47] {
            let index =
                (crate::BasisIndex::from(j) << first) | (crate::BasisIndex::from(1u32) << last);
            let expected =
                Complex64::from_polar(magnitude, std::f64::consts::TAU * f64::from(j % 32) / 32.0);
            assert!((output.amplitude_wide(&index).unwrap() - expected).norm() < 1e-12 * magnitude);
        }
    }

    #[test]
    fn insertion_preserves_existing_columns_and_their_annotations() {
        let mut original = Circuit::new(4);
        original.steps = (0..4)
            .map(|wire| vec![Operation::new(Gate::X, vec![wire], vec![])])
            .collect();
        original.sections = vec![
            CircuitSection {
                start: 1,
                end: 3,
                label: "Input".into(),
                description: "Prepare data.".into(),
            },
            CircuitSection {
                start: 4,
                end: 4,
                label: "Output".into(),
                description: String::new(),
            },
        ];
        for after in 0..=4 {
            let mut circuit = original.clone();
            circuit.sections.reverse(); // Public Circuit fields can be authored in any order.
            let depth = circuit.insert_qft(after, 1, 2).unwrap();
            assert_eq!(depth, 4);
            assert_eq!(
                &circuit.steps[after..after + depth],
                forward_qft_steps(1, 2).unwrap()
            );
            for step in 1..=4 {
                let new_step = if step > after { step + depth } else { step };
                assert_eq!(circuit.steps[new_step - 1], original.steps[step - 1]);
                let old = original.section_at(step).unwrap();
                let new = circuit.section_at(new_step).unwrap();
                assert_eq!(
                    (&new.label, &new.description),
                    (&old.label, &old.description)
                );
            }
            for step in after + 1..=after + depth {
                let section = circuit.section_at(step).unwrap();
                assert_eq!(section.label, "QFT");
                assert_eq!((section.start, section.end), (after + 1, after + depth));
            }
            let saved = serde_json::to_string(&CircuitData::from(circuit.clone())).unwrap();
            let mut restored = Circuit::new(1);
            restored
                .update_from_data(serde_json::from_str(&saved).unwrap())
                .unwrap();
            assert_eq!(restored.sections, circuit.sections);
            assert_eq!(restored.steps, circuit.steps);
        }
        for (after, first, width) in [(5, 1, 2), (1, 4, 1), (1, 3, 2), (1, 0, 0), (1, 0, 65)] {
            let mut circuit = original.clone();
            assert!(circuit.insert_qft(after, first, width).is_err());
            assert_eq!(circuit.steps, original.steps);
            assert_eq!(circuit.sections, original.sections);
        }
    }
}
