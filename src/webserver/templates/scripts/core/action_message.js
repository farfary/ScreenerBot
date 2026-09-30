/**
 * What a running or finished action SAYS — the wording, and nothing else.
 *
 * Split out of `action_toasts.js` so the sentences a user reads during a trade
 * can be asserted directly (`tools/tests/action_message.test.mjs`). Everything
 * here is a pure function of one streamed action: no DOM, no state.
 *
 * The backend writes the facts these read into each step's metadata: the router
 * that submitted the swap, and — when a route was refused for what it would
 * have cost outside the trade — which venue was avoided and how much SOL it
 * would have locked. A trade that quietly takes longer because it is re-routing
 * looks broken; saying so is the difference.
 */

import { stepLabel } from "../ui/action_step.js";
import { closeReasonText } from "../ui/trade_reason.js";
import { POOL_PROGRAM_LABELS, ROUTER_LABELS, venueLabel } from "../ui/venue.js";
import { formatNumber, formatSol, withPercentUnit, withSolUnit } from "./format.js";

/**
 * Router display names the step metadata carries instead of an id (`SwapRouter::name`),
 * keyed lower-case, for names that match no venue id.
 */
const ROUTER_NAME_LABELS = Object.freeze({
  "direct pool": "common-venue-direct-pool",
});

/**
 * The catalog label of a router or venue the backend named. A name that matches a known
 * venue id case-insensitively ("Jupiter" -> "jupiter") renders through `venueLabel`;
 * anything else shows as received.
 */
function venueText(name) {
  const key = name.toLowerCase();
  if (Object.hasOwn(ROUTER_LABELS, key) || Object.hasOwn(POOL_PROGRAM_LABELS, key)) {
    return venueLabel(key);
  }
  if (Object.hasOwn(ROUTER_NAME_LABELS, key)) return I18n.label(ROUTER_NAME_LABELS, key);
  return name;
}

/** The backend writes the literal "Unknown" when it could not resolve a symbol. */
export function symbolOf(action) {
  const symbol = action?.metadata?.symbol;
  return symbol && symbol !== "Unknown" ? symbol : "";
}

/** The router that submitted the trade, once the backend has recorded it. */
export function routerOf(action) {
  const steps = Array.isArray(action?.steps) ? action.steps : [];
  const router = steps.find((step) => typeof step?.metadata?.router === "string")?.metadata
    ?.router;
  return router || "";
}

/** Lamports are the backend's unit; SOL is the only one the user thinks in. */
function solFromLamports(lamports) {
  const value = Number(lamports);
  if (!Number.isFinite(value) || value <= 0) return "";
  // Four decimals resolves every rent deposit a venue realistically charges
  // without turning a toast into a number nobody can read.
  return formatSol(value / 1e9, { decimals: 4 });
}

/**
 * The cost-guard refusal recorded against this action, if any.
 *
 * The LAST one wins: a route can be refused more than once, and what the user
 * needs is what is happening now, not the first thing that went wrong.
 */
export function costGuardOf(action) {
  const steps = Array.isArray(action?.steps) ? action.steps : [];
  let latest = null;
  for (const step of steps) {
    const guard = step?.metadata?.cost_guard;
    if (guard && typeof guard === "object") latest = guard;
  }
  if (!latest) return null;
  const venue = typeof latest.venue === "string" ? latest.venue : "";
  const sol = solFromLamports(latest.extra_lamports);
  if (!venue && !sol) return null;
  return { venue, sol };
}

/** "avoiding HumidiFi · 0.0130 SOL" — why this trade is taking another route. */
export function costGuardNote(action) {
  const guard = costGuardOf(action);
  if (!guard) return "";
  const cost = guard.sol;
  if (!guard.venue) {
    return cost
      ? I18n.t("shell-action-cost-guard-avoiding-unnamed-cost", { cost })
      : I18n.t("shell-action-cost-guard-avoiding-unnamed");
  }
  const venue = venueText(guard.venue);
  return cost
    ? I18n.t("shell-action-cost-guard-avoiding-cost", { venue, cost })
    : I18n.t("shell-action-cost-guard-avoiding", { venue });
}

/** "<action> via <router>", or the action alone before a router is recorded. */
function viaRouter(action, router) {
  return router ? I18n.t("shell-action-via-router", { action, router: venueText(router) }) : action;
}

/** "Executing Swap via Jupiter · 3/4" — what the trade is actually doing now. */
export function stepMessage(action) {
  const state = action?.state;
  if (!state || state.status !== "in_progress") return null;

  const step = state.current_step;
  const total = Number(state.total_steps) || 0;
  const index = Number(state.current_step_index) || 0;
  if (!step) return null;

  let label = viaRouter(stepLabel(step), routerOf(action));
  const note = costGuardNote(action);
  if (note) label = I18n.t("shell-action-with-note", { label, note });
  return total > 0
    ? I18n.t("shell-action-step-progress", { label, current: String(index + 1), total: String(total) })
    : label;
}

/** What the trade committed, when the backend recorded it. */
export function outcomeMessage(action) {
  const meta = action?.metadata || {};
  const router = routerOf(action);
  // A trade that dodged a cost is worth saying on the way out too: it explains
  // the route taken, and it is the only place the saving is ever reported.
  const guard = costGuardOf(action);
  const settle = (committed) => {
    const outcome = viaRouter(committed, router);
    if (!guard || !guard.sol) return outcome;
    return guard.venue
      ? I18n.t("shell-action-cost-guard-avoided", { outcome, cost: guard.sol, venue: venueText(guard.venue) })
      : I18n.t("shell-action-cost-guard-avoided-unnamed", { outcome, cost: guard.sol });
  };

  const size = Number(meta.size_sol);
  if (Number.isFinite(size) && size > 0) {
    return settle(withSolUnit(formatNumber(size, { decimals: 0, maxDecimals: 9, useGrouping: false })));
  }

  const percentage = Number(meta.percentage);
  if (Number.isFinite(percentage) && percentage > 0) {
    return settle(
      percentage >= 100
        ? I18n.t("shell-action-exit-full")
        : I18n.t("shell-action-exit-percent", { percent: withPercentUnit(percentage) })
    );
  }

  return typeof meta.reason === "string" && meta.reason ? closeReasonText(meta.reason) : null;
}
