use quantsim_core::{
    circuit::{Circuit, CircuitSection, QubitLayout},
    circuits,
    types::{Gate, Operation},
};
use rand::{rngs::StdRng, Rng, SeedableRng};

pub const COLUMNS: usize = 8;
pub const WIRES: usize = 2 * COLUMNS;
const SEED: u64 = 53;

const LAYERS: [&str; 4] = [
    "Even horizontal bonds",
    "Even vertical bonds",
    "Odd horizontal bonds",
    "Odd vertical bonds",
];

pub fn layer(round: usize) -> &'static str {
    if round == 0 {
        "Input"
    } else {
        LAYERS[(round - 1) % LAYERS.len()]
    }
}

pub fn circuit(rounds: usize) -> Result<Circuit, Box<dyn std::error::Error>> {
    let chip = circuits::load_template("sycamore_53")?
        .layout
        .ok_or("Missing Sycamore layout")?;
    // Use the actual chip couplers from rows 4–5, columns 1–8. Local wire 0
    // starts the upper row and local wire 8 starts the lower row.
    let sites: Vec<_> = chip
        .positions
        .iter()
        .enumerate()
        .filter(|(_, [x, y])| (1..=8).contains(x) && (4..=5).contains(y))
        .collect();
    let positions: Vec<_> = sites.iter().map(|(_, position)| **position).collect();
    let links: Vec<_> = chip
        .links
        .iter()
        .filter_map(|[a, b]| {
            Some([
                sites.iter().position(|(site, _)| site == a)?,
                sites.iter().position(|(site, _)| site == b)?,
            ])
        })
        .collect();
    let expected_positions: Vec<_> = (4..=5).flat_map(|y| (1..=8).map(move |x| [x, y])).collect();
    if positions != expected_positions || links.len() != 22 {
        return Err("Expected a connected 2×8 patch in chip row order".into());
    }
    let layout = QubitLayout { positions, links };
    let mut circuit = Circuit::new(WIRES);
    let mut rng = StdRng::seed_from_u64(SEED);
    for round in 1..=rounds {
        let start = circuit.steps.len() + 1;
        let rotations = (0..WIRES)
            .map(|wire| {
                let gate = match rng.gen_range(0u32..3) {
                    0 => Gate::H,
                    1 => Gate::SqrtX,
                    _ => Gate::SqrtY,
                };
                Operation::new(gate, vec![wire as u32], vec![])
            })
            .collect();
        circuit.steps.push(rotations);
        let phase = (round - 1) % LAYERS.len();
        let gates = layout
            .links
            .iter()
            .filter(|&&[a, b]| {
                let horizontal = a / COLUMNS == b / COLUMNS;
                let first_column = (a % COLUMNS).min(b % COLUMNS);
                horizontal == phase.is_multiple_of(2) && first_column % 2 == phase / 2
            })
            .map(|&[a, b]| Operation::new(Gate::SYC, vec![a as u32, b as u32], vec![]))
            .collect();
        circuit.steps.push(gates);
        circuit.sections.push(CircuitSection {
            start,
            end: circuit.steps.len(),
            label: format!("Round {round}"),
            description: layer(round).into(),
        });
    }
    circuit.layout = Some(layout);
    Ok(circuit)
}
