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
  rejectionStats: null,
  analytics: null,
  hasChanges: false,
  isSaving: false,
  isRefreshing: false,
  isLoadingAnalytics: false, // Loading state for analytics section
  analyticsRequestId: 0, // Request ID to prevent race conditions
  lastSaved: null,
  searchQuery: AppState.load("filtering_searchQuery", ""),
  activeTab: AppState.load("filtering_activeTab", "status"),
  initialized: false,
  // Time range filter for analytics - persist all values
  timeRange: {
    preset: AppState.load("filtering_timeRangePreset", "all"),
    startTime: AppState.load("filtering_timeRangeStart", null),
    endTime: AppState.load("filtering_timeRangeEnd", null),
  },
};

const FILTER_TABS = [
  { id: "status", label: '<i class="icon-chart-bar"></i> Status' },
  { id: "analytics", label: '<i class="icon-chart-pie"></i> Analytics' },
  { id: "explorer", label: '<i class="icon-folder"></i> Explorer' },
  { id: "meta", label: '<i class="icon-settings"></i> Core' },
  { id: "dexscreener", label: '<i class="icon-trending-up"></i> DexScreener' },
  { id: "geckoterminal", label: '<i class="icon-trending-up"></i> GeckoTerminal' },
  { id: "rugcheck", label: '<i class="icon-shield"></i> RugCheck' },
];

const TABBAR_STATE_KEY = "filtering.tab";

// Time range presets (in seconds)
const TIME_RANGE_PRESETS = {
  "1h": { label: "1H", seconds: 60 * 60 },
  "6h": { label: "6H", seconds: 6 * 60 * 60 },
  "24h": { label: "24H", seconds: 24 * 60 * 60 },
  "7d": { label: "7D", seconds: 7 * 24 * 60 * 60 },
  all: { label: "All", seconds: null },
};

let tabBar = null;
const eventCleanups = [];

// Helper to track event listeners
function addTrackedListener(element, event, handler) {
  if (!element) return;
  element.addEventListener(event, handler);
  eventCleanups.push(() => element.removeEventListener(event, handler));
}

