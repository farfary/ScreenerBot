# ScreenerBot Workflow Order Investigation

**Date:** December 30, 2024  
**Investigation:** Service startup order and workflow description accuracy

---

## Executive Summary

**USER IS CORRECT.** The website documentation incorrectly describes the workflow order. Based on actual service priorities and dependencies in the codebase:

### Actual Order (Service Startup):

1. **Price Calculation** (Pool Services: priority 30-103)
2. **Security Analysis** (Tokens Service: priority 40)
3. **Filtering** (Filtering Service: priority 90)
4. **Trading** (Trader Service: priority 150)

### Website Claims (INCORRECT):

The website states: Discovery → Market Data → Security Analysis → Filtering → Trading

**Problem:** The website suggests Security Analysis happens BEFORE Price Calculation, but the code shows Pool Services (price calculation) start BEFORE Tokens Service (which includes security analysis).

---

## Complete Service Startup Order (By Priority)

Based on `/Users/farhad/Desktop/ScreenerBot/src/services/mod.rs` and individual service implementations:

| Priority | Service Name        | Category              | Dependencies                                                                              |
| -------- | ------------------- | --------------------- | ----------------------------------------------------------------------------------------- |
| 5        | **connectivity**    | Infrastructure        | None                                                                                      |
| 10       | **events**          | Infrastructure        | None                                                                                      |
| 30       | **pool_helpers**    | **PRICE CALCULATION** | transactions                                                                              |
| 30       | **webserver**       | Interface             | None                                                                                      |
| 40       | **tokens**          | **SECURITY ANALYSIS** | events, transactions, pools                                                               |
| 45       | **ohlcv**           | Market Data           | pools, tokens                                                                             |
| 50       | **positions**       | Trading Core          | transactions                                                                              |
| 80       | **transactions**    | Infrastructure        | None                                                                                      |
| 90       | **filtering**       | **FILTERING ENGINE**  | tokens, pool_helpers                                                                      |
| 90       | **wallet**          | Trading Core          | transactions                                                                              |
| 100      | **pool_discovery**  | Price Calculation     | pool_helpers, tokens                                                                      |
| 100      | **rpc_stats**       | Monitoring            | None                                                                                      |
| 101      | **pool_fetcher**    | Price Calculation     | pool_helpers, tokens                                                                      |
| 102      | **pool_calculator** | Price Calculation     | pool_helpers, tokens                                                                      |
| 103      | **pool_analyzer**   | Price Calculation     | pool_helpers, tokens                                                                      |
| 110      | **ata_cleanup**     | Maintenance           | None                                                                                      |
| 120      | **sol_price**       | Market Data           | None                                                                                      |
| 150      | **notification**    | Alerts                | tokens, filtering                                                                         |
| 150      | **trader**          | **TRADE EXECUTION**   | positions, pool_discovery, pool_fetcher, pool_calculator, pool_helpers, tokens, filtering |
| 999      | **update_check**    | Maintenance           | None                                                                                      |

---

## Dependency Chain Analysis

### Tokens Service (Priority 40)

```rust
fn dependencies(&self) -> Vec<&'static str> {
    vec!["events", "transactions", "pools"]  // "pools" = pool_helpers
}
```

**Tokens DEPENDS ON pools being ready first!**

### Filtering Service (Priority 90)

```rust
fn dependencies(&self) -> Vec<&'static str> {
    vec!["tokens", "pool_helpers"]
}
```

**Filtering requires BOTH tokens AND pool_helpers**

### Trader Service (Priority 150)

```rust
fn dependencies(&self) -> Vec<&'static str> {
    vec![
        "positions",
        "pool_discovery",
        "pool_fetcher",
        "pool_calculator",
        "pool_helpers",
        "tokens",
        "filtering",
    ]
}
```

**Trader requires everything to be ready**

---

## Actual Workflow Order

Based on dependency chain and startup sequence:

### 1. **Pool Service (Price Calculation)** - Priority 30

- **File:** `src/services/implementations/pools_service.rs`
- **Purpose:** Initializes pool components, starts discovery, fetcher, calculator
- **Dependencies:** transactions
- **What it does:**
  - Fetches on-chain pool data from DEXs (Raydium, Orca, Meteora, Pumpfun, etc.)
  - Calculates SOL prices from pool reserves
  - Tracks liquidity and price history
  - Provides pricing data to all other services

### 2. **Tokens Service (Security Analysis)** - Priority 40

