/**
 * Filtering Configuration Page
 *
 * Provides UI for:
 * - Viewing/editing filtering configuration
 * - Monitoring filtering statistics
 * - Manual snapshot refresh
 */

import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";

// ============================================================================
// STATE
// ============================================================================

const state = {
  config: null,
  draft: null,
  stats: null,
  hasChanges: false,
  isSaving: false,
  isRefreshing: false,
  lastSaved: null,
  searchQuery: "",
  activeTab: AppState.load("filtering_activeTab", "meta"),
};

// ============================================================================
// CONFIGURATION METADATA
// ============================================================================

const CONFIG_CATEGORIES = {
  Performance: {
    source: "meta",
    fields: [
      {
        key: "filter_cache_ttl_secs",
        label: "Cache TTL",
        type: "number",
        unit: "seconds",
        min: 5,
        max: 300,
        step: 5,
        hint: "How long to cache filter results (lower = more current)",
        impact: "critical",
      },
      {
        key: "target_filtered_tokens",
        label: "Target Filtered Tokens",
        type: "number",
        unit: "tokens",
        min: 10,
        max: 10000,
        step: 100,
        hint: "Bot processes up to this many qualified tokens",
        impact: "medium",
      },
      {
        key: "max_tokens_to_process",
        label: "Max Tokens to Process",
        type: "number",
        unit: "tokens",
        min: 100,
        max: 100000,
        step: 100,
        hint: "Max tokens to evaluate before filtering",
        impact: "medium",
      },
    ],
  },
  "Meta Requirements": {
    source: "meta",
    fields: [
      {
        key: "require_decimals_in_db",
        label: "Require Decimals in Database",
        type: "boolean",
        hint: "Skip tokens without cached decimal data",
        impact: "high",
      },
      {
        key: "check_cooldown",
        label: "Check Cooldown",
        type: "boolean",
        hint: "Skip tokens in cooldown period after exit",
        impact: "high",
      },
      {
        key: "min_token_age_minutes",
        label: "Min Token Age",
        type: "number",
        unit: "minutes",
        min: 0,
        max: 10080,
        step: 10,
        hint: "60min avoids brand new tokens, lower for sniping",
        impact: "critical",
      },
    ],
  },
  "DexScreener - Token Info": {
    source: "dexscreener",
    enableKey: "token_info_enabled",
    fields: [
      {
        key: "require_name_and_symbol",
        label: "Require Name & Symbol",
        type: "boolean",
        hint: "Recommended: true. Filters incomplete tokens",
        impact: "high",
      },
      {
        key: "require_logo_url",
        label: "Require Logo",
        type: "boolean",
        hint: "Optional. Logo may indicate legitimacy",
        impact: "medium",
      },
      {
        key: "require_website_url",
        label: "Require Website",
        type: "boolean",
        hint: "Optional. Website may indicate serious project",
        impact: "medium",
      },
    ],
  },
  "DexScreener - Liquidity": {
    source: "dexscreener",
    enableKey: "liquidity_enabled",
    fields: [
      {
        key: "min_liquidity_usd",
        label: "Min Liquidity",
        type: "number",
        unit: "USD",
        min: 0,
        max: 10000000,
        step: 10,
        hint: "$1 very low, $1000+ for serious trading",
        impact: "critical",
      },
      {
        key: "max_liquidity_usd",
        label: "Max Liquidity",
        type: "number",
        unit: "USD",
        min: 100,
        max: 1000000000,
        step: 100000,
        hint: "High max to avoid filtering established tokens",
        impact: "medium",
      },
    ],
  },
  "DexScreener - Market Cap": {
    source: "dexscreener",
    enableKey: "market_cap_enabled",
    fields: [
      {
        key: "min_market_cap_usd",
        label: "Min Market Cap",
        type: "number",
        unit: "USD",
        min: 0,
        max: 10000000,
        step: 100,
        hint: "$1000 filters micro-cap tokens",
        impact: "high",
      },
      {
        key: "max_market_cap_usd",
        label: "Max Market Cap",
        type: "number",
        unit: "USD",
        min: 1000,
        max: 1000000000,
        step: 100000,
        hint: "Filters out large-cap tokens",
        impact: "high",
      },
    ],
  },
  "DexScreener - Activity": {
    source: "dexscreener",
    enableKey: "transactions_enabled",
    fields: [
      {
        key: "min_transactions_5min",
        label: "Min TX (5min)",
        type: "number",
        unit: "txs",
        min: 0,
        max: 1000,
        step: 1,
        hint: "Min transactions in last 5 minutes (1+ is minimal)",
        impact: "medium",
      },
      {
        key: "min_transactions_1h",
        label: "Min TX (1h)",
        type: "number",
        unit: "txs",
        min: 0,
        max: 10000,
        step: 5,
        hint: "Min transactions in last hour (sustained activity)",
        impact: "medium",
      },
    ],
  },
  "RugCheck - Risk Score": {
    source: "rugcheck",
    enableKey: "risk_score_enabled",
    fields: [
      {
        key: "max_risk_score",
        label: "Max Risk Score",
        type: "number",
        unit: "score",
        min: 0,
        max: 100000,
        step: 100,
        hint: "Lower = safer. Max acceptable risk score (0 = safest, 100000+ = highest risk)",
        impact: "critical",
      },
    ],
  },
  "RugCheck - Holder Distribution": {
    source: "rugcheck",
    enableKey: "holder_distribution_enabled",
    fields: [
      {
        key: "max_top_holder_pct",
        label: "Max Top Holder %",
        type: "number",
        unit: "%",
        min: 0,
        max: 100,
        step: 1,
        hint: "15% means top holder can own max 15% supply",
        impact: "critical",
      },
      {
        key: "max_top_3_holders_pct",
        label: "Max Top 3 Holders %",
        type: "number",
        unit: "%",
        min: 0,
        max: 100,
        step: 1,
        hint: "Combined max for top 3 holders (lower = more distributed)",
        impact: "high",
      },
      {
        key: "min_unique_holders",
        label: "Min Unique Holders",
        type: "number",
        unit: "holders",
        min: 0,
        max: 1000000,
        step: 50,
        hint: "500+ indicates community adoption",
        impact: "medium",
      },
    ],
  },
  "RugCheck - LP Lock": {
    source: "rugcheck",
    enableKey: "lp_lock_enabled",
    fields: [
      {
        key: "min_pumpfun_lp_lock_pct",
        label: "Min PumpFun LP Lock",
        type: "number",
        unit: "%",
        min: 0,
        max: 100,
        step: 5,
        hint: "50%+ reduces rug risk for PumpFun tokens",
        impact: "high",
      },
      {
        key: "min_regular_lp_lock_pct",
        label: "Min Regular LP Lock",
        type: "number",
        unit: "%",
        min: 0,
        max: 100,
        step: 5,
        hint: "50%+ indicates locked liquidity for regular tokens",
        impact: "high",
      },
    ],
  },
  "RugCheck - Authorities": {
    source: "rugcheck",
    enableKey: "authority_checks_enabled",
    fields: [
      {
        key: "require_authorities_safe",
        label: "Require Authorities Safe",
        type: "boolean",
        hint: "Reject if authorities are not safe (recommended: true)",
        impact: "critical",
      },
      {
        key: "mint_authority_enabled",
        label: "Enable Mint Authority Check",
        type: "boolean",
        hint: "Check for mint authority presence",
        impact: "high",
      },
      {
        key: "allow_mint_authority",
        label: "Allow Mint Authority",
        type: "boolean",
        hint: "Allow tokens with mint authority (false = reject if present)",
        impact: "high",
      },
      {
        key: "freeze_authority_enabled",
        label: "Enable Freeze Authority Check",
        type: "boolean",
        hint: "Check for freeze authority presence",
        impact: "high",
      },
      {
        key: "allow_freeze_authority",
        label: "Allow Freeze Authority",
        type: "boolean",
        hint: "Allow tokens with freeze authority (false = reject if present)",
        impact: "high",
      },
    ],
  },
  "RugCheck - Risk Level": {
    source: "rugcheck",
    enableKey: "risk_level_enabled",
    fields: [
      {
        key: "block_danger_level",
        label: "Block High Risk Tokens",
        type: "boolean",
        hint: "Reject tokens with 'Danger' risk level",
        impact: "high",
      },
    ],
  },
  "RugCheck - Security Flags": {
    source: "rugcheck",
    fields: [
      {
        key: "block_rugged_tokens",
        label: "Block Rugged Tokens",
        type: "boolean",
        hint: "Reject tokens flagged as rugged by RugCheck",
        impact: "critical",
      },
    ],
  },
  "RugCheck - Insider Detection": {
    source: "rugcheck",
    enableKey: "insider_holder_checks_enabled",
    fields: [
      {
        key: "max_graph_insiders",
        label: "Max Graph Insiders",
        type: "number",
        unit: "wallets",
        min: 0,
        max: 20,
        step: 1,
        hint: "Maximum detected insider wallets (0 = no limit)",
        impact: "high",
      },
      {
        key: "max_insider_holders_in_top_10",
        label: "Max Insider Holders in Top 10",
        type: "number",
        unit: "holders",
        min: 0,
        max: 10,
        step: 1,
        hint: "Maximum insider wallets allowed in top 10 holders",
        impact: "high",
      },
      {
        key: "max_insider_total_pct",
        label: "Max Insider Total %",
        type: "number",
        unit: "%",
        min: 0,
        max: 100,
        step: 5,
        hint: "Maximum combined % held by all insider wallets",
        impact: "high",
      },
    ],
  },
  "RugCheck - Creator Checks": {
    source: "rugcheck",
    fields: [
      {
        key: "max_creator_balance_pct",
        label: "Max Creator Balance %",
        type: "number",
        unit: "%",
        min: 0,
        max: 100,
        step: 5,
        hint: "Maximum % creator can hold (0 = no limit)",
        impact: "medium",
      },
    ],
  },
  "RugCheck - LP Providers": {
    source: "rugcheck",
    fields: [
      {
        key: "min_lp_providers",
        label: "Min LP Providers",
        type: "number",
        unit: "providers",
        min: 0,
        max: 100,
        step: 1,
        hint: "Minimum LP providers required (0 = no limit)",
        impact: "medium",
      },
    ],
  },
  "RugCheck - Transfer Fees": {
    source: "rugcheck",
    enableKey: "transfer_fee_enabled",
    fields: [
      {
        key: "max_transfer_fee_pct",
        label: "Max Transfer Fee %",
        type: "number",
        unit: "%",
        min: 0,
        max: 100,
        step: 1,
        hint: "Maximum acceptable transfer fee percentage (5% recommended)",
        impact: "critical",
      },
      {
        key: "block_transfer_fee_tokens",
        label: "Block Any Transfer Fee",
        type: "boolean",
        hint: "Reject tokens with any transfer fee at all",
        impact: "high",
      },
    ],
  },
};

