# Async Rusqlite Quick Reference

## Comparison Table

| Feature | tokio-rusqlite | async-rusqlite | nd-async-rusqlite |
|---------|-----------------|-----------------|-------------------|
| **Crates.io** | ✅ `tokio-rusqlite` | ✅ `async-rusqlite` | ✅ `nd-async-rusqlite` |
| **Latest Version** | 0.7.0 (Dec 2024) | 0.5.0 (Nov 2024) | 0.0.0 (Feb 2025) |
| **GitHub Stars** | 900+ | 100+ | 50+ |
| **Thread Model** | 1 per connection | 1 per connection | 1 per connection (or pooled) |
| **Channel Type** | Unbounded | Bounded (configurable) | Unbounded (WalPool) |
| **Backpressure** | ❌ No | ✅ Yes | ⚠️ Configurable |
| **Connection Pooling** | ❌ Manual | ❌ Manual | ✅ Optional (WalPool) |
| **WAL Support** | ✅ Via PRAGMA | ✅ Via PRAGMA | ✅ Built-in (WalPool) |
| **Error Types** | `Error<E>` enum | `anyhow`-compatible | Rich error enum |
| **Safe Code Only** | ✅ forbid(unsafe) | ✅ forbid(unsafe) | ✅ forbid(unsafe) |
| **Panic Safety** | ⚠️ No recovery | ⚠️ No recovery | ✅ Can recover |
| **Executor Agnostic** | ❌ Tokio only | ✅ Any async | ❌ Tokio only |
| **Dependencies** | 3 | 2 | 2-3 |

---

## Installation

### tokio-rusqlite
```toml
[dependencies]
tokio-rusqlite = "0.7"
tokio = { version = "1", features = ["full"] }
```

### async-rusqlite
```toml
[dependencies]
async-rusqlite = "0.5"
tokio = { version = "1", features = ["rt"] }  # or async-std, smol, etc.
```

### nd-async-rusqlite
```toml
[dependencies]
nd-async-rusqlite = "0.0"
tokio = { version = "1", features = ["full"] }

# Optional: for connection pooling with WAL
# nd-async-rusqlite = { version = "0.0", features = ["wal-pool"] }
```

---

## Usage Patterns

### 1. Opening a Connection

#### tokio-rusqlite
```rust
use tokio_rusqlite::Connection;

let conn = Connection::open("db.sqlite").await?;
let conn = Connection::open_in_memory().await?;
```

#### async-rusqlite
```rust
use async_rusqlite::Connection;

let conn = Connection::open("db.sqlite").await?;
let conn = Connection::open_in_memory().await?;

// With custom configuration
let conn = Connection::builder()
    .channel_size(100)  // Bounded channel
    .open("db.sqlite")
    .await?;
```

#### nd-async-rusqlite
```rust
use nd_async_rusqlite::AsyncConnection;

let conn = AsyncConnection::open("db.sqlite").await?;

// With builder for more control
let conn = AsyncConnection::builder()
    .open("db.sqlite")
    .await?;
```

---

### 2. Simple Query

#### tokio-rusqlite
```rust
use tokio_rusqlite::params;

let result = conn.call(|c| {
    c.query_row(
        "SELECT COUNT(*) FROM users WHERE age > ?1",
        params![18],
        |row| row.get::<_, i32>(0)
    )
}).await??;

println!("Adult users: {}", result);
```

#### async-rusqlite
```rust
let count = conn.call(|c| {
    c.query_row(
        "SELECT COUNT(*) FROM users WHERE age > ?1",
        [18],  // Note: simpler parameter syntax
        |row| row.get::<_, i32>(0)
    )
}).await??;

println!("Adult users: {}", count);
```

#### nd-async-rusqlite
```rust
use rusqlite::named_params;

let count = conn.access(|c| {
    c.query_row(
        "SELECT COUNT(*) FROM users WHERE age > :age",
        named_params! { ":age": 18 },
        |row| row.get::<_, i32>(0)
    )
}).await??;

println!("Adult users: {}", count);
```

---

### 3. Insert with Last Row ID

#### tokio-rusqlite
```rust
let user_id = conn.call(|c| {
    c.execute(
        "INSERT INTO users (name, email) VALUES (?1, ?2)",
        params!["Alice", "alice@example.com"],
    )?;
    Ok(c.last_insert_rowid())
}).await??;

println!("Created user ID: {}", user_id);
```

#### async-rusqlite
```rust
let user_id = conn.call(|c| {
    c.execute(
        "INSERT INTO users (name, email) VALUES (?1, ?2)",
        ("Alice", "alice@example.com"),
    )?;
    Ok(c.last_insert_rowid())
}).await??;

println!("Created user ID: {}", user_id);
```

#### nd-async-rusqlite
```rust
let user_id = conn.access(|c| {
    c.execute(
        "INSERT INTO users (name, email) VALUES (:name, :email)",
        named_params! {
            ":name": "Alice",
            ":email": "alice@example.com"
        },
    )?;
    Ok(c.last_insert_rowid())
}).await??;

println!("Created user ID: {}", user_id);
```

