// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Token Tools Module
 * Contains token-related utilities: create token, token watch, and token analyzer
 */

import { $, $$, on, logoFallbackAttr } from "../../core/dom.js";
import {
  formatBooleanFlag,
  formatCurrencyUSD,
  formatFixed,
  formatNumber,
  formatPercentValue,
  withSolUnit,
} from "../../core/format.js";
import * as Utils from "../../core/utils.js";
import { ConfirmationDialog } from "../../ui/confirmation_dialog.js";
import { apiErrorMessage } from "../../core/request_manager.js";
import { RISK_SEVERITY_LABELS } from "../../ui/risk_severity.js";
import { venueLabel } from "../../ui/venue.js";
import { rugcheckRiskDescription, rugcheckRiskName } from "../../ui/rugcheck_risk.js";

// Ids are the bands of getTaScoreBand.
const SCORE_BAND_LABELS = Object.freeze({
  good: "tools-analyzer-score-good",
  moderate: "tools-analyzer-score-moderate",
  risky: "tools-analyzer-score-risky",
  unknown: "format-unknown",
});

// Authority state by whether the authority is still set.
const AUTHORITY_STATE_LABELS = Object.freeze({
  active: "tools-analyzer-authority-active",
  revoked: "tools-analyzer-authority-revoked",
});

function renderCreateTokenTool(container, actionsContainer) {
  container.innerHTML = `
    <div class="tool-panel create-token-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-file-plus"></i> <span data-l10n-id="tools-create-token-details-title"></span></h3>
        </div>
        <div class="section-content">
          <form class="tool-form" id="create-token-form">
            <div class="form-group">
              <label for="token-name" data-l10n-id="tools-create-token-name-label"></label>
              <input type="text" id="token-name" data-l10n-id="tools-create-token-name-input" maxlength="32" />
            </div>
            <div class="form-group">
              <label for="token-symbol" data-l10n-id="tools-create-token-symbol-label"></label>
              <input type="text" id="token-symbol" data-l10n-id="tools-create-token-symbol-input" maxlength="10" />
            </div>
            <div class="form-group">
              <label for="token-decimals" data-l10n-id="tools-create-token-decimals-label"></label>
              <input type="number" id="token-decimals" value="9" min="0" max="9" />
            </div>
            <div class="form-group">
              <label for="token-supply" data-l10n-id="tools-create-token-supply-label"></label>
              <input type="number" id="token-supply" placeholder="1000000000" min="1" />
            </div>
            <div class="form-group">
              <label for="token-description" data-l10n-id="tools-create-token-description-label"></label>
              <textarea id="token-description" data-l10n-id="tools-create-token-description-input" rows="3"></textarea>
            </div>
          </form>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-image"></i> <span data-l10n-id="tools-create-token-image-title"></span></h3>
        </div>
        <div class="section-content">
          <div class="image-upload-area" id="token-image-upload">
            <i class="icon-upload"></i>
            <p data-l10n-id="tools-create-token-image-drop"></p>
            <small data-l10n-id="tools-create-token-image-hint"></small>
          </div>
        </div>
      </div>
    </div>
  `;

  actionsContainer.innerHTML = `
    <button class="btn" id="preview-token-btn">
      <i class="icon-eye"></i> <span data-l10n-id="tools-create-token-action-preview"></span>
    </button>
    <button class="btn primary" id="create-token-btn">
      <i class="icon-circle-plus"></i> <span data-l10n-id="tools-create-token-action-create"></span>
    </button>
  `;
  I18n.localizeTree(container);
  I18n.localizeTree(actionsContainer);

  // TODO: Wire up token creation functionality
}

