/**
 * Activity Tab Mixin for the Position Details Dialog.
 *
 * ONE tab for everything that ever happened to this TOKEN — not just to the position the
 * dialog was opened on. A token can be entered, exited and re-entered any number of times,
 * and each round is its own position; showing only the current one hid every earlier round.
 * On top of that the wallet can touch a mint without any position at all (a transfer, an
 * airdrop, a swap made in another app), and none of that was visible anywhere.
 *
 * So the timeline is served whole by `GET /api/positions/{key}/activity`, which merges each
 * swap's position RECORD with its on-chain TRANSACTION, walks the result chronologically to
 * derive every position's running state and each exit's realized P&L, and tags every event
 * with the round it belongs to.
 *
 * It is fetched LAZILY, only while this tab is open: it spans every position and scans the
 * wallet's transactions, which is far more work than the dialog's 5s `/details` poll — the
 * same poll the trade dialog and the row context menu ride on — should ever carry.
 *
 * All token amounts in the payload are already whole tokens (the server scaled them by the
 * mint's decimals), so nothing here converts.
 */
import * as Utils from "../../core/utils.js";
import { requestManager } from "../../core/request_manager.js";
import { activityEventKey, renderActivityCard } from "./activity_event.js";

export function applyActivityTabMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

  /**
   * Fetch the token's all-time activity. Called when the tab is first opened and on every
   * dialog poll tick WHILE it is open, so a fresh DCA or exit shows up live.
   */
  proto._fetchActivity = async function () {
    if (this._activityLoading) return;
    this._activityLoading = true;

    try {
      const key = this._getPositionKey();
      this._activity = await requestManager.fetch(`/api/positions/${key}/activity`, {
        priority: "normal",
      });
      this._activityError = null;
    } catch (error) {
      console.error("Error loading token activity:", error);
      this._activityError = "Failed to load activity";
    } finally {
      this._activityLoading = false;
    }

    const content = this.dialogEl?.querySelector('[data-tab-content="activity"]');
    if (content && this.currentTab === "activity") {
      this._paintActivity(content);
    }
  };

  /**
   * Render, but only when the timeline actually changed. Both the poll tick and the tab
   * switch come through here; an unconditional redraw every 5s would collapse every
   * expanded card and yank the scroll position out from under the user.
   *
   * The sort toggle deliberately bypasses this and calls `_renderActivityTab` directly —
   * the data is identical, only the order changed.
   */
  proto._paintActivity = function (content) {
    const fingerprint = this._activityFingerprint();
    if (fingerprint === this._activityRenderedFp) return;
    this._renderActivityTab(content);
    this._activityRenderedFp = fingerprint;
  };

  proto._renderActivityTab = function (content) {
    if (this._activityError) {
      content.innerHTML = `
        <div class="pdd-empty-state">
          <i class="icon-circle-alert"></i>
          <p>${Utils.escapeHtml(this._activityError)}</p>
        </div>`;
      return;
    }

    if (!this._activity) {
      content.innerHTML = '<div class="loading-spinner">Loading activity...</div>';
      return;
    }

    const events = this._activity.events || [];
    const totals = this._activity.totals || {};
    const positions = this._activity.positions || [];
    const stateHistory = this._activity.state_history || [];

    if (events.length === 0 && stateHistory.length === 0) {
      content.innerHTML = `
        <div class="pdd-empty-state">
          <i class="icon-activity"></i>
          <p>Nothing has happened to this token in this wallet yet</p>
        </div>`;
      return;
    }

    // The Wallet / Pending / Failed pills only exist while such an event does. Once the
    // last one settles its pill disappears, so a filter still pointing at it would hide
    // every card with no lit pill to click back — fall back to All.
    const available = new Set(["all", "entry", "exit"]);
    if (totals.wallet_events) available.add("wallet");
    if (totals.pending) available.add("pending");
    if (totals.failed) available.add("failed");
    if (!available.has(this._activityFilter)) this._activityFilter = "all";

    // The server builds the timeline oldest-first (the order its running state is derived
    // in). Display defaults to newest-first.
    const ordered = this._activitySort === "oldest" ? events : [...events].reverse();

    const currentId = this._activity.positions?.length
      ? (this.fullDetails?.position?.id ?? this.positionData?.id ?? null)
      : null;

    const ctx = {
      symbol: this._activity.symbol || this.fullDetails?.position?.symbol || "tokens",
      solPriceUsd: this._activity.sol_price_usd || null,
      expanded: this._activityExpanded,
      formatPrice: (price) => this._formatPrice(price),
      currentPositionId: currentId,
      positionCount: positions.length,
      sideCounts: {
        entry: events.filter((event) => event.side === "entry").length,
        exit: events.filter((event) => event.side === "exit").length,
        wallet: events.filter((event) => event.side === "wallet").length,
      },
    };

    const cards = ordered.map((event) => renderActivityCard(event, ctx)).join("");

    content.innerHTML = `
      <div class="pdd-activity">
        ${this._buildActivitySummary(totals, ctx)}
        ${this._buildActivityContext(positions, stateHistory, ctx)}
        ${this._buildActivityToolbar(totals)}
        <div class="pdd-act-list">${cards}</div>
      </div>`;

    this._applyActivityFilter(content);
    this._bindActivityHandlers(content);
  };

  /**
   * The two lists that give the timeline its context, side by side: what rounds of trading
   * the token has been through, and what the positions' state machine did. Both are short,
   * scannable and reference-only, so they belong next to each other above the feed rather
   * than stacked around it — the state history used to sit below every event card, where a
   * long timeline buried it.
   *
   * Either can be absent (one round of trading needs no round list), and whichever is left
   * takes the full width rather than leaving a hole.
   */
  proto._buildActivityContext = function (positions, stateHistory, ctx) {
    const panels = [
      this._buildActivityRounds(positions, ctx),
      this._buildStateHistory(stateHistory, ctx),
    ].filter(Boolean);

    if (panels.length === 0) return "";
    return `<div class="pdd-act-context${panels.length === 1 ? " is-single" : ""}">${panels.join("")}</div>`;
  };

  /** The shared panel shell — one header treatment for both lists. */
  proto._activityPanel = function (icon, title, count, body) {
    return `
      <section class="pdd-act-panel">
        <header class="pdd-act-panel-head">
          <i class="${icon}"></i>
          <span class="pdd-act-panel-title">${title}</span>
          <span class="pdd-act-count">${count}</span>
        </header>
        <div class="pdd-act-panel-body">${body}</div>
      </section>`;
  };

  /**
   * All-time headline totals for the token. The server sums these while it walks the
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

    // Rounds first, then anything worth flagging. The notes used to REPLACE the round count
    // rather than follow it, so as soon as the token saw one wallet event you could no
    // longer tell how many times it had been traded.
    const rounds = totals.positions || 0;
    const notes = [`${rounds} position${rounds === 1 ? "" : "s"}`];
    if (totals.wallet_events) notes.push(`${totals.wallet_events} wallet`);
    if (totals.pending) notes.push(`${totals.pending} pending`);
    if (totals.failed) notes.push(`${totals.failed} failed`);

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
        ${cell("Events", String(totals.events ?? 0), notes.join(" · "))}
        ${cell(
          "Bought",
          `${Utils.formatCompactNumber(totals.tokens_bought || 0)} ${Utils.escapeHtml(ctx.symbol)}`,
          `${sol(totals.sol_invested)} spent`
        )}
        ${cell(
          "Sold",
          `${Utils.formatCompactNumber(totals.tokens_sold || 0)} ${Utils.escapeHtml(ctx.symbol)}`,
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

  /**
   * Every round of trading this token. Rendered only when there is more than one — with a
   * single position the cards' own chips already say everything this would.
   */
  proto._buildActivityRounds = function (positions, ctx) {
    if (positions.length < 2) return "";

    // Newest round first, to match the feed's default order.
    const rows = [...positions]
      .reverse()
      .map((position) => {
        const isCurrent = position.id === ctx.currentPositionId;
        const pnl = position.realized_pnl || 0;
        const status = position.is_open ? "Open" : position.archived ? "Archived" : "Closed";
        const closed = position.closed_at ? Utils.formatTimeAgo(position.closed_at) : "in progress";

        return `
          <div class="pdd-act-row${isCurrent ? " is-current" : ""}">
            <span class="pdd-act-row-name">Position ${position.index}</span>
            <span class="pdd-act-round-status is-${status.toLowerCase()}">${status}</span>
            <span class="pdd-act-row-note" title="Opened ${Utils.formatTimestamp(position.opened_at)}">
              ${Utils.formatTimeAgo(position.opened_at)} &rarr; ${closed} &middot; ${position.swaps} swap${position.swaps === 1 ? "" : "s"}
            </span>
            <span class="pdd-act-row-value ${pnl >= 0 ? "pdd-positive" : "pdd-negative"}">
              ${pnl >= 0 ? "+" : ""}${Utils.formatSol(pnl, { decimals: 4, suffix: "" })} SOL
            </span>
          </div>`;
      })
      .join("");

    return this._activityPanel("icon-layers", "Trading Rounds", positions.length, rows);
  };

  proto._buildActivityToolbar = function (totals) {
    const filters = [
      ["all", "All", totals.events ?? 0],
      ["entry", "Entries", totals.entries ?? 0],
      ["exit", "Exits", totals.exits ?? 0],
    ];
    if (totals.wallet_events) filters.push(["wallet", "Wallet", totals.wallet_events]);
    if (totals.pending) filters.push(["pending", "Pending", totals.pending]);
    if (totals.failed) filters.push(["failed", "Failed", totals.failed]);

    const pills = filters
      .map(
        ([id, label, count]) => `
        <button type="button" class="pdd-act-filter${this._activityFilter === id ? " active" : ""}" data-filter="${id}">
          ${label}<span class="pdd-act-count">${count}</span>
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
   * The positions' own state machine (opening, open, closing, closed, failed retries…) —
   * the context that explains WHY the swaps above happened, and the only place a state
   * change with no swap of its own is visible at all.
   */
  proto._buildStateHistory = function (stateHistory, ctx) {
    if (!stateHistory.length) return "";

    const rows = [...stateHistory]
      .sort((a, b) => (b.changed_at ?? 0) - (a.changed_at ?? 0))
      .map(
        (entry) => `
        <div class="pdd-act-row">
          <span class="pdd-act-row-name">${Utils.escapeHtml(entry.state)}</span>
          ${
            ctx.positionCount > 1
              ? `<span class="pdd-act-chip-pos">Position ${entry.position_index}</span>`
              : ""
          }
          <span class="pdd-act-row-note">${entry.reason ? Utils.escapeHtml(entry.reason) : ""}</span>
          <span class="pdd-act-row-time" title="${Utils.formatTimestamp(entry.changed_at)}">${Utils.formatTimeAgo(entry.changed_at)}</span>
        </div>`
      )
      .join("");

    return this._activityPanel("icon-history", "State History", stateHistory.length, rows);
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
      empty.textContent = "No events match this filter";
      list.appendChild(empty);
    } else if (visible > 0 && empty) {
      empty.remove();
    }
  };

  /**
   * ONE delegated listener for the whole tab (filters, sort, expand, copy). The tab content
   * node outlives every re-render, so binding here — rather than per element on each render
   * — means nothing to unbind and nothing to leak when a poll redraws the list.
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
   * What the tab is currently showing. Covers the loading and error states too, or a failed
   * load would fingerprint the same as "no data yet" and never paint its message.
   */
  proto._activityFingerprint = function () {
    if (this._activityError) return `error:${this._activityError}`;
    if (!this._activity) return "loading";

    const events = this._activity.events || [];
    const states = this._activity.state_history || [];
    const last = events.at(-1);
    return [
      events.length,
      events.filter((event) => event.state === "pending").length,
      last ? `${activityEventKey(last)}:${last.state}` : "",
      states.length,
      this._activity.positions?.length ?? 0,
    ].join("|");
  };
}
