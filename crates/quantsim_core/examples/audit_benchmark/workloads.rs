use quantsim_core::{
    circuit::{forward_qft_steps, Circuit},
    circuits,
    types::{Gate, Operation, Param, QubitPrep},
    BasisIndex,
};
use rand::{rngs::StdRng, Rng, SeedableRng};

pub const LADDER_SEED: u64 = 53;
pub const LADDER_ROUNDS: usize = 8;

pub struct Workload {
    pub circuit: Circuit,
    pub forward_depth: usize,
    pub target: BasisIndex,
    pub inverse: bool,
    pub estimated_group_width: usize,
    pub estimated_group_entries: usize,
}

fn gate(id: Gate, wires: &[usize]) -> Operation {
    Operation::new(id, wires.iter().map(|&q| q as u32).collect(), vec![])
}

fn phase(id: Gate, wires: &[usize], exponent: f32) -> Operation {
    let mut op = gate(id, wires);
    op.params.push(Param::Scalar(exponent));
    op
}

pub fn product_phase(wire: usize) -> f32 {
    ((wire % 7) + 1) as f32 / 16.0
}

pub fn qft_input(n: usize) -> BasisIndex {
    (BasisIndex::from(1u8) << (n - 5)) | BasisIndex::from(1u8)
}

fn inverse_steps(steps: &[Vec<Operation>]) -> Vec<Vec<Operation>> {
    steps
        .iter()
        .rev()
        .map(|column| {
            column
                .iter()
                .rev()
                .cloned()
                .map(|mut op| {
                    if let Some(Param::Scalar(exponent)) = op.params.first_mut() {
                        *exponent = -*exponent;
                    }
                    op
                })
                .collect()
        })
        .collect()
}

fn tree(n: usize) -> Vec<Vec<Operation>> {
    let mut steps = vec![vec![gate(Gate::H, &[0])]];
    let mut stride = 1;
    while stride < n {
        steps.push(
            (0..stride.min(n - stride))
                .map(|q| gate(Gate::CX, &[q, q + stride]))
                .collect(),
        );
        stride *= 2;
    }
    steps
}

fn sycamore_echo(n: usize, phase_flip: bool) -> (Circuit, BasisIndex, usize) {
    let template = circuits::load_template("sycamore_echo_53").unwrap();
    if n == 53 {
        let mut circuit = template;
        if !phase_flip {
            circuit.steps.remove(8);
            circuit.sections.clear();
        }
        return (
            circuit,
            BasisIndex::from(u32::from(phase_flip)) << 23usize,
            8,
        );
    }
    assert!(n < 53);
    let mut circuit = Circuit::new(n);
    circuit.steps.push(vec![gate(Gate::H, &[0])]);
    let mut mapped = [None; 53];
    mapped[23] = Some(0usize);
    let mut next = 1;
    for layer in &template.steps[1..8] {
        let mut column = Vec::new();
        for op in layer {
            if next == n {
                break;
            }
            let parent = mapped[op.qubits[0] as usize].unwrap();
            mapped[op.qubits[1] as usize] = Some(next);
            column.push(gate(Gate::CX, &[parent, next]));
            next += 1;
        }
        if !column.is_empty() {
            circuit.steps.push(column);
        }
    }
    let forward_depth = circuit.steps.len();
    let reverse = inverse_steps(&circuit.steps);
    if phase_flip {
        circuit
            .steps
            .push(vec![gate(Gate::Z, &[mapped[33].unwrap_or(n - 1)])]);
    }
    circuit.steps.extend(reverse);
    (
        circuit,
        BasisIndex::from(u32::from(phase_flip)),
        forward_depth,
    )
}

fn ladder(n: usize) -> Circuit {
    let cols = n / 2;
    let mut circuit = Circuit::new(n);
    let mut rng = StdRng::seed_from_u64(LADDER_SEED);
    for round in 0..LADDER_ROUNDS {
        circuit.steps.push(
            (0..n)
                .map(|q| {
                    let id = match rng.gen_range(0u32..3) {
                        0 => Gate::H,
                        1 => Gate::SqrtX,
                        _ => Gate::SqrtY,
                    };
                    gate(id, &[q])
                })
                .collect(),
        );
        let phase = round % 4;
        let mut links = Vec::new();
        if phase % 2 == 0 {
            for row in 0..2 {
                for col in (phase / 2..cols - 1).step_by(2) {
                    let q = row * cols + col;
                    links.push(gate(Gate::SYC, &[q, q + 1]));
                }
            }
        } else {
            for col in (phase / 2..cols).step_by(2) {
                links.push(gate(Gate::SYC, &[col, col + cols]));
            }
        }
        circuit.steps.push(links);
    }
    circuit
}

