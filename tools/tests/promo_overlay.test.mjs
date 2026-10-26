// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Promo cursor placement and travel, and every capture command failing with a reason when frames stop.

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { chromium } from "playwright";

const promoDir = new URL("../../src/webserver/templates/scripts/promo/", import.meta.url);
const sources = {
  "/scripts/promo/runtime.js": readFileSync(new URL("runtime.js", promoDir), "utf8"),
  "/scripts/promo/overlay.js": readFileSync(new URL("overlay.js", promoDir), "utf8"),
  "/scripts/promo/audio.js": readFileSync(new URL("audio.js", promoDir), "utf8"),
  "/scripts/promo/framing.js": readFileSync(new URL("framing.js", promoDir), "utf8"),
  // The runtime's dashboard dependencies, reduced to the surface it calls at load.
  "/scripts/core/router.js":
    'export async function loadPage() {}\nexport function getCurrentPage() { return "promo"; }',
  "/scripts/core/poller.js":
    "export function pauseAllPollers() {}\nexport function resumeAllPollers() {}\nexport function listPollers() { return []; }",
  "/scripts/core/sounds.js": "export function ensureAudioGraph() { return null; }",
  "/scripts/core/dom.js": "export function dirSign() { return 1; }",
};
const captureCss = readFileSync(
  new URL("../../src/webserver/templates/styles/promo/capture.css", import.meta.url),
  "utf8"
);

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

async function fixture(t) {
  const context = await browser.newContext({ viewport: { width: 800, height: 600 } });
  t.after(() => context.close());
  await context.route("https://promo.test/**", (route) => {
    const { pathname } = new URL(route.request().url());
    if (sources[pathname]) {
      return route.fulfill({ contentType: "application/javascript", body: sources[pathname] });
    }
    return route.fulfill({
      contentType: "text/html",
      body: `<html><head><style>
        :root { --z-promo-capture: 200000; }
        body { margin: 0; }
        #a, #b { position: absolute; width: 100px; height: 40px; }
        #a { left: 40px; top: 40px; }
        #b { left: 600px; top: 400px; }
        ${captureCss}
      </style></head><body><button id="a">A</button><button id="b">B</button></body></html>`,
    });
  });
  const page = await context.newPage();
  await page.goto("https://promo.test/");
  await page.evaluate(async () => {
    await import("/scripts/promo/runtime.js");
  });
  await page.evaluate(`window.promoCursorGeometry = ${cursorGeometry.toString()};`);
  return page;
}

/** Stop the document timeline the way a hidden window does: animations stop advancing. */
async function stopTimeline(page) {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Animation.enable");
  await cdp.send("Animation.setPlaybackRate", { playbackRate: 0 });
}

/**
 * Where the arrow tip is drawn, read from the painted shape itself (its fill
 * geometry, without the outline), plus the painted arrow height and the ripple
 * ring's centre. The tip is the hotspot every aimed point must land on.
 */
function cursorGeometry() {
  const el = document.getElementById("promoCaptureCursor");
  const shape = el.querySelector(".promo-cursor-arrow path").getBoundingClientRect();
  const ring = el.querySelector(".promo-cursor-ring").getBoundingClientRect();
  return {
    tip: { x: Math.round(shape.left), y: Math.round(shape.top) },
    height: shape.height,
    ring: { x: Math.round(ring.left + ring.width / 2), y: Math.round(ring.top + ring.height / 2) },
  };
}

test("a cursor placed after travel shows its tip where it was placed, with no animation left behind", async (t) => {
  const page = await fixture(t);
  const state = await page.evaluate(async () => {
    const { overlay } = window.__SB_PROMO__;
    await overlay.cursorShow({ selector: "#a" });
    await overlay.cursorTo({ selector: "#b", anchor: { x: 0.25, y: 0.75 } });
    await overlay.cursorHide();
    await overlay.cursorShow({ x: 50, y: 60 });
    const el = document.getElementById("promoCaptureCursor");
    const { tip, ring } = window.promoCursorGeometry();
    return {
      tip,
      ring,
      opacity: getComputedStyle(el).opacity,
      animations: el.getAnimations({ subtree: true }).length,
    };
  });
  assert.deepEqual(state, {
    tip: { x: 50, y: 60 },
    ring: { x: 50, y: 60 },
    opacity: "1",
    animations: 0,
  });
});

