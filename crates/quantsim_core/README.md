# quantsim_core

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-informational?style=flat-square)](../../COPYRIGHT.md)
![Rust](https://img.shields.io/badge/Rust-orange?logo=rust)

A pure Rust library for quantum circuit simulation. This crate provides the core simulation engine for quantum computing, designed to be portable and reusable without any UI dependencies.

## Features

- **Pure Rust Implementation**: No UI dependencies, making it suitable for integration into any Rust project
- **Comprehensive Gate Library**: Support for standard quantum gates including single-qubit, multi-qubit, and parametric gates
- **Custom Gate Support**: Define your own unitary gates programmatically
- **Embedded Circuit Templates**: Built-in quantum algorithm examples (Bell states, Grover's algorithm, QFT, etc.)
- **Parallel Execution**: Uses Rayon for efficient parallelization of computationally intensive operations
- **Introspection API**: Detailed per-timestep snapshots of quantum state evolution
- **JSON Serialization**: Easy circuit persistence and sharing

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
quantsim_core = { git = "https://github.com/benschneider/quantum_algorithm_simulator", tag = "v0.4.1" }
```

### Basic Example

```rust
use quantsim_core::{circuit::RunOptions, CircuitBuilder};

let mut builder = CircuitBuilder::new(2);
builder.h(0).next_step().cx(0, 1);
let result = builder.build().run(&RunOptions::default());
assert!(result.diagnostics.is_empty());
assert!((result.final_probabilities[0b11] - 0.5).abs() < 1e-6);
```

### Compact circuit readout (current checkout)

`Circuit::run` and `run_prefix` use a dense state vector. For a wide circuit, select one of the compact methods below. For example, the [512-wire phase echo](circuits/phase_echo_512.json) works with `run_product_prefix`, and the [1,024-wire entangled blocks](circuits/grouped_blocks_1024.json) work with `run_grouped_prefix`.

`circuit.run_stabilizer_prefix(step_count)` accepts per-wire preparations and the built-in H, SqrtZ (S), CX, X, Y, and Z gates. Call `state.probability_one(wire)` for an exact wire probability or `state.sample_wide(shots, seed)` for full-width joint samples, with q0 at bit zero. For registers of up to 128 wires, `state.sample(shots, seed)` returns samples packed into `u128`. See the [50-wire GHZ example](src/core/stabilizer.rs).

The native GHZ50 test measured **28,264 bytes of peak additional live heap** on Apple Silicon after circuit construction, including evolution, all 50 wire probabilities, sampling scratch space, and 1,024 returned samples. To reproduce from the workspace root, run `cargo test --manifest-path crates/quantsim_core/Cargo.toml --test stabilizer_scale -- --nocapture`.

`circuit.run_product_prefix(step_count)` provides the same readout interface for product states. It evolves built-in single-wire gates, SWAP, and eligible CX, CZ and CZPow operations using one pair of complex amplitudes per wire. In the [50-qubit QFT](circuits/qft_50.json), each controlled phase has a partner wire that is still a definite bit, so its exponent can be edited freely. A controlled gate that the backend cannot prove preserves separability returns its column and operation index. This can also happen after arbitrary rotations cancel back to a basis state.

`circuit.run_grouped_prefix(step_count)` keeps a state vector for each group of wires that must evolve together. A group can span nonadjacent wires. A group of `k` wires stores `2^k` complex amplitudes at 16 bytes each. Merges and splits also need temporary buffers. SWAP exchanges wire labels. After a multi-wire gate, the simulator checks the affected group for individual wires whose two conditional amplitude rows are proportional at stored precision. It stores those wires independently again. Rounding can leave a separable wire grouped. `state.group_wires(wire)` reports which physical wires are stored together.

For an individual complex amplitude, call `product.amplitude_wide(&index)`, `grouped.amplitude_wide(&index)`, or construct `let view = stabilizer.amplitude_view()` once and call `view.amplitude_wide(&index)` for each point. Use `quantsim_core::BasisIndex` for indices of any register width. The corresponding `amplitude(index)` methods accept a `u128` index. The product and grouped states retain the gate-defined phase. The stabilizer view fixes global phase by making the lowest occupied index positive real.

### Matrix product state experiment on a 2×8 Sycamore patch

This native example compares an eight-tensor MPS against the simulator’s single-precision dense engine and a double-precision dense reference. It checks all 65,536 complex amplitudes after every round. To try the same circuit in this checkout, paste the exported JSON into the editor’s **JSON** tab, select **Apply Changes**, then **Run ▶**. Each round contains a rotation step and a coupling step. The final dense readout is at step 32. [16-round patch JSON](examples/sycamore_mps/patch.json)

Run from the repository root. The first argument sets the number of rounds and the optional second argument creates a new circuit file. Longer runs extend the same seeded circuit.

```sh
cargo run --release --locked --manifest-path crates/quantsim_core/Cargo.toml \
  --example sycamore_mps -- 16 patch.json > rounds.csv
```

The patch uses 22 links from the built-in 53-site layout. Its upper row is chip q19–q26 and its lower row is q29–q36. Each round applies one of H, √X, or √Y to every qubit (seed 53), followed by one matching of ideal SYC gates. The four-round cycle covers even horizontal, even vertical, odd horizontal, then odd vertical links, using zero-based patch columns.

Each MPS tensor holds one column of two qubits. A vertical gate acts within one tensor. A horizontal gate factors a pair of tensors using column-pivoted QR. The uncapped run retains all bond directions above numerical roundoff, with a local discarded Frobenius norm at most `max(rows, cols) × f64::EPSILON × norm(pair)`. Its CSV records all seven bond dimensions and the accumulated squared local residuals.

In the recorded 16 rounds the MPS tensors occupied 264.5 KiB at round 5. At round 7 the middle bond reached 256 and the tensors occupied 2.13 MiB. The double-precision dense state uses 1 MiB throughout, and the app’s single-precision state uses 512 KiB. The largest absolute amplitude differences were 5.56 × 10⁻¹⁶ against double precision and 1.50 × 10⁻⁷ against the app. [Growth chart](examples/sycamore_mps/growth.png) · [Measurements](examples/sycamore_mps/rounds.csv) · [Editable circuit](examples/sycamore_mps/patch.json)

Peak MPS heap reached 6.13 MiB during round 9 in a native Apple Silicon release build. The meter counts requested heap bytes for the state and temporary allocations during evolution. Circuit preparation, validation, allocator overhead, and the dense contraction used for comparison are outside that measurement.

#### Capped bonds and fidelity

The cap sweep evolves one MPS per requested cap and an uncapped control on the same gate sequence. After each horizontal gate, a binding cap keeps the largest Schmidt components across that bond and renormalizes the retained state. Later gates act on that approximate state. Each round compares it to all dense amplitudes using squared normalized overlap, $F = |\langle\psi_{\rm dense}|\psi_{\rm MPS}\rangle|^2/(\|\psi_{\rm dense}\|^2\|\psi_{\rm MPS}\|^2)$.

```sh
cargo run --release --locked --manifest-path crates/quantsim_core/Cargo.toml \
  --example sycamore_mps -- --caps 16,32,64 16 > bond_caps.csv
```

Caps 16 and 32 first lose fidelity in round 5, reaching 40.34% and 67.48%. Cap 64 first loses fidelity in round 7, reaching 52.95%. A vertical round applies the same within-column unitaries to both states, so their overlap stays constant. The figures below are for the seed-53 patch and its gate ordering.

| Bond cap | Round-16 fidelity | Round-16 tensor storage | Peak evolution heap over 16 rounds |
|---|---:|---:|---:|
| 16 | 0.77% | 72.5 KiB | 332 KiB |
| 32 | 3.24% | 200.5 KiB | 1.20 MiB |
| 64 | 17.11% | 648.5 KiB | 4.65 MiB |
| Uncapped | 100% | 2.13 MiB | 6.13 MiB |

The double-precision dense reference used 1 MiB for amplitudes and peaked at 1 MiB plus 256 bytes during evolution. The sweep measures each backend separately. [Fidelity and memory chart](examples/sycamore_mps/bond_caps.png) · [Round-by-round data](examples/sycamore_mps/bond_caps.csv)

#### Paired gates and per-cut caps

The native example can apply both horizontal gates on a pair of neighboring column tensors before performing a single factorization. With `--paired 32` every cut uses the same maximum rank. With `--adaptive 32 585` the example first profiles the circuit at cap 32, then replays it from the input, assigning extra capacity to cuts that lost the most weight to the cap. The budget of 585 KiB covers retained complex tensor entries. QR and eigensolver workspace require additional memory.

```sh
cargo run --release --locked --manifest-path crates/quantsim_core/Cargo.toml \
  --example sycamore_mps -- --adaptive 32 585 8
cargo run --release --locked --manifest-path crates/quantsim_core/Cargo.toml \
  --example sycamore_mps -- --cut-caps 4,16,32,128,32,16,4 8
```

The second command reuses the eight-round plan in a single pass. The CSV gives the forward fidelity against all 65,536 dense amplitudes. A different circuit or depth can concentrate loss on different cuts. [Measured fidelity, memory, time and reversal](../../docs/benchmarks/README.md#paired-gates-and-a-tensor-budget)

The optional Cargo feature `mps-ladder` exposes `quantsim_core::mps_ladder::run_echo(cap)` for the fixed 256-wire, eight-round benchmark. It returns the squared all-zero amplitude after paired forward and adjoint evolution. The native desktop experiment uses this entry point for **Run echo**. [Bond-cap selection and measured return](../../docs/benchmarks/README.md#choosing-a-bond-cap)

#### Local qubit and coupler readouts

```sh
cargo run --release --locked --manifest-path crates/quantsim_core/Cargo.toml \
  --example sycamore_mps -- --readouts 16,32,64 16 > local_readouts.csv
```

The CSV pairs each MPS value with the dense value at the input and after every round. It gives the probability of reading 1 on each of the 16 patch wires and three Z-basis measurements for all 22 hardware links, including links idle in that round. The pair measurements are the probability of reading 11, the raw expectation $\langle Z_a Z_b\rangle$, and the connected correlation $C_{ab}=\langle Z_a Z_b\rangle-\langle Z_a\rangle\langle Z_b\rangle$. Values come from summing the full normalized basis probabilities of the 16-qubit verification states. Wire IDs 0–7 are the upper row and 8–15 the lower row.

At round 7, cap 64 predicted 73.12% on wire 7, versus 62.50% dense. At round 16, its mean absolute error was 0.42 percentage points across wires and 0.00681 in connected correlation across links. By then the dense values clustered so closely around 50% and zero that those constant predictions had smaller mean errors of 0.14 percentage points and 0.00268. [Mean and worst error by round](examples/sycamore_mps/local_readouts.png) · [All individual readouts](examples/sycamore_mps/local_readouts.csv)

### Loading Built-in Templates

The current checkout includes a five-qubit plaquette with an explicit physical layout.

```rust
use quantsim_core::circuits;

let plaquette = circuits::load_template("plaquette_z_check").unwrap();
let layout = plaquette.layout.as_ref().unwrap();
assert_eq!(layout.positions.len(), 5);
assert_eq!(layout.links.len(), 4);
```

## Architecture

The library is organized into several key modules:

- **`circuit`**: Quantum circuit representation and manipulation
- **`engine`**: Simulation execution engine
- **`gates`**: Gate definitions and registry system
- **`types`**: Core data types and representations
- **`circuits`**: Built-in circuit templates

## Performance

- Parallel gate application using Rayon
- Efficient state vector representation
- Optimized for both small educational circuits and larger quantum algorithms

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.
