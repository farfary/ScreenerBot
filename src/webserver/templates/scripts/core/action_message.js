/**
 * What a running or finished action SAYS — the wording, and nothing else.
 *
 * Split out of `action_toasts.js` so the sentences a user reads during a trade
 * can be asserted directly (`tools/tests/action_message.test.mjs`). Everything
 * here is a pure function of one streamed action: no DOM, no imports, no state.
 *
 * The backend writes the facts these read into each step's metadata: the router
 * that submitted the swap, and — when a route was refused for what it would
 * have cost outside the trade — which venue was avoided and how much SOL it
 * would have locked. A trade that quietly takes longer because it is re-routing
 * looks broken; saying so is the difference.
 */

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
  return `${Number(value / 1e9).toFixed(4)} SOL`;
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
  const venue = guard.venue || "a venue";
  return guard.sol ? `avoiding ${venue} · ${guard.sol}` : `avoiding ${venue}`;
}

/** "Executing Swap via Jupiter · 3/4" — what the trade is actually doing now. */
export function stepMessage(action) {
  const state = action?.state;
  if (!state || state.status !== "in_progress") return null;

  const step = state.current_step;
  const total = Number(state.total_steps) || 0;
  const index = Number(state.current_step_index) || 0;
  if (!step) return null;

  const router = routerOf(action);
  let label = router ? `${step} via ${router}` : step;
  const note = costGuardNote(action);
  if (note) label = `${label} · ${note}`;
  return total > 0 ? `${label} · ${index + 1}/${total}` : label;
}

/** What the trade committed, when the backend recorded it. */
export function outcomeMessage(action) {
  const meta = action?.metadata || {};
  const router = routerOf(action);
  const via = router ? ` via ${router}` : "";
  // A trade that dodged a cost is worth saying on the way out too: it explains
  // the route taken, and it is the only place the saving is ever reported.
  const guard = costGuardOf(action);
  const avoided = guard && guard.sol ? ` · avoided ${guard.sol} in ${guard.venue || "venue"} rent` : "";

  const size = Number(meta.size_sol);
  if (Number.isFinite(size) && size > 0) return `${size} SOL${via}${avoided}`;

  const percentage = Number(meta.percentage);
  if (Number.isFinite(percentage) && percentage > 0) {
    return `${percentage >= 100 ? "Full exit" : `${percentage}% exit`}${via}${avoided}`;
  }

  return typeof meta.reason === "string" && meta.reason ? meta.reason : null;
}
