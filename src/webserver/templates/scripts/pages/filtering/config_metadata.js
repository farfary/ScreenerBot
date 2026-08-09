/**
 * Filtering page configuration helpers.
 *
 * Field presentation metadata is owned by the Rust config schema and delivered
 * by /api/config/metadata. This module owns only page navigation and the small
 * amount of grouping logic unique to the filtering workspace.
 */

export const FILTER_TABS = [
  { id: "status", label: '<i class="icon-chart-bar"></i> Status' },
  { id: "analytics", label: '<i class="icon-chart-pie"></i> Analytics' },
  { id: "explorer", label: '<i class="icon-folder"></i> Explorer' },
  { id: "meta", label: '<i class="icon-settings"></i> Core' },
  { id: "onchain", label: '<i class="icon-shield"></i> On-Chain' },
  { id: "dexscreener", label: '<i class="icon-trending-up"></i> DexScreener' },
  { id: "geckoterminal", label: '<i class="icon-trending-up"></i> GeckoTerminal' },
  { id: "rugcheck", label: '<i class="icon-shield"></i> RugCheck' },
];

export const TIME_RANGE_PRESETS = {
  "1h": { label: "1H", seconds: 60 * 60 },
  "6h": { label: "6H", seconds: 6 * 60 * 60 },
  "24h": { label: "24H", seconds: 24 * 60 * 60 },
  "7d": { label: "7D", seconds: 7 * 24 * 60 * 60 },
  all: { label: "All", seconds: null },
};

const SOURCE_LABELS = {
  meta: "Core",
  onchain: "On-Chain",
  dexscreener: "DexScreener",
  geckoterminal: "GeckoTerminal",
  rugcheck: "RugCheck",
};

function groupFields(source, fields) {
  const groups = new Map();

  for (const [key, metadata] of Object.entries(fields || {})) {
    if (source !== "meta" && key === "enabled") continue;
    const category = metadata.category || "General";
    if (!groups.has(category)) groups.set(category, []);
    groups.get(category).push({ key, ...metadata });
  }

  return Array.from(groups, ([category, categoryFields]) => {
    const enableField = categoryFields.find(
      (field) => field.type === "boolean" && field.key.endsWith("_enabled")
    );
    const visibleFields = enableField
      ? categoryFields.filter((field) => field.key !== enableField.key)
      : categoryFields;

    return [
      `${SOURCE_LABELS[source] || source} — ${category}`,
      {
        source,
        enableKey: enableField?.key,
        fields: visibleFields,
      },
    ];
  }).filter(([, group]) => group.fields.length > 0);
}

/**
 * Convert authoritative FilteringConfig metadata into the card groups used by
 * this page. Labels, hints, types, constraints, units and impact levels stay
 * identical to the main Configuration tab because both consume the same API.
 */
export function buildConfigCategories(filteringMetadata = {}) {
  const entries = [];
  const directFields = {};

  for (const [key, metadata] of Object.entries(filteringMetadata)) {
    if (metadata?.type === "object" && metadata.children) {
      entries.push(...groupFields(key, metadata.children));
    } else {
      directFields[key] = metadata;
    }
  }

  return Object.fromEntries([...groupFields("meta", directFields), ...entries]);
}

export function formatTimestampForInput(timestamp) {
  if (!timestamp) return "";
  const date = new Date(timestamp * 1000);
  const pad = (number) => number.toString().padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function getTimeRangeLabel(timeRange) {
  const { preset, startTime, endTime } = timeRange;
  if (preset === "all" || (!startTime && !endTime)) return "All Time";
  if (preset === "custom") {
    const start = startTime ? new Date(startTime * 1000).toLocaleString() : "∞";
    const end = endTime ? new Date(endTime * 1000).toLocaleString() : "Now";
    return `${start} → ${end}`;
  }
  return TIME_RANGE_PRESETS[preset]?.label || "Custom";
}

export function getStatusMessage({ isSaving, isRefreshing, hasChanges, lastSaved, Utils }) {
  if (isSaving) return "Saving changes...";
  if (isRefreshing) return "Refreshing snapshot...";
  if (hasChanges) return "Unsaved changes pending";
  if (lastSaved) return `Last saved ${Utils.formatTimeAgo(lastSaved)}`;
  return "Configuration in sync";
}

export function getConfigValue(config, source, key) {
  return source === "meta" ? config[key] : config[source]?.[key];
}

export function setConfigValue(config, source, key, value) {
  if (source === "meta") {
    config[key] = value;
    return;
  }
  if (!config[source]) config[source] = {};
  config[source][key] = value;
}

export function getSourceEnabled(config, source) {
  return source === "meta" || config[source]?.enabled !== false;
}

export function setSourceEnabled(config, source, enabled) {
  if (source === "meta") return;
  if (!config[source]) config[source] = {};
  config[source].enabled = enabled;
}

export function getCategoryEnabled(config, source, enableKey) {
  if (!enableKey) return true;
  return source === "meta" ? config[enableKey] !== false : config[source]?.[enableKey] !== false;
}

export function setCategoryEnabled(config, source, enableKey, enabled) {
  if (!enableKey) return;
  if (source === "meta") {
    config[enableKey] = enabled;
    return;
  }
  if (!config[source]) config[source] = {};
  config[source][enableKey] = enabled;
}

export function deepMerge(target, source) {
  const output = !target || typeof target !== "object" || Array.isArray(target) ? {} : target;
  if (!source || typeof source !== "object" || Array.isArray(source)) return output;

  for (const [key, value] of Object.entries(source)) {
    if (value && typeof value === "object" && !Array.isArray(value)) {
      output[key] = deepMerge(output[key], value);
    } else {
      output[key] = value;
    }
  }
  return output;
}

export function configsEqual(config1, config2) {
  return JSON.stringify(config1) === JSON.stringify(config2);
}
