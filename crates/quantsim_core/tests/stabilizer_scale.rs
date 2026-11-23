#![cfg(not(target_arch = "wasm32"))]

use quantsim_core::{
    circuit::Circuit,
    types::{Gate, Operation},
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

// This integration-test binary has one test so unrelated tests do not contribute
// allocations during the measured interval. Count live requested heap bytes.
struct HeapMeter {
    live: AtomicUsize,
    peak: AtomicUsize,
}

#[global_allocator]
static HEAP: HeapMeter = HeapMeter {
    live: AtomicUsize::new(0),
    peak: AtomicUsize::new(0),
};

impl HeapMeter {
    fn add(&self, bytes: usize) {
        let live = self.live.fetch_add(bytes, Relaxed) + bytes;
        self.peak.fetch_max(live, Relaxed);
    }
}

// SAFETY: Each operation forwards its pointer and layout unchanged to System.
// The accounting uses only atomics and performs no allocations.
unsafe impl GlobalAlloc for HeapMeter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            self.add(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            self.add(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.live.fetch_sub(layout.size(), Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let ptr = unsafe { System.realloc(ptr, layout, size) };
        if !ptr.is_null() {
            if size >= layout.size() {
                self.add(size - layout.size());
            } else {
                self.live.fetch_sub(layout.size() - size, Relaxed);
            }
        }
        ptr
    }
}

const WIRES: usize = 50;
const SHOTS: u32 = 1_024;
const SEED: u64 = 42;

fn push(circuit: &mut Circuit, gate: Gate, wires: &[u32]) {
    circuit
        .steps
        .push(vec![Operation::new(gate, wires.to_vec(), vec![])]);
}

#[test]
fn wide_stabilizers_preserve_correlations_with_bounded_heap() {
    let mut ghz = Circuit::new(WIRES);
    push(&mut ghz, Gate::H, &[0]);
    for wire in 1..WIRES as u32 {
        push(&mut ghz, Gate::CX, &[0, wire]);
    }

    // Measure evolution, all wire readouts and all returned samples above the
    // live heap after circuit construction. The sampler's workspace is included.
    let baseline = HEAP.live.load(Relaxed);
    HEAP.peak.store(baseline, Relaxed);
    let state = ghz.run_stabilizer_prefix(WIRES).unwrap();
    let marginals: Vec<_> = (0..WIRES)
        .map(|wire| state.probability_one(wire).unwrap())
        .collect();
    let samples = state.sample(SHOTS, SEED);
    let peak = HEAP.peak.load(Relaxed) - baseline;

    eprintln!(
        "GHZ50, {WIRES} marginals and {SHOTS} packed shots: {peak} peak additional heap bytes"
    );
    assert!(peak >= std::mem::size_of_val(samples.as_slice()));
    assert!(peak < 1024 * 1024, "peak additional heap: {peak} bytes");
    assert!(marginals.iter().all(|&p| p == 0.5));
    let all_ones = (1u64 << WIRES) - 1;
    assert_eq!(samples.len(), SHOTS as usize);
    assert!(samples.iter().all(|&bits| bits == 0 || bits == all_ones));
    assert!(samples.contains(&0) && samples.contains(&all_ones));
    assert_eq!(state.sample(SHOTS, SEED), samples);

    for step in 0..=WIRES {
        let state = ghz.run_stabilizer_prefix(step).unwrap();
        for wire in 0..WIRES {
            let expected = if wire < step { 0.5 } else { 0.0 };
            assert_eq!(
                state.probability_one(wire),
                Some(expected),
                "step {step}, q{wire}"
            );
        }
    }

    // Z readouts alone cannot distinguish coherent GHZ from a classical mixture.
    for wire in (1..WIRES as u32).rev() {
        push(&mut ghz, Gate::CX, &[0, wire]);
    }
    push(&mut ghz, Gate::H, &[0]);
    let reversed = ghz.run_stabilizer_prefix(ghz.steps.len()).unwrap();
    assert!((0..WIRES).all(|wire| reversed.probability_one(wire) == Some(0.0)));
    assert!(reversed.sample(8, SEED).iter().all(|&bits| bits == 0));

    let mut independent = Circuit::new(WIRES);
    independent.steps.push(
        (0..WIRES as u32)
            .map(|wire| Operation::new(Gate::H, vec![wire], vec![]))
            .collect(),
    );
    let state = independent.run_stabilizer_prefix(1).unwrap();
    assert!((0..WIRES).all(|wire| state.probability_one(wire) == Some(0.5)));
    let mut samples = state.sample(SHOTS, SEED);
    assert!(samples.iter().all(|&bits| bits <= all_ones));
    samples.sort_unstable();
    samples.dedup();
    assert!(
        samples.len() >= 1000,
        "only {} distinct outcomes",
        samples.len()
    );

    let mut top_bit = Circuit::new(64);
    push(&mut top_bit, Gate::X, &[63]);
    let state = top_bit.run_stabilizer_prefix(1).unwrap();
    assert_eq!(state.probability_one(63), Some(1.0));
    assert_eq!(state.probability_one(64), None);
    assert_eq!(state.sample(1, SEED), vec![1u64 << 63]);
}
