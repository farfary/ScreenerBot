# Crossbeam Utils - Detailed Documentation

## Overview

Crossbeam Utils is a Rust crate providing miscellaneous tools for concurrent programming. It's part of the Crossbeam ecosystem and supports both `std` and `no_std` environments.

**Current Version:** 0.8+  
**Minimum Rust Version:** 1.56+  
**License:** MIT OR Apache-2.0

## Key Features

### Atomics
- **`AtomicCell<T>`** - Thread-safe mutable memory location (no_std compatible)
- **`AtomicConsume`** - Reading from primitive atomic types with "consume" ordering (no_std compatible)

### Thread Synchronization
- **`Parker`** - Thread parking primitive
- **`ShardedLock`** - Sharded reader-writer lock with fast concurrent reads
- **`WaitGroup`** - For synchronizing the beginning or end of computation

### Utilities
- **`Backoff`** - Exponential backoff in spin loops (no_std compatible)
- **`CachePadded<T>`** - Padding and aligning values to cache line length (no_std compatible)
- **`scope()`** - Spawning threads that borrow local variables from the stack

---

## Detailed Implementation

### 1. CachePadded<T>

**Purpose:** Prevents false sharing in concurrent code by padding values to cache line boundaries.

**Cache Line Sizes by Architecture:**
```
- x86-64, aarch64, arm64ec, powerpc64: 128 bytes
- arm, mips, mips32r6, mips64, mips64r6, sparc, hexagon: 32 bytes
- m68k: 16 bytes
- s390x: 256 bytes
- All others (x86, wasm, riscv, sparc64): 64 bytes
```

**Key Characteristics:**
- Uses Rust's `repr(align(...))` attribute for architecture-specific alignment
- Generic wrapper around any type `T`
- Implements `Deref` and `DerefMut` for transparent access
- Derives `Clone, Copy, Default, Hash, PartialEq, Eq`

**Use Cases:**
```rust
// Preventing cache line conflicts in concurrent queues
struct Queue<T> {
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
    buffer: *mut T,
}
```

**Why Intel 128-byte Alignment?**
Modern Intel processors (Sandy Bridge+) use spatial prefetcher that pulls pairs of 64-byte cache lines at once, requiring 128-byte alignment for optimal performance.

---

### 2. Backoff

**Purpose:** Implements exponential backoff strategy for spin loops to reduce contention and improve performance.

**Core Constants:**
```rust
const SPIN_LIMIT: u32 = 6;     // Maximum spins before yielding thread
const YIELD_LIMIT: u32 = 10;   // Maximum yields before backoff completes
```

**Internal State:**
- Tracks backoff step using `Cell<u32>` for interior mutability (no locking)

**Methods:**

#### `new() -> Self`
Creates a new backoff starting at step 0.

#### `reset()`
Resets backoff counter to 0. Useful for retrying operations.

#### `spin()`
Performs CPU-level spinning with PAUSE instructions.
- Executes `2^min(step, SPIN_LIMIT)` spin_loop() calls
- Each spin_loop() is a CPU PAUSE instruction (reduces power, helps hyperthreading)
- Increments step after each call
- Best for: Lock-free loops where another thread is actively working

**Implementation:**
```rust
pub fn spin(&self) {
    for _ in 0..1 << self.step.get().min(SPIN_LIMIT) {
        hint::spin_loop();  // CPU PAUSE instruction
    }
    
    if self.step.get() <= SPIN_LIMIT {
        self.step.set(self.step.get() + 1);
    }
}
```

#### `snooze()`
Performs thread-level yielding after initial spinning.
- Steps 0-6: CPU spin_loop() calls (exponential)
- Steps 7+: OS thread::yield_now() (gives up timeslice)
- Continues incrementing step up to YIELD_LIMIT (10)
- Best for: Waiting loops where thread may not be progressing

**Implementation Stages:**
1. **Spin phase** (steps 0-6): Use CPU PAUSE instructions
2. **Yield phase** (steps 7-10): Yield thread to OS scheduler
3. **Completed phase** (step > 10): Backoff complete, consider blocking

#### `is_completed() -> bool`
Returns true when backoff has completed (step > YIELD_LIMIT).
- Signals that blocking mechanisms (park, Condvar) should be used
- Prevents busy-waiting indefinitely

**Exponential Progression:**
```
Step 0: 1 operation    (2^0)
Step 1: 2 operations   (2^1)
Step 2: 4 operations   (2^2)
Step 3: 8 operations   (2^3)
...
Step 6: 64 operations  (2^6) [SPIN_LIMIT]
Step 7-10: OS yield_now()
Step 11+: Backoff completed
```

