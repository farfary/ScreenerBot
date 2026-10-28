// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
//! The Trader Stats tab's figures: the book and window switches, the exposure chips, the realized figures, the P&L and outcome charts and the breakdown tables of one book.

import { $, paintHtml } from "../../core/dom.js";
import { formatFixed, signedToneClass } from "../../core/format.js";
import { histogram, pnlBars } from "../../ui/inline_chart.js";
import { renderSegmented, RANGE_OPTIONS } from "../../ui/segmented.js";
import { renderStateView } from "../../ui/state_view.js";
import { renderSummaryChip } from "../../ui/table_toolbar.js";
import { TRADING_MODE_ICONS, tradingModeLabel } from "../../ui/trading_mode.js";
import { createPaperAccount } from "./paper_account.js";
import { createStatsTables } from "./stats_tables.js";

const DASH = "—";

/** Chip variant of a signed amount's tone as shown. */
const TONE_VARIANTS = Object.freeze({ positive: "success", negative: "error" });

/**
 * The Stats view of one book. The handler aggregates every figure; this view only
 * formats what `/api/trader/stats` returns, so an absent figure renders as a dash and a
 * count as its number. `controls` paints the state chips and the notices, and is told
 * about load failures; `switchTab` opens another Trader tab.
 */
export function createStatsView({
  requestManager,
  Utils,
  DataTable,
  ConfirmationDialog,
  controls,
  switchTab,
}) {
  const esc = Utils.escapeHtml;
  const tables = createStatsTables({ DataTable, Utils, emptyText });
  const paperAccount = createPaperAccount({
    requestManager,
    ConfirmationDialog,
    onReset: () => void refresh(),
  });

  // The book and the realized window the tab reads. The two books are never summed.
  let book = "live";
  let statsWindow = "30d";
  // The last response of the current book and window, `null` until it arrives.
  let data = null;
  let account = null;
  let status = null;
  // Each request carries a number; a response that a later switch made stale is dropped.
  let requestSeq = 0;
  const painted = new Map();
  const cleanups = [];

  /** Run `paint(input)` only when its input differs from the last painted one. */
  function guarded(key, input, paint) {
    const hash = JSON.stringify(input);
    if (painted.get(key) === hash) return;
    painted.set(key, hash);
    paint(input);
  }

  function listen(element, event, handler) {
    if (!element) return;
    element.addEventListener(event, handler);
    cleanups.push(() => element.removeEventListener(event, handler));
  }

  const sol = (value) => Utils.formatSol(value, { decimals: 4, fallback: DASH });
  const signedSol = (value) => Utils.formatSignedSol(value, { fallback: DASH });
  // The arguments of a plural message: `count` selects the form, `amount` is shown.
  const counted = (value) => ({ count: value, amount: String(value) });
  const record = (wins, losses) =>
    I18n.t("trader-stats-record", {
      wins: I18n.t("trader-stats-wins", counted(wins)),
      losses: I18n.t("trader-stats-losses", counted(losses)),
    });
  const hold = (hours, style) =>
    Number.isFinite(hours) ? Utils.formatUptime(hours * 3600, { style }) : DASH;

  /** The message of an empty window: the last day, the last N days, or the whole history. */
  function emptyText(periodDays) {
    if (periodDays == null) return I18n.t("trader-stats-empty-all");
    if (periodDays === 1) return I18n.t("trader-stats-empty-day");
    return I18n.t("trader-stats-empty-days", counted(periodDays));
  }

  // ---------------------------------------------------------------------------
  // Toolbar
  // ---------------------------------------------------------------------------

  function paintSwitches() {
    paintHtml(
      $("#stats-book"),
      renderSegmented(
        {
          name: "stats-book",
          options: ["live", "paper"].map((mode) => ({
            value: mode,
            label: tradingModeLabel(mode),
            icon: TRADING_MODE_ICONS[mode],
          })),
          value: book,
          ariaLabel: I18n.attr("trader-stats-book", "aria-label"),
          compact: true,
        },
        esc
      )
    );
    paintHtml(
      $("#stats-period"),
      renderSegmented(
        {
          name: "stats-period",
          options: RANGE_OPTIONS,
          value: statsWindow,
          ariaLabel: I18n.attr("trader-stats-window", "aria-label"),
          compact: true,
        },
        esc
      )
    );
  }

  function exposureChips({ stats, paper, cash }) {
    if (!stats) return "";
    const open = stats.open_positions_count;
    const idle = open === 0;
    const chips = [
      renderSummaryChip({
        id: "open",
        label: I18n.t("trader-exposure-open"),
        value: `${open} / ${stats.max_open_positions}`,
        format: "text",
        tooltip: I18n.t("trader-exposure-open-title"),
      }),
      renderSummaryChip({
        id: "in-use",
        label: I18n.t("trader-exposure-in-use"),
        value: Utils.formatSol(stats.locked_native, { fallback: DASH }),
        format: "text",
        tooltip: I18n.t("trader-exposure-in-use-title"),
        hidden: idle,
      }),
      renderSummaryChip({
        id: "unrealized",
        label: I18n.t("trader-exposure-unrealized"),
        value: signedSol(stats.unrealized_native),
        format: "text",
        variant: TONE_VARIANTS[Utils.signedTone(stats.unrealized_native, 4)],
        tooltip:
          stats.open_unpriced > 0
            ? I18n.t("trader-exposure-unpriced", counted(stats.open_unpriced))
            : "",
        hidden: idle,
      }),
    ];
    if (paper) {
      chips.push(
        renderSummaryChip({
          id: "cash",
          label: I18n.t("trader-exposure-cash"),
          value: sol(cash?.cash_native),
          format: "text",
          tooltip: cash
            ? I18n.t("trader-exposure-cash-title", {
                date: Utils.formatDate(cash.started_at, { fallback: DASH }),
                amount: sol(cash.starting_balance_native),
              })
            : "",
        })
      );
    }
    return chips.join("");
  }

  function paintToolbar() {
    const paper = book === "paper";
    const reset = $("#stats-paper-reset");
    if (reset) reset.hidden = !paper;
    // The results below name the paper book, and the loss limit, which governs live
    // rounds only, steps out while it is shown.
    const marker = $("#stats-book-marker");
    if (marker) {
      marker.hidden = !paper;
      if (paper) marker.textContent = tradingModeLabel("paper");
    }
    const lossLimit = $("#loss-limit-panel");
    if (lossLimit) lossLimit.hidden = paper;
    guarded(
      "exposure",
      {
        stats: data && {
          open_positions_count: data.open_positions_count,
          max_open_positions: data.max_open_positions,
          locked_native: data.locked_native,
          unrealized_native: data.unrealized_native,
          open_unpriced: data.open_unpriced,
        },
        paper,
        cash: paper ? account : null,
      },
      (input) => paintHtml($("#stats-exposure"), exposureChips(input))
    );
  }

  // ---------------------------------------------------------------------------
  // Figures
  // ---------------------------------------------------------------------------

  function setValue(id, text, tone = "neutral") {
    const el = $(`#${id}`);
    if (!el) return;
    el.textContent = text;
    el.classList.toggle("positive", tone === "positive");
    el.classList.toggle("negative", tone === "negative");
    if (el.classList.contains("metric-value-fit")) {
      el.style.setProperty("--metric-value-chars", String([...text].length));
    }
  }

  function setDetail(id, text) {
    const el = $(`#${id}`);
    if (el) el.textContent = text;
  }

  /** The eight cards. Without a closed trade only the count is a fact; one empty state says why. */
  function paintFigures(stats) {
    const has = Boolean(stats) && stats.total_trades > 0;
    const pct = (value) => Utils.formatPercentValue(value, { decimals: 1, fallback: DASH });

    setValue(
      "net-pnl",
      has ? signedSol(stats.total_pnl_native) : DASH,
      has ? Utils.signedTone(stats.total_pnl_native, 4) : "neutral"
    );
    setDetail(
      "net-pnl-detail",
      has
        ? I18n.t("trader-stats-won-lost", {
            won: sol(stats.gross_profit_native),
            lost: sol(stats.gross_loss_native),
          })
        : ""
    );

    setValue(
      "win-rate",
      has
        ? Utils.formatPercentValue(stats.win_rate_pct, {
            decimals: 1,
            fallback: DASH,
            includeSign: false,
          })
        : DASH
    );
    setDetail("win-rate-detail", has ? record(stats.winners, stats.losers) : "");

    setValue(
      "profit-factor",
      has ? formatFixed(stats.profit_factor, { decimals: 2, fallback: DASH }) : DASH
    );
    setDetail(
      "profit-factor-detail",
      !has
        ? ""
        : Number.isFinite(stats.expectancy_native)
          ? I18n.t("trader-stats-expected", { amount: signedSol(stats.expectancy_native) })
          : I18n.t("trader-stats-profit-factor-basis")
    );

    // An unsigned magnitude: never toned, never accented.
    setValue("max-drawdown", has ? sol(stats.max_drawdown_native) : DASH);
    setDetail("max-drawdown-detail", has ? I18n.t("trader-stats-drawdown-basis") : "");

    const hasAverage =
      has && (Number.isFinite(stats.avg_win_pct) || Number.isFinite(stats.avg_loss_pct));
    setValue(
      "avg-win-loss",
      hasAverage ? `${pct(stats.avg_win_pct)} / ${pct(stats.avg_loss_pct)}` : DASH
    );
    setDetail(
      "avg-win-loss-detail",
      has && Number.isFinite(stats.payoff_ratio)
        ? I18n.t("trader-stats-payoff", {
            ratio: formatFixed(stats.payoff_ratio, { decimals: 2, fallback: DASH }),
          })
        : ""
    );

    setValue("total-trades", stats ? String(stats.total_trades) : DASH);
    const streak = has ? stats.current_streak : null;
    setDetail(
      "total-trades-detail",
      !streak
        ? ""
        : streak.outcome === "win"
          ? I18n.t("trader-stats-streak-win", counted(streak.length))
          : I18n.t("trader-stats-streak-loss", counted(streak.length))
    );

    setValue("median-hold", has ? hold(stats.median_hold_time_hours) : DASH);
    setDetail(
      "median-hold-detail",
      has
        ? I18n.t("trader-stats-hold-split", {
            wins: hold(stats.median_win_hold_hours, "trimmed"),
            losses: hold(stats.median_loss_hold_hours, "trimmed"),
          })
        : ""
    );

    setValue("fees-paid", has ? sol(stats.fees_native) : DASH);
    setDetail("fees-paid-detail", has ? I18n.t("trader-stats-fees-basis") : "");

    // Rounds without a complete cost basis carry no honest P&L and are left out of every
    // figure; saying so separates a filtered number from a wrong one.
    const excluded = $("#stats-excluded");
    if (excluded) {
      const n = stats?.excluded_untrusted || 0;
      excluded.hidden = n === 0;
      excluded.textContent = n === 0 ? "" : I18n.t("trader-stats-excluded", counted(n));
    }
  }

  // ---------------------------------------------------------------------------
  // States
  // ---------------------------------------------------------------------------

  /** The loading, empty and breakdown sections: exactly one of them shows. */
  function paintStates(input) {
    const loading = $("#stats-loading");
    const empty = $("#stats-empty");
    const breakdowns = $("#stats-breakdowns");
    if (loading) {
      loading.hidden = input.kind !== "loading";
      if (!loading.hidden) {
        paintHtml(
          loading,
          renderStateView({
            kind: "loading",
            message: I18n.t("trader-stats-loading"),
            compact: true,
          })
        );
      }
    }
    if (empty) {
      empty.hidden = input.kind !== "empty";
      if (!empty.hidden) {
        paintHtml(
          empty,
          renderStateView({
            kind: "empty",
            icon: "icon-inbox",
            title: emptyText(input.periodDays),
            message: input.openStrategies ? "" : I18n.t("trader-stats-empty-hint"),
            compact: true,
            action: input.openStrategies
              ? { id: "open-strategies", label: I18n.t("trader-open-strategies") }
              : null,
          })
        );
      }
    }
    if (breakdowns) breakdowns.hidden = input.kind !== "data";
  }

  function stateInput() {
    // Before the first response, or after a failed first load (the notice says why).
    if (!data) return { kind: painted.has("loaded") ? "none" : "loading" };
    if (data.total_trades > 0) return { kind: "data" };
    return {
      kind: "empty",
      periodDays: data.period_days ?? null,
      openStrategies: book === "paper" && status?.paper_entry_idle?.kind === "no_entry_strategy",
    };
  }

  // ---------------------------------------------------------------------------
  // Charts
  // ---------------------------------------------------------------------------

  /** Point a heading at another message, localizing it only when the id changes. */
  function setMessage(element, id) {
    if (!element || element.getAttribute("data-l10n-id") === id) return;
    element.setAttribute("data-l10n-id", id);
    I18n.localizeTree(element);
  }

  function paintPnl({ days, periodDays }) {
    const daily = days[0]?.bucket !== "week";
    setMessage($("#stats-pnl-title"), daily ? "trader-daily-title" : "trader-weekly-title");
    setMessage(
      $("#stats-pnl-subtitle"),
      daily ? "trader-daily-subtitle" : "trader-weekly-subtitle"
    );
    const points = days.map((day) => {
      // Buckets start on UTC calendar dates ("2026-08-31").
      const at = Date.parse(`${day.date}T00:00:00Z`);
      const args = {
        date: Utils.formatDate(at, { utc: true, fallback: DASH }),
        pnl: signedSol(day.net_pnl_native),
        ...counted(day.trades),
      };
      return {
        at,
        value: day.net_pnl_native,
        trades: day.trades,
        title: daily ? I18n.t("trader-daily-bar", args) : I18n.t("trader-weekly-bar", args),
      };
    });
    paintHtml(
      $("#daily-pnl"),
      pnlBars(points, {
        label: daily ? I18n.t("trader-daily-chart") : I18n.t("trader-weekly-chart"),
        empty: emptyText(periodDays),
        escapeHtml: esc,
      })
    );
  }

  /** The edge of an outcome bucket as a signed whole percent. */
  const edge = (value) => Utils.formatPercentValue(value, { decimals: 0, fallback: DASH });

  function bucketRange(bucket) {
    if (bucket.lower_pct == null)
      return I18n.t("trader-outcome-below", { value: edge(bucket.upper_pct) });
    if (bucket.upper_pct == null)
      return I18n.t("trader-outcome-above", { value: edge(bucket.lower_pct) });
    return I18n.t("trader-outcome-range", {
      from: edge(bucket.lower_pct),
      to: edge(bucket.upper_pct),
    });
  }

  /**
   * The tone of a bucket's returns. The edges include 0, so every return in a bucket has
   * one sign, which the middle of its range (or its one edge, for an open bucket) carries.
   */
  function bucketTone(bucket) {
    const middle =
      bucket.lower_pct == null
        ? bucket.upper_pct
        : bucket.upper_pct == null
          ? bucket.lower_pct
          : (bucket.lower_pct + bucket.upper_pct) / 2;
    return signedToneClass(middle, 1);
  }

  function paintOutcomes({ buckets, periodDays }) {
    // Each column names its lower edge as a percent, the first its upper bound; the full
    // range is the column's tooltip and accessible name.
    const columns = buckets.map((bucket) => ({
      label: bucket.lower_pct == null ? `<${edge(bucket.upper_pct)}` : edge(bucket.lower_pct),
      count: bucket.count,
      tone: bucketTone(bucket),
      title: I18n.t("trader-outcome-bucket", {
        range: bucketRange(bucket),
        pnl: signedSol(bucket.net_pnl_native),
        ...counted(bucket.count),
      }),
    }));
    paintHtml(
      $("#outcome-distribution"),
      histogram(columns, {
        escapeHtml: esc,
        empty: emptyText(periodDays),
        label: I18n.t("trader-outcome-subtitle"),
      })
    );
  }

  function paintExtremes(stats) {
    const side = (tokenId, valueId, token, value) => {
      const tokenEl = $(`#${tokenId}`);
      if (tokenEl) tokenEl.textContent = Number.isFinite(value) && token ? token : "";
      paintHtml(
        $(`#${valueId}`),
        Utils.formatPercent(value, { style: "pnl", decimals: 1, fallback: DASH })
      );
    };
    side("best-trade-token", "best-trade", stats.best_trade_token, stats.best_trade_pct);
    side("worst-trade-token", "worst-trade", stats.worst_trade_token, stats.worst_trade_pct);
    const runs = $("#longest-runs");
    if (runs) runs.textContent = record(stats.longest_win_streak, stats.longest_loss_streak);
  }

  // ---------------------------------------------------------------------------
  // Paint and load
  // ---------------------------------------------------------------------------

  function paint() {
    paintToolbar();
    guarded("figures", data, paintFigures);
    guarded("states", stateInput(), paintStates);
    if (!data || data.total_trades === 0) return;
    const periodDays = data.period_days ?? null;
    guarded("pnl", { days: data.daily_pnl || [], periodDays }, paintPnl);
    guarded("outcomes", { buckets: data.outcome_distribution || [], periodDays }, paintOutcomes);
    guarded(
      "extremes",
      {
        best_trade_token: data.best_trade_token,
        best_trade_pct: data.best_trade_pct,
        worst_trade_token: data.worst_trade_token,
        worst_trade_pct: data.worst_trade_pct,
        longest_win_streak: data.longest_win_streak,
        longest_loss_streak: data.longest_loss_streak,
      },
      paintExtremes
    );
    // The tables are created only now, with their section on screen, so they measure
    // their real width.
    tables.paint(data, book);
  }

  /** Read the shown book and window; the paper book also reads its run for the cash chip. */
  async function refresh() {
    const seq = ++requestSeq;
    const shownBook = book;
    try {
      const [stats, run] = await Promise.all([
        requestManager.fetch(`/api/trader/stats?window=${statsWindow}&book=${shownBook}`, {
          priority: "normal",
        }),
        shownBook === "paper" ? paperAccount.load() : null,
      ]);
      if (seq !== requestSeq) return;
      data = stats;
      account = run;
      painted.set("loaded", "1");
      controls.setLoadError(null);
    } catch (error) {
      if (seq !== requestSeq) return;
      console.error("[Trader] Failed to load stats:", error);
      // The last painted figures stay; the notice says they are not current.
      painted.set("loaded", "1");
      controls.setLoadError(error);
    }
    paint();
  }

  /** Show the new book or window as loading, so no figure of the previous one lingers. */
  function reload() {
    data = null;
    account = null;
    painted.delete("loaded");
    paint();
    void refresh();
  }

  return {
    /** Paint the switches and the loading state, and wire the tab's own controls. */
    mount() {
      paintSwitches();
      paint();
      listen($("#stats-book"), "change", (event) => {
        const next = event.target.value;
        if (next === book) return;
        book = next;
        reload();
        void controls.refresh();
      });
      listen($("#stats-period"), "change", (event) => {
        const next = event.target.value;
        if (next === statsWindow) return;
        statsWindow = next;
        reload();
      });
      listen($("#stats-paper-reset"), "click", () => void paperAccount.reset());
      listen($("#stats-empty"), "click", (event) => {
        if (event.target.closest('[data-action="open-strategies"]')) switchTab("strategies");
      });
      listen($("#stats-notices"), "click", (event) => {
        if (event.target.closest('[data-action="stats-retry"]')) void refresh();
      });
    },

    refresh,

    /** The book the tab shows, `live` or `paper`. */
    getBook: () => book,

    /** The latest `/api/trader/status`, which decides the empty state's action. */
    setStatus(next) {
      status = next;
      if (data) guarded("states", stateInput(), paintStates);
    },

    destroy() {
      cleanups.forEach((cleanup) => cleanup());
      cleanups.length = 0;
      tables.destroy();
      painted.clear();
      data = null;
      account = null;
      status = null;
      requestSeq += 1;
    },
  };
}
