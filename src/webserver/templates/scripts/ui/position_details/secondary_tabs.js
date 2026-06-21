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

  // ===========================================================================
  // TOKEN TAB
  // ===========================================================================

  proto._renderTokenTab = function (content) {
    const pos = this.fullDetails?.position;
    const tokenInfo = this.fullDetails?.token_info;
    const security = this.fullDetails?.security;
    const poolInfo = this.fullDetails?.pool_info;
    const marketData = this.fullDetails?.market_data;
    const externalLinks = this.fullDetails?.external_links;

    if (!pos) {
      content.innerHTML = '<div class="pdd-empty-state">No token data available</div>';
      return;
    }

    // Token identity section
    const logoHtml = tokenInfo?.image_url
      ? `<img class="pdd-token-logo-lg" src="${tokenInfo.image_url}" alt="${pos.symbol || ""}" />`
      : `<div class="pdd-token-logo-lg pdd-logo-placeholder">${(pos.symbol || "?").slice(0, 2).toUpperCase()}</div>`;

    const descriptionHtml = tokenInfo?.description
      ? `<p class="pdd-token-desc">${Utils.escapeHtml(tokenInfo.description)}</p>`
      : "";

    // Security assessment section
    const score = security?.score_normalized ?? null;
    const riskLevel = security?.risk_level || "unknown";
    const riskLabelClass =
      {
        low: "pdd-risk-low",
        medium: "pdd-risk-medium",
        high: "pdd-risk-high",
        unknown: "pdd-risk-unknown",
      }[riskLevel] || "pdd-risk-unknown";

    const riskLabelText =
      {
        low: "Low Risk",
        medium: "Medium Risk",
        high: "High Risk",
        unknown: "Unknown",
      }[riskLevel] || "Unknown";

    const mintAuthorityHtml =
      security !== undefined
        ? `<div class="pdd-authority-row">
          <span class="label">Mint Authority:</span>
          <span class="pdd-status ${security.has_mint_authority ? "warning" : "safe"}">
            ${security.has_mint_authority ? "Active <i class='icon-triangle-alert'></i>" : "Revoked ✓"}
          </span>
        </div>`
        : "";

    const freezeAuthorityHtml =
      security !== undefined
        ? `<div class="pdd-authority-row">
          <span class="label">Freeze Authority:</span>
          <span class="pdd-status ${security.has_freeze_authority ? "warning" : "safe"}">
            ${security.has_freeze_authority ? "Active <i class='icon-triangle-alert'></i>" : "Revoked ✓"}
          </span>
        </div>`
        : "";

    const risksListHtml = security?.top_risks?.length
      ? `<div class="pdd-token-risks">
          <h5>Risk Factors</h5>
          <ul>
            ${security.top_risks.map((risk) => `<li>${Utils.escapeHtml(risk)}</li>`).join("")}
          </ul>
        </div>`
      : "";

    // Social links section
    const socialLinks = [];
    if (tokenInfo?.website) {
      socialLinks.push(
        `<a href="${tokenInfo.website}" target="_blank" class="pdd-social-link"><i class="icon-globe"></i> Website</a>`
      );
    }
    if (tokenInfo?.twitter) {
      socialLinks.push(
        `<a href="${tokenInfo.twitter}" target="_blank" class="pdd-social-link"><i class="icon-twitter"></i> Twitter</a>`
      );
    }
    if (tokenInfo?.telegram) {
      socialLinks.push(
        `<a href="${tokenInfo.telegram}" target="_blank" class="pdd-social-link"><i class="icon-smartphone"></i> Telegram</a>`
      );
    }

    const socialLinksHtml =
      socialLinks.length > 0
        ? `<div class="pdd-social-links">
            ${socialLinks.join("")}
          </div>`
        : "";

    // Pool info section
    const poolInfoHtml = poolInfo
      ? `<div class="pdd-pool-info">
          <h4>Pool Information</h4>
          <div class="pdd-pool-grid">
            ${poolInfo.dex_name ? `<div class="pdd-pool-row"><span class="label">DEX:</span><span class="value">${Utils.escapeHtml(poolInfo.dex_name)}</span></div>` : ""}
            ${poolInfo.pool_address ? `<div class="pdd-pool-row"><span class="label">Pool:</span><span class="value">${Utils.renderAddressChip(poolInfo.pool_address)}</span></div>` : ""}
            ${poolInfo.liquidity_sol !== null && poolInfo.liquidity_sol !== undefined ? `<div class="pdd-pool-row"><span class="label">Liquidity:</span><span class="value">${Utils.formatSol(poolInfo.liquidity_sol, { suffix: "" })} SOL</span></div>` : ""}
          </div>
        </div>`
      : "";

    // Market data section
    const marketDataHtml = marketData
      ? `<div class="pdd-market-data">
          <h4>Market Data</h4>
          <div class="pdd-market-grid">
            ${marketData.market_cap ? `<div class="pdd-market-item"><span class="label">Market Cap</span><span class="value">${Utils.formatCurrencyUSD(marketData.market_cap)}</span></div>` : ""}
            ${marketData.fdv ? `<div class="pdd-market-item"><span class="label">FDV</span><span class="value">${Utils.formatCurrencyUSD(marketData.fdv)}</span></div>` : ""}
            ${marketData.liquidity_usd ? `<div class="pdd-market-item"><span class="label">Liquidity</span><span class="value">${Utils.formatCurrencyUSD(marketData.liquidity_usd)}</span></div>` : ""}
            ${marketData.volume_24h ? `<div class="pdd-market-item"><span class="label">24h Volume</span><span class="value">${Utils.formatCurrencyUSD(marketData.volume_24h)}</span></div>` : ""}
            ${marketData.price_change_h1 !== null && marketData.price_change_h1 !== undefined ? `<div class="pdd-market-item"><span class="label">1h Change</span><span class="value ${marketData.price_change_h1 >= 0 ? "positive" : "negative"}">${marketData.price_change_h1 >= 0 ? "+" : ""}${marketData.price_change_h1.toFixed(2)}%</span></div>` : ""}
            ${marketData.price_change_h24 !== null && marketData.price_change_h24 !== undefined ? `<div class="pdd-market-item"><span class="label">24h Change</span><span class="value ${marketData.price_change_h24 >= 0 ? "positive" : "negative"}">${marketData.price_change_h24 >= 0 ? "+" : ""}${marketData.price_change_h24.toFixed(2)}%</span></div>` : ""}
            ${marketData.holder_count ? `<div class="pdd-market-item"><span class="label">Holders</span><span class="value">${Utils.formatCompactNumber(marketData.holder_count)}</span></div>` : ""}
          </div>
        </div>`
      : "";

    // External links section
    const explorerLinksHtml = externalLinks
      ? `<div class="pdd-explorer-links">
          <h4>Explore</h4>
          <div class="pdd-explorer-grid">
            <a href="${externalLinks.solscan}" target="_blank" class="pdd-explorer-link">Solscan</a>
            <a href="${externalLinks.dexscreener}" target="_blank" class="pdd-explorer-link">DexScreener</a>
            <a href="${externalLinks.birdeye}" target="_blank" class="pdd-explorer-link">Birdeye</a>
            <a href="${externalLinks.rugcheck}" target="_blank" class="pdd-explorer-link">RugCheck</a>
            <a href="${externalLinks.photon}" target="_blank" class="pdd-explorer-link">Photon</a>
          </div>
        </div>`
      : "";

    // Build security section HTML
    const securityHtml = security
      ? `
      <div class="pdd-security-full">
        <h4>Security Assessment</h4>
        <div class="pdd-security-header">
          <div class="pdd-score-display">
            ${score !== null ? `<div class="pdd-score-circle" style="--score: ${score}; --score-color: ${this._getScoreColor(score)}">${score}</div>` : '<div class="pdd-score-circle pdd-score-unknown">?</div>'}
            <span class="pdd-risk-label ${riskLabelClass}">${riskLabelText}</span>
          </div>
        </div>
        <div class="pdd-security-details">
          ${mintAuthorityHtml}
          ${freezeAuthorityHtml}
        </div>
        ${risksListHtml}
      </div>
    `
      : "";

    content.innerHTML = `
      <div class="pdd-token-tab">
        <div class="pdd-token-identity">
          ${logoHtml}
          <div class="pdd-token-details">
            <h3>${Utils.escapeHtml(pos.name || pos.symbol || "Unknown Token")} ${pos.symbol ? `(${Utils.escapeHtml(pos.symbol)})` : ""}</h3>
            <div class="pdd-token-address-row">
              ${Utils.renderAddressChip(pos.mint, { full: true, kind: "token" })}
            </div>
            ${descriptionHtml}
          </div>
        </div>

        ${socialLinksHtml}

        ${
          securityHtml || poolInfoHtml
            ? `
        <div class="pdd-overview-grid">
          ${securityHtml}
          ${poolInfoHtml}
        </div>
        `
            : ""
        }

        ${marketDataHtml}
        ${explorerLinksHtml}
      </div>
    `;

    // Attach copy handlers
    content.querySelectorAll(".pdd-copy-btn").forEach((btn) => {
      btn.addEventListener("click", () => {
        const text = btn.dataset.copy;
        if (text) {
          Utils.copyToClipboard(text);
          Utils.showToast("Copied!", "success");
        }
      });
    });

    content.querySelectorAll(".pdd-address").forEach((el) => {
      el.style.cursor = "pointer";
      el.addEventListener("click", () => {
        const address = el.dataset.address;
        if (address) {
          Utils.copyToClipboard(address);
          Utils.showToast("Address copied!", "success");
        }
      });
    });
  };

  /**
   * Get color for security score
   * NOTE: score_normalized from Rugcheck is 0-100 where LOWER = SAFER
   */
  proto._getScoreColor = function (score) {
    // Lower score = safer (green), higher score = riskier (red)
    if (score <= 20) return "var(--success)";
    if (score <= 50) return "var(--warning)";
    return "var(--error)";
  };
}
