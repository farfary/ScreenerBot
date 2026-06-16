/**
 * Setup Dialog — wallet + RPC credential entry as a modal.
 *
 * Used to complete setup from preview mode (header banner / config tab) without the
 * full-screen wizard. Validates via /api/initialization/validate and persists via
 * /api/initialization/complete, then reloads into the fully-configured dashboard.
 *
 * Usage: SetupDialog.show() -> Promise<boolean> (true if setup completed).
 */

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
          <h2 class="setup-dialog-title" id="setup-dialog-title">Set up wallet &amp; RPC</h2>
          <p class="setup-dialog-subtitle">
            Connect your Solana wallet and a premium RPC endpoint to enable trading and live
            on-chain data. Your private key is encrypted on this device and never leaves it.
          </p>
        </div>
        <button type="button" class="setup-dialog-close" data-action="cancel" title="Close" aria-label="Close">
          <i class="icon-x"></i>
        </button>
      </div>

      <div class="setup-dialog-body">
        <label class="setup-dialog-field">
          <span class="setup-dialog-label">Wallet private key <span class="req">*</span></span>
          <div class="setup-dialog-input-wrap">
            <textarea id="setup-dialog-wallet" class="setup-dialog-input" rows="2"
              placeholder="Base58 string or JSON array [1,2,3,...]" spellcheck="false"
              autocomplete="off"></textarea>
            <button type="button" class="setup-dialog-reveal" data-action="reveal" title="Show / hide">
              <i class="icon-eye"></i>
            </button>
          </div>
          <span class="setup-dialog-hint" id="setup-dialog-wallet-hint"></span>
        </label>

        <label class="setup-dialog-field">
          <span class="setup-dialog-label">RPC endpoint(s) <span class="req">*</span></span>
          <textarea id="setup-dialog-rpc" class="setup-dialog-input" rows="2"
            placeholder="https://your-endpoint... (one per line)" spellcheck="false"></textarea>
          <span class="setup-dialog-hint">
            A premium provider (Helius, QuickNode, Alchemy) is strongly recommended — the public
            Solana RPC is rate-limited and may not work.
          </span>
        </label>

        <div class="setup-dialog-status" id="setup-dialog-status" hidden></div>
      </div>

      <div class="setup-dialog-footer">
        <button type="button" class="setup-dialog-btn secondary" data-action="cancel">Cancel</button>
        <button type="button" class="setup-dialog-btn primary" data-action="submit">
          Validate &amp; connect
        </button>
      </div>
    `;

    this.backdrop.appendChild(this.element);
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
    this.element.querySelector('[data-action="submit"]').addEventListener("click", () => this.submit());
    this.element.querySelector('[data-action="reveal"]').addEventListener("click", (e) => {
      const masked = this.walletInput.style.webkitTextSecurity !== "none";
      this.walletInput.style.webkitTextSecurity = masked ? "none" : "disc";
      e.currentTarget.querySelector("i").className = masked ? "icon-eye-off" : "icon-eye";
    });
    this.walletInput.style.webkitTextSecurity = "disc";

    document.addEventListener("keydown", this.onKeyDown);
    setTimeout(() => this.walletInput.focus(), 50);
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

  setStatus(kind, html) {
    if (!this.statusEl) return;
    this.statusEl.hidden = false;
    this.statusEl.className = `setup-dialog-status ${kind}`;
    this.statusEl.innerHTML = html;
  }

  setBusy(busy, label) {
    this.busy = busy;
    if (this.submitBtn) {
      this.submitBtn.disabled = busy;
      this.submitBtn.innerHTML = busy
        ? `<i class="icon-loader"></i> ${label || "Working..."}`
        : "Validate &amp; connect";
    }
  }

  async submit() {
    const walletPrivateKey = (this.walletInput.value || "").trim();
    const rpcUrls = this.parseRpcUrls();

    if (!walletPrivateKey || rpcUrls.length === 0) {
      this.setStatus("error", '<i class="icon-circle-x"></i> Enter both a wallet private key and at least one RPC URL.');
      return;
    }

    try {
      this.setBusy(true, "Validating...");
      const validateRes = await fetch("/api/initialization/validate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ wallet_private_key: walletPrivateKey, rpc_urls: rpcUrls }),
      });
      const validation = await validateRes.json();
      if (!validateRes.ok || !validation.valid) {
        const msg = validation?.errors?.length ? validation.errors.join("; ") : "Validation failed.";
        this.setStatus("error", `<i class="icon-circle-x"></i> ${msg}`);
        this.setBusy(false);
        return;
      }
      if (validation.warnings?.length) {
        this.setStatus("warning", `<i class="icon-triangle-alert"></i> ${validation.warnings.join("; ")}`);
      }

      this.setBusy(true, "Connecting...");
      const completeRes = await fetch("/api/initialization/complete", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ wallet_private_key: walletPrivateKey, rpc_urls: rpcUrls }),
      });
      const result = await completeRes.json();
      if (!completeRes.ok || !result.success) {
        const msg = result?.errors?.length ? result.errors.join("; ") : "Setup could not be completed.";
        this.setStatus("error", `<i class="icon-circle-x"></i> ${msg}`);
        this.setBusy(false);
        return;
      }

      this.setStatus("success", '<i class="icon-circle-check"></i> Connected — starting services and loading your dashboard...');
      this.close(true);
      setTimeout(() => {
        window.location.href = "/home";
      }, 900);
    } catch (err) {
      this.setStatus("error", `<i class="icon-circle-x"></i> ${err?.message || "Unexpected error."}`);
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