// ============================================================================
// NESTED CONFIG HELPERS
// ============================================================================

// Get value from nested config structure
function getConfigValue(config, source, key) {
  if (source === "meta") {
    return config[key];
  }
  return config[source]?.[key];
}

// Set value in nested config structure
function setConfigValue(config, source, key, value) {
  if (source === "meta") {
    config[key] = value;
  } else {
    if (!config[source]) {
      config[source] = {};
    }
    config[source][key] = value;
  }
}

// Get source enable status
function getSourceEnabled(config, source) {
  if (source === "meta") return true; // Meta is always enabled
  return config[source]?.enabled !== false;
}

// Set source enable status
function setSourceEnabled(config, source, enabled) {
  if (source === "meta") return; // Meta cannot be disabled
  if (!config[source]) {
    config[source] = {};
  }
  config[source].enabled = enabled;
}

// Get category enable status (for categories with enableKey)
function getCategoryEnabled(config, source, enableKey) {
  if (!enableKey) return true; // No enable key means always enabled
  if (source === "meta") return true;
  return config[source]?.[enableKey] !== false;
}

// Set category enable status
function setCategoryEnabled(config, source, enableKey, enabled) {
  if (!enableKey || source === "meta") return;
  if (!config[source]) {
    config[source] = {};
  }
  config[source][enableKey] = enabled;
}