function renderTokenWatchTool(container, actionsContainer) {
  // Load holder watch config and render UI
  container.innerHTML = `
    <div class="tool-panel holder-watch-tool">
      <div class="hw-loading">
        <i class="icon-loader spin"></i>
        <p data-l10n-id="tools-holder-watch-loading"></p>
      </div>
    </div>
  `;
  I18n.localizeTree(container);

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
      Utils.showToast(I18n.t("tools-holder-watch-saved"), "success");
    } else {
      const errData = await res.json().catch(() => ({}));
      Utils.showToast(apiErrorMessage(errData, I18n.t("tools-holder-watch-save-failed")), "error");
    }
  } catch (e) {
    console.error("[HolderWatch] Save error:", e);
    Utils.showToast(I18n.t("tools-holder-watch-save-error"), "error");
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
          <h3><i class="icon-settings"></i> <span data-l10n-id="tools-holder-watch-settings-title"></span></h3>
        </div>
        <div class="section-content">
          <div class="hw-form-row">
            <div class="hw-form-group hw-toggle-group">
              <label for="hw-enabled" data-l10n-id="tools-holder-watch-enabled-label"></label>
              <label class="toggle">
                <input type="checkbox" id="hw-enabled" ${config.enabled ? "checked" : ""}>
                <span class="toggle-track"></span>
              </label>
            </div>
          </div>

          <div class="hw-form-row hw-two-cols">
            <div class="hw-form-group">
              <label for="hw-interval" data-l10n-id="tools-holder-watch-interval-label"></label>
              <input type="number" id="hw-interval" class="form-input" 
                value="${config.check_interval_secs || 60}" min="10" max="3600" step="10">
              <span class="hint" data-l10n-id="tools-holder-watch-interval-hint"></span>
            </div>
            <div class="hw-form-group">
              <label for="hw-max-tokens" data-l10n-id="tools-holder-watch-max-tokens-label"></label>
              <input type="number" id="hw-max-tokens" class="form-input" 
                value="${config.max_watched_tokens || 20}" min="1" max="100">
              <span class="hint" data-l10n-id="tools-holder-watch-max-tokens-hint"></span>
            </div>
          </div>

          <div class="hw-form-row hw-two-cols">
            <div class="hw-form-group hw-toggle-group">
              <label for="hw-notify-new" data-l10n-id="tools-holder-watch-notify-new-label"></label>
              <label class="toggle">
                <input type="checkbox" id="hw-notify-new" ${config.notify_new_holders ? "checked" : ""}>
                <span class="toggle-track"></span>
              </label>
            </div>
            <div class="hw-form-group hw-toggle-group">
              <label for="hw-notify-drop" data-l10n-id="tools-holder-watch-notify-drop-label"></label>
              <label class="toggle">
                <input type="checkbox" id="hw-notify-drop" ${config.notify_holder_drop ? "checked" : ""}>
                <span class="toggle-track"></span>
              </label>
            </div>
          </div>

          <div class="hw-form-row hw-two-cols">
            <div class="hw-form-group">
              <label for="hw-min-change" data-l10n-id="tools-holder-watch-min-change-label"></label>
              <input type="number" id="hw-min-change" class="form-input" 
                value="${config.min_holder_change || 5}" min="1" max="1000">
              <span class="hint" data-l10n-id="tools-holder-watch-min-change-hint"></span>
            </div>
            <div class="hw-form-group">
              <label for="hw-drop-percent" data-l10n-id="tools-holder-watch-drop-percent-label"></label>
              <input type="number" id="hw-drop-percent" class="form-input" 
                value="${config.holder_drop_percent || 10.0}" min="1" max="100" step="0.5">
              <span class="hint" data-l10n-id="tools-holder-watch-drop-percent-hint"></span>
            </div>
          </div>

          <div class="hw-form-actions">
            <button class="btn primary" id="hw-save-config">
              <i class="icon-save"></i> <span data-l10n-id="tools-holder-watch-action-save"></span>
            </button>
          </div>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-eye"></i> <span data-l10n-id="tools-holder-watch-tokens-title"></span></h3>
        </div>
        <div class="section-content">
          <div class="hw-add-token-group">
            <input type="text" id="hw-token-input" class="form-input" 
              data-l10n-id="tools-holder-watch-token-input">
            <button class="btn primary" id="hw-add-token">
              <i class="icon-plus"></i> <span data-l10n-id="common-action-add"></span>
            </button>
          </div>
          <div id="hw-token-list" class="hw-token-list">
            <div class="empty-state">
              <i class="icon-eye-off"></i>
              <p data-l10n-id="tools-holder-watch-empty"></p>
              <small data-l10n-id="tools-holder-watch-empty-hint"></small>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;
  I18n.localizeTree(container);

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
        Utils.showToast(I18n.t("tools-holder-watch-coming-soon"), "info");
        tokenInput.value = "";
      } else {
        Utils.showToast(I18n.t("tools-validation-mint-invalid"), "error");
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
      <i class="icon-refresh-cw"></i> <span data-l10n-id="common-action-refresh"></span>
    </button>
  `;
  I18n.localizeTree(actionsContainer);

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
let taAnalysisData = null;

function renderTokenAnalyzerTool(container, actionsContainer) {
  container.innerHTML = `
    <div class="tool-panel token-analyzer-tool">
      <!-- Token Input Section -->
      <div class="tool-section ta-input-section">
        <div class="section-header">
          <h3><i class="icon-search"></i> <span data-l10n-id="tools-analyzer-input-title"></span></h3>
        </div>
        <div class="section-content">
          <div class="ta-input-group">
            <input dir="ltr" type="text" id="ta-mint-input" data-l10n-id="tools-analyzer-mint-input" />
            <button class="btn primary" id="ta-analyze-btn">
              <i class="icon-search"></i> <span data-l10n-id="tools-analyzer-action-analyze"></span>
            </button>
          </div>
        </div>
      </div>

      <!-- Loading State -->
      <div id="ta-loading" class="ta-loading" style="display: none;">
        <i class="icon-loader spin"></i>
        <p data-l10n-id="tools-analyzer-loading"></p>
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
            <i class="icon-info"></i> <span data-l10n-id="tools-analyzer-tab-overview"></span>
          </button>
          <button class="ta-tab" data-tab="security">
            <i class="icon-shield"></i> <span data-l10n-id="tools-analyzer-tab-security"></span>
          </button>
          <button class="ta-tab" data-tab="market">
            <i class="icon-trending-up"></i> <span data-l10n-id="tools-analyzer-tab-market"></span>
          </button>
          <button class="ta-tab" data-tab="liquidity">
            <i class="icon-droplet"></i> <span data-l10n-id="tools-analyzer-tab-liquidity"></span>
          </button>
        </div>

        <!-- Tab Content -->
        <div class="ta-content" id="ta-content"></div>
      </div>

      <!-- Empty State -->
      <div id="ta-empty" class="ta-empty-state">
        <i class="icon-search"></i>
        <p data-l10n-id="tools-analyzer-empty"></p>
        <small data-l10n-id="tools-analyzer-empty-hint"></small>
      </div>
    </div>
  `;

  actionsContainer.innerHTML = `
    <button class="btn" id="ta-refresh-btn" disabled>
      <i class="icon-refresh-cw"></i> <span data-l10n-id="common-action-refresh"></span>
    </button>
    <button class="btn" id="ta-copy-btn" disabled>
      <i class="icon-copy"></i> <span data-l10n-id="tools-analyzer-action-copy-report"></span>
    </button>
  `;
  I18n.localizeTree(container);
  I18n.localizeTree(actionsContainer);

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
    Utils.showToast(I18n.t("tools-validation-mint-required"), "warning");
    return;
  }

  // Validate mint format (base58)
  if (!/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(mint)) {
    Utils.showToast(I18n.t("tools-validation-mint-format"), "error");
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
    analyzeBtn.innerHTML =
      '<i class="icon-loader spin"></i> <span data-l10n-id="tools-analyzer-action-analyzing"></span>';
    I18n.localizeTree(analyzeBtn);
  }

  try {
    const response = await fetch(`/api/tokens/${mint}/analysis`);
    const data = await response.json();

    if (!response.ok || !data.success) {
      throw new Error(apiErrorMessage(data, I18n.t("tools-analyzer-failed")));
    }

    // Store data
    taCurrentMint = mint;
    taAnalysisData = data;

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
        <button class="btn btn-sm" data-l10n-id="common-action-dismiss" onclick="document.getElementById('ta-error').style.display='none'; document.getElementById('ta-empty').style.display='flex';"></button>
      `;
      I18n.localizeTree(errorEl);
    }
    if (refreshBtn) refreshBtn.disabled = true;
    if (copyBtn) copyBtn.disabled = true;
  } finally {
    if (analyzeBtn) {
      analyzeBtn.disabled = false;
      analyzeBtn.innerHTML =
        '<i class="icon-search"></i> <span data-l10n-id="tools-analyzer-action-analyze"></span>';
      I18n.localizeTree(analyzeBtn);
    }
  }
}

