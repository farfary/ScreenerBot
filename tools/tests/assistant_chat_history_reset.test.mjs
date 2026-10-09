// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a chat history that shrinks below what is rendered re-renders the shorter list.
 *
 * Regenerating the last answer drops every message after the last user message and
 * renders before the new answer streams in. That reset path clears the messages area
 * and re-appends the welcome block only when it exists; it once set the welcome's style
 * unconditionally and threw without one, leaving the list empty. Both shapes of the
 * messages area are covered, and a failed stream restores the full history.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const WAIT_MS = 15000;
const STREAM_PATH = "/api/assistant/chat/stream";
const MESSAGES = "#chat-panel .cw-chat-messages .message";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("assistant"), await loadIndex("shell")];

/**
 * Opens the populated Assistant chat with the streamed answer held open until
 * `releaseStream` answers it with an error.
 */
async function openChat() {
  const context = await browser.newContext({ viewport: { width: 1200, height: 780 } });
  const api = createApiHandler(indexes, "populated");
  let releaseStream = () => {};
  const streamHeld = new Promise((resolve) => {
    releaseStream = resolve;
  });
  const onApi = async (request) => {
    if (request.url.pathname !== STREAM_PATH) return api.onApi(request);
    await streamHeld;
    return { status: 503, body: JSON.stringify({ error: { message: "unavailable" } }) };
  };
  const host = await serveDashboard(context, { locale: "en", onApi });
  const page = await context.newPage();
  page.setDefaultTimeout(WAIT_MS);
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.goto(`${host.origin}/assistant`);
  await page.waitForFunction(
    (selector) => document.querySelectorAll(selector).length === 4,
    MESSAGES
  );
  const close = async () => {
    releaseStream();
    await context.close();
    await host.close();
  };
  return { page, pageErrors, releaseStream, close };
}

/** The rendered message roles in order, and whether a welcome block is visible. */
function renderedChat(page) {
  return page.evaluate((selector) => {
    const welcome = document.querySelector("#chat-panel .cw-chat-messages .chat-welcome");
    return {
      roles: [...document.querySelectorAll(selector)].map((el) =>
        el.classList.contains("user") ? "user" : "assistant"
      ),
      welcomeVisible: welcome ? welcome.getClientRects().length > 0 : false,
    };
  }, MESSAGES);
}

for (const withWelcome of [true, false]) {
  test(`regenerating renders the shortened history ${withWelcome ? "with" : "without"} a welcome block`, async () => {
    const { page, pageErrors, releaseStream, close } = await openChat();
    try {
      if (!withWelcome) {
        await page.evaluate(() =>
          document.querySelector("#chat-panel .cw-chat-messages .chat-welcome").remove()
        );
      }
      const lastAnswer = page.locator(`${MESSAGES}.assistant`).last();
      await lastAnswer.hover();
      await lastAnswer.locator('[data-action="regenerate"]').click();
      await page.waitForFunction(
        (selector) => document.querySelectorAll(selector).length === 3,
        MESSAGES
      );

      assert.deepEqual(await renderedChat(page), {
        roles: ["user", "assistant", "user"],
        welcomeVisible: false,
      });
      assert.equal(await page.locator("#chat-panel .chat-welcome").count(), withWelcome ? 1 : 0);

      releaseStream();
      await page.waitForFunction(
        (selector) => document.querySelectorAll(selector).length === 4,
        MESSAGES
      );
      assert.deepEqual((await renderedChat(page)).roles, [
        "user",
        "assistant",
        "user",
        "assistant",
      ]);
      assert.deepEqual(pageErrors, []);
    } finally {
      await close();
    }
  });
}
