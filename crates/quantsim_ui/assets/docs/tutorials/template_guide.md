# Explore the circuit templates

Open **File > Load Template**. Click a numbered column to read all wires after that step, or click a section heading to read its last step. Hover over a gate to see what it does in the example. In **Results**, q0 is the rightmost bit of each joint state. The probability beside a wire describes that wire alone.

## Find a correlation

- **Bell** — Click step 1. The two populated rows are `00` and `01`, so only q0 is uncertain. At step 2 the rows are `00` and `11`. Each wire now reads 1 half the time, and the two always agree.
- **GHZ** — Compare steps 2 and 3. The rows change from `000` and `011` to `000` and `111`. The second CNOT extends the correlation to q2. Only two of the eight possible outcomes occur.
- **Bell test complex** — q0 and q1 share a Bell pair. Ancillas q2 and q3 control the rotations used to read it. At step 5, group the rows by their first two bits (q3 q2). Each setting has 25% total probability. Within setting `00`, the last two bits agree about 85% of the time. Within setting `11`, they agree about 15% of the time. To check a setting, add the probabilities of its two matching rows and divide by 0.25.

## Turn a phase into an answer

- **Deutsch algorithm** — q0 asks the question and q1 is the helper. Compare steps 2 and 3. The oracle for the balanced function `f(x) = x` changes amplitude signs while every row stays at 25%. At step 4, the final Hadamard turns that difference into a definite 1 on q0, the answer for a balanced function.
- **Grover** — Of the four possible states, the oracle marks `11`. Compare its amplitude before and after step 2. Its sign flips while its probability stays at 25%. Click step 6 to see interference concentrate the entire probability on `11`.
- **Grover algorithm** — This version searches eight states. Follow the `100` row (q2 = 1). It starts at 12.5%, reaches 78.125% after the first amplification at step 7, and about 94.53% after the second at step 13.

## Read a Fourier transform

- **QFT** — Step 1 prepares `01`. At step 3, the populated rows `01` and `11` have the same probability and their amplitudes differ by a quarter turn in phase. At step 5, read the four amplitudes in basis order. They are `0.5`, `0.5i`, `−0.5` and `−0.5i`. A probability chart alone would show four equal bars.
- **QFT4** — Step 1 prepares `1011`, or 11. At step 13, all 16 rows have probability 6.25%. Follow the first two amplitudes. The first is `0.25` and the next is about `−0.0957 − 0.2310i`. Each successive row advances by 11/16 of a turn. [Fourier transform definition](https://quantum.cloud.ibm.com/docs/en/api/qiskit/qiskit.circuit.library.QFTGate)

## Follow a state from sender to receiver

Step 2 of **quantum teleportation** creates a shared Bell pair on q1 and q2. At step 4, the sender's input on q0 has a 25% chance of reading 1 and a positive imaginary amplitude on its 1 component.

Click step 6. The receiver q2 still reads 1 half the time. The X correction at step 7 restores its 25% probability. To see why the Z correction at step 8 matters, follow the `101` row in **Joint state**. Its amplitude changes from `−0.25i` to `+0.25i`, restoring the message's relative phase in that branch. This circuit uses quantum controls for the two corrections. In a separated sender–receiver protocol, the sender measures and communicates two classical bits. [Teleportation protocol](https://quantum.cloud.ibm.com/learning/en/courses/basics-of-quantum-information/entanglement-in-action/quantum-teleportation)

## Inspect a parity check

In **plaquette z check**, four data qubits surround one ancilla, q4. This is one local check from a surface-code-style layout. Under **Qubit layout**, choose **2D layout** and click the central square **A0 (q4)** to highlight its four neighbors.

Step 1 flips q3. Predict when q4 will first read 1. Switch to **3D stack** and click the central ancilla at **t4**, then **t5**. The dashed links show its physical neighbors. At t5 the gold connection shows the CNOT from q3 to q4. In **Results**, q4's probability of reading 1 rises from 0% to 100% as it records odd parity.

For the four repetition-code templates, open **Error correction** in this tutorial.
