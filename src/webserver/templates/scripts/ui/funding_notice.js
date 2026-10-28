// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Funding guidance for a trading wallet that holds no SOL: what a trade needs, the address to send SOL to with copy and its QR code, and the balance watch that sees the deposit arrive.
import { requestManager } from "../core/request_manager.js";
import { Poller } from "../core/poller.js";
import { renderAddress } from "./token_identity.js";
import { renderNotice } from "./notice.js";

/** True when the balance is known and the wallet holds no native coin at all. */
export function needsFunding(balance) {
  return typeof balance === "number" && Number.isFinite(balance) && balance <= 0;
}

/**
 * The notice for `address`; without an address it still says what is needed. A page that
 * already shows the wallet's address beside the notice passes none.
 */
export function renderFundingNotice(address) {
  return renderNotice({
    variant: "info",
    icon: "icon-wallet",
    title: I18n.t("wallets-funding-title"),
    message: I18n.t("wallets-funding-message"),
    extraHtml: address ? renderAddress(address, { explorer: "account" }) : "",
  });
}

/** How often the balance watch reads the trading wallet while it waits for a deposit. */
export const FUNDING_POLL_MS = 10000;

/** The main trading wallet's row from `/api/wallets`, or null when it cannot be read. */
async function fetchMainWallet(options = {}) {
  try {
    const data = await requestManager.fetch("/api/wallets", { priority: "normal", ...options });
    return (data?.wallets || []).find((wallet) => wallet.role === "main") || null;
  } catch {
    return null;
  }
}

/** The main trading wallet's address, or null when it cannot be read. */
export async function fetchMainWalletAddress() {
  return (await fetchMainWallet())?.address || null;
}

/** The main trading wallet's SOL balance, or null when it cannot be read yet. */
export async function fetchMainWalletBalance() {
  const balance = (await fetchMainWallet({ skipDedup: true, cache: "no-store" }))?.balance;
  return typeof balance === "number" && Number.isFinite(balance) ? balance : null;
}

/**
 * The QR code of `address` as an image data URL. `fetcher` is the caller's request
 * function (a page's scoped fetcher), so the read ends with the page; it throws when
 * the code cannot be made.
 */
export async function fetchWalletQr(
  address,
  fetcher = (path, options) => requestManager.fetch(path, options)
) {
  const data = await fetcher(`/api/wallet/qr/${encodeURIComponent(address)}`, {
    priority: "high",
    cache: "no-store",
  });
  return data.qr_data_url;
}

/**
 * Reads the trading wallet's balance now and every `FUNDING_POLL_MS` after, handing
 * each reading (null while unreadable) to `onBalance`. Returns the stop function;
 * leaving the page stops the watch as well.
 */
export function watchMainWalletBalance(onBalance) {
  let stopped = false;
  const read = async () => {
    const balance = await fetchMainWalletBalance();
    if (!stopped) onBalance(balance);
  };
  const poller = new Poller(read, {
    label: "FundingBalance", // l10n-ignore: internal poller name
    intervalMs: FUNDING_POLL_MS,
    pauseWhenHidden: false,
  });
  const stop = () => {
    if (stopped) return;
    stopped = true;
    poller.cleanup();
    window.removeEventListener("pagehide", stop);
  };
  window.addEventListener("pagehide", stop);
  void read();
  poller.start();
  return stop;
}
