# Trader UI - Phase 2 Implementation Plan

**Status:** IN PROGRESS (40% Complete - 4/10 tasks done)  
**Started:** November 1, 2025  
**Target Completion:** November 18-22, 2025 (2-3 weeks)

---

## Progress Overview

### ✅ Completed (4 tasks)

1. ✅ **Visual Previews - Backend** (November 1, 2025)
2. ✅ **Visual Previews - Frontend** (November 1, 2025)
3. ✅ **Visual Previews - HTML** (November 1, 2025)
4. ✅ **Preset Templates - Backend** (November 1, 2025)

### 🔄 In Progress (1 task)

5. 🔄 **Preset Templates - Frontend** (Next task)

### ⬜ Not Started (5 tasks)

6. ⬜ Rule Effectiveness - Backend
7. ⬜ Rule Effectiveness - Frontend
8. ⬜ Import/Export - Backend
9. ⬜ Import/Export - Frontend
10. ⬜ Integration Testing & Documentation

---

## Feature Status Details

### Feature 2.1: Visual Previews ✅ COMPLETE

**Completion Date:** November 1, 2025  
**Time Spent:** ~6 hours (including 6 debugging iterations)

#### Backend Implementation ✅

**File:** `src/webserver/routes/trader.rs`  
**Lines Added:** ~150

**Types Added:**

```rust
TrailingStopPreviewResponse {
    position_state: PositionState,
    trail_status: TrailStatus,
    what_if_scenarios: Vec<WhatIfScenario>
}

PositionState {
    symbol, mint, entry_price, current_price,
    peak_price, current_profit_pct
}

TrailStatus {
    is_active, activation_threshold_pct, current_distance_pct,
    trail_stop_price, distance_to_exit_pct,
    estimated_exit_price, estimated_profit_pct
}

WhatIfScenario {
    scenario_name, activation_pct, distance_pct,
    would_be_active, estimated_exit_price, estimated_profit_pct
}
```

**Handler:** `get_trailing_stop_preview(Query<TrailingStopPreviewQuery>)`

- Accepts: `position_id` (optional), `activation_pct` (optional), `distance_pct` (optional)
- Uses `positions::get_open_positions()` for real position data
- Falls back to simulated position if no position_id provided
- Fetches current price via `pools::get_pool_price(mint)`
- Calculates trail activation status and stop price
- Generates 4 what-if scenarios:
  1. Current settings
  2. Tighter activation (+2%)
  3. Looser activation (-2%)
  4. Tighter distance (-1%)

**Route:** `GET /api/trader/preview-trailing-stop`

**Debugging Journey:**

1. Fixed import: `crate::pools::get_pool_price` (not `get_sol_price`)
2. Fixed config field names: `trailing_stop_activation_pct` and `trailing_stop_distance_pct` (not `_threshold` or plain)
3. Fixed async: Removed `.await` from `get_pool_price()` (synchronous function)
4. Fixed type: Changed from `match get_open_positions().await { Ok(positions) => ... }` to direct Vec assignment
5. Fixed Position struct fields: Used `price_highest` (not `peak_price`), `symbol` is String (not Option<String>)
6. Verified clean compilation: `cargo check --lib` passes with 0 errors

#### Frontend Implementation ✅

**File:** `src/webserver/templates/scripts/pages/trader.js`  
**Lines Added:** ~100

**Functions Added:**

```javascript
loadTrailingStopPreview(positionId)
  - Builds query params (position_id, activation_pct, distance_pct)
  - Fetches from /api/trader/preview-trailing-stop
  - Calls updatePreviewPanel(preview)
  - Handles errors gracefully

updatePreviewPanel(preview)
  - Updates position state section (symbol, prices, profit)
  - Updates trail status section (active/inactive, stop price, distance)
  - Updates estimated exit and profit
  - Generates what-if scenario cards
  - Applies positive/negative styling classes

setupPreviewListeners()
  - Adds debounced listeners (500ms) to trail-activation and trail-distance inputs
  - Triggers loadTrailingStopPreview() on change
```

