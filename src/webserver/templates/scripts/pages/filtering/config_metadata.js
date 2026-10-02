// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Filtering page configuration helpers.
 *
 * Field presentation metadata is owned by the Rust config schema and delivered
 * by /api/config/metadata. This module owns only page navigation and the small
 * amount of grouping logic unique to the filtering workspace.
 */

import { formatTimeAgo, formatTimestamp } from "../../core/format.js";
import {
  categoryLabel,
  fieldHint,
  fieldLabel,
  fieldSubject,
  fieldUnit,
} from "../config/field_text.js";

// Ids are the settings sub-tabs: the `meta` (core) settings and the sources of the
// filtering config.
export const SOURCE_LABELS = Object.freeze({
  meta: "filtering-source-core",
  onchain: "filtering-source-onchain",
  dexscreener: "filtering-source-dexscreener",
  geckoterminal: "filtering-source-geckoterminal",
  rugcheck: "filtering-source-rugcheck",
});

// Ids are `FilterSource::as_str` (src/filtering/sources/mod.rs), as sent by the
// rejection statistics.
export const REJECTION_SOURCE_LABELS = Object.freeze({
  core: "filtering-source-core",
  onchain: "filtering-source-onchain",
  dexscreener: "filtering-source-dexscreener",
  geckoterminal: "filtering-source-geckoterminal",
  rugcheck: "filtering-source-rugcheck",
  llm_analysis: "filtering-source-llm-analysis",
});

// Ids are the keys of TIME_RANGE_PRESETS.
export const TIME_RANGE_LABELS = Object.freeze({
  "1h": "filtering-range-1h",
  "6h": "filtering-range-6h",
  "24h": "filtering-range-24h",
  "7d": "filtering-range-7d",
  all: "filtering-range-all",
});

/**
 * The sub-tabs, each labelled with its icon and localized name. `escapeHtml` escapes the
 * name, which the tab bar inserts as markup.
 */
export function buildFilterTabs(escapeHtml) {
  const tab = (id, icon, name) => ({ id, label: `<i class="${icon}"></i> ${escapeHtml(name)}` });
  return [
    tab("status", "icon-chart-bar", I18n.t("filtering-tab-status")),
    tab("analytics", "icon-chart-pie", I18n.t("filtering-tab-analytics")),
    tab("explorer", "icon-folder", I18n.t("filtering-tab-explorer")),
    tab("meta", "icon-settings", I18n.label(SOURCE_LABELS, "meta")),
    tab("onchain", "icon-shield", I18n.label(SOURCE_LABELS, "onchain")),
    tab("dexscreener", "icon-trending-up", I18n.label(SOURCE_LABELS, "dexscreener")),
    tab("geckoterminal", "icon-trending-up", I18n.label(SOURCE_LABELS, "geckoterminal")),
    tab("rugcheck", "icon-shield", I18n.label(SOURCE_LABELS, "rugcheck")),
  ];
}

export const TIME_RANGE_PRESETS = {
  "1h": { seconds: 60 * 60 },
  "6h": { seconds: 6 * 60 * 60 },
  "24h": { seconds: 24 * 60 * 60 },
  "7d": { seconds: 7 * 24 * 60 * 60 },
  all: { seconds: null },
};

/** The sub-tabs that edit configuration; the rest read filtering results. */
export const SETTINGS_TABS = Object.keys(SOURCE_LABELS);

const IMPACT_WEIGHT = { critical: 4, high: 3, medium: 2, low: 1 };

// Windows read in the order a trader thinks about them. `/api/config/metadata`
// is a BTreeMap keyed by field name, so the payload arrives in ALPHABETICAL key
// order — which puts every `max_*` before every `min_*` and sorts 24h ahead of
// 5m. Presentation order therefore has to be derived here.
const TIMEFRAME_RANK = {
  "1m": 1,
  "5m": 2,
  "5min": 2,
  "15m": 3,
  "30m": 4,
  "1h": 5,
  "2h": 6,
  "6h": 7,
  "12h": 8,
  "24h": 9,
  "7d": 10,
};

function boundOf(key) {
  const match = /^(min|max)_(.+)$/.exec(key);
  return match ? { bound: match[1], subject: match[2] } : null;
}

function timeframeRank(subject) {
  const match = /_(\d+(?:m|min|h|d))$/.exec(subject || "");
  return match ? (TIMEFRAME_RANK[match[1]] ?? 0) : 0;
}

function strongerImpact(a, b) {
  return (IMPACT_WEIGHT[b] ?? 0) > (IMPACT_WEIGHT[a] ?? 0) ? b : a;
}

/**
 * Fold a category's fields into display rows: a `min_x` / `max_x` pair becomes
 * ONE range row, everything else stays a single row.
 *
 * A bound pair is one parameter with two ends, and rendering it as two rows was
 * what made a two-bound group (FDV, Liquidity, Market Cap) print the same
 * subject, hint and unit twice while an eight-field group towered beside it.
 */
