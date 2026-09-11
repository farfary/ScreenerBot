// The status strip and the totals row above the wallet list.
import { fixed, plural, pct, seconds, signedSol, toneClass } from "./format.js";

export function renderStrip(page) {
  const { $, state } = page;
  const status = state.overview?.status;
  const node = $("#copy-system-state");
  const button = $("#copy-global-action");
  if (!node || !button) return;
  if (!status) {
    node.textContent = state.loadError ? "Unavailable" : "Loading";
    node.dataset.state = "unknown";
    button.disabled = true;
    return;
  }
  let label;
  let tone;
  if (!status.enabled) {
    label = "Paused globally · nothing is copied";
    tone = "paused";
  } else if (status.blocked_reason === "force_stop") {
    label = "Force stopped · nothing is copied";
    tone = "blocked";
  } else if (status.blocked_reason === "loss_limit") {
    label = "Loss limit · new entries blocked, exits still run";
    tone = "blocked";
  } else if (!status.live_tasks && !status.paper_tasks) {
    label = status.total_tasks
      ? `Idle · ${plural(status.total_tasks, "task")} paused`
      : "Idle · no tasks yet";
    tone = "idle";
  } else {
    const parts = [];
    if (status.live_tasks) parts.push(`${status.live_tasks} live`);
    parts.push(`${status.paper_tasks} paper`);
    label = `Processing · ${parts.join(" · ")}`;
    tone = "running";
  }
  node.textContent = label;
  node.dataset.state = tone;
  button.textContent = status.enabled ? "Pause all" : "Resume processing";
  button.disabled = !status.total_tasks;
}

function figure(label, value, { tone = "", note = "", extra = "" } = {}, escapeHtml) {
  return `<div class="copy-figure"><span class="copy-figure-label">${escapeHtml(label)}</span><strong class="copy-figure-value ${tone}">${escapeHtml(value)}</strong>${extra}<span class="copy-figure-note">${escapeHtml(note)}</span></div>`;
}

export function renderFigures(page) {
  const { $, Utils, state } = page;
  const root = $("#copy-figures");
  const totals = state.overview?.totals;
  if (!root || !totals) return;
  const esc = Utils.escapeHtml;
  const rounds = totals.wins + totals.losses;
  const budget = Number(totals.active_budget_sol) || 0;
  const spent = Number(totals.active_spent_sol) || 0;
  const budgetPct = budget > 0 ? Math.min(100, (spent / budget) * 100) : 0;
  const arrival = totals.active_arrival || {};
  root.innerHTML = [
    figure(
      "Realized P&L",
      signedSol(totals.realized_pnl_sol),
      { tone: toneClass(totals.realized_pnl_sol), note: plural(rounds, "closed round") },
      esc
    ),
    figure(
      "Unrealized P&L",
      signedSol(totals.unrealized_pnl_sol),
      {
        tone: toneClass(totals.unrealized_pnl_sol),
        note: totals.unpriced_holdings
          ? `${plural(totals.unpriced_holdings, "holding")} without a price`
          : "Marked at the pool price",
      },
      esc
    ),
    figure(
      "Win rate",
      rounds ? pct(totals.win_rate_pct, 0) : "—",
      { note: `${totals.wins} won · ${totals.losses} lost` },
      esc
    ),
    figure("Open holdings", String(totals.open_holdings), { note: "Across all tasks" }, esc),
    figure(
      "Budget in use",
      budget > 0 ? `${fixed(spent, 2)} / ${fixed(budget, 2)} SOL` : "—",
      {
        note: budget > 0 ? "Enabled tasks only" : "No enabled tasks",
        extra: `<span class="copy-meter" aria-hidden="true"><span style="width:${budgetPct.toFixed(1)}%"></span></span>`,
      },
      esc
    ),
    figure(
      "Median arrival",
      seconds(arrival.median_ms),
      {
        note: arrival.samples
          ? `p95 ${seconds(arrival.p95_ms)} · ${arrival.samples} trades`
          : "No samples from enabled tasks",
      },
      esc
    ),
  ].join("");
}
