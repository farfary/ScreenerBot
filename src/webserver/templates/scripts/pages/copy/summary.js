// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The status strip and the totals row above the wallet list.
import { pct, pnlTone, seconds, signedSol, solNumber, unrealizedFigure } from "./format.js";
import { metricCard } from "../../ui/metric_card.js";
import { gateControl, setupRequired } from "../../ui/setup_gate.js";

export function renderStrip(page) {
  const { $, state } = page;
  const status = state.overview?.status;
  const node = $("#copy-system-state");
  const button = $("#copy-global-action");
  if (!node || !button) return;
  const taskCount = state.overview?.tasks?.length ?? 0;
  // The bulk action exists only while there is a task to act on, in every mode.
  button.hidden = taskCount === 0;
  if (setupRequired()) {
    // Without tasks the setup notice below says it all; the strip repeats nothing.
    node.hidden = taskCount === 0;
    node.textContent = I18n.t("copy-strip-setup-required");
    node.dataset.state = "paused";
    gateControl(button);
    return;
  }
  node.hidden = false;
  if (!status) {
    node.textContent = state.loadError
      ? I18n.t("copy-strip-unavailable")
      : I18n.t("copy-strip-loading");
    node.dataset.state = "unknown";
    button.disabled = true;
    return;
  }
  let label;
  let tone;
  if (!status.total_tasks) {
    // Nothing to pause or process: the global switch has no effect to report yet.
    label = I18n.t("copy-strip-idle-empty");
    tone = "idle";
  } else if (!status.enabled) {
    label = I18n.t("copy-strip-paused-globally");
    tone = "paused";
  } else if (status.blocked_reason === "force_stop") {
    label = I18n.t("copy-strip-force-stopped");
    tone = "blocked";
  } else if (status.blocked_reason === "loss_limit") {
    label = I18n.t("copy-strip-loss-limit");
    tone = "blocked";
  } else if (!status.live_tasks && !status.paper_tasks) {
    label = I18n.t("copy-strip-idle-paused", { count: status.total_tasks });
    tone = "idle";
  } else {
    label = status.live_tasks
      ? I18n.t("copy-strip-processing-live", {
          live: status.live_tasks,
          paper: status.paper_tasks,
        })
      : I18n.t("copy-strip-processing", { paper: status.paper_tasks });
    tone = "running";
  }
  node.textContent = label;
  node.dataset.state = tone;
  button.textContent = status.enabled
    ? I18n.t("copy-strip-pause-all")
    : I18n.t("copy-strip-resume");
  // Pausing everything is offered only while a task copies; resuming, while any task exists.
  const copying = status.live_tasks || status.paper_tasks;
  button.disabled = status.enabled ? !copying : !status.total_tasks;
}

export function renderFigures(page) {
  const { $, Utils, state } = page;
  const root = $("#copy-figures");
  const totals = state.overview?.totals;
  if (!root || !totals) return;
  const esc = Utils.escapeHtml;
  const rounds = totals.wins + totals.losses;
  const budget = Number(totals.active_budget_native) || 0;
  const spent = Number(totals.active_spent_native) || 0;
  const arrival = totals.active_arrival || {};
  const marked = unrealizedFigure(
    totals.unrealized_pnl_native,
    totals.open_holdings,
    totals.unpriced_holdings
  );
  root.innerHTML = [
    metricCard(
      {
        label: I18n.t("copy-metric-realized-pnl"),
        value: signedSol(totals.realized_pnl_native),
        detail: I18n.t("copy-count-closed-rounds", { count: rounds }),
        tone: pnlTone(totals.realized_pnl_native),
      },
      esc
    ),
    metricCard(
      {
        label: I18n.t("copy-metric-unrealized-pnl"),
        value: signedSol(marked.value),
        detail: marked.note || I18n.t("copy-figure-marked-at-pool"),
        tone: pnlTone(marked.value),
      },
      esc
    ),
    metricCard(
      {
        label: I18n.t("copy-metric-win-rate"),
        value: rounds ? pct(totals.win_rate_pct, 0) : "—",
        detail: I18n.t("copy-record-won-lost", { won: totals.wins, lost: totals.losses }),
      },
      esc
    ),
    metricCard(
      {
        label: I18n.t("copy-metric-open-holdings"),
        value: String(totals.open_holdings),
        detail: I18n.t("copy-figure-across-tasks"),
      },
      esc
    ),
    metricCard(
      {
        label: I18n.t("copy-metric-budget-spent"),
        value:
          budget > 0
            ? I18n.t("copy-budget-of", { spent: solNumber(spent, 2), budget: solNumber(budget, 2) })
            : "—",
        detail:
          budget > 0 ? I18n.t("copy-figure-budget-lifetime") : I18n.t("copy-figure-budget-none"),
        fit: true,
      },
      esc
    ),
    metricCard(
      {
        label: I18n.t("copy-metric-median-arrival"),
        value: seconds(arrival.median_ms),
        detail: arrival.samples
          ? I18n.t("copy-figure-arrival-samples", {
              p95: seconds(arrival.p95_ms),
              count: arrival.samples,
            })
          : I18n.t("copy-figure-arrival-none"),
      },
      esc
    ),
  ].join("");
}
