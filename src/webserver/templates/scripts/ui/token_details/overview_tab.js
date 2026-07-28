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
        <div class="overview-banner-slot" id="overviewBannerSlot">${renderOverviewBanner(token)}</div>
        <div id="overviewLive">${renderOverviewLeft(token, options)}</div>
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
  return `
    <div class="overview-sheet">
      ${buildHeadlineMetrics(token)}
      ${buildOverviewContent(token, {
        renderHintTrigger,
        escapeHtml,
        formatShortAddress,
        getRejectionDisplayLabel,
      })}
    </div>
  `;
}

/**
 * Render the token's wide banner, or nothing when it has none.
 *
 * Kept OUT of `renderOverviewLeft` on purpose: that column is re-rendered via
 * innerHTML on every poll tick whose metrics changed, which would recreate the
 * <img> element each time. This lives in its own slot that only repaints when
 * the banner URL itself changes (see `_refreshOverviewTab`).
 *
 * @param {Object} token - Token data object
 * @returns {string} HTML string, or "" when the token has no banner
 */
export function renderOverviewBanner(token) {
  const url = Utils.resolveTokenBannerUrl(token);
  if (!url) return "";

  // A dead banner URL removes the element entirely rather than leaving a broken
  // image frame -- the banner is strictly optional chrome.
  return `
    <div
      class="token-banner"
      role="button"
      tabindex="0"
      aria-label="Open token banner"
    >
      <img
        src="${Utils.escapeHtml(url)}"
        alt=""
        class="token-banner-img"
        loading="lazy"
        onerror="this.closest('.token-banner').remove()"
      />
    </div>
  `;
}

function buildHeadlineMetrics(token) {
  const change24h = token.price_change_periods?.h24;
  const hasChange24h = typeof change24h === "number";
  const changeClass = getChangeClass(change24h);

  return `
    <div class="overview-headline" aria-label="Headline market metrics">
      <div class="overview-headline-item">
        <span class="overview-headline-label">Price</span>
        <span class="overview-headline-readout">
          <span class="overview-headline-value">${token.price_sol ? Utils.formatPriceSubscript(token.price_sol, { precision: 5 }) + " SOL" : "—"}</span>
          ${hasChange24h ? `<span class="overview-headline-change ${changeClass}">${formatChange(change24h)}</span>` : ""}
        </span>
      </div>
      <div class="overview-headline-item">
        <span class="overview-headline-label">Market Cap</span>
        <span class="overview-headline-value">${token.market_cap ? Utils.formatCompactNumber(token.market_cap, { prefix: "$" }) : token.fdv ? Utils.formatCompactNumber(token.fdv, { prefix: "$" }) : "—"}</span>
      </div>
      <div class="overview-headline-item">
        <span class="overview-headline-label">Liquidity</span>
        <span class="overview-headline-value">${token.liquidity_usd ? Utils.formatCompactNumber(token.liquidity_usd, { prefix: "$" }) : token.pool_reserves_sol ? Utils.formatSol(token.pool_reserves_sol, { decimals: 2 }) : "—"}</span>
      </div>
      <div class="overview-headline-item">
        <span class="overview-headline-label">Vol 24H</span>
        <span class="overview-headline-value">${token.volume_24h ? Utils.formatCompactNumber(token.volume_24h, { prefix: "$" }) : "—"}</span>
      </div>
    </div>
  `;
}

function buildOverviewContent(token, options) {
  const { renderHintTrigger, escapeHtml, formatShortAddress, getRejectionDisplayLabel } = options;

  return `
    <div class="overview-sections">
      ${buildTokenInfoSection(token, { renderHintTrigger, escapeHtml, formatShortAddress, getRejectionDisplayLabel })}
      ${buildLiquiditySection(token, { renderHintTrigger, formatShortAddress })}
      ${buildMarketPulseSection(token, { renderHintTrigger })}
      ${buildActivitySection(token, { renderHintTrigger })}
    </div>
  `;
}