**Integration:**

- Modified `switchTab()` to call `loadTrailingStopPreview()` when switching to trailing-stop tab
- Modified `init()` to call `setupPreviewListeners()`

#### HTML Implementation ✅

**File:** `src/webserver/templates/pages/trader.html`  
**Lines Modified:** Trailing Stop tab restructured

**Structure:**

```html
<div class="two-column-layout">
  <div class="config-column">
    <!-- Existing trailing stop config inputs -->
  </div>
  <div class="preview-column">
    <div class="panel preview-panel">
      <h3>Preview & What-If Analysis</h3>

      <select id="preview-position-select">
        <option value="">Simulate Random Position</option>
        <!-- Populated dynamically -->
      </select>

      <div class="position-state">
        <!-- Symbol, entry price, current price, peak price, profit -->
      </div>

      <div class="trail-status">
        <!-- Active/inactive, trail price, distance to exit -->
      </div>

      <div class="estimated-outcome">
        <!-- Exit price and profit estimates -->
      </div>

      <div id="preview-what-if-scenarios">
        <!-- What-if scenario cards generated dynamically -->
      </div>
    </div>
  </div>
</div>
```

**DOM IDs Added:**

- `#preview-position-select`
- `#preview-symbol`, `#preview-entry-price`, `#preview-current-price`, `#preview-peak-price`, `#preview-current-profit`
- `#preview-trail-status`, `#preview-trail-price`, `#preview-distance-to-exit`
- `#preview-estimated-exit`, `#preview-estimated-profit`
- `#preview-what-if-scenarios`

**Styling:**

- Uses `.two-column-layout` for 50/50 split
- Uses `.positive-value` and `.negative-value` for profit colors
- Uses `.status-chip` for active/inactive indicators

#### Testing Status ⏳

- ⬜ Integration test with live bot
- ⬜ Performance validation (<500ms response)
- ⬜ Edge case testing (no positions, missing price data)

---

### Feature 2.3: Preset Templates 🔄 50% COMPLETE

#### Backend Implementation ✅ COMPLETE

**Completion Date:** November 1, 2025  
**Time Spent:** ~4 hours (including refactoring iterations)

**File:** `src/webserver/routes/trader.rs`  
**Lines Added:** ~220

**Types Added:**

```rust
TemplateListResponse { templates: Vec<Template> }

Template {
    id: String,
    name: String,
    description: String,
    trading_style: String,
    config: TemplateConfig
}

TemplateConfig {
    trailing_stop_activation_pct: f64,
    trailing_stop_distance_pct: f64,
    roi_target_pct: f64,
    time_override_enabled: bool,
    time_override_max_age_hours: u64,
    time_override_loss_threshold_pct: f64
}

ApplyTemplateRequest { template_id: String }
```

**Handlers:**

1. `get_templates() -> Json<TemplateListResponse>`
   - Returns 4 hardcoded templates (Conservative, Balanced, Aggressive, DayTrade)
   - Calls shared `get_all_templates()` helper

2. `apply_template(Json<ApplyTemplateRequest>) -> Json<SuccessResponse>`
   - Validates template_id
   - Updates `positions` config section (trailing_stop_activation_pct, trailing_stop_distance_pct)
   - Updates `trader` config section (roi*target_pct, time_override*\*)
   - Uses `update_config_section` for atomic updates
   - Second update call saves to disk
   - Logs template application

**Helper Function:**

```rust
get_all_templates() -> Vec<Template>
  - DRY function for template definitions
  - Used by both get_templates() and apply_template()
```

**Template Definitions:**

