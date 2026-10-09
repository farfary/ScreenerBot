// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Token Details Dialog - Transactions Tab Mixin
 * Extracted from token_details_dialog.js to reduce file size
 * Handles transaction history display and chart
 */
import * as Utils from "../../core/utils.js";
import { withSolUnit } from "../../core/format.js";
import { requestManager } from "../../core/request_manager.js";
import { renderTabState } from "./state_handling.js";
import { typeKind, typeLabel, typeShortLabel } from "../transaction_type.js";

/**
 * Apply transactions tab mixin to TokenDetailsDialog class
 * @param {class} DialogClass - TokenDetailsDialog class
 */
export function applyTransactionsTabMixin(DialogClass) {
  const proto = DialogClass.prototype;

  /**
   * Load transactions tab content
   * @private
   * @param {HTMLElement} content - Tab content container
   */
  proto._loadTransactionsTab = async function (content) {
    if (content.dataset.loaded === "true") return;

    content.innerHTML = renderTabState({
      kind: "loading",
      message: I18n.t("tokens-transactions-loading"),
    });

    try {
      // Fetch 24h of transactions (limit 1000 usually enough for chart unless very high volume)
      const response = await requestManager.fetch(
        `/api/tokens/${this.tokenData.mint}/transactions?limit=1000`
      );

      if (Array.isArray(response)) {
        const transactions = response;

        if (transactions.length === 0) {
          content.innerHTML = renderTabState({
            icon: "icon-activity",
            title: I18n.t("tokens-transactions-empty-title"),
            message: I18n.t("tokens-transactions-empty-history"),
          });
          content.dataset.loaded = "true";
          return;
        }

        content.innerHTML = this._buildTransactionsHTML(transactions);

        // Wait for DOM
        setTimeout(() => {
          this._renderTransactionsChart(transactions);
          this._renderTransactionsList(transactions);
        }, 50);

        content.dataset.loaded = "true";
      } else {
        content.innerHTML = renderTabState({
          icon: "icon-activity",
          title: I18n.t("tokens-transactions-empty-title"),
          message: I18n.t("tokens-transactions-empty-data"),
        });
        content.dataset.loaded = "true";
      }
    } catch (err) {
      console.error("Failed to load transactions:", err);
      this._renderTabError(content, {
        title: I18n.t("tokens-transactions-error-title"),
        message: I18n.t("tokens-transactions-error-message"),
      });
    }
  };

  /**
   * Build transactions tab HTML structure
   * @private
   * @returns {string} HTML string
   */
  proto._buildTransactionsHTML = function (transactions) {
    const stats = summarizeTransactions(transactions);
    const esc = (text) => this._escapeHtml(text);
    return `
      <div class="transactions-container">
        <section class="transactions-overview">
          <div class="transactions-section-heading">
            <div>
              <strong>${esc(I18n.t("tokens-transactions-activity-title"))}</strong>
              <span>${esc(I18n.t("tokens-transactions-activity-subtitle"))}</span>
            </div>
          </div>
          <div class="transactions-metrics">
            <div><span>${esc(I18n.t("tokens-transactions-metric-total"))}</span><strong>${Utils.formatNumber(stats.total, { decimals: 0 })}</strong></div>
            <div><span>${esc(I18n.t("tokens-transactions-metric-buys"))}</span><strong class="positive">${Utils.formatNumber(stats.buys, { decimals: 0 })}</strong></div>
            <div><span>${esc(I18n.t("tokens-transactions-metric-sells"))}</span><strong class="negative">${Utils.formatNumber(stats.sells, { decimals: 0 })}</strong></div>
          </div>
          <div id="txns-chart" class="transactions-chart" dir="ltr"></div>
        </section>

        <section class="transactions-list-section">
          <div class="transactions-section-heading">
            <div>
              <strong>${esc(I18n.t("tokens-transactions-recent-title"))}</strong>
              <span>${esc(I18n.t("tokens-transactions-shown", { count: Math.min(transactions.length, 100) }))}</span>
            </div>
          </div>
          <div class="transactions-table-shell">
            <div class="transactions-table-header" aria-hidden="true">
              <span>${esc(I18n.t("tokens-transactions-column-time"))}</span>
              <span>${esc(I18n.t("tokens-transactions-column-type"))}</span>
              <span class="transaction-price-cell">${esc(I18n.t("tokens-transactions-column-price"))}</span>
              <span>${esc(I18n.t("tokens-transactions-column-total"))}</span>
              <span></span>
            </div>
            <div id="txns-list" class="transactions-list"></div>
          </div>
        </section>
      </div>
    `;
  };

  /**
   * Render transactions chart (hourly histogram)
   * @private
   * @param {Array} transactions - Transaction data
   */
  proto._renderTransactionsChart = function (transactions) {
    const chartContainer = this.dialogEl?.querySelector("#txns-chart");
    if (!chartContainer) return;
    this._disposeTransactionsChart();

    // Aggregate by hour buckets
    const buckets = {};
    const now = Math.floor(Date.now() / 1000);
    const start = now - 24 * 3600;

    // Initialize buckets
    for (let i = 0; i < 24; i++) {
      const ts = start + i * 3600;
      const hourKey = Math.floor(ts / 3600) * 3600;
      buckets[hourKey] = { time: hourKey, value: 0 };
    }

    transactions.forEach((tx) => {
      // Use timestamp (ISO string from backend)
      const date = new Date(tx.timestamp);
      const ts = date.getTime() / 1000;

      if (ts < start) return;
      const hourKey = Math.floor(ts / 3600) * 3600;
      if (!buckets[hourKey]) buckets[hourKey] = { time: hourKey, value: 0 };
      buckets[hourKey].value += 1;
    });

    const data = Object.values(buckets).sort((a, b) => a.time - b.time);

    // Create Chart
    // Ensure LightweightCharts is available
    if (!window.LightweightCharts) {
      chartContainer.textContent = I18n.t("tokens-transactions-chart-missing");
      return;
    }

    const styles = window.getComputedStyle(this.dialogEl);
    const chart = window.LightweightCharts.createChart(chartContainer, {
      layout: {
        background: { type: "solid", color: "transparent" },
        textColor: styles.getPropertyValue("--text-muted").trim(),
      },
      grid: {
        vertLines: { visible: false },
        horzLines: { color: styles.getPropertyValue("--border-color").trim() },
      },
      rightPriceScale: { borderVisible: false, scaleMargins: { top: 0.1, bottom: 0 } },
      timeScale: { borderVisible: false, timeVisible: true, secondsVisible: false },
      crosshair: { vertLine: { labelVisible: false }, horzLine: { labelVisible: false } },
    });

    const series = chart.addHistogramSeries({
      color: styles.getPropertyValue("--success-color").trim(),
    });
    series.setData(data);
    chart.timeScale().fitContent();

    // Auto-resize
    this.txChartResizeObserver = new ResizeObserver((entries) => {
      if (entries.length === 0 || !entries[0].contentRect) return;
      const { width, height } = entries[0].contentRect;
      chart.applyOptions({ width, height });
    });
    this.txChartResizeObserver.observe(chartContainer);

    this.txChart = chart;
  };

  proto._disposeTransactionsChart = function () {
    if (this.txChartResizeObserver) {
      this.txChartResizeObserver.disconnect();
      this.txChartResizeObserver = null;
    }
    if (this.txChart) {
      this.txChart.remove();
      this.txChart = null;
    }
  };

  /**
   * Render transactions list table
   * @private
   * @param {Array} transactions - Transaction data
   */
  proto._renderTransactionsList = function (transactions) {
    const container = this.dialogEl?.querySelector("#txns-list");
    if (!container) return;

    const recent = transactions.slice(0, 100);

    // Simple HTML table for speed
    const rows = recent
      .map((tx) => {
        const type = tx.transaction_type || tx.type;
        const label = typeShortLabel(type);
        const fullLabel = typeLabel(type);
        const kind = transactionKind(tx);
        const timeDisplay = Utils.formatTimestamp(new Date(tx.timestamp), { includeDate: false });
        const price = tx.price_sol
          ? Utils.formatPriceSubscript(tx.price_sol, { trim: false })
          : "—";
        const amount = tx.amount_sol !== undefined ? tx.amount_sol : Math.abs(tx.native_delta || 0);
        const total = withSolUnit(Utils.formatNumber(amount, { decimals: 2 }));

        const rowInner = `
          <span class="transaction-time">${this._escapeHtml(timeDisplay)}</span>
          <strong class="transaction-kind ${kind}" title="${this._escapeHtml(fullLabel)}">${this._escapeHtml(label)}</strong>
          <span class="transaction-price-cell">${price}</span>
          <span class="transaction-total">${this._escapeHtml(total)}</span>
          <i class="icon-external-link transaction-external" aria-hidden="true"></i>
        `;

        return tx.signature
          ? `
            <a class="token-transaction-row" href="${Utils.solscanTxUrl(tx.signature)}" target="_blank" rel="noopener noreferrer" title="${this._escapeHtml(I18n.t("tokens-transactions-view-solscan"))}">
              ${rowInner}
            </a>
          `
          : `
            <div class="token-transaction-row">
              ${rowInner}
            </div>
          `;
      })
      .join("");

    container.innerHTML = rows;
  };
}

/** The buy/sell/other colour class, from the shared type discriminant. */
function transactionKind(transaction) {
  const kind = typeKind(transaction.transaction_type || transaction.type);
  const direction = (transaction.direction || "").toLowerCase();
  if (kind === "buy" || (kind === "swap" && direction === "incoming")) return "buy";
  if (kind === "sell" || (kind === "swap" && direction === "outgoing")) return "sell";
  return "other";
}

function summarizeTransactions(transactions) {
  const cutoff = Date.now() - 24 * 60 * 60 * 1000;
  return transactions.reduce(
    (summary, transaction) => {
      const timestamp = new Date(transaction.timestamp).getTime();
      if (!Number.isFinite(timestamp) || timestamp < cutoff) return summary;
      summary.total += 1;
      const kind = transactionKind(transaction);
      if (kind === "buy") summary.buys += 1;
      if (kind === "sell") summary.sells += 1;
      return summary;
    },
    { total: 0, buys: 0, sells: 0 }
  );
}