function buildTokenInfoSection(token, options) {
  const { renderHintTrigger, escapeHtml, formatShortAddress, getRejectionDisplayLabel } = options;

  const age = token.pair_created_at
    ? Utils.formatTimeAgo(new Date(token.pair_created_at * 1000))
    : token.created_at
      ? Utils.formatTimeAgo(new Date(token.created_at * 1000))
      : "—";

  const tagsContent =
    token.tags && token.tags.length > 0
      ? `<div class="overview-tags">${token.tags.map((tag) => `<span class="overview-tag">${escapeHtml(tag)}</span>`).join("")}</div>`
      : '<span class="overview-tags-empty">No tags</span>';

  let filteringStatusHtml = "";
  if (token.last_rejection_reason) {
    const displayLabel = getRejectionDisplayLabel(token.last_rejection_reason);
    filteringStatusHtml = `
      <div class="overview-fact overview-fact-wide">
        <span class="overview-fact-label">Filter Status</span>
        <span
          class="overview-fact-value overview-filter-status"
          title="${escapeHtml(token.last_rejection_reason)}"
        >
          Rejected · ${escapeHtml(displayLabel)}
        </span>
      </div>
    `;
  }

  return `
    <section class="overview-section">
      <div class="overview-section-header">
        <span class="overview-section-title">Token Info</span>
        <div class="overview-section-actions">
          ${token.verified ? '<span class="overview-verified"><i class="icon-check"></i> Verified</span>' : ""}
          ${renderHintTrigger("tokenDetails.tokenInfo")}
        </div>
      </div>
      <div class="overview-facts">
          <div class="overview-fact">
            <span class="overview-fact-label">Mint</span>
            <button
              type="button"
              class="overview-copy-value"
              data-copy="${escapeHtml(token.mint)}"
              title="Copy mint address"
              aria-label="Copy mint address"
            >${formatShortAddress(token.mint)}</button>
          </div>
          <div class="overview-fact">
            <span class="overview-fact-label">Decimals</span>
            <span class="overview-fact-value">${token.decimals ?? "—"}</span>
          </div>
          <div class="overview-fact">
            <span class="overview-fact-label">Age</span>
            <span class="overview-fact-value">${age}</span>
          </div>
          <div class="overview-fact">
            <span class="overview-fact-label">DEX</span>
            <span class="overview-fact-value">${token.pool_dex ? escapeHtml(token.pool_dex) : "—"}</span>
          </div>
          ${
            token.total_holders
              ? `
          <div class="overview-fact">
            <span class="overview-fact-label">Holders</span>
            <span class="overview-fact-value">${Utils.formatNumber(token.total_holders, { decimals: 0 })}</span>
          </div>
          `
              : ""
          }
          ${
            token.top_10_concentration
              ? `
          <div class="overview-fact">
            <span class="overview-fact-label">Top 10 Hold</span>
            <span class="overview-fact-value">${token.top_10_concentration.toFixed(1)}%</span>
          </div>
          `
              : ""
          }
          ${filteringStatusHtml}
      </div>
      <div class="overview-tags-row">
        <span class="overview-inline-label">Tags</span>
        ${tagsContent}
      </div>
      ${token.description ? `<p class="overview-description">${escapeHtml(token.description)}</p>` : ""}
    </section>
  `;
}

function buildLiquiditySection(token, options) {
  const { renderHintTrigger, formatShortAddress } = options;

  return `
    <section class="overview-section">
      <div class="overview-section-header">
        <span class="overview-section-title">Liquidity & Market</span>
        ${renderHintTrigger("tokenDetails.liquidity")}
      </div>
      <div class="overview-facts overview-market-facts">
          <div class="overview-fact overview-fact-emphasis">
            <span class="overview-fact-label">FDV</span>
            <span class="overview-fact-value">${token.fdv ? Utils.formatCurrencyUSD(token.fdv) : "—"}</span>
          </div>
          <div class="overview-fact overview-fact-emphasis">
            <span class="overview-fact-label">Liquidity</span>
            <span class="overview-fact-value">${token.liquidity_usd ? Utils.formatCurrencyUSD(token.liquidity_usd) : "—"}</span>
          </div>
          <div class="overview-fact">
            <span class="overview-fact-label">Pool SOL</span>
            <span class="overview-fact-value">${token.pool_reserves_sol ? Utils.formatNumber(token.pool_reserves_sol, { decimals: 2 }) + " SOL" : "—"}</span>
          </div>
          <div class="overview-fact">
            <span class="overview-fact-label">Pool Token</span>
            <span class="overview-fact-value">${token.pool_reserves_token ? Utils.formatCompactNumber(token.pool_reserves_token) : "—"}</span>
          </div>
      </div>
        ${
          token.pool_address
            ? `
        <div class="overview-pool-row">
          <span class="overview-inline-label">Pool</span>
          <a href="https://solscan.io/account/${token.pool_address}" target="_blank" rel="noopener" class="overview-pool-link">${formatShortAddress(token.pool_address)}</a>
        </div>
        `
            : ""
        }
    </section>
  `;
}