- **File:** `src/services/implementations/tokens_service.rs`
- **Purpose:** Token database with market data AND security analysis
- **Dependencies:** events, transactions, **pools** ← REQUIRES POOLS FIRST
- **What it does:**
  - Fetches token metadata and market data (DexScreener, GeckoTerminal)
  - **Performs security analysis via RugCheck integration**
  - Tracks mint/freeze authority, holder distribution, risk scores
  - Stores comprehensive token information including security data

### 3. **Filtering Service** - Priority 90

- **File:** `src/services/implementations/filtering_service.rs`
- **Purpose:** Multi-criteria token evaluation
- **Dependencies:** tokens, pool_helpers
- **What it does:**
  - Applies market cap, liquidity, volume thresholds
  - Checks security scores from Tokens Service
  - Evaluates holder distribution
  - Filters out risky tokens
  - Produces passed/rejected token lists

### 4. **Trader Service** - Priority 150

- **File:** `src/trader/service.rs`
- **Purpose:** Automated trade execution
- **Dependencies:** positions, pool_discovery, pool_fetcher, pool_calculator, pool_helpers, tokens, filtering
- **What it does:**
  - Monitors filtered tokens for entry opportunities
  - Executes buy/sell swaps via Jupiter/GMGN
  - Manages DCA entries
  - Handles exit strategies (ROI, stop-loss, trailing stop)

---

## Website Documentation Issues

### File: `/Users/farhad/Desktop/ScreenerBot-Website/app/docs/getting-started/how-it-works/page.tsx`

**Current Order (Lines 61-161):**

```tsx
{
  /* Step 1 */
}
<DocsCard variant="blue">
  <DocsCardHeader icon={Search} title="Token Discovery" variant="blue" />
</DocsCard>;

{
  /* Step 2 */
}
<DocsCard variant="green">
  <DocsCardHeader icon={DollarSign} title="Price Calculation" variant="green" />
</DocsCard>;

{
  /* Step 3 */
}
<DocsCard variant="red">
  <DocsCardHeader icon={Shield} title="Security Analysis" variant="red" />
  <p>Checks RugCheck scores, mint/freeze authority status...</p>
</DocsCard>;

{
  /* Step 4 */
}
<DocsCard variant="purple">
  <DocsCardHeader icon={Target} title="Filtering Engine" variant="purple" />
</DocsCard>;

{
  /* Step 5 */
}
<DocsCard variant="orange">
  <DocsCardHeader icon={Zap} title="Trade Execution" variant="orange" />
</DocsCard>;
```

**Problem:** Steps 2 and 3 are in the wrong order!

### File: `/Users/farhad/Desktop/ScreenerBot-Website/app/page.tsx`

**Current Text (Line 789):**

```tsx
<span className="text-blue-400">Discovery</span>
<span className="text-gray-600">→</span>
<span className="text-purple-400">Market Data</span>
<span className="text-gray-600">→</span>
<span className="text-green-400">Security Analysis</span>
<span className="text-gray-600">→</span>
<span className="text-cyan-400">Intelligent Filtering</span>
<span className="text-gray-600">→</span>
<span className="text-white">You Trade</span>
```

**Problem:** "Market Data" (which includes price calculation) should come before "Security Analysis"

Also, the layered sections (Lines 639-695) are structured as:

- Layer 1: Token Discovery (DexScreener, GeckoTerminal)
- Layer 2: Market Data Enrichment (batch fetching, caching)
- Layer 3: Security Analysis (RugCheck)
- Layer 4: Filtering Engine

**Problem:** Layer 2 and Layer 3 describe the ORDER incorrectly. The code shows:

1. Pool initialization MUST happen before Tokens Service starts
2. Tokens Service includes BOTH market data AND security analysis
3. Both are fetched together by the Tokens Service

---

## Technical Evidence

### From `src/services/implementations/tokens_service.rs`:

```rust
fn priority(&self) -> i32 {
    40 // Before webserver and trader; after core infra
}

fn dependencies(&self) -> Vec<&'static str> {
    vec!["events", "transactions", "pools"]  // REQUIRES POOLS!
}
```

### From `src/services/implementations/pools_service.rs`:

```rust
fn priority(&self) -> i32 {
    30 // Before pool sub-services (31-34) - must initialize components first
}

fn dependencies(&self) -> Vec<&'static str> {
    vec!["transactions"]  // Does NOT depend on tokens
}
```

