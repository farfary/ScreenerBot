/**
 * Responsive contract for compact dashboard labels.
 *
 * These labels sit beside longer field names in auto-fitting grids. Browser
 * zoom reduces the effective layout width without crossing a page breakpoint,
 * which used to let the labels shrink and split into two-line pills.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { chromium } from "playwright";

import { PAGES_ROOT, STYLES_ROOT } from "../lib/dashboard_ui.mjs";

const traderHtml = await readFile(resolve(PAGES_ROOT, "trader.html"), "utf8");
const styles = await Promise.all(
  ["foundation.css", "pages/trader/config_components.css"].map((path) =>
    readFile(resolve(STYLES_ROOT, path), "utf8")
  )
);
const longLabelProbe = `
  <section class="config-section">
    <div class="config-card">
      <div class="config-group">
        <div class="config-label-row">
          <label class="config-label"><i></i><span>Trailing Stop Activation %</span></label>
          <span class="config-badge config-badge-info">When to start</span>
        </div>
      </div>
      <div class="config-group">
        <div class="config-label-row">
          <label class="config-label"><i></i><span>Trailing Stop Distance %</span></label>
          <span class="config-badge config-badge-warning">Safety margin</span>
        </div>
      </div>
    </div>
  </section>
`;

const layoutCases = [
  { width: 1440, zoom: 1 },
  { width: 1280, zoom: 1.25 },
  { width: 1200, zoom: 1.5 },
  { width: 1200, zoom: 1.75 },
  { width: 1280, zoom: 2 },
];

test("Auto Trader compact labels stay on one line across desktop widths and zoom", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());

  const page = await browser.newPage();
  const violations = [];

  for (const { width, zoom } of layoutCases) {
    const layoutWidth = Math.round(width / zoom);
    await page.setViewportSize({ width: layoutWidth, height: 900 });
    await page.setContent(`
      <style>
        ${styles.join("\n")}
        body { margin: 0; }
        .trader-tab-content { display: block !important; }
        i { display: inline-block; width: 1em; height: 1em; }
      </style>
      <main>${longLabelProbe}${traderHtml}</main>
    `);

    const result = await page
      .locator(".config-label-row > .config-label > span, .config-label-row > .config-badge")
      .evaluateAll((labels) =>
        labels.map((label) => {
          const computed = getComputedStyle(label);
          const range = document.createRange();
          range.selectNodeContents(label);
          const lineTops = new Set(
            [...range.getClientRects()]
              .filter((rect) => rect.width > 0)
              .map((rect) => Math.round(rect.top))
          );
          const parsedLineHeight = Number.parseFloat(computed.lineHeight);
          const singleLineHeight =
            (Number.isFinite(parsedLineHeight)
              ? parsedLineHeight
              : Number.parseFloat(computed.fontSize) * 1.2) +
            Number.parseFloat(computed.paddingTop) +
            Number.parseFloat(computed.paddingBottom);
          return {
            text: label.textContent.trim(),
            lines: lineTops.size,
            height: label.getBoundingClientRect().height,
            singleLineHeight,
          };
        })
      );

    assert.ok(result.length >= 20, `only ${result.length} config labels were exercised`);
    for (const label of result) {
      if (label.lines !== 1 || label.height > label.singleLineHeight + 1) {
        violations.push(`${width}px @ ${zoom}x (${layoutWidth}px layout): ${label.text}`);
      }
    }
  }

  assert.deepEqual(violations, []);
});
