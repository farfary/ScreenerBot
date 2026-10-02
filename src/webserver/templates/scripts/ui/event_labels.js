// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Presentation for an event's category, subtype and severity, shared by the
 * Events page and the event details dialog.
 */
import { escapeHtml } from "../core/utils.js";

// `EventCategory` string ids (src/events/types.rs), plus the legacy `entry` and
// `learner` categories older rows and the category filter still carry.
export const EVENT_CATEGORY_LABELS = Object.freeze({
  swap: "events-category-swap",
  transaction: "events-category-transaction",
  pool: "events-category-pool",
  position: "events-category-position",
  token: "events-category-token",
  wallet: "events-category-wallet",
  trader: "events-category-trader",
  entry: "events-category-entry",
  system: "events-category-system",
  ohlcv: "events-category-ohlcv",
  rpc: "events-category-rpc",
  api: "events-category-api",
  security: "events-category-security",
  connectivity: "events-category-connectivity",
  filtering: "events-category-filtering",
  scheduled_task: "events-category-scheduled-task",
  learner: "events-category-learner",
  other: "events-category-other",
});

// `Severity` string ids (src/events/types.rs); `critical` is a stored legacy level.
export const EVENT_SEVERITY_LABELS = Object.freeze({
  info: "common-severity-info",
  warn: "common-severity-warning",
  error: "common-severity-error",
  critical: "common-severity-critical",
  debug: "common-severity-debug",
});

/** Message key of each stable scheduled-task subtype code; older rows keep their stored subtype. */
export const EVENT_SUBTYPE_LABELS = Object.freeze({
  task_completed: "events-subtype-task-completed",
  task_failed: "events-subtype-task-failed",
  task_timed_out: "events-subtype-task-timed-out",
});

const SEVERITY_STYLES = {
  info: { variant: "", icon: "icon-info" },
  warn: { variant: " warning", icon: "icon-triangle-alert" },
  error: { variant: " error", icon: "icon-x" },
  critical: { variant: " error", icon: "icon-circle-alert" },
  debug: { variant: " secondary", icon: "icon-bug" },
};

export function eventCategoryLabel(category) {
  return I18n.label(EVENT_CATEGORY_LABELS, category);
}

/** Text of an event subtype: the label of a known code, else the stored value. */
export function eventSubtypeLabel(subtype) {
  return Object.hasOwn(EVENT_SUBTYPE_LABELS, subtype)
    ? I18n.label(EVENT_SUBTYPE_LABELS, subtype)
    : String(subtype);
}

/** Severity badge markup, or an empty string when the event has no severity. */
export function severityBadge(severity) {
  if (!severity) return "";
  const lowered = String(severity).toLowerCase();
  const key = lowered === "warning" ? "warn" : lowered;
  const style = SEVERITY_STYLES[key];
  if (!style) return `<span class="badge">${escapeHtml(String(severity))}</span>`;
  return `<span class="badge${style.variant}"><i class="${style.icon}"></i> ${escapeHtml(I18n.label(EVENT_SEVERITY_LABELS, key))}</span>`;
}
