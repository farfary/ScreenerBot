// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: no doubled separator. A list's last row draws no bottom line where a section
 * boundary follows it, and a section title never adds a second rule on top of the row
 * line under it, so two horizontal lines never stack with nothing between them.
 *
 * Measured on the rendered page: two horizontal borders that overlap sideways, lie
 * within `NEAR` pixels of each other and have neither text nor another line between
 * them are one separator drawn twice. Covers the Home right column (several, one and
 * no open positions) and the token details Pools and Links tabs.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

// Further apart than any section's padding pair, closer than a row with text in it.
const NEAR = 40;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [home, tokens, shell] = await Promise.all([
  loadIndex("home"),
  loadIndex("tokens"),
  loadIndex("shell"),
]);

async function openPage(t, indexes, variant, path, answers = {}) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(indexes, variant);
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      const body = answers[request.url.pathname];
      return body ? { body } : api.onApi(request);
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}${path}`);
  await page.waitForSelector(PAGE_READY);
  return page;
}

/** Pairs of horizontal lines under `selector` that draw one separator twice. */
function doubledRules(page, selector) {
  return page.$eval(
    selector,
    (root, near) => {
      const lines = [];
      for (const element of [root, ...root.querySelectorAll("*")]) {
        if (!element.checkVisibility()) continue;
        const style = getComputedStyle(element);
        const box = element.getBoundingClientRect();
        if (box.width === 0) continue;
        for (const side of ["top", "bottom"]) {
          if (style[`border-${side}-style`] === "none") continue;
          if (parseFloat(style[`border-${side}-width`]) === 0) continue;
          const name = `${element.className || element.tagName} (${side})`;
          lines.push({
            y: side === "top" ? box.top : box.bottom,
            left: box.left,
            right: box.right,
            name,
          });
        }
      }

      const texts = [];
      const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
      while (walker.nextNode()) {
        if (!walker.currentNode.textContent.trim()) continue;
        const range = document.createRange();
        range.selectNodeContents(walker.currentNode);
        texts.push(...range.getClientRects());
      }

      const overlap = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left);
      const found = [];
      for (const upper of lines) {
        for (const lower of lines) {
          const gap = lower.y - upper.y;
          if (gap <= 1 || gap > near || overlap(upper, lower) < 8) continue;
          const span = {
            left: Math.max(upper.left, lower.left),
            right: Math.min(upper.right, lower.right),
          };
          const between = (y) => y > upper.y + 1 && y < lower.y - 1;
          const hasText = texts.some(
            (rect) => overlap(rect, span) > 0 && rect.bottom > upper.y + 1 && rect.top < lower.y - 1
          );
          const hasLine = lines.some((line) => between(line.y) && overlap(line, span) > 0);
          if (!hasText && !hasLine)
            found.push(`${upper.name} -> ${lower.name} (${Math.round(gap)}px)`);
        }
      }
      return found;
    },
    NEAR
  );
}

const homeBody = JSON.parse(readFileSync(`${FIXTURES_ROOT}/home/dashboard_home.json`, "utf8"));
const HOME_CASES = [
  { name: "several open positions", variant: "populated", answers: {} },
  {
    name: "one open position",
    variant: "populated",
    answers: {
      "/api/dashboard/home": JSON.stringify({
        ...homeBody,
        positions: {
          ...homeBody.positions,
          open_count: 1,
          worst_performer: homeBody.positions.best_performer,
        },
      }),
    },
  },
  { name: "no open position", variant: "empty", answers: {} },
];

for (const { name, variant, answers } of HOME_CASES) {
  test(`the Home right column draws each separator once (${name})`, async (t) => {
    const page = await openPage(t, [home, shell], variant, "/", answers);
    await page.waitForSelector(".home-dashboard.loaded");
    assert.deepEqual(await doubledRules(page, ".home-operations"), []);
  });
}

async function openTokenTab(t, tab, ready) {
  const page = await openPage(t, [tokens, shell], "populated", "/tokens");
  await page.click("#tokens-root tr[data-row-id] .ti-row-cell__symbol");
  await page.click(`.token-details-dialog [data-dialog-tab="${tab}"]`);
  await page.waitForSelector(`.token-details-dialog ${ready}`);
  return page;
}

const TABS = [
  { tab: "pools", ready: ".pool-detail" },
  { tab: "links", ready: ".links-sheet-section" },
];

for (const { tab, ready } of TABS) {
  test(`the token details ${tab} tab draws each separator once`, async (t) => {
    const page = await openTokenTab(t, tab, ready);
    const panel = `.token-details-dialog [data-tab-content="${tab}"]`;
    assert.deepEqual(await doubledRules(page, panel), []);
  });
}

test("a last row's line above a section rule is reported as doubled", async (t) => {
  const page = await openTokenTab(t, "pools", ".pool-detail");
  await page.addStyleTag({
    content:
      ".pool-reserves > .pool-address-row:last-child { border-bottom: 1px solid #888 !important; }",
  });
  const found = await doubledRules(page, '.token-details-dialog [data-tab-content="pools"]');
  assert.ok(found.length > 0, "the measure misses a doubled separator");
});
