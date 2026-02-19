# Trader UI Implementation Progress

**Project:** ScreenerBot Trader UI/UX Enhancement  
**Reference:** `TRADER_IMPROVMENT.md` (v1.1)  
**Status:** Phase 1 Complete ✅ | Phase 2 Next 🎯  
**Last Updated:** November 1, 2025

---

## 📊 Implementation Status

### Phase 1: Foundation ✅ COMPLETE

**Timeline:** Completed  
**Goal:** Basic Trader UI without DB schema changes

#### Deliverables Status

| #   | Feature                           | Status      | Notes                                                             |
| --- | --------------------------------- | ----------- | ----------------------------------------------------------------- |
| 1.1 | Trader tab scaffolding            | ✅ Complete | 6 subtabs with routing, TabBar integration                        |
| 1.2 | Config UI - Trailing Stop         | ✅ Complete | Single-level, metadata-driven, positions.trailing_stop.\*         |
| 1.3 | Config UI - Take Profit (ROI)     | ✅ Complete | Single target, positions.roi.\*                                   |
| 1.4 | Config UI - Time Rules            | ✅ Complete | Basic max age + loss threshold, trader.time_override.\*           |
| 1.5 | Config UI - General Settings      | ✅ Complete | Position sizing, DCA, timing, dry-run mode                        |
| 1.6 | Stats Tab (lightweight)           | ✅ Complete | Win rate, total trades, avg hold time, best trade, exit breakdown |
| 1.7 | Strategy Control (basic)          | ✅ Complete | Enable/disable toggle, links to Strategies tab                    |
| 1.8 | Backend API - `/api/trader/stats` | ✅ Complete | Returns metrics from positions DB                                 |
| 1.9 | Observability logs/events         | ✅ Complete | Standardized exit logs with full context                          |

#### Files Created/Modified

**Frontend:**

- ✅ `src/webserver/templates/pages/trader.html` (490 lines)
- ✅ `src/webserver/templates/scripts/pages/trader.js` (782 lines)
- ✅ Uses existing `styles/components.css` (no custom CSS needed)

**Backend:**

- ✅ `src/webserver/routes/trader.rs` (extended to 589 lines)
  - Added `get_trader_stats()` handler
  - Added `TraderStatsResponse` type
  - Added `ExitBreakdownEntry` type

#### Bug Fixes Applied

1. ✅ **Hold Time Display Bug** (Lines ~211 in trader.js)
   - **Issue:** Displayed "147.9µs" instead of "1d 17h 5m"
   - **Root Cause:** Used `formatDuration()` (expects nanoseconds) instead of `formatUptime()` (expects seconds)
   - **Fix:** Changed to `Utils.formatUptime(data.avg_hold_time_hours * 3600, { style: "detailed" })`

2. ✅ **Trailing Stop Config Path** (Trailing Stop tab)
   - **Issue:** Connected to wrong config section (trader.\*)
   - **Root Cause:** Initial implementation used incorrect config path
   - **Fix:** Updated to use `positions.trailing_stop_enabled` and `positions.trailing_stop_*`

#### Acceptance Criteria - All Met ✅

- ✅ All forms reflect current config from `data/config.toml`
- ✅ Percent and hour inputs validated with bounds checking
- ✅ Read-only badges shown for system intervals (entry/exit monitor intervals)
- ✅ Subtabs render without blocking; errors degrade gracefully
- ✅ Saves update config via existing pipeline (`/api/config/update`)
- ✅ Hot-reload works (manual endpoint: `/api/config/reload`)
- ✅ New logs include full identifiers and winner rule
- ✅ Events present in `events.db` and exposed via `/api/events`
- ✅ No DB schema changes (positions.db untouched)
- ✅ No rule behavior changes (exit logic unchanged)
- ✅ No new RPC patterns

---

## 🎯 Phase 2: Enhanced UI (IN PROGRESS - No Schema Changes)

**Timeline:** 2-3 weeks (Started: November 1, 2025)  
**Goal:** Visual previews, templates, import/export, performance tracking

### Status: 40% Complete (4/10 tasks done)

**Completed Tasks:** ✅ ✅ ✅ ✅  
**In Progress:** 🔄  
**Remaining:** ⬜ ⬜ ⬜ ⬜ ⬜

---

### Implementation Progress

#### ✅ 2.1 Visual Previews - COMPLETE

