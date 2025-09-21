# Quantum Gate Reference

This document provides a simple explanation of each quantum gate available in the circuit builder.

---

## H: Hadamard Gate

-   **Description:** The Hadamard gate is one of the most important gates in quantum computing. It takes a qubit that is in a definite state (either `|0>` or `|1>`) and puts it into a perfect **superposition**. After a Hadamard gate, the qubit has a 50% chance of being measured as `0` and a 50% chance of being measured as `1`.
-   **Classical Analogy:** It's like a "quantum coin flip" that leaves the coin spinning perfectly in the air before it lands.
-   **How to Use:**
    1.  Select the **H** gate from the UI palette.
    2.  Click on the desired qubit wire at the desired time step (column) to place the gate.

---

## X: Pauli-X Gate

-   **Description:** The Pauli-X gate is the quantum equivalent of the classical NOT gate. It flips the state of a qubit. If the qubit is `|0>`, the X gate flips it to `|1>`. If it is `|1>`, it flips it to `|0>`.
-   **Classical Analogy:** This is a **NOT gate**.
-   **How to Use:**
    1.  Select the **X** gate from the UI palette.
    2.  Click on the desired qubit wire at the desired time step (column) to place the gate.

---

## Y: Pauli-Y Gate

-   **Description:** The Pauli-Y gate is another "bit-flip" gate, similar to the X gate. It flips `|0>` to `|1>` and `|1>` to `|0>`, but it also adds a phase shift. This distinction is important in more complex algorithms.
-   **Classical Analogy:** A NOT gate with a complex twist.
-   **How to Use:**
    1.  Select the **Y** gate from the UI palette.
    2.  Click on the desired qubit wire at the desired time step (column) to place the gate.

---

## Z: Pauli-Z Gate

-   **Description:** The Pauli-Z gate is a "phase-flip" gate. It leaves the `|0>` state unchanged but flips the phase of the `|1>` state. This is not a bit-flip, so it won't change the measurement probabilities, but it is crucial for controlling interference in quantum algorithms.
-   **Classical Analogy:** There is no direct classical analogy. It's a purely quantum effect.
-   **How to Use:**
    1.  Select the **Z** gate from the UI palette.
    2.  Click on the desired qubit wire at the desired time step (column) to place the gate.

---

## SQRT_X: Square Root of X Gate (√X)

-   **Description:** The Square Root of X gate is a gate that, when applied twice, is equivalent to a Pauli-X gate. It's a fundamental component for creating universal gate sets.
-   **Classical Analogy:** There is no direct classical analogy.
-   **How to Use:**
    1.  Select the **SQRT_X** gate from the UI palette.
    2.  Click on the desired qubit wire at the desired time step (column) to place the gate.

---

## SQRT_Y: Square Root of Y Gate (√Y)

-   **Description:** The Square Root of Y gate is a gate that, when applied twice, is equivalent to a Pauli-Y gate.
-   **Classical Analogy:** There is no direct classical analogy.
-   **How to Use:**
    1.  Select the **SQRT_Y** gate from the UI palette.
    2.  Click on the desired qubit wire at the desired time step (column) to place the gate.

---

## SQRT_Z: Square Root of Z Gate (√Z / S Gate)

-   **Description:** The Square Root of Z gate, also known as the S gate, is a gate that, when applied twice, is equivalent to a Pauli-Z gate. It applies a 90-degree rotation around the Z-axis.
-   **Classical Analogy:** There is no direct classical analogy.
-   **How to Use:**
    1.  Select the **SQRT_Z** gate from the UI palette.
    2.  Click on the desired qubit wire at the desired time step (column) to place the gate.

---

## CX / CNOT: Controlled-X (CNOT) Gate

-   **Description:** The CNOT gate is a two-qubit gate that is essential for creating entanglement. It has two parts: a **control** qubit and a **target** qubit. The gate works as follows:
    -   If the control qubit is `|0>`, it does nothing to the target qubit.
    -   If the control qubit is `|1>`, it applies an X gate (a bit-flip) to the target qubit.
-   **Classical Analogy:** This is a **Controlled NOT** or **XOR gate**. The target bit is flipped if and only if the control bit is true.
-   **How to Use:**
    1.  Select the **CX / CNOT** gate from the UI palette.
    2.  Click on the desired **control** qubit wire. A small circle will appear.
    3.  Click on the desired **target** qubit wire in the same column. A target symbol (⊕) will appear, connected by a line to the control.

---

## SWAP: SWAP Gate

-   **Description:** The SWAP gate is a two-qubit gate that swaps the states of the two qubits.
-   **Classical Analogy:** This is a **SWAP gate**.
-   **How to Use:**
    1.  Select the **SWAP** gate from the UI palette.
    2.  Click on the first qubit wire. A small cross will appear.
    3.  Click on the second qubit wire in the same column. A second cross will appear, connected by a line to the first.

