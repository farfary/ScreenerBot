// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Opens the Copy Trading page on recorded fixtures for the offline browser tests.

import { readFileSync } from "node:fs";

import { serveDashboard, PAGE_READY } from "./dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "./dashboard_fixtures.mjs";

/** A recorded Copy Trading response, parsed fresh so a test may change it freely. */
export const copyFixture = (name) =>
  JSON.parse(readFileSync(`${FIXTURES_ROOT}/copy/${name}`, "utf8"));

/** Rendered text without bidi isolates and with collapsed whitespace. */
export const plainText = (text) => text.replace(/[⁦-⁩]/g, "").replace(/\s+/g, " ").trim();

/**
 * The page of a fresh browser context served from the populated fixtures.
 * `routes` are `[pattern, handler]` pairs applied before the page loads; they take
 * precedence over the fixture host, so a test controls exactly what an endpoint returns.
 */
export async function openCopyPage(browser, t, { routes = [] } = {}) {
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
  page.setDefaultTimeout(15000);
  for (const [pattern, handler] of routes) await page.route(pattern, handler);
  await page.goto(`${host.origin}/copy`);
  await page.waitForSelector(PAGE_READY);
  await page.waitForSelector("#copy-tab-overview");
  return page;
}