**Status:** Fully implemented and compiled  
**Completion Date:** November 1, 2025

**Backend Changes:**

- ✅ Added `/api/trader/preview-trailing-stop` endpoint
- ✅ Created `TrailingStopPreviewResponse` and `WhatIfScenario` types
- ✅ Implemented preview calculation logic with:
  - Real position data integration (uses `get_pool_price` and `get_open_positions`)
  - Simulated position fallback when no position_id provided
  - Current profit calculation
  - Trail activation detection
  - Trail stop price calculation
  - 4 what-if scenarios (current, tighter activation, looser activation, tighter distance)
- ✅ Wire to router: `GET /api/trader/preview-trailing-stop?position_id={id}&activation_pct={pct}&distance_pct={pct}`

**Frontend Changes:**

- ✅ Added `loadTrailingStopPreview(positionId)` function
- ✅ Added `updatePreviewPanel(preview)` function
- ✅ Debounced input listeners (500ms delay on config changes)
- ✅ Integrated with tab switching (loads preview when entering Trailing Stop tab)
- ✅ Real-time updates when activation/distance inputs change

**HTML Changes:**

- ✅ Converted Trailing Stop tab to two-column layout
- ✅ Added preview panel with sections:
  - Position selector dropdown
  - Position state display (symbol, prices, profit)
  - Trail status display (active/inactive, stop price, distance to exit)
  - Estimated exit and profit
  - What-if scenarios container
- ✅ Proper styling classes for positive/negative values

**Files Modified:**

- `src/webserver/routes/trader.rs` (+150 lines)
- `src/webserver/templates/scripts/pages/trader.js` (+100 lines)
- `src/webserver/templates/pages/trader.html` (restructured trailing stop tab)

**Testing Status:** ⏳ Pending integration test

---

#### ✅ 2.3 Preset Templates - Backend COMPLETE

**Status:** Backend fully implemented  
**Completion Date:** November 1, 2025

**Backend Changes:**

- ✅ Added `/api/trader/templates` endpoint (GET)
- ✅ Added `/api/trader/apply-template` endpoint (POST)
- ✅ Created 4 hardcoded templates:

  **Conservative:**
  - Trailing Stop: 5% activation, 3% distance
  - ROI Target: 10%
  - Max Age: 72h, Loss Threshold: -20%

  **Balanced:**
  - Trailing Stop: 10% activation, 5% distance
  - ROI Target: 20%
  - Max Age: 168h, Loss Threshold: -40%

  **Aggressive:**
  - Trailing Stop: 15% activation, 7% distance
  - ROI Target: 50%
  - Max Age: 336h, Loss Threshold: -60%

  **Day Trade:**
  - Trailing Stop: 5% activation, 2% distance
  - ROI Target: 5%
  - Max Age: 24h, Loss Threshold: -15%

- ✅ Template application uses `update_config_section` (existing config system)
- ✅ Updates both `positions.*` and `trader.*` config sections
- ✅ Proper error handling and validation
- ✅ Logging on template application

**API Response Types:**

```rust
TemplateListResponse { templates: Vec<Template> }
Template { id, name, description, trading_style, config }
TemplateConfig { trailing_stop_*, roi_*, time_override_* }
ApplyTemplateRequest { template_id }
```

**Files Modified:**

- `src/webserver/routes/trader.rs` (+220 lines for templates)

**Testing Status:** ⏳ Pending frontend + integration test

---

#### 🔄 2.3 Preset Templates - Frontend IN PROGRESS

**Status:** Backend ready, frontend TODO  
**Next Steps:**

1. Add `showTemplateModal()` function to display template selection
2. Add `applyTemplate(templateId)` function to POST to `/api/trader/apply-template`
3. Add `closeTemplateModal()` function
4. Add template modal HTML structure
5. Add "Apply Preset" buttons to config tabs
6. Add confirmation dialog before applying
7. Reload config after template applied

**Estimated Effort:** 2-3 hours

---

#### ⬜ 2.2 Rule Effectiveness - NOT STARTED

**What:** Historical exit performance per rule type  
**Effort:** 2-3 days

**Backend TODO:**

- Add `/api/trader/rule-effectiveness?period={24h|7d|30d|all}` endpoint
- Query positions.db: `SELECT closed_reason, COUNT(*), AVG(pnl_percent), MIN(pnl_percent), MAX(pnl_percent), SUM(pnl) FROM positions WHERE exit_time IS NOT NULL AND exit_time >= ? GROUP BY closed_reason`
- Map closed_reason to friendly names
- Implement 5-minute response caching
- Return `RuleEffectivenessResponse` with per-rule stats

