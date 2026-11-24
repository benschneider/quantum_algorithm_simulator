# Follow a simulation one step at a time

Load **File > Load Template > Algorithms > grover**. The two search wires represent four possible answers. Follow the row `11` in **Results > Joint state** as the circuit marks it and then amplifies it.

1. Click **Spread** (step 1). The `11` row has amplitude `+0.5` and probability `0.25`, just like the other three rows.
2. Click **Oracle** (step 2). Its amplitude is now `−0.5`. Squaring the magnitude still gives `p=0.2500`. This is where the circuit marks its answer.
3. Click **Amplify 11** (step 6). The intervening gates make the marked row add constructively and the others cancel. Only `11` remains, with probability 1.

Click column **2** again to revisit the marked state. **Next step** advances one column, **Play** advances one every two seconds, and **Run ▶** shows the final step.

Before running a wide circuit, open **Est. peak RAM** beside **Run ▶**. It estimates the simulation buffers for the full run. Joining previously independent groups can raise the estimate even when the wire count stays the same. Set **Warn above** to the amount you want flagged. You can still run above that threshold, and the browser or desktop app can use additional memory.
