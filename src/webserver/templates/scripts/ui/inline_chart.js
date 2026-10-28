// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Small inline SVG charts shared by the dashboard pages: sparkline, P&L curve, P&L bars
 * with their running total, bar list, histogram and a multi-series comparison.
 *
 * Colours come from CSS classes (styles/ui/inline_chart.css) so the theme owns them;
 * nothing here draws a glow or a fill behind a line. Every chart lays out left to right
 * whatever the page direction, and its empty state is the caller's own message.
 */

import { formatDate, formatSignedSol, formatTimestamp, signedToneClass } from "../core/format.js";
import { renderStateView } from "./state_view.js";

/** Decimals of a SOL amount on a chart axis and in a curve's label. */
const SOL_DECIMALS = 4;

const SERIES_CLASSES = [
  "inline-series-0",
  "inline-series-1",
  "inline-series-2",
  "inline-series-3",
  "inline-series-4",
  "inline-series-5",
];

function scale(domainMin, domainMax, rangeMin, rangeMax) {
  const span = domainMax - domainMin || 1;
  return (value) => rangeMin + ((value - domainMin) / span) * (rangeMax - rangeMin);
}

/** The compact empty state of a chart, carrying the caller's message. */
function emptyChart(icon, message) {
  return renderStateView({ icon, message, compact: true });
}

/** Cumulative P&L trend for a list row; "" with fewer than two points. */
export function sparkline(values, { width = 88, height = 22 } = {}) {
  const points = (values || []).filter((value) => Number.isFinite(value));
  if (points.length < 2) return "";
  const series = [0, ...points];
  const y = scale(Math.min(0, ...series), Math.max(0, ...series), height - 2, 2);
  const x = scale(0, series.length - 1, 1, width - 1);
  const path = series
    .map((value, index) => `${x(index).toFixed(1)},${y(value).toFixed(1)}`)
    .join(" ");
  const tone = signedToneClass(points[points.length - 1], SOL_DECIMALS);
  return `<svg class="inline-spark ${tone}" viewBox="0 0 ${width} ${height}" width="${width}" height="${height}" aria-hidden="true"><line class="inline-chart-zero" x1="0" x2="${width}" y1="${y(0).toFixed(1)}" y2="${y(0).toFixed(1)}"/><polyline points="${path}"/></svg>`;
}

/** Plot coordinates: the SVG is stretched to its box, so it draws lines and bars only. */
const VIEW = 100;
const INSET = 4;

const signed = (value) =>
  formatSignedSol(value, { decimals: SOL_DECIMALS, minDecimals: SOL_DECIMALS, unit: false });

const path = (points, x, y) =>
  points.map((point) => `${x(point.at).toFixed(2)},${y(point.value).toFixed(2)}`).join(" ");

/** A time tick: the date across days, the clock time within one. */
function timeLabel(ms, spanMs) {
  const date = new Date(ms);
  return spanMs < 36 * 3_600_000
    ? formatTimestamp(date, { includeDate: false, includeSeconds: false })
    : formatDate(date, { includeYear: false });
}

/** The least vertical distance, in percent of the plot, between two value labels. */
const TICK_CLEARANCE = 14;

/**
 * A chart frame: value labels beside a stretched plot, time labels under it.
 * The labels are HTML so the plot's stretch never distorts text, and every
 * stroke keeps its width (`vector-effect` in inline_chart.css). The top label carries
 * the SOL unit, and the zero line is labelled when it clears both extremes. `xLabels`
 * replaces the time ticks derived from `start` and `end`.
 */
function frame({ lines, min, max, start, end, label, escapeHtml, tall = false, xLabels }) {
  const y = scale(min, max, VIEW - INSET, INSET);
  const x = scale(start, end, 0, VIEW);
  const zero = y(0).toFixed(2);
  const tick = (value, text) =>
    `<span style="top:${y(value).toFixed(2)}%">${escapeHtml(text)}</span>`;
  const top = tick(
    max,
    formatSignedSol(max, { decimals: SOL_DECIMALS, minDecimals: SOL_DECIMALS })
  );
  const bottom = min < max ? tick(min, signed(min)) : "";
  const zeroClear =
    min < 0 &&
    max > 0 &&
    Math.abs(y(0) - y(max)) >= TICK_CLEARANCE &&
    Math.abs(y(0) - y(min)) >= TICK_CLEARANCE;
  const zeroTick = zeroClear ? tick(0, "0") : "";
  const [first, last] = xLabels || [timeLabel(start, end - start), timeLabel(end, end - start)];
  return `<figure class="inline-chart${tall ? " inline-chart--tall" : ""}" dir="ltr" role="img" aria-label="${escapeHtml(label)}">
    <div class="inline-chart-y" aria-hidden="true">${top}${zeroTick}${bottom}</div>
    <svg class="inline-chart-plot" viewBox="0 0 ${VIEW} ${VIEW}" preserveAspectRatio="none" aria-hidden="true"><line class="inline-chart-zero" x1="0" x2="${VIEW}" y1="${zero}" y2="${zero}"/>${lines(x, y)}</svg>
    <div class="inline-chart-x" aria-hidden="true"><span>${escapeHtml(first)}</span><span>${escapeHtml(last)}</span></div>
  </figure>`;
}

