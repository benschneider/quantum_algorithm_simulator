# Estimate success from the joint state

Load **File > Load Template > Algorithms > grover algorithm**. For the target `100`, q2 must be 1 **and** q1 and q0 must both be 0. How often would reading all three wires give the right answer?

1. Click **First amplification** (step 7). **Wire readout** gives q2 an 87.50% chance of reading 1.
2. Find `100` in **Joint state**. Its probability is 25/32, so the search succeeds about 78% of the time. The wire probability also includes `101`, `110` and `111`. Each of those rows has probability 1/32.
3. Click **Second amplification** (step 13). The target row now has probability about 0.9453. A second round raised the chance of the complete answer to about 95%.

In **Joint probabilities**, hover over the target bar to inspect its amplitude and phase. The bar height is the probability of the whole bit string.

Select **Overview**. For this three-wire Grover circuit, the bars show exact probabilities. On larger registers each bar combines outcomes with the same leading bits, and the number of bars adjusts to the panel width.

Try **ghz 50** and press **Run ▶**. Above 21 wires the editor uses compact simulation. **Overview** and **Joint samples** use the same 1,024 shots, while **Wire readout** still gives each wire’s exact probability. Click either outer bar to see its count. All shots fall in those two bars because the circuit produces only all zeros or all ones.

Run **qft 50** next. Its shots spread across the ranges. An empty bar means that range received zero shots in this run. Switch to **Amplitude waveform** to follow the changing phase across individual basis indices. **Range** lets you choose the first index and stride.
