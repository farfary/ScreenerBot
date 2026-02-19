# Rust std::sync::Arc Documentation Analysis

## Overview
A thread-safe reference-counting pointer. 'Arc' stands for 'Atomically Reference Counted'.

**Official Documentation Source:** https://doc.rust-lang.org/std/sync/struct.Arc.html  
**Source Code:** https://doc.rust-lang.org/src/alloc/sync.rs.html  
**Available Since:** Rust 1.0.0

---

## Struct Definition

```rust
pub struct Arc<T, A = Global>
where
    A: Allocator,
    T: ?Sized,
{
    /* private fields */
}
```

### Generic Parameters:
- **T**: The type of value being reference counted (can be unsized with `?Sized`)
- **A**: Custom allocator (unstable feature, defaults to Global allocator)

---

## Memory Layout & Implementation Details

### Internal Structure (ArcInner)

```rust
#[repr(C, align(2))]
struct ArcInner<T: ?Sized> {
    strong: Atomic<usize>,
    weak: Atomic<usize>,
    data: T,
}
```

### Memory Layout Characteristics:

1. **Representation**: `#[repr(C, align(2))]`
   - C-compatible layout for FFI safety
   - Minimum alignment of 2 bytes (for consistency with RcInner)
   - Alignment is generally the same as atomic type's alignment

2. **Reference Counters**:
   - **strong**: Atomic `usize` - tracks strong references (Arc pointers)
   - **weak**: Atomic `usize` - tracks weak references (Weak pointers)
   - Both use atomic operations for thread-safe increment/decrement

3. **Data Field**:
   - Stored directly after the two atomic counters
   - Layout calculation: `Layout::new::<ArcInner<()>>().extend(layout).unwrap().0.pad_to_align()`

### Size Calculation:

For `Arc<T>` the size is a pointer (typically 8 bytes on 64-bit systems):
- Arc itself contains a `NonNull<ArcInner<T>>` pointer + `PhantomData` + allocator field
- The actual allocated heap memory contains: `ArcInner<T>` which includes two atomic `usize` counters

### Field Structure of Arc:
```rust
pub struct Arc<T, A = Global> {
    ptr: NonNull<ArcInner<T>>,           // Non-null pointer to heap
    phantom: PhantomData<ArcInner<T>>,   // Zero-size marker
    alloc: A,                             // Allocator (typically zero-size)
}
```

---

## Reference Counting Details

### Weak Pointer Sentinel Value:
```rust
const MAX_REFCOUNT: usize = (isize::MAX) as usize;
const INTERNAL_OVERFLOW_ERROR: &str = "Arc counter overflow";
```

The implementation uses:
- `usize::MAX` as a sentinel for temporarily "locking" the ability to upgrade weak pointers or downgrade strong ones
- This is used to avoid races in `make_mut` and `get_mut`

### Thread Synchronization:

The module uses platform-dependent atomic operations:
```rust
#[cfg(not(sanitize = "thread"))]
macro_rules! acquire {
    ($x:expr) => {
        atomic::fence(Acquire)  // Normal fence
    };
}

#[cfg(sanitize = "thread")]
macro_rules! acquire {
    ($x:expr) => {
        $x.load(Acquire)  // ThreadSanitizer uses atomic loads
    };
}
```

---

## Key Design Characteristics

### 1. **Thread-Safe Reference Counting**
- Uses atomic operations for reference counting (unlike Rc<T>)
- Atomic operations are more expensive than ordinary memory accesses
- Implements `Send + Sync` when `T: Send + Sync`

### 2. **Shared Ownership**
- Provides shared ownership of a value allocated on the heap
- Cloning produces new Arc pointing to same allocation
- Reference count increments on clone, decrements on drop

### 3. **Immutability by Default**
- Shared references disallow mutation by default
- Options for mutation:
  1. Interior mutability with `Mutex<T>`, `RwLock<T>`, or atomic types
  2. Clone-on-write with `Arc::make_mut()` - clones only when needed
  3. Direct access with `Arc::get_mut()` - when reference count is 1

### 4. **Weak References**
- `Weak<T>` pointers don't keep value alive
- Prevent circular reference memory leaks
- Can be upgraded to Arc with `upgrade()` method
- Upgrade returns `Option<Arc<T>>` (None if value dropped)

---

## Platform Support

**Note**: Arc is only available on platforms that support atomic loads and stores of pointers.

This includes:
- ✅ All platforms supporting the `std` crate
- ❌ Not all platforms supporting only `alloc` crate

Detection: `#[cfg(target_has_atomic = "ptr")]`

---

## Key Methods Summary

### Construction Methods:
- `Arc::new(data: T) -> Arc<T>` - Basic construction
- `Arc::new_cyclic(f) -> Arc<T>` - For self-referential structures
- `Arc::new_uninit() -> Arc<MaybeUninit<T>>` - Uninitialized memory
- `Arc::new_zeroed() -> Arc<MaybeUninit<T>>` - Zero-initialized memory (1.92.0+)
- `Arc::try_new(data: T) -> Result<Arc<T>>` - Fallible construction

### Reference Count Operations:
- `Arc::strong_count(&self) -> usize` - Get strong reference count
- `Arc::weak_count(&self) -> usize` - Get weak reference count
- `Arc::clone(&self) -> Arc<T>` - Increment strong count

### Ownership Manipulation:
- `Arc::try_unwrap(self) -> Result<T, Arc<T>>` - Convert to T if unique
- `Arc::into_inner(this: Arc<T>) -> T` - Consume Arc, extracting data
- `Arc::make_mut(&mut self) -> &mut T` - Clone-on-write mutation
- `Arc::get_mut(&mut self) -> Option<&mut T>` - Get mutable if unique

