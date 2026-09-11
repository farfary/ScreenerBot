// Copy Trading labels and formatters shared by every panel.

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
  target_sell: "Target sell",
  stop_loss: "Stop loss",
  trailing_stop: "Trailing stop",
  take_profit: "Take profit",
  time_override: "Time rule",
  manual: "Closed by hand",
};

const SKIP_LABELS = {
  not_buy_swap: "Target activity was not a buy",
  task_disabled: "Task is paused",
  mode_transition_required: "Execution mode must be changed separately",
  live_confirmation_required: "Live execution needs confirmation",
  unsupported_sizing_mode: "Sizing mode is not supported yet",
  self_copy: "Target is one of your wallets",
  target_below_minimum: "Target trade below the minimum",
  target_above_maximum: "Target trade above the maximum",
  already_bought: "Already bought this token (buy once)",
  blacklisted: "Token is blocked by risk controls",
  filter_required: "Token did not pass Filtering",
  budget_exhausted: "Task budget is spent",
  token_cap_reached: "Per-token limit reached",
  below_minimum_size: "Copy size too small",
  invalid_sizing: "Task sizing is invalid",
  invalid_slippage: "Task slippage is invalid",
  invalid_exit_policy: "Task exit rules are invalid",
  invalid_price: "No usable market price",
  not_sell_swap: "Target activity was not a sell",
  exit_mode_disabled: "Target sell ignored by the exit setting",
  force_stopped: "Trading is force-stopped",
  copy_position_not_found: "No position owned by this task",
  position_user_only: "Position is managed by you",
  position_management_mismatch: "Position no longer follows copy sells",
  latency_kill_switch: "Auto-paused: trades detected too late",
  claim_reconciled_abandoned: "Interrupted live submission closed without retry",
  stale_observation: "Replayed after downtime, too old to copy",
  entry_blocked: "Entry blocked",
};

const ENTRY_BLOCK_LABELS = {
  force_stopped: "Trading is force-stopped",
  loss_limit: "Loss limit blocks new entries",
  connectivity: "Required services are unavailable",
  position_limit: "Open-position limit reached",
  already_open: "A position is already open",
  reentry_cooldown: "Token re-entry cooldown",
  open_cooldown: "Global entry cooldown",
  entry_reserved: "Another entry is processing",
  blacklisted: "Token is blocked by risk controls",
  check_failed: "A safety check could not complete",
};

/** Label for a skip key as the backend groups them (`kind` or `kind.block`). */
export function skipLabel(key) {
  const [kind, block] = String(key || "").split(".");
  if (kind === "entry_blocked" && block) return ENTRY_BLOCK_LABELS[block] || "Entry blocked";
  return SKIP_LABELS[kind] || kind.replace(/_/g, " ");
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
  const number = finite(value);
  return number === null ? "—" : number.toFixed(decimals);
}

export function sol(value, decimals = 4) {
  const number = finite(value);
  return number === null ? "—" : `${number.toFixed(decimals)} SOL`;
}

export function signedSol(value, decimals = 4) {
  const number = finite(value);
  if (number === null) return "—";
  return `${number > 0 ? "+" : number < 0 ? "−" : ""}${Math.abs(number).toFixed(decimals)} SOL`;
}

export function signedPct(value, decimals = 1) {
  const number = finite(value);
  if (number === null) return "—";
  return `${number > 0 ? "+" : number < 0 ? "−" : ""}${Math.abs(number).toFixed(decimals)}%`;
}

export function pct(value, decimals = 1) {
  const number = finite(value);
  return number === null ? "—" : `${number.toFixed(decimals)}%`;
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
  if (number >= 1) return number.toFixed(4);
  const digits = Math.min(12, Math.max(4, Math.ceil(-Math.log10(number)) + 3));
  return number.toFixed(digits);
}

export function seconds(ms) {
  const number = finite(ms);
  if (number === null) return "—";
  return number < 10_000 ? `${(number / 1000).toFixed(1)}s` : `${Math.round(number / 1000)}s`;
}

/** Humanized duration from seconds: 45s, 12m, 3h 5m, 2d 4h. */
export function duration(totalSeconds) {
  const value = finite(totalSeconds);
  if (value === null) return "—";
  const secs = Math.max(0, Math.round(value));
  if (secs < 60) return `${secs}s`;
  const minutes = Math.floor(secs / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return minutes % 60 ? `${hours}h ${minutes % 60}m` : `${hours}h`;
  const days = Math.floor(hours / 24);
  return hours % 24 ? `${days}d ${hours % 24}h` : `${days}d`;
}

export function timeAgo(value) {
  if (!value) return "—";
  const at = new Date(value).getTime();
  if (!Number.isFinite(at)) return "—";
  const delta = (Date.now() - at) / 1000;
  return delta < 5 ? "just now" : `${duration(delta)} ago`;
}

export function dateTime(value) {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return String(value);
  return date.toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function shortAddress(address) {
  if (!address) return "—";
  return address.length > 12 ? `${address.slice(0, 5)}…${address.slice(-4)}` : address;
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