// Deep merge helper so imports keep nested source-level settings intact
function deepMerge(target, source) {
  const output = !target || typeof target !== "object" || Array.isArray(target) ? {} : target;
  if (!source || typeof source !== "object" || Array.isArray(source)) {
    return output;
  }

  for (const [key, value] of Object.entries(source)) {
    if (value && typeof value === "object" && !Array.isArray(value)) {
      const existing = output[key];
      output[key] = deepMerge(existing, value);
    } else {
      output[key] = value;
    }
  }

  return output;
}

// Compare configs for changes
function configsEqual(config1, config2) {
  const flat1 = JSON.stringify(config1);
  const flat2 = JSON.stringify(config2);
  return flat1 === flat2;
}

// ============================================================================
// API CALLS
// ============================================================================

async function fetchConfig() {
  const response = await fetch("/api/config/filtering");
  if (!response.ok) {
    throw new Error(`Failed to fetch config: ${response.statusText}`);
  }
  return response.json();
}

async function saveConfig(config) {
  const response = await fetch("/api/config/filtering", {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(config),
  });
  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: response.statusText }));
    throw new Error(error.message || "Failed to save config");
  }
  return response.json();
}

async function refreshSnapshot() {
  const response = await fetch("/api/filtering/refresh", {
    method: "POST",
  });
  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: response.statusText }));
    throw new Error(error.message || "Failed to refresh snapshot");
  }
  return response.json();
}

