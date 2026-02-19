# Pool & Token Blacklist Investigation - October 25, 2025

## Problem Statement

The system repeatedly fetches Solana accounts that don't exist on-chain, wasting RPC calls and resources. These "Account not found" errors persist across multiple cycles, indicating the system doesn't learn from failures.

### Observable Symptoms

1. **Repeated Account Fetches** - Same pool accounts are fetched repeatedly:

   ```
   20:49:22 [POOLFETCH] Account not found: 98fvdFEFmLTwHdsvZV2jHgWvXT2SzkHiq3JvffWGpsxs
   20:49:53 [POOLFETCH] Account not found: 98fvdFEFmLTwHdsvZV2jHgWvXT2SzkHiq3JvffWGpsxs
   20:50:25 [POOLFETCH] Account not found: 98fvdFEFmLTwHdsvZV2jHgWvXT2SzkHiq3JvffWGpsxs
   ```

2. **Token Processing Failures** - Same token errors repeat:

   ```
   20:49:00 [POOLDEC] Token decimals not found for 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R
   20:49:04 [POOLDEC] Token decimals not found for 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R
   20:49:04 [POOLDEC] Token decimals not found for 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R
   ```

3. **High Volume** - Dozens of unique accounts repeatedly fail every cycle (every 5-30 seconds)

---

## Current System Architecture

### Token Blacklist System (EXISTS)

**Location**: `src/tokens/`

- **Database**: `data/tokens.db` has `blacklist` table
- **Schema**: `mint, reason, source, added_at`
- **API**: `add_to_blacklist()`, `is_blacklisted()`, `remove_from_blacklist()`
- **Store**: `filtered_store.rs` tracks blacklisted tokens
- **Integration**: Filtering system respects blacklist

### Pool Blacklist System (MISSING)

**Current State**: NO BLACKLIST MECHANISM

- No `blacklist` table in `data/pools.db`
- No tracking of failed account fetches
- No mechanism to prevent re-fetching non-existent accounts
- No tracking of failed pools

---

## Data Flow Analysis

### 1. Pool Discovery → Analyzer → Fetcher → Calculator

```
Discovery (discovery.rs)
    ↓ [Discovers pools from APIs]
    ↓ [Sends AnalyzerMessage::AnalyzePool]
    ↓
Analyzer (analyzer.rs)
    ↓ [Analyzes pool, determines program kind]
    ↓ [Stores PoolDescriptor in pool_directory (HashMap)]
    ↓ [Requests account fetch via fetcher.request_pool_fetch()]
    ↓
Fetcher (fetcher.rs)
    ↓ [Batches accounts into pending_accounts (HashSet)]
    ↓ [Every 500ms: fetch_account_batch() via RPC]
    ↓ [If account not found: logs warning, continues]
    ↓ [Updates account_last_fetch timestamp]
    ↓ [Organizes accounts into PoolAccountBundle]
    ↓
Calculator (calculator.rs)
    ↓ [Receives complete bundles]
    ↓ [Calculates price if all accounts present]
    ↓ [On error: logs warning, continues]
```

### 2. Key Observations

**Problem**: `pool_directory` is in-memory only (`HashMap<Pubkey, PoolDescriptor>`)

- No persistence of discovered pools
- No tracking of failure reasons
- No blacklist check before fetching

**Fetcher Logic** (`fetcher.rs:255-302`):

```rust
async fn add_stale_accounts_to_pending(...) {
    for pool in pools {
        for account in &pool.reserve_accounts {
            let needs_fetch = match last_fetch_map.get(account) {
                Some(last_time) => last_time.elapsed().as_secs() > threshold,
                None => true, // ← ALWAYS FETCHES IF NEVER SEEN
            };
            if needs_fetch {
                pending_accounts.insert(*account); // ← NO BLACKLIST CHECK
            }
        }
    }
}
```

**Account Fetch** (`fetcher.rs:470-480`):

```rust
for (i, account_opt) in account_results.iter().enumerate() {
    if let Some(account) = account_opt {
        // Process account data
    } else {
        missing_accounts.push(accounts[i].to_string());
        logger::warning(LogTag::PoolFetcher, &format!("Account not found: {}", accounts[i]));
        // ← NO BLACKLIST UPDATE, NO PERMANENT TRACKING
    }
}
```

