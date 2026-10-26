// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Funding guidance for a trading wallet that holds no SOL: what a trade needs and the address to send SOL to, with copy.
import * as Utils from "../core/utils.js";
import { requestManager } from "../core/request_manager.js";
import { renderAddress } from "./token_identity.js";

/** True when the balance is known and the wallet holds no native coin at all. */
export function needsFunding(balance) {
  return typeof balance === "number" && Number.isFinite(balance) && balance <= 0;
}

/** The notice for `address`; without an address it still says what is needed. */
export function renderFundingNotice(address) {
  return `<div class="funding-notice" role="note">
    <p class="funding-notice-title"><i class="icon-wallet" aria-hidden="true"></i>${Utils.escapeHtml(I18n.t("wallets-funding-title"))}</p>
    <p class="funding-notice-message">${Utils.escapeHtml(I18n.t("wallets-funding-message"))}</p>
    ${address ? `<div class="funding-notice-address">${renderAddress(address, { explorer: "account" })}</div>` : ""}
  </div>`;
}

/** The main trading wallet's address, or null when it cannot be read. */
export async function fetchMainWalletAddress() {
  try {
    const data = await requestManager.fetch("/api/wallets", { priority: "normal" });
    return (data?.wallets || []).find((wallet) => wallet.role === "main")?.address || null;
  } catch {
    return null;
  }
}
