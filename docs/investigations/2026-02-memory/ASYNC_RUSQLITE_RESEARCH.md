# Async Support in the rusqlite Ecosystem

## Overview

SQLite is inherently **blocking** and cannot be used directly in async Rust code without workarounds. The rusqlite ecosystem provides several solutions to bridge this gap by running SQLite operations on dedicated background threads and communicating via channels.

---

## Available Async Wrappers

### 1. **tokio-rusqlite** (Most Popular & Battle-Tested)
- **GitHub**: https://github.com/programatik29/tokio-rusqlite
- **Crate**: `tokio-rusqlite` v0.7.0
- **Latest**: ✅ Active (updated Dec 2024)
- **Key Characteristics**:
  - **Architecture**: One background thread per connection
  - **Channel Type**: Unbounded `crossbeam_channel` (no backpressure)
  - **Dependencies**: `crossbeam-channel`, `rusqlite`, `tokio`
  - **Error Handling**: Custom `Error<E>` enum for connection-specific errors
  - **Safety**: `#![forbid(unsafe_code)]`

**Design Philosophy**:
```rust
// Each connection spawns a dedicated background thread
thread::spawn(move || event_loop(conn, receiver));

// Functions are sent as closures through a channel
pub async fn call<F, R, E>(&self, function: F) -> Result<R, Error<E>>
where
    F: FnOnce(&mut rusqlite::Connection) -> Result<R, E> + 'static + Send,
    R: Send + 'static,
    E: Send + 'static,
```

**Example Usage**:
```rust
let conn = Connection::open("db.sqlite").await?;

let people = conn.call(|conn| {
    conn.execute("CREATE TABLE person (id INTEGER PRIMARY KEY, name TEXT)", [])?;
    
    conn.execute("INSERT INTO person (name) VALUES (?1)", ["Alice"])?;
    
    let mut stmt = conn.prepare("SELECT id, name FROM person")?;
    stmt.query_map([], |row| {
        Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
    })?
    .collect::<Result<Vec<_>, _>>()
}).await?;

conn.close().await?;
```

**Strengths**:
- ✅ Heavily tested in production
- ✅ Unbounded channels = no blocking on sender
- ✅ Clean, intuitive API
- ✅ Feature-flag parity with rusqlite
- ✅ Good error types

**Weaknesses**:
- ❌ No backpressure if DB falls behind
- ❌ One thread per connection (not pooled)
- ❌ Requires tokio (not executor-agnostic)

---

### 2. **async-rusqlite** (Executor-Agnostic)
- **GitHub**: https://github.com/jsdw/async-rusqlite
- **Crate**: `async-rusqlite` v0.5.0
- **Latest**: ✅ Active (updated Nov 2024)
- **Key Characteristics**:
  - **Architecture**: One background thread per connection
  - **Channel Type**: Bounded channels (configurable)
  - **Dependencies**: Only `asyncified` + `rusqlite` (minimal!)
  - **Executor Agnostic**: Works with tokio, async-std, etc.
  - **Backpressure**: Built-in via bounded channels

**Design Philosophy**:
```rust
// Uses the `asyncified` crate for generic async wrapping
// Allows bounded channels for backpressure
pub fn channel_size(mut self, size: usize) -> Self {
    self.asyncified_builder = self.asyncified_builder.channel_size(size);
    self
}
```

**Example Usage**:
```rust
let conn = Connection::open_in_memory().await?;

conn.call(|conn| {
    conn.execute(
        "CREATE TABLE person (id INTEGER PRIMARY KEY, name TEXT)",
        ()
    )
}).await?;

let person = Person { id: 1, name: "Alice".to_string() };

conn.call(move |conn| {
    conn.execute(
        "INSERT INTO person (id, name) VALUES (?1, ?2)",
        (person.id, person.name)
    )
}).await?;
```

**Strengths**:
- ✅ Executor-agnostic (tokio, async-std, smol, etc.)
- ✅ Bounded channels = backpressure support
- ✅ Minimal dependencies (asyncified is tiny)
- ✅ Low overhead
- ✅ Good for resource-constrained environments

**Weaknesses**:
- ❌ Less battle-tested than tokio-rusqlite
- ❌ One thread per connection
- ❌ No built-in pooling

---

### 3. **nd-async-rusqlite** (Feature-Rich)
- **GitHub**: https://github.com/nathaniel-daniel/nd-async-rusqlite-rs
- **Crate**: `nd-async-rusqlite` v0.0.0
- **Latest**: ✅ Very Active (updated Feb 2025)
- **Key Characteristics**:
  - **Architecture**: Single background thread per connection OR connection pooling
  - **Optional Feature**: `wal-pool` for Write-Ahead Logging with pool support
  - **Dependencies**: `tokio`, optional `crossbeam-channel`
  - **Error Handling**: Comprehensive error types (Rusqlite, Aborted, AccessPanic)

