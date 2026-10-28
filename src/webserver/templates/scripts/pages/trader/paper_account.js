// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
//! The paper run behind the Trader Stats tab's Paper book: reading it, and the reset that starts the next run.

import { showToast } from "../../core/utils.js";

const ACCOUNT_URL = "/api/positions/paper/account";

/**
 * Reads the paper run (`{epoch, started_at, starting_balance_native, cash_native}`) and
 * runs its reset. `onReset` runs after a new run started.
 */
export function createPaperAccount({ requestManager, ConfirmationDialog, onReset }) {
  /** The current run, or `null` when it could not be read. */
  async function load() {
    try {
      return await requestManager.fetch(ACCOUNT_URL, { priority: "normal" });
    } catch (error) {
      console.error("[Trader] Failed to load the paper run:", error);
      return null;
    }
  }

  /** Confirm, then close every paper position and start a new run at the configured balance. */
  async function reset() {
    const choice = await ConfirmationDialog.show({
      title: I18n.t("trader-paper-reset-title"),
      message: I18n.t("trader-paper-reset-run-message"),
      confirmLabel: I18n.t("trader-paper-reset"),
      variant: "warning",
    });
    if (!choice?.confirmed) return;
    try {
      await requestManager.fetch(ACCOUNT_URL, { method: "POST", priority: "high" });
      showToast({ type: "success", title: I18n.t("trader-paper-reset-done") });
      onReset?.();
    } catch (error) {
      console.error("[Trader] Failed to reset the paper run:", error);
      showToast({
        type: "error",
        title: I18n.t("trader-paper-reset-failed"),
        message: error?.message || "",
      });
    }
  }

  return { load, reset };
}