---

## CZ: Controlled-Z Gate

-   **Description:** The Controlled-Z gate applies a Z gate to the target qubit if the control qubit is in the `|1>` state.
-   **Classical Analogy:** There is no direct classical analogy.
-   **How to Use:**
    1.  Select the **CZ** gate from the UI palette.
    2.  Click on the desired **control** qubit wire. A small circle will appear.
    3.  Click on the desired **target** qubit wire in the same column. A Z gate symbol will appear on the target qubit, connected by a line to the control.

---

## CY: Controlled-Y Gate

-   **Description:** The Controlled-Y gate applies a Y gate to the target qubit if the control qubit is in the `|1>` state.
-   **Classical Analogy:** There is no direct classical analogy.
-   **How to Use:**
    1.  Select the **CY** gate from the UI palette.
    2.  Click on the desired **control** qubit wire. A small circle will appear.
    3.  Click on the desired **target** qubit wire in the same column. A Y gate symbol will appear on the target qubit, connected by a line to the control.

---

## CCNOT: Controlled-Controlled-NOT (Toffoli) Gate

-   **Description:** The Toffoli gate is a three-qubit gate that flips the state of the target qubit (qubit 2) if and only if both control qubits (qubits 0 and 1) are in the `|1>` state.
-   **Classical Analogy:** This is a **Controlled-Controlled NOT** or **AND-NOT gate**.
-   **How to Use:**
    1.  Select the **CCNOT** gate from the UI palette.
    2.  Click on the desired **first control** qubit wire. A small circle will appear.
    3.  Click on the desired **second control** qubit wire in the same column. Another small circle will appear.
    4.  Click on the desired **target** qubit wire in the same column. A target symbol (⊕) will appear, connected by lines to both controls.

---

## CCZ: Controlled-Controlled-Z Gate

-   **Description:** The Controlled-Controlled-Z gate applies a Z gate to the target qubit (qubit 2) if and only if both control qubits (qubits 0 and 1) are in the `|1>` state. This gate flips the sign of the `|111>` state.
-   **Classical Analogy:** There is no direct classical analogy.
-   **How to Use:**
    1.  Select the **CCZ** gate from the UI palette.
    2.  Click on the desired **first control** qubit wire. A small circle will appear.
    3.  Click on the desired **second control** qubit wire in the same column. Another small circle will appear.
    4.  Click on the desired **target** qubit wire in the same column. A Z gate symbol will appear on the target qubit, connected by lines to both controls.

---

## ORACLE: Grover Oracle Gate

-   **Description:** This is a custom oracle gate specifically designed for Grover's algorithm within this simulation. It marks the `|100>` state by flipping its phase.
-   **Classical Analogy:** None. This is a problem-specific quantum operation.
-   **How to Use:**
    1.  Select the **ORACLE** gate from the UI palette.
    2.  Click on the desired qubit wires (usually three) at the desired time step (column) to place the gate. The gate will span the three qubits it operates on.

---

## Parametric Rotation Gates (Rx, Ry, Rz)

These gates allow for continuous rotations around the axes of the Bloch sphere by a specified angle.

### Rz: Rotation around Z-axis

-   **Description:** Rotates the qubit state around the Z-axis of the Bloch sphere by a specified angle `phi`. This is a phase shift gate.
-   **Implementation:** This gate directly calculates the rotation matrix for the specified angle.
-   **How to Use:**
    1.  Select the **Rz** gate from the UI palette.
    2.  Click on the desired qubit wire to place the gate.
    3.  The **Gate Editor** will open, allowing you to enter the desired rotation angle `phi` in radians.

### Rx: Rotation around X-axis

-   **Description:** Rotates the qubit state around the X-axis of the Bloch sphere by a specified angle `phi`.
-   **Implementation:** This gate directly calculates the rotation matrix for the specified angle.
-   **How to Use:**
    1.  Select the **Rx** gate from the UI palette.
    2.  Click on the desired qubit wire to place the gate.
    3.  The **Gate Editor** will open, allowing you to enter the desired rotation angle `phi` in radians.

### Ry: Rotation around Y-axis

-   **Description:** Rotates the qubit state around the Y-axis of the Bloch sphere by a specified angle `phi`.
-   **Implementation:** This gate directly calculates the rotation matrix for the specified angle.
-   **How to Use:**
    1.  Select the **Ry** gate from the UI palette.
    2.  Click on the desired qubit wire to place the gate.
    3.  The **Gate Editor** will open, allowing you to enter the desired rotation angle `phi` in radians.

---

## CRz: Controlled Rz Gate

