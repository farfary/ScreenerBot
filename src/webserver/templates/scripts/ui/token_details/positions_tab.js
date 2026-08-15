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
import { renderTabState } from "./state_handling.js";

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
      this._renderHtmlIfChanged(
        content,
        renderTabState({
          icon: "icon-chart-bar",
          title: "No position",
          message: "No token selected.",
        }),
        "__posHtml"
      );
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
      this._renderHtmlIfChanged(
        content,
        renderTabState({ kind: "loading", message: "Loading position…" }),
        "__posHtml"
      );
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
        renderTabState({
          icon: "icon-chart-bar",
          title: "No position",
          message: "No position for this token yet. Use Buy to open one.",
        }),
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

function renderPositionSummary(position) {
  const isClosed = !!position.exit_time;
  const stateLabel = position.archived ? "Archived" : isClosed ? "Closed" : "Open";
  const stateClass = position.archived ? "muted" : isClosed ? "warning" : "good";

  // A wallet-derived round whose cost basis could not be established, or whose history
  // does not reconcile with the chain, has no honest P&L or size. Rendering a number
  // there (invested is zero for such a round) would read as pure profit on an airdrop.
  const basisUnknown = position.basis_complete === false;
  const pnlUnknown = basisUnknown || position.history_complete === false;

  const pnlSol = pnlUnknown ? null : isClosed ? position.pnl : position.unrealized_pnl;
  const pnlPct = pnlUnknown ? null : isClosed ? position.pnl_percent : position.unrealized_pnl_percent;
  const entry = pickPrice(
    position.effective_entry_price,
    position.average_entry_price,
    position.entry_price
  );
  const current = position.current_price;
  const sizeSol = basisUnknown ? null : (position.total_size_sol ?? position.entry_size_sol);
  const tokensHeld = isClosed
    ? position.token_amount
    : (position.remaining_token_amount ?? position.token_amount);
  const ageStr = position.entry_time
    ? Utils.formatTimeAgo(new Date(position.entry_time * 1000))
    : "—";

  const metadata = [];
  const ownershipLabels = {
    auto_trader: "Auto Trader",
    user_only: "User Only",
    copy_task: "Copy Task",
    hybrid: "Hybrid",
  };
  metadata.push(
    `<span class="position-meta-item">${ownershipLabels[position.management] || "Auto Trader"}</span>`
  );
  if (position.origin?.kind === "external") {
    metadata.push('<span class="position-meta-item">From wallet history</span>');
  }
  if (position.holding_state === "frozen") {
    metadata.push('<span class="position-meta-item">Frozen — cannot be sold</span>');
  }
  if (pnlUnknown) {
    metadata.push(
      `<span class="position-meta-item">${
        basisUnknown ? "No cost basis" : "History incomplete"
      }</span>`
    );
  }
  if (position.dca_count > 0) {
    metadata.push(`<span class="position-meta-item">DCA ${position.dca_count}</span>`);
  }
  if (position.partial_exit_count > 0) {
    metadata.push(`<span class="position-meta-item">Exits ${position.partial_exit_count}</span>`);
  }

  const marketFacts = [
    ["Avg Entry", basisUnknown ? "—" : fmtPrice(entry)],
    ["Current", fmtPrice(current)],
    ["Tokens", tokensHeld != null ? Utils.formatCompactNumber(tokensHeld) : "—"],
    ["Opened", ageStr],
  ];
  if (isClosed) {
    marketFacts.push(
      ["Exit Price", fmtPrice(position.effective_exit_price ?? position.exit_price)],
      ["SOL Received", fmtSol(position.sol_received)]
    );
    if (position.closed_reason) {
      marketFacts.push(["Closed Reason", escapeText(position.closed_reason), "wide"]);
    }
  }

  const hasTargets = position.profit_target_min != null || position.profit_target_max != null;
  const hasExtremes = position.price_highest != null || position.price_lowest != null;
  const rangeSection =
    hasTargets || hasExtremes
      ? `
      <section class="position-section">
        <div class="position-section-heading">Targets &amp; range</div>
        <div class="position-facts">
          ${renderPositionFact("Profit Target Min", fmtPct(position.profit_target_min))}
          ${renderPositionFact("Profit Target Max", fmtPct(position.profit_target_max))}
          ${renderPositionFact("Highest Price", fmtPrice(position.price_highest))}
          ${renderPositionFact("Lowest Price", fmtPrice(position.price_lowest))}
        </div>
      </section>`
      : "";

  return `
    <div class="positions-tab-content">
      <div class="position-sheet">
        <header class="position-sheet-header">
          <div class="position-heading">
            <span class="position-kicker">Position</span>
            <strong>${escapeText(position.symbol || "Token")}</strong>
          </div>
          <div class="position-meta">
            ${metadata.join("")}
            <span class="position-state ${stateClass}">${stateLabel}</span>
          </div>
        </header>

        <div class="position-headline">
          <div class="position-headline-item">
            <span>${isClosed ? "Realized PnL" : "Unrealized PnL"}</span>
            <strong class="${toneClass(pnlPct ?? pnlSol)}">${fmtPnl(pnlSol, pnlPct)}</strong>
          </div>
          <div class="position-headline-item">
            <span>Size</span>
            <strong>${fmtSol(sizeSol)}</strong>
          </div>
        </div>

        <section class="position-section">
          <div class="position-section-heading">Market &amp; holdings</div>
          <div class="position-facts">
            ${marketFacts
              .map(([label, value, modifier]) => renderPositionFact(label, value, modifier))
              .join("")}
          </div>
        </section>

        ${rangeSection}
      </div>
    </div>
  `;
}

// ---- formatting helpers -----------------------------------------------------

function renderPositionFact(label, value, modifier = "") {
  return `
    <div class="position-fact ${modifier}">
      <span>${label}</span>
      <strong>${value}</strong>
    </div>
  `;
}

function toneClass(value) {
  if (value === null || value === undefined || Number.isNaN(Number(value))) return "";
  return Number(value) >= 0 ? "positive" : "negative";
}

function pickPrice(...candidates) {
  for (const c of candidates) {
    if (c !== null && c !== undefined && Number.isFinite(Number(c)) && Number(c) > 0) return c;
  }
  return null;
}

function fmtPrice(value) {
  if (value === null || value === undefined || !Number.isFinite(Number(value))) return "—";
  return Utils.formatPriceSubscript(Number(value), { precision: 5 }) + " SOL";
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
