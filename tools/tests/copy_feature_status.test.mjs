// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Guards the released Copy Trading presentation and its feature-map fixtures.

import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import test from "node:test";

const root = new URL("../../", import.meta.url);
const read = (path) => readFileSync(new URL(path, root), "utf8");

test("Copy Trading carries no release-stage badge or feature-label request", () => {
  const markup = read("src/webserver/templates/pages/copy.html");
  const script = read("src/webserver/templates/scripts/pages/copy.js");
  const styles = read("src/webserver/templates/styles/pages/copy.css");
  assert.doesNotMatch(
    markup + script + styles,
    /copy-feature-label|copy-strip-feature|copy-page-beta/
  );
  assert.doesNotMatch(script, /\/api\/features/);
  for (const entry of readdirSync(new URL("locales/", root), { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    assert.doesNotMatch(read(`locales/${entry.name}/copy.ftl`), /^copy-page-beta\s*=/m);
  }
});

test("every dashboard feature fixture presents copy trading as available", () => {
  const fixtures = new URL("tools/tests/fixtures/dashboard/", root);
  let checked = 0;
  for (const entry of readdirSync(fixtures, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const files = readdirSync(new URL(`${entry.name}/`, fixtures));
    if (!files.includes("features.json")) continue;
    const features = JSON.parse(read(`tools/tests/fixtures/dashboard/${entry.name}/features.json`));
    assert.equal(features.trading.copy_wallet, "available", entry.name);
    checked++;
  }
  assert.ok(checked >= 3, "feature fixtures must be covered");
});
