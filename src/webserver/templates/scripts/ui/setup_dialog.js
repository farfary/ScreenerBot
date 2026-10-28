// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Setup Dialog — the trading wallet (created here with its key saved, or imported)
 * and the RPC endpoints, as a modal.
 *
 * Setup mode completes setup from Explore Mode (header banner / config tab) without
 * the full-screen wizard, on your own RPC: it validates via
 * /api/initialization/validate and persists via /api/initialization/complete. Replace
 * mode replaces the trading wallet from the Wallets page: it first states what the
 * replacement stops, moves and keeps, then validates the same way, under the current
 * connection, and stages the replacement via /api/wallets/replace. On your own RPC
 * its RPC field may stay empty to keep the configured endpoints; on the ScreenerBot
 * service it has none. Both restart the app and reload into it. A created key lives
 * only in the backup panel and is dropped when the dialog closes.
 *
 * Usage: SetupDialog.show({ mode }) -> Promise<boolean> (true if the dialog finished).
 */
import { pushEscapeHandler } from "../core/escape_stack.js";
import { apiErrorMessage } from "../core/request_manager.js";
import { getBootstrapState } from "../core/bootstrap.js";

// The confirmation a replacement shows before its credential fields: what stops, what
// moves, what stays, what entered RPC endpoints replace and that the app restarts.
const confirmStep = ({ rpc }) => `
  <div class="setup-dialog-confirm">
    <p data-l10n-id="setup-dialog-replace-confirm-intro"></p>
    <ul class="setup-dialog-confirm-list">
      <li data-l10n-id="setup-dialog-replace-confirm-stop"></li>
      <li data-l10n-id="setup-dialog-replace-confirm-backup"></li>
      <li data-l10n-id="setup-dialog-replace-confirm-keep"></li>
      <li data-l10n-id="setup-dialog-replace-confirm-funds"></li>
      ${rpc ? '<li data-l10n-id="setup-dialog-replace-confirm-rpc"></li>' : ""}
      <li data-l10n-id="setup-dialog-replace-confirm-restart"></li>
    </ul>
  </div>
`;

