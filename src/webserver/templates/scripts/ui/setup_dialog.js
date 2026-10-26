// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Setup Dialog — wallet + RPC credential entry as a modal.
 *
 * Setup mode completes setup from Explore Mode (header banner / config tab) without
 * the full-screen wizard: it validates via /api/initialization/validate and persists
 * via /api/initialization/complete. Replace mode replaces the trading wallet from the
 * Wallets page: it first states what the replacement stops, moves and keeps, then
 * validates the same way and stages the replacement via /api/wallets/replace; its RPC
 * field may stay empty to keep the configured endpoints. Both restart the app and
 * reload into it.
 *
 * Usage: SetupDialog.show({ mode }) -> Promise<boolean> (true if the dialog finished).
 */
import { apiErrorMessage } from "../core/request_manager.js";

// The confirmation a replacement shows before its credential fields: what stops, what
// moves, what stays, what entered RPC endpoints replace and that the app restarts.
const CONFIRM_STEP = `
  <div class="setup-dialog-confirm">
    <p data-l10n-id="setup-dialog-replace-confirm-intro"></p>
    <ul class="setup-dialog-confirm-list">
      <li data-l10n-id="setup-dialog-replace-confirm-stop"></li>
      <li data-l10n-id="setup-dialog-replace-confirm-backup"></li>
      <li data-l10n-id="setup-dialog-replace-confirm-keep"></li>
      <li data-l10n-id="setup-dialog-replace-confirm-funds"></li>
      <li data-l10n-id="setup-dialog-replace-confirm-rpc"></li>
      <li data-l10n-id="setup-dialog-replace-confirm-restart"></li>
    </ul>
  </div>
`;

// What each mode shows and where it saves the validated credentials.
const MODES = Object.freeze({
  setup: Object.freeze({
    icon: "icon-key-round",
    title: "setup-dialog-title",
    subtitle: "setup-dialog-subtitle",
    submit: "setup-dialog-submit",
    saved: "setup-dialog-saved",
    endpoint: "/api/initialization/complete",
    confirm: false,
    rpcOptional: false,
  }),
  replace: Object.freeze({
    icon: "icon-replace",
    title: "setup-dialog-replace-title",
    subtitle: "setup-dialog-replace-subtitle",
    submit: "setup-dialog-replace-submit",
    saved: "setup-dialog-replace-saved",
    endpoint: "/api/wallets/replace",
    confirm: true,
    // An empty RPC field keeps the configured endpoints.
    rpcOptional: true,
  }),
});

class SetupDialog {
  static activeDialog = null;

  static async show({ mode = "setup" } = {}) {
    if (SetupDialog.activeDialog) {
      SetupDialog.activeDialog.destroy();
    }
    return new Promise((resolve) => {
      const dialog = new SetupDialog(resolve, MODES[mode] || MODES.setup);
      SetupDialog.activeDialog = dialog;
      dialog.render();
    });
  }

  constructor(resolver, mode) {
    this.resolver = resolver;
    this.mode = mode;
    this.backdrop = null;
    this.element = null;
    this.busy = false;
    this.requestAbort = null;
    this.onKeyDown = this.onKeyDown.bind(this);
  }

  render() {
    this.backdrop = document.createElement("div");
    this.backdrop.className = "setup-dialog-backdrop";
    this.backdrop.addEventListener("click", () => {
      if (!this.busy) this.close(false);
    });

    this.element = document.createElement("div");
    this.element.className = "setup-dialog";
    this.element.setAttribute("role", "dialog");
    this.element.setAttribute("aria-modal", "true");
    this.element.setAttribute("aria-labelledby", "setup-dialog-title");
    this.element.addEventListener("click", (e) => e.stopPropagation());

    this.element.innerHTML = `
      <div class="setup-dialog-header">
        <div>
          <h2 class="setup-dialog-title" id="setup-dialog-title">
            <i class="${this.mode.icon}" aria-hidden="true"></i>
            <span data-l10n-id="${this.mode.title}"></span>
          </h2>
          <p class="setup-dialog-subtitle" data-l10n-id="${this.mode.subtitle}"></p>
        </div>
        <button type="button" class="setup-dialog-close" data-action="cancel"
          data-l10n-id="setup-dialog-close">
          <i class="icon-x"></i>
        </button>
      </div>

      <div class="setup-dialog-body">
        ${this.mode.confirm ? CONFIRM_STEP : ""}
        <div class="setup-dialog-fields"${this.mode.confirm ? " hidden" : ""}></div>

        <div class="setup-dialog-status" id="setup-dialog-status" hidden></div>
      </div>

      <div class="setup-dialog-footer">
        <button type="button" class="btn btn-secondary" data-action="cancel"
          data-l10n-id="common-action-cancel"></button>
        <button type="button" class="btn btn-primary" data-action="submit"
          data-l10n-id="${this.mode.confirm ? "common-action-continue" : this.mode.submit}"></button>
      </div>
    `;

    this.backdrop.appendChild(this.element);
    I18n.localizeTree(this.element);
    document.body.appendChild(this.backdrop);

    // The same credential fields as the onboarding step, under ids of their own.
    this.fields = window.SetupRuntime.mountCredentialFields(
      this.element.querySelector(".setup-dialog-fields"),
      "setup-dialog-",
      { rpcOptional: this.mode.rpcOptional }
    );
    this.statusEl = this.element.querySelector("#setup-dialog-status");
    this.submitBtn = this.element.querySelector('[data-action="submit"]');

    this.element.querySelectorAll('[data-action="cancel"]').forEach((b) =>
      b.addEventListener("click", () => {
        if (!this.busy) this.close(false);
      })
    );
    this.confirmed = !this.mode.confirm;
    this.element
      .querySelector('[data-action="submit"]')
      .addEventListener("click", () => (this.confirmed ? this.submit() : this.showCredentials()));

    document.addEventListener("keydown", this.onKeyDown);
    if (this.confirmed) setTimeout(() => this.fields.walletInput.focus(), 50);
  }

