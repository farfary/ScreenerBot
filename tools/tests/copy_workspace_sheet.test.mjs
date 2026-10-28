// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Copy Trading task sheet is the one frame of the workspace.
 *
 * - No element inside the sheet draws a box of its own (a full border with a radius over
 *   its own background); a notice, a button, a segmented control and form controls are the
 *   only framed parts. Sections are divided by hairlines instead.
 * - The results row of the Overview holds eight figures and names each once.
 * - With a paused task's notice present, the tab row still starts within the first 260 px
 *   of the sheet, because the notice sits under the tab row rather than in the header.
 *
 * Checked at a wide and a stacked window, left to right and right to left, in both themes.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { copyFixture as fixture } from "../lib/copy_page.mjs";

const VIEWS = [
  [1440, "en"],
  [820, "en"],
  [1440, "fa"],
  [820, "fa"],
];

// Elements that may carry their own frame inside the sheet, with everything inside them.
const FRAMED = ".notice, .btn, button, .segmented, input, select, textarea";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [copy, shell] = await Promise.all([loadIndex("copy"), loadIndex("shell")]);

/** The populated page with its paused, unnamed task selected. */
async function openPausedTask(t, width, locale) {
  const overview = fixture("copy_overview.json");
  const paused = (overview.data ?? overview).tasks.find((task) => !task.enabled);
  assert.ok(paused, "the fixture keeps a paused task");
  const workspace = fixture("copy_workspace.json");
  const sheet = workspace.data ?? workspace;
  Object.assign(sheet, {
    id: paused.id,
    label: paused.label,
    target_address: paused.target_address,
    enabled: false,
    pause_reason: paused.pause_reason,
    recovery: paused.recovery,
  });
  const context = await browser.newContext({ viewport: { width, height: 900 }, locale });
  const api = createApiHandler([copy, shell], "populated");
  const host = await serveDashboard(context, {
    locale,
    onApi: (request) =>
      request.url.pathname === `/api/copy-trading/tasks/${paused.id}/workspace`
        ? { body: JSON.stringify(workspace) }
        : api.onApi(request),
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/copy`);
  await page.waitForSelector(PAGE_READY);
  await page.locator(`#copy-list-rows [data-task-id="${paused.id}"]`).click();
  await page.locator(`#copy-list-rows [data-task-id="${paused.id}"][aria-current="true"]`).waitFor();
  await page.locator("#copy-ws-notices .notice").waitFor();
  await page.locator("#copy-ws-panel .metric-grid .metric-card").first().waitFor();
  return page;
}

for (const [width, locale] of VIEWS) {
  test(`the task sheet is the only frame at ${width}px (${locale})`, async (t) => {
    const page = await openPausedTask(t, width, locale);

    for (const theme of ["light", "dark"]) {
      const result = await page.evaluate(
        ({ name, framed }) => {
          document.documentElement.setAttribute("data-theme", name);
          const sheet = document.querySelector("#copy-workspace");
          const boxed = [...sheet.querySelectorAll("*")]
            .filter((node) => !node.closest(framed))
            .filter((node) => {
              const style = getComputedStyle(node);
              const sides = ["Top", "Right", "Bottom", "Left"];
              const bordered = sides.every(
                (side) =>
                  style[`border${side}Style`] !== "none" &&
                  parseFloat(style[`border${side}Width`]) > 0
              );
              const rounded = parseFloat(style.borderTopLeftRadius) > 0;
              const filled = style.backgroundColor !== "rgba(0, 0, 0, 0)";
              return bordered && rounded && filled;
            })
            .map((node) => `${node.tagName.toLowerCase()}.${[...node.classList].join(".")}`);
          const results = document.querySelector("#copy-ws-panel .metric-grid");
          const labels = [...results.querySelectorAll(".metric-card .metric-label")].map((node) =>
            node.textContent.trim()
          );
          const top = sheet.getBoundingClientRect().top;
          const tabs = document.querySelector("#copy-ws-tabs").getBoundingClientRect().top;
          return { boxed, labels, tabOffset: tabs - top };
        },
        { name: theme, framed: FRAMED }
      );

      assert.deepEqual(result.boxed, [], `${theme}: nothing inside the sheet is boxed`);
      assert.equal(result.labels.length, 8, `${theme}: ${JSON.stringify(result.labels)}`);
      assert.equal(
        new Set(result.labels).size,
        result.labels.length,
        `${theme}: each figure is named once: ${JSON.stringify(result.labels)}`
      );
      assert.ok(
        result.tabOffset <= 260,
        `${theme}: the tab row starts ${result.tabOffset}px into the sheet`
      );
    }
  });
}
