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

### Compact Clifford readout (current checkout)

`circuit.run_stabilizer_prefix(step_count)` accepts 1–64 wires with per-wire preparations and the built-in H, SqrtZ (S), CX, X, Y, and Z gates. Call `state.probability_one(wire)` for an exact wire probability or `state.sample(shots, seed)` for joint computational-basis samples packed into `u64`, with q0 at bit zero. See the [50-wire GHZ example](src/core/stabilizer.rs).

The native GHZ50 test measured **13,192 bytes of peak additional live heap** on Apple Silicon after circuit construction, including evolution, all 50 wire probabilities, sampling scratch space, and 1,024 returned samples. To reproduce from the workspace root, run `cargo test --manifest-path crates/quantsim_core/Cargo.toml --test stabilizer_scale -- --nocapture`.

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