  // Leave the confirmation step for the credential fields.
  showCredentials() {
    this.confirmed = true;
    this.element.querySelector(".setup-dialog-confirm")?.setAttribute("hidden", "");
    this.element.querySelector(".setup-dialog-fields")?.removeAttribute("hidden");
    this.submitBtn.textContent = I18n.t(this.mode.submit);
    this.fields.walletInput.focus();
  }

  onKeyDown(e) {
    if (e.key === "Escape" && !this.busy) {
      this.close(false);
    }
  }

  setStatus(kind, message) {
    if (!this.statusEl) return;
    this.statusEl.hidden = false;
    this.statusEl.className = `setup-dialog-status ${kind}`;
    this.statusEl.textContent = message;
  }

  setBusy(busy, label) {
    this.busy = busy;
    if (this.submitBtn) {
      this.submitBtn.disabled = busy;
      this.submitBtn.textContent = busy
        ? label || I18n.t("setup-dialog-working")
        : I18n.t(this.mode.submit);
    }
  }

  async submit() {
    if (!this.fields.validateAll()) return;
    const walletPrivateKey = this.fields.walletInput.value.trim();
    const rpcUrls = this.fields.rpcUrls();

    const snapshot = Object.freeze({
      walletPrivateKey,
      rpcUrls: Object.freeze([...rpcUrls]),
    });
    this.requestAbort?.abort();
    this.requestAbort = new AbortController();

    try {
      this.setBusy(true, I18n.t("setup-dialog-validating"));
      const validateRes = await fetch("/api/initialization/validate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          wallet_private_key: snapshot.walletPrivateKey,
          rpc_urls: snapshot.rpcUrls,
        }),
        signal: this.requestAbort.signal,
      });
      const validation = await validateRes.json();
      if (!validateRes.ok || !validation.valid || !validation.validation_id) {
        const msg =
          apiErrorMessage(validation) ||
          (validation?.errors?.length ? validation.errors.join(" ") : I18n.t("setup-dialog-error-validation"));
        this.setStatus("error", msg);
        this.setBusy(false);
        return;
      }
      if (validation.wallet_address) this.fields.showWalletPreview(validation.wallet_address);
      if (validation.warnings?.length) {
        this.setStatus("warning", validation.warnings.join(" "));
      }

      this.setBusy(true, I18n.t("setup-dialog-saving"));
      const completeRes = await fetch(this.mode.endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          validation_id: validation.validation_id,
          wallet_private_key: snapshot.walletPrivateKey,
          rpc_urls: snapshot.rpcUrls,
        }),
        signal: this.requestAbort.signal,
      });
      const result = await completeRes.json();
      if (!completeRes.ok || !result.restart_required) {
        const msg =
          apiErrorMessage(result) ||
          (result?.errors?.length ? result.errors.join(" ") : I18n.t("setup-dialog-error-incomplete"));
        this.setStatus("error", msg);
        this.setBusy(false);
        return;
      }

      this.fields.walletInput.value = "";
      this.setStatus("success", I18n.t(this.mode.saved));
      this.submitBtn.textContent = I18n.t("setup-dialog-restarting");

      const waitForRestart = window.waitForScreenerBotRestart;
      if (typeof waitForRestart !== "function") {
        throw new Error(I18n.t("setup-dialog-error-restart-helper"));
      }

      await waitForRestart(result.instance_id, { target: window.location.pathname || "/home" });
    } catch (err) {
      if (err?.name === "AbortError") return;
      this.setStatus("error", err?.message || I18n.t("setup-dialog-error-unexpected"));
      this.setBusy(false);
    }
  }

  close(result) {
    this.destroy();
    if (this.resolver) {
      this.resolver(Boolean(result));
      this.resolver = null;
    }
  }

  destroy() {
    this.fields?.dispose();
    this.requestAbort?.abort();
    this.requestAbort = null;
    document.removeEventListener("keydown", this.onKeyDown);
    if (this.backdrop && this.backdrop.parentNode) {
      this.backdrop.parentNode.removeChild(this.backdrop);
    }
    if (SetupDialog.activeDialog === this) {
      SetupDialog.activeDialog = null;
    }
  }
}

export { SetupDialog };
