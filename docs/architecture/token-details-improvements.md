# Token Details Dialog Improvement Plan

Based on a deep investigation of the `ScreenerBot` backend (`Token` struct, API routes) and frontend (`TokenDetailsDialog`), here is a comprehensive review and improvement plan.

## 1. Current State Gap Analysis

### ✅ What is Currently Shown

- **Identity:** Symbol, Name, Mint, Decimals, Age, Tags.
- **Price:** SOL Price, USD Price (approx.), MCap, Liquidity, 24h Vol.
- **Changes:** 5m, 1h, 6h, 24h price changes.
- **Chart:** TradingView chart with OHLCV.
- **Transactions:** 5m-24h buy/sell counts, ratios, and net flow.
- **Security:**
  - Safety Score (0-100 normalized).
  - Authorities (Mint, Freeze).
  - Top 10 Holders concentration.
  - Top Holders List (Address + %, Insider tag).
  - Basic Risks List.
  - Transfer Fee info.
- **Pools:** (In separate tab, list of pools).

### 🔍 What is Available but MISSING / Underutilized

1.  **Security - Raw Score & Details:**
    - We have `security_score` (raw, e.g., 20500) which provides more granular risk detail than the normalized 0-100 score.
    - We have `security_summary` (text description of risk) which is often not shown prominently.
    - `RugcheckInfo` has `token_mutable` and `token_update_authority`, but these are **dropped** during conversion to `Token` struct. We need to add them to the database schema.
2.  **Basic Metadata Missing:**
    - **Total Supply:** `supply` field exists in `Token` but is not displayed in the "Token Info" card.
    - **Banner Image:** `header_image_url` is available. We could use this as a fancy background for the dialog header for a "branded" feel.
    - **Exact Creation Date:** We show relative "Age", but hovering should show the exact `blockchain_created_at` date.
3.  **Top Holders Analysis:**
    - We show a list, but visual distribution (Pie Chart) is much better for understanding concentration at a glance.
    - `lp_provider_count` is shown in text, but could be visualized alongside holders.
4.  **Real-time Price Accuracy:**
    - **CRITICAL ISSUE:** The `get_token_detail` endpoint uses a **hardcoded fallback** `let price_usd = price_sol.map(|p| p * 150.0);` if pool price is used. It _should_ use the real-time `sol_price` service like `get_token_analysis` does.
5.  **Transaction Depth:**
    - We only show _aggregates_ (counts). Users often want to see the _actual_ last 50 transactions to detect snipers or large dumps. The backend currently doesn't expose a "recent transactions" list for the token details, though the system tracks them.
6.  **Socials & Links:**
    - We have `websites` and `socials` vectors. Ensure we display _all_ of them with proper icons (Twitter, Telegram, Discord, Website).
7.  **Pool Depth:**
    - We have `token_pools` table. We can show _secondary_ pools (e.g., Raydium vs Orca prices).

## 2. Backend Improvements (Performance & Completeness)

### A. Fix USD Price Calculation (Immediate Fix)

In `src/webserver/routes/tokens.rs`, `get_token_detail` uses `p * 150.0`.
**Change:** Inject `get_sol_price()` to calculate accurate USD price.

### B. New Endpoint: `GET /api/tokens/:mint/transactions`

Currently, we cannot see individual transactions in the dialog. The backend `TransactionsManager` processes them. We should:

1.  Add a route to fetch recent transactions from `transactions.db` (if we store them per token) or cache recent ones in memory for the active token.
2.  Allow the dialog's "Activity" tab to show a scrolling list of recent buys/sells with "Whale" highlighting.

### C. Distribution API

Instead of sending a raw list of 20 holders and letting JS calculate stats, add a backend field `holder_distribution`:

```rust
struct HolderDistribution {
    top_10: f64,
    lp_locked: f64, // (if we track burn/lock)
    creator: f64,
    other: f64,
}
```

This moves math to Rust and ensures consistency.

## 3. Frontend / UI Suggestions (Creativity & UX)

### 🎨 Renamped Header (Pro Mode)

- **Status Indicators:** Add badges for "Mutable", "Renounced" (if mint_authority is null), "Burnt LP".
- **Real-time Pulse:** Add a "Live" dot that blinks when WebSocket updates arrive (we have pollers, but WS is better).

### 🛡️ Enhanced Security Tab

- **Visual Supply Pie Chart:** A D3.js or Chart.js donut chart showing:
  - Top 10 Holders (Red)
  - LP Pool (Blue)
  - Burned (Grey - if data available)
  - Public (Green)
- **Risk Map:** A "heatmap" of risks instead of a simple list. Low risks = Green, High risks = Red blocks.
- **Sniper Check:** If we have data on "first block buyers", show "Sniper Count".

### 📊 Advanced Activity Tab

- **Buy/Sell Pressure Gauge:** A visual gauge showing the Buy/Sell ratio (0-100%).
- **Whale Watch:** Filter transactions > 10 SOL and show them in a separate list.

### 💧 Pools Tab

- **Arbitrage Opportunities:** List all pools (Raydium, Orca, Meteora) with their current prices. Highlight price differences > 1% to show arb opportunities.

## 4. Proposed Implementation Plan

1.  **Backend Fixes:**
    - Fix `get_token_detail` price calculation.
    - Expose `is_mutable` (derived from authorities) in `TokenDetailResponse`.
    - Ensure `security_summary` is populated.

2.  **Frontend Redesign:**
    - Integrate `Chart.js` (lightweight) for Holder Distribution.
    - Redesign `Security` tab to use grid layout with visual gauges.
    - Add "Copy" buttons for all raw data fields.

3.  **New Features:**
    - **"Sniper Detector"**: If `graph_insiders_detected > 0`, show a prominent warning.
    - **"Dev Check"**: If `creator_balance_pct > 5%`, show warning "Dev holding large supply".

This investigation confirms we have rich data in `Token` struct that hides behind simple lists in the UI. Unlocking this data visually will significantly upgrade the "Pro" feel of ScreenerBot.
