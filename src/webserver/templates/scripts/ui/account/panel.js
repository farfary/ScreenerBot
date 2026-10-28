// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Shared ScreenerBot account panel for setup and Settings.
(function () {
  "use strict";

  // Scope ids are issued by the account server; a scope without an entry is not listed.
  const ACCOUNT_SCOPE_LABELS = Object.freeze({
    "data:read": "account-scope-data-read",
    "rpc:submit": "account-scope-rpc-submit",
    vote: "account-scope-vote",
    "referral:read": "account-scope-referral-read",
    "account:read": "account-scope-account-read",
  });

  // What signing in adds BEYOND the data service. The data block above the list
  // already states where market data comes from for this install, so repeating
  // "ScreenerBot market data" here would be the third sentence about the same
  // thing rather than a reason to sign in.
  const UNLOCKS = ["rpc:submit", "vote", "referral:read"];

  async function request(path, options = {}) {
    const response = await fetch(path, options);
    let body = null;
    try {
      body = await response.json();
    } catch {
      body = null;
    }

    if (!response.ok) {
      throw new Error(
        window.RequestManagerErrors.apiErrorMessage(body, I18n.t("account-panel-request-failed"))
      );
    }

    return body?.data ?? body;
  }

  // `location.hostname` keeps the brackets of an IPv6 literal.
  const LOOPBACK_HOSTNAMES = new Set(["127.0.0.1", "localhost", "[::1]"]);

  // Browser sign-in returns through the loopback callback on the server's own port,
  // which reaches this server only when the viewer's browser runs on a machine whose
  // loopback port leads to it: the desktop app, a browser on the same machine, or a
  // tunnel that keeps the port. A page opened by a network address (another machine
  // on the LAN) or behind another port offers the code flow instead.
  function browserCallbackReachable() {
    return (
      LOOPBACK_HOSTNAMES.has(window.location.hostname) &&
      window.location.port === String(window.__SCREENERBOT_PORT ?? "")
    );
  }

  // The server hands a page back when it has no browser of its own (a headless
  // install); the viewer's browser opens it instead.
  function openHandedOver(started) {
    if (started?.opened === false && started.url) {
      window.open(started.url, "_blank", "noopener,noreferrer");
    }
  }

  function escapeHtml(value) {
    return String(value ?? "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  class AccountPanelInstance {
    constructor(container, options) {
      this.container = container;
      this.options = options || {};
      this.status = null;
      this.loadFailed = false;
      this.mode = "menu";
      this.busyAction = null;
      this.error = null;
      this.notice = null;
      this.emailValue = "";
      this.walletHasAccount = false;
      this.statusWatch = null;
      this.device = null;
      this.deviceTimer = null;
      this.destroyed = false;
      this.focusTarget = null;
    }

    async load() {
      this.loadFailed = false;
      this.error = null;
      this.renderLoading();

      try {
        this.status = await request("/api/account/status");
        if (this.destroyed) return;
        this.walletHasAccount = Boolean(this.status.wallet_has_account);
        this.render();
        this.options.onChange?.(this.status);
        if (!this.status.signed_in) this.checkWallet();
      } catch (error) {
        if (this.destroyed) return;
        this.loadFailed = true;
        this.error = error?.message || I18n.t("account-panel-status-unavailable");
        this.status = null;
        this.render();
        this.options.onChange?.({
          signed_in: false,
          online: false,
          use_gateway_rpc: false,
        });
      }
    }

    renderLoading() {
      if (!this.container) return;
      this.container.innerHTML =
        '<p class="account-loading" role="status" data-l10n-id="account-panel-checking"></p>';
      I18n.localizeTree(this.container);
    }

    async checkWallet() {
      try {
        const result = await request("/api/account/signin/wallet/check");
        if (this.destroyed) return;
        const changed = this.walletHasAccount !== Boolean(result?.has_account);
        this.walletHasAccount = Boolean(result?.has_account);
        if (changed) this.render();
      } catch {
        // Wallet account discovery is optional and never blocks local setup.
      }
    }

    setBusy(action) {
      this.busyAction = action;
      this.render();
    }

    fail(error, focusTarget) {
      this.error = error?.message || String(error);
      this.busyAction = null;
      this.focusTarget = focusTarget || null;
      this.render();
    }

    succeed(status) {
      this.status = status;
      this.loadFailed = false;
      this.error = null;
      this.notice = null;
      this.busyAction = null;
      this.mode = "menu";
      this.emailValue = "";
      this.stopStatusWatch();
      this.stopDevicePoll();
      this.render();
      this.options.onChange?.(status);
    }

    async signInWithBrowser() {
      this.error = null;
      this.setBusy("browser");

      try {
        openHandedOver(await request("/api/account/signin/browser", { method: "POST" }));
        this.notice = I18n.t("account-panel-browser-notice");
        this.busyAction = null;
        this.render();
        this.startStatusWatch();
      } catch (error) {
        this.fail(error, '[data-action="browser"]');
      }
    }

    startStatusWatch() {
      this.stopStatusWatch();
      let elapsed = 0;

      this.statusWatch = window.setInterval(async () => {
        elapsed += 2000;
        if (elapsed > 5 * 60 * 1000) {
          this.stopStatusWatch();
          this.notice = I18n.t("account-panel-browser-timeout");
          this.render();
          return;
        }

        try {
          const status = await request("/api/account/status");
          if (status.signed_in) this.succeed(status);
        } catch {
          // Brief connectivity loss should not cancel an external sign-in.
        }
      }, 2000);
    }

    stopStatusWatch() {
      if (!this.statusWatch) return;
      window.clearInterval(this.statusWatch);
      this.statusWatch = null;
    }

    /**
     * Device-code sign-in (RFC 8628) for an install whose browser cannot reach
     * this machine's loopback address, such as a headless server viewed from
     * another computer. The code is approved on screenerbot.io from any device;
     * this panel polls the bot until the server answers.
     */
    async signInWithCode() {
      this.error = null;
      this.notice = null;
      this.setBusy("device");

      try {
        const started = await request("/api/account/signin/device", { method: "POST" });
        if (this.destroyed) return;
        this.device = {
          userCode: started.user_code,
          verificationUri: started.verification_uri,
          verificationLink: started.verification_uri_complete || started.verification_uri,
          deviceCode: started.device_code,
          intervalMs: Math.max(1, Number(started.interval) || 5) * 1000,
          deadline: Date.now() + Math.max(0, Number(started.expires_in) || 0) * 1000,
        };
        this.mode = "device";
        this.busyAction = null;
        this.focusTarget = '[data-action="device-cancel"]';
        this.render();
        this.scheduleDevicePoll();
      } catch (error) {
        this.fail(error, '[data-action="device"]');
      }
    }

    scheduleDevicePoll() {
      this.stopDevicePoll();
      if (!this.device || this.destroyed) return;
      this.deviceTimer = window.setTimeout(() => this.pollDevice(), this.device.intervalMs);
    }

    stopDevicePoll() {
      if (!this.deviceTimer) return;
      window.clearTimeout(this.deviceTimer);
      this.deviceTimer = null;
    }

    async pollDevice() {
      this.deviceTimer = null;
      const device = this.device;
      if (!device || this.destroyed) return;
      if (Date.now() >= device.deadline) {
        this.deviceExpired();
        return;
      }

      try {
        const result = await request("/api/account/signin/device/poll", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ device_code: device.deviceCode }),
        });
        if (this.device !== device || this.destroyed) return;

        switch (result?.state) {
          case "signed_in":
            this.endDeviceFlow();
            this.load();
            return;
          case "expired":
            this.deviceExpired();
            return;
          case "slow_down":
            device.intervalMs += Math.max(0, Number(result.backoff_seconds) || 0) * 1000;
            break;
          default:
            break;
        }
        this.scheduleDevicePoll();
      } catch (error) {
        if (this.device !== device || this.destroyed) return;
        this.endDeviceFlow();
        this.fail(error, '[data-action="device"]');
      }
    }

    endDeviceFlow() {
      this.stopDevicePoll();
      this.device = null;
      this.mode = "menu";
    }

    deviceExpired() {
      this.endDeviceFlow();
      this.notice = I18n.t("account-panel-code-expired");
      this.focusTarget = '[data-action="device"]';
      this.render();
    }

    async signInWithPassword(email, password) {
      this.emailValue = email;
      this.error = null;
      this.setBusy("password");

      try {
        const status = await request("/api/account/signin/password", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ email, password }),
        });
        this.succeed(status);
      } catch (error) {
        this.fail(error, 'input[name="password"]');
      }
    }

    async signInWithWallet() {
      this.error = null;
      this.setBusy("wallet");

      try {
        const status = await request("/api/account/signin/wallet", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ create: false }),
        });
        this.succeed(status);
      } catch (error) {
        this.fail(error, '[data-action="wallet"]');
      }
    }

    async signOut() {
      this.error = null;
      this.setBusy("signout");

      try {
        const status = await request("/api/account/signout", { method: "POST" });
        this.walletHasAccount = false;
        this.succeed(status);
      } catch (error) {
        this.fail(error, '[data-action="signout"]');
      }
    }

    async openSignup() {
      this.error = null;
      try {
        openHandedOver(await request("/api/account/signup", { method: "POST" }));
      } catch (error) {
        this.fail(error, '[data-action="signup"]');
      }
    }

    render() {
      if (!this.container || this.destroyed) return;

      if (this.loadFailed) this.container.innerHTML = this.renderUnavailable();
      else if (this.status?.signed_in) this.container.innerHTML = this.renderSignedIn();
      else this.container.innerHTML = this.renderSignedOut();

      I18n.localizeTree(this.container);
      this.container.setAttribute("aria-busy", String(Boolean(this.busyAction)));
      this.bind();
      this.restoreFocus();
    }

    renderUnavailable() {
      return `
        <div class="account-unavailable">
          <p class="account-lead" data-l10n-id="account-panel-unavailable"></p>
          ${this.renderError()}
          <button type="button" class="account-btn account-btn-ghost" data-action="retry-status" data-l10n-id="account-panel-retry-status"></button>
        </div>`;
    }

    renderSignedIn() {
      const name = escapeHtml(
        this.status.name || this.status.email || I18n.t("account-panel-signed-in-fallback")
      );
      const email = this.status.email ? escapeHtml(this.status.email) : null;
      const scopes = (this.status.scopes || [])
        .filter((scope) => Object.hasOwn(ACCOUNT_SCOPE_LABELS, scope))
        .map(
          (scope) =>
            `<li class="account-scope">${escapeHtml(I18n.label(ACCOUNT_SCOPE_LABELS, scope))}</li>`
        )
        .join("");

      return `
        <div class="account-panel-signed-in">
          <div class="account-identity">
            <div class="account-identity-text">
              <span class="account-identity-name">${name}</span>
              ${email && email !== name ? `<span class="account-identity-email">${email}</span>` : ""}
            </div>
          </div>
          ${scopes ? `<ul class="account-scopes" data-l10n-id="account-panel-features">${scopes}</ul>` : ""}
          ${this.renderDataAccess()}
          <div class="account-actions">
            <button type="button" class="account-btn account-btn-ghost" data-action="signout"
              ${this.busyAction ? "disabled" : ""}>
              ${escapeHtml(this.busyAction === "signout" ? I18n.t("account-panel-signing-out") : I18n.t("account-panel-sign-out"))}
            </button>
          </div>
          ${this.renderError()}
        </div>`;
    }

    renderSignedOut() {
      if (this.mode === "email") return this.renderEmailForm();
      if (this.mode === "device" && this.device) return this.renderDeviceCode();

      const disabled = this.busyAction ? "disabled" : "";
      const walletOption = this.walletHasAccount
        ? `<button type="button" class="account-option" data-action="wallet" ${disabled}>
             <span class="account-option-title">${escapeHtml(this.busyAction === "wallet" ? I18n.t("account-panel-signing-in") : I18n.t("account-panel-sign-in-wallet"))}</span>
           </button>`
        : "";
      const browserOption = browserCallbackReachable()
        ? `<button type="button" class="account-option" data-action="browser" ${disabled}>
             <i class="icon-external-link account-option-icon" aria-hidden="true"></i>
             <span class="account-option-title">${escapeHtml(this.busyAction === "browser" ? I18n.t("account-panel-opening-browser") : I18n.t("account-panel-continue-browser"))}</span>
           </button>`
        : "";

      return `
        <div class="account-panel-signed-out">
          ${this.renderDataAccess()}
          ${this.renderUnlocks()}
          <div class="account-options">
            ${browserOption}
            <button type="button" class="account-option" data-action="email" ${disabled}>
              <span class="account-option-title" data-l10n-id="account-panel-sign-in-email"></span>
            </button>
            <button type="button" class="account-option" data-action="device" ${disabled}>
              <span class="account-option-title">${escapeHtml(this.busyAction === "device" ? I18n.t("account-panel-getting-code") : I18n.t("account-panel-sign-in-code"))}</span>
            </button>
            ${walletOption}
          </div>
          <p class="account-note account-signup-note">
            <span data-l10n-id="account-panel-new-to"></span>
            <button type="button" class="account-link" data-action="signup" data-l10n-id="account-panel-create-account"></button>
          </p>
          ${this.renderNotice()}
          ${this.renderError()}
        </div>`;
    }

    renderDeviceCode() {
      const { userCode, verificationUri, verificationLink } = this.device;
      return `
        <div class="account-device">
          <button type="button" class="account-link account-back" data-action="device-cancel" data-l10n-id="account-panel-back-to-options"></button>
          <p class="account-lead" data-l10n-id="account-panel-code-instructions"></p>
          <a class="account-device-link" href="${escapeHtml(verificationLink)}" target="_blank" rel="noopener noreferrer" dir="ltr">${escapeHtml(verificationUri)}</a>
          <div class="account-device-code-block">
            <span class="account-field-label" data-l10n-id="account-panel-code-label"></span>
            <code class="account-device-code" dir="ltr" translate="no">${escapeHtml(userCode)}</code>
          </div>
          <p class="account-note" role="status" data-l10n-id="account-panel-code-waiting"></p>
          ${this.renderError()}
        </div>`;
    }

    renderUnlocks() {
      const items = UNLOCKS.map(
        (scope) =>
          `<li class="account-scope">${escapeHtml(I18n.label(ACCOUNT_SCOPE_LABELS, scope))}</li>`
      );

      return `
        <div class="account-unlocks">
          <p class="account-unlocks-title" data-l10n-id="account-panel-unlocks-title"></p>
          <ul class="account-scopes">${items.join("")}</ul>
        </div>`;
    }

    renderEmailForm() {
      const disabled = this.busyAction ? "disabled" : "";
      return `
        <form class="account-email-form" data-action="email-submit">
          <button type="button" class="account-link account-back" data-action="menu" data-l10n-id="account-panel-back-to-options" ${disabled}></button>
          <label class="account-field">
            <span class="account-field-label" data-l10n-id="account-panel-email-label"></span>
            <input type="email" class="account-input" name="email" autocomplete="email"
              value="${escapeHtml(this.emailValue)}" data-l10n-id="account-panel-email-input" required ${disabled} />
          </label>
          <label class="account-field">
            <span class="account-field-label" data-l10n-id="account-panel-password-label"></span>
            <input type="password" class="account-input" name="password" autocomplete="current-password"
              data-l10n-id="account-panel-password-input" required ${disabled} />
          </label>
          <button type="submit" class="account-btn account-btn-primary" ${disabled}>
            ${escapeHtml(this.busyAction === "password" ? I18n.t("account-panel-signing-in") : I18n.t("account-panel-sign-in"))}
          </button>
          <p class="account-note">
            <span data-l10n-id="account-panel-need-account"></span>
            <button type="button" class="account-link" data-action="signup" data-l10n-id="account-panel-open-website" ${disabled}></button>
          </p>
          ${this.renderError()}
        </form>`;
    }

    /**
     * What ScreenerBot data does for this install right now.
     *
     * Rendered in BOTH the signed-out and signed-in states, because the honest
     * answer differs in both directions: signing out costs the shared cache, and
     * a device authorised before data access existed is signed in and still
     * without it. The backend composes every sentence (`data_server::access`) so
     * this panel, Settings and the introduction cannot drift apart.
     */
    renderDataAccess() {
      if (this.options.showDataAccess === false) return "";

      const access = this.status?.data_access;
      if (!access) return "";

      const state = escapeHtml(access.state || "unknown");
      const detailText = I18n.textAttr(access.text, "detail");
      const detail = detailText
        ? `<p class="account-data-detail">${escapeHtml(detailText)}</p>`
        : "";

      return `
        <div class="account-data" data-state="${state}">
          <p class="account-data-headline">
            <i class="${access.available ? "icon-circle-check" : "icon-info"}" aria-hidden="true"></i>
            <span>${escapeHtml(I18n.text(access.text))}</span>
          </p>
          ${detail}
        </div>`;
    }

    renderError() {
      return this.error
        ? `<p class="account-error" role="alert">${escapeHtml(this.error)}</p>`
        : "";
    }

    renderNotice() {
      // A session the server ended explains itself until the next sign-in, so
      // the user is not left wondering why the account disappeared. A classic script
      // cannot import ui/notice.js, so it spells the notice's info shape.
      const notice =
        this.notice || (this.status?.session_ended ? I18n.t("account-panel-session-ended") : null);
      return notice
        ? `<div class="notice" data-variant="info" role="note"><div class="notice-heading"><i class="notice-icon icon-info" aria-hidden="true"></i><span class="notice-message">${escapeHtml(notice)}</span></div></div>`
        : "";
    }

    bind() {
      this.container.querySelectorAll("[data-action]").forEach((element) => {
        const action = element.dataset.action;
        if (action === "email-submit") {
          element.addEventListener("submit", (event) => {
            event.preventDefault();
            const data = new FormData(element);
            this.signInWithPassword(
              String(data.get("email") || "").trim(),
              String(data.get("password") || "")
            );
          });
          return;
        }

        element.addEventListener("click", (event) => {
          event.preventDefault();
          if (this.busyAction) return;

          switch (action) {
            case "browser":
              this.signInWithBrowser();
              break;
            case "email":
              this.mode = "email";
              this.error = null;
              this.focusTarget = 'input[name="email"]';
              this.render();
              break;
            case "menu":
              this.mode = "menu";
              this.error = null;
              this.focusTarget = '[data-action="email"]';
              this.render();
              break;
            case "device":
              this.signInWithCode();
              break;
            case "device-cancel":
              this.endDeviceFlow();
              this.error = null;
              this.focusTarget = '[data-action="device"]';
              this.render();
              break;
            case "wallet":
              this.signInWithWallet();
              break;
            case "signout":
              this.signOut();
              break;
            case "signup":
              this.openSignup();
              break;
            case "retry-status":
              this.load();
              break;
            default:
              break;
          }
        });
      });
    }

    restoreFocus() {
      if (!this.focusTarget) return;
      const target = this.container.querySelector(this.focusTarget);
      this.focusTarget = null;
      window.requestAnimationFrame(() => target?.focus({ preventScroll: true }));
    }

    destroy() {
      this.destroyed = true;
      this.stopStatusWatch();
      this.stopDevicePoll();
    }
  }

  window.AccountPanel = {
    mount(container, options) {
      if (!container) return null;
      const instance = new AccountPanelInstance(container, options);
      instance.load();
      return instance;
    },
  };
})();