**Frontend TODO:**

- Add `loadRuleEffectiveness(period)` function
- Add `updateRuleEffectivenessDisplay()` function
- Add time range selector buttons (24h, 7d, 30d, all)
- Add bar chart visualization
- Display: rule name, exit count, avg profit, min/max profit
- Handle "no data" case gracefully

---

#### ⬜ 2.4 Import/Export - NOT STARTED

**What:** Backup/restore trader config as JSON  
**Effort:** 2-3 days

**Backend TODO:**

- Add `/api/trader/export` endpoint (GET)
  - Extract all trader-related config sections
  - Return JSON with version, timestamp, config
- Add `/api/trader/import` endpoint (POST)
  - Validate JSON structure and version
  - Check all values within bounds
  - Update config via `update_config_section`
  - Return validation errors or success

**Frontend TODO:**

- Add `exportConfig()` function
  - Download JSON file with timestamp
- Add `importConfig()` function
  - File picker
  - JSON validation
  - Confirmation dialog
  - Apply and reload UI
- Add export/import buttons to General Settings tab

---

### Phase 2 Summary

**Total Estimated Effort:** 2-3 weeks (12-17 days)

**Current Status:**

- Days spent: 1
- Tasks completed: 4/10 (40%)
- Features ready for testing: 2.1 (Visual Previews)
- Features partially complete: 2.3 (Templates backend only)

**Priority Order:**

1. ✅ **2.1 Visual Previews** (DONE - highest value, enables users to test settings)
2. 🔄 **2.3 Preset Templates** (50% DONE - quick wins for users)
3. ⬜ **2.2 Rule Effectiveness** (TODO - requires events DB queries)
4. ⬜ **2.4 Import/Export** (TODO - useful but lower priority)
5. ⬜ **Testing & Documentation** (TODO - validate all features)

**Risk Mitigation:**

- ✅ All features are frontend/backend API only (no schema changes confirmed)
- ✅ Each feature is independent (can be implemented in any order)
- ✅ Graceful degradation if events DB has no data (built into logic)
- ⏳ Caching on backend to avoid query performance issues (TODO for rule effectiveness)

---

#### 2.2 Rule Effectiveness Tracking 🔲

**What:**

- Query events DB for historical exit performance
- Per-rule breakdown showing which exit rules are most profitable
- Time-range filters (24h, 7d, 30d, all)

**Implementation Plan:**

```
Backend (trader.rs):
- Add GET /api/trader/rule-effectiveness?period={24h|7d|30d|all}
  - Query events.db for exit events (category="position", subtype="exit")
  - Group by exit_reason from payload
  - Calculate: count, avg profit, min/max profit
  - Return RuleEffectivenessResponse

Frontend (trader.js - Stats tab):
- Add loadRuleEffectiveness() function
- Add time range selector buttons
- Display bar chart showing exits by rule
- Show avg profit % per rule
- Highlight best/worst performing rules
```

**Acceptance Criteria:**

- Query completes in <2s for 30d period
- Results cached on backend (5min expiry)
- Frontend shows loading state during query
- Time range filter updates instantly (client-side filter if data already loaded)

**Estimated Effort:** 2-3 days

---

#### 2.3 Preset Templates (Read-Only) 🔲

**What:**

- Hardcoded preset configurations (no DB yet)
- "Apply Template" button on each config tab
- Templates: Conservative, Balanced, Aggressive, Day Trade

**Implementation Plan:**

```
Backend (trader.rs):
- Add GET /api/trader/templates
  - Returns hardcoded presets array
  - Each preset contains: name, description, config values

- Add POST /api/trader/apply-template
  - Takes template_name in request
  - Loads preset config
  - Updates config.toml via existing config system
  - Returns success/error

Presets Definition:
Conservative:
  - Trailing Stop: 5% activation, 3% distance
  - ROI Target: 10% profit
  - Time Override: 72h max, -20% loss threshold

Balanced:
  - Trailing Stop: 10% activation, 5% distance
  - ROI Target: 20% profit
  - Time Override: 168h max, -40% loss threshold

Aggressive:
  - Trailing Stop: 15% activation, 7% distance
  - ROI Target: 50% profit
  - Time Override: 336h max, -60% loss threshold

Day Trade:
  - Trailing Stop: 5% activation, 2% distance
  - ROI Target: 5% profit
  - Time Override: 24h max, -15% loss threshold

Frontend (trader.js):
- Add [Apply Preset] button to each config tab
- Opens modal with preset list
- Shows preset details before applying
- Confirms with user ("This will overwrite current settings")
- Applies template via API
- Reloads config UI
```