async function fetchStats() {
  const response = await fetch("/api/filtering/stats");
  if (!response.ok) {
    throw new Error(`Failed to fetch stats: ${response.statusText}`);
  }
  return response.json();
}

// ============================================================================
// RENDERING
// ============================================================================

function renderStatsCard(title, value, meta) {
  return `
    <div class="filtering-stat-card">
      <h3>${Utils.escapeHtml(title)}</h3>
      <div class="value">${Utils.escapeHtml(value)}</div>
      ${meta ? `<div class="meta">${Utils.escapeHtml(meta)}</div>` : ""}
    </div>
  `;
}

function renderStats() {
  if (!state.stats) {
    return '<div class="filtering-stats">Loading statistics...</div>';
  }

  const {
    total_tokens,
    with_pool_price,
    open_positions,
    blacklisted,
    secure_tokens,
    with_ohlcv,
    passed_filtering,
    updated_at,
  } = state.stats;

  const priceRate = total_tokens > 0 ? ((with_pool_price / total_tokens) * 100).toFixed(1) : 0;
  const passedRate = total_tokens > 0 ? ((passed_filtering / total_tokens) * 100).toFixed(1) : 0;
  const cacheAge = updated_at ? Utils.formatTimeAgo(new Date(updated_at)) : "Never";

  return `
    <div class="filtering-stats">
      ${renderStatsCard("Total Tokens", Utils.formatNumber(total_tokens), "In filtering cache")}
      ${renderStatsCard(
        "With Price",
        Utils.formatNumber(with_pool_price),
        `${priceRate}% have pool price`
      )}
      ${renderStatsCard(
        "Passed Filtering",
        Utils.formatNumber(passed_filtering),
        `${passedRate}% passed all criteria`
      )}
      ${renderStatsCard(
        "Open Positions",
        Utils.formatNumber(open_positions),
        "Active trading positions"
      )}
      ${renderStatsCard(
        "Secure Tokens",
        Utils.formatNumber(secure_tokens),
        "Meeting security threshold"
      )}
      ${renderStatsCard("Blacklisted", Utils.formatNumber(blacklisted), "Flagged tokens")}
      ${renderStatsCard("With OHLCV", Utils.formatNumber(with_ohlcv), "Historical data available")}
      ${renderStatsCard("Cache Age", cacheAge, updated_at ? new Date(updated_at).toLocaleString() : "")}
    </div>
  `;
}

