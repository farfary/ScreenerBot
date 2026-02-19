# Trader UI - Phase 2 Implementation Plan

**Project:** ScreenerBot Trader UI Enhancement  
**Phase:** 2 - Enhanced UI (No Schema Changes)  
**Timeline:** 2-3 weeks  
**Start Date:** TBD  
**Reference:** `TRADER_IMPROVMENT.md`, `TRADER_UI_PROGRESS.md`

---

## 🎯 Phase 2 Goals

**Primary Objective:** Enhance Trader UI with visual previews, templates, and analytics **without any database schema changes**.

**Success Criteria:**
- Users can see real-time impact of config changes (visual previews)
- Users can quickly apply proven strategies (preset templates)
- Users can analyze historical performance (rule effectiveness)
- Users can backup/restore configs (import/export)
- All features work without blocking UI

---

## 📦 Feature Breakdown

### Feature 2.1: Visual Previews 🎯 HIGH PRIORITY

**Value:** Enables users to test settings before applying, reduces trial-and-error.

#### Backend Changes

**File:** `src/webserver/routes/trader.rs`

**New Endpoint:** `GET /api/trader/preview-trailing-stop`

```rust
// Add to trader.rs

#[derive(Debug, Serialize)]
pub struct TrailingStopPreviewResponse {
    // Position state
    pub position_id: Option<i64>,
    pub symbol: String,
    pub entry_price: f64,
    pub current_price: f64,
    pub peak_price: f64,
    pub current_profit_pct: f64,
    pub unrealized_pnl: f64,
    
    // Trail state with CURRENT settings
    pub trail_active: bool,
    pub trail_activated_at_pct: Option<f64>,
    pub trail_stop_price: Option<f64>,
    pub distance_to_exit_pct: Option<f64>,
    pub estimated_exit_price: f64,
    pub estimated_exit_profit_pct: f64,
    
    // What-if scenarios
    pub what_if_scenarios: Vec<WhatIfScenario>,
}

#[derive(Debug, Serialize)]
pub struct WhatIfScenario {
    pub description: String,
    pub activation_pct: f64,
    pub distance_pct: f64,
    pub trail_active: bool,
    pub exit_price: f64,
    pub exit_profit_pct: f64,
}

// Query params
#[derive(Debug, Deserialize)]
pub struct TrailingStopPreviewQuery {
    pub position_id: Option<i64>,  // If None, use simulation
    pub activation_pct: Option<f64>, // Override current config
    pub distance_pct: Option<f64>,   // Override current config
}

pub async fn get_trailing_stop_preview(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TrailingStopPreviewQuery>,
) -> Response {
    // 1. Get position (or create simulated position)
    // 2. Load current trailing stop config (or use overrides from query)
    // 3. Calculate current trail state
    // 4. Generate what-if scenarios (vary activation/distance by ±50%)
    // 5. Return preview response
    
    // Implementation notes:
    // - If position_id provided: query positions DB, get latest price from pools
    // - If no position_id: create simulated position (entry=0.001 SOL, current=0.00119 SOL, peak=0.00123 SOL)
    // - Calculate trail activation and stop price using same logic as exit monitor
    // - Generate 4 what-if scenarios:
    //   1. Current settings
    //   2. Tighter activation (current - 5%)
    //   3. Looser activation (current + 5%)
    //   4. Tighter distance (current - 2%)
}
```

**Estimated LOC:** ~150 lines

#### Frontend Changes

**File:** `src/webserver/templates/scripts/pages/trader.js`

**New Functions:**

