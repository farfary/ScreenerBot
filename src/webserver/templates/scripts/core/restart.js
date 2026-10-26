// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Confirmed restart of the ScreenerBot process from the dashboard: asks first, requests the restart, and reloads the page once the new process answers.

import * as Utils from "./utils.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";

/** Asks to restart ScreenerBot, then restarts it and waits for the new process. */
export async function restartApp() {
  const { confirmed } = await ConfirmationDialog.show({
    title: I18n.t("shell-restart-confirm-title"),
    message: I18n.t("shell-restart-confirm-message"),
    confirmLabel: I18n.t("shell-restart-confirm-action"),
    cancelLabel: I18n.t("common-action-cancel"),
    variant: "warning",
  });

  if (!confirmed) return;

  try {
    // ONE notice for the whole restart: it is replaced in place if the restart
    // fails, and the page reloads out from under it when it succeeds.
    Utils.showToast({
      key: "system-restart",
      type: "progress",
      title: I18n.t("shell-restart-progress"),
    });

    const res = await fetch("/api/system/reboot", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });

    if (!res.ok) {
      throw new Error(I18n.t("shell-restart-failed-status", { status: String(res.status) }));
    }

    const result = await res.json();

    const waitForRestart = window.waitForScreenerBotRestart;
    if (typeof waitForRestart !== "function") {
      throw new Error(I18n.t("shell-restart-helper-unavailable"));
    }
    await waitForRestart(result.instance_id, { target: window.location.pathname || "/home" });
  } catch (err) {
    console.error("[Restart] Restart failed:", err);
    Utils.showToast({
      key: "system-restart",
      type: "error",
      title: I18n.t("shell-restart-failed"),
      message: err.message,
    });
  }
}
