/**
 * Secondary Tabs Mixin for Position Details Dialog
 * Adds History, Transactions, and Token tab rendering functionality
 */
import * as Utils from "../../core/utils.js";

/**
 * Apply secondary tabs methods to PositionDetailsDialog prototype
 * @param {class} PositionDetailsDialog - The PositionDetailsDialog class
 */
export function applySecondaryTabsMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

  // ===========================================================================
  // HISTORY TAB
  // ===========================================================================

  proto._renderHistoryTab = function (content) {
    const entries = this.fullDetails?.entries || [];
    const exits = this.fullDetails?.exits || [];

    // Combine and sort by timestamp (newest first)
    const timeline = [
      ...entries.map((e) => ({ ...e, type: "entry" })),
      ...exits.map((e) => ({ ...e, type: "exit" })),
    ].sort((a, b) => b.timestamp - a.timestamp);

    if (timeline.length === 0) {
      content.innerHTML = '<div class="pdd-empty-state">No history available</div>';
      return;
    }

    const timelineHtml = timeline
      .map((item) => {
        const isEntry = item.type === "entry";
        const typeClass = isEntry ? "pdd-timeline-entry" : "pdd-timeline-exit";
        const icon = isEntry ? "icon-circle-arrow-down" : "icon-circle-arrow-up";
        const label = isEntry ? "Entry" : "Exit";

        let badges = "";
        if (isEntry && item.is_dca) {
          badges += '<span class="pdd-badge pdd-badge-info">DCA</span>';
        }
        if (!isEntry && item.is_partial) {
          badges += `<span class="pdd-badge pdd-badge-warning">${item.percentage}%</span>`;
        }

        const signature = item.transaction_signature;
        const shortSig = signature ? `${signature.slice(0, 8)}...${signature.slice(-8)}` : "—";

        return `
          <div class="pdd-timeline-item ${typeClass}">
            <div class="pdd-timeline-icon">
              <i class="${icon}"></i>
            </div>
            <div class="pdd-timeline-content">
              <div class="pdd-timeline-header">
                <span class="pdd-timeline-label">${label}</span>
                ${badges}
                <span class="pdd-timeline-time" title="${Utils.formatTimestamp(item.timestamp)}">${Utils.formatTimeAgo(item.timestamp)}</span>
              </div>
              <div class="pdd-timeline-details">
                <div class="pdd-timeline-stat">
                  <span class="label">Amount</span>
                  <span class="value">${Utils.formatCompactNumber(item.amount)} tokens</span>
                </div>
                <div class="pdd-timeline-stat">
                  <span class="label">Price</span>
                  <span class="value">${this._formatPrice(item.price)} SOL</span>
                </div>
                <div class="pdd-timeline-stat">
                  <span class="label">${isEntry ? "SOL Spent" : "SOL Received"}</span>
                  <span class="value">${Utils.formatSol(isEntry ? item.sol_spent : item.sol_received, { decimals: 4 })}</span>
                </div>
              </div>
              <div class="pdd-timeline-signature">
                <span class="pdd-signature-text" data-signature="${signature || ""}" title="Click to copy">${shortSig}</span>
                <a href="https://solscan.io/tx/${signature || ""}" target="_blank" class="pdd-signature-link" title="View on Solscan">
                  <i class="icon-external-link"></i>
                </a>
              </div>
            </div>
          </div>
        `;
      })
      .join("");

    content.innerHTML = `
      <div class="pdd-timeline">
        ${timelineHtml}
      </div>
    `;

    // Attach copy handlers for signatures
    content.querySelectorAll(".pdd-signature-text").forEach((el) => {
      el.addEventListener("click", () => {
        const sig = el.dataset.signature;
        if (sig) {
          Utils.copyToClipboard(sig);
          Utils.showToast("Signature copied!", "success");
        }
      });
    });
  };

  // ===========================================================================
  // TRANSACTIONS TAB
  // ===========================================================================

  proto._renderTransactionsTab = function (content) {
    const allTransactions = this.fullDetails?.transactions || [];
    const entries = this.fullDetails?.entries || [];
    const exits = this.fullDetails?.exits || [];
    const stateHistory = this.fullDetails?.state_history || [];

    // Filter out unavailable exit transactions (shown for open positions with no actual exit)
    const transactions = allTransactions.filter((tx) => {
      // Keep all available transactions
      if (tx.available !== false) return true;
      // For unavailable ones, only keep if it's NOT an exit with no actual exits
      if (tx.kind === "exit" && exits.length === 0) return false;
      return true;
    });

    if (transactions.length === 0 && stateHistory.length === 0) {
      content.innerHTML = '<div class="pdd-empty-state">No transactions available</div>';
      return;
    }

    // Merge entry/exit info with transactions for enhanced display
    const entrySignatures = new Set(entries.map((e) => e.transaction_signature));
    const exitSignatures = new Set(exits.map((e) => e.transaction_signature));

    // Build filter buttons
    const filterButtonsHtml = `
      <div class="pdd-tx-filters">
        <button class="pdd-filter-btn active" data-filter="all">All</button>
        <button class="pdd-filter-btn" data-filter="entry">Entries</button>
        <button class="pdd-filter-btn" data-filter="exit">Exits</button>
      </div>
    `;

    // Build transaction cards
    const txCardsHtml = transactions
      .map((tx) => {
        const signature = tx.signature || "";
        const shortSig = signature ? `${signature.slice(0, 8)}...${signature.slice(-8)}` : "—";

        // Determine transaction type
        let txType = "unknown";
        let txTypeLabel = "Transaction";
        let txTypeClass = "";

        if (entrySignatures.has(signature)) {
          const entryRecord = entries.find((e) => e.transaction_signature === signature);
          txType = "entry";
          txTypeLabel = entryRecord?.is_dca ? "DCA Entry" : "Entry";
          txTypeClass = "pdd-tx-type-entry";
        } else if (exitSignatures.has(signature)) {
          const exitRecord = exits.find((e) => e.transaction_signature === signature);
          txType = "exit";
          txTypeLabel = exitRecord?.is_partial ? "Partial Exit" : "Exit";
          txTypeClass = "pdd-tx-type-exit";
        } else if (tx.kind) {
          txType = tx.kind.toLowerCase();
          txTypeLabel = this._formatTransactionType(tx.kind);
        }

        // Status
        const isSuccess = tx.success !== false;
        const isPending = tx.status === "pending";
        const statusClass = isPending
          ? "pdd-status-pending"
          : isSuccess
            ? "pdd-status-success"
            : "pdd-status-failed";
        const statusLabel = isPending ? "Pending" : isSuccess ? "Confirmed" : "Failed";
        const statusIcon = isPending
          ? "icon-clock"
          : isSuccess
            ? "icon-circle-check"
            : "icon-circle-x";

        // Amount info
        const solChange = tx.sol_change;
        const solChangeHtml = solChange
          ? `<div class="pdd-tx-sol-change ${solChange > 0 ? "positive" : "negative"}">
              ${solChange > 0 ? "+" : ""}${Utils.formatSol(solChange, { decimals: 6, suffix: "" })} SOL
            </div>`
          : "";

        // Get token amount from entry/exit records
        let tokenAmountHtml = "";
        if (txType === "entry") {
          const entryRecord = entries.find((e) => e.transaction_signature === signature);
          if (entryRecord?.amount) {
            const symbol = this.fullDetails?.position?.symbol || "tokens";
            tokenAmountHtml = `<div class="pdd-tx-token-amount">+${Utils.formatCompactNumber(entryRecord.amount)} ${symbol}</div>`;
          }
        } else if (txType === "exit") {
          const exitRecord = exits.find((e) => e.transaction_signature === signature);
          if (exitRecord?.amount) {
            const symbol = this.fullDetails?.position?.symbol || "tokens";
            tokenAmountHtml = `<div class="pdd-tx-token-amount">-${Utils.formatCompactNumber(exitRecord.amount)} ${symbol}</div>`;
          }
        }

        // Fee display
        const feeSol = tx.fee_sol ? Utils.formatSol(tx.fee_sol, { decimals: 6 }) : null;
        const feeHtml = feeSol
          ? `<div class="pdd-tx-fee"><span class="label">Fee:</span> ${feeSol}</div>`
          : "";

        // P&L for exits
        let pnlHtml = "";
        if (txType === "exit") {
          const exitRecord = exits.find((e) => e.transaction_signature === signature);
          if (exitRecord?.sol_received) {
            // Find matching entry to calculate P&L
            const entryPrice = this.fullDetails?.position?.effective_entry_price || 0;
            if (entryPrice && exitRecord.price) {
              const pnlPercent = ((exitRecord.price - entryPrice) / entryPrice) * 100;
              pnlHtml = `<div class="pdd-tx-pnl ${pnlPercent >= 0 ? "positive" : "negative"}">
                P&L: ${pnlPercent >= 0 ? "+" : ""}${pnlPercent.toFixed(2)}%
              </div>`;
            }
          }
        }

        // Router info
        const routerHtml = tx.router
          ? `<div class="pdd-tx-router">${Utils.escapeHtml(tx.router)}</div>`
          : "";

        return `
          <div class="pdd-tx-card" data-tx-type="${txType}">
            <div class="pdd-tx-card-header">
              <div class="pdd-tx-type-badge ${txTypeClass}">${txTypeLabel}</div>
              <div class="pdd-tx-status-badge ${statusClass}">
                <i class="${statusIcon}"></i>
                <span>${statusLabel}</span>
              </div>
            </div>
            <div class="pdd-tx-card-body">
              <div class="pdd-tx-amounts">
                ${tokenAmountHtml}
                ${solChangeHtml}
              </div>
              <div class="pdd-tx-details">
                ${feeHtml}
                ${pnlHtml}
                ${routerHtml}
              </div>
            </div>
            <div class="pdd-tx-card-footer">
              <div class="pdd-tx-signature-row">
                <span class="pdd-tx-sig" data-signature="${signature}" title="Click to copy">
                  ${shortSig}
                </span>
                <button class="pdd-tx-copy-btn" data-signature="${signature}" title="Copy signature">
                  <i class="icon-copy"></i>
                </button>
              </div>
              <div class="pdd-tx-time" title="${Utils.formatTimestamp(tx.timestamp)}">
                ${Utils.formatTimeAgo(tx.timestamp)}
              </div>
              <a href="https://solscan.io/tx/${signature}" target="_blank" class="pdd-tx-explorer" title="View on Solscan">
                <i class="icon-external-link"></i>
              </a>
            </div>
          </div>
        `;
      })
      .join("");

    // State history timeline
    const stateHistoryHtml =
      stateHistory.length > 0
        ? `
        <div class="pdd-section-header">State History</div>
        <div class="pdd-state-history">
          ${stateHistory
            .map((state) => {
              return `
              <div class="pdd-state-item">
                <span class="pdd-state-name">${Utils.escapeHtml(state.state)}</span>
                <span class="pdd-state-time" title="${Utils.formatTimestamp(state.changed_at)}">${Utils.formatTimeAgo(state.changed_at)}</span>
                ${state.reason ? `<span class="pdd-state-reason">${Utils.escapeHtml(state.reason)}</span>` : ""}
              </div>
            `;
            })
            .join("")}
        </div>
      `
        : "";

    content.innerHTML = `
      <div class="pdd-transactions-container">
        ${filterButtonsHtml}
        <div class="pdd-tx-cards">
          ${txCardsHtml || '<div class="pdd-empty-state">No transactions</div>'}
        </div>
        ${stateHistoryHtml}
      </div>
    `;

    // Clean up old filter handlers
    if (this._filterHandlers) {
      this._filterHandlers.forEach(({ element, handler }) => {
        element.removeEventListener("click", handler);
      });
    }
    this._filterHandlers = [];

    // Attach filter handlers with tracking
    content.querySelectorAll(".pdd-filter-btn").forEach((btn) => {
      const handler = () => {
        // Update active button
        content.querySelectorAll(".pdd-filter-btn").forEach((b) => b.classList.remove("active"));
        btn.classList.add("active");

        // Filter transactions
        const filter = btn.dataset.filter;
        content.querySelectorAll(".pdd-tx-card").forEach((card) => {
          const txType = card.dataset.txType;
          if (filter === "all") {
            card.style.display = "";
          } else if (filter === "entry" && txType === "entry") {
            card.style.display = "";
          } else if (filter === "exit" && txType === "exit") {
            card.style.display = "";
          } else if (filter !== "all") {
            card.style.display = "none";
          }
        });
      };
      btn.addEventListener("click", handler);
      this._filterHandlers.push({ element: btn, handler });
    });

    // Attach copy handlers for signatures
    content.querySelectorAll(".pdd-tx-sig, .pdd-tx-copy-btn").forEach((el) => {
      el.addEventListener("click", (e) => {
        e.preventDefault();
        const sig = el.dataset.signature;
        if (sig) {
          Utils.copyToClipboard(sig);
          Utils.showToast("Signature copied!", "success");
        }
      });
    });
  };
}
