use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::spin_loop;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

const INACTIVE: usize = 0;
const INITIALISING: usize = 1;
const ACTIVE: usize = 2;

struct ObservedSystemAllocator;

#[global_allocator]
static GLOBAL_ALLOCATOR: ObservedSystemAllocator = ObservedSystemAllocator;

static STATE: AtomicUsize = AtomicUsize::new(INACTIVE);
static IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
static OVERFLOWED: AtomicBool = AtomicBool::new(false);
static ALLOCATION_CALLS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static DEALLOCATION_CALLS: AtomicU64 = AtomicU64::new(0);
static DEALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static REALLOCATION_CALLS: AtomicU64 = AtomicU64::new(0);
static REALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

fn checked_add(counter: &AtomicU64, amount: usize) {
    match u64::try_from(amount) {
        Ok(amount) => {
            if counter
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    value.checked_add(amount)
                })
                .is_err()
            {
                OVERFLOWED.store(true, Ordering::Relaxed);
            }
        }
        Err(_) => OVERFLOWED.store(true, Ordering::Relaxed),
    }
}

fn record(call_counter: &AtomicU64, byte_counter: &AtomicU64, size: usize) {
    if STATE.load(Ordering::Relaxed) != ACTIVE {
        return;
    }
    IN_FLIGHT.fetch_add(1, Ordering::Acquire);
    if STATE.load(Ordering::Acquire) == ACTIVE {
        checked_add(call_counter, 1);
        checked_add(byte_counter, size);
    }
    IN_FLIGHT.fetch_sub(1, Ordering::Release);
}

fn record_allocation(size: usize) {
    record(&ALLOCATION_CALLS, &ALLOCATED_BYTES, size);
}

fn record_deallocation(size: usize) {
    record(&DEALLOCATION_CALLS, &DEALLOCATED_BYTES, size);
}

fn record_reallocation(size: usize) {
    record(&REALLOCATION_CALLS, &REALLOCATED_BYTES, size);
}

// SAFETY: every operation delegates to `System` with the original pointer and
// layout contract. Successful calls only update non-allocating atomic counters.
unsafe impl GlobalAlloc for ObservedSystemAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller supplies the `GlobalAlloc` layout contract.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller supplies the `GlobalAlloc` layout contract.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_deallocation(layout.size());
        // SAFETY: the caller supplies the original pointer and layout contract.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller supplies the original pointer, layout, and new size.
        let replacement = unsafe { System.realloc(pointer, layout, new_size) };
        if !replacement.is_null() {
            record_reallocation(new_size);
        }
        replacement
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AllocationObservation {
    pub(crate) allocation_calls: u64,
    pub(crate) allocated_bytes: u64,
    pub(crate) deallocation_calls: u64,
    pub(crate) deallocated_bytes: u64,
    pub(crate) reallocation_calls: u64,
    pub(crate) reallocated_bytes: u64,
}

#[derive(Debug)]
pub(crate) struct AllocationObservationGuard {
    active: bool,
}

impl AllocationObservationGuard {
    pub(crate) fn start() -> Result<Self, String> {
        STATE
            .compare_exchange(INACTIVE, INITIALISING, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "allocator observation is already active".to_string())?;
        ALLOCATION_CALLS.store(0, Ordering::Relaxed);
        ALLOCATED_BYTES.store(0, Ordering::Relaxed);
        DEALLOCATION_CALLS.store(0, Ordering::Relaxed);
        DEALLOCATED_BYTES.store(0, Ordering::Relaxed);
        REALLOCATION_CALLS.store(0, Ordering::Relaxed);
        REALLOCATED_BYTES.store(0, Ordering::Relaxed);
        OVERFLOWED.store(false, Ordering::Relaxed);
        STATE.store(ACTIVE, Ordering::Release);
        Ok(Self { active: true })
    }

    pub(crate) fn finish(mut self) -> Result<AllocationObservation, String> {
        self.stop();
        if OVERFLOWED.load(Ordering::Relaxed) {
            return Err("allocator observation counter overflow".to_string());
        }
        Ok(AllocationObservation {
            allocation_calls: ALLOCATION_CALLS.load(Ordering::Relaxed),
            allocated_bytes: ALLOCATED_BYTES.load(Ordering::Relaxed),
            deallocation_calls: DEALLOCATION_CALLS.load(Ordering::Relaxed),
            deallocated_bytes: DEALLOCATED_BYTES.load(Ordering::Relaxed),
            reallocation_calls: REALLOCATION_CALLS.load(Ordering::Relaxed),
            reallocated_bytes: REALLOCATED_BYTES.load(Ordering::Relaxed),
        })
    }

    fn stop(&mut self) {
        if !self.active {
            return;
        }
        STATE.store(INACTIVE, Ordering::Release);
        while IN_FLIGHT.load(Ordering::Acquire) != 0 {
            spin_loop();
        }
        self.active = false;
    }
}

impl Drop for AllocationObservationGuard {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_counts_allocations_and_restores_inactive_state() {
        let guard = AllocationObservationGuard::start().unwrap();
        let buffer = vec![0u8; 4096];
        let observation = guard.finish().unwrap();
        assert!(observation.allocation_calls >= 1);
        assert!(observation.allocated_bytes >= 4096);
        drop(buffer);

        let guard = AllocationObservationGuard::start().unwrap();
        assert!(AllocationObservationGuard::start().is_err());
        drop(guard);
        AllocationObservationGuard::start()
            .unwrap()
            .finish()
            .unwrap();
    }
}
