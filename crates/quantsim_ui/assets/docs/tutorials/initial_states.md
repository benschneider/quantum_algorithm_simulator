# Change the input to a Bell circuit

Can the same two gates make the wires disagree every time? Try changing only the input on q1.

1. Load **File > Load Template > bell** and click **Bell pair**. The joint outcomes `00` and `11` each have probability 0.5.
2. Open **Settings > Set Initial State...**. Set **Real** to `0` for row `|00>` and to `1` for row `|10>`. Leave every other entry, including the **Imaginary** column, at zero. Click **Apply**.
3. Move **Timestep** to 0. The only populated row is now `10`. Reading the bits as q1 q0 shows that q1 starts at 1.
4. Click **Bell pair** again. Look for `01` and `10`, each with probability 0.5. When q0 is 1, the controlled X flips q1 from 1 to 0. When q0 is 0, q1 stays at 1.

The entries are amplitudes. Here the single amplitude of 1 gives the input total probability 1. To repeat the original run, open **Set Initial State...**, choose **Reset to |0>**, then click **Bell pair**.
