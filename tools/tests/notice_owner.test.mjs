// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a notice reads the same everywhere, from one owner (`scripts/ui/notice.js`, its
 * CSS in `styles/components.css`).
 *
 * - `renderNotice` gives each variant its surface and default glyph, sets the glyph bare
 *   and inline with the title, leads the message with the glyph when there is no title,
 *   and renders at most one action.
 * - No dashboard source builds a block (`div`, `p`, `section`) whose class names a private
 *   warning, notice, banner, callout, info box or alert. A notice still drawn privately is
 *   listed, and the list only shrinks; classes that look alike but are not notices are
 *   listed with the reason.
 * - No stylesheet outside `components.css` draws a `.notice*` element.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import "./fixtures/document_shim.mjs";
import test from "node:test";
import assert from "node:assert/strict";

import {
  classTokens,
  classesIn,
  findOpenTags,
  loadMarkupSources,
  loadStylesheets,
  subjectCompound,
} from "../lib/dashboard_ui.mjs";

const { NOTICE_ICONS, renderNotice } = await import(
  new URL("../../src/webserver/templates/scripts/ui/notice.js", import.meta.url)
);

const NOTICE_LIKE = /(^|[-_])(warning|notice|banner|callout|info-box|alert)([-_]|$)/;
const OWNER_CLASS = /^notice(?:-|$)/;
const OWNER_STYLES = "components.css";

/** Notices still drawn by their own markup; each moves to the owner, and the list only shrinks. */
const PRIVATE_NOTICES = [
  "account-notice",
  "action-detail__warning",
  "config-import-warning-item",
  "force-stop-banner",
  "home-update-notice",
  "pdd-activity-notice",
  "quote-buy-notice",
  "quote-impact-warning",
  "quote-warning",
  "security-loading-notice",
  "slippage-warning-buttons",
  "slippage-warning-icon",
  "slippage-warning-text",
  "slippage-warning-title",
  "status-report-notice",
  "trade-action-slippage-warning",
  "trade-slippage-warning",
  "tx-failure-callout",
  "updates-history-notice",
];

/** Classes that match the pattern but draw no notice. */
const NOT_NOTICES = {
  "copy-warning-text": "inline warning text in a table cell",
  "config-info-box": "a key-value box",
  "feat-card-banner": "a token's banner image",
  "is-warning": "a tone modifier of a security row",
  "overview-banner-slot": "the slot of a token's banner image",
  "promo-callout-body": "promo capture narration",
  "promo-callout-eyebrow": "promo capture narration",
  "promo-callout-title": "promo capture narration",
  "summary-item-warning": "the loss tone of an example figure",
  "token-banner": "a token's banner image",
};

test("each variant has its surface, its default glyph and its role", () => {
  for (const [variant, icon] of Object.entries(NOTICE_ICONS)) {
    const role = variant === "error" ? "alert" : "note";
    assert.equal(
      renderNotice({ variant, title: "Heads up", message: "Body & more" }),
      `<div class="notice" data-variant="${variant}" role="${role}">` +
        `<div class="notice-heading"><i class="notice-icon ${icon}" aria-hidden="true"></i>` +
        `<span class="notice-title">Heads up</span></div>` +
        `<div class="notice-message">Body &amp; more</div></div>`
    );
  }
});

test("without a title the glyph leads the message on one row", () => {
  assert.equal(
    renderNotice({ variant: "warning", message: "Exits still run" }),
    '<div class="notice" data-variant="warning" role="note">' +
      '<div class="notice-heading"><i class="notice-icon icon-triangle-alert" aria-hidden="true"></i>' +
      '<span class="notice-message">Exits still run</span></div></div>'
  );
});

test("an unknown variant or glyph falls back to the info notice", () => {
  const html = renderNotice({ variant: "loud", icon: "fa fa-bell", message: "x" });
  assert.match(html, /^<div class="notice" data-variant="info" role="note">/);
  assert.match(html, /<i class="notice-icon icon-info" aria-hidden="true"><\/i>/);
});

test("trusted markup, an extra block and one action render in their slots", () => {
  const html = renderNotice({
    variant: "error",
    icon: "icon-wallet",
    title: "Fund the wallet",
    messageHtml: "Send <strong>SOL</strong>",
    extraHtml: '<span class="ti-address">abc</span>',
    action: { id: "retry", label: "Retry" },
  });
  assert.equal(
    html,
    '<div class="notice" data-variant="error" role="alert">' +
      '<div class="notice-heading"><i class="notice-icon icon-wallet" aria-hidden="true"></i>' +
      '<span class="notice-title">Fund the wallet</span></div>' +
      '<div class="notice-message">Send <strong>SOL</strong></div>' +
      '<div class="notice-extra"><span class="ti-address">abc</span></div>' +
      '<div class="notice-actions"><button type="button" class="btn btn-outline btn-sm" data-action="retry">Retry</button></div>' +
      "</div>"
  );
  assert.doesNotMatch(
    renderNotice({ message: "x", action: { id: 'x" onclick="y', label: "Go" } }),
    /<button/,
    "an action id outside [a-z0-9-] renders no button"
  );
});

test("no dashboard source draws a private notice block", async () => {
  const found = new Set();
  for (const { path, source } of await loadMarkupSources()) {
    if (!/\/(?:scripts|pages)\//.test(path)) continue;
    for (const { tag } of findOpenTags(source, "(?:div|p|section)")) {
      const { statics, conditional } = classTokens(tag);
      for (const name of [...statics, ...conditional]) {
        if (NOTICE_LIKE.test(name) && !OWNER_CLASS.test(name) && !(name in NOT_NOTICES)) {
          found.add(name);
        }
      }
    }
  }
  assert.deepEqual([...found].sort(), PRIVATE_NOTICES, "render it with renderNotice");
});

test("only components.css draws the notice", async () => {
  const found = [];
  for (const { path, rules } of await loadStylesheets()) {
    if (path === OWNER_STYLES) continue;
    for (const { selector, line } of rules) {
      if (classesIn(subjectCompound(selector)).some((name) => OWNER_CLASS.test(name))) {
        found.push(`${path}:${line} ${selector}`);
      }
    }
  }
  assert.deepEqual(found, []);
});
