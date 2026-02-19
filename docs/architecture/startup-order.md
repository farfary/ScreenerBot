# Technical Service Startup vs. Logical User-Facing Workflow Order

## Executive Summary

There are **TWO DIFFERENT ORDERS** in ScreenerBot that must be clearly distinguished:

1. **TECHNICAL SERVICE STARTUP ORDER** - How services initialize on boot (CODE LEVEL)
2. **LOGICAL WORKFLOW ORDER** - How tokens are processed for trading (USER PERSPECTIVE)

**Key Finding:** The user is **CORRECT** about the logical workflow order for user-facing documentation.

---

## Part 1: Technical Service Startup Order

**Purpose:** Ensures system dependencies are satisfied during boot.

**Order (by priority):**

```
10  - Events Service
20  - RPC Stats Service
25  - Transactions Service
30  - Pool Service (pool_helpers)
31  - Pool Discovery Service
32  - Pool Fetcher Service
33  - Pool Calculator Service
34  - Pool Analyzer Service
40  - Tokens Service
50  - SOL Price Service
70  - Wallet Service
80  - Positions Service
90  - Filtering Service
100 - Notification Service
110 - ATA Cleanup Service
120 - OHLCV Service
150 - Trader Service (entry/exit monitors)
200 - Webserver Service
250 - Update Check Service
```

**Why This Order?**

- **Pools (30)** must start before **Tokens (40)** because tokens need price data
- **Tokens (40)** must start before **Filtering (90)** because filtering needs token metadata
- **Filtering (90)** must start before **Trader (150)** because trader needs filtered token lists

**Dependencies:**

```rust
// Tokens Service dependencies
fn dependencies(&self) -> Vec<&'static str> {
    vec!["events", "transactions", "pools"]
}

// Filtering Service dependencies
fn dependencies(&self) -> Vec<&'static str> {
    vec!["tokens", "pool_helpers"]
}

// Trader Service dependencies (implicit)
// Requires: tokens, pools, filtering, positions all ready
```

**This is TECHNICAL - not for user documentation!**

---

## Part 2: Logical User-Facing Workflow Order

**Purpose:** Describes how each token is evaluated for trading.

### The User's Claimed Order: Security → Filtering → Price → Trading

**Is this correct? YES!** Here's the evidence:

### Step 1: Token Discovery (Continuous)

```
Pools Module → Discovers new tokens from:
  - DexScreener API
  - GeckoTerminal API
  - Raydium pools
  - Other DEX sources

Result: New tokens added to tokens.db
```

### Step 2: Security Analysis (Background + On-Demand)

**Background Updates:**

```rust
// src/tokens/service.rs - TokensServiceNew
// Continuously updates security data from RugCheck API

Priority-based update loop:
1. Critical (open positions): every 5 minutes
2. High (passed filtering): every 15 minutes
3. Medium (has market data): every 1 hour
4. Low (other tokens): every 6 hours
```

**Database Storage:**

```sql
-- Table: security_rugcheck
mint_authority TEXT,          -- Can mint unlimited tokens?
freeze_authority TEXT,         -- Can freeze accounts?
security_score INTEGER,        -- 0-100 safety score
is_rugged BOOLEAN,            -- Flagged as scam?
security_risks TEXT,          -- JSON array of risks
top_holders TEXT,             -- JSON array of holders
lp_locked_pct REAL,           -- % of LP locked
```

**Evidence from Code:**

```rust
// src/tokens/updates.rs
async fn update_security_data(mint: &str, priority: Priority) {
    let info = rugcheck_client.fetch_token_info(mint).await?;
    // Store mint_authority, freeze_authority, security_score, etc.
    store_security_data(mint, info).await?;
}
```

**Key Point:** Security data is fetched and stored in the database BEFORE filtering runs.

### Step 3: Filtering (Every 30 seconds)

**Filtering Service:**

```rust
// src/filtering/engine.rs - compute_snapshot()

async fn compute_snapshot(config: FilteringConfig) -> FilteringSnapshot {
    // 1. Load ALL tokens from database (includes security data)
    let tokens = get_all_tokens_for_filtering_async().await?;

    // 2. For each token, check filters:
    for token in tokens {
        // Meta filters (age, liquidity, volume)
        sources::meta::evaluate(token, config)?;

        // DexScreener filters (market data)
        sources::dexscreener::evaluate(token, config)?;

        // GeckoTerminal filters (market data)
        sources::geckoterminal::evaluate(token, config)?;

        // RugCheck filters (SECURITY CHECK)
        sources::rugcheck::evaluate(token, config)?;
        //                    ↑
        //                    Uses security_score, mint_authority,
        //                    freeze_authority, is_rugged, etc.
    }

    // 3. Output: passed_tokens list (safe + meets criteria)
}
```

