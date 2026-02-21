# Batch Operations Quick Reference

> Quick lookup for ScreenerBot batch sizes, Solana RPC limits, and proven patterns.

---

## Solana RPC Hard Limits (Network-Enforced)

| RPC Method | Hard Limit | Notes |
|---|---|---|
| `getMultipleAccounts` | **100 accounts** | Will reject >100. Chunk required. |
| `getSignaturesForAddress` | **1000 signatures** | Use `before` cursor to paginate beyond. |
| `getTransaction` | 1 per call | No batch variant — parallelize with semaphore. |
| `sendTransaction` | 1 per call | Serialize or use Jito bundles for multi-tx. |

---

## Production-Proven Batch Sizes

| Operation | Our Value | Why |
|---|---|---|
| Account fetches (`getMultipleAccounts`) | **50** | Half the 100 limit — headroom for retries, avoids 429s on shared RPCs |
| Signature fetches (`getSignaturesForAddress`) | **1000** | Max allowed — always fetch full pages |
| Transaction detail fetches | **50 concurrent** | Balances throughput vs RPC rate limits |
| Bootstrap parallel processing | **10 concurrent** | Conservative — each tx does multiple RPC calls internally |
| Position verification | **10 per batch** | Keeps verification loop responsive |
| OHLCV parallel timeframes | **10 concurrent** | 7 timeframes per token, bounded by semaphore |
| DB bulk queries (SQLite) | **512 per chunk** | SQLite variable limit is 999 — 512 stays safe |
| DB write buffer | **100 items** or **10s** | Whichever comes first — time-based flush prevents stale data |
| API rate limiting | **per-endpoint semaphores** | DexScreener 300/min, GeckoTerminal 30/min, Rugcheck 5/min |

---

## ScreenerBot Tuning Cheat Sheet

| Constant | File | Value | Tunable? |
|---|---|---|---|
| `chunks(100)` | `rpc/client/methods.rs` | 100 | ❌ Solana hard limit |
| `RPC_BATCH_SIZE` | `transactions/utils.rs` | 1000 | ❌ Solana hard limit |
| `ACCOUNT_BATCH_SIZE` | `pools/fetcher.rs` | 50 | ✅ Safe: 20–100 |
| `PROCESS_BATCH_SIZE` | `transactions/utils.rs` | 50 | ✅ Safe: 10–100 |
| `CONCURRENT_BATCH_SIZE` | `transactions/service/config.rs` | 10 | ✅ Safe: 5–20 |
| `VERIFICATION_BATCH_SIZE` | `positions/worker.rs` | 10 | ✅ Safe: 5–50 |
| `PARALLEL_FETCH_LIMIT` | `ohlcvs/service.rs` | 10 | ✅ Safe: 5–20 |
| `DB_BATCH_SIZE` | `pools/database/writer.rs` | 100 | ✅ Safe: 50–500 |
| `DB_WRITE_INTERVAL_SECONDS` | `pools/database/writer.rs` | 10 | ✅ Safe: 1–30 |
| `CHUNK_SIZE` (SQL) | `ohlcvs/database.rs` | 512 | ✅ Safe: 100–900 (SQLite var limit = 999) |
| Sell concurrency | `trader/monitors/exit.rs` | config | ✅ Via `get_sell_concurrency()` |
| Entry check concurrency | `trader/monitors/entry.rs` | config | ✅ Via `get_entry_check_concurrency()` |
| Rate limit semaphores | `tokens/updates/rate_limiter.rs` | per-API | ⚠️ Match upstream API limits |

---

## Common Patterns

### Pattern 1: Basic Chunking for getMultipleAccounts

```rust
// src/rpc/client/methods.rs — chunks at Solana's 100-account hard limit
let mut all_accounts = Vec::with_capacity(pubkeys.len());

for chunk in pubkeys.chunks(100) {
    let keys: Vec<String> = chunk.iter().map(|p| p.to_string()).collect();
    let params = serde_json::json!([keys, { "encoding": "base64" }]);
    // ... RPC call, collect results into all_accounts
}
```

```rust
// src/pools/fetcher.rs — uses 50 (half limit) for resilience
const ACCOUNT_BATCH_SIZE: usize = 50;

for batch in accounts_to_fetch.chunks(ACCOUNT_BATCH_SIZE) {
    match Self::fetch_account_batch(batch).await { /* ... */ }
}
```

### Pattern 2: Parallel Processing with Semaphores

```rust
// src/trader/monitors/exit.rs — bounded concurrency for sell operations
let sell_concurrency = std::cmp::max(1, config::get_sell_concurrency());
let semaphore = Arc::new(Semaphore::new(sell_concurrency));

for position in positions {
    let sem = semaphore.clone();
    eval_tasks.push(tokio::spawn(async move {
        let _permit = sem.acquire().await.unwrap(); // blocks if at limit
        // ... evaluate and execute sell
    }));
}
```

```rust
// src/tokens/updates/rate_limiter.rs — per-endpoint rate limiting
pub struct RateLimitCoordinator {
    dexscreener_batch_sem: Arc<Semaphore>,    // 300/min
    dexscreener_profiles_sem: Arc<Semaphore>, // 60/min
    geckoterminal_sem: Arc<Semaphore>,        // 30/min
    rugcheck_sem: Arc<Semaphore>,             // 5/min
}
```

### Pattern 3: Time-Based Batching (Size OR Timer Flush)

```rust
// src/pools/database/writer.rs — dual-trigger flush
const DB_BATCH_SIZE: usize = 100;
const DB_WRITE_INTERVAL_SECONDS: u64 = 10;

let mut write_buffer = Vec::with_capacity(DB_BATCH_SIZE);
let mut interval = tokio::time::interval(Duration::from_secs(DB_WRITE_INTERVAL_SECONDS));

loop {
    tokio::select! {
        price = rx.recv() => {
            write_buffer.push(price);
            if write_buffer.len() >= DB_BATCH_SIZE {
                flush_write_buffer(&mut write_buffer, &db).await;
            }
        }
        _ = interval.tick() => {
            if !write_buffer.is_empty() {
                flush_write_buffer(&mut write_buffer, &db).await;
            }
        }
    }
}
```

### Pattern 4: Database Bulk Queries (Dynamic Placeholders)

```rust
// src/ohlcvs/database.rs — chunked SQL with dynamic IN clause
const CHUNK_SIZE: usize = 512; // stays under SQLite's 999 variable limit

for chunk in mints.chunks(CHUNK_SIZE) {
    let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT DISTINCT mint FROM ohlcv_candles WHERE mint IN ({})",
        placeholders
    );
    let params: Vec<&dyn rusqlite::ToSql> =
        chunk.iter().map(|m| m as &dyn rusqlite::ToSql).collect();
    let mut stmt = conn.prepare(&query)?;
    // ... execute and collect results
}
```

---

## Rules of Thumb

1. **Never exceed hard limits** — `getMultipleAccounts` = 100, `getSignaturesForAddress` = 1000
2. **Use half the limit** for production batch sizes (headroom for retries and error handling)
3. **Semaphore per resource** — don't let one slow API block others
4. **Time-based flush** — always pair size triggers with interval triggers to prevent data staleness
5. **SQLite bulk ops** — chunk at 512 max (999 variable limit, leave margin)
6. **Pre-allocate** — `Vec::with_capacity()` before batch loops to avoid reallocations
