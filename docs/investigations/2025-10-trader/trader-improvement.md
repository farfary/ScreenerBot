# Trader UI/UX & System Improvements

> **Project:** ScreenerBot — Advanced Solana DeFi Trading Bot  
> **Document Type:** Design Specification  
> **Version:** 1.1  
> **Last Updated:** November 1, 2025  
> **Status:** 🟢 Ready for Implementation (Phase 1)

---

## Document Overview

This document provides a comprehensive design specification for implementing an advanced Trader management interface in the ScreenerBot dashboard. It includes:

- **Clarifications & Fixes:** Resolved naming ambiguities, unit standardization, exit precedence contract
- **Phase-1 Plan:** Low-risk deliverables without database schema changes
- **Complete UI/UX Design:** Six subtabs with detailed mockups and features
- **Backend Architecture:** Multi-level exits, voting systems, templates, and emergency controls
- **Implementation Roadmap:** Four-phase delivery plan with timelines and acceptance criteria

**Target Audience:** Developers implementing the Trader UI/backend; product owners reviewing scope.

---

# Trader UI/Engine Improvements – Clarifications, Fixes, and Phase‑1 Plan (2025‑11‑01)

This addendum tightens the design with unambiguous contracts and a safe Phase‑1 scope you can ship without DB/schema changes.

## Core decisions (applies to UI and backend)

- Naming (unified):
  - Tabs: Stats, Trailing Stop, Take Profit (ROI), Time Rules, Strategy Control, General Settings.
  - Internal identifiers follow snake_case: `trailing_stop`, `roi`, `time_rules`.
- Units and precision:
  - Percent in UI: 0–100 with % suffix. Persist as decimals 0.0–1.0. Validate bounds.
  - Time: hours everywhere (integers or 1 decimal). Show minutes as derived when helpful.
  - Money: SOL only for logic and storage (never USD in decisions). Display SOL consistently.
  - Precision: compute with f64; comparisons with epsilon=1e-15; display up to 9+ decimals or scientific for tiny values.
- Exit precedence (deterministic, single-flight per position):
  1. Emergency/Panic (global override)
  2. Time Rules with force-close (e.g., max age)
  3. Strategy-driven exits (if applicable)
  4. Take Profit (ROI)
  5. Trailing Stop
  - Once an exit decision begins, lock the position via existing execution/decision cache until completion or timeout; suppress lower-priority rules. Log and emit events for both the winner and any suppressed rules.

## Confusions found → concrete fixes

- Mixed naming (ROI Targets vs Take Profit, Time Override vs Time Rules)
  - Fix: Use “Take Profit (ROI)” and “Time Rules” across UI; `roi`/`time_rules` internally.
- Percent vs decimal ambiguity
  - Fix: Accept 0–100% in UI; persist as decimal; add unit chips and validation.
- Time units mixed (minutes/hours)
  - Fix: Store/display hours; add small text showing minutes for readability when needed.
- Precedence unspecified; potential race conditions
  - Fix: Apply the precedence contract above; document single-flight locking per position.
- Read-only values presented as editable (monitor intervals)
  - Fix: Show as read-only badges with tooltips (“configured in code/config; not editable here”).
- Trailing stop what‑if example inconsistency
  - Fix: The example “If activation = 15%: Trail not active yet (only +19%)” is incorrect. 19% ≥ 15%; update example to a value below 15% (e.g., 12%) or rephrase as “active”.
- Multi-level exits and voting semantics unclear
  - Fix (future): Define partial-exit semantics against aggregate position PnL (not per-lot) unless a per-lot mode is introduced. For voting: separate `priority` (order) from `weight` (quorum contribution); document thresholds.
- Templates vs single source of truth
  - Fix: Templates are presets only; when applied, they transform/persist via the canonical config path (no second store).

## Phase‑1 scope (no schema changes; low‑risk)

- Trader tab scaffolding with six subtabs listed above.
- Config UIs for current single‑level Take Profit, Trailing Stop, and Time Rules using the existing metadata-driven config system. Clear units, validation, and helper text.
- Read‑only badges for non-editable system intervals with tooltips.
- Stats (lightweight): open positions count, locked SOL, last 24h exits (count, average PnL), exit reason breakdown if available. Backend caches results to avoid heavy queries; UI polls every 5–10s.
- Strategy Control (basic): enable/disable existing strategies; link to the Strategies tab for editing.
- Observability: add standardized exit arbitration logs and two events per evaluation (“exit_signal_evaluated”, “exit_signal_applied”) without changing decision logic.

## Phase‑1 acceptance criteria

- UI correctness
  - All forms reflect current config; percent and hour inputs validated; read-only badges shown for system intervals.
  - Subtabs render without blocking; errors degrade to N/A with user feedback.
- Persistence
  - Saves update `data/config.toml` through existing pipeline; hot‑reload works (manual reload endpoint OK).
- Observability
  - New logs include full identifiers and winner rule; suppressed rules listed. Events present in `events.db` and exposed via `/api/events`.
- Safety
  - No DB schema changes; no rule behavior changes; no new RPC patterns.

## Follow‑on phases (sketch)

- Phase‑2: Optional charts/what‑if previews using cached OHLCV; presets library (templates read‑only first).
- Phase‑3: Multi‑level exits, strategy voting modes, per‑position overrides, rule templates persisted (with migrations).
- Phase‑4: Backtests, analytics, and auto‑tuning.

## Quick doc corrections to apply during implementation

- Rename “ROI Targets” → “Take Profit (ROI)”; “Time Override” → “Time Rules”.
- Fix the trailing what‑if example as noted above.
- Add a short “Exit precedence” note near the first mention of exits.
- Add a “Units” note near the top of the Trader section (SOL only; % in UI; hours).

---

# 📊 Comprehensive Trader UI/UX & System Design

> **Document Purpose:** Complete design specification for an advanced Trader management interface with multi-phase implementation roadmap.

## Table of Contents

