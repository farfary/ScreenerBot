/**
 * Trading Tools Module
 * Contains trading-related utilities: trade watcher and volume aggregator
 */

import { $, on } from "../../core/dom.js";
import * as Utils from "../../core/utils.js";
import * as Hints from "../../core/hints.js";
import { HintTrigger } from "../../ui/hint_popover.js";
import { enhanceAllSelects } from "../../ui/custom_select.js";
import { PoolSelector } from "../../ui/pool_selector.js";

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
                <select id="tw-watch-type" data-custom-select>
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
      onSelect: (pool, _tokenMint) => {
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

// =============================================================================
// Exports
// =============================================================================

export {
  renderTradeWatcherTool,
};
