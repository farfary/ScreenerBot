// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the brand face (`--font-brand`, Orbitron) belongs to the logo and the
 * standalone wordmarks, never to running text.
 *
 * The product name inside prose, hints, the splash line or a filesystem path is set in
 * the surrounding text font. A display face switched in mid-sentence reads as a broken
 * font, and inside a path it changes how a copyable value looks.
 *
 * - Only the wordmark selectors below draw with `--font-brand`.
 * - Every wordmark element renders the brand name alone (`shell-brand-name`).
 * - No script or template wraps the name in its own typeface span.
 * - On the rendered dashboard, prose naming the product keeps the text font while the
 *   header wordmark keeps the brand face.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import {
  classesIn,
  findOpenTags,
  loadMarkupSources,
  loadStylesheets,
  subjectCompound,
} from "../lib/dashboard_ui.mjs";

/** The wordmarks: each is a standalone element holding the brand name alone. */
const WORDMARKS = new Set([
  "brand-text",
  "login-brand",
  "splash-brand",
  "onboarding-brand",
  "lockscreen-brand",
  "settings-about-brand",
]);
/** Brand-face selectors that are not a wordmark class. Shrink only. */
const ALLOWED_SELECTORS = new Set([".header h1"]);

const [stylesheets, sources] = await Promise.all([loadStylesheets(), loadMarkupSources()]);

test("only the wordmarks draw with the brand face", () => {
  const offenders = [];
  for (const sheet of stylesheets) {
    for (const rule of sheet.rules) {
      if (!/font-family\s*:\s*var\(--font-brand\)/.test(rule.body)) continue;
      if (ALLOWED_SELECTORS.has(rule.selector)) continue;
      const subject = classesIn(subjectCompound(rule.selector));
      if (!subject.some((name) => WORDMARKS.has(name))) {
        offenders.push(`${sheet.path}:${rule.line} ${rule.selector}`);
      }
    }
  }
  assert.deepEqual(offenders, []);
});

test("every wordmark holds the brand name alone and nothing wraps the name in prose", () => {
  const offenders = [];
  for (const { path, source } of sources) {
    if (/brand-name\b/.test(source.replace(/shell-brand-name/g, ""))) {
      offenders.push(`${path}: wraps the product name in its own typeface`);
    }
    for (const tag of findOpenTags(source)) {
      const classes = (tag.tag.match(/class="([^"]*)"/)?.[1] ?? "").split(/\s+/);
      if (!classes.some((name) => WORDMARKS.has(name))) continue;
      if (!/data-l10n-id="shell-brand-name"/.test(tag.tag)) {
        offenders.push(`${path}: ${tag.tag} is a wordmark without the brand name alone`);
      }
    }
  }
  assert.deepEqual(offenders, []);
});

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

test("prose naming the product keeps the text font beside the brand wordmark", async (t) => {
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    locale: "en",
  });
  const api = createApiHandler([await loadIndex("home"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector(".brand-text");
  const faces = await page.evaluate(() => {
    const prose = document.createElement("p");
    prose.textContent = "Open the folder containing all ScreenerBot data";
    document.querySelector("main")?.append(prose);
    const first = (element) =>
      getComputedStyle(element).fontFamily.split(",")[0].trim().replace(/"/g, "");
    return {
      wordmark: first(document.querySelector(".brand-text")),
      prose: first(prose),
      wrapped: prose.querySelectorAll("span").length,
    };
  });
  assert.equal(faces.wordmark, "Orbitron");
  assert.notEqual(faces.prose, "Orbitron");
  assert.equal(faces.wrapped, 0, "nothing rewrites the product name in prose");
});