// What each mode shows and where it saves the validated credentials.
const MODES = Object.freeze({
  setup: Object.freeze({
    icon: "icon-key-round",
    title: () => I18n.t("setup-dialog-title"),
    subtitle: () => I18n.t("setup-dialog-subtitle"),
    submit: () => I18n.t("setup-dialog-submit"),
    saved: () => I18n.t("setup-dialog-saved"),
    endpoint: "/api/initialization/complete",
    confirm: false,
    replacesWallet: false,
    rpcOptional: false,
  }),
  replace: Object.freeze({
    icon: "icon-replace",
    title: () => I18n.t("setup-dialog-replace-title"),
    subtitle: () => I18n.t("setup-dialog-replace-subtitle"),
    submit: () => I18n.t("setup-dialog-replace-submit"),
    saved: () => I18n.t("setup-dialog-replace-saved"),
    endpoint: "/api/wallets/replace",
    confirm: true,
    replacesWallet: true,
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
    this.releaseEscape = null;
    // Setup from Explore Mode is on your own RPC; a replacement keeps the current connection.
    this.access = mode.replacesWallet
      ? getBootstrapState().status?.chain_access?.mode || "own_rpc"
      : "own_rpc";
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
            <span></span>
          </h2>
          <p class="setup-dialog-subtitle"></p>
        </div>
        <button type="button" class="setup-dialog-close" data-action="cancel"
          data-l10n-id="setup-dialog-close">
          <i class="icon-x"></i>
        </button>
      </div>

      <div class="setup-dialog-body">
        ${this.mode.confirm ? confirmStep({ rpc: this.access !== "service" }) : ""}
        <div class="setup-dialog-fields"${this.mode.confirm ? " hidden" : ""}>
          <div class="setup-dialog-source"></div>
          <div class="setup-dialog-backup" hidden></div>
          <div class="setup-dialog-wallet" hidden></div>
          <div class="setup-dialog-rpc"></div>
        </div>

        <div class="setup-dialog-status" id="setup-dialog-status" hidden></div>
      </div>

      <div class="setup-dialog-footer">
        <button type="button" class="btn btn-secondary" data-action="cancel"
          data-l10n-id="common-action-cancel"></button>
        <button type="button" class="btn btn-primary" data-action="submit"></button>
      </div>
    `;
    this.element.querySelector("#setup-dialog-title span").textContent = this.mode.title();
    this.element.querySelector(".setup-dialog-subtitle").textContent = this.mode.subtitle();
    this.element.querySelector('[data-action="submit"]').textContent = this.mode.confirm
      ? I18n.t("common-action-continue")
      : this.mode.submit();

    this.backdrop.appendChild(this.element);
    I18n.localizeTree(this.element);
    document.body.appendChild(this.backdrop);

    // The same wallet choice, backup panel and fields as the onboarding steps, under
    // ids of their own.
    const runtime = window.SetupRuntime;
    const part = (name) => this.element.querySelector(`.setup-dialog-${name}`);
    this.walletSource = null;
    this.source = runtime.mountWalletSource(part("source"), "setup-dialog-", {
      onChange: (value) => this.onWalletSourceChanged(value),
    });
    this.backup = runtime.mountBackupPanel(part("backup"), "setup-dialog-", {
      onChange: () => this.updateSubmit(),
    });
    this.walletField = runtime.mountWalletField(part("wallet"), "setup-dialog-");
    this.rpcField =
      this.access === "service"
        ? null
        : runtime.mountRpcField(part("rpc"), "setup-dialog-", {
            rpcOptional: this.mode.rpcOptional,
          });
    part("rpc").hidden = !this.rpcField;
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

    // Escape is consumed while a request is in flight so nothing beneath closes.
    this.releaseEscape = pushEscapeHandler(() => {
      if (!this.busy) this.close(false);
    });
    this.updateSubmit();
    if (this.confirmed) setTimeout(() => this.source.focus(), 50);
  }

  onWalletSourceChanged(value) {
    this.walletSource = value;
    // A new wallet is created as soon as it is chosen; choosing to import drops it.
    if (value === "create") this.backup.ensureGenerated();
    else this.backup.clear();
    this.element.querySelector(".setup-dialog-backup").hidden = value !== "create";
    this.element.querySelector(".setup-dialog-wallet").hidden = value !== "import";
    this.element.classList.toggle("is-wide", value === "create");
    if (value === "import") this.walletField.walletInput.focus();
    this.updateSubmit();
  }

  /** Submit waits for a source, and a created wallet for its three acknowledgements. */
  updateSubmit() {
    if (!this.submitBtn || this.busy || !this.confirmed) return;
    this.submitBtn.disabled =
      !this.walletSource || (this.walletSource === "create" && !this.backup.ready());
  }

  // Leave the confirmation step for the credential fields.
  showCredentials() {
    this.confirmed = true;
    this.element.querySelector(".setup-dialog-confirm")?.setAttribute("hidden", "");
    this.element.querySelector(".setup-dialog-fields")?.removeAttribute("hidden");
    this.submitBtn.textContent = this.mode.submit();
    this.updateSubmit();
    this.source.focus();
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
        : this.mode.submit();
    }
    if (!busy) this.updateSubmit();
  }

  async submit() {
    if (!this.walletSource) return;
    const created = this.walletSource === "create";
    if (created ? !this.backup.ready() : !this.walletField.validate()) return;
    if (this.rpcField && !this.rpcField.validate()) return;
    const walletPrivateKey = created ? this.backup.privateKey() : this.walletField.value();
    const rpcUrls = this.rpcField ? this.rpcField.rpcUrls() : [];

    const snapshot = Object.freeze({
      walletPrivateKey,
      rpcUrls: Object.freeze([...rpcUrls]),
      access: this.access,
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
          access: snapshot.access,
        }),
        signal: this.requestAbort.signal,
      });
      const validation = await validateRes.json();
      if (!validateRes.ok || !validation.valid || !validation.validation_id) {
        const msg =
          apiErrorMessage(validation) ||
          (validation?.errors?.length
            ? validation.errors.join(" ")
            : I18n.t("setup-dialog-error-validation"));
        this.setStatus("error", msg);
        this.setBusy(false);
        return;
      }
      if (validation.wallet_address && !created) {
        this.walletField.showWalletPreview(validation.wallet_address);
      }
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
          access: snapshot.access,
        }),
        signal: this.requestAbort.signal,
      });
      const result = await completeRes.json();
      if (!completeRes.ok || !result.restart_required) {
        const msg =
          apiErrorMessage(result) ||
          (result?.errors?.length
            ? result.errors.join(" ")
            : I18n.t("setup-dialog-error-incomplete"));
        this.setStatus("error", msg);
        this.setBusy(false);
        return;
      }

      // The key is stored encrypted now; nothing in the dialog keeps it.
      this.backup.clear();
      this.walletField.clear();
      this.setStatus("success", this.mode.saved());
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
    this.source?.dispose();
    this.backup?.dispose();
    this.walletField?.clear();
    this.walletField?.dispose();
    this.rpcField?.dispose();
    this.requestAbort?.abort();
    this.requestAbort = null;
    this.releaseEscape?.();
    this.releaseEscape = null;
    if (this.backdrop && this.backdrop.parentNode) {
      this.backdrop.parentNode.removeChild(this.backdrop);
    }
    if (SetupDialog.activeDialog === this) {
      SetupDialog.activeDialog = null;
    }
  }
}

export { SetupDialog };
