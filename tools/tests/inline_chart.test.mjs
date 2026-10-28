// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Tests for the shared inline SVG charts (`ui/inline_chart.js`).
 *
 * - The P&L bars draw no bar for a period without a trade, put the bars and the running
 *   total on one scale, and tone each bar by its sign as shown.
 * - The histogram takes a bucket's own tooltip and lays out left to right on every page.
 * - Guard: no page module draws an inline SVG chart of its own; the allowlist only shrinks.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import "./fixtures/document_shim.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const { pnlBars, histogram, comparisonCurves } = await import(
  new URL("../../src/webserver/templates/scripts/ui/inline_chart.js", import.meta.url)
);

const ROOT = path.resolve(import.meta.dirname, "..", "..");
const SCRIPTS = path.join(ROOT, "src/webserver/templates/scripts");

const DAY_MS = 86_400_000;
const START = Date.UTC(2026, 9, 1);

const esc = (text) =>
  String(text)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");

/** One bar point per value, a day apart; `trades` defaults to one per day. */
function points(values, trades = values.map(() => 1)) {
  return values.map((value, index) => ({
    at: START + index * DAY_MS,
    value,
    trades: trades[index],
    title: `day ${index}`,
  }));
}

function rects(html) {
  return [...html.matchAll(/<rect class="([^"]*)" x="[^"]*" y="([^"]*)"/g)].map((match) => ({
    classes: match[1].split(/\s+/).filter(Boolean),
    y: match[2],
  }));
}

function curve(html) {
  const match = /<polyline class="([^"]*)" points="([^"]*)"/.exec(html);
  assert.ok(match, "no running total drawn");
  return {
    classes: match[1].split(/\s+/).filter(Boolean),
    points: match[2].split(" ").map((pair) => pair.split(",")),
  };
}

test("pnlBars draws no bar for a bucket without trades", () => {
  const html = pnlBars(points([0.1, 0, 0.2], [2, 0, 1]), {
    label: "P&L",
    empty: "none",
    escapeHtml: esc,
  });
  assert.equal(rects(html).length, 2);
  // The quiet day keeps its slot: the running total has a point for each of the three
  // periods after its start at zero.
  assert.equal(curve(html).points.length, 4);

  const quiet = pnlBars(points([0, 0], [0, 0]), {
    label: "P&L",
    empty: "Nothing closed",
    escapeHtml: esc,
  });
  assert.doesNotMatch(quiet, /<rect/);
  assert.match(quiet, /Nothing closed/);
});

test("pnlBars puts bars and the running total on one scale", () => {
  // Running total 0.1, 0.05, 0.25: the top of the scale is the total, the bottom the loss.
  const html = pnlBars(points([0.1, -0.05, 0.2]), { label: "P&L", empty: "none", escapeHtml: esc });
  const last = curve(html).points.at(-1);
  assert.equal(last[1], "4.00", "the running total's peak sits at the top inset");
  // y(0.2) on the [-0.05, 0.25] scale drawn from 96 down to 4.
  assert.equal(rects(html)[2].y, "19.33");
  assert.match(html, /0\.2500/);
  assert.match(html, /0\.0500/);
  // The top label carries the unit once, and the zero line is labelled between them.
  assert.match(html, /\+0\.2500\s+SOL<\/span>/);
  assert.match(html, />0<\/span>/);
  // The value column is as wide as its longest label, the top one with its unit.
  assert.match(html, /class="inline-chart-y" style="--inline-chart-y-chars:11"/);
});

test("pnlBars tones each bar by its sign as shown", () => {
  // 0.00001 prints as 0.0000 at four decimals, so its bar stays neutral.
  const html = pnlBars(points([0.00001, -0.01, 0.02]), {
    label: "P&L",
    empty: "none",
    escapeHtml: esc,
  });
  const tones = rects(html).map((rect) => rect.classes.filter((name) => name.startsWith("is-")));
  assert.deepEqual(tones, [[], ["is-negative"], ["is-positive"]]);
  assert.ok(curve(html).classes.includes("is-positive"), "the running total takes its last sign");
  assert.match(html, /<title>day 1<\/title>/);
});

test("histogram uses a bucket's own title", () => {
  const html = histogram(
    [
      { label: "-5", count: 1, tone: "is-negative", title: "From -5% to 0%: 1 trade" },
      { label: "0", count: 1, tone: "is-positive" },
    ],
    { escapeHtml: esc, empty: "none" }
  );
  assert.match(html, /title="From -5% to 0%: 1 trade"/);
  assert.match(html, /title="[^"]*1[^"]* of [^"]*2[^"]*"/);
  assert.match(html, /<span class="is-negative"/);
});

test("histogram lays out left to right on an RTL page", () => {
  const html = histogram([{ label: "0", count: 3 }], { escapeHtml: esc, empty: "none" });
  assert.match(html, /^<div class="inline-histogram" dir="ltr"/);
});

test("a comparison line starts at its own first point, with no jump from zero", () => {
  const html = comparisonCurves(
    [
      {
        nameHtml: "A",
        points: [
          { at: "2026-10-01T00:00:00Z", cumulative_pnl_native: 0.1 },
          { at: "2026-10-02T00:00:00Z", cumulative_pnl_native: 0.2 },
        ],
      },
    ],
    { escapeHtml: esc, empty: "none", label: "Compare" }
  );
  const points = html.match(/<polyline class="inline-chart-compare[^"]*" points="([^"]*)"/)[1];
  assert.equal(points.trim().split(" ").length, 2);
});

test("histogram carries an accessible name, and each column its range", () => {
  const html = histogram([{ label: "-5%", count: 1, title: "From -5% to 0%: 1 trade" }], {
    escapeHtml: esc,
    empty: "none",
    label: "Closed trades by return",
  });
  assert.match(html, /role="list" aria-label="Closed trades by return"/);
  assert.match(html, /role="listitem" aria-label="From -5% to 0%: 1 trade"/);
});

/** Page modules that still draw an SVG chart themselves. Only ever shrinks. */
const OWN_CHART_ALLOWLIST = [];

function scriptFiles(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) return scriptFiles(full);
    return entry.name.endsWith(".js") ? [full] : [];
  });
}

test("no page module draws an inline SVG chart of its own", () => {
  const drawing = scriptFiles(path.join(SCRIPTS, "pages"))
    .filter((file) => /<svg|<polyline|<rect|viewBox/.test(fs.readFileSync(file, "utf8")))
    .map((file) => path.relative(SCRIPTS, file).split(path.sep).join("/"))
    .sort();
  const unexpected = drawing.filter((file) => !OWN_CHART_ALLOWLIST.includes(file));
  assert.deepEqual(unexpected, [], "draw the chart with ui/inline_chart.js");
  const stale = OWN_CHART_ALLOWLIST.filter((file) => !drawing.includes(file));
  assert.deepEqual(stale, [], "remove the entry: the module no longer draws a chart");
});
