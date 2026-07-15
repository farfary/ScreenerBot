// Connectivity watcher — detects when the ScreenerBot backend becomes
// unreachable (process crash, mid-session network loss, restart) and shows a
// single non-blocking "Waiting for core…" overlay instead of letting individual
// pages/pollers fail with bare errors. Auto-recovers: when the backend answers
// again it dismisses the overlay, emits a `screenerbot:reconnected` event so
// the router and pollers can refresh, and shows a brief toast.
//
// Detection is driven by TWO signals so it reacts within ~1 request instead of
// waiting out a poll interval:
//   1. Real traffic — we wrap `window.fetch` and observe every request the app
//      makes (pollers fire several per second). A failed request to our own
//      origin trips an immediate health re-check; a successful one clears the
//      failure streak. This makes a dead backend visible almost instantly.
//   2. A periodic `/api/health` poll as the backstop (fast cadence while down).
//
// Loaded as a module from base.html; self-initializes on import.

const POLL_OK_MS = 5000; // health backstop cadence while online
const POLL_DOWN_MS = 1000; // fast retry cadence while offline
const FAIL_THRESHOLD = 2; // consecutive failures before declaring offline (anti-flap)
const HEALTH_TIMEOUT_MS = 3000;

const state = {
  online: true,
  consecutiveFailures: 0,
  timer: null,
  overlay: null,
  started: false,
  probeInFlight: false,
};

function buildOverlay() {
  const overlay = document.createElement("div");
  overlay.className = "conn-overlay";
  overlay.setAttribute("role", "status");
  overlay.setAttribute("aria-live", "polite");
  overlay.innerHTML = `
    <div class="conn-overlay-card">
      <span class="conn-overlay-spinner" aria-hidden="true"></span>
      <div class="conn-overlay-text">
        <span class="conn-overlay-title">Waiting for core…</span>
        <span class="conn-overlay-sub">The core is unreachable. Trading is paused; this will recover automatically.</span>
      </div>
      <button type="button" class="conn-overlay-retry">Retry now</button>
    </div>`;
  overlay
    .querySelector(".conn-overlay-retry")
    .addEventListener("click", () => checkHealth(true));
  return overlay;
}

function showOverlay() {
  if (!state.overlay) state.overlay = buildOverlay();
  if (!state.overlay.isConnected) document.body.appendChild(state.overlay);
  // Force reflow so the enter transition runs even on first append.
  void state.overlay.offsetWidth;
  state.overlay.classList.add("is-visible");
}

function hideOverlay() {
  if (state.overlay) state.overlay.classList.remove("is-visible");
}

function setOnline(isOnline) {
  if (isOnline === state.online) return;
  state.online = isOnline;
  if (isOnline) {
    hideOverlay();
    document.documentElement.removeAttribute("data-backend-offline");
    window.dispatchEvent(new CustomEvent("screenerbot:reconnected"));
    try {
      if (window.showToast) window.showToast("Core connection restored", "success");
    } catch { /* toast optional */ }
  } else {
    document.documentElement.setAttribute("data-backend-offline", "true");
    showOverlay();
    window.dispatchEvent(new CustomEvent("screenerbot:offline"));
  }
}

// Record a successful round-trip to our backend (health poll OR observed app
// request): clears the failure streak and recovers immediately if we were down.
function markSuccess() {
  state.consecutiveFailures = 0;
  if (!state.online) setOnline(true);
}

// Record a failed round-trip. Trips offline once the streak crosses the
// threshold (anti-flap). Shared by the health poll and observed traffic so a
// couple of failed requests are enough — no waiting for a poll tick.
function markFailure() {
  state.consecutiveFailures += 1;
  if (state.consecutiveFailures >= FAIL_THRESHOLD) setOnline(false);
}

async function checkHealth(force) {
  // Coalesce concurrent probes — observed-failure bursts (e.g. several dialog
  // requests timing out at once) must not each spawn their own health probe.
  if (state.probeInFlight) return;
  state.probeInFlight = true;
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HEALTH_TIMEOUT_MS);
  let ok = false;
  try {
    const res = await rawFetch("/api/health", {
      method: "GET",
      cache: "no-store",
      signal: controller.signal,
    });
    ok = res.ok;
  } catch {
    ok = false;
  } finally {
    clearTimeout(timeoutId);
    state.probeInFlight = false;
  }

  if (ok) markSuccess();
  else markFailure();

  // Reschedule at the cadence matching the current state.
  if (state.started || force) schedule();
}

function schedule() {
  if (state.timer) clearTimeout(state.timer);
  const delay = state.online ? POLL_OK_MS : POLL_DOWN_MS;
  state.timer = setTimeout(() => checkHealth(false), delay);
}

// The original fetch, captured before we wrap it, so our own health probe and
// the recursion-free observation path don't re-enter the wrapper.
let rawFetch = window.fetch.bind(window);

// Wrap window.fetch to observe real request outcomes. Same-origin failures are
// strong evidence the backend is unreachable, so they feed the failure streak
// directly; successes clear it. Cross-origin requests (e.g. external APIs) are
// ignored so a flaky third-party host never trips our backend overlay.
function instrumentFetch() {
  const wrapped = async function (input, init) {
    let isSameOrigin = true;
    try {
      const url = typeof input === "string" ? input : input && input.url;
      if (url) isSameOrigin = new URL(url, window.location.href).origin === window.location.origin;
    } catch { /* treat unparseable URLs as same-origin */ }

    try {
      const res = await rawFetch(input, init);
      if (isSameOrigin) markSuccess();
      return res;
    } catch (err) {
      // Ignore deliberate aborts (cancelled polls / navigation) — they are not
      // connectivity failures.
      if (isSameOrigin && err && err.name !== "AbortError") {
        // A single observed request failure is NOT proof the backend is down —
        // heavy views (the token dialog fires ~10 parallel requests, incl. 7
        // OHLCV probes) can have an individual request time out while the server
        // is perfectly healthy. So instead of counting this toward the offline
        // streak directly (which flipped the "Waiting for core" overlay on 2 such
        // blips), confirm authoritatively against /api/health. Only the probe
        // decides offline — a genuinely dead core fails it too, so detection
        // stays fast.
        if (!state.probeInFlight) checkHealth(true);
      }
      throw err;
    }
  };
  // Preserve the token-injecting wrapper installed in base.html by delegating to
  // the current window.fetch (captured as rawFetch above).
  window.fetch = wrapped;
}

export function isBackendOnline() {
  return state.online;
}

export function pingNow() {
  return checkHealth(true);
}

export function initConnectivityWatcher() {
  if (state.started) return;
  state.started = true;

  instrumentFetch();

  // Browser-level offline events give us an instant signal; still verify via
  // the health endpoint since the OS being online doesn't mean our backend is.
  window.addEventListener("offline", () => {
    state.consecutiveFailures = FAIL_THRESHOLD;
    setOnline(false);
  });
  window.addEventListener("online", () => checkHealth(true));

  schedule();
}

window.__SB_CONNECTIVITY__ = {
  isBackendOnline,
  pingNow,
};

initConnectivityWatcher();
