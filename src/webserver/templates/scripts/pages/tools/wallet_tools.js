/**
 * Wallet Tools Module
 * Contains wallet-related utilities: cleanup, burn tokens, consolidation, and generator
 */

import { $, $$, on } from "../../core/dom.js";
import * as Utils from "../../core/utils.js";
import * as Hints from "../../core/hints.js";
import { HintTrigger } from "../../ui/hint_popover.js";

// =============================================================================
// Wallet Cleanup Tool
// =============================================================================

function renderWalletCleanupTool(container, actionsContainer) {
  const hint = Hints.getHint("tools.walletCleanup");
  const hintHtml = hint ? HintTrigger.render(hint, "tools.walletCleanup", { size: "sm" }) : "";

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
            : '<span class="burn-token-value no-value">No value</span>'
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
        ? `<br><br><i class="icon-triangle-alert"></i> Total estimated value: <strong>${Utils.formatSol(totalValue)}</strong>`
        : ""),
    "Continue",
    "Cancel"
  );

  if (!firstConfirm) return;

  // Second confirmation (critical warning)
  const secondConfirm = await showBurnConfirmation(
    "Final Warning",
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
      Utils.notifyCopied("Public key");
      break;
    }
    case "copy-secret": {
      const secret = btn.dataset.secret;
      Utils.copyToClipboard(secret);
      // A warning, not a confirmation: the clipboard now holds full control of
      // this wallet, which is the part the user needs to be told.
      Utils.showToast({
        key: "clipboard",
        type: "warning",
        title: "Private key copied",
        message: "Anyone with this key controls the wallet",
      });
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

  Utils.showToast("Wallets exported - store securely", "warning");
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
      summary.innerHTML = '<span class="text-muted">Select wallets to consolidate</span>';
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


// =============================================================================
// Exports
// =============================================================================

export {
  renderWalletCleanupTool,
  renderBurnTokensTool,
  renderWalletConsolidationTool,
  renderAirdropCheckerTool,
  renderWalletGeneratorTool,
  // State exports for cleanup (if needed by main tools.js)
  burnTokensState,
  consolidationState,
  generatedWallets,
};
