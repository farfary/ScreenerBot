// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Token Details Dialog - Overview Tab
 * Extracted from token_details_dialog.js to reduce file size
 */
import * as Utils from "../../core/utils.js";
import { formatFixed, formatPercentValue, withSolUnit } from "../../core/format.js";
import { venueLabel } from "../venue.js";

const esc = (text) => Utils.escapeHtml(text);

/** Buy share as a whole-number percentage without a sign. */
function wholePercent(value) {
  return formatPercentValue(value, { decimals: 0, plus: "" });
}

/**
 * Render the overview tab content
 * @param {Object} token - Token data object
 * @param {Object} options - Rendering options
 * @param {Function} options.renderHintTrigger - Function to render hint triggers
 * @param {Function} options.escapeHtml - HTML escape function
 * @param {Function} options.formatShortAddress - Address formatting function
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
                <span class="chart-data-label">${esc(I18n.t("chart-data"))}</span>
                <div class="chart-data-tip" id="chartDataTip" role="tooltip">
                  <div class="chart-data-tip-empty">${esc(I18n.t("tokens-overview-chart-checking"))}</div>
                </div>
              </div>
              ${renderHintTrigger("tokenDetails.chart")}
            </div>
            <div class="chart-ohlcv-display" id="chartOhlcvDisplay">
              <span class="ohlcv-item"><span class="ohlcv-label">${esc(I18n.t("chart-ohlc-open"))}</span> <span class="ohlcv-value" id="ohlcvOpen">—</span></span>
              <span class="ohlcv-item"><span class="ohlcv-label">${esc(I18n.t("chart-ohlc-high"))}</span> <span class="ohlcv-value" id="ohlcvHigh">—</span></span>
              <span class="ohlcv-item"><span class="ohlcv-label">${esc(I18n.t("chart-ohlc-low"))}</span> <span class="ohlcv-value" id="ohlcvLow">—</span></span>
              <span class="ohlcv-item"><span class="ohlcv-label">${esc(I18n.t("chart-ohlc-close"))}</span> <span class="ohlcv-value" id="ohlcvClose">—</span></span>
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
                <div class="chart-loading-text">${esc(I18n.t("chart-loading"))}</div>
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
  const { renderHintTrigger, escapeHtml, formatShortAddress } = options;
  return `
    <div class="overview-sheet">
      ${buildHeadlineMetrics(token)}
      ${buildOverviewContent(token, {
        renderHintTrigger,
        escapeHtml,
        formatShortAddress,
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
      aria-label="${esc(I18n.t("tokens-overview-banner-open"))}"
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
    <div class="overview-headline" aria-label="${esc(I18n.t("tokens-overview-headline-label"))}">
      <div class="overview-headline-item">
        <span class="overview-headline-label">${esc(I18n.t("tokens-overview-price"))}</span>
        <span class="overview-headline-readout">
          <span class="overview-headline-value">${token.price_sol ? withSolUnit(Utils.formatPriceSubscript(token.price_sol, { precision: 5 })) : "—"}</span>
          ${hasChange24h ? `<span class="overview-headline-change ${changeClass}">${formatChange(change24h)}</span>` : ""}
        </span>
      </div>
      <div class="overview-headline-item">
        <span class="overview-headline-label">${esc(I18n.t("tokens-overview-market-cap"))}</span>
        <span class="overview-headline-value">${token.market_cap ? Utils.formatCompactNumber(token.market_cap, { usd: true }) : token.fdv ? Utils.formatCompactNumber(token.fdv, { usd: true }) : "—"}</span>
      </div>
      <div class="overview-headline-item">
        <span class="overview-headline-label">${esc(I18n.t("tokens-overview-liquidity"))}</span>
        <span class="overview-headline-value">${token.liquidity_usd ? Utils.formatCompactNumber(token.liquidity_usd, { usd: true }) : token.pool_reserves_sol ? Utils.formatSol(token.pool_reserves_sol, { decimals: 2 }) : "—"}</span>
      </div>
      <div class="overview-headline-item">
        <span class="overview-headline-label">${esc(I18n.t("tokens-overview-volume-24h"))}</span>
        <span class="overview-headline-value">${token.volume_24h ? Utils.formatCompactNumber(token.volume_24h, { usd: true }) : "—"}</span>
      </div>
    </div>
  `;
}

function buildOverviewContent(token, options) {
  const { renderHintTrigger, escapeHtml, formatShortAddress } = options;

  return `
    <div class="overview-sections">
      ${buildTokenInfoSection(token, { renderHintTrigger, escapeHtml, formatShortAddress })}
      ${buildLiquiditySection(token, { renderHintTrigger, formatShortAddress })}
      ${buildMarketPulseSection(token, { renderHintTrigger })}
      ${buildActivitySection(token, { renderHintTrigger })}
    </div>
  `;
}

function buildTokenInfoSection(token, options) {
  const { renderHintTrigger, escapeHtml, formatShortAddress } = options;

  const age = token.pair_created_at
    ? Utils.formatTimeAgo(new Date(token.pair_created_at * 1000))
    : token.created_at
      ? Utils.formatTimeAgo(new Date(token.created_at * 1000))
      : "—";

  const tagsContent =
    token.tags && token.tags.length > 0
      ? `<div class="overview-tags">${token.tags.map((tag) => `<span class="overview-tag">${escapeHtml(tag)}</span>`).join("")}</div>`
      : `<span class="overview-tags-empty">${esc(I18n.t("tokens-overview-no-tags"))}</span>`;

  return `
    <section class="overview-section">
      <div class="overview-section-header">
        <span class="overview-section-title">${esc(I18n.t("tokens-overview-info-title"))}</span>
        <div class="overview-section-actions">
          ${token.profile ? `<span class="overview-verified"><i class="icon-badge-check"></i> ${esc(I18n.t("tokens-overview-profile"))}</span>` : ""}
          ${token.verified ? `<span class="overview-verified"><i class="icon-shield-check"></i> ${esc(I18n.t("positions-risk-low"))}</span>` : ""}
          ${renderHintTrigger("tokenDetails.tokenInfo")}
        </div>
      </div>
      <div class="overview-facts">
          <div class="overview-fact">
            <span class="overview-fact-label">${esc(I18n.t("tokens-overview-fact-mint"))}</span>
            <button
              type="button"
              class="overview-copy-value"
              data-copy="${escapeHtml(token.mint)}"
              dir="ltr"
              title="${esc(I18n.attr("links-copy-mint", "title"))}"
              aria-label="${esc(I18n.attr("links-copy-mint", "aria-label"))}"
            >${formatShortAddress(token.mint)}</button>
          </div>
          <div class="overview-fact">
            <span class="overview-fact-label">${esc(I18n.t("tokens-overview-fact-decimals"))}</span>
            <span class="overview-fact-value">${token.decimals ?? "—"}</span>
          </div>
          <div class="overview-fact">
            <span class="overview-fact-label">${esc(I18n.t("tokens-overview-fact-age"))}</span>
            <span class="overview-fact-value">${age}</span>
          </div>
          <div class="overview-fact">
            <span class="overview-fact-label">${esc(I18n.t("tokens-overview-fact-dex"))}</span>
            <span class="overview-fact-value">${token.pool_program ? escapeHtml(venueLabel(token.pool_program)) : "—"}</span>
          </div>
          ${
            token.total_holders
              ? `
          <div class="overview-fact">
            <span class="overview-fact-label">${esc(I18n.t("tokens-overview-fact-holders"))}</span>
            <span class="overview-fact-value">${Utils.formatNumber(token.total_holders, { decimals: 0 })}</span>
          </div>
          `
              : ""
          }
          ${
            token.top_10_concentration
              ? `
          <div class="overview-fact">
            <span class="overview-fact-label">${esc(I18n.t("tokens-overview-fact-top-10"))}</span>
            <span class="overview-fact-value">${formatPercentValue(token.top_10_concentration, { decimals: 1, plus: "" })}</span>
          </div>
          `
              : ""
          }
      </div>
      <div class="overview-tags-row">
        <span class="overview-inline-label">${esc(I18n.t("tokens-overview-tags"))}</span>
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
        <span class="overview-section-title">${esc(I18n.t("tokens-overview-liquidity-title"))}</span>
        ${renderHintTrigger("tokenDetails.liquidity")}
      </div>
      <div class="overview-facts overview-market-facts">
          <div class="overview-fact overview-fact-emphasis">
            <span class="overview-fact-label">${esc(I18n.t("tokens-overview-fact-fdv"))}</span>
            <span class="overview-fact-value">${token.fdv ? Utils.formatCurrencyUSD(token.fdv) : "—"}</span>
          </div>
          <div class="overview-fact overview-fact-emphasis">
            <span class="overview-fact-label">${esc(I18n.t("tokens-overview-liquidity"))}</span>
            <span class="overview-fact-value">${token.liquidity_usd ? Utils.formatCurrencyUSD(token.liquidity_usd) : "—"}</span>
          </div>
          <div class="overview-fact">
            <span class="overview-fact-label">${esc(I18n.t("tokens-overview-fact-pool-sol"))}</span>
            <span class="overview-fact-value">${token.pool_reserves_sol ? withSolUnit(Utils.formatNumber(token.pool_reserves_sol, { decimals: 2 })) : "—"}</span>
          </div>
          <div class="overview-fact">
            <span class="overview-fact-label">${esc(I18n.t("tokens-overview-fact-pool-token"))}</span>
            <span class="overview-fact-value">${token.pool_reserves_token ? Utils.formatCompactNumber(token.pool_reserves_token) : "—"}</span>
          </div>
      </div>
        ${
          token.pool_address
            ? `
        <div class="overview-pool-row">
          <span class="overview-inline-label">${esc(I18n.t("tokens-overview-pool"))}</span>
          <a href="https://solscan.io/account/${token.pool_address}" target="_blank" rel="noopener" class="overview-pool-link" dir="ltr">${formatShortAddress(token.pool_address)}</a>
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
        <span class="overview-section-title">${esc(I18n.t("tokens-overview-pulse-title"))}</span>
        ${renderHintTrigger("tokenDetails.marketPulse")}
      </div>
      <div class="overview-pulse-matrix">
        <span aria-hidden="true"></span>
        <span class="overview-pulse-time">5M</span>
        <span class="overview-pulse-time">1H</span>
        <span class="overview-pulse-time">6H</span>
        <span class="overview-pulse-time">24H</span>
        <span class="overview-pulse-label">${esc(I18n.t("tokens-overview-price"))}</span>
        ${buildPulseChange(changes.m5)}
        ${buildPulseChange(changes.h1)}
        ${buildPulseChange(changes.h6)}
        ${buildPulseChange(changes.h24)}
        <span class="overview-pulse-label">${esc(I18n.t("tokens-overview-volume"))}</span>
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
  return `<span class="overview-pulse-volume">${value ? Utils.formatCompactNumber(value, { usd: true }) : "—"}</span>`;
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
      ? Utils.formatSignedNumber(netFlow24h, { decimals: 0, fallback: "—" })
      : "—";
  const netFlowClass = typeof netFlow24h === "number" ? Utils.signedTone(netFlow24h, 0) : "";

  return `
    <section class="overview-section">
      <div class="overview-section-header">
        <span class="overview-section-title">${esc(I18n.t("tokens-overview-activity-title"))}</span>
        <div class="overview-section-actions">
          ${
            typeof buyPct24 === "number"
              ? `<span class="overview-ratio ${buyPct24 >= 50 ? "bullish" : "bearish"}">${esc(I18n.t("tokens-overview-buy-share", { percent: wholePercent(buyPct24) }))}</span>`
              : ""
          }
          ${buySellRatio ? `<span class="overview-ratio ${ratioClass}">${esc(I18n.t("tokens-overview-buy-sell-ratio", { ratio: formatFixed(buySellRatio, { decimals: 2 }) }))}</span>` : ""}
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
          <span class="overview-flow-stat-label">${esc(I18n.t("tokens-overview-buys-24h"))}</span>
          <span class="overview-flow-stat-value">${typeof buys24 === "number" ? Utils.formatNumber(buys24, { decimals: 0 }) : "—"}</span>
        </div>
        <div class="overview-flow-stat sells">
          <span class="overview-flow-stat-label">${esc(I18n.t("tokens-overview-sells-24h"))}</span>
          <span class="overview-flow-stat-value">${typeof sells24 === "number" ? Utils.formatNumber(sells24, { decimals: 0 }) : "—"}</span>
        </div>
        <div class="overview-flow-stat ${netFlowClass}">
          <span class="overview-flow-stat-label">${esc(I18n.t("tokens-overview-net-flow"))}</span>
          <span class="overview-flow-stat-value">${netFlowLabel}</span>
        </div>
        <div class="overview-flow-stat">
          <span class="overview-flow-stat-label">${esc(I18n.t("tokens-overview-total-24h"))}</span>
          <span class="overview-flow-stat-value">${total24 > 0 ? Utils.formatNumber(total24, { decimals: 0 }) : "—"}</span>
        </div>
        <div class="overview-flow-stat">
          <span class="overview-flow-stat-label">${esc(I18n.t("tokens-overview-average-24h"))}</span>
          <span class="overview-flow-stat-value">${
            total24 > 0
              ? esc(I18n.t("tokens-overview-rate-per-hour", { amount: Utils.formatNumber(total24 / 24, { decimals: 1 }) }))
              : "—"
          }</span>
        </div>
        <div class="overview-flow-stat">
          <span class="overview-flow-stat-label">${esc(I18n.t("tokens-overview-spike-5m"))}</span>
          <span class="overview-flow-stat-value">${
            typeof spikeFactor === "number" && Number.isFinite(spikeFactor)
              ? esc(I18n.t("tokens-overview-spike-factor", { factor: Utils.formatNumber(spikeFactor, { decimals: 2 }) }))
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

  const buyText = typeof buys === "number" ? Utils.formatNumber(buys, { decimals: 0 }) : "—";
  const sellText = typeof sells === "number" ? Utils.formatNumber(sells, { decimals: 0 }) : "—";
  const countsTitle = hasAny
    ? I18n.t("tokens-overview-flow-counts", {
        buys: buyText,
        buyPercent: total > 0 ? wholePercent(buyPct) : "—",
        sells: sellText,
        sellPercent: total > 0 ? wholePercent(sellPct) : "—",
        total: Utils.formatNumber(total, { decimals: 0 }),
      })
    : I18n.t("tokens-overview-flow-no-data");

  const ratePerMin = minutes && total >= 0 ? total / minutes : null;
  const rateText = hasAny
    ? I18n.t("tokens-overview-rate-per-minute", {
        amount: Utils.formatNumber(ratePerMin ?? 0, { decimals: 1 }),
      })
    : "—";
  const pctText =
    total > 0
      ? `${wholePercent(buyPct)} / ${wholePercent(sellPct)}`
      : hasAny
        ? `${wholePercent(0)} / ${wholePercent(0)}`
        : "—";

  return `
    <div class="overview-flow-row" title="${esc(countsTitle)}">
      <div class="overview-flow-time">
        <span class="overview-flow-period">${label}</span>
        <span class="overview-flow-rate">${rateText}</span>
      </div>
      <div class="overview-flow-bar ${hasAny ? "" : "is-empty"}" aria-label="${esc(countsTitle)}">
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
  return formatPercentValue(change, { decimals: 2, signZero: true });
}

function getChangeClass(change) {
  if (change === undefined || change === null) return "";
  return change >= 0 ? "positive" : "negative";
}
