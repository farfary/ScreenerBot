// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Wallets Page Module
 * The trading wallet with its holdings and its replacement, the other wallets
 * (each with its private-key export and, for a replaced wallet, its backup), and
 * watched wallets
 */

import { registerPage } from "../core/lifecycle.js";
import { $, on } from "../core/dom.js";
import { Poller } from "../core/poller.js";
import { requestManager, apiErrorMessage } from "../core/request_manager.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import * as Utils from "../core/utils.js";
import * as Hints from "../core/hints.js";
import { createWalletRenderers } from "./wallets/renderers.js";
import { createWatchedWallets } from "./wallets/watched.js";
import { renderSetupGate, setupRequired } from "../ui/setup_gate.js";
import { SetupDialog } from "../ui/setup_dialog.js";

// =============================================================================
// Constants
// =============================================================================

const POLL_INTERVAL = 30000; // 30 seconds for balance updates

const buildWalletTabs = () => [
  { id: "main", label: I18n.t("wallets-tab-main") },
  { id: "other", label: I18n.t("wallets-tab-other") },
  { id: "watched", label: I18n.t("wallets-tab-watched") },
];

// Replaces a button's content with an icon and a text label built through the DOM.
function setButtonContent(button, iconClass, label) {
  const icon = document.createElement("i");
  icon.className = iconClass;
  const text = document.createElement("span");
  text.textContent = label;
  button.replaceChildren(icon, " ", text);
}

// =============================================================================
// State
// =============================================================================

let poller = null;
let tabBar = null;
let walletsData = [];
// The wallet-reset backups, newest first: a replaced wallet's row names its backup.
let walletBackups = [];
let tokenHoldings = [];
// Why the holdings are unknown: the cause of a failed wallet read, else null.
let holdingsReadError = null;
let currentTab = "main";
let hasLoadedWalletData = false;
let currentExportWalletId = null;

// Module instances
let renderers = null;
let watched = null;

// =============================================================================
// Lifecycle
// =============================================================================

function createLifecycle() {
  return {
    init(ctx) {
      console.log("[Wallets] Initializing...");

      // Hints are optional enhancement data; they must never delay the page
      // shell, restored subtab, or the subtab bar.
      void Hints.init();

      // Explore Mode has no wallet store: the page is the setup notice alone, with
      // no sub-tabs, no wallet reads and no poller.
      if (setupRequired()) {
        const panels = $(".wallets-tab-panels");
        if (panels) panels.hidden = true;
        const gate = $("#wallets-setup-gate");
        if (gate) gate.hidden = false;
        renderSetupGate(gate, I18n.t("wallets-setup-gate-title"));
        return;
      }

      // Initialize sub-modules
      renderers = createWalletRenderers({
        walletsData: () => walletsData,
        walletBackups: () => walletBackups,
        tokenHoldings: () => tokenHoldings,
        holdingsReadError: () => holdingsReadError,
        currentTab: () => currentTab,
        $,
        Utils,
        handleWalletAction,
        onRefresh: handleRefresh,
      });

      watched = createWatchedWallets({
        $,
        on,
        Utils,
        requestManager,
        showModal,
        hideModal,
        onRefresh: handleRefresh,
      });

      // Initialize tab bar
      tabBar = new TabBar({
        container: "#subTabsContainer",
        tabs: buildWalletTabs(),
        defaultTab: "main",
        stateKey: "wallets.activeTab",
        pageName: "wallets",
        onChange: (tabId) => switchTab(tabId),
      });

      TabBarManager.register("wallets", tabBar);
      ctx.manageTabBar(tabBar);
      tabBar.show();

      // Sync state with TabBar's restored state
      currentTab = tabBar.getActiveTab() || "main";

      // Setup event handlers
      setupEventHandlers();
      watched.setup();

      // Show the restored tab's panel BEFORE loading it. Every panel but Main
      // ships hidden in the markup, and a DataTable built inside a hidden panel
      // measures a zero-width container - it would render at minimum column
      // widths. switchTab() already orders it this way.
      updatePanelVisibility();

      // Paint the selected panel's canonical loading state before starting I/O.
      renderActiveTabShell();
      void loadActiveTab();
    },

    activate(ctx) {
      console.log("[Wallets] Activating...");
      if (setupRequired()) return;

      // Re-register deactivate cleanup (cleared after each deactivate) and
      // force-show tab bar to handle race conditions with TabBarManager.
      if (tabBar) {
        ctx.manageTabBar(tabBar);
        tabBar.show({ force: true });
      }

      // Start polling for balance updates. Reuse the same poller across page
      // visits; lifecycle deactivation stops it and activation re-registers it.
      if (!poller) {
        poller = new Poller(
          async () => {
            await loadActiveTab();
          },
          // l10n-ignore: poller diagnostic label, never displayed
          { label: "Wallets", intervalMs: POLL_INTERVAL }
        );
      }

      ctx.managePoller(poller);
      poller.start();
    },

    deactivate() {
      console.log("[Wallets] Deactivating...");
      // Poller is managed by lifecycle context
    },

    dispose() {
      console.log("[Wallets] Disposing...");
      cleanup();
    },
  };
}

