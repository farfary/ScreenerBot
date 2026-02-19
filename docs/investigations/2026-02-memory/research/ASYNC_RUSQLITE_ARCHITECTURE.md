# Async Rusqlite Architecture Deep Dive

## The Problem: Blocking in Async Code

### Why SQLite/rusqlite Can't Be Async Natively

```
Async Runtime (Tokio)
├─ Task 1: wait for data
├─ Task 2: wait for network
├─ Task 3: ❌ wait for sqlite (BLOCKS EVERYTHING)
└─ Task 4: can't run while Task 3 blocks the thread
```

**The Issue**:
- Tokio schedulers have a fixed number of threads (usually 4-8 per core)
- If one thread blocks, other tasks queued on that thread can't run
- rusqlite is purely synchronous; it blocks while SQLite compiles/executes queries
- **Result**: Stalled async runtime if you call rusqlite directly

### The Solution: Move Blocking to Dedicated Thread

```
Async Runtime (Tokio threads)    Background Thread (Blocked OK)
├─ Task 1: await                 
├─ Task 2: await                 └─ Blocking SQLite operations
├─ Task 3: await                    (no impact on async tasks)
└─ Task 4: await
```

---

## Architecture: Thread-per-Connection Pattern

### High-Level Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                     Async Caller (Tokio Task)                   │
│                                                                 │
│  async fn main() {                                              │
│      let user = conn.call(|c| {  ← Closure created            │
│          c.execute("INSERT ...")  ← Defined here, not run     │
│      }).await?;  ← Awaits result, doesn't block               │
│  }                                                              │
└────────────────────────────────────────────────────────────────┘
                           │
                           │ Send closure via channel
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│              Message Channel (crossbeam/bounded)                │
│                                                                 │
│  Loop {                                                         │
│      recv() Message::Execute(closure)  ← Receive work         │
│      recv() Message::Close(responder)  ← Receive close signal │
│  }                                                              │
└────────────────────────────────────────────────────────────────┘
                           │
                           │ Execute closure on background thread
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│            Background Thread (OK to Block)                      │
│                                                                 │
│  fn event_loop(mut conn: rusqlite::Connection) {               │
│      loop {                                                     │
│          match recv() {                                         │
│              Execute(f) => f(&mut conn),  ← BLOCKING HERE     │
│              Close(responder) => {                             │
│                  conn.close()?                                │
│                  responder.send(result)  ← Send result back   │
│              }                                                 │
│          }                                                     │
│      }                                                         │
│  }                                                              │
│                                                                 │
│  SQLite operations are synchronous; can block freely           │
└────────────────────────────────────────────────────────────────┘
```

### Message Types

```rust
enum Message {
    Execute(Box<dyn Fn(&mut rusqlite::Connection) + Send>),
    Close(oneshot::Sender<rusqlite::Result<()>>),
}
```

---

## Implementation Comparison

### 1. tokio-rusqlite (Reference Implementation)

#### Struct Definition
```rust
pub struct Connection {
    sender: crossbeam_channel::Sender<Message>,
}

// Internally:
// - Stores only the channel sender
// - Connection is Arc-wrapped (cheap to clone)
```

#### Creating a Connection
```rust
async fn open(path: impl AsRef<Path>) -> Result<Self> {
    let (sender, receiver) = crossbeam_channel::unbounded();
    
    // Spawn background thread
    thread::spawn(move || event_loop(conn, receiver));
    
    Ok(Connection { sender })
}
```

**Key Design Choices**:
- ✅ **Unbounded channel**: Sender never blocks
- ✅ **One thread per connection**: Simple mental model
- ✅ **crossbeam-channel**: Battle-tested library
- ❌ **No backpressure**: Queue can grow unbounded

#### The call() Method
```rust
pub async fn call<F, R, E>(&self, function: F) -> Result<R, Error<E>>
where
    F: FnOnce(&mut rusqlite::Connection) -> Result<R, E> + Send + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    // 1. Create a one-shot channel for the response
    let (responder, receiver) = oneshot::channel();
    
    // 2. Send the function wrapped in a closure
    self.sender.send(Message::Execute(Box::new(move |conn| {
        let result = function(conn);  // Execute on background thread
        let _ = responder.send(result);  // Send result back
    })))?;
    
    // 3. Wait asynchronously for the result
    receiver.await
        .map_err(|_| Error::ConnectionClosed)
        .and_then(|result| result.map_err(Error::Error))
}
```

**Execution Timeline**:
```
Time │ Async Task           │ Background Thread        │ Return Channel
─────┼──────────────────────┼──────────────────────────┼─────────────────
  0  │ create closure       │ (idle)                   │ empty
  1  │ send(closure)        │ (idle)                   │ empty
  2  │ await receiver       │ recv(closure)            │ empty
  3  │ await receiver       │ f(&mut conn) executing   │ empty
  4  │ await receiver       │ f(&mut conn) executing   │ empty
  5  │ await receiver       │ responder.send(result)   │ result ready
  6  │ ready, get result    │ loop again               │ consumed