**Analyzer Failure Tracking** (`analyzer.rs:125-180`):

```rust
// In-memory tracking only (resets on restart)
let failed_pairs: Arc<RwLock<HashSet<(Pubkey, Pubkey)>>> = Arc::new(RwLock::new(HashSet::new()));

// Records failure for this run only
if already_failed { /* skip */ }
else { /* try again, mark failed if fails */ }
```

---

## Missing Components

### 1. Pool-Level Blacklist (DATABASE)

**Needed**: `data/pools.db` table for blacklisted pools/accounts

```sql
CREATE TABLE IF NOT EXISTS blacklist_accounts (
    account_pubkey TEXT PRIMARY KEY,
    reason TEXT NOT NULL,              -- "account_not_found", "decode_error", etc.
    source TEXT,                        -- "rpc_fetch", "decoder", "analyzer"
    pool_id TEXT,                       -- Associated pool (if applicable)
    token_mint TEXT,                    -- Associated token (if applicable)
    error_count INTEGER DEFAULT 1,      -- How many times it failed
    first_failed_at INTEGER NOT NULL,   -- Unix timestamp of first failure
    last_failed_at INTEGER NOT NULL,    -- Unix timestamp of last failure
    added_at INTEGER NOT NULL           -- Unix timestamp when blacklisted
);

CREATE TABLE IF NOT EXISTS blacklist_pools (
    pool_id TEXT PRIMARY KEY,
    reason TEXT NOT NULL,               -- "account_not_found", "unsupported_dex", etc.
    token_mint TEXT,                    -- Associated token
    program_id TEXT,                    -- Program ID of pool
    error_count INTEGER DEFAULT 1,
    first_failed_at INTEGER NOT NULL,
    last_failed_at INTEGER NOT NULL,
    added_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_blacklist_accounts_pool ON blacklist_accounts(pool_id);
CREATE INDEX IF NOT EXISTS idx_blacklist_accounts_token ON blacklist_accounts(token_mint);
CREATE INDEX IF NOT EXISTS idx_blacklist_pools_token ON blacklist_pools(token_mint);
```

### 2. Token-Level Blacklist Enhancement

**Current**: Token blacklist exists but not integrated with pools
**Needed**: Bidirectional integration

- When token blacklisted → Remove ALL pools for that token
- When pool repeatedly fails → Blacklist the token (if token-specific failure)

### 3. Fetcher Blacklist Integration

**Location**: `src/pools/fetcher.rs`

**Add before fetching**:

```rust
async fn add_stale_accounts_to_pending(...) {
    for pool in pools {
        // CHECK POOL BLACKLIST
        if is_pool_blacklisted(&pool.pool_id) {
            logger::debug(LogTag::PoolFetcher, &format!("Skipping blacklisted pool: {}", pool.pool_id));
            continue;
        }

        for account in &pool.reserve_accounts {
            // CHECK ACCOUNT BLACKLIST
            if is_account_blacklisted(&account) {
                logger::debug(LogTag::PoolFetcher, &format!("Skipping blacklisted account: {}", account));
                continue;
            }

            // ... existing logic
        }
    }
}
```

**Add after fetch failure**:

```rust
for (i, account_opt) in account_results.iter().enumerate() {
    if account_opt.is_none() {
        missing_accounts.push(accounts[i].to_string());
        logger::warning(LogTag::PoolFetcher, &format!("Account not found: {}", accounts[i]));

        // BLACKLIST ACCOUNT
        add_account_to_blacklist(
            &accounts[i],
            "account_not_found",
            "rpc_fetch",
            None, // pool_id (lookup from pool_directory)
            None  // token_mint (lookup from pool_directory)
        ).await;
    }
}
```

### 4. Discovery Blacklist Check

**Location**: `src/pools/discovery.rs:537-548`

**Add before streaming to analyzer**:

