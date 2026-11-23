use super::{Circuit, CircuitData};
use serde::{Deserialize, Serialize};

/// A simulation-neutral annotation spanning inclusive, one-based gate columns.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircuitSection {
    pub start: usize,
    pub end: usize,
    pub label: String,
}

impl CircuitSection {
    fn validate(&self, num_steps: usize) -> Result<(), String> {
        if self.label.trim().is_empty() {
            return Err("Enter a label for the section.".into());
        }
        if self.start == 0 || self.start > self.end || self.end > num_steps {
            return Err(format!(
                "Section steps {}–{} are outside the valid range or reversed (circuit length {num_steps}).",
                self.start, self.end
            ));
        }
        Ok(())
    }

    fn check_disjoint(&self, other: &Self) -> Result<(), String> {
        if self.start <= other.end && other.start <= self.end {
            return Err(format!(
                "Section steps {}–{} overlap {}–{} ({}).",
                self.start, self.end, other.start, other.end, other.label
            ));
        }
        Ok(())
    }
}

pub(super) fn validate_sorted_sections<'a>(
    sections: impl Iterator<Item = &'a CircuitSection>,
    num_steps: usize,
) -> Result<(), String> {
    let mut previous = None;
    for section in sections {
        section.validate(num_steps)?;
        if let Some(previous) = previous {
            section.check_disjoint(previous)?;
        }
        previous = Some(section);
    }
    Ok(())
}

impl CircuitData {
    /// Validates annotated ranges in either sorted or unsorted input.
    pub fn validate_sections(&self) -> Result<(), String> {
        let mut ordered: Vec<_> = self.sections.iter().collect();
        ordered.sort_unstable_by_key(|section| section.start);
        validate_sorted_sections(ordered.into_iter(), self.steps.len())
    }
}

impl Circuit {
    /// Adds a section, or replaces the section identified by its original first step.
    pub fn set_section(
        &mut self,
        original_start: Option<usize>,
        section: CircuitSection,
    ) -> Result<(), String> {
        section.validate(self.steps.len())?;
        let mut replacing = None;
        for (index, other) in self.sections.iter().enumerate() {
            if Some(other.start) == original_start {
                replacing = Some(index);
            } else {
                section.check_disjoint(other)?;
            }
        }
        if let Some(index) = replacing {
            self.sections[index] = section;
        } else if original_start.is_none() {
            self.sections.push(section);
        } else {
            return Err("The section being edited no longer exists.".into());
        }
        self.sections.sort_unstable_by_key(|section| section.start);
        Ok(())
    }

    pub fn remove_section(&mut self, start: usize) {
        self.sections.retain(|section| section.start != start);
    }

    pub fn section_at(&self, step: usize) -> Option<&CircuitSection> {
        self.sections
            .iter()
            .find(|section| (section.start..=section.end).contains(&step))
    }

    /// Keeps only surviving gate columns after a circuit is shortened.
    pub fn trim_sections(&mut self) {
        let last = self.steps.len();
        self.sections.retain_mut(|section| {
            section.end = section.end.min(last);
            section.start <= section.end
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn section(start: usize, end: usize, label: &str) -> CircuitSection {
        CircuitSection {
            start,
            end,
            label: label.into(),
        }
    }

    #[test]
    fn sections_import_in_any_order_and_round_trip() {
        let mut circuit = Circuit::new(1);
        let json = json!({
            "num_qubits": 2,
            "steps": [[], [], [], [], []],
            "sections": [
                {"start": 5, "end": 5, "label": "Readout"},
                {"start": 2, "end": 3, "label": "Couple"}
            ]
        });
        let data: CircuitData = serde_json::from_value(json).unwrap();
        data.validate_sections().unwrap();
        circuit.update_from_data(data).unwrap();
        assert_eq!(
            circuit.sections,
            [section(2, 3, "Couple"), section(5, 5, "Readout")]
        );
        assert_eq!(circuit.section_at(0), None);
        assert_eq!(circuit.section_at(2), circuit.section_at(3));
        assert_eq!(circuit.section_at(3).unwrap().label, "Couple");
        assert_eq!(circuit.section_at(4), None);
        let saved = serde_json::to_value(CircuitData::from(circuit.clone())).unwrap();
        assert_eq!(
            saved["sections"][0],
            json!({"start":2, "end":3, "label":"Couple"})
        );
        let legacy = serde_json::from_value(json!({"num_qubits":1,"steps":[]})).unwrap();
        circuit.update_from_data(legacy).unwrap();
        let saved = serde_json::to_value(CircuitData::from(circuit)).unwrap();
        assert!(saved.get("sections").is_none());
    }

    #[test]
    fn failed_section_import_or_edit_preserves_the_current_circuit() {
        let mut circuit = Circuit::new(1);
        circuit.steps.resize(5, vec![]);
        circuit.set_section(None, section(1, 2, "Encode")).unwrap();
        circuit.set_section(None, section(4, 5, "Read")).unwrap();
        let prior = circuit.sections.clone();
        for invalid in [
            section(0, 2, "First"),
            section(1, 6, "Past end"),
            section(3, 2, "Reversed"),
            section(1, 2, "  "),
            section(2, 4, "Overlap"),
        ] {
            assert!(circuit.set_section(Some(1), invalid.clone()).is_err());
            let mut data = CircuitData::from(circuit.clone());
            data.num_qubits = 4;
            data.sections[0] = invalid;
            assert!(data.validate_sections().is_err());
            assert!(circuit.update_from_data(data).is_err());
            assert_eq!(circuit.num_qubits, 1);
            assert_eq!(circuit.sections, prior);
        }
        assert!(circuit
            .set_section(Some(3), section(3, 3, "Missing"))
            .is_err());
        assert_eq!(circuit.sections, prior);
    }

    #[test]
    fn moving_and_trimming_sections_keeps_surviving_columns() {
        let mut circuit = Circuit::new(2);
        circuit.steps.resize(7, vec![]);
        circuit.set_section(None, section(4, 4, "Read")).unwrap();
        circuit.set_section(None, section(1, 2, "Encode")).unwrap();
        circuit
            .set_section(Some(1), section(5, 7, "Decode"))
            .unwrap();
        circuit.steps.truncate(6);
        circuit.trim_sections();
        assert_eq!(
            circuit.sections,
            [section(4, 4, "Read"), section(5, 6, "Decode")]
        );
        circuit.remove_section(4);
        assert_eq!(circuit.section_at(4), None);
        circuit.steps.truncate(3);
        circuit.trim_sections();
        assert!(circuit.sections.is_empty());
    }
}
