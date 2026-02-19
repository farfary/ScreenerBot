// =============================================================================
// Practical Examples: Arc Cache Line and Alignment
// =============================================================================

use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicUsize, Ordering::*};
use std::thread;
use std::time::Instant;

// =============================================================================
// EXAMPLE 1: Basic Arc Usage and Ordering
// =============================================================================

fn example_1_basic_arc() {
    println!("\n=== Example 1: Basic Arc Usage ===\n");
    
    let data = Arc::new(vec![1, 2, 3, 4, 5]);
    println!("Created Arc with data: {:?}", *data);
    
    // Clone Arc (cheap, Relaxed ordering)
    let data_clone = Arc::clone(&data);
    println!("Cloned Arc: {:?}", *data_clone);
    
    // Both point to same allocation
    println!("Same allocation: {}", Arc::as_ptr(&data) == Arc::as_ptr(&data_clone));
    
    // Drop is Release-Acquire synchronized
    drop(data_clone);
    println!("After drop, original still valid: {:?}", *data);
}

// =============================================================================
// EXAMPLE 2: False Sharing Demonstration
// =============================================================================

// ❌ BAD: Both counters on same cache line (potential false sharing)
struct PoorDesignCounters {
    head: AtomicUsize,
    tail: AtomicUsize,
}

// ✅ GOOD: Padding prevents false sharing
struct BetterDesignCounters {
    head: AtomicUsize,
    _head_padding: [u8; 56],  // Pad to 64 bytes (typical cache line)
    tail: AtomicUsize,
    _tail_padding: [u8; 56],
}

fn example_2_false_sharing() {
    println!("\n=== Example 2: False Sharing Impact ===\n");
    
    // Demonstrate false sharing impact
    println!("PoorDesignCounters size: {} bytes", std::mem::size_of::<PoorDesignCounters>());
    println!("  Both head and tail likely on SAME 64-byte cache line");
    println!("  Concurrent access = cache line ping-pong\n");
    
    println!("BetterDesignCounters size: {} bytes", std::mem::size_of::<BetterDesignCounters>());
    println!("  head on cache line 0, tail on cache line 1");
    println!("  Independent access = better performance\n");
    
    // Quick benchmark
    let poor = Arc::new(PoorDesignCounters {
        head: AtomicUsize::new(0),
        tail: AtomicUsize::new(0),
    });
    
    let start = Instant::now();
    let mut handles = vec![];
    
    for i in 0..4 {
        let poor_clone = Arc::clone(&poor);
        handles.push(thread::spawn(move || {
            for _ in 0..100_000 {
                if i % 2 == 0 {
                    poor_clone.head.fetch_add(1, Relaxed);
                } else {
                    poor_clone.tail.fetch_add(1, Relaxed);
                }
            }
        }));
    }
    
    for h in handles {
        h.join().unwrap();
    }
    
    let elapsed = start.elapsed();
    println!("Time for 4 threads × 100K ops on PoorDesign: {:?}\n", elapsed);
}

// =============================================================================
// EXAMPLE 3: Arc with Interior Mutability
// =============================================================================

fn example_3_interior_mutability() {
    println!("\n=== Example 3: Arc + Mutex for Interior Mutability ===\n");
    
    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];
    
    for _ in 0..4 {
        let counter_clone = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                // Acquire lock, update, release lock
                let mut val = counter_clone.lock().unwrap();
                *val += 1;
                // Lock automatically released here
            }
        }));
    }
    
    for h in handles {
        h.join().unwrap();
    }
    
    println!("Final counter value: {}", counter.lock().unwrap());
    println!("Expected: 4000 (4 threads × 1000 increments)\n");
}

// =============================================================================
// EXAMPLE 4: Arc with AtomicUsize (Better for High Frequency)
// =============================================================================

fn example_4_atomic_counter() {
    println!("\n=== Example 4: Arc + AtomicUsize (Better Performance) ===\n");
    
    let counter = Arc::new(AtomicUsize::new(0));
    let start = Instant::now();
    let mut handles = vec![];
    
    for _ in 0..4 {
        let counter_clone = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                // Direct atomic operation, no lock acquisition
                counter_clone.fetch_add(1, Relaxed);
            }
        }));
    }
    
    for h in handles {
        h.join().unwrap();
    }
    
    let elapsed = start.elapsed();
    println!("Final counter value: {}", counter.load(Relaxed));
    println!("Time for 4 threads × 1000 ops: {:?}\n", elapsed);
}

// =============================================================================
// EXAMPLE 5: Understanding Ordering - Clone vs Drop
// =============================================================================

