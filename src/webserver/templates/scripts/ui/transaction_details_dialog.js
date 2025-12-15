/**
 * Transaction Details Dialog
 * Full-screen dialog showing comprehensive transaction information with multiple tabs
 */
import * as Utils from "../core/utils.js";
import { createFocusTrap } from "../core/utils.js";
import { requestManager } from "../core/request_manager.js";

export class TransactionDetailsDialog {
  constructor(options = {}) {
    this.onClose = options.onClose || (() => {});
    this.dialogEl = null;
    this.currentTab = "overview";
    this.transactionData = null;
    this.fullTransactionData = null;
    this.isLoading = false;
    this.logSearchQuery = "";
    this._focusTrap = null;
  }

  /**
   * Show dialog with transaction data
   * @param {Object} txData - Basic transaction data (at minimum needs signature)
   */
  async show(txData) {
    if (!txData || !txData.signature) {
      console.error("Invalid transaction data provided to TransactionDetailsDialog");
      return;
    }

    if (this.dialogEl) {
      this.close();
      await new Promise((resolve) => setTimeout(resolve, 350));
    }

    this.transactionData = txData;
    this.fullTransactionData = null;
    this.currentTab = "overview";
    this.logSearchQuery = "";

    this._createDialog();
    this._attachEventHandlers();

    requestAnimationFrame(() => {
      if (this.dialogEl) {
        this.dialogEl.classList.add("active");
        // Add ARIA attributes for accessibility
        const container = this.dialogEl.querySelector(".dialog-container");
        if (container) {
          container.setAttribute("role", "dialog");
          container.setAttribute("aria-modal", "true");
          container.setAttribute("aria-labelledby", "txd-dialog-title");
        }
        // Activate focus trap
        this._focusTrap = createFocusTrap(this.dialogEl);
        this._focusTrap.activate();
      }
    });

    // Fetch full transaction details
    this._fetchFullTransaction();
  }

  async _fetchFullTransaction() {
    if (this.isLoading) return;
    this.isLoading = true;

    try {
      const data = await requestManager.fetch(
        `/api/transactions/${this.transactionData.signature}`,
        {
          priority: "high",
        }
      );
      this.fullTransactionData = data;
      this._updateDialogContent();
    } catch (error) {
      console.error("Error loading transaction details:", error);
      this._showError("Failed to load transaction details");
    } finally {
      this.isLoading = false;
    }
  }

  _showError(message) {
    const content = this.dialogEl?.querySelector(".tab-content.active");
    if (content) {
      content.innerHTML = `<div class="error-state"><i class="icon-alert-circle"></i><p>${Utils.escapeHtml(message)}</p></div>`;
    }
  }

  _updateDialogContent() {
    if (!this.fullTransactionData) return;
    this._updateHeader();
    this._loadTabContent(this.currentTab);
  }

  close() {
    if (!this.dialogEl) return;

    // Deactivate focus trap
    if (this._focusTrap) {
      this._focusTrap.deactivate();
      this._focusTrap = null;
    }

    this.dialogEl.classList.remove("active");

    setTimeout(() => {
      if (this._escapeHandler) {
        document.removeEventListener("keydown", this._escapeHandler);
        this._escapeHandler = null;
      }

      if (this.dialogEl) {
        if (this._closeHandler) {
          const closeBtn = this.dialogEl.querySelector(".dialog-close");
          if (closeBtn) {
            closeBtn.removeEventListener("click", this._closeHandler);
          }
          this._closeHandler = null;
        }

        if (this._backdropHandler) {
          const backdrop = this.dialogEl.querySelector(".dialog-backdrop");
          if (backdrop) {
            backdrop.removeEventListener("click", this._backdropHandler);
          }
          this._backdropHandler = null;
        }

        if (this._tabHandlers) {
          this._tabHandlers.forEach(({ element, handler }) => {
            element.removeEventListener("click", handler);
          });
          this._tabHandlers = null;
        }

        this.dialogEl.remove();
        this.dialogEl = null;
      }

      this.transactionData = null;
      this.fullTransactionData = null;
      this.currentTab = "overview";
      this.isLoading = false;
      this.logSearchQuery = "";

      this.onClose();
    }, 300);
  }

  _createDialog() {
    this.dialogEl = document.createElement("div");
    this.dialogEl.className = "transaction-details-dialog";
    this.dialogEl.innerHTML = this._getDialogHTML();
    document.body.appendChild(this.dialogEl);
  }

