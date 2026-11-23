# Define a gate by its matrix

A custom gate can live inside the circuit JSON. Try a one-wire phase flip between two Hadamards. The first H gives both outcomes equal amplitude. The custom gate changes the sign of the `1` component. Predict which outcome the last H will produce.

**Apply Changes** replaces the current circuit. To keep your circuit, open **JSON**, choose **Copy to Clipboard** and save the text first. Then replace the editor contents with this example and apply it.

```json
{
  "num_qubits": 1,
  "steps": [
    [{"id": "H", "qubits": [0]}],
    [{"id": "Custom", "qubits": [0], "name": "Custom diagonal gate",
      "matrix": {"BigEndian": [[1, 0], [0, -1]]}}],
    [{"id": "H", "qubits": [0]}]
  ]
}
```

Return to **Circuit**. At step 2, the two rows still have equal probability, but the amplitude of `1` is about `−0.7071`. At step 3, q0 reads 1 with certainty.

For a second run, paste the example again and change its `-1` to `1`. Apply it and click step 3 in **Circuit**. The custom gate is now the identity. It preserves both signs, so the last H returns q0 to `0`.
