# Define a gate by its matrix

Can a gate change the final answer while leaving both probabilities at 50% immediately after it? Place a custom phase flip between two Hadamards to find out.

1. Start a **New Circuit**, set **Qubits** to 1, and place **H** from **Pauli Gates** on q0 in columns 1 and 3.
2. Open **Custom Gates > Edit**. In **Real Part**, set the four entries to `1, 0` on the top row and `0, -1` on the bottom. In **Imaginary Part**, set all four entries to zero. Choose **Save**, select **Custom**, and place it in column 2.
3. Click column 2. Both outcomes have probability 0.5. In **Joint state**, the amplitude of `1` is about `−0.7071`. Predict what the last H does with that minus sign, then click column 3. Q0 reads 1 with certainty.

For a comparison, change the bottom-right real entry back to `1` and save. The palette now holds the identity. Each placed gate keeps its own matrix. Right-click the Custom gate in column 2 to remove it, then place the new Custom there and inspect column 3 again. With both amplitudes positive before the last H, q0 returns to `0`.

Click a placed gate to inspect its matrix, or open **JSON** to see the matrix stored with that operation.
