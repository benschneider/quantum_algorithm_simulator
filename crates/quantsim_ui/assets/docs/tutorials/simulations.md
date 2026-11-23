# Follow a simulation one step at a time

Load **File > Load Template > grover**. The two search wires represent four possible answers. Follow the row `11` in **Results > Joint state** as the circuit marks it and then amplifies it.

1. Click **Spread** (step 1). The `11` row has amplitude `+0.5` and probability `0.25`, just like the other three rows.
2. Click **Oracle** (step 2). Its amplitude is now `−0.5`. Squaring the magnitude still gives `p=0.2500`. This is where the circuit marks its answer.
3. Click **Amplify 11** (step 6). The intervening gates make the marked row add constructively and the others cancel. Only `11` remains, with probability 1.

Click column **2** again to revisit the marked state. **Run ▶** takes you back to the final step. Inspecting a step leaves the circuit ready to run again.
