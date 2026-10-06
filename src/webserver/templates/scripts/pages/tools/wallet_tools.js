// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Wallet Tools Module
 * Contains wallet-related utilities: cleanup, burn tokens, consolidation, and generator
 */

import { $, $$, on } from "../../core/dom.js";
import * as Utils from "../../core/utils.js";
import * as Hints from "../../core/hints.js";
import { HintTrigger } from "../../ui/hint_popover.js";
import { renderAddress } from "../../ui/token_identity.js";
import { apiErrorMessage } from "../../core/request_manager.js";

// Ids are the categories of the burn scan.
const BURN_CATEGORY_LABELS = Object.freeze({
  open_position: "tools-burn-category-open-position",
  has_value: "tools-burn-category-has-value",
  closed_position: "tools-burn-category-closed-position",
  zero_liquidity: "tools-burn-category-zero-liquidity",
});

const BURN_CATEGORY_HINT_LABELS = Object.freeze({
  open_position: "tools-burn-category-hint-open-position",
  has_value: "tools-burn-category-hint-has-value",
  closed_position: "tools-burn-category-hint-closed-position",
  zero_liquidity: "tools-burn-category-hint-zero-liquidity",
});

/** Replace a button's content with an icon and an already localized label. */
function setButton(button, icon, label) {
  const glyph = document.createElement("i");
  glyph.className = icon;
  button.replaceChildren(glyph, ` ${label}`);
}

/** Message of a failed response: the localized envelope text, else the HTTP status. */
async function responseError(response) {
  const body = await response.json().catch(() => ({}));
  return new Error(apiErrorMessage(body, `HTTP ${response.status}`));
}

function errorStateHtml(text) {
  return `
      <div class="error-state">
        <i class="icon-circle-alert"></i>
        <p>${Utils.escapeHtml(text)}</p>
      </div>
    `;
}

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
          <h3><i class="icon-search"></i> <span data-l10n-id="tools-wallet-cleanup-results-title"></span></h3>
          <div class="section-header-actions">
            ${hintHtml}
          </div>
        </div>
        <div class="section-content">
          <div class="scan-stats">
            <div class="stat-card">
              <div class="stat-value" id="empty-atas-count">—</div>
              <div class="stat-label" data-l10n-id="tools-wallet-cleanup-stat-empty"></div>
            </div>
            <div class="stat-card">
              <div class="stat-value" id="reclaimable-sol">—</div>
              <div class="stat-label" data-l10n-id="tools-wallet-cleanup-stat-reclaimable"></div>
            </div>
            <div class="stat-card">
              <div class="stat-value" id="failed-atas-count">—</div>
              <div class="stat-label" data-l10n-id="tools-wallet-cleanup-stat-failed"></div>
            </div>
          </div>
          <div class="ata-list" id="ata-list">
            <div class="empty-state">
              <i class="icon-scan"></i>
              <p data-l10n-id="tools-wallet-cleanup-prompt"></p>
              <small data-l10n-id="tools-wallet-cleanup-prompt-hint"></small>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;
  I18n.localizeTree(container);

  HintTrigger.initAll();

  actionsContainer.innerHTML = `
    <button class="btn primary" id="scan-atas-btn">
      <i class="icon-scan"></i> <span data-l10n-id="tools-wallet-action-scan"></span>
    </button>
    <button class="btn success" id="cleanup-atas-btn" disabled>
      <i class="icon-trash-2"></i> <span data-l10n-id="tools-wallet-cleanup-action-cleanup"></span>
    </button>
  `;
  I18n.localizeTree(actionsContainer);

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
  setButton(scanBtn, "icon-loader spin", I18n.t("tools-wallet-action-scanning"));
  listEl.innerHTML = `<div class="loading-state"><i class="icon-loader spin"></i> ${Utils.escapeHtml(I18n.t("tools-wallet-cleanup-scanning"))}</div>`;

  try {
    const response = await fetch("/api/tools/ata-scan");
    if (!response.ok) {
      throw await responseError(response);
    }
    const stats = await response.json();

    const countEl = $("#empty-atas-count");
    const solEl = $("#reclaimable-sol");
    const failedEl = $("#failed-atas-count");
    const cleanupBtn = $("#cleanup-atas-btn");

    if (countEl) countEl.textContent = stats.empty_count || 0;
    if (solEl) solEl.textContent = Utils.formatSol(stats.reclaimable_native || 0);
    if (failedEl) failedEl.textContent = stats.failed_count || 0;

    if (stats.empty_count > 0) {
      const found = I18n.t("tools-wallet-cleanup-found", {
        count: stats.empty_count,
        amount: Utils.formatSol(stats.reclaimable_native || 0),
      });
      listEl.innerHTML = `
        <div class="success-state">
          <i class="icon-circle-check"></i>
          <p>${Utils.escapeHtml(found)}</p>
        </div>
      `;
      if (cleanupBtn) cleanupBtn.disabled = false;
    } else {
      listEl.innerHTML = `
        <div class="empty-state">
          <i class="icon-circle-check"></i>
          <p data-l10n-id="tools-wallet-cleanup-clean"></p>
        </div>
      `;
      I18n.localizeTree(listEl);
    }
  } catch (error) {
    console.error("ATA scan failed:", error);
    listEl.innerHTML = errorStateHtml(I18n.t("tools-wallet-scan-failed", { reason: error.message }));
    Utils.showToast(I18n.t("tools-wallet-cleanup-scan-failed"), "error");
  } finally {
    scanBtn.disabled = false;
    setButton(scanBtn, "icon-scan", I18n.t("tools-wallet-action-scan"));
  }
}

