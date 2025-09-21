# Quantum Gate Reference

This document provides a simple explanation of each quantum gate available in the circuit builder.

---
---

## A Note on Endianness

**IMPORTANT:** The simulation engine and all state vector outputs use a **little-endian** convention. This means the first qubit (q0) is the least significant bit (LSB). However, for consistency with common quantum computing literature, the gate matrices in this document are presented in a **big-endian** format, where the first qubit is the most significant bit (MSB). The simulator automatically converts these matrices to the correct little-endian format before applying them to the state vector.

## H: Hadamard Gate

-   **Description:** The Hadamard gate is one of the most important gates in quantum computing. It takes a qubit that is in a definite state (either `|0>` or `|1>`) and puts it into a perfect **superposition**. After a Hadamard gate, the qubit has a 50% chance of being measured as `0` and a 50% chance of being measured as `1`.
-   **Classical Analogy:** It's like a "quantum coin flip" that leaves the coin spinning perfectly in the air before it lands.
-   **Matrix:**
    ```
    1/sqrt(2) * [[1,  1],
                 [1, -1]]
    ```
-   **How to Use:**
    1.  Select the **H** gate from the UI palette.
    2.  Click on the desired qubit wire at the desired time step (column) to place the gate.

---

## X: Pauli-X Gate

-   **Description:** The Pauli-X gate is the quantum equivalent of the classical NOT gate. It flips the state of a qubit. If the qubit is `|0>`, the X gate flips it to `|1>`. If it is `|1>`, it flips it to `|0>`.
-   **Classical Analogy:** This is a **NOT gate**.
-   **Matrix:**
    ```
    [[0, 1],
     [1, 0]]
    ```
-   **How to Use:**
    1.  Select the **X** gate from the UI palette.
    2.  Click on the desired qubit wire at the desired time step (column) to place the gate.

---

## Y: Pauli-Y Gate

-   **Description:** The Pauli-Y gate is another "bit-flip" gate, similar to the X gate. It flips `|0>` to `|1>` and `|1>` to `|0>`, but it also adds a phase shift. This distinction is important in more complex algorithms.
-   **Classical Analogy:** A NOT gate with a complex twist.
-   **Matrix:**
    ```
    [[0, -i],
     [i,  0]]
    ```
-   **How to Use:**
    1.  Select the **Y** gate from the UI palette.
    2.  Click on the desired qubit wire at the desired time step (column) to place the gate.

---

## Z: Pauli-Z Gate

-   **Description:** The Pauli-Z gate is a "phase-flip" gate. It leaves the `|0>` state unchanged but flips the phase of the `|1>` state. This is not a bit-flip, so it won't change the measurement probabilities, but it is crucial for controlling interference in quantum algorithms.
-   **Classical Analogy:** There is no direct classical analogy. It's a purely quantum effect.
-   **Matrix:**
    ```
    [[1,  0],
     [0, -1]]
    ```
-   **How to Use:**
    1.  Select the **Z** gate from the UI palette.
    2.  Click on the desired qubit wire at the desired time step (column) to place the gate.

---

## SQRT_X: Square Root of X Gate (√X)

-   **Description:** The Square Root of X gate is a gate that, when applied twice, is equivalent to a Pauli-X gate. It's a fundamental component for creating universal gate sets.
-   **Classical Analogy:** There is no direct classical analogy.
-   **Matrix:**
    ```
    [[0.5+0.5i, 0.5-0.5i],
     [0.5-0.5i, 0.5+0.5i]]
    ```
-   **How to Use:**
    1.  Select the **SQRT_X** gate from the UI palette.
    2.  Click on the desired qubit wire at the desired time step (column) to place the gate.

---

## SQRT_Y: Square Root of Y Gate (√Y)

-   **Description:** The Square Root of Y gate is a gate that, when applied twice, is equivalent to a Pauli-Y gate.
-   **Classical Analogy:** There is no direct classical analogy.
-   **Matrix:**
    ```
    [[0.5+0.5i, -0.5-0.5i],
     [0.5+0.5i,  0.5+0.5i]]
    ```
-   **How to Use:**
    1.  Select the **SQRT_Y** gate from the UI palette.
    2.  Click on the desired qubit wire at the desired time step (column) to place the gate.

---

## SQRT_Z: Square Root of Z Gate (√Z / S Gate)

-   **Description:** The Square Root of Z gate, also known as the S gate, is a gate that, when applied twice, is equivalent to a Pauli-Z gate. It applies a 90-degree rotation around the Z-axis.
-   **Classical Analogy:** There is no direct classical analogy.
-   **Matrix:**
    ```
    [[1, 0],
     [0, i]]
    ```