```

#### Event Loop
```rust
fn event_loop(mut conn: rusqlite::Connection, receiver: Receiver<Message>) {
    while let Ok(message) = receiver.recv() {
        match message {
            Message::Execute(f) => {
                f(&mut conn);  // Blocking call happens here
            }
            Message::Close(responder) => {
                let result = conn.close();
                let _ = responder.send(result);
                break;  // Exit loop
            }
        }
    }
    // Thread exits after receiver is dropped
}
```

**Thread Lifecycle**:
1. Thread spawned when connection created
2. Waits on `receiver.recv()` (blocked but not consuming CPU)
3. When message arrives, executes work
4. Sends result back via one-shot
5. Returns to waiting
6. Exits when receiver is dropped

---

### 2. async-rusqlite (Bounded Channel Variant)

#### Key Difference: Bounded Channels
```rust
pub fn channel_size(mut self, size: usize) -> Self {
    self.asyncified_builder = self.asyncified_builder.channel_size(size);
    self
}

// Default bounded channel with backpressure
let conn = Connection::builder()
    .channel_size(100)  // Queue up to 100 tasks before sender blocks
    .open("db.sqlite")
    .await?;
```

#### Behavior
```
Unbounded (tokio-rusqlite):
┌──────────────────────────────────┐
│ Sender ─ [∞ items queued] ─ Receiver
│ (never blocks)                   │
└──────────────────────────────────┘

Bounded (async-rusqlite):
┌──────────────────────────────────────┐
│ Sender ─ [100 items max] ─ Receiver  │
│ (blocks if full)                    │
└──────────────────────────────────────┘
```

**When Queue Full**:
```
Time │ Sender                      │ Receiver                │
─────┼────────────────────────────┼───────────────────────┤
  0  │ send(task_101) → BLOCKED   │ processing task_1     │
  1  │ send(task_101) → BLOCKED   │ processing task_2     │
  2  │ send(task_101) → BLOCKED   │ processing task_3     │
  3  │ send(task_101) → BLOCKED   │ ✓ done with task_1    │
  4  │ send(task_101) → BLOCKED   │ ✓ done with task_2    │
  5  │ send(task_101) ✓ succeeds  │ (now space in queue)   │
```

**Advantage**: Sender experiences backpressure; prevents unbounded queue growth

---

### 3. nd-async-rusqlite (Pool Variant)

#### WalPool Architecture
```
┌─────────────────────────────────────────────────────────┐
│                    WalPool                              │
│  ┌──────────────┬──────────────┬──────────────┐        │
│  │ Thread 1     │ Thread 2     │ Thread N     │        │
│  │ (read task)  │ (write task) │ (read task)  │        │
│  └──────────────┴──────────────┴──────────────┘        │
│           ▲            ▲            ▲                  │
│           │            │            │                  │
│      [conn1]       [conn2]      [connN]               │
│           │            │            │                  │
│           └────────────┴────────────┘                  │
│              Async Callers                            │
└─────────────────────────────────────────────────────────┘
```

**Key Feature**: WAL (Write-Ahead Logging)
- Enables **concurrent readers + single writer**
- Unlike traditional SQLite: lock contention is lower
- Each thread can hold a connection to same DB

#### Pool vs Single Connection

**Single Connection (tokio-rusqlite)**:
```
Caller 1: INSERT user  ──┐
Caller 2: SELECT posts   ├─ Queue  ─ Background Thread ─ SQLite
Caller 3: UPDATE order  ──┤         (serialized)
Caller 4: SELECT users  ──┘

Problem: SELECT blocked by INSERT (writer lock)
```

**WalPool with WAL Mode**:
```
Caller 1: INSERT ──┐
Caller 2: SELECT ──├─ Thread Pool ─ SQLite (WAL mode)
Caller 3: UPDATE ──┤     (concurrent
Caller 4: SELECT ──┘      readers + 1 writer)

