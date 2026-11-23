# Read one wire from the joint state

Open **File > Load Template > deutsch algorithm**, then click **Decode**. The circuit asks whether a function is constant or balanced. In this example, a 1 on q0 is the answer for a balanced function.

1. In **Results > Joint state**, find `01` and `11`. Each row has probability `0.5000`. Their amplitudes are about `+0.7071` and `−0.7071`. Squaring the magnitude of either amplitude gives the same probability.
2. The **rightmost** bit is q0. Add the probabilities of the rows where q0 is 1. Both qualify, giving 100%. For q1, only `11` qualifies, giving 50%. Compare your totals with **P(1)** in **Wire readout**.
3. Revisit column **3**, then click **Decode** again. Selecting a step computes the probabilities for all wires without drawing a sample or collapsing the state. A single physical measurement of both wires at step 4 would return one bit string, either `01` or `11`.
