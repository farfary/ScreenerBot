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
    const symbol = this.fullDetails?.position?.symbol || "tokens";

    // Combine and sort newest first
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
        const label = isEntry
          ? item.is_dca ? "DCA Entry" : "Entry"
          : item.is_partial ? "Partial Exit" : "Exit";

        let badges = "";
        if (!isEntry && item.is_partial) {
          badges += `<span class="pdd-badge pdd-badge-warning">${item.percentage}%</span>`;
        }

        const signature = item.transaction_signature;
        const hasSig = !!signature;
        const shortSig = hasSig ? `${signature.slice(0, 8)}…${signature.slice(-8)}` : "—";

        const solCostVal = isEntry ? item.sol_spent : item.sol_received;

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
                  <span class="value">${Utils.formatCompactNumber(this._toUiAmount(item.amount))} ${Utils.escapeHtml(symbol)}</span>
                </div>
                <div class="pdd-timeline-stat">
                  <span class="label">Price</span>
                  <span class="value">${item.price ? `${this._formatPrice(item.price)} SOL` : "—"}</span>
                </div>
                <div class="pdd-timeline-stat">
                  <span class="label">${isEntry ? "SOL Spent" : "SOL Received"}</span>
                  <span class="value">${solCostVal != null ? Utils.formatSol(solCostVal, { decimals: 4 }) : "—"}</span>
                </div>
              </div>
              <div class="pdd-timeline-signature">
                <span class="pdd-signature-text${hasSig ? "" : " pdd-sig-unavailable"}" data-signature="${hasSig ? signature : ""}" title="${hasSig ? "Click to copy" : "No signature"}">${shortSig}</span>
                ${hasSig ? `<a href="https://solscan.io/tx/${signature}" target="_blank" class="pdd-signature-link" title="View on Solscan"><i class="icon-external-link"></i></a>` : ""}
              </div>
            </div>
          </div>
        `;
      })
      .join("");

    content.innerHTML = `<div class="pdd-timeline">${timelineHtml}</div>`;

    // Copy handlers — only on items with an actual signature
    content.querySelectorAll(".pdd-signature-text:not(.pdd-sig-unavailable)").forEach((el) => {
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
    const symbol = this.fullDetails?.position?.symbol || "tokens";

    // Filter out placeholder exit entries when there are no real exits, then sort newest first
    const transactions = allTransactions
      .filter((tx) => {
        if (tx.available !== false) return true;
        return !(tx.kind === "exit" && exits.length === 0);
      })
      .sort((a, b) => (b.timestamp ?? 0) - (a.timestamp ?? 0));

    if (transactions.length === 0 && stateHistory.length === 0) {
      content.innerHTML = '<div class="pdd-empty-state">No transactions available</div>';
      return;
    }

    const entrySignatures = new Set(entries.map((e) => e.transaction_signature));
    const exitSignatures = new Set(exits.map((e) => e.transaction_signature));

    const txCardsHtml = transactions
      .map((tx) => {
        const signature = tx.signature || "";
        const shortSig = signature ? `${signature.slice(0, 8)}…${signature.slice(-8)}` : "—";

        // Resolve type and matching record in one pass
        const entryRecord = entrySignatures.has(signature)
          ? entries.find((e) => e.transaction_signature === signature)
          : null;
        const exitRecord = exitSignatures.has(signature)
          ? exits.find((e) => e.transaction_signature === signature)
          : null;

        let txType = "unknown";
        let txTypeLabel = "Transaction";
        let txTypeClass = "";

        if (entryRecord) {
          txType = "entry";
          txTypeLabel = entryRecord.is_dca ? "DCA Entry" : "Entry";
          txTypeClass = "pdd-tx-type-entry";
        } else if (exitRecord) {
          txType = "exit";
          txTypeLabel = exitRecord.is_partial ? "Partial Exit" : "Exit";
          txTypeClass = "pdd-tx-type-exit";
        } else if (tx.kind) {
          txType = tx.kind.toLowerCase();
          txTypeLabel = this._formatTransactionType(tx.kind);
        }

        const typeIcon =
          txType === "entry" ? "icon-circle-arrow-down" :
          txType === "exit"  ? "icon-circle-arrow-up"   :
                               "icon-activity";

        // Status
        const isPending = tx.status === "pending";
        const isSuccess = tx.success !== false && !isPending;
        const statusClass = isPending ? "pdd-status-pending" : isSuccess ? "pdd-status-success" : "pdd-status-failed";
        const statusLabel = isPending ? "Pending" : isSuccess ? "Confirmed" : "Failed";
        const statusIcon  = isPending ? "icon-clock" : isSuccess ? "icon-circle-check" : "icon-circle-x";

        // Token amount
        let tokenAmountHtml = "";
        if (txType === "entry" && entryRecord?.amount)
          tokenAmountHtml = `<div class="pdd-tx-token positive">+${Utils.formatCompactNumber(this._toUiAmount(entryRecord.amount))} ${Utils.escapeHtml(symbol)}</div>`;
        else if (txType === "exit" && exitRecord?.amount)
          tokenAmountHtml = `<div class="pdd-tx-token negative">-${Utils.formatCompactNumber(this._toUiAmount(exitRecord.amount))} ${Utils.escapeHtml(symbol)}</div>`;

        // SOL change
        const solChange = tx.sol_change;
        const solChangeHtml = solChange != null
          ? `<div class="pdd-tx-sol ${solChange >= 0 ? "positive" : "negative"}">${solChange >= 0 ? "+" : ""}${Utils.formatSol(solChange, { decimals: 6, suffix: "" })} SOL</div>`
          : "";

        // Meta: price, exit %, fee, P&L, router
        const txPrice = entryRecord?.price ?? exitRecord?.price;
        const priceHtml = txPrice
          ? `<span class="pdd-tx-meta-item"><span class="pdd-tx-meta-label">Price</span><span class="pdd-tx-meta-val">${this._formatPrice(txPrice)} SOL</span></span>`
          : "";

        const exitPctHtml = txType === "exit" && exitRecord?.percentage != null
          ? `<span class="pdd-tx-meta-item"><span class="pdd-tx-meta-label">Exit</span><span class="pdd-tx-meta-val">${exitRecord.percentage}%</span></span>`
          : "";

        const solCostHtml = txType === "entry" && entryRecord?.sol_spent != null
          ? `<span class="pdd-tx-meta-item"><span class="pdd-tx-meta-label">Spent</span><span class="pdd-tx-meta-val">${Utils.formatSol(entryRecord.sol_spent, { decimals: 4 })}</span></span>`
          : txType === "exit" && exitRecord?.sol_received != null
            ? `<span class="pdd-tx-meta-item"><span class="pdd-tx-meta-label">Received</span><span class="pdd-tx-meta-val">${Utils.formatSol(exitRecord.sol_received, { decimals: 4 })}</span></span>`
            : "";

        const feeSol = tx.fee_sol != null && tx.fee_sol > 0 ? Utils.formatSol(tx.fee_sol, { decimals: 6 }) : null;
        const feeHtml = feeSol
          ? `<span class="pdd-tx-meta-item"><span class="pdd-tx-meta-label">Fee</span><span class="pdd-tx-meta-val">${feeSol}</span></span>`
          : "";

        let pnlHtml = "";
        if (txType === "exit" && exitRecord?.price) {
          const entryPrice = this.fullDetails?.position?.effective_entry_price || 0;
          if (entryPrice) {
            const pnlPct = ((exitRecord.price - entryPrice) / entryPrice) * 100;
            const pnlClass = pnlPct >= 0 ? "positive" : "negative";
            pnlHtml = `<span class="pdd-tx-meta-item pdd-tx-pnl-item ${pnlClass}"><span class="pdd-tx-meta-label">P&amp;L</span><span class="pdd-tx-meta-val">${pnlPct >= 0 ? "+" : ""}${pnlPct.toFixed(2)}%</span></span>`;
          }
        }

        const routerHtml = tx.router
          ? `<span class="pdd-tx-meta-item"><span class="pdd-tx-meta-val pdd-tx-router-name">${Utils.escapeHtml(tx.router)}</span></span>`
          : "";

        const amountsSection = tokenAmountHtml || solChangeHtml
          ? `<div class="pdd-tx-amounts">${tokenAmountHtml}${solChangeHtml}</div>` : "";
        const metaSection = priceHtml || exitPctHtml || solCostHtml || feeHtml || pnlHtml || routerHtml
          ? `<div class="pdd-tx-meta">${priceHtml}${exitPctHtml}${solCostHtml}${feeHtml}${pnlHtml}${routerHtml}</div>` : "";

        return `
          <div class="pdd-tx-card" data-tx-type="${txType}">
            <div class="pdd-tx-top">
              <span class="pdd-tx-label ${txTypeClass}">
                <i class="${typeIcon}"></i>
                ${txTypeLabel}
              </span>
              <div class="pdd-tx-top-right">
                <span class="pdd-tx-time" title="${Utils.formatTimestamp(tx.timestamp)}">${Utils.formatTimeAgo(tx.timestamp)}</span>
                <span class="pdd-tx-status ${statusClass}">
                  <i class="${statusIcon}"></i>
                  ${statusLabel}
                </span>
              </div>
            </div>
            ${amountsSection}
            ${metaSection}
            <div class="pdd-tx-foot">
              <div class="pdd-tx-sig-group">
                ${signature
                  ? `<span class="pdd-tx-sig" data-signature="${signature}" title="Click to copy">${shortSig}</span>
                     <button class="pdd-tx-copy-btn" data-signature="${signature}" title="Copy signature"><i class="icon-copy"></i></button>`
                  : `<span class="pdd-tx-sig-na">No signature</span>`}
              </div>
              ${signature
                ? `<a href="https://solscan.io/tx/${signature}" target="_blank" class="pdd-tx-link"><i class="icon-external-link"></i><span>Solscan</span></a>`
                : ""}
            </div>
          </div>
        `;
      })
      .join("");

    const stateHistoryHtml = stateHistory.length > 0
      ? `<div class="pdd-section-header">State History</div>
         <div class="pdd-state-history">
           ${stateHistory.map((s) => `
             <div class="pdd-state-item">
               <span class="pdd-state-name">${Utils.escapeHtml(s.state)}</span>
               <span class="pdd-state-time" title="${Utils.formatTimestamp(s.changed_at)}">${Utils.formatTimeAgo(s.changed_at)}</span>
               ${s.reason ? `<span class="pdd-state-reason">${Utils.escapeHtml(s.reason)}</span>` : ""}
             </div>
           `).join("")}
         </div>`
      : "";

    content.innerHTML = `
      <div class="pdd-transactions-container">
        <div class="pdd-tx-filters">
          <button class="pdd-filter-btn active" data-filter="all">All</button>
          <button class="pdd-filter-btn" data-filter="entry">Entries</button>
          <button class="pdd-filter-btn" data-filter="exit">Exits</button>
        </div>
        <div class="pdd-tx-cards">
          ${txCardsHtml || '<div class="pdd-empty-state">No transactions</div>'}
        </div>
        ${stateHistoryHtml}
      </div>
    `;

    // Clean up old filter handlers
    if (this._filterHandlers) {
      this._filterHandlers.forEach(({ element, handler }) => element.removeEventListener("click", handler));
    }
    this._filterHandlers = [];

    // Filter handlers
    content.querySelectorAll(".pdd-filter-btn").forEach((btn) => {
      const handler = () => {
        content.querySelectorAll(".pdd-filter-btn").forEach((b) => b.classList.remove("active"));
        btn.classList.add("active");
        const filter = btn.dataset.filter;
        content.querySelectorAll(".pdd-tx-card").forEach((card) => {
          card.style.display = filter === "all" || card.dataset.txType === filter ? "" : "none";
        });
      };
      btn.addEventListener("click", handler);
      this._filterHandlers.push({ element: btn, handler });
    });

    // Copy handlers
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
