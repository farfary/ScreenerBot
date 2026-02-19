# Dust Comments Audit – Full Codebase Scan (Oct 25, 2025)

**Scan Parameters:**

- Pattern: `TODO|FIXME|HACK|XXX|deprecated|removed|ported|backward.compatibility|legacy|old`
- Scope: Full `src/` directory tree
- Total matches: 150+
- Result: Categorized findings below

---

## 1. ACTIONABLE TODOs (26 items) — ✅ KEEP ALL

These are genuine incomplete features or integration tasks. All legitimate work items.

### Service Integration TODOs

- `src/services/implementations/trader_service.rs:1` — TODO: Integrate with new trader module structure
- `src/services/implementations/trader_service.rs:44` — TODO: Integrate with new trader module when ready
- `src/trader/service.rs:27` — TODO: Implement Service trait when services module exports are fixed
- `src/services/implementations/events_service.rs:46` — TODO: Add actual health check if needed

### Webserver Route TODOs

- `src/webserver/routes/system.rs:10` — TODO: Re-enable when trader module is fully integrated
- `src/webserver/routes/system.rs:33` — TODO: Re-enable critical operations check when trader module is integrated
- `src/webserver/routes/trading.rs:6` — TODO: Re-enable when profit module is refactored
- `src/webserver/routes/trading.rs:57` — TODO: Get from config when profit module is refactored
- `src/webserver/routes/transactions.rs:211` — TODO: Get from TransactionsManager if exposed
- `src/webserver/snapshot.rs:895` — TODO: Rewire to new API stats trackers under tokens::api

### Positions Module TODOs

- `src/positions/apply.rs:399` — TODO: Implement retry logic if needed
- `src/positions/apply.rs:528` — TODO: Implement retry logic if needed
- `src/trader/manual/orders.rs:45` — TODO: Get position info when positions module is ready
- `src/trader/manual/orders.rs:107` — TODO: Get position info when positions module is ready

### Other TODOs

- `src/pools/decoders/mod.rs:81` — TODO: Add other decoders as needed
- `src/bin_legacy/burn_dust_tokens.rs:237` — TODO: Could add price checking here to determine SOL value

### Debug Binary TODOs (debug_manual_trading.rs — 8 items)

All in `src/bin/debug_manual_trading.rs`:

- Line 430 — TODO: Implement position opening logic
- Line 457 — TODO: Implement position closing logic
- Line 481 — TODO: Implement DCA buy logic
- Line 508 — TODO: Implement partial exit logic
- Line 528 — TODO: Implement list positions logic
- Line 559 — TODO: Implement position inspection logic
- Line 589 — TODO: Implement interactive mode
- Line 605 — TODO: Implement reconcile logic
- Line 635 — TODO: Implement quote testing logic

---

## 2. LEGITIMATE LEGACY/BACKWARD COMPAT REFERENCES (40+) — ✅ KEEP ALL

### Decoder Module Names (Legitimate DEX Identifiers)

- `src/pools/decoders/raydium_legacy_amm.rs` — "Raydium Legacy AMM" is the actual DEX name
- `src/pools/decoders/pumpfun_legacy.rs` — "PumpFun Legacy" is the actual DEX program name
- Throughout codebase, "Legacy AMM" and "Legacy PumpFun" are technical references, not historical dust

### RPC Backward Compatibility Documentation (src/rpc.rs)

These are intentional compatibility wrappers for deprecated API methods:

- Line 1263 — "Backward compatibility: get next available URL"
- Line 1270 — "Backward compatibility: get premium URL"
- Line 1275 — "Backward compatibility: create premium client"
- Line 1280 — "Backward compatibility: create main client"
- Line 1325 — "Backward compatibility: access to current URL as rpc_url field"
- Line 1330 — "Backward compatibility: fallback URLs"
- Line 5240 — "Backward compatibility structure for old config access patterns"

