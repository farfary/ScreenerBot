// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Copy Trading labels and formatters shared by every panel. The formatters
// compose the functions of core/format.js with this feature's precision.

import {
  formatFixed,
  formatPercentValue,
  formatPriceSubscript,
  formatSignatureCompact,
  formatSignedSol,
  formatSol,
  formatTimeAgo,
  formatTimeSpan,
  formatTimestamp,
  formatUptime,
  signedTone,
  signedToneClass,
  withSolUnit,
} from "../../core/format.js";
import { closeReasonText } from "../../ui/trade_reason.js";
import { ENTRY_BLOCK_LABELS, entryBlockText } from "../../ui/entry_block.js";
import { RANGE_OPTIONS } from "../../ui/segmented.js";

export const SOLANA_ADDRESS_RE = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

// Task states computed by `effective_state` (src/trader/copy/control.rs).
const STATE_LABELS = Object.freeze({
  system_paused: "copy-state-system-paused",
  force_stopped: "copy-state-force-stopped",
  paused: "copy-state-paused",
  entries_blocked: "copy-state-entries-blocked",
  own_rpc_required: "copy-state-own-rpc-required",
  live: "copy-state-running-live",
  paper: "copy-state-running-paper",
});

// Exit modes serialized by `ExitMode` (src/trader/copy/types.rs).
const EXIT_MODE_LABELS = Object.freeze({
  buy_only: "copy-exit-mode-buy-only",
  mirror: "copy-exit-mode-mirror",
  hybrid: "copy-exit-mode-hybrid",
});

// The `PaperExitRule` a sale of the archived copy paper book recorded
// (src/trader/copy/types/paper.rs).
const EXIT_LABELS = Object.freeze({
  stop_loss: "copy-exit-stop-loss",
  trailing_stop: "copy-exit-trailing-stop",
  break_even: "copy-exit-break-even",
  take_profit: "copy-exit-take-profit",
  time_override: "copy-exit-time-override",
  time_stagnation: "copy-exit-time-stagnation",
  manual: "copy-exit-manual",
});

export const stateLabel = (state) =>
  Object.hasOwn(STATE_LABELS, state) ? I18n.label(STATE_LABELS, state) : I18n.t("format-unknown");

export const exitModeLabel = (mode) => I18n.label(EXIT_MODE_LABELS, mode);

/** A closed round carries its trade close reason; an archived sale names its exit rule. */
export const exitLabel = (exit) =>
  Object.hasOwn(EXIT_LABELS, exit) ? I18n.label(EXIT_LABELS, exit) : closeReasonText(exit);

// Skip ids serialized by `CopySkip` (src/trader/copy/types.rs).
const SKIP_LABELS = Object.freeze({
  not_buy_swap: "copy-skip-not-buy-swap",
  task_disabled: "copy-skip-task-disabled",
  mode_transition_required: "copy-skip-mode-transition-required",
  live_confirmation_required: "copy-skip-live-confirmation-required",
  unsupported_sizing_mode: "copy-skip-unsupported-sizing-mode",
  self_copy: "copy-skip-self-copy",
  target_below_minimum: "copy-skip-target-below-minimum",
  target_above_maximum: "copy-skip-target-above-maximum",
  already_bought: "copy-skip-already-bought",
  blacklisted: "copy-skip-blacklisted",
  filter_required: "copy-skip-filter-required",
  budget_exhausted: "copy-skip-budget-exhausted",
  token_cap_reached: "copy-skip-token-cap-reached",
  below_minimum_size: "copy-skip-below-minimum-size",
  invalid_sizing: "copy-skip-invalid-sizing",
  invalid_slippage: "copy-skip-invalid-slippage",
  invalid_exit_policy: "copy-skip-invalid-exit-policy",
  invalid_price: "copy-skip-invalid-price",
  not_sell_swap: "copy-skip-not-sell-swap",
  exit_mode_disabled: "copy-skip-exit-mode-disabled",
  force_stopped: "copy-skip-force-stopped",
  copy_position_not_found: "copy-skip-copy-position-not-found",
  position_user_only: "copy-skip-position-user-only",
  position_management_mismatch: "copy-skip-position-management-mismatch",
  latency_kill_switch: "copy-skip-latency-kill-switch",
  claim_reconciled_abandoned: "copy-skip-claim-reconciled-abandoned",
  stale_observation: "copy-skip-stale-observation",
  unknown_observation_time: "copy-skip-unknown-observation-time",
  paper_quote_unavailable: "copy-skip-paper-quote-unavailable",
  paper_quote_unfillable: "copy-skip-paper-quote-unfillable",
  paper_minimum_output_not_met: "copy-skip-paper-minimum-output-not-met",
  paper_queue_full: "copy-skip-paper-queue-full",
  paper_execution_unavailable: "copy-skip-paper-execution-unavailable",
  entry_blocked: "copy-skip-entry-blocked",
  stable_quote_disabled: "copy-skip-stable-quote-disabled",
  stable_quote_price_unavailable: "copy-skip-stable-quote-price-unavailable",
  swap_mixed_quote: "copy-skip-swap-mixed-quote",
  swap_multiple_tokens: "copy-skip-swap-multiple-tokens",
  swap_unreadable: "copy-skip-swap-unreadable",
});

