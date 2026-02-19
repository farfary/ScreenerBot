# Arc Atomic Operations: Architecture-Specific Implementation Details

## Table of Contents
1. [x86-64 Architecture](#x86-64-architecture)
2. [ARM64 Architecture](#arm64-architecture)
3. [RISC-V Architecture](#risc-v-architecture)
4. [Comparison Matrix](#comparison-matrix)

---

## x86-64 Architecture

### Memory Model
- **Memory Consistency:** Total Store Order (TSO)
- **Cache Line Size:** 64 bytes (typical)
- **Atomic Alignment:** Natural alignment (8 bytes for u64)

### Atomic Operations Implementation

#### Relaxed Load (no barriers)
```asm
mov rax, [rdi]           ; Simple memory load
```
**Cost:** 1-3 cycles (L1 hit), 10-100+ cycles (cache miss)

#### Relaxed Store (no barriers)
```asm
mov [rdi], rax           ; Simple memory store
```
**Cost:** 1-3 cycles (L1 hit), 10-100+ cycles (cache miss)

#### Release Store (store fence)
```asm
mov [rdi], rax           ; Store with implicit ordering
; x86-64 stores are already ordered, so no explicit fence needed
```
**Cost:** ~2-4 cycles (same as relaxed store due to TSO)

#### Acquire Load (load fence)
```asm
mov rax, [rdi]           ; Load with implicit ordering
; x86-64 loads are already ordered against subsequent ops
```
**Cost:** ~2-4 cycles (same as relaxed load due to TSO)

#### Memory Fence (full barrier)
```asm
mfence                   ; Memory Fence - All Cores
```
**Cost:** ~10-15 cycles
**Semantics:** Prevents all reordering; strictest barrier
**Usage:** Necessary between Arc drop's Release and the Acquire fence

#### Compare-and-Swap (CAS) - Relaxed
```asm
mov rax, [rsi]           ; Load expected value
cmp rcx, rax             ; Compare with memory
jne fail
mov [rdi], rdx           ; Store if equal
success:
```
**Cost:** 2-8 cycles (depends on contention and success)

#### Compare-and-Swap (CAS) - Acquire
```asm
lock cmpxchg [rdi], rax  ; Atomic CAS with lock prefix
```
**Cost:** 2-8 cycles base + internal synchronization
**Lock Prefix:** Generates atomic bus lock, all other CPUs pause during operation

#### Fetch-Add (Relaxed)
```asm
mov rax, [rdi]
add rax, 1
mov [rdi], rax
```
**Cost:** 1-3 cycles (uncontended)

#### Fetch-Add (with internal locking due to atomicity)
```asm
lock add [rdi], 1        ; Lock-prefixed add
```
**Cost:** 2-8 cycles uncontended, much higher under contention

### Arc Clone on x86-64
```asm
; Arc::clone() -> fetch_add(1, Relaxed)
lock add qword [rdi + offset_strong], 1
```
**Cost:** 2-8 cycles depending on contention

### Arc Drop on x86-64
```asm
; Arc::drop() -> fetch_sub(1, Release)
lock sub qword [rdi + offset_strong], 1  ; Release is implicit
jne skip_fence

; Refcount is 1, need to delete
mfence                   ; Acquire fence (Acquire semantics)
; Drop the data
call destructor
call deallocate

skip_fence:
```
**Cost:** 
- When refcount > 1: 2-8 cycles
- When refcount = 1: 2-8 cycles + 10-15 cycles (mfence)

### False Sharing Example on x86-64

```
Before optimization (both on same cache line):

Thread 0          Cache L1/L2        Thread 1
├─ modify head ──→ [64-byte cache   ←─ modify tail
                    line]

Each write invalidates the line for the other thread
Result: Cache line ping-pong, severe performance loss
```

### Cache Coherence Protocol: MESIF (Intel)
- **Modified:** Written by one core, invalid on others
- **Exclusive:** Read by one core, valid only there
- **Shared:** Valid on multiple cores
- **Invalidated:** Invalid, needs fetch
- **Forwarded:** Shared with recent writer

Atomic operations trigger MESIF state changes.

---

## ARM64 (AArch64) Architecture

### Memory Model
- **Memory Consistency:** Weakly Ordered (like RISC-V)
- **Cache Line Size:** 64-128 bytes (varies by core)
- **Atomic Alignment:** Natural alignment (8 bytes for u64)

### Atomic Operations Implementation

#### Relaxed Load
```asm
ldr x0, [x1]             ; Load register from memory
```
**Cost:** 1-3 cycles (L1 hit)

#### Relaxed Store
```asm
str x0, [x1]             ; Store register to memory
```
**Cost:** 1-3 cycles (L1 hit)

#### Release Store
```asm
str x0, [x1]             ; Release stored implicitly
; No explicit barrier needed; store is released
```
**Cost:** ~3-4 cycles

#### Acquire Load
```asm
ldr x0, [x1]             ; Acquire loaded implicitly
; Load acts as acquire
```
**Cost:** ~3-4 cycles

#### Memory Fence (full barrier)
```asm
dmb sy                   ; Data Memory Barrier, SY (Full system)
```
**Cost:** ~5-10 cycles (lower than x86-64 mfence)
**Semantics:** Full sequential consistency point

**Other DMB variants:**
```asm
dmb ld                   ; Load-Load barrier
dmb st                   ; Store-Store barrier
dmb osh                  ; Outer Shareable domain
dmb ish                  ; Inner Shareable domain
```

#### Load-Exclusive / Store-Exclusive (LL/SC)
```asm
; Compare-and-Swap implementation
ldaxr x0, [x1]           ; Load-Exclusive with Acquire
cmp x0, x2               ; Compare with expected
bne fail
stlxr w3, x3, [x1]       ; Store-Exclusive with Release
cbnz w3, loop            ; Retry if failed
success:
```
**Cost:** 2-8 cycles uncontended, higher under contention

#### Atomic Add (Relaxed)
```asm
loop:
ldxr x0, [x1]            ; Load exclusive
add x0, x0, 1
stxr w2, x0, [x1]        ; Store exclusive
cbnz w2, loop            ; Retry if failed
```
**Cost:** 2-8 cycles (loop due to LL/SC nature)

#### Atomic Add (with ordering)
```asm
loop:
ldaxr x0, [x1]           ; Load-Acquire-Exclusive
add x0, x0, 1
stlxr w2, x0, [x1]       ; Store-Release-Exclusive
cbnz w2, loop
```
**Cost:** 3-10 cycles

### Arc Clone on ARM64
```asm
; Arc::clone() -> fetch_add(1, Relaxed)
loop:
ldxr x0, [x1]
add x0, x0, 1
stxr w2, x0, [x1]
cbnz w2, loop
```
**Cost:** 2-8 cycles

### Arc Drop on ARM64
```asm
; Arc::drop() -> fetch_sub(1, Release)
loop:
ldxr x0, [x1]
sub x0, x0, 1
stlxr w2, x0, [x1]       ; Release semantics on store
cbnz w2, loop

cmp x0, 0
bne skip_fence
dmb sy                   ; Full barrier before delete
call destructor
skip_fence:
```
**Cost:**
- When refcount > 1: 2-8 cycles
- When refcount = 1: 2-8 cycles + 5-10 cycles (dmb sy)

### ARM64 vs x86-64 Comparison
| Operation | x86-64 | ARM64 |
|-----------|--------|-------|
| Simple Load | 1-3 | 1-3 |
| Simple Store | 1-3 | 1-3 |
| Acquire Load | 1-3 (TSO) | 3-4 |
| Release Store | 1-3 (TSO) | 3-4 |
| Full Fence | 10-15 (mfence) | 5-10 (dmb sy) |
| LL/SC loop | N/A | 2-8 |
| Lock-prefixed | 2-8 | N/A |

---

## RISC-V Architecture

### Memory Model
- **Memory Consistency:** Weakly Ordered
- **Cache Line Size:** 64 bytes (typical)
- **Atomic Alignment:** Natural alignment (8 bytes for u64)

### Atomic Operations Implementation

#### Relaxed Load
```asm
ld x10, 0(x11)           ; Load doubleword
```
**Cost:** 1-3 cycles (L1 hit)

#### Relaxed Store
```asm
sd x10, 0(x11)           ; Store doubleword
```
**Cost:** 1-3 cycles (L1 hit)

#### Load-Reserved / Store-Conditional (LR/SC)

```asm
; Atomic increment (Relaxed)
loop:
lr.d x10, (x11)          ; Load-Reserved doubleword
addi x10, x10, 1
sc.d x12, x10, (x11)     ; Store-Conditional doubleword
bnez x12, loop           ; Retry if failed
```

**LR/SC Semantics:**
- **LR (Load-Reserved):** Marks memory location as reserved for current hart
- **SC (Store-Conditional):** Succeeds only if location still reserved
- Multiple SCs on same hart will fail
- Interrupts or other harts accessing location will clear reservation

**Cost:** 2-10 cycles (loop depends on contention)

#### Load-Acquire / Store-Release

```asm
; With Acquire semantics
loop:
lr.d.aq x10, (x11)       ; Load-Reserved with Acquire
addi x10, x10, 1
sc.d x12, x10, (x11)     ; Store-Conditional
bnez x12, loop
```

```asm
; With Release semantics
loop:
lr.d x10, (x11)          ; Load-Reserved
addi x10, x10, 1
sc.d.rl x12, x10, (x11)  ; Store-Conditional with Release
bnez x12, loop
```

**Ordering Suffixes:**
- **.aq:** Acquire semantics (load barrier)
- **.rl:** Release semantics (store barrier)
- **.aqrl:** Both (full barrier on atomic operation)

#### Memory Fence (FENCE instruction)

```asm
fence                    ; Equivalent to fence.rw.rw
fence.r.r                ; Load-Load barrier
fence.w.w                ; Store-Store barrier
fence.rw.rw              ; Full barrier
fence.tso                ; Total Store Order (for compatibility)
```

**Cost:** 
- FENCE: 5-15 cycles (varies by implementation)
- More efficient than x86-64 on weak-memory cores

### Arc Clone on RISC-V
```asm
; Arc::clone() -> fetch_add(1, Relaxed)
loop:
lr.d x10, (x11)
addi x10, x10, 1
sc.d x12, x10, (x11)
bnez x12, loop
```
**Cost:** 2-8 cycles

### Arc Drop on RISC-V
```asm
; Arc::drop() -> fetch_sub(1, Release)
loop:
lr.d.rl x10, (x11)       ; Load with Release
addi x10, x10, -1
sc.d x12, x10, (x11)
bnez x12, loop

bne x10, zero, skip_fence
fence.rw.rw              ; Full barrier before delete
call destructor
skip_fence:
```
**Cost:**
- When refcount > 1: 2-8 cycles
- When refcount = 1: 2-8 cycles + 5-15 cycles (FENCE)

### RISC-V Atomic Extensions (if available)

Modern RISC-V with Atomic Extensions (RVA):

```asm
; Atomic Add instruction
amoadd.d x10, x11, (x12)  ; Add x11 to memory[x12], return old value
amoadd.d.aq x10, x11, (x12)
amoadd.d.rl x10, x11, (x12)
amoadd.d.aqrl x10, x11, (x12)
```

**Cost:** 3-8 cycles depending on ordering

---

## Comparison Matrix

### Performance Characteristics

| Operation | x86-64 | ARM64 | RISC-V |
|-----------|--------|-------|--------|
| **Relaxed Load** | 1-3 | 1-3 | 1-3 |
| **Relaxed Store** | 1-3 | 1-3 | 1-3 |
| **Acquire Load** | 1-3 | 3-4 | 3-5 |
| **Release Store** | 1-3 | 3-4 | 3-5 |
| **Compare-and-Swap** | 2-8 | 2-8 | 2-8 |
| **Atomic Add** | 2-8 | 2-8 | 2-8 |
| **Full Fence** | 10-15 | 5-10 | 5-15 |
| **Lock Prefix** | 2-8 | N/A | N/A |
| **LL/SC Loop** | N/A | 2-8 | 2-8 |

### Barrier Cost Efficiency

**FENCE/DMB Cost Per Operation (estimated):**
- x86-64 MFENCE: 100% (baseline)
- ARM64 DMB SY: 50-60% (more efficient)
- RISC-V FENCE: 50-80% (varies by core)

### Memory Model Impact on Arc

**x86-64 (Total Store Order):**
- Stores are already ordered to all cores
- Loads wait for prior stores (TSO)
- Arc clone: Relaxed is very efficient
- Arc drop: Fence overhead is significant

**ARM64 (Weak Ordering):**
- No implicit ordering between loads/stores
- LDAXR/STLXR provide fine-grained control
- Arc clone: Slightly more expensive than x86-64
- Arc drop: Fence more efficient than x86-64

**RISC-V (Weak Ordering):**
- Most flexible memory model
- LR/SC primitives efficient for LL/SC loops
- Atomic extensions provide better performance
- Arc implementation varies by core microarchitecture

---

## Cache Behavior Analysis

### Cache Line Ping-Pong Across Architectures

**All architectures suffer false sharing:**

```
Hypothetical: Two adjacent atomics in Arc structure

x86-64:
  Thread A modifies strong → LOCK signal → MESI state change
  Thread B modifies weak → Must wait → Cache miss
  Result: 10-100+ cycle penalty on weak access

ARM64:
  Thread A: ldaxr strong
  Thread B: ldxr weak (same cache line)
  Both read from L3 because line in different cores' caches
  Result: 20-100+ cycle penalty (depends on cache coherence protocol)

RISC-V:
  Thread A: lr.d.aq strong
  Thread B: lr.d weak (same cache line)
  RISC-V doesn't require coherence at atomic level (only at memory order)
  Result: Varies widely by implementation
```

---

## Real-World Implications for Arc

### When Ordering Matters Most

1. **Drop of last Arc (refcount becomes 0):**
   - **x86-64:** MFENCE ~10-15 cycles
   - **ARM64:** DMB SY ~5-10 cycles
   - **RISC-V:** FENCE ~5-15 cycles
   - This is where 80% of atomic overhead lives

2. **Clone operations (most common):**
   - **All architectures:** 2-8 cycles
   - Low impact on throughput
   - Minimal difference between Relaxed and Release

3. **Weak upgrade operations:**
   - Requires Acquire semantics
   - Less frequent than clone/drop
   - Still 2-8 cycles

### Optimization Opportunities

**x86-64:**
- Consider lock-free structures for frequently dropped Arcs
- Use `crossbeam_utils::CachePadded` aggressively
- Profile MFENCE stalls with `perf stat -e mem-loads-aux-mem`

**ARM64:**
- Fewer fence penalties than x86-64
- LL/SC loops naturally fair on contention
- Consider Weak pointers to reduce Arc drops

**RISC-V:**
- Most flexible for custom implementations
- Can use fine-grained ordering (.aq/.rl suffixes)
- Profile with RISC-V-specific tools (depends on implementation)

---

## Summary: Key Takeaways

1. **x86-64 is most synchronization-friendly but has expensive fences**
2. **ARM64 provides balanced performance across operations**
3. **RISC-V offers flexibility but implementation-dependent performance**
4. **Arc drop fence is the expensive operation across all architectures**
5. **False sharing affects all architectures equally** (hardware issue, not atomic ordering)
6. **Profile on target architecture** - Numbers vary significantly

---

## References

- **x86-64:** Intel 64 and IA-32 Architectures Software Developer Manual, Volume 3B
- **ARM64:** ARM A64 Instruction Set Architecture (ISA) Manual
- **RISC-V:** RISC-V Instruction Set Manual, Volume I: User-Level ISA, Atomic Extension
- **ARM DMB:** "Synchronization and semaphores," ARM white paper
- **RISC-V Atomics:** "Weak Memory Ordering in RISC-V," RISC-V Foundation