function renderConfigField(field, source) {
  const value = getConfigValue(state.draft, source, field.key);
  const originalValue = getConfigValue(state.config, source, field.key);
  const isChanged = state.config && value !== originalValue;
  const fieldClass = isChanged ? "filtering-config-field changed" : "filtering-config-field";
  const fieldId = `field-${source}-${field.key}`;

  if (field.type === "boolean") {
    return `
      <div class="${fieldClass}">
        <label>
          <span class="field-label">${Utils.escapeHtml(field.label)}</span>
          <span class="field-hint">${Utils.escapeHtml(field.hint)}</span>
        </label>
        <div>
          <input
            type="checkbox"
            id="${fieldId}"
            data-field="${field.key}"
            data-source="${source}"
            ${value ? "checked" : ""}
          />
          <div class="field-meta">
            <span class="field-impact ${field.impact}">${field.impact}</span>
          </div>
        </div>
      </div>
    `;
  }

  return `
    <div class="${fieldClass}">
      <label>
        <span class="field-label">${Utils.escapeHtml(field.label)}</span>
        <span class="field-hint">${Utils.escapeHtml(field.hint)}</span>
      </label>
      <div>
        <input
          type="number"
          id="${fieldId}"
          data-field="${field.key}"
          data-source="${source}"
          value="${value}"
          min="${field.min}"
          max="${field.max}"
          step="${field.step}"
        />
        <div class="field-meta">
          <span class="field-unit">${Utils.escapeHtml(field.unit)}</span>
          <span class="field-impact ${field.impact}">${field.impact}</span>
        </div>
      </div>
    </div>
  `;
}

function renderSourceToggle(source, _categoryName) {
  if (source === "meta") return "";

  const enabled = getSourceEnabled(state.draft, source);
  const sourceLabel = source === "dexscreener" ? "DexScreener" : "RugCheck";
  const toggleId = `source-toggle-${source}`;

  return `
    <label class="source-switch">
      <input
        type="checkbox"
        id="${toggleId}"
        data-source-toggle="${source}"
        ${enabled ? "checked" : ""}
      />
      <span class="slider"></span>
      <span class="label">${sourceLabel}</span>
    </label>
  `;
}

function renderCategoryToggle(source, enableKey, _categoryName) {
  if (!enableKey || source === "meta") return "";

  const enabled = getCategoryEnabled(state.draft, source, enableKey);
  const toggleId = `category-toggle-${source}-${enableKey}`;

  return `
    <label class="category-switch">
      <input
        type="checkbox"
        id="${toggleId}"
        data-category-toggle="${source}"
        data-enable-key="${enableKey}"
        ${enabled ? "checked" : ""}
      />
      <span class="slider"></span>
    </label>
  `;
}

function renderConfigCategory(categoryName, categoryData) {
  const { source, enableKey, fields } = categoryData;
  const sourceEnabled = getSourceEnabled(state.draft, source);
  const categoryEnabled = getCategoryEnabled(state.draft, source, enableKey);
  const isDisabled = (source !== "meta" && !sourceEnabled) || (enableKey && !categoryEnabled);
  const disabledClass = isDisabled ? "disabled" : "";
  const matchesSearch = (field) =>
    !state.searchQuery ||
    field.label.toLowerCase().includes(state.searchQuery) ||
    field.key.toLowerCase().includes(state.searchQuery);

  return `
    <div class="filter-card ${disabledClass}" data-source="${source}">
      <div class="card-header">
        <h3>${Utils.escapeHtml(categoryName)}</h3>
        ${renderCategoryToggle(source, enableKey, categoryName)}
      </div>
      <div class="card-body">
        ${
          fields
            .filter(matchesSearch)
            .map((field) => renderConfigField(field, source))
            .join("") || '<div class="no-matches">No fields match</div>'
        }
      </div>
    </div>
  `;
}