**Acceptance Criteria:**

- Templates apply successfully to config
- User confirmation required before overwriting settings
- Config UI updates immediately after template applied
- Template values match design spec exactly

**Estimated Effort:** 3-4 days

---

#### 2.4 Import/Export Configuration 🔲

**What:**

- Export entire trader config as JSON
- Import JSON with validation
- Useful for backup, sharing, A/B testing

**Implementation Plan:**

```
Backend (trader.rs):
- Add GET /api/trader/export
  - Extracts all trader-related config sections
  - Returns JSON with structure:
    {
      "version": "1.0",
      "exported_at": "2025-11-01T12:00:00Z",
      "config": {
        "trader": { ... },
        "positions": {
          "trailing_stop": { ... },
          "roi": { ... }
        }
      }
    }

- Add POST /api/trader/import
  - Validates JSON structure and version
  - Checks all values are within bounds
  - Returns validation errors if invalid
  - If valid, updates config.toml
  - Returns success with updated config

Frontend (trader.js - General Settings tab):
- Add [Export Config] button
  - Downloads trader-config-{timestamp}.json

- Add [Import Config] button
  - Opens file picker
  - Reads JSON file
  - Sends to backend for validation
  - Shows validation errors if any
  - Confirms with user before applying
  - Applies and reloads UI
```

**Acceptance Criteria:**

- Export generates valid JSON
- Import validates before applying
- Validation errors are user-friendly
- Config roundtrips without data loss (export → import → export produces identical JSON)
- Invalid JSON shows clear error messages

**Estimated Effort:** 2-3 days

---

#### 2.5 Performance Comparison 🔲

**What:**

- Compare metrics between time periods
- Export performance reports (CSV/JSON)
- Side-by-side comparison view

**Implementation Plan:**

```
Backend (trader.rs):
- Add GET /api/trader/performance-comparison?period1={7d}&period2={30d}
  - Calculates stats for both periods
  - Returns comparison object with deltas

Frontend (trader.js - Stats tab):
- Add [Compare Periods] button
- Opens modal with period selectors
- Shows side-by-side metrics
- Highlights improvements (green) and regressions (red)
- Add [Export Report] button (downloads CSV or JSON)

CSV Format:
Metric,Period 1 (7d),Period 2 (30d),Change
Win Rate,67.3%,62.1%,+5.2%
Total Trades,42,142,+100
Avg Hold Time,3.8h,4.2h,-0.4h
Best Trade,+247%,+247%,0%
...
```

**Acceptance Criteria:**

- Comparison calculates deltas correctly (absolute and percentage)
- Export generates valid CSV/JSON
- Side-by-side view is easy to read
- Handles missing data gracefully (one period has no trades)

**Estimated Effort:** 2-3 days

---

### Phase 2 Summary

**Total Estimated Effort:** 2-3 weeks (12-17 days)

**Priority Order:**

1. **2.1 Visual Previews** (highest value, enables users to test settings)
2. **2.3 Preset Templates** (quick wins for users, no complex queries)
3. **2.2 Rule Effectiveness** (requires events DB queries, may need optimization)
4. **2.4 Import/Export** (useful but lower priority than previews/templates)
5. **2.5 Performance Comparison** (enhancement, can be last)

**Risk Mitigation:**

- All features are frontend/backend API only (no schema changes)
- Each feature is independent (can be implemented in any order)
- Graceful degradation if events DB has no data
- Caching on backend to avoid query performance issues

---

## 🚀 Phase 3: Backend Enhancements (FUTURE - Requires Schema Changes)

**Timeline:** 4-6 weeks  
**Goal:** Multi-level exits, strategy voting, per-position overrides, templates DB

### Planned Features (High-Level)

1. **Multi-level exit rules** - Tiered exit system with partial position exits
2. **Strategy voting/weighting** - Weighted voting with configurable combination modes
3. **Per-position overrides** - Custom exit rules per position
4. **Rule templates (persisted)** - Save/load custom templates to DB
5. **Emergency exit rules** - Global panic rules that override all other logic

