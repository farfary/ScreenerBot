// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Stability suite: every dashboard page opens against recorded `/api` data.
 *
 * `tools/lib/dashboard_host.mjs` serves the real shell, scripts and styles; every
 * `/api` request is answered from `tools/tests/fixtures/dashboard/<page>/` and a
 * request no fixture covers fails the test. Each page runs populated and empty,
 * in both themes, in a right-to-left locale and at phone width.
 *
 * A page's `views` (in its fixture index) say what must render: tables with rows,
 * charts that draw a canvas, empty-state text, and dialogs that open and close.
 * A check carrying a `defect` description documents a dashboard fault that is
 * not fixed yet: it leaves the page's regular tests and runs alone as a todo test.
 *
 * Select pages with `DASHBOARD_PAGES=home,tokens`. Refresh the recordings with
 * `npm run record:dashboard -- <dashboard url>`.
 */

import test, { describe, after } from "node:test";
import assert from "node:assert/strict";

import { availableParallelism } from "node:os";

import { chromium } from "playwright";

import { PAGE_IDS, serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const PAGES = (process.env.DASHBOARD_PAGES?.split(",") ?? PAGE_IDS).filter((id) =>
  PAGE_IDS.includes(id)
);
const WAIT_MS = 15000;
const DESKTOP = { width: 1440, height: 900 };
const PHONE = { width: 390, height: 844 };
const RTL_LOCALE = "fa";

/** Page loads are CPU bound; more than one per core only makes every load time out. */
let freeSlots = Number(process.env.DASHBOARD_SLOTS) || Math.max(2, availableParallelism());
const waiting = [];

async function withSlot(run) {
  if (freeSlots === 0) await new Promise((resolve) => waiting.push(resolve));
  else freeSlots -= 1;
  try {
    return await run();
  } finally {
    const next = waiting.shift();
    if (next) next();
    else freeSlots += 1;
  }
}

const scenario = (name, options, run) => {
  if (typeof options === "function") return test(name, (t) => withSlot(() => options(t)));
  return test(name, options, (t) => withSlot(() => run(t)));
};

const browser = await chromium.launch({ headless: true });
const hosts = new Set();
after(async () => {
  await browser.close();
  await Promise.all([...hosts].map((host) => host.close()));
});

const shellIndex = await loadIndex("shell");
const pageIndexes = new Map(await Promise.all(PAGES.map(async (id) => [id, await loadIndex(id)])));

/** Open `id` in a fresh context; every browser-side problem lands in `problems`. */
async function open(
  id,
  { variant = "populated", locale = "en", theme = "dark", viewport = DESKTOP } = {}
) {
  const context = await browser.newContext({ viewport, locale });
  const api = createApiHandler([pageIndexes.get(id), shellIndex], variant, { theme });
  const host = await serveDashboard(context, { locale, onApi: api.onApi });
  hosts.add(host);
  const page = await context.newPage();
  page.setDefaultTimeout(WAIT_MS);
  const problems = [];
  page.on(
    "console",
    (message) => message.type() === "error" && problems.push(`console error: ${message.text()}`)
  );
  page.on("pageerror", (error) => problems.push(`uncaught exception: ${error.message}`));
  page.on("requestfailed", (request) => {
    // A fetch the page aborts itself (a superseded tab load) is cancellation, not failure.
    const reason = request.failure()?.errorText;
    if (reason !== "net::ERR_ABORTED")
      problems.push(`request failed: ${request.url()} (${reason})`);
  });
  page.on(
    "response",
    (response) =>
      response.status() >= 400 && problems.push(`HTTP ${response.status()}: ${response.url()}`)
  );
  await page.goto(`${host.origin}/${id}?theme=${theme}`);
  await page.waitForSelector("body:not(.initialization-mode) main.content:not([data-loading])");
  return { page, context, problems, api, host };
}

async function finish(session) {
  const { page, problems, api, host } = session;
  const unmocked = api.unmocked.map((request) => `unmocked request: ${request}`);
  const unserved = host.unserved.map((url) => `external request: ${url}`);
  const found = [...problems, ...unmocked, ...unserved];
  // Requests still in flight are aborted by closing the context; that is not a page failure.
  page.removeAllListeners();
  await session.context.close();
  await host.close();
  hosts.delete(host);
  assert.deepEqual(found, [], `browser problems on ${page.url()}`);
}

const sound = (checks = []) => checks.filter((check) => !check.defect);

async function waitForCount(page, selector, min) {
  await page
    .waitForFunction(
      ([css, least]) => document.querySelectorAll(css).length >= least,
      [selector, min],
      { timeout: WAIT_MS }
    )
    .catch(async () => {
      const found = await page.locator(selector).count();
      assert.fail(`${selector}: expected at least ${min} elements, found ${found}`);
    });
}

async function clickAll(page, selectors = []) {
  for (const selector of selectors) await page.locator(selector).first().click();
}

async function assertPopulated(page, view) {
  await clickAll(page, view.click);
  for (const { selector, min = 1 } of sound(view.populated))
    await waitForCount(page, selector, min);
  for (const selector of view.canvas ?? []) {
    await page
      .waitForFunction(
        (css) =>
          [...document.querySelectorAll(`${css} canvas`)].some(
            (canvas) => canvas.width > 0 && canvas.height > 0
          ),
        selector,
        { timeout: WAIT_MS }
      )
      .catch(() => assert.fail(`${selector}: no canvas was drawn`));
  }
}

async function assertEmpty(page, view) {
  if (!sound(view.empty).length) return;
  await clickAll(page, view.click);
  for (const { selector, text } of sound(view.empty)) {
    const pattern = new RegExp(text, "i");
    await page
      .waitForFunction(
        ([css, source]) => {
          const element = [...document.querySelectorAll(css)].find(
            (candidate) => candidate.offsetParent
          );
          return (
            Boolean(element) && new RegExp(source, "i").test(element.innerText.replace(/\s+/g, " "))
          );
        },
        [selector, pattern.source],
        { timeout: WAIT_MS }
      )
      .catch(async () => {
        const shown = await page
          .locator(selector)
          .first()
          .innerText({ timeout: 1000 })
          .catch(() => null);
        assert.fail(
          `${selector}: expected empty state text ${pattern}, found ${JSON.stringify(shown)}`
        );
      });
  }
}

async function assertDialogs(page, view) {
  if (!sound(view.dialogs).length) return;
  await clickAll(page, view.click);
  for (const { trigger, dialog, close } of sound(view.dialogs)) {
    await page.locator(trigger).first().click();
    const opened = page.locator(dialog).first();
    await opened
      .waitFor({ state: "visible" })
      .catch(() => assert.fail(`${trigger}: ${dialog} did not open`));
    if (close) await page.locator(close).first().click();
    else await page.keyboard.press("Escape");
    await opened.waitFor({ state: "hidden" }).catch(() => assert.fail(`${dialog}: did not close`));
  }
}

/** Horizontal overflow of the page itself, as a scroll bar would show it. */
async function overflow(page) {
  return page.evaluate(() => {
    const root = document.documentElement;
    return {
      scrollWidth: Math.max(root.scrollWidth, document.body.scrollWidth),
      clientWidth: root.clientWidth,
    };
  });
}

async function assertNoHorizontalOverflow(page, label) {
  const { scrollWidth, clientWidth } = await overflow(page);
  assert.ok(
    scrollWidth <= clientWidth,
    `${label}: page is ${scrollWidth}px wide in a ${clientWidth}px viewport`
  );
}

/** Render every view, then check the page does not scroll sideways. */
async function assertViewsFit(page, views, label) {
  for (const view of views) {
    await assertPopulated(page, view);
    if (!view.fitDefect) await assertNoHorizontalOverflow(page, `${label} ${view.name}`);
  }
}

async function assertTheme(page, theme) {
  assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), theme);
  const background = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  assert.equal(
    luminance(background) > 0.5,
    theme === "light",
    `body background ${background} does not read as ${theme}`
  );
}

