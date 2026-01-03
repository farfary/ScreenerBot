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

// Bulk import/export state
let importPreviewData = null;
let importColumnMapping = {};
let importCurrentStep = 1;
let selectedImportFile = null;

// =============================================================================
// Lifecycle
// =============================================================================

function createLifecycle() {
  return {
    async init(ctx) {
      console.log("[Wallets] Initializing...");

      // Initialize hints system
      await Hints.init();

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

      // Update panel visibility
      updatePanelVisibility();
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

  // Re-render content for the active tab
  if (tabId === "main") {
    renderMainWalletPanel();
  } else if (tabId === "secondaries") {
    renderSecondariesPanel();
  } else if (tabId === "archive") {
    renderArchivePanel();
  }
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

function setupEventHandlers() {
  // Header buttons
  const refreshBtn = $("#refresh-wallets-btn");
  const addBtn = $("#add-wallet-btn");
  const importBtn = $("#import-wallets-btn");
  const exportBtn = $("#export-wallets-btn");

  if (refreshBtn) {
    on(refreshBtn, "click", handleRefresh);
  }
  if (addBtn) {
    on(addBtn, "click", () => showModal("add-wallet-modal"));
  }
  if (importBtn) {
    on(importBtn, "click", () => showBulkImportModal());
  }
  if (exportBtn) {
    on(exportBtn, "click", () => showBulkExportModal());
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
  setupBulkImportModal();
  setupBulkExportModal();
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
  renderCurrentPanel();
  updateStats();
  updateWalletCountBadge();
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
// Render Functions
// =============================================================================

function renderCurrentPanel() {
  if (currentTab === "main") {
    renderMainWalletPanel();
  } else if (currentTab === "secondaries") {
    renderSecondariesPanel();
  } else if (currentTab === "archive") {
    renderArchivePanel();
  }
}

function updateStats() {
  const mainWallet = walletsData.find((w) => w.role === "main");
  const activeWallets = walletsData.filter((w) => w.is_active);
  const secondaryWallets = activeWallets.filter((w) => w.role === "secondary");

  // SOL Balance
  const solBalanceEl = $("#stat-sol-balance");
  if (solBalanceEl) {
    solBalanceEl.textContent =
      mainWallet?.balance != null ? Utils.formatSol(mainWallet.balance, { decimals: 4 }) : "—";
  }

  // Token Count
  const tokenCountEl = $("#stat-token-count");
  if (tokenCountEl) {
    tokenCountEl.textContent = tokenHoldings.length;
  }

  // Secondary Count
  const secondaryCountEl = $("#stat-secondary-count");
  if (secondaryCountEl) {
    secondaryCountEl.textContent = secondaryWallets.length;
  }

  // Last Activity
  const lastActivityEl = $("#stat-last-activity");
  if (lastActivityEl) {
    const lastUsed = mainWallet?.last_used_at;
    lastActivityEl.textContent = lastUsed
      ? Utils.formatTimestamp(lastUsed, { variant: "relative" })
      : "—";
  }
}

function updateWalletCountBadge() {
  const countBadge = $("#wallet-count-badge");
  if (countBadge) {
    const activeCount = walletsData.filter((w) => w.is_active).length;
    countBadge.textContent = `${activeCount} wallet${activeCount !== 1 ? "s" : ""}`;
  }
}

function renderMainWalletPanel() {
  const mainWallet = walletsData.find((w) => w.role === "main");
  const container = $("#main-wallet-card");

  if (!container) return;

  if (!mainWallet) {
    container.innerHTML = `
      <div class="empty-state">
        <i class="icon-wallet"></i>
        <p>No main wallet configured</p>
        <small>Click "Add Wallet" to create or import your first wallet</small>
      </div>
    `;
    return;
  }

  container.innerHTML = renderMainWalletCard(mainWallet);
  wireMainWalletActions(container);

  // Render token holdings
  renderTokenHoldings();
}

function renderMainWalletCard(wallet) {
  const balance = wallet.balance ?? null;
  const balanceDisplay = balance !== null ? Utils.formatSol(balance, { decimals: 4 }) : "—";
  const lastUsed = wallet.last_used_at
    ? Utils.formatTimestamp(wallet.last_used_at, { variant: "relative" })
    : "Never";
  const typeBadge = `<span class="wallet-badge ${wallet.wallet_type}">${capitalizeFirst(wallet.wallet_type)}</span>`;

  return `
    <div class="main-wallet-content">
      <div class="main-wallet-header">
        <div class="main-wallet-identity">
          <div class="main-wallet-icon">
            <i class="icon-star"></i>
          </div>
          <div class="main-wallet-info">
            <div class="main-wallet-name-row">
              <span class="main-wallet-name">${Utils.escapeHtml(wallet.name)}</span>
              <span class="wallet-badge main"><i class="icon-star"></i> Main</span>
              ${typeBadge}
            </div>
            <div class="main-wallet-address">
              <code>${wallet.address}</code>
              <button type="button" class="copy-btn" data-address="${wallet.address}" data-tooltip="Copy address">
                <i class="icon-copy"></i>
              </button>
            </div>
          </div>
        </div>
        <div class="main-wallet-balance">
          <span class="balance-value">${balanceDisplay}</span>
          <span class="balance-label">SOL</span>
        </div>
      </div>
      <div class="main-wallet-meta">
        <div class="meta-item">
          <i class="icon-clock"></i>
          <span>Last used: ${lastUsed}</span>
        </div>
        <div class="meta-item">
          <i class="icon-calendar"></i>
          <span>Created: ${Utils.formatTimestamp(wallet.created_at, { variant: "short" })}</span>
        </div>
        ${wallet.notes ? `<div class="meta-item notes"><i class="icon-file-text"></i><span>${Utils.escapeHtml(wallet.notes)}</span></div>` : ""}
      </div>
      <div class="main-wallet-actions">
        <button type="button" class="btn" data-action="export" data-id="${wallet.id}">
          <i class="icon-key"></i> Export Key
        </button>
      </div>
    </div>
  `;
}

function wireMainWalletActions(container) {
  // Copy button
  container.querySelectorAll(".copy-btn").forEach((btn) => {
    on(btn, "click", (e) => {
      e.stopPropagation();
      const address = btn.dataset.address;
      Utils.copyToClipboard(address);
      Utils.showToast("Address copied!", "success");
    });
  });

  // Export action
  container.querySelectorAll("[data-action='export']").forEach((btn) => {
    on(btn, "click", () => showExportModal(btn.dataset.id));
  });
}

function renderTokenHoldings() {
  const container = $("#token-holdings-table");
  if (!container) return;

  if (tokenHoldings.length === 0) {
    container.innerHTML = `
      <div class="empty-state">
        <i class="icon-inbox"></i>
        <p>No token holdings</p>
      </div>
    `;
    return;
  }

  // Sort by balance (highest first)
  const sorted = [...tokenHoldings].sort((a, b) => (b.ui_amount || 0) - (a.ui_amount || 0));

  const rows = sorted
    .map((token) => {
      const symbol = token.symbol || "Unknown";
      const balance =
        token.ui_amount != null ? Utils.formatNumber(token.ui_amount, { decimals: 4 }) : "—";
      const mint = token.mint || "";
      const shortMint = mint ? `${mint.slice(0, 6)}...${mint.slice(-4)}` : "—";

      return `
        <tr>
          <td class="token-symbol">${Utils.escapeHtml(symbol)}</td>
          <td class="token-balance">${balance}</td>
          <td class="token-mint">
            <code>${shortMint}</code>
            ${mint ? `<button type="button" class="copy-btn-mini" data-address="${mint}" data-tooltip="Copy mint"><i class="icon-copy"></i></button>` : ""}
          </td>
        </tr>
      `;
    })
    .join("");

  container.innerHTML = `
    <table class="holdings-table">
      <thead>
        <tr>
          <th>Token</th>
          <th>Balance</th>
          <th>Mint Address</th>
        </tr>
      </thead>
      <tbody>
        ${rows}
      </tbody>
    </table>
  `;

  // Wire copy buttons
  container.querySelectorAll(".copy-btn-mini").forEach((btn) => {
    on(btn, "click", (e) => {
      e.stopPropagation();
      Utils.copyToClipboard(btn.dataset.address);
      Utils.showToast("Mint address copied!", "success");
    });
  });
}

function renderSecondariesPanel() {
  const container = $("#secondaries-table-container");
  if (!container) return;

  const secondaryWallets = walletsData.filter((w) => w.role === "secondary" && w.is_active);

  if (secondaryWallets.length === 0) {
    container.innerHTML = `
      <div class="empty-state">
        <i class="icon-wallet"></i>
        <p>No secondary wallets</p>
        <small>Click "Add Wallet" to create additional wallets</small>
      </div>
    `;
    return;
  }

  const rows = secondaryWallets
    .map((wallet) => {
      const balance =
        wallet.balance != null ? Utils.formatSol(wallet.balance, { decimals: 4 }) : "—";
      const shortAddress = `${wallet.address.slice(0, 6)}...${wallet.address.slice(-4)}`;
      const created = Utils.formatTimestamp(wallet.created_at, { variant: "short" });

      return `
        <tr data-id="${wallet.id}">
          <td class="wallet-name-cell">${Utils.escapeHtml(wallet.name)}</td>
          <td class="wallet-address-cell">
            <code>${shortAddress}</code>
            <button type="button" class="copy-btn-mini" data-address="${wallet.address}" data-tooltip="Copy address">
              <i class="icon-copy"></i>
            </button>
          </td>
          <td class="wallet-balance-cell">${balance}</td>
          <td class="wallet-type-cell">
            <span class="wallet-badge ${wallet.wallet_type}">${capitalizeFirst(wallet.wallet_type)}</span>
          </td>
          <td class="wallet-created-cell">${created}</td>
          <td class="wallet-actions-cell">
            <div class="table-actions">
              <button type="button" class="btn btn-sm" data-action="set-main" data-id="${wallet.id}" data-tooltip="Set as main wallet">
                <i class="icon-star"></i>
              </button>
              <button type="button" class="btn btn-sm" data-action="export" data-id="${wallet.id}" data-tooltip="Export private key">
                <i class="icon-key"></i>
              </button>
              <button type="button" class="btn btn-sm" data-action="archive" data-id="${wallet.id}" data-tooltip="Archive wallet">
                <i class="icon-archive"></i>
              </button>
            </div>
          </td>
        </tr>
      `;
    })
    .join("");

  container.innerHTML = `
    <table class="wallets-table">
      <thead>
        <tr>
          <th>Name</th>
          <th>Address</th>
          <th>Balance</th>
          <th>Type</th>
          <th>Created</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        ${rows}
      </tbody>
    </table>
  `;

  wireTableActions(container);
}

function renderArchivePanel() {
  const container = $("#archive-table-container");
  if (!container) return;

  const archivedWallets = walletsData.filter((w) => w.role === "archive" || !w.is_active);

  if (archivedWallets.length === 0) {
    container.innerHTML = `
      <div class="empty-state">
        <i class="icon-archive"></i>
        <p>No archived wallets</p>
        <small>Archived wallets will appear here</small>
      </div>
    `;
    return;
  }

  const rows = archivedWallets
    .map((wallet) => {
      const balance =
        wallet.balance != null ? Utils.formatSol(wallet.balance, { decimals: 4 }) : "—";
      const shortAddress = `${wallet.address.slice(0, 6)}...${wallet.address.slice(-4)}`;
      const created = Utils.formatTimestamp(wallet.created_at, { variant: "short" });

      return `
        <tr data-id="${wallet.id}">
          <td class="wallet-name-cell">${Utils.escapeHtml(wallet.name)}</td>
          <td class="wallet-address-cell">
            <code>${shortAddress}</code>
            <button type="button" class="copy-btn-mini" data-address="${wallet.address}" data-tooltip="Copy address">
              <i class="icon-copy"></i>
            </button>
          </td>
          <td class="wallet-balance-cell">${balance}</td>
          <td class="wallet-type-cell">
            <span class="wallet-badge ${wallet.wallet_type}">${capitalizeFirst(wallet.wallet_type)}</span>
          </td>
          <td class="wallet-created-cell">${created}</td>
          <td class="wallet-actions-cell">
            <div class="table-actions">
              <button type="button" class="btn btn-sm success" data-action="restore" data-id="${wallet.id}" data-tooltip="Restore wallet">
                <i class="icon-archive-restore"></i>
              </button>
              <button type="button" class="btn btn-sm" data-action="export" data-id="${wallet.id}" data-tooltip="Export private key">
                <i class="icon-key"></i>
              </button>
              <button type="button" class="btn btn-sm danger" data-action="delete" data-id="${wallet.id}" data-tooltip="Delete permanently">
                <i class="icon-trash-2"></i>
              </button>
            </div>
          </td>
        </tr>
      `;
    })
    .join("");

  container.innerHTML = `
    <table class="wallets-table">
      <thead>
        <tr>
          <th>Name</th>
          <th>Address</th>
          <th>Balance</th>
          <th>Type</th>
          <th>Created</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        ${rows}
      </tbody>
    </table>
  `;

  wireTableActions(container);
}

function wireTableActions(container) {
  // Copy buttons
  container.querySelectorAll(".copy-btn-mini").forEach((btn) => {
    on(btn, "click", (e) => {
      e.stopPropagation();
      Utils.copyToClipboard(btn.dataset.address);
      Utils.showToast("Address copied!", "success");
    });
  });

  // Action buttons
  container.querySelectorAll("[data-action]").forEach((btn) => {
    on(btn, "click", (e) => {
      e.stopPropagation();
      handleWalletAction(btn.dataset.action, btn.dataset.id);
    });
  });
}

// =============================================================================
// Action Handlers
// =============================================================================

async function handleRefresh() {
  const btn = $("#refresh-wallets-btn");
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
  hideBulkImportModal();
  hideBulkExportModal();
}

// =============================================================================
// Bulk Import Modal Functions
// =============================================================================

function setupBulkImportModal() {
  const modal = $("#bulk-import-modal");
  if (!modal) return;

  // Close button
  const closeBtn = $("#bulk-import-modal-close");
  if (closeBtn) {
    on(closeBtn, "click", () => hideBulkImportModal());
  }

  // Backdrop click
  on(modal, "click", (e) => {
    if (e.target === modal) {
      hideBulkImportModal();
    }
  });

  // File dropzone
  setupFileDropzone();

  // Step 1 buttons
  const step1Cancel = $("#import-step1-cancel");
  const step1Next = $("#import-step1-next");
  if (step1Cancel) on(step1Cancel, "click", () => hideBulkImportModal());
  if (step1Next) on(step1Next, "click", () => goToImportStep(2));

  // Step 2 buttons
  const step2Back = $("#import-step2-back");
  const step2Execute = $("#import-step2-execute");
  if (step2Back) on(step2Back, "click", () => goToImportStep(1));
  if (step2Execute) on(step2Execute, "click", () => executeImport());

  // Step 3 buttons
  const step3Done = $("#import-step3-done");
  if (step3Done) {
    on(step3Done, "click", () => {
      hideBulkImportModal();
      loadAllData();
    });
  }
}

function setupFileDropzone() {
  const dropzone = $("#import-dropzone");
  const fileInput = $("#import-file-input");
  const clearBtn = $("#clear-file-btn");

  if (!dropzone || !fileInput) return;

  // Click to browse
  on(dropzone, "click", () => fileInput.click());

  // File selected
  on(fileInput, "change", (e) => {
    if (e.target.files && e.target.files[0]) {
      handleFileSelect(e.target.files[0]);
    }
  });

  // Drag and drop
  on(dropzone, "dragover", (e) => {
    e.preventDefault();
    dropzone.classList.add("drag-over");
  });

  on(dropzone, "dragleave", (e) => {
    e.preventDefault();
    dropzone.classList.remove("drag-over");
  });

  on(dropzone, "drop", (e) => {
    e.preventDefault();
    dropzone.classList.remove("drag-over");
    if (e.dataTransfer.files && e.dataTransfer.files[0]) {
      handleFileSelect(e.dataTransfer.files[0]);
    }
  });

  // Clear file
  if (clearBtn) {
    on(clearBtn, "click", (e) => {
      e.stopPropagation();
      clearSelectedFile();
    });
  }
}

function handleFileSelect(file) {
  const validExtensions = [".csv", ".xlsx", ".xls"];
  const ext = file.name.toLowerCase().slice(file.name.lastIndexOf("."));

  if (!validExtensions.includes(ext)) {
    Utils.showToast("Invalid file type. Please use CSV or Excel files.", "error");
    return;
  }

  selectedImportFile = file;

  // Update UI
  const dropzone = $("#import-dropzone");
  const fileInfo = $("#selected-file-info");
  const fileName = $("#selected-file-name");
  const nextBtn = $("#import-step1-next");

  if (dropzone) dropzone.classList.add("hidden");
  if (fileInfo) fileInfo.classList.remove("hidden");
  if (fileName) fileName.textContent = file.name;
  if (nextBtn) nextBtn.disabled = false;
}

function clearSelectedFile() {
  selectedImportFile = null;
  importPreviewData = null;

  const dropzone = $("#import-dropzone");
  const fileInfo = $("#selected-file-info");
  const fileInput = $("#import-file-input");
  const nextBtn = $("#import-step1-next");

  if (dropzone) dropzone.classList.remove("hidden");
  if (fileInfo) fileInfo.classList.add("hidden");
  if (fileInput) fileInput.value = "";
  if (nextBtn) nextBtn.disabled = true;
}

function showBulkImportModal() {
  resetImportState();
  showModal("bulk-import-modal");
}

function hideBulkImportModal() {
  hideModal("bulk-import-modal");
  resetImportState();
}

function resetImportState() {
  importPreviewData = null;
  importColumnMapping = {};
  importCurrentStep = 1;
  selectedImportFile = null;

  // Reset UI
  clearSelectedFile();
  updateImportStepIndicator(1);

  // Hide all steps except step 1
  const step1 = $("#import-step-1");
  const step2 = $("#import-step-2");
  const step3 = $("#import-step-3");
  if (step1) step1.classList.remove("hidden");
  if (step2) step2.classList.add("hidden");
  if (step3) step3.classList.add("hidden");
}

function updateImportStepIndicator(step) {
  const steps = document.querySelectorAll(".import-step");
  steps.forEach((stepEl) => {
    const stepNum = parseInt(stepEl.dataset.step, 10);
    stepEl.classList.remove("active", "completed");
    if (stepNum < step) {
      stepEl.classList.add("completed");
    } else if (stepNum === step) {
      stepEl.classList.add("active");
    }
  });
}

async function goToImportStep(step) {
  if (step === 2 && selectedImportFile) {
    // Upload file and get preview
    const success = await uploadFileForPreview();
    if (!success) return;
  }

  importCurrentStep = step;
  updateImportStepIndicator(step);

  const step1 = $("#import-step-1");
  const step2 = $("#import-step-2");
  const step3 = $("#import-step-3");

  if (step1) step1.classList.toggle("hidden", step !== 1);
  if (step2) step2.classList.toggle("hidden", step !== 2);
  if (step3) step3.classList.toggle("hidden", step !== 3);
}

async function uploadFileForPreview() {
  if (!selectedImportFile) return false;

  const nextBtn = $("#import-step1-next");
  if (nextBtn) {
    nextBtn.disabled = true;
    nextBtn.innerHTML = '<i class="icon-loader spin"></i> Processing...';
  }

  try {
    const formData = new FormData();
    formData.append("file", selectedImportFile);

    const response = await fetch("/api/wallets/import/preview", {
      method: "POST",
      body: formData,
    });

    const data = await response.json();
    if (!response.ok) {
      throw new Error(data.error || data.message || "Failed to process file");
    }

    importPreviewData = data;
    renderColumnMapping(data);
    renderPreviewTable(data);
    updateImportSummary(data);

    return true;
  } catch (error) {
    console.error("[Wallets] File preview failed:", error);
    Utils.showToast(`Failed to process file: ${error.message}`, "error");
    return false;
  } finally {
    if (nextBtn) {
      nextBtn.disabled = false;
      nextBtn.innerHTML = '<i class="icon-arrow-right"></i> Next';
    }
  }
}

function renderColumnMapping(preview) {
  const container = $("#column-mapping-grid");
  if (!container) return;

  const columns = preview.columns || [];
  const fields = [
    { id: "name", label: "Wallet Name", required: true },
    { id: "private_key", label: "Private Key", required: true },
    { id: "notes", label: "Notes", required: false },
  ];

  const optionsList = columns
    .map((col, idx) => `<option value="${idx}">${Utils.escapeHtml(col)}</option>`)
    .join("");

  container.innerHTML = fields
    .map((field) => {
      const requiredMark = field.required ? '<span class="required">*</span>' : "";
      // Try to auto-detect column
      const autoIdx = autoDetectColumn(field.id, columns);
      if (autoIdx >= 0) {
        importColumnMapping[field.id] = autoIdx;
      }

      return `
        <div class="mapping-field">
          <label>${field.label} ${requiredMark}</label>
          <select data-field="${field.id}">
            <option value="">-- Select column --</option>
            ${optionsList}
          </select>
        </div>
      `;
    })
    .join("");

  // Set auto-detected values and wire events
  container.querySelectorAll("select").forEach((select) => {
    const fieldId = select.dataset.field;
    if (importColumnMapping[fieldId] !== undefined) {
      select.value = importColumnMapping[fieldId];
    }

    on(select, "change", () => {
      const val = select.value;
      if (val === "") {
        delete importColumnMapping[fieldId];
      } else {
        importColumnMapping[fieldId] = parseInt(val, 10);
      }
      updatePreviewWithMapping();
      validateImportMapping();
    });
  });

  validateImportMapping();
}

function autoDetectColumn(fieldId, columns) {
  const patterns = {
    name: ["name", "wallet", "label", "title"],
    private_key: ["private", "key", "secret", "privatekey", "private_key"],
    notes: ["note", "notes", "description", "memo", "comment"],
  };

  const fieldPatterns = patterns[fieldId] || [];
  for (let i = 0; i < columns.length; i++) {
    const colLower = columns[i].toLowerCase();
    if (fieldPatterns.some((p) => colLower.includes(p))) {
      return i;
    }
  }
  return -1;
}

function renderPreviewTable(preview) {
  const container = $("#preview-table-wrapper");
  if (!container) return;

  const columns = preview.columns || [];
  const rows = preview.rows || [];

  if (rows.length === 0) {
    container.innerHTML = '<p class="info-text">No data rows found in file</p>';
    return;
  }

  const headerCells = columns.map((col) => `<th>${Utils.escapeHtml(col)}</th>`).join("");

  const bodyRows = rows
    .slice(0, 5)
    .map((row, rowIdx) => {
      const validation = preview.validations?.[rowIdx];
      const statusBadge = renderValidationBadge(validation);
      const cells = row.map((cell) => `<td>${Utils.escapeHtml(cell || "")}</td>`).join("");
      return `<tr>${cells}<td>${statusBadge}</td></tr>`;
    })
    .join("");

  container.innerHTML = `
    <table class="preview-table">
      <thead>
        <tr>${headerCells}<th>Status</th></tr>
      </thead>
      <tbody>
        ${bodyRows}
      </tbody>
    </table>
  `;
}

function renderValidationBadge(validation) {
  if (!validation) return '<span class="validation-badge">—</span>';

  if (validation.status === "valid") {
    return '<span class="validation-badge valid"><i class="icon-check"></i> Valid</span>';
  } else if (validation.status === "duplicate") {
    return '<span class="validation-badge duplicate"><i class="icon-copy"></i> Duplicate</span>';
  } else {
    return `<span class="validation-badge invalid"><i class="icon-x"></i> ${Utils.escapeHtml(validation.reason || "Invalid")}</span>`;
  }
}

function updatePreviewWithMapping() {
  // Re-validate with current mapping if needed
  if (importPreviewData) {
    renderPreviewTable(importPreviewData);
  }
}

function updateImportSummary(preview) {
  const validations = preview.validations || [];
  let valid = 0,
    invalid = 0,
    duplicate = 0;

  validations.forEach((v) => {
    if (v.status === "valid") valid++;
    else if (v.status === "duplicate") duplicate++;
    else invalid++;
  });

  const validEl = $("#valid-count");
  const invalidEl = $("#invalid-count");
  const duplicateEl = $("#duplicate-count");

  if (validEl) validEl.textContent = valid;
  if (invalidEl) invalidEl.textContent = invalid;
  if (duplicateEl) duplicateEl.textContent = duplicate;
}

function validateImportMapping() {
  const executeBtn = $("#import-step2-execute");
  if (!executeBtn) return;

  const hasName = importColumnMapping.name !== undefined;
  const hasKey = importColumnMapping.private_key !== undefined;
  const hasValidRows = importPreviewData?.validations?.some((v) => v.status === "valid");

  executeBtn.disabled = !(hasName && hasKey && hasValidRows);
}

async function executeImport() {
  const executeBtn = $("#import-step2-execute");
  if (executeBtn) {
    executeBtn.disabled = true;
    executeBtn.innerHTML = '<i class="icon-loader spin"></i> Importing...';
  }

  try {
    const response = await fetch("/api/wallets/import/execute", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        preview_id: importPreviewData?.preview_id,
        column_mapping: importColumnMapping,
      }),
    });

    const data = await response.json();
    if (!response.ok) {
      throw new Error(data.error || data.message || "Import failed");
    }

    renderImportResults(data);
    goToImportStep(3);

    const successCount = data.results?.filter((r) => r.success).length || 0;
    Utils.showToast(`Imported ${successCount} wallet(s)`, "success");
  } catch (error) {
    console.error("[Wallets] Import failed:", error);
    Utils.showToast(`Import failed: ${error.message}`, "error");
  } finally {
    if (executeBtn) {
      executeBtn.disabled = false;
      executeBtn.innerHTML = '<i class="icon-upload"></i> Import Wallets';
    }
  }
}

function renderImportResults(data) {
  const headerEl = $("#import-results-header");
  const tableEl = $("#import-results-table");

  if (!headerEl || !tableEl) return;

  const results = data.results || [];
  const successCount = results.filter((r) => r.success).length;
  const errorCount = results.filter((r) => !r.success).length;

  // Determine result type
  let resultClass, icon, title, subtitle;
  if (errorCount === 0) {
    resultClass = "success";
    icon = "icon-circle-check";
    title = "Import Successful";
    subtitle = `All ${successCount} wallet(s) imported successfully`;
  } else if (successCount > 0) {
    resultClass = "partial";
    icon = "icon-triangle-alert";
    title = "Partial Success";
    subtitle = `${successCount} imported, ${errorCount} failed`;
  } else {
    resultClass = "error";
    icon = "icon-circle-x";
    title = "Import Failed";
    subtitle = `All ${errorCount} wallet(s) failed to import`;
  }

  headerEl.className = `import-results-header ${resultClass}`;
  headerEl.innerHTML = `
    <div class="results-summary">
      <i class="${icon}"></i>
      <div class="results-text">
        <strong>${title}</strong>
        <span>${subtitle}</span>
      </div>
    </div>
  `;

  // Render results table
  const rows = results
    .map((result) => {
      const rowClass = result.success ? "success-row" : "error-row";
      const statusIcon = result.success
        ? '<span class="result-status success"><i class="icon-check"></i> Imported</span>'
        : `<span class="result-status error"><i class="icon-x"></i> ${Utils.escapeHtml(result.error || "Failed")}</span>`;

      return `
        <tr class="${rowClass}">
          <td>${Utils.escapeHtml(result.name || "—")}</td>
          <td><code>${result.address ? `${result.address.slice(0, 8)}...${result.address.slice(-6)}` : "—"}</code></td>
          <td>${statusIcon}</td>
        </tr>
      `;
    })
    .join("");

  tableEl.innerHTML = `
    <table class="results-table">
      <thead>
        <tr>
          <th>Name</th>
          <th>Address</th>
          <th>Status</th>
        </tr>
      </thead>
      <tbody>
        ${rows}
      </tbody>
    </table>
  `;
}

// =============================================================================
// Bulk Export Modal Functions
// =============================================================================

function setupBulkExportModal() {
  const modal = $("#bulk-export-modal");
  const confirmModal = $("#export-keys-confirm-modal");

  if (!modal) return;

  // Enhance native selects with custom styling
  enhanceAllSelects(modal);

  // Close buttons
  const closeBtn = $("#bulk-export-modal-close");
  const cancelBtn = $("#export-cancel-btn");
  if (closeBtn) on(closeBtn, "click", () => hideBulkExportModal());
  if (cancelBtn) on(cancelBtn, "click", () => hideBulkExportModal());

  // Backdrop click
  on(modal, "click", (e) => {
    if (e.target === modal) {
      hideBulkExportModal();
    }
  });

  // Safe export button
  const safeExportBtn = $("#export-safe-btn");
  if (safeExportBtn) {
    on(safeExportBtn, "click", () => exportWallets(false));
  }

  // Dangerous export button - shows confirmation
  const dangerExportBtn = $("#export-with-keys-btn");
  if (dangerExportBtn) {
    on(dangerExportBtn, "click", () => showExportKeysConfirmation());
  }

  // Setup confirmation modal
  if (confirmModal) {
    const confirmClose = $("#export-keys-confirm-close");
    const confirmCancel = $("#export-keys-confirm-cancel");
    const confirmInput = $("#export-confirm-input");
    const confirmBtn = $("#export-keys-confirm-btn");

    if (confirmClose) on(confirmClose, "click", () => hideExportKeysConfirmation());
    if (confirmCancel) on(confirmCancel, "click", () => hideExportKeysConfirmation());

    on(confirmModal, "click", (e) => {
      if (e.target === confirmModal) {
        hideExportKeysConfirmation();
      }
    });

    if (confirmInput && confirmBtn) {
      on(confirmInput, "input", () => {
        confirmBtn.disabled = confirmInput.value !== "EXPORT KEYS";
      });
    }

    if (confirmBtn) {
      on(confirmBtn, "click", () => {
        hideExportKeysConfirmation();
        exportWallets(true);
      });
    }
  }
}

function showBulkExportModal() {
  showModal("bulk-export-modal");
}

function hideBulkExportModal() {
  hideModal("bulk-export-modal");
}

function showExportKeysConfirmation() {
  const activeCount = walletsData.filter((w) => w.is_active).length;
  const includeInactive = $("#export-include-inactive")?.checked;
  const totalCount = includeInactive ? walletsData.length : activeCount;

  const countEl = $("#export-wallet-count");
  if (countEl) countEl.textContent = totalCount;

  const confirmInput = $("#export-confirm-input");
  const confirmBtn = $("#export-keys-confirm-btn");
  if (confirmInput) confirmInput.value = "";
  if (confirmBtn) confirmBtn.disabled = true;

  showModal("export-keys-confirm-modal");
}

function hideExportKeysConfirmation() {
  hideModal("export-keys-confirm-modal");
}

async function exportWallets(includeKeys) {
  const format = $("#export-format")?.value || "csv";
  const includeInactive = $("#export-include-inactive")?.checked || false;

  const exportBtn = includeKeys ? $("#export-with-keys-btn") : $("#export-safe-btn");
  const originalHtml = exportBtn?.innerHTML;

  if (exportBtn) {
    exportBtn.disabled = true;
    exportBtn.innerHTML = '<i class="icon-loader spin"></i> Exporting...';
  }

  try {
    const endpoint = includeKeys ? "/api/wallets/export/with-keys" : "/api/wallets/export";
    const response = await fetch(endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        format,
        include_inactive: includeInactive,
      }),
    });

    if (!response.ok) {
      const data = await response.json();
      throw new Error(data.error || data.message || "Export failed");
    }

    // Get filename from header or generate one
    const contentDisposition = response.headers.get("Content-Disposition");
    let filename = `wallets_export.${format}`;
    if (contentDisposition) {
      const match = contentDisposition.match(/filename="?([^"]+)"?/);
      if (match) filename = match[1];
    }

    // Download the file
    const blob = await response.blob();
    const url = window.URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    window.URL.revokeObjectURL(url);

    Utils.showToast(`Exported wallets to ${filename}`, "success");
    hideBulkExportModal();
  } catch (error) {
    console.error("[Wallets] Export failed:", error);
    Utils.showToast(`Export failed: ${error.message}`, "error");
  } finally {
    if (exportBtn) {
      exportBtn.disabled = false;
      exportBtn.innerHTML = originalHtml;
    }
  }
}

// =============================================================================
// Utility Functions
// =============================================================================

function capitalizeFirst(str) {
  if (!str) return "";
  return str.charAt(0).toUpperCase() + str.slice(1);
}

function cleanup() {
  walletsData = [];
  tokenHoldings = [];
  currentTab = "main";
  currentExportWalletId = null;
  currentArchiveWalletId = null;
  currentDeleteWalletId = null;
  poller = null;
  tabBar = null;

  // Reset import/export state
  importPreviewData = null;
  importColumnMapping = {};
  importCurrentStep = 1;
  selectedImportFile = null;
}

// =============================================================================
// Register Page
// =============================================================================

registerPage("wallets", createLifecycle());
