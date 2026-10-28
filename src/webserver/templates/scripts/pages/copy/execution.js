// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The Execution tab, as sections of the task sheet: how late the target's trades are
// detected, how far the copies fill from the target's own price, the decisions in the
// range, and for a paper task the paper model that produced its fills.
import { histogram } from "./charts.js";
import { definitionRows, finite, section, seconds, signedPct, splitSection } from "./format.js";
import { insightsBody, rangeHead } from "./overview.js";
import { metricCard } from "../../ui/metric_card.js";

function bucketLabel(bucket, index, buckets) {
  if (bucket.upper_ms != null) {
    return I18n.t("copy-execution-bucket-upto", { limit: seconds(bucket.upper_ms) });
  }
  const previous = buckets[index - 1]?.upper_ms;
  return previous != null
    ? I18n.t("copy-execution-bucket-over", { limit: seconds(previous) })
    : I18n.t("copy-execution-bucket-any");
}

/** The quality row of the range: arrival delay and the difference from the target's price. */
function quality(insights, defaults, esc) {
  const arrival = insights.arrival || {};
  const slippage = insights.slippage || {};
  const limit = defaults
    ? defaults.latency_kill_switch_enabled
      ? I18n.t("copy-execution-limit-on", {
          limit: seconds(defaults.max_arrival_distance_ms),
          count: defaults.latency_window_size,
        })
      : I18n.t("copy-execution-limit-off")
    : "—";
  return `<div class="metric-grid" data-surface="sheet">${[
    metricCard(
      {
        label: I18n.t("copy-metric-median-arrival"),
        value: seconds(arrival.median_ms),
        detail: I18n.t("copy-execution-arrival-samples", { count: arrival.samples || 0 }),
      },
      esc
    ),
    metricCard(
      { label: I18n.t("copy-execution-p95"), value: seconds(arrival.p95_ms), detail: limit },
      esc
    ),
    metricCard(
      {
        label: I18n.t("copy-execution-target-price-difference"),
        value: signedPct(slippage.median_pct, 2),
        detail: I18n.t("copy-execution-target-difference-samples", {
          count: slippage.samples || 0,
        }),
      },
      esc
    ),
    metricCard(
      {
        label: I18n.t("copy-execution-target-difference-worst"),
        value: signedPct(slippage.worst_pct, 2),
        detail: I18n.t("copy-execution-target-difference-average", {
          amount: signedPct(slippage.average_pct, 2),
        }),
      },
      esc
    ),
  ].join("")}</div>`;
}

/** The delay distribution beside the price difference, then the decisions in the range. */
function body(insights, defaults, esc) {
  const arrival = insights.arrival || {};
  const slippage = insights.slippage || {};
  const decisions = insights.decisions || {};
  const limitMs = defaults?.latency_kill_switch_enabled ? defaults.max_arrival_distance_ms : null;
  const buckets = (insights.arrival_histogram || []).map((bucket, index, all) => ({
    label: bucketLabel(bucket, index, all),
    count: bucket.count,
    tone: limitMs != null && (all[index - 1]?.upper_ms ?? 0) >= limitMs ? "is-late" : "",
  }));
  return `${splitSection(
    [
      [
        I18n.t("copy-execution-delay-title"),
        `<p class="copy-note">${esc(I18n.t("copy-execution-delay-note"))}${limitMs != null ? esc(` ${I18n.t("copy-execution-delay-limit", { limit: seconds(limitMs) })}`) : ""}</p>${histogram(buckets, esc)}
      <dl class="copy-defs">${definitionRows(
        [
          [I18n.t("copy-execution-fastest"), seconds(arrival.minimum_ms)],
          [I18n.t("copy-execution-average"), seconds(arrival.average_ms)],
          [I18n.t("copy-execution-slowest"), seconds(arrival.maximum_ms)],
        ],
        esc
      )}</dl>`,
      ],
      [
        I18n.t("copy-execution-target-difference-title"),
        `<p class="copy-note">${esc(I18n.t("copy-execution-target-difference-note"))}</p>
      <dl class="copy-defs">${definitionRows(
        [
          [I18n.t("copy-execution-samples"), String(slippage.samples || 0)],
          [I18n.t("copy-execution-average"), signedPct(slippage.average_pct, 2)],
          [I18n.t("copy-execution-median"), signedPct(slippage.median_pct, 2)],
          [I18n.t("copy-execution-worst"), signedPct(slippage.worst_pct, 2)],
        ],
        esc
      )}</dl>`,
      ],
    ],
    esc
  )}${section(
    I18n.t("copy-execution-decisions"),
    `<dl class="copy-defs">${definitionRows(
      [
        [I18n.t("copy-kind-fills"), String(decisions.fills ?? 0)],
        [I18n.t("copy-kind-exits"), String(decisions.exits ?? 0)],
        [I18n.t("copy-kind-skips"), String(decisions.skips ?? 0)],
        [I18n.t("copy-kind-errors"), String(decisions.errors ?? 0)],
      ],
      esc
    )}</dl>`,
    esc
  )}`;
}

/**
 * The paper model as configured now, for a paper task only: the workspace's own settings,
 * then the defaults. A live task's figures are not produced by this model.
 */
function paperModel(ws, defaults, esc) {
  if (ws.mode === "live") return "";
  const execution = ws.paper_execution || {};
  const delay = finite(execution.landing_delay_ms ?? defaults?.paper_landing_delay_ms);
  const capacity = finite(execution.queue_capacity ?? defaults?.paper_queue_capacity);
  const rows = [
    delay === null ? null : [I18n.t("copy-paper-wait-label"), seconds(delay, { precise: true })],
    capacity === null ? null : [I18n.t("copy-paper-queue-label"), String(capacity)],
  ];
  return section(
    I18n.t("copy-paper-model"),
    `<p class="copy-note">${esc(I18n.t("copy-paper-model-note"))}</p><p class="copy-note">${esc(I18n.t("copy-execution-wait-note"))}</p><dl class="copy-defs">${definitionRows(rows, esc)}</dl>`,
    esc
  );
}

export function renderExecution({ ws, insights, error, range, defaults }, esc) {
  return `<section class="copy-section">${rangeHead(I18n.t("copy-execution-title"), range, esc)}${insightsBody(insights, error, esc, (data) => quality(data, defaults, esc))}</section>${insights ? body(insights, defaults, esc) : ""}${paperModel(ws, defaults, esc)}`;
}
