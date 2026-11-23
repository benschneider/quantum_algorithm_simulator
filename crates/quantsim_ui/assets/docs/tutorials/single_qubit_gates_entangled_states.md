# Try two readout bases on a Bell pair

The Bell state has two amplitudes that add coherently, `(|00⟩ + |11⟩) / √2`. Applying H immediately before reading a wire lets you inspect it in the X basis. Try changing the basis of one partner, then both.

1. Load **File > Load Template > bell** and click **Bell pair**. Each wire alone reads 1 half the time. The joint outcomes agree every time.
2. Increase **Timesteps** to **4**. Expand **Pauli Gates** in the palette and select **H**. Place it on q0 at step **3** and on q1 at step **4**.
3. Click column **3**. With H on q0 alone, all four joint outcomes have probability 25%. In **Joint state**, find the negative amplitude on `11`. The other three amplitudes are positive.
4. Click column **4**. Both wires now have the same basis change. The two `01` and `10` amplitudes cancel, leaving `00` and `11` at 50% each.

Each wire has the same 50% chance of reading 1 at all three checkpoints. The pattern changes when you compare the wires. A Hadamard combines amplitudes in pairs that differ only on its target wire. Their relative signs determine which outcomes survive.