pub fn build(name: &str, n: usize, variant: &str) -> Workload {
    let inverse = matches!(variant, "reverse" | "decode");
    let mut circuit = Circuit::new(n);
    let (mut target, mut width, mut entries) = (BasisIndex::default(), 1, 2 * n);
    match name {
        "product" => {
            circuit
                .steps
                .push((0..n).map(|q| gate(Gate::H, &[q])).collect());
            circuit.steps.push(
                (0..n)
                    .map(|q| phase(Gate::ZPow, &[q], product_phase(q)))
                    .collect(),
            );
        }
        "ghz" => {
            circuit.steps = tree(n);
            (width, entries) = (n, 1usize.checked_shl(n as u32).unwrap_or(0));
        }
        "groups" => {
            let blocks: Vec<_> = (0..n).step_by(4).map(|q| (q, (n - q).min(4))).collect();
            circuit.steps = vec![
                blocks.iter().map(|&(q, _)| gate(Gate::H, &[q])).collect(),
                blocks
                    .iter()
                    .filter(|&&(_, k)| k >= 2)
                    .map(|&(q, _)| gate(Gate::CX, &[q, q + 1]))
                    .collect(),
                blocks
                    .iter()
                    .flat_map(|&(q, k)| (2..k).map(move |i| gate(Gate::CX, &[q + i - 2, q + i])))
                    .collect(),
                blocks
                    .iter()
                    .map(|&(q, k)| phase(Gate::ZPow, &[q + k - 1], 0.25))
                    .collect(),
            ];
            (width, entries) = (4, blocks.iter().map(|&(_, k)| 1usize << k).sum());
        }
        "qft" => {
            target = qft_input(n);
            for q in 0..n {
                if target.bit(q as u64) {
                    circuit.initial_preps[q] = Some(QubitPrep::One);
                }
            }
            circuit.steps = forward_qft_steps(0, n).unwrap();
        }
        "non_clifford" => {
            // Bell pairs across the fixed half-register cut. Phases within the
            // low half preserve that Schmidt spectrum and link the pair factors.
            let half = n / 2;
            circuit.steps = vec![
                (0..half).map(|q| gate(Gate::H, &[q])).collect(),
                (0..half).map(|q| gate(Gate::CX, &[q, q + half])).collect(),
                (0..half).map(|q| phase(Gate::ZPow, &[q], 0.25)).collect(),
                (0..half - 1)
                    .step_by(2)
                    .map(|q| phase(Gate::CZPow, &[q, q + 1], 0.25))
                    .collect(),
                (1..half - 1)
                    .step_by(2)
                    .map(|q| phase(Gate::CZPow, &[q, q + 1], 0.25))
                    .collect(),
            ];
            (width, entries) = (n, 1usize.checked_shl(n as u32).unwrap_or(0));
        }
        "sycamore_echo" => {
            let (circuit, target, forward_depth) = sycamore_echo(n, variant == "phase_flip");
            return Workload {
                circuit,
                forward_depth,
                target,
                inverse: true,
                estimated_group_width: n,
                estimated_group_entries: 1usize.checked_shl(n as u32).unwrap_or(0),
            };
        }
        "mps_ladder" => circuit = ladder(n),
        _ => panic!("unknown workload {name}"),
    }
    let forward_depth = circuit.steps.len();
    if variant == "decode" {
        assert_eq!(name, "qft");
        // Undo the two independent Fourier phase contributions on each wire,
        // then map |+> back to zero. This measures coherent phase recovery
        // without requiring the product backend to admit the generic IQFT.
        circuit.steps.push(
            (0..5)
                .map(|q| phase(Gate::ZPow, &[q], -2f32.powi(q as i32 - 4)))
                .collect(),
        );
        circuit.steps.push(
            (n.saturating_sub(150)..n)
                .map(|q| phase(Gate::ZPow, &[q], -2f64.powi(q as i32 + 1 - n as i32) as f32))
                .collect(),
        );
        circuit
            .steps
            .push((0..n).map(|q| gate(Gate::H, &[q])).collect());
        target = BasisIndex::default();
    } else if inverse {
        // For the ladder, the runner treats the reversed operations as
        // adjoints; this keeps the gate count and authored depth explicit.
        circuit.steps.extend(inverse_steps(&circuit.steps));
    }
    Workload {
        circuit,
        forward_depth,
        target,
        inverse,
        estimated_group_width: width,
        estimated_group_entries: entries,
    }
}
