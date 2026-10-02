// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Trading Tools Module
 * Contains trading-related utilities: trade watcher and volume aggregator
 */

import { $, on } from "../../core/dom.js";
import { formatNumber } from "../../core/format.js";
import * as Utils from "../../core/utils.js";
import * as Hints from "../../core/hints.js";
import { HintTrigger } from "../../ui/hint_popover.js";
import { enhanceAllSelects } from "../../ui/custom_select.js";
import { PoolSelector } from "../../ui/pool_selector.js";
import { venueLabel } from "../../ui/venue.js";
import { apiErrorMessage } from "../../core/request_manager.js";

// Message key of each watch type, as the badge in the active watches table.
const WATCH_TYPE_LABELS = Object.freeze({
  "buy-on-sell": "tools-watch-type-buy-on-sell",
  "sell-on-buy": "tools-watch-type-sell-on-buy",
  "notify-only": "tools-watch-type-notify",
});

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
          <h3><i class="icon-target"></i> <span data-l10n-id="tools-trade-watcher-setup-title"></span></h3>
          <div class="section-header-actions">
            ${hintHtml}
          </div>
        </div>
        <div class="section-content">
          <form class="tool-form" id="tw-form">
            <div class="form-row">
              <div class="form-group flex-2">
                <label for="tw-mint" data-l10n-id="tools-trade-watcher-mint-label"></label>
                <div class="input-with-action">
                  <input dir="ltr" type="text" id="tw-mint" data-l10n-id="tools-trade-watcher-mint-input" />
                  <button type="button" class="btn btn-sm" id="tw-search-pools-btn">
                    <i class="icon-search"></i> <span data-l10n-id="tools-trade-watcher-action-search-pools"></span>
                  </button>
                </div>
              </div>
            </div>

            <div class="form-row" id="tw-pool-row" style="display: none;">
              <div class="form-group">
                <label data-l10n-id="tools-trade-watcher-pool-label"></label>
                <div class="selected-pool-card" id="tw-selected-pool">
                  <span class="pool-info" data-l10n-id="tools-trade-watcher-pool-none"></span>
                  <button type="button" class="btn btn-sm btn-icon" id="tw-clear-pool-btn" data-l10n-id="tools-trade-watcher-pool-clear">
                    <i class="icon-x"></i>
                  </button>
                </div>
              </div>
            </div>

            <div class="form-row">
              <div class="form-group">
                <label for="tw-watch-type" data-l10n-id="tools-trade-watcher-type-label"></label>
                <select id="tw-watch-type" data-custom-select>
                  <option value="buy-on-sell" data-l10n-id="tools-watch-type-buy-on-sell"></option>
                  <option value="sell-on-buy" data-l10n-id="tools-watch-type-sell-on-buy"></option>
                  <option value="notify-only" data-l10n-id="tools-watch-type-notify-only"></option>
                </select>
                <small class="form-hint" data-l10n-id="tools-trade-watcher-type-hint"></small>
              </div>
            </div>

            <div class="form-row" id="tw-trigger-row">
              <div class="form-group">
                <label for="tw-trigger-amount" data-l10n-id="tools-trade-watcher-trigger-label"></label>
                <input type="number" id="tw-trigger-amount" placeholder="0.1" min="0.001" step="0.001" value="0.1" />
                <small class="form-hint" data-l10n-id="tools-trade-watcher-trigger-hint"></small>
              </div>
              <div class="form-group">
                <label for="tw-action-amount" data-l10n-id="tools-trade-watcher-action-amount-label"></label>
                <input type="number" id="tw-action-amount" placeholder="0.1" min="0.001" step="0.001" value="0.1" />
                <small class="form-hint" data-l10n-id="tools-trade-watcher-action-amount-hint"></small>
              </div>
              <div class="form-group">
                <label for="tw-slippage" data-l10n-id="tools-trade-watcher-slippage-label"></label>
                <input type="number" id="tw-slippage" placeholder="5" min="0.5" max="50" step="0.5" value="5" />
                <small class="form-hint" data-l10n-id="tools-trade-watcher-slippage-hint"></small>
              </div>
            </div>
          </form>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-activity"></i> <span data-l10n-id="tools-trade-watcher-active-title"></span></h3>
          <span class="section-badge" id="tw-watch-count">0</span>
        </div>
        <div class="section-content">
          <div class="tw-watches-table" id="tw-watches-table">
            <div class="empty-state">
              <i class="icon-eye-off"></i>
              <p data-l10n-id="tools-trade-watcher-empty"></p>
              <small data-l10n-id="tools-trade-watcher-empty-hint"></small>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;
  I18n.localizeTree(container);

  HintTrigger.initAll();
  enhanceAllSelects(container);

  actionsContainer.innerHTML = `
    <button class="btn primary" id="tw-start-btn" disabled>
      <i class="icon-play"></i> <span data-l10n-id="tools-trade-watcher-action-start"></span>
    </button>
    <button class="btn danger" id="tw-stop-all-btn" disabled>
      <i class="icon-square"></i> <span data-l10n-id="tools-trade-watcher-action-stop-all"></span>
    </button>
  `;
  I18n.localizeTree(actionsContainer);

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
    Utils.showToast(I18n.t("tools-validation-mint-required"), "warning");
    return;
  }

  // Validate mint format
  if (!/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(mint)) {
    Utils.showToast(I18n.t("tools-validation-mint-format"), "error");
    return;
  }

  // Create pool selector if not exists
  if (!twPoolSelector) {
    twPoolSelector = new PoolSelector({
      onSelect: (pool, _tokenMint) => {
        twSelectedPool = pool;
        updateTwPoolDisplay();
        updateTwStartButtonState();
        Utils.showToast(
          I18n.t("tools-trade-watcher-pool-selected", {
            dex: pool.dex ? venueLabel(pool.dex) : I18n.t("format-unknown"),
            base: pool.base_symbol,
            quote: pool.quote_symbol,
          }),
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
        <span class="pool-dex">${Utils.escapeHtml(twSelectedPool.dex ? venueLabel(twSelectedPool.dex) : I18n.t("format-unknown"))}</span>
        <span class="pool-pair">${Utils.escapeHtml(twSelectedPool.base_symbol || "?")}/${Utils.escapeHtml(twSelectedPool.quote_symbol || "?")}</span>
        <span class="pool-source ${(twSelectedPool.source || "").toLowerCase()}">${Utils.escapeHtml(twSelectedPool.source || "")}</span>
      </div>
      <button type="button" class="btn btn-sm btn-icon" id="tw-clear-pool-btn" data-l10n-id="tools-trade-watcher-pool-clear">
        <i class="icon-x"></i>
      </button>
    `;
    I18n.localizeTree(poolCard);

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
    poolCard.innerHTML =
      '<span class="pool-info" data-l10n-id="tools-trade-watcher-pool-none"></span>';
    I18n.localizeTree(poolCard);
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
    Utils.showToast(I18n.t("tools-validation-mint-required"), "warning");
    return;
  }

  startBtn.disabled = true;
  startBtn.innerHTML =
    '<i class="icon-loader spin"></i> <span data-l10n-id="tools-trade-watcher-action-starting"></span>';
  I18n.localizeTree(startBtn);

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
      throw new Error(apiErrorMessage(data, I18n.t("tools-trade-watcher-start-failed")));
    }

    Utils.showToast(
      I18n.t("tools-trade-watcher-started", { token: data.symbol || mint.slice(0, 8) }),
      "success"
    );

    // Clear form
    mintInput.value = "";
    twSelectedPool = null;
    updateTwPoolDisplay();
    updateTwStartButtonState();

    // Refresh watches list
    loadTwActiveWatches();
  } catch (error) {
    Utils.showToast(I18n.t("common-error-with-message", { message: error.message }), "error");
  } finally {
    startBtn.disabled = false;
    startBtn.innerHTML =
      '<i class="icon-play"></i> <span data-l10n-id="tools-trade-watcher-action-start"></span>';
    I18n.localizeTree(startBtn);
    updateTwStartButtonState();
  }
}

/**
 * Handle stop all watches
 */
async function handleTwStopAllWatches() {
  const stopAllBtn = $("#tw-stop-all-btn");

  stopAllBtn.disabled = true;
  stopAllBtn.innerHTML =
    '<i class="icon-loader spin"></i> <span data-l10n-id="tools-trade-watcher-action-stopping"></span>';
  I18n.localizeTree(stopAllBtn);

  try {
    const response = await fetch("/api/tools/trade-watcher/stop-all", {
      method: "POST",
    });

    const data = await response.json();

    if (!response.ok || !data.success) {
      throw new Error(apiErrorMessage(data, I18n.t("tools-trade-watcher-stop-all-failed")));
    }

    Utils.showToast(I18n.t("tools-trade-watcher-stopped-all"), "success");
    loadTwActiveWatches();
  } catch (error) {
    Utils.showToast(I18n.t("common-error-with-message", { message: error.message }), "error");
  } finally {
    stopAllBtn.disabled = false;
    stopAllBtn.innerHTML =
      '<i class="icon-square"></i> <span data-l10n-id="tools-trade-watcher-action-stop-all"></span>';
    I18n.localizeTree(stopAllBtn);
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
      throw new Error(apiErrorMessage(data, I18n.t("tools-trade-watcher-load-failed")));
    }

    const watches = data.watches || [];

    if (countEl) countEl.textContent = formatNumber(watches.length, 0);
    if (stopAllBtn) stopAllBtn.disabled = watches.length === 0;

    if (watches.length === 0) {
      tableEl.innerHTML = `
        <div class="empty-state">
          <i class="icon-eye-off"></i>
          <p data-l10n-id="tools-trade-watcher-empty"></p>
          <small data-l10n-id="tools-trade-watcher-empty-hint"></small>
        </div>
      `;
      I18n.localizeTree(tableEl);
      return;
    }

    tableEl.innerHTML = `
      <table class="tw-table">
        <thead>
          <tr>
            <th data-l10n-id="tools-trade-watcher-column-token"></th>
            <th data-l10n-id="tools-trade-watcher-column-type"></th>
            <th data-l10n-id="tools-trade-watcher-column-trigger"></th>
            <th data-l10n-id="tools-trade-watcher-column-action"></th>
            <th data-l10n-id="tools-trade-watcher-column-triggered"></th>
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
                  <span class="tw-symbol">${Utils.escapeHtml(watch.symbol || I18n.t("format-unknown"))}</span>
                  <span class="tw-mint" dir="ltr">${watch.mint.slice(0, 8)}...</span>
                </div>
              </td>
              <td>
                <span class="tw-type-badge ${watch.watch_type}">${Utils.escapeHtml(I18n.label(WATCH_TYPE_LABELS, watch.watch_type))}</span>
              </td>
              <td class="mono">${watch.trigger_amount_sol ? Utils.formatSol(watch.trigger_amount_sol) : "—"}</td>
              <td class="mono">${watch.action_amount_sol ? Utils.formatSol(watch.action_amount_sol) : "—"}</td>
              <td class="mono">${formatNumber(watch.trigger_count || 0, 0)}</td>
              <td>
                <button class="btn btn-sm btn-icon danger tw-stop-btn" data-l10n-id="tools-trade-watcher-stop-watch">
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
    I18n.localizeTree(tableEl);

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
        <p data-l10n-id="tools-trade-watcher-load-failed"></p>
      </div>
    `;
    I18n.localizeTree(tableEl);
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
      throw new Error(apiErrorMessage(data, I18n.t("tools-trade-watcher-stop-failed")));
    }

    Utils.showToast(I18n.t("tools-trade-watcher-stopped"), "success");
    loadTwActiveWatches();
  } catch (error) {
    Utils.showToast(I18n.t("common-error-with-message", { message: error.message }), "error");
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

// =============================================================================
// Exports
// =============================================================================

export { renderTradeWatcherTool };
