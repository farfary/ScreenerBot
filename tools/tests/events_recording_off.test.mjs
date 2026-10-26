// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: an empty event log promises events only while the events service records.
 *
 * With Events off in the configuration nothing is ever recorded, so the empty log
 * says recording is off and how to start it instead of "No events yet". With
 * recording on, the empty log keeps the usual text.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex, FIXTURES_ROOT } from "../lib/dashboard_fixtures.mjs";

const EMPTY_STATE = "#events-root .dt-state-cell .state-view-empty";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const strip = (text) => text.replace(/[⁨⁩]/g, "").replace(/\s+/g, " ").trim();

async function emptyLogText(recording) {
  const config = JSON.parse(
    await readFile(join(FIXTURES_ROOT, "events/config_events.json"), "utf8")
  );
  config.data.enabled = recording;
  const context = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const api = createApiHandler([await loadIndex("events"), await loadIndex("shell")], "empty");
  const onApi = async (request) =>
    request.url.pathname === "/api/config/events"
      ? { body: JSON.stringify(config) }
      : api.onApi(request);
  const host = await serveDashboard(context, { locale: "en", onApi });
  try {
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/events`);
    await page.waitForSelector(PAGE_READY);
    await page.waitForSelector(EMPTY_STATE);
    return strip(await page.textContent(EMPTY_STATE));
  } finally {
    await context.close();
    await host.close();
  }
}

test("with recording off the empty log says how to start it", async () => {
  const text = await emptyLogText(false);
  assert.ok(text.includes("Event recording is off"), text);
  assert.ok(text.includes("Turn on the events system in the configuration"), text);
  assert.ok(!text.includes("No events yet"), text);
});

test("with recording on the empty log waits for events", async () => {
  const text = await emptyLogText(true);
  assert.ok(text.includes("No events yet"), text);
  assert.ok(!text.includes("Event recording is off"), text);
});
