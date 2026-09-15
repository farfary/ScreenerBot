/**
 * Shared OHLCV chart data access for the dashboard's chart surfaces
 * (token details and position details).
 *
 * Both dialogs hit the same endpoint with the same rules, and every rule here
 * was a bug in one of them first. Keeping one copy is the point of the module:
 * a fix made for the token chart must not have to be rediscovered for the
 * position chart.
 */
import { requestManager } from "../core/request_manager.js";

/**
 * Candle count requested for a chart. 0 = the FULL stored history for the
 * selected timeframe (no cap) — the chart shows every candle we have, like a
 * normal price chart, and lightweight-charts virtualizes rendering so large
 * sets stay smooth. This MUST be identical on the initial load and on every
 * poll refresh: a mismatch (omitting the param → backend default 100, while
 * the poll asked for more) makes the chart visibly jump to "different data" a
 * few seconds after opening, and a 100-candle window is short enough that a
 * position's own entry falls outside it. Single source of truth for every
 * fetch site.
 */
export const CHART_CANDLE_LIMIT = 0;

/** Every timeframe the backend serves, finest first. */
export const CHART_TIMEFRAMES = ["1m", "5m", "15m", "1h", "4h", "12h", "1d"];

const TIMEFRAME_SECONDS = {
  "1m": 60,
  "5m": 300,
  "15m": 900,
  "1h": 3600,
  "4h": 14400,
  "12h": 43200,
  "1d": 86400,
};

/**
 * Fetch the full candle series for one timeframe.
 * @param {string} mint
 * @param {string} timeframe
 * @param {{ priority?: string }} [opts]
 * @returns {Promise<Array>} Chart-shaped bars, empty when the timeframe has none
 */
export async function fetchCandles(mint, timeframe, { priority = "normal" } = {}) {
  const data = await requestManager.fetch(
    `/api/tokens/${mint}/ohlcv?timeframe=${timeframe}&limit=${CHART_CANDLE_LIMIT}`,
    { priority }
  );
  if (!Array.isArray(data)) return [];
  return data.map((c) => ({
    time: c.timestamp,
    open: c.open,
    high: c.high,
    low: c.low,
    close: c.close,
    volume: c.volume || 0,
  }));
}

/**
 * Ask the backend to fetch this token's candles now. Opening a timeframe that
 * has never been collected otherwise waits for ordinary monitoring cadence,
 * which reads to the user as "the chart is broken". Never throws.
 * @param {string} mint
 */
export async function triggerRefresh(mint) {
  try {
    await requestManager.fetch(`/api/tokens/${mint}/ohlcv/refresh`, {
      method: "POST",
      priority: "high",
    });
  } catch {
    /* monitoring is already active; the next cadence tick covers it */
  }
}

/**
 * The finest timeframe that currently has candles, or null. One cheap status
 * call rather than a probe request per timeframe.
 * @param {string} mint
 * @param {string} [exclude] - timeframe already known to be empty
 * @returns {Promise<string|null>}
 */
export async function findTimeframeWithData(mint, exclude) {
  const status = await fetchOhlcvStatus(mint);
  if (!status) return null;
  const withData = new Set(
    (status.timeframes || []).filter((tf) => tf.candles > 0).map((tf) => tf.timeframe)
  );
  return CHART_TIMEFRAMES.find((tf) => tf !== exclude && withData.has(tf)) || null;
}

/**
 * Per-timeframe OHLCV state for a token (candle counts, backfill, freshness).
 * Returns null on failure so callers keep their last rendered state.
 * @param {string} mint
 * @param {{ from: number, to: number }} [range] - unix seconds; adds `range_candles` per timeframe
 */
export async function fetchOhlcvStatus(mint, range) {
  const query = range ? `?from=${Math.floor(range.from)}&to=${Math.ceil(range.to)}` : "";
  try {
    const status = await requestManager.fetch(`/api/tokens/${mint}/ohlcv/status${query}`, {
      priority: "low",
    });
    return status && typeof status === "object" ? status : null;
  } catch {
    return null;
  }
}

/**
 * Pick the timeframe that renders a span of time as a readable number of
 * candles — a position open for 20 minutes wants 1m, one open for a month
 * wants 1d. Without this the chart opened on a fixed 5m for every position and
 * a week-old position's entry sat thousands of candles off-screen.
 * @param {number} spanSeconds
 * @returns {string} one of CHART_TIMEFRAMES
 */
export function timeframeForSpan(spanSeconds) {
  // Aim for roughly this many candles across the span.
  const target = 120;
  const ideal = Math.max(1, spanSeconds) / target;
  let chosen = CHART_TIMEFRAMES[0];
  for (const tf of CHART_TIMEFRAMES) {
    if (TIMEFRAME_SECONDS[tf] <= ideal) chosen = tf;
  }
  return chosen;
}

/** Seconds in one candle of a timeframe, or null for an unknown one. */
export function timeframeSeconds(timeframe) {
  return TIMEFRAME_SECONDS[timeframe] || null;
}