/** Cumulative P&L over time from `{ at, cumulative_pnl_native }` points. */
export function pnlCurve(points, { escapeHtml = String, empty = "" } = {}) {
  const data = (points || [])
    .map((point) => ({
      at: new Date(point.at).getTime(),
      value: Number(point.cumulative_pnl_native),
    }))
    .filter((point) => Number.isFinite(point.at) && Number.isFinite(point.value));
  if (!data.length) return emptyChart("icon-chart-line", empty);
  const series =
    data.length === 1
      ? [{ at: data[0].at - 60_000, value: 0 }, ...data]
      : [{ at: data[0].at, value: 0 }, ...data];
  const values = series.map((point) => point.value);
  const last = series[series.length - 1].value;
  const tone = signedToneClass(last, SOL_DECIMALS);
  return frame({
    lines: (x, y) =>
      `<polyline class="inline-chart-curve ${tone}" points="${path(series, x, y)}"/>`,
    min: Math.min(0, ...values),
    max: Math.max(0, ...values),
    start: series[0].at,
    end: series[series.length - 1].at,
    label: I18n.t("chart-inline-curve-label", { amount: signed(last) }),
    escapeHtml,
  });
}

/**
 * Net P&L per period as bars, with the running total drawn over them on the same scale:
 * `[{ at, value, trades, title }]`, oldest first, one slot per period. A period without
 * a trade keeps its slot and draws no bar. Each bar is toned by its sign as shown, the
 * running total by the sign of its last value; `title` is the bar's tooltip.
 */
export function pnlBars(points, { label = "", empty = "", escapeHtml = String } = {}) {
  const data = (points || [])
    .map((point) => ({
      at: Number(point.at),
      value: Number(point.value) || 0,
      trades: Number(point.trades) || 0,
      title: point.title || "",
    }))
    .filter((point) => Number.isFinite(point.at));
  if (!data.some((point) => point.trades > 0)) return emptyChart("icon-chart-bar", empty);
  let running = 0;
  const totals = data.map((point) => (running += point.value));
  const values = [0, ...data.map((point) => point.value), ...totals];
  const slots = data.length;
  const gap = Math.min(0.2, 8 / slots);
  return frame({
    lines: (x, y) => {
      const zero = y(0);
      const bars = data
        .map((point, index) => {
          if (point.trades === 0) return "";
          const top = Math.min(zero, y(point.value));
          const height = Math.max(Math.abs(y(point.value) - zero), 0.5);
          return `<rect class="inline-chart-bar ${signedToneClass(point.value, SOL_DECIMALS)}" x="${x(index + gap / 2).toFixed(2)}" y="${top.toFixed(2)}" width="${(x(index + 1 - gap / 2) - x(index + gap / 2)).toFixed(2)}" height="${height.toFixed(2)}"><title>${escapeHtml(point.title)}</title></rect>`;
        })
        .join("");
      const curve = [
        { at: 0, value: 0 },
        ...totals.map((value, index) => ({ at: index + 0.5, value })),
      ];
      const tone = signedToneClass(totals[totals.length - 1], SOL_DECIMALS);
      return `${bars}<polyline class="inline-chart-curve ${tone}" points="${path(curve, x, y)}"/>`;
    },
    min: Math.min(...values),
    max: Math.max(...values),
    start: 0,
    end: slots,
    label,
    escapeHtml,
    xLabels: [
      formatDate(new Date(data[0].at), { includeYear: false, utc: true }),
      formatDate(new Date(data[slots - 1].at), { includeYear: false, utc: true }),
    ],
  });
}

