// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Activity section of the Position Details dialog: the token's whole history in this wallet,
 * set under the chart. Several positions on one token are grouped into trading rounds; a token
 * traded once shows its events directly. Wallet transactions that never belonged to a
 * position get a group of their own.
 */
import * as Utils from "../../core/utils.js";
import { requestManager } from "../../core/request_manager.js";
import { POSITION_STATUS_LABELS } from "../position_status.js";
import { activityEventKey, renderActivityCard } from "./activity_event.js";

// State reasons written by src/positions; other stored reasons render as stored.
const POSITION_STATE_REASON_LABELS = Object.freeze({
  position_created: "positions-state-reason-position-created",
});

// `PositionState` (src/positions/database/types.rs) as written to the state history.
const POSITION_STATE_LABELS = Object.freeze({
  Open: "positions-state-open",
  Closing: "positions-state-closing",
  Closed: "positions-state-closed",
  ExitPending: "positions-state-exit-pending",
  ExitFailed: "positions-state-exit-failed",
  Phantom: "positions-state-phantom",
  Reconciling: "positions-state-reconciling",
});

const esc = (text) => Utils.escapeHtml(text);

// The activity endpoint walks every swap and wallet transaction the token ever had, so it is
// not refetched on the details tick: only when the position's own trades moved, and otherwise
// at this slower cadence while the position is still open.
const ACTIVITY_REFRESH_MS = 30000;
// After a failed read, wait this long before the next details tick may try again.
const ACTIVITY_RETRY_MS = 10000;

const dateOnly = (ts) => Utils.formatDate(new Date(ts * 1000));

