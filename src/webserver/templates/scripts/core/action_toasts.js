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
import { apiErrorMessage } from "./request_manager.js";
// The wording lives in its own module so it can be tested without a DOM.
import { outcomeMessage, stepMessage, symbolOf } from "./action_message.js";

/** Actions whose terminal state has already been announced. */
const resolved = new Set();
const RESOLVED_LIMIT = 200;

// Toast wording per action type and phase, as message ids.
const SUBJECT_LABELS = Object.freeze({
  swap_buy: Object.freeze({
    live: "shell-action-swap-buy-live",
    done: "shell-action-swap-buy-done",
    failed: "shell-action-swap-buy-failed",
  }),
  swap_sell: Object.freeze({
    live: "shell-action-swap-sell-live",
    done: "shell-action-swap-sell-done",
    failed: "shell-action-swap-sell-failed",
  }),
  position_open: Object.freeze({
    live: "shell-action-position-open-live",
    done: "shell-action-position-open-done",
    failed: "shell-action-position-open-failed",
  }),
  position_close: Object.freeze({
    live: "shell-action-position-close-live",
    done: "shell-action-position-close-done",
    failed: "shell-action-position-close-failed",
  }),
  position_dca: Object.freeze({
    live: "shell-action-position-dca-live",
    done: "shell-action-position-dca-done",
    failed: "shell-action-position-dca-failed",
  }),
  position_partial_exit: Object.freeze({
    live: "shell-action-partial-exit-live",
    done: "shell-action-partial-exit-done",
    failed: "shell-action-partial-exit-failed",
  }),
  manual_order: Object.freeze({
    live: "shell-action-manual-order-live",
    done: "shell-action-manual-order-done",
    failed: "shell-action-manual-order-failed",
  }),
});

const FALLBACK_SUBJECT_LABELS = Object.freeze({
  live: "shell-action-trade-live",
  done: "shell-action-trade-done",
  failed: "shell-action-trade-failed",
});

/** A trade the user asked for, as opposed to one the auto-trader decided on. */
function isUserInitiated(action) {
  return String(action?.metadata?.operation || "").startsWith("manual");
}

function subjectOf(action) {
  return SUBJECT_LABELS[action?.action_type] || FALLBACK_SUBJECT_LABELS;
}

function titleFor(action, phase) {
  const symbol = symbolOf(action);
  const label = I18n.label(subjectOf(action), phase);
  return symbol ? I18n.t("shell-action-title-symbol", { label, symbol }) : label;
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
  if (status === "completed" || status === "failed") {
    window.dispatchEvent(new CustomEvent("screenerbot:trade-settled"));
  }

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
      message: apiErrorMessage({ error: action?.state?.error }, null) || null,
    });
    return;
  }

  // Cancelled: the user (or a shutdown) stopped it — an outcome, not a fault.
  toastManager.show({
    key: keyFor(action),
    type: "info",
    title: I18n.t("shell-action-cancelled", { title: titleFor(action, "live") }),
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
