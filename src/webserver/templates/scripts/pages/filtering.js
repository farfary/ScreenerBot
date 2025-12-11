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
import { requestManager } from "../core/request_manager.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";

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
  searchQuery: AppState.load("filtering_searchQuery", ""),
  activeTab: AppState.load("filtering_activeTab", "status"),
  initialized: false,
};

const FILTER_TABS = [
  { id: "status", label: '<i class="icon-chart-bar"></i> Status' },
  { id: "meta", label: '<i class="icon-settings"></i> Core' },
  { id: "dexscreener", label: '<i class="icon-trending-up"></i> DexScreener' },
  { id: "geckoterminal", label: '<i class="icon-trending-up"></i> GeckoTerminal' },
  { id: "rugcheck", label: '<i class="icon-shield"></i> RugCheck' },
];

const TABBAR_STATE_KEY = "filtering.tab";

let tabBar = null;
const eventCleanups = [];

// Helper to track event listeners
function addTrackedListener(element, event, handler) {
  if (!element) return;
  element.addEventListener(event, handler);
  eventCleanups.push(() => element.removeEventListener(event, handler));
}

// ============================================================================
// CONFIGURATION METADATA
// ============================================================================

const CONFIG_CATEGORIES = {
  "Meta Requirements - Cooldown": {
    source: "meta",
    enableKey: "cooldown_enabled",
    fields: [
      {
        key: "check_cooldown",
        label: "Check Cooldown",
        type: "boolean",
        hint: "Skip tokens in cooldown period after exit",
        impact: "high",
      },
    ],
  },
  "Meta Requirements - Age": {
    source: "meta",
    enableKey: "age_enabled",
    fields: [
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
  "DexScreener - Volume": {
    source: "dexscreener",
    enableKey: "volume_enabled",
    fields: [
      {
        key: "min_volume_24h",
        label: "Min Volume 24h",
        type: "number",
        unit: "USD",
        min: 0,
        max: 10000000,
        step: 100,
        hint: "Minimum 24h trading volume in USD",
        impact: "medium",
      },
    ],
  },
  "DexScreener - Price Change": {
    source: "dexscreener",
    enableKey: "price_change_enabled",
    fields: [
      {
        key: "min_price_change_h1",
        label: "Min Price Change 1h",
        type: "number",
        unit: "%",
        min: -100,
        max: 10000,
        step: 5,
        hint: "Minimum 1h price change % (negative = dump filter)",
        impact: "low",
      },
      {
        key: "max_price_change_h1",
        label: "Max Price Change 1h",
        type: "number",
        unit: "%",
        min: 0,
        max: 100000,
        step: 50,
        hint: "Maximum 1h price change % (filter extreme pumps)",
        impact: "low",
      },
    ],
  },
  "GeckoTerminal - Liquidity": {
    source: "geckoterminal",
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
        hint: "Minimum liquidity in USD",
        impact: "critical",
      },
      {
        key: "max_liquidity_usd",
        label: "Max Liquidity",
        type: "number",
        unit: "USD",
        min: 0,
        max: 1000000000,
        step: 10000,
        hint: "Maximum liquidity in USD",
        impact: "medium",
      },
    ],
  },
  "GeckoTerminal - Market Cap": {
    source: "geckoterminal",
    enableKey: "market_cap_enabled",
    fields: [
      {
        key: "min_market_cap_usd",
        label: "Min Market Cap",
        type: "number",
        unit: "USD",
        min: 0,
        max: 1000000000,
        step: 1000,
        hint: "Minimum market cap in USD",
        impact: "medium",
      },
      {
        key: "max_market_cap_usd",
        label: "Max Market Cap",
        type: "number",
        unit: "USD",
        min: 0,
        max: 1000000000,
        step: 1000,
        hint: "Maximum market cap in USD",
        impact: "medium",
      },
    ],
  },
  "GeckoTerminal - Volume": {
    source: "geckoterminal",
    enableKey: "volume_enabled",
    fields: [
      {
        key: "min_volume_5m",
        label: "Min Volume 5m",
        type: "number",
        unit: "USD",
        min: 0,
        max: 1000000,
        step: 10,
        hint: "Minimum 5 minute trading volume in USD",
        impact: "medium",
      },
      {
        key: "min_volume_1h",
        label: "Min Volume 1h",
        type: "number",
        unit: "USD",
        min: 0,
        max: 10000000,
        step: 10,
        hint: "Minimum 1 hour trading volume in USD",
        impact: "medium",
      },
      {
        key: "min_volume_24h",
        label: "Min Volume 24h",
        type: "number",
        unit: "USD",
        min: 0,
        max: 10000000,
        step: 100,
        hint: "Minimum 24 hour trading volume in USD",
        impact: "medium",
      },
    ],
  },
  "GeckoTerminal - Price Change": {
    source: "geckoterminal",
    enableKey: "price_change_enabled",
    fields: [
      {
        key: "min_price_change_m5",
        label: "Min Price Change 5m",
        type: "number",
        unit: "%",
        min: -100,
        max: 10000,
        step: 5,
        hint: "Minimum 5 minute price change %",
        impact: "low",
      },
      {
        key: "max_price_change_m5",
        label: "Max Price Change 5m",
        type: "number",
        unit: "%",
        min: 0,
        max: 100000,
        step: 50,
        hint: "Maximum 5 minute price change %",
        impact: "low",
      },
      {
        key: "min_price_change_h1",
        label: "Min Price Change 1h",
        type: "number",
        unit: "%",
        min: -100,
        max: 10000,
        step: 5,
        hint: "Minimum 1 hour price change %",
        impact: "low",
      },
      {
        key: "max_price_change_h1",
        label: "Max Price Change 1h",
        type: "number",
        unit: "%",
        min: 0,
        max: 100000,
        step: 50,
        hint: "Maximum 1 hour price change %",
        impact: "low",
      },
      {
        key: "min_price_change_h24",
        label: "Min Price Change 24h",
        type: "number",
        unit: "%",
        min: -100,
        max: 10000,
        step: 5,
        hint: "Minimum 24 hour price change %",
        impact: "low",
      },
      {
        key: "max_price_change_h24",
        label: "Max Price Change 24h",
        type: "number",
        unit: "%",
        min: 0,
        max: 100000,
        step: 50,
        hint: "Maximum 24 hour price change %",
        impact: "low",
      },
    ],
  },
  "GeckoTerminal - Pool Metrics": {
    source: "geckoterminal",
    enableKey: "pool_metrics_enabled",
    fields: [
      {
        key: "min_pool_count",
        label: "Min Pool Count",
        type: "number",
        unit: "pools",
        min: 0,
        max: 1000,
        step: 1,
        hint: "Minimum number of pools tracked",
        impact: "low",
      },
      {
        key: "max_pool_count",
        label: "Max Pool Count",
        type: "number",
        unit: "pools",
        min: 0,
        max: 1000,
        step: 1,
        hint: "Maximum number of pools tracked",
        impact: "low",
      },
      {
        key: "min_reserve_usd",
        label: "Min Reserve USD",
        type: "number",
        unit: "USD",
        min: 0,
        max: 100000000,
        step: 100,
        hint: "Minimum reserve liquidity across pools in USD",
        impact: "low",
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
        key: "allow_mint_authority",
        label: "Allow Mint Authority",
        type: "boolean",
        hint: "Allow tokens with mint authority (false = reject if present)",
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
    enableKey: "rugged_check_enabled",
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
    enableKey: "graph_insiders_enabled",
    fields: [
      {
        key: "max_graph_insiders",
        label: "Max Graph Insiders",
        type: "number",
        unit: "wallets",
        min: 0,
        max: 20,
        step: 1,
        hint: "Maximum detected insider wallets",
        impact: "high",
      },
    ],
  },
  "RugCheck - Insider Holder Checks": {
    source: "rugcheck",
    enableKey: "insider_holder_checks_enabled",
    fields: [
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
    enableKey: "creator_balance_enabled",
    fields: [
      {
        key: "max_creator_balance_pct",
        label: "Max Creator Balance %",
        type: "number",
        unit: "%",
        min: 0,
        max: 100,
        step: 5,
        hint: "Maximum % creator can hold",
        impact: "medium",
      },
    ],
  },
  "RugCheck - LP Providers": {
    source: "rugcheck",
    enableKey: "lp_providers_enabled",
    fields: [
      {
        key: "min_lp_providers",
        label: "Min LP Providers",
        type: "number",
        unit: "providers",
        min: 0,
        max: 100,
        step: 1,
        hint: "Minimum LP providers required",
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
  if (source === "meta") {
    return config[enableKey] !== false;
  }
  return config[source]?.[enableKey] !== false;
}

// Set category enable status
function setCategoryEnabled(config, source, enableKey, enabled) {
  if (!enableKey) return;
  if (source === "meta") {
    config[enableKey] = enabled;
    return;
  }
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
  return await requestManager.fetch("/api/config/filtering", {
    priority: "high",
  });
}

async function saveConfig(config) {
  return await requestManager.fetch("/api/config/filtering", {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(config),
    priority: "high",
  });
}

async function refreshSnapshot() {
  return await requestManager.fetch("/api/filtering/refresh", {
    method: "POST",
    priority: "high",
  });
}

async function fetchStats() {
  return await requestManager.fetch("/api/filtering/stats", {
    priority: "normal",
  });
}

// ============================================================================
// RENDERING
// ============================================================================

// ============================================================================
// RENDERING
// ============================================================================

function renderInfoBar() {
  if (!state.stats) return "";

  const {
    total_tokens,
    with_pool_price,
    passed_filtering,
    open_positions,
    blacklisted,
    updated_at,
  } = state.stats;

  const priceRate = total_tokens > 0 ? ((with_pool_price / total_tokens) * 100).toFixed(1) : 0;
  const passedRate = total_tokens > 0 ? ((passed_filtering / total_tokens) * 100).toFixed(1) : 0;
  const cacheAge = updated_at ? Utils.formatTimeAgo(new Date(updated_at)) : "Never";

  return `
      <div class="info-item highlight">
        <span class="label">Total:</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(total_tokens, 0))}</span>
      </div>
      <div class="info-item">
        <span class="label">Priced:</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(with_pool_price, 0))} (${Utils.escapeHtml(priceRate)}%)</span>
      </div>
      <div class="info-item highlight">
        <span class="label">Passed:</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(passed_filtering, 0))} (${Utils.escapeHtml(passedRate)}%)</span>
      </div>
      <div class="info-item">
        <span class="label">Positions:</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(open_positions, 0))}</span>
      </div>
      <div class="info-item warning">
        <span class="label">Blacklisted:</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(blacklisted, 0))}</span>
      </div>
      <div class="info-item">
        <span class="label">Cache:</span>
        <span class="value">${Utils.escapeHtml(cacheAge)}</span>
      </div>
  `;
}

function renderStatusView() {
  if (!state.stats) return '<div class="filtering-config-empty">Loading statistics...</div>';

  const {
    total_tokens,
    with_pool_price,
    open_positions,
    blacklisted,
    with_ohlcv,
    passed_filtering,
    updated_at,
  } = state.stats;

  const priceRate = total_tokens > 0 ? ((with_pool_price / total_tokens) * 100).toFixed(1) : 0;
  const passedRate = total_tokens > 0 ? ((passed_filtering / total_tokens) * 100).toFixed(1) : 0;

  return `
    <div class="status-view">
      <div class="status-card dominant">
        <span class="metric-label">Total Tokens</span>
        <span class="metric-value">${Utils.escapeHtml(Utils.formatNumber(total_tokens, 0))}</span>
        <span class="metric-meta">In filtering cache</span>
      </div>
      <div class="status-card">
        <span class="metric-label">With Price</span>
        <span class="metric-value">${Utils.escapeHtml(Utils.formatNumber(with_pool_price, 0))}</span>
        <span class="metric-meta">${Utils.escapeHtml(`${priceRate}% have pricing`)}</span>
      </div>
      <div class="status-card dominant">
        <span class="metric-label">Passed Filters</span>
        <span class="metric-value">${Utils.escapeHtml(Utils.formatNumber(passed_filtering, 0))}</span>
        <span class="metric-meta">${Utils.escapeHtml(`${passedRate}% passed`)}</span>
      </div>
      <div class="status-card">
        <span class="metric-label">Open Positions</span>
        <span class="metric-value">${Utils.escapeHtml(Utils.formatNumber(open_positions, 0))}</span>
        <span class="metric-meta">Active trades</span>
      </div>
      <div class="status-card warning">
        <span class="metric-label">Blacklisted</span>
        <span class="metric-value">${Utils.escapeHtml(Utils.formatNumber(blacklisted, 0))}</span>
        <span class="metric-meta">Flagged tokens</span>
      </div>
      <div class="status-card">
        <span class="metric-label">With OHLCV</span>
        <span class="metric-value">${Utils.escapeHtml(Utils.formatNumber(with_ohlcv, 0))}</span>
        <span class="metric-meta">Historical data ready</span>
      </div>
      <div class="status-card">
        <span class="metric-label">Last Updated</span>
        <span class="metric-value">${updated_at ? Utils.escapeHtml(Utils.formatTimeAgo(new Date(updated_at))) : "Never"}</span>
        <span class="metric-meta">${updated_at ? Utils.escapeHtml(new Date(updated_at).toLocaleString()) : "No refresh yet"}</span>
      </div>
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

function renderCategoryToggle(source, enableKey, _categoryName) {
  if (!enableKey) return "";

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
  // Only disable the card body (fields) if source is disabled OR category is disabled
  // Keep header interactive so users can toggle categories on/off
  const isDisabled = (source !== "meta" && !sourceEnabled) || (enableKey && !categoryEnabled);
  const matchesSearch = (field) =>
    !state.searchQuery ||
    field.label.toLowerCase().includes(state.searchQuery) ||
    field.key.toLowerCase().includes(state.searchQuery);

  const disabledClass = isDisabled ? "disabled" : "";
  const visibleFields = fields.filter(matchesSearch);

  if (state.searchQuery && visibleFields.length === 0) {
    return "";
  }

  return `
    <div class="filter-card" data-source="${source}" data-category="${Utils.escapeHtml(categoryName)}">
      <div class="card-header">
        <h3>${Utils.escapeHtml(categoryName)}</h3>
        ${renderCategoryToggle(source, enableKey, categoryName)}
      </div>
      <div class="card-body ${disabledClass}">
        ${
          visibleFields.map((field) => renderConfigField(field, source)).join("") ||
          '<div class="no-matches">No fields match</div>'
        }
      </div>
    </div>
  `;
}

function renderConfigPanels() {
  if (!state.draft) {
    return '<div class="filtering-config-empty">Loading configuration...</div>';
  }

  // Status tab shows overview
  if (state.activeTab === "status") {
    return renderStatusView();
  }

  const targetSource = state.activeTab || "meta";
  const categories = Object.entries(CONFIG_CATEGORIES).filter(([, data]) => {
    if (targetSource === "meta") return data.source === "meta";
    return data.source === targetSource;
  });

  const cards = categories
    .map(([name, data]) => renderConfigCategory(name, data))
    .filter((html) => html && html.trim().length > 0)
    .join("");

  if (!cards) {
    return '<div class="filtering-config-empty">No settings match your search</div>';
  }

  return `<div class="cards-grid">${cards}</div>`;
}

function renderSourceToggle(source) {
  if (!state.draft) return "";

  const enabled = getSourceEnabled(state.draft, source);
  const sourceLabelMap = {
    dexscreener: "DexScreener Enabled",
    geckoterminal: "GeckoTerminal Enabled",
    rugcheck: "RugCheck Enabled",
  };
  const sourceLabel = sourceLabelMap[source] || "Source Enabled";
  const toggleId = `source-toggle-${source}`;

  return `
    <div class="source-toggle-wrapper">
      <span class="label">${sourceLabel}</span>
      <label class="source-switch">
        <input
          type="checkbox"
          id="${toggleId}"
          data-source-toggle="${source}"
          ${enabled ? "checked" : ""}
        />
        <span class="slider"></span>
      </label>
    </div>
  `;
}

function renderSearchBar() {
  const showSourceToggle =
    state.activeTab === "dexscreener" ||
    state.activeTab === "geckoterminal" ||
    state.activeTab === "rugcheck";
  const sourceToggle = showSourceToggle ? renderSourceToggle(state.activeTab) : "";

  return `
      <input
        type="text"
        id="filtering-search"
        placeholder="Search settings..."
        value="${Utils.escapeHtml(state.searchQuery)}"
      />
      ${sourceToggle}
  `;
}

function getStatusMessage() {
  if (state.isSaving) return "Saving changes...";
  if (state.isRefreshing) return "Refreshing snapshot...";
  if (state.hasChanges) return "Unsaved changes pending";
  if (state.lastSaved) return `Last saved ${Utils.formatTimeAgo(state.lastSaved)}`;
  return "Configuration in sync";
}

function renderShell() {
  return `
    <div class="filtering-page">
      <div class="filtering-shell">
        <div class="filtering-info-bar" id="filtering-info-bar">${renderInfoBar()}</div>
        <div class="filtering-search-bar" id="filtering-search-bar">${renderSearchBar()}</div>
        <div class="filtering-content" id="filtering-config-panels">
          ${renderConfigPanels()}
        </div>
        <footer class="filtering-footer">
          <div class="footer-left">
            <span id="filtering-status-message">${Utils.escapeHtml(getStatusMessage())}</span>
          </div>
          <div class="footer-actions">
            <button class="ghost" id="reset-config-btn"><i class="icon-rotate-ccw"></i> Reset</button>
            <button class="ghost" id="refresh-snapshot-btn"><i class="icon-refresh-cw"></i> Refresh</button>
            <button class="ghost" id="export-config-btn"><i class="icon-download"></i> Export</button>
            <button class="ghost" id="import-config-btn"><i class="icon-upload"></i> Import</button>
            <button class="primary" id="save-config-btn"><i class="icon-save"></i> Save</button>
          </div>
        </footer>
      </div>
    </div>
  `;
}

let globalHandlersBound = false;

function bindGlobalHandlers() {
  if (globalHandlersBound) return;

  const saveBtn = $("#save-config-btn");
  const resetBtn = $("#reset-config-btn");
  const refreshBtn = $("#refresh-snapshot-btn");
  const exportBtn = $("#export-config-btn");
  const importBtn = $("#import-config-btn");

  if (saveBtn) addTrackedListener(saveBtn, "click", handleSaveConfig);
  if (resetBtn) addTrackedListener(resetBtn, "click", handleResetConfig);
  if (refreshBtn) addTrackedListener(refreshBtn, "click", handleRefreshSnapshot);
  if (exportBtn) addTrackedListener(exportBtn, "click", handleExportConfig);
  if (importBtn) addTrackedListener(importBtn, "click", handleImportConfig);

  globalHandlersBound = true;
}

function bindSourceToggleHandlers() {
  $$("[data-source-toggle]").forEach((input) => {
    addTrackedListener(input, "change", handleSourceToggle);
  });
}

function bindConfigHandlers() {
  const container = $("#filtering-config-panels");
  if (!container) return;

  container.querySelectorAll("[data-field]").forEach((input) => {
    addTrackedListener(input, "input", handleFieldChange);
    if (input.type === "checkbox") {
      addTrackedListener(input, "change", handleFieldChange);
    }
  });

  container.querySelectorAll("[data-category-toggle]").forEach((input) => {
    addTrackedListener(input, "change", handleCategoryToggle);
  });
}

function updateInfoBar() {
  const container = $("#filtering-info-bar");
  if (container) {
    container.innerHTML = renderInfoBar();
  }
}

function updateSearchBar() {
  const container = $("#filtering-search-bar");
  if (!container) return;

  container.innerHTML = renderSearchBar();
  bindSearchHandler();
  bindSourceToggleHandlers();
}

function bindSearchHandler() {
  const searchInput = $("#filtering-search");
  if (searchInput) {
    addTrackedListener(searchInput, "input", (event) => {
      state.searchQuery = (event.target.value || "").toLowerCase();
      AppState.save("filtering_searchQuery", state.searchQuery);
      updateConfigPanels({ scrollTop: 0 });
    });
  }
}

function updateConfigPanels(options = {}) {
  const container = $("#filtering-config-panels");
  if (!container) return;

  const previousScroll = container.scrollTop;
  container.innerHTML = renderConfigPanels();
  bindConfigHandlers();

  if (options.scrollTop === 0) {
    container.scrollTo({ top: 0, behavior: options.smooth ? "smooth" : "auto" });
  } else if (options.preserveScroll) {
    container.scrollTop = previousScroll;
  }
}

function updateStatusMessage() {
  const statusEl = $("#filtering-status-message");
  if (statusEl) {
    statusEl.textContent = getStatusMessage();
  }
}

function render() {
  const root = $("#filtering-root");
  if (!root) return;

  if (!state.initialized) {
    root.innerHTML = renderShell();
    state.initialized = true;
    bindGlobalHandlers();
  }

  updateInfoBar();
  updateSearchBar();
  updateConfigPanels({ preserveScroll: true });
  updateStatusMessage();
  updateActionButtons();
}
// ============================================================================
// EVENT HANDLERS
// ============================================================================

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
  updateStatusMessage();
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
  const refreshBtn = $("#refresh-snapshot-btn");

  if (saveBtn) {
    saveBtn.disabled = !state.hasChanges || state.isSaving;
  }
  if (resetBtn) {
    resetBtn.disabled = !state.hasChanges;
  }
  if (refreshBtn) {
    refreshBtn.disabled = state.isRefreshing;
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
  updateStatusMessage();

  try {
    await saveConfig(state.draft);
    state.config = JSON.parse(JSON.stringify(state.draft)); // Deep clone
    state.hasChanges = false;
    state.lastSaved = new Date();

    // Trigger filtering refresh after config save so changes take effect immediately
    try {
      await refreshSnapshot();
      // Reload stats to show updated results
      setTimeout(() => loadStats(), 500);
    } catch (refreshError) {
      console.warn("Auto-refresh after save failed:", refreshError);
    }

    Utils.showToast({
      type: "success",
      title: "Configuration Saved",
      message: "Filtering settings saved and snapshot refreshed",
    });
  } catch (error) {
    console.error("Failed to save config:", error);
    Utils.showToast({
      type: "error",
      title: "Save Failed",
      message: error.message || "Failed to save filtering configuration",
    });
  } finally {
    state.isSaving = false;
    updateActionButtons();
    updateStatusMessage();
  }
}

function handleResetConfig() {
  if (!state.hasChanges) return;

  state.draft = JSON.parse(JSON.stringify(state.config)); // Deep clone
  state.hasChanges = false;
  render(); // Need full render to restore original values
  Utils.showToast({
    type: "info",
    title: "Changes Reset",
    message: "Configuration restored to last saved state",
  });
}

async function handleRefreshSnapshot() {
  if (state.isRefreshing) return;

  state.isRefreshing = true;
  updateActionButtons();
  updateStatusMessage();

  try {
    await refreshSnapshot();
    // Removed success toast - silent refresh, only show errors
    // Reload stats after refresh
    setTimeout(() => loadStats(), 1000);
  } catch (error) {
    console.error("Failed to refresh snapshot:", error);
    Utils.showToast({
      type: "error",
      title: "Refresh Failed",
      message: error.message || "Failed to refresh filtering snapshot",
    });
  } finally {
    state.isRefreshing = false;
    updateActionButtons();
    updateStatusMessage();
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
  Utils.showToast({
    type: "success",
    title: "Configuration Exported",
    message: "Filtering settings saved to file",
  });
}

function handleImportConfig() {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".json";
  input.addEventListener("change", async (e) => {
    const file = e.target.files?.[0];
    if (!file) return;

    try {
      const text = await file.text();
      const imported = JSON.parse(text);
      const current = JSON.parse(JSON.stringify(state.draft ?? {}));
      state.draft = deepMerge(current, imported);
      checkForChanges();
      render();
      Utils.showToast({
        type: "success",
        title: "Configuration Imported",
        message: "Filtering settings loaded from file",
      });
    } catch (error) {
      console.error("Failed to import config:", error);
      Utils.showToast({
        type: "error",
        title: "Import Failed",
        message: error.message || "Failed to import configuration - invalid file format",
      });
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
    Utils.showToast({
      type: "error",
      title: "Load Failed",
      message: error.message || "Failed to load filtering configuration",
    });
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

      // Initialize tab bar with global container
      if (!tabBar) {
        tabBar = new TabBar({
          container: "#subTabsContainer",
          tabs: FILTER_TABS,
          defaultTab: state.activeTab,
          stateKey: TABBAR_STATE_KEY,
          pageName: "filtering",
          onChange: (tabId) => {
            state.activeTab = tabId;
            AppState.save("filtering_activeTab", tabId);
            updateSearchBar();
            updateConfigPanels({ scrollTop: 0 });
          },
        });
        TabBarManager.register("filtering", tabBar);
        ctx.manageTabBar(tabBar);
        tabBar.show();

        // Sync state.activeTab with TabBar's restored state (from server or URL)
        const active = tabBar.getActiveTab();
        if (active && active !== state.activeTab) {
          state.activeTab = active;
          AppState.save("filtering_activeTab", active);
        }
      }

      if (!poller) {
        poller = ctx.managePoller(
          new Poller(
            async () => {
              await loadStats();
              updateInfoBar();
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

      // Clean up all tracked event listeners
      eventCleanups.forEach((cleanup) => cleanup());
      eventCleanups.length = 0;

      if (poller) {
        poller.stop();
        poller = null;
      }
      tabBar = null; // Cleaned up automatically by ctx.manageTabBar
      TabBarManager.unregister("filtering");
      state.initialized = false;
      globalHandlersBound = false;
    },
  };
}

// Register the page with the lifecycle system
registerPage("filtering", createLifecycle());
