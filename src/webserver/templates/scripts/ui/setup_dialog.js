/**
 * Setup Dialog — wallet + RPC credential entry as a modal.
 *
 * Used to complete setup from Explore Mode (header banner / config tab) without the
 * full-screen wizard. Validates via /api/initialization/validate and persists via
 * /api/initialization/complete, then reloads into the fully-configured dashboard.
 *
 * Usage: SetupDialog.show() -> Promise<boolean> (true if setup completed).
 */
import { apiErrorMessage } from "../core/request_manager.js";

class SetupDialog {
  static activeDialog = null;

  static async show() {
    if (SetupDialog.activeDialog) {
      SetupDialog.activeDialog.destroy();
    }
    return new Promise((resolve) => {
      const dialog = new SetupDialog(resolve);
      SetupDialog.activeDialog = dialog;
      dialog.render();
    });
  }

  constructor(resolver) {
    this.resolver = resolver;
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
        <div class="setup-dialog-icon"><i class="icon-key-round"></i></div>
        <div>
          <h2
            class="setup-dialog-title"
            id="setup-dialog-title"
            data-l10n-id="setup-dialog-title"
          ></h2>
          <p class="setup-dialog-subtitle" data-l10n-id="setup-dialog-subtitle"></p>
        </div>
        <button type="button" class="setup-dialog-close" data-action="cancel"
          data-l10n-id="setup-dialog-close">
          <i class="icon-x"></i>
        </button>
      </div>

      <div class="setup-dialog-body">
        <label class="setup-dialog-field">
          <span class="setup-dialog-label">
            <span data-l10n-id="setup-dialog-wallet-label"></span> <span class="req">*</span>
          </span>
          <div class="setup-dialog-input-wrap">
            <textarea id="setup-dialog-wallet" class="setup-dialog-input" rows="2"
              data-l10n-id="setup-dialog-wallet-input" spellcheck="false"
              autocomplete="off"></textarea>
            <button type="button" class="setup-dialog-reveal" data-action="reveal"
              aria-pressed="false">
              <i class="icon-eye" aria-hidden="true"></i>
            </button>
          </div>
          <span class="setup-dialog-hint" id="setup-dialog-wallet-hint"></span>
        </label>

        <label class="setup-dialog-field">
          <span class="setup-dialog-label">
            <span data-l10n-id="setup-dialog-rpc-label"></span> <span class="req">*</span>
          </span>
          <textarea id="setup-dialog-rpc" class="setup-dialog-input" rows="2"
            data-l10n-id="setup-dialog-rpc-input" spellcheck="false"></textarea>
          <span class="setup-dialog-hint" data-l10n-id="setup-dialog-rpc-hint"></span>
        </label>

        <div class="setup-dialog-status" id="setup-dialog-status" hidden></div>
      </div>

      <div class="setup-dialog-footer">
        <button type="button" class="setup-dialog-btn secondary" data-action="cancel"
          data-l10n-id="common-action-cancel"></button>
        <button type="button" class="setup-dialog-btn primary" data-action="submit"
          data-l10n-id="setup-dialog-submit"></button>
      </div>
    `;

    this.backdrop.appendChild(this.element);
    I18n.localizeTree(this.element);
    document.body.appendChild(this.backdrop);

    this.walletInput = this.element.querySelector("#setup-dialog-wallet");
    this.rpcInput = this.element.querySelector("#setup-dialog-rpc");
    this.statusEl = this.element.querySelector("#setup-dialog-status");
    this.submitBtn = this.element.querySelector('[data-action="submit"]');

    this.element.querySelectorAll('[data-action="cancel"]').forEach((b) =>
      b.addEventListener("click", () => {
        if (!this.busy) this.close(false);
      })
    );
    this.element
      .querySelector('[data-action="submit"]')
      .addEventListener("click", () => this.submit());
    const revealBtn = this.element.querySelector('[data-action="reveal"]');
    this.setRevealed(revealBtn, false);
    revealBtn.addEventListener("click", (e) => {
      const masked = this.walletInput.style.webkitTextSecurity !== "none";
      this.walletInput.style.webkitTextSecurity = masked ? "none" : "disc";
      this.setRevealed(e.currentTarget, masked);
    });
    this.walletInput.style.webkitTextSecurity = "disc";

    document.addEventListener("keydown", this.onKeyDown);
    setTimeout(() => this.walletInput.focus(), 50);
  }

  setRevealed(button, revealed) {
    const label = revealed ? I18n.t("setup-wallet-hide-key") : I18n.t("setup-wallet-show-key");
    button.querySelector("i").className = revealed ? "icon-eye-off" : "icon-eye";
    button.setAttribute("aria-pressed", String(revealed));
    button.setAttribute("aria-label", label);
    button.title = label;
  }

  onKeyDown(e) {
    if (e.key === "Escape" && !this.busy) {
      this.close(false);
    }
  }

  parseRpcUrls() {
    return (this.rpcInput.value || "")
      .split("\n")
      .map((u) => u.trim())
      .filter(Boolean);
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
        : I18n.t("setup-dialog-submit");
    }
  }

  async submit() {
    const walletPrivateKey = (this.walletInput.value || "").trim();
    const rpcUrls = this.parseRpcUrls();

    if (!walletPrivateKey || rpcUrls.length === 0) {
      this.setStatus("error", I18n.t("setup-dialog-error-missing-fields"));
      return;
    }

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
      if (validation.warnings?.length) {
        this.setStatus("warning", validation.warnings.join(" "));
      }

      this.setBusy(true, I18n.t("setup-dialog-saving"));
      const completeRes = await fetch("/api/initialization/complete", {
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
      if (!completeRes.ok || !result.success) {
        const msg =
          apiErrorMessage(result) ||
          (result?.errors?.length ? result.errors.join(" ") : I18n.t("setup-dialog-error-incomplete"));
        this.setStatus("error", msg);
        this.setBusy(false);
        return;
      }

      this.walletInput.value = "";
      this.setStatus("success", I18n.t("setup-dialog-saved"));
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