Benefit: SELECT can run while INSERT happens
```

---

## Channel Synchronization Details

### crossbeam-channel (Used by tokio-rusqlite)

```rust
// Unbounded MPSC (Multi-Producer, Single-Consumer)
let (sender, receiver) = crossbeam_channel::unbounded::<Message>();

// Sender side
sender.send(Message::Execute(closure))
  .map_err(|_| Error::ConnectionClosed)?  // Error if receiver dropped

// Receiver side (in background thread)
while let Ok(message) = receiver.recv() {
    // Process message
}
// Returns when all senders drop
```

**Properties**:
- Thread-safe (no mutex needed)
- Lock-free (using atomic operations)
- No heap allocation per message
- Sender can never block

### bounded channel (Used by async-rusqlite)

```rust
let (sender, receiver) = bounded::<Message>(100);

// Sender will block if queue has 100 items and receiver hasn't consumed
sender.send(Message::Execute(closure)).await?  // Can block
```

**Trade-offs**:
- Prevents memory explosion from unbounded queue
- Sender experiences feedback about processing speed
- Must tune channel size for workload

### tokio::sync::oneshot (Response Channel)

```rust
let (responder, receiver) = oneshot::channel::<R>();

// Sender (in background thread)
responder.send(result)  // Can only send once

// Receiver (in async task)
let result = receiver.await?;  // Wait for response
```

**Properties**:
- One-way, one-time message
- Clean up on drop
- No blocking operations

---

## Memory Layout

### Connection Struct (tokio-rusqlite)

```rust
pub struct Connection {
    sender: crossbeam_channel::Sender<Message>,
}

// Actual memory usage:
// - sender: Arc<crossbeam_channel::ChannelInner> ≈ 16-24 bytes
// - Total: ~20-30 bytes in stack

// Clone is cheap:
let conn2 = conn.clone();  // Just increments Arc refcount

// Real memory consumption:
// - Background thread: ~2MB (OS thread overhead)
// - Channel buffer: 0 for unbounded (zero-copy)
// - Message: heap-allocated closures (depends on captured data)
```

### Message Storage

```rust
enum Message {
    Execute(Box<dyn Fn(&mut rusqlite::Connection) + Send>),
    Close(oneshot::Sender<rusqlite::Result<()>>),
}

// Memory breakdown:
// - Execute variant:
//   - Box ptr: 8 bytes
//   - Captured data: varies
// - Close variant:
//   - Sender: ~8 bytes
```

---

## Error Handling Patterns

### tokio-rusqlite Error Types

```rust
pub enum Error<E = rusqlite::Error> {
    ConnectionClosed,                    // Channel broken
    Close((Connection, rusqlite::Error)), // Close failed, return connection
    Error(E),                            // Application error
}

// Usage pattern:
match conn.call(|c| c.execute(sql, params)).await {
    Ok(_) => {},
    Err(Error::ConnectionClosed) => {
        // Connection dead, recreate it
    }
    Err(Error::Close((conn, e))) => {
        // Close failed, can retry
        conn.close().await?;
    }
    Err(Error::Error(rusqlite::Error::QueryReturnedNoRows)) => {
        // Not found
    }
}
```

### Panic Safety

```rust
// tokio-rusqlite/async-rusqlite: ⚠️ Panic in closure will:
let result = conn.call(|c| {
    panic!("oops");  // ⚠️ Thread panics, connection closed
}).await;
// Result: connection permanently closed

// nd-async-rusqlite: ✅ Can recover from panic
use nd_async_rusqlite::Error;

match conn.access(|c| {
    panic!("oops");
}).await {
    Err(Error::AccessPanic(data)) => {
        eprintln!("Recovered from panic");
        // Connection still alive, can retry
    }
    _ => {}
}
```

---

## Performance Characteristics

### Latency Analysis

```
Operation: conn.call(|c| c.query_row(...)).await

Timeline:
Time │ Duration   │ Action
─────┼────────────┼─────────────────────────────────────
  0  │ 0 µs       │ Create closure in async context
  1  │ 100 ns     │ Send through channel (atomic ops)
  2  │ 50 µs      │ Context switch to background thread
  3  │ XXX µs     │ SQLite query execution (the real work)
  4  │ 50 µs      │ Context switch back
  5  │ 100 ns     │ Return via one-shot channel
     │ XXX + 200 µs│ Total

