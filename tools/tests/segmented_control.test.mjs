// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: one segmented control (`scripts/ui/segmented.js`, its CSS in
 * `styles/components/form_controls.css`) and one performance range set.
 *
 * - `renderSegmented` draws a radiogroup of native radios, each painted on the label
 *   beside it, with a radio set of its own per render; tone, glyph, count and tooltip
 *   land in their slots and nothing else is accepted.
 * - `RANGE_OPTIONS` is 24H, 7D, 30D, All, shortest first.
 * - No dashboard source keeps a retired private segmented control, and the controls
 *   still drawn privately are listed; the list only shrinks.
 * - No source spells the control's markup by hand, and no stylesheet outside the owner
 *   paints its parts, apart from the keyboard ring and the toolbar's collapse.
 * - In the running dashboard, an arrow key moves the checked option and fires one
 *   change, the keyboard ring shows on the checked option, and every group holds only
 *   native radios.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { installEnglishI18n } from "../lib/dashboard_i18n.mjs";
import {
  classesIn,
  loadMarkupSources,
  loadStylesheets,
  subjectCompound,
} from "../lib/dashboard_ui.mjs";
import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

installEnglishI18n();
const { RANGE_OPTIONS, renderSegmented } = await import(
  new URL("../../src/webserver/templates/scripts/ui/segmented.js", import.meta.url)
);

const esc = (text) => String(text).replace(/[&<>"']/g, (char) => `&#${char.charCodeAt(0)};`);

const OWNER_SCRIPT = "src/webserver/templates/scripts/ui/segmented.js";
const OWNER_STYLES = "components/form_controls.css";
const PART = /^segmented-(?:option|label|icon|text|count)$/;

/** Private segmented controls that were moved to the owner; none may come back. */
const RETIRED = [
  "agent-perm-choice",
  "agent-perm-choices",
  "copy-seg",
  "copy-seg-btn",
  "dt-pagination-mode-btn",
  "dt-pagination-mode-toggle",
  "filter-tab",
  "pdd-act-filter",
  "pdd-act-filters",
  "stats-period-btn",
  "stats-period-seg",
  "table-toolbar-seg",
  "table-toolbar-seg-wrap",
  "table-toolbar-seg__btn",
  "time-preset-btn",
  "time-range-presets",
  "timeframe-btn",
  "timeframe-buttons",
  "trade-action-slippage-btn",
  "trade-action-slippage-presets",
];

/** One-of-many choices still drawn privately; each moves to the owner, and the list only shrinks. */
const PRIVATE_CONTROLS = [];

/** Rules outside the owner that may name a part of the control, with the reason. */
const PART_RULES = {
  "foundation.css": "the keyboard ring on the label of the focused radio",
  "ui/table_toolbar.css": "a narrow toolbar drops the text of an option that has a glyph",
};

const named = (source, name) => new RegExp(`(?<![\\w-])${name}(?![\\w-])`).test(source);

test("a group is native radios painted on their labels", () => {
  const html = renderSegmented(
    {
      name: "book",
      options: [
        { value: "live", label: "Live" },
        { value: "paper", label: "Paper", icon: "icon-test-tube-diagonal", count: 3 },
      ],
      value: "live",
      ariaLabel: "Trading book",
    },
    esc
  );
  const radios = [...html.matchAll(/<input type="radio" name="(seg-book-\d+)"/g)].map(
    (match) => match[1]
  );
  assert.equal(radios.length, 2);
  assert.equal(new Set(radios).size, 1, "one group shares one radio set");
  const [name] = radios;
  assert.equal(
    html,
    '<div class="segmented" role="radiogroup" aria-label="Trading book" data-seg="book">' +
      `<label class="segmented-option"><input type="radio" name="${name}" value="live" data-seg-value="live" checked>` +
      '<span class="segmented-label"><span class="segmented-text">Live</span></span></label>' +
      `<label class="segmented-option"><input type="radio" name="${name}" value="paper" data-seg-value="paper">` +
      '<span class="segmented-label"><i class="segmented-icon icon-test-tube-diagonal" aria-hidden="true"></i>' +
      '<span class="segmented-text">Paper</span><span class="segmented-count">3</span></span></label>' +
      "</div>"
  );
  assert.doesNotMatch(html, /aria-pressed|<button|role="radio"/);
});

test("two groups with one name never share a radio set", () => {
  const nameOf = (html) => html.match(/name="([^"]+)"/)[1];
  const options = [{ value: "a", label: "A" }];
  const first = renderSegmented({ name: "range", options }, esc);
  const second = renderSegmented({ name: "range", options }, esc);
  assert.notEqual(nameOf(first), nameOf(second));
});

test("tone, tooltip and the compact track land in their slots; anything else is dropped", () => {
  const html = renderSegmented(
    {
      name: "trading",
      options: [
        { value: "allow", label: "Allow", tone: "warning", tooltip: "Runs at once" },
        { value: "deny", label: "Off", tone: "danger" },
        { value: "loud", label: "Loud", tone: "success", icon: "fa fa-bell" },
        { value: "scroll", icon: "icon-arrow-down", tooltip: "Scroll" },
      ],
      value: "deny",
      compact: true,
    },
    esc
  );
  assert.match(html, /data-seg="trading" data-compact>/);
  assert.match(html, /<label class="segmented-option" data-tone="warning" title="Runs at once">/);
  assert.match(
    html,
    /<label class="segmented-option" data-tone="danger"><input [^>]*value="deny"[^>]* checked>/
  );
  assert.doesNotMatch(html, /data-tone="success"|fa-bell/);
  assert.match(
    html,
    /<input [^>]*value="scroll" data-seg-value="scroll" aria-label="Scroll">/,
    "an option drawn as a glyph alone is named by its tooltip"
  );
  assert.match(
    renderSegmented({ name: 'x"', options: [{ value: "<v>", label: "<b>" }] }, esc),
    /data-seg="x&#34;"[\s\S]*value="&#60;v&#62;"[\s\S]*&#60;b&#62;/
  );
});

test("the range set is 24H, 7D, 30D and All, shortest first", () => {
  assert.deepEqual(
    RANGE_OPTIONS.map(({ value, label, hours }) => ({ value, label, hours })),
    [
      { value: "24h", label: "24H", hours: 24 },
      { value: "7d", label: "7D", hours: 168 },
      { value: "30d", label: "30D", hours: 720 },
      { value: "all", label: "All", hours: null },
    ]
  );
  assert.ok(Object.isFrozen(RANGE_OPTIONS));
});

test("no retired private segmented control comes back; the private ones only shrink", async () => {
  const sheets = await loadStylesheets();
  const sources = [
    ...(await loadMarkupSources()).map(({ path, source }) => ({ path, source })),
    ...sheets.map(({ path, css }) => ({ path: `styles/${path}`, source: css })),
  ];
  const retired = [];
  const privateFound = new Set();
  for (const { path, source } of sources) {
    for (const name of RETIRED) if (named(source, name)) retired.push(`${path}: ${name}`);
    for (const match of source.matchAll(/(?<![\w-])[\w-]+-seg-btn(?![\w-])/g)) {
      retired.push(`${path}: ${match[0]}`);
    }
    for (const name of PRIVATE_CONTROLS) if (named(source, name)) privateFound.add(name);
  }
  assert.deepEqual(retired, [], "render it with renderSegmented");
  assert.deepEqual([...privateFound].sort(), PRIVATE_CONTROLS, "a member moved: shrink the list");
});

test("only the owner spells the control's markup", async () => {
  const offenders = [];
  for (const { path, source } of await loadMarkupSources()) {
    if (path === OWNER_SCRIPT) continue;
    if (/class="segmented(?:-option|-label)?[\s"]/.test(source)) offenders.push(path);
  }
  assert.deepEqual(offenders, []);
});

