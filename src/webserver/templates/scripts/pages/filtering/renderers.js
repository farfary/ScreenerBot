// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Filtering Page Renderers
 *
 * All rendering functions for the filtering configuration page.
 * Uses factory pattern to get access to state and dependencies.
 */

import { impactLabel } from "../config/field_text.js";
import { renderTokenLogo } from "../../ui/token_identity.js";
import {
  buildConfigGroups,
  formatTimestampForInput,
  getTimeRangeLabel,
  getStatusMessage,
  getConfigValue,
  getCategoryEnabled,
  getSourceEnabled,
  getSourceMasterField,
  getFieldDefault,
  REJECTION_SOURCE_LABELS,
  SETTINGS_TABS,
  SOURCE_LABELS,
  TIME_RANGE_LABELS,
  TIME_RANGE_PRESETS,
} from "./config_metadata.js";

let reasonClickBound = false;

/**
 * Create filtering renderers with access to state and dependencies
 */
export function createFilteringRenderers({ state, $: _$, Utils, requestManager: _requestManager }) {
  // Catalog text of a rejection entry; a payload without it shows the stored code.
  function reasonLabel(entry) {
    return entry.reason_text ? I18n.text(entry.reason_text) : entry.reason;
  }

  // Reason rows carry the stored code in `data-reason`; one delegated listener selects it.
  // The factory runs on every page init, so the listener is bound once per document.
  if (!reasonClickBound) {
    reasonClickBound = true;
    document.addEventListener("click", (event) => {
      const row = event.target instanceof Element ? event.target.closest("[data-reason]") : null;
      if (row?.dataset.reason) window.filteringPage.selectReason(row.dataset.reason);
    });
  }

  // A share of a snapshot-derived count, or `null` when the snapshot has not produced one
  // yet. Every count on this page is absent until the first snapshot exists (the API sends
  // null with `snapshot_state: "building"`), and `0 / null` would quietly become a
  // confident "0.0%". Null keeps the formatters on "—".
  function percentOf(part, total) {
    if (part === null || part === undefined || total === null || total === undefined) {
      return null;
    }
    const partNum = Number(part);
    const totalNum = Number(total);
    if (!Number.isFinite(partNum) || !Number.isFinite(totalNum) || totalNum <= 0) {
      return null;
    }
    return (partNum / totalNum) * 100;
  }

  // What to show where a timestamp belongs while the snapshot behind it is still building.
  // `new Date(null)` is the epoch and `new Date(undefined)` is Invalid Date, so neither can
  // be handed to a time formatter — the state has to be read, not inferred.
  function refreshedLabel(updatedAt, snapshotState = state.stats?.snapshot_state) {
    if (updatedAt) return Utils.formatTimeAgo(new Date(updatedAt));
    return snapshotState === "building"
      ? I18n.t("filtering-refresh-building")
      : I18n.t("filtering-refresh-never");
  }

  // A count followed by its share of the total, e.g. "120 (4.0%)".
  function countWithShare(count, share) {
    return I18n.t("filtering-count-share", {
      count: Utils.formatNumber(count, 0),
      share: Utils.formatPercentValue(share, { includeSign: false, decimals: 1 }),
    });
  }

  // The token count line of a chart bar, e.g. "120 tokens".
  function tokensCount(count) {
    return I18n.t("filtering-tokens-count", { count, amount: Utils.formatNumber(count, 0) });
  }

  function renderInfoBar() {
    if (!state.stats) return "";

    const {
      total_tokens,
      with_pool_price,
      passed_filtering,
      open_positions,
      blacklisted,
      updated_at,
    } = state.stats;

    const priceRate = percentOf(with_pool_price, total_tokens);
    const passedRate = percentOf(passed_filtering, total_tokens);
    const cacheAge = refreshedLabel(updated_at);

    return `
      <div class="info-item highlight">
        <span class="label">${Utils.escapeHtml(I18n.t("filtering-info-total"))}</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(total_tokens, 0))}</span>
      </div>
      <div class="info-item">
        <span class="label">${Utils.escapeHtml(I18n.t("filtering-info-priced"))}</span>
        <span class="value">${Utils.escapeHtml(countWithShare(with_pool_price, priceRate))}</span>
      </div>
      <div class="info-item highlight">
        <span class="label">${Utils.escapeHtml(I18n.t("filtering-info-passed"))}</span>
        <span class="value">${Utils.escapeHtml(countWithShare(passed_filtering, passedRate))}</span>
      </div>
      <div class="info-item">
        <span class="label">${Utils.escapeHtml(I18n.t("filtering-info-positions"))}</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(open_positions, 0))}</span>
      </div>
      <div class="info-item warning">
        <span class="label">${Utils.escapeHtml(I18n.t("filtering-info-blacklisted"))}</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(blacklisted, 0))}</span>
      </div>
      <div class="info-item">
        <span class="label">${Utils.escapeHtml(I18n.t("filtering-info-cache"))}</span>
        <span class="value">${Utils.escapeHtml(cacheAge)}</span>
      </div>
  `;
  }

  function renderStatusView() {
    if (!state.stats) {
      return `<div class="filtering-config-empty">${Utils.escapeHtml(I18n.t("filtering-status-loading"))}</div>`;
    }

    const {
      total_tokens,
      with_pool_price,
      open_positions,
      blacklisted,
      with_ohlcv,
      passed_filtering,
      updated_at,
    } = state.stats;

    const priceRate = percentOf(with_pool_price, total_tokens);
    const passedRate = percentOf(passed_filtering, total_tokens);
    const building = state.stats.snapshot_state === "building";

    // While the snapshot builds, every metric card below shows "—" (its value is null) and
    // this line says why, instead of the cards asserting a corpus of zero tokens.
    const cacheDetail = building
      ? I18n.t("filtering-status-total-detail-building")
      : I18n.t("filtering-status-total-detail");
    const refreshDetail = updated_at
      ? Utils.formatTimestamp(updated_at)
      : building
        ? I18n.t("filtering-status-refresh-building")
        : I18n.t("filtering-status-refresh-none");
    const shareText = (rate) => Utils.formatPercentValue(rate, { includeSign: false, decimals: 1 });

    const metricsHtml = `
      <div class="status-view">
        <div class="metric-card" data-accent="primary">
          <span class="metric-label">${Utils.escapeHtml(I18n.t("filtering-status-total"))}</span>
          <span class="metric-value">${Utils.formatNumber(total_tokens, 0)}</span>
          <span class="metric-detail">${Utils.escapeHtml(cacheDetail)}</span>
        </div>
        <div class="metric-card">
          <span class="metric-label">${Utils.escapeHtml(I18n.t("filtering-status-priced"))}</span>
          <span class="metric-value">${Utils.formatNumber(with_pool_price, 0)}</span>
          <span class="metric-detail">${Utils.escapeHtml(I18n.t("filtering-status-priced-detail", { share: shareText(priceRate) }))}</span>
        </div>
        <div class="metric-card" data-accent="primary">
          <span class="metric-label">${Utils.escapeHtml(I18n.t("filtering-status-passed"))}</span>
          <span class="metric-value">${Utils.formatNumber(passed_filtering, 0)}</span>
          <span class="metric-detail">${Utils.escapeHtml(I18n.t("filtering-status-passed-detail", { share: shareText(passedRate) }))}</span>
        </div>
        <div class="metric-card">
          <span class="metric-label">${Utils.escapeHtml(I18n.t("filtering-status-positions"))}</span>
          <span class="metric-value">${Utils.formatNumber(open_positions, 0)}</span>
          <span class="metric-detail">${Utils.escapeHtml(I18n.t("filtering-status-positions-detail"))}</span>
        </div>
        <div class="metric-card" data-accent="warning">
          <span class="metric-label">${Utils.escapeHtml(I18n.t("filtering-status-blacklisted"))}</span>
          <span class="metric-value">${Utils.formatNumber(blacklisted, 0)}</span>
          <span class="metric-detail">${Utils.escapeHtml(I18n.t("filtering-status-blacklisted-detail"))}</span>
        </div>
        <div class="metric-card">
          <span class="metric-label">${Utils.escapeHtml(I18n.t("filtering-status-ohlcv"))}</span>
          <span class="metric-value">${Utils.formatNumber(with_ohlcv, 0)}</span>
          <span class="metric-detail">${Utils.escapeHtml(I18n.t("filtering-status-ohlcv-detail"))}</span>
        </div>
        <div class="metric-card">
          <span class="metric-label">${Utils.escapeHtml(I18n.t("filtering-status-refresh"))}</span>
          <span class="metric-value">${Utils.escapeHtml(refreshedLabel(updated_at))}</span>
          <span class="metric-detail">${Utils.escapeHtml(refreshDetail)}</span>
        </div>
      </div>
    `;

    let rejectionHtml = `<div class="status-rejection-empty">${Utils.escapeHtml(I18n.t("filtering-status-no-rejections"))}</div>`;

    if (state.rejectionStats?.stats?.length > 0) {
      const bySource = state.rejectionStats.by_source || {};
      const topReasons = state.rejectionStats.stats.slice(0, 20);
      const maxCount = topReasons[0]?.count || 1;

      const sourcePills = Object.entries(bySource)
        .sort((a, b) => b[1] - a[1])
        .map(
          ([src, cnt]) => `
          <div class="rej-source-pill ${Utils.escapeHtml(src)}">
            <span class="rej-source-name">${Utils.escapeHtml(I18n.label(REJECTION_SOURCE_LABELS, src))}</span>
            <span class="rej-source-count">${Utils.formatNumber(cnt, 0)}</span>
          </div>`
        )
        .join("");

      const rejItems = topReasons
        .map(({ reason, reason_text, source, count }) => {
          const barWidth = Math.min((count / maxCount) * 100, 100).toFixed(1);
          return `
          <div class="rejection-item">
            <div class="rej-bar" style="width: ${barWidth}%"></div>
            <span class="rej-label">${Utils.escapeHtml(reasonLabel({ reason, reason_text }))}</span>
            <span class="rej-source-tag ${Utils.escapeHtml(source)}">${Utils.escapeHtml(I18n.label(REJECTION_SOURCE_LABELS, source))}</span>
            <span class="rej-count">${Utils.formatNumber(count, 0)}</span>
          </div>`;
        })
        .join("");

      rejectionHtml = `
        <div class="rej-source-row">${sourcePills}</div>
        <div class="rejection-list">${rejItems}</div>
      `;
    }

    return `
    <div class="filtering-status-layout">
      <div class="status-metrics-section">${metricsHtml}</div>
      <div class="status-rejection-section">${rejectionHtml}</div>
    </div>
  `;
  }

  // ============================================================================
  // ANALYTICS VIEW - Advanced filtering analysis
  // ============================================================================

  function renderAnalyticsView() {
    // Show loading state when switching time ranges or initially loading
    if (state.isLoadingAnalytics || !state.analytics) {
      return `<div class="loading-spinner">${Utils.escapeHtml(I18n.t("filtering-analytics-loading", { range: getTimeRangeLabel(state.timeRange) }))}</div>`;
    }

    const data = state.analytics;

    // Time range filter (no title header — data only)
    const headerHtml = `
    <div class="time-range-filter">
      <div class="time-range-presets">
        ${Object.keys(TIME_RANGE_PRESETS)
          .map(
            (preset) =>
              `<button class="time-preset-btn ${state.timeRange.preset === preset ? "active" : ""}" onclick="window.filteringPage.setTimeRangePreset('${preset}')">${Utils.escapeHtml(I18n.label(TIME_RANGE_LABELS, preset))}</button>`
          )
          .join("")}
      </div>
      <div class="time-range-custom">
        <div class="custom-range-toggle ${state.timeRange.preset === "custom" ? "active" : ""}" onclick="window.filteringPage.toggleCustomRange()">
          <i class="icon-calendar"></i> ${Utils.escapeHtml(I18n.t("filtering-range-custom"))}
        </div>
        <div class="custom-range-inputs ${state.timeRange.preset === "custom" ? "show" : ""}">
          <input type="datetime-local" id="time-range-start" class="time-input" 
            value="${formatTimestampForInput(state.timeRange.startTime)}"
            onchange="window.filteringPage.updateCustomRange()">
          <span class="time-separator">→</span>
          <input type="datetime-local" id="time-range-end" class="time-input" 
            value="${formatTimestampForInput(state.timeRange.endTime)}"
            onchange="window.filteringPage.updateCustomRange()">
          <button class="btn btn-sm btn-primary" onclick="window.filteringPage.applyCustomRange()">${Utils.escapeHtml(I18n.t("common-action-apply"))}</button>
        </div>
      </div>
    </div>
  `;

    // KPI Cards
    const kpiHtml = `
    <div class="kpi-grid">
      <!-- Total Tokens -->
      <div class="kpi-card">
        <div class="kpi-content">
          <span class="kpi-label">${Utils.escapeHtml(I18n.t("filtering-analytics-scanned"))}</span>
          <span class="kpi-value">${Utils.formatNumber(data.total_tokens, 0)}</span>
          <span class="kpi-subtext">
            <i class="icon-clock"></i> ${Utils.escapeHtml(I18n.t("filtering-analytics-updated", { time: refreshedLabel(data.last_updated, data.snapshot_state) }))}
          </span>
        </div>
        <i class="icon-database kpi-icon"></i>
      </div>

      <!-- Passed -->
      <div class="kpi-card">
        <div class="kpi-content">
          <span class="kpi-label">${Utils.escapeHtml(I18n.t("filtering-analytics-passed"))}</span>
          <span class="kpi-value text-success">${Utils.formatNumber(data.total_passed, 0)}</span>
          <span class="kpi-subtext">
            <span class="kpi-rate kpi-rate--success">${I18n.markup("filtering-analytics-pass-rate", { share: Utils.formatPercentValue(data.pass_rate, { includeSign: false }) })}</span>
          </span>
        </div>
        <i class="icon-circle-check kpi-icon text-success" style="opacity: 0.2"></i>
        <div class="pass-rate-visual">
          <div class="pass-rate-segment passed" style="width: ${Number.isFinite(data.pass_rate) ? data.pass_rate : 0}%"></div>
        </div>
      </div>

      <!-- Rejected -->
      <div class="kpi-card">
        <div class="kpi-content">
          <span class="kpi-label">${Utils.escapeHtml(I18n.t("filtering-analytics-rejected"))}</span>
          <span class="kpi-value text-error">${Utils.formatNumber(data.total_rejected, 0)}</span>
          <span class="kpi-subtext">
            <span class="kpi-rate kpi-rate--error">${I18n.markup("filtering-analytics-rejection-rate", { share: Utils.formatPercentValue(data.rejection_rate, { includeSign: false }) })}</span>
          </span>
        </div>
        <i class="icon-circle-x kpi-icon text-error" style="opacity: 0.2"></i>
      </div>
    </div>
  `;

    // Charts Section
    const chartsHtml = `
    <div class="charts-grid">
      <!-- Rejection by Category -->
      <div class="chart-card">
        <div class="chart-header">
          <div class="chart-title"><i class="icon-layers"></i> ${Utils.escapeHtml(I18n.t("filtering-analytics-by-category"))}</div>
        </div>
        <div class="chart-body">
          ${
            data.by_category && data.by_category.length > 0
              ? data.by_category
                  .map(
                    (cat) => `
            <div class="bar-chart-row">
              <div class="bar-label-col">
                <div class="bar-icon"><i class="icon-${Utils.escapeHtml(cat.icon)}"></i></div>
                <div class="bar-label" title="${Utils.escapeHtml(I18n.text(cat.category_text))}">${Utils.escapeHtml(I18n.text(cat.category_text))}</div>
              </div>
              <div class="bar-track-col">
                <div class="bar-track">
                  <div class="bar-fill" style="width: ${Math.min(cat.percentage, 100)}%; background-color: var(--error-color)"></div>
                </div>
                <div class="bar-meta">
                  <span>${Utils.escapeHtml(tokensCount(cat.count))}</span>
                  <span>${Utils.formatPercentValue(cat.percentage, { includeSign: false })}</span>
                </div>
              </div>
            </div>
          `
                  )
                  .join("")
              : `<div class="analytics-empty">${Utils.escapeHtml(I18n.t("filtering-analytics-no-category"))}</div>`
          }
        </div>
      </div>

      <!-- Rejection by Source -->
      <div class="chart-card">
        <div class="chart-header">
          <div class="chart-title"><i class="icon-git-branch"></i> ${Utils.escapeHtml(I18n.t("filtering-analytics-by-source"))}</div>
        </div>
        <div class="chart-body">
          ${
            data.by_source && data.by_source.length > 0
              ? data.by_source
                  .map(
                    (src) => `
            <div class="bar-chart-row">
              <div class="bar-label-col">
                <div class="bar-label font-bold w-auto">${Utils.escapeHtml(I18n.label(REJECTION_SOURCE_LABELS, src.source))}</div>
              </div>
              <div class="bar-track-col">
                <div class="bar-track">
                  <div class="bar-fill" style="width: ${Math.min(src.percentage, 100)}%; background-color: var(--warning-color)"></div>
                </div>
                <div class="bar-meta">
                  <span>${Utils.escapeHtml(tokensCount(src.count))}</span>
                  <span>${Utils.formatPercentValue(src.percentage, { includeSign: false })}</span>
                </div>
              </div>
            </div>
          `
                  )
                  .join("")
              : `<div class="analytics-empty">${Utils.escapeHtml(I18n.t("filtering-analytics-no-source"))}</div>`
          }
        </div>
      </div>
    </div>
  `;

    // Bottom Section: Top Reasons & Data Quality
    const bottomHtml = `
    <div class="charts-grid">
      <!-- Top Rejection Reasons -->
      <div class="chart-card span-2">
        <div class="chart-header">
          <div class="chart-title"><i class="icon-list"></i> ${Utils.escapeHtml(I18n.t("filtering-analytics-top-reasons"))}</div>
        </div>
        <div class="reasons-table-container">
          <table class="reasons-table">
            <thead>
              <tr>
                <th>${Utils.escapeHtml(I18n.t("filtering-analytics-column-reason"))}</th>
                <th>${Utils.escapeHtml(I18n.t("filtering-analytics-column-category"))}</th>
                <th class="text-end">${Utils.escapeHtml(I18n.t("filtering-analytics-column-count"))}</th>
                <th class="text-end">${Utils.escapeHtml(I18n.t("filtering-analytics-column-share"))}</th>
                <th class="text-end">${Utils.escapeHtml(I18n.t("filtering-analytics-column-impact"))}</th>
              </tr>
            </thead>
            <tbody>
              ${
                data.top_reasons && data.top_reasons.length > 0
                  ? data.top_reasons
                      .slice(0, 10)
                      .map((r) => {
                        const maxCount = data.top_reasons[0].count;
                        const relativePercent = (r.count / maxCount) * 100;
                        return `
                  <tr>
                    <td>
                      <span class="font-medium">${Utils.escapeHtml(reasonLabel(r))}</span>
                    </td>
                    <td>
                      <span class="reason-badge">${Utils.escapeHtml(I18n.text(r.category_text))}</span>
                    </td>
                    <td class="text-end font-data">
                      ${Utils.formatNumber(r.count, 0)}
                    </td>
                    <td class="text-end font-data text-secondary">
                      ${Utils.formatPercentValue(r.percentage, { includeSign: false })}
                    </td>
                    <td class="reason-bar-cell">
                      <div class="mini-bar">
                        <div class="mini-bar-fill" style="width: ${relativePercent}%"></div>
                      </div>
                    </td>
                  </tr>
                `;
                      })
                      .join("")
                  : `<tr><td colspan="5" class="text-center p-20">${Utils.escapeHtml(I18n.t("filtering-analytics-no-data"))}</td></tr>`
              }
            </tbody>
          </table>
        </div>
      </div>
    </div>
  `;

    return `
    <div class="analytics-scroll-area">
      <div class="analytics-view">
        ${headerHtml}
        ${kpiHtml}
        ${chartsHtml}
        ${bottomHtml}
      </div>
    </div>
  `;
  }

  // ============================================================================
  // EXPLORER VIEW - Tree-based rejection explorer
  // ============================================================================

  function renderExplorerDashboard(data) {
    const topReasons = data.top_reasons || [];
    const recentRejections = data.recent_rejections || [];

    return `
    <div class="explorer-overview">
      <div class="explorer-overview-col">
        <div class="overview-col-header">
          <span class="overview-col-title">${Utils.escapeHtml(I18n.t("filtering-explorer-top-reasons"))}</span>
          <span class="overview-col-count">${topReasons.length}</span>
        </div>
        <div class="overview-col-list">
          ${topReasons
            .slice(0, 30)
            .map(
              (r) => `
            <div class="overview-list-item" data-reason="${Utils.escapeHtml(r.reason)}">
              <span class="overview-item-label">${Utils.escapeHtml(reasonLabel(r))}</span>
              <span class="overview-item-count">${Utils.formatNumber(r.count, 0)}</span>
            </div>
          `
            )
            .join("")}
          ${topReasons.length === 0 ? `<div class="analytics-empty-compact">${Utils.escapeHtml(I18n.t("filtering-explorer-none"))}</div>` : ""}
        </div>
      </div>
      <div class="explorer-overview-col explorer-overview-col--sep">
        <div class="overview-col-header">
          <span class="overview-col-title">${Utils.escapeHtml(I18n.t("filtering-explorer-recent"))}</span>
          <span class="overview-col-count">${recentRejections.length}</span>
        </div>
        <div class="overview-col-list">
          ${recentRejections
            .slice(0, 30)
            .map((t) => {
              const sym = t.symbol || "?";
              return `
              <div class="overview-list-item overview-list-item--token" data-reason="${Utils.escapeHtml(t.reason)}">
                ${renderTokenLogo({ mint: t.mint, symbol: t.symbol, name: t.name, logoUrl: t.image_url }, { size: "sm" })}
                <div class="overview-item-info">
                  <span class="overview-item-symbol">${Utils.escapeHtml(sym)}${t.name ? ` <span class="overview-item-name">${Utils.escapeHtml(t.name)}</span>` : ""}</span>
                  <span class="overview-item-reason">${Utils.escapeHtml(reasonLabel(t))}</span>
                </div>
                <span class="overview-item-time">${Utils.formatTimeAgo(new Date(t.rejected_at))}</span>
              </div>
            `;
            })
            .join("")}
          ${recentRejections.length === 0 ? `<div class="analytics-empty-compact">${Utils.escapeHtml(I18n.t("filtering-explorer-none-recent"))}</div>` : ""}
        </div>
      </div>
    </div>
  `;
  }

  function renderExplorerView() {
    if (!state.analytics) {
      return `<div class="loading-spinner">${Utils.escapeHtml(I18n.t("common-loading"))}</div>`;
    }

    const data = state.analytics;

    const totalRejected = data.total_rejected || 0;
    const rejectionRate = data.rejection_rate
      ? Utils.formatPercentValue(data.rejection_rate, { decimals: 1, includeSign: false })
      : "";

    // Compact Tree View
    const treeHtml = `
    <div class="explorer-layout">
      <div class="explorer-sidebar">
        <div class="explorer-sidebar-search">
          <div class="explorer-search-input-wrapper">
            <i class="icon-search"></i>
            <input type="text" placeholder="${Utils.escapeHtml(I18n.attr("filtering-explorer-search", "placeholder"))}" oninput="window.filteringPage.filterExplorerTree(this.value)">
          </div>
        </div>

        <div class="explorer-nav-overview ${!window.filteringPage.currentReason ? "active" : ""}" onclick="window.filteringPage.selectSummary()">
          <i class="icon-chart-bar tree-icon"></i>
          <span class="tree-label">${Utils.escapeHtml(I18n.t("filtering-explorer-overview"))}</span>
          ${rejectionRate ? `<span class="explorer-nav-rate">${rejectionRate}</span>` : ""}
          <span class="tree-count">${Utils.formatCompactNumber(totalRejected)}</span>
        </div>

        <div class="explorer-tree" id="explorer-tree">
          ${data.by_category
            .map(
              (cat) => `
            <div class="tree-category" data-category="${cat.category}">
              <div class="tree-category-header" onclick="window.filteringPage.toggleCategory('${cat.category}')">
                <i class="icon-${Utils.escapeHtml(cat.icon)} tree-icon"></i>
                <span class="tree-label">${Utils.escapeHtml(I18n.text(cat.category_text))}</span>
                <span class="tree-count">${Utils.formatCompactNumber(cat.count)}</span>
                <i class="icon-chevron-down tree-toggle" id="toggle-${cat.category}"></i>
              </div>
              <div class="tree-reasons" id="reasons-${cat.category}" style="display: none">
                ${cat.reasons
                  .map(
                    (r) => `
                  <div class="tree-reason ${window.filteringPage.currentReason === r.reason ? "active" : ""}"
                       data-reason="${Utils.escapeHtml(r.reason)}"
                       id="reason-${r.reason}"
                       data-label="${Utils.escapeHtml(reasonLabel(r).toLowerCase())}">
                    <span class="tree-reason-label">${Utils.escapeHtml(reasonLabel(r))}</span>
                    <span class="tree-reason-count">${Utils.formatCompactNumber(r.count)}</span>
                  </div>
                `
                  )
                  .join("")}
              </div>
            </div>
          `
            )
            .join("")}
          <div class="tree-empty-state" id="explorer-tree-empty" style="display: none">${Utils.escapeHtml(I18n.t("filtering-explorer-no-match"))}</div>
        </div>
      </div>
      <div class="explorer-content">
        <div id="explorer-detail-view">
          ${window.filteringPage.currentReason ? "" : renderExplorerDashboard(data)}
        </div>
      </div>
    </div>
  `;

    // Trigger initial load if reason is selected
    if (window.filteringPage.currentReason) {
      setTimeout(() => window.filteringPage.loadExplorer(window.filteringPage.explorerPage), 0);
    }

    return `
    <div class="explorer-view-container">
      ${treeHtml}
    </div>
  `;
  }

  // A number input, sized and aligned by the shared settings-field system. The
  // metadata's constraints are only emitted when the schema declares them —
  // `min="undefined"` is not a constraint, it is an attribute the browser
  // silently ignores while making the markup invalid.
  function renderNumberInput(field, source, ariaLabel = "") {
    const value = getConfigValue(state.draft, source, field.key);
    const attrs = [
      `id="field-${source}-${field.key}"`,
      `data-field="${Utils.escapeHtml(field.key)}"`,
      `data-source="${Utils.escapeHtml(source)}"`,
      `value="${Utils.escapeHtml(value ?? "")}"`,
    ];
    if (Number.isFinite(field.min)) attrs.push(`min="${field.min}"`);
    if (Number.isFinite(field.max)) attrs.push(`max="${field.max}"`);
    if (Number.isFinite(field.step)) attrs.push(`step="${field.step}"`);
    if (ariaLabel) attrs.push(`aria-label="${Utils.escapeHtml(ariaLabel)}"`);
    if (field.hint) attrs.push(`title="${Utils.escapeHtml(field.hint)}"`);

    return `<input type="number" ${attrs.join(" ")} />`;
  }

  // A rule is on or off, so it gets the app's switch — the same control as the
  // group and source masters right above it, never a bare checkbox.
  function renderBooleanInput(field, source) {
    const value = getConfigValue(state.draft, source, field.key);
    return `
      <label class="toggle" data-level="item" aria-label="${Utils.escapeHtml(field.label)}">
        <input
          type="checkbox"
          id="field-${source}-${field.key}"
          data-field="${Utils.escapeHtml(field.key)}"
          data-source="${Utils.escapeHtml(source)}"
          ${value ? "checked" : ""}
        />
        <span class="toggle-track"></span>
      </label>`;
  }

  // "3 parameters"; `count` selects the plural.
  function parameterCount(count) {
    return I18n.t("filtering-parameter-count", { count, amount: Utils.formatNumber(count, 0) });
  }

  function groupsFor(source) {
    return buildConfigGroups(state.metadata).filter((group) => group.source === source);
  }

  function rowMatchesSearch(row, group) {
    const query = state.searchQuery;
    if (!query) return true;
    if (group.title.toLowerCase().includes(query)) return true;
    if (
      String(row.label || "")
        .toLowerCase()
        .includes(query)
    )
      return true;
    return row.fields.some(
      (field) =>
        field.key.toLowerCase().includes(query) ||
        String(field.label || "")
          .toLowerCase()
          .includes(query)
    );
  }

  /**
   * Whether the pipeline evaluates this group at all — its source master switch
   * or its own group switch is off.
   *
   * An inactive row is muted, never disabled: turning a source off to retune it
   * and back on is a normal workflow, and the previous card body blocked exactly
   * that with `pointer-events: none` while still letting the keyboard tab into
   * the fields it had just made unclickable.
   */
  function isRowInactive(group) {
    if (group.source !== "meta" && !getSourceEnabled(state.draft, group.source)) return true;
    return (
      Boolean(group.enableKey) && !getCategoryEnabled(state.draft, group.source, group.enableKey)
    );
  }

  function renderRow(row, group) {
    const changed = row.fields.some((field) => {
      if (!state.config) return false;
      return (
        getConfigValue(state.draft, group.source, field.key) !==
        getConfigValue(state.config, group.source, field.key)
      );
    });

    const inactive = isRowInactive(group);
    const classes = ["config-field"];
    if (changed) classes.push("config-field--changed");
    if (inactive) classes.push("config-field--inactive");

    const impact = row.impact
      ? `<span class="config-field-impact ${Utils.escapeHtml(row.impact)}">${Utils.escapeHtml(impactLabel(row.impact))}</span>`
      : "";
    const hint = row.hint
      ? `<div class="config-field-hint">${Utils.escapeHtml(row.hint)}</div>`
      : "";

    return `
      <div class="${classes.join(" ")}" data-row="${Utils.escapeHtml(row.key)}">
        <div class="config-field-label">
          <div class="config-field-title">
            <span class="config-field-name">${Utils.escapeHtml(row.label)}</span>
            ${impact}
          </div>
          ${hint}
        </div>
        <div class="config-field-control">${renderRowControl(row, group.source)}</div>
        ${renderRowReset(row, group.source)}
      </div>`;
  }

  /**
   * Reset this parameter to the schema default — the row anatomy's third track,
   * which the settings-field grid always reserves. A range row resets both of its
   * bounds at once, because they are two ends of one parameter.
   *
   * This is not the footer's Reset: that one reverts unsaved edits to the last
   * saved config, this one goes back to what ScreenerBot ships.
   */
  function renderRowReset(row, source) {
    const defaults = row.fields.map((field) => getFieldDefault(state.metadata, source, field.key));
    if (defaults.some((value) => value === undefined || value === null)) {
      return '<button type="button" class="config-field-reset" hidden></button>';
    }
    const defaultText =
      defaults.length === 2
        ? I18n.t("filtering-range-bounds", { min: defaults[0], max: defaults[1] })
        : defaults.join(", ");
    const resetArgs = { default: defaultText, label: row.label };

    const atDefault = row.fields.every(
      (field, index) => getConfigValue(state.draft, source, field.key) === defaults[index]
    );
    const keys = row.fields.map((field) => field.key).join(",");

    return `
      <button
        type="button"
        class="config-field-reset"
        data-reset-keys="${Utils.escapeHtml(keys)}"
        data-reset-source="${Utils.escapeHtml(source)}"
        title="${Utils.escapeHtml(I18n.attr("filtering-field-reset", "title", resetArgs))}"
        aria-label="${Utils.escapeHtml(I18n.attr("filtering-field-reset", "aria-label", resetArgs))}"
        ${atDefault ? "disabled" : ""}
      ><i class="icon-rotate-ccw" aria-hidden="true"></i></button>`;
  }

  function renderRowControl(row, source) {
    if (row.kind === "range") {
      const [min, max] = row.fields;
      const unit = row.unit
        ? `<span class="config-field-range-unit">${Utils.escapeHtml(row.unit)}</span>`
        : "";
      return `
        <div class="config-field-range">
          <span class="config-field-bound">
            <span class="config-field-bound-label">${Utils.escapeHtml(I18n.t("filtering-field-min"))}</span>
            ${renderNumberInput(min, source, I18n.attr("filtering-field-min-aria", "aria-label", { label: row.label }))}
          </span>
          <span class="config-field-range-sep" aria-hidden="true">–</span>
          <span class="config-field-bound">
            <span class="config-field-bound-label">${Utils.escapeHtml(I18n.t("filtering-field-max"))}</span>
            ${renderNumberInput(max, source, I18n.attr("filtering-field-max-aria", "aria-label", { label: row.label }))}
          </span>
          ${unit}
        </div>`;
    }

    const [field] = row.fields;
    if (field.type === "boolean") {
      return renderBooleanInput(field, source);
    }
    // The unit is written straight after the input; `ui/number_field.js` builds
    // the shell around both and decides whether the suffix belongs inside the
    // field's box or beside it.
    if (row.unit) {
      return `${renderNumberInput(field, source)}<span class="input-unit">${Utils.escapeHtml(row.unit)}</span>`;
    }
    return renderNumberInput(field, source);
  }

  function renderGroup(group) {
    const rows = group.rows.filter((row) => rowMatchesSearch(row, group));
    if (rows.length === 0) return "";

    const enableToggle = group.enableKey
      ? `
        <label class="toggle" data-level="group" ${group.enableHint ? `title="${Utils.escapeHtml(group.enableHint)}"` : ""} aria-label="${Utils.escapeHtml(I18n.attr("filtering-group-enable", "aria-label", { group: group.title }))}">
          <input
            type="checkbox"
            data-category-toggle="${Utils.escapeHtml(group.source)}"
            data-enable-key="${Utils.escapeHtml(group.enableKey)}"
            ${getCategoryEnabled(state.draft, group.source, group.enableKey) ? "checked" : ""}
          />
          <span class="toggle-track"></span>
        </label>`
      : "";

    return `
      <section class="filtering-group" data-group="${Utils.escapeHtml(group.id)}">
        <div class="filtering-group-header">
          <h3 class="filtering-group-title">${Utils.escapeHtml(group.title)}</h3>
          <span class="filtering-group-count">${Utils.escapeHtml(parameterCount(rows.length))}</span>
          ${enableToggle}
        </div>
        <div class="filtering-group-body">${rows.map((row) => renderRow(row, group)).join("")}</div>
      </section>`;
  }

  function renderConfigPanels() {
    if (!state.draft) {
      return `<div class="filtering-config-empty">${Utils.escapeHtml(I18n.t("filtering-config-loading"))}</div>`;
    }

    // Status tab shows overview
    if (state.activeTab === "status") {
      return renderStatusView();
    }

    // Analytics tab shows advanced analysis
    if (state.activeTab === "analytics") {
      return renderAnalyticsView();
    }

    // Explorer tab shows tree view
    if (state.activeTab === "explorer") {
      return renderExplorerView();
    }

    const source = state.activeTab || "meta";
    const groups = groupsFor(source).map(renderGroup).join("");

    if (!groups) {
      const reason = state.searchQuery
        ? I18n.t("filtering-config-no-match", { query: state.searchQuery })
        : I18n.t("filtering-config-no-parameters");
      return `<div class="filtering-config-empty">${Utils.escapeHtml(reason)}</div>`;
    }

    // One line of prose instead of a page of silently dimmed controls: a source
    // whose master switch is off evaluates none of these parameters.
    const sourceOff =
      source !== "meta" && !getSourceEnabled(state.draft, source)
        ? `<p class="filtering-source-off">${Utils.escapeHtml(I18n.t("filtering-source-off", { source: I18n.label(SOURCE_LABELS, source) }))}</p>`
        : "";

    return `<div class="config-scroll-area"><div class="filtering-config-list">${sourceOff}${groups}</div></div>`;
  }

  /**
   * The sub-tab's own control strip: filter-by-name on the left, how much of the
   * source is showing, and the source master switch at the far end.
   */
  function renderToolbar() {
    if (!SETTINGS_TABS.includes(state.activeTab) || !state.draft) {
      return "";
    }

    const master = getSourceMasterField(state.metadata, state.activeTab);
    const enabled = getSourceEnabled(state.draft, state.activeTab);
    const masterHtml = master
      ? `
        <label class="toggle filtering-source-switch" data-level="root" ${master.hint ? `title="${Utils.escapeHtml(master.hint)}"` : ""}>
          <input type="checkbox" data-source-toggle="${Utils.escapeHtml(state.activeTab)}" ${enabled ? "checked" : ""} />
          <span class="toggle-track"></span>
          <span class="toggle-state" id="filtering-source-state">${Utils.escapeHtml(enabled ? I18n.t("common-state-enabled") : I18n.t("common-state-disabled"))}</span>
        </label>`
      : "";

    return `
      <div class="search-wrapper filtering-toolbar-search">
        <input
          type="text"
          id="filtering-search"
          placeholder="${Utils.escapeHtml(I18n.attr("filtering-toolbar-filter", "placeholder"))}"
          value="${Utils.escapeHtml(state.searchQuery)}"
          autocomplete="off"
          spellcheck="false"
          aria-label="${Utils.escapeHtml(I18n.attr("filtering-toolbar-filter", "aria-label"))}"
        />
        <i class="icon-search search-icon" aria-hidden="true"></i>
        <button type="button" class="search-clear" id="filtering-search-clear" aria-label="${Utils.escapeHtml(I18n.attr("filtering-toolbar-clear", "aria-label"))}">
          <i class="icon-x" aria-hidden="true"></i>
        </button>
      </div>
      <span class="filtering-toolbar-count" id="filtering-param-count">${Utils.escapeHtml(renderParamCount())}</span>
      ${masterHtml}`;
  }

  function renderParamCount() {
    const groups = groupsFor(state.activeTab || "meta");
    let total = 0;
    let visible = 0;
    for (const group of groups) {
      for (const row of group.rows) {
        total += row.fields.length;
        if (rowMatchesSearch(row, group)) visible += row.fields.length;
      }
    }
    if (state.searchQuery) {
      return I18n.t("filtering-parameter-count-filtered", {
        count: total,
        visible: Utils.formatNumber(visible, 0),
        total: Utils.formatNumber(total, 0),
      });
    }
    return parameterCount(total);
  }

  function renderShell() {
    const statusMsg = getStatusMessage({
      isSaving: state.isSaving,
      isRefreshing: state.isRefreshing,
      hasChanges: state.hasChanges,
      lastSaved: state.lastSaved,
    });

    return `
    <div class="filtering-page">
      <div class="filtering-shell">
        <div class="filtering-info-bar" id="filtering-info-bar">${renderInfoBar()}</div>
        <div class="filtering-toolbar" id="filtering-toolbar">${renderToolbar()}</div>
        <div class="filtering-content" id="filtering-config-panels">
          ${renderConfigPanels()}
        </div>
        <footer class="filtering-footer">
          <div class="footer-left">
            <span id="filtering-status-message">${Utils.escapeHtml(statusMsg)}</span>
          </div>
          <div class="footer-actions">
            <button class="filtering-footer-btn ghost" id="reset-config-btn"><i class="icon-rotate-ccw"></i> ${Utils.escapeHtml(I18n.t("common-action-reset"))}</button>
            <button class="filtering-footer-btn ghost" id="refresh-snapshot-btn"><i class="icon-refresh-cw"></i> ${Utils.escapeHtml(I18n.t("common-action-refresh"))}</button>
            <button class="filtering-footer-btn ghost" id="export-config-btn"><i class="icon-download"></i> ${Utils.escapeHtml(I18n.t("common-action-export"))}</button>
            <button class="filtering-footer-btn ghost" id="import-config-btn"><i class="icon-upload"></i> ${Utils.escapeHtml(I18n.t("common-action-import"))}</button>
            <button class="filtering-footer-btn primary" id="save-config-btn"><i class="icon-save"></i> ${Utils.escapeHtml(I18n.t("common-action-save"))}</button>
          </div>
        </footer>
      </div>
    </div>
  `;
  }

  // Return all renderers
  return {
    renderInfoBar,
    renderStatusView,
    renderAnalyticsView,
    renderExplorerDashboard,
    renderExplorerView,
    renderConfigPanels,
    renderParamCount,
    renderToolbar,
    renderShell,
  };
}
