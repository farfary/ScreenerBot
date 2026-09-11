// The Overview tab: results over a date range, the P&L curve, how rounds ended
// and why trades were skipped, the all-time book, and readiness for live.
import { barList, pnlCurve } from "./charts.js";
import {
  EXIT_LABELS,
  RANGES,
  duration,
  fixed,
  plural,
  pct,
  segmented,
  signedSol,
  skipLabel,
  sol,
  toneClass,
} from "./format.js";

export function metric(label, value, note, tone, esc) {
  return `<div class="copy-metric"><span class="copy-metric-label">${esc(label)}</span><strong class="copy-metric-value ${tone || ""}">${esc(value)}</strong><span class="copy-metric-note">${esc(note || "")}</span></div>`;
}

export function rangeHead(title, range, esc) {
  return `<div class="copy-panel-head"><h3>${esc(title)}</h3>${segmented("range", RANGES, range, esc, "Date range")}</div>`;
}

export function panelMessage(text, esc, tone = "") {
  return `<div class="copy-panel-message ${tone}" role="status">${esc(text)}</div>`;
}

/** Results body shared by the tabs that read insights: data, error or loading. */
export function insightsBody(insights, error, esc, render) {
  if (insights) return render(insights);
  if (error) return panelMessage(`Analytics could not be loaded: ${error}`, esc, "is-error");
  return panelMessage("Loading analytics…", esc);
}

function results(insights, esc) {
  const rounds = insights.rounds;
  const exits = (insights.exit_breakdown || []).map((bucket) => ({
    label: EXIT_LABELS[bucket.exit] || bucket.exit,
    value: bucket.pnl_sol,
    display: `${bucket.legs}× · ${signedSol(bucket.pnl_sol, 3)}`,
    tone: toneClass(bucket.pnl_sol),
  }));
  const skips = (insights.skip_breakdown || []).map((bucket) => ({
    label: skipLabel(bucket.key),
    value: bucket.count,
    display: String(bucket.count),
  }));
  return `<div class="copy-metrics">${[
    metric(
      "Realized P&L",
      signedSol(insights.realized_pnl_sol),
      plural(rounds, "closed round"),
      toneClass(insights.realized_pnl_sol),
      esc
    ),
    metric(
      "Win rate",
      rounds ? pct(insights.win_rate_pct, 0) : "—",
      `${insights.wins} won · ${insights.losses} lost`,
      "",
      esc
    ),
    metric(
      "Average win",
      signedSol(insights.average_win_sol),
      `Average loss ${signedSol(insights.average_loss_sol)}`,
      toneClass(insights.average_win_sol),
      esc
    ),
    metric("Profit factor", fixed(insights.profit_factor, 2), "Gross wins ÷ gross losses", "", esc),
    metric("Average hold", duration(insights.average_hold_seconds), "Entry to exit", "", esc),
    metric(
      "Best round",
      signedSol(insights.best_round_sol),
      `Worst ${signedSol(insights.worst_round_sol)}`,
      toneClass(insights.best_round_sol),
      esc
    ),
  ].join("")}</div>
  <section class="copy-card"><h4>Cumulative P&amp;L</h4>${pnlCurve(insights.pnl_curve, { escapeHtml: esc })}</section>
  <div class="copy-split">
    <section class="copy-card"><h4>How rounds ended</h4>${barList(exits, esc)}</section>
    <section class="copy-card"><h4>Why trades were skipped</h4>${barList(skips, esc)}</section>
  </div>`;
}

function book(ws, esc) {
  const stats = ws.stats || {};
  const title = stats.book === "live" ? "Live book" : "Paper book";
  const split = [
    [stats.filled_buys, "buys"],
    [stats.policy_exits, "exits by your rules"],
    [stats.target_sells, "wallet sells"],
    [stats.manual_closes, "closed by hand"],
    [stats.skipped, "skipped"],
    [stats.failed, "failed"],
  ]
    .map(
      ([count, label]) => `<span><strong>${esc(String(count ?? 0))}</strong> ${esc(label)}</span>`
    )
    .join("");
  return `<section class="copy-card"><h4>${esc(title)} <span class="copy-card-sub">All time</span></h4>
    <div class="copy-metrics copy-metrics--compact">${[
      metric(
        "Unrealized P&L",
        signedSol(stats.unrealized_pnl_sol),
        stats.unpriced_positions
          ? `${plural(stats.unpriced_positions, "holding")} without a pool price`
          : plural(stats.open_positions ?? 0, "open holding"),
        toneClass(stats.unrealized_pnl_sol),
        esc
      ),
      metric(
        "Realized P&L",
        signedSol(stats.realized_pnl_sol),
        `${stats.closed_positions ?? 0} closed`,
        toneClass(stats.realized_pnl_sol),
        esc
      ),
      metric(
        "Budget used",
        sol(ws.spent_sol, 3),
        `of ${fixed(ws.total_budget_sol, 3)} · ${fixed(ws.remaining_budget_sol, 3)} left`,
        "",
        esc
      ),
    ].join("")}</div>
    <p class="copy-decision-split">${split}</p>
  </section>`;
}

function readiness(ws, esc) {
  const checks = ws.readiness?.checks || [];
  const items = checks
    .map(
      (check) =>
        `<li class="copy-check ${check.passed ? "is-passed" : "is-failed"}"><i class="${check.passed ? "icon-circle-check" : "icon-circle-x"}" aria-hidden="true"></i><span><strong>${esc(check.label)}</strong><small>${esc(check.detail)}</small></span><span class="sr-only">${check.passed ? "passed" : "not passed"}</span></li>`
    )
    .join("");
  const footer =
    ws.mode === "live"
      ? '<p class="copy-note">This task trades live. Return it to Paper from the header above.</p>'
      : `<div class="copy-readiness-foot"><span>${esc(ws.readiness?.ready ? "Every check passes." : "Arming needs an explicit review of what is not ready.")}</span><button class="btn btn-outline btn-sm" type="button" data-ws-action="arm">Review and arm live</button></div>`;
  return `<section class="copy-card"><h4>Before going live</h4><ul class="copy-checks">${items}</ul>${footer}</section>`;
}

export function renderOverview({ ws, insights, error, range }, esc) {
  return `${rangeHead("Results", range, esc)}${insightsBody(insights, error, esc, (data) => results(data, esc))}
    <div class="copy-split">${book(ws, esc)}${readiness(ws, esc)}</div>`;
}