### Pointer Operations:
- `Arc::into_raw(self) -> *const T` - Convert to raw pointer
- `Arc::from_raw(ptr: *const T) -> Arc<T>` - Reconstruct from raw pointer
- `Arc::as_ptr(&self) -> *const T` - Get const raw pointer
- `Arc::ptr_eq(&self, other: &Arc<T>) -> bool` - Compare pointers

### Weak References:
- `Arc::downgrade(&self) -> Weak<T>` - Create weak reference
- `Arc::downcast<U>` - Cast to different type

### Memory Operations:
- `Arc::get_mut_unchecked(&mut self) -> &mut T` - Unsafe mutable access
- `Arc::assume_init()` - Initialize MaybeUninit Arc
- `Arc::into_array()` - Convert arc of array into array of arcs

### Allocator Support (Unstable):
- `Arc::new_in(data, alloc)` - Construct in custom allocator
- `Arc::try_new_in(data, alloc)` - Fallible construction
- Methods suffixed with `_in` for custom allocators

---

## Safety Guarantees

### Send & Sync Implementation:
```rust
unsafe impl<T: ?Sized + Sync + Send, A: Allocator + Send> Send for Arc<T, A> {}
unsafe impl<T: ?Sized + Sync + Send, A: Allocator + Sync> Sync for Arc<T, A> {}
```

Arc is `Send` and `Sync` iff:
- T is `Send` and `Sync`
- Allocator A is `Send` (for Send impl)
- Allocator A is `Sync` (for Sync impl)

### Unwinding Safety:
```rust
impl<T: RefUnwindSafe + ?Sized, A: Allocator + UnwindSafe> UnwindSafe for Arc<T, A> {}
```

---

## Trait Implementations

### Core Traits:
- `Clone` - Atomic increment of strong count
- `Drop` - Atomic decrement of strong count
- `Deref<Target = T>` - Automatic dereference
- `Default` - Creates Arc with T::default()

### Comparison Traits:
- `Eq`, `PartialEq` - Compares dereferenced values
- `Ord`, `PartialOrd` - Ordered comparison
- `Hash` - Hashes dereferenced value

### I/O Traits:
- `Read` (for Arc<File>)
- `Write` (for Arc<File>)
- `Seek` (for Arc<File>)

### Conversion Traits:
- `From<T>`, `From<Box<T>>`, `From<Vec<T>>`
- `From<&[T]>`, `From<&str>`, `From<String>`
- `From<Cow<T>>`
- `TryFrom` for array conversion

### Other:
- `Debug`, `Display`, `Pointer` - Formatting
- `CoerceUnsized` - Coercion support
- `DispatchFromDyn` - Dynamic dispatch support

---

## Comparison with Similar Types

### Arc vs Rc:
| Feature | Arc | Rc |
|---------|-----|-----|
| Thread-safe | ✅ Atomic operations | ❌ Non-atomic |
| Performance | Slower (atomic cost) | Faster |
| Use when sharing | Between threads | Within single thread |
| `Send + Sync` | Conditional | Never |

### Arc vs Box:
- Box: Single ownership
- Arc: Shared ownership (multiple owners)

### Arc vs Mutex/RwLock:
- Arc: Provides shared ownership only
- Mutex/RwLock: Add synchronization for mutation
- Often used together: `Arc<Mutex<T>>`

---

## Common Use Patterns

### 1. Thread-Shared Immutable Data:
```rust
use std::sync::Arc;
use std::thread;

let data = Arc::new(vec![1, 2, 3]);
for _ in 0..10 {
    let data = Arc::clone(&data);
    thread::spawn(move || {
        println!("{:?}", *data);
    });
}
```

### 2. Interior Mutability:
```rust
use std::sync::{Arc, Mutex};

let counter = Arc::new(Mutex::new(0));
for _ in 0..10 {
    let counter = Arc::clone(&counter);
    thread::spawn(move || {
        *counter.lock().unwrap() += 1;
    });
}
```

### 3. Clone-on-Write:
```rust
use std::sync::Arc;

let mut data = Arc::new(vec![1, 2, 3]);
Arc::make_mut(&mut data).push(4);  // Clones only if needed
assert_eq!(*data, vec![1, 2, 3, 4]);
```

### 4. Preventing Memory Leaks (Weak References):
```rust
use std::sync::{Arc, Weak};

struct Node {
    value: i32,
    parent: Option<Weak<Node>>,
}
```

---

## Performance Considerations

1. **Atomic Operations Overhead**: Arc uses atomic operations for thread safety, making it slower than Rc for single-threaded code
2. **Cache Coherency**: Atomic operations may require cache-line synchronization between CPUs
3. **Reference Count Locality**: Strong and weak counts are adjacent in memory for cache efficiency
4. **Allocation Strategy**: Uses `Box::leak()` for initial allocation, then constructs Arc from the leaked allocation

---

## Notable Features

### Reference Count Overflow Protection:
- Soft limit of `isize::MAX` references
- Will abort at `MAX_REFCOUNT + 1` references
- Protects against integer overflow attacks

### Sentinel Value Usage:
- `usize::MAX` used to temporarily lock pointer upgrades/downgrades
- Prevents races in `make_mut` and `get_mut`

### Layout Optimization:
- `repr(C)` for future-proofing against field reordering
- Important for safe `into_raw()`/`from_raw()` operations
- Alignment of 2 ensures pointer can never be dangling sentinel value

---

## Source Code Location

**File**: `alloc/sync.rs` (lines 264-267 for struct definition)  
**Module**: Part of `std::sync` re-exported from `alloc::sync`

Key source locations:
- Struct definition: Lines 264-267
- ArcInner struct: Lines 382-392
- Arc::new implementation: Lines 419-428
- Reference counting constants: Lines 47-59
- Atomic operation handling: Lines 61-76