export function applyActivityMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

  /**
   * Drop everything the section held. The view state (filter, expanded events, open rounds)
   * lives here rather than in the DOM, so a repaint restores it.
   */
  proto._resetActivityState = function () {
    this._activity = null;
    this._activityLoading = false;
    this._activityError = null;
    this._activityFetchedAt = 0;
    this._activityTrigger = null;
    this._activityRenderedFp = null;
    this._activityFilter = "all";
    this._activityExpanded = new Set();
    this._activityOpenRounds = new Set();
    this._activityRoundsInitialized = false;
  };

  /** Decide, after each details response, whether the activity needs a fresh read. */
  proto._syncActivity = function () {
    const details = this.fullDetails;
    const pos = details?.position;
    if (!pos) return;

    const trigger = [
      details.entries?.length ?? 0,
      details.exits?.length ?? 0,
      details.pending_swaps?.length ?? 0,
      pos.status,
      pos.transaction_exit_verified,
    ].join("|");
    const sinceFetch = Date.now() - this._activityFetchedAt;
    const due = this._activity
      ? trigger !== this._activityTrigger || (pos.status === "open" && sinceFetch > ACTIVITY_REFRESH_MS)
      : sinceFetch > ACTIVITY_RETRY_MS;

    if (due) {
      this._activityTrigger = trigger;
      this._fetchActivity();
    }
  };

  proto._fetchActivity = async function () {
    if (this._activityLoading || !this.positionData) return;
    this._activityLoading = true;
    this._activityFetchedAt = Date.now();
    const seq = this._openSeq;

    try {
      const data = await requestManager.fetch(`/api/positions/${this._getPositionKey()}/activity`, {
        priority: "normal",
      });
      if (seq !== this._openSeq) return;
      this._activity = data;
      this._activityError = null;
    } catch (error) {
      console.error("Error loading token activity:", error);
      if (seq === this._openSeq) this._activityError = I18n.t("positions-activity-load-failed");
    } finally {
      this._activityLoading = false;
    }

    if (seq === this._openSeq) this._paintActivity();
  };

  /**
   * The pane's frame (header strip, scroll body) is static markup in the dialog; only these
   * slots repaint, so the list keeps its scroll position and the header its pane controls.
   */
  proto._paintActivity = function () {
    const section = this.dialogEl?.querySelector("#pddActivity");
    if (!section) return;
    const fingerprint = this._activityFingerprint();
    if (fingerprint === this._activityRenderedFp) return;
    this._activityRenderedFp = fingerprint;

    const { meta, filters, body } = this._buildActivity();
    const fill = (selector, html) => {
      const el = section.querySelector(selector);
      if (el) el.innerHTML = html;
    };
    fill("#pddActivityMeta", meta);
    fill("#pddActivityFilters", filters);
    fill("#pddActivityContent", body);
    this._applyActivityFilter(section);
  };

  /** @returns {{meta: string, filters: string, body: string}} */
  proto._buildActivity = function () {
    const parts = (body, meta = "", filters = "") => ({ body, meta, filters });
    const notice = (icon, text) =>
      `<div class="pdd-activity-notice"><i class="${icon}"></i><span>${Utils.escapeHtml(text)}</span></div>`;

    if (!this._activity) {
      return parts(
        this._activityError
          ? notice("icon-circle-alert", this._activityError)
          : `<div class="loading-spinner">${esc(I18n.t("positions-activity-loading"))}</div>`
      );
    }

    const events = this._activity.events || [];
    const totals = this._activity.totals || {};
    const positions = this._activity.positions || [];
    const stateHistory = this._activity.state_history || [];

    if (!events.length && !stateHistory.length) {
      return parts(notice("icon-activity", I18n.t("positions-activity-empty")));
    }

    const currentPositionId = this._position()?.id ?? null;
    const ctx = {
      symbol: this._activity.symbol || this._position()?.symbol || I18n.t("positions-event-tokens-fallback"),
      solPriceUsd: this._activity.native_price_usd || null,
      expanded: this._activityExpanded,
      formatPrice: (price) => this._formatPrice(price),
      formatSol: (value, options) => this._formatSol(value, options),
      toneClass: (value) => this._toneClass(value),
      currentPositionId,
    };

    this._initializeActivityRounds(positions, currentPositionId);

    return parts(
      this._buildActivityTotals(totals, positions) +
        this._buildActivityTimeline(positions, events, stateHistory, ctx),
      this._activityMeta(totals, events, positions),
      this._buildActivityFilters(totals, events)
    );
  };

  proto._initializeActivityRounds = function (positions, currentPositionId) {
    if (this._activityRoundsInitialized) return;
    this._activityRoundsInitialized = true;

    const current = positions.find((position) => position.id === currentPositionId);
    const initial = current || positions.at(-1);
    if (initial) this._activityOpenRounds.add(`position:${initial.id || initial.index}`);
  };

  proto._activityMeta = function (totals, events, positions) {
    const timestamps = events.map((event) => event.timestamp).filter(Number.isFinite);
    const parts = [];
    if (positions.length > 1) {
      parts.push(I18n.t("positions-activity-round-count", { count: positions.length }));
    }
    parts.push(I18n.t("positions-activity-event-count", { count: totals.events || events.length }));
    if (timestamps.length) {
      const first = dateOnly(Math.min(...timestamps));
      const last = dateOnly(Math.max(...timestamps));
      parts.push(first === last ? first : `${first} – ${last}`);
    }

    const alerts = [];
    if (totals.pending) {
      alerts.push(I18n.t("positions-activity-pending-count", { count: totals.pending }));
    }
    if (totals.failed) {
      alerts.push(I18n.t("positions-activity-failed-count", { count: totals.failed }));
    }
    const alertHtml = alerts.length
      ? ` · <span class="pdd-activity-alert">${esc(alerts.join(" · "))}</span>`
      : "";

    return `${esc(parts.join(" · "))}${alertHtml}`;
  };

  proto._buildActivityFilters = function (totals, events) {
    const issueCount = events.filter((event) =>
      ["pending", "failed", "synthetic"].includes(event.state)
    ).length;
    // A filter that can only show nothing is not offered.
    const filters = [
      ["all", I18n.t("positions-filter-all"), totals.events || 0],
      ["trades", I18n.t("positions-filter-trades"), (totals.entries || 0) + (totals.exits || 0)],
      ["entry", I18n.t("positions-filter-buys"), totals.entries || 0],
      ["exit", I18n.t("positions-filter-sells"), totals.exits || 0],
      ["wallet", I18n.t("positions-filter-wallet"), totals.wallet_events || 0],
      ["issues", I18n.t("positions-filter-issues"), issueCount],
    ].filter(([id, , count]) => id === "all" || count > 0);

    if (!filters.some(([id]) => id === this._activityFilter)) this._activityFilter = "all";

    return `
      <div class="timeframe-buttons pdd-act-filters" role="group" aria-label="${esc(I18n.attr("positions-activity-filters", "aria-label"))}">
        ${filters
          .map(([id, label, count]) => {
            const active = this._activityFilter === id;
            return `<button type="button" class="timeframe-btn pdd-act-filter${active ? " active" : ""}" data-filter="${id}" aria-pressed="${active}">${esc(label)}<span>${count}</span></button>`;
          })
          .join("")}
      </div>`;
  };

  /** Token-wide totals, shown only when there is more than the current position to add up. */
  proto._buildActivityTotals = function (totals, positions) {
    if (positions.length < 2) return "";
    const realized = totals.realized_pnl || 0;
    const item = (label, value, tone = "") =>
      `<div class="pdd-act-total"><span>${esc(label)}</span><strong class="${tone}">${value}</strong></div>`;

    return `
      <div class="pdd-act-totals" aria-label="${esc(I18n.attr("positions-activity-totals", "aria-label"))}">
        ${item(I18n.t("positions-activity-realized-all"), this._formatSol(realized, { sign: true }), this._toneClass(realized))}
        ${item(I18n.t("positions-activity-invested"), this._formatSol(totals.native_invested))}
        ${item(I18n.t("positions-activity-returned"), this._formatSol(totals.native_returned))}
        ${item(I18n.t("positions-summary-network-fees"), this._formatSol(totals.network_fees_native))}
      </div>`;
  };

  proto._roundEvents = function (position, events, stateHistory) {
    return {
      events: events.filter(
        (event) =>
          event.position_id === position.id ||
          (event.position_id == null && event.position_index === position.index)
      ),
      states: stateHistory.filter(
        (state) => state.position_id === position.id || state.position_index === position.index
      ),
    };
  };

  proto._buildRoundItems = function (events, stateHistory, ctx) {
    return [
      ...events.map((event) => ({ timestamp: event.timestamp || 0, event })),
      ...stateHistory.map((state) => ({ timestamp: state.changed_at || 0, state })),
    ]
      .sort((a, b) => b.timestamp - a.timestamp)
      .map((item) =>
        item.event ? renderActivityCard(item.event, ctx) : this._buildStateMilestone(item.state)
      )
      .join("");
  };

  proto._buildActivityTimeline = function (positions, events, stateHistory, ctx) {
    const walletEvents = events.filter((event) => event.side === "wallet");

    // One position and nothing outside it: a round header would only repeat the summary.
    if (positions.length === 1 && !walletEvents.length) {
      const [position] = positions;
      const round = this._roundEvents(position, events, stateHistory);
      return `
        <div class="pdd-act-list">
          <section class="pdd-act-round is-single is-open" data-round="position:${position.id || position.index}">
            <div class="pdd-act-round-body">${this._buildRoundItems(round.events, round.states, ctx)}</div>
          </section>
        </div>`;
    }

    const rounds = [...positions]
      .reverse()
      .map((position) => {
        const round = this._roundEvents(position, events, stateHistory);
        return this._buildActivityRound(position, round.events, round.states, ctx);
      });
    if (walletEvents.length) rounds.push(this._buildWalletActivity(walletEvents, ctx));

    return `<div class="pdd-act-list">${rounds.join("")}</div>`;
  };

  proto._buildActivityRound = function (position, events, stateHistory, ctx) {
    const key = `position:${position.id || position.index}`;
    const isOpen = this._activityOpenRounds.has(key);
    const isCurrent = position.id === ctx.currentPositionId;
    // Archived first: an archived round that never exited still reports `is_open`.
    const status = position.archived ? "archived" : position.is_open ? "open" : "closed";
    const statusLabel = I18n.label(POSITION_STATUS_LABELS, status);
    const pnl = position.realized_pnl || 0;
    const when = (ts) => Utils.formatTimestamp(ts, { includeSeconds: false });
    const dates = position.closed_at
      ? `${when(position.opened_at)} – ${when(position.closed_at)}`
      : I18n.t("positions-activity-opened", { when: when(position.opened_at) });
    const fact = (label, value, tone = "") =>
      `<span class="pdd-act-round-fact"><small>${esc(label)}</small><strong class="${tone}">${value}</strong></span>`;

    return `
      <section class="pdd-act-round${isOpen ? " is-open" : ""}${isCurrent ? " is-current" : ""}" data-round="${key}">
        <button type="button" class="pdd-act-round-toggle" data-round-toggle="${key}" aria-expanded="${isOpen}">
          <span class="pdd-act-round-main">
            <span class="pdd-act-round-title">
              ${esc(I18n.t("positions-activity-round-title", { index: position.index }))}
              ${isCurrent ? `<span class="pdd-act-tag is-current">${esc(I18n.t("positions-activity-this-position"))}</span>` : ""}
              <span class="pdd-act-tag is-${status}">${esc(statusLabel)}</span>
            </span>
            <span class="pdd-act-round-date">${esc(dates)} · ${esc(I18n.t("positions-activity-event-count", { count: position.swaps }))}</span>
          </span>
          <span class="pdd-act-round-facts">
            ${fact(I18n.t("positions-activity-invested"), this._formatSol(position.native_invested || 0))}
            ${fact(I18n.t("positions-activity-returned"), this._formatSol(position.native_returned || 0))}
            <span class="pdd-act-round-fact is-pnl"><small>${esc(I18n.t("positions-fact-realized"))}</small><strong class="${this._toneClass(pnl)}">${this._formatSol(pnl, { sign: true })}</strong></span>
            <i class="icon-chevron-down"></i>
          </span>
        </button>
        <div class="pdd-act-round-body">${this._buildRoundItems(events, stateHistory, ctx)}</div>
      </section>`;
  };

  proto._buildWalletActivity = function (events, ctx) {
    const key = "wallet";
    const isOpen = this._activityOpenRounds.has(key);
    const timestamps = events.map((event) => event.timestamp).filter(Number.isFinite);
    const range = timestamps.length
      ? `${dateOnly(Math.min(...timestamps))} – ${dateOnly(Math.max(...timestamps))}`
      : I18n.t("positions-activity-dates-unavailable");

    return `
      <section class="pdd-act-round is-wallet${isOpen ? " is-open" : ""}" data-round="${key}">
        <button type="button" class="pdd-act-round-toggle" data-round-toggle="${key}" aria-expanded="${isOpen}">
          <span class="pdd-act-round-main">
            <span class="pdd-act-round-title">${esc(I18n.t("positions-activity-wallet-title"))}</span>
            <span class="pdd-act-round-date">${esc(I18n.t("positions-activity-outside", { range, count: events.length }))}</span>
          </span>
          <span class="pdd-act-round-facts"><i class="icon-chevron-down"></i></span>
        </button>
        <div class="pdd-act-round-body">${this._buildRoundItems(events, [], ctx)}</div>
      </section>`;
  };

  proto._buildStateMilestone = function (state) {
    const normalized = String(state.state || "state").toLowerCase();
    return `
      <div class="pdd-act-milestone" data-side="state" data-state="${Utils.escapeHtml(normalized)}">
        <i class="pdd-act-glyph icon-history" aria-hidden="true"></i>
        <span class="pdd-act-milestone-main">
          <strong>${esc(I18n.label(POSITION_STATE_LABELS, state.state))}</strong>
          ${state.reason ? `<span>${Utils.escapeHtml(I18n.label(POSITION_STATE_REASON_LABELS, state.reason))}</span>` : ""}
        </span>
        <time title="${Utils.formatTimestamp(state.changed_at)}">${Utils.formatTimestamp(state.changed_at, { includeSeconds: false })}</time>
      </div>`;
  };

  proto._applyActivityFilter = function (section) {
    const filter = this._activityFilter;
    const matches = (item) => {
      const side = item.dataset.side;
      const state = item.dataset.state;
      if (filter === "all") return true;
      if (filter === "trades") return side === "entry" || side === "exit" || side === "state";
      if (filter === "issues") {
        return (
          ["pending", "failed", "synthetic"].includes(state) ||
          state?.includes("fail") ||
          state?.includes("pending")
        );
      }
      return side === filter;
    };

    section.querySelectorAll(".pdd-act-card, .pdd-act-milestone").forEach((item) => {
      item.classList.toggle("is-filtered-out", !matches(item));
    });

    let visibleGroups = 0;
    section.querySelectorAll(".pdd-act-round").forEach((round) => {
      const visible = round.querySelectorAll(
        ".pdd-act-card:not(.is-filtered-out), .pdd-act-milestone:not(.is-filtered-out)"
      ).length;
      round.classList.toggle("is-filtered-out", visible === 0);
      if (visible > 0) visibleGroups += 1;
    });

    const list = section.querySelector(".pdd-act-list");
    if (!list) return;
    let empty = list.querySelector(".pdd-act-filter-empty");
    if (visibleGroups === 0 && !empty) {
      empty = document.createElement("div");
      empty.className = "pdd-act-filter-empty";
      empty.textContent = I18n.t("positions-activity-filter-empty");
      list.appendChild(empty);
    } else if (visibleGroups > 0 && empty) {
      empty.remove();
    }
  };

  /** Bound once per dialog on the section, which survives every repaint of its content. */
  proto._bindActivityHandlers = function () {
    const section = this.dialogEl?.querySelector("#pddActivity");
    if (!section) return;

    section.addEventListener("click", (event) => {
      const copyEl = event.target.closest("[data-copy]");
      if (copyEl) {
        event.preventDefault();
        event.stopPropagation();
        Utils.copyToClipboard(copyEl.dataset.copy);
        Utils.notifyCopied(I18n.t("positions-details-signature-label"));
        return;
      }

      const roundBtn = event.target.closest(".pdd-act-round-toggle");
      if (roundBtn) {
        const round = roundBtn.closest(".pdd-act-round");
        this._setRoundOpen(round, !round.classList.contains("is-open"));
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
        if (label) label.firstChild.textContent = open
          ? I18n.t("positions-event-hide-details")
          : I18n.t("positions-event-details");
        if (open) this._activityExpanded.add(key);
        else this._activityExpanded.delete(key);
        return;
      }

      const filterBtn = event.target.closest(".pdd-act-filter");
      if (filterBtn) this._setActivityFilter(filterBtn.dataset.filter);
    });
  };

  proto._setActivityFilter = function (filter) {
    const section = this.dialogEl?.querySelector("#pddActivity");
    if (!section) return;
    this._activityFilter = filter;
    section.querySelectorAll(".pdd-act-filter").forEach((button) => {
      const active = button.dataset.filter === filter;
      button.classList.toggle("active", active);
      button.setAttribute("aria-pressed", String(active));
    });
    this._applyActivityFilter(section);
  };

  /** Open or fold a round, remembered so a repaint keeps it that way. */
  proto._setRoundOpen = function (round, open) {
    const key = round.dataset.round;
    round.classList.toggle("is-open", open);
    round.querySelector(".pdd-act-round-toggle")?.setAttribute("aria-expanded", String(open));
    if (open) this._activityOpenRounds.add(key);
    else this._activityOpenRounds.delete(key);
  };

  proto._activityFingerprint = function () {
    if (!this._activity) return this._activityError ? `error:${this._activityError}` : "loading";

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
