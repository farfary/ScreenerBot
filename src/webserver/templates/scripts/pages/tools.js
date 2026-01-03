/**
 * Tools Page Module
 * Provides utility tools for wallet management, token operations, and trading
 */

import { registerPage } from "../core/lifecycle.js";
import { $, $$, on, off } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as Hints from "../core/hints.js";
import { HintTrigger } from "../ui/hint_popover.js";
import { DataTable } from "../ui/data_table.js";
import { ToolFavorites } from "../ui/tool_favorites.js";
import { enhanceAllSelects } from "../ui/custom_select.js";
import { PoolSelector } from "../ui/pool_selector.js";

// =============================================================================
// Constants
// =============================================================================

const TOOLS_STATE_KEY = "tools.page";
const DEFAULT_TOOL = "wallet-cleanup";

/**
 * Feature status values from the API
 */
const FEATURE_STATUS = {
  AVAILABLE: "available",
  COMING_SOON: "coming_soon",
  BETA: "beta",
  DISABLED: "disabled",
};

/**
 * Maps tool IDs (from HTML data-tool) to feature API keys
 */
const TOOL_TO_FEATURE_MAP = {
  "wallet-cleanup": "wallet_cleanup",
  "burn-tokens": "burn_tokens",
  "token-analyzer": "token_analyzer",
  "create-token": "create_token",
  "trade-watcher": "trade_watcher",
  "token-watch": "holder_watch",
  "volume-aggregator": "volume_aggregator",
  "buy-multi-wallets": "multi_buy",
  "sell-multi-wallets": "multi_sell",
  "wallet-consolidation": "wallet_consolidation",
  "airdrop-checker": "airdrop_checker",
  "wallet-generator": "wallet_generator",
};

/**
 * Status display configuration
 */
const STATUS_CONFIG = {
  [FEATURE_STATUS.COMING_SOON]: {
    label: "Coming Soon",
    cssClass: "coming-soon",
    dataStatus: "coming",
    tooltip: "Coming soon",
  },
  [FEATURE_STATUS.BETA]: {
    label: "Beta",
    cssClass: "beta",
    dataStatus: "beta",
    tooltip: "Beta - may have bugs",
  },
  [FEATURE_STATUS.DISABLED]: {
    label: "Disabled",
    cssClass: "disabled",
    dataStatus: "disabled",
    tooltip: "Currently disabled",
  },
};

/**
 * Tool definitions with metadata and content generators
 */
const TOOL_DEFINITIONS = {
  "wallet-cleanup": {
    id: "wallet-cleanup",
    title: "Wallet Cleanup",
    description: "Close empty Associated Token Accounts to reclaim SOL",
    icon: "icon-trash-2",
    category: "wallet",
    render: renderWalletCleanupTool,
  },
  "burn-tokens": {
    id: "burn-tokens",
    title: "Burn Tokens",
    description: "Permanently destroy tokens from your wallet",
    icon: "icon-flame",
    category: "wallet",
    render: renderBurnTokensTool,
  },
  "token-analyzer": {
    id: "token-analyzer",
    title: "Token Analyzer",
    description: "Deep analysis of any Solana token with multi-dimensional insights",
    icon: "icon-search",
    category: "token",
    render: renderTokenAnalyzerTool,
  },
  "create-token": {
    id: "create-token",
    title: "Create Token",
    description: "Deploy a new SPL token on Solana",
    icon: "icon-circle-plus",
    category: "token",
    render: renderCreateTokenTool,
  },
  "token-watch": {
    id: "token-watch",
    title: "Holder Watch",
    description: "Track and monitor new token holders in real-time",
    icon: "icon-eye",
    category: "single-token",
    render: renderTokenWatchTool,
  },
  "trade-watcher": {
    id: "trade-watcher",
    title: "Trade Watcher",
    description: "Monitor token trades and trigger automatic buy/sell actions",
    icon: "icon-activity",
    category: "single-token",
    render: renderTradeWatcherTool,
  },
  "volume-aggregator": {
    id: "volume-aggregator",
    title: "Volume Aggregator",
    description: "Generate trading volume using multiple wallets",
    icon: "icon-chart-bar",
    category: "single-token",
    render: renderVolumeAggregatorTool,
  },
  "buy-multi-wallets": {
    id: "buy-multi-wallets",
    title: "Multi-Buy",
    description: "Execute coordinated buy orders across multiple wallets with randomized amounts",
    icon: "icon-shopping-cart",
    category: "single-token",
    render: renderBuyMultiWalletsTool,
  },
  "sell-multi-wallets": {
    id: "sell-multi-wallets",
    title: "Multi-Sell",
    description: "Execute coordinated sell orders across multiple wallets with SOL consolidation",
    icon: "icon-package",
    category: "single-token",
    render: renderSellMultiWalletsTool,
  },
  "wallet-consolidation": {
    id: "wallet-consolidation",
    title: "Wallet Consolidation",
    description: "Consolidate SOL and tokens from sub-wallets back to main wallet",
    icon: "icon-git-merge",
    category: "utilities",
    render: renderWalletConsolidationTool,
  },
  "airdrop-checker": {
    id: "airdrop-checker",
    title: "Airdrop Checker",
    description: "Check for pending airdrops and claimable rewards",
    icon: "icon-gift",
    category: "more",
    render: renderAirdropCheckerTool,
  },
  "wallet-generator": {
    id: "wallet-generator",
    title: "Wallet Generator",
    description: "Generate new Solana keypairs securely",
    icon: "icon-key",
    category: "more",
    render: renderWalletGeneratorTool,
  },
};

// =============================================================================
// State
// =============================================================================

let currentTool = null;
let toolClickHandler = null;
let featureStatus = {}; // Stores feature status from API

// =============================================================================
// Feature Status Functions
// =============================================================================

/**
 * Fetch feature status from the API
 * @returns {Promise<Object>} Feature status by tool key
 */
async function fetchFeatureStatus() {
  try {
    const response = await fetch("/api/features");
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    const data = await response.json();
    return data.tools || {};
  } catch (error) {
    console.warn("Failed to fetch feature status, defaulting to available:", error);
    // Default to all available if API fails
    return {};
  }
}

/**
 * Get the feature status for a tool
 * @param {string} toolId - The tool ID (e.g., "wallet-cleanup")
 * @returns {string} The status ("available", "coming_soon", "beta", "disabled")
 */
function getToolFeatureStatus(toolId) {
  const featureKey = TOOL_TO_FEATURE_MAP[toolId];
  if (!featureKey || !featureStatus[featureKey]) {
    return FEATURE_STATUS.AVAILABLE;
  }
  return featureStatus[featureKey];
}

/**
 * Check if a tool is available (can be clicked/used)
 * @param {string} toolId - The tool ID
 * @returns {boolean} True if tool is available or beta
 */
function _isToolAvailable(toolId) {
  const status = getToolFeatureStatus(toolId);
  return status === FEATURE_STATUS.AVAILABLE || status === FEATURE_STATUS.BETA;
}

/**
 * Apply feature status to all tool navigation items
 */
function applyFeatureStatusToUI() {
  const navItems = $$(".nav-item[data-tool]");

  navItems.forEach((navItem) => {
    const toolId = navItem.dataset.tool;
    const status = getToolFeatureStatus(toolId);

    // Remove any existing status badges
    const existingBadge = navItem.querySelector(".status-badge");
    if (existingBadge) {
      existingBadge.remove();
    }

    // If available, ensure clean state
    if (status === FEATURE_STATUS.AVAILABLE) {
      navItem.dataset.status = "ready";
      navItem.classList.remove("feature-disabled", "feature-beta", "feature-coming-soon");
      const statusIndicator = navItem.querySelector(".nav-item-status");
      if (statusIndicator) {
        statusIndicator.dataset.tooltip = "Ready to use";
      }
      return;
    }

    // Get status configuration
    const config = STATUS_CONFIG[status];
    if (!config) return;

    // Apply data-status attribute
    navItem.dataset.status = config.dataStatus;

    // Add appropriate class
    navItem.classList.remove("feature-disabled", "feature-beta", "feature-coming-soon");
    if (status === FEATURE_STATUS.DISABLED) {
      navItem.classList.add("feature-disabled");
    } else if (status === FEATURE_STATUS.BETA) {
      navItem.classList.add("feature-beta");
    } else if (status === FEATURE_STATUS.COMING_SOON) {
      navItem.classList.add("feature-coming-soon");
    }

    // Update status indicator tooltip
    const statusIndicator = navItem.querySelector(".nav-item-status");
    if (statusIndicator) {
      statusIndicator.dataset.tooltip = config.tooltip;
    }

    // Add status badge for non-available tools
    if (status !== FEATURE_STATUS.AVAILABLE) {
      const badge = document.createElement("span");
      badge.className = `status-badge ${config.cssClass}`;
      badge.textContent = config.label;
      navItem.appendChild(badge);
    }
  });
}

// =============================================================================
// Tool Renderers
// =============================================================================

function renderWalletCleanupTool(container, actionsContainer) {
  const hint = Hints.getHint("tools.walletCleanup");
  const hintHtml = hint ? HintTrigger.render(hint, "tools.walletCleanup", { size: "sm" }) : "";

  // Load auto cleanup state from localStorage
  const autoCleanupEnabled = localStorage.getItem("ata.autoCleanup") === "true";

  container.innerHTML = `
    <div class="tool-panel wallet-cleanup-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-search"></i> Scan Results</h3>
          <div class="section-header-actions">
            ${hintHtml}
          </div>
        </div>
        <div class="section-content">
          <div class="auto-cleanup-row">
            <div class="auto-cleanup-info">
              <label class="toggle-switch">
                <input type="checkbox" id="auto-cleanup-toggle" ${autoCleanupEnabled ? "checked" : ""}>
                <span class="toggle-slider"></span>
              </label>
              <div class="auto-cleanup-text">
                <span class="auto-cleanup-label">Auto Cleanup</span>
                <span class="auto-cleanup-desc">Automatically close empty ATAs every 5 minutes (preference saved locally)</span>
              </div>
            </div>
            <div class="auto-cleanup-status ${autoCleanupEnabled ? "active" : ""}" id="auto-cleanup-status">
              <i class="icon-${autoCleanupEnabled ? "check-circle" : "circle"}"></i>
              <span>${autoCleanupEnabled ? "Active" : "Inactive"}</span>
            </div>
          </div>
          <div class="scan-stats">
            <div class="stat-card">
              <div class="stat-value" id="empty-atas-count">—</div>
              <div class="stat-label">Empty ATAs</div>
            </div>
            <div class="stat-card">
              <div class="stat-value" id="reclaimable-sol">—</div>
              <div class="stat-label">Reclaimable SOL</div>
            </div>
            <div class="stat-card">
              <div class="stat-value" id="failed-atas-count">—</div>
              <div class="stat-label">Failed (cached)</div>
            </div>
          </div>
          <div class="ata-list" id="ata-list">
            <div class="empty-state">
              <i class="icon-scan"></i>
              <p>Click "Scan Wallet" to find empty ATAs</p>
              <small>This will check all token accounts in your wallet</small>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;

  HintTrigger.initAll();

  // Wire up auto cleanup toggle
  const autoCleanupToggle = $("#auto-cleanup-toggle");
  if (autoCleanupToggle) {
    on(autoCleanupToggle, "change", handleAutoCleanupToggle);
  }

  actionsContainer.innerHTML = `
    <button class="btn primary" id="scan-atas-btn">
      <i class="icon-scan"></i> Scan Wallet
    </button>
    <button class="btn success" id="cleanup-atas-btn" disabled>
      <i class="icon-trash-2"></i> Cleanup All
    </button>
  `;

  // Wire up event handlers
  const scanBtn = $("#scan-atas-btn");
  const cleanupBtn = $("#cleanup-atas-btn");

  if (scanBtn) {
    on(scanBtn, "click", handleScanATAs);
  }
  if (cleanupBtn) {
    on(cleanupBtn, "click", handleCleanupATAs);
  }
}

/**
 * Handle auto cleanup toggle change
 *
 * Note: This toggle saves to localStorage and is a frontend-only preference.
 * The auto-cleanup runs on app startup and periodically when the Tools page
 * is active. The preference persists across sessions via localStorage.
 */
function handleAutoCleanupToggle(event) {
  const enabled = event.target.checked;
  localStorage.setItem("ata.autoCleanup", enabled ? "true" : "false");

  // Update status indicator
  const statusEl = $("#auto-cleanup-status");
  if (statusEl) {
    statusEl.className = `auto-cleanup-status ${enabled ? "active" : ""}`;
    statusEl.innerHTML = `
      <i class="icon-${enabled ? "check-circle" : "circle"}"></i>
      <span>${enabled ? "Active" : "Inactive"}</span>
    `;
  }

  Utils.showToast(
    enabled
      ? "Auto cleanup enabled - empty ATAs will be closed automatically"
      : "Auto cleanup disabled",
    enabled ? "success" : "info"
  );
}

async function handleScanATAs() {
  const scanBtn = $("#scan-atas-btn");
  const listEl = $("#ata-list");

  if (!scanBtn || !listEl) return;

  scanBtn.disabled = true;
  scanBtn.innerHTML = '<i class="icon-loader spin"></i> Scanning...';
  listEl.innerHTML =
    '<div class="loading-state"><i class="icon-loader spin"></i> Scanning wallet...</div>';

  try {
    const response = await fetch("/api/tools/ata-scan");
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    const stats = await response.json();

    // Check for error response
    if (stats.error) {
      throw new Error(stats.error);
    }

    const countEl = $("#empty-atas-count");
    const solEl = $("#reclaimable-sol");
    const failedEl = $("#failed-atas-count");
    const cleanupBtn = $("#cleanup-atas-btn");

    if (countEl) countEl.textContent = stats.empty_count || 0;
    if (solEl) solEl.textContent = Utils.formatSol(stats.reclaimable_sol || 0);
    if (failedEl) failedEl.textContent = stats.failed_count || 0;

    if (stats.empty_count > 0) {
      listEl.innerHTML = `
        <div class="success-state">
          <i class="icon-circle-check"></i>
          <p>Found ${stats.empty_count} empty ATAs worth ~${Utils.formatSol(stats.reclaimable_sol || 0)} SOL</p>
        </div>
      `;
      if (cleanupBtn) cleanupBtn.disabled = false;
    } else {
      listEl.innerHTML = `
        <div class="empty-state">
          <i class="icon-circle-check"></i>
          <p>No empty ATAs found - wallet is clean!</p>
        </div>
      `;
    }
  } catch (error) {
    console.error("ATA scan failed:", error);
    listEl.innerHTML = `
      <div class="error-state">
        <i class="icon-circle-alert"></i>
        <p>Scan failed: ${error.message}</p>
      </div>
    `;
    Utils.showToast("Failed to scan ATAs", "error");
  } finally {
    scanBtn.disabled = false;
    scanBtn.innerHTML = '<i class="icon-scan"></i> Scan Wallet';
  }
}

async function handleCleanupATAs() {
  const cleanupBtn = $("#cleanup-atas-btn");
  if (!cleanupBtn) return;

  cleanupBtn.disabled = true;
  cleanupBtn.innerHTML = '<i class="icon-loader spin"></i> Cleaning...';

  try {
    const response = await fetch("/api/tools/ata-cleanup", { method: "POST" });
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    const data = await response.json();

    // Check for error response
    if (data.error) {
      throw new Error(data.error);
    }

    Utils.showToast(`Cleaned ${data.closed_count || 0} ATAs`, "success");
    // Refresh scan
    handleScanATAs();
  } catch (error) {
    console.error("ATA cleanup failed:", error);
    Utils.showToast(`Cleanup failed: ${error.message}`, "error");
  } finally {
    cleanupBtn.disabled = false;
    cleanupBtn.innerHTML = '<i class="icon-trash-2"></i> Cleanup All';
  }
}

// =============================================================================
// Burn Tokens Tool State
// =============================================================================

let burnTokensState = {
  tokens: [],
  selectedMints: new Set(),
  isLoading: false,
  isBurning: false,
};

function renderBurnTokensTool(container, actionsContainer) {
  const hint = Hints.getHint("tools.burnTokens");
  const hintHtml = hint ? HintTrigger.render(hint, "tools.burnTokens", { size: "sm" }) : "";

  container.innerHTML = `
    <div class="tool-panel burn-tokens-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-flame"></i> Burn Tokens</h3>
          <div class="section-header-actions">
            ${hintHtml}
          </div>
        </div>
        <div class="section-content">
          <div class="burn-info-box">
            <i class="icon-info"></i>
            <div class="burn-info-content">
              <p><strong>What is burning?</strong></p>
              <p>Burning permanently destroys tokens, making them unrecoverable. After burning, run Wallet Cleanup to close empty ATAs and reclaim ~0.002 SOL rent per token.</p>
            </div>
          </div>
          
          <div class="burn-stats" id="burn-stats">
            <div class="stat-card">
              <div class="stat-value" id="burn-total-tokens">—</div>
              <div class="stat-label">Total Tokens</div>
            </div>
            <div class="stat-card">
              <div class="stat-value" id="burn-selected-count">0</div>
              <div class="stat-label">Selected</div>
            </div>
            <div class="stat-card">
              <div class="stat-value" id="burn-rent-reclaimable">—</div>
              <div class="stat-label">Rent Reclaimable</div>
            </div>
          </div>

          <div class="burn-token-list" id="burn-token-list">
            <div class="empty-state">
              <i class="icon-search"></i>
              <p>Click "Scan Wallet" to find tokens</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;

  HintTrigger.initAll();

  actionsContainer.innerHTML = `
    <button class="btn primary" id="scan-burn-tokens-btn">
      <i class="icon-search"></i> Scan Wallet
    </button>
    <button class="btn danger" id="burn-selected-btn" disabled>
      <i class="icon-flame"></i> Burn Selected (0)
    </button>
  `;

  // Wire up event handlers
  const scanBtn = $("#scan-burn-tokens-btn");
  const burnBtn = $("#burn-selected-btn");

  if (scanBtn) {
    on(scanBtn, "click", handleScanBurnTokens);
  }
  if (burnBtn) {
    on(burnBtn, "click", handleBurnSelectedTokens);
  }

  // Reset state
  burnTokensState = {
    tokens: [],
    selectedMints: new Set(),
    isLoading: false,
    isBurning: false,
  };
}

async function handleScanBurnTokens() {
  const scanBtn = $("#scan-burn-tokens-btn");
  const listEl = $("#burn-token-list");

  if (!scanBtn || !listEl || burnTokensState.isLoading) return;

  burnTokensState.isLoading = true;
  scanBtn.disabled = true;
  scanBtn.innerHTML = '<i class="icon-loader spin"></i> Scanning...';
  listEl.innerHTML =
    '<div class="loading-state"><i class="icon-loader spin"></i> Scanning wallet for tokens...</div>';

  try {
    const response = await fetch("/api/tools/burn-tokens/scan");
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    const result = await response.json();

    if (result.error) {
      throw new Error(result.error);
    }

    const data = result.data || result;
    burnTokensState.tokens = data.tokens || [];
    burnTokensState.selectedMints.clear();

    // Update stats
    const totalEl = $("#burn-total-tokens");
    const rentEl = $("#burn-rent-reclaimable");

    if (totalEl) totalEl.textContent = burnTokensState.tokens.length;
    if (rentEl) rentEl.textContent = Utils.formatSol(data.total_rent_reclaimable_sol || 0);

    // Render token list
    renderBurnTokenList();
  } catch (error) {
    console.error("Burn tokens scan failed:", error);
    listEl.innerHTML = `
      <div class="error-state">
        <i class="icon-circle-alert"></i>
        <p>Scan failed: ${error.message}</p>
      </div>
    `;
    Utils.showToast("Failed to scan tokens", "error");
  } finally {
    burnTokensState.isLoading = false;
    scanBtn.disabled = false;
    scanBtn.innerHTML = '<i class="icon-search"></i> Scan Wallet';
  }
}

