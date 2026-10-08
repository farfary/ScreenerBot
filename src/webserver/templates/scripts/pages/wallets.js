// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Wallets Page Module
 * Modern wallet management interface with subtabs for Main, Secondaries, and Archive
 */

import { registerPage } from "../core/lifecycle.js";
import { $, on } from "../core/dom.js";
import { Poller } from "../core/poller.js";
import { requestManager, apiErrorMessage } from "../core/request_manager.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import * as Utils from "../core/utils.js";
import * as Hints from "../core/hints.js";
import { enhanceAllSelects } from "../ui/custom_select.js";
import { createBulkOperations } from "./wallets/bulk_operations.js";
import { createWalletRenderers } from "./wallets/renderers.js";
import { createWatchedWallets } from "./wallets/watched.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { renderSetupGate, setupRequired } from "../ui/setup_gate.js";

// =============================================================================
// Constants
// =============================================================================

const POLL_INTERVAL = 30000; // 30 seconds for balance updates

const walletTab = (id, icon, name) => ({
  id,
  label: `<i class="${icon}"></i> ${Utils.escapeHtml(name)}`,
});

const buildWalletTabs = () => [
  walletTab("main", "icon-star", I18n.t("wallets-tab-main")),
  walletTab("secondaries", "icon-wallet", I18n.t("wallets-tab-secondaries")),
  walletTab("archive", "icon-archive", I18n.t("wallets-tab-archive")),
  walletTab("watched", "icon-eye", I18n.t("wallets-tab-watched")),
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
let tokenHoldings = [];
let currentTab = "main";
let hasLoadedWalletData = false;
let currentExportWalletId = null;
let currentArchiveWalletId = null;
let currentDeleteWalletId = null;

// Module instances
let bulk = null;
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
        $(".wallets-tab-panels")?.classList.add("hidden");
        const gate = $("#wallets-setup-gate");
        gate?.classList.remove("hidden");
        renderSetupGate(gate, I18n.t("wallets-setup-gate-title"));
        return;
      }

      // Initialize sub-modules
      bulk = createBulkOperations({
        $,
        Utils,
        on,
        showModal,
        hideModal,
        confirm: (config) => ConfirmationDialog.show(config),
        setButtonContent,
        enhanceAllSelects,
        loadAllData,
        walletsData: () => walletsData,
      });

      renderers = createWalletRenderers({
        walletsData: () => walletsData,
        tokenHoldings: () => tokenHoldings,
        currentTab: () => currentTab,
        $,
        Utils,
        handleWalletAction,
        onRefresh: handleRefresh,
        onAddWallet: () => showModal("add-wallet-modal"),
        onImportWallets: () => bulk.showBulkImportModal(),
        onExportWallets: () => bulk.showBulkExportModal(),
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
    if (panelId === currentTab) {
      panel.classList.remove("hidden");
    } else {
      panel.classList.add("hidden");
    }
  });
}

// =============================================================================
// Event Handlers Setup
// =============================================================================

// Per-tab actions (refresh, Add Wallet, Import/Export, Watch Wallet) are declared
// in each tab table's own toolbar config — this page owns only modals and forms.
function setupEventHandlers() {
  // Keyboard support for closing modals
  on(document, "keydown", (e) => {
    if (e.key === "Escape") {
      hideAllModals();
    }
  });

  // Add wallet modal
  setupAddWalletModal();

  // Export modal
  setupExportModal();

  // Archive modal
  setupArchiveModal();

  // Delete modal
  setupDeleteModal();

  // Bulk import/export modals
  bulk.setupBulkImportModal();
  bulk.setupBulkExportModal();
}