```javascript
// Add to trader.js

async function loadTrailingStopPreview(positionId = null) {
    // 1. Get current trailing stop config values from form
    // 2. Call API with position_id and current settings
    // 3. Update preview panel with results
    // 4. Update what-if scenarios
    
    const activation = parseFloat($('#trailing-activation').value) || 10;
    const distance = parseFloat($('#trailing-distance').value) || 5;
    
    try {
        showElement('#preview-loading');
        hideElement('#preview-content');
        
        const params = new URLSearchParams();
        if (positionId) params.append('position_id', positionId);
        params.append('activation_pct', activation);
        params.append('distance_pct', distance);
        
        const response = await fetch(`/api/trader/preview-trailing-stop?${params}`);
        const data = await response.json();
        
        if (data.success) {
            updatePreviewPanel(data.data);
        } else {
            showError('Preview failed: ' + data.error);
        }
    } finally {
        hideElement('#preview-loading');
        showElement('#preview-content');
    }
}

function updatePreviewPanel(preview) {
    // Update DOM elements with preview data
    $('#preview-symbol').textContent = preview.symbol;
    $('#preview-entry-price').textContent = Utils.formatPrice(preview.entry_price);
    $('#preview-current-price').textContent = Utils.formatPrice(preview.current_price);
    $('#preview-peak-price').textContent = Utils.formatPrice(preview.peak_price);
    $('#preview-current-profit').textContent = Utils.formatPercent(preview.current_profit_pct);
    
    // Trail status
    const trailStatus = preview.trail_active ? '✅ ACTIVE' : '⏸️ INACTIVE';
    $('#preview-trail-status').textContent = trailStatus;
    
    if (preview.trail_active) {
        $('#preview-trail-price').textContent = Utils.formatPrice(preview.trail_stop_price);
        $('#preview-distance-to-exit').textContent = Utils.formatPercent(preview.distance_to_exit_pct);
    }
    
    $('#preview-estimated-exit').textContent = Utils.formatPrice(preview.estimated_exit_price);
    $('#preview-estimated-profit').textContent = Utils.formatPercent(preview.estimated_exit_profit_pct);
    
    // What-if scenarios
    const scenariosContainer = $('#preview-what-if-scenarios');
    scenariosContainer.innerHTML = '';
    
    preview.what_if_scenarios.forEach(scenario => {
        const scenarioDiv = document.createElement('div');
        scenarioDiv.className = 'what-if-scenario';
        scenarioDiv.innerHTML = `
            <div class="scenario-description">${scenario.description}</div>
            <div class="scenario-result">
                Exit: ${Utils.formatPrice(scenario.exit_price)} 
                (${Utils.formatPercent(scenario.exit_profit_pct)} profit)
            </div>
        `;
        scenariosContainer.appendChild(scenarioDiv);
    });
}

// Add event listeners for real-time preview updates
function initTrailingStopTab() {
    // ... existing init code ...
    
    // Debounced preview update on config change
    const debouncedPreview = Utils.debounce(() => {
        const positionId = $('#preview-position-select').value;
        loadTrailingStopPreview(positionId === 'simulate' ? null : positionId);
    }, 500);
    
    $('#trailing-activation').addEventListener('input', debouncedPreview);
    $('#trailing-distance').addEventListener('input', debouncedPreview);
    $('#preview-position-select').addEventListener('change', debouncedPreview);
}
```

**Estimated LOC:** ~100 lines

**HTML Changes:**

Add preview panel to `trader.html` Trailing Stop tab (right column):