-   **Description:** The CRz gate is a two-qubit gate that applies a phase rotation (`Rz(phi)`) to the target qubit if and only if the control qubit is in the state `|1>`.
-   **Classical Analogy:** This is a **Controlled Phase Rotation**.
-   **How to Use:**
    1.  Select the **CRz** gate from the UI palette.
    2.  Click on the desired **control** qubit wire. A small circle will appear.
    3.  Click on the desired **target** qubit wire in the same column. An Rz gate will appear on the target qubit, connected by a line to the control.
    4.  The **Gate Editor** will open, allowing you to enter the desired rotation angle `phi` in radians.

---

## CXPow: Controlled X^t Gate

-   **Description:** Applies X raised to the power of `t` to the target qubit if the control qubit is in the `|1>` state. This allows for continuous controlled rotations around the X-axis.
-   **How to Use:**
    1.  Select the **CXPow** gate from the UI palette.
    2.  Click on the desired **control** qubit wire.
    3.  Click on the desired **target** qubit wire in the same column.
    4.  The **Gate Editor** will open, allowing you to enter the desired exponent `t`.

---

## CYPow: Controlled Y^t Gate

-   **Description:** Applies Y raised to the power of `t` to the target qubit if the control qubit is in the `|1>` state. This allows for continuous controlled rotations around the Y-axis.
-   **How to Use:**
    1.  Select the **CYPow** gate from the UI palette.
    2.  Click on the desired **control** qubit wire.
    3.  Click on the desired **target** qubit wire in the same column.
    4.  The **Gate Editor** will open, allowing you to enter the desired exponent `t`.

---

## CZPow: Controlled Z^t Gate

-   **Description:** Applies Z raised to the power of `t` to the target qubit if the control qubit is in the `|1>` state. This allows for continuous controlled rotations around the Z-axis.
-   **How to Use:**
    1.  Select the **CZPow** gate from the UI palette.
    2.  Click on the desired **control** qubit wire.
    3.  Click on the desired **target** qubit wire in the same column.
    4.  The **Gate Editor** will open, allowing you to enter the desired exponent `t`.

---

## EigenGates (X^t, Y^t, Z^t, CCZ^t)

These gates are implemented using the `EigenGate` trait, which allows them to be raised to a continuous power `t`. They include a global phase factor.

### X_pow: X^t Gate

-   **Description:** A rotation around the X-axis by an angle of `t*pi`. This gate includes a global phase factor of $e^{i \pi t / 2}$.
-   **How to Use:**
    1.  Select the **X_pow** gate from the UI palette.
    2.  Click on the desired qubit wire to place the gate.
    3.  The **Gate Editor** will open, allowing you to enter the desired exponent `t`.

### Y_pow: Y^t Gate

-   **Description:** A rotation around the Y-axis by an angle of `t*pi`. This gate includes a global phase factor of $e^{i \pi t / 2}$.
-   **How to Use:**
    1.  Select the **Y_pow** gate from the UI palette.
    2.  Click on the desired qubit wire to place the gate.
    3.  The **Gate Editor** will open, allowing you to enter the desired exponent `t`.

### Z_pow: Z^t Gate

-   **Description:** A rotation around the Z-axis by an angle of `t*pi`. This gate includes a global phase factor.
-   **How to Use:**
    1.  Select the **Z_pow** gate from the UI palette.
    2.  Click on the desired qubit wire to place the gate.
    3.  The **Gate Editor** will open, allowing you to enter the desired exponent `t`.

### CCZ_pow: CCZ^t Gate

-   **Description:** The Controlled-Controlled-Z gate raised to the power of `t`.
-   **How to Use:**
    1.  Select the **CCZ_pow** gate from the UI palette.
    2.  Click on the desired **first control** qubit wire.
    3.  Click on the desired **second control** qubit wire in the same column.
    4.  Click on the desired **target** qubit wire in the same column.
    5.  The **Gate Editor** will open, allowing you to enter the desired exponent `t`.

---

## Custom Gates

These are placeholder gates that can be defined by the user within the application.

### C-1Q: Custom 1-Qubit Gate

-   **Description:** A user-defined custom 1-qubit gate. Its matrix can be configured in the application.
-   **How to Use:**
    1.  Select the **C-1Q** gate from the UI palette.
    2.  Click on the desired qubit wire to place the gate.
    3.  Use the application's interface to define its unitary matrix.

### C-2Q: Custom 2-Qubit Gate

-   **Description:** A user-defined custom 2-qubit gate. Its matrix can be configured in the application.
-   **How to Use:**
    1.  Select the **C-2Q** gate from the UI palette.
    2.  Click on the desired qubit wires to place the gate.
    3.  Use the application's interface to define its unitary matrix.

### C-3Q: Custom 3-Qubit Gate

-   **Description:** A user-defined custom 3-qubit gate. Its matrix can be configured in the application.
-   **How to Use:**
    1.  Select the **C-3Q** gate from the UI palette.
    2.  Click on the desired qubit wires to place the gate.
    3.  Use the application's interface to define its unitary matrix.