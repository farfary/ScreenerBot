/**
 * Activity Tab Mixin for the Position Details Dialog.
 *
 * ONE tab for everything that ever happened to a position. It replaces the old History and
 * Transactions tabs, which showed the same swaps twice: History rendered the position
 * RECORDS (booked amount / price / SOL) and Transactions rendered the on-chain summaries
 * (status / fee / router / transfers) and then re-joined the records by signature to get
 * the amounts back. Two views of one event, each missing what the other had, and the join
 * written twice. The backend now serves the join (`activity`), so this renders one merged,
 * chronological timeline with the full record AND the full chain data on every card.
 */
import * as Utils from "../../core/utils.js";
import { activityEventKey, renderActivityCard } from "./activity_event.js";

export function applyActivityTabMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

  proto._renderActivityTab = function (content) {
    const events = this.fullDetails?.activity || [];
    const totals = this.fullDetails?.activity_totals || {};
    const stateHistory = this.fullDetails?.state_history || [];

    if (events.length === 0 && stateHistory.length === 0) {
      content.innerHTML = `
        <div class="pdd-empty-state">
          <i class="icon-activity"></i>
          <p>No activity recorded for this position yet</p>
        </div>`;
      return;
    }

    // The Pending / Failed pills only exist while such a swap does. Once the last one
    // settles its pill disappears, so a filter still pointing at it would hide every card
    // with no lit pill to click back — fall back to All.
    const available = new Set(["all", "entry", "exit"]);
    if (totals.pending) available.add("pending");
    if (totals.failed) available.add("failed");
    if (!available.has(this._activityFilter)) this._activityFilter = "all";

    // Backend serves the timeline oldest-first (that is the order its running position
    // state is derived in). Display defaults to newest-first.
    const ordered = this._activitySort === "oldest" ? events : [...events].reverse();

    const ctx = {
      symbol: this.fullDetails?.position?.symbol || "tokens",
      solPriceUsd: this.fullDetails?.sol_price_usd || null,
      expanded: this._activityExpanded,
      toUi: (raw) => this._toUiAmount(raw),
      formatPrice: (price) => this._formatPrice(price),
      sideCounts: {
        entry: events.filter((event) => event.side === "entry").length,
        exit: events.filter((event) => event.side === "exit").length,
      },
    };

    const cards = ordered.map((event) => renderActivityCard(event, ctx)).join("");

    content.innerHTML = `
      <div class="pdd-activity">
        ${this._buildActivitySummary(totals, ctx)}
        ${this._buildActivityToolbar(totals)}
        <div class="pdd-act-list">${cards}</div>
        ${this._buildStateHistory(stateHistory)}
      </div>`;

    this._applyActivityFilter(content);
    this._bindActivityHandlers(content);
  };

  /**
   * Headline totals for the whole timeline. The backend sums these while it walks the
   * events, so the numbers here can never drift from the cards below them.
   */
  proto._buildActivitySummary = function (totals, ctx) {
    const usd = (sol) =>
      ctx.solPriceUsd && sol ? Utils.formatCurrencyUSD(sol * ctx.solPriceUsd) : "";

    const netFlow = (totals.sol_returned || 0) - (totals.sol_invested || 0);
    const realized = totals.realized_pnl || 0;
    // The basis the realized P&L was earned on: proceeds minus profit.
    const realizedBasis = (totals.sol_returned || 0) - realized;
    const realizedPct = realizedBasis > 0 ? (realized / realizedBasis) * 100 : null;

    const openIssues = [];
    if (totals.pending) openIssues.push(`${totals.pending} pending`);
    if (totals.failed) openIssues.push(`${totals.failed} failed`);

    const cell = (label, value, sub = "", cls = "") => `
      <div class="pdd-act-sum-cell">
        <span class="pdd-act-sum-label">${label}</span>
        <span class="pdd-act-sum-value ${cls}">${value}</span>
        <span class="pdd-act-sum-sub">${sub || "&nbsp;"}</span>
      </div>`;

    const sign = (value) => (value >= 0 ? "+" : "");
    const tone = (value) => (value >= 0 ? "pdd-positive" : "pdd-negative");
    const sol = (value, decimals = 4) =>
      `${Utils.formatSol(value || 0, { decimals, suffix: "" })} SOL`;

    return `
      <div class="pdd-act-summary">
        ${cell(
          "Swaps",
          String(totals.events ?? 0),
          openIssues.length
            ? openIssues.join(" · ")
            : `${totals.entries || 0} in · ${totals.exits || 0} out`
        )}
        ${cell(
          "Bought",
          `${Utils.formatCompactNumber(ctx.toUi(totals.tokens_bought || 0))} ${Utils.escapeHtml(ctx.symbol)}`,
          `${sol(totals.sol_invested)} spent`
        )}
        ${cell(
          "Sold",
          `${Utils.formatCompactNumber(ctx.toUi(totals.tokens_sold || 0))} ${Utils.escapeHtml(ctx.symbol)}`,
          `${sol(totals.sol_returned)} received`
        )}
        ${cell("Network Fees", sol(totals.network_fees_sol, 6), usd(totals.network_fees_sol))}
        ${cell(
          "Realized P&L",
          `${sign(realized)}${sol(realized)}`,
          realizedPct !== null ? `${sign(realizedPct)}${Utils.formatNumber(realizedPct, 2)}%` : "",
          tone(realized)
        )}
        ${cell("Net Flow", `${sign(netFlow)}${sol(netFlow)}`, usd(netFlow), tone(netFlow))}
      </div>`;
  };

  proto._buildActivityToolbar = function (totals) {
    const filters = [
      ["all", "All", totals.events ?? 0],
      ["entry", "Entries", totals.entries ?? 0],
      ["exit", "Exits", totals.exits ?? 0],
    ];
    if (totals.pending) filters.push(["pending", "Pending", totals.pending]);
    if (totals.failed) filters.push(["failed", "Failed", totals.failed]);

    const pills = filters
      .map(
        ([id, label, count]) => `
        <button type="button" class="pdd-act-filter${this._activityFilter === id ? " active" : ""}" data-filter="${id}">
          ${label}<span class="pdd-act-filter-count">${count}</span>
        </button>`
      )
      .join("");

    const oldestFirst = this._activitySort === "oldest";
    return `
      <div class="pdd-act-toolbar">
        <div class="pdd-act-filters">${pills}</div>
        <button type="button" class="pdd-act-sort" id="pddActSort" title="Toggle chronological order">
          <i class="icon-arrow-up-down"></i>
          <span>${oldestFirst ? "Oldest first" : "Newest first"}</span>
        </button>
      </div>`;
  };

  /**
   * The position's own state machine (opening, open, closing, closed, failed retries…) —
   * the context that explains WHY the swaps above happened, and the only place a state
   * change with no swap of its own is visible at all.
   */
  proto._buildStateHistory = function (stateHistory) {
    if (!stateHistory.length) return "";

    const rows = [...stateHistory]
      .sort((a, b) => (b.changed_at ?? 0) - (a.changed_at ?? 0))
      .map(
        (entry) => `
        <div class="pdd-act-state-row">
          <span class="pdd-act-state-name">${Utils.escapeHtml(entry.state)}</span>
          <span class="pdd-act-state-reason">${entry.reason ? Utils.escapeHtml(entry.reason) : ""}</span>
          <span class="pdd-act-state-time" title="${Utils.formatTimestamp(entry.changed_at)}">${Utils.formatTimeAgo(entry.changed_at)}</span>
        </div>`
      )
      .join("");

    return `
      <section class="pdd-act-states">
        <h3 class="pdd-act-section-title">
          <i class="icon-history"></i>
          State History
          <span class="pdd-act-section-count">${stateHistory.length}</span>
        </h3>
        <div class="pdd-act-state-list">${rows}</div>
      </section>`;
  };

  /** Show only the cards the active filter selects. */
  proto._applyActivityFilter = function (content) {
    const filter = this._activityFilter;
    content.querySelectorAll(".pdd-act-card").forEach((card) => {
      const match =
        filter === "all" || card.dataset.side === filter || card.dataset.state === filter;
      card.classList.toggle("is-filtered-out", !match);
    });

    const list = content.querySelector(".pdd-act-list");
    if (!list) return;
    const visible = list.querySelectorAll(".pdd-act-card:not(.is-filtered-out)").length;
    let empty = list.querySelector(".pdd-act-filter-empty");
    if (visible === 0 && !empty) {
      empty = document.createElement("div");
      empty.className = "pdd-act-filter-empty";
      empty.textContent = "No swaps match this filter";
      list.appendChild(empty);
    } else if (visible > 0 && empty) {
      empty.remove();
    }
  };

  /**
   * ONE delegated listener for the whole tab (filters, sort, expand, copy). The tab content
   * node outlives every re-render, so binding here — rather than per element on each render
   * — means nothing to unbind and nothing to leak when the 5s poll redraws the list.
   */
  proto._bindActivityHandlers = function (content) {
    if (this._activityClickHandler) return;

    this._activityClickHandler = (event) => {
      const copyEl = event.target.closest("[data-copy]");
      if (copyEl) {
        event.preventDefault();
        Utils.copyToClipboard(copyEl.dataset.copy);
        Utils.showToast("Signature copied", "success");
        return;
      }

      const expandBtn = event.target.closest(".pdd-act-expand");
      if (expandBtn) {
        const key = expandBtn.dataset.expand;
        const card = expandBtn.closest(".pdd-act-card");
        const open = !card.classList.contains("is-open");
        card.classList.toggle("is-open", open);
        expandBtn.setAttribute("aria-expanded", String(open));
        expandBtn.querySelector("span").textContent = open ? "Hide" : "Details";
        // Survives the poll re-render.
        if (open) this._activityExpanded.add(key);
        else this._activityExpanded.delete(key);
        return;
      }

      const filterBtn = event.target.closest(".pdd-act-filter");
      if (filterBtn) {
        this._activityFilter = filterBtn.dataset.filter;
        content
          .querySelectorAll(".pdd-act-filter")
          .forEach((btn) => btn.classList.toggle("active", btn === filterBtn));
        this._applyActivityFilter(content);
        return;
      }

      if (event.target.closest("#pddActSort")) {
        this._activitySort = this._activitySort === "oldest" ? "newest" : "oldest";
        this._renderActivityTab(content);
      }
    };

    content.addEventListener("click", this._activityClickHandler);
  };

  /**
   * Re-render only when the timeline actually changed. The dialog polls every 5s and an
   * unconditional redraw would drop every expanded card and reset the scroll position.
   */
  proto._activityFingerprint = function () {
    const events = this.fullDetails?.activity || [];
    const states = this.fullDetails?.state_history || [];
    const last = events.at(-1);
    return [
      events.length,
      events.filter((event) => event.state === "pending").length,
      last ? `${activityEventKey(last)}:${last.state}` : "",
      states.length,
    ].join("|");
  };
}
