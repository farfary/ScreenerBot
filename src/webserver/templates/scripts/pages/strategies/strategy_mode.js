// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The trading mode of a strategy: the action that changes it, the gated step that arms a paper strategy live, and the return to paper.

/*
 * Arming live shows the strategy's readiness from its paper rounds and asks for every
 * acknowledgement plus the exact confirmation phrase the API names; the request carries
 * that phrase, so the server applies the same gate.
 */

import { errorMessageOf } from "../../core/request_manager.js";
import {
  ackList,
  pendingAcks,
  phraseField,
  phraseMatches,
  readinessList,
  runtimeBlocked,
  unavailableNote,
} from "../../ui/arm_live_gate.js";

/**
 * The action that changes the mode of `strategy`: Arm live on paper, Return to paper on
 * live, and none while the strategy is off, since an off strategy trades on neither book.
 */
export function modeAction(strategy) {
  if (!strategy?.enabled) return null;
  return strategy.mode === "paper"
    ? { id: "arm", label: I18n.t("strategies-mode-arm") }
    : { id: "return-paper", label: I18n.t("strategies-mode-return-paper") };
}

/** True for an exit strategy, whose live mode sells positions rather than opening them. */
const isExit = (strategy) => strategy?.strategy_type === "EXIT" || strategy?.type === "EXIT";

/**
 * The arm dialog's body for `readiness` (the readiness API response): the checks, what
 * happens to open paper rounds, then either the acknowledgements and the phrase field or
 * the reason live trading cannot run.
 */
export function armDialogBody(readiness, esc, { exit = false } = {}) {
  const checks = `<h5 class="arm-gate-title">${esc(
    I18n.t("strategies-mode-arm-readiness")
  )}</h5>${readinessList(readiness?.checks, esc)}<p class="arm-gate-note">${esc(
    I18n.t("strategies-mode-arm-open-paper")
  )}</p>`;
  if (runtimeBlocked(readiness)) {
    return `${checks}${unavailableNote(I18n.t("strategies-mode-arm-unavailable"), esc)}`;
  }
  const acks = ackList(
    [
      I18n.t(exit ? "strategies-mode-arm-ack-real-native-exit" : "strategies-mode-arm-ack-real-native"),
      I18n.t("strategies-mode-arm-ack-fees"),
      readiness.ready ? null : I18n.t("strategies-mode-arm-ack-unready"),
    ],
    esc
  );
  return `${checks}${acks}${phraseField(readiness.live_confirmation, esc)}`;
}

/** True when the dialog under `root` may arm live: runtime ready, all ticked, phrase typed. */
export function armAllowed(root, readiness) {
  return (
    !runtimeBlocked(readiness) &&
    pendingAcks(root) === 0 &&
    phraseMatches(root, readiness?.live_confirmation)
  );
}

/**
 * The arm and return actions. `deps` carries the page's request manager, dialog, toast
 * and escaping; each action resolves to true when the strategy's mode changed.
 */
export function createStrategyMode({ requestManager, ConfirmationDialog, Utils }) {
  const esc = Utils.escapeHtml;
  const modeUrl = (strategy) => `/api/strategies/${encodeURIComponent(strategy.id)}/mode`;
  const name = (strategy) =>
    strategy.name ? String(strategy.name) : I18n.t("strategies-unnamed");

  async function setMode(strategy, body) {
    try {
      await requestManager.fetch(modeUrl(strategy), {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
        priority: "high",
      });
      return true;
    } catch (error) {
      console.error("[Strategies] Failed to change the strategy mode:", error);
      Utils.showToast({
        type: "error",
        title: I18n.t("strategies-mode-toast-failed"),
        message: await errorMessageOf(error, ""),
      });
      return false;
    }
  }

  async function armLive(strategy) {
    let readiness;
    try {
      readiness = await requestManager.fetch(
        `/api/strategies/${encodeURIComponent(strategy.id)}/readiness`,
        { priority: "high" }
      );
    } catch (error) {
      console.error("[Strategies] Failed to read the strategy readiness:", error);
      Utils.showToast({
        type: "error",
        title: I18n.t("strategies-mode-readiness-failed"),
        message: await errorMessageOf(error, ""),
      });
      return false;
    }
    const choice = await ConfirmationDialog.show({
      title: I18n.t("strategies-mode-arm-title", { name: name(strategy) }),
      message: I18n.t(isExit(strategy) ? "strategies-mode-arm-lead-exit" : "strategies-mode-arm-lead", {
        name: name(strategy),
      }),
      body: armDialogBody(readiness, esc, { exit: isExit(strategy) }),
      confirmLabel: I18n.t("strategies-mode-arm"),
      variant: "danger",
      canConfirm: (element) => armAllowed(element, readiness),
    });
    if (!choice?.confirmed) return false;
    const changed = await setMode(strategy, {
      mode: "live",
      confirmation: readiness.live_confirmation,
    });
    if (changed) {
      Utils.showToast({
        type: "success",
        title: I18n.t("strategies-mode-toast-armed", { name: name(strategy) }),
      });
    }
    return changed;
  }

  async function returnToPaper(strategy) {
    const choice = await ConfirmationDialog.show({
      title: I18n.t("strategies-mode-return-title", { name: name(strategy) }),
      message: I18n.t(
        isExit(strategy) ? "strategies-mode-return-message-exit" : "strategies-mode-return-message"
      ),
      confirmLabel: I18n.t("strategies-mode-return-paper"),
      variant: "info",
    });
    if (!choice?.confirmed) return false;
    const changed = await setMode(strategy, { mode: "paper" });
    if (changed) {
      Utils.showToast({
        type: "success",
        title: I18n.t("strategies-mode-toast-paper", { name: name(strategy) }),
      });
    }
    return changed;
  }

  return { armLive, returnToPaper };
}
