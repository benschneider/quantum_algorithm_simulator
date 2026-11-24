# quantsim_core

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-informational?style=flat-square)](../../COPYRIGHT.md)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange?logo=rust)]

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
quantsim_core = { git = "https://github.com/benschneider/quantum_algorithm_simulator", tag = "v0.3.0" }
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

`circuit.run_stabilizer_prefix(step_count)` accepts per-wire preparations and the built-in H, SqrtZ (S), CX, X, Y, and Z gates. Call `state.probability_one(wire)` for an exact wire probability or `state.sample_wide(shots, seed)` for full-width joint samples, with q0 at bit zero. For registers of up to 128 wires, `state.sample(shots, seed)` returns samples packed into `u128`. See the [50-wire GHZ example](src/core/stabilizer.rs).

The native GHZ50 test measured **28,264 bytes of peak additional live heap** on Apple Silicon after circuit construction, including evolution, all 50 wire probabilities, sampling scratch space, and 1,024 returned samples. To reproduce from the workspace root, run `cargo test --manifest-path crates/quantsim_core/Cargo.toml --test stabilizer_scale -- --nocapture`.

`circuit.run_product_prefix(step_count)` provides the same readout interface for product states. It evolves built-in single-wire gates, SWAP, and eligible CX, CZ and CZPow operations using one pair of complex amplitudes per wire. In the [50-qubit QFT](circuits/qft_50.json), each controlled phase has a partner wire that is still a definite bit, so its exponent can be edited freely. A controlled gate that the backend cannot prove preserves separability returns its column and operation index. This can also happen after arbitrary rotations cancel back to a basis state.

`circuit.run_grouped_prefix(step_count)` keeps a state vector for each group of wires that must evolve together. A group can span nonadjacent wires. A group of `k` wires stores `2^k` complex amplitudes at 16 bytes each. Merges and splits also need temporary buffers. SWAP exchanges wire labels. After a multi-wire gate, the simulator checks the affected group for individual wires whose two conditional amplitude rows are proportional at stored precision. It stores those wires independently again. Rounding can leave a separable wire grouped. `state.group_wires(wire)` reports which physical wires are stored together.

For an individual complex amplitude, call `product.amplitude_wide(&index)`, `grouped.amplitude_wide(&index)`, or construct `let view = stabilizer.amplitude_view()` once and call `view.amplitude_wide(&index)` for each point. Use `quantsim_core::BasisIndex` for indices of any register width. The corresponding `amplitude(index)` methods accept a `u128` index. The product and grouped states retain the gate-defined phase. The stabilizer view fixes global phase by making the lowest occupied index positive real.

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
