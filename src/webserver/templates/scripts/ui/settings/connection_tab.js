// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Settings > Connection: how ScreenerBot reaches Solana (the ScreenerBot service or
// your own RPC), the switch between them with a restart, and how signed swaps are sent.

import * as Utils from "../../core/utils.js";
import { errorMessageOf, requestManager } from "../../core/request_manager.js";
import { ConfirmationDialog } from "../confirmation_dialog.js";

const MODES = Object.freeze(["service", "own_rpc"]);

let state = null;

/** The tab's markup; the rows fill in once the connection status arrives. */
export function buildConnectionTab() {
  return `
    <div class="settings-section">
      <h3 class="settings-section-title">
        <i class="icon-network"></i>
        <span data-l10n-id="settings-connection-title"></span>
      </h3>
      <p class="settings-section-description" data-l10n-id="settings-connection-description"></p>

      <div class="connection-modes" role="radiogroup" data-l10n-id="settings-connection-choice"
        aria-busy="true">
        <div class="connection-mode" data-mode="service">
          <label class="connection-mode-head" for="settings-connection-service">
            <input type="radio" class="connection-mode-input" id="settings-connection-service"
              name="settings-connection" value="service" />
            <span class="connection-mode-title">
              <i class="icon-cloud" aria-hidden="true"></i>
              <span data-l10n-id="settings-connection-service-title"></span>
            </span>
            <span class="connection-mode-active" data-l10n-id="settings-connection-active"
              hidden></span>
          </label>
          <p class="connection-mode-state" hidden></p>
          <p class="connection-mode-detail" data-l10n-id="settings-connection-service-detail"></p>
        </div>
        <div class="connection-mode" data-mode="own_rpc">
          <label class="connection-mode-head" for="settings-connection-own_rpc">
            <input type="radio" class="connection-mode-input" id="settings-connection-own_rpc"
              name="settings-connection" value="own_rpc" />
            <span class="connection-mode-title">
              <i class="icon-server" aria-hidden="true"></i>
              <span data-l10n-id="settings-connection-own-title"></span>
            </span>
            <span class="connection-mode-active" data-l10n-id="settings-connection-active"
              hidden></span>
          </label>
          <p class="connection-mode-state" hidden></p>
          <p class="connection-mode-detail" data-l10n-id="settings-connection-own-detail"></p>
        </div>
      </div>
      <div class="connection-rpc" id="settingsConnectionRpc" hidden></div>
      <div class="connection-signin" id="settingsConnectionSignIn" hidden>
        <p class="connection-signin-text" data-l10n-id="setup-verify-service-signin"></p>
        <button type="button" class="btn btn-secondary btn-sm" id="settingsConnectionSignInBtn"
          data-l10n-id="shell-service-action-signin"></button>
      </div>
      <p class="connection-error" id="settingsConnectionError" role="alert" hidden></p>
      <div class="connection-actions">
        <button type="button" class="btn btn-primary btn-sm" id="settingsConnectionApply"
          data-l10n-id="settings-connection-apply" disabled></button>
      </div>
    </div>

    <div class="settings-section">
      <h3 class="settings-section-title">
        <i class="icon-radio-tower"></i>
        <span data-l10n-id="settings-connection-sending-title"></span>
      </h3>
      <div class="settings-group">
        <div class="settings-field">
          <div class="settings-field-info">
            <label for="settingsUseGateway" data-l10n-id="settings-connection-sending-label"></label>
            <span class="settings-field-hint" id="settingsUseGatewayHint"></span>
          </div>
          <div class="settings-field-control">
            <label class="toggle">
              <input type="checkbox" id="settingsUseGateway" disabled />
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>
      </div>
    </div>`;
}

/** Binds the tab inside `content` and loads the connection status. */
export function attachConnectionHandlers(content) {
  teardownConnectionTab();
  state = {
    content,
    status: null,
    account: null,
    rpcField: null,
    busy: false,
    restartAbort: null,
  };
  const current = state;

  content
    .querySelectorAll(".connection-mode-input")
    .forEach((input) => input.addEventListener("change", () => renderSelection(current)));
  content
    .querySelector("#settingsConnectionApply")
    ?.addEventListener("click", () => void applySelection(current));
  content
    .querySelector("#settingsUseGateway")
    ?.addEventListener("change", (event) => void setGateway(current, event.target));
  // Sign-in lives on the Account tab of this same dialog.
  content
    .querySelector("#settingsConnectionSignInBtn")
    ?.addEventListener("click", () =>
      content
        .closest(".settings-dialog, [role='dialog']")
        ?.querySelector('.settings-nav-item[data-tab="account"]')
        ?.click()
    );

  void load(current);
}

async function load(current) {
  const [status, account, config] = await Promise.all([
    requestManager
      .fetch("/api/chain/access", { priority: "high", skipDedup: true })
      .catch(() => null),
    requestManager.fetch("/api/account/status", { priority: "normal" }).catch(() => null),
    requestManager.fetch("/api/config/account", { priority: "normal" }).catch(() => null),
  ]);
  if (state !== current) return;
  current.status = status;
  // The account routes answer in a `{ success, data }` envelope.
  current.account = account?.data ?? null;
  current.content.querySelector(".connection-modes")?.removeAttribute("aria-busy");
  renderModes(current);
  // Config GET routes wrap the section in { data, timestamp }.
  renderGateway(current, Boolean(config?.data?.use_gateway_rpc));
}

/** The chosen mode, or null while the status has not arrived. */
function selectedMode(current) {
  return current.content.querySelector(".connection-mode-input:checked")?.value ?? null;
}

