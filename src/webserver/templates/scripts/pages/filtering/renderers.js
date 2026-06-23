/**
 * Filtering Page Renderers
 *
 * All rendering functions for the filtering configuration page.
 * Uses factory pattern to get access to state and dependencies.
 */

import {
  CONFIG_CATEGORIES,
  formatTimestampForInput,
  getTimeRangeLabel,
  getStatusMessage,
  getConfigValue,
  getCategoryEnabled,
  getSourceEnabled,
} from "./config_metadata.js";

/**
 * Create filtering renderers with access to state and dependencies
 */
export function createFilteringRenderers({ state, $: _$, Utils, requestManager: _requestManager }) {
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

    const priceRate = total_tokens > 0 ? (with_pool_price / total_tokens) * 100 : 0;
    const passedRate = total_tokens > 0 ? (passed_filtering / total_tokens) * 100 : 0;
    const cacheAge = updated_at ? Utils.formatTimeAgo(new Date(updated_at)) : "Never";

    return `
      <div class="info-item highlight">
        <span class="label">Total:</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(total_tokens, 0))}</span>
      </div>
      <div class="info-item">
        <span class="label">Priced:</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(with_pool_price, 0))} (${Utils.escapeHtml(Utils.formatPercentValue(priceRate, { includeSign: false, decimals: 1 }))})</span>
      </div>
      <div class="info-item highlight">
        <span class="label">Passed:</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(passed_filtering, 0))} (${Utils.escapeHtml(Utils.formatPercentValue(passedRate, { includeSign: false, decimals: 1 }))})</span>
      </div>
      <div class="info-item">
        <span class="label">Positions:</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(open_positions, 0))}</span>
      </div>
      <div class="info-item warning">
        <span class="label">Blacklisted:</span>
        <span class="value">${Utils.escapeHtml(Utils.formatNumber(blacklisted, 0))}</span>
      </div>
      <div class="info-item">
        <span class="label">Cache:</span>
        <span class="value">${Utils.escapeHtml(cacheAge)}</span>
      </div>
  `;
  }

  function renderStatusView() {
    if (!state.stats) return '<div class="filtering-config-empty">Loading statistics...</div>';

    const {
      total_tokens,
      with_pool_price,
      open_positions,
      blacklisted,
      with_ohlcv,
      passed_filtering,
      updated_at,
    } = state.stats;

    const priceRate = total_tokens > 0 ? (with_pool_price / total_tokens) * 100 : 0;
    const passedRate = total_tokens > 0 ? (passed_filtering / total_tokens) * 100 : 0;

    const metricsHtml = `
      <div class="status-view">
        <div class="status-card dominant">
          <span class="metric-label">Total Tokens</span>
          <span class="metric-value">${Utils.formatNumber(total_tokens, 0)}</span>
          <span class="metric-meta">In filtering cache</span>
        </div>
        <div class="status-card">
          <span class="metric-label">With Price</span>
          <span class="metric-value">${Utils.formatNumber(with_pool_price, 0)}</span>
          <span class="metric-meta">${Utils.formatPercentValue(priceRate, { includeSign: false, decimals: 1 })} have pricing</span>
        </div>
        <div class="status-card dominant">
          <span class="metric-label">Passed Filters</span>
          <span class="metric-value">${Utils.formatNumber(passed_filtering, 0)}</span>
          <span class="metric-meta">${Utils.formatPercentValue(passedRate, { includeSign: false, decimals: 1 })} passed</span>
        </div>
        <div class="status-card">
          <span class="metric-label">Open Positions</span>
          <span class="metric-value">${Utils.formatNumber(open_positions, 0)}</span>
          <span class="metric-meta">Active trades</span>
        </div>
        <div class="status-card warning">
          <span class="metric-label">Blacklisted</span>
          <span class="metric-value">${Utils.formatNumber(blacklisted, 0)}</span>
          <span class="metric-meta">Flagged tokens</span>
        </div>
        <div class="status-card">
          <span class="metric-label">With OHLCV</span>
          <span class="metric-value">${Utils.formatNumber(with_ohlcv, 0)}</span>
          <span class="metric-meta">Historical data</span>
        </div>
        <div class="status-card">
          <span class="metric-label">Last Refresh</span>
          <span class="metric-value">${updated_at ? Utils.formatTimeAgo(new Date(updated_at)) : "Never"}</span>
          <span class="metric-meta">${updated_at ? new Date(updated_at).toLocaleString() : "No refresh yet"}</span>
        </div>
      </div>
    `;

    let rejectionHtml = '<div class="status-rejection-empty">No rejection data available</div>';

    if (state.rejectionStats?.stats?.length > 0) {
      const bySource = state.rejectionStats.by_source || {};
      const topReasons = state.rejectionStats.stats.slice(0, 20);
      const maxCount = topReasons[0]?.count || 1;

      const sourcePills = Object.entries(bySource)
        .sort((a, b) => b[1] - a[1])
        .map(
          ([src, cnt]) => `
          <div class="rej-source-pill ${Utils.escapeHtml(src)}">
            <span class="rej-source-name">${Utils.escapeHtml(src)}</span>
            <span class="rej-source-count">${Utils.formatNumber(cnt, 0)}</span>
          </div>`
        )
        .join("");

      const rejItems = topReasons
        .map(({ reason, display_label, source, count }) => {
          const barWidth = Math.min((count / maxCount) * 100, 100).toFixed(1);
          return `
          <div class="rejection-item">
            <div class="rej-bar" style="width: ${barWidth}%"></div>
            <span class="rej-label">${Utils.escapeHtml(display_label || reason)}</span>
            <span class="rej-source-tag ${Utils.escapeHtml(source)}">${Utils.escapeHtml(source)}</span>
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
      return `<div class="loading-spinner">Loading analytics for ${getTimeRangeLabel(state.timeRange)}…</div>`;
    }

    const data = state.analytics;

    // Time range filter (no title header — data only)
    const headerHtml = `
    <div class="time-range-filter">
      <div class="time-range-presets">
        <button class="time-preset-btn ${state.timeRange.preset === "1h" ? "active" : ""}" onclick="window.filteringPage.setTimeRangePreset('1h')">1H</button>
        <button class="time-preset-btn ${state.timeRange.preset === "6h" ? "active" : ""}" onclick="window.filteringPage.setTimeRangePreset('6h')">6H</button>
        <button class="time-preset-btn ${state.timeRange.preset === "24h" ? "active" : ""}" onclick="window.filteringPage.setTimeRangePreset('24h')">24H</button>
        <button class="time-preset-btn ${state.timeRange.preset === "7d" ? "active" : ""}" onclick="window.filteringPage.setTimeRangePreset('7d')">7D</button>
        <button class="time-preset-btn ${state.timeRange.preset === "all" ? "active" : ""}" onclick="window.filteringPage.setTimeRangePreset('all')">All</button>
      </div>
      <div class="time-range-custom">
        <div class="custom-range-toggle ${state.timeRange.preset === "custom" ? "active" : ""}" onclick="window.filteringPage.toggleCustomRange()">
          <i class="icon-calendar"></i> Custom
        </div>
        <div class="custom-range-inputs ${state.timeRange.preset === "custom" ? "show" : ""}">
          <input type="datetime-local" id="time-range-start" class="time-input" 
            value="${formatTimestampForInput(state.timeRange.startTime)}"
            onchange="window.filteringPage.updateCustomRange()">
          <span class="time-separator">→</span>
          <input type="datetime-local" id="time-range-end" class="time-input" 
            value="${formatTimestampForInput(state.timeRange.endTime)}"
            onchange="window.filteringPage.updateCustomRange()">
          <button class="btn btn-sm btn-primary" onclick="window.filteringPage.applyCustomRange()">Apply</button>
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
          <span class="kpi-label">Total Scanned</span>
          <span class="kpi-value">${Utils.formatNumber(data.total_tokens, 0)}</span>
          <span class="kpi-subtext">
            <i class="icon-clock"></i> Updated ${Utils.formatTimeAgo(new Date(data.last_updated))}
          </span>
        </div>
        <i class="icon-database kpi-icon"></i>
      </div>

      <!-- Passed -->
      <div class="kpi-card">
        <div class="kpi-content">
          <span class="kpi-label">Passed Tokens</span>
          <span class="kpi-value text-success">${Utils.formatNumber(data.total_passed, 0)}</span>
          <span class="kpi-subtext">
            <span class="text-success">${Utils.formatPercentValue(data.pass_rate, { includeSign: false })}</span> pass rate
          </span>
        </div>
        <i class="icon-circle-check kpi-icon text-success" style="opacity: 0.2"></i>
        <div class="pass-rate-visual">
          <div class="pass-rate-segment passed" style="width: ${data.pass_rate}%"></div>
        </div>
      </div>

      <!-- Rejected -->
      <div class="kpi-card">
        <div class="kpi-content">
          <span class="kpi-label">Rejected Tokens</span>
          <span class="kpi-value text-error">${Utils.formatNumber(data.total_rejected, 0)}</span>
          <span class="kpi-subtext">
            <span class="text-error">${Utils.formatPercentValue(data.rejection_rate, { includeSign: false })}</span> rejection rate
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
          <div class="chart-title"><i class="icon-layers"></i> Rejection by Category</div>
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
                <div class="bar-label" title="${Utils.escapeHtml(cat.label)}">${Utils.escapeHtml(cat.label)}</div>
              </div>
              <div class="bar-track-col">
                <div class="bar-track">
                  <div class="bar-fill" style="width: ${Math.min(cat.percentage, 100)}%; background-color: var(--error-color)"></div>
                </div>
                <div class="bar-meta">
                  <span>${Utils.formatNumber(cat.count, 0)} tokens</span>
                  <span>${Utils.formatPercentValue(cat.percentage, { includeSign: false })}</span>
                </div>
              </div>
            </div>
          `
                  )
                  .join("")
              : '<div class="analytics-empty">No category data</div>'
          }
        </div>
      </div>

      <!-- Rejection by Source -->
      <div class="chart-card">
        <div class="chart-header">
          <div class="chart-title"><i class="icon-git-branch"></i> Rejection by Source</div>
        </div>
        <div class="chart-body">
          ${
            data.by_source && data.by_source.length > 0
              ? data.by_source
                  .map(
                    (src) => `
            <div class="bar-chart-row">
              <div class="bar-label-col">
                <div class="bar-label font-bold w-auto">${Utils.escapeHtml(src.source)}</div>
              </div>
              <div class="bar-track-col">
                <div class="bar-track">
                  <div class="bar-fill" style="width: ${Math.min(src.percentage, 100)}%; background-color: var(--warning-color)"></div>
                </div>
                <div class="bar-meta">
                  <span>${Utils.formatNumber(src.count, 0)} tokens</span>
                  <span>${Utils.formatPercentValue(src.percentage, { includeSign: false })}</span>
                </div>
              </div>
            </div>
          `
                  )
                  .join("")
              : '<div class="analytics-empty">No source data</div>'
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
          <div class="chart-title"><i class="icon-list"></i> Top Rejection Reasons</div>
        </div>
        <div class="reasons-table-container">
          <table class="reasons-table">
            <thead>
              <tr>
                <th>Reason</th>
                <th>Category</th>
                <th class="text-end">Count</th>
                <th class="text-end">%</th>
                <th class="text-end">Impact</th>
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
                      <span class="font-medium">${Utils.escapeHtml(r.display_label)}</span>
                    </td>
                    <td>
                      <span class="reason-badge">${Utils.escapeHtml(r.category)}</span>
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
                  : '<tr><td colspan="5" class="text-center p-20">No data available</td></tr>'
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
    const totalRejected = data.total_rejected || 0;
    const totalPassed = data.total_passed || 0;

    return `
    <div class="explorer-empty-dashboard">
      <div class="dashboard-welcome">
        <h2>Rejection Explorer</h2>
        <p>Rejected: ${Utils.formatNumber(totalRejected, 0)} · Passed: ${Utils.formatNumber(totalPassed, 0)}</p>
      </div>

      <div class="dashboard-grid">
        <div class="dashboard-card">
          <h3><i class="icon-trending-down"></i> Top Reasons</h3>
          <div class="top-reasons-list">
            ${topReasons
              .slice(0, 10)
              .map(
                (r) => `
              <div class="top-reason-item" onclick="window.filteringPage.selectReason('${r.reason}', '${Utils.escapeHtml(r.display_label.replace(/'/g, "\\'"))}')">
                <span class="top-reason-label">${Utils.escapeHtml(r.display_label)}</span>
                <span class="top-reason-count">${Utils.formatNumber(r.count, 0)}</span>
              </div>
            `
              )
              .join("")}
            ${topReasons.length === 0 ? '<div class="analytics-empty-compact">No data</div>' : ""}
          </div>
        </div>

        <div class="dashboard-card">
          <h3><i class="icon-clock"></i> Recent</h3>
          <div class="top-reasons-list">
            ${recentRejections
              .slice(0, 10)
              .map(
                (t) => `
              <div class="top-reason-item" onclick="window.filteringPage.selectReason('${t.reason}', '${Utils.escapeHtml(t.display_label.replace(/'/g, "\\'"))}')">
                <div class="flex-col min-w-0">
                  <span class="top-reason-label truncate">${Utils.escapeHtml(t.symbol || "Unknown")}</span>
                  <span class="text-xs truncate">${Utils.escapeHtml(t.display_label)}</span>
                </div>
                <span class="top-reason-count">${Utils.formatTimeAgo(new Date(t.rejected_at))}</span>
              </div>
            `
              )
              .join("")}
            ${recentRejections.length === 0 ? '<div class="analytics-empty-compact">No recent</div>' : ""}
          </div>
        </div>
      </div>
    </div>
  `;
  }

  function renderExplorerView() {
    if (!state.analytics) {
      return "<div class=\"loading-spinner\">Loading…</div>";
    }

    const data = state.analytics;

    // Compact Tree View
    const treeHtml = `
    <div class="explorer-layout">
      <div class="explorer-sidebar">
        <div class="explorer-sidebar-search">
          <div class="explorer-search-input-wrapper">
            <i class="icon-search"></i>
            <input type="text" placeholder="Search reasons..." oninput="window.filteringPage.filterExplorerTree(this.value)">
          </div>
        </div>

        <div class="explorer-summary-item ${!window.filteringPage.currentReason ? "active" : ""}" onclick="window.filteringPage.selectSummary()">
          <i class="icon-layout-dashboard"></i>
          <div class="explorer-summary-info">
            <span class="explorer-summary-label">Overview</span>
            <span class="explorer-summary-desc">Summary & recent</span>
          </div>
        </div>

        <div class="explorer-tree">
          ${data.by_category
            .map(
              (cat) => `
            <div class="tree-category" data-category="${cat.category}">
              <div class="tree-category-header" onclick="window.filteringPage.toggleCategory('${cat.category}')">
                <i class="icon-${Utils.escapeHtml(cat.icon)} tree-icon"></i>
                <span class="tree-label">${Utils.escapeHtml(cat.label)}</span>
                <span class="tree-count">${Utils.formatCompactNumber(cat.count)}</span>
                <i class="icon-chevron-down tree-toggle" id="toggle-${cat.category}"></i>
              </div>
              <div class="tree-reasons" id="reasons-${cat.category}" style="display: none">
                ${cat.reasons
                  .map(
                    (r) => `
                  <div class="tree-reason ${window.filteringPage.currentReason === r.reason ? "active" : ""}" 
                       onclick="window.filteringPage.selectReason('${r.reason}', '${Utils.escapeHtml(r.display_label.replace(/'/g, "\\'"))}')" 
                       id="reason-${r.reason}"
                       data-label="${Utils.escapeHtml(r.display_label.toLowerCase())}">
                    <span class="tree-reason-label">${Utils.escapeHtml(r.display_label)}</span>
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

  function renderConfigField(field, source) {
    const value = getConfigValue(state.draft, source, field.key);
    const originalValue = getConfigValue(state.config, source, field.key);
    const isChanged = state.config && value !== originalValue;
    const fieldClass = isChanged ? "filtering-config-field changed" : "filtering-config-field";
    const fieldId = `field-${source}-${field.key}`;

    if (field.type === "boolean") {
      return `
      <div class="${fieldClass}">
        <label>
          <span class="field-label">${Utils.escapeHtml(field.label)}</span>
          <span class="field-hint">${Utils.escapeHtml(field.hint)}</span>
        </label>
        <div>
          <input
            type="checkbox"
            id="${fieldId}"
            data-field="${field.key}"
            data-source="${source}"
            ${value ? "checked" : ""}
          />
          <div class="field-meta">
            <span class="field-impact ${field.impact}">${field.impact}</span>
          </div>
        </div>
      </div>
    `;
    }

    return `
    <div class="${fieldClass}">
      <label>
        <span class="field-label">${Utils.escapeHtml(field.label)}</span>
        <span class="field-hint">${Utils.escapeHtml(field.hint)}</span>
      </label>
      <div>
        <input
          type="number"
          id="${fieldId}"
          data-field="${field.key}"
          data-source="${source}"
          value="${value}"
          min="${field.min}"
          max="${field.max}"
          step="${field.step}"
        />
        <div class="field-meta">
          <span class="field-unit">${Utils.escapeHtml(field.unit)}</span>
          <span class="field-impact ${field.impact}">${field.impact}</span>
        </div>
      </div>
    </div>
  `;
  }

  function renderCategoryToggle(source, enableKey, _categoryName) {
    if (!enableKey) return "";

    const enabled = getCategoryEnabled(state.draft, source, enableKey);
    const toggleId = `category-toggle-${source}-${enableKey}`;

    return `
    <label class="category-switch">
      <input
        type="checkbox"
        id="${toggleId}"
        data-category-toggle="${source}"
        data-enable-key="${enableKey}"
        ${enabled ? "checked" : ""}
      />
      <span class="slider"></span>
    </label>
  `;
  }

  function renderConfigCategory(categoryName, categoryData) {
    const { source, enableKey, fields } = categoryData;
    const sourceEnabled = getSourceEnabled(state.draft, source);
    const categoryEnabled = getCategoryEnabled(state.draft, source, enableKey);
    // Only disable the card body (fields) if source is disabled OR category is disabled
    // Keep header interactive so users can toggle categories on/off
    const isDisabled = (source !== "meta" && !sourceEnabled) || (enableKey && !categoryEnabled);
    const matchesSearch = (field) =>
      !state.searchQuery ||
      field.label.toLowerCase().includes(state.searchQuery) ||
      field.key.toLowerCase().includes(state.searchQuery);

    const disabledClass = isDisabled ? "disabled" : "";
    const visibleFields = fields.filter(matchesSearch);

    if (state.searchQuery && visibleFields.length === 0) {
      return "";
    }

    return `
    <div class="filter-card" data-source="${source}" data-category="${Utils.escapeHtml(categoryName)}">
      <div class="card-header">
        <h3>${Utils.escapeHtml(categoryName)}</h3>
        ${renderCategoryToggle(source, enableKey, categoryName)}
      </div>
      <div class="card-body ${disabledClass}">
        ${
          visibleFields.map((field) => renderConfigField(field, source)).join("") ||
          '<div class="no-matches">No fields match</div>'
        }
      </div>
    </div>
  `;
  }

  function renderConfigPanels() {
    if (!state.draft) {
      return '<div class="filtering-config-empty">Loading configuration...</div>';
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

    const targetSource = state.activeTab || "meta";
    const categories = Object.entries(CONFIG_CATEGORIES).filter(([, data]) => {
      if (targetSource === "meta") return data.source === "meta";
      return data.source === targetSource;
    });

    const cards = categories
      .map(([name, data]) => renderConfigCategory(name, data))
      .filter((html) => html && html.trim().length > 0)
      .join("");

    if (!cards) {
      return '<div class="filtering-config-empty">No settings match your search</div>';
    }

    return `<div class="config-scroll-area"><div class="cards-grid">${cards}</div></div>`;
  }

  function renderSourceToggle(source) {
    if (!state.draft) return "";

    const enabled = getSourceEnabled(state.draft, source);
    const sourceLabelMap = {
      dexscreener: "DexScreener Enabled",
      geckoterminal: "GeckoTerminal Enabled",
      rugcheck: "RugCheck Enabled",
    };
    const sourceLabel = sourceLabelMap[source] || "Source Enabled";
    const toggleId = `source-toggle-${source}`;

    return `
    <div class="source-toggle-wrapper">
      <span class="label">${sourceLabel}</span>
      <label class="source-switch">
        <input
          type="checkbox"
          id="${toggleId}"
          data-source-toggle="${source}"
          ${enabled ? "checked" : ""}
        />
        <span class="slider"></span>
      </label>
    </div>
  `;
  }

  function renderSearchBar() {
    // Only show search bar on settings tabs (not status/analytics/explorer)
    const isSettingsTab = ["meta", "dexscreener", "geckoterminal", "rugcheck"].includes(
      state.activeTab
    );
    if (!isSettingsTab) {
      return "";
    }

    const showSourceToggle =
      state.activeTab === "dexscreener" ||
      state.activeTab === "geckoterminal" ||
      state.activeTab === "rugcheck";
    const sourceToggle = showSourceToggle ? renderSourceToggle(state.activeTab) : "";

    return `
      <input
        type="text"
        id="filtering-search"
        placeholder="Search settings..."
        value="${Utils.escapeHtml(state.searchQuery)}"
      />
      ${sourceToggle}
  `;
  }

  function renderShell() {
    const statusMsg = getStatusMessage({
      isSaving: state.isSaving,
      isRefreshing: state.isRefreshing,
      hasChanges: state.hasChanges,
      lastSaved: state.lastSaved,
      Utils,
    });

    return `
    <div class="filtering-page">
      <div class="filtering-shell">
        <div class="filtering-info-bar" id="filtering-info-bar">${renderInfoBar()}</div>
        <div class="filtering-search-bar" id="filtering-search-bar">${renderSearchBar()}</div>
        <div class="filtering-content" id="filtering-config-panels">
          ${renderConfigPanels()}
        </div>
        <footer class="filtering-footer">
          <div class="footer-left">
            <span id="filtering-status-message">${Utils.escapeHtml(statusMsg)}</span>
          </div>
          <div class="footer-actions">
            <button class="ghost" id="reset-config-btn"><i class="icon-rotate-ccw"></i> Reset</button>
            <button class="ghost" id="refresh-snapshot-btn"><i class="icon-refresh-cw"></i> Refresh</button>
            <button class="ghost" id="export-config-btn"><i class="icon-download"></i> Export</button>
            <button class="ghost" id="import-config-btn"><i class="icon-upload"></i> Import</button>
            <button class="primary" id="save-config-btn"><i class="icon-save"></i> Save</button>
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
    renderConfigField,
    renderCategoryToggle,
    renderConfigCategory,
    renderConfigPanels,
    renderSourceToggle,
    renderSearchBar,
    renderShell,
  };
}