function buildRows(fields) {
  const byKey = new Map(fields.map((field) => [field.key, field]));
  const consumed = new Set();
  const rows = [];

  for (const field of fields) {
    if (consumed.has(field.key)) continue;

    const bound = boundOf(field.key);
    const partner = bound
      ? byKey.get(`${bound.bound === "min" ? "max" : "min"}_${bound.subject}`)
      : undefined;
    // The lower bound's catalog message names the shared subject ("Liquidity") in `.subject`.
    const pairable =
      bound &&
      partner &&
      partner.type === field.type &&
      field.type !== "boolean" &&
      Boolean(bound.bound === "min" ? field.subject : partner.subject);

    if (pairable) {
      const [min, max] = bound.bound === "min" ? [field, partner] : [partner, field];
      consumed.add(min.key);
      consumed.add(max.key);
      rows.push({
        kind: "range",
        key: `${min.key}+${max.key}`,
        subject: bound.subject,
        label: min.subject,
        hint: min.hint || max.hint,
        unit: min.unit || max.unit,
        impact: strongerImpact(min.impact, max.impact),
        fields: [min, max],
      });
      continue;
    }

    consumed.add(field.key);
    rows.push({
      kind: "field",
      key: field.key,
      subject: bound?.subject || field.key,
      label: field.label,
      hint: field.hint,
      unit: field.unit,
      impact: field.impact,
      fields: [field],
    });
  }

  return rows
    .map((row, index) => ({ row, index }))
    .sort(
      (a, b) => timeframeRank(a.row.subject) - timeframeRank(b.row.subject) || a.index - b.index
    )
    .map((entry) => entry.row);
}

/**
 * A field as the sub-tabs render it: `key` is the config key, `catalogKey` the
 * localization key, and `label`, `hint` and `unit` are read from the catalog.
 */
function withCatalogText(key, metadata) {
  return {
    ...metadata,
    key,
    catalogKey: metadata.key,
    label: fieldLabel(metadata.key),
    subject: fieldSubject(metadata.key),
    hint: fieldHint(metadata.key),
    unit: fieldUnit(metadata.key),
  };
}

function groupFields(source, fields) {
  const categories = new Map();

  for (const [key, metadata] of Object.entries(fields || {})) {
    // A source's own master switch is not one of its parameters — it is the
    // sub-tab's master control (see `getSourceMasterField`).
    if (source !== "meta" && key === "enabled") continue;
    const category = metadata.category || "general";
    if (!categories.has(category)) categories.set(category, []);
    categories.get(category).push(withCatalogText(key, metadata));
  }

  return Array.from(categories, ([category, categoryFields]) => {
    const enableField = categoryFields.find(
      (field) => field.type === "boolean" && field.key.endsWith("_enabled")
    );
    const valueFields = enableField
      ? categoryFields.filter((field) => field.key !== enableField.key)
      : categoryFields;

    return {
      // Categories repeat across sources ("Liquidity", "Volume", "General"), so
      // the identity has to carry the source — a bare category name silently
      // collided and dropped groups when they were keyed by title.
      id: `${source}:${category}`,
      title: categoryLabel(category),
      source,
      enableKey: enableField?.key,
      enableHint: enableField?.hint,
      rows: buildRows(valueFields),
    };
  }).filter((group) => group.rows.length > 0);
}

/**
 * Convert authoritative FilteringConfig metadata into the ordered groups this
 * page renders. Labels, hints, types, constraints, units and impact levels stay
 * identical to the main Configuration tab because both consume the same API.
 */
export function buildConfigGroups(filteringMetadata = {}) {
  const nested = [];
  const directFields = {};

  for (const [key, metadata] of Object.entries(filteringMetadata)) {
    if (metadata?.type === "object" && metadata.children) {
      nested.push(...groupFields(key, metadata.children));
    } else {
      directFields[key] = metadata;
    }
  }

  return [...groupFields("meta", directFields), ...nested];
}

/**
 * The `enabled` field of a source, i.e. its master switch. Read from the
 * metadata rather than a hardcoded list, so a new source cannot ship with an
 * unreachable master switch — which is exactly how On-Chain lost its own.
 */
export function getSourceMasterField(filteringMetadata, source) {
  if (source === "meta") return null;
  const field = filteringMetadata?.[source]?.children?.enabled;
  return field && field.type === "boolean" ? withCatalogText("enabled", field) : null;
}

export function formatTimestampForInput(timestamp) {
  if (!timestamp) return "";
  const date = new Date(timestamp * 1000);
  const pad = (number) => number.toString().padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function getTimeRangeLabel(timeRange) {
  const { preset, startTime, endTime } = timeRange;
  if (preset === "all" || (!startTime && !endTime)) return I18n.t("filtering-range-all-time");
  if (preset === "custom") {
    const start = startTime ? formatTimestamp(startTime) : "∞";
    const end = endTime ? formatTimestamp(endTime) : I18n.t("filtering-range-now");
    return I18n.t("filtering-range-span", { start, end });
  }
  return Object.hasOwn(TIME_RANGE_PRESETS, preset)
    ? I18n.label(TIME_RANGE_LABELS, preset)
    : I18n.t("filtering-range-custom");
}

export function getStatusMessage({ isSaving, isRefreshing, hasChanges, lastSaved }) {
  if (isSaving) return I18n.t("filtering-footer-saving");
  if (isRefreshing) return I18n.t("filtering-footer-refreshing");
  if (hasChanges) return I18n.t("filtering-footer-unsaved");
  if (lastSaved) return I18n.t("filtering-footer-last-saved", { time: formatTimeAgo(lastSaved) });
  return I18n.t("filtering-footer-in-sync");
}

/** The schema's declared default for one field, or `undefined` if it has none. */
export function getFieldDefault(filteringMetadata, source, key) {
  const field =
    source === "meta" ? filteringMetadata?.[key] : filteringMetadata?.[source]?.children?.[key];
  return field?.default;
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
