/**
 * Multi-Wallet Tools Module
 * Contains multi-wallet trading utilities: multi-buy and multi-sell
 */

import { $, $$, on } from "../../core/dom.js";
import * as Utils from "../../core/utils.js";
import * as Hints from "../../core/hints.js";
import { HintTrigger } from "../../ui/hint_popover.js";
import { enhanceAllSelects } from "../../ui/custom_select.js";

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
    pending: "...",
    running: "↻",
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
      summary.innerHTML = '<span class="text-muted">No wallets selected</span>';
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
// Exports
// =============================================================================

export {
  renderBuyMultiWalletsTool,
  renderSellMultiWalletsTool,
  stopMultiBuyPolling,
  resetMultiBuyUI,
  stopMultiSellPolling,
  resetMultiSellUI,
};