  _getDialogHTML() {
    const tx = this.transactionData;
    const typeLabel = this._getTypeLabel(tx.transaction_type);
    const statusBadge = this._getStatusBadge(tx.status, tx.success);

    return `
      <div class="dialog-backdrop"></div>
      <div class="dialog-container">
        <div class="dialog-header">
          <div class="header-top-row">
            <div class="header-left">
              <div class="header-icon">
                <i class="${this._getTypeIcon(tx.transaction_type)}"></i>
              </div>
              <div class="header-title">
                <span class="title-main">${typeLabel}</span>
                <span class="title-sub mono-text" id="headerSignature">${Utils.escapeHtml(tx.signature)}</span>
              </div>
            </div>
            <div class="header-center">
              <div class="header-badges" id="headerBadges">
                ${statusBadge}
                ${this._getDirectionBadge(tx.direction)}
              </div>
            </div>
            <div class="header-right">
              <div class="header-actions">
                <button class="action-btn" id="copySignatureBtn" title="Copy Signature">
                  <i class="icon-copy"></i>
                </button>
                <a href="https://solscan.io/tx/${Utils.escapeHtml(tx.signature)}" target="_blank" class="action-btn" title="View on Solscan">
                  <i class="icon-external-link"></i>
                </a>
                <a href="https://solana.fm/tx/${Utils.escapeHtml(tx.signature)}" target="_blank" class="action-btn" title="View on Solana FM">
                  <i class="icon-external-link"></i>
                </a>
              </div>
              <button class="dialog-close" type="button" title="Close (ESC)">
                <i class="icon-x"></i>
              </button>
            </div>
          </div>
          <div class="header-meta-row" id="headerMetaRow">
            <span class="meta-item" id="metaTimestamp"><i class="icon-clock"></i> <span>—</span></span>
            <span class="meta-item" id="metaSlot"><i class="icon-layers"></i> Slot: <span>—</span></span>
            <span class="meta-item" id="metaFee"><i class="icon-zap"></i> Fee: <span>—</span></span>
          </div>
        </div>

        <div class="dialog-tabs">
          <button class="tab-button active" data-tab="overview">
            <i class="icon-info"></i>
            Overview
          </button>
          <button class="tab-button" data-tab="balances">
            <i class="icon-wallet"></i>
            Balances
          </button>
          <button class="tab-button" data-tab="instructions">
            <i class="icon-code"></i>
            Instructions
            <span class="tab-badge" id="instructionsBadge">${tx.instructions_count || 0}</span>
          </button>
          <button class="tab-button" data-tab="logs">
            <i class="icon-file-text"></i>
            Logs
            <span class="tab-badge" id="logsBadge">0</span>
          </button>
          <button class="tab-button" data-tab="ata">
            <i class="icon-layers"></i>
            ATA
          </button>
          <button class="tab-button" data-tab="raw">
            <i class="icon-braces"></i>
            Raw
          </button>
        </div>

        <div class="dialog-body">
          <div class="tab-content active" data-tab-content="overview">
            <div class="loading-spinner">Loading transaction details...</div>
          </div>
          <div class="tab-content" data-tab-content="balances">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="instructions">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="logs">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="ata">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="raw">
            <div class="loading-spinner">Loading...</div>
          </div>
        </div>
      </div>
    `;
  }

  _updateHeader() {
    const tx = this.fullTransactionData;
    if (!tx) return;

    const badgesEl = this.dialogEl?.querySelector("#headerBadges");
    if (badgesEl) {
      badgesEl.innerHTML = `
        ${this._getStatusBadge(tx.status, tx.success)}
        ${this._getDirectionBadge(tx.direction)}
      `;
    }

    const instructionsBadge = this.dialogEl?.querySelector("#instructionsBadge");
    if (instructionsBadge) {
      instructionsBadge.textContent = tx.instructions_count || tx.instructions?.length || 0;
    }

    const logsBadge = this.dialogEl?.querySelector("#logsBadge");
    if (logsBadge) {
      logsBadge.textContent = tx.log_messages?.length || 0;
    }

    // Update metadata row
    const metaTimestamp = this.dialogEl?.querySelector("#metaTimestamp span");
    if (metaTimestamp) {
      const timestamp = tx.timestamp || tx.block_time;
      metaTimestamp.textContent = timestamp ? Utils.formatTimestamp(timestamp) : "—";
    }

    const metaSlot = this.dialogEl?.querySelector("#metaSlot span");
    if (metaSlot) {
      metaSlot.textContent = tx.slot ? Utils.formatNumber(tx.slot, { decimals: 0 }) : "—";
    }

    const metaFee = this.dialogEl?.querySelector("#metaFee span");
    if (metaFee) {
      metaFee.textContent = tx.fee_sol ? Utils.formatSol(tx.fee_sol, { decimals: 9 }) : "—";
    }
  }

