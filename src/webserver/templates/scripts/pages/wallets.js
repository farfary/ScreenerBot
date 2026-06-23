/**
 * Wallets Page Module
 * Modern wallet management interface with subtabs for Main, Secondaries, and Archive
 */

import { registerPage } from "../core/lifecycle.js";
import { $, on } from "../core/dom.js";
import { Poller } from "../core/poller.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import * as Utils from "../core/utils.js";
import * as Hints from "../core/hints.js";
import { HintPopover } from "../ui/hint_popover.js";
import { enhanceAllSelects } from "../ui/custom_select.js";
import { createBulkOperations } from "./wallets/bulk_operations.js";
import { createWalletRenderers } from "./wallets/renderers.js";

// =============================================================================
// Constants
// =============================================================================

const POLL_INTERVAL = 30000; // 30 seconds for balance updates

const WALLET_TABS = [
  { id: "main", label: '<i class="icon-star"></i> Main Wallet' },
  { id: "secondaries", label: '<i class="icon-wallet"></i> Secondaries' },
  { id: "archive", label: '<i class="icon-archive"></i> Archive' },
];

// =============================================================================
// State
// =============================================================================

let poller = null;
let tabBar = null;
let walletsData = [];
let tokenHoldings = [];
let currentTab = "main";
let currentExportWalletId = null;
let currentArchiveWalletId = null;
let currentDeleteWalletId = null;

// Module instances
let bulk = null;
let renderers = null;

// =============================================================================
// Lifecycle
// =============================================================================

