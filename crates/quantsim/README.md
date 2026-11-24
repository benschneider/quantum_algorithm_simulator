# quantsim

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-informational?style=flat-square)](../../COPYRIGHT.md)
[![Crates.io](https://img.shields.io/crates/v/quantsim.svg)](https://crates.io/crates/quantsim)
[![Documentation](https://docs.rs/quantsim/badge.svg)](https://docs.rs/quantsim)

A quantum circuit simulator ecosystem providing both a core simulation library and a web-based graphical user interface.

## Quick Start

Install and run the interactive simulator:

```bash
cargo install --git https://github.com/benschneider/quantum_algorithm_simulator --tag v0.4.0 --locked quantsim
quantsim
```

To try the current checkout’s wide circuits and run the 256-qubit MPS ladder locally, use `cargo run --release --locked --manifest-path crates/quantsim/Cargo.toml` from the repository root. [Wide circuit walkthroughs](../quantsim_ui/assets/docs/tutorials/template_guide.md#try-a-wider-circuit) · [Live web demo](https://benschneider.github.io/quantum_algorithm_simulator/)

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
cargo install --git https://github.com/benschneider/quantum_algorithm_simulator --tag v0.4.0 --locked quantsim
```

This installs the interactive simulator that you can run with:

```bash
quantsim
```

### As a Library Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
quantsim = { git = "https://github.com/benschneider/quantum_algorithm_simulator", tag = "v0.4.0" }
```

## Usage

### Running the Interactive Simulator

```bash
cargo install --git https://github.com/benschneider/quantum_algorithm_simulator --tag v0.4.0 --locked quantsim
quantsim
```

This launches the interactive simulator where you can:
- Build quantum circuits visually
- Run simulations with real-time results
- Explore built-in algorithm templates
- Learn with interactive tutorials

### Using as a Library

```rust
use quantsim::core::{circuit::RunOptions, circuits};

let circuit = circuits::load_template("bell").unwrap();
let first = circuit.run_prefix(1, &RunOptions::default());
assert!(first.diagnostics.is_empty());
assert!((first.final_probabilities[0b01] - 0.5).abs() < 1e-6);
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
use quantsim::core::{circuit::RunOptions, CircuitBuilder};

let mut builder = CircuitBuilder::new(2);
builder.h(0).next_step().cx(0, 1);
let result = builder.build().run(&RunOptions::default());
assert!(result.diagnostics.is_empty());
assert!((result.final_probabilities[0b11] - 0.5).abs() < 1e-6);
```

### Loading Built-in Templates

```rust
use quantsim::core::{circuit::RunOptions, circuits};

let grover = circuits::load_template("grover").unwrap();
let result = grover.run(&RunOptions::default());
assert!(result.diagnostics.is_empty());
assert!((result.final_probabilities[0b11] - 1.0).abs() < 1e-6);
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
