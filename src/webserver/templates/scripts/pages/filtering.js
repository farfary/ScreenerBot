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
};

// ============================================================================
// CONFIGURATION METADATA
// ============================================================================

const CONFIG_CATEGORIES = {
  Performance: [
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
      max: 50000,
      step: 500,
      hint: "Max tokens to evaluate before filtering",
      impact: "medium",
    },
  ],
  Requirements: [
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
  Age: [
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
  Activity: [
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
  Liquidity: [
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
  "Market Cap": [
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
  Security: [
    {
      key: "min_security_score",
      label: "Min Security Score",
      type: "number",
      unit: "score",
      min: 0,
      max: 100,
      step: 5,
      hint: "10+ decent, 50+ safer (rugcheck score)",
      impact: "critical",
    },
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
  Community: [
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
};

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

function renderConfigField(field) {
  const value = state.draft[field.key];
  const isChanged = state.config && value !== state.config[field.key];
  const fieldClass = isChanged ? "filtering-config-field changed" : "filtering-config-field";

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
            data-field="${field.key}"
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
          data-field="${field.key}"
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

function renderConfigCategory(categoryName, fields) {
  return `
    <div class="filtering-config-category">
      <h3>${Utils.escapeHtml(categoryName)}</h3>
      ${fields.map(renderConfigField).join("")}
    </div>
  `;
}

function renderConfigEditor() {
  if (!state.draft) {
    return '<div class="filtering-config-editor">Loading configuration...</div>';
  }

  return `
    <div class="filtering-config-editor">
      ${Object.entries(CONFIG_CATEGORIES)
        .map(([category, fields]) => renderConfigCategory(category, fields))
        .join("")}
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
  // Input change handlers
  $$("[data-field]").forEach((input) => {
    input.addEventListener("input", handleFieldChange);
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
}

function handleFieldChange(e) {
  const field = e.target.getAttribute("data-field");
  const value = e.target.type === "checkbox" ? e.target.checked : parseFloat(e.target.value);

  state.draft[field] = value;
  checkForChanges();
  updateSaveButton();
}

function updateSaveButton() {
  const saveBtn = $("#save-config-btn");
  if (saveBtn) {
    saveBtn.disabled = !state.hasChanges || state.isSaving;
  }
}

function updateActionButtons() {
  const actionsContainer = $(".filtering-actions");
  if (actionsContainer) {
    actionsContainer.innerHTML = renderActions();
    // Re-attach event listeners for action buttons
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
  }
}

function checkForChanges() {
  if (!state.config || !state.draft) {
    state.hasChanges = false;
    return;
  }

  state.hasChanges = Object.keys(state.draft).some((key) => state.draft[key] !== state.config[key]);
}

async function handleSaveConfig() {
  if (!state.hasChanges || state.isSaving) return;

  state.isSaving = true;
  updateActionButtons();

  try {
    await saveConfig(state.draft);
    state.config = { ...state.draft };
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

  state.draft = { ...state.config };
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
      state.draft = { ...state.draft, ...imported };
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
    state.draft = { ...state.config };
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
