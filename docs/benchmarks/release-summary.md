# LinkedIn release summary

How far can a classical quantum simulator go when it uses the structure of the circuit?

I benchmarked Quantum Algorithm Simulator in native Rust release mode on an Apple M1 Max. Independent product circuits, Clifford GHZ, independent four-qubit entangled groups, and basis-input QFT all ran at 1,024 qubits. I checked complex amplitudes and ran coherent reversals, including a 53-wire Sycamore-layout Clifford echo that converts a relative phase into a single output bit.

On the connected non-Clifford test, I ran the dense and grouped simulators through 24 qubits. Each additional qubit tracked in the same dense vector doubles its storage.

On a 16-qubit ladder, I measured where the experimental matrix product state simulator discarded correlations. Letting the middle cut keep more terms raised eight-round whole-state fidelity from 0.299 to 0.675. Peak additional heap was 1.14 MiB, and the complete run took 25 ms, including the pass that chose the caps. On the 256-qubit ladder, a 12 MiB tensor budget gave an echo probability of only 2.6 × 10⁻¹². Allowing the bond rank of 256 that this shallow circuit can require brought the return to one within 10⁻¹², in 5.14 seconds at 134 MiB peak process memory. Inserting a midpoint phase flip changed that return to 0.0002812, as an independent eight-qubit calculation predicted.

The audit includes the native runner, raw CSV, memory and time plots, and three reproducible limitations in numerical pruning and separability handling. The results quantify how circuit structure and acceptable approximation determine which simulations fit on a laptop.
