// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The Overview tab: results over a date range, the P&L curve, how rounds ended
// and why trades were skipped, the all-time book, and readiness for live.
import { barList, pnlCurve } from "./charts.js";
import {
  RANGES,
  duration,
  exitLabel,
  fixed,
  modeLabel,
  pct,
  segmented,
  signedSol,
  skipLabel,
  sol,
  toneClass,
  unrealizedFigure,
} from "./format.js";

export function metric(label, value, note, tone, esc) {
  return `<div class="copy-metric"><span class="copy-metric-label">${esc(label)}</span><strong class="copy-metric-value ${tone || ""}">${esc(value)}</strong><span class="copy-metric-note">${esc(note || "")}</span></div>`;
}

export function rangeHead(title, range, esc) {
  return `<div class="copy-panel-head"><h3>${esc(title)}</h3>${segmented("range", RANGES, range, esc, I18n.attr("copy-range-label", "aria-label"))}</div>`;
}

export function panelMessage(text, esc, tone = "") {
  return `<div class="copy-panel-message ${tone}" role="status">${esc(text)}</div>`;
}

/** Results body shared by the tabs that read insights: data, error or loading. */
export function insightsBody(insights, error, esc, render) {
  if (insights) return render(insights);
  if (error) {
    return panelMessage(I18n.t("copy-analytics-load-failed", { error }), esc, "is-error");
  }
  return panelMessage(I18n.t("copy-analytics-loading"), esc);
}

/** The readiness checklist of a task: one row per check with its pass state. */
export function readinessChecks(ws, esc) {
  const items = (ws.readiness?.checks || [])
    .map(
      (check) =>
        `<li class="copy-check ${check.passed ? "is-passed" : "is-failed"}"><i class="${check.passed ? "icon-circle-check" : "icon-circle-x"}" aria-hidden="true"></i><span><strong>${esc(I18n.text(check.text))}</strong><small>${esc(I18n.text(check.detail))}</small></span><span class="sr-only">${esc(check.passed ? I18n.t("copy-check-passed") : I18n.t("copy-check-not-passed"))}</span></li>`
    )
    .join("");
  return `<ul class="copy-checks">${items}</ul>`;
}

function results(insights, esc) {
  const rounds = insights.rounds;
  const exits = (insights.exit_breakdown || []).map((bucket) => ({
    label: exitLabel(bucket.exit),
    value: bucket.pnl_sol,
    display: I18n.t("copy-exit-bucket", {
      count: bucket.legs,
      pnl: signedSol(bucket.pnl_sol),
    }),
    tone: toneClass(bucket.pnl_sol),
  }));
  const skips = (insights.skip_breakdown || []).map((bucket) => ({
    label: skipLabel(bucket.key),
    value: bucket.count,
    display: String(bucket.count),
  }));
  return `<div class="copy-metrics">${[
    metric(
      I18n.t("copy-metric-realized-pnl"),
      signedSol(insights.realized_pnl_native),
      I18n.t("copy-count-closed-rounds", { count: rounds }),
      toneClass(insights.realized_pnl_native),
      esc
    ),
    metric(
      I18n.t("copy-metric-win-rate"),
      rounds ? pct(insights.win_rate_pct, 0) : "—",
      I18n.t("copy-record-won-lost", { won: insights.wins, lost: insights.losses }),
      "",
      esc
    ),
    metric(
      I18n.t("copy-overview-average-win"),
      signedSol(insights.average_win_native),
      I18n.t("copy-overview-average-loss", { amount: signedSol(insights.average_loss_native) }),
      toneClass(insights.average_win_native),
      esc
    ),
    metric(
      I18n.t("copy-overview-profit-factor"),
      fixed(insights.profit_factor, 2),
      I18n.t("copy-overview-profit-factor-note"),
      "",
      esc
    ),
    metric(
      I18n.t("copy-overview-average-hold"),
      duration(insights.average_hold_seconds),
      I18n.t("copy-overview-average-hold-note"),
      "",
      esc
    ),
    metric(
      I18n.t("copy-overview-best-round"),
      signedSol(insights.best_round_native),
      I18n.t("copy-overview-worst-round", { amount: signedSol(insights.worst_round_native) }),
      toneClass(insights.best_round_native),
      esc
    ),
  ].join("")}</div>
  <section class="copy-card"><h4>${esc(I18n.t("copy-overview-curve-title"))}</h4>${pnlCurve(insights.pnl_curve, { escapeHtml: esc })}</section>
  <div class="copy-split">
    <section class="copy-card"><h4>${esc(I18n.t("copy-overview-exits-title"))}</h4>${barList(exits, esc)}</section>
    <section class="copy-card"><h4>${esc(I18n.t("copy-overview-skips-title"))}</h4>${barList(skips, esc)}</section>
  </div>`;
}

