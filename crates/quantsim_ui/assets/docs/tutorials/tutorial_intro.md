# Read your first Bell circuit

Can two wires each look random and still agree every time? Follow the two gates of a Bell circuit to find out.

1. Click **Load Bell example** above, or choose **File > Load Template > bell**. In **Circuit**, the rows are wires q0 and q1. The numbered columns run from left to right.
2. Click column **1**, under **Split**. In **Results > Joint state**, the two possible outcomes are `00` and `01`, each with probability `0.5000`. Read each row as **q1 q0**. The Hadamard gate on q0 gives it an equal chance of 0 or 1. The other wire still reads 0.
3. Before clicking **Bell pair**, predict which bit will change when the controlled-X gate acts on q1. Click the heading. You should see `00` and `11`. Each wire has a 50% chance of reading 1, and their results always agree.

Clicking a section heading takes you to its last step. Click a numbered column to inspect any earlier step, or choose **Run ▶** to see the final state.
