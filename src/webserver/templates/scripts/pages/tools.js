/**
 * Tools Page Module
 * Provides utility tools for wallet management, token operations, and trading
 */

import { registerPage } from "../core/lifecycle.js";
import { $, $$, on, off } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as Hints from "../core/hints.js";
import { HintTrigger } from "../ui/hint_popover.js";

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

  container.innerHTML = `
    <div class="tool-panel wallet-cleanup-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-search"></i> Scan Results</h3>
          ${hintHtml}
        </div>
        <div class="section-content">
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

// Volume aggregator state
let volumeAggregatorPoller = null;

function renderVolumeAggregatorTool(container, actionsContainer) {
  const hint = Hints.getHint("tools.volumeAggregator");
  const hintHtml = hint ? HintTrigger.render(hint, "tools.volumeAggregator", { size: "sm" }) : "";

  container.innerHTML = `
    <div class="tool-panel volume-aggregator-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-settings"></i> Configuration</h3>
          ${hintHtml}
        </div>
        <div class="section-content">
          <form class="tool-form" id="volume-aggregator-form">
            <div class="form-group">
              <label for="va-token-mint">Token Mint Address *</label>
              <input type="text" id="va-token-mint" placeholder="e.g., EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" required />
              <small>The token to generate volume for</small>
            </div>
            
            <div class="form-row">
              <div class="form-group">
                <label for="va-total-volume">Total Volume (SOL)</label>
                <input type="number" id="va-total-volume" value="10" min="0.1" step="0.1" />
                <small>Total SOL volume to generate</small>
              </div>
              <div class="form-group">
                <label for="va-num-wallets">Number of Wallets</label>
                <select id="va-num-wallets">
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
                <small>Secondary wallets to use</small>
              </div>
            </div>
            
            <div class="form-row">
              <div class="form-group">
                <label for="va-min-amount">Min Amount per Tx (SOL)</label>
                <input type="number" id="va-min-amount" value="0.05" min="0.001" step="0.01" />
              </div>
              <div class="form-group">
                <label for="va-max-amount">Max Amount per Tx (SOL)</label>
                <input type="number" id="va-max-amount" value="0.2" min="0.001" step="0.01" />
              </div>
            </div>
            
            <div class="form-row">
              <div class="form-group">
                <label for="va-delay">Delay Between Txs (ms)</label>
                <input type="number" id="va-delay" value="3000" min="1000" step="100" />
                <small>Minimum 1000ms</small>
              </div>
              <div class="form-group checkbox-group">
                <label>
                  <input type="checkbox" id="va-randomize" checked />
                  Randomize Amounts
                </label>
                <small>Vary transaction amounts within min/max</small>
              </div>
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
  `;

  HintTrigger.initAll();

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

  if (startBtn) on(startBtn, "click", handleVolumeAggregatorStart);
  if (stopBtn) on(stopBtn, "click", handleVolumeAggregatorStop);
  if (clearLogBtn) on(clearLogBtn, "click", clearVolumeAggregatorLog);

  // Check current status on load
  checkVolumeAggregatorStatus();
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
