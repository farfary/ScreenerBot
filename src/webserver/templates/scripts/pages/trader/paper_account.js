// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
//! The paper run card of the Trader Stats tab: when the run began, its starting balance, cash, open cost and unrealized result, and the reset that starts the next run.

import { formatDate, formatSignedSol, formatSol } from "../../core/format.js";

const DASH = "—";

/**
 * The card markup for `account` (the `/api/positions/paper/account` response). A figure
 * the account does not carry renders as a dash rather than a fabricated zero.
 */
export function paperAccountCard(account, esc) {
  const sol = (value) => formatSol(value, { decimals: 4, fallback: DASH });
  const row = (label, value) => `<div><dt>${esc(label)}</dt><dd>${esc(value)}</dd></div>`;
  const since = Number.isFinite(account?.started_at)
    ? formatDate(account.started_at, { fallback: DASH })
    : DASH;
  return `<i class="icon-test-tube-diagonal" aria-hidden="true"></i><span>${esc(
    I18n.t("trader-paper-run-since", { date: since })
  )}</span><dl>${row(I18n.t("trader-paper-start"), sol(account?.starting_balance_native))}${row(
    I18n.t("trader-paper-cash"),
    sol(account?.cash_native)
  )}${row(I18n.t("trader-paper-open"), sol(account?.open_cost_native))}${row(
    I18n.t("trader-paper-unrealized"),
    formatSignedSol(account?.open_unrealized_pnl_native, { decimals: 4, fallback: DASH })
  )}</dl><button type="button" class="btn btn-sm btn-ghost" data-paper-reset>${esc(
    I18n.t("trader-paper-reset")
  )}</button>`;
}

/**
 * Loads the paper run into its card and runs the reset. `deps` carries the page's request
 * manager, dialog and utilities; `onReset` runs after a new run started.
 */
export function createPaperAccount({ requestManager, ConfirmationDialog, Utils, onReset }) {
  const esc = Utils.escapeHtml;
  const ACCOUNT_URL = "/api/positions/paper/account";

  function paint(container, account) {
    container.innerHTML = paperAccountCard(account, esc);
    container.querySelector("[data-paper-reset]")?.addEventListener("click", () => {
      void reset(container);
    });
  }

  async function load(container) {
    if (!container) return;
    try {
      paint(container, await requestManager.fetch(ACCOUNT_URL, { priority: "normal" }));
    } catch (error) {
      console.error("[Trader] Failed to load the paper run:", error);
      container.textContent = I18n.t("trader-paper-account-failed");
    }
  }

  async function reset(container) {
    const choice = await ConfirmationDialog.show({
      title: I18n.t("trader-paper-reset-title"),
      message: I18n.t("trader-paper-reset-run-message"),
      confirmLabel: I18n.t("trader-paper-reset"),
      variant: "warning",
    });
    if (!choice?.confirmed) return;
    try {
      const account = await requestManager.fetch(ACCOUNT_URL, {
        method: "POST",
        priority: "high",
      });
      paint(container, account);
      Utils.showToast({ type: "success", title: I18n.t("trader-paper-reset-done") });
      onReset?.();
    } catch (error) {
      console.error("[Trader] Failed to reset the paper run:", error);
      Utils.showToast({
        type: "error",
        title: I18n.t("trader-paper-reset-failed"),
        message: error?.message || "",
      });
    }
  }

  return { load };
}