**Design Philosophy**:
```rust
// Offers both simple AsyncConnection and advanced WalPool
pub struct AsyncConnection;  // Simple wrapper
pub struct WalPool;          // For pooled access with WAL journaling

// Access pattern similar to others
connection.access(|conn| {
    conn.execute(sql, named_params! { ":name": value })
}).await?
```

**Strengths**:
- ✅ Optional connection pooling via `wal-pool` feature
- ✅ WAL (Write-Ahead Logging) support for better concurrency
- ✅ Panic recovery in access functions
- ✅ Active development
- ✅ Better for multi-threaded scenarios

**Weaknesses**:
- ❌ Tokio-specific (not executor-agnostic)
- ❌ WalPool feature adds complexity
- ❌ Newer, less battle-tested

---

### 4. **Additional Wrappers**
- **hchap1/rusqlite-async**: Another tokio wrapper (less maintained)
- **derekfrye/sql-middleware**: Lightweight async middleware (newer)
- **patte/tower-sessions-rusqlite-store**: SessionStore for tower (domain-specific)

---

## How They Handle Blocking Operations

### The Fundamental Problem
SQLite's rusqlite binding is **synchronous** - operations block the calling thread. In async code, blocking is poison:
```rust
// ❌ WRONG: Will block the entire async runtime
let result = connection.execute("INSERT ...", params![...])?;
```

### Solution Pattern: Thread-per-Connection

All async wrappers use the same fundamental approach:

```
┌─────────────────────────────────────────┐
│        Async Runtime (Tokio/async-std)  │
│  ┌──────────────┐   ┌──────────────┐   │
│  │ Async Task 1 │   │ Async Task 2 │   │
│  └──────┬───────┘   └──────┬───────┘   │
│         │                   │           │
│         └─────── call() ────┴────────┐  │
│                                      │  │
├──────────────────────────────────────┼──┤
│         Async Runtime Boundary       │  │
├──────────────────────────────────────┼──┤
│      Background Thread (Blocked OK)  │  │
│  ┌────────────────────────────────┐  │  │
│  │    rusqlite::Connection        │  │  │
│  │  (blocking operations allowed) │◄─┘  │
│  └────────────────────────────────┘     │
│      Via Channel (crossbeam/bounded)    │
└─────────────────────────────────────────┘
```

### Message Flow
1. **Async caller** creates a closure with the SQL operation
2. **Closure is sent** through a channel to the background thread
3. **Background thread** executes the closure against rusqlite (blocking is OK here)
4. **Result is returned** via a `oneshot` channel back to the async caller
5. **Caller receives** the result asynchronously

**Code Example** (tokio-rusqlite internals):
```rust
pub async fn call<F, R, E>(&self, function: F) -> Result<R, Error<E>>
where
    F: FnOnce(&mut rusqlite::Connection) -> Result<R, E> + 'static + Send,
{
    let (sender, receiver) = oneshot::channel::<R>();

    // Send closure to background thread
    self.sender.send(Message::Execute(Box::new(move |conn| {
        let value = function(conn);  // Blocking happens HERE (on background thread)
        let _ = sender.send(value);  // Send result back
    })))?;

    // Wait for result asynchronously
    receiver.await.map_err(|_| Error::ConnectionClosed)
}

// Background thread event loop
fn event_loop(mut conn: rusqlite::Connection, receiver: Receiver<Message>) {
    while let Ok(message) = receiver.recv() {
        match message {
            Message::Execute(f) => f(&mut conn),  // Blocking happens here, not in async context
            Message::Close(s) => { /* ... */ }
        }
    }
}
```

### Key Insight
- **Blocking is moved off the async runtime thread**
- **Async waiting is efficient** (no busy-looping)
- **One thread per connection** is spawned (can be expensive at scale)

---

## Channel Choice: Unbounded vs Bounded

### Unbounded Channels (tokio-rusqlite)
```rust
let (sender, receiver) = crossbeam_channel::unbounded::<Message>();
```
- **Pros**: Sender never blocks; fast
- **Cons**: If DB can't keep up, unbounded queue grows → memory pressure

### Bounded Channels (async-rusqlite)
```rust
let (sender, receiver) = crossbeam_channel::bounded::<Message>(size);
```
- **Pros**: Backpressure propagates; predictable memory usage
- **Cons**: Sender can block; requires tuning channel size