fn example_5_ordering_semantics() {
    println!("\n=== Example 5: Ordering Semantics ===\n");
    
    println!("Arc::clone() uses Relaxed ordering:");
    println!("  let arc = Arc::new(data);");
    println!("  let clone = Arc::clone(&arc);  // fetch_add(1, Relaxed)");
    println!("  - Why safe? Thread holding reference prevents deletion");
    println!("  - Cost: ~1-3 cycles (no fence needed)\n");
    
    println!("Arc::drop() uses Release ordering:");
    println!("  drop(arc);  // fetch_sub(1, Release)");
    println!("  - If refcount reaches 0, an Acquire fence is inserted");
    println!("  - Why? Ensures data access happens-before data deletion");
    println!("  - Cost: 2-4 cycles for fetch_sub + 10-15 cycles for fence\n");
    
    println!("Synchronization chain:");
    println!("  Thread A: ... modify data ... Arc::drop(Release)");
    println!("           └─────────────────────┬─────────────────");
    println!("  Thread B:                       Arc::upgrade(Acquire)");
    println!("           ┌─────────────────────┴─────────────────");
    println!("           └─> ... use data safely ...\n");
}

// =============================================================================
// EXAMPLE 6: Make Mut - Copy-on-Write Pattern
// =============================================================================

fn example_6_make_mut() {
    println!("\n=== Example 6: Arc::make_mut - Copy-on-Write ===\n");
    
    let mut data = Arc::new(vec![1, 2, 3]);
    println!("Initial: {:?}", *data);
    
    // First modification (no clone, single reference)
    Arc::make_mut(&mut data).push(4);
    println!("After first push: {:?}", *data);
    
    // Clone the arc
    let data2 = Arc::clone(&data);
    println!("Cloned arc");
    
    // Second modification (clones because 2 references)
    Arc::make_mut(&mut data).push(5);
    println!("After push with shared ref: {:?}", *data);
    println!("Other reference unchanged: {:?}", *data2);
    
    // No longer shared
    drop(data2);
    
    // Third modification (no clone again, single reference)
    Arc::make_mut(&mut data).push(6);
    println!("After final push: {:?}", *data);
}

// =============================================================================
// EXAMPLE 7: Cache Padding Example (using manual struct)
// =============================================================================

// Simulating crossbeam_utils::CachePadded behavior
#[repr(align(64))]
struct CachePadded<T> {
    data: T,
}

impl<T> CachePadded<T> {
    fn new(data: T) -> Self {
        CachePadded { data }
    }
    
    fn load(&self) -> T where T: Copy {
        self.data
    }
}

fn example_7_cache_padding() {
    println!("\n=== Example 7: Cache Padding ===\n");
    
    let padded = CachePadded::new(AtomicUsize::new(0));
    println!("CachePadded<T> alignment: {} bytes", std::mem::align_of_val(&padded));
    println!("Size: {} bytes", std::mem::size_of_val(&padded));
    
    println!("\nWith CachePadded:");
    println!("  Each atomic guaranteed on separate cache line (64 bytes)");
    println!("  No false sharing between independent threads\n");
}

// =============================================================================
// EXAMPLE 8: Performance Comparison - Clone Operations
// =============================================================================

fn example_8_clone_performance() {
    println!("\n=== Example 8: Clone Performance ===\n");
    
    let data = Arc::new(vec![0; 1000]);
    
    // Benchmark clones
    let start = Instant::now();
    for _ in 0..1_000_000 {
        let _ = Arc::clone(&data);
    }
    let elapsed = start.elapsed();
    
    println!("1,000,000 Arc::clone() operations: {:?}", elapsed);
    println!("Average per clone: {:.2} nanoseconds\n", 
        elapsed.as_nanos() as f64 / 1_000_000.0);
}

// =============================================================================
// EXAMPLE 9: Weak References and Upgrade
// =============================================================================

fn example_9_weak_references() {
    println!("\n=== Example 9: Weak References ===\n");
    
    let strong = Arc::new("Hello, World!".to_string());
    let weak = Arc::downgrade(&strong);
    
    println!("Strong refs: {}", Arc::strong_count(&strong));
    println!("Weak refs: {}", Arc::weak_count(&strong));
    
    // Upgrade weak to strong
    match weak.upgrade() {
        Some(upgraded) => {
            println!("Successfully upgraded weak ref: {}", upgraded);
            println!("Strong refs now: {}", Arc::strong_count(&strong));
        }
        None => println!("Could not upgrade (original dropped)"),
    }
    
    // Drop strong
    drop(strong);
    
    // Upgrade fails after original is dropped
    match weak.upgrade() {
        Some(_) => println!("Unexpectedly upgraded!"),
        None => println!("Upgrade failed as expected (original dropped)"),
    }
}