function renderBurnTokenList() {
  const listEl = $("#burn-token-list");
  if (!listEl) return;

  const tokens = burnTokensState.tokens;

  if (tokens.length === 0) {
    listEl.innerHTML = `
      <div class="empty-state">
        <i class="icon-circle-check"></i>
        <p>No tokens found in wallet</p>
      </div>
    `;
    return;
  }

  // Group tokens by category
  const groups = {
    open_position: tokens.filter((t) => t.category === "open_position"),
    has_value: tokens.filter((t) => t.category === "has_value"),
    closed_position: tokens.filter((t) => t.category === "closed_position"),
    zero_liquidity: tokens.filter((t) => t.category === "zero_liquidity"),
  };

  let html = "";

  // Open Positions (warning, can't burn)
  if (groups.open_position.length > 0) {
    html += renderBurnCategory(
      "Open Positions",
      "icon-lock",
      groups.open_position,
      "Cannot burn tokens from open positions",
      "warning"
    );
  }

  // Has Value (caution)
  if (groups.has_value.length > 0) {
    html += renderBurnCategory(
      "Has Value",
      "icon-dollar-sign",
      groups.has_value,
      "Consider selling instead of burning",
      "caution"
    );
  }

  // Closed Positions
  if (groups.closed_position.length > 0) {
    html += renderBurnCategory(
      "Closed Positions",
      "icon-archive",
      groups.closed_position,
      "Leftovers from closed trades",
      "info"
    );
  }

  // Zero Liquidity (safe to burn)
  if (groups.zero_liquidity.length > 0) {
    html += renderBurnCategory(
      "Zero Liquidity",
      "icon-trash-2",
      groups.zero_liquidity,
      "Safe to burn - no market value",
      "safe"
    );
  }

  listEl.innerHTML = html;

  // Wire up checkbox events
  wireUpBurnCheckboxes();
}

function renderBurnCategory(title, icon, tokens, description, type) {
  const burnableTokens = tokens.filter((t) => t.can_burn);
  const allSelected =
    burnableTokens.length > 0 &&
    burnableTokens.every((t) => burnTokensState.selectedMints.has(t.mint));

  return `
    <div class="burn-category burn-category-${type}">
      <div class="burn-category-header">
        <div class="burn-category-info">
          <i class="${icon}"></i>
          <div class="burn-category-text">
            <span class="burn-category-title">${title}</span>
            <span class="burn-category-desc">${description}</span>
          </div>
          <span class="burn-category-count">${tokens.length}</span>
        </div>
        ${
          burnableTokens.length > 0
            ? `<label class="burn-select-all">
            <input type="checkbox" data-category="${type}" ${allSelected ? "checked" : ""}>
            <span>Select All</span>
          </label>`
            : ""
        }
      </div>
      <div class="burn-category-tokens">
        ${tokens.map((t) => renderBurnTokenRow(t)).join("")}
      </div>
    </div>
  `;
}

function renderBurnTokenRow(token) {
  const isSelected = burnTokensState.selectedMints.has(token.mint);
  const symbol = token.symbol || "Unknown";
  const displayName = token.name || token.mint.substring(0, 8) + "...";

  return `
    <div class="burn-token-row ${!token.can_burn ? "disabled" : ""} ${isSelected ? "selected" : ""}" data-mint="${token.mint}">
      <div class="burn-token-select">
        ${
          token.can_burn
            ? `<input type="checkbox" class="burn-token-checkbox" data-mint="${token.mint}" ${isSelected ? "checked" : ""}>`
            : `<i class="icon-lock" title="${token.burn_warning || "Cannot burn"}"></i>`
        }
      </div>
      <div class="burn-token-info">
        <div class="burn-token-name">
          <span class="burn-token-symbol">${symbol}</span>
          <span class="burn-token-title">${displayName}</span>
        </div>
        <div class="burn-token-mint" title="${token.mint}">
          ${token.mint.substring(0, 8)}...${token.mint.substring(token.mint.length - 6)}
        </div>
      </div>
      <div class="burn-token-balance">
        <span class="burn-token-amount">${Utils.formatCompactNumber(token.ui_amount)}</span>
        ${
          token.value_sol && token.value_sol > 0.0001
            ? `<span class="burn-token-value">~${Utils.formatSol(token.value_sol)}</span>`
            : "<span class=\"burn-token-value no-value\">No value</span>"
        }
      </div>
      <div class="burn-token-rent">
        ${token.can_burn ? `+${Utils.formatSol(token.rent_reclaimable_sol)}` : "—"}
      </div>
    </div>
  `;
}

function wireUpBurnCheckboxes() {
  // Individual token checkboxes
  const checkboxes = $$(".burn-token-checkbox");
  checkboxes.forEach((cb) => {
    on(cb, "change", (e) => {
      const mint = e.target.dataset.mint;
      if (e.target.checked) {
        burnTokensState.selectedMints.add(mint);
      } else {
        burnTokensState.selectedMints.delete(mint);
      }
      updateBurnTokenRowState(mint, e.target.checked);
      updateBurnSelectionUI();
    });
  });

  // Category select-all checkboxes
  const selectAlls = $$(".burn-select-all input");
  selectAlls.forEach((cb) => {
    on(cb, "change", (e) => {
      const category = e.target.dataset.category;
      const categoryTokens = burnTokensState.tokens.filter(
        (t) => t.can_burn && getCategoryType(t.category) === category
      );

      categoryTokens.forEach((t) => {
        if (e.target.checked) {
          burnTokensState.selectedMints.add(t.mint);
        } else {
          burnTokensState.selectedMints.delete(t.mint);
        }
        updateBurnTokenRowState(t.mint, e.target.checked);
      });

      updateBurnSelectionUI();
    });
  });
}

function getCategoryType(category) {
  const mapping = {
    open_position: "warning",
    has_value: "caution",
    closed_position: "info",
    zero_liquidity: "safe",
  };
  return mapping[category] || "info";
}

function updateBurnTokenRowState(mint, isSelected) {
  const row = $(`.burn-token-row[data-mint="${mint}"]`);
  const checkbox = $(`.burn-token-checkbox[data-mint="${mint}"]`);

  if (row) {
    row.classList.toggle("selected", isSelected);
  }
  if (checkbox) {
    checkbox.checked = isSelected;
  }
}

function updateBurnSelectionUI() {
  const count = burnTokensState.selectedMints.size;
  const countEl = $("#burn-selected-count");
  const burnBtn = $("#burn-selected-btn");

  if (countEl) countEl.textContent = count;
  if (burnBtn) {
    burnBtn.disabled = count === 0 || burnTokensState.isBurning;
    burnBtn.innerHTML = `<i class="icon-flame"></i> Burn Selected (${count})`;
  }

  // Update category select-all states
  const categories = ["warning", "caution", "info", "safe"];
  categories.forEach((cat) => {
    const selectAll = $(`.burn-select-all input[data-category="${cat}"]`);
    if (selectAll) {
      const categoryTokens = burnTokensState.tokens.filter(
        (t) => t.can_burn && getCategoryType(t.category) === cat
      );
      const allSelected =
        categoryTokens.length > 0 &&
        categoryTokens.every((t) => burnTokensState.selectedMints.has(t.mint));
      selectAll.checked = allSelected;
    }
  });
}

async function handleBurnSelectedTokens() {
  const burnBtn = $("#burn-selected-btn");
  if (!burnBtn || burnTokensState.selectedMints.size === 0 || burnTokensState.isBurning) return;

  const selectedCount = burnTokensState.selectedMints.size;
  const selectedMints = Array.from(burnTokensState.selectedMints);

  // Calculate total value at risk
  let totalValue = 0;
  selectedMints.forEach((mint) => {
    const token = burnTokensState.tokens.find((t) => t.mint === mint);
    if (token && token.value_sol) {
      totalValue += token.value_sol;
    }
  });

  // First confirmation
  const firstConfirm = await showBurnConfirmation(
    "Confirm Burn",
    `Are you sure you want to burn <strong>${selectedCount}</strong> token${selectedCount !== 1 ? "s" : ""}?` +
      (totalValue > 0.0001
        ? `<br><br>⚠️ Total estimated value: <strong>${Utils.formatSol(totalValue)}</strong>`
        : ""),
    "Continue",
    "Cancel"
  );

  if (!firstConfirm) return;

  // Second confirmation (critical warning)
  const secondConfirm = await showBurnConfirmation(
    "⚠️ Final Warning",
    `<div class="burn-final-warning">
      <p><strong>This action is IRREVERSIBLE!</strong></p>
      <p>The following ${selectedCount} token${selectedCount !== 1 ? "s" : ""} will be permanently destroyed and cannot be recovered under any circumstances.</p>
    </div>`,
    "Yes, Burn Tokens",
    "Cancel",
    true
  );

  if (!secondConfirm) return;

  // Execute burn
  burnTokensState.isBurning = true;
  burnBtn.disabled = true;
  burnBtn.innerHTML = '<i class="icon-loader spin"></i> Burning...';

  try {
    const response = await fetch("/api/tools/burn-tokens/burn", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ mints: selectedMints }),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    const result = await response.json();
    const data = result.data || result;

    if (data.successful > 0) {
      Utils.showToast(
        `Burned ${data.successful}/${data.total} tokens. Run Wallet Cleanup to reclaim ~${Utils.formatSol(data.sol_reclaimed)} SOL`,
        "success"
      );
    }

    if (data.failed > 0) {
      Utils.showToast(
        `${data.failed} token${data.failed !== 1 ? "s" : ""} failed to burn`,
        "warning"
      );
    }

    // Refresh the list
    await handleScanBurnTokens();
  } catch (error) {
    console.error("Burn tokens failed:", error);
    Utils.showToast(`Burn failed: ${error.message}`, "error");
  } finally {
    burnTokensState.isBurning = false;
    updateBurnSelectionUI();
  }
}

function showBurnConfirmation(title, message, confirmText, cancelText, isDanger = false) {
  return new Promise((resolve) => {
    // Create overlay
    const overlay = document.createElement("div");
    overlay.className = "burn-confirm-overlay";
    overlay.innerHTML = `
      <div class="burn-confirm-dialog ${isDanger ? "danger" : ""}">
        <div class="burn-confirm-header">
          <h3>${title}</h3>
        </div>
        <div class="burn-confirm-body">
          ${message}
        </div>
        <div class="burn-confirm-actions">
          <button class="btn" id="burn-confirm-cancel">${cancelText}</button>
          <button class="btn ${isDanger ? "danger" : "primary"}" id="burn-confirm-ok">${confirmText}</button>
        </div>
      </div>
    `;

    document.body.appendChild(overlay);

    // Focus trap and event handlers
    const cancelBtn = overlay.querySelector("#burn-confirm-cancel");
    const confirmBtn = overlay.querySelector("#burn-confirm-ok");

    const cleanup = () => {
      overlay.remove();
    };

    cancelBtn.addEventListener("click", () => {
      cleanup();
      resolve(false);
    });

    confirmBtn.addEventListener("click", () => {
      cleanup();
      resolve(true);
    });

    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) {
        cleanup();
        resolve(false);
      }
    });

    // Focus confirm button
    confirmBtn.focus();
  });
}

function renderCreateTokenTool(container, actionsContainer) {
  container.innerHTML = `
    <div class="tool-panel create-token-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-file-plus"></i> Token Details</h3>
        </div>
        <div class="section-content">
          <form class="tool-form" id="create-token-form">
            <div class="form-group">
              <label for="token-name">Token Name</label>
              <input type="text" id="token-name" placeholder="My Token" maxlength="32" />
            </div>
            <div class="form-group">
              <label for="token-symbol">Symbol</label>
              <input type="text" id="token-symbol" placeholder="MTK" maxlength="10" />
            </div>
            <div class="form-group">
              <label for="token-decimals">Decimals</label>
              <input type="number" id="token-decimals" value="9" min="0" max="9" />
            </div>
            <div class="form-group">
              <label for="token-supply">Initial Supply</label>
              <input type="number" id="token-supply" placeholder="1000000000" min="1" />
            </div>
            <div class="form-group">
              <label for="token-description">Description</label>
              <textarea id="token-description" placeholder="Token description..." rows="3"></textarea>
            </div>
          </form>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-image"></i> Token Image</h3>
        </div>
        <div class="section-content">
          <div class="image-upload-area" id="token-image-upload">
            <i class="icon-upload"></i>
            <p>Drop image here or click to upload</p>
            <small>Recommended: 512x512 PNG</small>
          </div>
        </div>
      </div>
    </div>
  `;

  actionsContainer.innerHTML = `
    <button class="btn" id="preview-token-btn">
      <i class="icon-eye"></i> Preview
    </button>
    <button class="btn primary" id="create-token-btn">
      <i class="icon-circle-plus"></i> Create Token
    </button>
  `;

  // TODO: Wire up token creation functionality
}

function renderTokenWatchTool(container, actionsContainer) {
  // Load holder watch config and render UI
  container.innerHTML = `
    <div class="tool-panel holder-watch-tool">
      <div class="hw-loading">
        <i class="icon-loader spin"></i>
        <p>Loading settings...</p>
      </div>
    </div>
  `;

  loadHolderWatchConfig().then((config) => {
    renderHolderWatchContent(container, actionsContainer, config);
  });
}

/**
 * Load holder watch configuration from the server
 */
async function loadHolderWatchConfig() {
  try {
    const res = await fetch("/api/config");
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    const data = await res.json();
    return (
      data.data?.holder_watch || {
        enabled: false,
        check_interval_secs: 60,
        notify_new_holders: true,
        notify_holder_drop: true,
        min_holder_change: 5,
        holder_drop_percent: 10.0,
        max_watched_tokens: 20,
      }
    );
  } catch (e) {
    console.error("[HolderWatch] Failed to load config:", e);
    return {
      enabled: false,
      check_interval_secs: 60,
      notify_new_holders: true,
      notify_holder_drop: true,
      min_holder_change: 5,
      holder_drop_percent: 10.0,
      max_watched_tokens: 20,
    };
  }
}

/**
 * Save holder watch configuration to the server
 */
async function saveHolderWatchConfig() {
  const config = {
    enabled: $("#hw-enabled")?.checked ?? false,
    check_interval_secs: parseInt($("#hw-interval")?.value, 10) || 60,
    notify_new_holders: $("#hw-notify-new")?.checked ?? true,
    notify_holder_drop: $("#hw-notify-drop")?.checked ?? true,
    min_holder_change: parseInt($("#hw-min-change")?.value, 10) || 5,
    holder_drop_percent: parseFloat($("#hw-drop-percent")?.value) || 10.0,
    max_watched_tokens: parseInt($("#hw-max-tokens")?.value, 10) || 20,
  };

  try {
    const res = await fetch("/api/config/holder_watch", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config),
    });

    if (res.ok) {
      Utils.showToast("Holder Watch settings saved", "success");
    } else {
      const errData = await res.json().catch(() => ({}));
      Utils.showToast(errData.error || "Failed to save settings", "error");
    }
  } catch (e) {
    console.error("[HolderWatch] Save error:", e);
    Utils.showToast("Error saving settings", "error");
  }
}

/**
 * Render the holder watch content after config is loaded
 */