All legitimate explanations of deprecated methods kept for compatibility.

### Token Type Handling (Legacy vs Token-2022)

- `src/pools/swap/programs/raydium_clmm.rs:254,454,458` — "Check owner to determine legacy SPL Token vs Token-2022"
  - This is a legitimate code branch, not a historical note

### Position Fields Marked Deprecated (src/positions/types.rs)

- Line 25 — `effective_entry_price: Option<f64>, // Initial entry price (deprecated, use average_entry_price)`
- Line 26 — `effective_exit_price: Option<f64>,  // Final exit price (deprecated, use average_exit_price)`
- **Rationale:** Old fields kept for backward compatibility; users should migrate to new fields

### Data Cleanup Documentation (Legitimate operational descriptions)

- `src/pools/db.rs:352` — "Cleanup old price history entries"
- `src/pools/db.rs:673` — "Cleanup old database entries"
- `src/events/maintenance.rs:19` — "Cleans up old events and performs database optimization"
- `src/wallet.rs:2065,2093` — "Cleanup old snapshots (keep last 1000)"
- `src/events/db.rs:556` — "Cleanup old events (older than MAX_EVENT_AGE_DAYS)"
- All describe actual current functionality, not historical notes

### Config Documentation Examples (src/config/utils.rs)

- Lines in doc comments showing `(old, new)` variable names — Code examples, not dust

---

## 3. OBSOLETE DUST COMMENTS — ❌ CANDIDATES FOR REMOVAL (15 items)

These describe code/systems that have been removed or refactored. Safe to remove.

### 3.1 References to Removed Code/Implementations

| File                                         | Line | Comment                                                    | Status                                    | Recommendation |
| -------------------------------------------- | ---- | ---------------------------------------------------------- | ----------------------------------------- | -------------- |
| `src/global.rs`                              | 9    | "is*debug*\* functions are removed - use logger module..." | Historical; logger is now used everywhere | **Remove**     |
| `src/wallet.rs`                              | 2147 | "Helper functions removed to avoid lifetime issues..."     | Historical refactoring note               | **Remove**     |
| `src/swaps/jupiter.rs`                       | 8    | "debug flags removed from global; no direct imports..."    | Old architectural note                    | **Remove**     |
| `src/bin_legacy/debug_entry_single_token.rs` | 14   | "start_pool_service removed - pool service now managed..." | Old migration note in debug binary        | **Remove**     |
| `src/tokens/decimals.rs`                     | 3    | "Refactored for new clean architecture..."                 | Historical; implementation is clear       | **Remove**     |

### 3.2 Section Headers Needing Clarification

| File                | Line | Current                                             | Recommendation                                                  |
| ------------------- | ---- | --------------------------------------------------- | --------------------------------------------------------------- |
| `src/errors/mod.rs` | 326  | "CONVERSION FUNCTIONS FROM OLD ERRORS"              | Rename to "**BACKWARD COMPAT LAYER**: Error type conversions"   |
| `src/errors/mod.rs` | 347  | "HELPER FUNCTIONS FOR MIGRATION"                    | Rename to "**ERROR PARSING BUILDERS**"                          |
| `src/errors/mod.rs` | 2    | "Replaces old ScreenerBotError with..."             | Optional: remove if implementation is clear                     |
| `src/run.rs`        | 2    | "The old implementation is preserved in run_old.rs" | **Verify** if `run_old.rs` exists; remove comment if it doesn't |

### 3.3 Debug Placeholder Comments (Now Outdated)

All in `src/bin/debug_pool_decoders.rs` — Decoders are now **implemented**:

