# Epoch-Based Memory Reclamation Research

## Key Resources Found

### 1. Aaron Turon's Blog Post on Epoch-Based Reclamation
**URL:** https://aturon.github.io/tech/2015/08/27/epoch/
**Title:** Lock-freedom without garbage collection

**Key Points:**
- Describes epoch-based memory reclamation implementation in Crossbeam
- Shows how to achieve lock-free data structures without GC in Rust
- Provides performance benchmarks comparing to Java/Scala implementations
- Documents the Rust API (Guard, Owned, Shared pointers, Atomic)

**Performance Findings:**
- Crossbeam epoch implementation is competitive with Java GC implementations
- MPSC (multi-producer, single-consumer) test: Rust outperforms Java
- MPMC (multi-producer, multi-consumer) test: comparable performance
- Key advantage: garbage cost is proportional to number of threads, not data size
- Mutex baseline: 3040ns/operation (20x slower than Crossbeam for MPMC)

### 2. Crossbeam Repositories Found
- **crossbeam-rs/crossbeam:** Main tools library for concurrent programming
- **crossbeam-rs/crossbeam-epoch:** Epoch-based garbage collection (archived)
- **crossbeam-rs/crossbeam-channel:** Multi-producer multi-consumer channels (archived)
- **dgarvit/epoch-based-manager:** Epoch-based reclamation system for Chapel programming language
- **cmuparlay/verlib:** EBR implementation with benchmarks
- **huangjiahua/neatlib:** Epoch-based memory reclamation
- **ericseppanen/epoch_playground:** Learning crossbeam::epoch

### 3. References to Keir Fraser's Work
**Paper:** "Practical Lock-Freedom" (2004)
**Reference ID:** UCAM-CL-TR-579
**Author:** Keir Fraser
- Found in computer science literature databases
- Foundational work on lock-free algorithms and memory management

### 4. Hazard Pointers (Alternative Approach)
**Key Paper:** "Hazard Pointers: Safe Memory Reclamation for Lock-Free Objects"
**Author:** Maged Michael (2004)
**Published:** IEEE Transactions on Parallel and Distributed Systems, Vol. 15, Issue 8, pp. 491-504
**URL:** https://www.research.ibm.com/people/m/michael/ieeetpds-2004.pdf

**Comparison with Epoch-Based:**
- Both solve the same problem: safe memory reclamation in lock-free structures
- Hazard pointers require per-thread "hazard" records
- Epoch-based is simpler conceptually but requires epoch advancement coordination

### 5. Technical Implementations Found
- **Chapel language**: EBR with distributed memory support
- **Java**: Epoch implementation in cmuparlay/verlib with benchmarks
- **Rust**: Multiple implementations (Crossbeam primary)
- **Lock-free queue implementations**: Michael-Scott queue referenced (1996)
- **Lock-free stack**: Treiber's stack examples

## Key Concepts

### Epoch-Based Reclamation (EBR)
1. Threads enter "epochs" marking current time
2. Memory freed only when no thread may access it
3. Requires tracking when epochs advance
4. Simpler than hazard pointers but with epoch coordination overhead

### Memory Management API Components
- **Guard**: Represents membership in an epoch
- **Owned/Shared pointers**: Type-safe memory management
- **Atomic operations**: Lock-free updates
- **Garbage collection**: Deferred memory cleanup

## Performance Characteristics

### Advantages of Epoch-Based:
- Garbage cost scales with number of threads, not data size
- Simpler API than hazard pointers
- Type-safe in languages like Rust
- Predictable performance

### Trade-offs:
- Requires epoch advancement mechanism
- All threads must participate in epoch management
- RCU (Read-Copy-Update) is even simpler for read-heavy workloads

## Further Research Needed
- Direct comparison: Epoch vs. Hazard Pointers vs. RCU
- Performance on different workload patterns
- NUMA considerations
- Modern CPU cache effects
