// Wallet Copy view helpers: decision labels, pure formatters and activity rows.

export const SOLANA_ADDRESS_RE = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;
export const LIVE_ARM_CONFIRMATION = "ARM LIVE COPY TRADING";

const SKIP_LABELS = {
  not_buy_swap: "Target activity was not a buy",
  task_disabled: "Task is paused",
  mode_transition_required: "Execution mode must be changed separately",
  live_confirmation_required: "Live execution needs confirmation",
  unsupported_sizing_mode: "Sizing mode is not supported yet",
  self_copy: "Target belongs to this account",
  target_below_minimum: "Target trade is below the minimum",
  target_above_maximum: "Target trade is above the maximum",
  already_bought: "Buy-once limit reached",
  blacklisted: "Token is blocked by risk controls",
  filter_required: "Token did not pass filtering",
  budget_exhausted: "Task budget is exhausted",
  token_cap_reached: "Per-token limit reached",
  below_minimum_size: "Calculated copy size is too small",
  invalid_sizing: "Task sizing is invalid",
  invalid_slippage: "Task slippage is invalid",
  invalid_exit_policy: "Task exit policy is invalid",
  invalid_price: "No usable market price",
  not_sell_swap: "Target activity was not a sell",
  exit_mode_disabled: "Target sell ignored by task exit mode",
  force_stopped: "Trading is force-stopped",
  copy_position_not_found: "No position owned by this copy task",
  position_user_only: "Position is managed by the user",
  position_management_mismatch: "Position ownership no longer permits copy sells",
  latency_kill_switch: "Task auto-paused because target activity arrived too late",
  claim_reconciled_abandoned: "Interrupted live submission was closed without retrying",
  stale_observation: "Replayed after downtime and too old to copy",
};

const ENTRY_BLOCK_LABELS = {
  force_stopped: "Trading is force-stopped",
  loss_limit: "Loss limit blocks new entries",
  connectivity: "Required services are unavailable",
  position_limit: "Open-position limit reached",
  already_open: "A position is already open",
  reentry_cooldown: "Token re-entry cooldown is active",
  open_cooldown: "Global entry cooldown is active",
  entry_reserved: "Another entry is processing",
  blacklisted: "Token is blocked by risk controls",
  check_failed: "A safety check could not complete",
};

export const POLICY_CONTROLS = [
  ["stop-loss", "stop_loss", "threshold_pct"],
  ["roi", "roi", "target_profit_pct"],
  ["trailing", "trailing", "distance_pct"],
  ["time", "time", "duration_seconds"],
];

export const STATE_LABELS = {
  system_paused: "Paused globally",
  force_stopped: "Force stopped",
  paused: "Paused",
  entries_blocked: "Entries blocked",
  live: "Live",
  paper: "Paper",
};

export function number(value) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function signedSol(value) {
  return `${value >= 0 ? "+" : ""}${number(value).toFixed(4)} SOL`;
}

export function formatSizing(sizing) {
  if (sizing?.kind === "ratio_of_target")
    return `${number(sizing.pct).toFixed(1)}% of target trade`;
  return `${number(sizing?.sol).toFixed(3)} SOL fixed`;
}

export function formatExitMode(mode) {
  return (
    {
      buy_only: "Use my exit rules",
      mirror: "Mirror wallet sells",
      hybrid: "Wallet sells + my rules",
    }[mode] || "Use my exit rules"
  );
}

export function formatPolicy(policy, field, unit) {
  if (policy?.enabled === false) return "Disabled for this task";
  if (policy?.enabled === true) return `${number(policy[field]).toFixed(1)}${unit}`;
  return "Use Trader default";
}

export function formatTimePolicy(policy) {
  if (policy?.enabled === false) return "Disabled for this task";
  if (policy?.enabled === true)
    return `${(number(policy.duration_seconds) / 3600).toFixed(1)} hours`;
  return "Use Trader default";
}

export function definitionRows(rows, Utils) {
  return rows
    .map(
      ([label, value]) =>
        `<div><dt>${Utils.escapeHtml(label)}</dt><dd>${Utils.escapeHtml(value)}</dd></div>`
    )
    .join("");
}

function formatActivityTime(value) {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return String(value);
  return date.toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function renderActivityRow(item, Utils) {
  const outcome = item.outcome || {};
  const titles = {
    paper_filled: "Paper fill",
    live_submitted: "Live submitted",
    live_confirmed: "Live confirmed",
    live_failed: "Live failed",
    paper_sell_observed: "Paper sell",
    live_sell_submitted: "Copy sell submitted",
    live_sell_failed: "Copy sell failed",
    skipped: "Skipped",
  };
  const isSkip = outcome.outcome === "skipped";
  const blockKind = outcome.reason?.block?.kind;
  const isSell = outcome.outcome?.includes("sell");
  const exitShare =
    outcome.exit_percentage == null
      ? "Full close"
      : `${number(outcome.exit_percentage).toFixed(1)}% exit`;
  const paperFill = outcome.paper_fill;
  const detail = isSkip
    ? blockKind
      ? ENTRY_BLOCK_LABELS[blockKind] || "Entry blocked"
      : SKIP_LABELS[outcome.reason?.kind] || "Policy skip"
    : outcome.error ||
      (isSell
        ? paperFill
          ? `${exitShare} · ${Utils.formatNumber(paperFill.token_amount)} tokens for ${number(paperFill.net_proceeds_sol).toFixed(4)} SOL`
          : outcome.outcome === "paper_sell_observed"
            ? `Target sold ${outcome.target_token_amount ?? "—"} tokens · observation only`
            : `${exitShare} · target sold ${outcome.target_token_amount ?? "—"} tokens`
        : `${outcome.sized_sol ?? "—"} SOL`);
  const telemetry = outcome.telemetry;
  const arrivalMs =
    telemetry?.target_block_time && telemetry?.detected_at
      ? new Date(telemetry.detected_at).getTime() - Number(telemetry.target_block_time) * 1000
      : null;
  const arrival =
    Number.isFinite(arrivalMs) && arrivalMs >= 0
      ? ` · ${(arrivalMs / 1000).toFixed(1)}s arrival`
      : "";
  const identity = outcome.mint || outcome.signature || "—";
  const timestamp = telemetry?.decided_at || outcome.decided_at || item.created_at;
  return `<div class="wallet-copy-activity-row"><strong>${Utils.escapeHtml(titles[outcome.outcome] || "Decision")}</strong><span class="wallet-copy-activity-detail"><span class="wallet-copy-activity-mint" title="${Utils.escapeHtml(identity)}">${Utils.escapeHtml(identity)}</span><span class="wallet-copy-activity-result">${Utils.escapeHtml(detail + arrival)}</span></span><time class="wallet-copy-activity-time">${Utils.escapeHtml(formatActivityTime(timestamp))}</time></div>`;
}
