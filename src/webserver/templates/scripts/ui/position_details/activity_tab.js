/**
 * Activity Tab Mixin for the Position Details dialog.
 *
 * Activity is organized into trading rounds. Each round is a self-contained lifecycle,
 * while transactions that never belonged to a position live in a separate wallet chapter.
 */
import * as Utils from "../../core/utils.js";
import { requestManager } from "../../core/request_manager.js";
import { activityEventKey, renderActivityCard } from "./activity_event.js";

export function applyActivityTabMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

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
    if (content && this.currentTab === "activity") this._paintActivity(content);
  };

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

    const currentPositionId = this.fullDetails?.position?.id ?? this.positionData?.id ?? null;
    const ctx = {
      symbol: this._activity.symbol || this.fullDetails?.position?.symbol || "tokens",
      solPriceUsd: this._activity.sol_price_usd || null,
      expanded: this._activityExpanded,
      formatPrice: (price) => this._formatPrice(price),
      currentPositionId,
    };

    this._initializeActivityRounds(positions, currentPositionId);

    content.innerHTML = `
      <div class="pdd-activity">
        ${this._buildActivitySummary(totals, events)}
        ${this._buildActivityToolbar(totals, events)}
        ${this._buildActivityTimeline(positions, events, stateHistory, ctx)}
      </div>`;

    this._applyActivityFilter(content);
    this._bindActivityHandlers(content);
  };

  proto._initializeActivityRounds = function (positions, currentPositionId) {
    if (this._activityRoundsInitialized) return;
    this._activityRoundsInitialized = true;

    const current = positions.find((position) => position.id === currentPositionId);
    const newest = positions.at(-1);
    const initial = current || newest;
    if (initial) this._activityOpenRounds.add(`position:${initial.id || initial.index}`);
  };

  proto._buildActivitySummary = function (totals, events) {
    const realized = totals.realized_pnl || 0;
    const realizedBasis = (totals.sol_returned || 0) - realized;
    const realizedPct = realizedBasis > 0 ? (realized / realizedBasis) * 100 : null;
    const sign = realized >= 0 ? "+" : "";
    const tone = realized >= 0 ? "is-positive" : "is-negative";
    const sol = (value, decimals = 4) =>
      `${Utils.formatSol(value || 0, { decimals, suffix: "" })} SOL`;

    const timestamps = events
      .map((event) => event.timestamp)
      .filter((timestamp) => Number.isFinite(timestamp));
    const first = timestamps.length ? Math.min(...timestamps) : null;
    const last = timestamps.length ? Math.max(...timestamps) : null;
    const range = first
      ? `${Utils.formatTimestamp(first, { includeSeconds: false })} – ${Utils.formatTimestamp(last, { includeSeconds: false })}`
      : "No dated events";

    const alerts = [];
    if (totals.pending) alerts.push(`${totals.pending} pending`);
    if (totals.failed) alerts.push(`${totals.failed} failed`);

    const item = (label, value, extra = "") => `
      <span class="pdd-act-summary-item">
        <span>${label}</span>
        <strong>${value}</strong>
        ${extra ? `<small>${extra}</small>` : ""}
      </span>`;

    return `
      <section class="pdd-act-summary">
        <div class="pdd-act-summary-lead">
          <span class="pdd-act-summary-eyebrow">All-time token history</span>
          <strong class="pdd-act-summary-pnl ${tone}">${sign}${sol(realized)}</strong>
          <span class="pdd-act-summary-caption">Realized P&amp;L${
            realizedPct !== null ? ` · ${sign}${Utils.formatNumber(realizedPct, 2)}%` : ""
          }</span>
        </div>
        <div class="pdd-act-summary-facts">
          ${item("Trading rounds", totals.positions || 0, `${totals.events || 0} events`)}
          ${item("Capital invested", sol(totals.sol_invested))}
          ${item("Capital returned", sol(totals.sol_returned))}
          ${item("Network fees", sol(totals.network_fees_sol, 6))}
        </div>
        <div class="pdd-act-summary-meta">
          <span><i class="icon-calendar"></i>${range}</span>
          ${
            alerts.length
              ? `<span class="pdd-act-summary-alert"><i class="icon-triangle-alert"></i>${alerts.join(" · ")}</span>`
              : ""
          }
        </div>
      </section>`;
  };

  proto._buildActivityToolbar = function (totals, events) {
    const tradeCount = (totals.entries || 0) + (totals.exits || 0);
    const issueCount = events.filter((event) =>
      ["pending", "failed", "synthetic"].includes(event.state)
    ).length;
    const filters = [
      ["all", "All", totals.events || 0],
      ["trades", "Trades", tradeCount],
      ["entry", "Entries", totals.entries || 0],
      ["exit", "Exits", totals.exits || 0],
    ];
    if (totals.wallet_events) filters.push(["wallet", "Wallet", totals.wallet_events]);
    if (issueCount) filters.push(["issues", "Issues", issueCount]);

    const available = new Set(filters.map(([id]) => id));
    if (!available.has(this._activityFilter)) this._activityFilter = "all";

    const buttons = filters
      .map(
        ([id, label, count]) => `
        <button type="button" class="pdd-act-filter${this._activityFilter === id ? " active" : ""}" data-filter="${id}">
          ${label}<span>${count}</span>
        </button>`
      )
      .join("");

    return `
      <div class="pdd-act-toolbar">
        <div class="pdd-act-filters" aria-label="Activity filters">${buttons}</div>
      </div>`;
  };

  proto._buildActivityTimeline = function (positions, events, stateHistory, ctx) {
    const orderedPositions = [...positions].reverse();

    const rounds = orderedPositions.map((position) => {
      const roundEvents = events.filter(
        (event) =>
          event.position_id === position.id ||
          (event.position_id == null && event.position_index === position.index)
      );
      const roundStates = stateHistory.filter(
        (state) => state.position_id === position.id || state.position_index === position.index
      );
      return this._buildActivityRound(position, roundEvents, roundStates, ctx);
    });

    const walletEvents = events.filter((event) => event.side === "wallet");
    if (walletEvents.length) rounds.push(this._buildWalletActivity(walletEvents, ctx));

    return `<div class="pdd-act-list">${rounds.join("")}</div>`;
  };

  proto._buildActivityRound = function (position, events, stateHistory, ctx) {
    const key = `position:${position.id || position.index}`;
    const isOpen = this._activityOpenRounds.has(key);
    const isCurrent = position.id === ctx.currentPositionId;
    const status = position.is_open ? "Open" : position.archived ? "Archived" : "Closed";
    const pnl = position.realized_pnl || 0;
    const sign = pnl >= 0 ? "+" : "";
    const duration = position.closed_at
      ? `${Utils.formatTimestamp(position.opened_at, { includeSeconds: false })} – ${Utils.formatTimestamp(position.closed_at, { includeSeconds: false })}`
      : `Opened ${Utils.formatTimestamp(position.opened_at, { includeSeconds: false })}`;

    const items = [
      ...events.map((event) => ({ type: "event", timestamp: event.timestamp || 0, event })),
      ...stateHistory.map((state) => ({
        type: "state",
        timestamp: state.changed_at || 0,
        state,
      })),
    ]
      .sort((a, b) => b.timestamp - a.timestamp)
      .map((item) =>
        item.type === "event"
          ? renderActivityCard(item.event, ctx)
          : this._buildStateMilestone(item.state)
      )
      .join("");

    return `
      <section class="pdd-act-round${isOpen ? " is-open" : ""}${isCurrent ? " is-current" : ""}" data-round="${key}">
        <button type="button" class="pdd-act-round-toggle" data-round-toggle="${key}" aria-expanded="${isOpen}">
          <span class="pdd-act-round-index">${position.index}</span>
          <span class="pdd-act-round-main">
            <span class="pdd-act-round-title">
              Position ${position.index}
              ${isCurrent ? '<span class="pdd-act-current-badge">Current</span>' : ""}
              <span class="pdd-act-round-status is-${status.toLowerCase()}">${status}</span>
            </span>
            <span class="pdd-act-round-date">${duration}</span>
          </span>
          <span class="pdd-act-round-facts">
            <span><small>Invested</small>${Utils.formatSol(position.sol_invested || 0, { decimals: 4 })}</span>
            <span><small>Returned</small>${Utils.formatSol(position.sol_returned || 0, { decimals: 4 })}</span>
            <strong class="${pnl >= 0 ? "is-positive" : "is-negative"}">${sign}${Utils.formatSol(pnl, { decimals: 4 })}</strong>
            <span class="pdd-act-round-count">${position.swaps} event${position.swaps === 1 ? "" : "s"}</span>
            <i class="icon-chevron-down"></i>
          </span>
        </button>
        <div class="pdd-act-round-body">${items}</div>
      </section>`;
  };

  proto._buildWalletActivity = function (events, ctx) {
    const key = "wallet";
    const isOpen = this._activityOpenRounds.has(key);
    const first = events.at(0)?.timestamp;
    const last = events.at(-1)?.timestamp;
    const range = first
      ? `${Utils.formatTimestamp(first, { includeSeconds: false })} – ${Utils.formatTimestamp(last, { includeSeconds: false })}`
      : "Dates unavailable";
    const items = [...events]
      .sort((a, b) => (b.timestamp || 0) - (a.timestamp || 0))
      .map((event) => renderActivityCard(event, ctx))
      .join("");

    return `
      <section class="pdd-act-round is-wallet${isOpen ? " is-open" : ""}" data-round="${key}">
        <button type="button" class="pdd-act-round-toggle" data-round-toggle="${key}" aria-expanded="${isOpen}">
          <span class="pdd-act-round-index"><i class="icon-wallet"></i></span>
          <span class="pdd-act-round-main">
            <span class="pdd-act-round-title">Wallet-only activity</span>
            <span class="pdd-act-round-date">Transactions outside ScreenerBot · ${range}</span>
          </span>
          <span class="pdd-act-round-facts">
            <span class="pdd-act-round-count">${events.length} event${events.length === 1 ? "" : "s"}</span>
            <i class="icon-chevron-down"></i>
          </span>
        </button>
        <div class="pdd-act-round-body">${items}</div>
      </section>`;
  };

  proto._buildStateMilestone = function (state) {
    const normalized = String(state.state || "state").toLowerCase();
    return `
      <div class="pdd-act-milestone" data-side="state" data-state="${Utils.escapeHtml(normalized)}">
        <span class="pdd-act-milestone-node"><i class="icon-history"></i></span>
        <span class="pdd-act-milestone-main">
          <strong>Position ${Utils.escapeHtml(state.state)}</strong>
          ${state.reason ? `<span>${Utils.escapeHtml(state.reason)}</span>` : ""}
        </span>
        <time title="${Utils.formatTimestamp(state.changed_at)}">${Utils.formatTimestamp(state.changed_at, { includeSeconds: false })}</time>
      </div>`;
  };

  proto._applyActivityFilter = function (content) {
    const filter = this._activityFilter;
    const matches = (item) => {
      const side = item.dataset.side;
      const state = item.dataset.state;
      if (filter === "all") return true;
      if (filter === "trades") return side === "entry" || side === "exit" || side === "state";
      if (filter === "issues")
        return (
          ["pending", "failed", "synthetic"].includes(state) ||
          state?.includes("fail") ||
          state?.includes("pending")
        );
      return side === filter;
    };

    content.querySelectorAll(".pdd-act-card, .pdd-act-milestone").forEach((item) => {
      item.classList.toggle("is-filtered-out", !matches(item));
    });

    let visibleGroups = 0;
    content.querySelectorAll(".pdd-act-round").forEach((round) => {
      const visible = round.querySelectorAll(
        ".pdd-act-card:not(.is-filtered-out), .pdd-act-milestone:not(.is-filtered-out)"
      ).length;
      round.classList.toggle("is-filtered-out", visible === 0);
      if (visible > 0) visibleGroups += 1;
    });

    const list = content.querySelector(".pdd-act-list");
    if (!list) return;
    let empty = list.querySelector(".pdd-act-filter-empty");
    if (visibleGroups === 0 && !empty) {
      empty = document.createElement("div");
      empty.className = "pdd-act-filter-empty";
      empty.textContent = "No activity matches this filter";
      list.appendChild(empty);
    } else if (visibleGroups > 0 && empty) {
      empty.remove();
    }
  };

  proto._bindActivityHandlers = function (content) {
    if (this._activityClickHandler) return;

    this._activityClickHandler = (event) => {
      const copyEl = event.target.closest("[data-copy]");
      if (copyEl) {
        event.preventDefault();
        event.stopPropagation();
        Utils.copyToClipboard(copyEl.dataset.copy);
        Utils.showToast("Signature copied", "success");
        return;
      }

      const roundBtn = event.target.closest(".pdd-act-round-toggle");
      if (roundBtn) {
        const key = roundBtn.dataset.roundToggle;
        const round = roundBtn.closest(".pdd-act-round");
        const open = !round.classList.contains("is-open");
        round.classList.toggle("is-open", open);
        roundBtn.setAttribute("aria-expanded", String(open));
        if (open) this._activityOpenRounds.add(key);
        else this._activityOpenRounds.delete(key);
        return;
      }

      const expandBtn = event.target.closest(".pdd-act-expand");
      if (expandBtn) {
        const key = expandBtn.dataset.expand;
        const card = expandBtn.closest(".pdd-act-card");
        const open = !card.classList.contains("is-open");
        card.classList.toggle("is-open", open);
        expandBtn.setAttribute("aria-expanded", String(open));
        const label = expandBtn.querySelector(".pdd-act-details-label");
        if (label) label.firstChild.textContent = open ? "Hide details" : "Details";
        if (open) this._activityExpanded.add(key);
        else this._activityExpanded.delete(key);
        return;
      }

      const filterBtn = event.target.closest(".pdd-act-filter");
      if (filterBtn) {
        this._activityFilter = filterBtn.dataset.filter;
        content
          .querySelectorAll(".pdd-act-filter")
          .forEach((button) => button.classList.toggle("active", button === filterBtn));
        this._applyActivityFilter(content);
        return;
      }

    };

    content.addEventListener("click", this._activityClickHandler);
  };

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
