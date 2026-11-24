use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Physical sites in wire order and undirected connections between wire indices.
/// These annotations describe the drawing independently of the simulated gates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QubitLayout {
    pub positions: Vec<[i64; 2]>,
    #[serde(default)]
    pub links: Vec<[usize; 2]>,
}

impl QubitLayout {
    /// Sites in wire order, with no assumed physical connections.
    pub fn wire_order(count: usize) -> Self {
        Self {
            positions: (0..count).map(|wire| [wire as i64, 0]).collect(),
            links: Vec::new(),
        }
    }

    pub fn line(count: usize) -> Self {
        Self {
            links: (1..count).map(|wire| [wire - 1, wire]).collect(),
            ..Self::wire_order(count)
        }
    }

    pub fn grid(count: usize) -> Self {
        let columns = count.saturating_sub(1).isqrt() + 1;
        let mut positions = Vec::with_capacity(count);
        let mut links = Vec::new();
        for wire in 0..count {
            let column = wire % columns;
            positions.push([column as i64, (wire / columns) as i64]);
            if column > 0 {
                links.push([wire - 1, wire]);
            }
            if wire >= columns {
                links.push([wire - columns, wire]);
            }
        }
        Self { positions, links }
    }

    pub fn ring(count: usize) -> Self {
        if count < 3 {
            return Self::line(count);
        }
        // A radius proportional to the number of sites keeps them distinct on
        // the integer lattice as more qubits are added around the circle.
        let radius = count as f64;
        let positions = (0..count)
            .map(|wire| {
                let angle = std::f64::consts::TAU * wire as f64 / count as f64;
                [
                    (radius * angle.sin()).round() as i64,
                    (-radius * angle.cos()).round() as i64,
                ]
            })
            .collect();
        let links = (0..count).map(|wire| [wire, (wire + 1) % count]).collect();
        Self { positions, links }
    }

    pub fn validate(&self, count: usize) -> Result<(), String> {
        if self.positions.len() != count {
            return Err(format!(
                "Qubit layout needs one position per wire. Expected {count}, found {}.",
                self.positions.len()
            ));
        }
        let mut occupied = HashSet::new();
        for &[x, y] in &self.positions {
            if !occupied.insert([x, y]) {
                return Err(format!("Two qubit layout sites occupy ({x}, {y})."));
            }
        }
        let mut connected = HashSet::new();
        for &[a, b] in &self.links {
            if a >= count || b >= count {
                return Err(format!(
                    "Qubit layout link q{a}–q{b} is outside the register."
                ));
            }
            if a == b {
                return Err(format!(
                    "Qubit layout link q{a}–q{b} connects a site to itself."
                ));
            }
            if !connected.insert([a.min(b), a.max(b)]) {
                return Err(format!(
                    "Qubit layout link q{a}–q{b} appears more than once."
                ));
            }
        }
        Ok(())
    }

    /// Keeps surviving sites and links. Added wires occupy free sites with no links.
    pub fn resize(&mut self, count: usize) {
        if count <= self.positions.len() {
            self.positions.truncate(count);
            self.links.retain(|&[a, b]| a < count && b < count);
            return;
        }
        let occupied: HashSet<_> = self.positions.iter().copied().collect();
        let free_sites = (0..=i64::MAX)
            .map(|x| [x, 0])
            .filter(|position| !occupied.contains(position));
        self.positions
            .extend(free_sites.take(count - self.positions.len()));
    }
}

#[cfg(test)]
mod tests {
    use super::QubitLayout;
    use crate::circuit::{Circuit, CircuitData, RunOptions};
    use crate::CircuitBuilder;
    use serde_json::json;