function renderHolderWatchContent(container, actionsContainer, config) {
  container.innerHTML = `
    <div class="tool-panel holder-watch-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-settings"></i> Holder Watch Settings</h3>
        </div>
        <div class="section-content">
          <div class="hw-form-row">
            <div class="hw-form-group hw-toggle-group">
              <label for="hw-enabled">Enable Holder Watching</label>
              <label class="toggle-switch">
                <input type="checkbox" id="hw-enabled" ${config.enabled ? "checked" : ""}>
                <span class="slider"></span>
              </label>
            </div>
          </div>

          <div class="hw-form-row hw-two-cols">
            <div class="hw-form-group">
              <label for="hw-interval">Check Interval (seconds)</label>
              <input type="number" id="hw-interval" class="form-input" 
                value="${config.check_interval_secs || 60}" min="10" max="3600" step="10">
              <span class="hint">How often to check holder counts (10-3600s)</span>
            </div>
            <div class="hw-form-group">
              <label for="hw-max-tokens">Max Watched Tokens</label>
              <input type="number" id="hw-max-tokens" class="form-input" 
                value="${config.max_watched_tokens || 20}" min="1" max="100">
              <span class="hint">Maximum tokens to watch simultaneously</span>
            </div>
          </div>

          <div class="hw-form-row hw-two-cols">
            <div class="hw-form-group hw-toggle-group">
              <label for="hw-notify-new">Notify on New Holders</label>
              <label class="toggle-switch">
                <input type="checkbox" id="hw-notify-new" ${config.notify_new_holders ? "checked" : ""}>
                <span class="slider"></span>
              </label>
            </div>
            <div class="hw-form-group hw-toggle-group">
              <label for="hw-notify-drop">Notify on Holder Drop</label>
              <label class="toggle-switch">
                <input type="checkbox" id="hw-notify-drop" ${config.notify_holder_drop ? "checked" : ""}>
                <span class="slider"></span>
              </label>
            </div>
          </div>

          <div class="hw-form-row hw-two-cols">
            <div class="hw-form-group">
              <label for="hw-min-change">Min Holder Change</label>
              <input type="number" id="hw-min-change" class="form-input" 
                value="${config.min_holder_change || 5}" min="1" max="1000">
              <span class="hint">Minimum holder change to trigger notification</span>
            </div>
            <div class="hw-form-group">
              <label for="hw-drop-percent">Holder Drop Threshold (%)</label>
              <input type="number" id="hw-drop-percent" class="form-input" 
                value="${config.holder_drop_percent || 10.0}" min="1" max="100" step="0.5">
              <span class="hint">Percentage drop to trigger alert</span>
            </div>
          </div>

          <div class="hw-form-actions">
            <button class="btn primary" id="hw-save-config">
              <i class="icon-save"></i> Save Settings
            </button>
          </div>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-eye"></i> Watched Tokens</h3>
        </div>
        <div class="section-content">
          <div class="hw-add-token-group">
            <input type="text" id="hw-token-input" class="form-input" 
              placeholder="Enter token mint address...">
            <button class="btn primary" id="hw-add-token">
              <i class="icon-plus"></i> Add
            </button>
          </div>
          <div id="hw-token-list" class="hw-token-list">
            <div class="empty-state">
              <i class="icon-eye-off"></i>
              <p>No tokens being watched</p>
              <small>Add a token mint address above to start watching</small>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;

  // Wire up save config button
  const saveBtn = $("#hw-save-config");
  if (saveBtn) {
    saveBtn.addEventListener("click", saveHolderWatchConfig);
  }

  // Wire up add token button (placeholder - database integration needed)
  const addBtn = $("#hw-add-token");
  const tokenInput = $("#hw-token-input");
  if (addBtn && tokenInput) {
    addBtn.addEventListener("click", () => {
      const mint = tokenInput.value.trim();
      if (mint && mint.length >= 32) {
        Utils.showToast("Token watching feature coming soon", "info");
        tokenInput.value = "";
      } else {
        Utils.showToast("Please enter a valid mint address", "error");
      }
    });

    tokenInput.addEventListener("keypress", (e) => {
      if (e.key === "Enter") {
        addBtn.click();
      }
    });
  }

  // Render action bar
  actionsContainer.innerHTML = `
    <button class="btn" id="hw-refresh-action">
      <i class="icon-refresh-cw"></i> Refresh
    </button>
  `;

  const refreshBtn = $("#hw-refresh-action");
  if (refreshBtn) {
    refreshBtn.addEventListener("click", () => {
      renderTokenWatchTool(container, actionsContainer);
    });
  }
}

// =============================================================================
// Token Analyzer Tool
// =============================================================================

// Token analyzer state
let taCurrentMint = null;
let taCurrentTab = "overview";
let taAnalysisData = null;

function renderTokenAnalyzerTool(container, actionsContainer) {
  container.innerHTML = `
    <div class="tool-panel token-analyzer-tool">
      <!-- Token Input Section -->
      <div class="tool-section ta-input-section">
        <div class="section-header">
          <h3><i class="icon-search"></i> Analyze Token</h3>
        </div>
        <div class="section-content">
          <div class="ta-input-group">
            <input type="text" id="ta-mint-input" placeholder="Paste token mint address..." />
            <button class="btn primary" id="ta-analyze-btn">
              <i class="icon-search"></i> Analyze
            </button>
          </div>
        </div>
      </div>

      <!-- Loading State -->
      <div id="ta-loading" class="ta-loading" style="display: none;">
        <i class="icon-loader spin"></i>
        <p>Analyzing token...</p>
      </div>

      <!-- Error State -->
      <div id="ta-error" class="ta-error" style="display: none;"></div>

      <!-- Results Section (hidden until analyzed) -->
      <div id="ta-results" class="ta-results" style="display: none;">
        <!-- Token Header -->
        <div class="ta-token-header" id="ta-token-header"></div>

        <!-- Subtabs -->
        <div class="ta-tabs">
          <button class="ta-tab active" data-tab="overview">
            <i class="icon-info"></i> Overview
          </button>
          <button class="ta-tab" data-tab="security">
            <i class="icon-shield"></i> Security
          </button>
          <button class="ta-tab" data-tab="market">
            <i class="icon-trending-up"></i> Market
          </button>
          <button class="ta-tab" data-tab="liquidity">
            <i class="icon-droplet"></i> Liquidity
          </button>
        </div>

        <!-- Tab Content -->
        <div class="ta-content" id="ta-content"></div>
      </div>

      <!-- Empty State -->
      <div id="ta-empty" class="ta-empty-state">
        <i class="icon-search"></i>
        <p>Enter a token mint address to analyze</p>
        <small>Get comprehensive insights on any Solana token</small>
      </div>
    </div>
  `;

  actionsContainer.innerHTML = `
    <button class="btn" id="ta-refresh-btn" disabled>
      <i class="icon-refresh-cw"></i> Refresh
    </button>
    <button class="btn" id="ta-copy-btn" disabled>
      <i class="icon-copy"></i> Copy Report
    </button>
  `;

  // Wire up event handlers
  initTokenAnalyzer();
}

/**
 * Initialize Token Analyzer event handlers
 */
function initTokenAnalyzer() {
  const analyzeBtn = $("#ta-analyze-btn");
  const mintInput = $("#ta-mint-input");
  const refreshBtn = $("#ta-refresh-btn");
  const copyBtn = $("#ta-copy-btn");

  if (analyzeBtn) {
    on(analyzeBtn, "click", handleTokenAnalyze);
  }

  if (mintInput) {
    on(mintInput, "keypress", (e) => {
      if (e.key === "Enter") {
        handleTokenAnalyze();
      }
    });
  }

  if (refreshBtn) {
    on(refreshBtn, "click", () => {
      if (taCurrentMint) {
        analyzeToken(taCurrentMint);
      }
    });
  }

  if (copyBtn) {
    on(copyBtn, "click", copyAnalysisReport);
  }

  // Wire up tab switching
  const tabs = $$(".ta-tabs .ta-tab");
  tabs.forEach((tab) => {
    on(tab, "click", () => {
      const tabId = tab.dataset.tab;
      switchTaTab(tabId);
    });
  });
}

/**
 * Handle analyze button click
 */
function handleTokenAnalyze() {
  const mintInput = $("#ta-mint-input");
  const mint = mintInput?.value?.trim();

  if (!mint) {
    Utils.showToast("Please enter a token mint address", "warning");
    return;
  }

  // Validate mint format (base58)
  if (!/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(mint)) {
    Utils.showToast("Invalid token mint address format", "error");
    return;
  }

  analyzeToken(mint);
}

/**
 * Fetch and display token analysis
 */
async function analyzeToken(mint) {
  const loadingEl = $("#ta-loading");
  const errorEl = $("#ta-error");
  const resultsEl = $("#ta-results");
  const emptyEl = $("#ta-empty");
  const refreshBtn = $("#ta-refresh-btn");
  const copyBtn = $("#ta-copy-btn");
  const analyzeBtn = $("#ta-analyze-btn");

  // Show loading state
  if (emptyEl) emptyEl.style.display = "none";
  if (errorEl) errorEl.style.display = "none";
  if (resultsEl) resultsEl.style.display = "none";
  if (loadingEl) loadingEl.style.display = "flex";
  if (analyzeBtn) {
    analyzeBtn.disabled = true;
    analyzeBtn.innerHTML = '<i class="icon-loader spin"></i> Analyzing...';
  }

  try {
    const response = await fetch(`/api/tokens/${mint}/analysis`);
    const data = await response.json();

    if (!response.ok || !data.success) {
      throw new Error(data.error || "Failed to analyze token");
    }

    // Store data
    taCurrentMint = mint;
    taAnalysisData = data;
    taCurrentTab = "overview";

    // Enable action buttons
    if (refreshBtn) refreshBtn.disabled = false;
    if (copyBtn) copyBtn.disabled = false;

    // Render results
    renderTaTokenHeader(data.overview);
    renderTaTabContent("overview");

    // Show results
    if (loadingEl) loadingEl.style.display = "none";
    if (resultsEl) resultsEl.style.display = "block";

    // Update tab states
    const tabs = $$(".ta-tabs .ta-tab");
    tabs.forEach((tab) => {
      tab.classList.toggle("active", tab.dataset.tab === "overview");
    });
  } catch (error) {
    console.error("Token analysis failed:", error);
    if (loadingEl) loadingEl.style.display = "none";
    if (errorEl) {
      errorEl.style.display = "block";
      errorEl.innerHTML = `
        <i class="icon-circle-alert"></i>
        <p>${escapeHtml(error.message)}</p>
        <button class="btn btn-sm" onclick="document.getElementById('ta-error').style.display='none'; document.getElementById('ta-empty').style.display='flex';">
          Dismiss
        </button>
      `;
    }
    if (refreshBtn) refreshBtn.disabled = true;
    if (copyBtn) copyBtn.disabled = true;
  } finally {
    if (analyzeBtn) {
      analyzeBtn.disabled = false;
      analyzeBtn.innerHTML = '<i class="icon-search"></i> Analyze';
    }
  }
}

/**
 * Render token header with logo, name, price
 */
function renderTaTokenHeader(overview) {
  const headerEl = $("#ta-token-header");
  if (!headerEl || !overview) return;

  const symbol = overview.symbol || "Unknown";
  const name = overview.name || "Unknown Token";
  const logoUrl = overview.logo_url || "";
  const priceSol = overview.price_sol;
  const priceUsd = overview.price_usd;
  const mint = overview.mint || taCurrentMint;

  headerEl.innerHTML = `
    <div class="ta-header-left">
      <div class="ta-logo">
        ${logoUrl ? `<img src="${escapeHtml(logoUrl)}" alt="${escapeHtml(symbol)}" onerror="this.parentElement.innerHTML='<div class=\\'ta-logo-placeholder\\'>${escapeHtml(symbol.charAt(0))}</div>'" />` : `<div class="ta-logo-placeholder">${escapeHtml(symbol.charAt(0))}</div>`}
      </div>
      <div class="ta-header-info">
        <span class="ta-symbol">${escapeHtml(symbol)}</span>
        <span class="ta-name">${escapeHtml(name)}</span>
      </div>
    </div>
    <div class="ta-header-center">
      <div class="ta-header-actions">
        <button class="btn btn-sm btn-icon action-favorite" data-mint="${escapeHtml(mint)}" data-symbol="${escapeHtml(symbol)}" data-name="${escapeHtml(name)}" data-logo="${escapeHtml(logoUrl)}" title="Add to Favorites">
          <i class="icon-star"></i>
        </button>
        <button class="btn btn-sm btn-icon action-blacklist" data-mint="${escapeHtml(mint)}" data-symbol="${escapeHtml(symbol)}" title="Add to Blacklist">
          <i class="icon-slash"></i>
        </button>
        <button class="btn btn-sm btn-icon" onclick="navigator.clipboard.writeText('${escapeHtml(mint)}'); Utils.showToast('Mint copied', 'success');" title="Copy Mint Address">
          <i class="icon-copy"></i>
        </button>
        <button class="btn btn-sm btn-icon" onclick="window.open('https://dexscreener.com/solana/${escapeHtml(mint)}', '_blank');" title="View on DexScreener">
          <i class="icon-external-link"></i>
        </button>
      </div>
    </div>
    <div class="ta-header-right">
      ${priceSol ? `<div class="ta-price-sol">${Utils.formatSol(priceSol)} SOL</div>` : ""}
      ${priceUsd ? `<div class="ta-price-usd">${Utils.formatCurrencyUSD(priceUsd)}</div>` : ""}
    </div>
  `;

  // Attach event handlers for favorite and blacklist buttons
  const favoriteBtn = headerEl.querySelector(".action-favorite");
  const blacklistBtn = headerEl.querySelector(".action-blacklist");

  if (favoriteBtn) {
    on(favoriteBtn, "click", handleTaFavoriteClick);
  }
  if (blacklistBtn) {
    on(blacklistBtn, "click", handleTaBlacklistClick);
  }
}

/**
 * Handle favorite button click in token analyzer
 */
async function handleTaFavoriteClick(e) {
  const btn = e.currentTarget;
  const mint = btn.dataset.mint;
  const symbol = btn.dataset.symbol;
  const name = btn.dataset.name;
  const logoUrl = btn.dataset.logo;

  btn.disabled = true;
  btn.classList.add("loading");

  try {
    const response = await fetch("/api/tokens/favorites", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        mint,
        name,
        symbol,
        logo_url: logoUrl || null,
      }),
    });

    const data = await response.json();

    if (response.ok && data.success) {
      Utils.showToast(`Added ${symbol || mint} to favorites`, "success");
      btn.classList.add("active");
      btn.title = "Already in Favorites";
    } else {
      throw new Error(data.error || "Failed to add to favorites");
    }
  } catch (error) {
    Utils.showToast(`Error: ${error.message}`, "error");
  } finally {
    btn.disabled = false;
    btn.classList.remove("loading");
  }
}

/**
 * Handle blacklist button click in token analyzer
 */
async function handleTaBlacklistClick(e) {
  const btn = e.currentTarget;
  const mint = btn.dataset.mint;
  const symbol = btn.dataset.symbol;

  if (!window.confirm(`Blacklist ${symbol || mint}? This token will be excluded from trading.`)) {
    return;
  }

  btn.disabled = true;
  btn.classList.add("loading");

  try {
    const response = await fetch(`/api/tokens/${mint}/blacklist`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        mint,
        reason: "Manual blacklist via Token Analyzer",
      }),
    });

    const data = await response.json();

    if (response.ok && data.success) {
      Utils.showToast(`Blacklisted ${symbol || mint}`, "success");
      btn.classList.add("active");
      btn.title = "Blacklisted";
    } else {
      throw new Error(data.error || "Failed to blacklist token");
    }
  } catch (error) {
    Utils.showToast(`Error: ${error.message}`, "error");
  } finally {
    btn.disabled = false;
    btn.classList.remove("loading");
  }
}

/**
 * Switch between analysis tabs
 */
function switchTaTab(tabId) {
  taCurrentTab = tabId;

  // Update tab buttons
  const tabs = $$(".ta-tabs .ta-tab");
  tabs.forEach((tab) => {
    tab.classList.toggle("active", tab.dataset.tab === tabId);
  });

  // Render tab content
  renderTaTabContent(tabId);
}

/**
 * Render tab content based on current tab
 */
function renderTaTabContent(tabId) {
  if (!taAnalysisData) return;

  switch (tabId) {
    case "overview":
      renderTaOverviewTab();
      break;
    case "security":
      renderTaSecurityTab();
      break;
    case "market":
      renderTaMarketTab();
      break;
    case "liquidity":
      renderTaLiquidityTab();
      break;
  }
}

/**
 * Render Overview tab
 */
function renderTaOverviewTab() {
  const contentEl = $("#ta-content");
  if (!contentEl || !taAnalysisData) return;

  const { overview, security, market, liquidity } = taAnalysisData;

  contentEl.innerHTML = `
    <div class="ta-overview-grid">
      <!-- Quick Stats Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-activity"></i> Quick Stats
        </div>
        <div class="ta-stat-grid">
          <div class="ta-stat-item">
            <span class="ta-stat-label">Holders</span>
            <span class="ta-stat-value">${overview.total_holders ? Utils.formatCompactNumber(overview.total_holders) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label">Decimals</span>
            <span class="ta-stat-value">${overview.decimals}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label">Safety Score</span>
            <span class="ta-stat-value ${security?.normalized_score ? getTaScoreClass(security.normalized_score) : ""}">${security?.normalized_score ?? "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label">Pools</span>
            <span class="ta-stat-value">${liquidity?.pool_count ?? "—"}</span>
          </div>
        </div>
      </div>

      <!-- Market Summary Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-trending-up"></i> Market Summary
        </div>
        <div class="ta-stat-grid">
          <div class="ta-stat-item">
            <span class="ta-stat-label">24h Volume</span>
            <span class="ta-stat-value">${market?.volume_h24 ? Utils.formatCurrencyUSD(market.volume_h24) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label">24h Change</span>
            <span class="ta-stat-value ${market?.price_change_h24 ? getTaPriceChangeClass(market.price_change_h24) : ""}">${market?.price_change_h24 ? Utils.formatPercent(market.price_change_h24) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label">Market Cap</span>
            <span class="ta-stat-value">${market?.market_cap ? Utils.formatCurrencyUSD(market.market_cap) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label">Liquidity</span>
            <span class="ta-stat-value">${liquidity?.total_liquidity_sol ? Utils.formatSol(liquidity.total_liquidity_sol) : "—"}</span>
          </div>
        </div>
      </div>

      <!-- Token Info Card -->
      <div class="ta-card ta-full-width">
        <div class="ta-card-title">
          <i class="icon-info"></i> Token Information
        </div>
        <div class="ta-info-grid">
          <div class="ta-info-item">
            <span class="ta-info-label">Mint Address</span>
            <span class="ta-info-value mono">${escapeHtml(overview.mint)}</span>
          </div>
          ${
            overview.description
              ? `
          <div class="ta-info-item ta-full-width">
            <span class="ta-info-label">Description</span>
            <span class="ta-info-value">${escapeHtml(overview.description)}</span>
          </div>
          `
              : ""
          }
          ${
            overview.supply
              ? `
          <div class="ta-info-item">
            <span class="ta-info-label">Supply</span>
            <span class="ta-info-value mono">${escapeHtml(overview.supply)}</span>
          </div>
          `
              : ""
          }
        </div>
        <div class="ta-links">
          ${overview.website ? `<a href="${escapeHtml(overview.website)}" target="_blank" rel="noopener" class="ta-link"><i class="icon-globe"></i> Website</a>` : ""}
          ${overview.twitter ? `<a href="${escapeHtml(overview.twitter)}" target="_blank" rel="noopener" class="ta-link"><i class="icon-twitter"></i> Twitter</a>` : ""}
          ${overview.telegram ? `<a href="${escapeHtml(overview.telegram)}" target="_blank" rel="noopener" class="ta-link"><i class="icon-message-circle"></i> Telegram</a>` : ""}
        </div>
      </div>
    </div>
  `;
}

/**
 * Render Security tab
 */
function renderTaSecurityTab() {
  const contentEl = $("#ta-content");
  if (!contentEl || !taAnalysisData) return;

  const { security } = taAnalysisData;

  if (!security) {
    contentEl.innerHTML = `
      <div class="ta-empty-tab">
        <i class="icon-shield-off"></i>
        <p>No security data available</p>
        <small>Security analysis is not available for this token</small>
      </div>
    `;
    return;
  }

  const scoreClass = security.normalized_score ? getTaScoreClass(security.normalized_score) : "";

  contentEl.innerHTML = `
    <div class="ta-security-grid">
      <!-- Security Score Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-shield"></i> Safety Score
        </div>
        <div class="ta-security-score ${scoreClass}">
          <span class="ta-score-value">${security.normalized_score ?? "—"}</span>
          <span class="ta-score-label">${getTaScoreLabel(security.normalized_score)}</span>
        </div>
        ${security.score ? `<div class="ta-raw-score">Raw Risk Score: ${security.score}</div>` : ""}
      </div>

      <!-- Authorities Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-key"></i> Token Authorities
        </div>
        <div class="ta-authority-list">
          <div class="ta-authority-item ${security.mint_authority ? "warning" : "success"}">
            <span class="ta-authority-label">Mint Authority</span>
            <span class="ta-authority-value">${security.mint_authority ? "Active" : "Revoked"}</span>
            ${security.mint_authority ? `<span class="ta-authority-address mono">${escapeHtml(security.mint_authority)}</span>` : ""}
          </div>
          <div class="ta-authority-item ${security.freeze_authority ? "warning" : "success"}">
            <span class="ta-authority-label">Freeze Authority</span>
            <span class="ta-authority-value">${security.freeze_authority ? "Active" : "Revoked"}</span>
            ${security.freeze_authority ? `<span class="ta-authority-address mono">${escapeHtml(security.freeze_authority)}</span>` : ""}
          </div>
          <div class="ta-authority-item ${security.has_transfer_fee ? "warning" : "success"}">
            <span class="ta-authority-label">Transfer Fee</span>
            <span class="ta-authority-value">${security.has_transfer_fee ? "Yes" : "No"}</span>
          </div>
          <div class="ta-authority-item ${security.is_mutable ? "warning" : "success"}">
            <span class="ta-authority-label">Mutable</span>
            <span class="ta-authority-value">${security.is_mutable ? "Yes" : "No"}</span>
          </div>
        </div>
      </div>

      <!-- Top Holders Card -->
      ${
        security.top_holders_pct
          ? `
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-users"></i> Holder Concentration
        </div>
        <div class="ta-holder-concentration">
          <div class="ta-holder-bar">
            <div class="ta-holder-fill" style="width: ${Math.min(security.top_holders_pct, 100)}%"></div>
          </div>
          <span class="ta-holder-pct">${security.top_holders_pct.toFixed(2)}%</span>
          <span class="ta-holder-label">held by top 10 holders</span>
        </div>
      </div>
      `
          : ""
      }

      <!-- Risks Card -->
      ${
        security.risks && security.risks.length > 0
          ? `
      <div class="ta-card ta-full-width">
        <div class="ta-card-title">
          <i class="icon-triangle-alert"></i> Security Risks (${security.risks.length})
        </div>
        <div class="ta-risk-list">
          ${security.risks
            .map(
              (risk) => `
            <div class="ta-risk-item ${risk.level.toLowerCase()}">
              <span class="ta-risk-level">${escapeHtml(risk.level)}</span>
              <span class="ta-risk-name">${escapeHtml(risk.name)}</span>
              <span class="ta-risk-desc">${escapeHtml(risk.description)}</span>
            </div>
          `
            )
            .join("")}
        </div>
      </div>
      `
          : `
      <div class="ta-card ta-full-width">
        <div class="ta-card-title">
          <i class="icon-circle-check"></i> Security Risks
        </div>
        <div class="ta-no-risks">
          <i class="icon-shield-check"></i>
          <p>No security risks detected</p>
        </div>
      </div>
      `
      }
    </div>
  `;
}

/**
 * Render Market tab
 */
function renderTaMarketTab() {
  const contentEl = $("#ta-content");
  if (!contentEl || !taAnalysisData) return;

  const { market } = taAnalysisData;

  if (!market) {
    contentEl.innerHTML = `
      <div class="ta-empty-tab">
        <i class="icon-trending-up"></i>
        <p>No market data available</p>
        <small>Market data is not available for this token</small>
      </div>
    `;
    return;
  }

  contentEl.innerHTML = `
    <div class="ta-market-grid">
      <!-- Price Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-dollar-sign"></i> Current Price
        </div>
        <div class="ta-price-display">
          <div class="ta-price-main">${market.price_sol ? Utils.formatSol(market.price_sol) : "—"} SOL</div>
          ${market.price_usd ? `<div class="ta-price-sub">${Utils.formatCurrencyUSD(market.price_usd)}</div>` : ""}
        </div>
      </div>

      <!-- Price Changes Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-percent"></i> Price Changes
        </div>
        <div class="ta-stat-grid">
          <div class="ta-stat-item">
            <span class="ta-stat-label">1h</span>
            <span class="ta-stat-value ${market.price_change_h1 ? getTaPriceChangeClass(market.price_change_h1) : ""}">${market.price_change_h1 ? Utils.formatPercent(market.price_change_h1) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label">6h</span>
            <span class="ta-stat-value ${market.price_change_h6 ? getTaPriceChangeClass(market.price_change_h6) : ""}">${market.price_change_h6 ? Utils.formatPercent(market.price_change_h6) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label">24h</span>
            <span class="ta-stat-value ${market.price_change_h24 ? getTaPriceChangeClass(market.price_change_h24) : ""}">${market.price_change_h24 ? Utils.formatPercent(market.price_change_h24) : "—"}</span>
          </div>
        </div>
      </div>

      <!-- Volume Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-chart-bar"></i> Trading Volume
        </div>
        <div class="ta-stat-grid">
          <div class="ta-stat-item">
            <span class="ta-stat-label">1h Volume</span>
            <span class="ta-stat-value">${market.volume_h1 ? Utils.formatCurrencyUSD(market.volume_h1) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label">6h Volume</span>
            <span class="ta-stat-value">${market.volume_h6 ? Utils.formatCurrencyUSD(market.volume_h6) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label">24h Volume</span>
            <span class="ta-stat-value">${market.volume_h24 ? Utils.formatCurrencyUSD(market.volume_h24) : "—"}</span>
          </div>
        </div>
      </div>

      <!-- Transactions Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-repeat"></i> 24h Transactions
        </div>
        <div class="ta-txns-display">
          <div class="ta-txn-item buys">
            <span class="ta-txn-label">Buys</span>
            <span class="ta-txn-value">${market.txns_buys_h24 ? Utils.formatCompactNumber(market.txns_buys_h24) : "—"}</span>
          </div>
          <div class="ta-txn-item sells">
            <span class="ta-txn-label">Sells</span>
            <span class="ta-txn-value">${market.txns_sells_h24 ? Utils.formatCompactNumber(market.txns_sells_h24) : "—"}</span>
          </div>
        </div>
      </div>

      <!-- Valuation Card -->
      <div class="ta-card ta-full-width">
        <div class="ta-card-title">
          <i class="icon-chart-pie"></i> Valuation
        </div>
        <div class="ta-stat-grid">
          <div class="ta-stat-item">
            <span class="ta-stat-label">Market Cap</span>
            <span class="ta-stat-value">${market.market_cap ? Utils.formatCurrencyUSD(market.market_cap) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label">Fully Diluted Value</span>
            <span class="ta-stat-value">${market.fdv ? Utils.formatCurrencyUSD(market.fdv) : "—"}</span>
          </div>
        </div>
      </div>
    </div>
  `;
}

/**
 * Render Liquidity tab
 */
function renderTaLiquidityTab() {
  const contentEl = $("#ta-content");
  if (!contentEl || !taAnalysisData) return;

  const { liquidity } = taAnalysisData;

  if (!liquidity) {
    contentEl.innerHTML = `
      <div class="ta-empty-tab">
        <i class="icon-droplet"></i>
        <p>No liquidity data available</p>
        <small>No pools found for this token</small>
      </div>
    `;
    return;
  }

  contentEl.innerHTML = `
    <div class="ta-liquidity-grid">
      <!-- Total Liquidity Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-droplet"></i> Total Liquidity
        </div>
        <div class="ta-liquidity-total">
          <div class="ta-liquidity-sol">${Utils.formatSol(liquidity.total_liquidity_sol)} SOL</div>
          ${liquidity.total_liquidity_usd ? `<div class="ta-liquidity-usd">${Utils.formatCurrencyUSD(liquidity.total_liquidity_usd)}</div>` : ""}
        </div>
      </div>

      <!-- Pool Count Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-layers"></i> Pools
        </div>
        <div class="ta-pool-count">
          <span class="ta-pool-count-value">${liquidity.pool_count}</span>
          <span class="ta-pool-count-label">Active Pool${liquidity.pool_count !== 1 ? "s" : ""}</span>
        </div>
      </div>

      <!-- Pools Table Card -->
      <div class="ta-card ta-full-width">
        <div class="ta-card-title">
          <i class="icon-list"></i> Pool Details
        </div>
        <div class="ta-pools-table">
          <table>
            <thead>
              <tr>
                <th>DEX</th>
                <th>Pool Address</th>
                <th>Liquidity (SOL)</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              ${liquidity.pools
                .map(
                  (pool) => `
                <tr class="${pool.is_canonical ? "canonical" : ""}">
                  <td class="dex">${escapeHtml(pool.dex)}</td>
                  <td class="address mono">${escapeHtml(pool.address.slice(0, 8))}...${escapeHtml(pool.address.slice(-6))}</td>
                  <td class="liquidity">${Utils.formatSol(pool.liquidity_sol)}</td>
                  <td class="status">${pool.is_canonical ? '<span class="canonical-badge">Primary</span>' : ""}</td>
                </tr>
              `
                )
                .join("")}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  `;
}

/**
 * Copy analysis report to clipboard
 */
function copyAnalysisReport() {
  if (!taAnalysisData || !taCurrentMint) {
    Utils.showToast("No analysis to copy", "warning");
    return;
  }

  const { overview, security, market, liquidity } = taAnalysisData;

  let report = "Token Analysis Report\n";
  report += "====================\n\n";
  report += `Token: ${overview.symbol || "Unknown"} (${overview.name || "Unknown"})\n`;
  report += `Mint: ${overview.mint}\n`;
  report += "\n";

  if (overview.price_sol) {
    report += `Price: ${overview.price_sol} SOL`;
    if (overview.price_usd) report += ` ($${overview.price_usd.toFixed(6)})`;
    report += "\n";
  }

  if (security) {
    report += "\nSecurity:\n";
    report += `- Safety Score: ${security.normalized_score ?? "N/A"}/100\n`;
    report += `- Mint Authority: ${security.mint_authority ? "Active" : "Revoked"}\n`;
    report += `- Freeze Authority: ${security.freeze_authority ? "Active" : "Revoked"}\n`;
    if (security.risks && security.risks.length > 0) {
      report += `- Risks: ${security.risks.length}\n`;
    }
  }

  if (market) {
    report += "\nMarket:\n";
    if (market.volume_h24) report += `- 24h Volume: $${market.volume_h24.toFixed(2)}\n`;
    if (market.price_change_h24) report += `- 24h Change: ${market.price_change_h24.toFixed(2)}%\n`;
    if (market.market_cap) report += `- Market Cap: $${market.market_cap.toFixed(2)}\n`;
  }

  if (liquidity) {
    report += "\nLiquidity:\n";
    report += `- Total: ${liquidity.total_liquidity_sol.toFixed(4)} SOL\n`;
    report += `- Pools: ${liquidity.pool_count}\n`;
  }

  report += `\nGenerated: ${new Date(taAnalysisData.fetched_at).toLocaleString()}\n`;

  Utils.copyToClipboard(report);
  Utils.showToast("Analysis report copied to clipboard", "success");
}

/**
 * Helper: Get CSS class for security score
 * NOTE: normalized_score from Rugcheck is 0-100 where LOWER = SAFER, HIGHER = RISKIER
 */
function getTaScoreClass(score) {
  // Lower score = safer (green), higher score = riskier (red)
  if (score <= 30) return "success";
  if (score <= 60) return "warning";
  return "danger";
}

/**
 * Helper: Get label for security score
 * NOTE: normalized_score from Rugcheck is 0-100 where LOWER = SAFER, HIGHER = RISKIER
 */
function getTaScoreLabel(score) {
  if (!score && score !== 0) return "Unknown";
  // Lower score = safer
  if (score <= 30) return "Good";
  if (score <= 60) return "Moderate";
  return "Risky";
}

/**
 * Helper: Get CSS class for price change
 */
function getTaPriceChangeClass(change) {
  if (change > 0) return "success";
  if (change < 0) return "danger";
  return "";
}

/**
 * Helper: Escape HTML
 */
function escapeHtml(str) {
  if (!str) return "";
  return String(str)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

// =============================================================================
// Trade Watcher Tool
// =============================================================================

// Trade Watcher state
let twPoolSelector = null;
let twSelectedPool = null;
let twWatchesTable = null;
let twWatchPoller = null;

function renderTradeWatcherTool(container, actionsContainer) {
  const hint = Hints.getHint("tools.tradeWatcher");
  const hintHtml = hint ? HintTrigger.render(hint, "tools.tradeWatcher", { size: "sm" }) : "";

  container.innerHTML = `
    <div class="tool-panel trade-watcher-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-target"></i> Setup Watch</h3>
          <div class="section-header-actions">
            ${hintHtml}
          </div>
        </div>
        <div class="section-content">
          <form class="tool-form" id="tw-form">
            <div class="form-row">
              <div class="form-group flex-2">
                <label for="tw-mint">Token Mint Address</label>
                <div class="input-with-action">
                  <input type="text" id="tw-mint" placeholder="Enter token mint address..." />
                  <button type="button" class="btn btn-sm" id="tw-search-pools-btn">
                    <i class="icon-search"></i> Search Pools
                  </button>
                </div>
              </div>
            </div>

            <div class="form-row" id="tw-pool-row" style="display: none;">
              <div class="form-group">
                <label>Selected Pool</label>
                <div class="selected-pool-card" id="tw-selected-pool">
                  <span class="pool-info">No pool selected</span>
                  <button type="button" class="btn btn-sm btn-icon" id="tw-clear-pool-btn" title="Clear pool">
                    <i class="icon-x"></i>
                  </button>
                </div>
              </div>
            </div>

            <div class="form-row">
              <div class="form-group">
                <label for="tw-watch-type">Watch Type</label>
                <select id="tw-watch-type">
                  <option value="buy-on-sell">Buy on Sell</option>
                  <option value="sell-on-buy">Sell on Buy</option>
                  <option value="notify-only">Notify Only</option>
                </select>
                <small class="form-hint">Buy on Sell: Automatically buy when someone sells. Sell on Buy: Automatically sell when someone buys.</small>
              </div>
            </div>

            <div class="form-row" id="tw-trigger-row">
              <div class="form-group">
                <label for="tw-trigger-amount">Trigger Amount (SOL)</label>
                <input type="number" id="tw-trigger-amount" placeholder="0.1" min="0.001" step="0.001" value="0.1" />
                <small class="form-hint">Minimum trade size in SOL to trigger the action</small>
              </div>
              <div class="form-group">
                <label for="tw-action-amount">Action Amount (SOL)</label>
                <input type="number" id="tw-action-amount" placeholder="0.1" min="0.001" step="0.001" value="0.1" />
                <small class="form-hint">Amount to buy/sell when triggered</small>
              </div>
              <div class="form-group">
                <label for="tw-slippage">Slippage (%)</label>
                <input type="number" id="tw-slippage" placeholder="5" min="0.5" max="50" step="0.5" value="5" />
                <small class="form-hint">Maximum acceptable slippage for trades</small>
              </div>
            </div>
          </form>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-activity"></i> Active Watches</h3>
          <span class="section-badge" id="tw-watch-count">0</span>
        </div>
        <div class="section-content">
          <div class="tw-watches-table" id="tw-watches-table">
            <div class="empty-state">
              <i class="icon-eye-off"></i>
              <p>No active watches</p>
              <small>Configure a watch above and click "Start Watch" to begin monitoring</small>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;

  HintTrigger.initAll();
  enhanceAllSelects(container);

  actionsContainer.innerHTML = `
    <button class="btn primary" id="tw-start-btn" disabled>
      <i class="icon-play"></i> Start Watch
    </button>
    <button class="btn danger" id="tw-stop-all-btn" disabled>
      <i class="icon-square"></i> Stop All
    </button>
  `;

  // Wire up event handlers
  initTradeWatcher();
}

/**
 * Initialize Trade Watcher event handlers
 */
function initTradeWatcher() {
  const mintInput = $("#tw-mint");
  const searchPoolsBtn = $("#tw-search-pools-btn");
  const clearPoolBtn = $("#tw-clear-pool-btn");
  const watchTypeSelect = $("#tw-watch-type");
  const startBtn = $("#tw-start-btn");
  const stopAllBtn = $("#tw-stop-all-btn");

  // Search pools button
  if (searchPoolsBtn) {
    on(searchPoolsBtn, "click", handleTwSearchPools);
  }

  // Clear pool button
  if (clearPoolBtn) {
    on(clearPoolBtn, "click", () => {
      twSelectedPool = null;
      updateTwPoolDisplay();
      updateTwStartButtonState();
    });
  }

  // Watch type change - hide/show trigger inputs for notify-only
  if (watchTypeSelect) {
    on(watchTypeSelect, "change", () => {
      const triggerRow = $("#tw-trigger-row");
      if (triggerRow) {
        triggerRow.style.display = watchTypeSelect.value === "notify-only" ? "none" : "flex";
      }
    });
  }

  // Mint input validation
  if (mintInput) {
    on(mintInput, "input", () => {
      updateTwStartButtonState();
    });
  }

  // Start watch button
  if (startBtn) {
    on(startBtn, "click", handleTwStartWatch);
  }

  // Stop all button
  if (stopAllBtn) {
    on(stopAllBtn, "click", handleTwStopAllWatches);
  }

  // Load existing watches
  loadTwActiveWatches();
}

/**
 * Handle search pools button click
 */
function handleTwSearchPools() {
  const mintInput = $("#tw-mint");
  const mint = mintInput?.value?.trim();

  if (!mint) {
    Utils.showToast("Please enter a token mint address", "warning");
    return;
  }

  // Validate mint format
  if (!/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(mint)) {
    Utils.showToast("Invalid token mint address format", "error");
    return;
  }

  // Create pool selector if not exists
  if (!twPoolSelector) {
    twPoolSelector = new PoolSelector({
      onSelect: (pool, tokenMint) => {
        twSelectedPool = pool;
        updateTwPoolDisplay();
        updateTwStartButtonState();
        Utils.showToast(
          `Selected pool: ${pool.dex} ${pool.base_symbol}/${pool.quote_symbol}`,
          "success"
        );
      },
    });
  }

  twPoolSelector.open(mint);
}

/**
 * Update selected pool display
 */
function updateTwPoolDisplay() {
  const poolRow = $("#tw-pool-row");
  const poolCard = $("#tw-selected-pool");

  if (!poolRow || !poolCard) return;

  if (twSelectedPool) {
    poolRow.style.display = "flex";
    poolCard.innerHTML = `
      <div class="pool-info">
        <span class="pool-dex">${Utils.escapeHtml(twSelectedPool.dex || "Unknown")}</span>
        <span class="pool-pair">${Utils.escapeHtml(twSelectedPool.base_symbol || "?")}/${Utils.escapeHtml(twSelectedPool.quote_symbol || "?")}</span>
        <span class="pool-source ${(twSelectedPool.source || "").toLowerCase()}">${Utils.escapeHtml(twSelectedPool.source || "")}</span>
      </div>
      <button type="button" class="btn btn-sm btn-icon" id="tw-clear-pool-btn" title="Clear pool">
        <i class="icon-x"></i>
      </button>
    `;

    // Re-wire clear button
    const clearBtn = $("#tw-clear-pool-btn");
    if (clearBtn) {
      on(clearBtn, "click", () => {
        twSelectedPool = null;
        updateTwPoolDisplay();
        updateTwStartButtonState();
      });
    }
  } else {
    poolRow.style.display = "none";
    poolCard.innerHTML = '<span class="pool-info">No pool selected</span>';
  }
}

/**
 * Update start button state based on form validity
 */
function updateTwStartButtonState() {
  const startBtn = $("#tw-start-btn");
  const mintInput = $("#tw-mint");
  const watchType = $("#tw-watch-type");

  if (!startBtn) return;

  const mint = mintInput?.value?.trim();
  const isValidMint = mint && /^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(mint);
  const hasPool = twSelectedPool !== null;
  const isNotifyOnly = watchType?.value === "notify-only";

  // For notify-only, only need valid mint
  // For buy/sell actions, need pool selected
  startBtn.disabled = !isValidMint || (!isNotifyOnly && !hasPool);
}

/**
 * Handle start watch
 */
async function handleTwStartWatch() {
  const mintInput = $("#tw-mint");
  const watchTypeSelect = $("#tw-watch-type");
  const triggerAmountInput = $("#tw-trigger-amount");
  const actionAmountInput = $("#tw-action-amount");
  const slippageInput = $("#tw-slippage");
  const startBtn = $("#tw-start-btn");

  const mint = mintInput?.value?.trim();
  const watchType = watchTypeSelect?.value;
  const triggerAmount = parseFloat(triggerAmountInput?.value) || 0.1;
  const actionAmount = parseFloat(actionAmountInput?.value) || 0.1;
  const slippage = parseFloat(slippageInput?.value) || 5;

  if (!mint) {
    Utils.showToast("Please enter a token mint address", "warning");
    return;
  }

  startBtn.disabled = true;
  startBtn.innerHTML = '<i class="icon-loader spin"></i> Starting...';

  try {
    const response = await fetch("/api/tools/trade-watcher/start", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        mint,
        pool_address: twSelectedPool?.address || null,
        watch_type: watchType,
        trigger_amount_sol: triggerAmount,
        action_amount_sol: actionAmount,
        slippage_bps: slippage * 100,
      }),
    });

    const data = await response.json();

    if (!response.ok || !data.success) {
      throw new Error(data.error || "Failed to start watch");
    }

    Utils.showToast(`Watch started for ${data.symbol || mint.slice(0, 8)}...`, "success");

    // Clear form
    mintInput.value = "";
    twSelectedPool = null;
    updateTwPoolDisplay();
    updateTwStartButtonState();

    // Refresh watches list
    loadTwActiveWatches();
  } catch (error) {
    Utils.showToast(`Error: ${error.message}`, "error");
  } finally {
    startBtn.disabled = false;
    startBtn.innerHTML = '<i class="icon-play"></i> Start Watch';
    updateTwStartButtonState();
  }
}

/**
 * Handle stop all watches
 */
async function handleTwStopAllWatches() {
  const stopAllBtn = $("#tw-stop-all-btn");

  stopAllBtn.disabled = true;
  stopAllBtn.innerHTML = '<i class="icon-loader spin"></i> Stopping...';

  try {
    const response = await fetch("/api/tools/trade-watcher/stop-all", {
      method: "POST",
    });

    const data = await response.json();

    if (!response.ok || !data.success) {
      throw new Error(data.error || "Failed to stop watches");
    }

    Utils.showToast("All watches stopped", "success");
    loadTwActiveWatches();
  } catch (error) {
    Utils.showToast(`Error: ${error.message}`, "error");
  } finally {
    stopAllBtn.disabled = false;
    stopAllBtn.innerHTML = '<i class="icon-square"></i> Stop All';
  }
}

/**
 * Load and display active watches
 */
async function loadTwActiveWatches() {
  const tableEl = $("#tw-watches-table");
  const countEl = $("#tw-watch-count");
  const stopAllBtn = $("#tw-stop-all-btn");

  if (!tableEl) return;

  try {
    const response = await fetch("/api/tools/trade-watcher/list");
    const data = await response.json();

    if (!response.ok || !data.success) {
      throw new Error(data.error || "Failed to load watches");
    }

    const watches = data.watches || [];

    if (countEl) countEl.textContent = watches.length;
    if (stopAllBtn) stopAllBtn.disabled = watches.length === 0;

    if (watches.length === 0) {
      tableEl.innerHTML = `
        <div class="empty-state">
          <i class="icon-eye-off"></i>
          <p>No active watches</p>
          <small>Configure a watch above and click "Start Watch" to begin monitoring</small>
        </div>
      `;
      return;
    }

    tableEl.innerHTML = `
      <table class="tw-table">
        <thead>
          <tr>
            <th>Token</th>
            <th>Type</th>
            <th>Trigger</th>
            <th>Action</th>
            <th>Triggered</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          ${watches
            .map(
              (watch) => `
            <tr data-id="${watch.id}">
              <td>
                <div class="tw-token-cell">
                  <span class="tw-symbol">${Utils.escapeHtml(watch.symbol || "Unknown")}</span>
                  <span class="tw-mint">${watch.mint.slice(0, 8)}...</span>
                </div>
              </td>
              <td>
                <span class="tw-type-badge ${watch.watch_type}">${formatWatchType(watch.watch_type)}</span>
              </td>
              <td class="mono">${watch.trigger_amount_sol ? Utils.formatSol(watch.trigger_amount_sol) : "—"}</td>
              <td class="mono">${watch.action_amount_sol ? Utils.formatSol(watch.action_amount_sol) : "—"}</td>
              <td class="mono">${watch.trigger_count || 0}</td>
              <td>
                <button class="btn btn-sm btn-icon danger tw-stop-btn" title="Stop watch">
                  <i class="icon-x"></i>
                </button>
              </td>
            </tr>
          `
            )
            .join("")}
        </tbody>
      </table>
    `;

    // Wire up stop buttons
    tableEl.querySelectorAll(".tw-stop-btn").forEach((btn) => {
      on(btn, "click", (e) => {
        const row = e.target.closest("tr");
        const watchId = row?.dataset.id;
        if (watchId) {
          stopTwWatch(watchId);
        }
      });
    });
  } catch (error) {
    console.error("Failed to load watches:", error);
    tableEl.innerHTML = `
      <div class="error-state">
        <i class="icon-circle-alert"></i>
        <p>Failed to load watches</p>
      </div>
    `;
  }
}

/**
 * Stop a specific watch
 */
async function stopTwWatch(watchId) {
  try {
    const response = await fetch(`/api/tools/trade-watcher/stop/${watchId}`, {
      method: "POST",
    });

    const data = await response.json();

    if (!response.ok || !data.success) {
      throw new Error(data.error || "Failed to stop watch");
    }

    Utils.showToast("Watch stopped", "success");
    loadTwActiveWatches();
  } catch (error) {
    Utils.showToast(`Error: ${error.message}`, "error");
  }
}

/**
 * Format watch type for display
 */
function formatWatchType(type) {
  switch (type) {
    case "buy-on-sell":
      return "Buy on Sell";
    case "sell-on-buy":
      return "Sell on Buy";
    case "notify-only":
      return "Notify";
    default:
      return type;
  }
}

/**
 * Cleanup Trade Watcher resources
 */
function cleanupTradeWatcher() {
  if (twPoolSelector) {
    twPoolSelector.dispose();
    twPoolSelector = null;
  }
  if (twWatchesTable) {
    twWatchesTable.dispose();
    twWatchesTable = null;
  }
  if (twWatchPoller) {
    twWatchPoller.stop();
    twWatchPoller = null;
  }
  twSelectedPool = null;
}

// Volume aggregator state
let volumeAggregatorPoller = null;
let vaHistoryTable = null;
let vaToolFavorites = null;

function renderVolumeAggregatorTool(container, actionsContainer) {
  const hint = Hints.getHint("tools.volumeAggregator");
  const hintHtml = hint ? HintTrigger.render(hint, "tools.volumeAggregator", { size: "sm" }) : "";

  container.innerHTML = `
    <div class="tool-panel volume-aggregator-tool">
      <!-- Favorites -->
      <div class="tool-favorites-container" id="va-favorites-container"></div>

      <!-- Tab Navigation -->
      <div class="va-tabs">
        <button class="va-tab active" data-tab="config">
          <i class="icon-settings"></i> Configuration
        </button>
        <button class="va-tab" data-tab="history">
          <i class="icon-clock"></i> History
        </button>
      </div>

      <!-- Configuration Tab Content -->
      <div class="va-tab-content active" id="va-tab-config">
        <div class="tool-section">
          <div class="section-header">
            <h3><i class="icon-settings"></i> Configuration</h3>
            ${hintHtml}
          </div>
          <div class="section-content">
            <form class="tool-form" id="volume-aggregator-form">
              <div class="form-group">
                <label for="va-token-mint">Token Mint Address <span class="required">*</span></label>
                <input type="text" id="va-token-mint" placeholder="e.g., EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" required aria-required="true" aria-describedby="va-token-mint-hint" />
                <small id="va-token-mint-hint">The Solana token address to generate trading volume for</small>
              </div>
              
              <div class="form-row">
                <div class="form-group">
                  <label for="va-total-volume">Total Volume (SOL)</label>
                  <input type="number" id="va-total-volume" value="10" min="0.1" step="0.1" aria-describedby="va-total-volume-hint" />
                  <small id="va-total-volume-hint">Target SOL volume to generate across all transactions</small>
                </div>
                <div class="form-group">
                  <label for="va-num-wallets">Number of Wallets</label>
                  <select id="va-num-wallets" aria-describedby="va-num-wallets-hint" data-custom-select>
                    <option value="2">2 wallets</option>
                    <option value="3">3 wallets</option>
                    <option value="4">4 wallets</option>
                    <option value="5" selected>5 wallets</option>
                    <option value="6">6 wallets</option>
                    <option value="7">7 wallets</option>
                    <option value="8">8 wallets</option>
                    <option value="9">9 wallets</option>
                    <option value="10">10 wallets</option>
                  </select>
                  <small id="va-num-wallets-hint">Number of secondary wallets for trading</small>
                </div>
              </div>
              
              <div class="form-row">
                <div class="form-group">
                  <label for="va-min-amount">Min Amount per Tx (SOL)</label>
                  <input type="number" id="va-min-amount" value="0.05" min="0.001" step="0.01" aria-describedby="va-min-amount-hint" />
                  <small id="va-min-amount-hint">Smallest amount per transaction</small>
                </div>
                <div class="form-group">
                  <label for="va-max-amount">Max Amount per Tx (SOL)</label>
                  <input type="number" id="va-max-amount" value="0.2" min="0.001" step="0.01" aria-describedby="va-max-amount-hint" />
                  <small id="va-max-amount-hint">Largest amount per transaction</small>
                </div>
              </div>
              
              <div class="form-group">
                <label for="va-delay">Delay Between Txs (ms)</label>
                <input type="number" id="va-delay" value="3000" min="1000" step="100" aria-describedby="va-delay-hint" />
                <small id="va-delay-hint">Wait time between transactions (min 1000ms for rate limiting)</small>
              </div>
              
              <div class="form-group checkbox-group">
                <label>
                  <input type="checkbox" id="va-randomize" checked aria-describedby="va-randomize-hint" />
                  Randomize Amounts
                </label>
                <small id="va-randomize-hint">Vary transaction amounts within min/max range for natural trading patterns</small>
              </div>
            </form>
          </div>
        </div>

        <div class="tool-section">
          <div class="section-header">
            <h3><i class="icon-activity"></i> Session Status</h3>
          </div>
          <div class="section-content">
            <div class="va-status-display" id="va-status-display">
              <div class="va-status-header">
                <span class="va-status-badge ready" id="va-status-badge">Ready</span>
              </div>
              <div class="va-progress-section" id="va-progress-section" style="display: none;">
                <div class="va-stats-row">
                  <div class="va-stat">
                    <span class="va-stat-label">Volume Generated</span>
                    <span class="va-stat-value" id="va-volume-generated">0.00 SOL</span>
                  </div>
                  <div class="va-stat">
                    <span class="va-stat-label">Target</span>
                    <span class="va-stat-value" id="va-volume-target">— SOL</span>
                  </div>
                </div>
                <div class="va-progress-bar-wrapper">
                  <div class="va-progress-bar">
                    <div class="va-progress-fill" id="va-progress-fill" style="width: 0%"></div>
                  </div>
                  <span class="va-progress-percent" id="va-progress-percent">0%</span>
                </div>
                <div class="va-stats-row">
                  <div class="va-stat">
                    <span class="va-stat-label">Successful</span>
                    <span class="va-stat-value success" id="va-success-count">0</span>
                  </div>
                  <div class="va-stat">
                    <span class="va-stat-label">Failed</span>
                    <span class="va-stat-value error" id="va-failed-count">0</span>
                  </div>
                  <div class="va-stat">
                    <span class="va-stat-label">Duration</span>
                    <span class="va-stat-value" id="va-duration">0s</span>
                  </div>
                </div>
              </div>
              <div class="va-idle-state" id="va-idle-state">
                <i class="icon-chart-bar"></i>
                <p>Configure settings above and click Start to begin</p>
                <small>Requires at least 2 secondary wallets with SOL balance</small>
              </div>
            </div>
          </div>
        </div>

        <div class="tool-section" id="va-log-section" style="display: none;">
          <div class="section-header">
            <h3><i class="icon-list"></i> Transaction Log</h3>
            <div class="section-actions">
              <button class="btn btn-sm" id="va-clear-log" type="button">
                <i class="icon-trash-2"></i> Clear
              </button>
            </div>
          </div>
          <div class="section-content">
            <div class="va-transaction-log" id="va-transaction-log">
              <!-- Transaction entries will be added here -->
            </div>
          </div>
        </div>
      </div>

      <!-- History Tab Content -->
      <div class="va-tab-content" id="va-tab-history">
        <div class="tool-section">
          <div class="va-history-header">
            <h4><i class="icon-chart-bar"></i> Session Analytics</h4>
            <button class="btn btn-sm" id="va-refresh-history" type="button">
              <i class="icon-refresh-cw"></i> Refresh
            </button>
          </div>
          <div class="va-analytics-grid" id="va-analytics-grid">
            <div class="analytics-card">
              <span class="analytics-value" id="va-analytics-total-sessions">—</span>
              <span class="analytics-label">Total Sessions</span>
            </div>
            <div class="analytics-card">
              <span class="analytics-value" id="va-analytics-total-volume">—</span>
              <span class="analytics-label">Total Volume</span>
            </div>
            <div class="analytics-card">
              <span class="analytics-value" id="va-analytics-avg-success">—</span>
              <span class="analytics-label">Avg Success Rate</span>
            </div>
            <div class="analytics-card success">
              <span class="analytics-value" id="va-analytics-completed">—</span>
              <span class="analytics-label">Completed</span>
            </div>
            <div class="analytics-card error">
              <span class="analytics-value" id="va-analytics-failed">—</span>
              <span class="analytics-label">Failed</span>
            </div>
          </div>
        </div>

        <div class="tool-section">
          <div class="section-header">
            <h3><i class="icon-list"></i> Session History</h3>
          </div>
          <div class="section-content va-history-table" id="va-history-table-container">
            <div class="va-history-loading">
              <i class="icon-loader spin"></i>
              <span>Loading history...</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;

  HintTrigger.initAll();

  // Initialize favorites
  vaToolFavorites = new ToolFavorites({
    toolType: "volume_aggregator",
    container: "#va-favorites-container",
    onSelect: (favorite) => {
      // Populate form with favorite config
      populateVaFormFromFavorite(favorite);
    },
    getConfig: () => getVaFormConfig(),
  });

  actionsContainer.innerHTML = `
    <button class="btn success" id="va-start-btn">
      <i class="icon-play"></i> Start
    </button>
    <button class="btn danger" id="va-stop-btn" disabled>
      <i class="icon-square"></i> Stop
    </button>
  `;

  // Wire up event handlers
  const startBtn = $("#va-start-btn");
  const stopBtn = $("#va-stop-btn");
  const clearLogBtn = $("#va-clear-log");
  const refreshHistoryBtn = $("#va-refresh-history");

  if (startBtn) on(startBtn, "click", handleVolumeAggregatorStart);
  if (stopBtn) on(stopBtn, "click", handleVolumeAggregatorStop);
  if (clearLogBtn) on(clearLogBtn, "click", clearVolumeAggregatorLog);
  if (refreshHistoryBtn) on(refreshHistoryBtn, "click", loadVaSessionHistory);

  // Wire up tab switching
  const tabs = $$(".va-tabs .va-tab");
  tabs.forEach((tab) => {
    on(tab, "click", () => {
      const tabId = tab.dataset.tab;
      switchVaTab(tabId);
    });
  });

  // Check current status on load
  checkVolumeAggregatorStatus();
}

/**
 * Get current Volume Aggregator form configuration
 */
function getVaFormConfig() {
  return {
    mint: $("#va-token-mint")?.value?.trim() || "",
    total_volume_sol: parseFloat($("#va-total-volume")?.value) || 10,
    num_wallets: parseInt($("#va-num-wallets")?.value, 10) || 5,
    min_amount_sol: parseFloat($("#va-min-amount")?.value) || 0.05,
    max_amount_sol: parseFloat($("#va-max-amount")?.value) || 0.2,
    delay_ms: parseInt($("#va-delay")?.value, 10) || 3000,
    randomize: $("#va-randomize")?.checked ?? true,
  };
}

/**
 * Populate Volume Aggregator form from a favorite config
 */
function populateVaFormFromFavorite(favorite) {
  const config = favorite.config || {};

  // Populate form fields
  const mintInput = $("#va-token-mint");
  const volumeInput = $("#va-total-volume");
  const walletsInput = $("#va-num-wallets");
  const minInput = $("#va-min-amount");
  const maxInput = $("#va-max-amount");
  const delayInput = $("#va-delay");
  const randomizeInput = $("#va-randomize");

  if (mintInput) mintInput.value = favorite.mint || config.mint || "";
  if (volumeInput) volumeInput.value = config.total_volume_sol ?? 10;
  if (walletsInput) walletsInput.value = config.num_wallets ?? 5;
  if (minInput) minInput.value = config.min_amount_sol ?? 0.05;
  if (maxInput) maxInput.value = config.max_amount_sol ?? 0.2;
  if (delayInput) delayInput.value = config.delay_ms ?? 3000;
  if (randomizeInput) randomizeInput.checked = config.randomize ?? true;
}

/**
 * Validate volume aggregator form
 */
function validateVolumeAggregatorForm() {
  const tokenMint = $("#va-token-mint")?.value?.trim();
  const totalVolume = parseFloat($("#va-total-volume")?.value);
  const minAmount = parseFloat($("#va-min-amount")?.value);
  const maxAmount = parseFloat($("#va-max-amount")?.value);
  const delay = parseInt($("#va-delay")?.value, 10);

  // Token mint validation (base58 check)
  if (!tokenMint) {
    return { valid: false, error: "Token mint address is required" };
  }
  if (!/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(tokenMint)) {
    return { valid: false, error: "Invalid token mint address format" };
  }

  // Volume validation
  if (isNaN(totalVolume) || totalVolume <= 0) {
    return { valid: false, error: "Total volume must be greater than 0" };
  }

  // Amount validation
  if (isNaN(minAmount) || minAmount <= 0) {
    return { valid: false, error: "Minimum amount must be greater than 0" };
  }
  if (isNaN(maxAmount) || maxAmount < minAmount) {
    return { valid: false, error: "Maximum amount must be >= minimum amount" };
  }

  // Delay validation
  if (isNaN(delay) || delay < 1000) {
    return { valid: false, error: "Delay must be at least 1000ms" };
  }

  return { valid: true };
}

/**
 * Handle start button click
 */
async function handleVolumeAggregatorStart() {
  const validation = validateVolumeAggregatorForm();
  if (!validation.valid) {
    Utils.showToast(validation.error, "error");
    return;
  }

  const startBtn = $("#va-start-btn");
  if (startBtn) {
    startBtn.disabled = true;
    startBtn.innerHTML = '<i class="icon-loader spin"></i> Starting...';
  }

  const request = {
    token_mint: $("#va-token-mint")?.value?.trim(),
    total_volume_sol: parseFloat($("#va-total-volume")?.value),
    num_wallets: parseInt($("#va-num-wallets")?.value, 10),
    min_amount_sol: parseFloat($("#va-min-amount")?.value),
    max_amount_sol: parseFloat($("#va-max-amount")?.value),
    delay_between_ms: parseInt($("#va-delay")?.value, 10),
    randomize_amounts: $("#va-randomize")?.checked ?? true,
  };

  try {
    const response = await fetch("/api/tools/volume-aggregator/start", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    });

    const data = await response.json();

    if (!response.ok || data.error) {
      throw new Error(data.message || data.error || "Failed to start");
    }

    Utils.showToast("Volume aggregator started", "success");

    // Store target volume for progress calculation
    window.vaTargetVolume = request.total_volume_sol;

    // Update UI to running state
    updateVolumeAggregatorUI("running", null);

    // Start polling for status
    startVolumeAggregatorPolling();
  } catch (error) {
    console.error("Failed to start volume aggregator:", error);
    Utils.showToast(`Failed to start: ${error.message}`, "error");

    if (startBtn) {
      startBtn.disabled = false;
      startBtn.innerHTML = '<i class="icon-play"></i> Start';
    }
  }
}

/**
 * Handle stop button click
 */
async function handleVolumeAggregatorStop() {
  const stopBtn = $("#va-stop-btn");
  if (stopBtn) {
    stopBtn.disabled = true;
    stopBtn.innerHTML = '<i class="icon-loader spin"></i> Stopping...';
  }

  try {
    const response = await fetch("/api/tools/volume-aggregator/stop", {
      method: "POST",
    });

    const data = await response.json();

    if (!response.ok || data.error) {
      throw new Error(data.message || data.error || "Failed to stop");
    }

    Utils.showToast("Stop request sent", "info");
  } catch (error) {
    console.error("Failed to stop volume aggregator:", error);
    Utils.showToast(`Failed to stop: ${error.message}`, "error");

    if (stopBtn) {
      stopBtn.disabled = false;
      stopBtn.innerHTML = '<i class="icon-square"></i> Stop';
    }
  }
}

/**
 * Check current volume aggregator status
 */
async function checkVolumeAggregatorStatus() {
  try {
    const response = await fetch("/api/tools/volume-aggregator/status");
    const result = await response.json();
    const data = result.data || result;

    updateVolumeAggregatorUI(data.status, data.session);

    // If running, start polling
    if (data.status === "running") {
      startVolumeAggregatorPolling();
    }
  } catch (error) {
    console.error("Failed to check volume aggregator status:", error);
  }
}

/**
 * Start polling for status updates
 */
function startVolumeAggregatorPolling() {
  stopVolumeAggregatorPolling();

  volumeAggregatorPoller = setInterval(async () => {
    try {
      const response = await fetch("/api/tools/volume-aggregator/status");
      const result = await response.json();
      const data = result.data || result;

      updateVolumeAggregatorUI(data.status, data.session);

      // Stop polling if no longer running
      if (data.status !== "running") {
        stopVolumeAggregatorPolling();
      }
    } catch (error) {
      console.error("Failed to poll volume aggregator status:", error);
    }
  }, 2000);
}

/**
 * Stop polling
 */
function stopVolumeAggregatorPolling() {
  if (volumeAggregatorPoller) {
    clearInterval(volumeAggregatorPoller);
    volumeAggregatorPoller = null;
  }
}

/**
 * Update UI based on status
 */
function updateVolumeAggregatorUI(status, session) {
  const startBtn = $("#va-start-btn");
  const stopBtn = $("#va-stop-btn");
  const statusBadge = $("#va-status-badge");
  const progressSection = $("#va-progress-section");
  const idleState = $("#va-idle-state");
  const logSection = $("#va-log-section");
  const form = $("#volume-aggregator-form");

  // Update status badge
  if (statusBadge) {
    statusBadge.className = `va-status-badge ${status}`;
    statusBadge.textContent = getVolumeAggregatorStatusText(status);
  }

  // Update tool nav status
  updateToolStatus("volume-aggregator", status === "running" ? "running" : "ready");

  const isRunning = status === "running";
  const hasSession = session != null;

  // Button states
  if (startBtn) {
    startBtn.disabled = isRunning;
    startBtn.innerHTML = isRunning
      ? '<i class="icon-loader spin"></i> Running...'
      : '<i class="icon-play"></i> Start';
  }
  if (stopBtn) {
    stopBtn.disabled = !isRunning;
    stopBtn.innerHTML = '<i class="icon-square"></i> Stop';
  }

  // Form disabled state
  if (form) {
    const inputs = form.querySelectorAll("input, select");
    inputs.forEach((input) => {
      input.disabled = isRunning;
    });
  }

  // Show/hide sections
  if (progressSection) progressSection.style.display = hasSession ? "block" : "none";
  if (idleState) idleState.style.display = hasSession ? "none" : "flex";
  if (logSection) logSection.style.display = hasSession ? "block" : "none";

  // Update session data
  if (hasSession && session) {
    const targetVolume = window.vaTargetVolume || session.total_volume_sol || 10;
    const volumeGenerated = session.total_volume_sol || 0;
    const progress = Math.min(100, (volumeGenerated / targetVolume) * 100);

    const volumeGeneratedEl = $("#va-volume-generated");
    const volumeTargetEl = $("#va-volume-target");
    const progressFill = $("#va-progress-fill");
    const progressPercent = $("#va-progress-percent");
    const successCount = $("#va-success-count");
    const failedCount = $("#va-failed-count");
    const durationEl = $("#va-duration");

    if (volumeGeneratedEl) volumeGeneratedEl.textContent = `${volumeGenerated.toFixed(4)} SOL`;
    if (volumeTargetEl) volumeTargetEl.textContent = `${targetVolume.toFixed(2)} SOL`;
    if (progressFill) progressFill.style.width = `${progress}%`;
    if (progressPercent) progressPercent.textContent = `${progress.toFixed(1)}%`;
    if (successCount)
      successCount.textContent = (session.successful_buys || 0) + (session.successful_sells || 0);
    if (failedCount) failedCount.textContent = session.failed_count || 0;
    if (durationEl)
      durationEl.textContent = Utils.formatDuration((session.duration_secs || 0) * 1000);
  }
}

/**
 * Get status text for badge
 */
function getVolumeAggregatorStatusText(status) {
  const statusMap = {
    ready: "Ready",
    running: "Running",
    completed: "Completed",
    aborted: "Stopped",
    failed: "Failed",
  };
  return statusMap[status] || status;
}

/**
 * Clear the transaction log
 */
function clearVolumeAggregatorLog() {
  const logEl = $("#va-transaction-log");
  if (logEl) {
    logEl.innerHTML = '<div class="va-log-empty">Log cleared</div>';
  }
}

/**
 * Switch between Volume Aggregator tabs
 */
function switchVaTab(tabId) {
  // Update tab buttons
  const tabs = $$(".va-tabs .va-tab");
  tabs.forEach((tab) => {
    if (tab.dataset.tab === tabId) {
      tab.classList.add("active");
    } else {
      tab.classList.remove("active");
    }
  });

  // Update tab content visibility
  const configContent = $("#va-tab-config");
  const historyContent = $("#va-tab-history");

  if (tabId === "config") {
    if (configContent) configContent.classList.add("active");
    if (historyContent) historyContent.classList.remove("active");
  } else if (tabId === "history") {
    if (configContent) configContent.classList.remove("active");
    if (historyContent) historyContent.classList.add("active");
    // Load history data when switching to history tab
    loadVaSessionHistory();
  }
}

/**
 * Load Volume Aggregator session history from API
 */
async function loadVaSessionHistory() {
  const container = $("#va-history-table-container");
  if (!container) return;

  // Show loading state
  container.innerHTML = `
    <div class="va-history-loading">
      <i class="icon-loader spin"></i>
      <span>Loading history...</span>
    </div>
  `;

  try {
    const response = await fetch("/api/tools/volume-aggregator/sessions");
    const result = await response.json();

    if (!response.ok || result.error) {
      throw new Error(result.message || result.error || "Failed to load history");
    }

    const data = result.data || result;
    const sessions = data.sessions || [];
    const analytics = data.analytics || {};

    // Update analytics cards
    updateVaAnalytics(analytics);

    // Initialize or update DataTable
    if (sessions.length === 0) {
      container.innerHTML = `
        <div class="va-history-empty">
          <i class="icon-inbox"></i>
          <p>No session history yet</p>
        </div>
      `;
      return;
    }

    initVaHistoryTable(sessions);
  } catch (error) {
    console.error("Failed to load VA session history:", error);
    container.innerHTML = `
      <div class="va-history-empty">
        <i class="icon-circle-alert"></i>
        <p>Failed to load history: ${error.message}</p>
      </div>
    `;
  }
}

/**
 * Update Volume Aggregator analytics cards
 */
function updateVaAnalytics(analytics) {
  const totalSessions = $("#va-analytics-total-sessions");
  const totalVolume = $("#va-analytics-total-volume");
  const avgSuccess = $("#va-analytics-avg-success");
  const completed = $("#va-analytics-completed");
  const failed = $("#va-analytics-failed");

  if (totalSessions) {
    totalSessions.textContent = analytics.total_sessions ?? "—";
  }
  if (totalVolume) {
    const vol = analytics.total_volume_sol;
    totalVolume.textContent = vol != null ? `${vol.toFixed(2)} SOL` : "—";
  }
  if (avgSuccess) {
    const rate = analytics.avg_success_rate;
    avgSuccess.textContent = rate != null ? `${rate.toFixed(1)}%` : "—";
  }
  if (completed) {
    completed.textContent = analytics.completed_count ?? "—";
  }
  if (failed) {
    failed.textContent = analytics.failed_count ?? "—";
  }
}

/**
 * Initialize Volume Aggregator history DataTable
 */
function initVaHistoryTable(sessions) {
  // Clean up existing table
  if (vaHistoryTable) {
    vaHistoryTable.dispose();
    vaHistoryTable = null;
  }

  vaHistoryTable = new DataTable({
    container: "#va-history-table-container",
    columns: [
      {
        id: "created_at",
        label: "Date",
        sortable: true,
        width: 140,
        render: (value) => Utils.formatTimestamp(value),
      },
      {
        id: "token_mint",
        label: "Token",
        sortable: true,
        width: 120,
        render: (value) => `<span class="mono">${value.slice(0, 8)}...</span>`,
      },
      {
        id: "target_volume_sol",
        label: "Target",
        sortable: true,
        width: 90,
        render: (value) => `${value.toFixed(2)} SOL`,
      },
      {
        id: "actual_volume_sol",
        label: "Actual",
        sortable: true,
        width: 90,
        render: (value) => `${value.toFixed(2)} SOL`,
      },
      {
        id: "success_rate",
        label: "Success",
        sortable: true,
        width: 80,
        render: (value) => {
          const cls = value >= 90 ? "success" : value >= 50 ? "warning" : "error";
          return `<span class="${cls}">${value.toFixed(1)}%</span>`;
        },
      },
      {
        id: "duration_secs",
        label: "Duration",
        sortable: true,
        width: 80,
        render: (value) => Utils.formatDuration(value * 1000),
      },
      {
        id: "status",
        label: "Status",
        sortable: true,
        width: 100,
        render: (value) => {
          const statusClass =
            {
              completed: "success",
              failed: "error",
              aborted: "warning",
              running: "info",
            }[value] || "";
          return `<span class="badge ${statusClass}">${value}</span>`;
        },
      },
    ],
    sorting: { column: "created_at", direction: "desc" },
    stateKey: "va-history-table",
    onRowClick: (row) => showVaSessionDetails(row),
  });

  vaHistoryTable.setData(sessions);
}

/**
 * Show details for a Volume Aggregator session
 */
function showVaSessionDetails(session) {
  console.log("Session details:", session);
  // Future: show detailed session modal
}

/**
 * Resume a Volume Aggregator session (stub for future implementation)
 */
window.resumeVaSession = function (sessionId) {
  console.log("Resume session:", sessionId);
  Utils.showToast("Resume functionality coming soon", "info");
};

// =============================================================================
// Multi-Buy Tool
// =============================================================================

let multiBuyState = {
  sessionId: null,
  status: "idle", // idle, running, completed, failed
  walletResults: [],
  poller: null,
};

function renderBuyMultiWalletsTool(container, actionsContainer) {
  const hint = Hints.getHint("tools.multiBuy");
  const hintHtml = hint ? HintTrigger.render(hint, "tools.multiBuy", { size: "sm" }) : "";

  container.innerHTML = `
    <div class="tool-panel multi-buy-tool">
      <!-- Token Input -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-coins"></i> Token</h3>
          ${hintHtml}
        </div>
        <div class="section-content">
          <div class="form-group">
            <label for="mb-token-mint">Token Mint Address <span class="required">*</span></label>
            <input type="text" id="mb-token-mint" placeholder="Paste token mint address..." />
            <small>The token you want to buy across multiple wallets</small>
          </div>
        </div>
      </div>

      <!-- Wallet Settings -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-wallet"></i> Wallet Settings</h3>
        </div>
        <div class="section-content">
          <form class="tool-form" id="mb-wallet-form">
            <div class="form-row">
              <div class="form-group">
                <label for="mb-wallet-count">Wallet Count</label>
                <select id="mb-wallet-count" data-custom-select>
                  <option value="2">2 wallets</option>
                  <option value="3">3 wallets</option>
                  <option value="4">4 wallets</option>
                  <option value="5" selected>5 wallets</option>
                  <option value="6">6 wallets</option>
                  <option value="8">8 wallets</option>
                  <option value="10">10 wallets</option>
                </select>
                <small>Number of sub-wallets to use</small>
              </div>
              <div class="form-group">
                <label for="mb-sol-buffer">SOL Buffer per Wallet</label>
                <input type="number" id="mb-sol-buffer" value="0.015" min="0.005" step="0.005" />
                <small>Reserved for fees (0.015 SOL min)</small>
              </div>
            </div>
          </form>
        </div>
      </div>

      <!-- Amount Settings -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-dollar-sign"></i> Amount Settings</h3>
        </div>
        <div class="section-content">
          <form class="tool-form">
            <div class="form-row">
              <div class="form-group">
                <label for="mb-min-sol">Min SOL per Wallet</label>
                <input type="number" id="mb-min-sol" value="0.01" min="0.001" step="0.01" />
                <small>Minimum buy amount</small>
              </div>
              <div class="form-group">
                <label for="mb-max-sol">Max SOL per Wallet</label>
                <input type="number" id="mb-max-sol" value="0.05" min="0.001" step="0.01" />
                <small>Maximum buy amount</small>
              </div>
              <div class="form-group">
                <label for="mb-total-limit">Total SOL Limit (optional)</label>
                <input type="number" id="mb-total-limit" placeholder="—" min="0" step="0.1" />
                <small>Maximum total spend</small>
              </div>
            </div>
          </form>
        </div>
      </div>

      <!-- Execution Settings -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-settings"></i> Execution Settings</h3>
        </div>
        <div class="section-content">
          <form class="tool-form">
            <div class="form-row">
              <div class="form-group">
                <label for="mb-delay-min">Delay Min (ms)</label>
                <input type="number" id="mb-delay-min" value="1000" min="500" step="100" />
              </div>
              <div class="form-group">
                <label for="mb-delay-max">Delay Max (ms)</label>
                <input type="number" id="mb-delay-max" value="2000" min="500" step="100" />
              </div>
              <div class="form-group">
                <label for="mb-concurrency">Concurrency</label>
                <select id="mb-concurrency" data-custom-select>
                  <option value="1" selected>1 (Sequential)</option>
                  <option value="2">2 parallel</option>
                  <option value="3">3 parallel</option>
                </select>
              </div>
            </div>
            <div class="form-row">
              <div class="form-group">
                <label for="mb-slippage">Slippage (%)</label>
                <input type="number" id="mb-slippage" value="5" min="0.5" max="50" step="0.5" />
              </div>
              <div class="form-group">
                <label for="mb-router">Router</label>
                <select id="mb-router" data-custom-select>
                  <option value="auto" selected>Auto (Best Route)</option>
                  <option value="jupiter">Jupiter</option>
                  <option value="raydium">Raydium</option>
                </select>
              </div>
            </div>
          </form>
        </div>
      </div>

      <!-- Preview Section -->
      <div class="tool-section" id="mb-preview-section" style="display: none;">
        <div class="section-header">
          <h3><i class="icon-eye"></i> Preview</h3>
        </div>
        <div class="section-content">
          <div class="mw-preview-grid" id="mb-preview-grid">
            <!-- Preview stats populated dynamically -->
          </div>
        </div>
      </div>

      <!-- Progress Section -->
      <div class="tool-section" id="mb-progress-section" style="display: none;">
        <div class="section-header">
          <h3><i class="icon-activity"></i> Progress</h3>
        </div>
        <div class="section-content">
          <div class="mw-progress-container">
            <div class="mw-progress-bar-wrapper">
              <div class="mw-progress-bar">
                <div class="mw-progress-fill" id="mb-progress-fill" style="width: 0%"></div>
              </div>
              <span class="mw-progress-percent" id="mb-progress-percent">0%</span>
            </div>
            <div class="mw-progress-status" id="mb-progress-status">Preparing...</div>
          </div>
          <div class="mw-results-table" id="mb-results-table">
            <!-- Results populated dynamically -->
          </div>
        </div>
      </div>
    </div>
  `;

  HintTrigger.initAll();
  enhanceAllSelects(container);

  actionsContainer.innerHTML = `
    <button class="btn" id="mb-preview-btn">
      <i class="icon-eye"></i> Preview
    </button>
    <button class="btn success" id="mb-start-btn" disabled>
      <i class="icon-shopping-cart"></i> Start Multi-Buy
    </button>
    <button class="btn danger" id="mb-stop-btn" style="display: none;">
      <i class="icon-x"></i> Stop
    </button>
  `;

  // Wire up event handlers
  const previewBtn = $("#mb-preview-btn");
  const startBtn = $("#mb-start-btn");
  const stopBtn = $("#mb-stop-btn");

  if (previewBtn) on(previewBtn, "click", handleMultiBuyPreview);
  if (startBtn) on(startBtn, "click", handleMultiBuyStart);
  if (stopBtn) on(stopBtn, "click", handleMultiBuyStop);
}

async function handleMultiBuyPreview() {
  const tokenMint = $("#mb-token-mint")?.value?.trim();
  if (!tokenMint) {
    Utils.showToast("Please enter a token mint address", "error");
    return;
  }

  const previewBtn = $("#mb-preview-btn");
  const previewSection = $("#mb-preview-section");
  const previewGrid = $("#mb-preview-grid");
  const startBtn = $("#mb-start-btn");

  if (!previewBtn || !previewSection || !previewGrid) return;

  previewBtn.disabled = true;
  previewBtn.innerHTML = '<i class="icon-loader spin"></i> Loading...';

  const config = {
    token_mint: tokenMint,
    wallet_count: parseInt($("#mb-wallet-count")?.value || "5"),
    sol_buffer: parseFloat($("#mb-sol-buffer")?.value || "0.015"),
    min_sol: parseFloat($("#mb-min-sol")?.value || "0.01"),
    max_sol: parseFloat($("#mb-max-sol")?.value || "0.05"),
    total_limit: parseFloat($("#mb-total-limit")?.value) || null,
  };

  try {
    const response = await fetch("/api/tools/multi-buy/preview", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || `HTTP ${response.status}`);
    }

    const preview = await response.json();

    previewSection.style.display = "block";
    previewGrid.innerHTML = `
      <div class="mw-preview-item">
        <span class="mw-preview-label">Wallets to Create</span>
        <span class="mw-preview-value">${preview.wallet_count}</span>
      </div>
      <div class="mw-preview-item">
        <span class="mw-preview-label">Amount per Wallet</span>
        <span class="mw-preview-value">${Utils.formatSol(config.min_sol)} - ${Utils.formatSol(config.max_sol)}</span>
      </div>
      <div class="mw-preview-item">
        <span class="mw-preview-label">Total SOL Needed</span>
        <span class="mw-preview-value">${Utils.formatSol(preview.total_needed)}</span>
      </div>
      <div class="mw-preview-item ${preview.sufficient_balance ? "success" : "error"}">
        <span class="mw-preview-label">Main Balance</span>
        <span class="mw-preview-value">${Utils.formatSol(preview.main_balance)} ${preview.sufficient_balance ? "✓" : "✗"}</span>
      </div>
    `;

    if (startBtn) {
      startBtn.disabled = !preview.sufficient_balance;
    }
  } catch (error) {
    console.error("Multi-buy preview failed:", error);
    Utils.showToast(`Preview failed: ${error.message}`, "error");
    previewSection.style.display = "none";
  } finally {
    previewBtn.disabled = false;
    previewBtn.innerHTML = '<i class="icon-eye"></i> Preview';
  }
}

async function handleMultiBuyStart() {
  const tokenMint = $("#mb-token-mint")?.value?.trim();
  if (!tokenMint) return;

  const startBtn = $("#mb-start-btn");
  const stopBtn = $("#mb-stop-btn");
  const progressSection = $("#mb-progress-section");

  if (!startBtn || !stopBtn || !progressSection) return;

  startBtn.style.display = "none";
  stopBtn.style.display = "inline-flex";
  progressSection.style.display = "block";

  const config = {
    token_mint: tokenMint,
    wallet_count: parseInt($("#mb-wallet-count")?.value || "5"),
    sol_buffer: parseFloat($("#mb-sol-buffer")?.value || "0.015"),
    min_sol: parseFloat($("#mb-min-sol")?.value || "0.01"),
    max_sol: parseFloat($("#mb-max-sol")?.value || "0.05"),
    total_limit: parseFloat($("#mb-total-limit")?.value) || null,
    delay_min_ms: parseInt($("#mb-delay-min")?.value || "1000"),
    delay_max_ms: parseInt($("#mb-delay-max")?.value || "2000"),
    concurrency: parseInt($("#mb-concurrency")?.value || "1"),
    slippage_bps: parseFloat($("#mb-slippage")?.value || "5") * 100,
    router: $("#mb-router")?.value || "auto",
  };

  try {
    const response = await fetch("/api/tools/multi-buy/start", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || `HTTP ${response.status}`);
    }

    const result = await response.json();
    multiBuyState.sessionId = result.session_id;
    multiBuyState.status = "running";

    // Start polling for status
    startMultiBuyPolling();
    Utils.showToast("Multi-buy started", "success");
  } catch (error) {
    console.error("Multi-buy start failed:", error);
    Utils.showToast(`Failed to start: ${error.message}`, "error");
    resetMultiBuyUI();
  }
}

function startMultiBuyPolling() {
  if (multiBuyState.poller) {
    clearInterval(multiBuyState.poller);
  }

  multiBuyState.poller = setInterval(async () => {
    if (!multiBuyState.sessionId) return;

    try {
      const response = await fetch(`/api/tools/multi-buy/${multiBuyState.sessionId}`);
      if (!response.ok) return;

      const status = await response.json();
      updateMultiBuyProgress(status);

      if (status.status === "completed" || status.status === "failed") {
        stopMultiBuyPolling();
        multiBuyState.status = status.status;
        Utils.showToast(
          status.status === "completed"
            ? `Multi-buy completed! ${status.success_count}/${status.total_count} successful`
            : "Multi-buy failed",
          status.status === "completed" ? "success" : "error"
        );
      }
    } catch (error) {
      console.error("Multi-buy polling error:", error);
    }
  }, 2000);
}

function stopMultiBuyPolling() {
  if (multiBuyState.poller) {
    clearInterval(multiBuyState.poller);
    multiBuyState.poller = null;
  }
}

function updateMultiBuyProgress(status) {
  const progressFill = $("#mb-progress-fill");
  const progressPercent = $("#mb-progress-percent");
  const progressStatus = $("#mb-progress-status");
  const resultsTable = $("#mb-results-table");

  const percent =
    status.total_count > 0 ? Math.round((status.completed_count / status.total_count) * 100) : 0;

  if (progressFill) progressFill.style.width = `${percent}%`;
  if (progressPercent) progressPercent.textContent = `${percent}%`;
  if (progressStatus) {
    progressStatus.textContent = `${status.status === "running" ? "Executing buys..." : status.status} (${status.completed_count}/${status.total_count})`;
  }

  if (resultsTable && status.wallets) {
    resultsTable.innerHTML = `
      <table class="mw-results">
        <thead>
          <tr>
            <th>Wallet</th>
            <th>Address</th>
            <th>SOL</th>
            <th>Tokens</th>
            <th>Status</th>
          </tr>
        </thead>
        <tbody>
          ${status.wallets
            .map(
              (w) => `
            <tr class="${w.status}">
              <td>${Utils.escapeHtml(w.name)}</td>
              <td class="mono">${Utils.formatAddressCompact(w.address)}</td>
              <td class="mono">${Utils.formatSol(w.sol_spent, { suffix: "" })}</td>
              <td class="mono">${w.tokens_received ? Utils.formatNumber(w.tokens_received) : "—"}</td>
              <td><span class="mw-status-badge ${w.status}">${formatWalletStatus(w.status)}</span></td>
            </tr>
          `
            )
            .join("")}
        </tbody>
      </table>
    `;
  }
}

function formatWalletStatus(status) {
  const icons = {
    pending: "⏳",
    running: "🔄",
    success: "✓",
    failed: "✗",
  };
  return `${icons[status] || ""} ${status.charAt(0).toUpperCase() + status.slice(1)}`;
}

async function handleMultiBuyStop() {
  if (!multiBuyState.sessionId) return;

  try {
    await fetch(`/api/tools/multi-buy/${multiBuyState.sessionId}/stop`, { method: "POST" });
    stopMultiBuyPolling();
    Utils.showToast("Multi-buy stopped", "info");
    resetMultiBuyUI();
  } catch (error) {
    console.error("Failed to stop multi-buy:", error);
  }
}

function resetMultiBuyUI() {
  const startBtn = $("#mb-start-btn");
  const stopBtn = $("#mb-stop-btn");

  if (startBtn) startBtn.style.display = "inline-flex";
  if (stopBtn) stopBtn.style.display = "none";

  multiBuyState = { sessionId: null, status: "idle", walletResults: [], poller: null };
}

// =============================================================================
// Multi-Sell Tool
// =============================================================================

let multiSellState = {
  sessionId: null,
  status: "idle",
  walletResults: [],
  poller: null,
};

function renderSellMultiWalletsTool(container, actionsContainer) {
  const hint = Hints.getHint("tools.multiSell");
  const hintHtml = hint ? HintTrigger.render(hint, "tools.multiSell", { size: "sm" }) : "";

  container.innerHTML = `
    <div class="tool-panel multi-sell-tool">
      <!-- Token Input -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-coins"></i> Token</h3>
          ${hintHtml}
        </div>
        <div class="section-content">
          <div class="form-group">
            <label for="ms-token-mint">Token Mint Address <span class="required">*</span></label>
            <div class="input-group">
              <input type="text" id="ms-token-mint" placeholder="Paste token mint address..." />
              <button class="btn" id="ms-scan-btn" type="button">
                <i class="icon-search"></i> Scan
              </button>
            </div>
            <small>Enter a token address to scan for wallets holding it</small>
          </div>
        </div>
      </div>

      <!-- Sell Settings -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-settings"></i> Sell Settings</h3>
        </div>
        <div class="section-content">
          <form class="tool-form">
            <div class="form-row">
              <div class="form-group">
                <label for="ms-sell-percent">Sell Percentage</label>
                <input type="number" id="ms-sell-percent" value="100" min="1" max="100" step="1" />
                <small>% of tokens to sell per wallet</small>
              </div>
              <div class="form-group">
                <label for="ms-min-sol-fee">Min SOL for Fee</label>
                <input type="number" id="ms-min-sol-fee" value="0.01" min="0.005" step="0.005" />
                <small>Minimum SOL needed for tx fee</small>
              </div>
            </div>
            <div class="form-group checkbox-group">
              <label>
                <input type="checkbox" id="ms-auto-topup" checked />
                Auto topup if needed
              </label>
              <small>Transfer SOL from main wallet if sub-wallet has insufficient balance</small>
            </div>
          </form>
        </div>
      </div>

      <!-- Post-Sell Actions -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-arrow-right"></i> Post-Sell Actions</h3>
        </div>
        <div class="section-content">
          <form class="tool-form">
            <div class="form-group checkbox-group">
              <label>
                <input type="checkbox" id="ms-consolidate" checked />
                Consolidate SOL to main wallet
              </label>
              <small>Transfer all SOL from sub-wallets back to main wallet</small>
            </div>
            <div class="form-group checkbox-group">
              <label>
                <input type="checkbox" id="ms-close-atas" checked />
                Close token ATAs after sell
              </label>
              <small>Reclaim ~0.002 SOL per ATA</small>
            </div>
          </form>
        </div>
      </div>

      <!-- Execution Settings -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-zap"></i> Execution Settings</h3>
        </div>
        <div class="section-content">
          <form class="tool-form">
            <div class="form-row">
              <div class="form-group">
                <label for="ms-delay-min">Delay Min (ms)</label>
                <input type="number" id="ms-delay-min" value="1000" min="500" step="100" />
              </div>
              <div class="form-group">
                <label for="ms-delay-max">Delay Max (ms)</label>
                <input type="number" id="ms-delay-max" value="2000" min="500" step="100" />
              </div>
              <div class="form-group">
                <label for="ms-concurrency">Concurrency</label>
                <select id="ms-concurrency" data-custom-select>
                  <option value="1" selected>1 (Sequential)</option>
                  <option value="2">2 parallel</option>
                  <option value="3">3 parallel</option>
                </select>
              </div>
            </div>
            <div class="form-row">
              <div class="form-group">
                <label for="ms-slippage">Slippage (%)</label>
                <input type="number" id="ms-slippage" value="5" min="0.5" max="50" step="0.5" />
              </div>
              <div class="form-group">
                <label for="ms-router">Router</label>
                <select id="ms-router" data-custom-select>
                  <option value="auto" selected>Auto (Best Route)</option>
                  <option value="jupiter">Jupiter</option>
                  <option value="raydium">Raydium</option>
                </select>
              </div>
            </div>
          </form>
        </div>
      </div>

      <!-- Wallets with Token -->
      <div class="tool-section" id="ms-wallets-section" style="display: none;">
        <div class="section-header">
          <h3><i class="icon-wallet"></i> Wallets with Token</h3>
          <div class="section-actions">
            <button class="btn btn-sm" id="ms-select-all-btn" type="button">Select All</button>
          </div>
        </div>
        <div class="section-content">
          <div class="mw-wallet-list" id="ms-wallet-list">
            <!-- Populated by scan -->
          </div>
          <div class="mw-selection-summary" id="ms-selection-summary">
            <!-- Selection summary -->
          </div>
        </div>
      </div>

      <!-- Progress Section -->
      <div class="tool-section" id="ms-progress-section" style="display: none;">
        <div class="section-header">
          <h3><i class="icon-activity"></i> Progress</h3>
        </div>
        <div class="section-content">
          <div class="mw-progress-container">
            <div class="mw-progress-bar-wrapper">
              <div class="mw-progress-bar">
                <div class="mw-progress-fill" id="ms-progress-fill" style="width: 0%"></div>
              </div>
              <span class="mw-progress-percent" id="ms-progress-percent">0%</span>
            </div>
            <div class="mw-progress-status" id="ms-progress-status">Preparing...</div>
          </div>
          <div class="mw-results-table" id="ms-results-table">
            <!-- Results populated dynamically -->
          </div>
        </div>
      </div>
    </div>
  `;

  HintTrigger.initAll();
  enhanceAllSelects(container);

  actionsContainer.innerHTML = `
    <button class="btn success" id="ms-start-btn" disabled>
      <i class="icon-package"></i> Start Multi-Sell
    </button>
    <button class="btn danger" id="ms-stop-btn" style="display: none;">
      <i class="icon-x"></i> Stop
    </button>
  `;

  // Wire up event handlers
  const scanBtn = $("#ms-scan-btn");
  const selectAllBtn = $("#ms-select-all-btn");
  const startBtn = $("#ms-start-btn");
  const stopBtn = $("#ms-stop-btn");

  if (scanBtn) on(scanBtn, "click", handleMultiSellScan);
  if (selectAllBtn) on(selectAllBtn, "click", handleMultiSellSelectAll);
  if (startBtn) on(startBtn, "click", handleMultiSellStart);
  if (stopBtn) on(stopBtn, "click", handleMultiSellStop);
}

async function handleMultiSellScan() {
  const tokenMint = $("#ms-token-mint")?.value?.trim();
  if (!tokenMint) {
    Utils.showToast("Please enter a token mint address", "error");
    return;
  }

  const scanBtn = $("#ms-scan-btn");
  const walletsSection = $("#ms-wallets-section");
  const walletList = $("#ms-wallet-list");

  if (!scanBtn || !walletsSection || !walletList) return;

  scanBtn.disabled = true;
  scanBtn.innerHTML = '<i class="icon-loader spin"></i>';

  try {
    const response = await fetch(
      `/api/tools/multi-sell/scan?token_mint=${encodeURIComponent(tokenMint)}`
    );
    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || `HTTP ${response.status}`);
    }

    const data = await response.json();

    if (data.wallets.length === 0) {
      walletList.innerHTML = `
        <div class="empty-state">
          <i class="icon-inbox"></i>
          <p>No sub-wallets hold this token</p>
        </div>
      `;
      walletsSection.style.display = "block";
      return;
    }

    walletList.innerHTML = `
      <table class="mw-wallet-table">
        <thead>
          <tr>
            <th><input type="checkbox" id="ms-check-all" checked /></th>
            <th>Wallet</th>
            <th>Tokens</th>
            <th>SOL Balance</th>
            <th>Needs Topup</th>
          </tr>
        </thead>
        <tbody>
          ${data.wallets
            .map(
              (w) => `
            <tr data-wallet="${w.address}">
              <td><input type="checkbox" class="ms-wallet-check" data-address="${w.address}" checked /></td>
              <td>${Utils.escapeHtml(w.name)}</td>
              <td class="mono">${Utils.formatNumber(w.token_balance)}</td>
              <td class="mono">${Utils.formatSol(w.sol_balance, { suffix: "" })}</td>
              <td>${w.needs_topup ? `<span class="warning">Yes (+${Utils.formatSol(w.topup_amount, { suffix: "" })})</span>` : '<span class="success">No</span>'}</td>
            </tr>
          `
            )
            .join("")}
        </tbody>
      </table>
    `;

    walletsSection.style.display = "block";
    updateMultiSellSelectionSummary();

    // Wire up checkbox changes
    const checkAll = $("#ms-check-all");
    if (checkAll) {
      on(checkAll, "change", (e) => {
        const checks = $$(".ms-wallet-check");
        checks.forEach((c) => (c.checked = e.target.checked));
        updateMultiSellSelectionSummary();
      });
    }

    $$(".ms-wallet-check").forEach((check) => {
      on(check, "change", updateMultiSellSelectionSummary);
    });
  } catch (error) {
    console.error("Multi-sell scan failed:", error);
    Utils.showToast(`Scan failed: ${error.message}`, "error");
  } finally {
    scanBtn.disabled = false;
    scanBtn.innerHTML = '<i class="icon-search"></i> Scan';
  }
}

function handleMultiSellSelectAll() {
  const checks = $$(".ms-wallet-check");
  const allChecked = Array.from(checks).every((c) => c.checked);
  checks.forEach((c) => (c.checked = !allChecked));

  const checkAll = $("#ms-check-all");
  if (checkAll) checkAll.checked = !allChecked;

  updateMultiSellSelectionSummary();
}

function updateMultiSellSelectionSummary() {
  const summary = $("#ms-selection-summary");
  const startBtn = $("#ms-start-btn");
  const checks = $$(".ms-wallet-check:checked");

  const selectedCount = checks.length;

  if (summary) {
    if (selectedCount === 0) {
      summary.innerHTML = "<span class=\"text-muted\">No wallets selected</span>";
    } else {
      summary.innerHTML = `<span class="text-primary">Selected: ${selectedCount} wallet${selectedCount > 1 ? "s" : ""}</span>`;
    }
  }

  if (startBtn) {
    startBtn.disabled = selectedCount === 0;
  }
}

async function handleMultiSellStart() {
  const tokenMint = $("#ms-token-mint")?.value?.trim();
  if (!tokenMint) return;

  const selectedWallets = Array.from($$(".ms-wallet-check:checked")).map((c) => c.dataset.address);
  if (selectedWallets.length === 0) {
    Utils.showToast("Please select at least one wallet", "error");
    return;
  }

  const startBtn = $("#ms-start-btn");
  const stopBtn = $("#ms-stop-btn");
  const progressSection = $("#ms-progress-section");

  if (!startBtn || !stopBtn || !progressSection) return;

  startBtn.style.display = "none";
  stopBtn.style.display = "inline-flex";
  progressSection.style.display = "block";

  const config = {
    token_mint: tokenMint,
    wallets: selectedWallets,
    sell_percent: parseFloat($("#ms-sell-percent")?.value || "100"),
    min_sol_fee: parseFloat($("#ms-min-sol-fee")?.value || "0.01"),
    auto_topup: $("#ms-auto-topup")?.checked ?? true,
    consolidate: $("#ms-consolidate")?.checked ?? true,
    close_atas: $("#ms-close-atas")?.checked ?? true,
    delay_min_ms: parseInt($("#ms-delay-min")?.value || "1000"),
    delay_max_ms: parseInt($("#ms-delay-max")?.value || "2000"),
    concurrency: parseInt($("#ms-concurrency")?.value || "1"),
    slippage_bps: parseFloat($("#ms-slippage")?.value || "5") * 100,
    router: $("#ms-router")?.value || "auto",
  };

  try {
    const response = await fetch("/api/tools/multi-sell/start", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || `HTTP ${response.status}`);
    }

    const result = await response.json();
    multiSellState.sessionId = result.session_id;
    multiSellState.status = "running";

    startMultiSellPolling();
    Utils.showToast("Multi-sell started", "success");
  } catch (error) {
    console.error("Multi-sell start failed:", error);
    Utils.showToast(`Failed to start: ${error.message}`, "error");
    resetMultiSellUI();
  }
}

function startMultiSellPolling() {
  if (multiSellState.poller) {
    clearInterval(multiSellState.poller);
  }

  multiSellState.poller = setInterval(async () => {
    if (!multiSellState.sessionId) return;

    try {
      const response = await fetch(`/api/tools/multi-sell/${multiSellState.sessionId}`);
      if (!response.ok) return;

      const status = await response.json();
      updateMultiSellProgress(status);

      if (status.status === "completed" || status.status === "failed") {
        stopMultiSellPolling();
        multiSellState.status = status.status;
        Utils.showToast(
          status.status === "completed"
            ? `Multi-sell completed! ${Utils.formatSol(status.total_sol_received)} received`
            : "Multi-sell failed",
          status.status === "completed" ? "success" : "error"
        );
      }
    } catch (error) {
      console.error("Multi-sell polling error:", error);
    }
  }, 2000);
}

function stopMultiSellPolling() {
  if (multiSellState.poller) {
    clearInterval(multiSellState.poller);
    multiSellState.poller = null;
  }
}

function updateMultiSellProgress(status) {
  const progressFill = $("#ms-progress-fill");
  const progressPercent = $("#ms-progress-percent");
  const progressStatus = $("#ms-progress-status");
  const resultsTable = $("#ms-results-table");

  const percent =
    status.total_count > 0 ? Math.round((status.completed_count / status.total_count) * 100) : 0;

  if (progressFill) progressFill.style.width = `${percent}%`;
  if (progressPercent) progressPercent.textContent = `${percent}%`;
  if (progressStatus) {
    progressStatus.textContent = `${status.status === "running" ? "Executing sells..." : status.status} (${status.completed_count}/${status.total_count})`;
  }

  if (resultsTable && status.wallets) {
    resultsTable.innerHTML = `
      <table class="mw-results">
        <thead>
          <tr>
            <th>Wallet</th>
            <th>Tokens Sold</th>
            <th>SOL Received</th>
            <th>Status</th>
          </tr>
        </thead>
        <tbody>
          ${status.wallets
            .map(
              (w) => `
            <tr class="${w.status}">
              <td>${Utils.escapeHtml(w.name)}</td>
              <td class="mono">${w.tokens_sold ? Utils.formatNumber(w.tokens_sold) : "—"}</td>
              <td class="mono">${Utils.formatSol(w.sol_received, { suffix: "" })}</td>
              <td><span class="mw-status-badge ${w.status}">${formatWalletStatus(w.status)}</span></td>
            </tr>
          `
            )
            .join("")}
        </tbody>
      </table>
    `;
  }
}

async function handleMultiSellStop() {
  if (!multiSellState.sessionId) return;

  try {
    await fetch(`/api/tools/multi-sell/${multiSellState.sessionId}/stop`, { method: "POST" });
    stopMultiSellPolling();
    Utils.showToast("Multi-sell stopped", "info");
    resetMultiSellUI();
  } catch (error) {
    console.error("Failed to stop multi-sell:", error);
  }
}

function resetMultiSellUI() {
  const startBtn = $("#ms-start-btn");
  const stopBtn = $("#ms-stop-btn");

  if (startBtn) startBtn.style.display = "inline-flex";
  if (stopBtn) stopBtn.style.display = "none";

  multiSellState = { sessionId: null, status: "idle", walletResults: [], poller: null };
}

// =============================================================================
// Wallet Consolidation Tool
// =============================================================================

let consolidationState = {
  wallets: [],
  selectedAddresses: new Set(),
};

function renderWalletConsolidationTool(container, actionsContainer) {
  const hint = Hints.getHint("tools.walletConsolidation");
  const hintHtml = hint
    ? HintTrigger.render(hint, "tools.walletConsolidation", { size: "sm" })
    : "";

  container.innerHTML = `
    <div class="tool-panel wallet-consolidation-tool">
      <!-- Summary Section -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-chart-pie"></i> Summary</h3>
          ${hintHtml}
        </div>
        <div class="section-content">
          <div class="wc-summary-grid" id="wc-summary-grid">
            <div class="wc-summary-item">
              <span class="wc-summary-value" id="wc-wallet-count">—</span>
              <span class="wc-summary-label">Sub-wallets</span>
            </div>
            <div class="wc-summary-item">
              <span class="wc-summary-value" id="wc-total-sol">—</span>
              <span class="wc-summary-label">Total SOL</span>
            </div>
            <div class="wc-summary-item">
              <span class="wc-summary-value" id="wc-total-tokens">—</span>
              <span class="wc-summary-label">Token Types</span>
            </div>
            <div class="wc-summary-item">
              <span class="wc-summary-value" id="wc-reclaimable">—</span>
              <span class="wc-summary-label">Reclaimable Rent</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Wallets Table -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-wallet"></i> Wallets</h3>
          <div class="section-actions">
            <button class="btn btn-sm" id="wc-refresh-btn" type="button">
              <i class="icon-refresh-cw"></i> Refresh
            </button>
          </div>
        </div>
        <div class="section-content">
          <div class="wc-wallets-container" id="wc-wallets-container">
            <div class="loading-state">
              <i class="icon-loader spin"></i>
              <p>Loading wallets...</p>
            </div>
          </div>
          <div class="wc-selection-summary" id="wc-selection-summary">
            <!-- Selection summary -->
          </div>
        </div>
      </div>
    </div>
  `;

  HintTrigger.initAll();

  actionsContainer.innerHTML = `
    <button class="btn" id="wc-transfer-sol-btn" disabled>
      <i class="icon-arrow-right"></i> Transfer SOL
    </button>
    <button class="btn" id="wc-transfer-tokens-btn" disabled>
      <i class="icon-send"></i> Transfer All Tokens
    </button>
    <button class="btn primary" id="wc-cleanup-btn" disabled>
      <i class="icon-trash-2"></i> Cleanup ATAs
    </button>
  `;

  // Wire up event handlers
  const refreshBtn = $("#wc-refresh-btn");
  const transferSolBtn = $("#wc-transfer-sol-btn");
  const transferTokensBtn = $("#wc-transfer-tokens-btn");
  const cleanupBtn = $("#wc-cleanup-btn");

  if (refreshBtn) on(refreshBtn, "click", loadConsolidationData);
  if (transferSolBtn) on(transferSolBtn, "click", handleConsolidateSOL);
  if (transferTokensBtn) on(transferTokensBtn, "click", handleConsolidateTokens);
  if (cleanupBtn) on(cleanupBtn, "click", handleConsolidateCleanup);

  // Load initial data
  loadConsolidationData();
}

async function loadConsolidationData() {
  const container = $("#wc-wallets-container");
  const refreshBtn = $("#wc-refresh-btn");

  if (!container) return;

  if (refreshBtn) {
    refreshBtn.disabled = true;
    refreshBtn.innerHTML = '<i class="icon-loader spin"></i>';
  }

  container.innerHTML = `
    <div class="loading-state">
      <i class="icon-loader spin"></i>
      <p>Loading wallet data...</p>
    </div>
  `;

  try {
    const response = await fetch("/api/tools/wallets/summary");
    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || `HTTP ${response.status}`);
    }

    const data = await response.json();
    consolidationState.wallets = data.wallets || [];

    // Update summary
    const walletCount = $("#wc-wallet-count");
    const totalSol = $("#wc-total-sol");
    const totalTokens = $("#wc-total-tokens");
    const reclaimable = $("#wc-reclaimable");

    if (walletCount) walletCount.textContent = data.wallet_count || 0;
    if (totalSol) totalSol.textContent = Utils.formatSol(data.total_sol || 0);
    if (totalTokens) totalTokens.textContent = data.token_types || 0;
    if (reclaimable) reclaimable.textContent = `~${Utils.formatSol(data.reclaimable_rent || 0)}`;

    // Render wallets table
    if (consolidationState.wallets.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <i class="icon-wallet"></i>
          <p>No sub-wallets found</p>
          <small>Create sub-wallets using Multi-Buy to get started</small>
        </div>
      `;
      return;
    }

    container.innerHTML = `
      <table class="wc-wallet-table">
        <thead>
          <tr>
            <th><input type="checkbox" id="wc-check-all" /></th>
            <th>Name</th>
            <th>Address</th>
            <th>SOL Balance</th>
            <th>Tokens</th>
            <th>Empty ATAs</th>
          </tr>
        </thead>
        <tbody>
          ${consolidationState.wallets
            .map(
              (w) => `
            <tr data-address="${w.address}" class="${w.sol_balance === 0 && w.token_count === 0 ? "empty-wallet" : ""}">
              <td><input type="checkbox" class="wc-wallet-check" data-address="${w.address}" /></td>
              <td>${Utils.escapeHtml(w.name)}</td>
              <td class="mono">${Utils.formatAddressCompact(w.address)}</td>
              <td class="mono">${Utils.formatSol(w.sol_balance, { suffix: "" })}</td>
              <td class="mono">${w.token_count}</td>
              <td class="mono">${w.empty_atas}</td>
            </tr>
          `
            )
            .join("")}
        </tbody>
      </table>
    `;

    // Wire up checkboxes
    const checkAll = $("#wc-check-all");
    if (checkAll) {
      on(checkAll, "change", (e) => {
        const checks = $$(".wc-wallet-check");
        checks.forEach((c) => {
          c.checked = e.target.checked;
          if (e.target.checked) {
            consolidationState.selectedAddresses.add(c.dataset.address);
          } else {
            consolidationState.selectedAddresses.delete(c.dataset.address);
          }
        });
        updateConsolidationSelectionSummary();
      });
    }

    $$(".wc-wallet-check").forEach((check) => {
      on(check, "change", (e) => {
        if (e.target.checked) {
          consolidationState.selectedAddresses.add(e.target.dataset.address);
        } else {
          consolidationState.selectedAddresses.delete(e.target.dataset.address);
        }
        updateConsolidationSelectionSummary();
      });
    });

    updateConsolidationSelectionSummary();
  } catch (error) {
    console.error("Failed to load consolidation data:", error);
    container.innerHTML = `
      <div class="error-state">
        <i class="icon-circle-alert"></i>
        <p>Failed to load: ${error.message}</p>
      </div>
    `;
  } finally {
    if (refreshBtn) {
      refreshBtn.disabled = false;
      refreshBtn.innerHTML = '<i class="icon-refresh-cw"></i> Refresh';
    }
  }
}

function updateConsolidationSelectionSummary() {
  const summary = $("#wc-selection-summary");
  const transferSolBtn = $("#wc-transfer-sol-btn");
  const transferTokensBtn = $("#wc-transfer-tokens-btn");
  const cleanupBtn = $("#wc-cleanup-btn");

  const selectedCount = consolidationState.selectedAddresses.size;

  // Calculate totals for selected wallets
  let totalSol = 0;
  let totalTokens = 0;
  let totalEmptyAtas = 0;

  consolidationState.wallets
    .filter((w) => consolidationState.selectedAddresses.has(w.address))
    .forEach((w) => {
      totalSol += w.sol_balance || 0;
      totalTokens += w.token_count || 0;
      totalEmptyAtas += w.empty_atas || 0;
    });

  if (summary) {
    if (selectedCount === 0) {
      summary.innerHTML = "<span class=\"text-muted\">Select wallets to consolidate</span>";
    } else {
      summary.innerHTML = `
        <span class="text-primary">Selected: ${selectedCount} wallet${selectedCount > 1 ? "s" : ""}</span>
        <span class="text-secondary">| ${Utils.formatSol(totalSol)} | ${totalTokens} tokens | ${totalEmptyAtas} empty ATAs</span>
      `;
    }
  }

  const hasSelection = selectedCount > 0;
  if (transferSolBtn) transferSolBtn.disabled = !hasSelection || totalSol <= 0;
  if (transferTokensBtn) transferTokensBtn.disabled = !hasSelection || totalTokens <= 0;
  if (cleanupBtn) cleanupBtn.disabled = !hasSelection || totalEmptyAtas <= 0;
}

async function handleConsolidateSOL() {
  const addresses = Array.from(consolidationState.selectedAddresses);
  if (addresses.length === 0) return;

  const transferSolBtn = $("#wc-transfer-sol-btn");
  if (!transferSolBtn) return;

  transferSolBtn.disabled = true;
  transferSolBtn.innerHTML = '<i class="icon-loader spin"></i> Transferring...';

  try {
    const response = await fetch("/api/tools/wallets/consolidate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ wallets: addresses, type: "sol" }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || `HTTP ${response.status}`);
    }

    const result = await response.json();
    Utils.showToast(
      `Transferred ${Utils.formatSol(result.total_transferred)} to main wallet`,
      "success"
    );
    loadConsolidationData();
  } catch (error) {
    console.error("SOL consolidation failed:", error);
    Utils.showToast(`Transfer failed: ${error.message}`, "error");
  } finally {
    transferSolBtn.disabled = false;
    transferSolBtn.innerHTML = '<i class="icon-arrow-right"></i> Transfer SOL';
  }
}

async function handleConsolidateTokens() {
  const addresses = Array.from(consolidationState.selectedAddresses);
  if (addresses.length === 0) return;

  const transferTokensBtn = $("#wc-transfer-tokens-btn");
  if (!transferTokensBtn) return;

  transferTokensBtn.disabled = true;
  transferTokensBtn.innerHTML = '<i class="icon-loader spin"></i> Transferring...';

  try {
    const response = await fetch("/api/tools/wallets/consolidate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ wallets: addresses, type: "tokens" }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || `HTTP ${response.status}`);
    }

    const result = await response.json();
    Utils.showToast(`Transferred ${result.tokens_transferred} tokens to main wallet`, "success");
    loadConsolidationData();
  } catch (error) {
    console.error("Token consolidation failed:", error);
    Utils.showToast(`Transfer failed: ${error.message}`, "error");
  } finally {
    transferTokensBtn.disabled = false;
    transferTokensBtn.innerHTML = '<i class="icon-send"></i> Transfer All Tokens';
  }
}

async function handleConsolidateCleanup() {
  const addresses = Array.from(consolidationState.selectedAddresses);
  if (addresses.length === 0) return;

  const cleanupBtn = $("#wc-cleanup-btn");
  if (!cleanupBtn) return;

  cleanupBtn.disabled = true;
  cleanupBtn.innerHTML = '<i class="icon-loader spin"></i> Cleaning...';

  try {
    const response = await fetch("/api/tools/wallets/cleanup-atas", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ wallets: addresses }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || `HTTP ${response.status}`);
    }

    const result = await response.json();
    Utils.showToast(
      `Closed ${result.atas_closed} ATAs, reclaimed ${Utils.formatSol(result.sol_reclaimed)}`,
      "success"
    );
    loadConsolidationData();
  } catch (error) {
    console.error("ATA cleanup failed:", error);
    Utils.showToast(`Cleanup failed: ${error.message}`, "error");
  } finally {
    cleanupBtn.disabled = false;
    cleanupBtn.innerHTML = '<i class="icon-trash-2"></i> Cleanup ATAs';
  }
}

function renderAirdropCheckerTool(container, actionsContainer) {
  container.innerHTML = `
    <div class="tool-panel airdrop-checker-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-info"></i> About</h3>
        </div>
        <div class="section-content">
          <p class="tool-info">
            Check for pending airdrops, claimable rewards, and unclaimed allocations 
            across popular Solana protocols.
          </p>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-gift"></i> Available Airdrops</h3>
        </div>
        <div class="section-content">
          <div class="airdrop-list" id="airdrop-list">
            <div class="empty-state">
              <i class="icon-scan"></i>
              <p>Click "Check Airdrops" to scan for available claims</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;

  actionsContainer.innerHTML = `
    <button class="btn primary" id="check-airdrops-btn">
      <i class="icon-scan"></i> Check Airdrops
    </button>
    <button class="btn success" id="claim-all-btn" disabled>
      <i class="icon-gift"></i> Claim All
    </button>
  `;

  // TODO: Wire up airdrop checker functionality
}

function renderWalletGeneratorTool(container, actionsContainer) {
  const hint = Hints.getHint("tools.walletGenerator");
  const hintHtml = hint ? HintTrigger.render(hint, "tools.walletGenerator", { size: "sm" }) : "";

  container.innerHTML = `
    <div class="tool-panel wallet-generator-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-settings"></i> Generator Options</h3>
          ${hintHtml}
        </div>
        <div class="section-content">
          <div class="warning-box">
            <p><strong>Store your private keys securely!</strong></p>
            <p>Generated keypairs are created locally and never transmitted. 
               Always backup your keys in a secure location.</p>
          </div>
          <form class="tool-form" id="generator-form">
            <div class="form-group">
              <label for="wallet-count">Number of Wallets</label>
              <input type="number" id="wallet-count" value="1" min="1" max="10" />
            </div>
            <div class="form-group checkbox-group">
              <label>
                <input type="checkbox" id="vanity-enabled" />
                Vanity Address (starts with specific characters)
              </label>
            </div>
            <div class="form-group" id="vanity-prefix-group" style="display: none;">
              <label for="vanity-prefix">Prefix</label>
              <input type="text" id="vanity-prefix" placeholder="e.g., SOL" maxlength="4" />
              <small>Longer prefixes take exponentially longer to generate</small>
            </div>
          </form>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-key"></i> Generated Wallets</h3>
        </div>
        <div class="section-content">
          <div class="generated-wallets" id="generated-wallets">
            <div class="empty-state">
              <i class="icon-key"></i>
              <p>No wallets generated yet</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;

  HintTrigger.initAll();

  actionsContainer.innerHTML = `
    <button class="btn primary" id="generate-wallet-btn">
      <i class="icon-plus"></i> Generate
    </button>
    <button class="btn" id="export-wallets-btn" disabled>
      <i class="icon-download"></i> Export
    </button>
  `;

  // Wire up vanity checkbox toggle
  const vanityCheckbox = $("#vanity-enabled");
  const vanityGroup = $("#vanity-prefix-group");
  if (vanityCheckbox && vanityGroup) {
    on(vanityCheckbox, "change", () => {
      vanityGroup.style.display = vanityCheckbox.checked ? "block" : "none";
    });
  }

  // Wire up wallet generation functionality
  const generateBtn = $("#generate-wallet-btn");
  const exportBtn = $("#export-wallets-btn");

  if (generateBtn) {
    on(generateBtn, "click", handleGenerateWallets);
  }
  if (exportBtn) {
    on(exportBtn, "click", handleExportGeneratedWallets);
  }
}

// State for generated wallets
let generatedWallets = [];

/**
 * Handle wallet generation
 */
async function handleGenerateWallets() {
  const generateBtn = $("#generate-wallet-btn");
  const countInput = $("#wallet-count");
  const walletsContainer = $("#generated-wallets");

  if (!generateBtn || !walletsContainer) return;

  const count = parseInt(countInput?.value || "1", 10);
  if (count < 1 || count > 10) {
    Utils.showToast("Please enter a number between 1 and 10", "error");
    return;
  }

  generateBtn.disabled = true;
  generateBtn.innerHTML = '<i class="icon-loader spin"></i> Generating...';

  try {
    const response = await fetch("/api/tools/generate-keypairs", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ count }),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    const result = await response.json();

    // Check for error response
    if (result.error) {
      throw new Error(result.error);
    }

    // The API returns { success: true, data: [...] }
    const keypairs = result.data || result;

    if (!Array.isArray(keypairs) || keypairs.length === 0) {
      throw new Error("No keypairs returned");
    }

    // Add to our generated wallets list
    generatedWallets = [...generatedWallets, ...keypairs];

    // Render the wallets
    renderGeneratedWallets(walletsContainer);

    // Enable export button
    const exportBtn = $("#export-wallets-btn");
    if (exportBtn) exportBtn.disabled = false;

    Utils.showToast(
      `Generated ${keypairs.length} wallet${keypairs.length > 1 ? "s" : ""}`,
      "success"
    );
  } catch (error) {
    console.error("Failed to generate wallets:", error);
    Utils.showToast(`Failed to generate wallets: ${error.message}`, "error");
  } finally {
    generateBtn.disabled = false;
    generateBtn.innerHTML = '<i class="icon-plus"></i> Generate';
  }
}

/**
 * Render generated wallets list
 */
function renderGeneratedWallets(container) {
  if (!container) return;

  if (generatedWallets.length === 0) {
    container.innerHTML = `
      <div class="empty-state">
        <i class="icon-key"></i>
        <p>No wallets generated yet</p>
      </div>
    `;
    return;
  }

  container.innerHTML = generatedWallets
    .map(
      (wallet, index) => `
      <div class="generated-wallet-item">
        <div class="wallet-header">
          <span class="wallet-index">#${index + 1}</span>
          <div class="wallet-actions">
            <button class="btn-icon" data-action="copy-pubkey" data-pubkey="${wallet.pubkey}" title="Copy public key">
              <i class="icon-copy"></i>
            </button>
            <button class="btn-icon" data-action="copy-secret" data-secret="${wallet.secret}" title="Copy private key">
              <i class="icon-key"></i>
            </button>
            <button class="btn-icon danger" data-action="remove" data-index="${index}" title="Remove from list">
              <i class="icon-x"></i>
            </button>
          </div>
        </div>
        <div class="wallet-pubkey">
          <span class="label">Public Key:</span>
          <code class="pubkey-value">${wallet.pubkey}</code>
        </div>
        <div class="wallet-secret">
          <span class="label">Private Key:</span>
          <code class="secret-value masked">••••••••••••••••</code>
          <button class="btn-icon btn-reveal" data-action="reveal" data-secret="${wallet.secret}" title="Reveal private key">
            <i class="icon-eye"></i>
          </button>
        </div>
      </div>
    `
    )
    .join("");

  // Wire up action buttons
  container.querySelectorAll("[data-action]").forEach((btn) => {
    on(btn, "click", handleWalletAction);
  });
}

/**
 * Handle wallet action buttons (copy, reveal, remove)
 */
function handleWalletAction(event) {
  const btn = event.currentTarget;
  const action = btn.dataset.action;

  switch (action) {
    case "copy-pubkey": {
      const pubkey = btn.dataset.pubkey;
      Utils.copyToClipboard(pubkey);
      Utils.showToast("Public key copied to clipboard", "success");
      break;
    }
    case "copy-secret": {
      const secret = btn.dataset.secret;
      Utils.copyToClipboard(secret);
      Utils.showToast("Private key copied to clipboard (keep it safe!)", "warning");
      break;
    }
    case "reveal": {
      const secret = btn.dataset.secret;
      const secretEl = btn.parentElement.querySelector(".secret-value");
      if (secretEl) {
        const isRevealed = !secretEl.classList.contains("masked");
        if (isRevealed) {
          secretEl.textContent = "••••••••••••••••";
          secretEl.classList.add("masked");
          btn.innerHTML = '<i class="icon-eye"></i>';
        } else {
          secretEl.textContent = secret;
          secretEl.classList.remove("masked");
          btn.innerHTML = '<i class="icon-eye-off"></i>';
        }
      }
      break;
    }
    case "remove": {
      const index = parseInt(btn.dataset.index, 10);
      generatedWallets.splice(index, 1);
      const container = $("#generated-wallets");
      renderGeneratedWallets(container);

      // Disable export if no wallets left
      if (generatedWallets.length === 0) {
        const exportBtn = $("#export-wallets-btn");
        if (exportBtn) exportBtn.disabled = true;
      }
      break;
    }
  }
}

/**
 * Export generated wallets as JSON file
 */
function handleExportGeneratedWallets() {
  if (generatedWallets.length === 0) {
    Utils.showToast("No wallets to export", "warning");
    return;
  }

  const exportData = generatedWallets.map((wallet, index) => ({
    index: index + 1,
    pubkey: wallet.pubkey,
    secret: wallet.secret,
  }));

  const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `solana-wallets-${new Date().toISOString().slice(0, 10)}.json`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);

  Utils.showToast("Wallets exported - store securely!", "success");
}

// =============================================================================
// Tool Navigation
// =============================================================================

function selectTool(toolId) {
  const definition = TOOL_DEFINITIONS[toolId];
  if (!definition) {
    console.warn(`Unknown tool: ${toolId}`);
    return;
  }

  currentTool = toolId;

  // Update sidebar active state - support both old and new class names
  const navItems = $$(".nav-item, .tool-item");
  navItems.forEach((item) => {
    if (item.dataset.tool === toolId) {
      item.classList.add("active");
    } else {
      item.classList.remove("active");
    }
  });

  // Update header
  const iconEl = $("#tool-icon");
  const titleEl = $("#tool-title");
  const descEl = $("#tool-description");
  const statusEl = $("#tool-status");

  if (iconEl) iconEl.innerHTML = `<i class="${definition.icon}"></i>`;
  if (titleEl) titleEl.textContent = definition.title;
  if (descEl) descEl.textContent = definition.description;

  // Update status badge based on tool's current status
  if (statusEl) {
    const navItem = $(`.nav-item[data-tool="${toolId}"]`);
    const status = navItem?.dataset.status || "ready";
    statusEl.className = `tool-status ${status}`;
    statusEl.textContent = getStatusLabel(status);
  }

  // Render tool content
  const contentEl = $("#tools-content");
  const actionsEl = $("#tool-actions");

  if (contentEl && actionsEl && definition.render) {
    contentEl.innerHTML = "";
    actionsEl.innerHTML = "";
    definition.render(contentEl, actionsEl);

    // Enhance any native select elements with custom styling
    enhanceAllSelects(contentEl);
  }

  // Save state
  saveToolState(toolId);
}

/**
 * Get human-readable status label
 */
function getStatusLabel(status) {
  const labels = {
    ready: "Ready",
    running: "Running",
    error: "Error",
    coming: "Coming Soon",
  };
  return labels[status] || "Ready";
}

/**
 * Update a tool's status indicator
 */
function updateToolStatus(toolId, status, tooltip = null) {
  const navItem = $(`.nav-item[data-tool="${toolId}"]`);
  if (navItem) {
    navItem.dataset.status = status;
    const statusIndicator = navItem.querySelector(".nav-item-status");
    if (statusIndicator && tooltip) {
      statusIndicator.dataset.tooltip = tooltip;
    }
  }

  // Update header if this is the current tool
  if (currentTool === toolId) {
    const statusEl = $("#tool-status");
    if (statusEl) {
      statusEl.className = `tool-status ${status}`;
      statusEl.textContent = getStatusLabel(status);
    }
  }
}

function saveToolState(toolId) {
  try {
    localStorage.setItem(TOOLS_STATE_KEY, JSON.stringify({ activeTool: toolId }));
  } catch (e) {
    console.warn("Failed to save tools state:", e);
  }
}

function loadToolState() {
  try {
    const saved = localStorage.getItem(TOOLS_STATE_KEY);
    if (saved) {
      const state = JSON.parse(saved);
      return state.activeTool || DEFAULT_TOOL;
    }
  } catch (e) {
    console.warn("Failed to load tools state:", e);
  }
  return DEFAULT_TOOL;
}

// =============================================================================
// Lifecycle
// =============================================================================

function createLifecycle() {
  return {
    async init() {
      // Initialize hints system
      await Hints.init();

      // Fetch feature status from API and apply to UI
      featureStatus = await fetchFeatureStatus();
      applyFeatureStatusToUI();

      // Set up tool navigation click handler
      toolClickHandler = (event) => {
        const toolItem = event.target.closest(".nav-item, .tool-item");
        if (toolItem && toolItem.dataset.tool) {
          const toolId = toolItem.dataset.tool;
          const status = toolItem.dataset.status;

          // Handle non-available tools
          if (status === "coming") {
            Utils.showToast("This tool is coming soon!", "info");
            return;
          }
          if (status === "disabled") {
            Utils.showToast("This tool is currently disabled", "warning");
            return;
          }

          selectTool(toolId);
        }
      };

      const nav = $("#tools-nav");
      if (nav) {
        on(nav, "click", toolClickHandler);
      }

      // Set up help button handler
      const helpBtn = $("#tool-help-btn");
      if (helpBtn) {
        on(helpBtn, "click", showToolHelp);
      }

      // Load saved state or default
      const savedTool = loadToolState();
      selectTool(savedTool);
    },

    activate() {
      // Refresh current tool if needed
      if (currentTool) {
        const definition = TOOL_DEFINITIONS[currentTool];
        if (definition && definition.onActivate) {
          definition.onActivate();
        }
      }
    },

    deactivate() {
      // Pause any active operations
    },

    dispose() {
      // Clean up event listeners
      const nav = $("#tools-nav");
      if (nav && toolClickHandler) {
        off(nav, "click", toolClickHandler);
      }
      toolClickHandler = null;
      currentTool = null;
      featureStatus = {}; // Reset feature status

      // Clean up Volume Aggregator resources
      stopVolumeAggregatorPolling();
      if (vaHistoryTable) {
        vaHistoryTable.dispose();
        vaHistoryTable = null;
      }
      if (vaToolFavorites) {
        vaToolFavorites.dispose();
        vaToolFavorites = null;
      }

      // Clean up Multi-Buy resources
      stopMultiBuyPolling();
      resetMultiBuyUI();

      // Clean up Multi-Sell resources
      stopMultiSellPolling();
      resetMultiSellUI();
    },
  };
}

/**
 * Show help/documentation for current tool using hint popover
 */
function showToolHelp() {
  if (!currentTool) return;

  // Map tool IDs to hint paths
  const hintPathMap = {
    "wallet-cleanup": "tools.walletCleanup",
    "burn-tokens": "tools.burnTokens",
    "wallet-generator": "tools.walletGenerator",
    "volume-aggregator": "tools.volumeAggregator",
    "buy-multi-wallets": "tools.multiBuy",
    "sell-multi-wallets": "tools.multiSell",
    "wallet-consolidation": "tools.walletConsolidation",
  };

  const hintPath = hintPathMap[currentTool];
  if (!hintPath) {
    // Fallback for tools without hints yet
    const definition = TOOL_DEFINITIONS[currentTool];
    Utils.showToast(definition?.description || "Help not available", "info");
    return;
  }

  const hint = Hints.getHint(hintPath);
  if (!hint) {
    Utils.showToast("Help not available", "info");
    return;
  }

  // Find or create a trigger element for the popover
  const helpBtn = $("#tool-help-btn");
  if (helpBtn) {
    // Simulate a click on the hint trigger by creating a temporary one
    import("../ui/hint_popover.js").then(({ HintPopover }) => {
      const popover = new HintPopover(hint, helpBtn);
      popover.show();
    });
  }
}

// Register the page
registerPage("tools", createLifecycle());

export { createLifecycle };