**RugCheck Filter Evaluation:**

```rust
// src/filtering/sources/rugcheck.rs

pub fn evaluate(token: &Token, config: &RugCheckFilters) -> Result<(), FilterRejectionReason> {
    // Check if rugged
    if config.block_rugged_tokens && token.is_rugged {
        return Err(FilterRejectionReason::RugcheckRuggedToken);
    }

    // Check security score
    if let Some(score) = token.security_score {
        if score > config.max_risk_score {
            return Err(FilterRejectionReason::RugcheckRiskScoreTooHigh);
        }
    }

    // Check mint authority
    if !config.allow_mint_authority && token.mint_authority.is_some() {
        return Err(FilterRejectionReason::RugcheckMintAuthorityBlocked);
    }

    // Check freeze authority
    if !config.allow_freeze_authority && token.freeze_authority.is_some() {
        return Err(FilterRejectionReason::RugcheckFreezeAuthorityBlocked);
    }

    // ... more security checks ...

    Ok(())
}
```

**Key Point:** Filtering USES the security data that was already fetched. If a token has `mint_authority` or `freeze_authority` set, it gets rejected here.

### Step 4: Price Calculation (Continuous)

**Pool Service:**

```rust
// src/pools/calculator.rs

// Continuously calculates prices for discovered tokens
// Uses highest-liquidity SOL pair only
// Stores results in cache

async fn calculate_price(pool: &PoolDescriptor) -> PriceResult {
    // Get token reserves from pool
    // Calculate SOL price based on reserves
    // Store in cache with timestamp
}
```

**Key Point:** Price calculation happens independently but BEFORE trader checks it.

### Step 5: Trading Decision (Entry Monitor)

**Entry Evaluation Flow:**

```rust
// src/trader/monitors/entry.rs - monitor_entries()

loop {
    // 1. Get tokens that have prices
    let available_tokens = pools::get_available_tokens();
    //                     ↑ Only includes tokens with fresh price data

    for token in available_tokens {
        // 2. Get price info
        let price_info = pools::get_pool_price(&token)?;

        // 3. Evaluate entry (with safety checks)
        let decision = evaluators::evaluate_entry_for_token(&token, &price_info).await?;

        // 4. Execute trade if approved
        if let Some(decision) = decision {
            executors::execute_trade(&decision).await?;
        }
    }
}
```

**Entry Evaluator:**

```rust
// src/trader/evaluators/entry.rs

pub async fn evaluate_entry_for_token(
    token_mint: &str,
    price_info: &PriceResult,
) -> Result<Option<TradeDecision>, String> {

    // Safety checks:
    // 1. Connectivity check (RPC, DexScreener, RugCheck healthy?)
    check_endpoints_healthy(&["rpc", "dexscreener", "rugcheck"]).await?;

    // 2. Position limits
    safety::check_position_limits().await?;

    // 3. Existing position check
    if safety::has_open_position(token_mint).await? {
        return Ok(None);
    }

    // 4. Re-entry cooldown
    if safety::is_in_reentry_cooldown(token_mint).await? {
        return Ok(None);
    }

    // 5. Blacklist check
    if safety::is_blacklisted(token_mint) {
        return Ok(None);
    }

    // 6. Strategy evaluation (price, volume, indicators)
    evaluators::StrategyEvaluator::check_entry_strategies(token_mint, price_info).await
}
```

**Key Insight:** By the time the trader sees a token:

1. ✅ Security data is already in the database
2. ✅ Token already passed filtering (including security checks)
3. ✅ Price is already calculated and available
4. ✅ Trader just needs to check strategies and execute

---

## Part 3: Where Security Happens in the Flow

### Security Data Collection (Background)

**When:** Continuously, priority-based
**Where:** `src/tokens/service.rs` → `src/tokens/updates.rs` → `src/tokens/security/rugcheck.rs`

```rust
// Automatic security data updates based on priority:

Priority::Critical (5 min):  Tokens with open positions
Priority::High (15 min):     Tokens that passed filtering
Priority::Medium (1 hour):   Tokens with market data
Priority::Low (6 hours):     Other discovered tokens
```