| Line | Comment                                                          | Action     |
| ---- | ---------------------------------------------------------------- | ---------- |
| 296  | "Note: This is a placeholder - actual CLMM decoder..."           | **Remove** |
| 318  | "Note: This is a placeholder - actual Legacy AMM decoder..."     | **Remove** |
| 340  | "Note: This is a placeholder - actual Whirlpool decoder..."      | **Remove** |
| 362  | "Note: This is a placeholder - actual DAMM decoder..."           | **Remove** |
| 384  | "Note: This is a placeholder - actual DLMM decoder..."           | **Remove** |
| 406  | "Note: This is a placeholder - actual PumpFun AMM decoder..."    | **Remove** |
| 428  | "Note: This is a placeholder - actual PumpFun Legacy decoder..." | **Remove** |
| 450  | "Note: This is a placeholder - actual Moonit AMM decoder..."     | **Remove** |

### 3.4 Frontend Dust Comments

| File                                                  | Line | Comment                                                  | Status                                    | Action                                 |
| ----------------------------------------------------- | ---- | -------------------------------------------------------- | ----------------------------------------- | -------------------------------------- |
| `src/webserver/templates/scripts/pages/strategies.js` | 696  | "Removed createDefaultParameters (unused)"               | Dust; code is gone                        | **Remove**                             |
| `src/webserver/templates/scripts/core/utils.js`       | 1007 | "Keep window.Utils for legacy compatibility..."          | Check if migration complete               | **Remove if done**; clarify if ongoing |
| `src/webserver/templates/scripts/core/poller.js`      | 111  | "Track interval with Router for cleanup (legacy compat)" | May indicate incomplete cleanup           | **Clarify or remove**                  |
| `src/webserver/routes/positions.rs`                   | 15   | "Security database deprecated; security info..."         | Migration complete; comment now redundant | **Remove**                             |

---

## 4. STRUCTURAL SECTION HEADERS (Safe to keep or refactor)

These are used to break up long sections and are organizational markers:

- `// HELPER FUNCTIONS` (various files) — **Keep** as markers
- `// HELPER FUNCTIONS FOR POSITIONS MANAGEMENT` — **Keep** as descriptive markers
- `// CONVERSION FUNCTIONS FROM OLD ERRORS` — **Refactor** to "BACKWARD COMPAT LAYER"
- `// HELPER FUNCTIONS FOR MIGRATION` — **Refactor** to "ERROR PARSING BUILDERS"

---

## 5. SUMMARY TABLE

| Category                        | Count | Status            | Action                                                  |
| ------------------------------- | ----- | ----------------- | ------------------------------------------------------- |
| **Actionable TODOs**            | 26    | ✅ Legitimate     | **KEEP ALL** — ongoing work                             |
| **Legacy/backward compat refs** | 40+   | ✅ Legitimate     | **KEEP ALL** — architectural references                 |
| **Obsolete dust comments**      | 10-15 | ❌ Removable      | **REMOVE** (5 safe removes) + **RENAME** (2-3 sections) |
| **Section headers**             | 5-10  | 🟡 Organizational | **KEEP** most; clarify/rename a few                     |
| **Debug placeholders**          | 8     | ❌ Outdated       | **REMOVE** (all safe)                                   |
| **Frontend dust**               | 4     | ❌ Removable      | **REMOVE** 2-3; clarify 1-2                             |

**Total safe removals:** ~20 items
**Total renames/clarifications:** ~5 items

---

## 6. RECOMMENDED FIXES (Prioritized)

### 🔴 **High Priority** (5 items — 100% safe, low effort)

1. **src/bin/debug_pool_decoders.rs** — Remove all 8 placeholder comments (lines 296, 318, 340, 362, 384, 406, 428, 450)
   - **Effort:** 10 min | **Risk:** None | **Impact:** Cleans up outdated notes about implemented decoders

2. **src/webserver/templates/scripts/pages/strategies.js:696** — Remove "Removed createDefaultParameters (unused)"
   - **Effort:** 1 min | **Risk:** None | **Impact:** Removes dust comment

3. **src/global.rs:9** — Remove "is*debug*\* functions are removed..."
   - **Effort:** 1 min | **Risk:** None | **Impact:** Logger usage is now clear from code