| Template     | Activation | Distance | ROI Target | Max Age | Loss Threshold | Style                  |
| ------------ | ---------- | -------- | ---------- | ------- | -------------- | ---------------------- |
| Conservative | 5%         | 3%       | 10%        | 72h     | -20%           | Low risk, quick exits  |
| Balanced     | 10%        | 5%       | 20%        | 168h    | -40%           | Moderate risk/reward   |
| Aggressive   | 15%        | 7%       | 50%        | 336h    | -60%           | High risk, hold longer |
| DayTrade     | 5%         | 2%       | 5%         | 24h     | -15%           | Fast scalping          |

**Routes:**

- `GET /api/trader/templates`
- `POST /api/trader/apply-template`

**Config Integration:**

- Uses existing `update_config_section()` from config system
- No new database tables or migrations
- Updates propagate to all active systems

#### Frontend Implementation 🔄 IN PROGRESS

**Status:** TODO (Next task)

**Planned Functions:**

```javascript
showTemplateModal()
  - Fetch templates from /api/trader/templates
  - Generate modal HTML with template cards
  - Display template details (activation%, distance%, ROI%, max age)
  - Add "Apply" button per template
  - Append modal to DOM

applyTemplate(templateId)
  - Show confirmation dialog
  - POST to /api/trader/apply-template
  - Handle success/error
  - Reload config from /api/trader/config
  - Update UI inputs
  - Close modal
  - Show success message

closeTemplateModal()
  - Remove modal from DOM
  - Clean up event listeners
```

**HTML Changes Needed:**

- Add "Apply Preset" button to:
  - Trailing Stop tab
  - ROI Target tab
  - Time-based Rules tab
- Wire buttons to `showTemplateModal()`

**Estimated Effort:** 2-3 hours

---

### Feature 2.2: Rule Effectiveness ⬜ NOT STARTED

**What:** Historical exit performance grouped by rule type (trailing stop, ROI target, time override, manual)

**Estimated Effort:** 2-3 days

#### Backend TODO

**File:** `src/webserver/routes/trader.rs`

**Types to Add:**

```rust
RuleEffectivenessQuery { period: String } // "24h", "7d", "30d", "all"

RuleEffectivenessResponse {
    period: String,
    rules: Vec<RuleStats>
}

RuleStats {
    rule_name: String,
    exit_count: u32,
    avg_profit_pct: f64,
    min_profit_pct: f64,
    max_profit_pct: f64,
    total_pnl_sol: f64
}
```

**Handler to Add:**

```rust
get_rule_effectiveness(Query<RuleEffectivenessQuery>) -> Json<RuleEffectivenessResponse>
  - Parse period to timestamp cutoff
  - Query positions.db:
    SELECT
      closed_reason,
      COUNT(*) as count,
      AVG(pnl_percent) as avg_pnl,
      MIN(pnl_percent) as min_pnl,
      MAX(pnl_percent) as max_pnl,
      SUM(pnl) as total_pnl
    FROM positions
    WHERE exit_time IS NOT NULL
      AND exit_time >= ?
    GROUP BY closed_reason
  - Map closed_reason to friendly names
  - Implement 5-minute response caching
  - Return RuleEffectivenessResponse
```

**Route to Add:**

- `GET /api/trader/rule-effectiveness?period={24h|7d|30d|all}`

**Caching Strategy:**

- Store last query result and timestamp in memory
- Return cached result if < 5 minutes old
- Recalculate if stale or period changed

#### Frontend TODO

**File:** `src/webserver/templates/scripts/pages/trader.js`

**Functions to Add:**

```javascript
loadRuleEffectiveness(period)
  - Fetch from /api/trader/rule-effectiveness?period={period}
  - Call updateRuleEffectivenessDisplay(data)
  - Handle errors

updateRuleEffectivenessDisplay(data)
  - Generate table rows per rule
  - Display: rule name, exit count, avg profit, min/max profit
  - Add bar chart visualization (optional)
  - Handle "no data" case

setupRuleEffectivenessListeners()
  - Add click listeners to period selector buttons
  - Trigger loadRuleEffectiveness(period)
```

**HTML Changes Needed:**

