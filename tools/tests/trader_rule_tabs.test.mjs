// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Auto Trader rule tabs (Stop Loss, Trailing Stop, Take Profit, Time Rules,
 * DCA, Settings) and the strategies roster speak in plain fields and bare glyphs.
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
 * - A roster row shows whether its strategy is on only through its switch, named by the
 *   strategy. All states each row's kind once, in its Kind column; a kind view, which
 *   already names the kind, states it in no cell.
 * - An example figure (a SOL price or a percent) never ends its fraction in a zero, so
 *   one strip does not mix "+10%" with "+32.0%" or print "1.0000 SOL".
 * - A config card never draws more columns than it has field groups, so two lists
 *   (How DCA Works, Risk Warnings) share the width instead of leaving a third empty.
 * - A field's control (a number input or a switch such as Sell Part First) sits
 *   under its hint in the control row, never in the title row beside the label, so a
 *   switch field leaves no empty track under its hint.
 * - Every rule card's worked example carries the one section title, "How It Works".
 * - The time exit tab names the rule, each of its cards names the case it closes
 *   (losing or flat positions), and its duration field says what the duration does.
 * - Every sub-tab but the full-bleed strategy editor keeps the Stats tab's side gutter,
 *   and a newly chosen sub-tab opens at its top.
 * - A switch field (Sell Part First) sits in the same frame as the number fields
 *   beside it.
 * - A best or worst trade that is not known reads as one dash, never "— —".
 * - Each config card has its own title glyph, and no field borrows another card's
 *   (Trade Size once wore DCA's coins; Take Profit, Position Sizing and Entry
 *   Strategies shared one target).
 * - The number fields of one rule card share one width, whatever their unit or sign
 *   (the DCA card once drew four widths for four fields).
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const TABS = ["stop-loss", "trailing-stop", "take-profit", "time-rules", "dca", "general-settings"];

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
await page.waitForSelector(PAGE_READY);

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