/**
 * Render token header with logo, name, price
 */
function renderTaTokenHeader(overview) {
  const headerEl = $("#ta-token-header");
  if (!headerEl || !overview) return;

  const symbol = overview.symbol || I18n.t("format-unknown");
  const name = overview.name || I18n.t("tools-analyzer-unknown-token");
  const logoUrl = overview.logo_url || "";
  const priceSol = overview.price_sol;
  const priceUsd = overview.price_usd;
  const mint = overview.mint || taCurrentMint;

  headerEl.innerHTML = `
    <div class="ta-header-left">
      <div class="ta-logo token-logo-frame">
        ${logoUrl ? `<img class="token-logo-artwork" src="${escapeHtml(logoUrl)}" alt="${escapeHtml(symbol)}" ${logoFallbackAttr("ta-logo-placeholder")} />` : `<div class="ta-logo-placeholder">${escapeHtml(symbol.charAt(0))}</div>`}
      </div>
      <div class="ta-header-info">
        <span class="ta-symbol">${escapeHtml(symbol)}</span>
        <span class="ta-name">${escapeHtml(name)}</span>
      </div>
    </div>
    <div class="ta-header-center">
      <div class="ta-header-actions">
        <button class="btn btn-sm btn-icon action-favorite" data-mint="${escapeHtml(mint)}" data-symbol="${escapeHtml(symbol)}" data-name="${escapeHtml(name)}" data-logo="${escapeHtml(logoUrl)}" data-l10n-id="tools-analyzer-favorite-add">
          <i class="icon-star"></i>
        </button>
        <button class="btn btn-sm btn-icon action-blacklist" data-mint="${escapeHtml(mint)}" data-symbol="${escapeHtml(symbol)}" data-l10n-id="tools-analyzer-blacklist-add">
          <i class="icon-slash"></i>
        </button>
        <button class="btn btn-sm btn-icon action-copy-mint" data-mint="${escapeHtml(mint)}" data-l10n-id="links-copy-mint">
          <i class="icon-copy"></i>
        </button>
        <button class="btn btn-sm btn-icon action-open-dexscreener" data-mint="${escapeHtml(mint)}" data-l10n-id="links-view-dexscreener">
          <i class="icon-external-link"></i>
        </button>
      </div>
    </div>
    <div class="ta-header-right">
      ${priceSol ? `<div class="ta-price-sol">${Utils.formatSol(priceSol)}</div>` : ""}
      ${priceUsd ? `<div class="ta-price-usd">${Utils.formatCurrencyUSD(priceUsd)}</div>` : ""}
    </div>
  `;

  I18n.localizeTree(headerEl);

  // Attach event handlers for favorite and blacklist buttons
  const favoriteBtn = headerEl.querySelector(".action-favorite");
  const blacklistBtn = headerEl.querySelector(".action-blacklist");
  const copyMintBtn = headerEl.querySelector(".action-copy-mint");
  const dexscreenerBtn = headerEl.querySelector(".action-open-dexscreener");

  if (favoriteBtn) {
    on(favoriteBtn, "click", handleTaFavoriteClick);
  }
  if (blacklistBtn) {
    on(blacklistBtn, "click", handleTaBlacklistClick);
  }
  if (copyMintBtn) {
    on(copyMintBtn, "click", () => {
      navigator.clipboard.writeText(mint);
      Utils.notifyCopied(I18n.t("links-mint-address"));
    });
  }
  if (dexscreenerBtn) {
    on(dexscreenerBtn, "click", () => {
      window.open(`https://dexscreener.com/solana/${encodeURIComponent(mint)}`, "_blank");
    });
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
      Utils.showToast(
        I18n.t("tools-analyzer-favorite-added", { symbol: symbol || I18n.t("format-unknown") }),
        "success"
      );
      btn.classList.add("active");
      btn.title = I18n.t("tools-analyzer-favorite-already");
    } else {
      throw new Error(apiErrorMessage(data, I18n.t("tools-analyzer-favorite-failed")));
    }
  } catch (error) {
    Utils.showToast(I18n.t("common-error-with-message", { message: error.message }), "error");
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
    title: I18n.t("tools-analyzer-blacklist-title"),
    message: I18n.t("tools-analyzer-blacklist-message", {
      symbol: symbol || I18n.t("format-unknown"),
    }),
    confirmLabel: I18n.t("tools-analyzer-blacklist-confirm"),
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
        // l10n-ignore: stored blacklist reason sent to the API, not shown text
        reason: "Manual blacklist via Token Analyzer",
      }),
    });

    const data = await response.json();

    if (response.ok && data.success) {
      Utils.showToast(
        I18n.t("tools-analyzer-blacklist-done", { symbol: symbol || I18n.t("format-unknown") }),
        "success"
      );
      btn.classList.add("active");
      btn.title = I18n.t("tools-analyzer-blacklisted");
    } else {
      throw new Error(apiErrorMessage(data, I18n.t("tools-analyzer-blacklist-failed")));
    }
  } catch (error) {
    Utils.showToast(I18n.t("common-error-with-message", { message: error.message }), "error");
  } finally {
    btn.disabled = false;
    btn.classList.remove("loading");
  }
}