---

### 4. Batch Insert (Performance Best Practice)

#### tokio-rusqlite
```rust
conn.call(|c| {
    let tx = c.transaction()?;
    
    for (name, email) in users {
        tx.execute(
            "INSERT INTO users (name, email) VALUES (?1, ?2)",
            params![name, email],
        )?;
    }
    
    tx.commit()?;
    Ok(())
}).await??;
```

#### async-rusqlite
```rust
conn.call(|c| {
    let tx = c.transaction()?;
    
    for (name, email) in users {
        tx.execute(
            "INSERT INTO users (name, email) VALUES (?1, ?2)",
            (name, email),
        )?;
    }
    
    tx.commit()?;
    Ok(())
}).await??;
```

#### nd-async-rusqlite
```rust
conn.access(|c| {
    let tx = c.transaction()?;
    
    for (name, email) in users {
        tx.execute(
            "INSERT INTO users (name, email) VALUES (:name, :email)",
            named_params! {
                ":name": name,
                ":email": email
            },
        )?;
    }
    
    tx.commit()?;
    Ok(())
}).await??;
```

---

### 5. Map Rows to Struct

#### tokio-rusqlite
```rust
#[derive(Debug)]
struct User {
    id: i32,
    name: String,
    email: String,
}

let users = conn.call(|c| {
    let mut stmt = c.prepare("SELECT id, name, email FROM users")?;
    
    let users = stmt
        .query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    
    Ok(users)
}).await??;
```

#### async-rusqlite
```rust
let users = conn.call(|c| {
    let mut stmt = c.prepare("SELECT id, name, email FROM users")?;
    
    stmt.query_map([], |row| {
        Ok(User {
            id: row.get(0)?,
            name: row.get(1)?,
            email: row.get(2)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()
}).await??;
```

#### nd-async-rusqlite
```rust
let users = conn.access(|c| {
    let mut stmt = c.prepare("SELECT id, name, email FROM users")?;
    
    stmt.query_map([], |row| {
        Ok(User {
            id: row.get(0)?,
            name: row.get(1)?,
            email: row.get(2)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()
}).await??;
```

---

### 6. Transaction with Rollback

#### All Three (Pattern is Identical)
```rust
conn.call(|c| {
    let tx = c.transaction()?;
    
    match attempt_operation(&tx) {
        Ok(_) => {
            tx.commit()?;
            Ok(())
        }
        Err(e) => {
            tx.rollback()?;
            Err(e)
        }
    }
}).await??;
```

---

### 7. Connection Cleanup

#### tokio-rusqlite
```rust
let conn = Connection::open("db.sqlite").await?;

// ... do work ...

conn.close().await?;  // Explicit close
```

#### async-rusqlite
```rust
let conn = Connection::open("db.sqlite").await?;

// ... do work ...

// Drops automatically; can configure on_close callback
```

#### nd-async-rusqlite
```rust
let conn = AsyncConnection::open("db.sqlite").await?;

// ... do work ...

// Drops automatically; background thread shuts down
```

---

### 8. Multiple Concurrent Operations

#### tokio-rusqlite
```rust
let conn = Connection::open("db.sqlite").await?;

// Connection is cheap to clone (Arc-wrapped)
let conn1 = conn.clone();
let conn2 = conn.clone();

// Operations run sequentially on the shared background thread
let (result1, result2) = tokio::join!(
    conn1.call(|c| c.query_row("SELECT COUNT(*) FROM users", [], |r| r.get::<_, i32>(0))),
    conn2.call(|c| c.query_row("SELECT COUNT(*) FROM posts", [], |r| r.get::<_, i32>(0)))
);

println!("Users: {:?}, Posts: {:?}", result1, result2);
```

#### async-rusqlite & nd-async-rusqlite
```rust
// Same pattern - connection clones are cheap
let conn = Connection::open("db.sqlite").await?;
let conn1 = conn.clone();
let conn2 = conn.clone();

let (result1, result2) = tokio::join!(
    conn1.call(|c| c.query_row("SELECT COUNT(*) FROM users", [], |r| r.get::<_, i32>(0))),
    conn2.call(|c| c.query_row("SELECT COUNT(*) FROM posts", [], |r| r.get::<_, i32>(0)))
);
```

---

### 9. Error Handling

#### tokio-rusqlite
```rust
use tokio_rusqlite::Error;

match conn.call(|c| {
    c.execute("INSERT INTO users (email) VALUES (?1)", params![email])
}).await {
    Ok(_) => println!("User created"),
    Err(Error::ConnectionClosed) => eprintln!("Database offline"),
    Err(Error::Error(e)) => eprintln!("Database error: {}", e),
    Err(Error::Close(_, e)) => eprintln!("Close error: {}", e),
}
```