test("only the owner paints the control's parts", async () => {
  const found = [];
  for (const { path, rules } of await loadStylesheets()) {
    if (path === OWNER_STYLES) continue;
    for (const { selector, line } of rules) {
      if (!classesIn(subjectCompound(selector)).some((name) => PART.test(name))) continue;
      if (Object.hasOwn(PART_RULES, path)) continue;
      found.push(`${path}:${line} ${selector}`);
    }
  }
  assert.deepEqual(found, []);
});

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

/**
 * Focus the checked radio of `group`, press ArrowRight, and report the change events,
 * the newly checked value and the ring on its label.
 */
async function arrowRight(page, group) {
  await page.evaluate((selector) => {
    window.__segChanges = [];
    document.addEventListener(
      "change",
      (event) => {
        if (event.target.closest(selector)) window.__segChanges.push(event.target.value);
      },
      true
    );
  }, group);
  const before = await page.$eval(`${group} input:checked`, (input) => input.value);
  await page.focus(`${group} input:checked`);
  await page.keyboard.press("ArrowRight");
  const after = await page.$eval(`${group} input:checked`, (input) => input.value);
  const ring = await page.$eval(`${group} input:checked + .segmented-label`, (label) => {
    const style = getComputedStyle(label);
    return { style: style.outlineStyle, width: parseFloat(style.outlineWidth) };
  });
  const changes = await page.evaluate(() => window.__segChanges);
  return { before, after, ring, changes };
}

async function assertOnlyNativeRadios(page) {
  const groups = await page.$$eval(".segmented", (nodes) =>
    nodes.map((group) => ({
      seg: group.dataset.seg,
      role: group.getAttribute("role"),
      options: group.querySelectorAll(".segmented-option").length,
      radios: group.querySelectorAll('.segmented-option > input[type="radio"]').length,
      controls: group.querySelectorAll("input, button, select, [role], [aria-pressed]").length,
    }))
  );
  assert.ok(groups.length > 0, "the page renders a segmented control");
  for (const group of groups) {
    assert.equal(group.role, "radiogroup", group.seg);
    assert.ok(group.options > 0, group.seg);
    assert.equal(group.radios, group.options, `${group.seg}: one native radio per option`);
    assert.equal(group.controls, group.radios, `${group.seg}: nothing but the radios`);
  }
}

function assertMoved({ before, after, ring, changes }) {
  assert.notEqual(after, before, "ArrowRight checks the next option");
  assert.deepEqual(changes, [after], "one change event, carrying the new value");
  assert.notEqual(ring.style, "none", "the keyboard ring shows on the checked option");
  assert.ok(ring.width > 0, "the keyboard ring has a width");
}

test("the Copy range moves by keyboard and holds only radios", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("copy"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/copy`);
  await page.waitForSelector(PAGE_READY);
  await page.click("#copy-list-rows .copy-row");
  await page.waitForSelector('[data-seg="range"] input:checked');
  await assertOnlyNativeRadios(page);
  assertMoved(await arrowRight(page, '[data-seg="range"]'));
});

test("the Positions book moves by keyboard and holds only radios", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(
    [await loadIndex("positions"), await loadIndex("shell")],
    "populated"
  );
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/positions`);
  await page.waitForSelector(PAGE_READY);
  await page.waitForSelector('[data-seg="book"] input:checked');
  await assertOnlyNativeRadios(page);
  assertMoved(await arrowRight(page, '[data-seg="book"]'));
});