-   **How to Use:**
    1.  Select the **SQRT_Z** gate from the UI palette.
    2.  Click on the desired qubit wire at the desired time step (column) to place the gate.

---

## CX / CNOT: Controlled-X (CNOT) Gate

-   **Description:** The CNOT gate is a two-qubit gate that is essential for creating entanglement. It has two parts: a **control** qubit and a **target** qubit. The gate works as follows:
    -   If the control qubit is `|0>`, it does nothing to the target qubit.
    -   If the control qubit is `|1>`, it applies an X gate (a bit-flip) to the target qubit.
-   **Classical Analogy:** This is a **Controlled NOT** or **XOR gate**. The target bit is flipped if and only if the control bit is true.
-   **Matrix (control on qubit 0, target on qubit 1):**
    ```
    [[1, 0, 0, 0],
     [0, 1, 0, 0],
     [0, 0, 0, 1],
     [0, 0, 1, 0]]
    ```
-   **How to Use:**
    1.  Select the **CX / CNOT** gate from the UI palette.
    2.  Click on the desired **control** qubit wire. A small circle will appear.
    3.  Click on the desired **target** qubit wire in the same column. A target symbol (⊕) will appear, connected by a line to the control.

---

## SWAP: SWAP Gate

-   **Description:** The SWAP gate is a two-qubit gate that swaps the states of the two qubits.
-   **Classical Analogy:** This is a **SWAP gate**.
-   **Matrix:**
    ```
    [[1, 0, 0, 0],
     [0, 0, 1, 0],
     [0, 1, 0, 0],
     [0, 0, 0, 1]]
    ```
-   **How to Use:**
    1.  Select the **SWAP** gate from the UI palette.
    2.  Click on the first qubit wire. A small cross will appear.
    3.  Click on the second qubit wire in the same column. A second cross will appear, connected by a line to the first.

---

## CZ: Controlled-Z Gate

-   **Description:** The Controlled-Z gate applies a Z gate to the target qubit if the control qubit is in the `|1>` state.
-   **Classical Analogy:** There is no direct classical analogy.
-   **Matrix (control on qubit 0, target on qubit 1):**
    ```
    [[1, 0, 0,  0],
     [0, 1, 0,  0],
     [0, 0, 1,  0],
     [0, 0, 0, -1]]
    ```
-   **How to Use:**
    1.  Select the **CZ** gate from the UI palette.
    2.  Click on the desired **control** qubit wire. A small circle will appear.
    3.  Click on the desired **target** qubit wire in the same column. A Z gate symbol will appear on the target qubit, connected by a line to the control.

---

## CY: Controlled-Y Gate

-   **Description:** The Controlled-Y gate applies a Y gate to the target qubit if the control qubit is in the `|1>` state.
-   **Classical Analogy:** There is no direct classical analogy.
-   **Matrix (control on qubit 0, target on qubit 1):**
    ```
    [[1, 0, 0,  0],
     [0, 1, 0,  0],
     [0, 0, 0, -i],
     [0, 0, i,  0]]
    ```
-   **How to Use:**
    1.  Select the **CY** gate from the UI palette.
    2.  Click on the desired **control** qubit wire. A small circle will appear.
    3.  Click on the desired **target** qubit wire in the same column. A Y gate symbol will appear on the target qubit, connected by a line to the control.

---

## CCNOT: Controlled-Controlled-NOT (Toffoli) Gate

-   **Description:** The Toffoli gate is a three-qubit gate that flips the state of the target qubit (qubit 2) if and only if both control qubits (qubits 0 and 1) are in the `|1>` state.
-   **Classical Analogy:** This is a **Controlled-Controlled NOT** or **AND-NOT gate**.
-   **Matrix (controls on qubits 0 & 1, target on qubit 2):**
    ```
    [[1,0,0,0,0,0,0,0],
     [0,1,0,0,0,0,0,0],
     [0,0,1,0,0,0,0,0],
     [0,0,0,1,0,0,0,0],
     [0,0,0,0,1,0,0,0],
     [0,0,0,0,0,1,0,0],
     [0,0,0,0,0,0,0,1],
     [0,0,0,0,0,0,1,0]]
    ```
-   **How to Use:**
    1.  Select the **CCNOT** gate from the UI palette.
    2.  Click on the desired **first control** qubit wire. A small circle will appear.
    3.  Click on the desired **second control** qubit wire in the same column. Another small circle will appear.
    4.  Click on the desired **target** qubit wire in the same column. A target symbol (⊕) will appear, connected by lines to both controls.

---

## CCZ: Controlled-Controlled-Z Gate

