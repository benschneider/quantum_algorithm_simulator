# Single-Qubit Gates on Entangled States: A Clean Primer

## 1. What an Entangled State Physically Means

Consider two qubits, A and B. Each qubit is a two-level quantum system (e.g., spin up/down, photon H/V polarization, superconducting |0> / |1>). The computational basis is `|00>`, `|01>`, `|10>`, `|11>`.

A canonical entangled state is the Bell state:  
```
|psi> = 1/sqrt(2) * (|00> + |11>)
```

Physically:  
- If you measure only qubit A in the computational basis, you get 0 or 1 with 50-50 probabilities.  
- **Conditional correlations**: If the outcome for A is 0, then a subsequent measurement of B is guaranteed to be 0; if A is 1, B is guaranteed to be 1. Each qubit alone looks random, but joint outcomes are correlated.  
- The information is stored in the joint state, not in either subsystem individually. Entanglement ≠ faster-than-light signaling; it means definiteness lives in correlations.

**Key physical point**: A local action (on A alone) cannot change the statistics available at B without classical communication. But it can (and does) change the pattern of correlations between A and B.



```text
 [Qubit A] ---entangled--- [Qubit B]

 Apply a local gate on A:
    A rotates
    B unchanged locally
 But the connection (correlation) remains.
```

***

## 2. Single-Qubit Gates on Entangled States (Two-Qubit View)

A single-qubit gate `U` is a 2×2 unitary acting on one qubit’s Hilbert space. In a two-qubit system:  
- Acting on A is `U * I`.  
- Acting on B is `I * U`.  

This operation does not “break” entanglement; it just rotates the local basis and thereby reshapes correlations. (More formally, local unitaries preserve the amount of bipartite entanglement.)

***

## 3. Scaling to Three Qubits: How to Apply a Gate on One Slot

Let qubits be ordered A (left), B (middle), C (right). The basis is:  
```
|000>, |001>, |010>, |011>, |100>, |101>, |110>, |111>
```

A general state is:  
```
|psi> = sum_{a,b,c in {0,1}} alpha[abc] * |abc>
```
(Normalization: `sum |alpha[abc]|^2 = 1`)

To apply a gate on qubit B, use `I * U * I`. The operational rule is to collect pairs of amplitudes that differ only in the middle bit for each fixed `(a,c)`:  
```
(alpha[a0c], alpha[a1c]) -> U * [alpha[a0c], alpha[a1c]]^T
```

Do this for all four choices of `(a,c)` in `{0,1}^2`. This same “pairing rule” works for any target qubit: you group amplitudes that differ only in the target bit.

```text
Basis states (index order):
 000, 001, 010, 011, 100, 101, 110, 111

Pairing for middle bit (B):
 (000, 010), (001, 011), (100, 110), (101, 111)

apply 2x2 U to each pair
```

***

## 4. Worked Example with a Generic 3-Qubit State

We'll use a truly non-symmetric state supported on four basis strings with different probabilities, then apply a Hadamard to the middle qubit B.

**4.1 Initial State**

Choose probabilities that sum to 1:  
```
Pr(000) = 0.25, Pr(001) = 0.10, Pr(101) = 0.30, Pr(111) = 0.35
```

Take real, positive amplitudes (zero phases for simplicity):  
```
|psi> = 0.5 * |000> + 0.316227766 * |001> + 0.547722558 * |101> + 0.591607978 * |111>
```
(with all other basis amplitudes = 0)

**4.2 Apply `I * H * I` (Hadamard on B)**

The Hadamard is  
```
H = 1/sqrt(2) * [[1, 1], [1, -1]]
```
Apply the pairing rule for each fixed `(a,c)`:

- **(a,c) = (0,0)**: pair `(alpha[000], alpha[010]) = (0.5, 0)`  
  - `alpha'[000] = 1/sqrt(2) * (0.5 + 0) = 0.353553391`  
  - `alpha'[010] = 1/sqrt(2) * (0.5 - 0) = 0.353553391`  
- **(a,c) = (0,1)**: pair `(alpha[001], alpha[011]) = (0.316227766, 0)`  
  - `alpha'[001] = 1/sqrt(2) * (0.316227766 + 0) = 0.223606798`  
  - `alpha'[011] = 1/sqrt(2) * (0.316227766 - 0) = 0.223606798`  
- **(a,c) = (1,1)**: pair `(alpha[101], alpha[111]) = (0.547722558, 0.591607978)`  
  - `alpha'[101] = 1/sqrt(2) * (0.547722558 + 0.591607978) = 0.805628349`  
  - `alpha'[111] = 1/sqrt(2) * (0.547722558 - 0.591607978) = -0.031031679`  
- **(a,c) = (1,0)**: pair `(alpha[100], alpha[110]) = (0, 0)` remains zero.

| basis | amplitude'  | probability' |
|-------|-------------|--------------|
| 000   | 0.353553391 | 0.125000     |
| 001   | 0.223606798 | 0.050000     |
| 010   | 0.353553391 | 0.125000     |
| 011   | 0.223606798 | 0.050000     |
| 101   | 0.805628349 | 0.649037     |
| 111   | -0.031031679| 0.000963     |

**4.3 Measurement Probabilities After the Gate**

