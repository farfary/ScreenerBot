/**
 * Token Tools Module
 * Contains token-related utilities: create token, token watch, and token analyzer
 */

import { $, $$, on } from "../../core/dom.js";
import * as Utils from "../../core/utils.js";
import { ConfirmationDialog } from "../../ui/confirmation_dialog.js";

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
              <label class="toggle">
                <input type="checkbox" id="hw-enabled" ${config.enabled ? "checked" : ""}>
                <span class="toggle-track"></span>
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
              <label class="toggle">
                <input type="checkbox" id="hw-notify-new" ${config.notify_new_holders ? "checked" : ""}>
                <span class="toggle-track"></span>
              </label>
            </div>
            <div class="hw-form-group hw-toggle-group">
              <label for="hw-notify-drop">Notify on Holder Drop</label>
              <label class="toggle">
                <input type="checkbox" id="hw-notify-drop" ${config.notify_holder_drop ? "checked" : ""}>
                <span class="toggle-track"></span>
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

  const result = await ConfirmationDialog.show({
    title: "Blacklist Token",
    message: `Blacklist ${symbol || mint}? This token will be excluded from trading.`,
    confirmLabel: "Blacklist",
    variant: "warning",
  });
  if (!result.confirmed) {
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
// Exports
// =============================================================================

export {
  renderCreateTokenTool,
  renderTokenWatchTool,
  renderTokenAnalyzerTool,
};
