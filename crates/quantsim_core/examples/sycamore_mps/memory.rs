use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

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
        let live = self.live.fetch_add(bytes, Ordering::Relaxed) + bytes;
        self.peak.fetch_max(live, Ordering::Relaxed);
    }
}

// SAFETY: All operations forward their layout and pointer to System unchanged.
// The accounting uses only atomics and allocates no memory.
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
        self.live.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let ptr = unsafe { System.realloc(ptr, layout, size) };
        if !ptr.is_null() {
            if size >= layout.size() {
                self.add(size - layout.size());
            } else {
                self.live.fetch_sub(layout.size() - size, Ordering::Relaxed);
            }
        }
        ptr
    }
}

static MEASURING: AtomicBool = AtomicBool::new(false);

struct Measurement;

impl Drop for Measurement {
    fn drop(&mut self) {
        MEASURING.store(false, Ordering::Release);
    }
}

/// Requested Rust heap bytes attributable to one backend, including its
/// retained state and transient workspace. Excludes stack and allocator overhead.
#[derive(Default)]
pub struct BackendHeap {
    /// Bytes retained at the end of the last measured scope.
    pub live: usize,
    /// Peak bytes during the last measured scope, including its starting state.
    pub peak: usize,
}

impl BackendHeap {
    /// Measure backend initialization or one round. Allocate, resize and drop
    /// the backend's buffers inside its scopes, and run reporting outside.
    /// This meter is process-wide. Concurrent unrelated allocations would be
    /// counted too. Overlapping scopes panic. Discard this tracker if work panics.
    pub fn measure<T>(&mut self, work: impl FnOnce() -> T) -> T {
        assert!(
            !MEASURING.swap(true, Ordering::Acquire),
            "heap measurements cannot overlap"
        );
        let _measurement = Measurement;
        let baseline = HEAP.live.load(Ordering::Relaxed);
        HEAP.peak.store(baseline, Ordering::Relaxed);
        let result = work();
        let end = HEAP.live.load(Ordering::Relaxed);
        self.peak = self.live + (HEAP.peak.load(Ordering::Relaxed) - baseline);
        self.live = if end >= baseline {
            self.live + (end - baseline)
        } else {
            self.live
                .checked_sub(baseline - end)
                .expect("scope freed more heap than the backend owned")
        };
        result
    }
}
