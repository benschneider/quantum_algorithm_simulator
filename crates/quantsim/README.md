# quantsim

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-informational?style=flat-square)](../../COPYRIGHT.md)
[![Crates.io](https://img.shields.io/crates/v/quantsim.svg)](https://crates.io/crates/quantsim)
[![Documentation](https://docs.rs/quantsim/badge.svg)](https://docs.rs/quantsim)

A quantum circuit simulator ecosystem providing both a core simulation library and a web-based graphical user interface.

## Quick Start

Install and run the interactive simulator:

```bash
cargo install quantsim
quantsim
```

Or try the [live web demo](https://benschneider.github.io/quantum_algorithm_simulator/)!

## Architecture

This crate serves as the main entry point for the quantsim ecosystem, which consists of three components:

### 📦 [quantsim](https://crates.io/crates/quantsim) (this crate)
- **Library** that re-exports the quantsim ecosystem
- **Binary entry point** for the offline desktop app
- **Convenient shared crate name** on crates.io

### 🧠 [quantsim_core](https://crates.io/crates/quantsim_core)
- **Core simulation engine** written in pure Rust
- **Quantum gate library** with built-in circuit templates
- **Performance optimized** with parallel execution
- **No UI dependencies** - portable and reusable

### 🎨 [quantsim_ui](https://crates.io/crates/quantsim_ui)
- **WebAssembly-based GUI** built with `egui`
- **Interactive circuit builder** with drag-and-drop gates
- **Real-time visualization** with Bloch spheres
- **Built-in tutorials** and comprehensive documentation

## Installation

### As a Binary Application

```bash
cargo install quantsim
```

This installs the interactive simulator that you can run with:

```bash
quantsim
```

### As a Library Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
quantsim = "0.2.2"
```

## Usage

### Running the Interactive Simulator

```bash
cargo install quantsim
quantsim
```

This launches the interactive simulator where you can:
- Build quantum circuits visually
- Run simulations with real-time results
- Explore built-in algorithm templates
- Learn with interactive tutorials

### Using as a Library

```rust
use quantsim::core::*;

// Create and simulate a quantum circuit
let mut circuit = Circuit::new(2);
circuit.add_gate("H", &[0]).unwrap();
circuit.add_gate("CX", &[0, 1]).unwrap();

let engine = Engine::new();
let result = engine.run(&circuit, None).unwrap();

println!("Final state: {:?}", result.final_state);
```

### WebAssembly UI (Advanced)

When targeting WASM, you can also access the UI components:

```rust
#[cfg(target_arch = "wasm32")]
use quantsim::ui;

// UI components available for custom web applications
```

## Features

- **Visual Circuit Design**: Drag-and-drop quantum gate placement
- **Real-time Simulation**: Instant feedback on circuit execution
- **Educational Tools**: Interactive tutorials and documentation
- **Performance**: Parallel execution for large circuits
- **Extensible**: Custom gates and advanced simulation options
- **Cross-platform**: Runs in browsers via WebAssembly

## Examples

### Bell State Preparation

```rust
use quantsim::core::*;

let mut circuit = Circuit::new(2);
circuit.add_gate("H", &[0])?;      // Hadamard on qubit 0
circuit.add_gate("CX", &[0, 1])?;  // CNOT gate

let result = Engine::new().run(&circuit, None)?;
println!("Entangled state: {:?}", result.final_state);
```

### Loading Built-in Templates

```rust
use quantsim::core::circuits;

let grover_circuit = circuits::load_template("grover")?;
let result = Engine::new().run(&grover_circuit, None)?;
```

## Documentation

- [API Documentation](https://docs.rs/quantsim)
- [Core Library Docs](https://docs.rs/quantsim_core)
- [UI Library Docs](https://docs.rs/quantsim_ui)
- [Interactive Tutorials](https://benschneider.github.io/quantum_algorithm_simulator/)

## Contributing

See the [main repository](https://github.com/benschneider/quantum_algorithm_simulator) for contribution guidelines.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.