// Time range filter functions
// Helper to format timestamp for datetime-local input (local time, not UTC)
function formatTimestampForInput(timestamp) {
  if (!timestamp) return "";
  const date = new Date(timestamp * 1000);
  const pad = (n) => n.toString().padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

async function setTimeRangePreset(preset) {
  const now = Math.floor(Date.now() / 1000);
  state.timeRange.preset = preset;

  if (preset === "all" || preset === "custom") {
    state.timeRange.startTime = null;
    state.timeRange.endTime = null;
  } else if (TIME_RANGE_PRESETS[preset]) {
    state.timeRange.startTime = now - TIME_RANGE_PRESETS[preset].seconds;
    state.timeRange.endTime = now;
  }

  // Persist all time range state
  AppState.save("filtering_timeRangePreset", preset);
  AppState.save("filtering_timeRangeStart", state.timeRange.startTime);
  AppState.save("filtering_timeRangeEnd", state.timeRange.endTime);

  // Show loading state immediately
  state.isLoadingAnalytics = true;
  render();

  try {
    await loadAnalytics();
  } finally {
    // Always clear loading state, even on error
    state.isLoadingAnalytics = false;
    render();
  }
}

function getTimeRangeLabel() {
  const { preset, startTime, endTime } = state.timeRange;
  if (preset === "all" || (!startTime && !endTime)) {
    return "All Time";
  }
  if (preset === "custom") {
    const start = startTime ? new Date(startTime * 1000).toLocaleString() : "∞";
    const end = endTime ? new Date(endTime * 1000).toLocaleString() : "Now";
    return `${start} → ${end}`;
  }
  return TIME_RANGE_PRESETS[preset]?.label || "Custom";
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

async function fetchRejectionStats() {
  return await requestManager.fetch("/api/filtering/rejection-stats", {
    priority: "normal",
  });
}

async function fetchAnalytics() {
  // Build URL with time range parameters if set
  let url = "/api/filtering/analytics";
  const params = new URLSearchParams();

  if (state.timeRange.startTime) {
    params.set("start_time", state.timeRange.startTime.toString());
  }
  if (state.timeRange.endTime) {
    params.set("end_time", state.timeRange.endTime.toString());
  }
  if (state.timeRange.preset && state.timeRange.preset !== "all") {
    params.set("preset", state.timeRange.preset);
  }

  const queryString = params.toString();
  if (queryString) {
    url += "?" + queryString;
  }

  return await requestManager.fetch(url, {
    priority: "normal",
  });
}

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

  const priceRate = total_tokens > 0 ? (with_pool_price / total_tokens) * 100 : 0;
  const passedRate = total_tokens > 0 ? (passed_filtering / total_tokens) * 100 : 0;
  const cacheAge = updated_at ? Utils.formatTimeAgo(new Date(updated_at)) : "Never";

  return `
      <div class="info-item highlight">
        <span class="label">Total:</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(total_tokens, 0))}</span>
      </div>
      <div class="info-item">
        <span class="label">Priced:</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(with_pool_price, 0))} (${Utils.escapeHtml(Utils.formatPercentValue(priceRate, { includeSign: false, decimals: 1 }))})</span>
      </div>
      <div class="info-item highlight">
        <span class="label">Passed:</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(passed_filtering, 0))} (${Utils.escapeHtml(Utils.formatPercentValue(passedRate, { includeSign: false, decimals: 1 }))})</span>
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

  const priceRate = total_tokens > 0 ? (with_pool_price / total_tokens) * 100 : 0;
  const passedRate = total_tokens > 0 ? (passed_filtering / total_tokens) * 100 : 0;

  // Render rejection breakdown if available
  let rejectionBreakdown = "";
  if (state.rejectionStats && state.rejectionStats.stats && state.rejectionStats.stats.length > 0) {
    const topReasons = state.rejectionStats.stats.slice(0, 15); // Show top 15 reasons
    const bySource = state.rejectionStats.by_source || {};

    rejectionBreakdown = `
      <div class="rejection-breakdown">
        <h4>Rejection Breakdown</h4>
        <div class="rejection-sources">
          ${Object.entries(bySource)
            .sort((a, b) => b[1] - a[1])
            .map(
              ([source, count]) => `
            <div class="source-badge ${source}">
              <span class="source-name">${Utils.escapeHtml(source)}</span>
              <span class="source-count">${Utils.formatNumber(count, 0)}</span>
            </div>
          `
            )
            .join("")}
        </div>
        <div class="rejection-list">
          ${topReasons
            .map(
              ({ reason, display_label, source, count }) => `
            <div class="rejection-item">
              <span class="rejection-reason">${Utils.escapeHtml(display_label || reason)}</span>
              <span class="rejection-source ${source}">${Utils.escapeHtml(source)}</span>
              <span class="rejection-count">${Utils.formatNumber(count, 0)}</span>
            </div>
          `
            )
            .join("")}
          ${
            state.rejectionStats.stats.length > 15
              ? `<div class="rejection-more">+ ${state.rejectionStats.stats.length - 15} more reasons</div>`
              : ""
          }
        </div>
      </div>
    `;
  }

  return `
    <div class="filtering-status-layout">
      <div class="status-metrics-section">
        <h4 class="section-title">Overview</h4>
        <div class="status-view">
          <div class="status-card dominant">
            <span class="metric-label">Total Tokens</span>
            <span class="metric-value">${Utils.escapeHtml(Utils.formatNumber(total_tokens, 0))}</span>
            <span class="metric-meta">In filtering cache</span>
          </div>
          <div class="status-card">
            <span class="metric-label">With Price</span>
            <span class="metric-value">${Utils.escapeHtml(Utils.formatNumber(with_pool_price, 0))}</span>
            <span class="metric-meta">${Utils.escapeHtml(`${Utils.formatPercentValue(priceRate, { includeSign: false, decimals: 1 })} have pricing`)}</span>
          </div>
          <div class="status-card dominant">
            <span class="metric-label">Passed Filters</span>
            <span class="metric-value">${Utils.escapeHtml(Utils.formatNumber(passed_filtering, 0))}</span>
            <span class="metric-meta">${Utils.escapeHtml(`${Utils.formatPercentValue(passedRate, { includeSign: false, decimals: 1 })} passed`)}</span>
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
      </div>
      <div class="status-rejection-section">
        ${rejectionBreakdown}
      </div>
    </div>
  `;
}

// ============================================================================
// ANALYTICS VIEW - Advanced filtering analysis
// ============================================================================

function renderAnalyticsView() {
  // Show loading state when switching time ranges or initially loading
  if (state.isLoadingAnalytics || !state.analytics) {
    return `
      <div class="analytics-loading">
        <div class="loading-spinner"></div>
        <p>Loading analytics for ${getTimeRangeLabel()}...</p>
      </div>
    `;
  }

  const data = state.analytics;

  // Time range info text
  const timeRangeText =
    state.timeRange.preset === "all"
      ? "Showing all-time rejection statistics"
      : state.timeRange.preset === "custom"
        ? "Showing rejections from custom range"
        : `Showing rejections from last ${TIME_RANGE_PRESETS[state.timeRange.preset]?.label || "period"}`;

  // Header with time range filter (no refresh button - use footer refresh)
  const headerHtml = `
    <div class="analytics-header">
      <div class="analytics-title-group">
        <div class="analytics-title">
          <i class="icon-chart-pie"></i> Analytics
        </div>
        <div class="analytics-subtitle">
          ${Utils.escapeHtml(timeRangeText)}
        </div>
      </div>
    </div>
    
    <!-- Time Range Filter -->
    <div class="time-range-filter">
      <div class="time-range-presets">
        <button class="time-preset-btn ${state.timeRange.preset === "1h" ? "active" : ""}" onclick="window.filteringPage.setTimeRangePreset('1h')">1H</button>
        <button class="time-preset-btn ${state.timeRange.preset === "6h" ? "active" : ""}" onclick="window.filteringPage.setTimeRangePreset('6h')">6H</button>
        <button class="time-preset-btn ${state.timeRange.preset === "24h" ? "active" : ""}" onclick="window.filteringPage.setTimeRangePreset('24h')">24H</button>
        <button class="time-preset-btn ${state.timeRange.preset === "7d" ? "active" : ""}" onclick="window.filteringPage.setTimeRangePreset('7d')">7D</button>
        <button class="time-preset-btn ${state.timeRange.preset === "all" ? "active" : ""}" onclick="window.filteringPage.setTimeRangePreset('all')">All</button>
      </div>
      <div class="time-range-custom">
        <div class="custom-range-toggle ${state.timeRange.preset === "custom" ? "active" : ""}" onclick="window.filteringPage.toggleCustomRange()">
          <i class="icon-calendar"></i> Custom
        </div>
        <div class="custom-range-inputs ${state.timeRange.preset === "custom" ? "show" : ""}">
          <input type="datetime-local" id="time-range-start" class="time-input" 
            value="${formatTimestampForInput(state.timeRange.startTime)}"
            onchange="window.filteringPage.updateCustomRange()">
          <span class="time-separator">→</span>
          <input type="datetime-local" id="time-range-end" class="time-input" 
            value="${formatTimestampForInput(state.timeRange.endTime)}"
            onchange="window.filteringPage.updateCustomRange()">
          <button class="btn btn-sm btn-primary" onclick="window.filteringPage.applyCustomRange()">Apply</button>
        </div>
      </div>
    </div>
  `;

  // KPI Cards
  const kpiHtml = `
    <div class="kpi-grid">
      <!-- Total Tokens -->
      <div class="kpi-card">
        <div class="kpi-content">
          <span class="kpi-label">Total Scanned</span>
          <span class="kpi-value">${Utils.formatNumber(data.total_tokens, 0)}</span>
          <span class="kpi-subtext">
            <i class="icon-clock"></i> Updated ${Utils.formatTimeAgo(new Date(data.last_updated))}
          </span>
        </div>
        <i class="icon-database kpi-icon"></i>
      </div>

      <!-- Passed -->
      <div class="kpi-card">
        <div class="kpi-content">
          <span class="kpi-label">Passed Tokens</span>
          <span class="kpi-value text-success">${Utils.formatNumber(data.total_passed, 0)}</span>
          <span class="kpi-subtext">
            <span class="text-success">${Utils.formatPercentValue(data.pass_rate, { includeSign: false })}</span> pass rate
          </span>
        </div>
        <i class="icon-circle-check kpi-icon text-success" style="opacity: 0.2"></i>
        <div class="pass-rate-visual">
          <div class="pass-rate-segment passed" style="width: ${data.pass_rate}%"></div>
        </div>
      </div>

      <!-- Rejected -->
      <div class="kpi-card">
        <div class="kpi-content">
          <span class="kpi-label">Rejected Tokens</span>
          <span class="kpi-value text-error">${Utils.formatNumber(data.total_rejected, 0)}</span>
          <span class="kpi-subtext">
            <span class="text-error">${Utils.formatPercentValue(data.rejection_rate, { includeSign: false })}</span> rejection rate
          </span>
        </div>
        <i class="icon-circle-x kpi-icon text-error" style="opacity: 0.2"></i>
      </div>
    </div>
  `;

  // Charts Section
  const chartsHtml = `
    <div class="charts-grid">
      <!-- Rejection by Category -->
      <div class="chart-card">
        <div class="chart-header">
          <div class="chart-title"><i class="icon-layers"></i> Rejection by Category</div>
        </div>
        <div class="chart-body">
          ${
            data.by_category && data.by_category.length > 0
              ? data.by_category
                  .map(
                    (cat) => `
            <div class="bar-chart-row">
              <div class="bar-label-col">
                <div class="bar-icon"><i class="icon-${Utils.escapeHtml(cat.icon)}"></i></div>
                <div class="bar-label" title="${Utils.escapeHtml(cat.label)}">${Utils.escapeHtml(cat.label)}</div>
              </div>
              <div class="bar-track-col">
                <div class="bar-track">
                  <div class="bar-fill" style="width: ${Math.min(cat.percentage, 100)}%; background-color: var(--error-color)"></div>
                </div>
                <div class="bar-meta">
                  <span>${Utils.formatNumber(cat.count, 0)} tokens</span>
                  <span>${Utils.formatPercentValue(cat.percentage, { includeSign: false })}</span>
                </div>
              </div>
            </div>
          `
                  )
                  .join("")
              : '<div class="analytics-empty">No category data</div>'
          }
        </div>
      </div>

      <!-- Rejection by Source -->
      <div class="chart-card">
        <div class="chart-header">
          <div class="chart-title"><i class="icon-git-branch"></i> Rejection by Source</div>
        </div>
        <div class="chart-body">
          ${
            data.by_source && data.by_source.length > 0
              ? data.by_source
                  .map(
                    (src) => `
            <div class="bar-chart-row">
              <div class="bar-label-col">
                <div class="bar-label font-bold w-auto">${Utils.escapeHtml(src.source)}</div>
              </div>
              <div class="bar-track-col">
                <div class="bar-track">
                  <div class="bar-fill" style="width: ${Math.min(src.percentage, 100)}%; background-color: var(--warning-color)"></div>
                </div>
                <div class="bar-meta">
                  <span>${Utils.formatNumber(src.count, 0)} tokens</span>
                  <span>${Utils.formatPercentValue(src.percentage, { includeSign: false })}</span>
                </div>
              </div>
            </div>
          `
                  )
                  .join("")
              : '<div class="analytics-empty">No source data</div>'
          }
        </div>
      </div>
    </div>
  `;

  // Bottom Section: Top Reasons & Data Quality
  const bottomHtml = `
    <div class="charts-grid">
      <!-- Top Rejection Reasons -->
      <div class="chart-card span-2">
        <div class="chart-header">
          <div class="chart-title"><i class="icon-list"></i> Top Rejection Reasons</div>
        </div>
        <div class="reasons-table-container">
          <table class="reasons-table">
            <thead>
              <tr>
                <th>Reason</th>
                <th>Category</th>
                <th class="text-end">Count</th>
                <th class="text-end">%</th>
                <th class="text-end">Impact</th>
              </tr>
            </thead>
            <tbody>
              ${
                data.top_reasons && data.top_reasons.length > 0
                  ? data.top_reasons
                      .slice(0, 10)
                      .map((r) => {
                        const maxCount = data.top_reasons[0].count;
                        const relativePercent = (r.count / maxCount) * 100;
                        return `
                  <tr>
                    <td>
                      <span class="font-medium">${Utils.escapeHtml(r.display_label)}</span>
                    </td>
                    <td>
                      <span class="reason-badge">${Utils.escapeHtml(r.category)}</span>
                    </td>
                    <td class="text-end font-data">
                      ${Utils.formatNumber(r.count, 0)}
                    </td>
                    <td class="text-end font-data text-secondary">
                      ${Utils.formatPercentValue(r.percentage, { includeSign: false })}
                    </td>
                    <td class="reason-bar-cell">
                      <div class="mini-bar">
                        <div class="mini-bar-fill" style="width: ${relativePercent}%"></div>
                      </div>
                    </td>
                  </tr>
                `;
                      })
                      .join("")
                  : '<tr><td colspan="5" class="text-center p-20">No data available</td></tr>'
              }
            </tbody>
          </table>
        </div>
      </div>
    </div>
  `;

  // Trigger initial load of explorer data if not already loaded
  return `
    <div class="analytics-view">
      ${headerHtml}
      ${kpiHtml}
      ${chartsHtml}
      ${bottomHtml}
    </div>
  `;
}

// ============================================================================
// EXPLORER VIEW - Tree-based rejection explorer
// ============================================================================

function renderExplorerDashboard(data) {
  const topReasons = data.top_reasons || [];
  const recentRejections = data.recent_rejections || [];
  const totalRejected = data.total_rejected || 0;
  const totalPassed = data.total_passed || 0;

  return `
    <div class="explorer-empty-dashboard">
      <div class="dashboard-welcome">
        <h2>Rejection Explorer</h2>
        <p>Rejected: ${Utils.formatNumber(totalRejected, 0)} · Passed: ${Utils.formatNumber(totalPassed, 0)}</p>
      </div>

      <div class="dashboard-grid">
        <div class="dashboard-card">
          <h3><i class="icon-trending-down"></i> Top Reasons</h3>
          <div class="top-reasons-list">
            ${topReasons
              .slice(0, 10)
              .map(
                (r) => `
              <div class="top-reason-item" onclick="window.filteringPage.selectReason('${r.reason}', '${Utils.escapeHtml(r.display_label.replace(/'/g, "\\'"))}')">
                <span class="top-reason-label">${Utils.escapeHtml(r.display_label)}</span>
                <span class="top-reason-count">${Utils.formatNumber(r.count, 0)}</span>
              </div>
            `
              )
              .join("")}
            ${topReasons.length === 0 ? '<div class="analytics-empty-compact">No data</div>' : ""}
          </div>
        </div>

        <div class="dashboard-card">
          <h3><i class="icon-clock"></i> Recent</h3>
          <div class="top-reasons-list">
            ${recentRejections
              .slice(0, 10)
              .map(
                (t) => `
              <div class="top-reason-item" onclick="window.filteringPage.selectReason('${t.reason}', '${Utils.escapeHtml(t.display_label.replace(/'/g, "\\'"))}')">
                <div class="flex-col min-w-0">
                  <span class="top-reason-label truncate">${Utils.escapeHtml(t.symbol || "Unknown")}</span>
                  <span class="text-xs truncate">${Utils.escapeHtml(t.display_label)}</span>
                </div>
                <span class="top-reason-count">${Utils.formatTimeAgo(new Date(t.rejected_at))}</span>
              </div>
            `
              )
              .join("")}
            ${recentRejections.length === 0 ? '<div class="analytics-empty-compact">No recent</div>' : ""}
          </div>
        </div>
      </div>
    </div>
  `;
}

function renderExplorerView() {
  if (!state.analytics) {
    return `
      <div class="analytics-loading">
        <div class="loading-spinner"></div>
        <p>Loading...</p>
      </div>
    `;
  }

  const data = state.analytics;

  // Compact Tree View
  const treeHtml = `
    <div class="explorer-layout">
      <div class="explorer-sidebar">
        <div class="explorer-sidebar-search">
          <div class="explorer-search-input-wrapper">
            <i class="icon-search"></i>
            <input type="text" placeholder="Search reasons..." oninput="window.filteringPage.filterExplorerTree(this.value)">
          </div>
        </div>

        <div class="explorer-summary-item ${!window.filteringPage.currentReason ? "active" : ""}" onclick="window.filteringPage.selectSummary()">
          <i class="icon-layout"></i>
          <div class="explorer-summary-info">
            <span class="explorer-summary-label">Overview</span>
            <span class="explorer-summary-desc">Summary & recent</span>
          </div>
        </div>

        <div class="explorer-tree">
          ${data.by_category
            .map(
              (cat) => `
            <div class="tree-category" data-category="${cat.category}">
              <div class="tree-category-header" onclick="window.filteringPage.toggleCategory('${cat.category}')">
                <i class="icon-${Utils.escapeHtml(cat.icon)} tree-icon"></i>
                <span class="tree-label">${Utils.escapeHtml(cat.label)}</span>
                <span class="tree-count">${Utils.formatCompactNumber(cat.count)}</span>
                <i class="icon-chevron-down tree-toggle" id="toggle-${cat.category}"></i>
              </div>
              <div class="tree-reasons" id="reasons-${cat.category}" style="display: none">
                ${cat.reasons
                  .map(
                    (r) => `
                  <div class="tree-reason ${window.filteringPage.currentReason === r.reason ? "active" : ""}" 
                       onclick="window.filteringPage.selectReason('${r.reason}', '${Utils.escapeHtml(r.display_label.replace(/'/g, "\\'"))}')" 
                       id="reason-${r.reason}"
                       data-label="${Utils.escapeHtml(r.display_label.toLowerCase())}">
                    <span class="tree-reason-label">${Utils.escapeHtml(r.display_label)}</span>
                    <span class="tree-reason-count">${Utils.formatCompactNumber(r.count)}</span>
                  </div>
                `
                  )
                  .join("")}
              </div>
            </div>
          `
            )
            .join("")}
        </div>
      </div>
      <div class="explorer-content">
        <div id="explorer-detail-view">
          ${window.filteringPage.currentReason ? "" : renderExplorerDashboard(data)}
        </div>
      </div>
    </div>
  `;

  // Trigger initial load if reason is selected
  if (window.filteringPage.currentReason) {
    setTimeout(() => window.filteringPage.loadExplorer(window.filteringPage.explorerPage), 0);
  }

  return `
    <div class="explorer-view-container">
      ${treeHtml}
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

  // Analytics tab shows advanced analysis
  if (state.activeTab === "analytics") {
    return renderAnalyticsView();
  }

  // Explorer tab shows tree view
  if (state.activeTab === "explorer") {
    return renderExplorerView();
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
  // Only show search bar on settings tabs (not status/analytics/explorer)
  const isSettingsTab = ["meta", "dexscreener", "geckoterminal", "rugcheck"].includes(
    state.activeTab
  );
  if (!isSettingsTab) {
    return "";
  }

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

  const html = renderSearchBar();
  container.innerHTML = html;

  // Hide the container if no content
  container.style.display = html ? "" : "none";

  if (html) {
    bindSearchHandler();
    bindSourceToggleHandlers();
  }
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
    const [statsResponse, rejectionStatsResponse] = await Promise.all([
      fetchStats(),
      fetchRejectionStats(),
    ]);
    state.stats = statsResponse.data || statsResponse;
    state.rejectionStats = rejectionStatsResponse.data || rejectionStatsResponse;

    // Also load analytics if on analytics tab
    if (state.activeTab === "analytics") {
      await loadAnalytics();
    }
  } catch (error) {
    console.error("Failed to load stats:", error);
    // Don't show toast for stats errors (non-critical)
  }
}

