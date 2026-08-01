// Shared ScreenerBot account panel for setup and Settings.
(function () {
  "use strict";

  const GOOGLE_MARK =
    '<img class="account-google-mark" src="/assets/google-g.png" alt="" aria-hidden="true" />';

  const SCOPE_LABELS = {
    "rpc:submit": "Free signed-transaction submission",
    vote: "Token voting",
    "referral:read": "Referral earnings",
    "account:read": "Account details",
  };

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
        body?.error?.message || body?.message || "That did not work. Please try again."
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
        this.error = error?.message || "Account status is unavailable.";
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
        '<p class="account-loading" role="status">Checking account status…</p>';
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
        this.notice =
          "Finish signing in in your browser, then return here. This panel will update.";
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
          this.notice = "Browser sign-in was not completed. You can start it again.";
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

      this.container.setAttribute("aria-busy", String(Boolean(this.busyAction)));
      this.bind();
      this.restoreFocus();
    }

    renderUnavailable() {
      return `
        <div class="account-unavailable">
          <p class="account-lead">Account features are unavailable right now. Continue setup without signing in.</p>
          ${this.renderError()}
          <button type="button" class="account-btn account-btn-ghost" data-action="retry-status">
            Retry account status
          </button>
        </div>`;
    }

    renderSignedIn() {
      const name = escapeHtml(this.status.name || this.status.email || "Signed in");
      const email = this.status.email ? escapeHtml(this.status.email) : null;
      const scopes = (this.status.scopes || [])
        .map((scope) => SCOPE_LABELS[scope])
        .filter(Boolean)
        .map((label) => `<li class="account-scope">${escapeHtml(label)}</li>`)
        .join("");

      return `
        <div class="account-panel-signed-in">
          <div class="account-identity">
            <div class="account-identity-text">
              <span class="account-identity-name">${name}</span>
              ${email && email !== name ? `<span class="account-identity-email">${email}</span>` : ""}
            </div>
          </div>
          ${scopes ? `<ul class="account-scopes" aria-label="Account features">${scopes}</ul>` : ""}
          <div class="account-actions">
            <button type="button" class="account-btn account-btn-ghost" data-action="signout"
              ${this.busyAction ? "disabled" : ""}>
              ${this.busyAction === "signout" ? "Signing out…" : "Sign out"}
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
             <span class="account-option-title">${this.busyAction === "wallet" ? "Signing in…" : "Sign in with wallet"}</span>
           </button>`
        : "";

      return `
        <div class="account-panel-signed-out">
          <p class="account-lead">
            An account adds free transaction sending, token voting, and referral rewards.
          </p>
          <div class="account-options">
            <button type="button" class="account-option" data-action="browser" ${disabled}>
              ${GOOGLE_MARK}
              <span class="account-option-title">${this.busyAction === "browser" ? "Opening browser…" : "Continue in browser"}</span>
            </button>
            <button type="button" class="account-option" data-action="email" ${disabled}>
              <span class="account-option-title">Sign in with email</span>
            </button>
            ${walletOption}
          </div>
          <p class="account-note account-signup-note">
            <span>New to ScreenerBot?</span>
            <button type="button" class="account-link" data-action="signup">
              Create an account
            </button>
          </p>
          ${this.renderNotice()}
          ${this.renderError()}
        </div>`;
    }

    renderEmailForm() {
      const disabled = this.busyAction ? "disabled" : "";
      return `
        <form class="account-email-form" data-action="email-submit">
          <button type="button" class="account-link account-back" data-action="menu" ${disabled}>
            Back to sign-in options
          </button>
          <label class="account-field">
            <span class="account-field-label">Email</span>
            <input type="email" class="account-input" name="email" autocomplete="email"
              value="${escapeHtml(this.emailValue)}" placeholder="you@example.com" required ${disabled} />
          </label>
          <label class="account-field">
            <span class="account-field-label">Password</span>
            <input type="password" class="account-input" name="password" autocomplete="current-password"
              placeholder="Your password" required ${disabled} />
          </label>
          <button type="submit" class="account-btn account-btn-primary" ${disabled}>
            ${this.busyAction === "password" ? "Signing in…" : "Sign in"}
          </button>
          <p class="account-note">
            Need an account or forgot your password?
            <button type="button" class="account-link" data-action="signup" ${disabled}>
              Open screenerbot.io
            </button>
          </p>
          ${this.renderError()}
        </form>`;
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
