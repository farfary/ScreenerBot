// The Execution tab: how late the target's trades are detected and how far the
// copies fill from the target's own price.
import { histogram } from "./charts.js";
import { definitionRows, seconds, signedPct } from "./format.js";
import { insightsBody, metric, rangeHead } from "./overview.js";

function bucketLabel(bucket, index, buckets) {
  if (bucket.upper_ms != null) return `≤ ${seconds(bucket.upper_ms)}`;
  const previous = buckets[index - 1]?.upper_ms;
  return previous != null ? `> ${seconds(previous)}` : "Any";
}

function body(insights, defaults, esc) {
  const arrival = insights.arrival || {};
  const slippage = insights.slippage || {};
  const decisions = insights.decisions || {};
  const limit = defaults
    ? defaults.latency_kill_switch_enabled
      ? `Pauses above ${seconds(defaults.max_arrival_distance_ms)} on average over ${defaults.latency_window_size} trades`
      : "Kill switch off"
    : "—";
  const limitMs = defaults?.latency_kill_switch_enabled ? defaults.max_arrival_distance_ms : null;
  const buckets = (insights.arrival_histogram || []).map((bucket, index, all) => ({
    label: bucketLabel(bucket, index, all),
    count: bucket.count,
    tone: limitMs != null && (all[index - 1]?.upper_ms ?? 0) >= limitMs ? "is-late" : "",
  }));
  return `<div class="copy-metrics">${[
    metric(
      "Median arrival",
      seconds(arrival.median_ms),
      `${arrival.samples || 0} live-detected trades`,
      "",
      esc
    ),
    metric("p95 arrival", seconds(arrival.p95_ms), limit, "", esc),
    metric(
      "Median slippage",
      signedPct(slippage.median_pct, 2),
      `${slippage.samples || 0} priced fills`,
      "",
      esc
    ),
    metric(
      "Worst slippage",
      signedPct(slippage.worst_pct, 2),
      `Average ${signedPct(slippage.average_pct, 2)}`,
      "",
      esc
    ),
  ].join("")}</div>
  <div class="copy-split">
    <section class="copy-card"><h4>Detection delay</h4><p class="copy-note">Time from the target's block to this bot seeing the trade. Replays after downtime are excluded.${limitMs != null ? esc(` Bars past the ${seconds(limitMs)} arrival limit are amber.`) : ""}</p>${histogram(buckets, esc)}
      <dl class="copy-defs">${definitionRows(
        [
          ["Fastest", seconds(arrival.minimum_ms)],
          ["Average", seconds(arrival.average_ms)],
          ["Slowest", seconds(arrival.maximum_ms)],
        ],
        esc
      )}</dl></section>
    <section class="copy-card"><h4>Fill against the target</h4><p class="copy-note">Positive means worse than the target: paid more on a buy, received less on a mirrored sell.</p>
      <dl class="copy-defs">${definitionRows(
        [
          ["Samples", String(slippage.samples || 0)],
          ["Average", signedPct(slippage.average_pct, 2)],
          ["Median", signedPct(slippage.median_pct, 2)],
          ["Worst", signedPct(slippage.worst_pct, 2)],
        ],
        esc
      )}</dl>
      <h4>Decisions in range</h4>
      <dl class="copy-defs">${definitionRows(
        [
          ["Fills", String(decisions.fills ?? 0)],
          ["Exits", String(decisions.exits ?? 0)],
          ["Skips", String(decisions.skips ?? 0)],
          ["Errors", String(decisions.errors ?? 0)],
        ],
        esc
      )}</dl></section>
  </div>`;
}

export function renderExecution({ insights, error, range, defaults }, esc) {
  return `${rangeHead("Execution quality", range, esc)}${insightsBody(insights, error, esc, (data) => body(data, defaults, esc))}`;
}