1. [Clarifications & Phase-1 Plan](#clarifications--phase1-plan) _(see above)_
2. [UI/UX Architecture](#part-1-uiux-architecture)
   - [Tab Structure](#main-tab-structure)
   - [Stats Tab](#1-stats-tab-)
   - [Trailing Stop Tab](#2-trailing-stop-tab-)
   - [Take Profit Tab](#3-take-profit-roi-tab-)
   - [Time Rules Tab](#4-time-rules-tab-)
   - [Strategy Control Tab](#5-strategy-control-tab-)
   - [General Settings Tab](#6-general-settings-tab-)
3. [Advanced Backend System](#part-2-advanced-trader-system-improvements)
   - [Multi-Level Exit System](#1-multi-level-exit-system)
   - [Strategy Voting](#2-strategy-votingweighting-system)
   - [Rule Templates](#3-rule-templates-library)
   - [Position Overrides](#4-per-position-rule-overrides)
   - [Emergency Rules](#5-emergency-exit-rules)
   - [Market Context](#6-market-condition-awareness)
   - [Database Schema](#database-schema-extensions)
   - [API Endpoints](#api-endpoints-backend)
4. [Implementation Roadmap](#part-3-implementation-priorities)
5. [Summary & Next Steps](#summary)

---

## 📊 Part 1: UI/UX Architecture

### Main Tab Structure

**Navigation Location:** Position the Trader tab to the left of "Strategies" in the main navigation bar.

**Subtab Hierarchy:**

```
Trader
├── 📊 Stats              (Real-time performance dashboard)
├── 📈 Trailing Stop      (Advanced trailing stop configuration)
├── 🎯 Take Profit (ROI)  (Multi-level profit targets)
├── ⏱️ Time Rules         (Time-based exit conditions)
├── 🧩 Strategy Control   (Enable/disable/weight strategies)
└── ⚙️ General Settings   (Core trader configuration)
```

> **Note:** All tabs follow the metadata-driven config UI pattern. Exit configuration uses standardized units (see [Core Decisions](#core-decisions-applies-to-ui-and-backend)).

### 1. Stats Tab (📊)

**Purpose:** Real-time trader performance dashboard with key metrics and analysis.

**Layout:** Card-based responsive grid (matches existing dashboard patterns).

**Data Sources:** Positions DB, Events DB, Service Manager status.

**Refresh Rate:** Auto-refresh every 5s (configurable).

#### A. Performance Metrics (Top Row)

```
┌─────────────────┬─────────────────┬─────────────────┬─────────────────┐
│  Win Rate       │  Total Trades   │  Avg Hold Time  │  Best Trade     │
│  ⬆️ 67.3%       │  🔄 142         │  ⏱️ 4.2h        │  💰 +247%       │
│  +5.2% 24h     │  +12 today      │  -0.8h vs avg   │  Token: ABC     │
└─────────────────┴─────────────────┴─────────────────┴─────────────────┘
```

#### B. Exit Strategy Performance (Mid Section)

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Exit Reason Breakdown (Last 30 days)                                   │
├─────────────────────────────────────────────────────────────────────────┤
│  ✅ Take Profit (ROI)  ████████████░░░░  58 trades  Avg: +23.5%        │
│  📈 Trailing Stop      ██████░░░░░░░░░░  32 trades  Avg: +18.2%        │
│  ⏱️ Time Rules         ███░░░░░░░░░░░░░  15 trades  Avg: -12.1%        │
│  🧩 Strategy Signal    ██░░░░░░░░░░░░░░  12 trades  Avg: +8.4%         │
│  👤 Manual Exit        █░░░░░░░░░░░░░░░   8 trades  Avg: +5.2%         │
└─────────────────────────────────────────────────────────────────────────┘
```

> **Implementation Note:** Exit reason tracking requires events with category="position" and subtype="exit". Query last 30 days and group by exit_reason field in payload.

#### C. Current Positions Risk Analysis

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Active Positions Risk Matrix (2 open)                                  │
├─────────────────────────────────────────────────────────────────────────┤
│  Token ABC  │ ⬆️ +15.2%  │ 🎯 ROI: -4.8%  │ 📈 Trail: Active    │ ⏱️ 2.3h│
│             │ Entry: 3.2h│ Next: +20% ROI │ Stop: 0.000123 SOL │        │
├─────────────┼────────────┼────────────────┼────────────────────┼────────┤
│  Token XYZ  │ ⬇️ -8.3%   │ 🎯 ROI: -28.3% │ 📈 Trail: Inactive │⏱️ 24.1h│
│             │ Entry: 1d  │ Next: 143.9h   │ Override trigger   │ WARNING│
└─────────────┴────────────┴────────────────┴────────────────────┴────────┘
```

#### D. System Health & Activity

```
┌──────────────────────┬──────────────────────┬──────────────────────────┐
│  Trader Status       │  Entry Monitor       │  Exit Monitor            │
│  ✅ Running          │  ✅ Active           │  ✅ Active               │
│  2 positions open    │  🔍 Checking 47 tkns │  ⚡ Last check: 0.8s ago│
│  0.145 SOL locked    │  ⏱️ Interval: 30s    │  ⏱️ Interval: 5s         │
└──────────────────────┴──────────────────────┴──────────────────────────┘
```

> **Data Source:** Service Manager health status + Trader service state.

#### Key Features

- **Auto-refresh:** Every 5s (non-blocking; graceful degradation on error)
- **Drill-down:** Click metrics for detailed breakdown modal
- **Export:** CSV/JSON export for performance reports
- **Time range filters:** 24h, 7d, 30d, All
- **Comparison mode:** Compare metrics between time periods

---

### 2. Trailing Stop Tab (📈)

**Purpose:** Configure trailing stop loss with visual preview and what-if analysis.

**Layout:** Split view — Configuration (left) and Live Preview (right).

**Config Source:** `src/config/schemas/trader.rs` → `trailing_stop` section.

#### Left Panel: Configuration

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Trailing Stop Configuration                                            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ⚙️ BASIC SETTINGS                                                       │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ ◉ Enabled    ○ Disabled                                            │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  📊 SINGLE LEVEL (Current)                                               │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Activation Threshold:     [10]% profit                             │ │
│  │ ├─ When position is up 10%, start trailing                         │ │
│  │                                                                     │ │
│  │ Trail Distance:           [5]% below peak                          │ │
│  │ ├─ Exit if price drops 5% from highest recorded                    │ │
│  │                                                                     │ │
│  │ 💡 Example: Buy at 0.001 SOL → Peak at 0.0011 SOL (+10%)          │ │
│  │            → Trail starts → Exit at 0.001045 SOL (-5% from peak)  │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  🎯 MULTI-LEVEL (Advanced - Future)                          [UPGRADE]  │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Level 1:  +10% profit → Trail 5%  → Sell 25%    [+ Add]           │ │
│  │ Level 2:  +25% profit → Trail 7%  → Sell 50%    [+ Add]           │ │
│  │ Level 3:  +50% profit → Trail 10% → Sell 100%   [+ Add]           │ │
│  │                                                                     │ │
│  │ ⚠️ Multi-level requires backend changes                            │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ⏱️ CONDITIONAL RULES (Advanced - Future)                    [UPGRADE]  │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Time-based adjustments:                                            │ │
│  │ • After 24h: Tighten trail to 3% (secure gains)                   │ │
│  │ • After 48h: Tighten trail to 2% (maximize exit)                  │ │
│  │ • Market condition: Widen trail 2x if volume >200% avg            │ │
│  │                                                                     │ │
│  │ 🚧 Not yet implemented                                             │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  [Save Changes]  [Reset to Defaults]  [Test on Historical Data]        │
└─────────────────────────────────────────────────────────────────────────┘
```

#### Right Panel: Live Preview

```
┌─────────────────────────────────────────────────────────────────────────┐
│  📊 LIVE PREVIEW - Apply to Current Positions                           │
├─────────────────────────────────────────────────────────────────────────┤
│  Selected Position: Token ABC (or "Simulate Random")                    │
│                                                                          │
│  ┌─ Price Chart with Trailing Stop Visualization ────────────────────┐  │
│  │                                         Peak: 0.00123 SOL ●        │  │
│  │                                                           │         │  │
│  │          ╱╲                                              │         │  │
│  │         ╱  ╲                                             │         │  │
│  │    ●───╯    ╲                                           ▼         │  │
│  │  Entry      ╲                                   Trail: 0.001168  │  │
│  │  0.001      ╲───────○ Current: 0.00119 SOL              │         │  │
│  │  SOL            ╲                                        │         │  │
│  │                  ╲                                       ○ Exit if │  │
│  │                   ╲                                      price     │  │
│  │                    ╲───────                              drops     │  │
│  │                            ╲                             here      │  │
│  │  └──────┬──────┬──────┬──────┬──────┬──────┬──────┬────────────  │  │
│  │         1h    2h    3h    4h    5h    6h    7h    Now            │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│  Current Status:                                                         │
│  ├─ Entry Price:       0.001000 SOL                                     │
│  ├─ Current Price:     0.001190 SOL (+19.0%)                            │
│  ├─ Highest Price:     0.001230 SOL (+23.0%)                            │
│  ├─ Trail Active:      ✅ YES (since +10% at 2.4h)                      │
│  ├─ Trail Stop Price:  0.001168 SOL (-5% from peak)                     │
│  ├─ Distance to Exit:  1.8% drop needed                                 │
│  └─ Estimated Exit:    ~0.001168 SOL (+16.8% profit)                    │
│                                                                          │
│  What-If Analysis:                                                       │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │ If activation = 20%:  Trail not active yet (currently +19%)     │   │
│  │ If activation = 12%:  Trail active (since +12% threshold)       │   │
│  │ If distance = 10%:    Exit at 0.001107 SOL (+10.7% profit)      │   │
│  │ If distance = 2%:     Exit at 0.001205 SOL (+20.5% profit)      │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  [◄ Prev Position]  [Simulate Random]  [Next Position ►]                │
└─────────────────────────────────────────────────────────────────────────┘
```

#### Key Features

- **Real-time validation:** Client-side validation with instant feedback
- **Live preview:** Apply current settings to actual open positions
- **Historical backtest:** "Test on Historical Data" simulates on closed positions
- **Visual chart:** Lightweight-charts integration (Phase 2) showing price + trail levels
- **What-if analysis:** Real-time calculation with different parameters
- **Preset templates:** "Conservative (5%/3%)", "Balanced (10%/5%)", "Aggressive (15%/7%)"

> **Note:** Chart visualization is Phase 2. Phase 1 uses numeric/text-only preview.

---

### 3. Take Profit (ROI) Tab (🎯)

**Purpose:** Configure profit targets (currently single-level; designed for multi-level expansion).

**Layout:** Card-based with level management interface.

**Config Source:** `src/config/schemas/trader.rs` → `roi` section.

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Take Profit (ROI) Configuration                                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ⚙️ BASIC SETTINGS                                                       │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ ◉ Enabled    ○ Disabled                                            │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  🎯 SINGLE TARGET (Current)                                              │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Target Profit:           [2.0]%                                     │ │
│  │ Exit Amount:             100% (full position)                       │ │
│  │                                                                     │ │
│  │ 💡 Sell entire position when profit reaches +2%                    │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  🎯 LADDER TARGETS (Advanced - Future)                       [UPGRADE]  │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ ┌─ Level 1 ─────────────────────────────────────── [Edit] [×] ───┐ │ │
│  │ │  Profit Target: +20%    │  Exit Amount: 25%     │  Priority: 1 │ │ │
│  │ │  Status: Active         │  Triggered: 0 times                   │ │ │
│  │ └─────────────────────────────────────────────────────────────────┘ │ │
│  │                                                                     │ │
│  │ ┌─ Level 2 ─────────────────────────────────────── [Edit] [×] ───┐ │ │
│  │ │  Profit Target: +50%    │  Exit Amount: 50%     │  Priority: 2 │ │ │
│  │ │  Status: Active         │  Triggered: 0 times                   │ │ │
│  │ └─────────────────────────────────────────────────────────────────┘ │ │
│  │                                                                     │ │
│  │ ┌─ Level 3 ─────────────────────────────────────── [Edit] [×] ───┐ │ │
│  │ │  Profit Target: +100%   │  Exit Amount: 100%    │  Priority: 3 │ │ │
│  │ │  Status: Active         │  Triggered: 0 times                   │ │ │
│  │ └─────────────────────────────────────────────────────────────────┘ │ │
│  │                                                                     │ │
│  │ [+ Add Target Level]                                               │ │
│  │                                                                     │ │
│  │ Total Exit Plan: 25% @ +20%, 50% @ +50%, 25% @ +100%              │ │
│  │ ⚠️ Remaining % after all levels: 0% ✓                              │ │
│  │                                                                     │ │
│  │ 🚧 Multi-level requires backend changes                            │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  📊 SIMULATION                                                           │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Test your ladder on a hypothetical position:                       │ │
│  │                                                                     │ │
│  │ Entry: 0.001 SOL  │  Size: 0.01 SOL  │  [Simulate Price Movement] │ │
│  │                                                                     │ │
│  │ Price reaches +25%:  ✅ Level 1 triggered → Sold 25% @ 0.00125    │ │
│  │ Price reaches +60%:  ✅ Level 2 triggered → Sold 50% @ 0.00160    │ │
│  │ Price reaches +120%: ✅ Level 3 triggered → Sold 25% @ 0.00220    │ │
│  │                                                                     │ │
│  │ Total Realized: 0.01475 SOL → +47.5% effective return             │ │
│  │ (vs +120% if sold all at peak)                                     │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  📈 PRESETS                                                              │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ [Conservative]  [Balanced]  [Aggressive]  [Day Trade]             │ │
│  │                                                                     │ │
│  │ Conservative: 50% @ +10%, 50% @ +20%                               │ │
│  │ Balanced:     33% @ +20%, 33% @ +50%, 34% @ +100%                 │ │
│  │ Aggressive:   25% @ +50%, 25% @ +100%, 50% @ +200%                │ │
│  │ Day Trade:    100% @ +5%                                           │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  [Save Changes]  [Reset to Defaults]  [Export/Import Ladder]            │
└─────────────────────────────────────────────────────────────────────────┘
```

#### Key Features

- **Visual ladder builder:** Drag-and-drop level reordering (Phase 3)
- **Validation:** Ensure exit amounts sum correctly (≤100%)
- **Presets:** Quick templates for common strategies
- **Simulation tool:** Test ladder on hypothetical price movements
- **Performance tracking:** "Triggered: X times" from historical data
- **Import/Export:** Save/load ladder configs as JSON

> **Phase 1 Limitation:** Only single-level supported in backend. UI shows multi-level as "coming soon" with [UPGRADE] badges.

---

### 4. Time Rules Tab (⏱️)

**Purpose:** Configure time-based exit rules with conditional logic.

**Layout:** Timeline visualization + rules editor.

**Config Source:** `src/config/schemas/trader.rs` → `time_override` section.

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Time-Based Exit Rules                                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  📊 VISUAL TIMELINE                                                      │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                                                                     │ │
│  │  Position Lifecycle Visualization:                                 │ │
│  │                                                                     │ │
│  │  Entry ●───────────────────────────────────────────────────○ Exit  │ │
│  │        0h    24h     48h     72h     96h    120h    144h   168h   │ │
│  │             │        │        │        │        │        │     │   │ │
│  │             ▼        ▼        ▼        ▼        ▼        ▼     ▼   │ │
│  │          Rule 1   Rule 2   Rule 3   Rule 4   Rule 5   Rule 6  Max │ │
│  │          Active   Active   Active   Active   Active   Active  Time│ │
│  │                                                                     │ │
│  │  Current: Token ABC at 73.2h (between Rule 3 and Rule 4)          │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ⚙️ BASIC TIME OVERRIDE (Current)                                        │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Max Hold Duration:        [168] hours (7 days)                     │ │
│  │ Loss Threshold:           [-40]%                                   │ │
│  │                                                                     │ │
│  │ 💡 After 168h, exit if position is down 40% or more               │ │
│  │                                                                     │ │
│  │ Current positions:                                                 │ │
│  │ • Token ABC: 73.2h old, -8.3% → ⏳ 94.8h until rule active        │ │
│  │ • Token XYZ: 24.1h old, +15.2% → ⏳ 143.9h until rule active      │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  🎯 ESCALATION RULES (Advanced - Future)                     [UPGRADE]  │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Time-based rule escalation:                                        │ │
│  │                                                                     │ │
│  │ ┌─ Rule 1: After 24h ──────────────────────────── [Edit] [×] ───┐ │ │
│  │ │  Condition: Position age > 24h AND loss > -20%                 │ │ │
│  │ │  Action: Exit 50% of position                                  │ │ │
│  │ │  Status: Active │ Triggered: 3 times (last 30d)                │ │ │
│  │ └─────────────────────────────────────────────────────────────────┘ │ │
│  │                                                                     │ │
│  │ ┌─ Rule 2: After 48h ──────────────────────────── [Edit] [×] ───┐ │ │
│  │ │  Condition: Position age > 48h AND loss > -30%                 │ │ │
│  │ │  Action: Exit 100% of position (force close)                   │ │ │
│  │ │  Status: Active │ Triggered: 1 time (last 30d)                 │ │ │
│  │ └─────────────────────────────────────────────────────────────────┘ │ │
│  │                                                                     │ │
│  │ ┌─ Rule 3: After 72h ──────────────────────────── [Edit] [×] ───┐ │ │
│  │ │  Condition: Position age > 72h AND profit < +10%               │ │ │
│  │ │  Action: Exit 100% (cut stagnant positions)                    │ │ │
│  │ │  Status: Active │ Triggered: 5 times (last 30d)                │ │ │
│  │ └─────────────────────────────────────────────────────────────────┘ │ │
│  │                                                                     │ │
│  │ ┌─ Rule 4: After 168h (Max) ───────────────────── [Edit] [×] ───┐ │ │
│  │ │  Condition: Position age > 168h (unconditional)                │ │ │
│  │ │  Action: Exit 100% (max hold time reached)                     │ │ │
│  │ │  Status: Active │ Triggered: 0 times (last 30d)                │ │ │
│  │ └─────────────────────────────────────────────────────────────────┘ │ │
│  │                                                                     │ │
│  │ [+ Add Time Rule]                                                  │ │
│  │                                                                     │ │
│  │ 🚧 Escalation rules require backend changes                        │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  📊 RULE EFFECTIVENESS ANALYSIS                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Last 30 days performance by time rule:                             │ │
│  │                                                                     │ │
│  │ Rule 1 (24h, -20%):  █████████░░  3 exits  → Avg: -15.2%         │ │
│  │ Rule 2 (48h, -30%):  ██░░░░░░░░░  1 exit   → Avg: -28.1%         │ │
│  │ Rule 3 (72h, +10%):  ████████░░░  5 exits  → Avg: +4.3%          │ │
│  │ Rule 4 (168h, any):  ░░░░░░░░░░░  0 exits  → N/A                 │ │
│  │                                                                     │ │
│  │ 💡 Consider tightening Rule 3 to +15% (exits too early)           │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  [Save Changes]  [Reset to Defaults]  [Backtest on Historical Data]     │
└─────────────────────────────────────────────────────────────────────────┘
```

#### Key Features

- **Visual timeline:** See all rules on a time axis
- **Live position tracking:** Shows where current positions fall on timeline
- **Effectiveness analysis:** Historical performance per rule
- **Rule builder:** Combine conditions (age AND profit/loss) — Phase 3
- **Presets:** "Conservative", "Balanced", "Aggressive", "Cut Losses Fast"

> **Phase 1 Limitation:** Only basic time override (single max age + loss threshold). Escalation rules are Phase 3.

---

### 5. Strategy Control Tab (🧩)

**Purpose:** Enable/disable/weight strategies for trader integration (strategy creation happens in the Strategies tab).

**Layout:** List of strategies with control toggles and performance metrics.

**Data Source:** `src/strategies/db.rs` + performance tracking from events.

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Strategy Integration Control                                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  💡 Manage which strategies the trader uses for entry/exit decisions    │
│     (Create/edit strategies in the Strategies tab)                      │
│                                                                          │
│  🎯 ENTRY STRATEGIES (3 available)                                       │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ ┌─ Momentum Breakout ──────────────────────────────────────────── │ │
│  │ │  Status: ◉ Enabled  ○ Disabled    │  Weight: [●●●○○] 60%      │ │
│  │ │  Priority: 1 (High)                │  Last signal: 2.3h ago     │ │
│  │ │  Performance: +23.4% avg (12 trades last 30d)                  │ │
│  │ │  [Edit Strategy →]                                              │ │
│  │ └─────────────────────────────────────────────────────────────────┘ │ │
│  │                                                                     │ │
│  │ ┌─ Volume Surge ────────────────────────────────────────────────── │ │
│  │ │  Status: ◉ Enabled  ○ Disabled    │  Weight: [●●○○○] 40%      │ │
│  │ │  Priority: 2 (Medium)              │  Last signal: 45min ago    │ │
│  │ │  Performance: +18.2% avg (8 trades last 30d)                   │ │
│  │ │  [Edit Strategy →]                                              │ │
│  │ └─────────────────────────────────────────────────────────────────┘ │ │
│  │                                                                     │ │
│  │ ┌─ Liquidity Spike ─────────────────────────────────────────────── │ │
│  │ │  Status: ○ Enabled  ◉ Disabled    │  Weight: [○○○○○] 0%       │ │
│  │ │  Priority: 3 (Low)                 │  Last signal: Never        │ │
│  │ │  Performance: N/A (disabled)                                    │ │
│  │ │  [Edit Strategy →]                                              │ │
│  │ └─────────────────────────────────────────────────────────────────┘ │ │
│  │                                                                     │ │
│  │ Total weight: 100% ✓  │  [Enable All]  [Disable All]              │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  🚪 EXIT STRATEGIES (2 available)                                        │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ ┌─ Risk Exit ───────────────────────────────────────────────────── │ │
│  │ │  Status: ◉ Enabled  ○ Disabled    │  Weight: [●●●●○] 80%      │ │
│  │ │  Priority: 1 (High)                │  Last signal: 1.2h ago     │ │
│  │ │  Performance: -8.4% avg (4 exits last 30d)                     │ │
│  │ │  [Edit Strategy →]                                              │ │
│  │ └─────────────────────────────────────────────────────────────────┘ │ │
│  │                                                                     │ │
│  │ ┌─ Profit Secure ───────────────────────────────────────────────── │ │
│  │ │  Status: ◉ Enabled  ○ Disabled    │  Weight: [●○○○○] 20%      │ │
│  │ │  Priority: 2 (Medium)              │  Last signal: 8.5h ago     │ │
│  │ │  Performance: +31.2% avg (7 exits last 30d)                    │ │
│  │ │  [Edit Strategy →]                                              │ │
│  │ └─────────────────────────────────────────────────────────────────┘ │ │
│  │                                                                     │ │
│  │ Total weight: 100% ✓  │  [Enable All]  [Disable All]              │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ⚙️ STRATEGY VOTING MODE (Advanced - Future)                 [UPGRADE]  │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ How should multiple strategies be combined?                        │ │
│  │                                                                     │ │
│  │ ○ First Signal (current) - First strategy to signal wins          │ │
│  │ ○ Weighted Vote - Strategies vote by weight                       │ │
│  │ ○ Unanimous - All enabled strategies must agree                   │ │
│  │ ○ Majority - >50% of weighted strategies must signal              │ │
│  │                                                                     │ │
│  │ 🚧 Voting modes require backend changes                            │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  📊 STRATEGY PERFORMANCE COMPARISON                                      │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Last 30 days:                                                       │ │
│  │                                                                     │ │
│  │ Momentum Breakout  ████████████░░░░  +23.4%  12 trades            │ │
│  │ Volume Surge       █████████░░░░░░░  +18.2%   8 trades            │ │
│  │ Risk Exit          ██░░░░░░░░░░░░░░   -8.4%   4 exits             │ │
│  │ Profit Secure      ███████████████░  +31.2%   7 exits             │ │
│  │                                                                     │ │
│  │ [View Detailed Analytics →]                                         │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  [Save Changes]  [Reset Weights]  [Create New Strategy →]               │
└─────────────────────────────────────────────────────────────────────────┘
```

#### Key Features

- **Independent control:** Enable/disable per strategy without affecting others
- **Weight system:** Allocate voting power (Phase 3 enhancement)
- **Performance tracking:** Real-time metrics on strategy effectiveness
- **Quick navigation:** "Edit Strategy →" links to Strategies tab
- **Voting modes:** Configurable combination logic (Phase 3)
- **Status monitoring:** "Last signal: X ago" for debugging

> **Phase 1 Limitation:** Enable/disable only. Weight system and voting modes are Phase 3.

---

### 6. General Settings Tab (⚙️)

**Purpose:** Core trader configuration (position sizing, DCA, timing, testing modes).

**Layout:** Form-based with collapsible sections.

**Config Source:** `src/config/schemas/trader.rs` — multiple sections.

```
┌─────────────────────────────────────────────────────────────────────────┐
│  General Trader Settings                                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  🎯 POSITION SIZING                                                      │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Max Open Positions:      [2]    (1-100)                            │ │
│  │ Default Trade Size:      [0.005] SOL                               │ │
│  │ Preset Entry Sizes:      [0.005], [0.01], [0.02], [0.05] SOL      │ │
│  │                          [+ Add Size] [Edit]                        │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  💰 DCA (Dollar Cost Averaging)                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ DCA Enabled:             ◉ Yes  ○ No                               │ │
│  │ DCA Threshold:           [-10]% (enter DCA when down 10%)          │ │
│  │ Max DCA Count:           [2] additional entries                    │ │
│  │ DCA Size:                [50]% of initial position                 │ │
│  │ DCA Cooldown:            [30] minutes between DCA entries          │ │
│  │                                                                     │ │
│  │ 💡 Example: 0.01 SOL initial → DCA #1: 0.005 SOL @ -10%           │ │
│  │                             → DCA #2: 0.005 SOL @ -10% more       │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ⏱️ TIMING & COOLDOWNS                                                   │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Position Close Cooldown:  [15] minutes (before reopening token)   │ │
│  │ Entry Check Concurrency:  [10] tokens (check this many at once)   │ │
│  │ Entry Monitor Interval:   30s (hardcoded - shown for reference)   │ │
│  │ Exit Monitor Interval:    5s (hardcoded - shown for reference)    │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  🧪 TESTING & DEBUG                                                      │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Dry Run Mode:            ○ Yes  ◉ No                               │ │
│  │                          (simulate trades without executing)        │ │
│  │                                                                     │ │
│  │ ⚠️ Current mode: LIVE TRADING - Real transactions will execute     │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  [Save Changes]  [Reset to Defaults]  [Export Config]  [Import Config]  │
└─────────────────────────────────────────────────────────────────────────┘
```

#### Key Features

- **Real-time validation:** Input bounds checking with instant feedback
- **Contextual tooltips:** Hover for detailed explanations
- **Read-only badges:** System intervals shown as non-editable with "ⓘ" tooltip
- **Import/Export:** Save/load entire trader config as JSON
- **Dry-run mode:** Test trading logic without executing real transactions

> **Safety:** Dry-run toggle requires confirmation modal. Live trading mode shows warning banner.

---

## 🎯 Part 2: Advanced Trader System Improvements

> **Note:** Features in this section require backend changes and are phased (see [Implementation Roadmap](#part-3-implementation-priorities)).

### Backend Architecture Enhancements

#### 1. Multi-Level Exit System

**Phase:** 3 (requires DB schema changes)

**Current State:** Single threshold per exit type (one ROI target, one trailing stop config, one time rule).

**Proposed:** Tiered exit system with multiple levels and partial position exits.

```rust
// src/trader/exit/types.rs (NEW FILE)

pub struct ExitRule {
    pub id: String,
    pub rule_type: ExitRuleType,
    pub enabled: bool,
    pub priority: i32,
    pub conditions: Vec<ExitCondition>,
    pub action: ExitAction,
}

pub enum ExitRuleType {
    TrailingStop,
    ROITarget,
    TimeOverride,
    StrategySignal,
}

pub struct ExitCondition {
    pub condition_type: ConditionType,
    pub threshold: f64,
    pub operator: ComparisonOperator,
}

pub enum ConditionType {
    ProfitPercent,
    LossPercent,
    PositionAge,
    PriceFromPeak,
    VolumeChange,
    MarketCondition,
}

pub struct ExitAction {
    pub action_type: ExitActionType,
    pub amount_percent: f64, // 0-100
}

pub enum ExitActionType {
    SellPartial,
    SellAll,
    Notify,
    UpdateRule,
}
```

**Example Multi-Level Trailing Stop:**

```rust
// Level 1: At +10% profit, trail 5%, sell 25%
ExitRule {
    id: "trail_1",
    rule_type: TrailingStop,
    conditions: vec![
        ExitCondition { condition_type: ProfitPercent, threshold: 10.0, operator: GreaterEqual },
        ExitCondition { condition_type: PriceFromPeak, threshold: -5.0, operator: LessEqual },
    ],
    action: ExitAction { action_type: SellPartial, amount_percent: 25.0 },
}

// Level 2: At +25% profit, trail 7%, sell 50%
ExitRule {
    id: "trail_2",
    rule_type: TrailingStop,
    conditions: vec![
        ExitCondition { condition_type: ProfitPercent, threshold: 25.0, operator: GreaterEqual },
        ExitCondition { condition_type: PriceFromPeak, threshold: -7.0, operator: LessEqual },
    ],
    action: ExitAction { action_type: SellPartial, amount_percent: 50.0 },
}
```

**Benefits:**

- Partial profit-taking to lock in gains while maintaining upside exposure
- Flexible risk management with graduated exit strategies
- Per-position customization support

---

#### 2. Strategy Voting/Weighting System

**Phase:** 3 (requires voting engine)

**Current State:** First strategy to signal wins ("first signal" mode).

**Proposed:** Weighted voting with configurable combination modes.

```rust
// src/trader/strategy_voting.rs (NEW FILE)

pub struct StrategyVotingSystem {
    mode: VotingMode,
    entry_strategies: Vec<WeightedStrategy>,
    exit_strategies: Vec<WeightedStrategy>,
}

pub enum VotingMode {
    FirstSignal,    // Current behavior
    WeightedVote,   // Sum weights of signaling strategies
    Unanimous,      // All must agree
    Majority,       // >50% of weight must signal
}

pub struct WeightedStrategy {
    pub strategy_id: String,
    pub enabled: bool,
    pub weight: f64, // 0.0-1.0
    pub priority: i32,
}

impl StrategyVotingSystem {
    pub async fn evaluate_entry(&self, token: &str) -> Result<Option<TradeDecision>, String> {
        let signals: Vec<(String, f64)> = // Collect (strategy_id, weight) for signaling strategies

        match self.mode {
            VotingMode::FirstSignal => // First by priority
            VotingMode::WeightedVote => {
                let total_weight: f64 = signals.iter().map(|(_, w)| w).sum();
                if total_weight >= self.threshold { /* Enter */ }
            }
            VotingMode::Unanimous => {
                if signals.len() == self.entry_strategies.len() { /* All agreed */ }
            }
            VotingMode::Majority => {
                let total_weight: f64 = signals.iter().map(|(_, w)| w).sum();
                if total_weight > 0.5 { /* Majority */ }
            }
        }
    }
}
```

**Benefits:**

- More sophisticated signal combination logic
- Configurable quorum thresholds
- Strategy priority vs. weight distinction

---

#### 3. Rule Templates Library

**Phase:** 2-3 (read-only presets in Phase 2, persisted templates in Phase 3)

**Concept:** Predefined rule sets (presets) that users can quickly apply to trader config.

```rust
// src/trader/rule_templates.rs (NEW FILE)

pub struct RuleTemplate {
    pub name: String,
    pub description: String,
    pub category: TemplateCategory,
    pub exit_rules: Vec<ExitRule>,
    pub trading_style: TradingStyle,
}

pub enum TemplateCategory {
    TrailingStop,
    ROITargets,
    TimeRules,
    Combined,
}

pub enum TradingStyle {
    Conservative,
    Balanced,
    Aggressive,
    DayTrade,
    SwingTrade,
}

// Predefined templates
pub fn get_conservative_trail() -> RuleTemplate {
    RuleTemplate {
        name: "Conservative Trail".to_string(),
        description: "Tight trailing stop to lock in profits early".to_string(),
        category: TemplateCategory::TrailingStop,
        exit_rules: vec![
            // Activate at +5%, trail 3%
            ExitRule { /* ... */ },
        ],
        trading_style: TradingStyle::Conservative,
    }
}

pub fn get_ladder_roi_balanced() -> RuleTemplate {
    RuleTemplate {
        name: "Balanced ROI Ladder".to_string(),
        description: "Three-tier profit taking (20%, 50%, 100%)".to_string(),
        category: TemplateCategory::ROITargets,
        exit_rules: vec![
            // 25% @ +20%
            // 50% @ +50%
            // 25% @ +100%
        ],
        trading_style: TradingStyle::Balanced,
    }
}
```

**API Endpoints:**

```rust
// GET /api/trader/templates
pub async fn get_rule_templates() -> Response { /* List available templates */ }

// POST /api/trader/apply-template
pub async fn apply_template(payload: ApplyTemplateRequest) -> Response { /* Apply to config */ }
```

**Benefits:**

- Instant strategy deployment for beginners
- Proven configurations tested in production
- Easy A/B testing between rule sets

---

#### 4. Per-Position Rule Overrides

**Phase:** 3 (requires DB schema + UI modals)

**Concept:** Allow individual positions to use custom exit rules different from global trader config.

```rust
// src/positions/types.rs (EXTEND Position struct)

pub struct Position {
    // ... existing fields ...

    // NEW: Position-specific exit overrides
    pub exit_overrides: Option<PositionExitOverrides>,
}

pub struct PositionExitOverrides {
    pub trailing_stop: Option<TrailingStopOverride>,
    pub roi_targets: Option<Vec<ROITargetOverride>>,
    pub time_rules: Option<TimeRuleOverride>,
    pub enabled: bool,
}

pub struct TrailingStopOverride {
    pub activation_pct: f64,
    pub distance_pct: f64,
}

pub struct ROITargetOverride {
    pub target_pct: f64,
    pub sell_pct: f64,
}

pub struct TimeRuleOverride {
    pub max_hold_hours: f64,
    pub loss_threshold_pct: f64,
}
```

**UI Integration:**

- In Positions page, add "Override Exit Rules" action button
- Opens modal to set custom rules for specific position
- Override indicator badge on position card

**Benefits:**

- High-conviction trades can use different rules
- Emergency adjustments without changing global config
- Experimental rule testing on subset of positions

---

#### 5. Emergency Exit Rules

**Phase:** 2 (safety-critical; relatively simple to implement)

**Concept:** Global panic rules that override all other exit logic.

```rust
// src/trader/emergency.rs (NEW FILE)

pub struct EmergencyExitRules {
    pub enabled: bool,
    pub rules: Vec<EmergencyRule>,
}

pub struct EmergencyRule {
    pub trigger: EmergencyTrigger,
    pub action: EmergencyAction,
}

pub enum EmergencyTrigger {
    AllPositionsDown { threshold_pct: f64 },        // All positions down X%
    TotalPortfolioLoss { threshold_sol: f64 },      // Lost X SOL total
    WalletBalanceLow { threshold_sol: f64 },        // Wallet < X SOL
    RateLimitExceeded,                              // Can't get prices
    ConnectivityLoss { duration_sec: u64 },         // No RPC for X seconds
    ManualPanic,                                    // User pressed panic button
}

pub enum EmergencyAction {
    CloseAllPositions,
    CloseLosingPositions,
    NotifyOnly,
    PauseTrader,
}
```

**UI Components:**

- Emergency "Panic Button" in header (red, requires double-confirmation)
- Config page for enabling/disabling emergency rules
- Dashboard banner when emergency rules are armed
- Event logging for all emergency triggers

**Benefits:**

- Automatic protection against catastrophic scenarios
- Manual panic button for immediate exit
- Connectivity-aware failsafes

---

#### 6. Market Condition Awareness

**Phase:** 4 (advanced; requires market analysis engine)

**Concept:** Dynamically adjust exit rules based on detected market conditions (volatility, trend, volume).

```rust
// src/trader/market_context.rs (NEW FILE)

pub struct MarketContext {
    pub volatility: VolatilityLevel,
    pub trend: MarketTrend,
    pub volume_profile: VolumeProfile,
}

pub enum VolatilityLevel {
    Low,    // < 2% price movement per hour
    Medium, // 2-5% per hour
    High,   // 5-10% per hour
    Extreme,// > 10% per hour
}

pub enum MarketTrend {
    StrongUp,
    Up,
    Sideways,
    Down,
    StrongDown,
}

pub enum VolumeProfile {
    VeryLow,
    Low,
    Normal,
    High,
    VeryHigh,
}

// Adjust exit rules based on context
impl ExitRule {
    pub fn adjust_for_context(&mut self, context: &MarketContext) {
        match context.volatility {
            VolatilityLevel::High | VolatilityLevel::Extreme => {
                // Widen trailing stop distance (more room to breathe)
                if let Some(trail) = &mut self.trailing_distance_pct {
                    *trail *= 1.5; // 5% → 7.5%
                }
            }
            VolatilityLevel::Low => {
                // Tighten trailing stop (don't give back gains)
                if let Some(trail) = &mut self.trailing_distance_pct {
                    *trail *= 0.75; // 5% → 3.75%
                }
            }
            _ => {}
        }
    }
}
```

**Benefits:**

- Context-aware risk management
- Automatic adaptation to market regime changes
- Reduced whipsaw losses in volatile conditions

---

### Database Schema Extensions

**Phase:** 3 (requires migrations and WAL-mode SQLite best practices)

**Purpose:** Support multi-level rules, overrides, templates, and performance tracking.

```sql
-- Exit rules table (replaces single config values)
CREATE TABLE exit_rules (
    id TEXT PRIMARY KEY,
    rule_type TEXT NOT NULL, -- 'trailing_stop', 'roi_target', 'time_override'
    enabled BOOLEAN NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 0,
    conditions TEXT NOT NULL, -- JSON array of conditions
    action TEXT NOT NULL,     -- JSON exit action
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Position-specific overrides
CREATE TABLE position_exit_overrides (
    position_id INTEGER PRIMARY KEY,
    trailing_stop_override TEXT, -- JSON or NULL
    roi_targets_override TEXT,   -- JSON or NULL
    time_rules_override TEXT,    -- JSON or NULL
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (position_id) REFERENCES positions(id)
);

-- Exit rule performance tracking
CREATE TABLE exit_rule_performance (
    rule_id TEXT NOT NULL,
    position_id INTEGER NOT NULL,
    triggered_at INTEGER NOT NULL,
    entry_price REAL NOT NULL,
    exit_price REAL NOT NULL,
    profit_pct REAL NOT NULL,
    hold_duration_hours REAL NOT NULL,
    FOREIGN KEY (rule_id) REFERENCES exit_rules(id),
    FOREIGN KEY (position_id) REFERENCES positions(id)
);

-- Strategy weights (for voting system)
CREATE TABLE strategy_weights (
    strategy_id TEXT PRIMARY KEY,
    strategy_type TEXT NOT NULL, -- 'entry' or 'exit'
    enabled BOOLEAN NOT NULL DEFAULT 1,
    weight REAL NOT NULL DEFAULT 0.5, -- 0.0-1.0
    priority INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (strategy_id) REFERENCES strategies(id)
);

-- Rule templates library
CREATE TABLE rule_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    category TEXT NOT NULL,
    trading_style TEXT NOT NULL,
    rules_json TEXT NOT NULL, -- JSON array of exit rules
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
```

> **Migration Notes:** Use WAL mode, spawn_blocking for rusqlite ops, append-only pattern. Add indexes on frequently queried columns (rule_id, position_id, strategy_id).

---

### API Endpoints (Backend)

**Phase:** Incremental (Phase 1 for basic, Phase 3 for advanced)

**Location:** `src/webserver/routes/trader.rs`

// Exit rules management
pub fn exit_rules_routes() -> Router<Arc<AppState>> {
Router::new()
.route("/exit-rules", get(get_exit_rules))
.route("/exit-rules", post(create_exit_rule))
.route("/exit-rules/:id", put(update_exit_rule))
.route("/exit-rules/:id", delete(delete_exit_rule))
.route("/exit-rules/enable/:id", post(enable_exit_rule))
.route("/exit-rules/disable/:id", post(disable_exit_rule))
.route("/exit-rules/reorder", post(reorder_exit_rules)) // Change priorities
}

// Rule templates
pub fn template_routes() -> Router<Arc<AppState>> {
Router::new()
.route("/templates", get(get_rule_templates))
.route("/templates/:id", get(get_rule_template))
.route("/templates/apply", post(apply_rule_template))
.route("/templates/save", post(save_custom_template))
}

// Position overrides
pub fn override_routes() -> Router<Arc<AppState>> {
Router::new()
.route("/positions/:id/overrides", get(get_position_overrides))
.route("/positions/:id/overrides", post(set_position_overrides))
.route("/positions/:id/overrides", delete(clear_position_overrides))
}

// Strategy weights
pub fn strategy_weight_routes() -> Router<Arc<AppState>> {
Router::new()
.route("/strategies/weights", get(get_strategy_weights))
.route("/strategies/weights", post(update_strategy_weights))
.route("/strategies/voting-mode", get(get_voting_mode))
.route("/strategies/voting-mode", post(set_voting_mode))
}

// Performance analytics
pub fn analytics_routes() -> Router<Arc<AppState>> {
Router::new()
.route("/analytics/exit-rules", get(get_exit_rule_performance))
.route("/analytics/strategies", get(get_strategy_performance))
.route("/analytics/comparison", post(compare_rule_sets)) // Backtest comparison
}

// Emergency controls
pub fn emergency_routes() -> Router<Arc<AppState>> {
Router::new()
.route("/emergency/panic", post(emergency_close_all))
.route("/emergency/rules", get(get_emergency_rules))
.route("/emergency/rules", post(update_emergency_rules))
}

```

---

## 🎨 Part 3: Implementation Priorities

### Phase 1: Foundation (Immediate — No Schema Changes)

**Timeline:** 1-2 weeks

**Deliverables:**

1. **Trader tab scaffolding**
   - Six subtabs with proper routing
   - TabBar integration following existing patterns
   - Page lifecycle hooks (init, activate, deactivate, dispose)

2. **Config UI for existing settings**
   - Trailing Stop (single-level only)
   - Take Profit / ROI (single target)
   - Time Rules (basic override)
   - General Settings (all current fields)
   - Use metadata-driven form generation

3. **Stats tab (lightweight)**
   - Open positions count, locked SOL
   - Last 24h exits: count, avg PnL
   - Exit reason breakdown (from events if available)
   - Backend caching to avoid heavy queries
   - 5-10s polling with graceful degradation

4. **Strategy Control (basic)**
   - Enable/disable toggle per strategy
   - Link to Strategies tab for editing
   - No weights or voting modes yet

5. **Observability enhancements**
   - Standardized exit arbitration logs with full identifiers
   - Two new event types: "exit_signal_evaluated", "exit_signal_applied"
   - Events queryable via existing `/api/events`

**Files to Create/Modify:**
- `src/webserver/templates/pages/trader.html`
- `src/webserver/templates/scripts/pages/trader.js`
- `src/webserver/templates/styles/pages/trader.css` (optional; use components.css)
- `src/webserver/routes/trader.rs` (extend existing)

**Acceptance Criteria:**
- All forms reflect current config; validation works
- Saves persist via existing config pipeline
- Hot-reload works (manual endpoint OK)
- New logs/events appear with full context
- No DB schema changes; no rule behavior changes

---

### Phase 2: Enhanced UI (Next — Still No Schema Changes)

**Timeline:** 2-3 weeks

**Deliverables:**

1. **Visual previews**
   - Numeric "what-if" analysis on Trailing Stop tab
   - Position risk matrix on Stats tab
   - OHLCV integration for price charts (optional, using cached data)

2. **Rule effectiveness tracking**
   - Query events DB for historical exit performance
   - Per-rule breakdown on Stats tab
   - Time-range filters (24h, 7d, 30d, all)

3. **Preset templates (read-only)**
   - Hardcoded presets in backend (no DB yet)
   - "Apply Template" button transforms config
   - Conservative, Balanced, Aggressive, Day Trade presets

4. **Import/Export**
   - JSON export of entire trader config
   - Import with validation and confirmation modal

5. **Performance comparison**
   - Side-by-side metric comparisons
   - Export performance reports (CSV/JSON)

**Acceptance Criteria:**
- Charts render without blocking (lazy load)
- Templates apply correctly to config
- Import/export roundtrips without data loss
- Performance queries don't degrade UI responsiveness

---

### Phase 3: Backend Enhancements (Future — Requires Schema Changes)

**Timeline:** 4-6 weeks

**Deliverables:**

1. **Multi-level exit rules**
   - New `exit_rules` table
   - Rule engine with precedence/arbitration
   - Partial position exit support
   - UI for building multi-level ladders

2. **Strategy voting/weighting**
   - `strategy_weights` table
   - Voting mode selector (first/weighted/unanimous/majority)
   - Weight allocation UI with validation

3. **Per-position overrides**
   - `position_exit_overrides` table
   - Modal UI in Positions page
   - Override indicator badges

4. **Rule templates (persisted)**
   - `rule_templates` table
   - CRUD operations for custom templates
   - Template library UI

5. **Emergency exit rules**
   - `emergency_rules` config section
   - Panic button in header
   - Automatic trigger monitoring

**DB Migration Plan:**
- Use migrations with rollback support
- Test on copy of production DB first
- Gradual rollout with feature flags

**Acceptance Criteria:**
- All new tables indexed properly
- Migrations run without data loss
- Backward compatibility maintained
- Feature flags allow safe rollout

---

### Phase 4: Analytics & Optimization (Advanced)

**Timeline:** 6-8 weeks

**Deliverables:**

1. **Backtesting engine**
   - Historical simulation on closed positions
   - Compare rule set performance
   - Statistical significance testing

2. **Market condition awareness**
   - Volatility/trend/volume detection
   - Dynamic rule adjustment
   - Context-aware logging

3. **ML-based suggestions**
   - Optimal parameter recommendations
   - Pattern recognition for exit timing
   - Anomaly detection

4. **Portfolio-level analytics**
   - Risk/reward visualization
   - Correlation analysis
   - Drawdown tracking

5. **Auto-tuning**
   - Genetic algorithm for parameter optimization
   - A/B testing framework
   - Performance monitoring dashboard

**Acceptance Criteria:**
- Backtests complete in reasonable time (<5min for 1000 positions)
- ML suggestions are explainable and auditable
- Auto-tuning respects safety constraints
- Analytics don't impact live trading performance

---

## 📝 Summary

This design provides a comprehensive roadmap for building a very advanced Trader management interface with the following characteristics:

### ✅ Strengths

### ✅ Strengths

- **Clean Architecture:** Follows existing codebase patterns (TabBar, DataTable, metadata-driven config, Services integration)
- **Extensibility:** Designed for incremental enhancement without breaking changes
- **User-Friendly:** Visual previews, real-time feedback, contextual help, presets for beginners
- **Advanced Features:** Multi-level exits, strategy voting, per-position overrides, market awareness
- **Performance Tracking:** Analytics and metrics on every rule and strategy
- **Safety:** Emergency controls, validation, what-if analysis, dry-run mode
- **Observability:** Comprehensive logging with full identifiers; structured events for post-hoc analysis
- **Phased Delivery:** Low-risk Phase 1 delivers immediate value without schema changes

### 🎯 Design Principles Applied

1. **Single source of truth:** Config persisted via existing `data/config.toml` pipeline
2. **Metadata-driven UI:** Forms generated from config schemas, not hardcoded
3. **Exit precedence:** Deterministic arbitration with single-flight locking per position
4. **Standardized units:** SOL-only for money; 0-100% in UI; hours for time; f64 precision
5. **Graceful degradation:** Stats and previews handle missing data without blocking
6. **No duplication:** Extend existing modules; no `_v2` or legacy wrappers

### 📋 Recommended Next Steps

1. **Review & approve** Phase 1 scope (see [Clarifications](#clarifications--phase1-plan) at top)
2. **Implement Phase 1** (1-2 weeks):
   - Trader tab scaffolding
   - Config UIs for existing settings
   - Lightweight Stats tab
   - Basic Strategy Control
   - Standardized exit logs/events
3. **User testing** on Phase 1 before proceeding to Phase 2
4. **Iterate** based on feedback and usage patterns
5. **Plan Phase 2** with priorities based on user requests

### 🔗 Cross-References

- **Config patterns:** See `.github/Assistant-instructions.md` → Config System section
- **Frontend patterns:** See existing pages (tokens.js, positions.js, strategies.js)
- **Service integration:** See `src/services/` for health/metrics patterns
- **Events system:** See `src/events/` for structured logging
- **Position management:** See `src/positions/` for state transitions and DCA

---

**Document Version:** 1.1 (2025-11-01)
**Status:** Design Complete — Ready for Phase 1 Implementation
**Maintainer:** Development Team
```