-   **Description:** The Controlled-Controlled-Z gate applies a Z gate to the target qubit (qubit 2) if and only if both control qubits (qubits 0 and 1) are in the `|1>` state. This gate flips the sign of the `|111>` state.
-   **Classical Analogy:** There is no direct classical analogy.
-   **Matrix (controls on qubits 0 & 1, target on qubit 2):**
    ```
    diag(1, 1, 1, 1, 1, 1, 1, -1)
    ```
-   **How to Use:**
    1.  Select the **CCZ** gate from the UI palette.
    2.  Click on the desired **first control** qubit wire. A small circle will appear.
    3.  Click on the desired **second control** qubit wire in the same column. Another small circle will appear.
    4.  Click on the desired **target** qubit wire in the same column. A Z gate symbol will appear on the target qubit, connected by lines to both controls.

---


---

## ORACLE: Implementing Oracles

-   **Description:** An Oracle is a "black box" operation that is fundamental to certain quantum algorithms, like Grover's search. It marks one or more specific quantum states by flipping their phase (multiplying their amplitude by -1), without revealing which state was marked. Oracles are problem-specific and are not fixed gates.
-   **How to Implement:** You can create your own oracle using a **Custom Gate** (`C-1Q`, `C-2Q`, or `C-3Q`).
-   **Example: Grover Oracle for 3 Qubits:** To create an oracle that marks the `|101>` state (binary for 5), you would use a 3-qubit custom gate (`C-3Q`) and provide an 8x8 identity matrix, but with the element at `(5,5)` changed from `1` to `-1`.
-   **Matrix for marking `|101>`:**
    ```
    diag(1, 1, 1, 1, 1, -1, 1, 1)
    ```

---

## Parametric Rotation Gates (Rx, Ry, Rz)

These gates allow for continuous rotations around the axes of the Bloch sphere by a specified angle.

### Rz: Rotation around Z-axis

-   **Description:** Rotates the qubit state around the Z-axis of the Bloch sphere by a specified angle `phi`. This is a phase shift gate.
-   **Matrix:**
    ```
    [[1, 0],
     [0, e^(i*phi)]]
    ```
-   **How to Use:**
    1.  Select the **Rz** gate from the UI palette.
    2.  Click on the desired qubit wire to place the gate.
    3.  The **Gate Editor** will open, allowing you to enter the desired rotation angle `phi` in radians.

### Rx: Rotation around X-axis

-   **Description:** Rotates the qubit state around the X-axis of the Bloch sphere by a specified angle `phi`.
-   **Matrix:**
    ```
    [[cos(phi/2),    -i*sin(phi/2)],
     [-i*sin(phi/2), cos(phi/2)]]
    ```
-   **How to Use:**
    1.  Select the **Rx** gate from the UI palette.
    2.  Click on the desired qubit wire to place the gate.
    3.  The **Gate Editor** will open, allowing you to enter the desired rotation angle `phi` in radians.

### Ry: Rotation around Y-axis

-   **Description:** Rotates the qubit state around the Y-axis of the Bloch sphere by a specified angle `phi`.
-   **Matrix:**
    ```
    [[cos(phi/2), -sin(phi/2)],
     [sin(phi/2),  cos(phi/2)]]
    ```
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

### XPow: X^t Gate

-   **Description:** A rotation around the X-axis by an angle of `t*pi`. This gate includes a global phase factor of $e^{i \pi t / 2}$.
-   **How to Use:**
    1.  Select the **XPow** gate from the UI palette.
    2.  Click on the desired qubit wire to place the gate.
    3.  The **Gate Editor** will open, allowing you to enter the desired exponent `t`.

### YPow: Y^t Gate

-   **Description:** A rotation around the Y-axis by an angle of `t*pi`. This gate includes a global phase factor of $e^{i \pi t / 2}$.
-   **How to Use:**
    1.  Select the **YPow** gate from the UI palette.
    2.  Click on the desired qubit wire to place the gate.
    3.  The **Gate Editor** will open, allowing you to enter the desired exponent `t`.

### ZPow: Z^t Gate

-   **Description:** A rotation around the Z-axis by an angle of `t*pi`. This gate includes a global phase factor.
-   **How to Use:**
    1.  Select the **ZPow** gate from the UI palette.
    2.  Click on the desired qubit wire to place the gate.
    3.  The **Gate Editor** will open, allowing you to enter the desired exponent `t`.

### CCZPow: CCZ^t Gate

-   **Description:** The Controlled-Controlled-Z gate raised to the power of `t`.
-   **How to Use:**
    1.  Select the **CCZPow** gate from the UI palette.
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