```rust
for pool in deduped.into_iter() {
    // CHECK POOL BLACKLIST
    if is_pool_blacklisted(&pool.pool_id).await {
        logger::debug(LogTag::PoolDiscovery, &format!("Skipping blacklisted pool: {}", pool.pool_id));
        continue;
    }

    // CHECK TOKEN BLACKLIST
    let token_mint = if is_sol_mint(&pool.base_mint.to_string()) {
        pool.quote_mint.to_string()
    } else {
        pool.base_mint.to_string()
    };

    if is_token_blacklisted(&token_mint).await {
        logger::debug(LogTag::PoolDiscovery, &format!("Skipping pool for blacklisted token: {}", token_mint));
        continue;
    }

    // Send to analyzer
    let _ = sender.send(AnalyzerMessage::AnalyzePool { ... });
}
```

### 5. Analyzer Blacklist Persistence

**Location**: `src/pools/analyzer.rs:125-180`

**Replace in-memory tracking with database**:

```rust
// REMOVE: let failed_pairs: Arc<RwLock<HashSet<(Pubkey, Pubkey)>>> = ...

// ADD: Check database before analysis
if is_pool_blacklisted(&pool_id).await {
    logger::debug(LogTag::PoolAnalyzer, "Skipping blacklisted pool");
    continue;
}

// After failure
if descriptor.is_none() {
    // BLACKLIST POOL
    add_pool_to_blacklist(
        &pool_id,
        "analysis_failed",
        Some(token_to_check.to_string()),
        Some(program_id.to_string())
    ).await;
}
```

### 6. Calculator Error Handling

**Location**: `src/pools/calculator.rs`

**Add blacklist on persistent decode errors**:

```rust
Err(e) => {
    logger::warning(LogTag::PoolCalculator, &format!("Failed to calculate price: {}", e));

    // If "Token decimals not found" error repeats, blacklist token
    if e.contains("decimals not found") {
        increment_token_error_count(&token_mint).await;
        if should_blacklist_token(&token_mint).await {
            add_token_to_blacklist(&token_mint, "persistent_decimals_error", "calculator").await;
        }
    }
}
```

---

## Systematic Solution Design

### Phase 1: Database Schema (FOUNDATION)

**File**: `src/pools/db.rs`

1. Add `blacklist_accounts` table
2. Add `blacklist_pools` table
3. Add CRUD operations:
   - `add_account_to_blacklist()`
   - `is_account_blacklisted()`
   - `remove_account_from_blacklist()`
   - `add_pool_to_blacklist()`
   - `is_pool_blacklisted()`
   - `remove_pool_from_blacklist()`
   - `get_blacklist_stats()`

### Phase 2: Fetcher Integration (IMMEDIATE IMPACT)

**File**: `src/pools/fetcher.rs`

1. Add blacklist check in `add_stale_accounts_to_pending()`
2. Add blacklist update in `fetch_account_batch()` on failure
3. Add error counting (increment on repeat failure)

### Phase 3: Discovery Integration (PREVENT PROPAGATION)

**File**: `src/pools/discovery.rs`

1. Add blacklist check before streaming to analyzer
2. Filter out pools for blacklisted tokens
3. Filter out already-blacklisted pools

### Phase 4: Analyzer Integration (PERSIST FAILURES)

**File**: `src/pools/analyzer.rs`

1. Replace in-memory `failed_pairs` with database queries
2. Add blacklist update on analysis failure
3. Add blacklist check before analysis

### Phase 5: Calculator Integration (SMART BLACKLISTING)

**File**: `src/pools/calculator.rs`

1. Track repeated decode errors per token
2. Blacklist tokens with persistent errors (threshold: 5 failures)
3. Blacklist accounts that consistently fail decoding

### Phase 6: Cross-System Coordination

**Files**: `src/tokens/filtered_store.rs`, `src/filtering/engine.rs`

1. Ensure filtered_store respects pool blacklist
2. Add pool blacklist to filtering criteria
3. Bidirectional sync: token blacklist ↔ pool blacklist

---

## Blacklisting Rules & Thresholds

### Immediate Blacklist (Permanent Errors)

- **Account not found** (3 consecutive failures)
- **Invalid account owner** (1 failure)
- **Unsupported DEX program** (1 failure)
- **Token decimals permanently missing** (5 failures)

### Temporary Errors (Don't Blacklist)

- **Network timeout** (transient)
- **RPC rate limit** (transient)
- **Temporary RPC unavailability** (transient)

### Blacklist Expiry (Optional Future Enhancement)