```html
<div class="preview-panel">
    <h3>Live Preview</h3>
    
    <div class="position-selector">
        <label>Position:</label>
        <select id="preview-position-select">
            <option value="simulate">Simulate Random</option>
            <!-- Populated dynamically with open positions -->
        </select>
    </div>
    
    <div id="preview-loading" class="loading" style="display: none;">Loading...</div>
    
    <div id="preview-content">
        <div class="preview-section">
            <h4>Position State</h4>
            <div class="preview-row">
                <span>Symbol:</span>
                <span id="preview-symbol">-</span>
            </div>
            <div class="preview-row">
                <span>Entry Price:</span>
                <span id="preview-entry-price">-</span>
            </div>
            <div class="preview-row">
                <span>Current Price:</span>
                <span id="preview-current-price">-</span>
            </div>
            <div class="preview-row">
                <span>Peak Price:</span>
                <span id="preview-peak-price">-</span>
            </div>
            <div class="preview-row">
                <span>Current Profit:</span>
                <span id="preview-current-profit" class="profit-value">-</span>
            </div>
        </div>
        
        <div class="preview-section">
            <h4>Trailing Stop Status</h4>
            <div class="preview-row">
                <span>Trail Active:</span>
                <span id="preview-trail-status">-</span>
            </div>
            <div class="preview-row">
                <span>Stop Price:</span>
                <span id="preview-trail-price">-</span>
            </div>
            <div class="preview-row">
                <span>Distance to Exit:</span>
                <span id="preview-distance-to-exit">-</span>
            </div>
            <div class="preview-row">
                <span>Estimated Exit:</span>
                <span id="preview-estimated-exit">-</span>
            </div>
            <div class="preview-row">
                <span>Estimated Profit:</span>
                <span id="preview-estimated-profit" class="profit-value">-</span>
            </div>
        </div>
        
        <div class="preview-section">
            <h4>What-If Analysis</h4>
            <div id="preview-what-if-scenarios">
                <!-- Populated dynamically -->
            </div>
        </div>
    </div>
</div>
```

**Testing:**
- Test with open positions
- Test with simulated position
- Test config changes update preview in <500ms
- Test with no open positions (should show simulation)

---

### Feature 2.2: Rule Effectiveness Tracking 📊 MEDIUM PRIORITY

**Value:** Shows which exit rules are most profitable historically.

#### Backend Changes

**File:** `src/webserver/routes/trader.rs`

**New Endpoint:** `GET /api/trader/rule-effectiveness`

```rust
// Add to trader.rs

#[derive(Debug, Serialize)]
pub struct RuleEffectivenessResponse {
    pub period: String,
    pub total_exits: usize,
    pub rules: Vec<RuleEffectivenessEntry>,
}

#[derive(Debug, Serialize)]
pub struct RuleEffectivenessEntry {
    pub rule_name: String,
    pub exit_count: usize,
    pub avg_profit_pct: f64,
    pub min_profit_pct: f64,
    pub max_profit_pct: f64,
    pub total_profit_sol: f64,
}

#[derive(Debug, Deserialize)]
pub struct RuleEffectivenessQuery {
    pub period: Option<String>, // "24h", "7d", "30d", "all"
}

pub async fn get_rule_effectiveness(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RuleEffectivenessQuery>,
) -> Response {
    // 1. Parse period (default "30d")
    // 2. Calculate timestamp threshold
    // 3. Query positions DB for closed positions in period
    // 4. Group by closed_reason field
    // 5. Calculate stats per rule
    // 6. Return response
    
    // Implementation notes:
    // - Query: SELECT closed_reason, COUNT(*), AVG(pnl_percent), MIN(pnl_percent), MAX(pnl_percent), SUM(pnl) 
    //          FROM positions 
    //          WHERE exit_time IS NOT NULL AND exit_time >= ?
    //          GROUP BY closed_reason
    // - Map closed_reason to friendly names:
    //   - "ManualExit" -> "Manual Exit"
    //   - "ROITarget" -> "Take Profit (ROI)"
    //   - "TrailingStop" -> "Trailing Stop"
    //   - "TimeOverride" -> "Time Rules"
    //   - "StrategySignal" -> "Strategy Signal"
    // - Cache result for 5 minutes (use Arc<RwLock<HashMap<String, CachedResponse>>>)
}
```

**Estimated LOC:** ~120 lines

#### Frontend Changes

**File:** `src/webserver/templates/scripts/pages/trader.js`

**New Functions:**