/** Label for a skip key as the backend groups them (`kind` or `kind.block`). */
export function skipLabel(key) {
  const [kind, block] = String(key || "").split(".");
  if (kind === "entry_blocked" && block && Object.hasOwn(ENTRY_BLOCK_LABELS, block)) {
    return entryBlockText(block);
  }
  return I18n.label(SKIP_LABELS, kind);
}

/** Group key of a skipped outcome, matching the backend's breakdown keys. */
export function skipKey(reason) {
  if (!reason?.kind) return "unknown";
  return reason.block?.kind ? `${reason.kind}.${reason.block.kind}` : reason.kind;
}

// Pause kinds serialized by `CopyPauseReason` (src/trader/copy/types.rs).
const PAUSE_SHORT_LABELS = Object.freeze({
  user: "copy-pause-short-user",
  latency_kill_switch: "copy-pause-short-latency-kill-switch",
  watch_detached: "copy-pause-short-watch-detached",
  watch_budget_exceeded: "copy-pause-short-watch-budget-exceeded",
  helius_unavailable: "copy-pause-short-helius-unavailable",
  watch_processing_failed: "copy-pause-short-watch-processing-failed",
  storage_restored: "copy-pause-short-storage-restored",
});

/** Short cause of a pause for the task list; empty when the kind has none. */
export function pauseReasonShort(reason) {
  return Object.hasOwn(PAUSE_SHORT_LABELS, reason?.kind)
    ? I18n.label(PAUSE_SHORT_LABELS, reason.kind)
    : "";
}

/** A task's state in a few words: its effective state, or why it is paused. */
export function stateText(task) {
  if (!task.enabled) {
    const short = pauseReasonShort(task.pause_reason);
    return short
      ? I18n.t("copy-state-paused-reason", { reason: short })
      : I18n.t("copy-state-paused");
  }
  return stateLabel(task.effective_state);
}

/**
 * The warning a task carries while its stored exit overrides no longer fit the
 * Trader's exit rules (`exit_policy_issue`, a backend UiText naming the broken rule);
 * empty while they fit.
 */
export function exitPolicyIssueHtml(task, escapeHtml, className) {
  if (!task.exit_policy_issue) return "";
  return `<span class="${className}" role="note"><i class="icon-triangle-alert" aria-hidden="true"></i><span>${escapeHtml(I18n.text(task.exit_policy_issue))}</span></span>`;
}

export function finite(value) {
  const number = Number(value);
  return value !== null && value !== undefined && value !== "" && Number.isFinite(number)
    ? number
    : null;
}

export function fixed(value, decimals = 4) {
  return formatFixed(value, { decimals, fallback: "—" });
}

export function sol(value, decimals = 4) {
  return formatSol(value, { decimals, fallback: "—" });
}

/**
 * A free-standing SOL amount whose message names the unit ("{ $spent } / { $budget } SOL"):
 * trimmed like `sol()`, so it reads beside the P&L figures of the same card.
 */
export function solNumber(value, decimals = 4) {
  return formatSol(value, { decimals, fallback: "—", suffix: "" });
}

/** A SOL amount in a table cell: the column header names the unit, so the cell does not. */
export function solCell(value) {
  return formatSol(value, { decimals: 4, fallback: "—", suffix: "", trim: false });
}

/** Decimals of every Copy Trading P&L figure: list rows, cards, buckets and charts. */
export const PNL_SOL_DECIMALS = 4;

export function signedSol(value) {
  return formatSignedSol(value, { decimals: PNL_SOL_DECIMALS, fallback: "—" });
}

/** A signed SOL P&L in a table cell, whose column header names the unit. */
export function signedSolCell(value) {
  return formatSignedSol(value, {
    decimals: PNL_SOL_DECIMALS,
    minDecimals: PNL_SOL_DECIMALS,
    fallback: "—",
    unit: false,
  });
}

export function pct(value, decimals = 1) {
  return formatPercentValue(value, { decimals, plus: "", fallback: "—" });
}

/**
 * A configured amount or percentage (a cap, a budget, a rule threshold) as the user set
 * it: at most the field's precision, with no padded fraction zeros ("0.05 SOL", "2%").
 * Measured figures (P&L, balances, table columns) keep the fixed-decimal formatters.
 */
export function settingFixed(value) {
  return formatFixed(value, { decimals: 4, trim: true, fallback: "—" });
}

export function settingSol(value) {
  return formatSol(value, { decimals: 4, fallback: "—" });
}