function createLifecycle() {
  return {
    async init(ctx) {
      console.log("[Wallets] Initializing...");

      // Initialize hints system
      await Hints.init();

      // Initialize sub-modules
      bulk = createBulkOperations({
        $,
        Utils,
        on,
        showModal,
        hideModal,
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
        on,
        capitalizeFirst,
        handleWalletAction,
      });

      // Initialize tab bar
      tabBar = new TabBar({
        container: "#wallets-tabs-container",
        tabs: WALLET_TABS,
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

      // Setup security hint button
      setupSecurityHint();

      // Load initial data
      await loadAllData();

      // Update panel visibility and tab-specific toolbar buttons
      updatePanelVisibility();
      updateToolbarActions();
    },

    activate(ctx) {
      console.log("[Wallets] Activating...");

      // Start polling for balance updates
      poller = new Poller(async () => {
        await loadAllData();
      }, POLL_INTERVAL);

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
  updateToolbarActions();

  // Re-render content for the active tab (delegated to renderers)
  renderers.renderCurrentPanel();
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

function updateToolbarActions() {
  document.querySelectorAll(".wallets-tab-actions").forEach((group) => {
    const tab = group.dataset.tabActions;
    if (tab === currentTab) {
      group.classList.remove("hidden");
    } else {
      group.classList.add("hidden");
    }
  });
}

// =============================================================================
// Event Handlers Setup
// =============================================================================

function setupEventHandlers() {
  // Refresh buttons (one per tab, all call the same handler)
  ["#refresh-wallets-btn", "#refresh-wallets-btn-sec", "#refresh-wallets-btn-arc"].forEach((sel) => {
    const btn = $(sel);
    if (btn) on(btn, "click", handleRefresh);
  });

  // Secondaries-tab buttons
  const addBtn = $("#add-wallet-btn");
  const importBtn = $("#import-wallets-btn");
  const exportBtn = $("#export-wallets-btn");

  if (addBtn) {
    on(addBtn, "click", () => showModal("add-wallet-modal"));
  }
  if (importBtn) {
    on(importBtn, "click", () => bulk.showBulkImportModal());
  }
  if (exportBtn) {
    on(exportBtn, "click", () => bulk.showBulkExportModal());
  }

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

/**
 * Setup security hint button to show encryption details popover
 */
function setupSecurityHint() {
  const hintBtn = $("#security-hint-btn");
  if (!hintBtn) return;

  on(hintBtn, "click", () => {
    const hint = Hints.getHint("wallets.security");
    if (hint) {
      const popover = new HintPopover(hint, hintBtn);
      popover.show();
    }
  });
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
        createContent.classList.remove("hidden");
        importContent.classList.add("hidden");
      } else {
        createContent.classList.add("hidden");
        importContent.classList.remove("hidden");
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
        Utils.showToast("Private key copied to clipboard!", "success");
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

async function loadAllData() {
  await Promise.all([loadWallets(), loadTokenHoldings()]);
  renderers.renderCurrentPanel();
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

async function loadTokenHoldings() {
  try {
    const response = await fetch("/api/wallet/tokens");
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

async function handleRefresh(e) {
  const btn = e?.currentTarget ?? $("#refresh-wallets-btn");
  if (!btn) return;

  const icon = btn.querySelector("i");
  if (icon) icon.classList.add("spin");
  btn.disabled = true;

  try {
    await loadAllData();
    Utils.showToast("Wallets refreshed", "success");
  } catch {
    Utils.showToast("Failed to refresh", "error");
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
  submitBtn.innerHTML = '<i class="icon-loader spin"></i> Creating...';

  try {
    const response = await fetch("/api/wallets", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: $("#create-name").value.trim(),
        notes: $("#create-notes").value.trim() || null,
        set_as_main: $("#create-set-main").checked,
      }),
    });

    const data = await response.json();
    if (!response.ok) {
      throw new Error(data.error || data.message || "Creation failed");
    }

    Utils.showToast(`Wallet "${data.wallet.name}" created!`, "success");
    form.reset();
    hideModal("add-wallet-modal");
    await loadAllData();
  } catch (error) {
    console.error("[Wallets] Create failed:", error);
    Utils.showToast(`Failed: ${error.message}`, "error");
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
  submitBtn.innerHTML = '<i class="icon-loader spin"></i> Importing...';

  try {
    const response = await fetch("/api/wallets/import", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: $("#import-name").value.trim(),
        private_key: $("#import-private-key").value.trim(),
        notes: $("#import-notes").value.trim() || null,
        set_as_main: $("#import-set-main").checked,
      }),
    });

    const data = await response.json();
    if (!response.ok) {
      throw new Error(data.error || data.message || "Import failed");
    }

    Utils.showToast(`Wallet "${data.wallet.name}" imported!`, "success");
    form.reset();
    hideModal("add-wallet-modal");
    await loadAllData();
  } catch (error) {
    console.error("[Wallets] Import failed:", error);
    Utils.showToast(`Failed: ${error.message}`, "error");
  } finally {
    submitBtn.disabled = false;
    submitBtn.innerHTML = originalHtml;
  }
}

function handleWalletAction(action, id) {
  switch (action) {
    case "set-main":
      setMainWallet(id);
      break;
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

async function setMainWallet(id) {
  try {
    const response = await fetch(`/api/wallets/${id}/set-main`, { method: "POST" });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || "Failed");

    Utils.showToast(`${data.wallet.name} is now the main wallet`, "success");
    await loadAllData();
  } catch (error) {
    Utils.showToast(`Failed: ${error.message}`, "error");
  }
}

function showArchiveModal(id) {
  currentArchiveWalletId = id;
  const wallet = walletsData.find((w) => w.id === parseInt(id, 10));

  const nameEl = $("#archive-wallet-name");
  if (nameEl && wallet) nameEl.textContent = wallet.name;

  showModal("archive-modal");
}

async function handleArchiveWallet() {
  if (!currentArchiveWalletId) return;

  const confirmBtn = $("#confirm-archive-btn");
  if (confirmBtn) {
    confirmBtn.disabled = true;
    confirmBtn.innerHTML = '<i class="icon-loader spin"></i> Archiving...';
  }

  try {
    const response = await fetch(`/api/wallets/${currentArchiveWalletId}/archive`, {
      method: "POST",
    });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || "Failed");

    Utils.showToast("Wallet archived", "success");
    closeArchiveModal();
    await loadAllData();
  } catch (error) {
    Utils.showToast(`Failed: ${error.message}`, "error");
    if (confirmBtn) {
      confirmBtn.disabled = false;
      confirmBtn.innerHTML = '<i class="icon-archive"></i> Yes, Archive';
    }
  }
}

function closeArchiveModal() {
  hideModal("archive-modal");
  currentArchiveWalletId = null;

  const confirmBtn = $("#confirm-archive-btn");
  if (confirmBtn) {
    confirmBtn.disabled = false;
    confirmBtn.innerHTML = '<i class="icon-archive"></i> Yes, Archive';
  }
}

async function restoreWallet(id) {
  try {
    const response = await fetch(`/api/wallets/${id}/restore`, { method: "POST" });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || "Failed");

    Utils.showToast("Wallet restored", "success");
    await loadAllData();
  } catch (error) {
    Utils.showToast(`Failed: ${error.message}`, "error");
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
    confirmBtn.innerHTML = '<i class="icon-loader spin"></i> Decrypting...';
  }

  try {
    const response = await fetch(`/api/wallets/${currentExportWalletId}/export`, {
      method: "POST",
    });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || "Failed");

    const keyDisplay = $("#export-key-display");
    const keyEl = $("#exported-key");

    if (keyEl) keyEl.textContent = data.private_key;
    if (keyDisplay) keyDisplay.classList.remove("hidden");
    if (confirmBtn) confirmBtn.classList.add("hidden");

    Utils.showToast("Key revealed - handle with care!", "warning");
  } catch (error) {
    Utils.showToast(`Failed: ${error.message}`, "error");
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

  const nameEl = $("#delete-wallet-name");
  if (nameEl && wallet) nameEl.textContent = wallet.name;

  showModal("delete-modal");
}

async function handleDeleteWallet() {
  if (!currentDeleteWalletId) return;

  const confirmBtn = $("#confirm-delete-btn");
  if (confirmBtn) {
    confirmBtn.disabled = true;
    confirmBtn.innerHTML = '<i class="icon-loader spin"></i> Deleting...';
  }

  try {
    const response = await fetch(`/api/wallets/${currentDeleteWalletId}`, {
      method: "DELETE",
    });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || "Failed");

    Utils.showToast("Wallet deleted permanently", "success");
    closeDeleteModal();
    await loadAllData();
  } catch (error) {
    Utils.showToast(`Failed: ${error.message}`, "error");
    if (confirmBtn) {
      confirmBtn.disabled = false;
      confirmBtn.innerHTML = '<i class="icon-trash-2"></i> Yes, Delete';
    }
  }
}

function closeDeleteModal() {
  hideModal("delete-modal");
  currentDeleteWalletId = null;

  const confirmBtn = $("#confirm-delete-btn");
  if (confirmBtn) {
    confirmBtn.disabled = false;
    confirmBtn.innerHTML = '<i class="icon-trash-2"></i> Yes, Delete';
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
  bulk.hideBulkImportModal();
  bulk.hideBulkExportModal();
}

// =============================================================================
// Bulk Operations (delegated to bulk_operations module)
// =============================================================================

// =============================================================================
// Utility Functions
// =============================================================================

function capitalizeFirst(str) {
  if (!str) return "";
  return str.charAt(0).toUpperCase() + str.slice(1);
}

function cleanup() {
  if (renderers) {
    renderers.destroyTokenTable();
  }
  walletsData = [];
  tokenHoldings = [];
  currentTab = "main";
  currentExportWalletId = null;
  currentArchiveWalletId = null;
  currentDeleteWalletId = null;
  poller = null;
  tabBar = null;
  bulk = null;
  renderers = null;
}

// =============================================================================
// Register Page
// =============================================================================

registerPage("wallets", createLifecycle());