```javascript
// Add to Stats tab initialization

async function loadRuleEffectiveness(period = '30d') {
    try {
        const response = await fetch(`/api/trader/rule-effectiveness?period=${period}`);
        const data = await response.json();
        
        if (data.success) {
            updateRuleEffectivenessDisplay(data.data);
        } else {
            showError('Failed to load rule effectiveness: ' + data.error);
        }
    } catch (error) {
        logger.error('Failed to fetch rule effectiveness:', error);
    }
}

function updateRuleEffectivenessDisplay(effectiveness) {
    const container = $('#rule-effectiveness-container');
    container.innerHTML = '';
    
    if (effectiveness.total_exits === 0) {
        container.innerHTML = '<p class="no-data">No exits in this period</p>';
        return;
    }
    
    effectiveness.rules.forEach(rule => {
        const percentage = (rule.exit_count / effectiveness.total_exits) * 100;
        const barWidth = Math.round(percentage);
        
        const ruleDiv = document.createElement('div');
        ruleDiv.className = 'rule-effectiveness-row';
        ruleDiv.innerHTML = `
            <div class="rule-name">${rule.rule_name}</div>
            <div class="rule-bar">
                <div class="rule-bar-fill ${rule.avg_profit_pct >= 0 ? 'profit' : 'loss'}" 
                     style="width: ${barWidth}%"></div>
            </div>
            <div class="rule-stats">
                ${rule.exit_count} exits | Avg: ${Utils.formatPercent(rule.avg_profit_pct)}
            </div>
        `;
        container.appendChild(ruleDiv);
    });
}

// Add period selector event listener
function initStatsTab() {
    // ... existing code ...
    
    // Period selector buttons
    $$('.period-selector-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const period = btn.dataset.period;
            $$('.period-selector-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            loadRuleEffectiveness(period);
        });
    });
    
    // Load default (30d)
    loadRuleEffectiveness('30d');
}
```

**Estimated LOC:** ~80 lines

**HTML Changes:**

Add to Stats tab in `trader.html`:

```html
<div class="rule-effectiveness-section">
    <h3>Exit Strategy Performance</h3>
    
    <div class="period-selector">
        <button class="period-selector-btn" data-period="24h">24h</button>
        <button class="period-selector-btn active" data-period="30d">30d</button>
        <button class="period-selector-btn" data-period="all">All Time</button>
    </div>
    
    <div id="rule-effectiveness-container" class="rule-effectiveness-list">
        <!-- Populated dynamically -->
    </div>
</div>
```

**Testing:**
- Test with different time periods
- Test with no data (should show "no exits" message)
- Test performance with large datasets (ensure <2s query time)

---

### Feature 2.3: Preset Templates ⚡ HIGH PRIORITY

**Value:** Users can instantly apply proven configurations.

#### Backend Changes

**File:** `src/webserver/routes/trader.rs`

**New Endpoints:**
- `GET /api/trader/templates` - List available templates
- `POST /api/trader/apply-template` - Apply a template

