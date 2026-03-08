# quantsim_ui

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-informational?style=flat-square)](../../COPYRIGHT.md)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange?logo=rust)]
[![WASM](https://img.shields.io/badge/Target-WASM-563d7c?logo=webassembly&logoColor=white)]

A WebAssembly-based graphical user interface for quantum circuit simulation, built with `egui`. This application provides an interactive environment for building, simulating, and visualizing quantum circuits.

## Features

- **Visual Circuit Builder**: Drag-and-drop interface for constructing quantum circuits
- **Interactive Simulation**: Real-time quantum state visualization with time scrubbing
- **Bloch Sphere Visualization**: 3D visualization of single-qubit states
- **Built-in Tutorials**: Comprehensive in-app learning materials
- **JSON Import/Export**: Save and share circuits easily
- **Custom Gate Editor**: Define your own quantum gates
- **Template Library**: Pre-built circuits for common quantum algorithms
- **WebAssembly Support**: Runs directly in the browser

## Live Demo

Try the application online: https://benschneider.github.io/quantum_algorithm_simulator/

## Building and Running

### Prerequisites

- Rust 1.75+
- Trunk (for WASM builds)

### Local Development

```bash
# Install trunk if not already installed
cargo install trunk

# Clone the repository
git clone https://github.com/benschneider/quantum_algorithm_simulator.git
cd quantum_algorithm_simulator/crates/quantsim_ui

# Serve locally
trunk serve
```

The application will be available at `http://localhost:8080`.

### Building for Production

```bash
trunk build --release
```

## Usage

### Interface Overview

The application consists of several key panels:

- **Gate Palette** (Left): Available quantum gates organized by category
- **Circuit Grid** (Center): Main canvas for building circuits
- **Results Panel** (Right): Simulation results and state visualization
- **Top Panel**: Application controls and menu

### Basic Workflow

1. **Select Gates**: Click gates in the palette to select them
2. **Build Circuit**: Click on the circuit grid to place gates
3. **Configure**: Set initial states and circuit parameters
4. **Simulate**: Click "Run ▶" to execute the simulation
5. **Analyze**: View results in the results panel and use the time scrubber

### Key Features

#### Circuit Construction
- Visual drag-and-drop gate placement
- Support for multi-qubit gates
- Real-time circuit validation

#### Simulation
- Step-by-step execution with time scrubbing
- Probability and state vector visualization
- Measurement sampling with configurable shots

#### Learning Tools
- Interactive tutorials covering quantum concepts
- Gate reference with matrix representations
- Bloch sphere demonstrations

## Architecture

Built using the Model-View-Update (MVU) architectural pattern:

- **Model**: Centralized `AppState` managing all application data
- **View**: Composable UI components built with `egui`
- **Update**: Message-driven state updates through dedicated handlers

## Dependencies

- `quantsim_core`: Core simulation engine
- `egui`: Immediate mode GUI framework
- `eframe`: WebAssembly application framework
- `serde`: Serialization for circuit persistence

## Contributing

See the main repository for contribution guidelines.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.
