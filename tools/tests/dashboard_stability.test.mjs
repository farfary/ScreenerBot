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
    await assertUnitsInside(page, dialog);
    if (close) await page.locator(close).first().click();
    else await page.keyboard.press("Escape");
    await opened.waitFor({ state: "hidden" }).catch(() => assert.fail(`${dialog}: did not close`));
  }
}

/**
 * Every number-field unit sits inside its field, after the value, with room left for the
 * value itself; a unit outside a field or running past its edge is the defect.
 */
async function assertUnitsInside(page, label) {
  const found = await page.evaluate(() => {
    const problems = [];
    for (const unit of document.querySelectorAll(".input-unit")) {
      if (!unit.getClientRects().length) continue;
      const name = unit.textContent.trim();
      const shell = unit.closest(".number-field");
      if (!unit.parentElement?.matches(".number-field-suffix") || !shell) {
        problems.push(`"${name}" sits outside its field`);
        continue;
      }
      const input = shell.querySelector('input[type="number"]');
      const box = shell.getBoundingClientRect();
      const mark = unit.getBoundingClientRect();
      if (mark.left < box.left - 0.5 || mark.right > box.right + 0.5) {
        problems.push(`"${name}" runs out of its field`);
      }
      const style = getComputedStyle(input);
      const room =
        input.clientWidth -
        parseFloat(style.paddingInlineStart) -
        parseFloat(style.paddingInlineEnd);
      if (room < 24) problems.push(`"${name}" leaves the value ${Math.round(room)}px`);
    }
    return problems;
  });
  assert.deepEqual(found, [], `${label}: number-field units`);
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

/**
 * Every visible DataTable header label fits inside its cell's content box. A label
 * that spills into the end padding does not grow the cell's scroll width, so only
 * the text extent shows it; the column width owner must keep the header floor.
 */
async function assertHeadersFit(page, label) {
  const spilled = await page.evaluate(() => {
    const range = document.createRange();
    return [...document.querySelectorAll(".data-table th[data-column-id]")]
      .filter((th) => th.offsetParent && th.querySelector(".dt-header-label")?.textContent.trim())
      .flatMap((th) => {
        range.selectNodeContents(th.querySelector(".dt-header-label"));
        const text = range.getBoundingClientRect();
        const box = th.getBoundingClientRect();
        const style = getComputedStyle(th);
        const start = box.left + parseFloat(style.paddingLeft);
        const end = box.right - parseFloat(style.paddingRight);
        return text.left < start - 1 || text.right > end + 1 ? [th.dataset.columnId] : [];
      });
  });
  assert.deepEqual(spilled, [], `${label}: header labels spill out of their columns`);
}

/**
 * Scrolled to its end, no visible DataTable shows content from a column that is
 * cut by the pinned columns' edge: the cut sliver carries glyph ends otherwise.
 */
async function assertPinnedEdgeClean(page, label) {
  const leaks = await page.evaluate(async () => {
    const found = [];
    const containers = [...document.querySelectorAll(".data-table-scroll-container")].filter(
      (container) => container.offsetParent && container.querySelector("td.dt-col-sticky-last")
    );
    for (const container of containers) {
      container.scrollLeft = container.scrollWidth;
      container.dispatchEvent(new Event("scroll"));
      await new Promise((resolve) => setTimeout(resolve, 50));
      const wrapper = container.closest(".data-table-wrapper") ?? container.parentElement;
      const edge = container.querySelector("td.dt-col-sticky-last").getBoundingClientRect();
      for (const cell of wrapper.querySelectorAll(
        "th[data-column-id]:not(.dt-col-sticky), tbody tr:nth-child(-n+5) td[data-column-id]:not(.dt-col-sticky)"
      )) {
        const box = cell.getBoundingClientRect();
        const cut = box.left < edge.right - 1 && box.right > edge.right + 1;
        if (cut && !cell.classList.contains("dt-col-under"))
          found.push(`${cell.tagName.toLowerCase()} ${cell.dataset.columnId}`);
      }
      container.scrollLeft = 0;
      container.dispatchEvent(new Event("scroll"));
    }
    return [...new Set(found)];
  });
  assert.deepEqual(leaks, [], `${label}: columns cut by the pinned edge still show content`);
}

/**
 * A table's loading or empty message sits in the middle of the visible scroll area,
 * at either end of a table wider than its view: the state cell spans the table's
 * whole scroll width, so a message centred in the cell drifts off to one side.
 */
async function assertStatesCentred(page, label) {
  const offsets = await page.evaluate(() => {
    const found = [];
    const containers = [...document.querySelectorAll(".data-table-scroll-container")].filter(
      (container) => container.offsetParent && container.querySelector("td.dt-state-cell > *")
    );
    for (const container of containers) {
      const message = container.querySelector("td.dt-state-cell > *");
      for (const end of [0, container.scrollWidth]) {
        container.scrollLeft = end;
        const box = container.getBoundingClientRect();
        const centre = box.left + container.clientLeft + container.clientWidth / 2;
        const rect = message.getBoundingClientRect();
        const offset = Math.round(rect.left + rect.width / 2 - centre);
        if (Math.abs(offset) > 2) found.push(`${container.closest("[id]")?.id}: ${offset}px`);
      }
      container.scrollLeft = 0;
    }
    return found;
  });
  assert.deepEqual(offsets, [], `${label}: table state messages off the visible centre`);
}

/** Render every view, then check the page does not scroll sideways. */
async function assertViewsFit(page, views, label, alsoAssert) {
  for (const view of views) {
    await assertPopulated(page, view);
    if (!view.fitDefect) await assertNoHorizontalOverflow(page, `${label} ${view.name}`);
    await alsoAssert?.(page, `${label} ${view.name}`);
  }
}

/**
 * In a right-to-left page a short value whose number is followed by a Latin unit
 * ("134 MB", "0.5 SOL", "12%") reads number first, left to right. Split runs show the
 * unit on the far side of the number ("MB 134").
 */
async function assertValuesInOrder(page, label) {
  const split = await page.evaluate(() => {
    const found = new Set();
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const node = walker.currentNode;
      const text = node.textContent;
      const value = /\d[\d.,]*\s?(?:[A-Za-z]+\b|%)/.exec(text);
      const owner = node.parentElement;
      if (!value || text.trim().length > 32 || !owner?.getClientRects().length) continue;
      const unit = value.index + value[0].search(/[A-Za-z%]/);
      const box = (index) => {
        const range = document.createRange();
        range.setStart(node, index);
        range.setEnd(node, index + 1);
        return range.getBoundingClientRect();
      };
      const digit = box(value.index);
      const letter = box(unit);
      if (!digit.width || !letter.width || Math.abs(digit.top - letter.top) > 4) continue;
      if (digit.left > letter.left)
        found.add(`${owner.className || owner.tagName}: ${text.trim()}`);
    }
    return [...found];
  });
  assert.deepEqual(split, [], `${label}: number and unit split apart`);
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
        for (const view of views) {
          await assertPopulated(session.page, view);
          await assertHeadersFit(session.page, `${id} ${view.name}`);
          await assertPinnedEdgeClean(session.page, `${id} ${view.name}`);
          await assertUnitsInside(session.page, `${id} ${view.name}`);
        }
        if (id === "positions") {
          // A new column set opens at its start edge, not at the previous view's offset.
          const container = session.page.locator("#positions-root .data-table-scroll-container");
          await session.page.locator('#subTabsContainer [data-tab-id="open"]').click();
          await session.page.waitForTimeout(300);
          await container.evaluate((el) => (el.scrollLeft = el.scrollWidth));
          assert.ok((await container.evaluate((el) => el.scrollLeft)) > 0, "Open does not scroll");
          await session.page.locator('#subTabsContainer [data-tab-id="closed"]').click();
          await session.page.waitForTimeout(300);
          assert.equal(await container.evaluate((el) => el.scrollLeft), 0);
        }
        if (id === "trader") {
          await session.page.locator('#subTabsContainer [data-tab-id="general-settings"]').click();
          assert.equal(await session.page.locator("#close-cooldown").inputValue(), "15");
          assert.equal(await session.page.locator("#entry-concurrency").inputValue(), "10");
        }
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
        for (const view of views) {
          await assertEmpty(session.page, view);
          await assertStatesCentred(session.page, `${id} ${view.name}`);
        }
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
        await assertViewsFit(page, views, RTL_LOCALE, async (current, label) => {
          await assertValuesInOrder(current, label);
          await assertUnitsInside(current, label);
        });
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
