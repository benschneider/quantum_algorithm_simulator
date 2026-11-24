# Estimate success from the joint state

Load **File > Load Template > grover algorithm**. For the target `100`, q2 must be 1 **and** q1 and q0 must both be 0. How often would reading all three wires give the right answer?

1. Click **First amplification** (step 7). **Wire readout** gives q2 an 87.50% chance of reading 1.
2. Find `100` in **Joint state**. Its probability is 25/32, so the search succeeds about 78% of the time. The wire probability also includes `101`, `110` and `111`. Each of those rows has probability 1/32.
3. Click **Second amplification** (step 13). The target row now has probability about 0.9453. A second round raised the chance of the complete answer to about 95%.

In **Joint probabilities**, hover over the target bar to inspect its amplitude and phase. The bar height is the probability of the whole bit string.

**Overview** shows the probability across the whole register at the selected step. Try **ghz 50** and press **Run**. Select either outer bar to see how many shots landed there. Together those two bars contain every shot because the circuit produces only all zeros or all ones.

Try **qft 50** next. Its shots spread across the ranges, and the percentage axis adjusts so you can see the smaller bars. Switch to **Amplitude waveform** to see the changing phase up close. On wide circuits an empty range means none of the 1,024 shots landed there.
