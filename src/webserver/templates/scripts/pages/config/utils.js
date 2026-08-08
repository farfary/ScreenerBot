/**
 * Pure utility functions for config page.
 * These functions have no state dependencies and can be imported/used independently.
 */

export const SECTION_DISPLAY_ORDER = [
  "rpc",
  "trader",
  "positions",
  "filtering",
  "swaps",
  "tokens",
  "pools",
  "wallet",
  "copy_trading",
  "sol_price",
  "telegram",
  "ai",
  "strategies",
  "holder_watch",
  "events",
  "webserver",
  "services",
  "monitoring",
  "performance",
  "maintenance",
  "network",
  "referral",
  "account",
  "ohlcv",
  "summary",
];

export const SECTION_LABEL_OVERRIDES = {
  rpc: "RPC",
  account: "ScreenerBot Account",
  trader: "Auto Trader",
  positions: "Positions",
  filtering: "Filtering",
  swaps: "Swaps",
  tokens: "Tokens",
  pools: "Pools",
  wallet: "Wallet",
  copy_trading: "Wallet Copy",
  sol_price: "SOL Price",
  events: "Events",
  webserver: "Webserver",
  services: "Services",
  monitoring: "Monitoring",
  maintenance: "Maintenance",
  ohlcv: "OHLCV",
  summary: "Summary",
  telegram: "Telegram",
  ai: "Assistant",
  strategies: "Strategies",
  holder_watch: "Holder Watch",
  performance: "Performance",
  network: "Network",
};

/**
 * Convert snake_case or space-separated string to Title Case.
 * @param {string} id - The identifier to convert
 * @returns {string} Title-cased string
 */
export function toTitleCase(id) {
  return id
    .split(/[_\s]+/)
    .filter(Boolean)
    .map((chunk) => chunk.charAt(0).toUpperCase() + chunk.slice(1))
    .join(" ");
}

/**
 * Format a section ID into a display label.
 * Uses SECTION_LABEL_OVERRIDES if available, otherwise converts to title case.
 * @param {string} sectionId - The section identifier
 * @returns {string} Formatted label
 */
export function formatSectionLabel(sectionId) {
  if (SECTION_LABEL_OVERRIDES[sectionId]) {
    return SECTION_LABEL_OVERRIDES[sectionId];
  }
  return toTitleCase(sectionId);
}

/**
 * Parse array input from textarea (one value per line).
 * Validates values based on itemType.
 * @param {string} rawText - Raw textarea content
 * @param {string} itemType - Type of array items (string, number, integer, boolean)
 * @returns {{ values: Array, invalid: Array<{index: number, value: string}> }}
 */
export function parseArrayInput(rawText, itemType) {
  const lines = rawText
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

  const normalizedType = (itemType || "string").toLowerCase();

  if (normalizedType === "number" || normalizedType === "integer") {
    const values = [];
    const invalid = [];
    lines.forEach((line, index) => {
      const parsed = Number(line);
      const isFiniteNumber = Number.isFinite(parsed);
      const isIntegerValid = normalizedType === "integer" ? Number.isInteger(parsed) : true;
      if (!isFiniteNumber || !isIntegerValid) {
        invalid.push({ index, value: line });
        return;
      }
      values.push(parsed);
    });
    return { values, invalid };
  }

  if (normalizedType === "boolean") {
    const truthy = new Set(["true", "1", "yes", "y", "on"]);
    const falsy = new Set(["false", "0", "no", "n", "off"]);
    const values = [];
    const invalid = [];

    lines.forEach((line, index) => {
      const lowered = line.toLowerCase();
      if (truthy.has(lowered)) {
        values.push(true);
        return;
      }
      if (falsy.has(lowered)) {
        values.push(false);
        return;
      }
      invalid.push({ index, value: line });
    });

    return { values, invalid };
  }

  return { values: lines, invalid: [] };
}