### Security Data Usage (Filtering)

**When:** Every 30 seconds (filtering refresh)
**Where:** `src/filtering/engine.rs` → `src/filtering/sources/rugcheck.rs`

```rust
// Filtering reads security data from database:
async fn apply_all_filters(token: &Token, config: &FilteringConfig) {
    // token.security_score        (from database)
    // token.mint_authority        (from database)
    // token.freeze_authority      (from database)
    // token.is_rugged            (from database)
    // token.security_risks       (from database)

    sources::rugcheck::evaluate(token, &config.rugcheck)?;
}
```

### Security Validation (Trading)

**When:** Before each trade execution
**Where:** `src/trader/evaluators/entry.rs`

```rust
// Trader checks connectivity to RugCheck API:
if let Some(unhealthy) = check_endpoints_healthy(&["rugcheck"]).await {
    return Err(format!("RugCheck API unhealthy: {}", unhealthy));
}

// Also checks blacklist (tokens blacklisted due to security issues):
if safety::is_blacklisted(token_mint) {
    return Ok(None);
}
```

---

## Part 4: What Order Should Website Documentation Use?

### ❌ WRONG: Technical Startup Order

```
1. Events System boots
2. RPC Stats Service starts
3. Transactions Service initializes
4. Pool Service starts (priority 30)
5. Tokens Service starts (priority 40)
6. Filtering Service starts (priority 90)
7. Trader Service starts (priority 150)
```

**Why wrong?** Users don't care about service priorities. This is internal implementation detail.

### ✅ CORRECT: Logical Workflow Order

```
1. Token Discovery
   - Bot discovers new tokens from DEX pools
   - Tokens added to database for tracking

2. Security Analysis
   - RugCheck API fetches token security data
   - Checks: mint authority, freeze authority, rug status
   - Calculates safety score (0-100)
   - Analyzes holder distribution and LP locks
   - Data stored in database

3. Filtering
   - Bot applies your configured filters
   - Checks: age, liquidity, volume, market cap
   - Verifies security criteria (using data from step 2)
   - Rejects unsafe tokens (mint authority, low security score, etc.)
   - Only safe tokens that meet ALL criteria pass

4. Price Calculation
   - Bot monitors prices for passed tokens
   - Uses highest-liquidity SOL trading pair
   - Updates prices every few seconds
   - Tracks price history for analysis

5. Trading Decision
   - Bot evaluates entry strategies
   - Checks position limits and cooldowns
   - Verifies RPC and API connectivity
   - Confirms token still meets criteria
   - Executes trade if all conditions met
```

**Why correct?** This describes the LOGICAL flow from a user's perspective.

---

## Part 5: Code Evidence Summary

### Proof Security Happens Before Filtering

**1. Database Schema:**

```sql
-- Table: tokens
-- Columns include security_score, mint_authority, freeze_authority, is_rugged

-- Table: security_rugcheck
-- Dedicated table for detailed security data

-- Filtering reads from these tables
```

**2. Token Data Structure:**

```rust
// src/tokens/types.rs
pub struct Token {
    pub mint: String,
    pub symbol: String,

    // Security fields (populated before filtering)
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub security_score: Option<i32>,
    pub is_rugged: bool,
    pub security_risks: Vec<SecurityRisk>,

    // Market fields (for filtering)
    pub liquidity_usd: Option<f64>,
    pub volume_24h_usd: Option<f64>,
    pub market_cap_usd: Option<f64>,
}
```

**3. Filtering Engine:**

```rust
// src/filtering/engine.rs
pub async fn compute_snapshot() -> FilteringSnapshot {
    // Step 1: Load tokens WITH security data
    let tokens = get_all_tokens_for_filtering_async().await?;
    //           ↑ This query JOINs security_rugcheck table

    // Step 2: Apply filters using security data
    for token in tokens {
        sources::rugcheck::evaluate(token, config)?;
        //                   ↑ Uses token.security_score,
        //                     token.mint_authority, etc.
    }
}
```

**4. Trader Entry Flow:**

```rust
// src/trader/monitors/entry.rs
pub async fn monitor_entries() {
    // Step 1: Get tokens WITH prices (already filtered)
    let available_tokens = pools::get_available_tokens();
    //                     ↑ Only includes tokens that:
    //                       - Have security data
    //                       - Passed filtering
    //                       - Have price data

    // Step 2: Evaluate strategy and execute
    for token in available_tokens {
        let price_info = pools::get_pool_price(&token)?;
        let decision = evaluate_entry_for_token(&token, &price_info).await?;
        execute_trade(&decision).await?;
    }
}
```

