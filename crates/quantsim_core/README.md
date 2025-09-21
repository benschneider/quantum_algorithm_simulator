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
quantsim_core = "0.1.0"
```

### Basic Example

```rust
use quantsim_core::{Circuit, Engine, GateRegistry};

let mut registry = GateRegistry::new();
registry.register_standard_gates();

// Create a Bell state circuit
let mut circuit = Circuit::new(2);
circuit.add_gate("H", &[0]).unwrap();
circuit.add_gate("CX", &[0, 1]).unwrap();

// Simulate
let mut engine = Engine::new();
let result = engine.run(&circuit, None).unwrap();

println!("Final state: {:?}", result.final_state);
```

### Loading Built-in Templates

```rust
use quantsim_core::circuits;

let bell_circuit = circuits::load_template("bell").unwrap();
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