// =============================================================================
// EXAMPLE 10: Atomic Ordering Comparison
// =============================================================================

fn example_10_ordering_comparison() {
    println!("\n=== Example 10: Atomic Ordering Comparison ===\n");
    
    println!("Relaxed:");
    println!("  - No memory barriers");
    println!("  - Cost: Minimal (1-3 cycles)");
    println!("  - Use: When reference existence prevents use-after-free\n");
    
    println!("Release:");
    println!("  - Store fence (prevents prior ops reordering after)");
    println!("  - Cost: ~2-4 cycles base + 0 (no fence on this side)");
    println!("  - Use: Releasing synchronization\n");
    
    println!("Acquire:");
    println!("  - Load fence (prevents subsequent ops reordering before)");
    println!("  - Cost: ~2-4 cycles base + 0 (no fence on this side)");
    println!("  - Use: Acquiring synchronization\n");
    
    println!("Release-Acquire Fence:");
    println!("  - Both barriers");
    println!("  - Cost: ~10-15 cycles (full memory barrier)");
    println!("  - Use: Ensuring all prior memory is visible before deletion\n");
    
    let atomic = Arc::new(AtomicUsize::new(0));
    
    // Relaxed (cheap)
    let start = Instant::now();
    for _ in 0..1_000_000 {
        atomic.fetch_add(1, Relaxed);
    }
    let relaxed_time = start.elapsed();
    
    println!("1M fetch_add(Relaxed): {:?}", relaxed_time);
    println!("Per operation: {:.2} ns\n", 
        relaxed_time.as_nanos() as f64 / 1_000_000.0);
}

// =============================================================================
// EXAMPLE 11: High Contention Scenario
// =============================================================================

fn example_11_contention() {
    println!("\n=== Example 11: High Contention Impact ===\n");
    
    let counter = Arc::new(AtomicUsize::new(0));
    let start = Instant::now();
    let mut handles = vec![];
    
    // Spawn 8 threads all modifying same atomic
    for _ in 0..8 {
        let counter_clone = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..100_000 {
                counter_clone.fetch_add(1, Relaxed);
            }
        }));
    }
    
    for h in handles {
        h.join().unwrap();
    }
    
    let elapsed = start.elapsed();
    println!("8 threads × 100K operations on single AtomicUsize:");
    println!("  Time: {:?}", elapsed);
    println!("  Final value: {}", counter.load(Relaxed));
    println!("  Note: High contention causes atomic bus locks\n");
}

// =============================================================================
// EXAMPLE 12: Layout Analysis
// =============================================================================

fn example_12_layout_analysis() {
    println!("\n=== Example 12: Arc Layout Analysis ===\n");
    
    struct MyData {
        x: u64,
        y: u64,
    }
    
    let arc: Arc<MyData> = Arc::new(MyData { x: 1, y: 2 });
    
    println!("Arc<MyData> pointer size: {} bytes", std::mem::size_of_val(&arc));
    println!("MyData size: {} bytes", std::mem::size_of::<MyData>());
    
    println!("\nHeap allocation layout:");
    println!("  Offset 0-7:   strong counter (Atomic<usize>)");
    println!("  Offset 8-15:  weak counter (Atomic<usize>)");
    println!("  Offset 16-24: x (u64)");
    println!("  Offset 24-32: y (u64)\n");
    
    // Get actual pointer
    let ptr = Arc::as_ptr(&arc);
    println!("Arc points to: {:p}", ptr);
}

// =============================================================================
// Main
// =============================================================================

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║   Rust Arc: Cache Line & Alignment - Practical Examples      ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    
    example_1_basic_arc();
    example_2_false_sharing();
    example_3_interior_mutability();
    example_4_atomic_counter();
    example_5_ordering_semantics();
    example_6_make_mut();
    example_7_cache_padding();
    example_8_clone_performance();
    example_9_weak_references();
    example_10_ordering_comparison();
    example_11_contention();
    example_12_layout_analysis();
    
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║   Examples Complete                                           ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");
}

// =============================================================================
// Additional: ThreadSanitizer Note
// =============================================================================

/*
NOTE: ThreadSanitizer Compatibility

The Rust Arc implementation includes special handling for ThreadSanitizer:

```rust
#[cfg(not(sanitize = "thread"))]
macro_rules! acquire {
    ($x:expr) => {
        atomic::fence(Acquire)
    };
}

#[cfg(sanitize = "thread")]
macro_rules! acquire {
    ($x:expr) => {
        $x.load(Acquire)
    };
}
```

Why? ThreadSanitizer doesn't support memory fences. Instead of using 
atomic::fence(Acquire), it uses a load(Acquire) for synchronization.

This shows how Rust adapts atomic patterns to different runtime environments.
*/
