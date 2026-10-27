// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Funding guidance for a trading wallet that holds no SOL: what a trade needs and the address to send SOL to, with copy.
import { requestManager } from "../core/request_manager.js";
import { renderAddress } from "./token_identity.js";
import { renderNotice } from "./notice.js";

/** True when the balance is known and the wallet holds no native coin at all. */
export function needsFunding(balance) {
  return typeof balance === "number" && Number.isFinite(balance) && balance <= 0;
}

/** The notice for `address`; without an address it still says what is needed. */
export function renderFundingNotice(address) {
  return renderNotice({
    variant: "info",
    icon: "icon-wallet",
    title: I18n.t("wallets-funding-title"),
    message: I18n.t("wallets-funding-message"),
    extraHtml: address ? renderAddress(address, { explorer: "account" }) : "",
  });
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