function renderConfigEditor() {
  if (!state.draft) {
    return '<div class="filtering-config-editor">Loading configuration...</div>';
  }

  // Group categories by source
  const metaCategories = [];
  const dexCategories = [];
  const rugCategories = [];

  Object.entries(CONFIG_CATEGORIES).forEach(([name, data]) => {
    const item = { name, data };
    if (data.source === "meta") metaCategories.push(item);
    else if (data.source === "dexscreener") dexCategories.push(item);
    else if (data.source === "rugcheck") rugCategories.push(item);
  });

  return `
    <div class="filtering-layout">
      <div class="tabs-container">
        <button class="tab ${state.activeTab === "meta" ? "active" : ""}" data-tab="meta">
          ⚙️ Core Settings
        </button>
        <button class="tab ${state.activeTab === "dexscreener" ? "active" : ""}" data-tab="dexscreener">
          📊 DexScreener
          ${renderSourceToggle("dexscreener", "")}
        </button>
        <button class="tab ${state.activeTab === "rugcheck" ? "active" : ""}" data-tab="rugcheck">
          🛡️ RugCheck
          ${renderSourceToggle("rugcheck", "")}
        </button>
      </div>
      
      <div class="search-bar">
        <input type="text" id="filtering-search" placeholder="🔍 Search settings..." value="${Utils.escapeHtml(state.searchQuery)}" />
      </div>

      <div class="tab-content">
        <div class="tab-panel ${state.activeTab === "meta" ? "active" : ""}" data-panel="meta">
          <div class="cards-grid">
            ${metaCategories.map(({ name, data }) => renderConfigCategory(name, data)).join("")}
          </div>
        </div>
        <div class="tab-panel ${state.activeTab === "dexscreener" ? "active" : ""}" data-panel="dexscreener">
          <div class="cards-grid">
            ${dexCategories.map(({ name, data }) => renderConfigCategory(name, data)).join("")}
          </div>
        </div>
        <div class="tab-panel ${state.activeTab === "rugcheck" ? "active" : ""}" data-panel="rugcheck">
          <div class="cards-grid">
            ${rugCategories.map(({ name, data }) => renderConfigCategory(name, data)).join("")}
          </div>
        </div>
      </div>
    </div>
  `;
}

function renderActions() {
  const saveDisabled = !state.hasChanges || state.isSaving;
  const refreshDisabled = state.isRefreshing;

  let statusMessage = "";
  if (state.isSaving) {
    statusMessage = "Saving...";
  } else if (state.isRefreshing) {
    statusMessage = "Refreshing snapshot...";
  } else if (state.lastSaved) {
    statusMessage = `Last saved ${Utils.formatTimeAgo(state.lastSaved)}`;
  }

  return `
    <div class="filtering-actions">
      <button
        class="primary"
        id="save-config-btn"
        ${saveDisabled ? "disabled" : ""}
      >
        💾 Save Changes
      </button>
      <button
        class="secondary"
        id="reset-config-btn"
        ${!state.hasChanges ? "disabled" : ""}
      >
        ↩️ Reset
      </button>
      <button
        class="secondary"
        id="refresh-snapshot-btn"
        ${refreshDisabled ? "disabled" : ""}
      >
        🔄 Refresh Snapshot
      </button>
      <button class="secondary" id="export-config-btn">
        📤 Export
      </button>
      <button class="secondary" id="import-config-btn">
        📥 Import
      </button>
      ${statusMessage ? `<span class="status-message">${Utils.escapeHtml(statusMessage)}</span>` : ""}
    </div>
  `;
}