/** Horizontal bars: `[{ label, value, display, tone }]`. */
export function barList(items, { escapeHtml = String, empty = "" } = {}) {
  if (!items?.length) return emptyChart("icon-chart-bar", empty);
  // Bars scale to the largest magnitude whatever its unit, so 0.03 SOL fills its track.
  const max = Math.max(...items.map((item) => Math.abs(item.value) || 0));
  const width = (value) => (max > 0 ? (Math.abs(value) || 0) / max : 0) * 100;
  return `<ul class="inline-bars">${items
    .map(
      (item) =>
        `<li class="inline-bar"><span class="inline-bar-label">${escapeHtml(item.label)}</span><span class="inline-bar-track"><span class="inline-bar-fill ${item.tone || ""}" style="width:${width(item.value).toFixed(1)}%"></span></span><span class="inline-bar-value ${item.tone || ""}">${escapeHtml(item.display)}</span></li>`
    )
    .join("")}</ul>`;
}

/**
 * Vertical histogram: `[{ label, count, tone, title }]`, laid out left to right on every
 * page. `tone` is `is-positive`, `is-negative` or `is-warning`; `title` is the column's
 * tooltip, "count of total" when absent. `label` is the chart's accessible name; each
 * column is a list item named by its tooltip.
 */
export function histogram(buckets, { escapeHtml = String, empty = "", label = "" } = {}) {
  const total = (buckets || []).reduce((sum, bucket) => sum + bucket.count, 0);
  if (!total) return emptyChart("icon-chart-bar", empty);
  const max = Math.max(...buckets.map((bucket) => bucket.count), 1);
  const named = (title) => `title="${title}" role="listitem" aria-label="${title}"`;
  return `<div class="inline-histogram" dir="ltr" role="list" aria-label="${escapeHtml(label)}">${buckets
    .map(
      (bucket) =>
        `<div class="inline-histogram-col" ${named(escapeHtml(bucket.title || I18n.t("chart-inline-histogram-title", { count: bucket.count, total })))}><span class="inline-histogram-count" aria-hidden="true">${bucket.count}</span><span class="inline-histogram-bar" aria-hidden="true"><span class="${bucket.tone || ""}" style="height:${((bucket.count / max) * 100).toFixed(1)}%"></span></span><span class="inline-histogram-label" aria-hidden="true">${escapeHtml(bucket.label)}</span></div>`
    )
    .join("")}</div>`;
}

/**
 * A comparison series as drawn: it starts at its own first point, so no line jumps from
 * zero at one instant. A single point is joined to zero one minute before it, so it
 * still draws a segment.
 */
function comparisonLine(points) {
  return points.length === 1 ? [{ at: points[0].at - 60_000, value: 0 }, ...points] : points;
}

/** Several cumulative P&L curves on one time axis: `[{ nameHtml, points }]`, where
 * `nameHtml` is the legend entry's markup, already escaped. */
export function comparisonCurves(series, { escapeHtml = String, empty = "", label = "" } = {}) {
  const lines = (series || [])
    .map((entry) => ({
      nameHtml: entry.nameHtml,
      points: (entry.points || [])
        .map((point) => ({
          at: new Date(point.at).getTime(),
          value: Number(point.cumulative_pnl_native),
        }))
        .filter((point) => Number.isFinite(point.at) && Number.isFinite(point.value)),
    }))
    .filter((entry) => entry.points.length);
  if (!lines.length) return emptyChart("icon-chart-line", empty);
  const all = lines.flatMap((entry) => entry.points);
  const times = all.map((point) => point.at);
  const values = [0, ...all.map((point) => point.value)];
  const end = Math.max(...times);
  const start = Math.min(end - 60_000, ...times);
  const chart = frame({
    lines: (x, y) =>
      lines
        .map(
          (entry, index) =>
            `<polyline class="inline-chart-compare ${SERIES_CLASSES[index % SERIES_CLASSES.length]}" points="${path(comparisonLine(entry.points), x, y)}"/>`
        )
        .join(""),
    min: Math.min(...values),
    max: Math.max(...values),
    start,
    end,
    label,
    escapeHtml,
    tall: true,
  });
  const legend = lines
    .map(
      (entry, index) =>
        `<span class="inline-chart-legend-item ${SERIES_CLASSES[index % SERIES_CLASSES.length]}"><span class="inline-chart-legend-swatch"></span>${entry.nameHtml}</span>`
    )
    .join("");
  return `${chart}<div class="inline-chart-legend">${legend}</div>`;
}