/**
 * Create human-readable error message for invalid array entries.
 * @param {Array<{index: number, value: string}>} invalidEntries - Invalid entries
 * @param {string} itemType - Expected item type
 * @returns {string} Error message
 */
export function describeInvalidArrayEntries(invalidEntries, itemType) {
  if (!invalidEntries.length) {
    return "";
  }
  const lines = invalidEntries.map((entry) => entry.index + 1).join(", ");
  const typeLabel = itemType === "integer" ? "integer" : itemType || "value";
  return `Line${invalidEntries.length === 1 ? "" : "s"} ${lines} must be a valid ${typeLabel}.`;
}

/**
 * Normalize field value based on type.
 * @param {string} fieldType - Field type
 * @param {*} value - Value to normalize
 * @returns {*} Normalized value
 */
export function normalizeFieldValue(fieldType, value) {
  if (value === null || value === undefined) {
    return null;
  }
  if (fieldType === "number" || fieldType === "integer") {
    if (typeof value === "number") {
      return value;
    }
    if (typeof value === "string") {
      const parsed = Number(value);
      return Number.isFinite(parsed) ? parsed : value;
    }
  }
  if (fieldType === "boolean") {
    return Boolean(value);
  }
  if (fieldType === "array") {
    return Array.isArray(value) ? value.slice() : [];
  }
  if (fieldType === "object") {
    return value && typeof value === "object" ? { ...value } : {};
  }
  if (fieldType === "string") {
    return typeof value === "string" ? value : String(value);
  }
  return value;
}

/**
 * Deep clone a value (supports arrays and objects).
 * @param {*} value - Value to clone
 * @returns {*} Cloned value
 */
export function deepClone(value) {
  if (Array.isArray(value)) {
    return value.map((item) => deepClone(item));
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([key, val]) => [key, deepClone(val)]));
  }
  return value;
}

/**
 * Deep equality check for two values.
 * @param {*} a - First value
 * @param {*} b - Second value
 * @returns {boolean} True if deeply equal
 */
export function deepEqual(a, b) {
  if (a === b) {
    return true;
  }
  if (typeof a !== typeof b) {
    return false;
  }
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) {
      return false;
    }
    for (let i = 0; i < a.length; i += 1) {
      if (!deepEqual(a[i], b[i])) {
        return false;
      }
    }
    return true;
  }
  if (a && typeof a === "object" && b && typeof b === "object") {
    const aKeys = Object.keys(a);
    const bKeys = Object.keys(b);
    if (aKeys.length !== bKeys.length) {
      return false;
    }
    for (const key of aKeys) {
      if (!deepEqual(a[key], b[key])) {
        return false;
      }
    }
    return true;
  }
  return false;
}

/**
 * Summarize section fields (count total, critical, performance).
 * @param {Object} fields - Section fields metadata
 * @returns {{ total: number, critical: number, performance: number }}
 */
export function summarizeSectionFields(fields = {}) {
  const summary = { total: 0, critical: 0, performance: 0 };

  function countRecursive(fieldMap) {
    for (const field of Object.values(fieldMap)) {
      const children = field.children || field.fields;
      if (children && Object.keys(children).length > 0) {
        countRecursive(children);
      } else {
        summary.total += 1;
        const impact = (field.impact || "").toLowerCase();
        if (impact === "critical") summary.critical += 1;
        const category = (field.category || "").toLowerCase();
        if (category.includes("performance")) summary.performance += 1;
      }
    }
  }

  countRecursive(fields);
  return summary;
}

/**
 * Normalize field metadata recursively.
 * @param {Object} fieldMeta - Raw field metadata
 * @returns {Object} Normalized metadata
 */