// =============================================================================
// Tab Switching
// =============================================================================

function switchTab(tabId) {
  if (currentTab === tabId) return;

  currentTab = tabId;
  updatePanelVisibility();
  renderActiveTabShell();

  void loadActiveTab();
}

function renderActiveTabShell() {
  if (currentTab === "watched") return;
  renderers?.renderCurrentPanel({ loading: !hasLoadedWalletData });
}

function updatePanelVisibility() {
  const panels = document.querySelectorAll(".wallet-tab-panel");
  panels.forEach((panel) => {
    const panelId = panel.dataset.panel;
    panel.hidden = panelId !== currentTab;
  });
}

// =============================================================================
// Event Handlers Setup
// =============================================================================

// Per-tab actions (refresh, key export, Watch Wallet) are declared in each tab
// table's own toolbar config — this page owns only modals and forms.
function setupEventHandlers() {
  // Keyboard support for closing modals
  on(document, "keydown", (e) => {
    if (e.key === "Escape") {
      hideAllModals();
    }
  });

  // Export modal
  setupExportModal();
}

function setupExportModal() {
  const modal = $("#export-modal");
  if (!modal) return;

  const closeBtn = $("#export-modal-close");
  const cancelBtn = $("#export-cancel-btn");
  const confirmBtn = $("#confirm-export-btn");
  const copyBtn = $("#copy-key-btn");

  if (closeBtn) on(closeBtn, "click", () => closeExportModal());
  if (cancelBtn) on(cancelBtn, "click", () => closeExportModal());

  if (confirmBtn) {
    on(confirmBtn, "click", handleExportKey);
  }

  if (copyBtn) {
    on(copyBtn, "click", () => {
      const keyEl = $("#exported-key");
      if (keyEl && keyEl.textContent !== "...") {
        Utils.copyToClipboard(keyEl.textContent);
        Utils.notifyCopied(I18n.t("wallets-copied-private-key"));
      }
    });
  }

  on(modal, "click", (e) => {
    if (e.target === modal) {
      closeExportModal();
    }
  });
}

// =============================================================================
// API Functions
// =============================================================================

async function loadAllData({ force = false } = {}) {
  await Promise.all([loadWallets(), loadWalletBackups(), loadTokenHoldings({ force })]);
  hasLoadedWalletData = true;
  renderers.renderCurrentPanel();
}

async function loadActiveTab({ force = false } = {}) {
  if (currentTab === "watched") {
    await watched?.load({ force });
    return;
  }
  await loadAllData({ force });
}

async function loadWallets() {
  try {
    const response = await fetch("/api/wallets?include_inactive=true");
    if (!response.ok) throw new Error(`HTTP ${response.status}`);

    const data = await response.json();
    // Each row carries its SOL balance, read by the server for every listed wallet.
    walletsData = data.wallets || [];
  } catch (error) {
    console.error("[Wallets] Failed to load wallets:", error);
    walletsData = [];
  }
}

async function loadWalletBackups() {
  try {
    const response = await fetch("/api/wallets/backups");
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const data = await response.json();
    walletBackups = data.backups || [];
    showAbandonedReplacement(data.abandoned);
  } catch (error) {
    console.debug("[Wallets] Failed to load wallet backups:", error);
    walletBackups = [];
  }
}

// The message of each reason a restart abandons a staged replacement for.
const ABANDONED_REPLACEMENT_MESSAGES = Object.freeze({
  open_positions: (args) => I18n.t("wallets-replace-abandoned-open-positions", args),
  positions_unreadable: (args) => I18n.t("wallets-replace-abandoned-positions-unreadable", args),
  vault_changed: (args) => I18n.t("wallets-replace-abandoned-vault-changed", args),
});

// A replacement a restart abandoned is shown once; the server then forgets it.
function showAbandonedReplacement(abandoned) {
  if (!abandoned) return;
  const message =
    ABANDONED_REPLACEMENT_MESSAGES[abandoned.reason] || ABANDONED_REPLACEMENT_MESSAGES.vault_changed;
  Utils.showToast({
    key: "wallets-replace-abandoned",
    type: "warning",
    title: I18n.t("wallets-replace-abandoned-title"),
    message: message({ count: abandoned.count ?? 0 }),
  });
  fetch("/api/wallets/replacement/dismiss", { method: "POST" }).catch((error) => {
    console.debug("[Wallets] Failed to dismiss the abandoned replacement:", error);
  });
}