**Complete Example - Lock-free CAS Loop:**
```rust
use crossbeam_utils::Backoff;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

fn fetch_mul(a: &AtomicUsize, b: usize) -> usize {
    let backoff = Backoff::new();
    loop {
        let val = a.load(SeqCst);
        match a.compare_exchange(val, val.wrapping_mul(b), SeqCst, SeqCst) {
            Ok(_) => return val,
            Err(_) => backoff.spin(),  // Another thread succeeded, retry
        }
    }
}
```

**Complete Example - Parking with Backoff:**
```rust
use crossbeam_utils::Backoff;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::thread;

fn blocking_wait(ready: &AtomicBool) {
    let backoff = Backoff::new();
    while !ready.load(SeqCst) {
        if backoff.is_completed() {
            thread::park();  // Block until unparked
        } else {
            backoff.snooze();  // Exponential backoff
        }
    }
}
```

**Debug Output:**
```rust
impl fmt::Debug for Backoff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Backoff")
            .field("step", &self.step)
            .field("is_completed", &self.is_completed())
            .finish()
    }
}
```

---

### 3. AtomicCell<T>

**Purpose:** Thread-safe mutable memory location for any type, using atomics when possible and locks as fallback.

**Key Features:**
- Works with **any** type `T` (not just primitives)
- Automatically detects lock-free capability via `is_lock_free()`
- Uses `Acquire` ordering for loads and `Release` ordering for stores
- Safe even for non-Copy types through `swap()`
- Interior mutability via `AtomicCell<T>`

**Internal Implementation:**
```rust
#[repr(transparent)]
pub struct AtomicCell<T> {
    value: UnsafeCell<MaybeUninit<T>>,
}

// Safety traits
unsafe impl<T: Send> Send for AtomicCell<T> {}
unsafe impl<T: Send> Sync for AtomicCell<T> {}
impl<T> UnwindSafe for AtomicCell<T> {}
impl<T> RefUnwindSafe for AtomicCell<T> {}
```

**Why `MaybeUninit<T>`?**
- Prevents partially initialized state observation from outside the cell
- Works around rustc bugs related to uninitialized data
- Note: We never actually store uninitialized `T` due to API constraints

**Methods:**

#### `new(val: T) -> Self`
Creates new AtomicCell with const evaluation support.
```rust
pub const fn new(val: T) -> Self {
    Self {
        value: UnsafeCell::new(MaybeUninit::new(val)),
    }
}
```

#### `into_inner(self) -> T`
Consumes the cell and returns the value (only available when you own it).
```rust
pub fn into_inner(self) -> T {
    let this = ManuallyDrop::new(self);
    unsafe { this.as_ptr().read() }
}
```

#### `is_lock_free() -> bool`
Compile-time check: Returns true if operations use atomic instructions.
```rust
// Lock-free examples:
assert_eq!(AtomicCell::<usize>::is_lock_free(), true);  // Uses AtomicUsize
assert_eq!(AtomicCell::<()>::is_lock_free(), true);      // Zero-sized types
assert_eq!(AtomicCell::<[u8; 1000]>::is_lock_free(), false); // Uses global lock
```

#### `store(&self, val: T)`
Stores value, properly dropping previous value if needed.
```rust
pub fn store(&self, val: T) {
    if mem::needs_drop::<T>() {
        drop(self.swap(val));  // Drop old value
    } else {
        unsafe { atomic_store(self.as_ptr(), val); }
    }
}
```

#### `swap(&self, val: T) -> T`
Atomically stores value and returns the previous value.
```rust
pub fn swap(&self, val: T) -> T {
    unsafe { atomic_swap(self.as_ptr(), val) }
}
```

#### `take(&self) -> T` (when T: Default)
Takes value, replacing with default.
```rust
pub fn take(&self) -> T {
    self.swap(T::default())
}
```

#### `as_ptr(&self) -> *mut T`
Returns raw mutable pointer to underlying data.

**Lock-Free Types:**
The crate automatically detects and uses native atomics for:
- Primitive integer types (`u8`, `u16`, `u32`, `u64`, `usize`, etc.)
- Primitive float types (`f32`, `f64`)
- Wrapper types transmutable to primitives
- Zero-sized types `()`
- Types up to platform atomic word size

**Non-Lock-Free Fallback:**
For larger types, uses a global `SeqLock` (sequential lock):
- Readers don't block writers
- Writers acquire exclusive lock
- High-performance for read-dominated workloads

**Example Usage:**
```rust
use crossbeam_utils::atomic::AtomicCell;

let a = AtomicCell::new(7usize);
assert_eq!(a.load(), 7);

a.store(8);
assert_eq!(a.load(), 8);

let old = a.swap(9);
assert_eq!(old, 8);
assert_eq!(a.load(), 9);

// For non-Copy types
let cell = AtomicCell::new(vec![1, 2, 3]);
let old_vec = cell.swap(vec![4, 5, 6]);
assert_eq!(old_vec, vec![1, 2, 3]);
```