function book(ws, esc) {
  const stats = ws.stats || {};
  const title =
    stats.book === "live" ? I18n.t("copy-book-title-live") : I18n.t("copy-book-title-paper");
  const split = [
    ["copy-book-buys", stats.filled_buys],
    ["copy-book-policy-exits", stats.policy_exits],
    ["copy-book-wallet-sells", stats.target_sells],
    ["copy-book-manual-closes", stats.manual_closes],
    ["copy-book-skipped", stats.skipped],
    ["copy-book-failed", stats.failed],
  ]
    .map(([id, count]) => `<span>${I18n.markup(id, { count: count ?? 0 })}</span>`)
    .join("");
  const marked = unrealizedFigure(
    stats.unrealized_pnl_native,
    stats.open_positions,
    stats.unpriced_positions
  );
  return `<section class="copy-card"><h4>${esc(title)} <span class="copy-card-sub">${esc(I18n.t("copy-book-all-time"))}</span></h4>
    <div class="copy-metrics copy-metrics--compact">${[
      metric(
        I18n.t("copy-metric-unrealized-pnl"),
        signedSol(marked.value),
        marked.note || I18n.t("copy-count-open-holdings", { count: stats.open_positions ?? 0 }),
        toneClass(marked.value),
        esc
      ),
      metric(
        I18n.t("copy-metric-realized-pnl"),
        signedSol(stats.realized_pnl_native),
        I18n.t("copy-book-closed", { count: stats.closed_positions ?? 0 }),
        toneClass(stats.realized_pnl_native),
        esc
      ),
      metric(
        I18n.t("copy-metric-budget-spent"),
        sol(ws.spent_native, 3),
        I18n.t("copy-book-budget-note", {
          mode: modeLabel(ws.mode),
          total: fixed(ws.total_budget_native, 3),
          remaining: fixed(ws.remaining_budget_native, 3),
        }),
        "",
        esc
      ),
    ].join("")}</div>
    <p class="copy-decision-split">${split}</p>
  </section>`;
}

function readiness(ws, esc) {
  const footer =
    ws.mode === "live"
      ? `<p class="copy-note">${esc(I18n.t("copy-readiness-live-note"))}</p>`
      : `<div class="copy-readiness-foot"><span>${esc(ws.readiness?.ready ? I18n.t("copy-readiness-all-pass") : I18n.t("copy-readiness-needs-review"))}</span><button class="btn btn-outline btn-sm" type="button" data-ws-action="arm">${esc(I18n.t("copy-readiness-arm"))}</button></div>`;
  return `<section class="copy-card"><h4>${esc(I18n.t("copy-readiness-title"))}</h4>${readinessChecks(ws, esc)}${footer}</section>`;
}

export function renderOverview({ ws, insights, error, range }, esc) {
  return `${rangeHead(I18n.t("copy-overview-results"), range, esc)}${insightsBody(insights, error, esc, (data) => results(data, esc))}
    <div class="copy-split">${book(ws, esc)}${readiness(ws, esc)}</div>`;
}