  _attachEventHandlers() {
    const closeBtn = this.dialogEl.querySelector(".dialog-close");
    this._closeHandler = () => this.close();
    closeBtn.addEventListener("click", this._closeHandler);

    const backdrop = this.dialogEl.querySelector(".dialog-backdrop");
    this._backdropHandler = () => this.close();
    backdrop.addEventListener("click", this._backdropHandler);

    this._escapeHandler = (e) => {
      if (e.key === "Escape") {
        this.close();
      }
    };
    document.addEventListener("keydown", this._escapeHandler);

    // Copy signature button
    const copyBtn = this.dialogEl.querySelector("#copySignatureBtn");
    if (copyBtn) {
      copyBtn.addEventListener("click", () => {
        Utils.copyToClipboard(this.transactionData.signature);
        Utils.showToast("Signature copied!", "success");
      });
    }

    // Tab buttons
    const tabButtons = this.dialogEl.querySelectorAll(".tab-button");
    this._tabHandlers = [];
    tabButtons.forEach((btn) => {
      const handler = () => {
        const tabId = btn.dataset.tab;
        this._switchTab(tabId);
      };
      btn.addEventListener("click", handler);
      this._tabHandlers.push({ element: btn, handler });
    });
  }

  _switchTab(tabId) {
    if (tabId === this.currentTab) return;

    const tabButtons = this.dialogEl.querySelectorAll(".tab-button");
    tabButtons.forEach((btn) => {
      btn.classList.toggle("active", btn.dataset.tab === tabId);
    });

    const tabContents = this.dialogEl.querySelectorAll(".tab-content");
    tabContents.forEach((content) => {
      content.classList.toggle("active", content.dataset.tabContent === tabId);
    });

    this.currentTab = tabId;
    this._loadTabContent(tabId);
  }

  _loadTabContent(tabId) {
    const content = this.dialogEl?.querySelector(`[data-tab-content="${tabId}"]`);
    if (!content) return;

    if (!this.fullTransactionData) {
      content.innerHTML = '<div class="loading-spinner">Loading transaction details...</div>';
      return;
    }

    switch (tabId) {
      case "overview":
        this._loadOverviewTab(content);
        break;
      case "balances":
        this._loadBalancesTab(content);
        break;
      case "instructions":
        this._loadInstructionsTab(content);
        break;
      case "logs":
        this._loadLogsTab(content);
        break;
      case "ata":
        this._loadAtaTab(content);
        break;
      case "raw":
        this._loadRawTab(content);
        break;
    }
  }

  // =========================================================================
  // OVERVIEW TAB
  // =========================================================================

  _loadOverviewTab(content) {
    const tx = this.fullTransactionData;
    if (!tx) return;

    content.innerHTML = `
      <div class="tx-overview-layout">
        <div class="overview-section">
          <div class="section-header">Transaction Details</div>
          <div class="info-grid">
            ${this._buildInfoRow("Signature", this._buildSignatureValue(tx.signature))}
            ${this._buildInfoRow("Status", this._getStatusBadge(tx.status, tx.success))}
            ${this._buildInfoRow("Type", this._getTypeBadge(tx.transaction_type))}
            ${this._buildInfoRow("Direction", this._getDirectionBadge(tx.direction))}
            ${this._buildInfoRow("Timestamp", Utils.formatTimestamp(tx.timestamp || tx.block_time))}
            ${this._buildInfoRow("Slot", tx.slot ? Utils.formatNumber(tx.slot, { decimals: 0 }) : "—")}
            ${this._buildInfoRow("Fee", Utils.formatSol(tx.fee_sol, { decimals: 9 }) || "—")}
            ${tx.error_message ? this._buildInfoRow("Error", `<span class="error-text">${Utils.escapeHtml(tx.error_message)}</span>`) : ""}
          </div>
        </div>

        ${this._buildSwapSection(tx)}
        ${this._buildPnLSection(tx)}
        ${this._buildTokenSection(tx)}
      </div>
    `;
  }

  _buildSignatureValue(signature) {
    if (!signature) return "—";
    const short = `${signature.slice(0, 12)}...${signature.slice(-12)}`;
    return `
      <span class="signature-value">
        <span class="mono-text" title="${Utils.escapeHtml(signature)}">${Utils.escapeHtml(short)}</span>
        <button class="copy-btn-inline" data-copy="${Utils.escapeHtml(signature)}" title="Copy">
          <i class="icon-copy"></i>
        </button>
        <a href="https://solscan.io/tx/${Utils.escapeHtml(signature)}" target="_blank" class="link-btn-inline" title="View on Solscan">
          <i class="icon-external-link"></i>
        </a>
      </span>
    `;
  }

