// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the shared contract of the main tab row (`#navTabs`) and the page sub-tab row
 * (`#subTabsContainer`).
 *
 * - Alignment: the brand mark, the first main label and the first sub label start on the
 *   same edge (the shell gutter) at 1200, 1366 and 1920 px, mirrored in a right-to-left
 *   locale.
 * - Selected state: both rows mark the selected tab with the `--primary-color` label and
 *   a 2px bar, with no background; hover never draws the bar; idle and selected labels
 *   reach 4.5:1 against their row in both themes.
 * - Focus: a keyboard-focused main tab, sub-tab and page chevron draw an inset ring; a
 *   mouse click draws none.
 * - Keyboard: the sub-row's arrows move focus without selecting, Enter selects, and the
 *   arrows follow the reading direction.
 * - Semantics: the sub-row is a tablist named by its page's main tab and controlling the
 *   page, which is its tabpanel; the main `nav` is named; main tab glyphs are hidden from
 *   assistive technology.
 * - One owner of the main tab shape: the test host renders the golden file the server's
 *   `nav_tab_html` is pinned to, and the settings dialog's navigation apply path keeps
 *   every tab's URL, id, label and current mark.
 * - Transition: the sub-row keeps its slot between two pages that both have one.
 * - Overflow: only the tab-row owner (`ui/tab_bar.css`) decides how a tab strip
 *   overflows. A strip that sets its own `overflow` shows a scrollbar under tabs that
 *   fit, since its tabs' indicator bars reach its bottom edge.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { PAGE_READY, navTabHtml, serveDashboard } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { loadStylesheets } from "../lib/dashboard_ui.mjs";