function setupAddWalletModal() {
  const modal = $("#add-wallet-modal");
  if (!modal) return;

  // Close button
  const closeBtn = $("#modal-close-btn");
  if (closeBtn) {
    on(closeBtn, "click", () => hideModal("add-wallet-modal"));
  }

  // Tab switching
  const tabs = modal.querySelectorAll(".modal-tab");
  tabs.forEach((tab) => {
    on(tab, "click", () => {
      const tabId = tab.dataset.tab;
      tabs.forEach((t) => t.classList.remove("active"));
      tab.classList.add("active");

      const createContent = $("#create-tab-content");
      const importContent = $("#import-tab-content");

      if (tabId === "create") {
        createContent.classList.add("active");
        importContent.classList.remove("active");
      } else {
        createContent.classList.remove("active");
        importContent.classList.add("active");
      }
    });
  });

  // Create form
  const createForm = $("#create-wallet-form");
  if (createForm) {
    on(createForm, "submit", handleCreateWallet);
  }

  // Import form
  const importForm = $("#import-wallet-form");
  if (importForm) {
    on(importForm, "submit", handleImportWallet);
  }

  // Cancel buttons
  const createCancel = $("#create-cancel-btn");
  const importCancel = $("#import-cancel-btn");
  if (createCancel) on(createCancel, "click", () => hideModal("add-wallet-modal"));
  if (importCancel) on(importCancel, "click", () => hideModal("add-wallet-modal"));

  // Toggle visibility for private key
  const toggleBtn = modal.querySelector(".toggle-visibility");
  if (toggleBtn) {
    on(toggleBtn, "click", () => {
      const targetId = toggleBtn.dataset.target;
      const input = $(`#${targetId}`);
      if (input) {
        const isHidden = input.classList.contains("secure-hidden");
        if (isHidden) {
          input.classList.remove("secure-hidden");
        } else {
          input.classList.add("secure-hidden");
        }
        const icon = toggleBtn.querySelector("i");
        if (icon) {
          icon.className = isHidden ? "icon-eye-off" : "icon-eye";
        }
        toggleBtn.setAttribute("aria-pressed", isHidden ? "true" : "false");
      }
    });
  }

  // Close on backdrop click
  on(modal, "click", (e) => {
    if (e.target === modal) {
      hideModal("add-wallet-modal");
    }
  });
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

function setupArchiveModal() {
  const modal = $("#archive-modal");
  if (!modal) return;

  const closeBtn = $("#archive-modal-close");
  const cancelBtn = $("#archive-cancel-btn");
  const confirmBtn = $("#confirm-archive-btn");

  if (closeBtn) on(closeBtn, "click", () => closeArchiveModal());
  if (cancelBtn) on(cancelBtn, "click", () => closeArchiveModal());

  if (confirmBtn) {
    on(confirmBtn, "click", handleArchiveWallet);
  }

  on(modal, "click", (e) => {
    if (e.target === modal) {
      closeArchiveModal();
    }
  });
}

function setupDeleteModal() {
  const modal = $("#delete-modal");
  if (!modal) return;

  const closeBtn = $("#delete-modal-close");
  const cancelBtn = $("#delete-cancel-btn");
  const confirmBtn = $("#confirm-delete-btn");

  if (closeBtn) on(closeBtn, "click", () => closeDeleteModal());
  if (cancelBtn) on(cancelBtn, "click", () => closeDeleteModal());

  if (confirmBtn) {
    on(confirmBtn, "click", handleDeleteWallet);
  }

  on(modal, "click", (e) => {
    if (e.target === modal) {
      closeDeleteModal();
    }
  });
}

// =============================================================================
// API Functions
// =============================================================================

async function loadAllData({ force = false } = {}) {
  await Promise.all([loadWallets(), loadTokenHoldings({ force })]);
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
    walletsData = data.wallets || [];

    // Fetch balance for main wallet
    await fetchMainWalletBalance();
  } catch (error) {
    console.error("[Wallets] Failed to load wallets:", error);
    walletsData = [];
  }
}

async function loadTokenHoldings({ force = false } = {}) {
  try {
    // force=true (refresh button) hits the POST endpoint that re-captures a
    // fresh on-chain snapshot and bootstraps metadata for never-seen mints.
    // The normal poll uses the cheap GET that just reads the latest snapshot.
    const response = force
      ? await fetch("/api/wallet/tokens/refresh", { method: "POST" })
      : await fetch("/api/wallet/tokens");
    if (!response.ok) throw new Error(`HTTP ${response.status}`);

    const data = await response.json();
    tokenHoldings = data.tokens || [];
  } catch (error) {
    console.debug("[Wallets] Failed to load token holdings:", error);
    tokenHoldings = [];
  }
}

