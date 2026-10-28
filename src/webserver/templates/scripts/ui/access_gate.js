// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Access gate for wallet-backed and chain-backed controls: whether a full setup is
// still required (Explore Mode), whether the chain connection provides a capability,
// the one translated reason a control is unavailable, and the shared notices that
// state it and open setup or Settings > Connection.
import { getBootstrapState } from "../core/bootstrap.js";
import { renderStateView } from "./state_view.js";
import { renderNotice } from "./notice.js";
import { showSettingsDialog } from "./settings_dialog.js";

/**
 * The reason each capability the ScreenerBot service does not provide is
 * unavailable; a capability without its own sentence names the requirement only.
 */
const CAPABILITY_REASON_LABELS = Object.freeze({
  live_pool_prices: "shell-access-gate-live-pool-prices",
  direct_pool_swaps: "shell-access-gate-direct-swaps",
  watch_other_wallets: "shell-access-gate-watch-wallets",
  other_wallet_balances: "shell-access-gate-other-wallets",
});

const SERVICE_NOTICE_DISMISSED_KEY = "screenerbot.service-notice-dismissed";

/** True while the app runs in Explore Mode, without a wallet or RPC. */
export function setupRequired() {
  return Boolean(getBootstrapState().status?.explore_mode);
}

/**
 * Opens setup from Explore Mode: the full-screen first-run wizard over the dashboard
 * (`core/setup.js`), whose Back leads to the introduction.
 */
export function openSetup() {
  window.SetupController?.openOverDashboard();
}

/** The reason a wallet-backed control is unavailable in Explore Mode. */
export function setupRequiredReason() {
  return I18n.t("shell-setup-gate-detail");
}

/** The chain connection status the bootstrap carries (`ChainAccessStatus`), or null. */
export function accessStatus() {
  return getBootstrapState().status?.chain_access ?? null;
}

/** True when the chain is reached through the ScreenerBot service. */
export function serviceMode() {
  return accessStatus()?.mode === "service";
}

/**
 * Whether the chain connection provides `capability`. A capability the status does
 * not list, or a status not loaded yet, counts as available: the backend refuses
 * what it cannot do, so the gate never hides a control on missing data.
 */
export function capabilityAvailable(capability) {
  const capabilities = accessStatus()?.capabilities;
  if (!capabilities || !Object.hasOwn(capabilities, capability)) return true;
  return capabilities[capability] !== false;
}

/**
 * The reason a control is unavailable: the setup reason in Explore Mode, the
 * capability's reason when the connection lacks it, otherwise an empty string.
 */
export function capabilityReason(capability) {
  if (setupRequired()) return setupRequiredReason();
  if (!capability || capabilityAvailable(capability)) return "";
  return Object.hasOwn(CAPABILITY_REASON_LABELS, capability)
    ? I18n.label(CAPABILITY_REASON_LABELS, capability)
    : I18n.t("shell-access-gate-own-rpc-title");
}

/** True when `capability` (or, without one, the wallet) is out of reach right now. */
export function accessBlocked(capability) {
  return setupRequired() || Boolean(capability && !capabilityAvailable(capability));
}

/**
 * Marks a control unavailable in Explore Mode, or when the connection lacks
 * `capability`, and names the reason in its tooltip; otherwise it leaves the
 * control as the page set it.
 */
export function gateControl(control, capability) {
  if (!control || !accessBlocked(capability)) return;
  control.disabled = true;
  control.title = capabilityReason(capability);
}

/** Opens Settings on the Connection tab. */
export function openConnectionSettings() {
  showSettingsDialog({ tab: "connection" });
}

/**
 * Paints the gate into `container`. In Explore Mode: `setupTitle`, the setup reason
 * and the setup action. When the connection lacks `capability`: the own-RPC
 * requirement, its reason and the Connection settings action. A gate that stands
 * in for the page's content is a centred state view; with `overContent` the content
 * stays below, so the requirement is a notice above it, never a card inside it.
 */
export function renderAccessGate(container, setupTitle, capability, { overContent = false } = {}) {
  if (!container) return;
  const [requirement, action, open] = setupRequired()
    ? [
        { icon: "icon-wallet", title: setupTitle, message: setupRequiredReason() },
        { id: "open-setup", label: I18n.t("shell-explore-action") },
        openSetup,
      ]
    : [
        {
          icon: "icon-server",
          title: I18n.t("shell-access-gate-own-rpc-title"),
          message: capabilityReason(capability),
        },
        { id: "open-connection", label: I18n.t("shell-access-gate-action") },
        openConnectionSettings,
      ];
  container.innerHTML = overContent
    ? renderNotice({ variant: "info", ...requirement, action })
    : `<div class="access-gate">${renderStateView({
        ...requirement,
        action: { ...action, primary: true },
        compact: true,
      })}</div>`;
  container.querySelector(`[data-action="${action.id}"]`).addEventListener("click", open);
}

function serviceNoticeDismissed() {
  try {
    return window.sessionStorage.getItem(SERVICE_NOTICE_DISMISSED_KEY) === "1";
  } catch {
    return false;
  }
}

function dismissServiceNotice() {
  try {
    window.sessionStorage.setItem(SERVICE_NOTICE_DISMISSED_KEY, "1");
  } catch {
    // Without session storage the notice only stays closed until the next render.
  }
}

/**
 * The ScreenerBot service notice as notice markup while the chain runs on the service and
 * the notice has not been closed in this session, otherwise "". The page supplies its own
 * title and message, since what the service changes differs per page, and places the
 * notice in its own notice area, which scrolls with the content; its actions are
 * `open-connection` and `dismiss`, run by `runServiceNoticeAction`.
 */
export function serviceNoticeHtml({ title, message }) {
  if (!serviceMode() || setupRequired() || serviceNoticeDismissed()) return "";
  return renderNotice({
    variant: "info",
    icon: "icon-cloud",
    title,
    message,
    action: { id: "open-connection", label: I18n.t("shell-access-gate-action") },
    dismiss: I18n.t("common-action-close"),
  });
}

/** Runs a service notice action; false when `action` is not one, so the page handles it. */
export function runServiceNoticeAction(action) {
  if (action === "open-connection") openConnectionSettings();
  else if (action === "dismiss") dismissServiceNotice();
  else return false;
  return true;
}