async function loadAnalytics() {
  // Increment request ID to track this request and prevent race conditions
  const thisRequestId = ++state.analyticsRequestId;

  try {
    const response = await fetchAnalytics();

    // Check if this is still the latest request (prevent stale data from overwriting)
    if (thisRequestId !== state.analyticsRequestId) {
      return; // A newer request was made, discard this response
    }

    state.analytics = response.data || response;
    // Re-render to show analytics data
    updateConfigPanels({ preserveScroll: false });
  } catch (error) {
    // Only log error if this is still the active request
    if (thisRequestId === state.analyticsRequestId) {
      console.error("Failed to load analytics:", error);
      state.analytics = null;
    }
  }
}

async function loadData() {
  await Promise.all([loadConfig(), loadStats()]);

  // If active tab is analytics or explorer, load analytics data
  if (state.activeTab === "analytics" || state.activeTab === "explorer") {
    await loadAnalytics();
  }

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

      // Validate time range state consistency - if preset is "custom" but times are null, reset to "all"
      if (
        state.timeRange.preset === "custom" &&
        (!state.timeRange.startTime || !state.timeRange.endTime)
      ) {
        console.warn("[Filtering] Inconsistent time range state detected, resetting to 'all'");
        state.timeRange.preset = "all";
        state.timeRange.startTime = null;
        state.timeRange.endTime = null;
        AppState.save("filtering_timeRangePreset", "all");
        AppState.save("filtering_timeRangeStart", null);
        AppState.save("filtering_timeRangeEnd", null);
      }

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
          onChange: async (tabId) => {
            state.activeTab = tabId;
            AppState.save("filtering_activeTab", tabId);
            updateSearchBar();

            // Load analytics data if switching to analytics or explorer tab
            if ((tabId === "analytics" || tabId === "explorer") && !state.analytics) {
              await loadAnalytics();
            }

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
      } else {
        // Re-register deactivate cleanup (cleanups are cleared after each deactivate)
        // and force-show tab bar to handle race conditions with TabBarManager
        ctx.manageTabBar(tabBar);
        // Ensure registry is up to date in case of page reload/HMR
        TabBarManager.register("filtering", tabBar);

        // Force show in next frame to ensure DOM is ready and override any race conditions
        requestAnimationFrame(() => {
          if (tabBar) tabBar.show({ force: true });
        });
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

// Expose for inline handlers
window.filteringPage = {
  explorerPage: 0,
  explorerLimit: 50,
  currentReason: null,
  currentReasonLabel: null,
  tokenSearchQuery: "",

  debouncedFilterTokens: null,

  // Time range filter functions
  setTimeRangePreset: (preset) => {
    setTimeRangePreset(preset);
  },

  toggleCustomRange: () => {
    if (state.timeRange.preset === "custom") {
      // Already in custom mode, toggle off to all
      setTimeRangePreset("all");
    } else {
      // Switch to custom mode
      state.timeRange.preset = "custom";
      AppState.save("filtering_timeRangePreset", "custom");
      render();
    }
  },

  updateCustomRange: () => {
    // Just update state without triggering refresh yet
    const startInput = document.getElementById("time-range-start");
    const endInput = document.getElementById("time-range-end");
    if (startInput && startInput.value) {
      state.timeRange.startTime = Math.floor(new Date(startInput.value).getTime() / 1000);
    }
    if (endInput && endInput.value) {
      state.timeRange.endTime = Math.floor(new Date(endInput.value).getTime() / 1000);
    }
  },

  applyCustomRange: async () => {
    const startInput = document.getElementById("time-range-start");
    const endInput = document.getElementById("time-range-end");

    // Parse dates
    let startTime = null;
    let endTime = null;

    if (startInput && startInput.value) {
      startTime = Math.floor(new Date(startInput.value).getTime() / 1000);
    }
    if (endInput && endInput.value) {
      endTime = Math.floor(new Date(endInput.value).getTime() / 1000);
    }

    // Validation: ensure both dates are provided
    if (!startTime || !endTime) {
      Utils.showToast("Please select both start and end dates", "error");
      return;
    }

    // Validation: start must be before end
    if (startTime >= endTime) {
      Utils.showToast("Start time must be before end time", "error");
      return;
    }

    // Validation: end cannot be in the future (with 1 minute tolerance)
    const now = Math.floor(Date.now() / 1000);
    if (endTime > now + 60) {
      Utils.showToast("End time cannot be in the future", "error");
      return;
    }

    state.timeRange.startTime = startTime;
    state.timeRange.endTime = endTime;
    state.timeRange.preset = "custom";

    // Persist all state
    AppState.save("filtering_timeRangePreset", "custom");
    AppState.save("filtering_timeRangeStart", startTime);
    AppState.save("filtering_timeRangeEnd", endTime);

    // Show loading state
    state.isLoadingAnalytics = true;
    render();

    try {
      await loadAnalytics();
    } finally {
      // Always clear loading state, even on error
      state.isLoadingAnalytics = false;
      render();
    }
  },

  refreshAnalytics: async () => {
    // Simply load analytics and re-render - no inline button spinner needed
    // The footer refresh button and isLoadingAnalytics state handle UX
    state.isLoadingAnalytics = true;
    render();

    try {
      await loadAnalytics();
      if (window.filteringPage.currentReason) {
        window.filteringPage.loadExplorer(window.filteringPage.explorerPage);
      } else {
        const container = document.getElementById("explorer-detail-view");
        if (container && state.analytics) {
          container.innerHTML = renderExplorerDashboard(state.analytics);
        }
      }
    } finally {
      state.isLoadingAnalytics = false;
      render();
    }
  },

  debouncedFilterExplorerTree: null,

  filterExplorerTree: (query) => {
    if (!window.filteringPage.debouncedFilterExplorerTree) {
      window.filteringPage.debouncedFilterExplorerTree = Utils.debounce((q) => {
        const lowerQ = q.toLowerCase();
        document.querySelectorAll(".tree-category").forEach((cat) => {
          let hasVisibleReason = false;
          const reasons = cat.querySelectorAll(".tree-reason");
          reasons.forEach((r) => {
            const label = r.getAttribute("data-label") || "";
            const visible = label.includes(lowerQ);
            r.style.display = visible ? "flex" : "none";
            if (visible) hasVisibleReason = true;
          });

          cat.style.display = hasVisibleReason || lowerQ === "" ? "block" : "none";

          // Auto-expand if searching
          const reasonsList = cat.querySelector(".tree-reasons");
          const toggle = cat.querySelector(".tree-toggle");
          if (lowerQ !== "" && hasVisibleReason) {
            if (reasonsList) reasonsList.style.display = "block";
            if (toggle) toggle.style.transform = "rotate(180deg)";
          }
        });
      }, 150);
    }
    window.filteringPage.debouncedFilterExplorerTree(query);
  },

  selectSummary: () => {
    window.filteringPage.currentReason = null;
    window.filteringPage.currentReasonLabel = null;

    document.querySelectorAll(".tree-reason").forEach((el) => el.classList.remove("active"));
    const summaryItem = document.querySelector(".explorer-summary-item");
    if (summaryItem) summaryItem.classList.add("active");

    const container = document.getElementById("explorer-detail-view");
    if (container && state.analytics) {
      container.innerHTML = renderExplorerDashboard(state.analytics);
    }
  },

  toggleCategory: (category) => {
    const el = document.getElementById(`reasons-${category}`);
    const toggle = document.getElementById(`toggle-${category}`);
    if (el) {
      const isHidden = el.style.display === "none";
      el.style.display = isHidden ? "block" : "none";
      if (toggle) {
        toggle.style.transform = isHidden ? "rotate(180deg)" : "rotate(0deg)";
      }
    }
  },

  selectReason: (reason, label) => {
    // Update active state
    document.querySelectorAll(".tree-reason").forEach((el) => el.classList.remove("active"));
    const summaryItem = document.querySelector(".explorer-summary-item");
    if (summaryItem) summaryItem.classList.remove("active");

    const activeEl = document.getElementById(`reason-${reason}`);
    if (activeEl) {
      activeEl.classList.add("active");
      // Ensure category is expanded
      const category = activeEl.closest(".tree-reasons");
      if (category && category.style.display === "none") {
        category.style.display = "block";
        const catId = category.id.replace("reasons-", "");
        const toggle = document.getElementById(`toggle-${catId}`);
        if (toggle) toggle.style.transform = "rotate(180deg)";
      }
    }

    window.filteringPage.currentReason = reason;
    window.filteringPage.currentReasonLabel = label;
    window.filteringPage.tokenSearchQuery = "";
    window.filteringPage.loadExplorer(0);
  },

  filterTokens: (query) => {
    if (!window.filteringPage.debouncedFilterTokens) {
      window.filteringPage.debouncedFilterTokens = Utils.debounce((q) => {
        window.filteringPage.tokenSearchQuery = q;
        window.filteringPage.loadExplorer(0);
      }, 300);
    }
    window.filteringPage.debouncedFilterTokens(query);
  },

  firstPage: () => {
    if (window.filteringPage.explorerPage > 0) {
      window.filteringPage.loadExplorer(0);
    }
  },

  prevPage: () => {
    if (window.filteringPage.explorerPage > 0) {
      window.filteringPage.loadExplorer(window.filteringPage.explorerPage - 1);
    }
  },

  nextPage: () => {
    window.filteringPage.loadExplorer(window.filteringPage.explorerPage + 1);
  },

  lastPage: async () => {
    // Go forward in large steps until we hit the end
    const limit = window.filteringPage.explorerLimit;
    const reason = window.filteringPage.currentReason;
    if (!reason) return;

    // Estimate last page by fetching with large offset
    let testPage = window.filteringPage.explorerPage + 100;
    try {
      const url = `/api/filtering/rejected-tokens?limit=${limit}&offset=${testPage * limit}&reason=${encodeURIComponent(reason)}`;
      const response = await fetch(url);
      if (!response.ok) throw new Error("Failed to fetch tokens");
      const tokens = await response.json();

      if (tokens.length === 0) {
        // Binary search for actual last page
        let low = window.filteringPage.explorerPage;
        let high = testPage;
        while (low < high - 1) {
          const mid = Math.floor((low + high) / 2);
          const checkUrl = `/api/filtering/rejected-tokens?limit=${limit}&offset=${mid * limit}&reason=${encodeURIComponent(reason)}`;
          const checkRes = await fetch(checkUrl);
          if (!checkRes.ok) throw new Error("Failed to fetch tokens");
          const checkTokens = await checkRes.json();
          if (checkTokens.length === 0) {
            high = mid;
          } else if (checkTokens.length < limit) {
            // This is the last page
            window.filteringPage.loadExplorer(mid);
            return;
          } else {
            low = mid;
          }
        }
        window.filteringPage.loadExplorer(low);
      } else if (tokens.length < limit) {
        // testPage is the last page
        window.filteringPage.loadExplorer(testPage);
      } else {
        // Need to go further - just load testPage for now
        window.filteringPage.loadExplorer(testPage);
      }
    } catch {
      // Fallback: just go forward 10 pages
      window.filteringPage.loadExplorer(window.filteringPage.explorerPage + 10);
    }
  },

  exportCsv: () => {
    const reason = window.filteringPage.currentReason;
    if (!reason) return;

    let url = `/api/filtering/export-rejected-tokens?reason=${encodeURIComponent(reason)}`;
    window.open(url, "_blank");
  },

  loadExplorer: async (page) => {
    window.filteringPage.explorerPage = page;
    const container = document.getElementById("explorer-detail-view");
    const reason = window.filteringPage.currentReason;
    const label = window.filteringPage.currentReasonLabel;
    const searchQuery = window.filteringPage.tokenSearchQuery;

    if (!container || !reason) return;

    // Initial render with compact header and search
    if (page === 0 && !container.querySelector(".explorer-detail-header")) {
      container.innerHTML = `
        <div class="explorer-detail-header">
          <div class="detail-title-group">
            <span class="reason-badge large">${Utils.escapeHtml(label)}</span>
            <div class="explorer-search-input-wrapper width-180">
              <i class="icon-search"></i>
              <input type="text" placeholder="Filter..." value="${Utils.escapeHtml(searchQuery)}" 
                     oninput="window.filteringPage.filterTokens(this.value)">
            </div>
          </div>
          <div class="detail-actions">
            <button class="btn btn-sm btn-secondary" onclick="window.filteringPage.exportCsv()" title="Export CSV">
              <i class="icon-download"></i>
            </button>
          </div>
        </div>
        <div class="explorer-table-wrapper">
          <div class="explorer-empty-state">Loading...</div>
        </div>
        <div class="pagination-controls">
          <button class="page-btn" onclick="window.filteringPage.firstPage()" disabled title="First"><i class="icon-chevrons-left"></i></button>
          <button class="page-btn" onclick="window.filteringPage.prevPage()" disabled title="Previous"><i class="icon-chevron-left"></i></button>
          <span class="page-info">Page ${page + 1}</span>
          <button class="page-btn" onclick="window.filteringPage.nextPage()" disabled title="Next"><i class="icon-chevron-right"></i></button>
          <button class="page-btn" onclick="window.filteringPage.lastPage()" disabled title="Last"><i class="icon-chevrons-right"></i></button>
        </div>
      `;
    } else {
      const wrapper = container.querySelector(".explorer-table-wrapper");
      if (wrapper)
        wrapper.innerHTML = '<div class="loading-spinner small explorer-loading-full"></div>';
    }

    try {
      let url = `/api/filtering/rejected-tokens?limit=${window.filteringPage.explorerLimit}&offset=${page * window.filteringPage.explorerLimit}&reason=${encodeURIComponent(reason)}`;
      if (searchQuery) {
        url += `&search=${encodeURIComponent(searchQuery)}`;
      }

      const response = await fetch(url);
      if (!response.ok) throw new Error("Failed to fetch tokens");

      const tokens = await response.json();
      const wrapper = container.querySelector(".explorer-table-wrapper");

      if (!wrapper) return;

      if (tokens.length === 0) {
        wrapper.innerHTML = `
          <div class="explorer-empty-state">
            No tokens found${searchQuery ? " matching filter" : ""}
          </div>`;
        // Update pagination
        const pagination = container.querySelector(".pagination-controls");
        if (pagination) {
          pagination.innerHTML = `
            <button class="page-btn" disabled><i class="icon-chevrons-left"></i></button>
            <button class="page-btn" disabled><i class="icon-chevron-left"></i></button>
            <span class="page-info">No results</span>
            <button class="page-btn" disabled><i class="icon-chevron-right"></i></button>
            <button class="page-btn" disabled><i class="icon-chevrons-right"></i></button>
          `;
        }
        return;
      }

      let html = `
        <table class="reasons-table">
          <thead>
            <tr>
              <th>Token</th>
              <th>Source</th>
              <th class="text-end">Time</th>
            </tr>
          </thead>
          <tbody>
      `;

      html += tokens
        .map((t) => {
          const src = t.image_url;
          const logo = src
            ? `<img class="token-logo" alt="" src="${Utils.escapeHtml(src)}" loading="lazy" />`
            : '<div class="token-logo token-logo-placeholder">?</div>';
          const sym = Utils.escapeHtml(t.symbol || "—");
          const name = Utils.escapeHtml(t.name || "Unknown");

          return `
        <tr>
          <td>
            <div class="token-info-cell">
              <div class="token-logo-wrapper">
                ${logo}
              </div>
              <div class="token-details">
                <div class="token-symbol">${sym}</div>
                <div class="token-name">${name}</div>
              </div>
              <div class="token-actions">
                <button class="btn-icon small" onclick="Utils.copyToClipboard('${t.mint}')" title="Copy Mint">
                  <i class="icon-copy"></i>
                </button>
                <a href="https://dexscreener.com/solana/${t.mint}" target="_blank" class="btn-icon small" title="DexScreener">
                  <i class="icon-external-link"></i>
                </a>
              </div>
            </div>
          </td>
          <td>
            <span class="source-badge ${t.source.toLowerCase()}">${Utils.escapeHtml(t.source)}</span>
          </td>
          <td class="table-time-cell">
            ${Utils.formatTimeAgo(new Date(t.rejected_at))}
          </td>
        </tr>
      `;
        })
        .join("");

      html += "</tbody></table>";
      wrapper.innerHTML = html;

      // Update pagination
      const pagination = container.querySelector(".pagination-controls");
      if (pagination) {
        const hasMore = tokens.length >= window.filteringPage.explorerLimit;
        pagination.innerHTML = `
          <button class="page-btn" onclick="window.filteringPage.firstPage()" ${page === 0 ? "disabled" : ""} title="First"><i class="icon-chevrons-left"></i></button>
          <button class="page-btn" onclick="window.filteringPage.prevPage()" ${page === 0 ? "disabled" : ""} title="Previous"><i class="icon-chevron-left"></i></button>
          <span class="page-info">Page ${page + 1}</span>
          <button class="page-btn" onclick="window.filteringPage.nextPage()" ${!hasMore ? "disabled" : ""} title="Next"><i class="icon-chevron-right"></i></button>
          <button class="page-btn" onclick="window.filteringPage.lastPage()" ${!hasMore ? "disabled" : ""} title="Last"><i class="icon-chevrons-right"></i></button>
        `;
      }
    } catch (err) {
      console.error("Failed to load explorer:", err);
      const wrapper = container.querySelector(".explorer-table-wrapper");
      if (wrapper) {
        wrapper.innerHTML = `<div class="error-message p-lg">Failed to load tokens: ${err.message}</div>`;
      }
    }
  },
};

// Register the page with the lifecycle system
registerPage("filtering", createLifecycle());
