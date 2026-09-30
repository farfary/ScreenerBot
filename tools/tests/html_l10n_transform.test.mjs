/**
 * Tests for the html-validate transformer that renders templates in English
 * (`tools/html/l10n_transform.mjs`), mirroring `localize_html` in src/i18n/html.rs.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { localizeTemplate } from "../html/l10n_transform.mjs";
import { readL10nAttributes } from "../i18n/catalogs.mjs";

test("an icon button gets its English aria-label", () => {
  const out = localizeTemplate('<button class="x" data-l10n-id="home-wallet-copy"><svg></svg></button>');
  assert.ok(out.includes('aria-label="Copy wallet address"'));
  assert.ok(out.includes("<svg></svg></button>"), "attribute-only messages keep children");
});

test("an image gets its alt text", () => {
  const out = localizeTemplate('<img src="a.svg" data-l10n-id="shell-brand-logo" />');
  assert.match(out, /alt="ScreenerBot"/);
  assert.ok(out.endsWith(" />"));
});

test("an element takes the message value as text content", () => {
  const out = localizeTemplate('<h1 data-l10n-id="home-portfolio-title"></h1>');
  assert.match(out, /<h1 [^>]*>[^<]+<\/h1>/);
});

test("placeholders resolve to valid values", () => {
  const out = localizeTemplate('<html lang="{{LANG}}" dir="{{DIR}}" data-x="{{OTHER_THING}}">');
  assert.equal(out, '<html lang="en" dir="ltr" data-x="0">');
});

test("a value replaces content as in localize_html and unknown ids are untouched", () => {
  const out = localizeTemplate(
    '<h1 data-l10n-id="home-portfolio-title"><span>old</span></h1><p data-l10n-id="no-such-id"><b>keep</b></p>'
  );
  assert.ok(!out.includes("old"));
  assert.ok(out.includes('<p data-l10n-id="no-such-id"><b>keep</b></p>'));
});

test("the dashboard runtime allowlist matches L10N_ATTRIBUTES", () => {
  const runtime = readFileSync(new URL("../../src/webserver/templates/scripts/core/i18n.js", import.meta.url), "utf8");
  const block = runtime.match(/ATTRIBUTE_ALLOWLIST = \[([^\]]*)\]/);
  assert.ok(block, "ATTRIBUTE_ALLOWLIST not found in core/i18n.js");
  const listed = [...block[1].matchAll(/"([^"]+)"/g)].map((match) => match[1]);
  assert.deepEqual(listed, readL10nAttributes());
});