#### async-rusqlite
```rust
match conn.call(|c| {
    c.execute("INSERT INTO users (email) VALUES (?1)", [email])
}).await {
    Ok(_) => println!("User created"),
    Err(e) => eprintln!("Error: {}", e),
}
```

#### nd-async-rusqlite
```rust
use nd_async_rusqlite::Error;

match conn.access(|c| {
    c.execute("INSERT INTO users (email) VALUES (?1)", [email])
}).await {
    Ok(_) => println!("User created"),
    Err(Error::Rusqlite(e)) => eprintln!("SQLite error: {}", e),
    Err(Error::Aborted) => eprintln!("Connection aborted"),
    Err(Error::AccessPanic(data)) => eprintln!("Panic in access closure"),
    _ => eprintln!("Other error"),
}
```

---

### 10. WAL Mode Setup

#### tokio-rusqlite
```rust
conn.call(|c| {
    c.execute_batch(
        "PRAGMA journal_mode=WAL; \
         PRAGMA synchronous=NORMAL;"
    )?;
    Ok(())
}).await??;
```

#### async-rusqlite
```rust
conn.call(|c| {
    c.execute_batch(
        "PRAGMA journal_mode=WAL; \
         PRAGMA synchronous=NORMAL;"
    )?;
    Ok(())
}).await??;
```

#### nd-async-rusqlite (Use WalPool feature)
```rust
use nd_async_rusqlite::WalPool;

let pool = WalPool::builder()
    .open("db.sqlite")
    .await?;

// WalPool handles WAL mode automatically
let result = pool.access(|c| {
    c.query_row("SELECT ...", [], |row| row.get::<_, String>(0))
}).await??;
```

---

## Decision Flowchart

```
Are you using Tokio?
├─ YES: Do you need executor agnosticism?
│   ├─ YES: async-rusqlite ✅
│   └─ NO: Do you need connection pooling?
│       ├─ YES: nd-async-rusqlite with wal-pool ✅
│       └─ NO: tokio-rusqlite (recommended) ✅⭐
└─ NO: Are you using async-std, smol, or other executor?
    └─ async-rusqlite ✅
```

---

## Common Mistakes to Avoid

### ❌ Don't: Make multiple calls instead of batching
```rust
// SLOW: 100 database round-trips
for item in items {
    conn.call(|c| c.execute("INSERT ...", [item])).await?;
}
```

### ✅ Do: Batch operations
```rust
// FAST: Single database transaction
conn.call(|c| {
    for item in items {
        c.execute("INSERT ...", [item])?;
    }
    Ok(())
}).await??;
```

---

### ❌ Don't: Hold locks across await
```rust
let db = Arc::new(Mutex::new(conn));
let guard = db.lock().unwrap();  // ❌ Lock held here
let result = guard.call(...).await?;  // ❌ Locked during await
```

### ✅ Do: Release locks before await
```rust
let db = Arc::new(Mutex::new(conn));
let conn = {
    let db = db.lock().unwrap();
    db.clone()  // ✅ Get reference, release lock
};
let result = conn.call(...).await?;  // ✅ No lock held
```

---

### ❌ Don't: Ignore backpressure
```rust
// ❌ tokio-rusqlite: Queue grows unbounded
for _ in 0..100000 {
    conn.call(|c| c.execute("INSERT ...", []));  // .await missing!
}
```

### ✅ Do: Await operations
```rust
// ✅ Proper: Await each call
for _ in 0..100000 {
    conn.call(|c| c.execute("INSERT ...", [])).await?;
}
```

---

## Performance Tips

1. **Batch inserts**: 10-100x faster than individual inserts
2. **Use transactions**: Significantly faster for multiple operations
3. **WAL mode**: Enables concurrent readers
4. **Prepared statements**: Pre-compile for repeated queries
5. **Index optimization**: Create indices on frequently queried columns

---

## Size Comparison

```
                Size (Bytes)    Compressed
tokio-rusqlite:    21 KB        ~5 KB
async-rusqlite:    8 KB         ~2 KB
nd-async-rusqlite: 12 KB        ~3 KB
```

---

## When to Switch Wrappers

| Situation | Solution |
|-----------|----------|
| "My database is slow, queue is growing" | Switch to async-rusqlite with bounded channels |
| "I need pooling for high concurrency" | Switch to nd-async-rusqlite with wal-pool |
| "I need to use multiple async runtimes" | Switch to async-rusqlite |
| "I have frequent N+1 queries" | Use JOINs, not wrapper change |
| "I'm getting connection errors" | Increase WAL buffer or use WalPool |

---

## Resources

- [tokio-rusqlite](https://docs.rs/tokio-rusqlite/)
- [async-rusqlite](https://docs.rs/async-rusqlite/)
- [nd-async-rusqlite](https://docs.rs/nd-async-rusqlite/)
- [rusqlite](https://docs.rs/rusqlite/)
- [SQLite Documentation](https://www.sqlite.org/docs.html)