4. **src/wallet.rs:2147** — Remove "Helper functions removed to avoid lifetime issues..."
   - **Effort:** 1 min | **Risk:** None | **Impact:** Implementation is self-evident

5. **src/swaps/jupiter.rs:8** — Remove "debug flags removed from global..."
   - **Effort:** 1 min | **Risk:** None | **Impact:** Clears historical note

### 🟡 **Medium Priority** (4 items — Low effort, improves clarity)

1. **src/errors/mod.rs:326** — Rename to "BACKWARD COMPAT LAYER: Error type conversions for deprecated SystemError"
   - **Effort:** 2 min | **Risk:** Low | **Impact:** Clarifies section purpose

2. **src/errors/mod.rs:347** — Rename to "ERROR PARSING BUILDERS"
   - **Effort:** 2 min | **Risk:** Low | **Impact:** Clarifies section purpose

3. **src/run.rs:2** — Verify if `run_old.rs` exists; remove/update comment accordingly
   - **Effort:** 3 min | **Risk:** Low | **Impact:** Removes stale reference if file doesn't exist

4. **Frontend legacy compat notes** — Review and remove if migration complete:
   - `src/webserver/templates/scripts/core/utils.js:1007` — Check window.Utils usage
   - `src/webserver/templates/scripts/core/poller.js:111` — Clarify Router cleanup status

### 🟢 **Low Priority** (Optional; context still useful)

1. **src/tokens/decimals.rs:3** — "Refactored for new clean architecture..." (can remove; implementation is clear)
2. **src/errors/mod.rs:2** — "Replaces old ScreenerBotError..." (optional; provides context)
3. **src/webserver/routes/positions.rs:15** — "Security database deprecated..." (can remove; migration complete)

---

## 7. IMPLEMENTATION PLAN

### Phase 1: High Priority Removals (15 min total)

1. Remove all 8 placeholders from `src/bin/debug_pool_decoders.rs`
2. Remove 4 single-line dust comments from: global.rs, wallet.rs, swaps/jupiter.rs, strategies.js
3. **Verify:** `cargo check --lib` ✓

### Phase 2: Clarifications (10 min total)

1. Update section headers in `src/errors/mod.rs` (2 comments)
2. Verify `src/run.rs:2` reference
3. **Verify:** `cargo check --lib` ✓

### Phase 3: Optional Frontend Cleanup (10 min)

1. Review window.Utils usage in `src/webserver/templates/scripts/core/utils.js`
2. Clarify Router cleanup in poller.js if needed
3. **Verify:** `npm run check` ✓

---

## 8. FILES READY FOR CHANGES

### High-Confidence (Safe removals)

- ✅ `src/bin/debug_pool_decoders.rs` (8 removals)
- ✅ `src/webserver/templates/scripts/pages/strategies.js` (1 removal)
- ✅ `src/global.rs` (1 removal)
- ✅ `src/wallet.rs` (1 removal)
- ✅ `src/swaps/jupiter.rs` (1 removal)

### Clarifications/Renames

- 🟡 `src/errors/mod.rs` (2 header clarifications)
- 🟡 `src/run.rs` (verify and remove/update 1 comment)

### Optional Frontend Cleanup

- 🟢 `src/webserver/templates/scripts/core/utils.js` (1 comment)
- 🟢 `src/webserver/templates/scripts/core/poller.js` (1 comment)
- 🟢 `src/webserver/routes/positions.rs` (1 removal)

---

## Summary

- **Total dust items found:** 150+
- **Legitimate TODOs (keep):** 26
- **Legitimate legacy refs (keep):** 40+
- **Safe to remove:** ~20
- **Safe to clarify/rename:** ~5

**All TODOs and actionable items are preserved.** Only historical/obsolete comments are candidates for removal.

---

## Audit Date

**October 25, 2025** | Full codebase scan completed