function buildMarketPulseSection(token, options) {
  const { renderHintTrigger } = options;
  const changes = token.price_change_periods || {};
  const volumes = token.volume_periods || {};

  return `
    <section class="overview-section">
      <div class="overview-section-header">
        <span class="overview-section-title">Market Pulse</span>
        ${renderHintTrigger("tokenDetails.marketPulse")}
      </div>
      <div class="overview-pulse-matrix">
        <span aria-hidden="true"></span>
        <span class="overview-pulse-time">5M</span>
        <span class="overview-pulse-time">1H</span>
        <span class="overview-pulse-time">6H</span>
        <span class="overview-pulse-time">24H</span>
        <span class="overview-pulse-label">Price</span>
        ${buildPulseChange(changes.m5)}
        ${buildPulseChange(changes.h1)}
        ${buildPulseChange(changes.h6)}
        ${buildPulseChange(changes.h24)}
        <span class="overview-pulse-label">Volume</span>
        ${buildPulseVolume(volumes.m5)}
        ${buildPulseVolume(volumes.h1)}
        ${buildPulseVolume(volumes.h6)}
        ${buildPulseVolume(volumes.h24)}
      </div>
    </section>
  `;
}

function buildPulseChange(value) {
  return `<span class="overview-pulse-value ${getChangeClass(value)}">${formatChange(value)}</span>`;
}

function buildPulseVolume(value) {
  return `<span class="overview-pulse-volume">${value ? Utils.formatCompactNumber(value, { prefix: "$" }) : "—"}</span>`;
}

function buildActivitySection(token, options) {
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
    <section class="overview-section">
      <div class="overview-section-header">
        <span class="overview-section-title">Transaction Activity</span>
        <div class="overview-section-actions">
          ${
            typeof buyPct24 === "number"
              ? `<span class="overview-ratio ${buyPct24 >= 50 ? "bullish" : "bearish"}">${buyPct24.toFixed(0)}% Buy</span>`
              : ""
          }
          ${buySellRatio ? `<span class="overview-ratio ${ratioClass}">${buySellRatio.toFixed(2)} B/S</span>` : ""}
          ${renderHintTrigger("tokenDetails.activity")}
        </div>
      </div>
      <div class="overview-flow">
        ${buildFlowRow("5M", txns.m5, { minutes: 5 })}
        ${buildFlowRow("1H", txns.h1, { minutes: 60 })}
        ${buildFlowRow("6H", txns.h6, { minutes: 360 })}
        ${buildFlowRow("24H", txns.h24, { minutes: 1440 })}
      </div>
      <div class="overview-flow-summary">
        <div class="overview-flow-stat buys">
          <span class="overview-flow-stat-label">Buys 24H</span>
          <span class="overview-flow-stat-value">${typeof buys24 === "number" ? Utils.formatNumber(buys24, { decimals: 0 }) : "—"}</span>
        </div>
        <div class="overview-flow-stat sells">
          <span class="overview-flow-stat-label">Sells 24H</span>
          <span class="overview-flow-stat-value">${typeof sells24 === "number" ? Utils.formatNumber(sells24, { decimals: 0 }) : "—"}</span>
        </div>
        <div class="overview-flow-stat ${netFlowClass}">
          <span class="overview-flow-stat-label">Net Flow</span>
          <span class="overview-flow-stat-value">${netFlowLabel}</span>
        </div>
        <div class="overview-flow-stat">
          <span class="overview-flow-stat-label">24H Total</span>
          <span class="overview-flow-stat-value">${total24 > 0 ? Utils.formatNumber(total24, { decimals: 0 }) : "—"}</span>
        </div>
        <div class="overview-flow-stat">
          <span class="overview-flow-stat-label">24H Avg</span>
          <span class="overview-flow-stat-value">${
            total24 > 0 ? `${Utils.formatNumber(total24 / 24, { decimals: 1 })}/h` : "—"
          }</span>
        </div>
        <div class="overview-flow-stat">
          <span class="overview-flow-stat-label">5M Spike</span>
          <span class="overview-flow-stat-value">${
            typeof spikeFactor === "number" && Number.isFinite(spikeFactor)
              ? `${Utils.formatNumber(spikeFactor, { decimals: 2 })}×`
              : "—"
          }</span>
        </div>
      </div>
    </section>
  `;
}

function buildFlowRow(label, data, { minutes }) {
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
    <div class="overview-flow-row" title="${countsTitle}">
      <div class="overview-flow-time">
        <span class="overview-flow-period">${label}</span>
        <span class="overview-flow-rate">${rateText}</span>
      </div>
      <div class="overview-flow-bar ${hasAny ? "" : "is-empty"}" aria-label="${countsTitle}">
        <span class="overview-flow-bar-buy" style="width: ${buyPct}%"></span>
        <span class="overview-flow-bar-sell" style="width: ${sellPct}%"></span>
      </div>
      <div class="overview-flow-counts">
        <div class="overview-flow-counts-main">
          <span class="overview-flow-buy-count">${buyText}</span>
          <span class="overview-flow-separator">/</span>
          <span class="overview-flow-sell-count">${sellText}</span>
        </div>
        <span class="overview-flow-counts-sub">${pctText}</span>
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
