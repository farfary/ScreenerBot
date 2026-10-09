// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

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
import { formatPercentValue, withSolUnit } from "../../core/format.js";
import { requestManager } from "../../core/request_manager.js";
import { renderStateView } from "../state_view.js";
import { POSITION_MANAGEMENT_LABELS } from "../position_management.js";
import { POSITION_STATUS_LABELS } from "../position_status.js";
import { closeReasonText } from "../trade_reason.js";

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
        renderStateView({
          icon: "icon-chart-bar",
          title: I18n.t("tokens-positions-empty-title"),
          message: I18n.t("tokens-positions-no-token"),
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
        renderStateView({ kind: "loading", message: I18n.t("tokens-positions-loading") }),
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

    const position = data?.position ?? null;

    if (!position) {
      this._renderHtmlIfChanged(
        content,
        renderStateView({
          icon: "icon-chart-bar",
          title: I18n.t("tokens-positions-empty-title"),
          message: I18n.t("tokens-positions-empty-message"),
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
  const stateKey = position.archived ? "archived" : isClosed ? "closed" : "open";
  const stateLabel = I18n.label(POSITION_STATUS_LABELS, stateKey);
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
  const sizeSol = basisUnknown ? null : (position.total_size_native ?? position.entry_size_native);
  const tokensHeld = isClosed
    ? position.token_amount
    : (position.remaining_token_amount ?? position.token_amount);
  const ageStr = position.entry_time
    ? Utils.formatTimeAgo(new Date(position.entry_time * 1000))
    : "—";

  const metadata = [];
  const metaItem = (text) => `<span class="position-meta-item">${escapeText(text)}</span>`;
  metadata.push(
    metaItem(I18n.label(POSITION_MANAGEMENT_LABELS, position.management || "auto_trader"))
  );
  if (position.origin?.kind === "external") {
    metadata.push(metaItem(I18n.t("tokens-positions-from-wallet-history")));
  }
  if (position.holding_state === "frozen") {
    metadata.push(metaItem(I18n.t("tokens-positions-frozen")));
  }
  if (pnlUnknown) {
    metadata.push(
      metaItem(
        basisUnknown
          ? I18n.t("tokens-positions-no-cost-basis")
          : I18n.t("tokens-positions-history-incomplete")
      )
    );
  }
  if (position.dca_count > 0) {
    metadata.push(metaItem(I18n.t("tokens-positions-dca-count", { count: position.dca_count })));
  }
  if (position.partial_exit_count > 0) {
    metadata.push(
      metaItem(I18n.t("tokens-positions-exit-count", { count: position.partial_exit_count }))
    );
  }

  const marketFacts = [
    [I18n.t("tokens-positions-fact-avg-entry"), basisUnknown ? "—" : fmtPrice(entry)],
    [I18n.t("tokens-positions-fact-current"), fmtPrice(current)],
    [
      I18n.t("tokens-positions-fact-tokens"),
      tokensHeld != null ? Utils.formatCompactNumber(tokensHeld) : "—",
    ],
    [I18n.t("tokens-positions-fact-opened"), ageStr],
  ];
  if (isClosed) {
    marketFacts.push(
      [
        I18n.t("tokens-positions-fact-exit-price"),
        fmtPrice(position.effective_exit_price ?? position.exit_price),
      ],
      [I18n.t("tokens-positions-fact-native-received"), fmtSol(position.sol_received)]
    );
    if (position.closed_reason) {
      marketFacts.push([
        I18n.t("tokens-positions-fact-closed-reason"),
        escapeText(closeReasonText(position.closed_reason)),
        "wide",
      ]);
    }
  }

  const hasTargets = position.profit_target_min != null || position.profit_target_max != null;
  const hasExtremes = position.price_highest != null || position.price_lowest != null;
  const rangeSection =
    hasTargets || hasExtremes
      ? `
      <section class="position-section">
        <div class="position-section-heading">${escapeText(I18n.t("tokens-positions-section-range"))}</div>
        <div class="position-facts">
          ${renderPositionFact(I18n.t("tokens-positions-fact-target-min"), fmtPct(position.profit_target_min))}
          ${renderPositionFact(I18n.t("tokens-positions-fact-target-max"), fmtPct(position.profit_target_max))}
          ${renderPositionFact(I18n.t("tokens-positions-fact-highest"), fmtPrice(position.price_highest))}
          ${renderPositionFact(I18n.t("tokens-positions-fact-lowest"), fmtPrice(position.price_lowest))}
        </div>
      </section>`
      : "";

  return `
    <div class="positions-tab-content">
      <div class="position-sheet">
        <header class="position-sheet-header">
          <div class="position-heading">
            <span class="position-kicker">${escapeText(I18n.t("tokens-positions-kicker"))}</span>
            <strong>${escapeText(position.symbol || I18n.t("tokens-positions-fallback-symbol"))}</strong>
          </div>
          <div class="position-meta">
            ${metadata.join("")}
            <span class="position-state ${stateClass}">${stateLabel}</span>
          </div>
        </header>

        <div class="position-headline">
          <div class="position-headline-item">
            <span>${escapeText(isClosed ? I18n.t("tokens-positions-realized-pnl") : I18n.t("tokens-positions-unrealized-pnl"))}</span>
            <strong class="${toneClass(pnlPct ?? pnlSol)}">${fmtPnl(pnlSol, pnlPct)}</strong>
          </div>
          <div class="position-headline-item">
            <span>${escapeText(I18n.t("tokens-positions-size"))}</span>
            <strong>${fmtSol(sizeSol)}</strong>
          </div>
        </div>

        <section class="position-section">
          <div class="position-section-heading">${escapeText(I18n.t("tokens-positions-section-market"))}</div>
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
      <span>${escapeText(label)}</span>
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
  return withSolUnit(Utils.formatPriceSubscript(Number(value), { trim: false }));
}

function fmtSol(value) {
  if (value === null || value === undefined || !Number.isFinite(Number(value))) return "—";
  return withSolUnit(Utils.formatNumber(Number(value), { decimals: 4 }));
}

function fmtPct(value) {
  if (value === null || value === undefined || !Number.isFinite(Number(value))) return "—";
  return formatPercentValue(value, { decimals: 1, signZero: true });
}

function fmtPnl(sol, pct) {
  const hasSol = sol !== null && sol !== undefined && Number.isFinite(Number(sol));
  const hasPct = pct !== null && pct !== undefined && Number.isFinite(Number(pct));
  if (!hasSol && !hasPct) return "—";
  const solStr = hasSol
    ? Utils.formatSignedSol(sol, { decimals: 4 })
    : "";
  const pctStr = hasPct ? formatPercentValue(pct, { decimals: 2, signZero: true }) : "";
  return [solStr, pctStr].filter(Boolean).join("  ");
}

function escapeText(text) {
  if (text === null || text === undefined) return "";
  const div = document.createElement("div");
  div.textContent = String(text);
  return div.innerHTML;
}
