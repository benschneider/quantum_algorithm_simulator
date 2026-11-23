# Read a five-wire repetition circuit

Open **File**, choose **Load Template**, and load **error correction phase flip**. Three data wires (q0, q1, q2) encode one logical qubit. Two separate ancillas (q3, q4) start in `|0>` and collect parity. The middle data wire, q1, holds the logical input before encoding and the output after decoding.

Click a numbered column to inspect the joint state of all five wires after that step. Click a section heading to inspect its last step. Step 0 shows the initial state. Read the basis rows in **q4 q3 q2 q1 q0** order, with the ancillas first. At step 2 the amplitudes are `0.8660` for `00000` and `0.5000i` for `00010`. Keep their relative phase in mind when you compare the decoded output. Inspecting a step leaves the state available for the next gate.

| End of section | Bit-flip template | Phase-flip template |
| --- | --- | --- |
| Prepare input | 2 | 2 |
| Encode data | 4 | 5 |
| 1 fault / 2 faults | 5 | 6 |
| Z coupling / X coupling | 7 | 10 |
| Correct X / Z errors | 11 | 14 |
| Decode output | 13 | 17 |

## Follow the parity checks

In **Qubit layout** below the circuit, read the sites from left to right as **q0 · A0 · q1 · A1 · q2**. Select A0 (wire q3) to highlight its two neighbors, q0 and q1. A1 (wire q4) neighbors q1 and q2.

In the two-fault bit-flip template, choose **3D stack** and look at step 5. The X gates act on the two outer data sites. Follow A0's vertical trail to see its CNOT with q0 at step 6 and with q1 at step 7. Select A0 on layer 7 and read q3 and q4 in **Results**. Both now hold 1.

The links in these examples show the four parity-check couplings. Other steps can use gates between distant sites. For example, the encoding CNOT from q1 to q0 acts directly in the simulator.

In **error correction bit flip**, the encoded data occupy `000` and `111`. During **Z coupling**, CNOTs from q0 and q1 write their parity into q3. CNOTs from q1 and q2 do the same for q4. A 1 flags a disagreement in that pair.

In **error correction phase flip**, the data are encoded in the X basis. Compare steps 5 and 6. All eight populated basis rows still have probability 12.5%, but the Z fault changes amplitude signs. In **X coupling**, each ancilla gets a Hadamard, controls one CNOT into each of its two data wires, then gets a final Hadamard. That last H makes the X parity readable as a 0 or 1 on the ancilla. At step 10 both checks are ready. For the default middle-wire fault, every populated row starts with `11`.

| Data fault | q4 q3 syndrome |
| --- | --- |
| None | `00` |
| q0 only | `01` |
| q1 only | `11` |
| q2 only | `10` |
| q0 and q2 together | `11` |

## Compare one fault with two

Run **error correction phase flip** and **error correction phase flip two faults** in turn. Check the syndrome at step 10 and the logical output at step 17. For the bit-flip pair, use steps 7 and 13. Both versions of each pair use the same encoder, parity checks and correction gates.

A fault on q1 changes both pair parities. Faults on q0 and q2 change one parity each, producing the same `11`. The correction circuit uses q3 and q4 directly as controls. Syndrome `01` corrects q0, `11` corrects q1 and `10` corrects q2. With one matching fault this restores the encoded state on all three data wires. With faults on both outer wires, the same correction acts on q1, leaving a logical flip. The final decoder returns q0 and q2 to zero and leaves the two syndrome bits on q3 and q4.

| Faults | Final amplitude of `11000` | Final amplitude of `11010` | Chance q1 is 0 |
| --- | --- | --- | --- |
| q1 only | `0.8660` | `0.5000i` | 75% |
| q0 and q2 | `0.5000i` | `0.8660` | 25% |

To try another syndrome, open a single-fault template and find the injection gate in the JSON tab. Change its `qubits` entry from `[1]` to `[0]` or `[2]`, choose **Apply Changes**, and inspect the end of the coupling section. For a clean run, replace the entire fault step with `[]`.

## Annotate your own circuit

Click a wire label to choose **Data**, **Ancilla** or **Unlabeled**. Open **Sections** beside the timestep slider, choose **New section**, enter its label and first and last steps, then save. Both endpoints are included. The wire roles and section ranges are saved with the circuit JSON.

For the derivation of the two parity checks, see [IBM Quantum Learning on repetition codes](https://quantum.cloud.ibm.com/learning/en/courses/foundations-of-quantum-error-correction/correcting-quantum-errors/repetition-codes).
