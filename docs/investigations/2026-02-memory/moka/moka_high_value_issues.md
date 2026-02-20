# High-Value Moka Issues: Mini-Moka & Comparisons

## 1. MiniArc Implementation (PR #456) ✅ MERGED
**Status:** Closed/Merged (2025-01-01)
**Link:** https://github.com/moka-rs/moka/pull/456

### Key Achievement
- Switched from `triomphe::Arc` to custom `MiniArc` implementation
- Reduces bloat by implementing minimal Arc specifically for moka's needs
- **Differences from triomphe::Arc:**
  - No `Weak` references (saves one counter)
  - Uses `AtomicU32` instead of `AtomicUsize` for strong ref counter
  - ~100 lines of code vs much larger triomphe
  - Only necessary methods for moka and mini-moka

### Impact
- Addresses dependency bloat concerns
- Custom implementation tailored to moka's specific use cases
- Potential for creating lightweight mini-moka variant

---

## 2. Bloat & Polymorphism Issue #203
**Status:** Open
**Created:** 2022-11-19
**Link:** https://github.com/moka-rs/moka/issues/203

### Problem
Moka generates excessive LLVM IR due to internal generics:
- Multiple copies of functions with different type parameters
- Hurts compile times
- Can lead to binary bloat
- May reduce instruction cache effectiveness

### Relevance to Mini-Moka
- **Direct motivation for creating mini-moka:** A lightweight version without the polymorphism overhead
- Example showing the problem: simple `Cache<usize, usize>` generates huge amount of IR

### Related Work
- Issue was identified using `cargo-llvm-lines`
- Connected to need for simplified, dependency-light variant

---

## 3. Performance Alternatives & Comparisons

### Issue #411: TinyUFO Alternative
**Status:** Open (2024-04-05)
**Link:** https://github.com/moka-rs/moka/issues/411

- Proposal to switch eviction strategy to TinyUFO for performance boost
- 2 comments discussing feasibility

### Issue #446: SIEVE LRU Alternative  
**Status:** Open (2024-07-21)
**Link:** https://github.com/moka-rs/moka/issues/446

- SIEVE advertised as simpler, better alternative to LRU (https://cachemon.github.io/SIEVE-website/)
- Maintains write-order naturally (no reordering needed)
- 4 comments on feasibility and access-order requirements
- Related to broader eviction strategy research

### Issue #385: seize vs crossbeam-epoch
**Status:** Open (2024-01-20) 
**Link:** https://github.com/moka-rs/moka/issues/385

- Long-standing issue (14 comments) with `crossbeam-epoch`
- Problem: No guarantee destructors will be executed
- Exploring `seize` crate as lighter-weight alternative
- **Key constraint:** crossbeam-epoch is used in lock-free concurrent hash table (cht)

---

## 4. Benchmark & Performance Infrastructure

### PR #550: Add benches
**Status:** Open
**Link:** https://github.com/moka-rs/moka/pull/550

- Active work on benchmarking infrastructure
- Critical for validating performance claims vs alternatives

### Issue #473: Memory Usage
**Status:** Open
**Link:** https://github.com/moka-rs/moka/issues/473

- "Keys occupy double memory size after updating"
- Direct relevance to mini-moka memory footprint goals

---

## 5. Design & Comparison Issues

### PR #516: Unsized Keys & str Support
**Status:** Open
**Link:** https://github.com/moka-rs/moka/pull/516

- Allow key type to be unsized
- Prefer `str` to `String` as key type
- Memory optimization for common use cases

### Issue #203: Polymorphism Bloat (repeated)
**Status:** Open
**Key Insight:** This is THE core motivator for mini-moka
- Shows moka generates too much monomorphic copies
- Mini-moka should reduce this through deliberate constraints

---

## Key Insights for Mini-Moka Strategy

### 1. **Lightweight Arc Implementation** ✅
   - MiniArc PR (#456) shows custom Arc is viable and merged
   - Reduces dependency on triomphe
   - Model for other lightweight optimizations

### 2. **Polymorphism as Root Cause** 
   - Issue #203 clearly documents the bloat problem
   - Mini-moka should constrain generics to reduce LLVM IR
   - Example: Fixed key/value types rather than arbitrary T

### 3. **Performance Comparison Landscape**
   - TinyUFO (#411): Eviction strategy alternative
   - SIEVE (#446): Emerging research-based algorithm
   - seize (#385): Lighter replacement for crossbeam-epoch
   - Each represents potential for "lighter weight = better" tradeoffs

### 4. **Memory Footprint**
   - Issue #473: Keys occupy double memory → optimization opportunity  
   - PR #516: str vs String matters → mini-moka should use optimized types
   - Custom Arc saves memory vs triomphe

### 5. **Benchmark Infrastructure Needed**
   - PR #550: Active work on benchmarks
   - Essential to validate mini-moka performance claims
   - Comparison with alternatives needed

---

## Dependency Chain for Mini-Moka Research

1. **Understand current bloat** → Issue #203
2. **Review MiniArc work** → PR #456 (merged approach)
3. **Study performance alternatives** → Issues #411, #446, #385
4. **Identify polymorphism constraints** → What can mini-moka sacrifice?
5. **Memory optimization targets** → Issue #473, PR #516
6. **Establish benchmarks** → PR #550