async function fetchMainWalletBalance() {
  try {
    const response = await fetch("/api/wallet/current");
    if (response.ok) {
      const data = await response.json();
      if (data) {
        const mainWallet = walletsData.find((w) => w.role === "main");
        if (mainWallet) {
          mainWallet.balance = data.sol_balance || 0;
        }
      }
    }
  } catch (error) {
    console.debug("[Wallets] Balance fetch failed:", error);
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

async function handleCreateWallet(e) {
  e.preventDefault();
  const form = e.target;
  const submitBtn = form.querySelector('button[type="submit"]');
  const originalHtml = submitBtn.innerHTML;

  submitBtn.disabled = true;
  setButtonContent(submitBtn, "icon-loader spin", I18n.t("wallets-create-busy"));

  try {
    const response = await fetch("/api/wallets", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: $("#create-name").value.trim(),
        notes: $("#create-notes").value.trim() || null,
      }),
    });

    const data = await response.json();
    if (!response.ok) {
      throw new Error(apiErrorMessage(data, I18n.t("wallets-create-fallback")));
    }

    Utils.showToast(I18n.t("wallets-create-done", { name: data.wallet.name }), "success");
    form.reset();
    hideModal("add-wallet-modal");
    await loadAllData();
  } catch (error) {
    console.error("[Wallets] Create failed:", error);
    Utils.showToast(I18n.t("wallets-toast-failed", { reason: error.message }), "error");
  } finally {
    submitBtn.disabled = false;
    submitBtn.innerHTML = originalHtml;
  }
}

async function handleImportWallet(e) {
  e.preventDefault();
  const form = e.target;
  const submitBtn = form.querySelector('button[type="submit"]');
  const originalHtml = submitBtn.innerHTML;

  submitBtn.disabled = true;
  setButtonContent(submitBtn, "icon-loader spin", I18n.t("wallets-import-busy"));

  try {
    const response = await fetch("/api/wallets/import", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: $("#import-name").value.trim(),
        private_key: $("#import-private-key").value.trim(),
        notes: $("#import-notes").value.trim() || null,
      }),
    });

    const data = await response.json();
    if (!response.ok) {
      throw new Error(apiErrorMessage(data, I18n.t("wallets-import-failed")));
    }

    Utils.showToast(I18n.t("wallets-import-done", { name: data.wallet.name }), "success");
    form.reset();
    hideModal("add-wallet-modal");
    await loadAllData();
  } catch (error) {
    console.error("[Wallets] Import failed:", error);
    Utils.showToast(I18n.t("wallets-toast-failed", { reason: error.message }), "error");
  } finally {
    submitBtn.disabled = false;
    submitBtn.innerHTML = originalHtml;
  }
}

function handleWalletAction(action, id) {
  switch (action) {
    case "export":
      showExportModal(id);
      break;
    case "archive":
      showArchiveModal(id);
      break;
    case "restore":
      restoreWallet(id);
      break;
    case "delete":
      showDeleteModal(id);
      break;
  }
}

function showArchiveModal(id) {
  currentArchiveWalletId = id;
  const wallet = walletsData.find((w) => w.id === parseInt(id, 10));

  const textEl = $("#archive-confirm-text");
  if (textEl) {
    textEl.innerHTML = I18n.markup("wallets-archive-confirm-text", {
      name: wallet ? wallet.name : I18n.t("wallets-this-wallet"),
    });
  }

  showModal("archive-modal");
}

async function handleArchiveWallet() {
  if (!currentArchiveWalletId) return;

  const confirmBtn = $("#confirm-archive-btn");
  if (confirmBtn) {
    confirmBtn.disabled = true;
    setButtonContent(confirmBtn, "icon-loader spin", I18n.t("wallets-archive-busy"));
  }

  try {
    const response = await fetch(`/api/wallets/${currentArchiveWalletId}/archive`, {
      method: "POST",
    });
    const data = await response.json();
    if (!response.ok) throw new Error(apiErrorMessage(data, I18n.t("wallets-action-failed")));

    Utils.showToast(I18n.t("wallets-archive-done"), "success");
    closeArchiveModal();
    await loadAllData();
  } catch (error) {
    Utils.showToast(I18n.t("wallets-toast-failed", { reason: error.message }), "error");
    if (confirmBtn) {
      confirmBtn.disabled = false;
      setButtonContent(confirmBtn, "icon-archive", I18n.t("wallets-archive-confirm"));
    }
  }
}

