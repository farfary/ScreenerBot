/**
 * Transaction Details Dialog
 * Full-screen dialog showing comprehensive transaction information with multiple tabs
 */
import * as Utils from "../core/utils.js";
import { createFocusTrap } from "../core/utils.js";
import { requestManager } from "../core/request_manager.js";
import { DialogTabBar, renderDialogTabRow } from "./dialog_tab_bar.js";
import {
  getIdentity,
  isSolMint,
  renderAssetInline,
  renderAddress,
  renderTokenChip,
  renderTokenLogo,
  resolveIdentities,
  SOL_MINT,
} from "./token_identity.js";

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
    this._dialogTabBar = null;
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
      const subjectQuery = this.transactionData.subject
        ? `?subject=${encodeURIComponent(this.transactionData.subject)}`
        : "";
      const data = await requestManager.fetch(
        `/api/transactions/${this.transactionData.signature}${subjectQuery}`,
        {
          priority: "high",
        }
      );
      this.fullTransactionData = data;
      // Resolve every asset the transaction touches BEFORE the first render, so no
      // tab ever paints a bare mint and then swaps in a logo underneath the user.
      await resolveIdentities(this._collectMints(data));
      if (!this.dialogEl) return;
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
      content.innerHTML = `<div class="error-state"><i class="icon-circle-alert"></i><p>${Utils.escapeHtml(message)}</p></div>`;
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

        if (this._dialogTabBar) {
          this._dialogTabBar.destroy();
          this._dialogTabBar = null;
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
    const tabs = this._getDialogTabs(tx);

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
                <span class="title-sub mono-text" id="headerSignature" title="${Utils.escapeHtml(tx.signature)}">${Utils.escapeHtml(tx.signature)}</span>
                <div class="header-assets" id="headerAssets"></div>
              </div>
            </div>
            <div class="header-right">
              <div class="dialog-header-actions">
                <button class="dialog-header-action" id="copySignatureBtn" title="Copy Signature">
                  <i class="icon-copy"></i>
                </button>
                <a href="https://solscan.io/tx/${Utils.escapeHtml(tx.signature)}" target="_blank" class="dialog-header-action" title="View on Solscan">
                  <i class="icon-external-link"></i>
                </a>
                <a href="https://solana.fm/tx/${Utils.escapeHtml(tx.signature)}" target="_blank" class="dialog-header-action" title="View on Solana FM">
                  <i class="icon-external-link"></i>
                </a>
              </div>
              <button class="dialog-close" type="button" title="Close (ESC)">
                <i class="icon-x"></i>
              </button>
            </div>
          </div>
          <div class="header-meta-row" id="headerMetaRow">
            <div class="header-badges" id="headerBadges">
              ${statusBadge}
              ${this._getDirectionBadge(tx.direction)}
            </div>
            <div class="header-meta-items">
              <span class="meta-item" id="metaTimestamp"><i class="icon-clock"></i> <span>—</span></span>
              <span class="meta-item" id="metaSlot"><i class="icon-layers"></i> Slot: <span>—</span></span>
              <span class="meta-item" id="metaFee"><i class="icon-zap"></i> Fee: <span>—</span></span>
            </div>
          </div>
        </div>

        ${renderDialogTabRow({
          tabs,
          activeTab: this.currentTab,
          idPrefix: "transaction-details",
          ariaLabel: "Transaction details sections",
        })}

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

  _getDialogTabs(tx) {
    return [
      { id: "overview", label: "Overview", icon: "icon-info" },
      { id: "balances", label: "Balances", icon: "icon-wallet" },
      {
        id: "instructions",
        label: "Instructions",
        icon: "icon-code",
        badge: tx.instructions_count || 0,
        badgeId: "instructionsBadge",
      },
      {
        id: "logs",
        label: "Logs",
        icon: "icon-file-text",
        badge: 0,
        badgeId: "logsBadge",
      },
      { id: "ata", label: "ATA", icon: "icon-layers" },
      { id: "raw", label: "Raw", icon: "icon-braces" },
    ];
  }

  /**
   * Every mint this transaction touches. Resolved in one batch so the swap legs,
   * the balance rows and the ATA rows all name the same assets.
   */
  _collectMints(tx) {
    const mints = [];
    const push = (mint) => {
      if (mint) mints.push(mint);
    };

    const swap = tx.token_swap_info || tx.token_info;
    if (swap) {
      push(swap.input_mint);
      push(swap.output_mint);
      push(swap.mint);
    }
    push(tx.swap_pnl_info?.token_mint);

    const richType = typeof tx.transaction_type === "object" ? tx.transaction_type : {};
    push(richType.TokenTransfer?.mint);

    (tx.token_transfers || []).forEach((transfer) => push(transfer.mint));
    (tx.token_balance_changes || []).forEach((change) => push(change.mint));
    (tx.ata_operations || []).forEach((op) => push(op.token_mint || op.mint));

    return mints;
  }

  /** The token this transaction is ABOUT — the non-SOL leg of a swap, or the asset moved. */
  _primaryMint(tx) {
    const swap = tx.token_swap_info || tx.token_info;
    if (swap) {
      if (swap.mint && !isSolMint(swap.mint)) return swap.mint;
      if (swap.output_mint && !isSolMint(swap.output_mint)) return swap.output_mint;
      if (swap.input_mint && !isSolMint(swap.input_mint)) return swap.input_mint;
      return SOL_MINT;
    }
    if (tx.swap_pnl_info?.token_mint) return tx.swap_pnl_info.token_mint;

    const richType = typeof tx.transaction_type === "object" ? tx.transaction_type : {};
    if (richType.SolTransfer) return SOL_MINT;

    const transferMint = richType.TokenTransfer?.mint || tx.token_transfers?.[0]?.mint;
    if (transferMint) return transferMint;

    const changed = (tx.token_balance_changes || []).find((change) => change.mint);
    return changed ? changed.mint : null;
  }

  /**
   * Header asset strip: which assets moved, with their logos, and the FULL mint of
   * the token the transaction is about. Without it the dialog named a transaction
   * only by its signature — the one thing a user cannot read.
   */
  _buildHeaderAssets(tx) {
    const swap = tx.token_swap_info || tx.token_info;
    const primary = this._primaryMint(tx);

    let assets = "";
    if (swap && swap.input_mint && swap.output_mint) {
      assets = `
        ${renderTokenChip(swap.input_mint, { size: "sm", showName: false })}
        <i class="icon-arrow-right tx-asset-arrow" aria-hidden="true"></i>
        ${renderTokenChip(swap.output_mint, { size: "sm", showName: false })}
      `;
    } else if (primary) {
      assets = renderTokenChip(primary, { size: "sm" });
    } else {
      return "";
    }

    const identity = primary ? getIdentity(primary) : null;
    const name = identity?.name && identity.name !== identity.symbol ? identity.name : "";

    return `
      <div class="header-asset-strip">${assets}</div>
      ${name ? `<span class="header-asset-name">${Utils.escapeHtml(name)}</span>` : ""}
      ${primary ? renderAddress(primary) : ""}
    `;
  }

  _updateHeader() {
    const tx = this.fullTransactionData;
    if (!tx) return;

    const assetsEl = this.dialogEl?.querySelector("#headerAssets");
    if (assetsEl) {
      assetsEl.innerHTML = this._buildHeaderAssets(tx);
    }

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

    this._dialogTabBar = new DialogTabBar({
      root: this.dialogEl,
      tabs: this._getDialogTabs(this.transactionData),
      activeTab: this.currentTab,
      onChange: (tabId) => {
        this.currentTab = tabId;
        this._loadTabContent(tabId);
      },
    });
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

    const failed = tx.success === false || Boolean(tx.status?.Failed);
    const failureMessage = tx.error_message || (failed ? "No program error was provided." : "");
    const routeHtml = this._buildOverviewRoute(tx);

    content.innerHTML = `
      <div class="tx-overview-layout">
        ${
          failureMessage
            ? `<div class="tx-failure-callout" role="alert"><i class="icon-circle-alert"></i><div><strong>Transaction failed</strong><span>${Utils.escapeHtml(failureMessage)}</span></div></div>`
            : ""
        }
        ${this._buildOverviewStory(tx)}
        ${this._buildOverviewMetrics(tx)}
        <div class="tx-overview-details${routeHtml ? "" : " single-column"}">
          ${routeHtml}
          ${this._buildOverviewTechnical(tx)}
        </div>
      </div>
    `;
  }

  _buildOverviewStory(tx) {
    const swap = tx.token_swap_info || tx.token_info;
    if (swap) {
      const router = swap.router ? `via ${Utils.escapeHtml(swap.router)}` : "";
      return `
        <section class="tx-story-card">
          <div class="tx-story-heading"><span>What happened</span>${router ? `<small>${router}</small>` : ""}</div>
          <div class="tx-flow">
            ${this._buildFlowSide("Paid", swap.input_ui_amount, swap.input_mint, "")}
            <span class="tx-flow-arrow" aria-hidden="true"><i class="icon-arrow-right"></i></span>
            ${this._buildFlowSide("Received", swap.output_ui_amount, swap.output_mint, "tx-flow-received")}
          </div>
        </section>`;
    }

    const richType = typeof tx.transaction_type === "object" ? tx.transaction_type : {};
    const solTransfer = richType.SolTransfer;
    const tokenTransfer = richType.TokenTransfer || tx.token_transfers?.[0];
    if (solTransfer || tokenTransfer) {
      const transfer = solTransfer || tokenTransfer;
      const mint = solTransfer ? SOL_MINT : transfer.mint;
      return `
        <section class="tx-story-card">
          <div class="tx-story-heading"><span>What happened</span><small>${Utils.escapeHtml(this._getTypeLabel(tx.transaction_type))}</small></div>
          <div class="tx-transfer-asset">${renderTokenChip(mint, { size: "md", showMint: true })}</div>
          <div class="tx-flow">
            <div class="tx-flow-side">
              <span class="tx-flow-label">From</span>
              <strong class="tx-flow-address" title="${Utils.escapeHtml(transfer.from || "")}">${this._shortenAddress(transfer.from)}</strong>
            </div>
            <span class="tx-flow-arrow" aria-hidden="true"><i class="icon-arrow-right"></i></span>
            <div class="tx-flow-side tx-flow-received">
              <span class="tx-flow-label">To</span>
              <strong class="tx-flow-address" title="${Utils.escapeHtml(transfer.to || "")}">${this._shortenAddress(transfer.to)}</strong>
            </div>
          </div>
          <div class="tx-story-amount"><span>Amount</span><strong>${this._formatOverviewAmount(transfer.amount)} ${renderAssetInline(mint, { size: "xs" })}</strong></div>
        </section>`;
    }

    const netChange = Number(tx.sol_balance_change);
    const hasNetChange = Number.isFinite(netChange);
    return `
      <section class="tx-story-card">
        <div class="tx-story-heading"><span>What happened</span></div>
        <div class="tx-generic-result">
          <i class="${this._getTypeIcon(tx.transaction_type)}"></i>
          <div><strong>${Utils.escapeHtml(this._getTypeLabel(tx.transaction_type))}</strong><span>${hasNetChange ? `Net wallet change: ${Utils.formatPnL(netChange, { decimals: 6 })}` : "Processed on Solana"}</span></div>
        </div>
      </section>`;
  }

  /** One leg of a swap: amount, then the asset it is denominated in (logo + symbol + full mint). */
  _buildFlowSide(label, amount, mint, sideClass) {
    const identity = getIdentity(mint);
    return `
      <div class="tx-flow-side ${sideClass}">
        <span class="tx-flow-label">${Utils.escapeHtml(label)}</span>
        <strong title="${this._formatOverviewExact(amount)}">${this._formatOverviewAmount(amount)}</strong>
        <span class="tx-flow-asset">
          ${renderTokenLogo(identity, { size: "xs" })}
          <span>${Utils.escapeHtml(identity.symbol || "Unknown asset")}</span>
        </span>
        ${mint ? renderAddress(mint) : ""}
      </div>`;
  }

  _buildOverviewMetrics(tx) {
    const pnl = tx.swap_pnl_info;
    const swap = tx.token_swap_info || tx.token_info;
    const metrics = [];
    const add = (label, value, tone = "", title = "") => {
      if (!value) return;
      metrics.push(
        `<div class="tx-execution-metric ${tone}"${title ? ` title="${title}"` : ""}><span>${label}</span><strong>${value}</strong></div>`
      );
    };

    const price = pnl?.calculated_price_sol ?? tx.calculated_token_price_sol;
    if (price !== null && price !== undefined) {
      add(
        "Execution price",
        `${Utils.formatPriceSol(price, { decimals: 8 })} SOL`,
        "",
        `${Utils.formatPriceSol(price, { decimals: 12 })} SOL`
      );
    }

    if (pnl) {
      const swapType = String(pnl.swap_type || swap?.swap_type || "").toLowerCase();
      const isSell = swapType.includes("sell") || swapType.includes("token_to_sol");
      const effective = isSell ? pnl.effective_sol_received : pnl.effective_sol_spent;
      if (effective !== null && effective !== undefined) {
        add(
          isSell ? "Effective received" : "Effective spent",
          Utils.formatSol(effective, { decimals: 6 }),
          "",
          Utils.formatSol(effective, { decimals: 9 })
        );
      }
    }

    if (tx.fee_sol !== null && tx.fee_sol !== undefined) {
      add(
        "Network fee",
        Utils.formatSol(tx.fee_sol, { decimals: 6 }),
        "",
        Utils.formatSol(tx.fee_sol, { decimals: 9 })
      );
    }

    if (pnl?.estimated_pnl_sol !== null && pnl?.estimated_pnl_sol !== undefined) {
      const tone =
        pnl.estimated_pnl_sol > 0 ? "positive" : pnl.estimated_pnl_sol < 0 ? "negative" : "";
      add("Estimated P&L", Utils.formatPnL(pnl.estimated_pnl_sol, { decimals: 6 }), tone);
    }

    if (!swap && Number.isFinite(Number(tx.sol_balance_change))) {
      const change = Number(tx.sol_balance_change);
      add(
        "Net SOL change",
        Utils.formatPnL(change, { decimals: 6 }),
        change > 0 ? "positive" : change < 0 ? "negative" : ""
      );
    }

    return metrics.length > 0
      ? `<section class="tx-execution-strip"><div class="tx-section-label">Execution</div><div class="tx-execution-grid">${metrics.join("")}</div></section>`
      : "";
  }

  _buildOverviewRoute(tx) {
    const swap = tx.token_swap_info || tx.token_info;
    if (!swap) return "";
    const rows = [];
    if (swap.router)
      rows.push(["Router", `<span class="tx-router-name">${Utils.escapeHtml(swap.router)}</span>`]);
    rows.push(["Input asset", this._buildOverviewAsset(swap.input_mint)]);
    rows.push(["Output asset", this._buildOverviewAsset(swap.output_mint)]);
    if (swap.pool_address)
      rows.push(["Pool", renderAddress(swap.pool_address, { explorer: "account" })]);
    if (swap.program_id)
      rows.push(["Program", renderAddress(swap.program_id, { explorer: "account" })]);
    return `<section class="tx-detail-card"><div class="tx-detail-heading">Route and assets</div><div class="tx-detail-list">${rows.map(([label, value]) => this._buildOverviewDetailRow(label, value)).join("")}</div></section>`;
  }

  _buildOverviewTechnical(tx) {
    const rows = [
      ["Signature", renderAddress(tx.signature, { explorer: "tx" })],
      ["Timestamp", Utils.formatTimestamp(tx.timestamp || tx.block_time)],
      [
        "Slot",
        tx.slot !== null && tx.slot !== undefined
          ? Utils.formatNumber(tx.slot, { decimals: 0 })
          : "—",
      ],
      [
        "Exact fee",
        tx.fee_sol !== null && tx.fee_sol !== undefined
          ? Utils.formatSol(tx.fee_sol, { decimals: 9 })
          : "—",
      ],
      ["Accounts", Utils.formatNumber(tx.accounts_count ?? 0, { decimals: 0 })],
      ["Instructions", Utils.formatNumber(tx.instructions_count ?? 0, { decimals: 0 })],
    ];
    if (tx.compute_units_consumed !== null && tx.compute_units_consumed !== undefined) {
      rows.push(["Compute units", Utils.formatNumber(tx.compute_units_consumed, { decimals: 0 })]);
    }
    if (tx.token_decimals !== null && tx.token_decimals !== undefined) {
      rows.push(["Token decimals", String(tx.token_decimals)]);
    }
    return `<details class="tx-technical-card"><summary><span><strong>Technical details</strong><small>Signature, slot and resources</small></span><i class="icon-chevron-down"></i></summary><div class="tx-detail-list">${rows.map(([label, value]) => this._buildOverviewDetailRow(label, value)).join("")}</div></details>`;
  }

  _buildOverviewDetailRow(label, value) {
    return `<div class="tx-detail-row"><span>${Utils.escapeHtml(label)}</span><div>${value}</div></div>`;
  }

  _buildOverviewAsset(mint) {
    if (!mint) return "—";
    return `<span class="tx-asset-reference">${renderTokenChip(mint, { size: "sm", showMint: true })}</span>`;
  }

  _formatOverviewAmount(value) {
    const number = Number(value);
    if (!Number.isFinite(number)) return "—";
    const absolute = Math.abs(number);
    if (absolute >= 1_000) return Utils.formatCompactNumber(number, { digits: 2 });
    const decimals = absolute >= 1 ? 4 : absolute >= 0.01 ? 6 : 9;
    return Utils.formatNumber(number, { decimals }).replace(/(\.\d*?[1-9])0+$|\.0+$/, "$1");
  }

  _formatOverviewExact(value) {
    const number = Number(value);
    return Number.isFinite(number) ? Utils.formatNumber(number, { decimals: 9 }) : "Unavailable";
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
            <span class="section-title">${renderTokenLogo(SOL_MINT, { size: "xs" })} SOL Balance Changes</span>
            <span class="section-count">${solChanges.length}</span>
          </div>
          ${solChanges.length > 0 ? this._buildSolChangesTable(solChanges) : '<div class="empty-message">No SOL balance changes</div>'}
        </div>

        <div class="balance-section">
          <div class="section-header">
            <span class="section-title"><i class="icon-coins"></i> Token Balance Changes</span>
            <span class="section-count">${tokenChanges.length}</span>
          </div>
          ${tokenChanges.length > 0 ? this._buildTokenChangesTable(tokenChanges) : '<div class="empty-message">No token balance changes</div>'}
        </div>

        <div class="balance-summary">
          <div class="summary-item">
            <span class="summary-label">Net SOL Change</span>
            <span class="summary-value ${tx.sol_balance_change >= 0 ? "positive" : "negative"}">${renderTokenLogo(SOL_MINT, { size: "xs" })} ${Utils.formatPnL(tx.sol_balance_change, { decimals: 9 })}</span>
          </div>
          <div class="summary-item">
            <span class="summary-label">Transaction Fee</span>
            <span class="summary-value negative">${renderTokenLogo(SOL_MINT, { size: "xs" })} -${Utils.formatSol(tx.fee_sol, { decimals: 9 })}</span>
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
        <td class="tx-address-cell">${renderAddress(c.account, { explorer: "account" })}</td>
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
        <td>${renderTokenChip(c.mint, { size: "sm" })}</td>
        <td class="tx-mint-cell">${renderAddress(c.mint)}</td>
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
            <th>Token</th>
            <th>Mint Address</th>
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
            <span class="detail-value">${renderAddress(programId, { explorer: "account" })}</span>
          </div>
          ${
            accounts.length > 0
              ? `
            <div class="instruction-accounts">
              <span class="detail-label">Accounts (${accounts.length})</span>
              <div class="accounts-list">
                ${accounts.map((acc, i) => `<div class="account-item"><span class="account-index">${i}</span>${renderAddress(acc, { explorer: "account" })}</div>`).join("")}
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
            <span class="stat-value negative">${renderTokenLogo(SOL_MINT, { size: "xs" })} -${Utils.formatSol(analysis.total_rent_spent || 0, { decimals: 9 })}</span>
          </div>
          <div class="ata-stat">
            <span class="stat-label">Rent Recovered</span>
            <span class="stat-value positive">${renderTokenLogo(SOL_MINT, { size: "xs" })} +${Utils.formatSol(analysis.total_rent_recovered || 0, { decimals: 9 })}</span>
          </div>
          <div class="ata-stat highlight">
            <span class="stat-label">Net Rent Impact</span>
            <span class="stat-value ${analysis.net_rent_impact >= 0 ? "positive" : "negative"}">${renderTokenLogo(SOL_MINT, { size: "xs" })} ${analysis.net_rent_impact >= 0 ? "+" : ""}${Utils.formatSol(analysis.net_rent_impact || 0, { decimals: 9 })}</span>
          </div>
        </div>
      </div>
    `;
  }

  _buildAtaOperationsList(operations) {
    const rows = operations
      .map((op) => {
        const mint = op.token_mint || op.mint;
        return `
      <tr>
        <td><span class="badge ${op.operation_type === "Creation" ? "info" : "warning"}">${op.operation_type}</span></td>
        <td class="tx-address-cell">${renderAddress(op.account_address, { explorer: "account" })}</td>
        <td>${renderTokenChip(mint, { size: "sm", showName: false })}</td>
        <td class="tx-mint-cell">${renderAddress(mint)}</td>
        <td class="numeric">${Utils.formatSol(op.rent_amount || op.rent_cost_sol || 0, { decimals: 9 })}</td>
        <td>${op.is_wsol ? '<span class="badge secondary">WSOL</span>' : "—"}</td>
      </tr>
    `;
      })
      .join("");

    return `
      <div class="ata-operations">
        <div class="section-header">ATA Operations (${operations.length})</div>
        <table class="ata-table">
          <thead>
            <tr>
              <th>Type</th>
              <th>Account</th>
              <th>Token</th>
              <th>Mint Address</th>
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
    if (!type) return "icon-info";
    const typeStr = typeof type === "string" ? type : Object.keys(type)[0] || "Unknown";
    const icons = {
      Buy: "icon-shopping-cart",
      Sell: "icon-dollar-sign",
      Transfer: "icon-send",
      Compute: "icon-cpu",
      AtaOperation: "icon-layers",
      Failed: "icon-circle-x",
      Unknown: "icon-info",
      SwapSolToToken: "icon-shopping-cart",
      SwapTokenToSol: "icon-dollar-sign",
      SwapTokenToToken: "icon-repeat",
      SolTransfer: "icon-send",
      TokenTransfer: "icon-send",
      AtaClose: "icon-layers",
      Other: "icon-ellipsis",
    };
    return icons[typeStr] || "icon-info";
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
}
