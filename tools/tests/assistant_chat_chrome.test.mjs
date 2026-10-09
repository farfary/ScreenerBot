// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Assistant chat chrome on a fresh New Chat with no saved sessions.
 *
 * - One "new chat" action: the chat header's. The sessions sidebar and its empty list
 *   carry none of their own.
 * - The empty sessions list is the shared empty state, inside the sidebar with no frame
 *   of its own. The Telegram sessions list in Settings once shared the `.sessions-list`
 *   and `.session-item` class names from a global stylesheet and drew its bordered,
 *   italic box here.
 * - The header delete action is disabled while the chat is an unsaved draft, and the
 *   header actions are the shared icon button.
 * - The messages area marks an edge with more content beyond it, so a suggestion list
 *   cut by the composer reads as scrollable.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const WAIT_MS = 15000;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("assistant"), await loadIndex("shell")];

/** Opens the Assistant chat with no saved sessions at the given viewport. */
async function openChat(viewport) {
  const context = await browser.newContext({ viewport });
  const api = createApiHandler(indexes, "empty");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => api.onApi(request),
  });
  const page = await context.newPage();
  page.setDefaultTimeout(WAIT_MS);
  await page.goto(`${host.origin}/assistant`);
  await page.waitForSelector("#chat-panel .cw-sessions-list .state-view-empty", {
    state: "visible",
  });
  await page.waitForSelector("#chat-panel .quick-prompt", { state: "visible" });
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { page, close };
}

test("a fresh chat offers one new-chat action and a disabled delete", async () => {
  const { page, close } = await openChat({ width: 1200, height: 780 });
  try {
    const chrome = await page.evaluate(() => {
      const panel = document.querySelector("#chat-panel");
      const visible = (element) => element.getClientRects().length > 0;
      const list = panel.querySelector(".cw-sessions-list");
      const message = list.querySelector(".state-view-message");
      const listStyle = getComputedStyle(list);
      return {
        newChat: [...panel.querySelectorAll("button")]
          .filter(visible)
          .filter((button) => button.querySelector(".icon-plus"))
          .map((button) => button.className),
        listFrame: listStyle.borderTopWidth,
        messageStyle: getComputedStyle(message).fontStyle,
        headerActions: [...panel.querySelectorAll(".chat-actions button")]
          .filter(visible)
          .map((button) => button.classList.contains("btn-icon")),
        deleteDisabled: panel.querySelector(".cw-delete-btn").disabled,
      };
    });
    assert.deepEqual(chrome.newChat, ["btn-icon cw-new-session-btn"]);
    assert.equal(chrome.listFrame, "0px");
    assert.equal(chrome.messageStyle, "normal");
    assert.ok(chrome.headerActions.length > 0 && chrome.headerActions.every(Boolean));
    assert.equal(chrome.deleteDisabled, true);
  } finally {
    await close();
  }
});

test("a short chat panel marks the suggestions as scrollable", async () => {
  const { page, close } = await openChat({ width: 1328, height: 650 });
  try {
    const messages = await page.evaluate(() => {
      const area = document.querySelector("#chat-panel .chat-messages");
      return {
        overflows: area.scrollHeight > area.clientHeight,
        attachments: getComputedStyle(area).backgroundAttachment.split(/,\s*/),
      };
    });
    assert.ok(messages.overflows, "the empty chat overflows a 650px window");
    assert.deepEqual(messages.attachments, ["local", "local", "scroll", "scroll", "scroll"]);
  } finally {
    await close();
  }
});
