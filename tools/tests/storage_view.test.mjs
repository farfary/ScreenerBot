// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Tests for the storage view of Settings > Data - the usage overview, the last cleanup and its
// failures, the storage limit choices, and the poll teardown when the dialog leaves the tab.

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import "./fixtures/i18n_en.mjs";

import {
  STORAGE_CATEGORY_LABELS,
  STORAGE_LIMIT_CHOICES_GB,
  createStorageView,
} from "../../src/webserver/templates/scripts/ui/settings/storage_view.js";

const MIB = 1024 * 1024;
const GIB = 1024 * MIB;

// Fluent wraps arguments in bidi isolates; assertions read the visible text.
const visible = (html) => html.replace(/[⁨⁩]/g, "");

function escapeHtml(value) {
  return String(value ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

const view = createStorageView({
  escapeHtml,
  formatBytes: (bytes) => `${bytes / MIB} MiB`,
  formatTimeAgo: () => "5 minutes ago",
  formatSizeAt: (value, { unit }) => `${value} ${unit.toUpperCase()}`,
});

function report(overrides = {}) {
  return {
    measured_at: 2_000_000_000,
    total_bytes: 300 * MIB,
    limit_bytes: 5 * GIB,
    limit_mode: "automatic",
    limit_megabytes: 0,
    automatic_limit_bytes: 5 * GIB,
    tier: "normal",
    disk: { free_bytes: 400 * GIB, total_bytes: 1000 * GIB },
    categories: Object.keys(STORAGE_CATEGORY_LABELS).map((category) => ({
      category,
      bytes: { market_data: 200 * MIB, wallet_keys: MIB, logs: 99 * MIB }[category] ?? 0,
    })),
    last_cleanup: null,
    running: false,
    ...overrides,
  };
}

test("the overview shows usage against the limit and one bar per category that takes space", () => {
  const html = visible(view.overviewHtml(report()));

  assert.match(html, /300 MiB of 5120 MiB/);
  assert.match(html, /Market data/);
  assert.match(html, /Wallet keys/);
  assert.match(html, /Logs/);
  assert.doesNotMatch(html, /Trading records/, "an empty category has no bar");
  assert.equal(html.match(/class="data-db-item"/g).length, 3);
  assert.match(html, /style="width: 100%"/, "the largest category fills its bar");
  assert.match(html, /409600 MiB of 1024000 MiB/);
  assert.match(html, /Not run yet/);
  assert.doesNotMatch(html, /Cleanup problems/);
});

test("the overview reports the last cleanup, its failures, a running pass and an unknown disk", () => {
  const done = visible(
    view.overviewHtml(
      report({
        disk: null,
        last_cleanup: {
          finished_at: 1_999_999_700,
          freed_bytes: 64 * MIB,
          deleted_files: 3,
          evicted_rows: 1200,
          failures: 2,
        },
      })
    )
  );
  assert.match(done, /5 minutes ago, 64 MiB freed/);
  assert.match(done, /Unknown/);
  assert.match(done, /data-fact-value warning">2 steps failed; they are retried on the next pass/);

  const running = visible(
    view.overviewHtml(report({ running: true, last_cleanup: { finished_at: 1, freed_bytes: 0, failures: 1 } }))
  );
  assert.match(running, /Cleaning up\.\.\./);
  assert.doesNotMatch(running, /Cleanup problems/, "failures of the previous pass hide while one runs");
});

test("category names are escaped", () => {
  const html = view.overviewHtml(report({ categories: [{ category: "<b>", bytes: MIB }] }));
  assert.doesNotMatch(html, /<b>/);
});

test("limit choices are automatic with its resolved size, then the fixed sizes in megabytes", () => {
  const options = view.limitOptions(report({ automatic_limit_bytes: 3 * GIB }));

  assert.deepEqual(
    options.map((option) => option.value),
    ["0", ...STORAGE_LIMIT_CHOICES_GB.map((gigabytes) => String(gigabytes * 1024))]
  );
  assert.equal(visible(options[0].label), "Automatic (3072 MiB)");
  assert.equal(options[1].label, "1 GB");
  assert.deepEqual(
    options.filter((option) => option.selected).map((option) => option.value),
    ["0"]
  );
});

test("a configured limit outside the choices is offered and selected", () => {
  const fixed = view.limitOptions(report({ limit_mode: "fixed", limit_megabytes: 2048 }));
  assert.deepEqual(
    fixed.filter((option) => option.selected).map((option) => option.value),
    ["2048"]
  );

  const custom = view.limitOptions(report({ limit_mode: "fixed", limit_megabytes: 700 }));
  const last = custom.at(-1);
  assert.equal(last.value, "700");
  assert.equal(last.label, "700 MiB");
  assert.equal(last.selected, true);
  assert.equal(custom.filter((option) => option.selected).length, 1);
});

test("the settings dialog stops the storage poll when it closes or leaves the data tab", () => {
  const dialog = readFileSync(
    new URL("../../src/webserver/templates/scripts/ui/settings_dialog.js", import.meta.url),
    "utf8"
  );
  assert.match(dialog, /import \{[^}]*\bteardownDataTab\b[^}]*\} from "\.\/settings\/data_tab\.js"/);
  assert.equal(dialog.match(/teardownDataTab\(\);/g)?.length, 2);
});