- Add rule effectiveness section to Stats tab
- Add period selector buttons (24h, 7d, 30d, all)
- Add table/chart container

---

### Feature 2.4: Import/Export ⬜ NOT STARTED

**What:** Backup and restore trader config as JSON file

**Estimated Effort:** 2-3 days

#### Backend TODO

**File:** `src/webserver/routes/trader.rs`

**Types to Add:**

```rust
ExportResponse {
    version: String,
    timestamp: i64,
    config: ConfigExport
}

ConfigExport {
    trader: TraderConfigExport,
    positions: PositionsConfigExport
}

ImportRequest {
    version: String,
    config: ConfigExport
}

ImportValidationResult {
    valid: bool,
    errors: Vec<String>
}
```

**Handlers to Add:**

```rust
export_trader_config() -> Json<ExportResponse>
  - Extract trader.* config section
  - Extract positions.* config section
  - Return JSON with version, timestamp, config

import_trader_config(Json<ImportRequest>) -> Json<ImportValidationResult>
  - Validate version compatibility
  - Validate all numeric values within bounds
  - Check required fields present
  - If valid:
    - Update positions config via update_config_section
    - Update trader config via update_config_section
    - Save to disk
    - Return success
  - If invalid:
    - Return error list
```

**Routes to Add:**

- `GET /api/trader/export`
- `POST /api/trader/import`

**Validation Rules:**

- Version must match current bot version (or be compatible)
- All percentage values: 0.0 <= x <= 100.0
- Max age hours: 1 <= x <= 8760 (1 year)
- SOL amounts: 0.001 <= x <= 1000.0
- Max positions: 1 <= x <= 50

#### Frontend TODO

**File:** `src/webserver/templates/scripts/pages/trader.js`

**Functions to Add:**

```javascript
exportConfig()
  - Fetch from /api/trader/export
  - Generate filename: trader_config_YYYYMMDD_HHMMSS.json
  - Create download link and trigger
  - Show success message

importConfig()
  - Show file picker
  - Read file as JSON
  - Validate JSON structure
  - Show preview dialog with changes
  - Confirm with user
  - POST to /api/trader/import
  - Handle validation errors
  - If success:
    - Reload config from /api/trader/config
    - Update UI
    - Show success message
```

**HTML Changes Needed:**

- Add import/export buttons to General Settings tab
- Add file input element (hidden, triggered by button)
- Add preview modal for import

---

## Testing & Validation ⬜ NOT STARTED

**Estimated Effort:** 2-3 days

### Integration Testing TODO

**Visual Previews:**

- [ ] Test with live bot running
- [ ] Verify preview accuracy against actual positions
- [ ] Test edge cases:
  - No open positions (should show simulation)
  - Missing price data (should use fallback)
  - Multiple positions (selector works)
- [ ] Verify debouncing works (500ms delay)
- [ ] Check performance (<500ms response time)

**Preset Templates:**

- [ ] Test all 4 templates apply correctly
- [ ] Verify config updates in data/config.toml
- [ ] Check config propagates to trader module
- [ ] Test confirmation dialog flow
- [ ] Verify reload updates UI inputs

**Rule Effectiveness:**

- [ ] Test with empty positions DB (no exits yet)
- [ ] Test with populated DB (various exit reasons)
- [ ] Verify SQL query performance (<2s)
- [ ] Check caching works (5-minute window)
- [ ] Test all period filters (24h, 7d, 30d, all)

**Import/Export:**

- [ ] Export config and verify JSON structure
- [ ] Import valid config and check updates
- [ ] Test validation errors:
  - Invalid version
  - Out-of-bounds values
  - Missing required fields
- [ ] Test import preview dialog
- [ ] Verify file download works across browsers

### Performance Validation

**Targets:**

- Preview endpoint: <500ms response time
- Templates fetch: <200ms response time
- Template apply: <1s total time
- Rule effectiveness: <2s query time (cold cache), <100ms (warm cache)
- Export: <500ms response time
- Import validation: <1s total time