  _buildInfoRow(label, value) {
    return `
      <div class="info-row">
        <span class="info-label">${Utils.escapeHtml(label)}</span>
        <span class="info-value">${value}</span>
      </div>
    `;
  }

  _buildSwapSection(tx) {
    const swapInfo = tx.token_swap_info || tx.token_info;
    if (!swapInfo) return "";

    return `
      <div class="overview-section">
        <div class="section-header">Swap Details</div>
        <div class="info-grid">
          ${this._buildInfoRow("Router", Utils.escapeHtml(swapInfo.router || "—"))}
          ${this._buildInfoRow("Swap Type", Utils.escapeHtml(swapInfo.swap_type || "—"))}
          ${this._buildInfoRow("Input", `${Utils.formatNumber(swapInfo.input_ui_amount || 0, { decimals: 9 })} ${this._getMintLabel(swapInfo.input_mint)}`)}
          ${this._buildInfoRow("Output", `${Utils.formatNumber(swapInfo.output_ui_amount || 0, { decimals: 9 })} ${this._getMintLabel(swapInfo.output_mint)}`)}
          ${swapInfo.pool_address ? this._buildInfoRow("Pool", this._buildAddressLink(swapInfo.pool_address, "account")) : ""}
        </div>
      </div>
    `;
  }

  _buildPnLSection(tx) {
    const pnl = tx.swap_pnl_info;
    if (!pnl) return "";

    return `
      <div class="overview-section">
        <div class="section-header">P&L Analysis</div>
        <div class="info-grid">
          ${this._buildInfoRow("Token", `${Utils.escapeHtml(pnl.token_symbol || "Unknown")} <span class="mono-text-sm">${this._shortenAddress(pnl.token_mint)}</span>`)}
          ${this._buildInfoRow("Type", pnl.swap_type || "—")}
          ${this._buildInfoRow("SOL Amount", Utils.formatSol(pnl.sol_amount, { decimals: 9 }))}
          ${this._buildInfoRow("Token Amount", Utils.formatNumber(pnl.token_amount, { decimals: 9 }))}
          ${this._buildInfoRow("Calculated Price", Utils.formatPriceSol(pnl.calculated_price_sol, { decimals: 12 }) + " SOL")}
          ${pnl.effective_sol_spent ? this._buildInfoRow("Effective SOL Spent", Utils.formatSol(pnl.effective_sol_spent, { decimals: 9 })) : ""}
          ${pnl.effective_sol_received ? this._buildInfoRow("Effective SOL Received", Utils.formatSol(pnl.effective_sol_received, { decimals: 9 })) : ""}
          ${pnl.estimated_pnl_sol !== null && pnl.estimated_pnl_sol !== undefined ? this._buildInfoRow("Estimated P&L", Utils.formatPnL(pnl.estimated_pnl_sol, { decimals: 6 })) : ""}
        </div>
      </div>
    `;
  }

  _buildTokenSection(tx) {
    if (!tx.token_symbol && !tx.token_decimals) return "";

    return `
      <div class="overview-section">
        <div class="section-header">Token Info</div>
        <div class="info-grid">
          ${tx.token_symbol ? this._buildInfoRow("Symbol", Utils.escapeHtml(tx.token_symbol)) : ""}
          ${tx.token_decimals !== undefined ? this._buildInfoRow("Decimals", tx.token_decimals) : ""}
          ${tx.calculated_token_price_sol ? this._buildInfoRow("Price", Utils.formatPriceSol(tx.calculated_token_price_sol, { decimals: 12 }) + " SOL") : ""}
        </div>
      </div>
    `;
  }

  // =========================================================================
  // BALANCES TAB
  // =========================================================================

