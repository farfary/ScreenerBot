/**
 * Multi-Wallet Tools Module
 * Contains multi-wallet trading utilities: multi-buy and multi-sell
 */

import { $, $$, on } from "../../core/dom.js";
import * as Utils from "../../core/utils.js";
import * as Hints from "../../core/hints.js";
import { HintTrigger } from "../../ui/hint_popover.js";
import { enhanceAllSelects } from "../../ui/custom_select.js";
import { apiErrorMessage } from "../../core/request_manager.js";

// Ids are the states of a multi-wallet session.
const SESSION_STATE_LABELS = Object.freeze({
  pending: "tools-multi-state-pending",
  funding: "tools-multi-state-funding",
  executing: "tools-multi-state-executing",
  consolidating: "tools-multi-state-consolidating",
  completed: "tools-multi-state-completed",
  failed: "tools-multi-state-failed",
  aborted: "tools-multi-state-aborted",
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

// =============================================================================
// Multi-Buy Tool
// =============================================================================

let multiBuyState = {
  sessionId: null,
  status: "idle", // idle, running, completed, failed
  walletResults: [],
  poller: null,
};

/**
 * Offer only the routers that are actually enabled in Settings > Swaps.
 * A router the user picks here is used verbatim — nothing silently falls back to
 * another one — so a disabled option would just fail the whole session.
 * @param {HTMLElement} container
 */
async function syncRouterChoices(container) {
  let enabled;
  try {
    const response = await fetch("/api/config/swaps");
    if (!response.ok) return;
    const swaps = (await response.json())?.data;
    if (!swaps) return;
    enabled = {
      jupiter: swaps.jupiter?.enabled === true,
      direct: swaps.direct?.enabled === true,
      raptor: swaps.raptor?.enabled === true,
    };
  } catch (error) {
    // Availability is a refinement: leave every option selectable rather than
    // blocking the tool because one config read failed.
    console.warn("Could not read which swap routers are enabled", error);
    return;
  }

  container.querySelectorAll("select[data-router-choice]").forEach((select) => {
    select.querySelectorAll("option[data-router]").forEach((option) => {
      option.disabled = !enabled[option.dataset.router];
    });
    if (select.selectedOptions[0]?.disabled) {
      // The custom select mirrors the native one; set both so the visible label
      // cannot disagree with what gets submitted.
      select.value = "auto";
      select._customSelectInstance?.setValue("auto", { emitChange: false });
    }
  });
}

function renderBuyMultiWalletsTool(container, actionsContainer) {
  const hint = Hints.getHint("tools.multiBuy");
  const hintHtml = hint ? HintTrigger.render(hint, "tools.multiBuy", { size: "sm" }) : "";

  container.innerHTML = `
    <div class="tool-panel multi-buy-tool">
      <!-- Token Input -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-coins"></i> <span data-l10n-id="tools-multi-token-title"></span></h3>
          ${hintHtml}
        </div>
        <div class="section-content">
          <div class="form-group">
            <label for="mb-token-mint"><span data-l10n-id="tools-multi-mint-label"></span> <span class="required">*</span></label>
            <input dir="ltr" type="text" id="mb-token-mint" data-l10n-id="tools-multi-mint-input" />
            <small data-l10n-id="tools-multi-buy-mint-hint"></small>
          </div>
        </div>
      </div>

      <!-- Wallet Settings -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-wallet"></i> <span data-l10n-id="tools-multi-buy-wallets-title"></span></h3>
        </div>
        <div class="section-content">
          <form class="tool-form" id="mb-wallet-form">
            <div class="form-row">
              <div class="form-group">
                <label for="mb-wallet-count" data-l10n-id="tools-multi-buy-wallet-count-label"></label>
                <select id="mb-wallet-count" data-custom-select>
                  <option value="2" data-l10n-id="tools-multi-buy-wallet-count-option" data-l10n-args='{"count":2}'></option>
                  <option value="3" data-l10n-id="tools-multi-buy-wallet-count-option" data-l10n-args='{"count":3}'></option>
                  <option value="4" data-l10n-id="tools-multi-buy-wallet-count-option" data-l10n-args='{"count":4}'></option>
                  <option value="5" data-l10n-id="tools-multi-buy-wallet-count-option" data-l10n-args='{"count":5}' selected></option>
                  <option value="6" data-l10n-id="tools-multi-buy-wallet-count-option" data-l10n-args='{"count":6}'></option>
                  <option value="8" data-l10n-id="tools-multi-buy-wallet-count-option" data-l10n-args='{"count":8}'></option>
                  <option value="10" data-l10n-id="tools-multi-buy-wallet-count-option" data-l10n-args='{"count":10}'></option>
                </select>
                <small data-l10n-id="tools-multi-buy-wallet-count-hint"></small>
              </div>
              <div class="form-group">
                <label for="mb-sol-buffer" data-l10n-id="tools-multi-buy-buffer-label"></label>
                <input type="number" id="mb-sol-buffer" value="0.015" min="0.005" step="0.005" />
                <small data-l10n-id="tools-multi-buy-buffer-hint"></small>
              </div>
            </div>
          </form>
        </div>
      </div>

      <!-- Amount Settings -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-dollar-sign"></i> <span data-l10n-id="tools-multi-buy-amounts-title"></span></h3>
        </div>
        <div class="section-content">
          <form class="tool-form">
            <div class="form-row">
              <div class="form-group">
                <label for="mb-min-sol" data-l10n-id="tools-multi-buy-min-label"></label>
                <input type="number" id="mb-min-sol" value="0.01" min="0.001" step="0.01" />
                <small data-l10n-id="tools-multi-buy-min-hint"></small>
              </div>
              <div class="form-group">
                <label for="mb-max-sol" data-l10n-id="tools-multi-buy-max-label"></label>
                <input type="number" id="mb-max-sol" value="0.05" min="0.001" step="0.01" />
                <small data-l10n-id="tools-multi-buy-max-hint"></small>
              </div>
              <div class="form-group">
                <label for="mb-total-limit" data-l10n-id="tools-multi-buy-limit-label"></label>
                <input type="number" id="mb-total-limit" placeholder="—" min="0" step="0.1" />
                <small data-l10n-id="tools-multi-buy-limit-hint"></small>
              </div>
            </div>
          </form>
        </div>
      </div>

      <!-- Execution Settings -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-settings"></i> <span data-l10n-id="tools-multi-execution-title"></span></h3>
        </div>
        <div class="section-content">
          <form class="tool-form">
            <div class="form-row">
              <div class="form-group">
                <label for="mb-delay-min" data-l10n-id="tools-multi-delay-min-label"></label>
                <input type="number" id="mb-delay-min" value="1000" min="500" step="100" />
              </div>
              <div class="form-group">
                <label for="mb-delay-max" data-l10n-id="tools-multi-delay-max-label"></label>
                <input type="number" id="mb-delay-max" value="2000" min="500" step="100" />
              </div>
              <div class="form-group">
                <label for="mb-concurrency" data-l10n-id="tools-multi-concurrency-label"></label>
                <select id="mb-concurrency" data-custom-select>
                  <option value="1" data-l10n-id="tools-multi-concurrency-sequential" data-l10n-args='{"count":1}' selected></option>
                  <option value="2" data-l10n-id="tools-multi-concurrency-parallel" data-l10n-args='{"count":2}'></option>
                  <option value="3" data-l10n-id="tools-multi-concurrency-parallel" data-l10n-args='{"count":3}'></option>
                </select>
              </div>
            </div>
            <div class="form-row">
              <div class="form-group">
                <label for="mb-slippage" data-l10n-id="tools-multi-slippage-label"></label>
                <input type="number" id="mb-slippage" value="5" min="0.5" max="50" step="0.5" />
              </div>
              <div class="form-group">
                <label for="mb-router" data-l10n-id="tools-multi-router-label"></label>
                <select id="mb-router" data-custom-select data-router-choice>
                  <option value="auto" data-l10n-id="tools-multi-router-auto" selected></option>
                  <option value="jupiter" data-router="jupiter" data-l10n-id="tools-multi-router-jupiter"></option>
                  <option value="direct" data-router="direct" data-l10n-id="tools-multi-router-direct"></option>
                  <option value="raptor" data-router="raptor" data-l10n-id="tools-multi-router-raptor"></option>
                </select>
              </div>
            </div>
          </form>
        </div>
      </div>

      <!-- Preview Section -->
      <div class="tool-section" id="mb-preview-section" style="display: none;">
        <div class="section-header">
          <h3><i class="icon-eye"></i> <span data-l10n-id="tools-multi-buy-preview-title"></span></h3>
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
          <h3><i class="icon-activity"></i> <span data-l10n-id="tools-multi-progress-title"></span></h3>
        </div>
        <div class="section-content">
          <div class="mw-progress-container">
            <div class="mw-progress-bar-wrapper">
              <div class="mw-progress-bar">
                <div class="mw-progress-fill" id="mb-progress-fill" style="width: 0%"></div>
              </div>
              <span class="mw-progress-percent" id="mb-progress-percent">0%</span>
            </div>
            <div class="mw-progress-status" id="mb-progress-status" data-l10n-id="tools-multi-progress-preparing"></div>
          </div>
          <div class="mw-results-table" id="mb-results-table">
            <!-- Results populated dynamically -->
          </div>
        </div>
      </div>
    </div>
  `;

  I18n.localizeTree(container);

  HintTrigger.initAll();
  enhanceAllSelects(container);
  void syncRouterChoices(container);

  actionsContainer.innerHTML = `
    <button class="btn" id="mb-preview-btn">
      <i class="icon-eye"></i> <span data-l10n-id="tools-multi-buy-action-preview"></span>
    </button>
    <button class="btn success" id="mb-start-btn" disabled>
      <i class="icon-shopping-cart"></i> <span data-l10n-id="tools-multi-buy-action-start"></span>
    </button>
    <button class="btn danger" id="mb-stop-btn" style="display: none;">
      <i class="icon-x"></i> <span data-l10n-id="tools-multi-action-stop"></span>
    </button>
  `;
  I18n.localizeTree(actionsContainer);

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
    Utils.showToast(I18n.t("tools-validation-mint-required"), "error");
    return;
  }

  const previewBtn = $("#mb-preview-btn");
  const previewSection = $("#mb-preview-section");
  const previewGrid = $("#mb-preview-grid");
  const startBtn = $("#mb-start-btn");

  if (!previewBtn || !previewSection || !previewGrid) return;

  previewBtn.disabled = true;
  setButton(previewBtn, "icon-loader spin", I18n.t("tools-multi-action-loading"));

  const config = {
    token_mint: tokenMint,
    wallet_count: parseInt($("#mb-wallet-count")?.value || "5"),
    sol_buffer: parseFloat($("#mb-sol-buffer")?.value || "0.015"),
    min_amount_sol: parseFloat($("#mb-min-sol")?.value || "0.01"),
    max_amount_sol: parseFloat($("#mb-max-sol")?.value || "0.05"),
    total_sol_limit: parseFloat($("#mb-total-limit")?.value) || null,
  };

  try {
    const response = await fetch("/api/tools/multi-buy/preview", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config),
    });

    if (!response.ok) {
      throw await responseError(response);
    }

    const preview = await response.json();

    previewSection.style.display = "block";
    const range = I18n.t("tools-multi-buy-preview-range", {
      min: Utils.formatSol(config.min_amount_sol),
      max: Utils.formatSol(config.max_amount_sol),
    });
    previewGrid.innerHTML = `
      <div class="mw-preview-item">
        <span class="mw-preview-label" data-l10n-id="tools-multi-buy-preview-create"></span>
        <span class="mw-preview-value">${preview.wallets_to_create}</span>
      </div>
      <div class="mw-preview-item">
        <span class="mw-preview-label" data-l10n-id="tools-multi-buy-preview-amount"></span>
        <span class="mw-preview-value">${Utils.escapeHtml(range)}</span>
      </div>
      <div class="mw-preview-item">
        <span class="mw-preview-label" data-l10n-id="tools-multi-buy-preview-total"></span>
        <span class="mw-preview-value">${Utils.formatSol(preview.total_sol_needed)}</span>
      </div>
      <div class="mw-preview-item ${preview.can_proceed ? "success" : "error"}">
        <span class="mw-preview-label" data-l10n-id="tools-multi-buy-preview-balance"></span>
        <span class="mw-preview-value">${Utils.formatSol(preview.main_wallet_balance)} ${preview.can_proceed ? "✓" : "✗"}</span>
      </div>
    `;
    I18n.localizeTree(previewGrid);

    if (preview.warning) {
      Utils.showToast(I18n.text(preview.warning), "warning");
    }

    if (startBtn) {
      startBtn.disabled = !preview.can_proceed;
    }
  } catch (error) {
    console.error("Multi-buy preview failed:", error);
    Utils.showToast(I18n.t("tools-multi-buy-preview-failed", { reason: error.message }), "error");
    previewSection.style.display = "none";
  } finally {
    previewBtn.disabled = false;
    setButton(previewBtn, "icon-eye", I18n.t("tools-multi-buy-action-preview"));
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
    min_amount_sol: parseFloat($("#mb-min-sol")?.value || "0.01"),
    max_amount_sol: parseFloat($("#mb-max-sol")?.value || "0.05"),
    total_sol_limit: parseFloat($("#mb-total-limit")?.value) || null,
    delay_ms: parseInt($("#mb-delay-min")?.value || "1000"),
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
      throw await responseError(response);
    }

    const result = await response.json();
    multiBuyState.sessionId = result.session_id;
    multiBuyState.status = "running";

    // Start polling for status
    startMultiBuyPolling();
    Utils.showToast(I18n.t("tools-multi-buy-started"), "success");
  } catch (error) {
    console.error("Multi-buy start failed:", error);
    Utils.showToast(I18n.t("tools-multi-start-failed", { reason: error.message }), "error");
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

      // `is_complete` covers aborted runs too, which report a "completed"
      // status with the reason in `error`.
      if (status.is_complete) {
        stopMultiBuyPolling();
        multiBuyState.status = status.status;
        Utils.showToast(
          apiErrorMessage({ error: status.error }, null) ||
            I18n.t("tools-multi-buy-completed", {
              successful: status.successful_ops,
              total: status.total_wallets,
            }),
          status.error ? "error" : "success"
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

  const { completed, percent } = sessionProgress(status);

  if (progressFill) progressFill.style.width = `${percent}%`;
  if (progressPercent) progressPercent.textContent = `${percent}%`;
  if (progressStatus) {
    progressStatus.textContent = sessionStatusLine(
      status,
      I18n.t("tools-multi-buy-executing"),
      completed,
      status.total_wallets
    );
  }

  if (resultsTable && status.operations) {
    resultsTable.innerHTML = renderOperationRows(status.operations, [
      {
        head: I18n.t("tools-multi-column-wallet"),
        cell: (op) => Utils.formatAddressCompact(op.wallet_address),
        mono: true,
        ltr: true,
      },
      {
        head: I18n.t("tools-multi-buy-column-spent"),
        cell: (op) => Utils.formatSol(op.amount_sol, { suffix: "" }),
        mono: true,
      },
      {
        head: I18n.t("tools-multi-buy-column-tokens"),
        cell: (op) => (op.token_amount ? Utils.formatNumber(op.token_amount) : "—"),
        mono: true,
      },
    ]);
  }
}

/**
 * How far a multi-wallet session has got. The backend counts completed
 * operations, not attempts, and only knows the real total once it finalizes —
 * so the planned count is the denominator until then.
 * @param {{successful_ops?: number, failed_ops?: number, total_wallets?: number}} status
 */
function sessionProgress(status) {
  const completed = (status.successful_ops || 0) + (status.failed_ops || 0);
  const total = Math.max(status.total_wallets || 0, completed);
  return { completed, percent: total > 0 ? Math.round((completed / total) * 100) : 0 };
}

/**
 * The status line under the progress bar. While operations run it names the
 * tool's own activity; every other session state has a catalog label.
 * @param {{status?: string}} status
 * @param {string} executingLabel What to say while operations are running
 * @param {number} completed
 * @param {number} total
 */
function sessionStatusLine(status, executingLabel, completed, total) {
  const state = status.status || "";
  let label = "";
  if (state === "executing") {
    label = executingLabel;
  } else if (state) {
    label = I18n.label(SESSION_STATE_LABELS, state);
  }
  return I18n.t("tools-multi-status-line", { label, completed, total: total || completed });
}

/**
 * One results table for both tools: the operation columns differ, the route and
 * outcome columns never do. The route is the router that actually executed plus
 * the venue it traded on — with "Auto" that is the only place the answer shows.
 * @param {Array<Object>} operations
 * @param {Array<{head: string, cell: (op: Object) => string, mono?: boolean, ltr?: boolean}>} columns
 */
function renderOperationRows(operations, columns) {
  const head = columns.map((column) => `<th>${Utils.escapeHtml(column.head)}</th>`).join("");
  const rows = operations
    .map((op) => {
      const state = op.success ? "success" : "failed";
      const cells = columns
        .map(
          (column) =>
            `<td class="${column.mono ? "mono" : ""}"${column.ltr ? ' dir="ltr"' : ""}>${Utils.escapeHtml(String(column.cell(op)))}</td>`
        )
        .join("");
      const route = [op.router, op.venue].filter(Boolean).join(" · ") || "—";
      // The badge stays a one-word verdict; a failure reason is prose and belongs
      // under it, not squeezed into a capitalized pill.
      const reason =
        !op.success && op.error
          ? `<div class="mw-op-error">${Utils.escapeHtml(apiErrorMessage({ error: op.error }, ""))}</div>`
          : "";
      return `
            <tr class="${state}">
              ${cells}
              <td>${Utils.escapeHtml(route)}</td>
              <td>
                <span class="mw-status-badge ${state}">${Utils.escapeHtml(op.success ? I18n.t("tools-multi-op-completed") : I18n.t("tools-multi-op-failed"))}</span>
                ${reason}
              </td>
            </tr>`;
    })
    .join("");

  return `
      <table class="mw-results">
        <thead>
          <tr>${head}<th>${Utils.escapeHtml(I18n.t("tools-multi-column-route"))}</th><th>${Utils.escapeHtml(I18n.t("tools-multi-column-status"))}</th></tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    `;
}

async function handleMultiBuyStop() {
  if (!multiBuyState.sessionId) return;

  try {
    await fetch(`/api/tools/multi-buy/${multiBuyState.sessionId}/abort`, { method: "POST" });
    stopMultiBuyPolling();
    Utils.showToast(I18n.t("tools-multi-buy-stopped"), "info");
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
          <h3><i class="icon-coins"></i> <span data-l10n-id="tools-multi-token-title"></span></h3>
          ${hintHtml}
        </div>
        <div class="section-content">
          <div class="form-group">
            <label for="ms-token-mint"><span data-l10n-id="tools-multi-mint-label"></span> <span class="required">*</span></label>
            <div class="input-group">
              <input dir="ltr" type="text" id="ms-token-mint" data-l10n-id="tools-multi-mint-input" />
              <button class="btn" id="ms-scan-btn" type="button">
                <i class="icon-search"></i> <span data-l10n-id="tools-multi-sell-action-scan"></span>
              </button>
            </div>
            <small data-l10n-id="tools-multi-sell-mint-hint"></small>
          </div>
        </div>
      </div>

      <!-- Sell Settings -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-settings"></i> <span data-l10n-id="tools-multi-sell-settings-title"></span></h3>
        </div>
        <div class="section-content">
          <form class="tool-form">
            <div class="form-row">
              <div class="form-group">
                <label for="ms-sell-percent" data-l10n-id="tools-multi-sell-percent-label"></label>
                <input type="number" id="ms-sell-percent" value="100" min="1" max="100" step="1" />
                <small data-l10n-id="tools-multi-sell-percent-hint"></small>
              </div>
              <div class="form-group">
                <label for="ms-min-sol-fee" data-l10n-id="tools-multi-sell-min-fee-label"></label>
                <input type="number" id="ms-min-sol-fee" value="0.01" min="0.005" step="0.005" />
                <small data-l10n-id="tools-multi-sell-min-fee-hint"></small>
              </div>
            </div>
            <div class="form-group checkbox-group">
              <label>
                <input type="checkbox" id="ms-auto-topup" checked />
                <span data-l10n-id="tools-multi-sell-topup-label"></span>
              </label>
              <small data-l10n-id="tools-multi-sell-topup-hint"></small>
            </div>
          </form>
        </div>
      </div>

      <!-- Post-Sell Actions -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-arrow-right"></i> <span data-l10n-id="tools-multi-sell-post-title"></span></h3>
        </div>
        <div class="section-content">
          <form class="tool-form">
            <div class="form-group checkbox-group">
              <label>
                <input type="checkbox" id="ms-consolidate" checked />
                <span data-l10n-id="tools-multi-sell-consolidate-label"></span>
              </label>
              <small data-l10n-id="tools-multi-sell-consolidate-hint"></small>
            </div>
            <div class="form-group checkbox-group">
              <label>
                <input type="checkbox" id="ms-close-atas" checked />
                <span data-l10n-id="tools-multi-sell-close-atas-label"></span>
              </label>
              <small data-l10n-id="tools-multi-sell-close-atas-hint"></small>
            </div>
          </form>
        </div>
      </div>

      <!-- Execution Settings -->
      <div class="tool-section">
        <div class="section-header">
          <h3><i class="icon-zap"></i> <span data-l10n-id="tools-multi-execution-title"></span></h3>
        </div>
        <div class="section-content">
          <form class="tool-form">
            <div class="form-row">
              <div class="form-group">
                <label for="ms-delay-min" data-l10n-id="tools-multi-delay-min-label"></label>
                <input type="number" id="ms-delay-min" value="1000" min="500" step="100" />
              </div>
              <div class="form-group">
                <label for="ms-delay-max" data-l10n-id="tools-multi-delay-max-label"></label>
                <input type="number" id="ms-delay-max" value="2000" min="500" step="100" />
              </div>
              <div class="form-group">
                <label for="ms-concurrency" data-l10n-id="tools-multi-concurrency-label"></label>
                <select id="ms-concurrency" data-custom-select>
                  <option value="1" data-l10n-id="tools-multi-concurrency-sequential" data-l10n-args='{"count":1}' selected></option>
                  <option value="2" data-l10n-id="tools-multi-concurrency-parallel" data-l10n-args='{"count":2}'></option>
                  <option value="3" data-l10n-id="tools-multi-concurrency-parallel" data-l10n-args='{"count":3}'></option>
                </select>
              </div>
            </div>
            <div class="form-row">
              <div class="form-group">
                <label for="ms-slippage" data-l10n-id="tools-multi-slippage-label"></label>
                <input type="number" id="ms-slippage" value="5" min="0.5" max="50" step="0.5" />
              </div>
              <div class="form-group">
                <label for="ms-router" data-l10n-id="tools-multi-router-label"></label>
                <select id="ms-router" data-custom-select data-router-choice>
                  <option value="auto" data-l10n-id="tools-multi-router-auto" selected></option>
                  <option value="jupiter" data-router="jupiter" data-l10n-id="tools-multi-router-jupiter"></option>
                  <option value="direct" data-router="direct" data-l10n-id="tools-multi-router-direct"></option>
                  <option value="raptor" data-router="raptor" data-l10n-id="tools-multi-router-raptor"></option>
                </select>
              </div>
            </div>
          </form>
        </div>
      </div>

      <!-- Wallets with Token -->
      <div class="tool-section" id="ms-wallets-section" style="display: none;">
        <div class="section-header">
          <h3><i class="icon-wallet"></i> <span data-l10n-id="tools-multi-sell-wallets-title"></span></h3>
          <div class="section-actions">
            <button class="btn btn-sm" id="ms-select-all-btn" type="button" data-l10n-id="common-action-select-all"></button>
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
          <h3><i class="icon-activity"></i> <span data-l10n-id="tools-multi-progress-title"></span></h3>
        </div>
        <div class="section-content">
          <div class="mw-progress-container">
            <div class="mw-progress-bar-wrapper">
              <div class="mw-progress-bar">
                <div class="mw-progress-fill" id="ms-progress-fill" style="width: 0%"></div>
              </div>
              <span class="mw-progress-percent" id="ms-progress-percent">0%</span>
            </div>
            <div class="mw-progress-status" id="ms-progress-status" data-l10n-id="tools-multi-progress-preparing"></div>
          </div>
          <div class="mw-results-table" id="ms-results-table">
            <!-- Results populated dynamically -->
          </div>
        </div>
      </div>
    </div>
  `;

  I18n.localizeTree(container);

  HintTrigger.initAll();
  enhanceAllSelects(container);
  void syncRouterChoices(container);

  actionsContainer.innerHTML = `
    <button class="btn success" id="ms-start-btn" disabled>
      <i class="icon-package"></i> <span data-l10n-id="tools-multi-sell-action-start"></span>
    </button>
    <button class="btn danger" id="ms-stop-btn" style="display: none;">
      <i class="icon-x"></i> <span data-l10n-id="tools-multi-action-stop"></span>
    </button>
  `;
  I18n.localizeTree(actionsContainer);

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
    Utils.showToast(I18n.t("tools-validation-mint-required"), "error");
    return;
  }

  const scanBtn = $("#ms-scan-btn");
  const walletsSection = $("#ms-wallets-section");
  const walletList = $("#ms-wallet-list");

  if (!scanBtn || !walletsSection || !walletList) return;

  scanBtn.disabled = true;
  const spinner = document.createElement("i");
  spinner.className = "icon-loader spin";
  scanBtn.replaceChildren(spinner);

  try {
    // The preview endpoint IS the scan: it reports every sub-wallet holding the
    // mint, with balances already converted to token units.
    const response = await fetch("/api/tools/multi-sell/preview", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        token_mint: tokenMint,
        sell_percentage: parseFloat($("#ms-sell-percent")?.value || "100"),
      }),
    });
    if (!response.ok) {
      throw await responseError(response);
    }

    const data = await response.json();

    if (data.wallets.length === 0) {
      walletList.innerHTML = `
        <div class="empty-state">
          <i class="icon-inbox"></i>
          <p data-l10n-id="tools-multi-sell-empty"></p>
        </div>
      `;
      I18n.localizeTree(walletList);
      walletsSection.style.display = "block";
      return;
    }

    walletList.innerHTML = `
      <table class="mw-wallet-table">
        <thead>
          <tr>
            <th><input type="checkbox" id="ms-check-all" checked /></th>
            <th data-l10n-id="tools-multi-column-wallet"></th>
            <th data-l10n-id="tools-multi-sell-column-tokens"></th>
            <th data-l10n-id="tools-multi-sell-column-sol"></th>
            <th data-l10n-id="tools-multi-sell-column-topup"></th>
          </tr>
        </thead>
        <tbody>
          ${data.wallets
            .map(
              (w) => `
            <tr data-wallet="${w.address}">
              <td><input type="checkbox" class="ms-wallet-check" data-wallet-id="${w.wallet_id}" checked /></td>
              <td>${Utils.escapeHtml(w.wallet_name)}</td>
              <td class="mono">${Utils.formatNumber(w.token_balance)}</td>
              <td class="mono">${Utils.formatSol(w.sol_balance, { suffix: "" })}</td>
              <td>${w.needs_sol_topup ? `<span class="warning">${Utils.escapeHtml(I18n.t("format-yes"))}</span>` : `<span class="success">${Utils.escapeHtml(I18n.t("format-no"))}</span>`}</td>
            </tr>
          `
            )
            .join("")}
        </tbody>
      </table>
    `;

    I18n.localizeTree(walletList);

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
    Utils.showToast(I18n.t("tools-wallet-scan-failed", { reason: error.message }), "error");
  } finally {
    scanBtn.disabled = false;
    setButton(scanBtn, "icon-search", I18n.t("tools-multi-sell-action-scan"));
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
      summary.innerHTML = `<span class="text-muted">${Utils.escapeHtml(I18n.t("tools-multi-sell-none-selected"))}</span>`;
    } else {
      summary.innerHTML = `<span class="text-primary">${Utils.escapeHtml(I18n.t("tools-wallet-selected", { count: selectedCount }))}</span>`;
    }
  }

  if (startBtn) {
    startBtn.disabled = selectedCount === 0;
  }
}

async function handleMultiSellStart() {
  const tokenMint = $("#ms-token-mint")?.value?.trim();
  if (!tokenMint) return;

  const selectedWallets = Array.from($$(".ms-wallet-check:checked")).map((c) =>
    Number(c.dataset.walletId)
  );
  if (selectedWallets.length === 0) {
    Utils.showToast(I18n.t("tools-multi-sell-select-required"), "error");
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
    wallet_ids: selectedWallets,
    sell_percentage: parseFloat($("#ms-sell-percent")?.value || "100"),
    min_sol_for_fee: parseFloat($("#ms-min-sol-fee")?.value || "0.01"),
    auto_topup: $("#ms-auto-topup")?.checked ?? true,
    consolidate_after: $("#ms-consolidate")?.checked ?? true,
    close_atas_after: $("#ms-close-atas")?.checked ?? true,
    delay_ms: parseInt($("#ms-delay-min")?.value || "1000"),
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
      throw await responseError(response);
    }

    const result = await response.json();
    multiSellState.sessionId = result.session_id;
    multiSellState.status = "running";

    startMultiSellPolling();
    Utils.showToast(I18n.t("tools-multi-sell-started"), "success");
  } catch (error) {
    console.error("Multi-sell start failed:", error);
    Utils.showToast(I18n.t("tools-multi-start-failed", { reason: error.message }), "error");
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

      // `is_complete` covers aborted runs too, which report a "completed"
      // status with the reason in `error`.
      if (status.is_complete) {
        stopMultiSellPolling();
        multiSellState.status = status.status;
        Utils.showToast(
          apiErrorMessage({ error: status.error }, null) ||
            I18n.t("tools-multi-sell-completed", {
              amount: Utils.formatSol(status.total_sol_recovered),
            }),
          status.error ? "error" : "success"
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

  const { completed, percent } = sessionProgress(status);

  if (progressFill) progressFill.style.width = `${percent}%`;
  if (progressPercent) progressPercent.textContent = `${percent}%`;
  if (progressStatus) {
    progressStatus.textContent = sessionStatusLine(
      status,
      I18n.t("tools-multi-sell-executing"),
      completed,
      status.total_wallets
    );
  }

  if (resultsTable && status.operations) {
    resultsTable.innerHTML = renderOperationRows(status.operations, [
      {
        head: I18n.t("tools-multi-column-wallet"),
        cell: (op) => Utils.formatAddressCompact(op.wallet_address),
        mono: true,
        ltr: true,
      },
      {
        head: I18n.t("tools-multi-sell-column-sold"),
        cell: (op) => (op.token_amount ? Utils.formatNumber(op.token_amount) : "—"),
        mono: true,
      },
      {
        head: I18n.t("tools-multi-sell-column-received"),
        cell: (op) => Utils.formatSol(op.amount_sol, { suffix: "" }),
        mono: true,
      },
    ]);
  }
}

async function handleMultiSellStop() {
  if (!multiSellState.sessionId) return;

  try {
    await fetch(`/api/tools/multi-sell/${multiSellState.sessionId}/abort`, { method: "POST" });
    stopMultiSellPolling();
    Utils.showToast(I18n.t("tools-multi-sell-stopped"), "info");
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
