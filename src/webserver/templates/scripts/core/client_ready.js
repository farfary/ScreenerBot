/**
 * Client Ready Signal
 *
 * Notifies the backend exactly once, per app load, that the dashboard frontend
 * has fully loaded and started — the initial page has rendered and shown its
 * first live data. The backend logs this so boot-to-interactive time is
 * observable and can be used to know the UI is up.
 *
 * Fire-and-forget: a failed report never affects the UI. Safe to call from
 * multiple places (e.g. the landing page's first data render); only the first
 * call in a session actually sends.
 */
import { requestManager } from "./request_manager.js";

let sent = false;

/**
 * Signal that the frontend is fully loaded and started.
 *
 * @param {object} [detail]
 * @param {string} [detail.page] - The active page when readiness was reached.
 */
export function notifyClientReady(detail = {}) {
  if (sent) return;
  sent = true;

  const page = detail.page || "home";
  // Time from navigation start to now, when the Performance API is available.
  let loadMs;
  try {
    const perf = window.performance;
    const nav = perf?.getEntriesByType?.("navigation")?.[0];
    if (nav && Number.isFinite(nav.startTime)) {
      loadMs = Math.max(0, Math.round(perf.now() - nav.startTime));
    } else if (Number.isFinite(perf?.now?.())) {
      loadMs = Math.max(0, Math.round(perf.now()));
    }
  } catch {
    /* performance API unavailable — send without a timing */
  }

  const body = { page };
  if (loadMs !== undefined) body.load_ms = loadMs;

  // Fire-and-forget; swallow all errors so this never disturbs the UI.
  requestManager
    .fetch("/api/system/client-ready", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
      priority: "normal",
    })
    .catch(() => {
      // If the very first attempt fails (e.g. a transient boot hiccup), allow a
      // later caller to try again rather than leaving the backend unaware.
      sent = false;
    });
}
