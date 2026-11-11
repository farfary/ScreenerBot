// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: an action that finished while the action stream was down does not stay in flight.
 *
 * The resync after a reconnect only merged the actions the server still lists as active. An
 * action that completed or failed while the stream was down (sleep, a lost connection, a
 * backend restart) never received its final update, so its Positions row kept "Buying" or
 * "Selling" and the notification centre kept counting it until the page was reloaded. Now
 * each such action is fetched and replayed as its final update; one the server no longer
 * knows is dropped.
 *
 * The stream here answers and closes, so the dashboard reconnects every few seconds and
 * resyncs each time, as it does after a real outage.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [shell, positions] = await Promise.all([loadIndex("shell"), loadIndex("positions")]);
const RECORDED = JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/actions_history.json`, "utf8"))
  .actions[0];

const finished = (id) => ({ ...RECORDED, id, read: false, dismissed: false });
const inFlight = (id) => ({
  ...finished(id),
  state: {
    status: "in_progress",
    current_step: "quote",
    current_step_index: 1,
    total_steps: RECORDED.steps.length,
    progress_pct: 25,
  },
  current_step_index: 1,
  completed_at: null,
});
const activeList = (actions) => ({
  actions,
  count: actions.length,
  in_progress: actions.length,
  completed: 0,
  failed: 0,
  unread: 0,
});

test("actions that finished while the stream was down are settled on resync", async (t) => {
  const server = { outageOver: false };
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const fixtures = createApiHandler([positions, shell], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      const { pathname } = request.url;
      if (pathname === "/api/actions/stream") {
        return { contentType: "text/event-stream", body: ": connected\n\n" };
      }
      if (pathname === "/api/actions/active") {
        const active = server.outageOver
          ? []
          : [inFlight("finished-meanwhile"), inFlight("forgotten")];
        return { body: JSON.stringify(activeList(active)) };
      }
      if (pathname === "/api/actions/finished-meanwhile") {
        return { body: JSON.stringify({ success: true, action: finished("finished-meanwhile") }) };
      }
      if (pathname === "/api/actions/forgotten") {
        return { status: 404, body: JSON.stringify({ success: false }) };
      }
      return fixtures.onApi(request);
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/positions`);
  await page.waitForSelector(PAGE_READY);
  await page.evaluate(async () => {
    window.__actions = (await import("/scripts/core/notifications.js")).notificationManager;
  });
  await page.waitForFunction(
    () =>
      window.__actions.getStatus(window.__actions.notifications.get("finished-meanwhile")) ===
      "in_progress"
  );

  server.outageOver = true;
  await page.waitForFunction(() => !window.__actions.notifications.has("forgotten"), null, {
    timeout: 10000,
  });
  const status = await page.evaluate(() =>
    window.__actions.getStatus(window.__actions.notifications.get("finished-meanwhile"))
  );
  assert.equal(status, "completed", "the finished action carries its final state");
});