test("the press shrinks the arrow about its tip, which never leaves the target", async (t) => {
  const page = await fixture(t);
  const samples = await page.evaluate(async () => {
    const { overlay } = window.__SB_PROMO__;
    await overlay.cursorShow({ x: 600, y: 400 });
    const frames = [];
    let pressing = true;
    const press = overlay.cursorPress().finally(() => (pressing = false));
    while (pressing) {
      frames.push(window.promoCursorGeometry());
      await new Promise((resolve) => requestAnimationFrame(resolve));
    }
    await press;
    return frames;
  });
  assert.ok(samples.length > 3, `sampled ${samples.length} frames`);
  for (const { tip, ring } of samples) {
    assert.deepEqual(tip, { x: 600, y: 400 });
    assert.deepEqual(ring, { x: 600, y: 400 });
  }
  const heights = samples.map((sample) => sample.height);
  assert.ok(Math.min(...heights) < Math.max(...heights) - 1, `arrow heights ${heights.join(", ")}`);
});

test("cursor targets come from element geometry and travel time follows distance", async (t) => {
  const page = await fixture(t);
  const result = await page.evaluate(async () => {
    const { overlay } = window.__SB_PROMO__;
    await overlay.cursorShow({ selector: "#a" });
    const travel = await overlay.cursorTo({ selector: "#b", anchor: { x: 0.25, y: 0.75 } });
    const still = await overlay.cursorTo({ selector: "#b", anchor: { x: 0.25, y: 0.75 } });
    return {
      start: overlay.pointIn("#a"),
      travel,
      still,
      short: overlay.cursorTravelMs(50),
      long: overlay.cursorTravelMs(5000),
    };
  });
  assert.deepEqual(result.start, { x: 90, y: 60 });
  assert.equal(result.travel.x, 625);
  assert.equal(result.travel.y, 430);
  assert.ok(result.travel.duration >= 420 && result.travel.duration <= 800, result.travel.duration);
  assert.equal(result.still.duration, 0);
  assert.equal(result.short, 420);
  assert.equal(result.long, 800);
});

test("a click through the runtime presses at the target and reports where the cursor rests", async (t) => {
  const page = await fixture(t);
  const result = await page.evaluate(async () => {
    let clicked = false;
    document.getElementById("b").addEventListener("click", () => (clicked = true));
    await window.__SB_PROMO__.overlay.cursorShow({ selector: "#a" });
    const reply = await window.__SB_PROMO__.runCommand("ui.click", { selector: "#b" });
    return { reply, clicked, tip: window.promoCursorGeometry().tip };
  });
  assert.deepEqual(result, {
    reply: { selector: "#b", point: { x: 650, y: 420 } },
    clicked: true,
    tip: { x: 650, y: 420 },
  });
});

test(
  "overlay commands reject with the reason when the animation timeline stops",
  { timeout: 30000 },
  async (t) => {
    const page = await fixture(t);
    await page.evaluate(() => window.__SB_PROMO__.overlay.cursorShow({ selector: "#a" }));
    await stopTimeline(page);
    const outcomes = await page.evaluate(async () => {
      const run = (name, args) =>
        window.__SB_PROMO__.runCommand(name, args).then(
          () => "resolved",
          (error) => error.message
        );
      return Promise.all([
        run("overlay.cursorHide"),
        run("overlay.cursor", { selector: "#b" }),
        run("overlay.callout", { selector: "#b", title: "Callout" }),
      ]);
    });
    assert.match(outcomes[0], /^cursor fade-out did not finish within \d+ms: /);
    assert.match(outcomes[1], /^cursor travel did not finish within \d+ms: /);
    assert.match(outcomes[2], /^callout entrance did not finish within \d+ms: /);
  }
);

test(
  "frame-driven waits reject with the reason when the page stops painting",
  { timeout: 30000 },
  async (t) => {
    const page = await fixture(t);
    const message = await page.evaluate(async () => {
      // A hidden page delivers no animation frames at all.
      window.requestAnimationFrame = () => 0;
      const started = window.performance.now();
      try {
        await window.__SB_PROMO__.runCommand("wait.for", { selector: "#missing", timeout: 20000 });
        return "resolved";
      } catch (error) {
        return `${error.message} after ${Math.round(window.performance.now() - started)}ms`;
      }
    });
    const [, elapsed] = message.match(/after (\d+)ms$/) || [];
    assert.match(
      message,
      /^No animation frame within 4000ms: the renderer delivered no animation frames/
    );
    assert.ok(Number(elapsed) < 20000, message);
  }
);