const TAB_ROW_OWNER = "ui/tab_bar.css";
/** A rule addressing a tab strip: a class that names tabs, a tab list or a tab bar. */
const TAB_STRIP =
  /(?:^|[\s>+~,(])\.[\w-]*tabs?(?:-(?:bar|row|list|strip|nav|header))?(?![\w-])/;
const SETS_OVERFLOW = /(?:^|;)\s*overflow(?:-x|-y|-inline|-block)?\s*:/;
/**
 * Strips that still set their own overflow, with the reason each cannot use the owner
 * yet. The list only shrinks.
 */
const OWN_OVERFLOW_ALLOWLIST = new Map([
  [
    "ui/dialog_tab_bar.css .details-tab-list",
    "the dialog tab family keeps its own sizes and moves onto the owner as one unit",
  ],
  [
    "ui/search_dialog.css .search-tabs",
    "the row also holds the search-kinds hint pinned to its end edge, which a scroll strip would carry out of view",
  ],
]);

const browser = await chromium.launch({ headless: true });
const hosts = [];
after(async () => {
  await browser.close();
  await Promise.all(hosts.map((host) => host.close()));
});

const indexes = new Map(
  await Promise.all(
    ["shell", "tokens", "trader"].map(async (name) => [name, await loadIndex(name)])
  )
);

/** A fresh session on `id` with its sub-tab row rendered and transitions frozen. */
async function openPage(id, { locale = "en", theme = "dark", width = 1440 } = {}) {
  const context = await browser.newContext({ viewport: { width, height: 900 }, locale });
  // The page's own index first: the first match wins.
  const order = [id, ...[...indexes.keys()].filter((name) => name !== id && name !== "shell")];
  const api = createApiHandler(
    [...order, "shell"].map((name) => indexes.get(name)),
    "populated",
    {
      theme,
    }
  );
  const host = await serveDashboard(context, { locale, onApi: api.onApi });
  hosts.push(host);
  const page = await context.newPage();
  page.setDefaultTimeout(20000);
  await page.goto(`${host.origin}/${id}?theme=${theme}`);
  await page.waitForSelector(PAGE_READY);
  await page.waitForSelector("#subTabsContainer .sub-tab.active");
  await page.evaluate(async () => {
    await document.fonts.ready;
    const style = document.createElement("style");
    style.textContent = "*, *::before, *::after { transition: none !important; }";
    document.head.append(style);
  });
  return { context, page };
}

/** WCAG contrast ratio of two `rgb()`/`rgba()` colours, the first composited on the second. */
function contrast(foreground, background) {
  const parse = (value) => value.match(/[\d.]+/g).map(Number);
  const [br, bg, bb] = parse(background);
  const [fr, fg, fb, fa = 1] = parse(foreground);
  const mix = [fr * fa + br * (1 - fa), fg * fa + bg * (1 - fa), fb * fa + bb * (1 - fa)];
  const luminance = (rgb) => {
    const [r, g, b] = rgb.map((channel) => {
      const c = channel / 255;
      return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  };
  const [light, dark] = [luminance(mix), luminance([br, bg, bb])].sort((a, b) => b - a);
  return (light + 0.05) / (dark + 0.05);
}

for (const locale of ["en", "ar"]) {
  test(`the brand, the main row and the sub-row start on one edge (${locale})`, async () => {
    for (const width of [1200, 1366, 1920]) {
      const { context, page } = await openPage("tokens", { locale, width });
      const edges = await page.evaluate(() => {
        const rtl = document.documentElement.dir === "rtl";
        const viewport = document.documentElement.clientWidth;
        const start = (left, right) => Math.round((rtl ? viewport - right : left) * 10) / 10;
        const brand = document.querySelector("#headerBrand .brand-mark").getBoundingClientRect();
        const main = document
          .querySelector("#navTabs > a.tab:not([hidden]) span")
          .getBoundingClientRect();
        const sub = document.querySelector("#subTabsContainer .sub-tab");
        const subBox = sub.getBoundingClientRect();
        const pad = parseFloat(getComputedStyle(sub).paddingInlineStart);
        return {
          brand: start(brand.left, brand.right),
          main: start(main.left, main.right),
          sub: start(subBox.left + pad, subBox.right - pad),
        };
      });
      assert.ok(Math.abs(edges.main - edges.brand) <= 1, `${width}px: ${JSON.stringify(edges)}`);
      assert.ok(Math.abs(edges.sub - edges.brand) <= 1, `${width}px: ${JSON.stringify(edges)}`);
      await context.close();
    }
  });
}

for (const theme of ["dark", "light"]) {
  test(`both rows mark the selected tab one way and stay legible (${theme})`, async () => {
    const { context, page } = await openPage("tokens", { theme });
    await page.hover("#subTabsContainer .sub-tab:not(.active)");
    const state = await page.evaluate(() => {
      const probe = document.createElement("span");
      probe.style.color = "var(--primary-color)";
      document.body.append(probe);
      const primary = getComputedStyle(probe).color;
      probe.remove();
      const look = (element) => {
        const style = getComputedStyle(element);
        const bar = getComputedStyle(element, "::after");
        return {
          color: style.color,
          background: style.backgroundColor,
          bar: bar.content === "none" ? null : { height: bar.height, color: bar.backgroundColor },
        };
      };
      const indicator = getComputedStyle(document.querySelector(".nav-tabs-indicator"));
      return {
        primary,
        mainSurface: getComputedStyle(document.querySelector(".header-row-2")).backgroundColor,
        subSurface: getComputedStyle(document.getElementById("subTabsContainer")).backgroundColor,
        main: look(document.querySelector("#navTabs > a.tab.active")),
        mainIdle: look(document.querySelector("#navTabs > a.tab:not(.active)")),
        indicator: {
          height: indicator.height,
          color: indicator.backgroundColor,
          opacity: indicator.opacity,
        },
        sub: look(document.querySelector("#subTabsContainer .sub-tab.active")),
        subIdle: look(
          document.querySelector("#subTabsContainer .sub-tab:not(.active):not(:hover)")
        ),
        subHovered: look(document.querySelector("#subTabsContainer .sub-tab:hover")),
      };
    });
    const none = "rgba(0, 0, 0, 0)";

    assert.equal(state.main.color, state.primary, "selected main label");
    assert.equal(state.main.background, none, "selected main tab has no background");
    assert.deepEqual(
      state.indicator,
      { height: "2px", color: state.primary, opacity: "1" },
      "main bar"
    );
    assert.equal(state.sub.color, state.primary, "selected sub label");
    assert.equal(state.sub.background, none, "selected sub-tab has no background");
    assert.deepEqual(state.sub.bar, { height: "2px", color: state.primary }, "sub bar");
    assert.equal(state.subHovered.bar, null, "hover draws no bar");
    assert.equal(state.subHovered.background, none, "hover draws no background");

    for (const [name, color, surface] of [
      ["idle main", state.mainIdle.color, state.mainSurface],
      ["selected main", state.main.color, state.mainSurface],
      ["idle sub", state.subIdle.color, state.subSurface],
      ["selected sub", state.sub.color, state.subSurface],
    ]) {
      const ratio = contrast(color, surface);
      assert.ok(ratio >= 4.5, `${name}: ${color} on ${surface} is ${ratio.toFixed(2)}:1`);
    }
    await context.close();
  });
}

test("keyboard focus draws an inset ring in both rows and on the chevrons; a click draws none", async () => {
  // At 800px the Tokens sub-row overflows, so its page chevrons are reachable.
  const { context, page } = await openPage("tokens", { width: 800 });
  const ring = () => page.evaluate(() => getComputedStyle(document.activeElement).boxShadow);
  const focused = () =>
    page.evaluate(() => {
      const element = document.activeElement;
      return element.dataset.page ?? element.dataset.tabId ?? element.className;
    });

  await page.focus("#navTabs > a.tab");
  await page.keyboard.press("Tab");
  assert.ok(await page.evaluate(() => document.activeElement.matches("#navTabs > a.tab")));
  assert.match(await ring(), /inset/, `main tab ${await focused()}`);

  await page.focus("#subTabsContainer .sub-tab.active");
  await page.keyboard.press("End");
  assert.match(await ring(), /inset/, `sub-tab ${await focused()}`);

  await page.keyboard.press("Tab");
  assert.ok(
    await page.evaluate(() => document.activeElement.matches(".tab-scroll-btn")),
    `Tab after the sub-row reaches a chevron, not ${await focused()}`
  );
  assert.match(await ring(), /inset/, "chevron");

  await page.click("#subTabsContainer .sub-tab:not(.active)");
  assert.equal(await ring(), "none", "a clicked sub-tab");
  await context.close();
});

for (const { locale, next } of [
  { locale: "en", next: "ArrowRight" },
  { locale: "ar", next: "ArrowLeft" },
]) {
  test(`the sub-row arrows move focus and Enter selects (${locale})`, async () => {
    const { context, page } = await openPage("tokens", { locale });
    const read = () =>
      page.evaluate(() => ({
        active: document.querySelector("#subTabsContainer .sub-tab.active").dataset.tabId,
        focused: document.activeElement.dataset.tabId,
        stops: [...document.querySelectorAll("#subTabsContainer .sub-tab[tabindex='0']")].map(
          (tab) => tab.dataset.tabId
        ),
        hash: window.location.hash,
      }));
    const ids = await page.$$eval("#subTabsContainer .sub-tab", (tabs) =>
      tabs.map((tab) => tab.dataset.tabId)
    );
    await page.focus("#subTabsContainer .sub-tab.active");
    const before = await read();
    const following = ids[(ids.indexOf(before.active) + 1) % ids.length];

    await page.keyboard.press(next);
    const moved = await read();
    assert.equal(moved.focused, following, "the arrow moves to the next tab in reading order");
    assert.equal(moved.active, before.active, "the arrow does not select");
    assert.equal(moved.hash, before.hash, "the arrow does not navigate");
    assert.deepEqual(moved.stops, [following], "the tab stop follows focus");

    await page.keyboard.press("Enter");
    await page.waitForFunction(
      (id) => document.querySelector("#subTabsContainer .sub-tab.active")?.dataset.tabId === id,
      following
    );
    await context.close();
  });
}

test("the rows carry their accessible names and roles", async () => {
  const { context, page } = await openPage("tokens");
  const state = await page.evaluate(() => {
    const row = document.getElementById("subTabsContainer");
    const active = row.querySelector(".sub-tab.active");
    const panel = document.getElementById("page-tokens");
    const label = document.getElementById(row.getAttribute("aria-labelledby"));
    return {
      role: row.getAttribute("role"),
      labelledBy: label?.matches("#navTabs > a.tab") ? label.dataset.page : null,
      controls: [...row.querySelectorAll(".sub-tab")].map((tab) =>
        tab.getAttribute("aria-controls")
      ),
      panel: [panel.getAttribute("role"), panel.getAttribute("aria-labelledby")],
      activeId: active.id,
      navName: document.querySelector("nav.header-row-2").getAttribute("aria-label"),
      glyphs: [...document.querySelectorAll("#navTabs > a.tab i")].map((glyph) =>
        glyph.getAttribute("aria-hidden")
      ),
    };
  });
  assert.equal(state.role, "tablist");
  assert.equal(state.labelledBy, "tokens", "the tablist is named by its page's main tab");
  assert.ok(state.controls.every((value) => value === "page-tokens"));
  assert.deepEqual(state.panel, ["tabpanel", state.activeId]);
  assert.ok(state.navName?.trim(), "the main nav has a name");
  assert.ok(state.glyphs.length > 0 && state.glyphs.every((value) => value === "true"));
  await context.close();
});

test("the test host renders the server's golden nav tab", () => {
  const golden = readFileSync(`${FIXTURES_ROOT}/shell/nav_tab.html`, "utf8").trimEnd();
  assert.equal(navTabHtml("tokens", "icon-coins", "tokens", true), golden);
});

test("applying Settings > Navigation keeps every tab's URL, id, label and current mark", async () => {
  const { context, page } = await openPage("tokens");
  const result = await page.evaluate(async () => {
    const { SettingsDialog } = await import("/scripts/ui/settings_dialog.js");
    const rendered = [...document.querySelectorAll("#navTabs > a.tab")].map((tab) => ({
      id: tab.dataset.page,
      icon: tab.querySelector("i").className,
    }));
    const [first, second, ...rest] = rendered;
    const reordered = [second, first, ...rest];
    const apply = (tabs) =>
      SettingsDialog.prototype._applyNavigationSettings.call({
        settings: { dashboard: { navigation: { overflow: "scroll", tabs } } },
      });
    const removed = reordered.at(-1).id;
    apply(reordered.map((tab, order) => ({ ...tab, order, enabled: tab.id !== removed })));
    const without = [...document.querySelectorAll("#navTabs > a.tab")].map(
      (tab) => tab.dataset.page
    );
    apply(reordered.map((tab, order) => ({ ...tab, order, enabled: true })));
    return {
      expected: reordered.map((tab) => tab.id),
      removed,
      without,
      tabs: [...document.querySelectorAll("#navTabs > a.tab")].map((tab) => {
        const label = tab.querySelector("span");
        return {
          id: tab.dataset.page,
          href: tab.getAttribute("href"),
          elementId: tab.id,
          l10n: label.getAttribute("data-l10n-id"),
          text: label.textContent.trim(),
          current: tab.getAttribute("aria-current"),
        };
      }),
      moreLast: document.querySelector("#navTabs > a.tab:last-of-type ~ .nav-more") !== null,
    };
  });
  assert.ok(!result.without.includes(result.removed), "a disabled tab leaves the row");
  assert.deepEqual(
    result.tabs.map((tab) => tab.id),
    result.expected,
    "the row follows the saved order, with the re-enabled tab cloned back"
  );
  for (const tab of result.tabs) {
    assert.equal(tab.href, `/${tab.id}`);
    assert.equal(tab.elementId, `nav-tab-${tab.id}`);
    assert.equal(tab.l10n, `nav-${tab.id}`);
    assert.ok(tab.text, `${tab.id} keeps its label`);
  }
  assert.deepEqual(
    result.tabs.filter((tab) => tab.current === "page").map((tab) => tab.id),
    ["tokens"],
    "the current page stays marked"
  );
  assert.ok(result.moreLast, "the More control stays after the tabs");
  await context.close();
});

test("the sub-row keeps its slot between two pages that both have one", async () => {
  const { context, page } = await openPage("tokens");
  const visit = async (id) => {
    await page.click(`#navTabs > a.tab[data-page="${id}"]`);
    await page.waitForSelector(`${PAGE_READY} #page-${id}`, { state: "attached" });
    await page.waitForSelector(`#subTabsContainer[data-page="${id}"] .sub-tab.active`);
  };
  // The first visit registers the Auto Trader bar; the second is the one measured.
  await visit("trader");
  await visit("tokens");
  await page.evaluate(() => {
    const wrapper = document.getElementById("subTabsContainer").parentElement;
    window.__subRowHeights = [];
    const sample = () => {
      window.__subRowHeights.push(wrapper.getBoundingClientRect().height);
      if (!window.__subRowSampled) requestAnimationFrame(sample);
    };
    sample();
  });
  await visit("trader");
  const heights = await page.evaluate(() => {
    window.__subRowSampled = true;
    return window.__subRowHeights;
  });
  assert.ok(heights.length > 1, "the transition was sampled");
  assert.ok(Math.min(...heights) > 0, `the row collapsed: ${heights.join(", ")}`);
  await context.close();
});

test("no stylesheet but the tab-row owner gives a tab strip its own overflow", async () => {
  const found = [];
  for (const { path, rules } of await loadStylesheets()) {
    if (path === TAB_ROW_OWNER) continue;
    for (const { selector, body, line } of rules) {
      if (!TAB_STRIP.test(selector) || !SETS_OVERFLOW.test(body)) continue;
      if (!OWN_OVERFLOW_ALLOWLIST.has(`${path} ${selector}`)) found.push(`${path}:${line} ${selector}`);
    }
  }
  assert.deepEqual(found, [], "a tab strip scrolls through the tab scroll strip in ui/tab_bar.css");
});