async function handleCleanupATAs() {
  const cleanupBtn = $("#cleanup-atas-btn");
  if (!cleanupBtn) return;

  cleanupBtn.disabled = true;
  setButton(cleanupBtn, "icon-loader spin", I18n.t("tools-wallet-cleanup-action-cleaning"));

  try {
    const response = await fetch("/api/tools/ata-cleanup", { method: "POST" });
    if (!response.ok) {
      throw await responseError(response);
    }
    const data = await response.json().catch(() => ({}));

    Utils.showToast(
      I18n.t("tools-wallet-cleanup-done", { count: data.closed_count || 0 }),
      "success"
    );
    // Refresh scan
    handleScanATAs();
  } catch (error) {
    console.error("ATA cleanup failed:", error);
    Utils.showToast(I18n.t("tools-wallet-cleanup-failed", { reason: error.message }), "error");
  } finally {
    cleanupBtn.disabled = false;
    setButton(cleanupBtn, "icon-trash-2", I18n.t("tools-wallet-cleanup-action-cleanup"));
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
          <h3><i class="icon-flame"></i> <span data-l10n-id="tools-burn-section-title"></span></h3>
          <div class="section-header-actions">
            ${hintHtml}
          </div>
        </div>
        <div class="section-content">
          <div class="burn-info-box">
            <i class="icon-info"></i>
            <div class="burn-info-content">
              <p><strong data-l10n-id="tools-burn-info-title"></strong></p>
              <p data-l10n-id="tools-burn-info-body"></p>
            </div>
          </div>
          
          <div class="burn-stats" id="burn-stats">
            <div class="stat-card">
              <div class="stat-value" id="burn-total-tokens">—</div>
              <div class="stat-label" data-l10n-id="tools-burn-stat-total"></div>
            </div>
            <div class="stat-card">
              <div class="stat-value" id="burn-selected-count">0</div>
              <div class="stat-label" data-l10n-id="tools-burn-stat-selected"></div>
            </div>
            <div class="stat-card">
              <div class="stat-value" id="burn-rent-reclaimable">—</div>
              <div class="stat-label" data-l10n-id="tools-burn-stat-rent"></div>
            </div>
          </div>

          <div class="burn-token-list" id="burn-token-list">
            <div class="empty-state">
              <i class="icon-search"></i>
              <p data-l10n-id="tools-burn-prompt"></p>
            </div>
          </div>

          <div class="burn-failures" id="burn-failures" hidden></div>
        </div>
      </div>
    </div>
  `;
  I18n.localizeTree(container);

  HintTrigger.initAll();

  actionsContainer.innerHTML = `
    <button class="btn primary" id="scan-burn-tokens-btn">
      <i class="icon-search"></i> <span data-l10n-id="tools-wallet-action-scan"></span>
    </button>
    <button class="btn danger" id="burn-selected-btn" disabled>
      <i class="icon-flame"></i> ${Utils.escapeHtml(I18n.t("tools-burn-action-burn", { count: 0 }))}
    </button>
  `;
  I18n.localizeTree(actionsContainer);

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
  setButton(scanBtn, "icon-loader spin", I18n.t("tools-wallet-action-scanning"));
  listEl.innerHTML = `<div class="loading-state"><i class="icon-loader spin"></i> ${Utils.escapeHtml(I18n.t("tools-burn-scanning"))}</div>`;

  try {
    const response = await fetch("/api/tools/burn-tokens/scan");
    if (!response.ok) {
      throw await responseError(response);
    }
    const result = await response.json().catch(() => ({}));

    const data = result.data || result;
    burnTokensState.tokens = data.tokens || [];
    burnTokensState.selectedMints.clear();

    // Update stats
    const totalEl = $("#burn-total-tokens");
    const rentEl = $("#burn-rent-reclaimable");

    if (totalEl) totalEl.textContent = burnTokensState.tokens.length;
    if (rentEl) rentEl.textContent = Utils.formatSol(data.total_rent_reclaimable_native || 0);

    // Render token list
    renderBurnTokenList();
  } catch (error) {
    console.error("Burn tokens scan failed:", error);
    listEl.innerHTML = errorStateHtml(I18n.t("tools-wallet-scan-failed", { reason: error.message }));
    Utils.showToast(I18n.t("tools-burn-scan-failed"), "error");
  } finally {
    burnTokensState.isLoading = false;
    scanBtn.disabled = false;
    setButton(scanBtn, "icon-search", I18n.t("tools-wallet-action-scan"));
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
        <p data-l10n-id="tools-burn-empty"></p>
      </div>
    `;
    I18n.localizeTree(listEl);
    return;
  }

  // Group tokens by category
  const groups = {
    open_position: tokens.filter((t) => t.category === "open_position"),
    has_value: tokens.filter((t) => t.category === "has_value"),
    closed_position: tokens.filter((t) => t.category === "closed_position"),
    zero_liquidity: tokens.filter((t) => t.category === "zero_liquidity"),
  };

  const categoryIcons = {
    open_position: "icon-lock",
    has_value: "icon-dollar-sign",
    closed_position: "icon-archive",
    zero_liquidity: "icon-trash-2",
  };

  // Open positions (warning, cannot burn), has value (caution), closed
  // positions (info) and zero liquidity (safe to burn), in that order.
  let html = "";
  for (const category of ["open_position", "has_value", "closed_position", "zero_liquidity"]) {
    if (groups[category].length > 0) {
      html += renderBurnCategory(
        I18n.label(BURN_CATEGORY_LABELS, category),
        categoryIcons[category],
        groups[category],
        I18n.label(BURN_CATEGORY_HINT_LABELS, category),
        getCategoryType(category)
      );
    }
  }

  listEl.innerHTML = html;
  I18n.localizeTree(listEl);

  // The lock tooltip is the localized warning the scan returned.
  const byMint = new Map(tokens.map((t) => [t.mint, t]));
  listEl.querySelectorAll("[data-burn-lock]").forEach((icon) => {
    const token = byMint.get(icon.closest(".burn-token-row")?.dataset.mint);
    icon.title = token?.burn_warning
      ? I18n.text(token.burn_warning)
      : I18n.t("tools-burn-cannot-burn");
  });

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
            <span class="burn-category-title">${Utils.escapeHtml(title)}</span>
            <span class="burn-category-desc">${Utils.escapeHtml(description)}</span>
          </div>
          <span class="burn-category-count">${tokens.length}</span>
        </div>
        ${
          burnableTokens.length > 0
            ? `<label class="burn-select-all">
            <input type="checkbox" data-category="${type}" ${allSelected ? "checked" : ""}>
            <span data-l10n-id="common-action-select-all"></span>
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
  const symbol = token.symbol || I18n.t("format-unknown");
  const displayName = token.name || token.mint.substring(0, 8) + "...";

  return `
    <div class="burn-token-row ${!token.can_burn ? "disabled" : ""} ${isSelected ? "selected" : ""}" data-mint="${token.mint}">
      <div class="burn-token-select">
        ${
          token.can_burn
            ? `<input type="checkbox" class="burn-token-checkbox" data-mint="${token.mint}" ${isSelected ? "checked" : ""}>`
            : `<i class="icon-lock" data-burn-lock></i>`
        }
      </div>
      <div class="burn-token-info">
        <div class="burn-token-name">
          <span class="burn-token-symbol">${Utils.escapeHtml(symbol)}</span>
          <span class="burn-token-title">${Utils.escapeHtml(displayName)}</span>
        </div>
        <div class="burn-token-mint" dir="ltr" title="${token.mint}">
          ${token.mint.substring(0, 8)}...${token.mint.substring(token.mint.length - 6)}
        </div>
      </div>
      <div class="burn-token-balance">
        <span class="burn-token-amount">${Utils.formatCompactNumber(token.ui_amount)}</span>
        ${
          token.value_sol && token.value_sol > 0.0001
            ? `<span class="burn-token-value">${Utils.escapeHtml(I18n.t("tools-wallet-amount-approx", { amount: Utils.formatSol(token.value_sol) }))}</span>`
            : `<span class="burn-token-value no-value">${Utils.escapeHtml(I18n.t("tools-burn-no-value"))}</span>`
        }
      </div>
      <div class="burn-token-rent">
        ${token.can_burn ? Utils.escapeHtml(I18n.t("tools-wallet-amount-gain", { amount: Utils.formatSol(token.rent_reclaimable_native) })) : "—"}
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
    setButton(burnBtn, "icon-flame", I18n.t("tools-burn-action-burn", { count }));
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
    I18n.t("tools-burn-confirm-title"),
    I18n.markup("tools-burn-confirm-message", { count: selectedCount }) +
      (totalValue > 0.0001
        ? `<br><br><i class="icon-triangle-alert"></i> ${I18n.markup("tools-burn-confirm-value", { amount: Utils.formatSol(totalValue) })}`
        : ""),
    I18n.t("tools-burn-confirm-continue"),
    I18n.t("common-action-cancel")
  );

  if (!firstConfirm) return;

  // Second confirmation (critical warning)
  const secondConfirm = await showBurnConfirmation(
    I18n.t("tools-burn-final-title"),
    `<div class="burn-final-warning">
      <p><strong>${Utils.escapeHtml(I18n.t("tools-burn-final-headline"))}</strong></p>
      <p>${Utils.escapeHtml(I18n.t("tools-burn-final-message", { count: selectedCount }))}</p>
    </div>`,
    I18n.t("tools-burn-final-confirm"),
    I18n.t("common-action-cancel"),
    true
  );

  if (!secondConfirm) return;

  // Execute burn
  burnTokensState.isBurning = true;
  burnBtn.disabled = true;
  setButton(burnBtn, "icon-loader spin", I18n.t("tools-burn-action-burning"));
  renderBurnFailures([]);

  try {
    const response = await fetch("/api/tools/burn-tokens/burn", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ mints: selectedMints }),
    });

    if (!response.ok) {
      throw await responseError(response);
    }

    const result = await response.json();
    const data = result.data || result;

    if (data.successful > 0) {
      Utils.showToast(
        I18n.t("tools-burn-toast-burned", {
          successful: data.successful,
          total: data.total,
          amount: Utils.formatSol(data.native_reclaimed),
        }),
        "success"
      );
    }

    if (data.failed > 0) {
      Utils.showToast(I18n.t("tools-burn-toast-failed", { count: data.failed }), "warning");
    }
    renderBurnFailures((data.results || []).filter((item) => !item.success));

    // Refresh the list
    await handleScanBurnTokens();
  } catch (error) {
    console.error("Burn tokens failed:", error);
    Utils.showToast(I18n.t("tools-burn-failed", { reason: error.message }), "error");
  } finally {
    burnTokensState.isBurning = false;
    updateBurnSelectionUI();
  }
}

