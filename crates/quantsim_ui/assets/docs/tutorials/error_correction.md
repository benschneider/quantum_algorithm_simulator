# Recover a logical qubit

From the **File** menu, open **Load Template** and choose **error correction bit flip**. Click **Run**, then use **Timestep** to compare steps 2, 7 and 8 in Results. Hover over a gate to see its role.

Read each basis label from left to right as `q2 q1 q0`. The middle wire, q1, holds the input and the recovered output. At step 2 the amplitudes are about `0.8660` for `000` and `0.5000i` for `010`. The imaginary component gives you a way to check that recovery preserves the relative phase.

| What to inspect | Bit flip | Phase flip |
| --- | --- | --- |
| Prepare q1 | 2 | 2 |
| Finish encoding across three wires | 4 | 5 |
| Inject the faults | 5 · X | 6 · Z |
| Read the two parity bits | 7 | 9 |
| Run the decoder on q1 | 8 | 10 |

At step 7, the populated rows are `101` and `111`. Their outer bits agree on **11**. Together these two parity bits form the **syndrome**. Both parity checks flagged a mismatch. The last gate uses those bits as controls to flip q1. After recovery, `101` has amplitude `0.8660` and `111` has amplitude `0.5000i`. Compare their middle bits and amplitudes with step 2.

## Compare one bit flip with two

From **File**, open **Load Template** and choose **error correction bit flip two faults**. Click **Run**. At step 5, both outer wires flip together. Go to step 7 and read the outer bits of each populated row. Can you tell from the syndrome alone which run this is?

| Bit-flip template | Step 7 syndrome | Step 7 chance q1 is 0 | Step 8 chance q1 is 0 |
| --- | --- | --- | --- |
| error correction bit flip | `11` | 25% | 75% |
| error correction bit flip two faults | `11` | 75% | 25% |

Each parity bit records whether an outer wire disagrees with q1. Flipping q1 creates two disagreements. Flipping both outer wires creates the same two disagreements. The decoder assumes a single fault and flips q1 in both runs. In the first run that restores the input. In the second, q1 was already correct at step 7, so the same action introduces a logical bit flip. To check the percentages in Results, read the probability of `101`, the only populated row with q1 equal to zero.

## Compare one phase flip with two

From **File**, open **Load Template** and run **error correction phase flip**. Then run **error correction phase flip two faults**. In each run, compare steps 5 and 6 in Results. All eight probabilities stay at 12.5%. The raw amplitude for `010` changes sign when Z hits the middle wire and stays the same when Z hits both outer wires. For `001`, the opposite happens.

| Phase-flip template | Step 9 syndrome | Step 9 chance q1 is 0 | Step 10 chance q1 is 0 |
| --- | --- | --- | --- |
| error correction phase flip | `11` | 25% | 75% |
| error correction phase flip two faults | `11` | 75% | 25% |

At step 7 the Hadamard gates turn each phase flip into a bit flip on the affected wire. At step 9 the two stored parity bits report the same thing in both runs. Each outer wire disagreed with the middle wire. The decoder reads `11` and flips q1. That restores the input after one Z fault and introduces a logical bit flip after two. At steps 9 and 10, read the probability of `101` to check the table.

## Try a different fault

For an optional experiment, open either single-fault template. In the JSON tab, find the operation named **Inject a bit flip** or **Inject a phase flip**. Change its `qubits` entry to `[0]` or `[2]`, choose **Apply Changes**, and run again. Predict the outer bits before looking. With at most one fault of the template's chosen type, the final syndrome `q2 q0` is `00` for a clean run, `01` for q0, `11` for q1 and `10` for q2. To try the clean run, leave the fault timestep empty.

For the encoding and parity-check derivation, see [IBM Quantum Learning on repetition codes](https://quantum.cloud.ibm.com/learning/en/courses/foundations-of-quantum-error-correction/correcting-quantum-errors/repetition-codes).