  _loadBalancesTab(content) {
    const tx = this.fullTransactionData;
    if (!tx) return;

    const solChanges = tx.sol_balance_changes || [];
    const tokenChanges = tx.token_balance_changes || [];

    content.innerHTML = `
      <div class="tx-balances-layout">
        <div class="balance-section">
          <div class="section-header">
            <span>SOL Balance Changes</span>
            <span class="section-count">${solChanges.length}</span>
          </div>
          ${solChanges.length > 0 ? this._buildSolChangesTable(solChanges) : '<div class="empty-message">No SOL balance changes</div>'}
        </div>

        <div class="balance-section">
          <div class="section-header">
            <span>Token Balance Changes</span>
            <span class="section-count">${tokenChanges.length}</span>
          </div>
          ${tokenChanges.length > 0 ? this._buildTokenChangesTable(tokenChanges) : '<div class="empty-message">No token balance changes</div>'}
        </div>

        <div class="balance-summary">
          <div class="summary-item">
            <span class="summary-label">Net SOL Change</span>
            <span class="summary-value ${tx.sol_balance_change >= 0 ? "positive" : "negative"}">${Utils.formatPnL(tx.sol_balance_change, { decimals: 9 })}</span>
          </div>
          <div class="summary-item">
            <span class="summary-label">Transaction Fee</span>
            <span class="summary-value negative">-${Utils.formatSol(tx.fee_sol, { decimals: 9 })}</span>
          </div>
        </div>
      </div>
    `;
  }

  _buildSolChangesTable(changes) {
    const rows = changes
      .map(
        (c) => `
      <tr>
        <td class="mono-text">${this._buildAddressLink(c.account, "account")}</td>
        <td class="numeric">${Utils.formatSol(c.pre_balance, { decimals: 9, suffix: "" })}</td>
        <td class="numeric">${Utils.formatSol(c.post_balance, { decimals: 9, suffix: "" })}</td>
        <td class="numeric ${c.change >= 0 ? "positive" : "negative"}">${c.change >= 0 ? "+" : ""}${Utils.formatSol(c.change, { decimals: 9, suffix: "" })}</td>
      </tr>
    `
      )
      .join("");

    return `
      <table class="balance-table">
        <thead>
          <tr>
            <th>Account</th>
            <th>Pre Balance</th>
            <th>Post Balance</th>
            <th>Change</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    `;
  }

  _buildTokenChangesTable(changes) {
    const rows = changes
      .map(
        (c) => `
      <tr>
        <td class="mono-text">${this._buildAddressLink(c.mint, "token")}</td>
        <td class="numeric">${c.pre_balance !== null ? Utils.formatNumber(c.pre_balance, { decimals: c.decimals || 9 }) : "—"}</td>
        <td class="numeric">${c.post_balance !== null ? Utils.formatNumber(c.post_balance, { decimals: c.decimals || 9 }) : "—"}</td>
        <td class="numeric ${c.change >= 0 ? "positive" : "negative"}">${c.change >= 0 ? "+" : ""}${Utils.formatNumber(c.change, { decimals: c.decimals || 9 })}</td>
      </tr>
    `
      )
      .join("");

    return `
      <table class="balance-table">
        <thead>
          <tr>
            <th>Token Mint</th>
            <th>Pre Balance</th>
            <th>Post Balance</th>
            <th>Change</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    `;
  }

  // =========================================================================
  // INSTRUCTIONS TAB
  // =========================================================================

  _loadInstructionsTab(content) {
    const tx = this.fullTransactionData;
    if (!tx) return;

    const instructions = tx.instructions || tx.instruction_info || [];

    if (instructions.length === 0) {
      content.innerHTML =
        '<div class="empty-state"><i class="icon-code"></i><p>No instructions found</p></div>';
      return;
    }

    const instructionCards = instructions
      .map((instr, idx) => this._buildInstructionCard(instr, idx))
      .join("");

    content.innerHTML = `
      <div class="tx-instructions-layout">
        <div class="instructions-header">
          <span class="instructions-count">${instructions.length} instruction${instructions.length !== 1 ? "s" : ""}</span>
        </div>
        <div class="instructions-list">
          ${instructionCards}
        </div>
      </div>
    `;

    // Attach expand/collapse handlers
    content.querySelectorAll(".instruction-card-header").forEach((header) => {
      header.addEventListener("click", () => {
        const card = header.closest(".instruction-card");
        card.classList.toggle("expanded");
      });
    });
  }

