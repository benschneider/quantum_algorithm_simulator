# Tutorial 2: Single-Qubit Gates

**Goal:** Understand and experiment with 1-qubit gates and how they act inside larger registers.

**Key ideas**
- Gates: X, Y, Z, H, S, T, Rx(theta), Ry(theta), Rz(theta)
- Local action: apply on target bit only; rest is untouched
- Phases matter later (interference), even if probabilities do not change now
- Conventions: qubits slice order == gate matrix order; first is MSB of local index

## Quick reference (ASCII)
```
X = [[0,1],[1,0]]
Z = [[1,0],[0,-1]]
H = (1/sqrt(2))*[[1,1],[1,-1]]
S = diag(1, i)   ;  T = diag(1, e^{i*pi/4})
Rx(a) = cos(a/2) I - i sin(a/2) X (and similarly for Ry,Rz)
```

## Example: Apply H on qubit 0 in a 2-qubit register
```
Initial: |01>
Apply H on q0 (LSB): H|1> = (|0>-|1>)/sqrt(2)
Result: (|00>-|01>)/sqrt(2)
```

## Rust: calling a 1-qubit gate
```rust
// Applies a 2x2 dense gate to target_bit (LSB=0)
pub fn apply_dense_1q(state: &mut [Complex<f32>], target_bit: usize, gate: [[Complex<f32>;2];2]) {
    let n = state.len();
    let stride = 1 << target_bit;
    let step = stride << 1;
    for base in (0..n).step_by(step) {
        for i in 0..stride {
            let i0 = base + i;
            let i1 = i0 + stride;
            let a0 = state[i0];
            let a1 = state[i1];
            state[i0] = gate[0][0]*a0 + gate[0][1]*a1;
            state[i1] = gate[1][0]*a0 + gate[1][1]*a1;
        }
    }
}
```

## Exercises
- Compose S then H on q0 of |01>. What are the final amplitudes?
- Show that Rz(theta) followed by Z commute up to a phase.
- On |00>, apply Rx(pi/2) on q1 and compute probabilities.

*Next: Tutorial 3: Multi-Qubit Gates*