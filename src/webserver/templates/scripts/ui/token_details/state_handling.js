// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

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

import { renderStateView } from "../state_view.js";

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
    const icon = chip.querySelector(".tdd-connection-icon");
    if (state === "online") {
      chip.hidden = true;
      const text = chip.querySelector(".tdd-connection-text");
      if (text) text.textContent = "";
      return;
    }
    chip.hidden = false;
    if (icon) {
      icon.className = `tdd-connection-icon ${
        state === "offline" ? "icon-cloud-off" : "icon-refresh-cw"
      }`;
    }
    const text = chip.querySelector(".tdd-connection-text");
    if (text) {
      text.textContent = I18n.t("shell-connection-waiting");
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
    const title = opts.title || I18n.t("tokens-state-error-title");
    const message =
      opts.message ||
      (navigator.onLine === false
        ? I18n.t("tokens-state-offline")
        : I18n.t("tokens-state-request-failed"));
    const html = renderStateView({
      kind: "error",
      icon: "icon-triangle-alert",
      title,
      message,
      retry: "tdd-retry",
    });
    this._renderHtmlIfChanged(content, html, "__stateHtml");
    content.dataset.loaded = "false";
  };

  /**
   * Render the shared "waiting for data" placeholder into a tab body. Idempotent
   * via `_renderHtmlIfChanged`, so calling it on every poll is free.
   * @param {HTMLElement} content - the tab content element
   * @param {string} [label]
   */
  proto._renderTabWaiting = function (content, label = I18n.t("tokens-state-waiting")) {
    if (!content) return;
    const html = renderStateView({ kind: "loading", message: label });
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
    this._renderTabWaiting(content, I18n.t("common-loading"));
    this.isRefreshing = false;
    this._fetchTokenData();
  };
}