```rust
// Add to trader.rs

#[derive(Debug, Serialize)]
pub struct TemplateListResponse {
    pub templates: Vec<Template>,
}

#[derive(Debug, Serialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trading_style: String, // "conservative", "balanced", "aggressive", "day_trade"
    pub config: TemplateConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateConfig {
    pub trailing_stop_enabled: bool,
    pub trailing_stop_activation_pct: f64,
    pub trailing_stop_distance_pct: f64,
    pub roi_enabled: bool,
    pub roi_target_pct: f64,
    pub time_override_enabled: bool,
    pub time_override_max_age_hours: f64,
    pub time_override_loss_threshold_pct: f64,
}

pub async fn get_templates() -> Response {
    let templates = vec![
        Template {
            id: "conservative".to_string(),
            name: "Conservative".to_string(),
            description: "Low risk, secure profits early".to_string(),
            trading_style: "conservative".to_string(),
            config: TemplateConfig {
                trailing_stop_enabled: true,
                trailing_stop_activation_pct: 5.0,
                trailing_stop_distance_pct: 3.0,
                roi_enabled: true,
                roi_target_pct: 10.0,
                time_override_enabled: true,
                time_override_max_age_hours: 72.0,
                time_override_loss_threshold_pct: -20.0,
            },
        },
        Template {
            id: "balanced".to_string(),
            name: "Balanced".to_string(),
            description: "Balanced risk/reward".to_string(),
            trading_style: "balanced".to_string(),
            config: TemplateConfig {
                trailing_stop_enabled: true,
                trailing_stop_activation_pct: 10.0,
                trailing_stop_distance_pct: 5.0,
                roi_enabled: true,
                roi_target_pct: 20.0,
                time_override_enabled: true,
                time_override_max_age_hours: 168.0,
                time_override_loss_threshold_pct: -40.0,
            },
        },
        Template {
            id: "aggressive".to_string(),
            name: "Aggressive".to_string(),
            description: "High risk, chase large gains".to_string(),
            trading_style: "aggressive".to_string(),
            config: TemplateConfig {
                trailing_stop_enabled: true,
                trailing_stop_activation_pct: 15.0,
                trailing_stop_distance_pct: 7.0,
                roi_enabled: true,
                roi_target_pct: 50.0,
                time_override_enabled: true,
                time_override_max_age_hours: 336.0,
                time_override_loss_threshold_pct: -60.0,
            },
        },
        Template {
            id: "day_trade".to_string(),
            name: "Day Trade".to_string(),
            description: "Quick exits, tight stops".to_string(),
            trading_style: "day_trade".to_string(),
            config: TemplateConfig {
                trailing_stop_enabled: true,
                trailing_stop_activation_pct: 5.0,
                trailing_stop_distance_pct: 2.0,
                roi_enabled: true,
                roi_target_pct: 5.0,
                time_override_enabled: true,
                time_override_max_age_hours: 24.0,
                time_override_loss_threshold_pct: -15.0,
            },
        },
    ];
    
    success_response(TemplateListResponse { templates })
}

#[derive(Debug, Deserialize)]
pub struct ApplyTemplateRequest {
    pub template_id: String,
}

pub async fn apply_template(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ApplyTemplateRequest>,
) -> Response {
    // 1. Find template by ID
    // 2. Update config using existing config system
    // 3. Reload config
    // 4. Return success with updated config
    
    // Implementation:
    // - Call get_templates() to find template
    // - Use config::update() helpers to set each field
    // - Reload config via config::reload_config()
    // - Return updated config sections
}
```

**Estimated LOC:** ~150 lines

#### Frontend Changes

**File:** `src/webserver/templates/scripts/pages/trader.js`

```javascript
// Add template application logic

async function showTemplateModal() {
    try {
        const response = await fetch('/api/trader/templates');
        const data = await response.json();
        
        if (!data.success) {
            showError('Failed to load templates');
            return;
        }
        
        // Build modal HTML
        const modalHTML = `
            <div class="modal-overlay" id="template-modal">
                <div class="modal-content">
                    <h2>Apply Preset Template</h2>
                    <p class="warning">⚠️ This will overwrite your current settings</p>
                    
                    <div class="template-list">
                        ${data.data.templates.map(t => `
                            <div class="template-card" data-template-id="${t.id}">
                                <h3>${t.name}</h3>
                                <p class="template-description">${t.description}</p>
                                <div class="template-details">
                                    <div>Trail: ${t.config.trailing_stop_activation_pct}% activation, ${t.config.trailing_stop_distance_pct}% distance</div>
                                    <div>ROI: ${t.config.roi_target_pct}% target</div>
                                    <div>Max Age: ${t.config.time_override_max_age_hours}h</div>
                                </div>
                                <button class="btn-apply-template" data-template-id="${t.id}">Apply</button>
                            </div>
                        `).join('')}
                    </div>
                    
                    <button class="btn-cancel">Cancel</button>
                </div>
            </div>
        `;
        
        // Add to DOM
        document.body.insertAdjacentHTML('beforeend', modalHTML);
        
        // Event listeners
        $$('.btn-apply-template').forEach(btn => {
            btn.addEventListener('click', () => applyTemplate(btn.dataset.templateId));
        });
        
        $('.btn-cancel').addEventListener('click', closeTemplateModal);
        $('#template-modal').addEventListener('click', (e) => {
            if (e.target.id === 'template-modal') closeTemplateModal();
        });
        
    } catch (error) {
        logger.error('Failed to show template modal:', error);
        showError('Failed to load templates');
    }
}

async function applyTemplate(templateId) {
    if (!confirm('Are you sure? This will overwrite your current trader settings.')) {
        return;
    }
    
    try {
        const response = await fetch('/api/trader/apply-template', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ template_id: templateId })
        });
        
        const data = await response.json();
        
        if (data.success) {
            showSuccess('Template applied successfully');
            closeTemplateModal();
            
            // Reload all config tabs
            await loadAllConfigs();
        } else {
            showError('Failed to apply template: ' + data.error);
        }
    } catch (error) {
        logger.error('Failed to apply template:', error);
        showError('Failed to apply template');
    }
}

function closeTemplateModal() {
    const modal = $('#template-modal');
    if (modal) modal.remove();
}

// Add button to each config tab
function addTemplateButtons() {
    const tabs = ['trailing-stop', 'roi', 'time-rules'];
    tabs.forEach(tabId => {
        const tab = $(`#${tabId}-tab`);
        if (tab) {
            const btn = document.createElement('button');
            btn.className = 'btn-preset-template';
            btn.textContent = '📋 Apply Preset';
            btn.addEventListener('click', showTemplateModal);
            tab.querySelector('.config-actions').prepend(btn);
        }
    });
}
```

**Estimated LOC:** ~120 lines

**Testing:**
- Test each template applies correctly
- Test confirmation dialog works
- Test config reloads after apply
- Test cancel button
- Test with invalid template ID

---

### Feature 2.4: Import/Export 💾 MEDIUM PRIORITY

**Value:** Backup/restore configurations, share between instances.

#### Backend Changes

**File:** `src/webserver/routes/trader.rs`

```rust
// Add to trader.rs

