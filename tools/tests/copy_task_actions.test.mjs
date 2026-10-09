// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a Copy Trading task's action row speaks one button style. Every action is an
 * outline button; only the action that resumes copying may be the primary button, and
 * Delete differs by its danger colour alone, never by a bare or filled shape.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

test("the task action row uses one button style", async (t) => {
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    locale: "en",
  });
  const api = createApiHandler([await loadIndex("copy"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}/copy`);
  await page.waitForSelector("#copy-ws-actions [data-ws-action='delete']");
  const buttons = await page.$$eval("#copy-ws-actions .btn", (nodes) =>
    nodes.map((node) => {
      const style = getComputedStyle(node);
      return {
        action: node.dataset.wsAction,
        primary: node.classList.contains("btn-primary"),
        outline: node.classList.contains("btn-outline"),
        border: style.borderTopStyle !== "none" && parseFloat(style.borderTopWidth) > 0,
        background: style.backgroundColor,
      };
    })
  );
  assert.ok(buttons.length >= 4, JSON.stringify(buttons));
  for (const button of buttons) {
    if (button.primary) {
      assert.match(button.action, /^(resume|resume-budget|retry-watch)$/, button.action);
      continue;
    }
    assert.ok(button.outline && button.border, `${button.action} is an outline button`);
    assert.equal(button.background, "rgba(0, 0, 0, 0)", `${button.action} has no fill`);
  }
});
