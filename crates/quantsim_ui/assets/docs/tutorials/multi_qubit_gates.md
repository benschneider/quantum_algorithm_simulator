# Follow the control wire

To read a controlled-X gate (**CX**, also called CNOT), follow **Ctrl** to **X tgt** in the same column. The X flips its target on the part of the state where the control is 1. If a column contains several gates acting on multiple wires, match the number after each label to find the pair.

1. Open **File > Load Template > Entanglement > ghz**. Here the joint rows read **q2 q1 q0**. Click column **1**. The populated rows are `000` and `001`. Only q0 has been through a Hadamard.
2. Click **Bell pair** to inspect step 2. This CX uses q0 as control and q1 as target. Follow the `001` branch. Its control is 1, so it becomes `011`. The `000` branch stays `000`.
3. The last CX also uses q0 as its control, this time targeting q2. Predict what happens to `011`, then click **Add q2**. The only outcomes are `000` and `111`, each with probability 50%.

This is a three-qubit GHZ state. A joint measurement can give either outcome, and in both outcomes all three wires agree.
