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

  if (refreshBtn) {
    on(refreshBtn, "click", handleRefresh);
  }
  if (addBtn) {
    on(addBtn, "click", () => showModal("add-wallet-modal"));
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
    solBalanceEl.textContent = mainWallet?.balance != null ? Utils.formatSol(mainWallet.balance, { decimals: 4 }) : "—";
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
    lastActivityEl.textContent = lastUsed ? Utils.formatTimestamp(lastUsed, { variant: "relative" }) : "—";
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
  const lastUsed = wallet.last_used_at ? Utils.formatTimestamp(wallet.last_used_at, { variant: "relative" }) : "Never";
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
      const balance = token.ui_amount != null ? Utils.formatNumber(token.ui_amount, { decimals: 4 }) : "—";
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
      const balance = wallet.balance != null ? Utils.formatSol(wallet.balance, { decimals: 4 }) : "—";
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
      const balance = wallet.balance != null ? Utils.formatSol(wallet.balance, { decimals: 4 }) : "—";
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
    const response = await fetch(`/api/wallets/${currentArchiveWalletId}/archive`, { method: "POST" });
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
}

// =============================================================================
// Register Page
// =============================================================================

registerPage("wallets", createLifecycle());
