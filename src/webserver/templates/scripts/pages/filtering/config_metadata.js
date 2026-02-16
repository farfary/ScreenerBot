/**
 * Filtering Configuration Metadata
 *
 * Contains all configuration categories, fields, helpers, and utilities
 * for the filtering system.
 */

// ============================================================================
// FILTER TABS
// ============================================================================

export const FILTER_TABS = [
  { id: "status", label: '<i class="icon-chart-bar"></i> Status' },
  { id: "analytics", label: '<i class="icon-chart-pie"></i> Analytics' },
  { id: "explorer", label: '<i class="icon-folder"></i> Explorer' },
  { id: "meta", label: '<i class="icon-settings"></i> Core' },
  { id: "dexscreener", label: '<i class="icon-trending-up"></i> DexScreener' },
  { id: "geckoterminal", label: '<i class="icon-trending-up"></i> GeckoTerminal' },
  { id: "rugcheck", label: '<i class="icon-shield"></i> RugCheck' },
];

// ============================================================================
// TIME RANGE PRESETS
// ============================================================================

// Time range presets (in seconds)
export const TIME_RANGE_PRESETS = {
  "1h": { label: "1H", seconds: 60 * 60 },
  "6h": { label: "6H", seconds: 6 * 60 * 60 },
  "24h": { label: "24H", seconds: 24 * 60 * 60 },
  "7d": { label: "7D", seconds: 7 * 24 * 60 * 60 },
  all: { label: "All", seconds: null },
};

// ============================================================================
// CONFIGURATION CATEGORIES
// ============================================================================

export const CONFIG_CATEGORIES = {
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
// HELPER FUNCTIONS
// ============================================================================

/**
 * Format timestamp for datetime-local input (local time, not UTC)
 */
export function formatTimestampForInput(timestamp) {
  if (!timestamp) return "";
  const date = new Date(timestamp * 1000);
  const pad = (n) => n.toString().padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

/**
 * Get time range label for display
 */
export function getTimeRangeLabel(timeRange) {
  const { preset, startTime, endTime } = timeRange;
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

/**
 * Get status message for display
 */
export function getStatusMessage({ isSaving, isRefreshing, hasChanges, lastSaved, Utils }) {
  if (isSaving) return "Saving changes...";
  if (isRefreshing) return "Refreshing snapshot...";
  if (hasChanges) return "Unsaved changes pending";
  if (lastSaved) return `Last saved ${Utils.formatTimeAgo(lastSaved)}`;
  return "Configuration in sync";
}

/**
 * Get value from nested config structure
 */
export function getConfigValue(config, source, key) {
  if (source === "meta") {
    return config[key];
  }
  return config[source]?.[key];
}

/**
 * Set value in nested config structure
 */
export function setConfigValue(config, source, key, value) {
  if (source === "meta") {
    config[key] = value;
  } else {
    if (!config[source]) {
      config[source] = {};
    }
    config[source][key] = value;
  }
}

/**
 * Get source enable status
 */
export function getSourceEnabled(config, source) {
  if (source === "meta") return true; // Meta is always enabled
  return config[source]?.enabled !== false;
}

/**
 * Set source enable status
 */
export function setSourceEnabled(config, source, enabled) {
  if (source === "meta") return; // Meta cannot be disabled
  if (!config[source]) {
    config[source] = {};
  }
  config[source].enabled = enabled;
}

/**
 * Get category enable status (for categories with enableKey)
 */
export function getCategoryEnabled(config, source, enableKey) {
  if (!enableKey) return true; // No enable key means always enabled
  if (source === "meta") {
    return config[enableKey] !== false;
  }
  return config[source]?.[enableKey] !== false;
}

/**
 * Set category enable status
 */
export function setCategoryEnabled(config, source, enableKey, enabled) {
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

/**
 * Deep merge helper so imports keep nested source-level settings intact
 */
export function deepMerge(target, source) {
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

/**
 * Compare configs for changes
 */
export function configsEqual(config1, config2) {
  const flat1 = JSON.stringify(config1);
  const flat2 = JSON.stringify(config2);
  return flat1 === flat2;
}