### From `src/services/implementations/filtering_service.rs`:

```rust
fn priority(&self) -> i32 {
    90
}

fn dependencies(&self) -> Vec<&'static str> {
    // Note: tokens service handles all token data including store, discovery, and security
    vec!["tokens", "pool_helpers"]
}
```

**The comment explicitly states that tokens service handles security!**

---

## Correct Workflow Description

### Service Startup Order:

1. **Infrastructure** (connectivity, events, transactions) - Priority 5-80
2. **Pool Services (Price Calculation)** - Priority 30, 100-103
   - Initialize pool components
   - Discover pools from DEXs
   - Fetch on-chain pool data
   - Calculate token prices in SOL
3. **Tokens Service (Market + Security)** - Priority 40
   - Fetch token metadata
   - Get market data (liquidity, volume, market cap)
   - **Perform security analysis (RugCheck scores, authorities, risk)**
   - Store unified token information
4. **Filtering Engine** - Priority 90
   - Apply market thresholds (liquidity, volume, market cap)
   - Check security scores from Tokens Service
   - Filter by holder distribution
   - Produce passed/rejected lists
5. **Trader** - Priority 150
   - Monitor filtered tokens
   - Execute trades based on strategy
   - Manage positions and exits

### User-Facing Workflow (What Happens Continuously):

1. **Discovery:** Monitor market feeds for new tokens
2. **Price Calculation:** Fetch pool data, calculate accurate SOL prices
3. **Security + Market Analysis:** Evaluate token safety and market metrics (happens together in Tokens Service)
4. **Filtering:** Apply your criteria to find trading opportunities
5. **Execution:** Enter and exit positions based on strategy

---

## Files That Need Updates

### 1. `/Users/farhad/Desktop/ScreenerBot-Website/app/docs/getting-started/how-it-works/page.tsx`

**Lines 61-161:** The Trading Pipeline steps

- Swap Step 2 and Step 3
- Update descriptions to clarify that security analysis happens in the same service as market data enrichment

### 2. `/Users/farhad/Desktop/ScreenerBot-Website/app/page.tsx`

**Line 789:** The workflow summary

- Change order from "Discovery → Market Data → Security Analysis → Filtering → Trade"
- To: "Discovery → Price Calculation → Market + Security Analysis → Filtering → Trade"

**Lines 589-695:** Layer descriptions

- Update Layer 2 to mention it includes price calculation
- Update Layer 3 to clarify security is part of token data enrichment
- Consider merging or reordering layers to match actual flow

---

## Recommended Corrections

### For how-it-works/page.tsx:

**Step 2 - Change from "Price Calculation" to "Security Analysis"**

- Icon: Shield (red)
- Title: "Security Analysis"
- Description: "Checks RugCheck scores, mint/freeze authority status, holder distribution, and filters out known scams and risky tokens."

**Step 3 - Change from "Security Analysis" to "Price Calculation"**

- Icon: DollarSign (green)
- Title: "Price Calculation"
- Description: "Fetches on-chain pool data across major Solana DEX ecosystems, calculates SOL prices from reserves, and tracks liquidity for accurate pricing."

**OR BETTER:** Merge Steps 2 and 3 into one step called "Data Enrichment" that explains both happen together.

### For home page.tsx:

Update the summary flow from:

```
Discovery → Market Data → Security Analysis → Filtering → You Trade
```

To:

```
Discovery → Price + Security Analysis → Filtering → You Trade
```

Or more accurately:

```
Discovery → On-Chain Pricing → Token Analysis → Filtering → You Trade
```

---

## Conclusion

**The user's claim is CORRECT.** The actual service startup order in the code is:

1. **Price Calculation** (Pool Services, priority 30-103)
2. **Security Analysis** (Tokens Service, priority 40) - depends on pools!
3. **Filtering** (Filtering Service, priority 90) - depends on tokens!
4. **Trading** (Trader Service, priority 150) - depends on everything!

The website documentation reverses steps 2 and 3, incorrectly suggesting security analysis happens before price calculation. This needs to be corrected in:

- `/Users/farhad/Desktop/ScreenerBot-Website/app/docs/getting-started/how-it-works/page.tsx` (lines 61-161)
- `/Users/farhad/Desktop/ScreenerBot-Website/app/page.tsx` (line 789 and lines 589-695)

The dependency chain proves that **pools MUST start before tokens**, and tokens service handles BOTH market data AND security analysis together.