const luminance = (rgb) => {
  const [r, g, b] = rgb.match(/\d+(\.\d+)?/g).map(Number);
  return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
};

describe("dashboard stability", { concurrency: 4 }, () => {
  for (const id of PAGES) {
    const { views } = pageIndexes.get(id);
    describe(id, { concurrency: true }, () => {
      test("has views to assert", () => {
        assert.ok(
          views.length > 0,
          `tools/tests/fixtures/dashboard/${id}/index.mjs declares no views`
        );
      });

      scenario("renders populated in the dark theme, dialogs open and close", async () => {
        const session = await open(id);
        for (const view of views) await assertPopulated(session.page, view);
        await assertTheme(session.page, "dark");
        for (const view of views) await assertDialogs(session.page, view);
        await finish(session);
      });

      for (const view of views) {
        const known = [
          ...(view.populated ?? []).map((check) => ({
            check,
            key: "populated",
            run: assertPopulated,
            variant: "populated",
          })),
          ...(view.dialogs ?? []).map((check) => ({
            check,
            key: "dialogs",
            run: assertDialogs,
            variant: "populated",
          })),
          ...(view.empty ?? []).map((check) => ({
            check,
            key: "empty",
            run: assertEmpty,
            variant: "empty",
          })),
        ].filter(({ check }) => check.defect);
        for (const { check, key, run, variant } of known) {
          scenario(
            `${view.name}: ${check.selector ?? check.dialog}`,
            { todo: check.defect },
            async () => {
              const session = await open(id, { variant });
              await run(session.page, {
                click: view.click,
                [key]: [{ ...check, defect: undefined }],
              });
              await finish(session);
            }
          );
        }
      }

      for (const view of views.filter((candidate) => candidate.fitDefect)) {
        scenario(
          `${PHONE.width}px phone width: ${view.name} fits`,
          { todo: view.fitDefect },
          async () => {
            const session = await open(id, { viewport: PHONE });
            await clickAll(session.page, view.click);
            await assertNoHorizontalOverflow(session.page, `${PHONE.width}px ${view.name}`);
            await finish(session);
          }
        );
      }

      scenario("renders empty states", async () => {
        const session = await open(id, { variant: "empty" });
        for (const view of views) await assertEmpty(session.page, view);
        await finish(session);
      });

      scenario("light theme", async () => {
        const session = await open(id, { theme: "light" });
        await assertPopulated(session.page, views[0]);
        await assertTheme(session.page, "light");
        await finish(session);
      });

      scenario(`${RTL_LOCALE} locale is right-to-left without overflow`, async () => {
        const session = await open(id, { locale: RTL_LOCALE });
        const { page } = session;
        const [dir, lang] = await page.evaluate(() => [
          document.documentElement.dir,
          document.documentElement.lang,
        ]);
        assert.deepEqual([dir, lang], ["rtl", RTL_LOCALE]);
        await assertViewsFit(page, views, RTL_LOCALE);
        await finish(session);
      });

      scenario(`${PHONE.width}px phone width has no horizontal overflow`, async () => {
        const session = await open(id, { viewport: PHONE });
        const { page } = session;
        await assertViewsFit(page, views, `${PHONE.width}px`);
        await finish(session);
      });
    });
  }
});
