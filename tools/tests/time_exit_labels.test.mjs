// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the time-based exit has one name across the Auto Trader and Configuration.
 *
 * The Auto Trader tab is titled Time Exit, and the Configuration page groups the same
 * fields under their config category and labels them from the config metadata. In
 * every locale that category reads as the tab title, and in the source locale each
 * field label names the Time Exit, so neither page calls the rule something else.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { parse } from "@fluent/syntax";

import { LOCALES_ROOT } from "../i18n/catalogs.mjs";

const FIELDS = [
  "config-trader-time-override-duration",
  "config-trader-time-override-enabled",
  "config-trader-time-override-loss-threshold-percent",
  "config-trader-time-override-unit",
];
const LOCALES = readdirSync(LOCALES_ROOT).filter((name) =>
  statSync(join(LOCALES_ROOT, name)).isDirectory()
);

/** The plain text of each message value in `file` of `locale`, by id. */
function values(locale, file) {
  const resource = parse(readFileSync(join(LOCALES_ROOT, locale, file), "utf8"), {
    withSpans: false,
  });
  const out = new Map();
  for (const entry of resource.body) {
    if (entry.type !== "Message" || !entry.value) continue;
    out.set(
      entry.id.name,
      entry.value.elements
        .filter((element) => element.type === "TextElement")
        .map((element) => element.value)
        .join("")
        .trim()
    );
  }
  return out;
}

test("every locale names the Configuration group after the Time Exit tab", () => {
  for (const locale of LOCALES) {
    const title = values(locale, "trader.ftl").get("trader-time-title");
    const category = values(locale, "config.ftl").get("config-category-time-override");
    assert.ok(title && category, `${locale}: both labels exist`);
    assert.equal(
      category.toLocaleLowerCase(locale),
      title.toLocaleLowerCase(locale),
      `${locale}: the config group reads as the Auto Trader tab`
    );
  }
});

test("the Time Exit fields carry the Time Exit name", () => {
  const config = values("en", "config.ftl");
  const title = values("en", "trader.ftl").get("trader-time-title");
  for (const id of FIELDS) {
    assert.ok(config.get(id).includes(title), `${id} reads "${config.get(id)}"`);
  }
});
