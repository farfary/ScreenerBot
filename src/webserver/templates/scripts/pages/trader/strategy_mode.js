// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
//! The trading mode of a Strategy Control card: the book its new entries open on, the gated step that arms a paper strategy live, and the return to paper.

/*
 * Arming live shows the strategy's readiness from its paper rounds and asks for every
 * acknowledgement plus the exact confirmation phrase the API names; the request carries
 * that phrase, so the server applies the same gate.
 */

import { tradingModeMarker } from "../../ui/trading_mode.js";
import {
  ackList,
  pendingAcks,
  phraseField,
  phraseMatches,
  readinessList,
  runtimeBlocked,
  unavailableNote,
} from "../../ui/arm_live_gate.js";

/** The mode row of a strategy card: the mode as text and the action that changes it. */
export function strategyModeRow(strategy, esc) {
  const paper = strategy.mode === "paper";
  const id = esc(String(strategy.id));
  const action = paper
    ? `<button type="button" class="btn btn-sm btn-secondary" data-arm-strategy="${id}">${esc(
        I18n.t("trader-strategy-mode-arm")
      )}</button>`
    : `<button type="button" class="btn btn-sm btn-ghost" data-paper-strategy="${id}">${esc(
        I18n.t("trader-strategy-mode-return-paper")
      )}</button>`;
  return `<div class="strategy-control-mode">${tradingModeMarker(strategy.mode, esc)}${action}</div>`;
}

/**
 * The arm dialog's body for `readiness` (the readiness API response): the checks, what
 * happens to open paper rounds, then either the acknowledgements and the phrase field or
 * the reason live trading cannot run.
 */
export function armDialogBody(readiness, esc) {
  const checks = `<h5 class="arm-gate-title">${esc(
    I18n.t("trader-strategy-arm-readiness")
  )}</h5>${readinessList(readiness?.checks, esc)}<p class="arm-gate-note">${esc(
    I18n.t("trader-strategy-arm-open-paper")
  )}</p>`;
  if (runtimeBlocked(readiness)) {
    return `${checks}${unavailableNote(I18n.t("trader-strategy-arm-unavailable"), esc)}`;
  }
  const acks = ackList(
    [
      I18n.t("trader-strategy-arm-ack-real-native"),
      I18n.t("trader-strategy-arm-ack-fees"),
      readiness.ready ? null : I18n.t("trader-strategy-arm-ack-unready"),
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
    strategy.name ? String(strategy.name) : I18n.t("trader-strategy-unnamed");

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
      console.error("[Trader] Failed to change the strategy mode:", error);
      Utils.showToast({
        type: "error",
        title: I18n.t("trader-toast-strategy-mode-failed"),
        message: error?.message || "",
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
      console.error("[Trader] Failed to read the strategy readiness:", error);
      Utils.showToast({ type: "error", title: I18n.t("trader-strategy-readiness-failed") });
      return false;
    }
    const choice = await ConfirmationDialog.show({
      title: I18n.t("trader-strategy-arm-title", { name: name(strategy) }),
      message: I18n.t("trader-strategy-arm-lead", { name: name(strategy) }),
      body: armDialogBody(readiness, esc),
      confirmLabel: I18n.t("trader-strategy-mode-arm"),
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
        title: I18n.t("trader-toast-strategy-armed", { name: name(strategy) }),
      });
    }
    return changed;
  }

  async function returnToPaper(strategy) {
    const choice = await ConfirmationDialog.show({
      title: I18n.t("trader-strategy-return-title", { name: name(strategy) }),
      message: I18n.t("trader-strategy-return-message"),
      confirmLabel: I18n.t("trader-strategy-mode-return-paper"),
      variant: "info",
    });
    if (!choice?.confirmed) return false;
    const changed = await setMode(strategy, { mode: "paper" });
    if (changed) {
      Utils.showToast({
        type: "success",
        title: I18n.t("trader-toast-strategy-paper", { name: name(strategy) }),
      });
    }
    return changed;
  }

  return { armLive, returnToPaper };
}