  _buildInstructionCard(instr, idx) {
    const programId = instr.program_id || "Unknown";
    const instrType = instr.instruction_type || "Unknown";
    const accounts = instr.accounts || [];

    return `
      <div class="instruction-card">
        <div class="instruction-card-header">
          <div class="instruction-index">#${idx + 1}</div>
          <div class="instruction-info">
            <span class="instruction-type">${Utils.escapeHtml(instrType)}</span>
            <span class="instruction-program mono-text">${this._shortenAddress(programId)}</span>
          </div>
          <div class="instruction-expand">
            <i class="icon-chevron-down"></i>
          </div>
        </div>
        <div class="instruction-card-body">
          <div class="instruction-detail">
            <span class="detail-label">Program ID</span>
            <span class="detail-value">${this._buildAddressLink(programId, "account")}</span>
          </div>
          ${
            accounts.length > 0
              ? `
            <div class="instruction-accounts">
              <span class="detail-label">Accounts (${accounts.length})</span>
              <div class="accounts-list">
                ${accounts.map((acc, i) => `<div class="account-item"><span class="account-index">${i}</span>${this._buildAddressLink(acc, "account")}</div>`).join("")}
              </div>
            </div>
          `
              : ""
          }
          ${
            instr.data
              ? `
            <div class="instruction-data">
              <span class="detail-label">Data</span>
              <pre class="data-preview">${Utils.escapeHtml(instr.data.slice(0, 200))}${instr.data.length > 200 ? "..." : ""}</pre>
            </div>
          `
              : ""
          }
        </div>
      </div>
    `;
  }

  // =========================================================================
  // LOGS TAB
  // =========================================================================

  _loadLogsTab(content) {
    const tx = this.fullTransactionData;
    if (!tx) return;

    const logs = tx.log_messages || [];

    if (logs.length === 0) {
      content.innerHTML =
        '<div class="empty-state"><i class="icon-file-text"></i><p>No logs available</p></div>';
      return;
    }

    content.innerHTML = `
      <div class="tx-logs-layout">
        <div class="logs-toolbar">
          <input type="text" class="logs-search" placeholder="Filter logs..." id="logsSearchInput" value="${Utils.escapeHtml(this.logSearchQuery)}" />
          <span class="logs-count">${logs.length} log${logs.length !== 1 ? "s" : ""}</span>
        </div>
        <div class="logs-container" id="logsContainer">
          ${this._buildLogsList(logs, this.logSearchQuery)}
        </div>
      </div>
    `;

    // Attach search handler
    const searchInput = content.querySelector("#logsSearchInput");
    if (searchInput) {
      searchInput.addEventListener("input", (e) => {
        this.logSearchQuery = e.target.value;
        const container = content.querySelector("#logsContainer");
        if (container) {
          container.innerHTML = this._buildLogsList(logs, this.logSearchQuery);
        }
      });
    }
  }

  _buildLogsList(logs, filter) {
    const filterLower = (filter || "").toLowerCase();
    const filteredLogs = filterLower
      ? logs.filter((log) => log.toLowerCase().includes(filterLower))
      : logs;

    if (filteredLogs.length === 0) {
      return '<div class="empty-message">No matching logs</div>';
    }

    return filteredLogs
      .map(
        (log, idx) => `
      <div class="log-entry ${this._getLogClass(log)}">
        <span class="log-index">${idx + 1}</span>
        <span class="log-message">${this._highlightLog(log)}</span>
      </div>
    `
      )
      .join("");
  }

  _getLogClass(log) {
    if (log.includes("success")) return "log-success";
    if (log.includes("failed") || log.includes("error") || log.includes("Error"))
      return "log-error";
    if (log.includes("invoke")) return "log-invoke";
    if (log.includes("consumed")) return "log-consumed";
    return "";
  }

  _highlightLog(log) {
    let escaped = Utils.escapeHtml(log);
    // Highlight program invocations
    escaped = escaped.replace(/(Program \w+ invoke)/g, '<span class="hl-invoke">$1</span>');
    // Highlight success
    escaped = escaped.replace(/(success)/gi, '<span class="hl-success">$1</span>');
    // Highlight errors
    escaped = escaped.replace(/(failed|error)/gi, '<span class="hl-error">$1</span>');
    // Highlight consumed
    escaped = escaped.replace(/(\d+ of \d+ compute units)/g, '<span class="hl-compute">$1</span>');
    return escaped;
  }

  // =========================================================================
  // ATA TAB
  // =========================================================================

  _loadAtaTab(content) {
    const tx = this.fullTransactionData;
    if (!tx) return;

    const ataAnalysis = tx.ata_analysis;
    const ataOps = tx.ata_operations || [];

    if (!ataAnalysis && ataOps.length === 0) {
      content.innerHTML =
        '<div class="empty-state"><i class="icon-layers"></i><p>No ATA operations in this transaction</p></div>';
      return;
    }

    content.innerHTML = `
      <div class="tx-ata-layout">
        ${ataAnalysis ? this._buildAtaSummary(ataAnalysis) : ""}
        ${ataOps.length > 0 ? this._buildAtaOperationsList(ataOps) : ""}
      </div>
    `;
  }

