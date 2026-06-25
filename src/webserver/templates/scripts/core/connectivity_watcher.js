// Connectivity watcher — detects when the ScreenerBot backend becomes
// unreachable (process crash, mid-session network loss, restart) and shows a
// single non-blocking "Reconnecting…" overlay instead of letting individual
// pages/pollers fail with bare errors. Auto-recovers: when the backend answers
// again it dismisses the overlay, emits a `screenerbot:reconnected` event so
// the router and pollers can refresh, and shows a brief toast.
//
// Loaded as a module from base.html; self-initializes on import.

const POLL_OK_MS = 5000; // health cadence while online
const POLL_DOWN_MS = 2000; // faster retry cadence while offline
const FAIL_THRESHOLD = 2; // consecutive failures before declaring offline (anti-flap)
const HEALTH_TIMEOUT_MS = 4000;

const state = {
  online: true,
  consecutiveFailures: 0,
  timer: null,
  overlay: null,
  started: false,
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
        <span class="conn-overlay-title">Reconnecting to ScreenerBot…</span>
        <span class="conn-overlay-sub">The backend is unreachable. Trading is paused; this will recover automatically.</span>
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
      if (window.showToast) window.showToast("Reconnected to ScreenerBot", "success");
    } catch { /* toast optional */ }
  } else {
    document.documentElement.setAttribute("data-backend-offline", "true");
    showOverlay();
    window.dispatchEvent(new CustomEvent("screenerbot:offline"));
  }
}

async function checkHealth(force) {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HEALTH_TIMEOUT_MS);
  let ok = false;
  try {
    const res = await fetch("/api/health", {
      method: "GET",
      cache: "no-store",
      signal: controller.signal,
    });
    ok = res.ok;
  } catch {
    ok = false;
  } finally {
    clearTimeout(timeoutId);
  }

  if (ok) {
    state.consecutiveFailures = 0;
    setOnline(true);
  } else {
    state.consecutiveFailures += 1;
    if (state.consecutiveFailures >= FAIL_THRESHOLD) setOnline(false);
  }

  // Reschedule at the cadence matching the current state.
  if (state.started || force) schedule();
}

function schedule() {
  if (state.timer) clearTimeout(state.timer);
  const delay = state.online ? POLL_OK_MS : POLL_DOWN_MS;
  state.timer = setTimeout(() => checkHealth(false), delay);
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