/**
 * Switch between analysis tabs
 */
function switchTaTab(tabId) {
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
          <i class="icon-activity"></i> <span data-l10n-id="tools-analyzer-card-quick-stats"></span>
        </div>
        <div class="ta-stat-grid">
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-holders"></span>
            <span class="ta-stat-value">${overview.total_holders ? Utils.formatCompactNumber(overview.total_holders) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-decimals"></span>
            <span class="ta-stat-value">${formatNumber(overview.decimals, 0)}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-safety-score"></span>
            <span class="ta-stat-value ${security?.normalized_score ? getTaScoreClass(security.normalized_score) : ""}">${formatNumber(security?.normalized_score, 0)}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-pools"></span>
            <span class="ta-stat-value">${formatNumber(liquidity?.pool_count, 0)}</span>
          </div>
        </div>
      </div>

      <!-- Market Summary Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-trending-up"></i> <span data-l10n-id="tools-analyzer-card-market-summary"></span>
        </div>
        <div class="ta-stat-grid">
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-volume-24h"></span>
            <span class="ta-stat-value">${market?.volume_h24 ? Utils.formatCurrencyUSD(market.volume_h24) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-change-24h"></span>
            <span class="ta-stat-value ${market?.price_change_h24 ? getTaPriceChangeClass(market.price_change_h24) : ""}">${market?.price_change_h24 ? Utils.formatPercent(market.price_change_h24) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-market-cap"></span>
            <span class="ta-stat-value">${market?.market_cap ? Utils.formatCurrencyUSD(market.market_cap) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-liquidity"></span>
            <span class="ta-stat-value">${liquidity?.total_liquidity_native ? Utils.formatSol(liquidity.total_liquidity_native) : "—"}</span>
          </div>
        </div>
      </div>

      <!-- Token Info Card -->
      <div class="ta-card ta-full-width">
        <div class="ta-card-title">
          <i class="icon-info"></i> <span data-l10n-id="tools-analyzer-card-token-info"></span>
        </div>
        <div class="ta-info-grid">
          <div class="ta-info-item">
            <span class="ta-info-label" data-l10n-id="tools-analyzer-info-mint"></span>
            <span class="ta-info-value mono" dir="ltr">${escapeHtml(overview.mint)}</span>
          </div>
          ${
            overview.description
              ? `
          <div class="ta-info-item ta-full-width">
            <span class="ta-info-label" data-l10n-id="tools-analyzer-info-description"></span>
            <span class="ta-info-value">${escapeHtml(overview.description)}</span>
          </div>
          `
              : ""
          }
          ${
            overview.supply
              ? `
          <div class="ta-info-item">
            <span class="ta-info-label" data-l10n-id="tools-analyzer-info-supply"></span>
            <span class="ta-info-value mono" dir="ltr">${escapeHtml(overview.supply)}</span>
          </div>
          `
              : ""
          }
        </div>
        <div class="ta-links">
          ${overview.website ? `<a href="${escapeHtml(overview.website)}" target="_blank" rel="noopener" class="ta-link"><i class="icon-globe"></i> <span data-l10n-id="links-social-website"></span></a>` : ""}
          ${overview.twitter ? `<a href="${escapeHtml(overview.twitter)}" target="_blank" rel="noopener" class="ta-link"><i class="icon-twitter"></i> <span data-l10n-id="links-social-twitter"></span></a>` : ""}
          ${overview.telegram ? `<a href="${escapeHtml(overview.telegram)}" target="_blank" rel="noopener" class="ta-link"><i class="icon-message-circle"></i> <span data-l10n-id="links-social-telegram"></span></a>` : ""}
        </div>
      </div>
    </div>
  `;
  I18n.localizeTree(contentEl);
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
        <p data-l10n-id="tools-analyzer-security-empty"></p>
        <small data-l10n-id="tools-analyzer-security-empty-hint"></small>
      </div>
    `;
    I18n.localizeTree(contentEl);
    return;
  }

  const scoreClass = security.normalized_score ? getTaScoreClass(security.normalized_score) : "";

  contentEl.innerHTML = `
    <div class="ta-security-grid">
      <!-- Security Score Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-shield"></i> <span data-l10n-id="tools-analyzer-card-safety-score"></span>
        </div>
        <div class="ta-security-score ${scoreClass}">
          <span class="ta-score-value">${formatNumber(security.normalized_score, 0)}</span>
          <span class="ta-score-label">${escapeHtml(getTaScoreLabel(security.normalized_score))}</span>
        </div>
        ${security.score ? `<div class="ta-raw-score" data-l10n-id="tools-analyzer-raw-score" data-l10n-args='${escapeHtml(JSON.stringify({ score: formatNumber(security.score, 0) }))}'></div>` : ""}
      </div>

      <!-- Authorities Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-key"></i> <span data-l10n-id="tools-analyzer-card-authorities"></span>
        </div>
        <div class="ta-authority-list">
          <div class="ta-authority-item ${security.mint_authority ? "warning" : "success"}">
            <span class="ta-authority-label" data-l10n-id="tools-analyzer-authority-mint"></span>
            <span class="ta-authority-value">${escapeHtml(I18n.label(AUTHORITY_STATE_LABELS, security.mint_authority ? "active" : "revoked"))}</span>
            ${security.mint_authority ? `<span class="ta-authority-address mono" dir="ltr">${escapeHtml(security.mint_authority)}</span>` : ""}
          </div>
          <div class="ta-authority-item ${security.freeze_authority ? "warning" : "success"}">
            <span class="ta-authority-label" data-l10n-id="tools-analyzer-authority-freeze"></span>
            <span class="ta-authority-value">${escapeHtml(I18n.label(AUTHORITY_STATE_LABELS, security.freeze_authority ? "active" : "revoked"))}</span>
            ${security.freeze_authority ? `<span class="ta-authority-address mono" dir="ltr">${escapeHtml(security.freeze_authority)}</span>` : ""}
          </div>
          <div class="ta-authority-item ${security.has_transfer_fee ? "warning" : "success"}">
            <span class="ta-authority-label" data-l10n-id="tools-analyzer-authority-transfer-fee"></span>
            <span class="ta-authority-value">${escapeHtml(formatBooleanFlag(Boolean(security.has_transfer_fee)))}</span>
          </div>
          <div class="ta-authority-item ${security.is_mutable ? "warning" : "success"}">
            <span class="ta-authority-label" data-l10n-id="tools-analyzer-authority-mutable"></span>
            <span class="ta-authority-value">${escapeHtml(formatBooleanFlag(Boolean(security.is_mutable)))}</span>
          </div>
        </div>
      </div>

      <!-- Top Holders Card -->
      ${
        security.top_holders_pct
          ? `
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-users"></i> <span data-l10n-id="tools-analyzer-card-holder-concentration"></span>
        </div>
        <div class="ta-holder-concentration">
          <div class="ta-holder-bar">
            <div class="ta-holder-fill" style="width: ${Math.min(security.top_holders_pct, 100)}%"></div>
          </div>
          <span class="ta-holder-pct">${formatPercentValue(security.top_holders_pct, { plus: "" })}</span>
          <span class="ta-holder-label" data-l10n-id="tools-analyzer-top-holders"></span>
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
          <i class="icon-triangle-alert"></i> <span data-l10n-id="tools-analyzer-risks-title" data-l10n-args='${escapeHtml(JSON.stringify({ count: formatNumber(security.risks.length, 0) }))}'></span>
        </div>
        <div class="ta-risk-list">
          ${security.risks
            .map(
              (risk) => `
            <div class="ta-risk-item ${risk.level.toLowerCase()}">
              <span class="ta-risk-level">${escapeHtml(I18n.label(RISK_SEVERITY_LABELS, riskSeverityId(risk.level)))}</span>
              <span class="ta-risk-name">${escapeHtml(rugcheckRiskName(risk.name))}</span>
              <span class="ta-risk-desc">${escapeHtml(rugcheckRiskDescription(risk))}</span>
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
          <i class="icon-circle-check"></i> <span data-l10n-id="tools-analyzer-risks-title-none"></span>
        </div>
        <div class="ta-no-risks">
          <i class="icon-shield-check"></i>
          <p data-l10n-id="tools-analyzer-risks-none"></p>
        </div>
      </div>
      `
      }
    </div>
  `;
  I18n.localizeTree(contentEl);
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
        <p data-l10n-id="tools-analyzer-market-empty"></p>
        <small data-l10n-id="tools-analyzer-market-empty-hint"></small>
      </div>
    `;
    I18n.localizeTree(contentEl);
    return;
  }

  contentEl.innerHTML = `
    <div class="ta-market-grid">
      <!-- Price Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-dollar-sign"></i> <span data-l10n-id="tools-analyzer-card-price"></span>
        </div>
        <div class="ta-price-display">
          <div class="ta-price-main">${market.price_sol ? Utils.formatSol(market.price_sol) : "—"}</div>
          ${market.price_usd ? `<div class="ta-price-sub">${Utils.formatCurrencyUSD(market.price_usd)}</div>` : ""}
        </div>
      </div>

      <!-- Price Changes Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-percent"></i> <span data-l10n-id="tools-analyzer-card-price-changes"></span>
        </div>
        <div class="ta-stat-grid">
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-window-1h"></span>
            <span class="ta-stat-value ${market.price_change_h1 ? getTaPriceChangeClass(market.price_change_h1) : ""}">${market.price_change_h1 ? Utils.formatPercent(market.price_change_h1) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-window-6h"></span>
            <span class="ta-stat-value ${market.price_change_h6 ? getTaPriceChangeClass(market.price_change_h6) : ""}">${market.price_change_h6 ? Utils.formatPercent(market.price_change_h6) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-window-24h"></span>
            <span class="ta-stat-value ${market.price_change_h24 ? getTaPriceChangeClass(market.price_change_h24) : ""}">${market.price_change_h24 ? Utils.formatPercent(market.price_change_h24) : "—"}</span>
          </div>
        </div>
      </div>

      <!-- Volume Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-chart-bar"></i> <span data-l10n-id="tools-analyzer-card-volume"></span>
        </div>
        <div class="ta-stat-grid">
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-volume-1h"></span>
            <span class="ta-stat-value">${market.volume_h1 ? Utils.formatCurrencyUSD(market.volume_h1) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-volume-6h"></span>
            <span class="ta-stat-value">${market.volume_h6 ? Utils.formatCurrencyUSD(market.volume_h6) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-volume-24h"></span>
            <span class="ta-stat-value">${market.volume_h24 ? Utils.formatCurrencyUSD(market.volume_h24) : "—"}</span>
          </div>
        </div>
      </div>

      <!-- Transactions Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-repeat"></i> <span data-l10n-id="tools-analyzer-card-transactions"></span>
        </div>
        <div class="ta-txns-display">
          <div class="ta-txn-item buys">
            <span class="ta-txn-label" data-l10n-id="tools-analyzer-txn-buys"></span>
            <span class="ta-txn-value">${market.txns_buys_h24 ? Utils.formatCompactNumber(market.txns_buys_h24) : "—"}</span>
          </div>
          <div class="ta-txn-item sells">
            <span class="ta-txn-label" data-l10n-id="tools-analyzer-txn-sells"></span>
            <span class="ta-txn-value">${market.txns_sells_h24 ? Utils.formatCompactNumber(market.txns_sells_h24) : "—"}</span>
          </div>
        </div>
      </div>

      <!-- Valuation Card -->
      <div class="ta-card ta-full-width">
        <div class="ta-card-title">
          <i class="icon-chart-pie"></i> <span data-l10n-id="tools-analyzer-card-valuation"></span>
        </div>
        <div class="ta-stat-grid">
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-market-cap"></span>
            <span class="ta-stat-value">${market.market_cap ? Utils.formatCurrencyUSD(market.market_cap) : "—"}</span>
          </div>
          <div class="ta-stat-item">
            <span class="ta-stat-label" data-l10n-id="tools-analyzer-stat-fdv"></span>
            <span class="ta-stat-value">${market.fdv ? Utils.formatCurrencyUSD(market.fdv) : "—"}</span>
          </div>
        </div>
      </div>
    </div>
  `;
  I18n.localizeTree(contentEl);
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
        <p data-l10n-id="tools-analyzer-liquidity-empty"></p>
        <small data-l10n-id="tools-analyzer-liquidity-empty-hint"></small>
      </div>
    `;
    I18n.localizeTree(contentEl);
    return;
  }

  contentEl.innerHTML = `
    <div class="ta-liquidity-grid">
      <!-- Total Liquidity Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-droplet"></i> <span data-l10n-id="tools-analyzer-card-total-liquidity"></span>
        </div>
        <div class="ta-liquidity-total">
          <div class="ta-liquidity-sol">${Utils.formatSol(liquidity.total_liquidity_native)}</div>
          ${liquidity.total_liquidity_usd ? `<div class="ta-liquidity-usd">${Utils.formatCurrencyUSD(liquidity.total_liquidity_usd)}</div>` : ""}
        </div>
      </div>

      <!-- Pool Count Card -->
      <div class="ta-card">
        <div class="ta-card-title">
          <i class="icon-layers"></i> <span data-l10n-id="tools-analyzer-card-pools"></span>
        </div>
        <div class="ta-pool-count">
          <span class="ta-pool-count-value">${formatNumber(liquidity.pool_count, 0)}</span>
          <span class="ta-pool-count-label" data-l10n-id="tools-analyzer-active-pools" data-l10n-args='${escapeHtml(JSON.stringify({ count: liquidity.pool_count }))}'></span>
        </div>
      </div>

      <!-- Pools Table Card -->
      <div class="ta-card ta-full-width">
        <div class="ta-card-title">
          <i class="icon-list"></i> <span data-l10n-id="tools-analyzer-card-pool-details"></span>
        </div>
        <div class="ta-pools-table">
          <table>
            <thead>
              <tr>
                <th data-l10n-id="tools-analyzer-pools-column-dex"></th>
                <th data-l10n-id="tools-analyzer-pools-column-address"></th>
                <th data-l10n-id="tools-analyzer-pools-column-liquidity"></th>
                <th data-l10n-id="tools-analyzer-pools-column-status"></th>
              </tr>
            </thead>
            <tbody>
              ${liquidity.pools
                .map(
                  (pool) => `
                <tr class="${pool.is_canonical ? "canonical" : ""}">
                  <td class="dex">${escapeHtml(venueLabel(pool.dex))}</td>
                  <td class="address mono" dir="ltr">${escapeHtml(pool.address.slice(0, 8))}...${escapeHtml(pool.address.slice(-6))}</td>
                  <td class="liquidity">${Utils.formatSol(pool.liquidity_native)}</td>
                  <td class="status">${pool.is_canonical ? '<span class="canonical-badge" data-l10n-id="tools-analyzer-pool-primary"></span>' : ""}</td>
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
  I18n.localizeTree(contentEl);
}

/**
 * Copy analysis report to clipboard
 */
function copyAnalysisReport() {
  if (!taAnalysisData || !taCurrentMint) {
    Utils.showToast(I18n.t("tools-analyzer-report-empty"), "warning");
    return;
  }

  const { overview, security, market, liquidity } = taAnalysisData;
  const unknown = I18n.t("format-unknown");
  const authorityState = (authority) =>
    I18n.label(AUTHORITY_STATE_LABELS, authority ? "active" : "revoked");

  const lines = [
    I18n.t("tools-analyzer-report-title"),
    "====================",
    "",
    I18n.t("tools-analyzer-report-token", {
      symbol: overview.symbol || unknown,
      name: overview.name || unknown,
    }),
    I18n.t("tools-analyzer-report-mint", { mint: overview.mint }),
    "",
  ];

  if (overview.price_sol) {
    const priceSol = withSolUnit(formatFixed(overview.price_sol, { decimals: 12, trim: true }));
    lines.push(
      overview.price_usd
        ? I18n.t("tools-analyzer-report-price-with-usd", {
            sol: priceSol,
            usd: formatCurrencyUSD(overview.price_usd),
          })
        : I18n.t("tools-analyzer-report-price", { sol: priceSol })
    );
  }

  if (security) {
    lines.push(
      "",
      I18n.t("tools-analyzer-report-security"),
      I18n.t("tools-analyzer-report-safety-score", {
        score:
          security.normalized_score === null || security.normalized_score === undefined
            ? I18n.t("format-not-available")
            : formatNumber(security.normalized_score, 0),
      }),
      I18n.t("tools-analyzer-report-mint-authority", {
        state: authorityState(security.mint_authority),
      }),
      I18n.t("tools-analyzer-report-freeze-authority", {
        state: authorityState(security.freeze_authority),
      })
    );
    if (security.risks && security.risks.length > 0) {
      lines.push(
        I18n.t("tools-analyzer-report-risks", { count: formatNumber(security.risks.length, 0) })
      );
    }
  }

  if (market) {
    lines.push("", I18n.t("tools-analyzer-report-market"));
    if (market.volume_h24) {
      lines.push(
        I18n.t("tools-analyzer-report-volume", { amount: formatCurrencyUSD(market.volume_h24) })
      );
    }
    if (market.price_change_h24) {
      lines.push(
        I18n.t("tools-analyzer-report-change", {
          amount: formatPercentValue(market.price_change_h24, { plus: "" }),
        })
      );
    }
    if (market.market_cap) {
      lines.push(
        I18n.t("tools-analyzer-report-market-cap", { amount: formatCurrencyUSD(market.market_cap) })
      );
    }
  }

  if (liquidity) {
    lines.push(
      "",
      I18n.t("tools-analyzer-report-liquidity"),
      I18n.t("tools-analyzer-report-liquidity-total", {
        amount: Utils.formatSol(liquidity.total_liquidity_native),
      }),
      I18n.t("tools-analyzer-report-pools", { count: formatNumber(liquidity.pool_count, 0) })
    );
  }

  lines.push(
    "",
    I18n.t("tools-analyzer-report-generated", {
      time: Utils.formatTimestamp(taAnalysisData.fetched_at),
    })
  );

  // The report is copied, so the isolation marks Fluent adds around values must not ride along.
  Utils.copyToClipboard(lines.join("\n").replace(/[\u2068\u2069]/g, ""));
  Utils.notifyCopied(I18n.t("tools-analyzer-report-label"));
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
  return I18n.label(SCORE_BAND_LABELS, getTaScoreBand(score));
}

/**
 * Helper: Get the score band id
 */
function getTaScoreBand(score) {
  if (!score && score !== 0) return "unknown";
  // Lower score = safer
  if (score <= 30) return "good";
  if (score <= 60) return "moderate";
  return "risky";
}

/**
 * Helper: Normalize a RugCheck risk level to a severity id
 */
function riskSeverityId(level) {
  const normalized = String(level ?? "").toLowerCase();
  if (normalized === "danger") return "danger";
  if (normalized === "warn" || normalized === "warning") return "warn";
  return "info";
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

export { renderCreateTokenTool, renderTokenWatchTool, renderTokenAnalyzerTool };
