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
import { createPageDialogs } from "../ui/page_dialogs.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { accessBlocked, renderAccessGate, setupRequired } from "../ui/access_gate.js";
import { SetupDialog } from "../ui/setup_dialog.js";
import { showWalletKeyDialog } from "../ui/wallet_key_dialog.js";

// =============================================================================
// Constants
// =============================================================================

const POLL_INTERVAL = 30000; // 30 seconds for balance updates
/** The chain capability Wallets > Watched needs: following other wallets live. */
const WATCH_CAPABILITY = "watch_other_wallets";

const buildWalletTabs = () => [
  { id: "main", label: I18n.t("wallets-tab-main") },
  { id: "other", label: I18n.t("wallets-tab-other") },
  { id: "watched", label: I18n.t("wallets-tab-watched") },
];

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

// Module instances
let renderers = null;
let watched = null;
let dialogs = null;

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
        const gate = $("#wallets-access-gate");
        if (gate) gate.hidden = false;
        renderAccessGate(gate, I18n.t("wallets-access-gate-title"));
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

      dialogs = createPageDialogs($, on);
      watched = createWatchedWallets({
        $,
        on,
        Utils,
        requestManager,
        showModal: dialogs.show,
        hideModal: dialogs.hide,
        confirm: (config) => ConfirmationDialog.show(config),
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
  // Backdrop and close buttons of every modal; Escape is owned by the dialogs.
  dialogs.setup($(".wallets-page"));
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
    // Without the capability the tab is the access gate alone: no watch reads.
    const blocked = accessBlocked(WATCH_CAPABILITY);
    const gate = $("#watched-access-gate");
    const root = $("#watched-wallets-root");
    if (gate) gate.hidden = !blocked;
    if (root) root.hidden = blocked;
    if (blocked) {
      if (gate && !gate.childElementCount) renderAccessGate(gate, "", WATCH_CAPABILITY);
      return;
    }
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
    ABANDONED_REPLACEMENT_MESSAGES[abandoned.reason] ||
    ABANDONED_REPLACEMENT_MESSAGES.vault_changed;
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
  if (action === "export") showWalletKeyDialog(id);
  // Replacing the trading wallet restarts the app once it is staged.
  else if (action === "replace") void SetupDialog.show();
}

// =============================================================================
// Utility Functions
// =============================================================================

function cleanup() {
  dialogs?.closeAll();
  dialogs = null;
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
  poller = null;
  tabBar = null;
  renderers = null;
  watched = null;
}

// =============================================================================
// Register Page
// =============================================================================

registerPage("wallets", createLifecycle());
