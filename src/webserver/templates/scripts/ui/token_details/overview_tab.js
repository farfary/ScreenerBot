/**
 * Token Details Dialog - Overview Tab
 * Extracted from token_details_dialog.js to reduce file size
 */
import * as Utils from "../../core/utils.js";

/**
 * Render the overview tab content
 * @param {Object} token - Token data object
 * @param {Object} options - Rendering options
 * @param {Function} options.renderHintTrigger - Function to render hint triggers
 * @param {Function} options.escapeHtml - HTML escape function
 * @param {Function} options.formatShortAddress - Address formatting function
 * @param {Function} options.getRejectionDisplayLabel - Rejection label function
 * @returns {string} HTML string for overview tab
 */
export function renderOverviewTab(token, options = {}) {
  const { renderHintTrigger } = options;

  return `
    <div class="overview-split-layout">
      <div class="overview-left">
        ${renderOverviewLeft(token, options)}
      </div>
      <div class="overview-right">
        <div class="chart-container">
          <div class="chart-header">
            <div class="chart-header-left">
              <div class="chart-data-indicator" id="chartDataIndicator" tabindex="0" role="status">
                <span class="chart-data-dot"></span>
                <span class="chart-data-label">Data</span>
                <div class="chart-data-tip" id="chartDataTip" role="tooltip">
                  <div class="chart-data-tip-empty">Checking data…</div>
                </div>
              </div>
              ${renderHintTrigger("tokenDetails.chart")}
            </div>
            <div class="chart-ohlcv-display" id="chartOhlcvDisplay">
              <span class="ohlcv-item"><span class="ohlcv-label">O</span> <span class="ohlcv-value" id="ohlcvOpen">—</span></span>
              <span class="ohlcv-item"><span class="ohlcv-label">H</span> <span class="ohlcv-value" id="ohlcvHigh">—</span></span>
              <span class="ohlcv-item"><span class="ohlcv-label">L</span> <span class="ohlcv-value" id="ohlcvLow">—</span></span>
              <span class="ohlcv-item"><span class="ohlcv-label">C</span> <span class="ohlcv-value" id="ohlcvClose">—</span></span>
              <span class="ohlcv-change" id="ohlcvChange">—</span>
            </div>
            <div class="chart-controls">
              <div class="timeframe-buttons" id="timeframeButtons">
                <button class="timeframe-btn" data-tf="1m">1M</button>
                <button class="timeframe-btn active" data-tf="5m">5M</button>
                <button class="timeframe-btn" data-tf="15m">15M</button>
                <button class="timeframe-btn" data-tf="1h">1H</button>
                <button class="timeframe-btn" data-tf="4h">4H</button>
                <button class="timeframe-btn" data-tf="12h">12H</button>
                <button class="timeframe-btn" data-tf="1d">1D</button>
              </div>
            </div>
          </div>
          <div id="tradingview-chart" class="tradingview-chart">
            <div id="chartLoadingOverlay" class="chart-loading-overlay">
              <div class="chart-loading-content">
                <div class="chart-loading-spinner"></div>
                <div class="chart-loading-text">Loading chart data...</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;
}

/**
 * Render just the left column of the overview tab (quick stats + details).
 * Exposed so the dialog can repaint live-updating metrics in place without
 * rebuilding the chart on the right. Used by `_refreshOverviewTab`.
 * @param {Object} token - Token data object
 * @param {Object} options - Same options bag as renderOverviewTab
 * @returns {string} HTML string for the overview left column
 */
export function renderOverviewLeft(token, options = {}) {
  const { renderHintTrigger, escapeHtml, formatShortAddress, getRejectionDisplayLabel } = options;
  return `${buildQuickStats(token)}${buildOverviewContent(token, {
    renderHintTrigger,
    escapeHtml,
    formatShortAddress,
    getRejectionDisplayLabel,
  })}`;
}

function buildQuickStats(token) {
  const change24h = token.price_change_periods?.h24;
  const changeClass = change24h >= 0 ? "positive" : "negative";

  return `
    <div class="quick-stats">
      <div class="quick-stat">
        <span class="stat-label">Price</span>
        <span class="stat-value">${token.price_sol ? Utils.formatPriceSol(token.price_sol, { decimals: 9 }) + " SOL" : "—"}</span>
        ${change24h !== undefined ? `<span class="stat-change ${changeClass}">${formatChange(change24h)}</span>` : ""}
      </div>
      <div class="quick-stat">
        <span class="stat-label">Market Cap</span>
        <span class="stat-value">${token.market_cap ? Utils.formatCompactNumber(token.market_cap, { prefix: "$" }) : token.fdv ? Utils.formatCompactNumber(token.fdv, { prefix: "$" }) : "—"}</span>
      </div>
      <div class="quick-stat">
        <span class="stat-label">Liquidity</span>
        <span class="stat-value">${token.liquidity_usd ? Utils.formatCompactNumber(token.liquidity_usd, { prefix: "$" }) : token.pool_reserves_sol ? Utils.formatSol(token.pool_reserves_sol, { decimals: 2 }) : "—"}</span>
      </div>
      <div class="quick-stat">
        <span class="stat-label">Vol 24H</span>
        <span class="stat-value">${token.volume_24h ? Utils.formatCompactNumber(token.volume_24h, { prefix: "$" }) : "—"}</span>
      </div>
    </div>
  `;
}

function buildOverviewContent(token, options) {
  const { renderHintTrigger, escapeHtml, formatShortAddress, getRejectionDisplayLabel } = options;

  return `
    <div class="overview-grid">
      ${buildTokenInfoCard(token, { renderHintTrigger, escapeHtml, formatShortAddress, getRejectionDisplayLabel })}
      ${buildLiquidityCard(token, { renderHintTrigger, escapeHtml, formatShortAddress })}
      ${buildPriceChangesCard(token, { renderHintTrigger })}
      ${buildVolumeCard(token, { renderHintTrigger })}
      ${buildActivityCard(token, { renderHintTrigger })}
    </div>
  `;
}

function buildTokenInfoCard(token, options) {
  const { renderHintTrigger, escapeHtml, formatShortAddress, getRejectionDisplayLabel } = options;

  const age = token.pair_created_at
    ? Utils.formatTimeAgo(new Date(token.pair_created_at * 1000))
    : token.created_at
      ? Utils.formatTimeAgo(new Date(token.created_at * 1000))
      : "—";

  // Build tags display with wrapper to prevent height jump
  const tagsContent =
    token.tags && token.tags.length > 0
      ? `<div class="token-tags">${token.tags.map((t) => `<span class="token-tag">${escapeHtml(t)}</span>`).join("")}</div>`
      : '<div class="token-tags-placeholder">No tags</div>';
  const tagsHtml = `<div class="token-tags-wrapper">${tagsContent}</div>`;

  // Build filtering status display
  let filteringStatusHtml = "";
  if (token.last_rejection_reason) {
    const displayLabel = getRejectionDisplayLabel(token.last_rejection_reason);
    filteringStatusHtml = `
      <div class="info-cell full-width">
        <span class="cell-label">Filter Status</span>
        <span class="cell-value">
          <span class="status-badge rejected" title="${escapeHtml(token.last_rejection_reason)}">
            Rejected: ${escapeHtml(displayLabel)}
          </span>
        </span>
      </div>
    `;
  }

  return `
    <div class="info-card compact">
      <div class="card-header">
        <span>Token Info</span>
        <div class="card-header-actions">
          ${token.verified ? '<span class="verified-badge"><i class="icon-check"></i> Verified</span>' : ""}
          ${renderHintTrigger("tokenDetails.tokenInfo")}
        </div>
      </div>
      <div class="card-body">
        <div class="info-grid-2col">
          <div class="info-cell">
            <span class="cell-label">Mint</span>
            <span class="cell-value mono clickable" onclick="Utils.copyToClipboard('${token.mint}')" title="Click to copy">${formatShortAddress(token.mint)}</span>
          </div>
          <div class="info-cell">
            <span class="cell-label">Decimals</span>
            <span class="cell-value">${token.decimals ?? "—"}</span>
          </div>
          <div class="info-cell">
            <span class="cell-label">Age</span>
            <span class="cell-value">${age}</span>
          </div>
          <div class="info-cell">
            <span class="cell-label">DEX</span>
            <span class="cell-value">${token.pool_dex ? escapeHtml(token.pool_dex) : "—"}</span>
          </div>
          ${
            token.total_holders
              ? `
          <div class="info-cell">
            <span class="cell-label">Holders</span>
            <span class="cell-value">${Utils.formatNumber(token.total_holders, { decimals: 0 })}</span>
          </div>
          `
              : ""
          }
          ${
            token.top_10_concentration
              ? `
          <div class="info-cell">
            <span class="cell-label">Top 10 Hold</span>
            <span class="cell-value">${token.top_10_concentration.toFixed(1)}%</span>
          </div>
          `
              : ""
          }
          ${filteringStatusHtml}
        </div>
        ${tagsHtml}
        ${token.description ? `<div class="info-description">${escapeHtml(token.description)}</div>` : ""}
      </div>
    </div>
  `;
}

function buildLiquidityCard(token, options) {
  const { renderHintTrigger, formatShortAddress } = options;

  return `
    <div class="info-card compact">
      <div class="card-header">
        <span>Liquidity & Market</span>
        ${renderHintTrigger("tokenDetails.liquidity")}
      </div>
      <div class="card-body">
        <div class="info-grid-2col">
          <div class="info-cell highlight">
            <span class="cell-label">FDV</span>
            <span class="cell-value large">${token.fdv ? Utils.formatCurrencyUSD(token.fdv) : "—"}</span>
          </div>
          <div class="info-cell highlight">
            <span class="cell-label">Liquidity</span>
            <span class="cell-value large">${token.liquidity_usd ? Utils.formatCurrencyUSD(token.liquidity_usd) : "—"}</span>
          </div>
          <div class="info-cell">
            <span class="cell-label">Pool SOL</span>
            <span class="cell-value">${token.pool_reserves_sol ? Utils.formatNumber(token.pool_reserves_sol, { decimals: 2 }) + " SOL" : "—"}</span>
          </div>
          <div class="info-cell">
            <span class="cell-label">Pool Token</span>
            <span class="cell-value">${token.pool_reserves_token ? Utils.formatCompactNumber(token.pool_reserves_token) : "—"}</span>
          </div>
        </div>
        ${
          token.pool_address
            ? `
        <div class="pool-address">
          <span class="cell-label">Pool</span>
          <a href="https://solscan.io/account/${token.pool_address}" target="_blank" rel="noopener" class="pool-link">${formatShortAddress(token.pool_address)}</a>
        </div>
        `
            : ""
        }
      </div>
    </div>
  `;
}

function buildPriceChangesCard(token, options) {
  const { renderHintTrigger } = options;
  const changes = token.price_change_periods || {};

  return `
    <div class="info-card compact">
      <div class="card-header">
        <span>Price Changes</span>
        ${renderHintTrigger("tokenDetails.priceChanges")}
      </div>
      <div class="card-body">
        <div class="change-grid">
          <div class="change-item">
            <span class="change-label">5M</span>
            <span class="change-value ${getChangeClass(changes.m5)}">${formatChange(changes.m5)}</span>
          </div>
          <div class="change-item">
            <span class="change-label">1H</span>
            <span class="change-value ${getChangeClass(changes.h1)}">${formatChange(changes.h1)}</span>
          </div>
          <div class="change-item">
            <span class="change-label">6H</span>
            <span class="change-value ${getChangeClass(changes.h6)}">${formatChange(changes.h6)}</span>
          </div>
          <div class="change-item">
            <span class="change-label">24H</span>
            <span class="change-value ${getChangeClass(changes.h24)}">${formatChange(changes.h24)}</span>
          </div>
        </div>
      </div>
    </div>
  `;
}

function buildVolumeCard(token, options) {
  const { renderHintTrigger } = options;
  const volumes = token.volume_periods || {};

  return `
    <div class="info-card compact">
      <div class="card-header">
        <span>Trading Volume</span>
        ${renderHintTrigger("tokenDetails.volume")}
      </div>
      <div class="card-body">
        <div class="volume-grid-4">
          <div class="volume-item">
            <span class="volume-label">5M</span>
            <span class="volume-value">${volumes.m5 ? Utils.formatCompactNumber(volumes.m5, { prefix: "$" }) : "—"}</span>
          </div>
          <div class="volume-item">
            <span class="volume-label">1H</span>
            <span class="volume-value">${volumes.h1 ? Utils.formatCompactNumber(volumes.h1, { prefix: "$" }) : "—"}</span>
          </div>
          <div class="volume-item">
            <span class="volume-label">6H</span>
            <span class="volume-value">${volumes.h6 ? Utils.formatCompactNumber(volumes.h6, { prefix: "$" }) : "—"}</span>
          </div>
          <div class="volume-item">
            <span class="volume-label">24H</span>
            <span class="volume-value">${volumes.h24 ? Utils.formatCompactNumber(volumes.h24, { prefix: "$" }) : "—"}</span>
          </div>
        </div>
      </div>
    </div>
  `;
}

function buildActivityCard(token, options) {
  const { renderHintTrigger } = options;
  const txns = token.txn_periods || {};
  const buySellRatio = token.buy_sell_ratio_24h;
  const ratioClass = buySellRatio
    ? buySellRatio > 1
      ? "bullish"
      : buySellRatio < 1
        ? "bearish"
        : "neutral"
    : "";

  const h24 = txns.h24;
  const buys24 = typeof token.buys_24h === "number" ? token.buys_24h : h24?.buys;
  const sells24 = typeof token.sells_24h === "number" ? token.sells_24h : h24?.sells;
  const total24 =
    (typeof buys24 === "number" ? buys24 : 0) + (typeof sells24 === "number" ? sells24 : 0);
  const buyPct24 = total24 > 0 && typeof buys24 === "number" ? (buys24 / total24) * 100 : null;

  const m5 = txns.m5;
  const h1 = txns.h1;
  const total5m =
    (typeof m5?.buys === "number" ? m5.buys : 0) + (typeof m5?.sells === "number" ? m5.sells : 0);
  const total1h =
    (typeof h1?.buys === "number" ? h1.buys : 0) + (typeof h1?.sells === "number" ? h1.sells : 0);
  const rate5m = total5m / 5;
  const rate1h = total1h / 60;
  const spikeFactor = rate1h > 0 ? rate5m / rate1h : null;

  const netFlow24h = typeof token.net_flow_24h === "number" ? token.net_flow_24h : null;
  const netFlowLabel =
    typeof netFlow24h === "number"
      ? netFlow24h > 0
        ? `+${Utils.formatNumber(netFlow24h, { decimals: 0 })}`
        : Utils.formatNumber(netFlow24h, { decimals: 0 })
      : "—";
  const netFlowClass =
    typeof netFlow24h === "number" ? (netFlow24h >= 0 ? "positive" : "negative") : "";

  return `
    <div class="info-card compact">
      <div class="card-header">
        <span>Transaction Activity</span>
        <div class="card-header-actions">
          ${
            typeof buyPct24 === "number"
              ? `<span class="ratio-badge ${buyPct24 >= 50 ? "bullish" : "bearish"}">${buyPct24.toFixed(0)}% Buy</span>`
              : ""
          }
          ${buySellRatio ? `<span class="ratio-badge ${ratioClass}">${buySellRatio.toFixed(2)} B/S</span>` : ""}
          ${renderHintTrigger("tokenDetails.activity")}
        </div>
      </div>
      <div class="card-body">
        <div class="txn-grid">
          ${buildTxnRow("5M", txns.m5, { minutes: 5 })}
          ${buildTxnRow("1H", txns.h1, { minutes: 60 })}
          ${buildTxnRow("6H", txns.h6, { minutes: 360 })}
          ${buildTxnRow("24H", txns.h24, { minutes: 1440 })}
        </div>
        ${
          typeof token.buys_24h === "number" || typeof token.sells_24h === "number"
            ? `
        <div class="txn-summary">
          <div class="txn-summary-item buys">
            <span class="summary-icon">↗</span>
            <span class="summary-value">${typeof token.buys_24h === "number" ? Utils.formatNumber(token.buys_24h, { decimals: 0 }) : "—"}</span>
            <span class="summary-label">Buys</span>
          </div>
          <div class="txn-summary-item sells">
            <span class="summary-icon">↘</span>
            <span class="summary-value">${typeof token.sells_24h === "number" ? Utils.formatNumber(token.sells_24h, { decimals: 0 }) : "—"}</span>
            <span class="summary-label">Sells</span>
          </div>
          ${
            typeof token.net_flow_24h === "number"
              ? `
          <div class="txn-summary-item flow ${netFlowClass}">
            <span class="summary-icon">${netFlow24h >= 0 ? "+" : "−"}</span>
            <span class="summary-value">${netFlowLabel}</span>
            <span class="summary-label">Net</span>
          </div>
          `
              : ""
          }
        </div>
        `
            : ""
        }

        <div class="txn-insights">
          <div class="txn-insight">
            <div class="insight-label">24H Total</div>
            <div class="insight-value mono">${total24 > 0 ? Utils.formatNumber(total24, { decimals: 0 }) : "—"}</div>
          </div>
          <div class="txn-insight">
            <div class="insight-label">24H Avg</div>
            <div class="insight-value mono">${
              total24 > 0 ? `${Utils.formatNumber(total24 / 24, { decimals: 1 })}/h` : "—"
            }</div>
          </div>
          <div class="txn-insight">
            <div class="insight-label">5M Spike</div>
            <div class="insight-value mono">${
              typeof spikeFactor === "number" && Number.isFinite(spikeFactor)
                ? `${Utils.formatNumber(spikeFactor, { decimals: 2 })}×`
                : "—"
            }</div>
          </div>
        </div>
      </div>
    </div>
  `;
}

function buildTxnRow(label, data, { minutes }) {
  const buysRaw = data?.buys;
  const sellsRaw = data?.sells;

  const hasBuys = typeof buysRaw === "number";
  const hasSells = typeof sellsRaw === "number";
  const hasAny = hasBuys || hasSells;

  const buys = hasBuys ? buysRaw : null;
  const sells = hasSells ? sellsRaw : null;
  const total = (typeof buys === "number" ? buys : 0) + (typeof sells === "number" ? sells : 0);

  const buyPct = total > 0 && typeof buys === "number" ? (buys / total) * 100 : 50;
  const sellPct = 100 - buyPct;

  const countsTitle = hasAny
    ? `Buys: ${typeof buys === "number" ? buys : "—"} (${total > 0 ? buyPct.toFixed(0) : "—"}%), Sells: ${typeof sells === "number" ? sells : "—"} (${total > 0 ? sellPct.toFixed(0) : "—"}%), Total: ${total}`
    : "No transaction data";

  const buyText = typeof buys === "number" ? Utils.formatNumber(buys, { decimals: 0 }) : "—";
  const sellText = typeof sells === "number" ? Utils.formatNumber(sells, { decimals: 0 }) : "—";
  const ratePerMin = minutes && total >= 0 ? total / minutes : null;
  const rateText = hasAny ? `${Utils.formatNumber(ratePerMin ?? 0, { decimals: 1 })}/m` : "—";
  const pctText =
    total > 0 ? `${buyPct.toFixed(0)}% / ${sellPct.toFixed(0)}%` : hasAny ? "0% / 0%" : "—";

  return `
    <div class="txn-row" title="${countsTitle}">
      <div class="txn-time">
        <div class="txn-label">${label}</div>
        <div class="txn-rate">${rateText}</div>
      </div>
      <div class="txn-bar-container ${hasAny ? "" : "is-empty"}" aria-label="${countsTitle}">
        <div class="txn-bar buy-bar" style="width: ${buyPct}%"></div>
        <div class="txn-bar sell-bar" style="width: ${sellPct}%"></div>
      </div>
      <div class="txn-counts">
        <div class="txn-counts-main">
          <span class="buy-count">${buyText}</span>
          <span class="separator">/</span>
          <span class="sell-count">${sellText}</span>
        </div>
        <div class="txn-counts-sub">${pctText}</div>
      </div>
    </div>
  `;
}

// Helper functions

function formatChange(change) {
  if (change === undefined || change === null) return "—";
  const formatted = Math.abs(change).toFixed(2);
  return change >= 0 ? `+${formatted}%` : `${formatted}%`;
}

function getChangeClass(change) {
  if (change === undefined || change === null) return "";
  return change >= 0 ? "positive" : "negative";
}
