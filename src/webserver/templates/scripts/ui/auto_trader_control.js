// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The one path every dashboard control takes to turn the Auto Trader on or off; turning it on asks first.

import { ConfirmationDialog } from "./confirmation_dialog.js";
import { fetchTraderDefaults } from "./manual_trade.js";
import { requestManager } from "../core/request_manager.js";
import { formatSignatureCompact, formatSol } from "../core/format.js";
import { COMPACT_ADDRESS } from "./token_identity.js";

const START_ENDPOINT = "/api/trader/start";
const STOP_ENDPOINT = "/api/trader/stop";

/** The main wallet's address, or null when it cannot be read. */
async function fetchTradingWalletAddress() {
  try {
    const wallet = await requestManager.fetch("/api/wallets/main", { priority: "high" });
    return typeof wallet?.address === "string" && wallet.address ? wallet.address : null;
  } catch {
    return null;
  }
}

/**
 * The confirmation text: the wallet the trader will spend from and the size of each
 * trade, each named only when it could be read.
 */
async function startConfirmationMessage() {
  const [address, defaults] = await Promise.all([
    fetchTradingWalletAddress(),
    fetchTraderDefaults(),
  ]);
  if (!address) return I18n.t("shell-trader-start-confirm-message-plain");
  const wallet = formatSignatureCompact(address, COMPACT_ADDRESS);
  if (defaults.tradeSize === null) {
    return I18n.t("shell-trader-start-confirm-message-unsized", { wallet });
  }
  return I18n.t("shell-trader-start-confirm-message", {
    wallet,
    size: formatSol(defaults.tradeSize, { decimals: 9 }),
  });
}

/** Asks before live trading starts. Resolves true only on an explicit confirm. */
async function confirmStart() {
  const { confirmed } = await ConfirmationDialog.show({
    title: I18n.t("shell-trader-start-confirm-title"),
    message: await startConfirmationMessage(),
    confirmLabel: I18n.t("shell-trader-start-confirm-action"),
    cancelLabel: I18n.t("common-action-cancel"),
    variant: "warning",
  });
  return confirmed;
}

/** True while a start confirmation is open, so a second start request cannot stack a second one. */
let startPending = false;

/**
 * Turns the Auto Trader on (`enabled`) or off. Turning it on resolves
 * `{ cancelled: true }` without a request when the confirmation is declined or one is
 * already open; turning it off never asks. Otherwise `onRequest` runs just before the
 * request goes out, and the control response resolves with its parsed JSON body (null
 * when the body is not JSON) for the caller to report.
 *
 * @param {boolean} enabled
 * @param {{onRequest?: () => void}} [options]
 * @returns {Promise<{cancelled: true} | {cancelled: false, response: Response, payload: object | null}>}
 */
export async function setAutoTraderEnabled(enabled, { onRequest } = {}) {
  if (enabled) {
    if (startPending) return { cancelled: true };
    startPending = true;
    try {
      if (!(await confirmStart())) return { cancelled: true };
    } finally {
      startPending = false;
    }
  }
  onRequest?.();
  const response = await fetch(enabled ? START_ENDPOINT : STOP_ENDPOINT, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Requested-With": "fetch",
    },
    cache: "no-store",
  });
  const payload = await response.json().catch(() => null);
  return { cancelled: false, response, payload };
}
