# Introduction to `qcsim` GUI

`qcsim` is a powerful quantum circuit simulator that comes with an intuitive graphical user interface (GUI) built using the `egui` framework. This GUI application, `qcsim-egui`, allows you to easily design, simulate, and visualize quantum circuits without needing to write any code. It's an ideal tool for both learning the fundamentals of quantum computing and for more advanced research and experimentation.

The `qcsim-egui` application is designed with a clear, user-friendly layout, making complex quantum concepts accessible. It follows a Model-View-Update (MVU) architectural pattern, ensuring a predictable and robust interaction experience.

## Navigating the `qcsim-egui` Interface

The `qcsim-egui` application is organized into several interactive panels, each serving a specific purpose in your quantum circuit design and simulation workflow:

*   **Top Panel**: Located at the top of the application window, this panel contains the main menu bar. Here you'll find options for loading predefined circuit templates and application-level controls like changing the number of qubits and timesteps for your circuit.
    *   *Figure: Top Panel Overview*
*   **Gate Palette (Left Panel)**: On the left-hand side, the "Gates" panel provides a comprehensive palette of quantum gates. These are organized into categories for easy selection. Simply click on a gate to select it for placement on your circuit.
    *   *Figure: Gate Palette*
*   **Circuit Grid (Central Panel)**: This is the largest and most interactive area of the application, where you visually construct your quantum circuit. The grid represents qubits as rows and discrete time steps as columns.
    *   Click an empty cell to place the currently selected gate.
    *   Click an existing gate to open the gate editor for detailed modifications.
    *   Right-click a gate to remove it from the circuit.
    *   *Figure: Circuit Grid and Gate Placement*
*   **Results Panel (Right Panel)**: After running a simulation, the results will be displayed in this panel. You can view the raw state vector (amplitudes of non-zero basis states) and a clear bar chart visualizing the measurement probabilities for each basis state.
    *   *Figure: Simulation Results Panel*

## Your First Quantum Circuit: Building and Simulating

Let's walk through the basic steps to create and simulate a quantum circuit using the `qcsim-egui` application.

### Defining a Quantum Circuit

You have two primary methods to define a quantum circuit in `qcsim`:

#### Method 1: Using the Gate Palette and Circuit Grid (Recommended for Beginners)

This visual method allows for intuitive circuit construction:

1.  **Select a Gate**: In the **Gate Palette** (left panel), click on a desired quantum gate (e.g., the Hadamard gate `H` or Pauli-X gate `X`). The selected gate will be highlighted.
2.  **Place the Gate**: Move your cursor over the **Circuit Grid** (central panel). Click on an empty cell where you want to place the gate. Remember: rows represent qubits, and columns represent time steps. For multi-qubit gates like CNOT, ensure you click on the appropriate qubit lines for both control and target.
    *   *Figure: Step-by-step Gate Placement*
3.  **Continue Building**: Repeat the process to add more gates and build your circuit. The circuit grid updates in real-time.

#### Method 2: Using the JSON Editor (For Advanced Users or Loading Existing Circuits)

`qcsim` circuits can also be represented and edited directly as JSON. This is useful for loading pre-defined circuits or for users familiar with the circuit's underlying data structure:

1.  **Switch to JSON View**: In the **Central Panel**, locate and click the "JSON" tab. This will switch the view from the visual circuit grid to a text editor showing the circuit's JSON representation.
2.  **Edit JSON**: You can directly type or paste your circuit definition in JSON format here.
3.  **Apply Changes**: After editing, click the "Apply Changes" button within the JSON editor. The visual circuit grid will update to reflect your JSON changes. Conversely, changes in the visual editor will update the JSON displayed here.
4.  **Saving/Loading Circuits via JSON**: To "save" a circuit, copy the JSON text from the editor (using the "Copy to Clipboard" button or standard copy shortcuts) and paste it into a text file. To "load" a circuit, paste your JSON into this editor and click "Apply Changes".
    *   *Figure: JSON Editor Interface and Copy/Paste Buttons*

### Simulating Your Circuit

Once your circuit is defined, simulating it is straightforward:

1.  **Run Simulation**: In the **Top Panel**, click the "Run ▶" button. This will execute the quantum circuit simulation.
2.  **View Results**: After the simulation completes, the **Results Panel** (right panel) will automatically display the outcomes. You'll see the state vector and measurement probabilities, giving you insights into the quantum state.
3.  **Time Scrubber**: The "Timestep" slider, usually located near the "Run" button, allows you to "scrub" through the simulation, observing the evolution of the quantum state at each step of your circuit. This is an excellent tool for understanding how gates transform qubits over time.
    *   *Figure: Time Scrubber in Action*

## Accessing Pre-built Circuits and Advanced Features

`qcsim-egui` makes it easy to manage your quantum circuits and explore advanced functionalities:

*   **Load Template Circuits**: From the "File" menu in the **Top Panel**, select "Load Template" to choose from a list of pre-built example circuits (like the Bell state or Grover's algorithm) that you can load and experiment with instantly.
*   **Initial State Editor**: You can set a custom initial quantum state for your qubits, moving beyond the default all-zero state. Access this via the "Settings" menu -> "Set Initial State...".
    *   *Figure: Initial State Editor*
*   **Gate Editor with Bloch Spheres**: To understand the effect of individual gates, especially 1-qubit gates, use the "Gate Editor" (accessible from the "View" menu). For single-qubit gates, you'll see a dynamic Bloch Sphere visualization, showing how the gate transforms the qubit's state.
    *   *Figure: Gate Editor with Bloch Sphere*

This introduction should get you started with `qcsim-egui`. Explore the interface, experiment with different gates, and delve into the fascinating world of quantum computing!