function render() {
  const root = $("#filtering-root");
  if (!root) return;

  root.innerHTML = `
    <div class="filtering-header">
      <h1>🎯 Filtering Configuration</h1>
      <p>Configure token filtering rules and monitor filtering performance</p>
    </div>
    <div id="filtering-stats-container">
      ${renderStats()}
    </div>
    ${renderConfigEditor()}
    ${renderActions()}
  `;

  attachEventListeners();
}

// Update only the stats section without re-rendering inputs
function updateStats() {
  const statsContainer = $("#filtering-stats-container");
  if (statsContainer) {
    statsContainer.innerHTML = renderStats();
  }
}

// ============================================================================
// EVENT HANDLERS
// ============================================================================

function attachEventListeners() {
  // Field change handlers
  $$("[data-field]").forEach((input) => {
    input.addEventListener("input", handleFieldChange);
  });

  // Source toggle handlers
  $$("[data-source-toggle]").forEach((input) => {
    input.addEventListener("change", handleSourceToggle);
  });

  // Category toggle handlers
  $$("[data-category-toggle]").forEach((input) => {
    input.addEventListener("change", handleCategoryToggle);
  });

  // Tab navigation
  $$(".tab").forEach((tab) => {
    tab.addEventListener("click", (e) => {
      const tabName = e.currentTarget.dataset.tab;
      if (tabName && tabName !== state.activeTab) {
        state.activeTab = tabName;
        AppState.save("filtering_activeTab", state.activeTab);
        render();
      }
    });
  });

  // Action buttons
  const saveBtn = $("#save-config-btn");
  const resetBtn = $("#reset-config-btn");
  const refreshBtn = $("#refresh-snapshot-btn");
  const exportBtn = $("#export-config-btn");
  const importBtn = $("#import-config-btn");

  if (saveBtn) saveBtn.addEventListener("click", handleSaveConfig);
  if (resetBtn) resetBtn.addEventListener("click", handleResetConfig);
  if (refreshBtn) refreshBtn.addEventListener("click", handleRefreshSnapshot);
  if (exportBtn) exportBtn.addEventListener("click", handleExportConfig);
  if (importBtn) importBtn.addEventListener("click", handleImportConfig);

  // Search
  const searchInput = $("#filtering-search");
  if (searchInput) {
    searchInput.addEventListener("input", (e) => {
      state.searchQuery = (e.target.value || "").toLowerCase();
      render();
    });
  }
}

function handleFieldChange(event) {
  const input = event.target;
  const fieldKey = input.dataset.field;
  const source = input.dataset.source;

  if (!fieldKey || !source) return;

  let value;
  if (input.type === "checkbox") {
    value = input.checked;
  } else if (input.type === "number") {
    value = parseFloat(input.value);
    if (isNaN(value)) return;
  } else {
    value = input.value;
  }

  setConfigValue(state.draft, source, fieldKey, value);
  checkForChanges();
  updateActionButtons();
}

function handleSourceToggle(event) {
  const input = event.target;
  const source = input.dataset.sourceToggle;
  const enabled = input.checked;

  setSourceEnabled(state.draft, source, enabled);
  checkForChanges();
  render(); // Need full re-render to update disabled states
}

function handleCategoryToggle(event) {
  const input = event.target;
  const source = input.dataset.categoryToggle;
  const enableKey = input.dataset.enableKey;
  const enabled = input.checked;

  setCategoryEnabled(state.draft, source, enableKey, enabled);
  checkForChanges();
  render(); // Need full re-render to update disabled states
}

function updateActionButtons() {
  const saveBtn = $("#save-config-btn");
  const resetBtn = $("#reset-config-btn");

  if (saveBtn) {
    saveBtn.disabled = !state.hasChanges || state.isSaving;
  }
  if (resetBtn) {
    resetBtn.disabled = !state.hasChanges;
  }
}

function checkForChanges() {
  if (!state.config || !state.draft) {
    state.hasChanges = false;
    return;
  }

  state.hasChanges = !configsEqual(state.config, state.draft);
}

