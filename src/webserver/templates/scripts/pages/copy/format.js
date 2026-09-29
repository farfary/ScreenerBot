// Copy Trading labels and formatters shared by every panel. The formatters
// compose the functions of core/format.js with this feature's precision.

import {
  formatAddressCompact,
  formatFixed,
  formatPercentValue,
  formatPriceSol,
  formatSol,
  formatTimeAgo,
  formatTimeSpan,
  formatTimestamp,
  formatUptime,
} from "../../core/format.js";

export const SOLANA_ADDRESS_RE = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

export const STATE_LABELS = {
  system_paused: "Paused globally",
  force_stopped: "Force stopped",
  paused: "Paused",
  entries_blocked: "Entries blocked",
  live: "Running",
  paper: "Running",
};

export const MODE_LABELS = { paper: "Paper", live: "Live" };

export const EXIT_MODE_LABELS = {
  buy_only: "My exit rules",
  mirror: "Mirror wallet sells",
  hybrid: "Wallet sells and my rules",
};

export const EXIT_LABELS = {
  target_sell: "Wallet sold",
  stop_loss: "Stop loss",
  trailing_stop: "Trailing stop",
  take_profit: "Take profit",
  time_override: "Time rule",
  manual: "Closed by hand",
};

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

export function pauseReasonText(reason) {
  switch (reason?.kind) {
    case "user":
      return "Paused by you";
    case "latency_kill_switch":
      return `Auto-paused: trades arrived ${seconds(reason.average_ms)} late on average (limit ${seconds(reason.threshold_ms)})`;
    case "watch_detached":
      return "Auto-paused: the wallet is no longer watched";
    case "watch_budget_exceeded":
      return `Paused: this wallet reached its ${(Number(reason.page_budget) || 5) * 100}-signature watch check limit before catching up`;
    case "helius_unavailable":
      return "Paused: Helius wallet checks failed";
    case "watch_processing_failed":
      return "Paused: wallet activity could not be processed";
    default:
      return "Paused";
  }
}

export function pauseReasonShort(reason) {
  switch (reason?.kind) {
    case "latency_kill_switch":
      return "too slow";
    case "watch_detached":
      return "watch lost";
    case "watch_budget_exceeded":
      return "watch limit";
    case "helius_unavailable":
      return "watch provider";
    case "watch_processing_failed":
      return "watch processing";
    case "user":
      return "by you";
    default:
      return "";
  }
}

/** "1 task", "3 tasks": a count with its noun in agreement. */
export function plural(count, one, many = `${one}s`) {
  return `${count} ${Number(count) === 1 ? one : many}`;
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
  const number = finite(value);
  if (number === null) return "—";
  return `${changeSign(number)}${formatSol(Math.abs(number), { decimals })}`;
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
        ? `${plural(priced, "priced holding")} · ${unpriced} without a price`
        : `${plural(unpriced, "holding")} without a price`,
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
  { id: "24h", label: "24h", hours: 24 },
  { id: "7d", label: "7d", hours: 24 * 7 },
  { id: "30d", label: "30d", hours: 24 * 30 },
  { id: "all", label: "All", hours: null },
];

export function rangeQuery(rangeId) {
  const range = RANGES.find((item) => item.id === rangeId);
  if (!range?.hours) return {};
  return { from: new Date(Date.now() - range.hours * 3_600_000).toISOString() };
}

/** A `.copy-seg` segmented choice: native radios, one painted segment each. */
export function segmented(name, options, value, escapeHtml, ariaLabel = name) {
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
