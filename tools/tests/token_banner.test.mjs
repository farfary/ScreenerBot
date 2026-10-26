// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a token shows a banner only when it has one that is not its logo.
 *
 * A provider can report the logo as the header image; enlarged into the Overview
 * banner, a featured card or the Links media, it showed the logo blurred as if it
 * were a banner. The comparison ignores the query and fragment that only size or key
 * one picture on a CDN.
 *
 * Run with `npm run test:js`.
 */

import assert from "node:assert/strict";
import test from "node:test";
import "./fixtures/i18n_en.mjs";

globalThis.window = { addEventListener() {} };
// core/utils.js registers document listeners when it is imported.
globalThis.document = { addEventListener() {}, readyState: "complete" };

const { resolveTokenBannerUrl } = await import("../../src/webserver/templates/scripts/core/utils.js");

const LOGO = "https://cdn.example.com/cms/images/abc123?width=128&height=128";

test("an absent banner is none", () => {
  assert.equal(resolveTokenBannerUrl(null), null);
  assert.equal(resolveTokenBannerUrl({ logo_url: LOGO }), null);
  assert.equal(resolveTokenBannerUrl({ logo_url: LOGO, header_image_url: "  " }), null);
});

test("a banner that is the logo is none, from every field that names either", () => {
  const same = [
    { logo_url: LOGO, header_image_url: LOGO },
    { image_url: LOGO, header_image_url: "https://cdn.example.com/cms/images/abc123?width=1500" },
    { logo: LOGO, banner: " https://cdn.example.com/cms/images/abc123/#top " },
    { icon: "https://CDN.example.com/cms/images/abc123", header: LOGO },
  ];
  for (const token of same) assert.equal(resolveTokenBannerUrl(token), null, JSON.stringify(token));
});

test("a distinct banner is shown", () => {
  const banner = "https://cdn.example.com/cms/images/def456?width=1500&height=500";
  assert.equal(resolveTokenBannerUrl({ logo_url: LOGO, header_image_url: banner }), banner);
  assert.equal(resolveTokenBannerUrl({ header_image_url: banner }), banner);
  // A path that names nothing keeps its query, which then names the image.
  assert.equal(
    resolveTokenBannerUrl({
      logo_url: "https://img.example.com/?url=logo.png",
      banner: "https://img.example.com/?url=banner.png",
    }),
    "https://img.example.com/?url=banner.png"
  );
});
