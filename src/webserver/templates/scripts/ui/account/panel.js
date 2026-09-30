// Shared ScreenerBot account panel for setup and Settings.
(function () {
  "use strict";

  const GOOGLE_MARK =
    '<img class="account-google-mark" src="/assets/google-g.png" alt="" aria-hidden="true" />';

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
      this.render();
      this.options.onChange?.(status);
    }

    async signInWithBrowser() {
      this.error = null;
      this.setBusy("browser");

      try {
        await request("/api/account/signin/browser", { method: "POST" });
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
        await request("/api/account/signup", { method: "POST" });
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

      const disabled = this.busyAction ? "disabled" : "";
      const walletOption = this.walletHasAccount
        ? `<button type="button" class="account-option" data-action="wallet" ${disabled}>
             <span class="account-option-title">${escapeHtml(this.busyAction === "wallet" ? I18n.t("account-panel-signing-in") : I18n.t("account-panel-sign-in-wallet"))}</span>
           </button>`
        : "";

      return `
        <div class="account-panel-signed-out">
          ${this.renderDataAccess()}
          ${this.renderUnlocks()}
          <div class="account-options">
            <button type="button" class="account-option" data-action="browser" ${disabled}>
              ${GOOGLE_MARK}
              <span class="account-option-title">${escapeHtml(this.busyAction === "browser" ? I18n.t("account-panel-opening-browser") : I18n.t("account-panel-continue-browser"))}</span>
            </button>
            <button type="button" class="account-option" data-action="email" ${disabled}>
              <span class="account-option-title" data-l10n-id="account-panel-sign-in-email"></span>
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
      return this.notice
        ? `<p class="account-notice" role="status">${escapeHtml(this.notice)}</p>`
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
