# Explore the circuit templates

Open **File > Load Template**. Click a numbered column to read all wires after that step, or click a section heading to read its last step. Hover over a gate to see what it does in the example. In **Results**, q0 is the rightmost bit of each joint state. The probability beside a wire describes that wire alone.

## Find a correlation

- **Bell** — Click step 1. The two populated rows are `00` and `01`, so only q0 is uncertain. At step 2 the rows are `00` and `11`. Each wire now reads 1 half the time, and the two always agree.
- **GHZ** — Compare steps 2 and 3. The rows change from `000` and `011` to `000` and `111`. The second CNOT extends the correlation to q2. Only two of the eight possible outcomes occur.
- **ghz 50** — At step 6 the first 32 wires agree and q49 still reads zero. At step 7 all 50 agree. In **Joint samples**, each shot records either 50 zeros or 50 ones. Each wire's exact probability of reading one is 50%.
- **bell 256** pairs the two ends of a 256-wire register. At step 2, read the leftmost and rightmost bits in **Joint samples**. They always match. All 254 bits between them stay zero.
- **Bell test complex** — q0 and q1 share a Bell pair. Ancillas q2 and q3 control the rotations used to read it. At step 5, group the rows by their first two bits (q3 q2). Each setting has 25% total probability. Within setting `00`, the last two bits agree about 85% of the time. Within setting `11`, they agree about 15% of the time. To check a setting, add the probabilities of its two matching rows and divide by 0.25.

## Turn a phase into an answer

- **Deutsch algorithm** — q0 asks the question and q1 is the helper. Compare steps 2 and 3. The oracle for the balanced function `f(x) = x` changes amplitude signs while every row stays at 25%. At step 4, the final Hadamard turns that difference into a definite 1 on q0, the answer for a balanced function.
- **Grover** — Of the four possible states, the oracle marks `11`. Compare its amplitude before and after step 2. Its sign flips while its probability stays at 25%. Click step 6 to see interference concentrate the entire probability on `11`.
- **Grover algorithm** — This version searches eight states. Follow the `100` row (q2 = 1). It starts at 12.5%, reaches 78.125% after the first amplification at step 7, and about 94.53% after the second at step 13.

## Read a Fourier transform

To try the transform on your own input, choose **Gates → Insert QFT…** and select the first wire, the width, and the step after input preparation. Click the new **QFT** section heading to see its output, or select individual columns to follow the gates.

- **fourier wave 50** — Choose **Next step** to read each caption above **Results → Amplitude waveform**. Steps 1–4 spread the flat real amplitudes across the first 2, 4, 32 and 64 positions. Step 5 spreads them over all 2^50 positions. The graph still shows 64 at a time. In steps 6–10, five phase gates turn progressively smaller blocks of amplitudes. At the last step, the imaginary trace peaks at index 8, crosses zero at 16 and reaches its trough at 24. The real trace follows a cosine. Together the 64 points show two periods of a single Fourier mode.
- **QFT** — Step 1 prepares `01`. At step 3, the populated rows `01` and `11` have the same probability and their amplitudes differ by a quarter turn in phase. At step 5, read the four amplitudes in basis order. They are `0.5`, `0.5i`, `−0.5` and `−0.5i`. A probability chart alone would show four equal bars.
- **QFT4** — Step 1 prepares `1011`, or 11. At step 13, all 16 rows have probability 6.25%. Follow the first two amplitudes. The first is `0.25` and the next is about `−0.0957 − 0.2310i`. Each successive row advances by 11/16 of a turn. [Fourier transform definition](https://quantum.cloud.ibm.com/docs/en/api/qiskit/qiskit.circuit.library.QFTGate)
- **qft 50** — Select **Input** at step 1. An X gate on q45 prepares a single spike at basis index 2^45. Choose **Play** to follow the transform or **Run** to jump to step 101. The imaginary trace peaks at indices 8 and 40 and reaches its troughs at 24 and 56. Each wire reads one with 50% probability. To reveal a phase in the wire readout, add an H on q49 at step 100. The final swap carries that wire to q0, which now reads one about 0.96% of the time. At step 6, change the CZPow between q45 and q49 from `0.0625` to `0.3333333` and run again. Now q0 reads one with 25% probability.

## Follow a state from sender to receiver

Step 2 of **quantum teleportation** creates a shared Bell pair on q1 and q2. At step 4, the sender's input on q0 has a 25% chance of reading 1 and a positive imaginary amplitude on its 1 component.

Click step 6. The receiver q2 still reads 1 half the time. The X correction at step 7 restores its 25% probability. To see why the Z correction at step 8 matters, follow the `101` row in **Joint state**. Its amplitude changes from `−0.25i` to `+0.25i`, restoring the message's relative phase in that branch. This circuit uses quantum controls for the two corrections. In a separated sender–receiver protocol, the sender measures and communicates two classical bits. [Teleportation protocol](https://quantum.cloud.ibm.com/learning/en/courses/basics-of-quantum-information/entanglement-in-action/quantum-teleportation)

## Follow a Sycamore coupler

Open **sycamore echo 53** and press **Play** to watch seven layers of CNOTs reach every site from q23. At step 8 the only joint outcomes are all zeros and all ones. At step 9 a Z gate on q33 changes their relative phase while the colors stay at 50%. The circuit then reverses the CNOT layers. The final Hadamard makes q23 read 1 every time. To compare, right-click the Z in step 9 of the circuit grid to remove it, then run again. Now q23 reads 0 every time. [Watch the 29-second demo](https://benschneider.github.io/quantum_algorithm_simulator/showcase/sycamore-echo-53.mp4) · [Looping GIF](https://benschneider.github.io/quantum_algorithm_simulator/showcase/sycamore-echo-53.gif)

Open **File**, choose **Load Template**, and select **sycamore 53**. The five-step circuit uses a 2×4 patch on the 53-site chip. [Watch the 3D walkthrough](https://benschneider.github.io/quantum_algorithm_simulator/showcase/sycamore-3d-walkthrough.mp4)

- In the **Layout** tab, drag inside the **3D stack** to orbit the chip. Time runs upward. Gold bonds show which couplers are active on each layer.
- Choose **Next step** to prepare q21 in 1 and seven neighbors in superposition. Advance twice more. Four outer row couplers light up at step 2, then two middle row couplers at step 3.
- At step 3, choose **q33** under **Inspect**. It belongs to a four-qubit group. Hover over q33 on the active layer to see its 50% probability of reading 1.
- Advance to step 4. Four couplers connect the rows. The inspector now shows **Independent** for q33, and its probability of reading 1 has reached 100%. The other seven patch sites share one group.

Step 5 turns the accumulated phases into measurable probabilities on those seven sites. To watch the circuit again, return to step 0 and press **Play** for one step every two seconds. The patch uses the ideal SYC gate on ten of the chip’s 86 couplers. [Chip layout and ideal gate](https://quantumai.google/hardware/datasheet/weber.pdf)

## Inspect a parity check

In **plaquette z check**, four data qubits surround one ancilla, q4. This is one local check from a surface-code-style layout. Under **Qubit layout**, choose **2D layout** and click the central square **A0 (q4)** to highlight its four neighbors.

Step 1 flips q3. Predict when q4 will first read 1. Switch to **3D stack** and click the central ancilla at **t4**, then **t5**. The dashed links show its physical neighbors. At t5 the gold connection shows the CNOT from q3 to q4. In **Results**, q4's probability of reading 1 rises from 0% to 100% as it records odd parity.

For the four repetition-code templates, open **Error correction** in this tutorial.
