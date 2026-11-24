use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

struct Meter;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: Meter = Meter;

fn add(bytes: usize) {
    let now = LIVE.fetch_add(bytes, Relaxed) + bytes;
    PEAK.fetch_max(now, Relaxed);
}

// SAFETY: Allocation operations forward their inputs to System unchanged.
// Atomics record requested live bytes without allocating.
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            add(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            add(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, bytes: usize) -> *mut u8 {
        let result = unsafe { System.realloc(ptr, layout, bytes) };
        if !result.is_null() {
            if bytes >= layout.size() {
                add(bytes - layout.size());
            } else {
                LIVE.fetch_sub(layout.size() - bytes, Relaxed);
            }
        }
        result
    }
}

pub fn start() -> usize {
    let baseline = LIVE.load(Relaxed);
    PEAK.store(baseline, Relaxed);
    baseline
}

pub fn peak() -> usize {
    PEAK.load(Relaxed)
}
