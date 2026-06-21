/**
 * Token Details Dialog - Positions Tab Mixin
 *
 * Implements `_loadPositionsTab`, which was referenced by the tab loader but had
 * no definition after the dialog was split into per-tab modules — so selecting
 * the Positions tab threw "this._loadPositionsTab is not a function" and the tab
 * was stuck on its initial "Loading…" spinner forever. This loads the token's
 * position from /api/positions/{mint}/details and renders a summary (or a clean
 * empty state), and feeds entry/exit markers to the chart.
 */
import * as Utils from "../../core/utils.js";
import { requestManager } from "../../core/request_manager.js";

export function applyPositionsTabMixin(DialogClass) {
  const proto = DialogClass.prototype;

  /**
   * Load the Positions tab content for the current token.
   * @param {HTMLElement} content - the tab content element
   */
  proto._loadPositionsTab = async function (content) {
    if (!content) return;

    const mint = this.tokenData?.mint;
    if (!mint) {
      this._renderHtmlIfChanged(content, renderPositionsEmpty("No token selected."), "__posHtml");
      content.dataset.loaded = "true";
      return;
    }

    // Guard against overlapping fetches: the 5s poller calls this live while the
    // tab is open, so skip if a previous fetch is still in flight.
    if (this._positionsFetching) return;
    this._positionsFetching = true;

    // Show a spinner only on the very first paint (no cached markup yet), so
    // refreshes after a trade don't flash.
    if (!content.__posHtml) {
      content.innerHTML =
        '<div class="tdd-state tdd-state-loading"><div class="loading-spinner">Loading position…</div></div>';
    }

    let data = null;
    try {
      data = await requestManager.fetch(`/api/positions/${encodeURIComponent(mint)}/details`, {
        priority: "high",
      });
    } catch {
      // Network/HTTP error (incl. 404 "no position"); fall through to empty state.
      data = null;
    } finally {
      this._positionsFetching = false;
    }

    const position = data && !data.error ? data.position : null;

    if (!position) {
      this._renderHtmlIfChanged(
        content,
        renderPositionsEmpty("No position for this token yet. Use Buy to open one."),
        "__posHtml"
      );
      content.dataset.loaded = "true";
      return;
    }

    // Feed chart markers (entry/exit points) so the Overview chart can draw them.
    // NOTE: profit_target_min/max are PERCENTAGES, not prices, so they are not
    // passed as horizontal price lines (that would draw lines at 5/20 SOL).
    this.positionsData = {
      entries: Array.isArray(data.entries) ? data.entries : [],
      exits: Array.isArray(data.exits) ? data.exits : [],
      stop_loss_price: null,
      take_profit_price: null,
    };

    const html = renderPositionSummary(position);
    this._renderHtmlIfChanged(content, html, "__posHtml");
    content.dataset.loaded = "true";

    // Update chart markers if the chart already exists.
    if (this.advancedChart && typeof this._updateChartPositions === "function") {
      this._updateChartPositions();
    }
  };
}

function renderPositionsEmpty(message) {
  return `
    <div class="tdd-state">
      <i class="tdd-state-icon icon-chart-bar" aria-hidden="true"></i>
      <div class="tdd-state-title">No position</div>
      <div class="tdd-state-message">${message}</div>
    </div>
  `;
}

