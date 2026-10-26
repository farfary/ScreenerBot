// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: an empty event log promises events only while the events service records.
 *
 * With Events off in the configuration nothing is ever recorded, so the empty log
 * says recording is off instead of "No events yet", and states plainly that it is a
 * diagnostic tool that writes a lot of data to disk, without inviting anyone to turn
 * it on. With recording on, the empty log keeps the usual text. The recording-off
 * state's action opens the events section of the configuration.
 *
 * A configuration hint carries its warning in words, never behind a shouted label
 * ("WARNING:"): the field's impact level already marks it as critical.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { join } from "node:path";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex, FIXTURES_ROOT } from "../lib/dashboard_fixtures.mjs";
import { REPO_ROOT } from "../lib/dashboard_ui.mjs";

const EMPTY_STATE = "#events-root .dt-state-cell .state-view-empty";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const strip = (text) => text.replace(/[⁨⁩]/g, "").replace(/\s+/g, " ").trim();

/** The events page over an empty log, with recording on or off; `use` runs against it. */
async function withEmptyLog(recording, use) {
  const config = JSON.parse(
    await readFile(join(FIXTURES_ROOT, "events/config_events.json"), "utf8")
  );
  config.data.enabled = recording;
  const context = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const api = createApiHandler(
    [await loadIndex("events"), await loadIndex("config"), await loadIndex("shell")],
    "empty"
  );
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
    return await use(page);
  } finally {
    await context.close();
    await host.close();
  }
}

test("with recording off the empty log describes the diagnostic tool", async () => {
  await withEmptyLog(false, async (page) => {
    const text = strip(await page.textContent(EMPTY_STATE));
    assert.ok(text.includes("Event recording is off"), text);
    assert.ok(text.includes("Event recording is a diagnostic tool"), text);
    assert.ok(text.includes("writes a large amount of data to disk"), text);
    assert.ok(!/turn on|restart/i.test(text), `no invitation to turn it on: ${text}`);
    assert.ok(!text.includes("No events yet"), text);

    // Its action opens the events section of the configuration.
    await page.click(`${EMPTY_STATE} .state-view-action`);
    await page.waitForURL(/\/config#events$/);
    await page.waitForSelector("#configSectionsList .config-section-item.active");
    assert.equal(
      strip(
        await page.textContent(
          "#configSectionsList .config-section-item.active .config-section-label"
        )
      ),
      "Events"
    );
  });
});

test("with recording on the empty log waits for events", async () => {
  await withEmptyLog(true, async (page) => {
    const text = strip(await page.textContent(EMPTY_STATE));
    assert.ok(text.includes("No events yet"), text);
    assert.ok(!text.includes("Event recording is off"), text);
    assert.equal(await page.locator(`${EMPTY_STATE} .state-view-action`).count(), 0);
  });
});

test("no configuration hint opens with a shouted label", async () => {
  const shouted = /^\s+\.hint = \p{Lu}{3,}\s?[:：]/u;
  const offenders = [];
  const locales = join(REPO_ROOT, "locales");
  for (const entry of await readdir(locales, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const source = await readFile(join(locales, entry.name, "config.ftl"), "utf8");
    source.split("\n").forEach((line, index) => {
      if (shouted.test(line)) offenders.push(`${entry.name}/config.ftl:${index + 1}`);
    });
  }
  assert.deepEqual(offenders, [], "state the warning in words; the impact level marks it");
});