**Recommendation**:
- Use **unbounded** if operations are fast and consistent
- Use **bounded** if you have unpredictable load or memory constraints

---

## Best Practices for Async SQLite Access

### 1. **Choose the Right Wrapper**
| Use Case | Recommendation |
|----------|-----------------|
| Production app, tokio-based | **tokio-rusqlite** |
| Executor-agnostic, minimal deps | **async-rusqlite** |
| Need pooling & WAL | **nd-async-rusqlite** |
| Experimental/learning | Any (they're similar) |

### 2. **Batch Operations Within Closures**
```rust
// ❌ BAD: Multiple round-trips
let id = conn.call(|c| c.last_insert_rowid()).await?;
let user = conn.call(|c| c.query_row(sql, ..., mapper)).await?;

// ✅ GOOD: Single transaction
let (id, user) = conn.call(|c| {
    c.execute("INSERT INTO users (name) VALUES (?)", [name])?;
    let id = c.last_insert_rowid();
    
    let user = c.query_row(
        "SELECT * FROM users WHERE id = ?",
        [id],
        mapper
    )?;
    
    Ok((id, user))
}).await?;
```

### 3. **Use Transactions for Data Integrity**
```rust
conn.call(|c| {
    let tx = c.transaction()?;
    
    // Multiple operations in a transaction
    tx.execute("INSERT INTO accounts (name) VALUES (?)", [name])?;
    tx.execute("INSERT INTO log (action) VALUES ('account_created')", [])?;
    
    tx.commit()?;
    Ok(())
}).await?;
```

### 4. **Handle Errors Gracefully**
```rust
match conn.call(|c| {
    c.execute("INSERT INTO users (email) VALUES (?)", [email])
}).await {
    Ok(_) => println!("User created"),
    Err(tokio_rusqlite::Error::ConnectionClosed) => {
        eprintln!("Database connection lost");
    }
    Err(tokio_rusqlite::Error::Error(e)) => {
        eprintln!("Database error: {}", e);
    }
}
```

### 5. **Connection Lifecycle Management**
```rust
// ✅ Use try_finally pattern or defer cleanup
struct DbGuard(Connection);

impl Drop for DbGuard {
    fn drop(&mut self) {
        // In async context, you can't call async on drop
        // Better: explicitly call close() in main flow
    }
}

// ✅ Better: explicit cleanup
async fn database_operation() -> Result<()> {
    let conn = Connection::open("db.sqlite").await?;
    
    // ... do work ...
    
    conn.close().await?; // Explicit cleanup
    Ok(())
}
```

### 6. **Avoid Holding Locks Across await**
```rust
// ❌ BAD: Lock held across await
let db_lock = db.lock().unwrap();
let result = db_lock.call(|c| /* ... */).await?;

// ✅ GOOD: Get reference, release lock, then await
let conn_handle = {
    let db = db.lock().unwrap();
    db.clone() // Connection is cheap to clone (wrapped in Arc)
};
let result = conn_handle.call(|c| /* ... */).await?;
```

### 7. **Connection Cloning**
```rust
// Connection is cheap to clone (internally Arc-wrapped)
let conn1 = conn.clone();
let conn2 = conn.clone();

// Both share the same background thread and queue
tokio::join!(
    conn1.call(|c| { /* query 1 */ }),
    conn2.call(|c| { /* query 2 */ })
);
```

### 8. **WAL Mode for Better Concurrency**
```rust
conn.call(|c| {
    c.execute_batch("PRAGMA journal_mode=WAL")?;
    c.execute_batch("PRAGMA synchronous=NORMAL")?;
    Ok(())
}).await?;
```
- WAL enables concurrent readers + writer
- Trade-off: Extra WAL files on disk
- Especially important for nd-async-rusqlite's WalPool feature

---

## Performance Considerations

### 1. **Thread Overhead**
- Each connection spawns a dedicated OS thread
- **Cost**: ~2MB per thread on Linux
- **Recommendation**: For multi-connection scenarios, consider pooling wrappers

### 2. **Channel Latency**
- Closure submission: ~100-500ns per operation
- Negligible for real DB work (µs-ms range)
- Context switching overhead can dominate for very fast queries

### 3. **Batch Size**
```rust
// ✅ GOOD: 100 inserts in one closure
conn.call(|c| {
    for item in items {
        c.execute("INSERT ...", [item])?;
    }
    Ok(())
}).await?;

// ❌ BAD: 100 separate calls (100x context switches)
for item in items {
    conn.call(|c| c.execute("INSERT ...", [item])).await?;
}
```
- **100 batched**: ~1-10ms
- **100 individual**: ~100-500ms (50-100x slower)

### 4. **Query Optimization**
```rust
// ❌ N+1 problem still exists in async
let users = conn.call(|c| {
    c.query_map("SELECT id, name FROM users", [], |row| {
        Ok(row.get::<_, i32>(0)?)
    })?
    .collect::<Result<Vec<_>, _>>()
}).await?;

for user_id in users {
    let posts = conn.call(|c| {
        c.query_map("SELECT id, title FROM posts WHERE user_id = ?", [user_id], |row| {
            Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()
    }).await?;
}

// ✅ GOOD: Use JOINs
let data = conn.call(|c| {
    c.query_map(
        "SELECT u.id, u.name, p.id, p.title FROM users u LEFT JOIN posts p ON u.id = p.user_id",
        [],
        |row| { /* ... */ }
    )?
    .collect::<Result<Vec<_>, _>>()
}).await?;
```

### 5. **Memory Usage**
| Wrapper | Per-Connection Memory | Threads |
|---------|----------------------|---------|
| tokio-rusqlite | ~2-3MB | 1 per connection |
| async-rusqlite | ~2-3MB | 1 per connection |
| nd-async-rusqlite + WalPool | ~2-3MB per thread | Pooled |

### 6. **Saturation Behavior**
- **tokio-rusqlite**: Queue grows unbounded; memory pressure if DB can't keep up
- **async-rusqlite**: Sender blocks when channel full; natural backpressure
- **nd-async-rusqlite**: Depends on pool size; WAL helps parallelism

---

## When to Use Each Wrapper

### tokio-rusqlite ✅
- You're using tokio already
- Operations are reasonably fast
- You have a small number of connections
- You value battle-tested, widely-used code
- Example: Web servers, API backends

### async-rusqlite ✅
- You need executor flexibility
- You want minimal dependencies
- You have resource constraints
- You need bounded backpressure
- Example: Embedded systems, multi-runtime libraries

### nd-async-rusqlite ✅
- You need connection pooling
- You want WAL mode for concurrency
- You need advanced error handling (panic recovery)
- You're building a high-concurrency application
- Example: High-traffic applications, distributed systems

---

## Alternative Approaches (Not Recommended)

### 1. **tokio::task::block_in_place**
```rust
let result = tokio::task::block_in_place(|| {
    // This temporarily blocks the tokio thread
    // Dangerous: can starve other tasks if used too much
    connection.execute(sql, params)
});
```
**Cons**: Requires tokio, can starve executor, not portable

### 2. **spawn_blocking (Better)**
```rust
let conn = Arc::new(Mutex::new(rusqlite::Connection::open("db.sqlite")?));

let result = tokio::task::spawn_blocking({
    let conn = Arc::clone(&conn);
    move || {
        let conn = conn.lock().unwrap();
        connection.execute(sql, params)
    }
}).await?;
```
**Cons**: No compile-time guarantee of thread safety, manual Arc/Mutex management

### 3. **SQLx with sqlite (True Async)**
- SQLx provides runtime-async SQLite
- Not based on rusqlite
- Better async semantics
- More overhead, learning curve
- Use if you need true async from the ground up

---

## Summary & Recommendations

| Dimension | tokio-rusqlite | async-rusqlite | nd-async-rusqlite |
|-----------|-----------------|-----------------|-------------------|
| **Maturity** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |
| **Performance** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| **Backpressure** | ⭐⭐ (unbounded) | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| **Pooling** | ❌ | ❌ | ✅ (optional) |
| **Executor-Agnostic** | ❌ (tokio only) | ✅ | ❌ (tokio only) |
| **Learning Curve** | ⭐⭐ | ⭐⭐ | ⭐⭐⭐ |
| **Dependency Count** | 3 | 2 | 2-3 |

### **Default Choice**: tokio-rusqlite
- Most battle-tested and widely used
- Excellent for typical web applications
- Good performance and ergonomics
- Active development and community

### **Alternative if...**
- **You need executor flexibility** → async-rusqlite
- **You need pooling/high concurrency** → nd-async-rusqlite
- **You want true async from scratch** → SQLx with sqlite

---

## References

- **tokio-rusqlite**: https://github.com/programatik29/tokio-rusqlite
- **async-rusqlite**: https://github.com/jsdw/async-rusqlite
- **nd-async-rusqlite**: https://github.com/nathaniel-daniel/nd-async-rusqlite-rs
- **rusqlite**: https://github.com/rusqlite/rusqlite
- **crossbeam-channel**: https://docs.rs/crossbeam-channel/
- **asyncified**: https://crates.io/crates/asyncified
