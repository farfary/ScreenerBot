// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every dashboard tab of one browser shares a single action stream.
 *
 * A browser keeps at most six HTTP/1.1 connections open to one origin, and an
 * `EventSource` holds one for the life of its page. With a stream per tab, the sixth
 * tab's page fragment, styles and API reads queued behind the open streams and the page
 * stayed blank: no sub-tab bar, no loading state, no console error.
 *
 * - Seven tabs open one `/api/actions/stream` request between them, and the seventh tab
 *   paints its page and sub-tab bar.
 * - An action update the stream delivers to the tab that holds it reaches the other tabs.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

const WAIT_MS = 15000;
const STREAM = "/api/actions/stream";
const SUB_TABS = "#subTabsContainer [data-tab-id]";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("filtering"), await loadIndex("shell")];

/** Serves the filtering page; `stream` answers every action stream request. */
async function host(stream) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const api = createApiHandler(indexes, "populated");
  const streams = [];
  const served = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      if (request.url.pathname !== STREAM) return api.onApi(request);
      streams.push(request.url.pathname);
      return stream(streams.length);
    },
  });
  const open = async () => {
    const page = await context.newPage();
    page.setDefaultTimeout(WAIT_MS);
    await page.goto(`${served.origin}/filtering#status`);
    await page.waitForSelector(SUB_TABS, { state: "visible" });
    return page;
  };
  const close = async () => {
    await context.close();
    await served.close();
  };
  return { open, streams, close };
}

const held = () => new Promise(() => {});

test("seven tabs share one action stream and every tab paints", async () => {
  const dashboard = await host(held);
  try {
    for (let tab = 0; tab < 7; tab += 1) await dashboard.open();
    assert.deepEqual(dashboard.streams, [STREAM]);
  } finally {
    await dashboard.close();
  }
});

test("an action update reaches the tabs that do not hold the stream", async () => {
  const [recorded] = JSON.parse(
    readFileSync(`${FIXTURES_ROOT}/shell/actions_history.json`, "utf8")
  ).actions;
  const update = {
    action_id: recorded.id,
    update_type: "action_completed",
    timestamp: recorded.completed_at ?? recorded.started_at,
    data: {},
    action: recorded,
  };
  let deliver;
  const delivered = new Promise((resolve) => (deliver = resolve));
  const dashboard = await host((count) =>
    count === 1
      ? delivered.then(() => ({
          contentType: "text/event-stream",
          body: `data: ${JSON.stringify(update)}\n\n`,
        }))
      : held()
  );
  try {
    await dashboard.open();
    const follower = await dashboard.open();
    deliver();
    await follower.waitForFunction(
      async (id) => {
        const { notificationManager } = await import("/scripts/core/notifications.js");
        return notificationManager.notifications.has(id);
      },
      recorded.id,
      { polling: 100 }
    );
  } finally {
    await dashboard.close();
  }
});
