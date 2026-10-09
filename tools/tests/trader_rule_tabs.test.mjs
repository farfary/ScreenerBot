// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Auto Trader rule tabs (Stop Loss, Trailing Stop, Take Profit, Time Rules,
 * DCA, Settings, Strategy Control) speak in plain fields and bare glyphs.
 *
 * The "How It Works" steps sat on filled colour circles, every field carried a badge
 * that restated its title ("LOSS LIMIT", "ENTRY TRIGGER"), the DCA example line was set
 * in link blue, and DCA wore a dollar sign although it is denominated in SOL.
 *
 * - A step glyph has no background, border or shadow.
 * - No field carries a badge.
 * - A summary line's sentence is body text; only its glyph and figure take the tone.
 * - No rule tab uses a dollar-sign glyph.
 * - Entry Sizes, a list of SOL amounts, matches the number fields beside it.
 * - A strategy card shows its state only through its toggle, and no chip repeats its lane.
 * - An example figure (a SOL price or a percent) never ends its fraction in a zero, so
 *   one strip does not mix "+10%" with "+32.0%" or print "1.0000 SOL".
 * - A config card never draws more columns than it has field groups, so two lists
 *   (How DCA Works, Risk Warnings) share the width instead of leaving a third empty.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";
const TABS = ["stop-loss", "trailing-stop", "roi", "time-rules", "dca", "general-settings"];

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
const api = createApiHandler([await loadIndex("trader"), await loadIndex("shell")], "populated");
const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
after(async () => {
  await context.close();
  await host.close();
});
const page = await context.newPage();
page.setDefaultTimeout(15000);
await page.goto(`${host.origin}/trader`);
await page.waitForSelector(READY);

for (const id of TABS) {
  test(`the ${id} tab draws bare step glyphs, plain fields and body-text summaries`, async () => {
    await page.click(`#subTabsContainer [data-tab-id="${id}"]`);
    const panel = page.locator(`#${id}-tab`);
    await panel.waitFor();

    const report = await panel.evaluate((root) => {
      const transparent = (value) => value === "rgba(0, 0, 0, 0)" || value === "transparent";
      const steps = [...root.querySelectorAll(".step-icon")].map((el) => {
        const style = getComputedStyle(el);
        return {
          bare:
            transparent(style.backgroundColor) &&
            style.backgroundImage === "none" &&
            style.boxShadow === "none" &&
            style.borderStyle === "none",
        };
      });
      const badges = [...root.querySelectorAll(".config-badge")].map((el) => el.dataset.l10nId);
      const body = getComputedStyle(document.body).color;
      const summaries = [...root.querySelectorAll(".summary-item")].map((el) => ({
        text: el.textContent.trim().slice(0, 40),
        color: getComputedStyle(el).color,
      }));
      const dollars = root.querySelectorAll(".icon-dollar-sign").length;
      return { steps, badges, body, summaries, dollars };
    });

    assert.ok(
      report.steps.every((step) => step.bare),
      `${id}: every step glyph is bare`
    );
    assert.deepEqual(report.badges, [], `${id}: no field carries a badge`);
    for (const summary of report.summaries) {
      assert.equal(summary.color, report.body, `${id}: "${summary.text}" is body text`);
    }
    assert.equal(report.dollars, 0, `${id}: no dollar-sign glyph`);
  });
}

test("example figures carry no trailing fraction zero on any rule tab", async () => {
  for (const id of TABS) {
    await page.click(`#subTabsContainer [data-tab-id="${id}"]`);
    const panel = page.locator(`#${id}-tab`);
    await panel.waitFor();
    const figures = await panel.evaluate((root) =>
      [...root.querySelectorAll('[id*="example"]')]
        .filter((el) => el.children.length === 0 || el.id.endsWith("-pct"))
        .map((el) => ({ id: el.id, text: el.textContent.trim() }))
        .filter((figure) => /\d/.test(figure.text))
    );
    for (const figure of figures) {
      assert.doesNotMatch(
        figure.text,
        /\d\.\d*0(?!\d)/,
        `${id} #${figure.id}: "${figure.text}" has no trailing fraction zero`
      );
    }
  }
});

test("Entry Sizes, a list of amounts, uses the face and weight of the number fields", async () => {
  await page.click('#subTabsContainer [data-tab-id="general-settings"]');
  await page.locator("#entry-sizes").waitFor();
  const face = (selector) =>
    page.$eval(selector, (el) => {
      const style = getComputedStyle(el);
      return { family: style.fontFamily, weight: style.fontWeight };
    });
  assert.deepEqual(await face("#entry-sizes"), await face("#trade-size"));
});

test("a strategy card states its state once, through its named toggle", async () => {
  await page.click('#subTabsContainer [data-tab-id="strategy-control"]');
  await page.locator("#entry-strategies .strategy-control-item").first().waitFor();
  const cards = await page.$$eval(".strategy-control-item", (items) =>
    items.map((item) => ({
      name: item.querySelector(".strategy-control-name")?.textContent.trim(),
      toggles: item.querySelectorAll('input[type="checkbox"]').length,
      label: item.querySelector('input[type="checkbox"]')?.getAttribute("aria-label"),
      extras: item.querySelectorAll(".toggle-state, .strategy-control-status-dot").length,
      chips: [...item.querySelectorAll(".strategy-control-chip")].map((chip) =>
        chip.textContent.trim()
      ),
    }))
  );
  assert.ok(cards.length > 0, "strategy cards render");
  for (const card of cards) {
    assert.equal(card.toggles, 1, `${card.name}: one toggle`);
    assert.equal(card.label, card.name, `${card.name}: the toggle is named by the strategy`);
    assert.equal(card.extras, 0, `${card.name}: no state dot or state word`);
    assert.ok(
      card.chips.every((chip) => !/^(Entry|Exit)$/.test(chip)),
      `${card.name}: no chip repeats the lane`
    );
  }
});

test("a config card draws no more columns than it has field groups", async () => {
  const empty = [];
  for (const id of TABS) {
    await page.click(`#subTabsContainer [data-tab-id="${id}"]`);
    await page.locator(`#${id}-tab`).waitFor();
    empty.push(
      ...(await page.$$eval(
        `#${id}-tab .config-card`,
        (cards, tab) =>
          cards
            .filter((card) => card.getClientRects().length > 0)
            .map((card) => ({
              card,
              columns: getComputedStyle(card)
                .gridTemplateColumns.split(" ")
                .filter((track) => parseFloat(track) > 0).length,
              groups: card.querySelectorAll(":scope > .config-group:not(.config-group-full)")
                .length,
            }))
            .filter(({ columns, groups }) => groups > 0 && columns > groups)
            .map(
              ({ card, columns, groups }) =>
                `${tab}: ${card.querySelector("h3")?.textContent.trim()} (${groups} groups, ${columns} columns)`
            ),
        id
      ))
    );
  }
  assert.deepEqual(empty, []);
});
