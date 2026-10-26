// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the account panel signs in on a headless install reached from another machine.
 *
 * A headless server has no browser to open, and its browser sign-in returns through the
 * loopback callback on the server's own port.
 *
 * - Served on another port (a tunnel that remaps it), or opened by a network address
 *   from another machine on the server's own port, the panel offers the code flow and
 *   no browser option, whose callback could never reach the server.
 * - Served on a loopback host and the server's port, the browser option stays; when the server hands the
 *   sign-in page back, the viewer's browser opens it and the panel waits for the
 *   callback. A page the server opened itself is not opened twice.
 * - Sign-up is a website page: handed back, the viewer's browser opens it.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const SIGN_IN_URL = "https://screenerbot.io/oauth/authorize?client_id=desktop";
const SIGN_UP_URL = "https://screenerbot.io/signup";
const SIGNED_OUT = {
  signed_in: false,
  name: null,
  email: null,
  scopes: [],
  device_id: null,
  wallet_has_account: false,
  online: true,
  use_gateway_rpc: false,
  data_access: null,
  session_ended: false,
};

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [shell, home] = await Promise.all([loadIndex("shell"), loadIndex("home")]);

const envelope = (data) => ({ body: JSON.stringify({ success: true, data }) });

/**
 * Mounts the account panel on Home. `samePort` serves the page on the server's own
 * port; `opened` is what the server reports for the pages it was asked to open.
 * `hostname` opens the page by that name instead of 127.0.0.1, on the same port; its
 * requests are answered by the test server.
 */
async function openPanel({ samePort, opened, hostname = null }) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const fixtures = createApiHandler([home, shell], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      const path = request.url.pathname;
      if (path === "/api/account/status") return envelope(SIGNED_OUT);
      if (path === "/api/account/signin/browser") {
        return envelope(opened ? { opened: true } : { opened: false, url: SIGN_IN_URL });
      }
      if (path === "/api/account/signup") {
        return envelope(opened ? { opened: true } : { opened: false, url: SIGN_UP_URL });
      }
      return fixtures.onApi(request);
    },
  });
  let origin = host.origin;
  if (hostname) {
    const served = new URL(host.origin);
    origin = `http://${hostname}:${served.port}`;
    await context.route(
      (url) => url.origin === origin,
      async (route) => {
        const url = new URL(route.request().url());
        url.host = served.host;
        route.fulfill({ response: await route.fetch({ url: url.href }) });
      }
    );
  }
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${origin}/home`);
  await page.waitForSelector(PAGE_READY);
  await page.evaluate((same) => {
    // The test server listens on a free port; the shell names the default one.
    if (same) window.__SCREENERBOT_PORT = Number(window.location.port);
    window.__openedPages = [];
    window.open = (url) => {
      window.__openedPages.push(url);
      return null;
    };
    const container = document.createElement("div");
    container.id = "accountPanelProbe";
    // Above the shell's fixed chrome, so every control takes a real click.
    container.style.cssText =
      "position: fixed; inset: 80px 25%; z-index: 2147483647; overflow: auto; background: var(--bg-primary);";
    document.body.append(container);
    window.AccountPanel.mount(container);
  }, samePort);
  const panel = page.locator("#accountPanelProbe .account-panel-signed-out");
  await panel.waitFor();
  const openedPages = () => page.evaluate(() => window.__openedPages);
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { page, panel, openedPages, close };
}

test("behind another port the panel offers the code flow instead of the browser", async () => {
  const { panel, openedPages, close } = await openPanel({ samePort: false, opened: false });
  try {
    assert.equal(await panel.locator('[data-action="browser"]').count(), 0);
    assert.equal(
      (await panel.locator('[data-action="device"]').textContent()).trim(),
      "Sign in with a code"
    );

    await panel.locator('[data-action="signup"]').click();
    await panel.page().waitForFunction(() => window.__openedPages.length > 0);
    assert.deepEqual(await openedPages(), [SIGN_UP_URL]);
  } finally {
    await close();
  }
});

test("on the server's port opened from another machine the panel offers the code flow", async () => {
  const { panel, close } = await openPanel({
    samePort: true,
    opened: false,
    hostname: "192.168.1.20",
  });
  try {
    assert.equal(await panel.locator('[data-action="browser"]').count(), 0);
    assert.equal(await panel.locator('[data-action="device"]').count(), 1);
  } finally {
    await close();
  }
});

test("on localhost and the server's port the browser option is offered", async () => {
  const { panel, close } = await openPanel({ samePort: true, opened: false, hostname: "localhost" });
  try {
    assert.equal(await panel.locator('[data-action="browser"]').count(), 1);
  } finally {
    await close();
  }
});

test("on the server's port a sign-in page handed back opens in this browser", async () => {
  const { page, panel, openedPages, close } = await openPanel({ samePort: true, opened: false });
  try {
    await panel.locator('[data-action="browser"]').click();
    await page.waitForFunction(() => window.__openedPages.length > 0);
    assert.deepEqual(await openedPages(), [SIGN_IN_URL]);
    await page
      .locator("#accountPanelProbe .account-notice", {
        hasText: "Finish signing in in your browser",
      })
      .waitFor();
  } finally {
    await close();
  }
});

test("a page the server opened itself is not opened again", async () => {
  const { page, panel, openedPages, close } = await openPanel({ samePort: true, opened: true });
  try {
    await panel.locator('[data-action="browser"]').click();
    await page.locator("#accountPanelProbe .account-notice").waitFor();
    await Promise.all([
      page.waitForResponse((response) => response.url().endsWith("/api/account/signup")),
      page.locator("#accountPanelProbe .account-signup-note .account-link").click(),
    ]);
    // The panel reads the response in the task after it lands.
    await page.waitForTimeout(100);
    assert.deepEqual(await openedPages(), []);
  } finally {
    await close();
  }
});