  _buildAtaSummary(analysis) {
    return `
      <div class="ata-summary">
        <div class="section-header">ATA Analysis Summary</div>
        <div class="ata-stats-grid">
          <div class="ata-stat">
            <span class="stat-label">Creations</span>
            <span class="stat-value">${analysis.total_ata_creations || 0}</span>
          </div>
          <div class="ata-stat">
            <span class="stat-label">Closures</span>
            <span class="stat-value">${analysis.total_ata_closures || 0}</span>
          </div>
          <div class="ata-stat">
            <span class="stat-label">Rent Spent</span>
            <span class="stat-value negative">-${Utils.formatSol(analysis.total_rent_spent || 0, { decimals: 9 })}</span>
          </div>
          <div class="ata-stat">
            <span class="stat-label">Rent Recovered</span>
            <span class="stat-value positive">+${Utils.formatSol(analysis.total_rent_recovered || 0, { decimals: 9 })}</span>
          </div>
          <div class="ata-stat highlight">
            <span class="stat-label">Net Rent Impact</span>
            <span class="stat-value ${analysis.net_rent_impact >= 0 ? "positive" : "negative"}">${analysis.net_rent_impact >= 0 ? "+" : ""}${Utils.formatSol(analysis.net_rent_impact || 0, { decimals: 9 })}</span>
          </div>
        </div>
      </div>
    `;
  }

  _buildAtaOperationsList(operations) {
    const rows = operations
      .map(
        (op) => `
      <tr>
        <td><span class="badge ${op.operation_type === "Creation" ? "info" : "warning"}">${op.operation_type}</span></td>
        <td class="mono-text">${this._buildAddressLink(op.account_address, "account")}</td>
        <td class="mono-text">${this._buildAddressLink(op.token_mint || op.mint, "token")}</td>
        <td class="numeric">${Utils.formatSol(op.rent_amount || op.rent_cost_sol || 0, { decimals: 9 })}</td>
        <td>${op.is_wsol ? '<span class="badge secondary">WSOL</span>' : "—"}</td>
      </tr>
    `
      )
      .join("");

    return `
      <div class="ata-operations">
        <div class="section-header">ATA Operations (${operations.length})</div>
        <table class="ata-table">
          <thead>
            <tr>
              <th>Type</th>
              <th>Account</th>
              <th>Token Mint</th>
              <th>Rent (SOL)</th>
              <th>WSOL</th>
            </tr>
          </thead>
          <tbody>${rows}</tbody>
        </table>
      </div>
    `;
  }

  // =========================================================================
  // RAW TAB
  // =========================================================================

  _loadRawTab(content) {
    const tx = this.fullTransactionData;
    if (!tx) return;

    const rawData = tx.raw_transaction_data;

    content.innerHTML = `
      <div class="tx-raw-layout">
        <div class="raw-toolbar">
          <button class="raw-copy-btn" id="copyRawBtn">
            <i class="icon-copy"></i>
            Copy JSON
          </button>
          <button class="raw-expand-btn" id="expandAllBtn">
            <i class="icon-chevrons-down"></i>
            Expand All
          </button>
        </div>
        <div class="raw-json-container">
          <pre class="raw-json" id="rawJsonPre">${rawData ? Utils.escapeHtml(JSON.stringify(rawData, null, 2)) : "No raw data available"}</pre>
        </div>
      </div>
    `;

    // Copy button handler
    const copyBtn = content.querySelector("#copyRawBtn");
    if (copyBtn && rawData) {
      copyBtn.addEventListener("click", () => {
        Utils.copyToClipboard(JSON.stringify(rawData, null, 2));
        Utils.showToast("JSON copied to clipboard!", "success");
      });
    }
  }

  // =========================================================================
  // HELPER METHODS
  // =========================================================================

  _getTypeLabel(type) {
    if (!type) return "Unknown";
    if (typeof type === "string") {
      const labels = {
        Buy: "Buy",
        Sell: "Sell",
        Transfer: "Transfer",
        Compute: "Compute",
        AtaOperation: "ATA Operation",
        Failed: "Failed",
        Unknown: "Unknown",
      };
      return labels[type] || type;
    }
    // Handle rich enum variants
    if (type.SwapSolToToken) return "Buy (SOL → Token)";
    if (type.SwapTokenToSol) return "Sell (Token → SOL)";
    if (type.SwapTokenToToken) return "Swap (Token → Token)";
    if (type.SolTransfer) return "SOL Transfer";
    if (type.TokenTransfer) return "Token Transfer";
    if (type.AtaClose) return "ATA Close";
    if (type.Other) return type.Other.description || "Other";
    return "Unknown";
  }

