# Turn a phase change into a bit flip

1. Choose **New Circuit** and set **Qubits** to **1**. In the **Gate Palette**, expand **Pauli Gates**. Select **H** and click q0 in columns **1** and **3**. Select **Z** and click q0 in column **2**.
2. Click column **1**. The Hadamard gives `0` and `1` equal positive amplitudes of about `0.7071`. In column **2**, Z changes the sign of the `1` amplitude. Each outcome still has probability 50%. This sign change is a **relative phase** between the two amplitudes.
3. Predict the outcome at column **3**, then click it. Only `1` remains. The second H takes the sum and difference of the incoming amplitudes. The opposite signs cancel in the sum for `0` and reinforce in the difference for `1`.

Right-click **Z** to remove it, then click column **3** again. The two H gates now return q0 to `0` with certainty. Comparing the two runs reveals a phase change that was invisible in the probabilities at step 2.
