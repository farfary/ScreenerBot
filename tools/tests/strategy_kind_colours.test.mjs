// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a strategy's kind has one colour pair on every surface. Entry is the
 * primary blue and Exit the warning amber, declared once as the
 * `--strategy-kind-*` properties; no kind selector picks a success or error
 * colour of its own, so an Exit strategy never reads as a failure. The Auto Trader
 * Strategy Control lanes take the same pair on their header icon and count chip.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { STYLES_ROOT, repoPath, rulesIn, walk } from "../lib/dashboard_ui.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const KIND_SELECTOR = /\.(?:entry|exit)\b|data-type="(?:ENTRY|EXIT)"/;
const SEMANTIC_COLOUR = /--(?:success|error|danger)-|rgb\(/;

test("strategy kinds take one blue and amber pair", async () => {
  const pairs = new Map();
  const offenders = [];
  for (const file of (await walk(`${STYLES_ROOT}/pages`)).filter((path) => path.endsWith(".css"))) {
    if (!/strateg/.test(file)) continue;
    for (const rule of rulesIn(readFileSync(file, "utf8"))) {
      if (!KIND_SELECTOR.test(rule.selector)) continue;
      if (SEMANTIC_COLOUR.test(rule.body)) offenders.push(`${repoPath(file)}: ${rule.selector}`);
      const color = /--strategy-kind-color:\s*([^;]+);/.exec(rule.body)?.[1];
      // One declaration per kind: a selector list yields one rule per selector.
      if (color)
        pairs.set(`${repoPath(file)}:${rule.line}`, [
          /entry|ENTRY/.test(rule.selector) ? "entry" : "exit",
          color.trim(),
        ]);
    }
  }
  assert.deepEqual(offenders, [], "kind selectors carry no colour of their own");
  assert.deepEqual([...pairs.values()].sort(), [
    ["entry", "var(--primary-color)"],
    ["exit", "var(--warning-color)"],
  ]);
});

test("each Strategy Control lane takes its kind colour on its icon and count", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("trader"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}/trader`);
  await page.click('#subTabsContainer [data-tab-id="strategy-control"]');
  await page.waitForSelector("#exit-strategies .strategy-control-item");
  const lanes = await page.evaluate(() => {
    const doc = globalThis.document;
    const probe = (variable) => {
      const node = doc.createElement("span");
      node.style.color = `var(${variable})`;
      doc.body.append(node);
      const color = getComputedStyle(node).color;
      node.remove();
      return color;
    };
    const lane = (kind) => {
      const card = doc.querySelector(`.strategy-lane-card-${kind}`);
      return {
        icon: getComputedStyle(card.querySelector(".card-header h3 i")).color,
        count: getComputedStyle(card.querySelector(".strategy-lane-count")).color,
      };
    };
    return {
      entry: lane("entry"),
      exit: lane("exit"),
      blue: probe("--primary-color"),
      amber: probe("--warning-color"),
    };
  });
  assert.deepEqual(lanes.entry, { icon: lanes.blue, count: lanes.blue });
  assert.deepEqual(lanes.exit, { icon: lanes.amber, count: lanes.amber });
});