function closeArchiveModal() {
  hideModal("archive-modal");
  currentArchiveWalletId = null;

  const confirmBtn = $("#confirm-archive-btn");
  if (confirmBtn) {
    confirmBtn.disabled = false;
    setButtonContent(confirmBtn, "icon-archive", I18n.t("wallets-archive-confirm"));
  }
}

async function restoreWallet(id) {
  try {
    const response = await fetch(`/api/wallets/${id}/restore`, { method: "POST" });
    const data = await response.json();
    if (!response.ok) throw new Error(apiErrorMessage(data, I18n.t("wallets-action-failed")));

    Utils.showToast(I18n.t("wallets-restore-done"), "success");
    await loadAllData();
  } catch (error) {
    Utils.showToast(I18n.t("wallets-toast-failed", { reason: error.message }), "error");
  }
}

function showExportModal(id) {
  currentExportWalletId = id;
  const keyDisplay = $("#export-key-display");
  const confirmBtn = $("#confirm-export-btn");

  if (keyDisplay) keyDisplay.classList.add("hidden");
  if (confirmBtn) {
    confirmBtn.classList.remove("hidden");
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
    if (keyDisplay) keyDisplay.classList.remove("hidden");
    if (confirmBtn) confirmBtn.classList.add("hidden");

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

function showDeleteModal(id) {
  currentDeleteWalletId = id;
  const wallet = walletsData.find((w) => w.id === parseInt(id, 10));

  const textEl = $("#delete-confirm-text");
  if (textEl) {
    textEl.innerHTML = I18n.markup("wallets-delete-confirm-text", {
      name: wallet ? wallet.name : I18n.t("wallets-this-wallet"),
    });
  }

  showModal("delete-modal");
}

async function handleDeleteWallet() {
  if (!currentDeleteWalletId) return;

  const confirmBtn = $("#confirm-delete-btn");
  if (confirmBtn) {
    confirmBtn.disabled = true;
    setButtonContent(confirmBtn, "icon-loader spin", I18n.t("wallets-delete-busy"));
  }

  try {
    const response = await fetch(`/api/wallets/${currentDeleteWalletId}`, {
      method: "DELETE",
    });
    const data = await response.json();
    if (!response.ok) throw new Error(apiErrorMessage(data, I18n.t("wallets-action-failed")));

    Utils.showToast(I18n.t("wallets-delete-done"), "success");
    closeDeleteModal();
    await loadAllData();
  } catch (error) {
    Utils.showToast(I18n.t("wallets-toast-failed", { reason: error.message }), "error");
    if (confirmBtn) {
      confirmBtn.disabled = false;
      setButtonContent(confirmBtn, "icon-trash-2", I18n.t("wallets-delete-confirm"));
    }
  }
}

function closeDeleteModal() {
  hideModal("delete-modal");
  currentDeleteWalletId = null;

  const confirmBtn = $("#confirm-delete-btn");
  if (confirmBtn) {
    confirmBtn.disabled = false;
    setButtonContent(confirmBtn, "icon-trash-2", I18n.t("wallets-delete-confirm"));
  }
}

// =============================================================================
// Modal Helpers
// =============================================================================

function showModal(id) {
  const modal = $(`#${id}`);
  if (modal) {
    modal.classList.remove("hidden");
    document.body.style.overflow = "hidden";
  }
}

function hideModal(id) {
  const modal = $(`#${id}`);
  if (modal) {
    modal.classList.add("hidden");
    document.body.style.overflow = "";
  }
}

function hideAllModals() {
  closeExportModal();
  closeArchiveModal();
  closeDeleteModal();
  hideModal("add-wallet-modal");
  watched?.hideAddModal();
  bulk.hideBulkImportModal();
  bulk.hideBulkExportModal();
}

// =============================================================================
// Bulk Operations (delegated to bulk_operations module)
// =============================================================================

// =============================================================================
// Utility Functions
// =============================================================================

function cleanup() {
  if (renderers) {
    renderers.destroyTables();
  }
  watched?.reset();
  walletsData = [];
  tokenHoldings = [];
  hasLoadedWalletData = false;
  currentTab = "main";
  currentExportWalletId = null;
  currentArchiveWalletId = null;
  currentDeleteWalletId = null;
  poller = null;
  tabBar = null;
  bulk = null;
  renderers = null;
  watched = null;
}

// =============================================================================
// Register Page
// =============================================================================

registerPage("wallets", createLifecycle());
