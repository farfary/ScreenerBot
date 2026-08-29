/**
 * External-agent approval prompts.
 *
 * When an MCP-paired agent calls a tool that resolves to "requires approval",
 * ScreenerBot parks a durable request on the agent-control approval queue. This
 * module is the minimal integrated surface for acting on those: it polls the
 * pending queue and raises the shared confirmation dialog for each new request,
 * so a person approves or denies it inside ScreenerBot. The external caller can
 * never approve its own request — it has no route to the decision endpoint.
 *
 * This is deliberately small; the richer Agent Connections management UI is a
 * later checkpoint. It reuses the canonical Poller and ConfirmationDialog and
 * does NOT touch the Assistant chat's own tool-confirmation flow (a separate
 * transport with its own `/api/assistant/chat/confirm` path).
 *
 * Loaded as a side-effect module from base.html. The DOM-coupled bootstrap only
 * runs in a browser; the pure helpers below are unit-tested under node.
 */

export const PENDING_URL = "/api/agent-control/approvals";
export const decideUrl = (id) => `${PENDING_URL}/${encodeURIComponent(id)}/decide`;

export const POLL_INTERVAL_MS = 4000;
/** Cap on remembered ids, so the set cannot grow without bound. */
export const HANDLED_LIMIT = 500;
/** After a failed decision POST, wait this long before offering it again. */
export const DEFER_MS = 12_000;

/** "expires in 4m" / "expires in 25s" for the dialog body. */
export function expiryText(expiresAt, nowMs = Date.now()) {
  const secs = Math.max(0, Math.round(Number(expiresAt) - nowMs / 1000));
  if (secs >= 90) return `expires in ${Math.round(secs / 60)}m`;
  return `expires in ${secs}s`;
}

/**
 * Whether a decision POST reached a terminal outcome. `ok` means the decision
 * was recorded; `409` means it was already resolved/expired elsewhere. Anything
 * else (network error, 5xx) is transient — the request is still pending and
 * must be offered again later.
 */
export function decisionSettled(res) {
  return Boolean(res) && (res.ok === true || res.status === 409);
}

/**
 * From the server's pending list, the rows we should newly enqueue: a valid id
 * we have not already handled, that is not already queued, and that is not in
 * post-failure backoff.
 */
export function selectNewPending(rows, { handled, queued, deferredUntil, nowMs = Date.now() }) {
  if (!Array.isArray(rows)) return [];
  return rows.filter((row) => {
    if (!row || typeof row.id !== "string") return false;
    if (handled.has(row.id) || queued.has(row.id)) return false;
    const until = deferredUntil.get(row.id);
    return !(until && nowMs < until);
  });
}

function isBrowser() {
  return typeof window !== "undefined" && typeof document !== "undefined";
}

if (isBrowser()) {
  // Dynamic import so a node `import` of this module for testing never pulls in
  // the DOM-coupled Poller / ConfirmationDialog dependency graphs.
  Promise.all([import("./poller.js"), import("../ui/confirmation_dialog.js")])
    .then(([{ Poller }, { ConfirmationDialog }]) => {
      const handled = new Set();
      const queued = new Set();
      const deferredUntil = new Map();
      const queue = [];
      let draining = false;

      function rememberHandled(id) {
        handled.add(id);
        queued.delete(id);
        deferredUntil.delete(id);
        if (handled.size > HANDLED_LIMIT) {
          handled.delete(handled.values().next().value);
        }
      }

      async function postDecision(id, approve) {
        try {
          const res = await fetch(decideUrl(id), {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ approve }),
          });
          return decisionSettled(res);
        } catch {
          return false;
        }
      }

      async function drain() {
        if (draining) return;
        draining = true;
        try {
          while (queue.length) {
            // Never tear down a confirmation dialog another surface owns —
            // `ConfirmationDialog.show` would destroy it. Leave our items
            // queued; the next poll tick re-enters drain().
            if (ConfirmationDialog.activeDialog) break;

            const item = queue[0];
            if (handled.has(item.id)) {
              queue.shift();
              queued.delete(item.id);
              continue;
            }

            const summary =
              typeof item.args_summary === "string" && item.args_summary.length
                ? ` Arguments: ${item.args_summary}.`
                : "";
            const { confirmed } = await ConfirmationDialog.show({
              title: "Agent request",
              message:
                `${item.client_label || "A paired agent"} wants to run "${item.tool}" ` +
                `in ScreenerBot.${summary} This request ${expiryText(item.expires_at)}.`,
              confirmLabel: "Approve",
              cancelLabel: "Deny",
              variant: "warning",
            });

            // Dismissing the dialog (Escape / backdrop) resolves as not
            // confirmed and counts as Deny — the safe direction.
            const settled = await postDecision(item.id, confirmed === true);
            queue.shift();
            queued.delete(item.id);
            if (settled) {
              rememberHandled(item.id);
            } else {
              // Transient failure: offer it again after a short backoff rather
              // than silently dropping a still-pending request.
              deferredUntil.set(item.id, Date.now() + DEFER_MS);
            }
          }
        } finally {
          draining = false;
        }
      }

      async function poll() {
        const res = await fetch(PENDING_URL, { headers: { Accept: "application/json" } });
        if (!res.ok) return; // not initialized yet / transient — retry next tick
        const rows = await res.json();
        for (const row of selectNewPending(rows, { handled, queued, deferredUntil })) {
          queued.add(row.id);
          queue.push(row);
        }
        if (queue.length) void drain();
      }

      const poller = new Poller(poll, {
        label: "AgentApprovals",
        intervalMs: POLL_INTERVAL_MS,
        pauseWhenHidden: true,
      });
      poller.start({ silent: true });
      window.addEventListener("pagehide", () => poller.cleanup(), { once: true });
    })
    .catch(() => {
      /* dashboard chrome not present in this context */
    });
}
