/**
 * Tests for chart framing (`ui/advanced_chart/framing.js`).
 *
 * The rules a chart's opening view must follow: the latest view leaves RIGHT_MARGIN empty after
 * the newest candle; a span in the past is framed on its own candles with no invented
 * whitespace; a span no loaded candle covers falls back to the latest view instead of framing
 * the oldest loaded bar (a closed position older than a timeframe's stored depth did exactly
 * that); and a reference price only widens the price scale while its window is on screen.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";

const SOURCE = new URL(
  "../../src/webserver/templates/scripts/ui/advanced_chart/framing.js",
  import.meta.url
);

function framing() {
  const module = { exports: {} };
  vm.runInNewContext(readFileSync(SOURCE, "utf8"), { module });
  const api = module.exports;
  // Objects built inside the vm context carry that context's Object prototype, which strict
  // deepEqual rejects; copy plans into this realm.
  return {
    ...api,
    planRangeFrame: (input) => {
      const plan = api.planRangeFrame(input);
      return plan && { ...plan };
    },
  };
}

const MINUTE = 60;
const bars = (count) => Array.from({ length: count }, (_, i) => 1_700_000_000 + i * MINUTE);

test("latestView leaves RIGHT_MARGIN of the plot empty, in bar slots", () => {
  const { latestView, RIGHT_MARGIN } = framing();
  const view = latestView(1000, 10);
  assert.equal(view.barSpacing, 10);
  assert.equal(view.rightOffset * view.barSpacing, 1000 * RIGHT_MARGIN);
  assert.equal(latestView(0, 10).rightOffset, 0);
});

test("a span in the past is framed on its own candles with no whitespace after it", () => {
  const { planRangeFrame } = framing();
  const times = bars(1000);
  const plan = planRangeFrame({
    times,
    from: times[100],
    to: times[300],
    width: 1200,
    barSpacing: 10,
    barSeconds: MINUTE,
  });
  assert.deepEqual(plan, { mode: "logical", from: 84, to: 316 });
});

test("a span no loaded candle covers shows the latest candles, not the oldest loaded bar", () => {
  const { planRangeFrame } = framing();
  const times = bars(1000);
  const base = { times, width: 1200, barSpacing: 10, barSeconds: MINUTE };
  // Closed weeks before this timeframe's stored depth begins.
  assert.deepEqual(
    planRangeFrame({ ...base, from: times[0] - 86400 * 30, to: times[0] - 86400 * 27 }),
    {
      mode: "latest",
    }
  );
  // Entirely after the newest candle.
  assert.deepEqual(planRangeFrame({ ...base, from: times[999] + 3600, to: times[999] + 7200 }), {
    mode: "latest",
  });
});

test("a recent open position that fits the latest view opens on the latest view", () => {
  const { planRangeFrame } = framing();
  const times = bars(1000);
  const plan = planRangeFrame({
    times,
    from: times[990],
    to: times[999] + 30,
    width: 1000,
    barSpacing: 10,
    barSeconds: MINUTE,
  });
  assert.deepEqual(plan, { mode: "latest" });
});

test("a long span reaching the newest candle keeps RIGHT_MARGIN empty and its start in view", () => {
  const { planRangeFrame, RIGHT_MARGIN } = framing();
  const times = bars(1000);
  const plan = planRangeFrame({
    times,
    from: times[499],
    to: times[999] + 30,
    width: 1000,
    barSpacing: 10,
    barSeconds: MINUTE,
  });
  assert.equal(plan.mode, "logical");
  assert.ok(plan.from <= 499, "the entry candle is inside the frame");
  const empty = (plan.to - 999) / (plan.to - plan.from);
  assert.ok(Math.abs(empty - RIGHT_MARGIN) < 0.01, `empty share ${empty}`);
});

test("a short span at the start of the data keeps MIN_FRAME_BARS without empty space before it", () => {
  const { planRangeFrame, MIN_FRAME_BARS } = framing();
  const times = bars(200);
  const plan = planRangeFrame({
    times,
    from: times[1],
    to: times[2],
    width: 1200,
    barSpacing: 10,
    barSeconds: MINUTE,
  });
  assert.deepEqual(plan, { mode: "logical", from: 0, to: MIN_FRAME_BARS - 1 });
});

test("a candle that opened before the span but contains its start is inside the span", () => {
  const { planRangeFrame } = framing();
  const times = Array.from({ length: 100 }, (_, i) => i * 3600);
  const plan = planRangeFrame({
    times,
    from: 50 * 3600 + 100,
    to: 60 * 3600,
    width: 1200,
    barSpacing: 10,
    barSeconds: 3600,
  });
  assert.equal(plan.mode, "logical");
  // Span bars 50..60 padded by 2 and widened to 40 bars around them.
  assert.equal(plan.from, 50 - 2 - Math.ceil((40 - 15) / 2));
  assert.equal(planRangeFrame({ times: [], from: 0, to: 1, width: 100, barSpacing: 10 }), null);
});

test("a reference price counts only while the visible range overlaps its window", () => {
  const { referenceInView } = framing();
  const closed = { from: 100, to: 200 };
  assert.equal(referenceInView(closed, { from: 150, to: 400 }), true);
  assert.equal(referenceInView(closed, { from: 201, to: 400 }), false);
  assert.equal(referenceInView(closed, null), false);
  assert.equal(referenceInView({ from: 100, to: Infinity }, { from: 5000, to: 6000 }), true);
});
