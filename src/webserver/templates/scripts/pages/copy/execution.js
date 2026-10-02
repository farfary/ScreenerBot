// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The Execution tab: how late the target's trades are detected and how far the
// copies fill from the target's own price.
import { histogram } from "./charts.js";
import { definitionRows, seconds, signedPct } from "./format.js";
import { insightsBody, metric, rangeHead } from "./overview.js";

function bucketLabel(bucket, index, buckets) {
  if (bucket.upper_ms != null) {
    return I18n.t("copy-execution-bucket-upto", { limit: seconds(bucket.upper_ms) });
  }
  const previous = buckets[index - 1]?.upper_ms;
  return previous != null
    ? I18n.t("copy-execution-bucket-over", { limit: seconds(previous) })
    : I18n.t("copy-execution-bucket-any");
}

function body(insights, defaults, esc) {
  const arrival = insights.arrival || {};
  const slippage = insights.slippage || {};
  const decisions = insights.decisions || {};
  const limit = defaults
    ? defaults.latency_kill_switch_enabled
      ? I18n.t("copy-execution-limit-on", {
          limit: seconds(defaults.max_arrival_distance_ms),
          count: defaults.latency_window_size,
        })
      : I18n.t("copy-execution-limit-off")
    : "—";
  const limitMs = defaults?.latency_kill_switch_enabled ? defaults.max_arrival_distance_ms : null;
  const buckets = (insights.arrival_histogram || []).map((bucket, index, all) => ({
    label: bucketLabel(bucket, index, all),
    count: bucket.count,
    tone: limitMs != null && (all[index - 1]?.upper_ms ?? 0) >= limitMs ? "is-late" : "",
  }));
  return `<div class="copy-metrics">${[
    metric(
      I18n.t("copy-metric-median-arrival"),
      seconds(arrival.median_ms),
      I18n.t("copy-execution-arrival-samples", { count: arrival.samples || 0 }),
      "",
      esc
    ),
    metric(I18n.t("copy-execution-p95"), seconds(arrival.p95_ms), limit, "", esc),
    metric(
      I18n.t("copy-execution-median-slippage"),
      signedPct(slippage.median_pct, 2),
      I18n.t("copy-execution-slippage-samples", { count: slippage.samples || 0 }),
      "",
      esc
    ),
    metric(
      I18n.t("copy-execution-worst-slippage"),
      signedPct(slippage.worst_pct, 2),
      I18n.t("copy-execution-average-slippage", { amount: signedPct(slippage.average_pct, 2) }),
      "",
      esc
    ),
  ].join("")}</div>
  <div class="copy-split">
    <section class="copy-card"><h4>${esc(I18n.t("copy-execution-delay-title"))}</h4><p class="copy-note">${esc(I18n.t("copy-execution-delay-note"))}${limitMs != null ? esc(` ${I18n.t("copy-execution-delay-limit", { limit: seconds(limitMs) })}`) : ""}</p>${histogram(buckets, esc)}
      <dl class="copy-defs">${definitionRows(
        [
          [I18n.t("copy-execution-fastest"), seconds(arrival.minimum_ms)],
          [I18n.t("copy-execution-average"), seconds(arrival.average_ms)],
          [I18n.t("copy-execution-slowest"), seconds(arrival.maximum_ms)],
        ],
        esc
      )}</dl></section>
    <section class="copy-card"><h4>${esc(I18n.t("copy-execution-fill-title"))}</h4><p class="copy-note">${esc(I18n.t("copy-execution-fill-note"))}</p>
      <dl class="copy-defs">${definitionRows(
        [
          [I18n.t("copy-execution-samples"), String(slippage.samples || 0)],
          [I18n.t("copy-execution-average"), signedPct(slippage.average_pct, 2)],
          [I18n.t("copy-execution-median"), signedPct(slippage.median_pct, 2)],
          [I18n.t("copy-execution-worst"), signedPct(slippage.worst_pct, 2)],
        ],
        esc
      )}</dl>
      <h4>${esc(I18n.t("copy-execution-decisions"))}</h4>
      <dl class="copy-defs">${definitionRows(
        [
          [I18n.t("copy-kind-fills"), String(decisions.fills ?? 0)],
          [I18n.t("copy-kind-exits"), String(decisions.exits ?? 0)],
          [I18n.t("copy-kind-skips"), String(decisions.skips ?? 0)],
          [I18n.t("copy-kind-errors"), String(decisions.errors ?? 0)],
        ],
        esc
      )}</dl></section>
  </div>`;
}

export function renderExecution({ insights, error, range, defaults }, esc) {
  return `${rangeHead(I18n.t("copy-execution-title"), range, esc)}${insightsBody(insights, error, esc, (data) => body(data, defaults, esc))}`;
}
