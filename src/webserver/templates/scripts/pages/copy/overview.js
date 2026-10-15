// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The Overview tab: results over a date range, the P&L curve, how rounds ended
// and why trades were skipped, the all-time book, and readiness for live.
import { formatNumber } from "../../core/format.js";
import { barList, pnlCurve } from "./charts.js";
import {
  RANGES,
  dateTime,
  definitionRows,
  duration,
  exitLabel,
  finite,
  fixed,
  modeLabel,
  price,
  pct,
  seconds,
  segmented,
  signedSol,
  signedSolCell,
  skipLabel,
  sol,
  solCell,
  solNumber,
  toneClass,
  unrealizedFigure,
} from "./format.js";
import { ensureIdentities, tokenWithMint } from "./tokens.js";

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
          total: solNumber(ws.total_budget_native, 3),
          remaining: solNumber(ws.remaining_budget_native, 3),
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

/**
 * The paper model as configured now, for a paper task only: the workspace's own settings,
 * then the defaults. A live task's figures are not produced by this model.
 */
export function paperModelCard(ws, defaults, esc, extraNote = "") {
  if (ws.mode === "live") return "";
  const execution = ws.paper_execution || {};
  const delay = finite(execution.landing_delay_ms ?? defaults?.paper_landing_delay_ms);
  const capacity = finite(execution.queue_capacity ?? defaults?.paper_queue_capacity);
  const rows = [
    delay === null ? null : [I18n.t("copy-paper-wait-label"), seconds(delay, { precise: true })],
    capacity === null ? null : [I18n.t("copy-paper-queue-label"), String(capacity)],
  ];
  const extra = extraNote ? `<p class="copy-note">${esc(extraNote)}</p>` : "";
  return `<section class="copy-card"><h4>${esc(I18n.t("copy-paper-model"))}</h4><p class="copy-note">${esc(I18n.t("copy-paper-model-note"))}</p>${extra}<dl class="copy-defs">${definitionRows(rows, esc)}</dl></section>`;
}

/** Read-only legacy estimates: labeled columns, values at the last saved price, no actions. */
function legacyCard(ws, esc) {
  const legacy = ws.legacy_paper || {};
  const holdings = legacy.holdings || [];
  if (ws.mode === "live" || (!holdings.length && !(legacy.spent_native > 0))) return "";
  ensureIdentities(holdings.map((holding) => holding.mint));
  const head = [
    ["", I18n.t("copy-holdings-col-token")],
    ["num", I18n.t("copy-paper-legacy-col-tokens")],
    ["num", I18n.t("copy-paper-legacy-col-cost")],
    ["num", I18n.t("copy-paper-legacy-col-realized")],
    ["num", I18n.t("copy-paper-legacy-col-price")],
    ["num", I18n.t("copy-paper-legacy-col-remaining")],
  ]
    .map(([cls, label]) => `<th scope="col"${cls ? ` class="${cls}"` : ""}>${esc(label)}</th>`)
    .join("");
  const rows = holdings
    .map((holding) => {
      const held = holding.closed_at == null;
      const saved = holding.last_recorded_price_native;
      const savedAt = holding.last_recorded_price_at;
      return `<tr>
        <td>${tokenWithMint(holding.mint)}</td>
        <td class="num">${esc(formatNumber(holding.token_amount, 4))}</td>
        <td class="num">${esc(solCell(holding.remaining_cost_basis_native))}</td>
        <td class="num"><span class="${toneClass(holding.realized_pnl_native)}">${esc(signedSolCell(holding.realized_pnl_native))}</span></td>
        <td class="num copy-cell-stack">${esc(saved == null ? "—" : price(saved))}<small>${esc(savedAt ? dateTime(savedAt) : "")}</small></td>
        <td class="num">${
          held && holding.remaining_pnl_at_last_recorded_price_native != null
            ? `<span class="${toneClass(holding.remaining_pnl_at_last_recorded_price_native)}">${esc(signedSolCell(holding.remaining_pnl_at_last_recorded_price_native))}</span>`
            : "—"
        }</td>
      </tr>`;
    })
    .join("");
  const table = holdings.length
    ? `<div class="copy-table-wrap"><table class="copy-table"><thead><tr>${head}</tr></thead><tbody>${rows}</tbody></table></div>`
    : "";
  return `<section class="copy-card"><h4>${esc(I18n.t("copy-paper-legacy-title"))}</h4><p class="copy-note">${esc(I18n.t("copy-paper-legacy-note"))}</p><div class="copy-metrics copy-metrics--compact">${[
    metric(I18n.t("copy-paper-legacy-spent"), sol(legacy.spent_native, 3), "", "", esc),
    metric(
      I18n.t("copy-paper-legacy-realized"),
      signedSol(legacy.realized_pnl_native),
      "",
      toneClass(legacy.realized_pnl_native),
      esc
    ),
  ].join("")}</div>${table}</section>`;
}

export function renderOverview({ ws, insights, error, range, defaults }, esc) {
  return `${rangeHead(I18n.t("copy-overview-results"), range, esc)}${insightsBody(insights, error, esc, (data) => results(data, esc))}
    ${paperModelCard(ws, defaults, esc)}${legacyCard(ws, esc)}<div class="copy-split">${book(ws, esc)}${readiness(ws, esc)}</div>`;
}
