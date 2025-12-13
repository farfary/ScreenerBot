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

// =============================================================================
// Constants
// =============================================================================

const TOOLS_STATE_KEY = "tools.page";
const DEFAULT_TOOL = "wallet-cleanup";

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
    icon: "icon-plus-circle",
    category: "token",
    render: renderCreateTokenTool,
  },
  "token-watch": {
    id: "token-watch",
    title: "Token Watch",
    description: "Monitor token activity and price movements",
    icon: "icon-eye",
    category: "token",
    render: renderTokenWatchTool,
  },
  "volume-aggregator": {
    id: "volume-aggregator",
    title: "Volume Aggregator",
    description: "Generate trading volume using multiple wallets",
    icon: "icon-bar-chart-2",
    category: "trading",
    render: renderVolumeAggregatorTool,
  },
  "buy-multi-wallets": {
    id: "buy-multi-wallets",
    title: "Buy Multi Wallets",
    description: "Execute coordinated buy orders across multiple wallets",
    icon: "icon-shopping-cart",
    category: "trading",
    render: renderBuyMultiWalletsTool,
  },
  "sell-multi-wallets": {
    id: "sell-multi-wallets",
    title: "Sell Multi Wallets",
    description: "Execute coordinated sell orders across multiple wallets",
    icon: "icon-package",
    category: "trading",
    render: renderSellMultiWalletsTool,
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
    enabled ? "Auto cleanup enabled - empty ATAs will be closed automatically" : "Auto cleanup disabled",
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
          <i class="icon-check-circle"></i>
          <p>Found ${stats.empty_count} empty ATAs worth ~${Utils.formatSol(stats.reclaimable_sol || 0)} SOL</p>
        </div>
      `;
      if (cleanupBtn) cleanupBtn.disabled = false;
    } else {
      listEl.innerHTML = `
        <div class="empty-state">
          <i class="icon-check-circle"></i>
          <p>No empty ATAs found - wallet is clean!</p>
        </div>
      `;
    }
  } catch (error) {
    console.error("ATA scan failed:", error);
    listEl.innerHTML = `
      <div class="error-state">
        <i class="icon-alert-circle"></i>
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

function renderBurnTokensTool(container, actionsContainer) {
  const hint = Hints.getHint("tools.burnTokens");
  const hintHtml = hint ? HintTrigger.render(hint, "tools.burnTokens", { size: "sm" }) : "";

  container.innerHTML = `
    <div class="tool-panel burn-tokens-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-coins"></i> Select Tokens to Burn</h3>
          ${hintHtml}
        </div>
        <div class="section-content">
          <div class="warning-box">
            <p><strong>Burning tokens is irreversible!</strong></p>
            <p>Burned tokens are permanently destroyed and cannot be recovered.</p>
          </div>
          <div class="token-select-list" id="burn-token-list">
            <div class="empty-state">
              <i class="icon-loader spin"></i>
              <p>Loading tokens...</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;

  HintTrigger.initAll();

  actionsContainer.innerHTML = `
    <button class="btn danger" id="burn-selected-btn" disabled>
      <i class="icon-flame"></i> Burn Selected
    </button>
  `;

  // TODO: Load tokens and wire up burn functionality
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
      <i class="icon-plus-circle"></i> Create Token
    </button>
  `;

  // TODO: Wire up token creation functionality
}

function renderTokenWatchTool(container, actionsContainer) {
  container.innerHTML = `
    <div class="tool-panel token-watch-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-plus"></i> Add Token to Watch</h3>
        </div>
        <div class="section-content">
          <div class="input-group">
            <input type="text" id="watch-token-input" placeholder="Enter token mint address..." />
            <button class="btn primary" id="add-watch-btn">
              <i class="icon-plus"></i> Add
            </button>
          </div>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-list"></i> Watched Tokens</h3>
        </div>
        <div class="section-content">
          <div class="watched-tokens-list" id="watched-tokens-list">
            <div class="empty-state">
              <i class="icon-eye-off"></i>
              <p>No tokens being watched</p>
              <small>Add a token above to start monitoring</small>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;

  actionsContainer.innerHTML = `
    <button class="btn" id="refresh-watch-btn">
      <i class="icon-refresh-cw"></i> Refresh All
    </button>
    <button class="btn danger" id="clear-watch-btn">
      <i class="icon-trash-2"></i> Clear All
    </button>
  `;

  // TODO: Wire up token watch functionality
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
        <i class="icon-alert-circle"></i>
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
    <div class="ta-header-right">
      ${priceSol ? `<div class="ta-price-sol">${Utils.formatSol(priceSol)} SOL</div>` : ""}
      ${priceUsd ? `<div class="ta-price-usd">${Utils.formatCurrencyUSD(priceUsd)}</div>` : ""}
    </div>
  `;
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
          ${overview.description ? `
          <div class="ta-info-item ta-full-width">
            <span class="ta-info-label">Description</span>
            <span class="ta-info-value">${escapeHtml(overview.description)}</span>
          </div>
          ` : ""}
          ${overview.supply ? `
          <div class="ta-info-item">
            <span class="ta-info-label">Supply</span>
            <span class="ta-info-value mono">${escapeHtml(overview.supply)}</span>
          </div>
          ` : ""}
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
      ${security.top_holders_pct ? `
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
      ` : ""}

      <!-- Risks Card -->
      ${security.risks && security.risks.length > 0 ? `
      <div class="ta-card ta-full-width">
        <div class="ta-card-title">
          <i class="icon-alert-triangle"></i> Security Risks (${security.risks.length})
        </div>
        <div class="ta-risk-list">
          ${security.risks.map((risk) => `
            <div class="ta-risk-item ${risk.level.toLowerCase()}">
              <span class="ta-risk-level">${escapeHtml(risk.level)}</span>
              <span class="ta-risk-name">${escapeHtml(risk.name)}</span>
              <span class="ta-risk-desc">${escapeHtml(risk.description)}</span>
            </div>
          `).join("")}
        </div>
      </div>
      ` : `
      <div class="ta-card ta-full-width">
        <div class="ta-card-title">
          <i class="icon-check-circle"></i> Security Risks
        </div>
        <div class="ta-no-risks">
          <i class="icon-shield-check"></i>
          <p>No security risks detected</p>
        </div>
      </div>
      `}
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
          <i class="icon-bar-chart-2"></i> Trading Volume
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
          <i class="icon-pie-chart"></i> Valuation
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
              ${liquidity.pools.map((pool) => `
                <tr class="${pool.is_canonical ? "canonical" : ""}">
                  <td class="dex">${escapeHtml(pool.dex)}</td>
                  <td class="address mono">${escapeHtml(pool.address.slice(0, 8))}...${escapeHtml(pool.address.slice(-6))}</td>
                  <td class="liquidity">${Utils.formatSol(pool.liquidity_sol)}</td>
                  <td class="status">${pool.is_canonical ? '<span class="canonical-badge">Primary</span>' : ""}</td>
                </tr>
              `).join("")}
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
 */
function getTaScoreClass(score) {
  if (score >= 70) return "success";
  if (score >= 40) return "warning";
  return "danger";
}

/**
 * Helper: Get label for security score
 */
function getTaScoreLabel(score) {
  if (!score) return "Unknown";
  if (score >= 70) return "Good";
  if (score >= 40) return "Moderate";
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
                  <select id="va-num-wallets" aria-describedby="va-num-wallets-hint">
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
                <i class="icon-bar-chart-2"></i>
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
            <h4><i class="icon-bar-chart"></i> Session Analytics</h4>
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
        <i class="icon-alert-circle"></i>
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

function renderBuyMultiWalletsTool(container, actionsContainer) {
  container.innerHTML = `
    <div class="tool-panel multi-wallet-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-alert-triangle"></i> Advanced Feature</h3>
        </div>
        <div class="section-content">
          <div class="warning-box">
            <p>Multi-wallet trading requires additional wallet setup and carries higher risk.</p>
          </div>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-wallet"></i> Wallets</h3>
        </div>
        <div class="section-content">
          <div class="wallet-list" id="buy-wallet-list">
            <div class="empty-state">
              <i class="icon-wallet"></i>
              <p>No additional wallets configured</p>
              <small>Configure wallets in Settings</small>
            </div>
          </div>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-settings"></i> Buy Configuration</h3>
        </div>
        <div class="section-content">
          <form class="tool-form" id="multi-buy-form">
            <div class="form-group">
              <label for="buy-token-mint">Token Mint</label>
              <input type="text" id="buy-token-mint" placeholder="Token address..." />
            </div>
            <div class="form-group">
              <label for="buy-amount-per-wallet">Amount per Wallet (SOL)</label>
              <input type="number" id="buy-amount-per-wallet" placeholder="0.1" step="0.001" min="0" />
            </div>
            <div class="form-group">
              <label for="buy-slippage">Slippage (%)</label>
              <input type="number" id="buy-slippage" value="1" step="0.1" min="0.1" max="50" />
            </div>
          </form>
        </div>
      </div>
    </div>
  `;

  actionsContainer.innerHTML = `
    <button class="btn" id="simulate-buy-btn" disabled>
      <i class="icon-play"></i> Simulate
    </button>
    <button class="btn success" id="execute-buy-btn" disabled>
      <i class="icon-shopping-cart"></i> Execute Buy
    </button>
  `;

  // TODO: Wire up multi-wallet buy functionality
}

function renderSellMultiWalletsTool(container, actionsContainer) {
  container.innerHTML = `
    <div class="tool-panel multi-wallet-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-alert-triangle"></i> Advanced Feature</h3>
        </div>
        <div class="section-content">
          <div class="warning-box">
            <p>Multi-wallet trading requires additional wallet setup and carries higher risk.</p>
          </div>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-wallet"></i> Wallets</h3>
        </div>
        <div class="section-content">
          <div class="wallet-list" id="sell-wallet-list">
            <div class="empty-state">
              <i class="icon-wallet"></i>
              <p>No additional wallets configured</p>
              <small>Configure wallets in Settings</small>
            </div>
          </div>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-settings"></i> Sell Configuration</h3>
        </div>
        <div class="section-content">
          <form class="tool-form" id="multi-sell-form">
            <div class="form-group">
              <label for="sell-token-mint">Token Mint</label>
              <input type="text" id="sell-token-mint" placeholder="Token address..." />
            </div>
            <div class="form-group">
              <label for="sell-percentage">Sell Percentage</label>
              <input type="number" id="sell-percentage" value="100" step="1" min="1" max="100" />
            </div>
            <div class="form-group">
              <label for="sell-slippage">Slippage (%)</label>
              <input type="number" id="sell-slippage" value="1" step="0.1" min="0.1" max="50" />
            </div>
          </form>
        </div>
      </div>
    </div>
  `;

  actionsContainer.innerHTML = `
    <button class="btn" id="simulate-sell-btn" disabled>
      <i class="icon-play"></i> Simulate
    </button>
    <button class="btn danger" id="execute-sell-btn" disabled>
      <i class="icon-package"></i> Execute Sell
    </button>
  `;

  // TODO: Wire up multi-wallet sell functionality
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

  // TODO: Wire up wallet generation functionality
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

      // Set up tool navigation click handler
      toolClickHandler = (event) => {
        const toolItem = event.target.closest(".nav-item, .tool-item");
        if (toolItem && toolItem.dataset.tool) {
          // Check if tool is coming soon
          if (toolItem.dataset.status === "coming") {
            Utils.showToast("This tool is coming soon!", "info");
            return;
          }
          selectTool(toolItem.dataset.tool);
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
