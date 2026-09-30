// Copy Trading labels and formatters shared by every panel. The formatters
// compose the functions of core/format.js with this feature's precision.

import {
  formatAddressCompact,
  formatFixed,
  formatPercentValue,
  formatPriceSol,
  formatSignedSol,
  formatSol,
  formatTimeAgo,
  formatTimeSpan,
  formatTimestamp,
  formatUptime,
  withSolUnit,
} from "../../core/format.js";
import { closeReasonText } from "../../ui/trade_reason.js";

export const SOLANA_ADDRESS_RE = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

// Task states computed by `effective_state` (src/trader/copy/control.rs).
const STATE_LABELS = Object.freeze({
  system_paused: "copy-state-system-paused",
  force_stopped: "copy-state-force-stopped",
  paused: "copy-state-paused",
  entries_blocked: "copy-state-entries-blocked",
  live: "copy-state-running-live",
  paper: "copy-state-running-paper",
});

// Execution modes serialized by `CopyMode` (src/trader/copy/types.rs).
const MODE_LABELS = Object.freeze({
  paper: "copy-mode-paper",
  live: "copy-mode-live",
});

// Exit modes serialized by `ExitMode` (src/trader/copy/types.rs).
const EXIT_MODE_LABELS = Object.freeze({
  buy_only: "copy-exit-mode-buy-only",
  mirror: "copy-exit-mode-mirror",
  hybrid: "copy-exit-mode-hybrid",
});

// What closed a round: `target_sell` or a `PaperExitRule` (src/trader/copy/insights.rs).
const EXIT_LABELS = Object.freeze({
  target_sell: "copy-exit-target-sell",
  stop_loss: "copy-exit-stop-loss",
  trailing_stop: "copy-exit-trailing-stop",
  take_profit: "copy-exit-take-profit",
  time_override: "copy-exit-time-override",
  manual: "copy-exit-manual",
});

export const stateLabel = (state) =>
  Object.hasOwn(STATE_LABELS, state) ? I18n.label(STATE_LABELS, state) : I18n.t("format-unknown");

export const modeLabel = (mode) => I18n.label(MODE_LABELS, mode);

export const exitModeLabel = (mode) => I18n.label(EXIT_MODE_LABELS, mode);

/** A closed live round carries the trade's close reason instead of a copy exit id. */
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
  entry_blocked: "copy-skip-entry-blocked",
});

// Block ids serialized by `EntryBlock` (src/trader/admission.rs).
const ENTRY_BLOCK_LABELS = Object.freeze({
  force_stopped: "copy-entry-block-force-stopped",
  loss_limit: "copy-entry-block-loss-limit",
  connectivity: "copy-entry-block-connectivity",
  position_limit: "copy-entry-block-position-limit",
  already_open: "copy-entry-block-already-open",
  reentry_cooldown: "copy-entry-block-reentry-cooldown",
  open_cooldown: "copy-entry-block-open-cooldown",
  entry_reserved: "copy-entry-block-entry-reserved",
  blacklisted: "copy-entry-block-blacklisted",
  check_failed: "copy-entry-block-check-failed",
});

/** Label for a skip key as the backend groups them (`kind` or `kind.block`). */
export function skipLabel(key) {
  const [kind, block] = String(key || "").split(".");
  if (kind === "entry_blocked" && block && Object.hasOwn(ENTRY_BLOCK_LABELS, block)) {
    return I18n.label(ENTRY_BLOCK_LABELS, block);
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
});

/** Short cause of a pause for the task list; empty when the kind has none. */
export function pauseReasonShort(reason) {
  return Object.hasOwn(PAUSE_SHORT_LABELS, reason?.kind)
    ? I18n.label(PAUSE_SHORT_LABELS, reason.kind)
    : "";
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

/** Sign glyph of a change: a typographic minus so it aligns with the plus. */
const changeSign = (number) => (number > 0 ? "+" : number < 0 ? "−" : "");

export function signedSol(value, decimals = 4) {
  return formatSignedSol(value, { decimals, fallback: "—" });
}

export function signedPct(value, decimals = 1) {
  const number = finite(value);
  if (number === null) return "—";
  return `${changeSign(number)}${formatPercentValue(Math.abs(number), { decimals, includeSign: false })}`;
}

export function pct(value, decimals = 1) {
  return formatPercentValue(value, { decimals, plus: "", fallback: "—" });
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

export function toneClass(value) {
  const number = finite(value);
  if (number === null || number === 0) return "";
  return number > 0 ? "is-positive" : "is-negative";
}

/** A pool price in SOL, with enough significant digits for micro-priced tokens. */
export function price(value) {
  const number = finite(value);
  if (number === null) return "—";
  if (number === 0) return "0";
  const decimals = number >= 1 ? 4 : Math.min(12, Math.max(4, Math.ceil(-Math.log10(number)) + 3));
  return formatPriceSol(number, { decimals });
}

/** A pool price with its SOL unit, for tooltips and messages. */
export function priceSol(value) {
  return withSolUnit(price(value));
}

export function seconds(ms) {
  const number = finite(ms);
  if (number === null) return "—";
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

export function shortAddress(address) {
  return formatAddressCompact(address, { start: 5, end: 4 });
}

export function taskName(task) {
  return task?.label || shortAddress(task?.target_address);
}

/** Range presets for analytics, as `from` timestamps. */
export const RANGES = [
  {
    id: "24h",
    hours: 24,
    get label() {
      return I18n.t("copy-range-24h");
    },
  },
  {
    id: "7d",
    hours: 24 * 7,
    get label() {
      return I18n.t("copy-range-7d");
    },
  },
  {
    id: "30d",
    hours: 24 * 30,
    get label() {
      return I18n.t("copy-range-30d");
    },
  },
  {
    id: "all",
    hours: null,
    get label() {
      return I18n.t("copy-range-all");
    },
  },
];

export function rangeQuery(rangeId) {
  const range = RANGES.find((item) => item.id === rangeId);
  if (!range?.hours) return {};
  return { from: new Date(Date.now() - range.hours * 3_600_000).toISOString() };
}

/** A `.copy-seg` segmented choice: native radios, one painted segment each. */
export function segmented(name, options, value, escapeHtml, ariaLabel) {
  const group = escapeHtml(name);
  return `<div class="copy-seg" role="radiogroup" aria-label="${escapeHtml(ariaLabel)}" data-seg="${group}">${options
    .map((option) => {
      const id = escapeHtml(option.id);
      const checked = option.id === value;
      return `<label class="copy-seg-btn${checked ? " is-active" : ""}"><input type="radio" name="copy-seg-${group}" value="${id}" data-seg-value="${id}"${checked ? " checked" : ""} /><span>${escapeHtml(option.label)}</span></label>`;
    })
    .join("")}</div>`;
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
