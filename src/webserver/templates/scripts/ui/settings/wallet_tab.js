// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Settings > Wallet: the trading wallet ScreenerBot signs with, its private-key
// export and its replacement; in Explore Mode, the way into setup.
//
// Every action opens the owner the Wallets page uses as well (the wallet key dialog,
// the Replace wallet dialog), so the two places cannot drift apart.

import * as Utils from "../../core/utils.js";
import { loadPage } from "../../core/router.js";
import { errorMessageOf, requestManager } from "../../core/request_manager.js";
import { openSetup, setupRequired, setupRequiredReason } from "../access_gate.js";
import { renderAddress } from "../token_identity.js";
import { SetupDialog } from "../setup_dialog.js";
import { showWalletKeyDialog } from "../wallet_key_dialog.js";

// Each attach counts up, so a read that returns after the tab was rebuilt is dropped.
let generation = 0;

/** The tab's markup; the address and balance fill in once the wallet is read. */
export function buildWalletTab() {
  if (setupRequired()) {
    return `
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-wallet"></i>
          <span data-l10n-id="settings-wallet-title"></span>
        </h3>
        <p class="settings-section-description">${Utils.escapeHtml(setupRequiredReason())}</p>
        <button type="button" class="btn btn-primary btn-sm" id="settingsWalletSetup"
          data-l10n-id="shell-explore-action"></button>
      </div>`;
  }
  return `
    <div class="settings-section">
      <h3 class="settings-section-title">
        <i class="icon-wallet"></i>
        <span data-l10n-id="settings-wallet-title"></span>
      </h3>
      <p class="settings-section-description" data-l10n-id="settings-wallet-description"></p>
      <div class="settings-group settings-wallet" aria-busy="true">
        <div class="settings-field settings-wallet-address">
          <div class="settings-field-info">
            <span class="settings-field-label" data-l10n-id="settings-wallet-address"></span>
            <span class="settings-wallet-value" id="settingsWalletAddress">—</span>
          </div>
        </div>
        <div class="settings-field">
          <div class="settings-field-info">
            <span class="settings-field-label" data-l10n-id="settings-wallet-balance"></span>
          </div>
          <div class="settings-field-control">
            <span class="settings-wallet-value" id="settingsWalletBalance">—</span>
          </div>
        </div>
      </div>
      <p class="settings-wallet-error" id="settingsWalletError" role="alert" hidden></p>
      <div class="settings-wallet-actions">
        <button type="button" class="btn btn-secondary btn-sm" id="settingsWalletOpen"
          data-l10n-id="settings-wallet-open"></button>
      </div>
    </div>

    <div class="settings-section">
      <h3 class="settings-section-title">
        <i class="icon-key"></i>
        <span data-l10n-id="settings-wallet-key-title"></span>
      </h3>
      <p class="settings-section-description" data-l10n-id="settings-wallet-key-description"></p>
      <button type="button" class="btn btn-secondary btn-sm" id="settingsWalletReveal"
        data-l10n-id="settings-wallet-key-reveal" disabled></button>
    </div>

    <div class="settings-section">
      <h3 class="settings-section-title">
        <i class="icon-replace"></i>
        <span data-l10n-id="settings-wallet-replace-title"></span>
      </h3>
      <p class="settings-section-description" data-l10n-id="settings-wallet-replace-description"></p>
      <button type="button" class="btn btn-secondary btn-sm" id="settingsWalletReplace"
        data-l10n-id="settings-wallet-replace-action"></button>
    </div>`;
}

/** Binds the tab's actions in `content` and reads the trading wallet. */
export function attachWalletHandlers(dialog, content) {
  const current = ++generation;
  const button = (id) => content.querySelector(`#${id}`);
  // Setup and the Wallets page open in place of Settings, which asks about unsaved changes first.
  const leaveFor = (open) => async () => {
    if (await dialog.requestClose()) open();
  };

  button("settingsWalletSetup")?.addEventListener("click", leaveFor(openSetup));
  if (setupRequired()) return;

  let walletId = null;
  button("settingsWalletOpen").addEventListener(
    "click",
    leaveFor(() => void loadPage("wallets"))
  );
  button("settingsWalletReveal").addEventListener("click", () => {
    if (walletId !== null) showWalletKeyDialog(walletId);
  });
  // Replacing the trading wallet restarts the app once it is staged.
  button("settingsWalletReplace").addEventListener("click", () => void SetupDialog.show());

  void readWallet().then((result) => {
    if (current !== generation || !content.isConnected) return;
    const group = content.querySelector(".settings-wallet");
    group.removeAttribute("aria-busy");
    if (!result.wallet) {
      const error = button("settingsWalletError");
      error.textContent = result.reason || I18n.t("settings-wallet-unread");
      error.hidden = false;
      return;
    }
    walletId = result.wallet.id;
    button("settingsWalletAddress").innerHTML = renderAddress(result.wallet.address, {
      explorer: "account",
    });
    button("settingsWalletBalance").textContent = Utils.formatSol(result.wallet.balance, {
      subscript: true,
    });
    button("settingsWalletReveal").disabled = false;
  });
}

/** The trading wallet's row from `/api/wallets`, or the reason it could not be read. */
async function readWallet() {
  try {
    const data = await requestManager.fetch("/api/wallets", {
      priority: "normal",
      skipDedup: true,
      cache: "no-store",
    });
    return { wallet: (data?.wallets || []).find((wallet) => wallet.role === "main") || null };
  } catch (error) {
    return { wallet: null, reason: await errorMessageOf(error, I18n.t("settings-wallet-unread")) };
  }
}
