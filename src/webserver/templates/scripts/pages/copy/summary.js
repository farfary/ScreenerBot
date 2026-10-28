// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The page toolbar (state and summary chips, the one bulk action, Settings and Add
// wallet), the page notices under it, and the all-tasks figures above the wallet list.
import { pct, pnlTone, seconds, signedSol, solNumber, unrealizedFigure } from "./format.js";
import { metricCard } from "../../ui/metric_card.js";
import { renderNotice } from "../../ui/notice.js";
import { renderSummaryChip } from "../../ui/table_toolbar.js";
import { tradingModeLabel } from "../../ui/trading_mode.js";
import { gateControl, setupRequired } from "../../ui/setup_gate.js";

/** Message key of each page state the status chip names. */
const STATUS_LABELS = Object.freeze({
  loading: "copy-status-loading",
  unavailable: "copy-status-unavailable",
  setup_required: "copy-status-setup-required",
  paused: "copy-status-paused",
  force_stopped: "copy-status-force-stopped",
  loss_limit: "copy-status-loss-limit",
  all_paused: "copy-status-all-paused",
  processing: "copy-status-processing",
});

/** Chip tone of each page state. */
const STATUS_VARIANTS = Object.freeze({
  loading: "secondary",
  unavailable: "error",
  setup_required: "warning",
  paused: "warning",
  force_stopped: "error",
  loss_limit: "warning",
  all_paused: "warning",
  processing: "success",
});

/** The page notice of a state that stops copying, as variant, title and message text. */
const STATE_NOTICES = Object.freeze({
  paused: {
    variant: "warning",
    title: () => I18n.t("copy-notice-paused-title"),
    message: () => I18n.t("copy-notice-paused-message"),
  },
  force_stopped: {
    variant: "error",
    title: () => I18n.t("copy-notice-force-stopped-title"),
    message: () => I18n.t("copy-notice-force-stopped-message"),
  },
  loss_limit: {
    variant: "warning",
    title: () => I18n.t("copy-notice-loss-limit-title"),
    message: () => I18n.t("copy-notice-loss-limit-message"),
  },
});

/** The page state, in the precedence the status chip and the state notice share. */
function pageState(state) {
  if (!state.overview) return state.loadError ? "unavailable" : "loading";
  const status = state.overview.status || {};
  if (setupRequired()) return "setup_required";
  if (!status.enabled) return "paused";
  if (status.blocked_reason === "force_stop") return "force_stopped";
  if (status.blocked_reason === "loss_limit") return "loss_limit";
  if (!status.active_tasks) return "all_paused";
  return "processing";
}

function chipsHtml(state, kind) {
  const status = state.overview?.status;
  const taskCount = state.overview?.tasks?.length ?? 0;
  // A loaded book without a task has nothing to summarize: the onboarding card or the
  // setup notice speaks instead.
  if (state.overview && !taskCount) return "";
  const chips = [
    renderSummaryChip({
      id: "status",
      label: I18n.t("copy-toolbar-status"),
      value: I18n.label(STATUS_LABELS, kind),
      variant: STATUS_VARIANTS[kind],
    }),
  ];
  if (!status) return chips.join("");
  chips.push(
    renderSummaryChip({
      id: "active",
      label: I18n.t("copy-toolbar-active"),
      value: I18n.t("copy-toolbar-active-value", {
        active: status.active_tasks,
        total: status.total_tasks,
      }),
    })
  );
  if (status.live_tasks > 0) {
    chips.push(
      renderSummaryChip({
        id: "live",
        label: tradingModeLabel("live"),
        value: String(status.live_tasks),
      })
    );
  }
  if (status.paper_tasks > 0) {
    chips.push(
      renderSummaryChip({
        id: "paper",
        label: tradingModeLabel("paper"),
        value: String(status.paper_tasks),
      })
    );
  }
  return chips.join("");
}

function noticesHtml(state, kind) {
  const notices = [];
  if (state.loadError) {
    notices.push(
      renderNotice({
        variant: "error",
        message: I18n.t("copy-load-failed", { error: state.loadError }),
        action: { id: "retry", label: I18n.t("common-action-retry") },
      })
    );
  }
  const notice = STATE_NOTICES[kind];
  if (notice && state.overview?.tasks?.length) {
    notices.push(
      renderNotice({
        variant: notice.variant,
        title: notice.title(),
        message: notice.message(),
      })
    );
  }
  return notices.join("");
}

/**
 * Paints the toolbar and the page notices. At most one bulk action shows: Pause copying
 * while a task copies, Resume copying while processing is off, and Resume all tasks while
 * processing is on, no task copies and a paused task can be resumed.
 */
export function renderToolbar(page) {
  const { $, Utils, state, paint } = page;
  const esc = Utils.escapeHtml;
  const kind = pageState(state);
  const summary = $("#copy-summary");
  if (summary) paint(summary, chipsHtml(state, kind));
  const notices = $("#copy-notices");
  if (notices) {
    const html = noticesHtml(state, kind);
    paint(notices, html);
    notices.hidden = !html;
  }

  const status = state.overview?.status;
  const taskCount = state.overview?.tasks?.length ?? 0;
  const ready = Boolean(status) && taskCount > 0;
  const pause = ready && status.enabled && status.active_tasks > 0;
  const resume = ready && !status.enabled;
  const resumeTasks =
    ready && status.enabled && !status.active_tasks && status.resumable_tasks > 0;

  const global = $("#copy-global-action");
  if (global) {
    global.hidden = !(pause || resume);
    if (!global.hidden) {
      const [icon, label] = pause
        ? ["icon-pause", I18n.t("copy-toolbar-pause")]
        : ["icon-play", I18n.t("copy-toolbar-resume")];
      paint(global, `<i class="${icon}" aria-hidden="true"></i> <span>${esc(label)}</span>`);
    }
  }
  const bulk = $("#copy-resume-tasks");
  if (bulk) bulk.hidden = !resumeTasks;
  for (const button of [global, bulk]) {
    if (!button) continue;
    button.disabled = Boolean(state.toolbarBusy);
    gateControl(button);
  }
  // An empty page offers one way forward: the onboarding card's own action, or the
  // setup notice in Explore Mode. Add wallet joins the toolbar once a task exists.
  const add = $("#copy-add");
  if (add) add.hidden = taskCount === 0;
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
  const unpriced = Number(totals.unpriced_holdings) || 0;
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
        detail: unpriced ? I18n.t("copy-unrealized-unpriced", { count: unpriced }) : "",
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