    #[test]
    fn presets_have_unique_sites_and_only_their_neighbor_links() {
        for count in 0..=21 {
            for layout in [
                QubitLayout::line(count),
                QubitLayout::grid(count),
                QubitLayout::ring(count),
            ] {
                layout.validate(count).unwrap();
            }
        }
        assert_eq!(QubitLayout::line(3).links, [[0, 1], [1, 2]]);
        assert_eq!(QubitLayout::ring(4).links, [[0, 1], [1, 2], [2, 3], [3, 0]]);
        let grid = QubitLayout::grid(5);
        assert_eq!(grid.positions, [[0, 0], [1, 0], [2, 0], [0, 1], [1, 1]]);
        assert_eq!(grid.links, [[0, 1], [1, 2], [0, 3], [3, 4], [1, 4]]);
    }

    #[test]
    fn wide_presets_and_added_sites_keep_distinct_coordinates() {
        let count = 65_537;
        let mut line = QubitLayout::line(count);
        line.validate(count).unwrap();
        assert_eq!(line.positions[count - 1], [65_536, 0]);
        line.resize(count + 2);
        assert_eq!(&line.positions[count..], &[[65_537, 0], [65_538, 0]]);
        QubitLayout::ring(count).validate(count).unwrap();
    }

    #[test]
    fn layout_round_trips_and_does_not_restrict_gates() {
        let mut builder = CircuitBuilder::new(2);
        builder.h(0).next_step().cx(0, 1);
        let mut circuit = builder.build();
        let options = RunOptions::default();
        let expected = circuit.run(&options).final_state_vector.unwrap();
        let layout = QubitLayout {
            positions: vec![[-1, 2], [4, 7]],
            links: vec![],
        };
        circuit.layout = Some(layout.clone());
        let saved = serde_json::to_value(CircuitData::from(circuit)).unwrap();
        assert_eq!(
            saved["layout"],
            json!({"positions":[[-1,2],[4,7]],"links":[]})
        );
        let mut circuit = Circuit::new(1);
        circuit
            .update_from_data(serde_json::from_value(saved).unwrap())
            .unwrap();
        assert_eq!(circuit.layout, Some(layout));
        assert_eq!(circuit.run(&options).final_state_vector.unwrap(), expected);
        let legacy = serde_json::from_value(json!({"num_qubits":1,"steps":[]})).unwrap();
        circuit.update_from_data(legacy).unwrap();
        assert_eq!(circuit.layout, None);
        assert!(serde_json::to_value(CircuitData::from(circuit))
            .unwrap()
            .get("layout")
            .is_none());
    }

    #[test]
    fn invalid_layout_import_preserves_the_existing_circuit() {
        let mut circuit = Circuit::new(1);
        circuit.layout = Some(QubitLayout::line(1));
        for invalid in [
            json!({"positions":[[0,0]]}),
            json!({"positions":[[0,0],[0,0]]}),
            json!({"positions":[[0,0],[0,1]],"links":[[0,2]]}),
            json!({"positions":[[0,0],[0,1]],"links":[[1,1]]}),
            json!({"positions":[[0,0],[0,1]],"links":[[0,1],[1,0]]}),
        ] {
            let data =
                serde_json::from_value(json!({"num_qubits":2,"steps":[[]],"layout":invalid}))
                    .unwrap();
            assert!(circuit.update_from_data(data).is_err());
            assert_eq!(circuit.num_qubits, 1);
            assert_eq!(circuit.layout, Some(QubitLayout::line(1)));
            assert!(circuit.steps.is_empty());
        }
    }

    #[test]
    fn resizing_retains_wire_identity_and_adds_unconnected_sites() {
        let mut layout = QubitLayout {
            positions: vec![[1, 0], [i64::MAX, 0], [-1, 0]],
            links: vec![[0, 1], [1, 2]],
        };
        layout.resize(2);
        assert_eq!(layout.positions, [[1, 0], [i64::MAX, 0]]);
        assert_eq!(layout.links, [[0, 1]]);
        layout.resize(4);
        assert_eq!(layout.positions, [[1, 0], [i64::MAX, 0], [0, 0], [2, 0]]);
        assert_eq!(layout.links, [[0, 1]]);
        layout.validate(4).unwrap();
    }
}
