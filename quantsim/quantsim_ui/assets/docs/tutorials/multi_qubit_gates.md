# Tutorial 3: Multi-Qubit Gates (todo)

**Goal:** Use CX, CZ, CY, SWAP and understand qubit-order conventions.

**Key ideas**
- Gate matrix qubit order: first = MSB, last = LSB
- Controlled gates: control=MSB, target=LSB (unless noted)
- Pairing rule for acting on a specific slot in N qubits

## ASCII pairing diagram (B is middle bit)
```
[000] [001] [010] [011] [100] [101] [110] [111]
 (000,010) (001,011) (100,110) (101,111)  <- apply 2x2 on each pair for B
```

## Exercises
- Build Bell: H on q0 then CX(q0,q1)
- Build GHZ: H(0); CX(0,1); CX(0,2)

*Next: Tutorial 4: Measurements*