**Monitoring:**

- Add timing logs to all new endpoints
- Track query execution times
- Monitor memory usage (caching)

### Documentation Updates TODO

**Files to Update:**

- [ ] `docs/TRADER_UI_PROGRESS.md` (mark Phase 2 complete)
- [ ] `docs/API.md` (document new endpoints)
- [ ] `README.md` (update features list)
- [ ] Inline code comments (where complex logic exists)

---

## Timeline & Milestones

### Week 1 (November 1-8, 2025)

- [x] Day 1: Visual Previews complete (November 1)
- [x] Day 1: Templates backend complete (November 1)
- [ ] Day 2: Templates frontend complete
- [ ] Day 3-4: Rule Effectiveness (backend + frontend)
- [ ] Day 5: Import/Export backend

### Week 2 (November 9-15, 2025)

- [ ] Day 6: Import/Export frontend
- [ ] Day 7-9: Integration testing
- [ ] Day 10: Bug fixes and polish

### Week 3 (November 16-22, 2025) - Buffer

- [ ] Performance optimization if needed
- [ ] Documentation finalization
- [ ] Phase 2 completion review

---

## Risk Assessment

### Low Risk ✅

- All features are UI/API only (no database schema changes)
- Features are independent (can fail independently)
- Existing config system handles updates safely
- Preview calculations don't affect live trading

### Medium Risk ⚠️

- Rule effectiveness query performance (mitigation: caching)
- Import validation complexity (mitigation: comprehensive tests)
- Template application propagation (mitigation: use existing config system)

### High Risk ❌

- None identified

---

## Next Actions

### Immediate (Today)

1. 🔄 Complete templates frontend (Task 5)
   - Add `showTemplateModal()`, `applyTemplate()`, `closeTemplateModal()` functions
   - Add "Apply Preset" buttons to config tabs
   - Test template application flow

### This Week

2. Implement Rule Effectiveness backend (Task 6)
3. Implement Rule Effectiveness frontend (Task 7)
4. Implement Import/Export backend (Task 8)
5. Implement Import/Export frontend (Task 9)

### Next Week

6. Integration testing (Task 10)
7. Performance validation
8. Bug fixes and polish
9. Documentation updates
10. Phase 2 completion review

---

## Completion Criteria

Phase 2 is complete when:

- [x] Visual previews work with real and simulated positions
- [ ] All 4 templates apply correctly and update trader behavior
- [ ] Rule effectiveness displays historical data accurately
- [ ] Import/export preserves config integrity
- [ ] All features tested with live bot
- [ ] Performance targets met (<500ms previews, <2s queries)
- [ ] Documentation updated
- [ ] Zero critical bugs

---

## Key Learnings (Running Log)

### November 1, 2025

**Visual Previews Implementation:**

- Position struct uses `price_highest` field, not `peak_price`
- `symbol` field is `String`, not `Option<String>`
- `get_pool_price()` is synchronous, returns `Option<PriceResult>`
- `get_open_positions()` returns `Vec<Position>` directly, not `Result<Vec<Position>>`
- Config fields use `_pct` suffix: `trailing_stop_activation_pct`, `trailing_stop_distance_pct`

**Preset Templates Implementation:**

- Config update pattern: Two calls to `update_config_section()`, second call saves to disk
- Template application updates both `positions.*` and `trader.*` sections
- Template IDs must match exactly for validation
- Refactored to shared `get_all_templates()` helper for DRY principle

**Debugging Patterns:**

- Always verify struct fields in source before accessing
- Check function signatures (async vs sync, return types)
- Use `cargo check --lib` for fast compilation checks
- Parse error messages carefully for field name mismatches

**Performance Notes:**

- Debouncing at 500ms prevents excessive API calls during rapid config changes
- Preview calculations are fast enough for real-time updates
- No noticeable UI blocking with current implementation