---

## Performance Characteristics

### CachePadded
- **Overhead:** One cache line of memory (~64-256 bytes depending on architecture)
- **Benefit:** Eliminates cache coherency traffic between cores
- **Best Use:** Frequently accessed shared data in concurrent structures

### Backoff
- **Spin Phase:** CPU-efficient, costs only CPU cycles
- **Yield Phase:** OS-friendly, reduces power consumption
- **Completion:** Signals need for blocking synchronization
- **Memory:** Minimal - single `Cell<u32>` (~4-8 bytes)

### AtomicCell
- **Lock-Free (native atomics):**
  - Latency: Single atomic instruction (~nanoseconds)
  - Throughput: Can operate in parallel across cores
  
- **Lock-Based (global SeqLock):**
  - Latency: Lock acquisition + sequential operation
  - Read performance: No writer blocking (optimistic reads)
  - Write performance: Exclusive lock required

---

## Memory Safety

### Unsafe Patterns
All tools properly handle:
- **Drop semantics:** Values are properly dropped
- **Uninitialized data:** Protected via `MaybeUninit<T>`
- **Alignment:** Maintained via `repr(align(...))`
- **Memory ordering:** Correct atomic orderings for synchronization

### Thread Safety Guarantees
- `Send` + `Sync` bounds properly enforced
- Panic safety (`UnwindSafe`, `RefUnwindSafe`)
- No data races under documented usage

---

## Architecture-Specific Optimizations

The crate uses compile-time `cfg` attributes to optimize for each platform:

```rust
// Conditional compilation for cache line alignment
#[cfg_attr(
    any(target_arch = "x86_64", target_arch = "aarch64", ...),
    repr(align(128))
)]
pub struct CachePadded<T> { ... }
```

This approach ensures:
- No runtime overhead
- Correct alignment for each architecture
- Works seamlessly across heterogeneous systems

---

## no_std Support

Several utilities work without `std`:
- `CachePadded<T>` - Pure memory alignment
- `Backoff` - CPU spin loops (no yield in no_std)
- `AtomicCell<T>` - Atomic operations only

This enables use in:
- Embedded systems
- Kernel modules
- WASM environments
- Real-time systems

---

## Common Patterns

### Pattern 1: False Sharing Prevention
```rust
struct Metrics {
    reads: CachePadded<AtomicUsize>,
    writes: CachePadded<AtomicUsize>,
}
// Each counter on separate cache line
```

### Pattern 2: Lock-Free Retry Loop
```rust
let backoff = Backoff::new();
loop {
    match try_operation() {
        Ok(result) => return result,
        Err(_) => backoff.spin(),  // Exponential backoff
    }
}
```

### Pattern 3: Blocking With Backoff
```rust
let backoff = Backoff::new();
while condition.load(SeqCst) == expected {
    if backoff.is_completed() {
        parked_thread.park();
    } else {
        backoff.snooze();
    }
}
```

### Pattern 4: Generic Atomic Storage
```rust
// Works with any type
let cell = AtomicCell::new(MyStruct {
    a: 42,
    b: "hello",
    c: vec![1, 2, 3],
});

// Check if lock-free
if AtomicCell::<MyStruct>::is_lock_free() {
    println!("Native atomic operations");
} else {
    println!("Using fallback locks");
}
```

---

## Repository Structure

**Source Files:**
- `crossbeam-utils/src/backoff.rs` (8.3 KB) - Backoff implementation
- `crossbeam-utils/src/cache_padded.rs` (7.5 KB) - Cache padding
- `crossbeam-utils/src/atomic/atomic_cell.rs` (42.4 KB) - AtomicCell with platform-specific code
- `crossbeam-utils/src/atomic/consume.rs` - Consume ordering support
- `crossbeam-utils/src/atomic/seq_lock.rs` - Sequential lock (fallback)
- `crossbeam-utils/src/atomic/seq_lock_wide.rs` - Wide sequential lock variant
- `crossbeam-utils/src/sync/` - Thread synchronization primitives
- `crossbeam-utils/src/thread.rs` - Scoped thread utilities

---

## Conclusion

Crossbeam Utils provides battle-tested, high-performance primitives for concurrent Rust programming. The key design principles are:

1. **Zero-cost abstractions** - No runtime overhead when not needed
2. **Platform-aware** - Architecture-specific optimizations
3. **Type-safe** - Works with any type, not just primitives
4. **Flexible** - Lock-free when possible, fallback to locks when necessary
5. **no_std friendly** - Core utilities work in restricted environments

This makes it invaluable for building efficient concurrent systems, from high-performance data structures to embedded networking code.