The context switching overhead (~100 µs) is negligible compared to actual
SQLite work (typically 100 µs to 100 ms depending on query complexity).
```

### Throughput Analysis

#### Batch Operations (Recommended)
```rust
// 1000 inserts in one closure
conn.call(|c| {
    for item in items {
        c.execute("INSERT ...", [item])?;
    }
    Ok(())
}).await?

// Cost: 1 channel send + 1000 inserts + 1 channel receive
// Time: ~1-10 ms (SQLite dominated)
// Throughput: 100-1000 ops/ms
```

#### Individual Operations (Anti-pattern)
```rust
// 1000 separate calls
for item in items {
    conn.call(|c| c.execute("INSERT ...", [item])).await?;
}

// Cost: 1000 channel sends + 1000 inserts + 1000 channel receives
// Context switching: 1000 × 2 × 50 µs = 100 ms overhead
// Time: ~100-120 ms (context switching dominated)
// Throughput: 8-10 ops/ms (10x slower!)
```

### Memory Usage Comparison

```
                 Per Connection    Per Thread    Total (10 connections)
tokio-rusqlite:  ~30 bytes         ~2 MB        ~20 MB + heap for DBs
async-rusqlite:  ~30 bytes         ~2 MB        ~20 MB + heap for DBs
nd-async-rusqlite: ~30 bytes       ~2 MB        ~20 MB + heap for DBs
                                   (with pool:  ~2 MB per pool thread)

SQLite DB file: ~100 KB - many GB (data dependent)
```

---

## Concurrency Model

### tokio-rusqlite

```
Connection 1 ──┐
Connection 2 ──┼─ Each has its own background thread
Connection 3 ──┘

Async Task 1 ──┐  
Async Task 2 ──┼─ Can call conn1, conn2, or conn3
Async Task 3 ──┘

Parallelism: ✅ Multiple connections can run in parallel
            ❌ Single connection serializes operations
```

**Database Locks**:
- SQLite has a file-level lock
- Only one writer at a time
- Multiple readers OK (depends on journal mode)

### nd-async-rusqlite with WalPool

```
WalPool ──┬─ Thread 1 ──┐
          ├─ Thread 2 ──┤─ All connected to same database
          ├─ Thread 3 ──┤  with WAL mode
          └─ Thread N ──┘

Async Tasks ──┬─ call access() ──┐
              ├─ call access() ──┤─ Round-robin to threads
              └─ call access() ──┘

Parallelism: ✅ Multiple threads can access same database
            ✅ Concurrent readers + single writer (WAL mode)
            ⚠️  More complex state management
```

---

## When to Use Which Architecture

### Thread-per-Connection (tokio-rusqlite, async-rusqlite)
- ✅ Simple to understand
- ✅ Low overhead for few connections
- ✅ Good for web servers (one connection per request)
- ❌ Scales poorly with many connections (thread per connection)
- ❌ No concurrency on single connection

### Connection Pooling (nd-async-rusqlite with WalPool)
- ✅ Scales to many concurrent operations
- ✅ Concurrent reader support
- ✅ Bounded resource usage
- ❌ More complex API
- ❌ Requires WAL mode setup

---

## Debugging & Monitoring

### Watching the Message Queue

```rust
// Current implementation: no introspection
// But you can infer state from response times

// Healthy: call() returns in ~1-100 ms
// Sick: call() returns in 1+ seconds
//   → Database thread is backed up
//   → Consider using bounded channels or pooling
```

### Thread Inspection

```bash
# See background threads created for each connection
ps aux | grep rusqlite

# Monitor with top/htop
# Each connection = 1 additional thread
# Memory usage = (num_connections × 2MB) + heap
```

### Logging Patterns

```rust
// Add logging to understand flow
conn.call(|c| {
    debug!("Starting query");
    let result = c.query_row(...)?;
    debug!("Query complete");
    Ok(result)
}).await?;

// Gaps in logs indicate queue wait time
```

---

## Summary of Design Trade-offs

| Design | Pros | Cons | Use Case |
|--------|------|------|----------|
| **Unbounded** | Simple, no tuning | Can starve memory | Production with monitoring |
| **Bounded** | Backpressure, predictable | Tuning required | Resource-constrained |
| **Pooling** | Concurrent access, scalable | Complex API | High-throughput systems |

