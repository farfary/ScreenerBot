/**
 * Action notices — the ONE place that turns backend actions into toasts.
 *
 * Every trade the backend runs (manual or automatic) is an `Action` streamed
 * over `/api/actions/stream` with a step-by-step lifecycle. Previously the
 * header raised a separate toast on `added` and another on `updated`, and the
 * manual-trade flow raised a third of its own when the POST returned, so one
 * swap produced three stacked toasts saying the same thing.
 *
 * The rule now:
 *   - A trade the USER started gets ONE toast, keyed by the token, created when
 *     the action starts and UPDATED in place through its steps until it
 *     succeeds or fails.
 *   - A trade the BOT started gets no live toast — it is not work the user is
 *     waiting on — only a single result notice when it resolves.
 *   - Everything, either way, is recorded in the notification center; the toast
 *     is only the transient nudge.
 *
 * Loaded as a side-effect module from base.html.
 */

import { notificationManager } from "./notifications.js";
import { toastManager } from "./toast.js";

/** Actions whose terminal state has already been announced. */
const resolved = new Set();
const RESOLVED_LIMIT = 200;

const SUBJECTS = {
  swap_buy: { live: "Buying", done: "Bought", failed: "Buy failed" },
  swap_sell: { live: "Selling", done: "Sold", failed: "Sell failed" },
  position_open: { live: "Opening position", done: "Opened", failed: "Open failed" },
  position_close: { live: "Closing position", done: "Closed", failed: "Close failed" },
  position_dca: { live: "Adding to position", done: "Added to", failed: "Add failed" },
  position_partial_exit: {
    live: "Partial exit",
    done: "Partial exit",
    failed: "Partial exit failed",
  },
  manual_order: { live: "Placing order", done: "Order placed", failed: "Order failed" },
};

const FALLBACK_SUBJECT = { live: "Trade", done: "Trade done", failed: "Trade failed" };

/** The backend writes the literal "Unknown" when it could not resolve a symbol. */
function symbolOf(action) {
  const symbol = action?.metadata?.symbol;
  return symbol && symbol !== "Unknown" ? symbol : "";
}

/** A trade the user asked for, as opposed to one the auto-trader decided on. */
function isUserInitiated(action) {
  return String(action?.metadata?.operation || "").startsWith("manual");
}

function subjectOf(action) {
  return SUBJECTS[action?.action_type] || FALLBACK_SUBJECT;
}

function titleFor(action, phase) {
  const symbol = symbolOf(action);
  const label = subjectOf(action)[phase];
  return symbol ? `${label} ${symbol}` : label;
}

/** "Executing Swap · 3/4" — what the trade is actually doing right now. */
function stepMessage(action) {
  const state = action?.state;
  if (!state || state.status !== "in_progress") return null;

  const step = state.current_step;
  const total = Number(state.total_steps) || 0;
  const index = Number(state.current_step_index) || 0;
  if (!step) return null;

  return total > 0 ? `${step} · ${index + 1}/${total}` : step;
}

/** What the trade committed, when the backend recorded it. */
function outcomeMessage(action) {
  const meta = action?.metadata || {};
  const size = Number(meta.size_sol);
  if (Number.isFinite(size) && size > 0) return `${size} SOL`;

  const percentage = Number(meta.percentage);
  if (Number.isFinite(percentage) && percentage > 0) {
    return percentage >= 100 ? "Full exit" : `${percentage}% exit`;
  }

  return typeof meta.reason === "string" && meta.reason ? meta.reason : null;
}

function markResolved(actionId) {
  resolved.add(actionId);
  if (resolved.size > RESOLVED_LIMIT) {
    resolved.delete(resolved.values().next().value);
  }
}

/**
 * Keyed by the TOKEN, not the action: `ui/manual_trade.js` reports a rejected
 * request under the same key, so a failure that both the HTTP response and the
 * action stream know about lands as ONE toast instead of two. The backend
 * allows only one trade in flight per mint, so the token key cannot collide
 * with a second live trade.
 */
export function tradeToastKey(mint) {
  return `trade:${mint}`;
}

function keyFor(action) {
  return tradeToastKey(action.entity_id || action.id);
}

function showLive(action) {
  toastManager.show({
    key: keyFor(action),
    type: "progress",
    title: titleFor(action, "live"),
    message: stepMessage(action),
    progress: Number(action?.state?.progress_pct) || 0,
  });
}

function showResolved(action, status) {
  markResolved(action.id);

  if (status === "completed") {
    toastManager.show({
      key: keyFor(action),
      type: "success",
      title: titleFor(action, "done"),
      message: outcomeMessage(action),
    });
    return;
  }

  if (status === "failed") {
    toastManager.show({
      key: keyFor(action),
      type: "error",
      title: titleFor(action, "failed"),
      message: action?.state?.error || null,
    });
    return;
  }

  // Cancelled: the user (or a shutdown) stopped it — an outcome, not a fault.
  toastManager.show({
    key: keyFor(action),
    type: "info",
    title: `${titleFor(action, "live")} cancelled`,
  });
}

function handle(event) {
  const action = event?.notification;
  if (!action?.id) return;

  const status = notificationManager.getStatus(action);

  if (status === "in_progress") {
    // The bot's own trades are not work the user is waiting on: no live toast.
    if (isUserInitiated(action) && !resolved.has(action.id)) showLive(action);
    return;
  }

  if (!status || resolved.has(action.id)) return;
  showResolved(action, status);
}

notificationManager.subscribe((event) => {
  if (event?.type === "added" || event?.type === "updated") handle(event);
});
