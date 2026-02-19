# Epoch-Based Memory Reclamation - Research Index

## 📚 Overview

This research directory contains comprehensive information about epoch-based memory reclamation, a technique used for safe memory management in lock-free concurrent data structures.

## 📄 Documents in This Package

### 1. **research_summary.txt** ⭐ START HERE
Complete overview of the entire research, including:
- Key academic papers and references
- Implementation repositories
- Performance benchmarks
- Comparisons with alternatives
- Real-world usage examples
- Implementation considerations

**Best for:** Getting a complete picture of epoch-based reclamation in one document.

### 2. **epoch_research.md**
Structured research findings from GitHub and web searches:
- Aaron Turon's blog post summary
- Crossbeam repository discoveries
- Keir Fraser's paper references
- Hazard pointers alternative
- Technical implementations across languages

**Best for:** Quick lookup of specific resources and repositories.

### 3. **implementation_guide.md**
Practical guide for understanding and implementing epoch-based reclamation:
- Problem statement and core concepts
- Component breakdown
- Detailed comparison tables (vs. Hazard Pointers, RCU, GC)
- Use cases and anti-patterns
- Implementation checklist

**Best for:** Learning how to implement or use epoch-based reclamation.

### 4. **aturon_epoch_post.html**
Full text of Aaron Turon's seminal blog post:
- Introduction to lock-free data structures
- Epoch-based reclamation algorithm explanation
- Rust API design patterns
- Performance benchmarks with code examples
- Treiber's stack implementation walkthrough

**Best for:** Deep dive into practical Rust implementation and design rationale.

## 🎯 Quick Reference

### Key Figures
- **Keir Fraser**: "Practical Lock-Freedom" (2004)
- **Maged Michael**: "Hazard Pointers" (2004) - alternative approach
- **Aaron Turon**: Crossbeam epoch-based reclamation (2015) - practical Rust impl

### Key Papers
1. Keir Fraser - Practical Lock-Freedom (UCAM-CL-TR-579)
2. Maged Michael - Hazard Pointers (IEEE TPDS Vol 15, Issue 8)
3. Michael & Scott - Queue Algorithms (PODC 1996)

### Production Implementations
- **Rust**: Crossbeam (https://github.com/crossbeam-rs/crossbeam)
- **Chapel**: dgarvit/epoch-based-manager
- **Java**: cmuparlay/verlib

### Performance Summary
- MPSC: ~200-400 ns/operation (Rust with epoch vs GC)
- MPMC: ~400-600 ns/operation (competitive with Java GC)
- Mutex baseline: ~3040 ns/operation (20x slower)

## 🔍 Research Findings

### What is Epoch-Based Reclamation?
A memory management technique where:
1. Time is divided into discrete "epochs"
2. Threads announce their current epoch
3. Memory is freed only when no thread needs it
4. No pause times (unlike GC) - deterministic cleanup

### When to Use It
✅ Lock-free data structures
✅ Systems with bounded threads
✅ Real-time systems (need deterministic latency)
✅ High-performance concurrent applications

### When NOT to Use It
❌ Simple reference counting
❌ Purely read-heavy workloads (use RCU instead)
❌ Dynamic thread spawning
❌ Memory allocation hot-spot

## 🔗 External Resources

### Papers
- Keir Fraser's "Practical Lock-Freedom": https://www.cl.cam.ac.uk/
- Maged Michael's "Hazard Pointers": https://www.research.ibm.com/people/m/michael/ieeetpds-2004.pdf

### Blogs & Articles
- Aaron Turon: https://aturon.github.io/tech/2015/08/27/epoch/
- Aaron Turon's Tech Blog: https://aturon.github.io/

### Code Repositories
- Crossbeam (Rust): https://github.com/crossbeam-rs/crossbeam
- Crossbeam Epoch Docs: https://docs.rs/crossbeam-epoch/latest/crossbeam_epoch/
- Example Learning: https://github.com/ericseppanen/epoch_playground

## 📊 Comparison Matrix

| Aspect | Epoch | Hazard Pointers | RCU | GC |
|--------|-------|-----------------|-----|-----|
| Simplicity | ★★★★ | ★★★ | ★★★★★ | ★★★★★ |
| Per-thread overhead | Low | High | Medium | Varies |
| Cleanup cost | O(threads) | O(threads×pointers) | O(threads) | Variable |
| API ease | Simple | Complex | Very simple | Transparent |
| Best for | Lock-free | Complex pointers | Read-heavy | General |
| Latency | Deterministic | Deterministic | Deterministic | Variable |

## 🚀 Getting Started

1. **Want a quick overview?** → Read `research_summary.txt`
2. **Want to understand the algorithm?** → Read `aturon_epoch_post.html`
3. **Want to implement it?** → Read `implementation_guide.md`
4. **Want specific repositories?** → Check `epoch_research.md`

## 📝 Notes

- Research completed: February 2026
- GitHub search hits: 6+ direct implementations found
- Academic papers: 4 foundational papers identified
- Rust implementation maturity: Production-ready (Crossbeam)
- Performance data: Benchmarked on 4-core i7 (2015)

## 🔄 Next Steps

1. Study the Crossbeam source code
2. Review comparison with hazard pointer implementations
3. Run benchmarks on your specific hardware/workload
4. Consider NUMA-aware variants for large systems
5. Explore advanced patterns in distributed systems

---

**Research completed by**: GitHub + curl research sweep
**Last updated**: February 19, 2026
**Status**: ✅ Complete with high-quality resources found
