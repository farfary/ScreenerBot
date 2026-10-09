// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Small inline SVG charts for Copy Trading: sparkline, P&L curve, bar list,
// histogram and a multi-series comparison. Colours come from CSS classes so the
// theme owns them; nothing here draws a glow or background.

import { formatDate, formatSignedSol, formatTimestamp } from "../../core/format.js";

const SERIES_CLASSES = [
  "copy-series-0",
  "copy-series-1",
  "copy-series-2",
  "copy-series-3",
  "copy-series-4",
  "copy-series-5",
];

function scale(domainMin, domainMax, rangeMin, rangeMax) {
  const span = domainMax - domainMin || 1;
  return (value) => rangeMin + ((value - domainMin) / span) * (rangeMax - rangeMin);
}

/** Cumulative P&L trend for a task list row. */
export function sparkline(values, { width = 88, height = 22 } = {}) {
  const points = (values || []).filter((value) => Number.isFinite(value));
  if (points.length < 2) return '<span class="copy-spark-empty">—</span>';
  const series = [0, ...points];
  const y = scale(Math.min(0, ...series), Math.max(0, ...series), height - 2, 2);
  const x = scale(0, series.length - 1, 1, width - 1);
  const path = series
    .map((value, index) => `${x(index).toFixed(1)},${y(value).toFixed(1)}`)
    .join(" ");
  const tone = points[points.length - 1] >= 0 ? "is-positive" : "is-negative";
  return `<svg class="copy-spark ${tone}" viewBox="0 0 ${width} ${height}" width="${width}" height="${height}" aria-hidden="true"><line class="copy-chart-zero" x1="0" x2="${width}" y1="${y(0).toFixed(1)}" y2="${y(0).toFixed(1)}"/><polyline points="${path}"/></svg>`;
}

/** Plot coordinates: the SVG is stretched to its box, so it draws lines only. */
const VIEW = 100;
const INSET = 4;

const signed = (value) => formatSignedSol(value, { decimals: 3, unit: false });

const path = (points, x, y) =>
  points.map((point) => `${x(point.at).toFixed(2)},${y(point.value).toFixed(2)}`).join(" ");

/** A time tick: the date across days, the clock time within one. */
function timeLabel(ms, spanMs) {
  const date = new Date(ms);
  return spanMs < 36 * 3_600_000
    ? formatTimestamp(date, { includeDate: false, includeSeconds: false })
    : formatDate(date, { includeYear: false });
}

/**
 * A chart frame: value labels beside a stretched plot, time labels under it.
 * The labels are HTML so the plot's stretch never distorts text, and every
 * stroke keeps its width (`vector-effect` in charts.css).
 */
function frame({ lines, min, max, start, end, label, escapeHtml, tall = false }) {
  const y = scale(min, max, VIEW - INSET, INSET);
  const x = scale(start, end, 0, VIEW);
  const zero = y(0).toFixed(2);
  const tick = (value) => `<span style="top:${y(value).toFixed(2)}%">${signed(value)}</span>`;
  return `<figure class="copy-chart${tall ? " copy-chart--tall" : ""}" dir="ltr" role="img" aria-label="${escapeHtml(label)}">
    <div class="copy-chart-y" aria-hidden="true">${tick(max)}${min < max ? tick(min) : ""}</div>
    <svg class="copy-chart-plot" viewBox="0 0 ${VIEW} ${VIEW}" preserveAspectRatio="none" aria-hidden="true"><line class="copy-chart-zero" x1="0" x2="${VIEW}" y1="${zero}" y2="${zero}"/>${lines(x, y)}</svg>
    <div class="copy-chart-x" aria-hidden="true"><span>${escapeHtml(timeLabel(start, end - start))}</span><span>${escapeHtml(timeLabel(end, end - start))}</span></div>
  </figure>`;
}

