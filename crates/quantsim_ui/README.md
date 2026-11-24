# quantsim_ui

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-informational?style=flat-square)](../../COPYRIGHT.md)
[![Rust](https://img.shields.io/badge/Rust-1.87.0-orange?logo=rust)](rust-toolchain)
[![WASM](https://img.shields.io/badge/Target-WASM-563d7c?logo=webassembly&logoColor=white)]

A quantum circuit editor built with `egui` for the desktop and WebAssembly. Build gates on the timeline, inspect their physical positions in **2D layout**, or follow a wire through successive timesteps in **3D stack**.

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

- Rust 1.87.0 and the `wasm32-unknown-unknown` target for the web build, as specified in the [UI toolchain](rust-toolchain)
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

To update the GitHub Pages build, run `./deploy-web.sh` from the repository root with Trunk 0.21.14, Python 3 and the WebAssembly target installed. When Pages serves `main /docs`, commit the regenerated `docs/` assets with the source. When Pages uses the GitHub Actions source, a push to `main` builds and deploys the site. [Deployment workflow](../../.github/workflows/pages.yml)

## Usage

### Interface Overview

The application consists of several key panels:

- **Gate Palette** (Left): Available quantum gates organized by category
- **Circuit Grid** (Center): Main canvas for building circuits
- **Qubit layout** sits below the grid. Choose **Edit layout** to arrange wires in a line, grid, ring or custom set of sites and links.
- **Results** shows each wire’s probability of reading 1 after the selected step. Choose **Amplitude waveform** to inspect individual amplitudes or **Overview** to see how probability is distributed across the register.
- **Top Panel**: Application controls and menu

### Basic Workflow

1. **Select Gates**: Click gates in the palette to select them
2. **Build Circuit**: Click on the circuit grid to place gates
3. **Configure**: Set initial states and circuit parameters
4. **Simulate**: Click "Run ▶" to execute the simulation
5. Choose **3D stack** and click a site to select its wire and step in **Results**. Drag to rotate the stack. For a five-wire example, load **File → Load Template → Error correction → plaquette z check** and compare q4 at steps 4 and 5.

### Key Features

#### Circuit Construction

- Choose a gate from the palette and click an empty cell to place it. For a controlled-X gate, click the control and then the target in the same column.
- Choose **Insert QFT…** to insert an editable Fourier transform. Select its first wire, width and starting point. Later gates move to the right.
- Use **Layout → Edit layout** to set site positions and physical links. The circuit JSON stores one integer `[x, y]` position per wire and lists each link as a pair of wire indices. You can apply gates between unlinked sites too.
- Use **Sections** beside the timestep slider to label a range of steps. In **JSON**, choose **Copy to Clipboard** to save a copy of the circuit or paste an edited version and choose **Apply Changes**.

#### Simulation

- Step-by-step execution with time scrubbing
- Up to 21 wires, **Joint state** lists the populated basis states above the display threshold. For wider circuits, **Joint samples** summarizes 1,024 shots and **Wire readout** gives each wire’s probability directly from the simulated state.
- Open the estimated peak RAM beside **Run** to set **Warn above** in MiB. The estimate covers simulation buffers. A warning still lets you run the circuit.

<details>
<summary>Memory estimates for three 30-wire circuits</summary>

These estimates were read before running in Chrome on November 23, 2026. The warning threshold in the second case was set to 1 MiB.

| Gates | Estimated peak RAM | Browser check |
| --- | ---: | --- |
| H on q0, then a chain of 29 CNOTs | ≤197 KiB | Inspected at step 0 |
| Controlled quarter-phase gates joining 16 wires | ≤3 MiB | Completed all 16 steps |
| Controlled quarter-phase gates joining all 30 wires | ≤32.1 GiB | Inspected at step 0 |

</details>

In this checkout, **File → Load Template** includes two editable wide circuits, **phase echo 512** and **grouped blocks 1024**. **MPS ladder echo 256** opens a separate bond-cap experiment. Its browser view shows the recorded return curve, and its desktop **Run echo** button executes the selected cap. [Wide circuit walkthroughs](assets/docs/tutorials/template_guide.md#try-a-wider-circuit)

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