test("a roster row states whether its strategy is on once, through its named switch", async () => {
  await page.click('#subTabsContainer [data-tab-id="strategies"]');
  await page.locator("#strategies-roster tbody tr[data-row-id]").first().waitFor();
  const rowsOfView = () =>
    page.$$eval("#strategies-roster tbody tr[data-row-id]", (items) =>
      items.map((item) => {
        const texts = [...item.querySelectorAll("td")].map((cell) => cell.textContent.trim());
        return {
          name: item.querySelector(".strategy-roster-name")?.textContent.trim(),
          toggles: item.querySelectorAll('input[type="checkbox"]').length,
          label: item.querySelector('input[type="checkbox"]')?.getAttribute("aria-label"),
          states: texts.filter((text) => /^(Enabled|Disabled|Active|Inactive)$/.test(text)),
          kinds: texts.filter((text) => /^(Entry|Exit)$/.test(text)),
        };
      })
    );
  const all = await rowsOfView();
  assert.ok(all.length > 0, "roster rows render");
  for (const row of all) {
    assert.equal(row.toggles, 1, `${row.name}: one switch`);
    assert.equal(row.label, row.name, `${row.name}: the switch is named by the strategy`);
    assert.deepEqual(row.states, [], `${row.name}: no state word`);
    assert.equal(row.kinds.length, 1, `${row.name}: All states the kind once`);
  }

  await page.click('#strategies-roster label:has(> input[data-seg-value="entry"])');
  await page
    .locator('#strategies-roster thead th[data-column-id="kind"]')
    .waitFor({ state: "detached" });
  for (const row of await rowsOfView()) {
    assert.deepEqual(
      row.kinds,
      [],
      `${row.name}: the Entry view names the kind, no cell repeats it`
    );
  }
  await page.click('#strategies-roster label:has(> input[data-seg-value="all"])');
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

test("every field control sits under its hint, never in the title row", async () => {
  const misplaced = [];
  for (const id of TABS) {
    await page.click(`#subTabsContainer [data-tab-id="${id}"]`);
    await page.locator(`#${id}-tab`).waitFor();
    misplaced.push(
      ...(await page.$$eval(
        `#${id}-tab .config-card > .config-group:not(.config-group-full)`,
        (groups, tab) =>
          groups
            .filter((group) => group.getClientRects().length > 0)
            .flatMap((group) => {
              const hint = group.querySelector(":scope > .config-hint");
              const name = group.querySelector(".config-label")?.textContent.trim();
              const inRow = group.querySelectorAll(".config-label-row :is(input, select)").length;
              const control = group.querySelector(
                ":scope > :is(.toggle, .input-group-enhanced, select, textarea)"
              );
              const issues = [];
              if (inRow) issues.push(`${tab}: ${name} has a control in its title row`);
              if (hint && control) {
                const below =
                  control.getBoundingClientRect().top >= hint.getBoundingClientRect().bottom - 1;
                if (!below) issues.push(`${tab}: ${name} control is not under its hint`);
              }
              return issues;
            }),
        id
      ))
    );
  }
  assert.deepEqual(misplaced, []);
});

test("every rule card's worked example is titled How It Works", async () => {
  const titles = [];
  for (const id of TABS) {
    await page.click(`#subTabsContainer [data-tab-id="${id}"]`);
    await page.locator(`#${id}-tab`).waitFor();
    titles.push(
      ...(await page.$$eval(
        `#${id}-tab .config-card > .visual-example > h4`,
        (heads, tab) => heads.map((head) => `${tab}: ${head.textContent.trim()}`),
        id
      ))
    );
  }
  // The DCA tab edits a ladder of steps and carries no single worked example.
  assert.deepEqual(
    [...new Set(titles.map((title) => title.split(":")[0]))],
    ["stop-loss", "trailing-stop", "take-profit", "time-rules"],
    "the rule cards carry worked examples"
  );
  assert.deepEqual(
    titles.filter((title) => !title.endsWith(": How It Works")),
    []
  );
});

test("the time exit's tab names the rule and each card names its case", async () => {
  await page.click('#subTabsContainer [data-tab-id="time-rules"]');
  await page.locator("#time-rules-tab").waitFor();
  const names = await page.evaluate(() => ({
    tab: document.querySelector('#subTabsContainer [data-tab-id="time-rules"]').textContent.trim(),
    cards: [...document.querySelectorAll("#time-rules-tab .card-header h3")].map((head) =>
      head.textContent.trim()
    ),
    hint: document
      .querySelector("#time-max-hold")
      .closest(".config-group")
      .querySelector(".config-hint")
      .textContent.trim(),
  }));
  assert.equal(names.tab, "Time Exit");
  assert.deepEqual(names.cards, ["Close Losing Positions", "Close Flat Positions"]);
  assert.match(names.hint, /^How long a position is held before the time exit checks its loss/);
});

test("every sub-tab keeps the Stats tab's side gutter", async () => {
  const gutters = {};
  for (const id of ["stats", ...TABS]) {
    await page.click(`#subTabsContainer [data-tab-id="${id}"]`);
    await page.locator(`#${id}-tab`).waitFor();
    gutters[id] = await page.evaluate((tab) => {
      const content = document.getElementById("trader-content").getBoundingClientRect();
      const first = document
        .querySelector(`#${tab}-tab :is(.trader-status-bar, .config-card)`)
        .getBoundingClientRect();
      return Math.round(first.left - content.left);
    }, id);
  }
  const expected = Object.fromEntries(Object.keys(gutters).map((id) => [id, gutters.stats]));
  assert.deepEqual(gutters, expected);
  assert.equal(gutters.stats, 24);
});

test("a newly chosen sub-tab opens at its top", async () => {
  await page.click('#subTabsContainer [data-tab-id="dca"]');
  await page.locator("#dca-tab").waitFor();
  const scrolled = await page.evaluate(() => {
    const content = document.getElementById("trader-content");
    content.scrollTop = content.scrollHeight;
    return content.scrollTop;
  });
  assert.ok(scrolled > 0, "the DCA tab is long enough to scroll");
  await page.click('#subTabsContainer [data-tab-id="stop-loss"]');
  await page.locator("#stop-loss-tab").waitFor();
  assert.equal(await page.$eval("#trader-content", (el) => el.scrollTop), 0);
});

test("Sell Part First sits in the frame of the number fields beside it", async () => {
  await page.click('#subTabsContainer [data-tab-id="stop-loss"]');
  await page.locator("#stop-loss-tab").waitFor();
  const frame = (selector) =>
    page.$eval(selector, (el) => {
      const style = getComputedStyle(el);
      return {
        border: `${style.borderTopWidth} ${style.borderTopStyle} ${style.borderTopColor}`,
        radius: style.borderTopLeftRadius,
        background: style.backgroundColor,
        height: Math.round(el.getBoundingClientRect().height),
      };
    });
  assert.deepEqual(
    await frame("#stop-loss-allow-partial-frame"),
    await frame("#stop-loss-threshold")
  );
});

test("an unknown best or worst trade reads as one dash", async () => {
  const stats = JSON.parse(readFileSync(`${FIXTURES_ROOT}/trader/trader_stats.json`, "utf8"));
  const statsContext = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const statsApi = createApiHandler(
    [await loadIndex("trader"), await loadIndex("shell")],
    "populated"
  );
  const statsHost = await serveDashboard(statsContext, {
    locale: "en",
    onApi: (request) =>
      request.url.pathname === "/api/trader/stats"
        ? { body: JSON.stringify({ ...stats, worst_trade_pct: null, worst_trade_token: null }) }
        : statsApi.onApi(request),
  });
  try {
    const statsPage = await statsContext.newPage();
    await statsPage.goto(`${statsHost.origin}/trader`);
    await statsPage.waitForSelector(PAGE_READY);
    await statsPage.click('#subTabsContainer [data-tab-id="stats"]');
    await statsPage.waitForSelector("#stats-extremes:not([hidden]) #best-trade.positive");
    const worst = await statsPage.$eval(
      "#worst-trade",
      (el) => el.closest(".stats-extreme").innerText
    );
    assert.equal(worst.match(/—/g)?.length, 1, `"${worst}" shows one dash`);
  } finally {
    await statsContext.close();
    await statsHost.close();
  }
});

test("every config card has its own glyph and no field borrows another card's", async () => {
  const report = await page.evaluate(() => {
    const glyph = (el) => [...(el?.classList ?? [])].find((name) => name.startsWith("icon-"));
    const cards = [...document.querySelectorAll(".trader-tab-content .config-card")]
      .map((card) => ({
        card,
        title: card.querySelector(".card-header h3")?.textContent.trim(),
        glyph: glyph(card.querySelector(".card-header h3 i")),
      }))
      .filter((card) => card.glyph);
    const borrowed = [];
    for (const { card, title } of cards) {
      for (const label of card.querySelectorAll(".config-label")) {
        const owner = cards.find(
          (other) => other.card !== card && other.glyph === glyph(label.querySelector("i"))
        );
        if (owner)
          borrowed.push(`${title}: ${label.textContent.trim()} wears ${owner.title}'s glyph`);
      }
    }
    return { glyphs: cards.map((card) => `${card.glyph} ${card.title}`), borrowed };
  });
  const glyphs = report.glyphs.map((entry) => entry.split(" ")[0]);
  assert.equal(new Set(glyphs).size, glyphs.length, report.glyphs.join(", "));
  assert.deepEqual(report.borrowed, []);
});

test("the number fields of one rule card share one width", async () => {
  for (const id of TABS) {
    await page.click(`#subTabsContainer [data-tab-id="${id}"]`);
    const panel = page.locator(`#${id}-tab`);
    await panel.waitFor();
    const cards = await panel.evaluate((root) =>
      [...root.querySelectorAll(".config-card")].map((card) =>
        [...card.querySelectorAll(".input-group-enhanced > .number-field")]
          .filter((field) => field.getClientRects().length > 0)
          .map((field) => ({
            id: field.querySelector("input").id,
            width: Math.round(field.getBoundingClientRect().width),
          }))
      )
    );
    for (const fields of cards.filter((fields) => fields.length > 1)) {
      const widths = new Set(fields.map((field) => field.width));
      assert.equal(widths.size, 1, `${id}: ${JSON.stringify(fields)}`);
    }
  }
});

test("a worked example's steps are inline icon-and-title grids that start at the left", async () => {
  for (const id of TABS) {
    await page.click(`#subTabsContainer [data-tab-id="${id}"]`);
    const panel = page.locator(`#${id}-tab`);
    await panel.waitFor();
    const report = await panel.evaluate((root) => {
      const timelines = [...root.querySelectorAll(".example-timeline")].map((timeline) => {
        const steps = [...timeline.querySelectorAll(".timeline-step")]
          .filter((step) => step.getClientRects().length > 0)
          .map((step) => {
            const icon = step.querySelector(".step-icon").getBoundingClientRect();
            const title = step.querySelector(".step-title").getBoundingClientRect();
            return {
              grid: getComputedStyle(step).display === "grid",
              sideBySide: icon.right <= title.left + 1 && Math.abs(icon.top - title.top) < 8,
            };
          });
        return { justify: getComputedStyle(timeline).justifyContent, steps };
      });
      return timelines;
    });
    for (const timeline of report) {
      assert.notEqual(timeline.justify, "space-between", `${id}: steps are not spread apart`);
      for (const step of timeline.steps) {
        assert.ok(step.grid, `${id}: a step is a grid`);
        assert.ok(step.sideBySide, `${id}: a step's icon sits inline with its title`);
      }
    }
  }
});