/**
 * List the tokens that failed to burn under the token list. Symbols come from the
 * scan being replaced by the refresh that follows, so they are read here; the
 * mint is an LTR address island and the reason is the localized failure text.
 */
function renderBurnFailures(failures) {
  const box = $("#burn-failures");
  if (!box) return;
  if (failures.length === 0) {
    box.hidden = true;
    box.replaceChildren();
    return;
  }
  const rows = failures
    .map((failure) => {
      const token = burnTokensState.tokens.find((t) => t.mint === failure.mint);
      const symbol = token?.symbol || I18n.t("format-unknown");
      const reason = failure.error?.text
        ? I18n.text(failure.error.text)
        : I18n.t("tools-burn-failure-unknown");
      const details = failure.error?.details
        ? ` title="${Utils.escapeHtml(failure.error.details)}"`
        : "";
      return `
        <li class="burn-failure-item">
          <span class="burn-failure-symbol">${Utils.escapeHtml(symbol)}</span>
          ${renderAddress(failure.mint)}
          <span class="burn-failure-reason"${details}>${Utils.escapeHtml(reason)}</span>
        </li>`;
    })
    .join("");
  box.innerHTML = `
    <div class="burn-failures-title">
      <i class="icon-triangle-alert"></i>
      ${Utils.escapeHtml(I18n.t("tools-burn-failures-title", { count: failures.length }))}
    </div>
    <ul class="burn-failure-list">${rows}</ul>`;
  box.hidden = false;
}