### Proof Price Happens Independently

**Pool Service:**

```rust
// src/pools/service.rs
// Background tasks run continuously:
// - Pool Discovery: finds new pools
// - Pool Fetcher: fetches pool account data
// - Pool Calculator: calculates prices
// - Pool Analyzer: tracks pool metadata

// These run INDEPENDENT of filtering
// But trader only sees tokens that passed filtering
```

**Cache Storage:**

```rust
// src/pools/cache.rs
pub fn get_available_tokens() -> Vec<String> {
    // Returns tokens with fresh price data
    // Does NOT check if token passed filtering
    // That check happens in trader/monitors/entry.rs
}
```

---

## Part 6: Final Answer

### Question: What's the difference?

**TECHNICAL STARTUP ORDER (Code Level):**

- How services boot up in dependency order
- Pools (30) → Tokens (40) → Filtering (90) → Trader (150)
- Needed for correct initialization
- **Internal implementation detail**

**LOGICAL WORKFLOW ORDER (User Perspective):**

- How each token is processed for trading
- Discovery → Security → Filtering → Price → Trading
- What actually happens to a token
- **User-facing explanation**

### Question: Is the user correct about Security → Filtering → Price → Trading?

**YES!** The user is correct for user-facing documentation.

**Evidence:**

1. Security data is fetched and stored BEFORE filtering runs
2. Filtering READS security data from database to reject unsafe tokens
3. Price calculation happens continuously but trader only checks passed tokens
4. Trading evaluation is the last step after all safety checks

### Question: Which order should the WEBSITE document?

**Answer: LOGICAL WORKFLOW ORDER**

The website should explain:

```
1. Discovery (new tokens found)
2. Security Analysis (safety checks)
3. Filtering (apply your criteria + security)
4. Price Monitoring (track prices)
5. Trading (execute based on strategy)
```

**NOT the technical service startup order** - users don't need to know about service priorities.

---

## Part 7: Recommended Website Documentation

### How ScreenerBot Works

**Step 1: Token Discovery**
ScreenerBot continuously monitors Solana DEXs (Raydium, Orca, Meteora, PumpFun, etc.) to discover new token listings. Newly discovered tokens are added to the tracking database for analysis.

**Step 2: Security Analysis**  
Before any trading decisions, ScreenerBot fetches comprehensive security data from RugCheck API:

- Mint authority status (can creator mint unlimited tokens?)
- Freeze authority status (can accounts be frozen?)
- Security risk score (0-100 scale)
- Rug detection (has token been flagged as scam?)
- Holder distribution analysis
- Liquidity provider lock percentage

This security data is continuously updated based on priority (active positions checked every 5 minutes).

**Step 3: Filtering**
ScreenerBot applies your configured filters to determine which tokens meet your trading criteria:

- **Security Filters:** Rejects tokens with mint authority, freeze authority, low security scores, or rug flags
- **Market Filters:** Checks minimum liquidity, volume, market cap, age requirements
- **Technical Filters:** Validates pool depth, holder distribution, LP locks

Only tokens that pass ALL filters (including security checks) are approved for trading consideration.

**Step 4: Price Monitoring**
For approved tokens, ScreenerBot continuously monitors real-time prices:

- Uses highest-liquidity SOL trading pair for accuracy
- Updates prices every few seconds
- Tracks price history for technical analysis
- Calculates price changes and volatility

**Step 5: Trading Decisions**
When a trading opportunity is detected, ScreenerBot evaluates:

- Entry strategy conditions (price action, volume, indicators)
- Position limits (max open positions, allocation)
- Cooldown periods (prevents rapid re-entry)
- Connectivity health (RPC and API status)
- Final safety checks (token still approved, not blacklisted)

If all conditions are met, the trade is executed automatically.

---

## Conclusion

**Two Orders, Two Purposes:**

1. **Technical Startup Order (Internal)**
   - Service initialization dependencies
   - Code implementation detail
   - Not for user documentation

2. **Logical Workflow Order (User-Facing)**
   - How tokens are processed for trading
   - Security → Filtering → Price → Trading
   - Perfect for website documentation

**The user is CORRECT** - the logical workflow order is Security Analysis → Filtering → Price Calculation → Trading Decision.

This is what users need to understand. The technical service startup order is irrelevant to end users.
