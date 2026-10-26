// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a chat session started from the dashboard is named by its start time in the
 * viewer's time zone and locale, never by the server's UTC clock.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const WAIT_MS = 15000;
const SESSIONS_PATH = "/api/assistant/chat/sessions";
const STARTED_AT = new Date("2026-10-26T13:05:00Z");

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("assistant"), await loadIndex("shell")];

test("a new chat session is named by its local start time", async () => {
  const context = await browser.newContext({
    viewport: { width: 1200, height: 780 },
    timezoneId: "Asia/Tokyo",
  });
  const api = createApiHandler(indexes, "populated");
  const onApi = async (request) => {
    if (request.url.pathname === "/api/assistant/chat/stream") {
      return { status: 503, body: JSON.stringify({ error: { message: "unavailable" } }) };
    }
    if (request.method === "POST" && request.url.pathname === SESSIONS_PATH) {
      return { body: JSON.stringify({ session_id: 99 }) };
    }
    return api.onApi(request);
  };
  const host = await serveDashboard(context, { locale: "en", onApi });
  try {
    const page = await context.newPage();
    page.setDefaultTimeout(WAIT_MS);
    await page.clock.setFixedTime(STARTED_AT);
    const created = new Promise((resolve) => {
      page.on("request", (request) => {
        if (request.method() === "POST" && new URL(request.url()).pathname === SESSIONS_PATH) {
          resolve(request.postDataJSON());
        }
      });
    });
    await page.goto(`${host.origin}/assistant`);
    await page.click("#chat-panel .cw-new-session-btn");
    await page.fill("#chat-panel .cw-chat-input", "Hello");
    await page.click("#chat-panel .cw-send-btn");

    const { title } = await created;
    const plain = title.replace(/[⁨⁩]/g, "");
    assert.match(plain, /^Chat Oct 26, 2026, 10:05\s?PM$/, `title was ${plain}`);
  } finally {
    await context.close();
    await host.close();
  }
});