/**
 * Two-step confirmation dialog. `message` is markup built by the caller from
 * escaped or sanitized text; the other labels are plain text.
 */
function showBurnConfirmation(title, message, confirmText, cancelText, isDanger = false) {
  return new Promise((resolve) => {
    // Create overlay
    const overlay = document.createElement("div");
    overlay.className = "burn-confirm-overlay";
    overlay.innerHTML = `
      <div class="burn-confirm-dialog ${isDanger ? "danger" : ""}">
        <div class="burn-confirm-header">
          <h3>${Utils.escapeHtml(title)}</h3>
        </div>
        <div class="burn-confirm-body">
          ${message}
        </div>
        <div class="burn-confirm-actions">
          <button class="btn" id="burn-confirm-cancel">${Utils.escapeHtml(cancelText)}</button>
          <button class="btn ${isDanger ? "danger" : "primary"}" id="burn-confirm-ok">${Utils.escapeHtml(confirmText)}</button>
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
          <h3><i class="icon-info"></i> <span data-l10n-id="tools-airdrop-about-title"></span></h3>
        </div>
        <div class="section-content">
          <p class="tool-info" data-l10n-id="tools-airdrop-about-body"></p>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-gift"></i> <span data-l10n-id="tools-airdrop-list-title"></span></h3>
        </div>
        <div class="section-content">
          <div class="airdrop-list" id="airdrop-list">
            <div class="empty-state">
              <i class="icon-scan"></i>
              <p data-l10n-id="tools-airdrop-prompt"></p>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;
  I18n.localizeTree(container);

  actionsContainer.innerHTML = `
    <button class="btn primary" id="check-airdrops-btn">
      <i class="icon-scan"></i> <span data-l10n-id="tools-airdrop-action-check"></span>
    </button>
    <button class="btn success" id="claim-all-btn" disabled>
      <i class="icon-gift"></i> <span data-l10n-id="tools-airdrop-action-claim-all"></span>
    </button>
  `;
  I18n.localizeTree(actionsContainer);

  // TODO: Wire up airdrop checker functionality
}

function renderWalletGeneratorTool(container, actionsContainer) {
  const hint = Hints.getHint("tools.walletGenerator");
  const hintHtml = hint ? HintTrigger.render(hint, "tools.walletGenerator", { size: "sm" }) : "";

  container.innerHTML = `
    <div class="tool-panel wallet-generator-tool">
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-settings"></i> <span data-l10n-id="tools-generator-options-title"></span></h3>
          ${hintHtml}
        </div>
        <div class="section-content">
          <div class="warning-box">
            <p><strong data-l10n-id="tools-generator-warning-title"></strong></p>
            <p data-l10n-id="tools-generator-warning-body"></p>
          </div>
          <form class="tool-form" id="generator-form">
            <div class="form-group">
              <label for="wallet-count" data-l10n-id="tools-generator-count-label"></label>
              <input type="number" id="wallet-count" value="1" min="1" max="10" />
            </div>
            <div class="form-group checkbox-group">
              <label>
                <input type="checkbox" id="vanity-enabled" />
                <span data-l10n-id="tools-generator-vanity-label"></span>
              </label>
            </div>
            <div class="form-group" id="vanity-prefix-group" style="display: none;">
              <label for="vanity-prefix" data-l10n-id="tools-generator-prefix-label"></label>
              <input type="text" id="vanity-prefix" data-l10n-id="tools-generator-prefix-input" maxlength="4" />
              <small data-l10n-id="tools-generator-prefix-hint"></small>
            </div>
          </form>
        </div>
      </div>

      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-key"></i> <span data-l10n-id="tools-generator-list-title"></span></h3>
        </div>
        <div class="section-content">
          <div class="generated-wallets" id="generated-wallets">
            <div class="empty-state">
              <i class="icon-key"></i>
              <p data-l10n-id="tools-generator-empty"></p>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;
  I18n.localizeTree(container);

  HintTrigger.initAll();

  actionsContainer.innerHTML = `
    <button class="btn primary" id="generate-wallet-btn">
      <i class="icon-plus"></i> <span data-l10n-id="tools-generator-action-generate"></span>
    </button>
    <button class="btn" id="export-wallets-btn" disabled>
      <i class="icon-download"></i> <span data-l10n-id="common-action-export"></span>
    </button>
  `;
  I18n.localizeTree(actionsContainer);

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
    Utils.showToast(I18n.t("tools-generator-count-invalid"), "error");
    return;
  }

  generateBtn.disabled = true;
  setButton(generateBtn, "icon-loader spin", I18n.t("tools-generator-action-generating"));

  try {
    const response = await fetch("/api/tools/generate-keypairs", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ count }),
    });

    if (!response.ok) {
      throw await responseError(response);
    }
    const result = await response.json().catch(() => ({}));

    // The API returns { success: true, data: [...] }
    const keypairs = result.data || result;

    if (!Array.isArray(keypairs) || keypairs.length === 0) {
      throw new Error(I18n.t("tools-generator-no-keypairs"));
    }

    // Add to our generated wallets list
    generatedWallets = [...generatedWallets, ...keypairs];

    // Render the wallets
    renderGeneratedWallets(walletsContainer);

    // Enable export button
    const exportBtn = $("#export-wallets-btn");
    if (exportBtn) exportBtn.disabled = false;
    Utils.showToast(I18n.t("tools-generator-generated", { count: keypairs.length }), "success");
  } catch (error) {
    console.error("Failed to generate wallets:", error);
    Utils.showToast(I18n.t("tools-generator-failed", { reason: error.message }), "error");
  } finally {
    generateBtn.disabled = false;
    setButton(generateBtn, "icon-plus", I18n.t("tools-generator-action-generate"));
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
        <p data-l10n-id="tools-generator-empty"></p>
      </div>
    `;
    I18n.localizeTree(container);
    return;
  }

  container.innerHTML = generatedWallets
    .map(
      (wallet, index) => `
      <div class="generated-wallet-item">
        <div class="wallet-header">
          <span class="wallet-index">#${index + 1}</span>
          <div class="wallet-actions">
            <button class="btn-icon" data-action="copy-pubkey" data-pubkey="${wallet.pubkey}" data-l10n-id="tools-generator-copy-public-key">
              <i class="icon-copy"></i>
            </button>
            <button class="btn-icon" data-action="copy-secret" data-secret="${wallet.secret}" data-l10n-id="tools-generator-copy-private-key">
              <i class="icon-key"></i>
            </button>
            <button class="btn-icon danger" data-action="remove" data-index="${index}" data-l10n-id="tools-generator-remove">
              <i class="icon-x"></i>
            </button>
          </div>
        </div>
        <div class="wallet-pubkey">
          <span class="label" data-l10n-id="tools-generator-public-key-label"></span>
          <code class="pubkey-value" dir="ltr">${wallet.pubkey}</code>
        </div>
        <div class="wallet-secret">
          <span class="label" data-l10n-id="tools-generator-private-key-label"></span>
          <code class="secret-value masked" dir="ltr">••••••••••••••••</code>
          <button class="btn-icon btn-reveal" data-action="reveal" data-secret="${wallet.secret}" data-l10n-id="tools-generator-reveal">
            <i class="icon-eye"></i>
          </button>
        </div>
      </div>
    `
    )
    .join("");
  I18n.localizeTree(container);
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
      Utils.notifyCopied(I18n.t("tools-generator-public-key-name"));
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
        title: I18n.t("tools-generator-private-key-copied"),
        message: I18n.t("tools-generator-private-key-warning"),
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
    Utils.showToast(I18n.t("tools-generator-export-empty"), "warning");
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

  Utils.showToast(I18n.t("tools-generator-exported"), "warning");
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
          <h3><i class="icon-chart-pie"></i> <span data-l10n-id="tools-consolidation-summary-title"></span></h3>
          ${hintHtml}
        </div>
        <div class="section-content">
          <div class="wc-summary-grid" id="wc-summary-grid">
            <div class="wc-summary-item">
              <span class="wc-summary-value" id="wc-wallet-count">—</span>
              <span class="wc-summary-label" data-l10n-id="tools-consolidation-stat-wallets"></span>
            </div>
            <div class="wc-summary-item">
              <span class="wc-summary-value" id="wc-total-sol">—</span>
              <span class="wc-summary-label" data-l10n-id="tools-consolidation-stat-native"></span>
            </div>
            <div class="wc-summary-item">
              <span class="wc-summary-value" id="wc-total-tokens">—</span>
              <span class="wc-summary-label" data-l10n-id="tools-consolidation-stat-tokens"></span>
            </div>
            <div class="wc-summary-item">
              <span class="wc-summary-value" id="wc-reclaimable">—</span>
              <span class="wc-summary-label" data-l10n-id="tools-consolidation-stat-rent"></span>
            </div>
          </div>
        </div>
      </div>

      <!-- Wallets Table -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-wallet"></i> <span data-l10n-id="tools-consolidation-wallets-title"></span></h3>
          <div class="section-actions">
            <button class="btn btn-sm" id="wc-refresh-btn" type="button">
              <i class="icon-refresh-cw"></i> <span data-l10n-id="common-action-refresh"></span>
            </button>
          </div>
        </div>
        <div class="section-content">
          <div class="wc-wallets-container" id="wc-wallets-container">
            <div class="loading-state">
              <i class="icon-loader spin"></i>
              <p data-l10n-id="tools-consolidation-loading-wallets"></p>
            </div>
          </div>
          <div class="wc-selection-summary" id="wc-selection-summary">
            <!-- Selection summary -->
          </div>
        </div>
      </div>
    </div>
  `;
  I18n.localizeTree(container);

  HintTrigger.initAll();

  actionsContainer.innerHTML = `
    <button class="btn" id="wc-transfer-sol-btn" disabled>
      <i class="icon-arrow-right"></i> <span data-l10n-id="tools-consolidation-action-transfer-native"></span>
    </button>
    <button class="btn" id="wc-transfer-tokens-btn" disabled>
      <i class="icon-send"></i> <span data-l10n-id="tools-consolidation-action-transfer-tokens"></span>
    </button>
    <button class="btn primary" id="wc-cleanup-btn" disabled>
      <i class="icon-trash-2"></i> <span data-l10n-id="tools-consolidation-action-cleanup"></span>
    </button>
  `;
  I18n.localizeTree(actionsContainer);

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
      <p data-l10n-id="tools-consolidation-loading-data"></p>
    </div>
  `;
  I18n.localizeTree(container);

  try {
    const response = await fetch("/api/tools/wallets/summary");
    if (!response.ok) {
      throw await responseError(response);
    }

    const data = await response.json();
    consolidationState.wallets = data.wallets || [];

    // Update summary
    const walletCount = $("#wc-wallet-count");
    const totalSol = $("#wc-total-sol");
    const totalTokens = $("#wc-total-tokens");
    const reclaimable = $("#wc-reclaimable");

    if (walletCount) walletCount.textContent = data.wallet_count || 0;
    if (totalSol) totalSol.textContent = Utils.formatSol(data.total_native || 0);
    if (totalTokens) totalTokens.textContent = data.token_types || 0;
    if (reclaimable) {
      reclaimable.textContent = I18n.t("tools-wallet-amount-approx", {
        amount: Utils.formatSol(data.reclaimable_rent || 0),
      });
    }

    // Render wallets table
    if (consolidationState.wallets.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <i class="icon-wallet"></i>
          <p data-l10n-id="tools-consolidation-empty"></p>
          <small data-l10n-id="tools-consolidation-empty-hint"></small>
        </div>
      `;
      I18n.localizeTree(container);
      return;
    }

    container.innerHTML = `
      <table class="wc-wallet-table">
        <thead>
          <tr>
            <th><input type="checkbox" id="wc-check-all" /></th>
            <th data-l10n-id="tools-consolidation-column-name"></th>
            <th data-l10n-id="tools-consolidation-column-address"></th>
            <th data-l10n-id="tools-consolidation-column-native"></th>
            <th data-l10n-id="tools-consolidation-column-tokens"></th>
            <th data-l10n-id="tools-consolidation-column-atas"></th>
          </tr>
        </thead>
        <tbody>
          ${consolidationState.wallets
            .map(
              (w) => `
            <tr data-address="${w.address}" class="${w.sol_balance === 0 && w.token_count === 0 ? "empty-wallet" : ""}">
              <td><input type="checkbox" class="wc-wallet-check" data-address="${w.address}" /></td>
              <td>${Utils.escapeHtml(w.name)}</td>
              <td class="mono" dir="ltr">${Utils.formatAddressCompact(w.address)}</td>
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
    I18n.localizeTree(container);

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
    container.innerHTML = errorStateHtml(
      I18n.t("tools-consolidation-load-failed", { reason: error.message })
    );
  } finally {
    if (refreshBtn) {
      refreshBtn.disabled = false;
      setButton(refreshBtn, "icon-refresh-cw", I18n.t("common-action-refresh"));
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
      summary.innerHTML = `<span class="text-muted">${Utils.escapeHtml(I18n.t("tools-consolidation-select-prompt"))}</span>`;
    } else {
      summary.innerHTML = `
        <span class="text-primary">${Utils.escapeHtml(I18n.t("tools-wallet-selected", { count: selectedCount }))}</span>
        <span class="text-secondary">${Utils.escapeHtml(
          I18n.t("tools-consolidation-selection-totals", {
            amount: Utils.formatSol(totalSol),
            tokens: totalTokens,
            atas: totalEmptyAtas,
          })
        )}</span>
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
  setButton(transferSolBtn, "icon-loader spin", I18n.t("tools-consolidation-action-transferring"));

  try {
    const response = await fetch("/api/tools/wallets/consolidate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ wallets: addresses, type: "sol" }),
    });

    if (!response.ok) {
      throw await responseError(response);
    }

    const result = await response.json();
    Utils.showToast(
      I18n.t("tools-consolidation-transferred-native", {
        amount: Utils.formatSol(result.total_transferred),
      }),
      "success"
    );
    loadConsolidationData();
  } catch (error) {
    console.error("SOL consolidation failed:", error);
    Utils.showToast(I18n.t("tools-wallet-transfer-failed", { reason: error.message }), "error");
  } finally {
    transferSolBtn.disabled = false;
    setButton(
      transferSolBtn,
      "icon-arrow-right",
      I18n.t("tools-consolidation-action-transfer-native")
    );
  }
}

async function handleConsolidateTokens() {
  const addresses = Array.from(consolidationState.selectedAddresses);
  if (addresses.length === 0) return;

  const transferTokensBtn = $("#wc-transfer-tokens-btn");
  if (!transferTokensBtn) return;

  transferTokensBtn.disabled = true;
  setButton(
    transferTokensBtn,
    "icon-loader spin",
    I18n.t("tools-consolidation-action-transferring")
  );

  try {
    const response = await fetch("/api/tools/wallets/consolidate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ wallets: addresses, type: "tokens" }),
    });

    if (!response.ok) {
      throw await responseError(response);
    }

    const result = await response.json();
    Utils.showToast(
      I18n.t("tools-consolidation-transferred-tokens", { count: result.tokens_transferred }),
      "success"
    );
    loadConsolidationData();
  } catch (error) {
    console.error("Token consolidation failed:", error);
    Utils.showToast(I18n.t("tools-wallet-transfer-failed", { reason: error.message }), "error");
  } finally {
    transferTokensBtn.disabled = false;
    setButton(
      transferTokensBtn,
      "icon-send",
      I18n.t("tools-consolidation-action-transfer-tokens")
    );
  }
}

async function handleConsolidateCleanup() {
  const addresses = Array.from(consolidationState.selectedAddresses);
  if (addresses.length === 0) return;

  const cleanupBtn = $("#wc-cleanup-btn");
  if (!cleanupBtn) return;

  cleanupBtn.disabled = true;
  setButton(cleanupBtn, "icon-loader spin", I18n.t("tools-wallet-cleanup-action-cleaning"));

  try {
    const response = await fetch("/api/tools/wallets/cleanup-atas", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ wallets: addresses }),
    });

    if (!response.ok) {
      throw await responseError(response);
    }

    const result = await response.json();
    Utils.showToast(
      I18n.t("tools-consolidation-cleaned", {
        count: result.atas_closed,
        amount: Utils.formatSol(result.native_reclaimed),
      }),
      "success"
    );
    loadConsolidationData();
  } catch (error) {
    console.error("ATA cleanup failed:", error);
    Utils.showToast(I18n.t("tools-wallet-cleanup-failed", { reason: error.message }), "error");
  } finally {
    cleanupBtn.disabled = false;
    setButton(cleanupBtn, "icon-trash-2", I18n.t("tools-consolidation-action-cleanup"));
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
