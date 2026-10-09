// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Explore Mode gate for the wallet-backed pages: whether a full setup is still
// required, the one translated reason a control is unavailable, and the shared
// notice that states it and opens setup.
import { getBootstrapState } from "../core/bootstrap.js";
import { SetupDialog } from "./setup_dialog.js";
import { renderStateView } from "./state_view.js";

/** True while the app runs in Explore Mode, without a wallet or RPC. */
export function setupRequired() {
  return Boolean(getBootstrapState().status?.explore_mode);
}

/** The reason a wallet-backed control is unavailable in Explore Mode. */
export function setupRequiredReason() {
  return I18n.t("shell-setup-gate-detail");
}

/**
 * Marks a wallet-backed control unavailable in Explore Mode and names the reason
 * in its tooltip; outside Explore Mode it leaves the control as the page set it.
 */
export function gateControl(control) {
  if (!control || !setupRequired()) return;
  control.disabled = true;
  control.title = setupRequiredReason();
}

/** Paints the notice into `container`: a title, the reason and the setup action. */
export function renderSetupGate(container, title) {
  if (!container) return;
  container.innerHTML = `<div class="setup-gate">${renderStateView({
    icon: "icon-wallet",
    title,
    message: setupRequiredReason(),
    action: { id: "open-setup", label: I18n.t("shell-explore-action"), primary: true },
    compact: true,
  })}</div>`;
  container.querySelector(".state-view-action").addEventListener("click", () => SetupDialog.show());
}