/**
 * The bar whose candle CONTAINS a unix-seconds timestamp, or null when that candle is not
 * loaded. Candles sit on the canonical UTC grid, so the containing bar is the one at
 * floor(ts / bucket) * bucket. A missing bucket is a real gap — no-trade candles are never
 * stored — so it answers null rather than borrowing the previous bar, which can be hours or
 * days away on an illiquid token.
 * @param {Array<{time:number}>} bars - ascending
 * @param {number} ts
 * @param {number} bucketSeconds
 */
export function barForTimestamp(bars, ts, bucketSeconds) {
  if (!bars?.length || !Number.isFinite(ts) || !(bucketSeconds > 0)) return null;
  const bucket = Math.floor(ts / bucketSeconds) * bucketSeconds;
  let lo = 0;
  let hi = bars.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const time = bars[mid].time;
    if (time === bucket) return bars[mid];
    if (time < bucket) lo = mid + 1;
    else hi = mid - 1;
  }
  return null;
}

/**
 * The timeframe that shows a span from what is actually STORED. Stored depth is capped per
 * timeframe, so the span-ideal timeframe of a position from weeks ago can hold only candles
 * from after it closed; a coarser one still covers it.
 *
 * Reads `range_candles` from a status fetched for the span. Starts at the span-ideal timeframe
 * and walks coarser, then finer: first the timeframe that covers at least half of the span's
 * buckets, else the first holding any candle in the span. Null when none does, or when the
 * status carries no range counts.
 * @param {Object|null} status - fetchOhlcvStatus(mint, { from, to })
 * @param {number} from - unix seconds
 * @param {number} to - unix seconds
 * @returns {string|null}
 */
export function timeframeCoveringSpan(status, from, to) {
  const counts = new Map(
    (status?.timeframes || [])
      .filter((tf) => Number.isFinite(tf.range_candles))
      .map((tf) => [tf.timeframe, tf.range_candles])
  );
  if (!counts.size) return null;

  const idealIdx = CHART_TIMEFRAMES.indexOf(timeframeForSpan(to - from));
  const order = [
    ...CHART_TIMEFRAMES.slice(idealIdx),
    ...CHART_TIMEFRAMES.slice(0, idealIdx).reverse(),
  ];
  const coverage = (tf) => {
    const seconds = TIMEFRAME_SECONDS[tf];
    const buckets = Math.floor(to / seconds) - Math.floor(from / seconds) + 1;
    return (counts.get(tf) || 0) / buckets;
  };
  return (
    order.find((tf) => coverage(tf) >= 0.5) || order.find((tf) => counts.get(tf) > 0) || null
  );
}

/**
 * Render the per-timeframe data-status chip. Shows an at-a-glance state —
 * ready / collecting / no data — with a hover breakdown of candle counts and
 * backfill completion, so an empty chart can always be told apart from a
 * broken one.
 * @param {Object} refs - { indicator, tip } elements (tip optional)
 * @param {Object} status - payload from fetchOhlcvStatus
 * @param {Function} formatTimeAgo - Utils.formatTimeAgo
 */
export function renderOhlcvStatus(refs, status, formatTimeAgo) {
  const { indicator, tip } = refs;
  if (!indicator || !status) return;

  let state = "none";
  let summary = "No chart data yet";
  if (status.has_data && status.backfill_complete) {
    state = "ready";
    summary = "Data ready";
  } else if (status.has_data) {
    state = "partial";
    summary = "Collecting history…";
  } else if (status.monitored) {
    state = "collecting";
    summary = "Fetching data…";
  }
  indicator.dataset.state = state;
  indicator.setAttribute("aria-label", `Chart data: ${summary}`);

  if (!tip) return;

  const ago = (ts) => (ts ? formatTimeAgo(ts) : "—");
  const rows = (status.timeframes || [])
    .map((tf) => {
      const has = tf.candles > 0;
      const dot = has ? (tf.backfill_complete ? "ready" : "partial") : "none";
      const count = has ? Number(tf.candles).toLocaleString() : "—";
      const fresh = has ? ago(tf.last_new_data_at) : "—";
      return `
        <div class="chart-data-tip-row">
          <span class="chart-data-tip-dot" data-state="${dot}"></span>
          <span class="chart-data-tip-tf">${tf.timeframe.toUpperCase()}</span>
          <span class="chart-data-tip-count">${count}</span>
          <span class="chart-data-tip-fresh" title="Last new candle">${fresh}</span>
        </div>`;
    })
    .join("");
  const checked = status.last_checked_at
    ? `checked ${ago(status.last_checked_at)}`
    : status.monitored
      ? "checking…"
      : "not checked";
  const updated = status.last_new_data_at
    ? `updated ${ago(status.last_new_data_at)}`
    : "no candles yet";

  tip.innerHTML = `
    <div class="chart-data-tip-head">
      <span class="chart-data-tip-title">${summary}</span>
      <span class="chart-data-tip-checked">${checked}</span>
    </div>
    <div class="chart-data-tip-cols">
      <span></span>
      <span>TF</span>
      <span class="chart-data-tip-count">Candles</span>
      <span class="chart-data-tip-fresh">New</span>
    </div>
    <div class="chart-data-tip-list">${rows}</div>
    <div class="chart-data-tip-foot">
      <span>${Number(status.total_candles || 0).toLocaleString()} candles · ${
        status.monitored ? "monitoring" : "idle"
      }</span>
      <span class="chart-data-tip-foot-time">${updated}</span>
    </div>`;
}
