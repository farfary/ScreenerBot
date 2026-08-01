// ScreenerBot account panel — one renderer, two homes.
//
// It is mounted on the SETUP screen (right column, beside wallet and RPC) and
// in the SETTINGS dialog (Account tab). Both show the same states and call the
// same endpoints, so they cannot drift into disagreeing about whether somebody
// is signed in.
//
// A classic script rather than a module, matching context_menu/builders.js and
// advanced_chart/themes.js: core/setup.js is not a module and cannot import.
//
// The security token is attached to every fetch by the wrapper in base.html, so
// nothing here handles it.

(function () {
  "use strict";

  const SIGN_UP_URL = "https://screenerbot.io/signup";

  /**
   * Google's own mark, inline. Their brand guidelines do not permit recolouring
   * it, so it keeps its four colours and — like every icon in this app — gets no
   * plate, ring or tile behind it.
   */
  const GOOGLE_MARK = `<svg class="account-google-mark" viewBox="0 0 18 18" aria-hidden="true">
    <path fill="#4285F4" d="M17.64 9.2c0-.64-.06-1.25-.16-1.84H9v3.48h4.84a4.14 4.14 0 0 1-1.8 2.72v2.26h2.92c1.7-1.57 2.68-3.88 2.68-6.62Z"/>
    <path fill="#34A853" d="M9 18c2.43 0 4.47-.8 5.96-2.18l-2.92-2.26c-.8.54-1.84.86-3.04.86-2.34 0-4.32-1.58-5.03-3.7H.96v2.33A9 9 0 0 0 9 18Z"/>
    <path fill="#FBBC05" d="M3.97 10.72a5.4 5.4 0 0 1 0-3.44V4.95H.96a9 9 0 0 0 0 8.1l3-2.33Z"/>
    <path fill="#EA4335" d="M9 3.58c1.32 0 2.5.45 3.44 1.35l2.58-2.59C13.46.9 11.43 0 9 0A9 9 0 0 0 .96 4.95l3.01 2.33C4.68 5.16 6.66 3.58 9 3.58Z"/>
  </svg>`;

  const SCOPE_LABELS = {
    "rpc:submit": "Send signed transactions through ScreenerBot",
    vote: "Vote on tokens",
    "referral:read": "Show your referral earnings",
    "account:read": "Show your account details",
  };

  async function request(path, options) {
    const response = await fetch(path, options);
    let body = null;
    try {
      body = await response.json();
    } catch {
      body = null;
    }

    if (!response.ok) {
      // Every account route answers a failure with a full sentence, and the
      // server's own wording is what the user reads. A status code never is.
      const message =
        body?.error?.message || body?.message || "That did not work. Please try again.";
      throw new Error(message);
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
      this.mode = "menu";
      this.busy = false;
      this.error = null;
      this.notice = null;
      this.walletHasAccount = false;
      this.devicePoll = null;
    }

    async load() {
      try {
        this.status = await request("/api/account/status");
      } catch {
        // Signed out is the safe assumption; the panel must render either way.
        this.status = { signed_in: false, scopes: [] };
      }

      this.render();

      // Asked separately and never awaited by the first paint: it is a network
      // round trip, and the panel must not sit blank waiting for it.
      if (!this.status.signed_in) {
        this.checkWallet();
      }
    }

    async checkWallet() {
      try {
        const result = await request("/api/account/signin/wallet/check");
        this.walletHasAccount = Boolean(result?.has_account);
        if (this.walletHasAccount) this.render();
      } catch {
        // Offline, or no wallet yet. Neither is worth telling anyone about.
      }
    }

    setBusy(busy) {
      this.busy = busy;
      this.render();
    }

    fail(error) {
      this.error = error?.message || String(error);
      this.busy = false;
      this.render();
    }

    succeed(status) {
      this.status = status;
      this.error = null;
      this.notice = null;
      this.busy = false;
      this.mode = "menu";
      this.stopDevicePoll();
      this.render();
      this.options.onChange?.(status);
    }

    // -----------------------------------------------------------------------
    // Actions
    // -----------------------------------------------------------------------

    async signInWithBrowser() {
      this.error = null;
      this.setBusy(true);

      try {
        const result = await request("/api/account/signin/browser", { method: "POST" });

        // Opened through the backend so it lands in the SYSTEM browser. An
        // embedded window would be able to read what the user types, which is
        // the whole thing this flow exists to avoid.
        await fetch("/api/system/open-url", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ url: result.url }),
        });

        this.notice =
          "Finish signing in in your browser, then come back — this panel updates on its own.";
        this.busy = false;
        this.render();
        this.startStatusWatch();
      } catch (error) {
        this.fail(error);
      }
    }

    /**
     * After the browser leg the app has no event to wait on: the callback lands
     * on a different route in the same process. Poll briefly rather than make
     * the user press something to find out whether it worked.
     */
    startStatusWatch() {
      this.stopDevicePoll();
      let elapsed = 0;

      this.devicePoll = setInterval(async () => {
        elapsed += 2000;
        if (elapsed > 5 * 60 * 1000) return this.stopDevicePoll();

        try {
          const status = await request("/api/account/status");
          if (status.signed_in) this.succeed(status);
        } catch {
          // Keep waiting; a blip is not a failure.
        }
      }, 2000);
    }

    stopDevicePoll() {
      if (this.devicePoll) {
        clearInterval(this.devicePoll);
        this.devicePoll = null;
      }
    }

    async signInWithPassword(email, password) {
      this.error = null;
      this.setBusy(true);

      try {
        const status = await request("/api/account/signin/password", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ email, password }),
        });
        this.succeed(status);
      } catch (error) {
        this.fail(error);
      }
    }

    async signInWithWallet(create) {
      this.error = null;
      this.setBusy(true);

      try {
        const status = await request("/api/account/signin/wallet", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ create: Boolean(create) }),
        });
        this.succeed(status);
      } catch (error) {
        this.fail(error);
      }
    }

    async signOut() {
      this.error = null;
      this.setBusy(true);

      try {
        const status = await request("/api/account/signout", { method: "POST" });
        this.walletHasAccount = false;
        this.succeed(status);
      } catch (error) {
        this.fail(error);
      }
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    render() {
      if (!this.container) return;

      this.container.innerHTML = this.status?.signed_in
        ? this.renderSignedIn()
        : this.renderSignedOut();

      this.bind();
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
            <i class="icon-circle-check account-identity-mark" aria-hidden="true"></i>
            <div class="account-identity-text">
              <span class="account-identity-name">${name}</span>
              ${email && email !== name ? `<span class="account-identity-email">${email}</span>` : ""}
            </div>
          </div>

          ${scopes ? `<ul class="account-scopes">${scopes}</ul>` : ""}

          <p class="account-note">
            Signing in adds features. It never gates any of them — your wallet, your RPC and
            your trading are unchanged.
          </p>

          <div class="account-actions">
            <button type="button" class="account-btn account-btn-ghost" data-action="signout"
              ${this.busy ? "disabled" : ""}>
              ${this.busy ? "Signing out…" : "Sign out on this machine"}
            </button>
          </div>
          ${this.renderError()}
        </div>`;
    }

    renderSignedOut() {
      if (this.mode === "email") return this.renderEmailForm();

      const walletOption = this.walletHasAccount
        ? `<button type="button" class="account-option" data-action="wallet" ${this.busy ? "disabled" : ""}>
             <i class="icon-wallet account-option-icon" aria-hidden="true"></i>
             <span class="account-option-text">
               <span class="account-option-title">Sign in with this wallet</span>
               <span class="account-option-hint">Your trading wallet already has an account. Signs a free message — not a transaction.</span>
             </span>
           </button>`
        : "";

      return `
        <div class="account-panel-signed-out">
          <p class="account-lead">
            Optional. An account adds free transaction sending, token voting and referral
            earnings. ScreenerBot works exactly the same without one.
          </p>

          <div class="account-options">
            <button type="button" class="account-option" data-action="browser" ${this.busy ? "disabled" : ""}>
              ${GOOGLE_MARK}
              <span class="account-option-text">
                <span class="account-option-title">Continue in your browser</span>
                <span class="account-option-hint">Google or email, in your own browser. ScreenerBot never sees your password.</span>
              </span>
            </button>

            <button type="button" class="account-option" data-action="email" ${this.busy ? "disabled" : ""}>
              <i class="icon-mail account-option-icon" aria-hidden="true"></i>
              <span class="account-option-text">
                <span class="account-option-title">Sign in with email</span>
                <span class="account-option-hint">Enter your ScreenerBot email and password here.</span>
              </span>
            </button>

            ${walletOption}
          </div>

          <p class="account-note">
            No account yet?
            <button type="button" class="account-link" data-action="signup">Create one on screenerbot.io</button>
          </p>

          ${this.renderNotice()}
          ${this.renderError()}
        </div>`;
    }

    renderEmailForm() {
      return `
        <form class="account-email-form" data-action="email-submit">
          <button type="button" class="account-link account-back" data-action="menu">
            <i class="icon-arrow-left" aria-hidden="true"></i> Other ways to sign in
          </button>

          <label class="account-field">
            <span class="account-field-label">Email</span>
            <input type="email" class="account-input" name="email" autocomplete="email"
              placeholder="you@example.com" required ${this.busy ? "disabled" : ""} />
          </label>

          <label class="account-field">
            <span class="account-field-label">Password</span>
            <input type="password" class="account-input" name="password" autocomplete="current-password"
              placeholder="Your password" required ${this.busy ? "disabled" : ""} />
          </label>

          <button type="submit" class="account-btn account-btn-primary" ${this.busy ? "disabled" : ""}>
            ${this.busy ? "Signing in…" : "Sign in"}
          </button>

          <p class="account-note">
            Sign up and password resets happen on screenerbot.io.
            <button type="button" class="account-link" data-action="signup">Open it</button>
          </p>

          ${this.renderError()}
        </form>`;
    }

    renderError() {
      if (!this.error) return "";
      return `<p class="account-error" role="alert">${escapeHtml(this.error)}</p>`;
    }

    renderNotice() {
      if (!this.notice) return "";
      return `<p class="account-notice" role="status">${escapeHtml(this.notice)}</p>`;
    }

    bind() {
      const root = this.container;

      root.querySelectorAll("[data-action]").forEach((element) => {
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

          switch (action) {
            case "browser":
              this.signInWithBrowser();
              break;
            case "email":
              this.mode = "email";
              this.error = null;
              this.render();
              break;
            case "menu":
              this.mode = "menu";
              this.error = null;
              this.render();
              break;
            case "wallet":
              this.signInWithWallet(false);
              break;
            case "signout":
              this.signOut();
              break;
            case "signup":
              fetch("/api/system/open-url", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ url: SIGN_UP_URL }),
              });
              break;
            default:
              break;
          }
        });
      });
    }

    destroy() {
      this.stopDevicePoll();
    }
  }

  window.AccountPanel = {
    /**
     * Mount into a container. Returns the instance so a caller that owns a
     * lifecycle (the settings dialog) can tear down its poller on close.
     */
    mount(container, options) {
      if (!container) return null;
      const instance = new AccountPanelInstance(container, options);
      instance.load();
      return instance;
    },
  };
})();