#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub version: String,
    pub exported_at: String,
    pub config: ExportedConfig,
}

#[derive(Debug, Serialize)]
pub struct ExportedConfig {
    // All trader-related config sections
    pub trader: serde_json::Value,
    pub positions: serde_json::Value,
}

pub async fn export_config() -> Response {
    // 1. Extract trader and positions config sections
    // 2. Build export object with version and timestamp
    // 3. Return JSON
    
    // Use existing config::with_config() to extract values
}

#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    pub config: ExportedConfig,
}

pub async fn import_config(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ImportRequest>,
) -> Response {
    // 1. Validate version compatibility
    // 2. Validate all values are within bounds
    // 3. Apply to config
    // 4. Reload
    // 5. Return success/errors
}
```

**Estimated LOC:** ~100 lines

#### Frontend Changes

```javascript
async function exportConfig() {
    try {
        const response = await fetch('/api/trader/export');
        const data = await response.json();
        
        if (data.success) {
            const blob = new Blob([JSON.stringify(data.data, null, 2)], { type: 'application/json' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `trader-config-${Date.now()}.json`;
            a.click();
            URL.revokeObjectURL(url);
            
            showSuccess('Config exported successfully');
        }
    } catch (error) {
        logger.error('Export failed:', error);
        showError('Failed to export config');
    }
}

async function importConfig() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'application/json';
    
    input.onchange = async (e) => {
        const file = e.target.files[0];
        if (!file) return;
        
        try {
            const text = await file.text();
            const config = JSON.parse(text);
            
            // Validate structure
            if (!config.version || !config.config) {
                showError('Invalid config file format');
                return;
            }
            
            if (!confirm('Import this configuration? Current settings will be overwritten.')) {
                return;
            }
            
            const response = await fetch('/api/trader/import', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(config)
            });
            
            const data = await response.json();
            
            if (data.success) {
                showSuccess('Config imported successfully');
                await loadAllConfigs();
            } else {
                showError('Import failed: ' + data.error);
            }
        } catch (error) {
            logger.error('Import failed:', error);
            showError('Failed to import config: ' + error.message);
        }
    };
    
    input.click();
}
```

**Estimated LOC:** ~80 lines

---

### Feature 2.5: Performance Comparison 📈 LOW PRIORITY

**Value:** Compare metrics across time periods.

#### Backend Changes

```rust
#[derive(Debug, Serialize)]
pub struct PerformanceComparisonResponse {
    pub period1: PeriodStats,
    pub period2: PeriodStats,
    pub deltas: PeriodDeltas,
}

