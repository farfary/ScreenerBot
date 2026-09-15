/**
 * Tests for the shared chart data rules (`ui/chart_data.js`).
 *
 * Covers the pure helpers the position chart relies on: which stored timeframe can show a
 * position, and which candle an event belongs to. Both were bugs on real positions — a
 * weeks-old position opened on a timeframe whose capped history started after it closed, and
 * an event on an illiquid token snapped to a candle days before it happened.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

const MODULE = "../../src/webserver/templates/scripts/ui/chart_data.js";

async function mod() {
  return import(`${MODULE}?t=${Math.random()}`);
}

const status = (rows) => ({
  timeframes: Object.entries(rows).map(([timeframe, range_candles]) => ({
    timeframe,
    candles: 1000,
    range_candles,
  })),
});

test("timeframeCoveringSpan leaves the span-ideal timeframe when stored depth has rolled past", async () => {
  const { timeframeCoveringSpan, timeframeForSpan } = await mod();
  // Position #192 shape: ~4.4 days, ideal 1h, but 1h storage starts after it closed.
  const from = 1786813818;
  const to = 1787196103;
  assert.equal(timeframeForSpan(to - from), "1h");
  const picked = timeframeCoveringSpan(
    status({ "1m": 0, "5m": 0, "15m": 0, "1h": 0, "4h": 27, "12h": 9, "1d": 5 }),
    from,
    to
  );
  assert.equal(picked, "4h");
});

test("timeframeCoveringSpan keeps the ideal timeframe when it covers the span", async () => {
  const { timeframeCoveringSpan } = await mod();
  const from = 1_000_000_020;
  const to = from + 3600 * 2;
  // Ideal for 2h is 1m; a fully stored 1m series wins over coarser ones.
  assert.equal(
    timeframeCoveringSpan(status({ "1m": 121, "5m": 25, "15m": 9, "1h": 3 }), from, to),
    "1m"
  );
});

test("timeframeForSpan picks the finest timeframe that fits the span in the pane", async () => {
  const { timeframeForSpan } = await mod();
  const hour = 3600;
  const day = 86400;
  assert.equal(timeframeForSpan(0), "1m");
  assert.equal(timeframeForSpan(3 * hour), "1m");
  // One second more needs 181 one-minute candles.
  assert.equal(timeframeForSpan(3 * hour + 1), "5m");
  // ORCA #188: 3 days 17 hours. The old from-below rule chose 15m, 356 candles.
  assert.equal(timeframeForSpan(3 * day + 17 * hour), "1h");
  assert.equal(timeframeForSpan(30 * day), "4h");
  assert.equal(timeframeForSpan(90 * day), "12h");
  assert.equal(timeframeForSpan(1000 * day), "1d");
});

test("timeframeCoveringSpan walks coarser before finer, and settles for sparse coverage", async () => {
  const { timeframeCoveringSpan } = await mod();
  // A 4-second position on an illiquid token: only the hourly bucket holding it exists.
  const from = 1787952383;
  const to = 1787952387;
  assert.equal(
    timeframeCoveringSpan(status({ "1m": 0, "5m": 0, "15m": 0, "1h": 1, "4h": 1 }), from, to),
    "1h"
  );
  // Nothing covers half the buckets: the first timeframe with any candle, coarser first.
  const dayFrom = 1_700_000_000;
  const dayTo = dayFrom + 86400 * 3;
  assert.equal(
    timeframeCoveringSpan(status({ "1m": 3, "5m": 2, "15m": 0, "1h": 4, "4h": 0 }), dayFrom, dayTo),
    "1h"
  );
  // Only a finer timeframe has anything.
  assert.equal(
    timeframeCoveringSpan(status({ "1m": 3, "5m": 0, "15m": 0, "1h": 0, "4h": 0 }), dayFrom, dayTo),
    "1m"
  );
});

test("timeframeCoveringSpan answers null without range counts or any covering candle", async () => {
  const { timeframeCoveringSpan } = await mod();
  assert.equal(timeframeCoveringSpan(null, 0, 60), null);
  assert.equal(
    timeframeCoveringSpan({ timeframes: [{ timeframe: "1m", candles: 5 }] }, 0, 60),
    null
  );
  assert.equal(timeframeCoveringSpan(status({ "1m": 0, "1h": 0 }), 0, 60), null);
});

test("barForTimestamp returns the containing candle and never borrows a distant one", async () => {
  const { barForTimestamp, timeframeSeconds } = await mod();
  const hour = timeframeSeconds("1h");
  assert.equal(hour, 3600);
  const bars = [0, 3600, 7200, 36000].map((time) => ({ time }));

  assert.equal(barForTimestamp(bars, 3600, hour)?.time, 3600);
  assert.equal(barForTimestamp(bars, 7199, hour)?.time, 3600);
  assert.equal(barForTimestamp(bars, 36001, hour)?.time, 36000);
  // Inside the no-trade gap between 7200 and 36000: no candle holds it.
  assert.equal(barForTimestamp(bars, 20000, hour), null);
  // Before the first and after the last loaded candle.
  assert.equal(barForTimestamp(bars, -1, hour), null);
  assert.equal(barForTimestamp(bars, 39600, hour), null);
  // A gap between the FIRST two bars must not widen the bucket (the old detection did).
  const sparse = [0, 86400, 86460].map((time) => ({ time }));
  assert.equal(barForTimestamp(sparse, 40000, 60), null);
  assert.equal(barForTimestamp(sparse, 86430, 60)?.time, 86400);
});

test("barForTimestamp rejects unusable input", async () => {
  const { barForTimestamp, timeframeSeconds } = await mod();
  assert.equal(barForTimestamp([], 0, 60), null);
  assert.equal(barForTimestamp([{ time: 0 }], Number.NaN, 60), null);
  assert.equal(barForTimestamp([{ time: 0 }], 0, null), null);
  assert.equal(timeframeSeconds("7m"), null);
});