- Accounts: Never expire (chain state immutable)
- Pools: 7 days expiry (may be re-enabled)
- Tokens: Manual review required

---

## Implementation Priority

### P0 (Critical - Immediate RPC Savings)

1. **Database schema** for pool/account blacklist
2. **Fetcher blacklist check** before fetching
3. **Fetcher blacklist update** on "account not found"

### P1 (High - Prevent Waste Propagation)

4. **Discovery blacklist filter** before analyzer
5. **Analyzer blacklist persistence** instead of in-memory

### P2 (Medium - Smart Blacklisting)

6. **Calculator error tracking** and threshold-based blacklisting
7. **Token↔Pool bidirectional blacklist** sync

### P3 (Low - Observability)

8. **Webserver endpoints** for blacklist management
9. **Dashboard UI** for blacklist stats
10. **Manual blacklist management** API

---

## Expected Impact

### RPC Call Reduction

**Current**: ~40 failed accounts × 60 fetches/hour = 2,400 wasted RPC calls/hour

**After P0**: ~0 wasted calls (blacklisted accounts never fetched)

**Savings**: 2,400 RPC calls/hour = **~60,000 calls/day**

### Performance Impact

- **Reduced CPU**: No processing for blacklisted entities
- **Reduced memory**: No tracking of failed pools in-memory
- **Reduced logs**: No repeated warnings for same failures

### Code Quality

- **Systematic solution**: Database-backed, persistent across restarts
- **Observability**: Clear blacklist stats and management
- **Maintainability**: Single source of truth for blacklist state

---

## Testing Strategy

### Unit Tests

- Blacklist CRUD operations
- Blacklist check integration
- Error threshold logic

### Integration Tests

1. Create pool with non-existent account
2. Verify account gets blacklisted after 3 failures
3. Verify account never fetched again
4. Verify pool removed from discovery

### Manual Testing

1. Run bot with logs
2. Observe "Account not found" warnings
3. Verify blacklist table populated
4. Verify no repeat warnings for blacklisted accounts
5. Check RPC stats for reduced call volume

---

## Open Questions

1. **Should we blacklist tokens when their pools fail?**
   - YES if token-specific error (decimals missing, invalid mint)
   - NO if pool-specific error (unsupported DEX)

2. **Should we auto-unblacklist on successful fetch?**
   - NO for accounts (chain state immutable - if not found, won't exist later)
   - MAYBE for pools (could be re-enabled)

3. **Should we expose blacklist management in webserver?**
   - YES for observability
   - YES for manual intervention
   - Add to Phase P3

4. **Should we sync blacklist to tokens.db blacklist table?**
   - YES for tokens (use existing `tokens.db` blacklist)
   - NO for accounts/pools (pools-specific, keep in `pools.db`)

---

## Files to Modify

### Core Implementation

1. `src/pools/db.rs` - Add blacklist tables and CRUD
2. `src/pools/fetcher.rs` - Integrate blacklist checks
3. `src/pools/discovery.rs` - Filter blacklisted before analyzer
4. `src/pools/analyzer.rs` - Persist failures to blacklist
5. `src/pools/calculator.rs` - Track errors, blacklist on threshold
6. `src/pools/blacklist.rs` - **CREATE NEW** - Centralized blacklist API

### Integration

7. `src/tokens/filtered_store.rs` - Respect pool blacklist
8. `src/filtering/engine.rs` - Add pool blacklist to criteria

### Webserver (Optional P3)

9. `src/webserver/routes/pools.rs` - Add blacklist endpoints
10. `templates/pages/pools.html` - Add blacklist UI

---

## Next Steps

**DO NOT START IMPLEMENTATION YET** - Wait for user confirmation

1. Review this investigation document
2. Confirm systematic approach
3. Prioritize phases (P0 → P1 → P2 → P3)
4. Begin with Phase 1: Database schema

---

## References

- Token blacklist: `src/tokens/database.rs:1014-1052`
- Pool discovery: `src/pools/discovery.rs`
- Pool fetcher: `src/pools/fetcher.rs`
- Pool analyzer: `src/pools/analyzer.rs`
- Pool calculator: `src/pools/calculator.rs`
- Database: `data/pools.db`, `data/tokens.db`