/** Cumulative P&L over time from `{ at, cumulative_pnl_native }` points. */
export function pnlCurve(points, { escapeHtml = String } = {}) {
  const data = (points || [])
    .map((point) => ({ at: new Date(point.at).getTime(), value: Number(point.cumulative_pnl_native) }))
    .filter((point) => Number.isFinite(point.at) && Number.isFinite(point.value));
  if (!data.length) {
    return `<div class="copy-chart-empty">${escapeHtml(I18n.t("copy-chart-empty-curve"))}</div>`;
  }
  const series =
    data.length === 1
      ? [{ at: data[0].at - 60_000, value: 0 }, ...data]
      : [{ at: data[0].at, value: 0 }, ...data];
  const values = series.map((point) => point.value);
  const last = series[series.length - 1].value;
  const tone = last >= 0 ? "is-positive" : "is-negative";
  return frame({
    lines: (x, y) => `<polyline class="copy-curve-line ${tone}" points="${path(series, x, y)}"/>`,
    min: Math.min(0, ...values),
    max: Math.max(0, ...values),
    start: series[0].at,
    end: series[series.length - 1].at,
    label: I18n.t("copy-chart-curve-label", { amount: signed(last) }),
    escapeHtml,
  });
}

/** Horizontal bars: `[{ label, value, display, tone }]`. */
export function barList(items, escapeHtml) {
  if (!items?.length) {
    return `<div class="copy-chart-empty">${escapeHtml(I18n.t("copy-chart-empty-bars"))}</div>`;
  }
  // Bars scale to the largest magnitude whatever its unit, so 0.03 SOL fills its track.
  const max = Math.max(...items.map((item) => Math.abs(item.value) || 0));
  const width = (value) => (max > 0 ? (Math.abs(value) || 0) / max : 0) * 100;
  return `<ul class="copy-bars">${items
    .map(
      (item) =>
        `<li class="copy-bar"><span class="copy-bar-label">${escapeHtml(item.label)}</span><span class="copy-bar-track"><span class="copy-bar-fill ${item.tone || ""}" style="width:${width(item.value).toFixed(1)}%"></span></span><span class="copy-bar-value ${item.tone || ""}">${escapeHtml(item.display)}</span></li>`
    )
    .join("")}</ul>`;
}

/** Vertical histogram: `[{ label, count, tone }]`. */
export function histogram(buckets, escapeHtml) {
  const total = (buckets || []).reduce((sum, bucket) => sum + bucket.count, 0);
  if (!total) {
    return `<div class="copy-chart-empty">${escapeHtml(I18n.t("copy-chart-empty-histogram"))}</div>`;
  }
  const max = Math.max(...buckets.map((bucket) => bucket.count), 1);
  return `<div class="copy-histogram">${buckets
    .map(
      (bucket) =>
        `<div class="copy-histogram-col" title="${escapeHtml(I18n.t("copy-chart-histogram-title", { count: bucket.count, total }))}"><span class="copy-histogram-count">${bucket.count}</span><span class="copy-histogram-bar"><span class="${bucket.tone || ""}" style="height:${((bucket.count / max) * 100).toFixed(1)}%"></span></span><span class="copy-histogram-label">${escapeHtml(bucket.label)}</span></div>`
    )
    .join("")}</div>`;
}

/** Several cumulative P&L curves on one time axis: `[{ nameHtml, points }]`, where
 * `nameHtml` is the legend entry's markup, already escaped. */
export function comparisonCurves(series, { escapeHtml = String } = {}) {
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
  if (!lines.length) {
    return `<div class="copy-chart-empty">${escapeHtml(I18n.t("copy-chart-empty-compare"))}</div>`;
  }
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
            `<polyline class="copy-compare-line ${SERIES_CLASSES[index % SERIES_CLASSES.length]}" points="${path([{ at: entry.points[0].at, value: 0 }, ...entry.points], x, y)}"/>`
        )
        .join(""),
    min: Math.min(...values),
    max: Math.max(...values),
    start,
    end,
    label: I18n.t("copy-chart-compare-label"),
    escapeHtml,
    tall: true,
  });
  const legend = lines
    .map(
      (entry, index) =>
        `<span class="copy-legend-item ${SERIES_CLASSES[index % SERIES_CLASSES.length]}"><span class="copy-legend-swatch"></span>${entry.nameHtml}</span>`
    )
    .join("");
  return `${chart}<div class="copy-legend">${legend}</div>`;
}
