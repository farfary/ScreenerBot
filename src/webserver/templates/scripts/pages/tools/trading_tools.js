/**
 * Trading Tools Module
 * Contains trading-related utilities: trade watcher and volume aggregator
 */

import { $, $$, on } from "../../core/dom.js";
import * as Utils from "../../core/utils.js";
import * as Hints from "../../core/hints.js";
import { HintTrigger } from "../../ui/hint_popover.js";
import { DataTable } from "../../ui/data_table.js";
import { ToolFavorites } from "../../ui/tool_favorites.js";
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

// Volume aggregator state
let volumeAggregatorPoller = null;
export let vaHistoryTable = null;
export let vaToolFavorites = null;

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
// Exports
// =============================================================================

export {
  renderTradeWatcherTool,
  renderVolumeAggregatorTool,
  stopVolumeAggregatorPolling,
};
