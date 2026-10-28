// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The private-key export of one wallet, opened from Wallets and from Settings > Wallet: a warning first, the key decrypted only on an explicit reveal, and the key removed from the page whenever the dialog closes.
import * as Utils from "../core/utils.js";
import { createFocusTrap } from "../core/utils.js";
import { pushEscapeHandler } from "../core/escape_stack.js";
import { apiErrorMessage } from "../core/request_manager.js";
import { renderNotice } from "./notice.js";

let active = null;

/** Opens the export dialog of wallet `walletId`, replacing one already open. */
export function showWalletKeyDialog(walletId) {
  active?.close();
  active = createDialog(walletId);
}

function createDialog(walletId) {
  const overlay = document.createElement("div");
  overlay.className = "modal-overlay wallet-key-dialog";
  overlay.innerHTML = `
    <div class="modal-dialog modal-sm" role="dialog" aria-modal="true"
      aria-labelledby="wallet-key-dialog-title">
      <div class="modal-header">
        <h2 id="wallet-key-dialog-title">
          <i class="icon-key"></i> <span data-l10n-id="wallets-export-title"></span>
        </h2>
        <button class="modal-close" type="button" data-action="close"
          data-l10n-id="wallets-modal-close">
          <i class="icon-x"></i>
        </button>
      </div>
      <div class="modal-body">
        ${renderNotice({
          variant: "error",
          icon: "icon-octagon-alert",
          title: I18n.t("wallets-export-warning-title"),
          message: I18n.t("wallets-export-warning-body"),
        })}
        <div class="wallet-key-reveal" hidden>
          <span class="wallet-key-label" data-l10n-id="wallets-export-key-label"></span>
          <div class="wallet-key-value">
            <code dir="ltr"></code>
            <button type="button" class="btn-icon" data-action="copy"
              data-l10n-id="wallets-export-copy">
              <i class="icon-copy"></i>
            </button>
          </div>
        </div>
        <div class="wallet-key-actions">
          <button type="button" class="btn btn-secondary" data-action="close"
            data-l10n-id="common-action-cancel"></button>
          <button type="button" class="btn btn-danger" data-action="reveal">
            <i class="icon-lock-open"></i> <span data-l10n-id="wallets-export-reveal"></span>
          </button>
        </div>
      </div>
    </div>`;
  I18n.localizeTree(overlay);
  document.body.appendChild(overlay);

  const panel = overlay.querySelector(".modal-dialog");
  const reveal = overlay.querySelector('[data-action="reveal"]');
  const keyEl = overlay.querySelector(".wallet-key-value code");
  const trap = createFocusTrap(panel);
  let closed = false;

  const dialog = {
    close() {
      if (closed) return;
      closed = true;
      releaseEscape();
      trap.deactivate();
      // Removing the overlay drops the only copy of the key the page held.
      overlay.remove();
      if (active === dialog) active = null;
    },
  };
  const releaseEscape = pushEscapeHandler(() => dialog.close());

  overlay.addEventListener("click", (event) => {
    if (event.target === overlay) {
      dialog.close();
      return;
    }
    const action = event.target.closest("[data-action]")?.dataset.action;
    if (action === "close") dialog.close();
    else if (action === "reveal") void revealKey();
    else if (action === "copy" && keyEl.textContent) {
      Utils.copyToClipboard(keyEl.textContent);
      Utils.notifyCopied(I18n.t("wallets-copied-private-key"));
    }
  });

  async function revealKey() {
    reveal.disabled = true;
    const icon = document.createElement("i");
    icon.className = "icon-loader spin";
    const label = document.createElement("span");
    label.textContent = I18n.t("wallets-export-busy");
    reveal.replaceChildren(icon, " ", label);
    try {
      const response = await fetch(`/api/wallets/${walletId}/export`, { method: "POST" });
      const data = await response.json();
      if (!response.ok) throw new Error(apiErrorMessage(data, I18n.t("wallets-action-failed")));
      if (closed) return;
      keyEl.textContent = data.private_key;
      overlay.querySelector(".wallet-key-reveal").hidden = false;
      reveal.hidden = true;
      Utils.showToast(I18n.t("wallets-export-revealed"), "warning");
    } catch (error) {
      Utils.showToast(I18n.t("wallets-toast-failed", { reason: error.message }), "error");
      dialog.close();
    }
  }

  trap.activate();
  return dialog;
}