export function settingPct(value) {
  return formatPercentValue(value, { decimals: 2, plus: "", trim: true, fallback: "—" });
}

/**
 * Unrealized P&L covers priced holdings only: with none priced there is no
 * figure, and a partial one says what it leaves out.
 */
export function unrealizedFigure(pnlSol, openHoldings, unpricedHoldings) {
  const open = Number(openHoldings) || 0;
  const unpriced = Number(unpricedHoldings) || 0;
  const priced = open - unpriced;
  if (!unpriced) return { value: pnlSol, note: null };
  return {
    value: priced > 0 ? pnlSol : null,
    note:
      priced > 0
        ? I18n.t("copy-unrealized-partial", { priced, unpriced })
        : I18n.t("copy-unrealized-unpriced", { count: unpriced }),
  };
}

/** Sign of a SOL P&L as `signedSol` prints it (`signedTone`), the tone of a stat card's figure. */
export function pnlTone(value) {
  return signedTone(finite(value), PNL_SOL_DECIMALS);
}

/** Tone class of a SOL P&L as `signedSol` prints it; "" when it reads as zero or is absent. */
export function toneClass(value) {
  return signedToneClass(finite(value), PNL_SOL_DECIMALS);
}

/** A pool price in SOL at a fixed count of significant digits, the price rule of every
 * Copy table and chart: one column keeps one digit count. */
export function price(value) {
  const number = finite(value);
  return number === null ? "—" : formatPriceSubscript(number, { trim: false });
}

/** A pool price with its SOL unit, for tooltips and messages. */
export function priceSol(value) {
  return withSolUnit(price(value));
}

/**
 * A millisecond span in seconds. Arrival distances read at one decimal below ten seconds
 * and whole seconds above. `precise` is for configured paper waits, which may carry
 * millisecond precision: it shows one decimal when the value is a whole tenth, two when a
 * whole hundredth and three otherwise, and never rounds.
 */
export function seconds(ms, { precise = false } = {}) {
  const number = finite(ms);
  if (number === null) return "—";
  if (precise) {
    const decimals = number % 100 === 0 ? 1 : number % 10 === 0 ? 2 : 3;
    return formatTimeSpan(number / 1000, { decimals });
  }
  return number < 10_000
    ? formatTimeSpan(number / 1000, { decimals: 1 })
    : formatTimeSpan(Math.round(number / 1000));
}

/** Humanized duration from seconds: 45s, 12m, 3h 5m, 2d 4h. */
export function duration(totalSeconds) {
  const value = finite(totalSeconds);
  if (value === null) return "—";
  return formatUptime(Math.max(0, Math.round(value)), { style: "trimmed" });
}

export function timeAgo(value) {
  if (!value) return "—";
  return formatTimeAgo(value, { style: "detailed", fallback: "—" });
}

export function dateTime(value) {
  if (!value) return "—";
  return formatTimestamp(value, { includeYear: false, fallback: String(value) });
}

/** The `head…tail` split of a compact address, as `COMPACT_ADDRESS` in ui/token_identity.js. */
const TASK_ADDRESS_SPLIT = Object.freeze({ start: 4, end: 4 });

/**
 * A task's name: its label, or the short form of its wallet, which tells unnamed tasks
 * apart the way the task list shows them.
 */
export function taskName(task) {
  if (task?.label) return task.label;
  if (task?.target_address) return formatSignatureCompact(task.target_address, TASK_ADDRESS_SPLIT);
  return I18n.t("copy-task-unnamed");
}

/** The `from` timestamp of a range from `RANGE_OPTIONS`; the whole history sends none. */
export function rangeQuery(rangeId) {
  const range = RANGE_OPTIONS.find((item) => item.value === rangeId);
  if (!range?.hours) return {};
  return { from: new Date(Date.now() - range.hours * 3_600_000).toISOString() };
}

/** A section of the task sheet with its heading; the sheet draws the hairline between sections. */
export function section(title, body, esc, className = "") {
  return `<section class="sheet-section${className ? ` ${className}` : ""}"><h3 class="sheet-section-title">${esc(title)}</h3>${body}</section>`;
}

/**
 * Two parts of the sheet side by side on a wide sheet, stacked on a narrow one; each part is
 * `[title, bodyHtml]`.
 */
export function splitSection(parts, esc) {
  const html = parts
    .map(
      ([title, body]) =>
        `<section><h3 class="sheet-section-title">${esc(title)}</h3>${body}</section>`
    )
    .join("");
  return `<div class="sheet-section sheet-split">${html}</div>`;
}

export function definitionRows(rows, escapeHtml) {
  return rows
    .filter(Boolean)
    .map(
      ([label, value, note]) =>
        `<div class="copy-def"><dt>${escapeHtml(label)}</dt><dd>${escapeHtml(value)}${
          note ? `<span class="copy-def-note">${escapeHtml(note)}</span>` : ""
        }</dd></div>`
    )
    .join("");
}
