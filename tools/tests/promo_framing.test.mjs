// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Promo still framing is independent of prior scrolling, theme and text direction.

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { chromium } from "playwright";

const source = readFileSync(
  new URL("../../src/webserver/templates/scripts/promo/framing.js", import.meta.url),
  "utf8"
);
const domSource = readFileSync(
  new URL("../../src/webserver/templates/scripts/core/dom.js", import.meta.url),
  "utf8"
);
const browser = await chromium.launch({ headless: true });
after(() => browser.close());

async function fixture(t, dir = "ltr") {
  const context = await browser.newContext({ viewport: { width: 700, height: 700 } });
  t.after(() => context.close());
  await context.route("https://promo.test/**", (route) =>
    route.fulfill({
      contentType: route.request().url().endsWith(".js") ? "application/javascript" : "text/html",
      body: route.request().url().endsWith(".js")
        ? route.request().url().endsWith("/core/dom.js")
          ? domSource
          : source
        : `
      <html dir="${dir}"><body>
      <style>
        body { margin: 0; }
        .tab-scroll-area { width: 350px; overflow-x: auto; scroll-padding-inline: 40px; }
        .row { display: flex; width: max-content; }
        .tab, .nav-more-toggle { flex: 0 0 100px; height: 36px; }
        .pane { width: 300px; height: 120px; overflow: auto; }
        .large { width: 800px; height: 900px; }
        #anchor { margin-top: 250px; }
        #hidden { visibility: hidden; }
      </style>
      <nav id="nav" class="tab-scroll-area"><div class="row">
        ${Array.from({ length: 10 }, (_, i) => `<a id="tab${i}" class="tab${i === 7 ? " active" : ""}">${i}</a>`).join("")}
      </div></nav>
      <main id="page" class="pane"><div class="large"><div id="anchor">Section</div></div></main>
      <div role="dialog"><div id="dialog" class="pane"><div class="large">Dialog</div></div>
        <div id="rail" class="pane"><div class="large">Independent pane</div></div></div>
      <div id="hidden" class="pane"><div class="large">Hidden pane</div></div>
      <textarea id="input" style="height:30px">${"text\n".repeat(50)}</textarea>
      </body></html>`,
    })
  );
  const page = await context.newPage();
  await page.goto("https://promo.test/");
  await page.evaluate(async () => {
    window.framing = await import("/framing.js");
  });
  return page;
}

for (const dir of ["ltr", "rtl"]) {
  test(`paired themes get identical tab, page and dialog positions in ${dir}`, async (t) => {
    const page = await fixture(t, dir);
    const snapshots = await page.evaluate(() => {
      const capture = (offset, theme) => {
        document.documentElement.dataset.theme = theme;
        for (const id of ["nav", "page", "dialog", "rail", "hidden", "input"]) {
          const el = document.getElementById(id);
          el.scrollTop = offset;
          el.scrollLeft = document.documentElement.dir === "rtl" ? -offset : offset;
        }
        const plan = window.framing.frameScrollers();
        return {
          mismatch: window.framing.framingMismatch(plan),
          scrolls: window.framing.describeFraming(plan),
          hidden: document.getElementById("hidden").scrollTop,
          input: document.getElementById("input").scrollTop,
        };
      };
      return [capture(50, "dark"), capture(600, "light")];
    });
    assert.equal(snapshots[0].mismatch, null);
    assert.equal(snapshots[1].mismatch, null);
    assert.deepEqual(snapshots[0].scrolls, snapshots[1].scrolls);
    for (const id of ["page", "dialog", "rail"]) {
      const scroller = snapshots[0].scrolls.find((entry) => entry.selector === `#${id}`);
      assert.equal(scroller.top, 0);
      assert.equal(scroller.left, 0);
    }
    const nav = snapshots[0].scrolls.find((entry) => entry.selector === "#nav");
    assert.equal(Math.abs(nav.left), 490);
    assert.equal(nav.selected.selector, "#tab7");
    assert.ok(snapshots[0].hidden > 0 && snapshots[1].hidden > 0);
    assert.ok(snapshots[0].input > 0 && snapshots[1].input > 0);

    const first = await page.evaluate(() => {
      document.querySelector(".tab.active").classList.remove("active");
      document.getElementById("tab0").classList.add("active");
      const plan = window.framing.frameScrollers();
      return {
        mismatch: window.framing.framingMismatch(plan),
        left: document.getElementById("nav").scrollLeft,
      };
    });
    assert.equal(first.mismatch, null);
    assert.equal(first.left, 0);
  });
}

test("shot overrides anchor only their own scroller and clamp to its bounds", async (t) => {
  const page = await fixture(t);
  const result = await page.evaluate(() => {
    const plan = window.framing.frameScrollers({
      scrolls: [
        { selector: "#page", top: "#anchor" },
        { selector: "#dialog", top: "bottom", left: "end" },
        { selector: "#rail", top: 10000 },
      ],
    });
    return {
      mismatch: window.framing.framingMismatch(plan),
      scrolls: window.framing.describeFraming(plan),
    };
  });
  assert.equal(result.mismatch, null);
  const byId = new Map(result.scrolls.map((entry) => [entry.selector, entry]));
  assert.equal(byId.get("#page").top, 250);
  assert.equal(byId.get("#dialog").top, 780);
  assert.equal(byId.get("#dialog").left, 500);
  assert.equal(byId.get("#rail").top, 780);
  assert.equal(byId.get("#nav").top, 0);
  await assert.rejects(
    page.evaluate(() =>
      window.framing.frameScrollers({
        scrolls: [{ selector: "#dialog", top: "#anchor" }],
      })
    ),
    /outside its scroller/
  );
  await assert.rejects(
    page.evaluate(() =>
      window.framing.frameScrollers({
        scrolls: [{ selector: "#missing", top: 0 }],
      })
    ),
    /not visible/
  );
});

test("verification detects scroll motion, layout changes and replaced content", async (t) => {
  const page = await fixture(t);
  const reasons = await page.evaluate(() => {
    const reasons = [];
    let plan = window.framing.frameScrollers();
    document.getElementById("dialog").scrollTop = 10;
    reasons.push(window.framing.framingMismatch(plan));
    plan = window.framing.frameScrollers();
    document.getElementById("nav").style.width = "320px";
    reasons.push(window.framing.framingMismatch(plan));
    plan = window.framing.frameScrollers();
    document.getElementById("rail").remove();
    reasons.push(window.framing.framingMismatch(plan));
    return reasons;
  });
  assert.match(reasons[0], /scroll position/);
  assert.match(reasons[1], /layout changed/);
  assert.match(reasons[2], /removed or hidden/);
});

test("folded navigation reveals its active More entry", async (t) => {
  const page = await fixture(t);
  const result = await page.evaluate(() => {
    document.getElementById("tab7").className = "nav-more-toggle active";
    const plan = window.framing.frameScrollers();
    return {
      mismatch: window.framing.framingMismatch(plan),
      scrolls: window.framing.describeFraming(plan),
    };
  });
  assert.equal(result.mismatch, null);
  const nav = result.scrolls.find((entry) => entry.selector === "#nav");
  assert.equal(nav.left, 490);
  assert.equal(nav.selected.selector, "#tab7");
});