function renderPositionSummary(position) {
  const isClosed = !!position.exit_time;
  const stateLabel = position.archived ? "Archived" : isClosed ? "Closed" : "Open";
  const stateClass = position.archived ? "muted" : isClosed ? "warning" : "good";

  // PnL: open positions track unrealized, closed positions track realized.
  const pnlSol = isClosed ? position.pnl : position.unrealized_pnl;
  const pnlPct = isClosed ? position.pnl_percent : position.unrealized_pnl_percent;

  const entry = pickPrice(position.effective_entry_price, position.average_entry_price, position.entry_price);
  const current = position.current_price;
  const sizeSol = position.total_size_sol ?? position.entry_size_sol;
  const tokensHeld = isClosed ? position.token_amount : position.remaining_token_amount ?? position.token_amount;
  const ageStr = position.entry_time
    ? Utils.formatTimeAgo(new Date(position.entry_time * 1000))
    : "—";

  const badges = [];
  if (position.manual_management) {
    badges.push('<span class="badge badge-warning">Manual</span>');
  }
  if (position.dca_count > 0) {
    badges.push(`<span class="badge badge-secondary">DCA ${position.dca_count}</span>`);
  }
  if (position.partial_exit_count > 0) {
    badges.push(`<span class="badge badge-secondary">Exits ${position.partial_exit_count}</span>`);
  }

  const closedRows = isClosed
    ? `
      <div class="info-cell">
        <span class="cell-label">Exit Price</span>
        <span class="cell-value">${fmtPrice(position.effective_exit_price ?? position.exit_price)}</span>
      </div>
      <div class="info-cell">
        <span class="cell-label">SOL Received</span>
        <span class="cell-value">${fmtSol(position.sol_received)}</span>
      </div>
      ${
        position.closed_reason
          ? `<div class="info-cell full-width"><span class="cell-label">Closed Reason</span><span class="cell-value">${escapeText(position.closed_reason)}</span></div>`
          : ""
      }
    `
    : "";

  const hasTargets = position.profit_target_min != null || position.profit_target_max != null;
  const hasExtremes = position.price_highest != null || position.price_lowest != null;
  const targets =
    hasTargets || hasExtremes
      ? `
      <div class="info-card compact">
        <div class="card-header"><span>Targets &amp; Range</span></div>
        <div class="card-body">
          <div class="info-grid-2col">
            <div class="info-cell"><span class="cell-label">Profit Target Min</span><span class="cell-value">${fmtPct(position.profit_target_min)}</span></div>
            <div class="info-cell"><span class="cell-label">Profit Target Max</span><span class="cell-value">${fmtPct(position.profit_target_max)}</span></div>
            <div class="info-cell"><span class="cell-label">Highest Price</span><span class="cell-value">${fmtPrice(position.price_highest)}</span></div>
            <div class="info-cell"><span class="cell-label">Lowest Price</span><span class="cell-value">${fmtPrice(position.price_lowest)}</span></div>
          </div>
        </div>
      </div>`
      : "";

  return `
    <div class="positions-tab-content" style="padding: 16px; overflow-y: auto; height: 100%;">
      <div class="info-card compact">
        <div class="card-header">
          <span>Position — ${escapeText(position.symbol || "")}</span>
          <div class="card-header-actions">
            ${badges.join("")}
            <span class="status-badge ${stateClass}">${stateLabel}</span>
          </div>
        </div>
        <div class="card-body">
          <div class="info-grid-2col">
            <div class="info-cell highlight">
              <span class="cell-label">${isClosed ? "Realized PnL" : "Unrealized PnL"}</span>
              <span class="cell-value large" style="${pnlColor(pnlPct)}">${fmtPnl(pnlSol, pnlPct)}</span>
            </div>
            <div class="info-cell highlight">
              <span class="cell-label">Size</span>
              <span class="cell-value large">${fmtSol(sizeSol)}</span>
            </div>
            <div class="info-cell">
              <span class="cell-label">Avg Entry</span>
              <span class="cell-value">${fmtPrice(entry)}</span>
            </div>
            <div class="info-cell">
              <span class="cell-label">Current</span>
              <span class="cell-value">${fmtPrice(current)}</span>
            </div>
            <div class="info-cell">
              <span class="cell-label">Tokens</span>
              <span class="cell-value">${tokensHeld != null ? Utils.formatCompactNumber(tokensHeld) : "—"}</span>
            </div>
            <div class="info-cell">
              <span class="cell-label">Opened</span>
              <span class="cell-value">${ageStr}</span>
            </div>
            ${closedRows}
          </div>
        </div>
      </div>
      ${targets}
    </div>
  `;
}

// ---- formatting helpers -----------------------------------------------------

function pnlColor(value) {
  if (value === null || value === undefined || Number.isNaN(Number(value))) return "";
  return Number(value) >= 0
    ? "color: var(--success-color);"
    : "color: var(--error-color);";
}

function pickPrice(...candidates) {
  for (const c of candidates) {
    if (c !== null && c !== undefined && Number.isFinite(Number(c)) && Number(c) > 0) return c;
  }
  return null;
}

function fmtPrice(value) {
  if (value === null || value === undefined || !Number.isFinite(Number(value))) return "—";
  return Utils.formatPriceSol(Number(value), { decimals: 9 }) + " SOL";
}

function fmtSol(value) {
  if (value === null || value === undefined || !Number.isFinite(Number(value))) return "—";
  return Utils.formatNumber(Number(value), { decimals: 4 }) + " SOL";
}

function fmtPct(value) {
  if (value === null || value === undefined || !Number.isFinite(Number(value))) return "—";
  return `${Number(value) >= 0 ? "+" : ""}${Number(value).toFixed(1)}%`;
}

function fmtPnl(sol, pct) {
  const hasSol = sol !== null && sol !== undefined && Number.isFinite(Number(sol));
  const hasPct = pct !== null && pct !== undefined && Number.isFinite(Number(pct));
  if (!hasSol && !hasPct) return "—";
  const solStr = hasSol
    ? `${Number(sol) >= 0 ? "+" : ""}${Utils.formatNumber(Number(sol), { decimals: 4 })} SOL`
    : "";
  const pctStr = hasPct ? `${Number(pct) >= 0 ? "+" : ""}${Number(pct).toFixed(2)}%` : "";
  return [solStr, pctStr].filter(Boolean).join("  ");
}

function escapeText(text) {
  if (text === null || text === undefined) return "";
  const div = document.createElement("div");
  div.textContent = String(text);
  return div.innerHTML;
}