export function normalizeFieldMetadata(fieldMeta = {}) {
  const normalized = {
    ...fieldMeta,
    type: typeof fieldMeta.type === "string" ? fieldMeta.type.toLowerCase() : "string",
    default: deepClone(fieldMeta.default ?? null),
  };

  if (typeof fieldMeta.item_type === "string") {
    normalized.item_type = fieldMeta.item_type.toLowerCase();
  }

  if (fieldMeta.children && typeof fieldMeta.children === "object") {
    const normalizedChildren = {};
    for (const [childKey, childMeta] of Object.entries(fieldMeta.children)) {
      normalizedChildren[childKey] = normalizeFieldMetadata(childMeta);
    }
    normalized.children = normalizedChildren;
  }

  return normalized;
}

/**
 * Check if field metadata matches search term.
 * @param {string} fieldKey - Field key
 * @param {Object} fieldMeta - Field metadata
 * @param {string} term - Search term (already lowercased)
 * @returns {boolean} True if matches
 */
export function metadataMatchesSearch(fieldKey, fieldMeta, term) {
  if (!term || term.length === 0) {
    return false;
  }
  const matches = (value) => typeof value === "string" && value.toLowerCase().includes(term);

  if (matches(fieldKey)) {
    return true;
  }
  if (
    matches(fieldMeta.label) ||
    matches(fieldMeta.hint) ||
    matches(fieldMeta.docs) ||
    matches(fieldMeta.unit)
  ) {
    return true;
  }

  if (fieldMeta.children) {
    for (const [childKey, childMeta] of Object.entries(fieldMeta.children)) {
      if (matches(childKey) || metadataMatchesSearch(childKey, childMeta, term)) {
        return true;
      }
    }
  }

  return false;
}

/**
 * Check if a section has fields matching search term.
 * @param {string} sectionId - Section identifier
 * @param {string} term - Search term (already lowercased)
 * @param {Object} metadata - Full metadata object
 * @returns {boolean} True if section has matching fields
 */
export function sectionHasMatchingFields(sectionId, term, metadata) {
  if (!term || term.length === 0) {
    return true;
  }
  const sectionMeta = metadata?.[sectionId];
  if (!sectionMeta) {
    return false;
  }

  const label = sectionMeta.label ?? formatSectionLabel(sectionId);
  if (sectionId.toLowerCase().includes(term) || label.toLowerCase().includes(term)) {
    return true;
  }

  const fields = sectionMeta.fields ?? {};
  for (const [fieldKey, fieldMeta] of Object.entries(fields)) {
    const category = fieldMeta.category ?? "General";
    if (category.toLowerCase().includes(term)) {
      return true;
    }
    if (metadataMatchesSearch(fieldKey, fieldMeta, term)) {
      return true;
    }
  }
  return false;
}

/**
 * Transform raw metadata into structured format.
 * @param {Object} raw - Raw metadata from API
 * @returns {Object} Transformed metadata
 */
export function transformMetadata(raw) {
  const sections = {};
  for (const [sectionId, fields] of Object.entries(raw || {})) {
    const normalizedFields = {};
    for (const [fieldKey, fieldMeta] of Object.entries(fields || {})) {
      normalizedFields[fieldKey] = normalizeFieldMetadata(fieldMeta);
    }

    sections[sectionId] = {
      id: sectionId,
      label: formatSectionLabel(sectionId),
      fields: normalizedFields,
      summary: summarizeSectionFields(normalizedFields),
    };
  }
  return sections;
}

/**
 * Sort section entries for display using SECTION_DISPLAY_ORDER.
 * @param {Array<[string, *]>} entries - Section entries
 * @returns {Array<[string, *]>} Sorted entries
 */
export function sortSectionsForDisplay(entries) {
  const orderIndex = (sectionId) => {
    const index = SECTION_DISPLAY_ORDER.indexOf(sectionId);
    return index === -1 ? Number.POSITIVE_INFINITY : index;
  };

  return entries.sort(([idA], [idB]) => {
    const orderA = orderIndex(idA);
    const orderB = orderIndex(idB);
    if (orderA === orderB) {
      return idA.localeCompare(idB);
    }
    return orderA - orderB;
  });
}
