// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a Copy Trading task's action row speaks one button style. Every action is an
 * outline button; only the action that resumes copying may be the primary button, and
 * Delete differs by its danger colour alone, never by a bare or filled shape.
 *
 * A watch paused over its signature limit offers two ways out of one decision: catch up
 * from saved progress, or skip and resume from now. Both sit in the recovery panel as real
 * button variants, and the action row offers neither.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

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
      assert.match(button.action, /^(resume|retry-watch)$/, button.action);
      continue;
    }
    assert.ok(button.outline && button.border, `${button.action} is an outline button`);
    assert.equal(button.background, "rgba(0, 0, 0, 0)", `${button.action} has no fill`);
  }
});

test("a watch over its limit offers both ways out inside the recovery panel", async (t) => {
  const fixture = (name) => JSON.parse(readFileSync(`${FIXTURES_ROOT}/copy/${name}`, "utf8"));
  const task = fixture("copy_workspace.json");
  task.enabled = false;
  task.pause_reason = { kind: "watch_budget_exceeded", page_budget: 5, signatures_checked: 500 };
  const watch = fixture("wallets_watch_status.json");
  watch.target.enabled = false;
  watch.catch_up_options = [{ provider: "helius", available: true }];
  const answers = {
    [`/api/copy-trading/tasks/${task.id}/workspace`]: JSON.stringify(task),
    [`/api/wallets/watch/${watch.target.id}/status`]: JSON.stringify(watch),
  };
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("copy"), await loadIndex("shell")], "populated");
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
  await page.goto(`${host.origin}/copy`);
  const panel = page.locator("#copy-ws-watch-recovery");
  await panel.locator("[data-ws-action='approve-helius']").waitFor();

  const choices = await panel.locator("[data-ws-action]").evaluateAll((nodes) =>
    nodes.map((node) => {
      const style = getComputedStyle(node);
      return {
        action: node.dataset.wsAction,
        variant: ["btn-primary", "btn-outline"].find((name) => node.classList.contains(name)),
        legible: style.color !== style.backgroundColor,
      };
    })
  );
  assert.deepEqual(
    choices.map(({ action, variant }) => `${action}:${variant}`),
    ["approve-helius:btn-primary", "resume-budget:btn-outline"]
  );
  for (const choice of choices) assert.ok(choice.legible, `${choice.action} is legible`);
  assert.equal(await page.locator("#copy-ws-actions [data-ws-action='resume-budget']").count(), 0);

  const resume = panel.locator("[data-ws-action='resume-budget']");
  assert.ok(await resume.isDisabled(), "resuming from now waits for the acknowledgement");
  await panel.locator("[data-watch-resume-ack]").check();
  assert.ok(await resume.isEnabled(), "the acknowledgement enables resuming from now");
});