**Database Changes Required:**

- New tables: `exit_rules`, `position_exit_overrides`, `exit_rule_performance`, `strategy_weights`, `rule_templates`
- Migrations needed with rollback support
- WAL mode, proper indexing, spawn_blocking for rusqlite

**Note:** Phase 3 requires careful planning and testing. Will not proceed until Phase 2 is complete and stable.

---

## 🎨 Phase 4: Analytics & Optimization (ADVANCED)

**Timeline:** 6-8 weeks  
**Goal:** Backtesting, ML suggestions, market awareness, auto-tuning

### Planned Features (High-Level)

1. **Backtesting engine** - Historical simulation on closed positions
2. **Market condition awareness** - Volatility/trend/volume detection
3. **ML-based suggestions** - Optimal parameter recommendations
4. **Portfolio-level analytics** - Risk/reward visualization
5. **Auto-tuning** - Genetic algorithm for parameter optimization

**Note:** Phase 4 is exploratory. Requires Phase 3 to be complete and production-tested.

---

## 📋 Next Actions

### Immediate (Before Starting Phase 2)

1. ✅ **User Testing of Phase 1**
   - Test all 6 subtabs with real config changes
   - Verify stats display is accurate
   - Test config save/reload cycle
   - Verify trailing stop/ROI/time rules behave correctly

2. ✅ **Code Review**
   - Review trader.js lifecycle implementation
   - Review trader.rs API endpoint security
   - Verify no memory leaks in pollers

3. ✅ **Documentation Update**
   - Update this progress doc ✅
   - Add inline code comments where needed
   - Update FLOW.md if architecture changed

### Starting Phase 2

1. **Prioritize Features**
   - Get user feedback: which Phase 2 features are most valuable?
   - Recommend order: Visual Previews → Templates → Rule Effectiveness → Import/Export → Comparison

2. **Technical Prep**
   - Review events DB schema for rule effectiveness queries
   - Design preset template structure (JSON format)
   - Plan OHLCV integration if doing charts

3. **Set Milestones**
   - Week 1: Visual Previews + Template foundation
   - Week 2: Rule Effectiveness + Import/Export
   - Week 3: Performance Comparison + testing

---

## 🐛 Known Issues / Tech Debt

### Current (Phase 1)

- **None** - All Phase 1 acceptance criteria met

### Future Considerations

1. **Events DB Query Performance**
   - Rule effectiveness queries may be slow with large events DB
   - Solution: Add indexes on (category, subtype, event_time)
   - Consider materialized views or periodic aggregation

2. **OHLCV Chart Performance**
   - Loading full OHLCV data for charts may be slow
   - Solution: Use cached/aggregated data only, limit time range
   - Lazy load charts (don't render until tab visible)

3. **Config Hot-Reload UI Feedback**
   - Currently requires manual reload API call
   - Enhancement: WebSocket-based config push to frontend
   - Low priority (current manual reload works fine)

---

## 📊 Metrics & Success Criteria

### Phase 1 Success Metrics ✅

- ✅ All 6 subtabs functional
- ✅ Config changes persist correctly
- ✅ Stats display accurate data
- ✅ Zero crashes or blocking errors
- ✅ Code follows existing patterns

### Phase 2 Success Metrics (Target)

- Visual previews update in <500ms
- Rule effectiveness queries complete in <2s
- Template application works 100% of time
- Import/export roundtrips with zero data loss
- Performance comparison calculations are accurate
- Zero new bugs introduced

### User Satisfaction Goals

- Users can understand and configure all trader settings
- Users can see real-time impact of config changes (via previews)
- Users can quickly apply proven strategies (via templates)
- Users can analyze and optimize their trading performance

---

## 🔗 References

- **Main Design Doc:** `TRADER_IMPROVMENT.md` (v1.1)
- **Assistant Instructions:** `.github/Assistant-instructions.md`
- **Config System:** `src/config/schemas/trader.rs`
- **Frontend Patterns:** `src/webserver/templates/scripts/pages/positions.js`, `tokens.js`
- **Events System:** `src/events/types.rs`, `src/events/db.rs`

---

**Document Owner:** Development Team  
**Review Cycle:** Update after each phase completion  
**Next Review:** After Phase 2 complete
