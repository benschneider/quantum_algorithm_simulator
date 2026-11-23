# Tutorial: A 2-Qubit Example

This tutorial will walk you through a 2-qubit example, explaining the fundamental concepts of quantum computation using `quantsim`.

## The Basics of Quantum Computing

Quantum computing is based on a few key ideas:

*   **Qubits**: Unlike classical bits, which can be either 0 or 1, a qubit can be in a superposition of both states. This means it can be 0, 1, or a combination of both at the same time.
*   **Superposition**: A qubit's state is described by a vector. We can represent the state of a qubit as `α|0⟩ + β|1⟩`, where `α` and `β` are complex numbers called amplitudes. The probability of measuring the qubit as 0 is `|α|²`, and the probability of measuring it as 1 is `|β|²`.
*   **Entanglement** means the joint state cannot be written as a separate state for each qubit. In the Bell example, each wire alone gives a random bit, and measuring both in the same computational basis always gives matching bits.

## 1-Qubit Gates

A 1-qubit gate is an operation that acts on a single qubit. It can be represented by a 2x2 unitary matrix. Let's look at a common example, the Hadamard gate (H).

The matrix for the Hadamard gate is:
```
H = (1/√2) * [[1, 1], [1, -1]]
```

When we apply the H gate to a qubit in the `|0⟩` state (represented by the vector `[1, 0]`), the calculation is as follows:
```
H|0⟩ = (1/√2) * [[1, 1], [1, -1]] * [1, 0] = (1/√2) * [1, 1]
```
This result, `(1/√2)|0⟩ + (1/√2)|1⟩`, is an equal superposition of `|0⟩` and `|1⟩`.

## 2-Qubit Gates and Entanglement

A 2-qubit gate acts on two qubits. A common example is the Controlled-NOT (CNOT) gate. The CNOT gate has a control qubit and a target qubit. It flips the target qubit if and only if the control qubit is `|1⟩`.

In the **bell** template q0 controls q1. Results writes the basis in q1 q0 order, so this CNOT exchanges `01` and `11`.
```
CNOT = [[1, 0, 0, 0],
        [0, 0, 0, 1],
        [0, 0, 1, 0],
        [0, 1, 0, 0]]
```

### Creating a Bell State

Let's see how to create an entangled state, also known as a Bell state:

1.  **Start with two qubits, both in the `|0⟩` state.**
    The combined state is `|00⟩`, represented by the vector `[1, 0, 0, 0]`.

2.  **Apply a Hadamard gate to q0.**
    The left factor acts on q1 and the right factor acts on q0.
    ```
    I ⊗ H = (1/√2) * [[1, 1, 0, 0], [1, -1, 0, 0], [0, 0, 1, 1], [0, 0, 1, -1]]
    ```
    Applying this to `|00⟩`:
    ```
    (I ⊗ H)|00⟩ = (1/√2) * [1, 1, 0, 0]
    ```
    This state is `(1/√2)(|00⟩ + |01⟩)`.

3.  **Apply a CNOT gate.**
    Now we apply the CNOT matrix to the result from the previous step:
    ```
    CNOT * (1/√2) * [1, 1, 0, 0] = (1/√2) * [1, 0, 0, 1]
    ```
The final state is `(1/√2)(|00⟩ + |11⟩)`. This is the famous Bell state. If you measure the first qubit and get 0, the second will also be 0. If you get 1, the second will also be 1. The outcomes are perfectly correlated.

This concludes our brief tutorial. You can use `quantsim` to experiment with these concepts and build your own quantum circuits.