async function loadTokenHoldings({ force = false } = {}) {
  try {
    // force=true (refresh button) hits the POST endpoint that re-captures a
    // fresh on-chain snapshot and bootstraps metadata for never-seen mints.
    // The normal poll uses the cheap GET that just reads the latest snapshot.
    const response = force
      ? await fetch("/api/wallet/tokens/refresh", { method: "POST" })
      : await fetch("/api/wallet/tokens");
    const data = await response.json().catch(() => null);
    if (!response.ok) {
      // A failed wallet read leaves the holdings unknown, not empty.
      holdingsReadError = apiErrorMessage(data, null);
      tokenHoldings = [];
      return;
    }
    holdingsReadError = null;
    tokenHoldings = data?.tokens || [];
  } catch (error) {
    console.debug("[Wallets] Failed to load token holdings:", error);
    tokenHoldings = [];
  }
}

// =============================================================================
// Action Handlers
// =============================================================================

// Called from each tab table's toolbar refresh button, which passes its own
// element so the spinner lands on the button the user actually pressed.
async function handleRefresh(btn) {
  if (!btn) return;

  const icon = btn.querySelector("i");
  if (icon) icon.classList.add("spin");
  btn.disabled = true;

  try {
    // No success toast: the table repaints and the button's spinner stops.
    await loadActiveTab({ force: true });
  } catch {
    Utils.showToast({
      key: "wallets-load",
      type: "error",
      title: I18n.t("wallets-refresh-failed"),
    });
  } finally {
    if (icon) icon.classList.remove("spin");
    btn.disabled = false;
  }
}

function handleWalletAction(action, id) {
  if (action === "export") showExportModal(id);
  // Replacing the trading wallet restarts the app once it is staged.
  else if (action === "replace") void SetupDialog.show({ mode: "replace" });
}

function showExportModal(id) {
  currentExportWalletId = id;
  const keyDisplay = $("#export-key-display");
  const confirmBtn = $("#confirm-export-btn");

  if (keyDisplay) keyDisplay.hidden = true;
  if (confirmBtn) {
    confirmBtn.hidden = false;
    confirmBtn.disabled = false;
  }

  const keyEl = $("#exported-key");
  if (keyEl) keyEl.textContent = "...";

  showModal("export-modal");
}

async function handleExportKey() {
  if (!currentExportWalletId) return;

  const confirmBtn = $("#confirm-export-btn");
  if (confirmBtn) {
    confirmBtn.disabled = true;
    setButtonContent(confirmBtn, "icon-loader spin", I18n.t("wallets-export-busy"));
  }

  try {
    const response = await fetch(`/api/wallets/${currentExportWalletId}/export`, {
      method: "POST",
    });
    const data = await response.json();
    if (!response.ok) throw new Error(apiErrorMessage(data, I18n.t("wallets-action-failed")));

    const keyDisplay = $("#export-key-display");
    const keyEl = $("#exported-key");

    if (keyEl) keyEl.textContent = data.private_key;
    if (keyDisplay) keyDisplay.hidden = false;
    if (confirmBtn) confirmBtn.hidden = true;

    Utils.showToast(I18n.t("wallets-export-revealed"), "warning");
  } catch (error) {
    Utils.showToast(I18n.t("wallets-toast-failed", { reason: error.message }), "error");
    closeExportModal();
  }
}

function closeExportModal() {
  hideModal("export-modal");
  currentExportWalletId = null;

  const keyEl = $("#exported-key");
  if (keyEl) keyEl.textContent = "...";
}

// =============================================================================
// Modal Helpers
// =============================================================================

function showModal(id) {
  const modal = $(`#${id}`);
  if (modal) {
    modal.hidden = false;
    document.body.style.overflow = "hidden";
  }
}

function hideModal(id) {
  const modal = $(`#${id}`);
  if (modal) {
    modal.hidden = true;
    document.body.style.overflow = "";
  }
}

function hideAllModals() {
  closeExportModal();
  watched?.hideAddModal();
}

// =============================================================================
// Utility Functions
// =============================================================================

function cleanup() {
  if (renderers) {
    renderers.destroyTables();
  }
  watched?.reset();
  walletsData = [];
  walletBackups = [];
  tokenHoldings = [];
  holdingsReadError = null;
  hasLoadedWalletData = false;
  currentTab = "main";
  currentExportWalletId = null;
  poller = null;
  tabBar = null;
  renderers = null;
  watched = null;
}

// =============================================================================
// Register Page
// =============================================================================

registerPage("wallets", createLifecycle());