function renderModes(current) {
  const active = current.status?.mode;
  for (const value of MODES) {
    const row = current.content.querySelector(`.connection-mode[data-mode="${value}"]`);
    if (!row) continue;
    const input = row.querySelector(".connection-mode-input");
    input.checked = value === active;
    row.querySelector(".connection-mode-active").hidden = value !== active;
    const line = row.querySelector(".connection-mode-state");
    const { title, detail } =
      value === "service" && active === "service"
        ? serviceState(current)
        : { title: "", detail: "" };
    line.textContent = title;
    if (detail) {
      const note = document.createElement("span");
      note.className = "connection-mode-state-detail";
      note.textContent = detail;
      line.append(note);
    }
    line.hidden = !title;
    // A paused service reads as a warning; connected, it is a plain statement.
    if (detail) line.dataset.tone = "warning";
    else delete line.dataset.tone;
  }
  renderSelection(current);
}

/**
 * The service row's state: signed in and connected, or the headline of what stops it
 * with its detail (when trading resumes, what resumes it).
 */
function serviceState(current) {
  const status = current.status;
  if (status?.ready) {
    const name = current.account?.name || current.account?.email;
    if (name) return { title: I18n.t("settings-connection-signed-in-as", { name }), detail: "" };
  }
  if (!status?.text) return { title: "", detail: "" };
  return {
    title: I18n.text(status.text),
    detail: status.ready ? "" : I18n.textAttr(status.text, "detail"),
  };
}

function renderSelection(current) {
  const selected = selectedMode(current);
  const active = current.status?.mode ?? null;
  const switchingToOwn = selected === "own_rpc" && active === "service";
  const rpcHost = current.content.querySelector("#settingsConnectionRpc");
  if (rpcHost) {
    rpcHost.hidden = !switchingToOwn;
    if (switchingToOwn && !current.rpcField && window.SetupRuntime) {
      current.rpcField = window.SetupRuntime.mountRpcField(rpcHost, "settings-connection-");
    }
  }
  // The service runs under the account, so a signed-out switch would only fail after the
  // restart; ask for the sign-in first instead.
  const needsSignIn =
    selected === "service" && active !== "service" && current.account?.signed_in === false;
  const signIn = current.content.querySelector("#settingsConnectionSignIn");
  if (signIn) signIn.hidden = !needsSignIn;
  const apply = current.content.querySelector("#settingsConnectionApply");
  if (apply) {
    apply.disabled = current.busy || !selected || !active || selected === active || needsSignIn;
  }
  showError(current, "");
}

function showError(current, message) {
  const error = current.content.querySelector("#settingsConnectionError");
  if (!error) return;
  error.textContent = message;
  error.hidden = !message;
}

async function applySelection(current) {
  const mode = selectedMode(current);
  if (current.busy || !mode || mode === current.status?.mode) return;
  const toService = mode === "service";
  if (!toService && !current.rpcField?.validate()) return;
  const rpcUrls = toService ? [] : current.rpcField.rpcUrls();

  const { confirmed } = await ConfirmationDialog.show({
    title: toService
      ? I18n.t("settings-connection-to-service-title")
      : I18n.t("settings-connection-to-own-title"),
    message: toService
      ? I18n.t("settings-connection-to-service-message")
      : I18n.t("settings-connection-to-own-message"),
    confirmLabel: I18n.t("settings-connection-apply"),
    cancelLabel: I18n.t("common-action-cancel"),
    variant: "warning",
  });
  if (!confirmed || state !== current) return;

  current.busy = true;
  renderSelection(current);
  try {
    const result = await requestManager.fetch("/api/chain/access", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ mode, rpc_urls: rpcUrls }),
      priority: "high",
      skipDedup: true,
    });
    // The server has scheduled the restart; one notice covers it until the page reloads.
    Utils.showToast({
      key: "system-restart",
      type: "progress",
      title: I18n.t("shell-restart-progress"),
    });
    current.restartAbort = new AbortController();
    await window.waitForScreenerBotRestart(result.instance_id, {
      target: window.location.pathname || "/home",
      signal: current.restartAbort.signal,
    });
  } catch (error) {
    if (error?.name === "AbortError" || state !== current) return;
    current.busy = false;
    renderSelection(current);
    showError(current, await errorMessageOf(error, I18n.t("settings-connection-switch-failed")));
  }
}

function renderGateway(current, enabled) {
  const toggle = current.content.querySelector("#settingsUseGateway");
  const hint = current.content.querySelector("#settingsUseGatewayHint");
  if (!toggle || !hint) return;
  // On the service, the service is how signed swaps reach the chain: always on.
  const service = current.status?.mode === "service";
  toggle.checked = service || enabled;
  toggle.disabled = service || !current.status;
  hint.textContent = service
    ? I18n.t("settings-connection-sending-service-hint")
    : I18n.t("settings-connection-sending-own-hint");
}

async function setGateway(current, toggle) {
  const requested = toggle.checked;
  toggle.disabled = true;
  try {
    const status = await requestManager.fetch("/api/account/gateway", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ enabled: requested }),
      priority: "high",
      skipDedup: true,
    });
    // Apply the value the server stored; account routes answer in a `{ success, data }` envelope.
    if (state === current) renderGateway(current, Boolean(status?.data?.use_gateway_rpc));
  } catch {
    // A refused write rolls the control back to the stored value.
    if (state === current) renderGateway(current, !requested);
  }
}

/** The dialog is closing or another tab opens: stop waiting and drop the field. */
export function teardownConnectionTab() {
  if (!state) return;
  state.restartAbort?.abort();
  state.rpcField?.dispose();
  state = null;
}
