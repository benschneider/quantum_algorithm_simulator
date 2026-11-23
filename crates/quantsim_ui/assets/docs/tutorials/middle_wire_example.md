For a three-wire register, read each row as **q2 q1 q0**. The plots call the middle wire **B**. A Hadamard on q1 combines pairs with the same outer bits. For incoming amplitudes `a` (middle bit 0) and `b` (middle bit 1), the outgoing amplitudes are `(a + b)/√2` and `(a − b)/√2`.

Suppose the input probabilities of `000`, `001`, `101`, and `111` are 25%, 10%, 30%, and 35%, with positive real amplitudes. All other rows start at zero.

| Paired rows | Probability before H | Probability after H |
| --- | --- | --- |
| `000` and `010` | 25% and 0% | 12.5% each |
| `001` and `011` | 10% and 0% | 5% each |
| `100` and `110` | 0% and 0% | 0% each |
| `101` and `111` | 30% and 35% | 64.90% and 0.10% |

The last pair starts with amplitudes √0.30 and √0.35. Their sum reinforces the `101` outcome, while their difference almost cancels the `111` outcome. The pair still accounts for 65% of the total probability.