async function handleSaveConfig() {
  if (!state.hasChanges || state.isSaving) return;

  state.isSaving = true;
  updateActionButtons();

  try {
    await saveConfig(state.draft);
    state.config = JSON.parse(JSON.stringify(state.draft)); // Deep clone
    state.hasChanges = false;
    state.lastSaved = new Date();
    Utils.showToast("Configuration saved successfully", "success");
  } catch (error) {
    console.error("Failed to save config:", error);
    Utils.showToast(`Failed to save: ${error.message}`, "error");
  } finally {
    state.isSaving = false;
    updateActionButtons();
  }
}

function handleResetConfig() {
  if (!state.hasChanges) return;

  state.draft = JSON.parse(JSON.stringify(state.config)); // Deep clone
  state.hasChanges = false;
  render(); // Need full render to restore original values
  Utils.showToast("Changes reset", "info");
}

async function handleRefreshSnapshot() {
  if (state.isRefreshing) return;

  state.isRefreshing = true;
  updateActionButtons();

  try {
    await refreshSnapshot();
    Utils.showToast("Filtering snapshot refreshed", "success");
    // Reload stats after refresh
    setTimeout(() => loadStats(), 1000);
  } catch (error) {
    console.error("Failed to refresh snapshot:", error);
    Utils.showToast(`Failed to refresh: ${error.message}`, "error");
  } finally {
    state.isRefreshing = false;
    updateActionButtons();
  }
}

function handleExportConfig() {
  const json = JSON.stringify(state.draft, null, 2);
  const blob = new Blob([json], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `filtering-config-${Date.now()}.json`;
  a.click();
  URL.revokeObjectURL(url);
  Utils.showToast("Configuration exported", "success");
}

function handleImportConfig() {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".json";
  input.addEventListener("change", async (e) => {
    const file = e.target.files[0];
    if (!file) return;

    try {
      const text = await file.text();
      const imported = JSON.parse(text);
      const current = JSON.parse(JSON.stringify(state.draft ?? {}));
      state.draft = deepMerge(current, imported);
      checkForChanges();
      render();
      Utils.showToast("Configuration imported", "success");
    } catch (error) {
      console.error("Failed to import config:", error);
      Utils.showToast(`Failed to import: ${error.message}`, "error");
    }
  });
  input.click();
}

// ============================================================================
// DATA LOADING
// ============================================================================

async function loadConfig() {
  try {
    const response = await fetchConfig();
    state.config = response.data || response;
    state.draft = JSON.parse(JSON.stringify(state.config)); // Deep clone for nested structures
    state.hasChanges = false;
  } catch (error) {
    console.error("Failed to load config:", error);
    Utils.showToast(`Failed to load config: ${error.message}`, "error");
  }
}

async function loadStats() {
  try {
    const response = await fetchStats();
    state.stats = response.data || response;
  } catch (error) {
    console.error("Failed to load stats:", error);
    // Don't show toast for stats errors (non-critical)
  }
}

async function loadData() {
  await Promise.all([loadConfig(), loadStats()]);
  render();
}

// ============================================================================
// LIFECYCLE
// ============================================================================

let poller = null;

export function createLifecycle() {
  return {
    async init() {
      console.log("[Filtering] Initializing");
      await loadData();
    },

    activate(ctx) {
      console.log("[Filtering] Activating");

      if (!poller) {
        poller = ctx.managePoller(
          new Poller(
            async () => {
              await loadStats();
              updateStats(); // Update only stats, don't re-render inputs
            },
            { label: "Filtering Stats" }
          )
        );
      }

      poller.start();
    },

    deactivate() {
      console.log("[Filtering] Deactivating");
      if (poller) {
        poller.stop();
      }
    },

    dispose() {
      console.log("[Filtering] Disposing");
      if (poller) {
        poller.stop();
        poller = null;
      }
    },
  };
}

// Register the page with the lifecycle system
registerPage("filtering", createLifecycle());
