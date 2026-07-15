/**
 * Token Details Dialog - State Handling Mixin
 *
 * Centralizes the dialog's loading / fetching / loaded / no-data / error /
 * connection-drop behavior so every tab handles those states the same way and,
 * critically, so repeated polls never re-paint unchanged content.
 *
 * The fundamental primitive is `_renderHtmlIfChanged`: it only writes to the DOM
 * when the freshly produced HTML differs from what is already there. Because a
 * tab that is still "loading" produces byte-for-byte identical HTML on every
 * poll, the body is left untouched and its one-shot entry animations never
 * restart — fixing the "everything animates over and over" flicker — while a
 * genuine data change still repaints exactly once.
 */

const CONNECTION_TEXT = {
  online: "",
  reconnecting: "Waiting for core…",
  offline: "Waiting for core…",
};

function backendLooksOffline() {
  if (navigator.onLine === false) return true;
  if (document.documentElement.hasAttribute("data-backend-offline")) return true;
  try {
    return window.__SB_CONNECTIVITY__?.isBackendOnline?.() === false;
  } catch {
    return false;
  }
}

export function applyStateHandlingMixin(DialogClass) {
  const proto = DialogClass.prototype;

  /**
   * Replace an element's innerHTML ONLY when the new HTML differs from the last
   * HTML rendered into it. The last HTML is cached on the element under `key` so
   * the comparison is O(string length) and survives across polls. Returns true
   * when a repaint actually happened (so callers can run post-render wiring).
   * @param {HTMLElement} el - target container
   * @param {string} html - candidate markup
   * @param {string} [key] - expando cache key (lets one element track sections)
   * @returns {boolean} whether the DOM was updated
   */
  proto._renderHtmlIfChanged = function (el, html, key = "__sbHtml") {
    if (!el) return false;
    if (el[key] === html) return false;
    el.innerHTML = html;
    el[key] = html;
    return true;
  };

  /**
   * Record the outcome of a token-data poll and drive the connection indicator.
   * Kept separate from the fetch logic so the rules live in one place:
   * - success resets the failure streak and clears any reconnect chip;
   * - a post-initial-load failure is treated as a transient connection blip:
   *   the last good data stays on screen and only after a couple of misses does
   *   a subtle chip appear (offline vs reconnecting based on `navigator.onLine`).
   * @param {boolean} ok - true if the poll succeeded
   */
  proto._recordPollOutcome = function (ok) {
    if (ok) {
      this._consecutiveFailures = 0;
      this._setConnectionState("online");
      return;
    }
    this._consecutiveFailures = (this._consecutiveFailures || 0) + 1;
    if (this._consecutiveFailures >= 2 && backendLooksOffline()) {
      this._setConnectionState(navigator.onLine === false ? "offline" : "reconnecting");
    }
  };

  /**
   * Toggle the header connection chip. "online" hides it; the other states show
   * a small, non-intrusive badge so a dropped connection is visible without
   * wiping content or throwing the user into a scary full-tab error.
   * @param {"online"|"reconnecting"|"offline"} state
   */
  proto._setConnectionState = function (state) {
    if (this._connectionState === state && state !== "online") return;
    this._connectionState = state;
    const chip = this.dialogEl?.querySelector(".tdd-connection-chip");
    if (!chip) return;
    chip.dataset.state = state;
    if (state === "online") {
      chip.hidden = true;
      const text = chip.querySelector(".tdd-connection-text");
      if (text) text.textContent = "";
      return;
    }
    chip.hidden = false;
    const text = chip.querySelector(".tdd-connection-text");
    if (text) {
      text.textContent = CONNECTION_TEXT[state] || CONNECTION_TEXT.reconnecting;
    }
  };

  /**
   * Render a friendly, consistent error state with a Retry button into a tab
   * body. Used only when the INITIAL load fails outright (no data to show);
   * transient poll failures never reach here — they keep the last good view.
   * @param {HTMLElement} content - the tab content element
   * @param {{title?: string, message?: string}} [opts]
   */
  proto._renderTabError = function (content, opts = {}) {
    if (!content) return;
    const title = opts.title || "Couldn't load data";
    const message =
      opts.message ||
      (navigator.onLine === false
        ? "You appear to be offline."
        : "The request failed after several attempts.");
    const html = `
      <div class="tdd-state tdd-state-error" role="alert">
        <i class="tdd-state-icon icon-triangle-alert" aria-hidden="true"></i>
        <div class="tdd-state-title">${title}</div>
        <div class="tdd-state-message">${message}</div>
        <button type="button" class="tdd-state-retry" data-action="tdd-retry">
          <i class="icon-refresh-cw" aria-hidden="true"></i> Retry
        </button>
      </div>
    `;
    this._renderHtmlIfChanged(content, html, "__stateHtml");
    content.dataset.loaded = "false";
  };

  /**
   * Render the shared "waiting for data" placeholder into a tab body. Idempotent
   * via `_renderHtmlIfChanged`, so calling it on every poll is free.
   * @param {HTMLElement} content - the tab content element
   * @param {string} [label]
   */
  proto._renderTabWaiting = function (content, label = "Waiting for data…") {
    if (!content) return;
    const html = `<div class="tdd-state tdd-state-loading"><div class="loading-spinner">${label}</div></div>`;
    this._renderHtmlIfChanged(content, html, "__stateHtml");
  };

  /**
   * Retry the initial load after an error-state Retry click: reset the retry
   * budget, show the waiting placeholder again, and kick a fresh fetch. The
   * background poller keeps running regardless, so this is just for immediacy.
   */
  proto._retryInitialLoad = function () {
    this._retryCount = 0;
    this._setConnectionState("reconnecting");
    const content = this.dialogEl?.querySelector(`[data-tab-content="${this.currentTab}"]`);
    this._renderTabWaiting(content, "Loading…");
    this.isRefreshing = false;
    this._fetchTokenData();
  };
}