#[derive(Debug, Serialize)]
pub struct PeriodStats {
    pub label: String,
    pub win_rate_pct: f64,
    pub total_trades: usize,
    pub avg_hold_time_hours: f64,
    pub best_trade_pct: f64,
    pub total_profit_sol: f64,
}

#[derive(Debug, Serialize)]
pub struct PeriodDeltas {
    pub win_rate_delta: f64,
    pub trades_delta: i64,
    pub hold_time_delta: f64,
    pub profit_delta: f64,
}

pub async fn get_performance_comparison(
    Query(query): Query<ComparisonQuery>,
) -> Response {
    // Calculate stats for both periods
    // Calculate deltas
    // Return comparison
}
```

**Estimated LOC:** ~150 lines

---

## 📅 Implementation Schedule

### Week 1: High Priority Features
- **Days 1-3:** Feature 2.1 - Visual Previews
  - Backend endpoint
  - Frontend integration
  - Testing
  
- **Days 4-5:** Feature 2.3 - Preset Templates
  - Backend templates
  - Modal UI
  - Testing

### Week 2: Medium Priority Features
- **Days 1-2:** Feature 2.2 - Rule Effectiveness
  - Backend queries
  - Frontend display
  - Performance testing
  
- **Days 3-4:** Feature 2.4 - Import/Export
  - Backend endpoints
  - Frontend file handling
  - Validation

### Week 3: Polish & Testing
- **Days 1-2:** Feature 2.5 - Performance Comparison
- **Days 3-5:** Integration testing, bug fixes, documentation

---

## ✅ Acceptance Criteria

### Overall Phase 2 Success

- [ ] Visual previews update in <500ms
- [ ] Rule effectiveness queries complete in <2s
- [ ] All 4 templates apply successfully
- [ ] Import/export roundtrips without data loss
- [ ] Zero new bugs introduced
- [ ] No database schema changes
- [ ] All features work with no open positions (graceful degradation)
- [ ] Documentation updated

### Performance Targets

- API response times: <500ms (p95)
- Frontend rendering: <200ms (p95)
- Zero blocking operations
- Memory usage: No leaks in pollers

---

## 🐛 Testing Checklist

### Per-Feature Testing

**Visual Previews:**
- [ ] Works with open positions
- [ ] Works with simulated position
- [ ] Updates on config change
- [ ] What-if scenarios accurate
- [ ] Handles missing data

**Rule Effectiveness:**
- [ ] Queries complete in <2s
- [ ] Time filters work correctly
- [ ] Shows correct stats
- [ ] Handles no data case

**Preset Templates:**
- [ ] All 4 templates apply
- [ ] Confirmation works
- [ ] Config reloads after apply
- [ ] Template values match spec

**Import/Export:**
- [ ] Export downloads JSON
- [ ] Import validates format
- [ ] Roundtrip preserves data
- [ ] Error messages clear

**Performance Comparison:**
- [ ] Calculates deltas correctly
- [ ] Export works
- [ ] Handles missing periods

### Integration Testing

- [ ] All features work together
- [ ] No conflicts between features
- [ ] Config changes persist
- [ ] Service restarts preserve state

---

## 📚 Resources

- **Design Doc:** `TRADER_IMPROVMENT.md`
- **Progress Doc:** `TRADER_UI_PROGRESS.md`
- **Config Patterns:** `.github/Assistant-instructions.md`
- **Similar Features:** Positions page, Tokens page
- **Events System:** `src/events/db.rs`
- **Config System:** `src/config/`

---

## 🎯 Next Steps

1. **Review this plan** with team
2. **Prioritize features** (current order: 2.1 → 2.3 → 2.2 → 2.4 → 2.5)
3. **Set start date** for Phase 2
4. **Assign developers** to features
5. **Begin implementation** following this spec

---

**Document Owner:** Development Team  
**Status:** Ready for Implementation  
**Next Review:** After Phase 2 completion