Probabilities are squared magnitudes of amplitudes (rounding shown to 6-9 significant figures):  
- `Pr'(000) = 0.125`  
- `Pr'(010) = 0.125`  
- `Pr'(001) = 0.050`  
- `Pr'(011) = 0.050`  
- `Pr'(101) ≈ 0.649037`  
- `Pr'(111) ≈ 0.000963`

All others remain 0. (Sanity check: they sum to ≈ 1 within rounding.)

*Interpretation*: Inside each two-element pair tied to the target qubit, the Hadamard forms the sum and difference of the incoming amplitudes. Where both entries were nonzero (the `(1,1)` pair), we see constructive interference in `|101>` and destructive interference in `|111>`. A local gate re-expresses the entangled state in a new local basis, changing the correlation pattern but not the existence of entanglement.

***

## 5. General Recipe (Any N Qubits, Any Target, Any 1-Qubit Gate)

Let  
```
U = [[u00, u01], [u10, u11]]
```
act on qubit `k` (`0` = leftmost). Write:  
```
|psi> = sum_{b in {0,1}^n} alpha[b] * |b>
```

For every fixed assignment of all bits except `b_k`, form the pair:  
```
(alpha...0..., alpha...1...) -> [alpha'...0..., alpha'...1...] = U * [alpha...0..., alpha...1...]
```

This is equivalent to multiplying the full state vector by  
```
I^{⊗k} * U * I^{⊗(N-k-1)}
```
but the pairing rule is the most efficient way to “see” what happens.

**Local-unitary invariance of bipartite entanglement**: If you bipartition the system as (target qubit) vs (the rest), any unitary on the target alone leaves the entanglement entropy across that cut unchanged. Practically: local gates change correlations’ orientation, not their total “amount”.

***

## 6. What Do You Actually Measure?

- Measuring all qubits in the computational basis yields one basis string; probability = squared amplitude.  
- Measuring one qubit collapses its slot and leaves a conditional state on the others. In entangled states, these conditional states depend on the outcome, producing non-classical correlations between separated measurements.  
- Without classical communication, a local gate on A cannot change B’s local statistics; it can only change joint statistics (correlations) seen when comparing notes.

***

## 7. Implementation Tips (State-Vector Ordering)

1.  **Bit order**: Adopt a consistent convention (here: A=leftmost/most-significant bit). Index basis states as integers in binary (e.g., `|abc>` maps to index `4a + 2b + c`).  
2.  **Efficient application**: To apply `U` on qubit `k`, stride through the state vector in blocks that pair indices differing only in bit `k`. Apply the 2×2 multiply to each pair.  
3.  **Phases matter**: Even if probabilities are the same, relative phases change interference at the next gates.

***

## 8. Quick Exercises

1.  **Pauli-X on C**: Show it swaps amplitudes inside each fixed `(a,b)` pair: `|ab0>` ↔ `|ab1>`.  
2.  **Phase gate S = diag(1,i) on B**: Multiply every amplitude with `b=1` by `i`. No mixing now, but later gates will turn these phases into probability shifts via interference.  
3.  **Conditioned measurement**: For the worked example after `I * H * I`, compute the conditional state of AC given that measuring B returns 0 vs 1.

***

### One-Line Intuition

A local gate is a local rotation. It doesn’t cut the “rope” of entanglement; it just twists how that rope aligns the outcomes across the systems, changing interference within target-bit pairs and thus reshaping correlations seen in measurements.

***

## Appendix: Try-It Code (Rust)

This snippet demonstrates applying a 1-qubit gate to any target bit using striding/pairing on a little-endian indexed state vector (bit 0 is rightmost, C).

```rust
fn apply_gate_on_qubit(state: &mut [f64], U: [[f64; 2]; 2], target_bit: usize) {
    let n = state.len();
    let mask = 1 << target_bit;
    let half_block = mask;
    let block_size = 2 * half_block;

    // Iterate over blocks of size block_size
    for block_start in (0..n).step_by(block_size) {
        // Within each block, iterate over pairs differing in target_bit
        for i in 0..half_block {
            let idx0 = block_start + i;
            let idx1 = idx0 + half_block;

            let amp0 = state[idx0];
            let amp1 = state[idx1];

            state[idx0] = U[0][0] * amp0 + U[0][1] * amp1;
            state[idx1] = U[1][0] * amp0 + U[1][1] * amp1;
        }
    }
}

fn main() {
    // Example state vector for |psi> = 0.5|000> + 0.316227766|001> + 0.547722558|101> + 0.591607978|111>
    // Indexed little-endian: bit0 = C (rightmost)
    // Indices: |abc> -> a*4 + b*2 + c*1
    let mut state = [0.0_f64; 8];
    state[0b000] = 0.5;
    state[0b001] = 0.316227766;
    state[0b101] = 0.547722558;
    state[0b111] = 0.591607978;

    // Hadamard matrix
    let inv_sqrt2 = 1.0 / 2f64.sqrt();
    let H = [[inv_sqrt2, inv_sqrt2], [inv_sqrt2, -inv_sqrt2]];

    // Apply H on middle bit (bit 1)
    apply_gate_on_qubit(&mut state, H, 1);

    // Print resulting amplitudes
    for (i, amp) in state.iter().enumerate() {
        println!("|{:03b}>: {:.9}", i, amp);
    }
}
```