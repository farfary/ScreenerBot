// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Settings > Account.
//
// The same panel the setup screen mounts, plus the things that only make sense
// once you have an account: the ScreenerBot data state, and where to manage the
// account itself. On the ScreenerBot service, signing out pauses trading, so it
// asks first.
//
// Deliberately thin. Anything that can be changed from the website is LINKED to
// rather than reimplemented here — password, email, connected devices and
// referral payouts all live at screenerbot.io/dashboard, and a second
// implementation in the app would be a second thing to keep correct.

import { getBootstrapState } from "../../core/bootstrap.js";
import { ConfirmationDialog } from "../confirmation_dialog.js";

const DASHBOARD_URL = "https://screenerbot.io/dashboard";

let instance = null;

export function buildAccountTab() {
  // The access mode is fixed for the process, so the mode-specific copy is chosen once.
  const service = getBootstrapState().status?.chain_access?.mode === "service";
  const description = service
    ? `<p class="settings-section-description" data-l10n-id="settings-account-description-service"></p>`
    : `<p class="settings-section-description" data-l10n-id="settings-account-description"></p>`;
  const fallback = service
    ? `<p class="settings-section-description" data-l10n-id="settings-account-data-fallback-service"></p>`
    : `<p class="settings-section-description" data-l10n-id="settings-account-data-fallback"></p>`;
  return `
    <div class="settings-section">
      <h3 class="settings-section-title">
        <i class="icon-circle-user"></i>
        <span data-l10n-id="settings-account-title"></span>
      </h3>
      ${description}

      <div class="account-panel" id="settingsAccountPanel"></div>
    </div>

    <div class="settings-section">
      <h3 class="settings-section-title">
        <i class="icon-database"></i>
        <span data-l10n-id="settings-account-data-title"></span>
      </h3>
      <p class="settings-section-description" data-l10n-id="settings-account-data-description"></p>
      <div class="settings-data-access" id="settingsDataAccess" aria-live="polite"></div>
      ${fallback}
    </div>

    <div class="settings-section">
      <h3 class="settings-section-title">
        <i class="icon-external-link"></i>
        <span data-l10n-id="settings-account-manage-title"></span>
      </h3>
      <p class="settings-section-description" data-l10n-id="settings-account-manage-description"></p>
      <button
        type="button"
        class="btn btn-secondary btn-sm"
        id="settingsOpenDashboard"
        data-l10n-id="settings-account-open-dashboard"
      ></button>
    </div>`;
}

export function attachAccountHandlers() {
  const container = document.getElementById("settingsAccountPanel");
  if (container && window.AccountPanel) {
    instance = window.AccountPanel.mount(container, {
      // Settings has a "ScreenerBot data" section of its own below, so the panel
      // must not paint the same availability block a second time here.
      showDataAccess: false,
      // The panel already fetched the status this section renders, so it hands
      // it over rather than making Settings fetch the same thing again.
      onChange: (status) => renderDataAccess(status?.data_access),
      beforeSignOut: confirmServiceSignOut,
    });
  }

  const dashboard = document.getElementById("settingsOpenDashboard");
  if (dashboard) {
    dashboard.addEventListener("click", () => {
      fetch("/api/system/open-url", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url: DASHBOARD_URL }),
      });
    });
  }
}

/**
 * Paint the data-service state.
 *
 * Every sentence comes from the backend (`data_server::access`), so this section,
 * the account panel and the first-run introduction cannot describe the same
 * state three different ways.
 */
function renderDataAccess(access) {
  const host = document.getElementById("settingsDataAccess");
  if (!host) return;

  if (!access) {
    host.innerHTML = "";
    return;
  }

  const escape = (value) =>
    String(value ?? "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");

  host.innerHTML = `
    <div class="account-data" data-state="${escape(access.state)}">
      <p class="account-data-headline">
        <i class="${access.available ? "icon-circle-check" : "icon-info"}" aria-hidden="true"></i>
        <span>${escape(I18n.text(access.text))}</span>
      </p>
      <p class="account-data-detail">${escape(I18n.textAttr(access.text, "detail"))}</p>
    </div>`;
}

/**
 * On the ScreenerBot service the account is how the app reads the wallet and sends
 * trades, so signing out pauses trading: confirm it. On your own RPC nothing pauses.
 */
async function confirmServiceSignOut() {
  if (getBootstrapState().status?.chain_access?.mode !== "service") return true;
  const { confirmed } = await ConfirmationDialog.show({
    title: I18n.t("shell-signout-service-title"),
    message: I18n.t("shell-signout-service-message"),
    confirmLabel: I18n.t("shell-signout-service-confirm"),
    cancelLabel: I18n.t("common-action-cancel"),
    variant: "warning",
  });
  return confirmed;
}

/** The dialog is closing. Stop the panel's sign-in watcher. */
export function teardownAccountTab() {
  instance?.destroy();
  instance = null;
}