  _getTypeIcon(type) {
    if (!type) return "icon-help-circle";
    const typeStr = typeof type === "string" ? type : Object.keys(type)[0] || "Unknown";
    const icons = {
      Buy: "icon-shopping-cart",
      Sell: "icon-dollar-sign",
      Transfer: "icon-send",
      Compute: "icon-cpu",
      AtaOperation: "icon-layers",
      Failed: "icon-x-circle",
      Unknown: "icon-help-circle",
      SwapSolToToken: "icon-shopping-cart",
      SwapTokenToSol: "icon-dollar-sign",
      SwapTokenToToken: "icon-repeat",
      SolTransfer: "icon-send",
      TokenTransfer: "icon-send",
      AtaClose: "icon-layers",
      Other: "icon-more-horizontal",
    };
    return icons[typeStr] || "icon-help-circle";
  }

  _getTypeBadge(type) {
    const label = this._getTypeLabel(type);
    const typeStr = typeof type === "string" ? type : Object.keys(type)[0] || "Unknown";
    const variants = {
      Buy: "success",
      Sell: "error",
      SwapSolToToken: "success",
      SwapTokenToSol: "error",
      SwapTokenToToken: "info",
      Transfer: "secondary",
      SolTransfer: "secondary",
      TokenTransfer: "secondary",
      Compute: "secondary",
      AtaOperation: "secondary",
      AtaClose: "secondary",
      Failed: "error",
      Unknown: "secondary",
      Other: "secondary",
    };
    const variant = variants[typeStr] || "secondary";
    return `<span class="badge ${variant}">${Utils.escapeHtml(label)}</span>`;
  }

  _getStatusBadge(status, success) {
    if (!status) return '<span class="badge secondary">Unknown</span>';

    // Handle string status
    if (typeof status === "string") {
      const badges = {
        Pending: '<span class="badge warning"><i class="icon-loader"></i> Pending</span>',
        Confirmed: '<span class="badge success"><i class="icon-check"></i> Confirmed</span>',
        Finalized: '<span class="badge success"><i class="icon-check-check"></i> Finalized</span>',
      };
      if (badges[status]) return badges[status];
    }

    // Handle Failed variant with message
    if (status.Failed) {
      return '<span class="badge error"><i class="icon-x"></i> Failed</span>';
    }

    // Fallback based on success boolean
    if (success === true) {
      return '<span class="badge success"><i class="icon-check"></i> Success</span>';
    }
    if (success === false) {
      return '<span class="badge error"><i class="icon-x"></i> Failed</span>';
    }

    return '<span class="badge secondary">Unknown</span>';
  }

  _getDirectionBadge(direction) {
    if (!direction) return "";
    const badges = {
      Incoming: '<span class="badge success">↓ Incoming</span>',
      Outgoing: '<span class="badge error">↑ Outgoing</span>',
      Internal: '<span class="badge secondary">⟲ Internal</span>',
      Unknown: '<span class="badge secondary">? Unknown</span>',
    };
    return badges[direction] || "";
  }

  _shortenAddress(address) {
    if (!address) return "—";
    if (address.length <= 12) return Utils.escapeHtml(address);
    return `${address.slice(0, 4)}...${address.slice(-4)}`;
  }

  _buildAddressLink(address, type = "account") {
    if (!address) return "—";
    const url =
      type === "token"
        ? `https://solscan.io/token/${address}`
        : `https://solscan.io/account/${address}`;
    // Show FULL address, not shortened
    return `
      <span class="address-full">
        <a href="${url}" target="_blank" rel="noopener" class="mono-text address-link" title="View on Solscan">${Utils.escapeHtml(address)}</a>
        <button class="copy-btn-mini" data-copy="${Utils.escapeHtml(address)}" title="Copy address">
          <i class="icon-copy"></i>
        </button>
      </span>
    `;
  }

  _getMintLabel(mint) {
    if (!mint) return "Unknown";
    // Known mints
    const known = {
      So11111111111111111111111111111111111111112: "SOL",
      EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v: "USDC",
      Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB: "USDT",
    };
    return known[mint] || this._shortenAddress(mint);
  }
}

// Initialize copy button handlers via event delegation
document.addEventListener("click", (e) => {
  const copyBtn = e.target.closest("[data-copy]");
  if (copyBtn) {
    const text = copyBtn.dataset.copy;
    if (text) {
      Utils.copyToClipboard(text);
      Utils.showToast("Copied!", "success");
    }
  }
});
