/**
 * Tests for the Home update notice (`pages/home/update_notice.js`).
 *
 * Only the pure description is exercised: which updater states show the notice,
 * what it says, and where its action leads. Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import "./fixtures/i18n_en.mjs";

import { describeUpdateNotice } from "../../src/webserver/templates/scripts/pages/home/update_notice.js";

// Fluent wraps arguments in bidi isolates; assertions read the visible text.
const visible = (text) => String(text).replace(/[⁨⁩]/g, "");

const Utils = {
  formatBytes: (value) => `${value} B`,
  formatPercentValue: (value) => `${value}%`,
};

const update = { version: "0.2.13", kind: "core" };

function state(overrides) {
  return {
    phase: "up_to_date",
    available_update: null,
    download_progress: {},
    blocked_reason: null,
    self_install: true,
    current_version: "0.2.12",
    applied_version: null,
    applied_acknowledged: false,
    ...overrides,
  };
}

test("an installation with nothing in play shows no notice", () => {
  for (const phase of ["idle", "up_to_date", "checking", "check_failed"]) {
    assert.equal(describeUpdateNotice(state({ phase }), Utils), null, phase);
  }
});

test("an available update names its version and opens the status view", () => {
  const notice = describeUpdateNotice(
    state({ phase: "available", available_update: update }),
    Utils
  );
  assert.equal(visible(notice.headline), "Version 0.2.13 is available");
  assert.equal(notice.detail, "See what changed and install it from Settings.");
  assert.deepEqual(notice.action, { label: "View update", view: "status" });
  assert.equal(notice.dismissible, false);
  assert.equal(notice.percent, null);
});

test("a headless installation is told how it updates instead", () => {
  const notice = describeUpdateNotice(
    state({ phase: "available", available_update: update, self_install: false }),
    Utils
  );
  assert.match(visible(notice.detail), /screenerbot-manager update/);
});

test("a download reports its progress", () => {
  const notice = describeUpdateNotice(
    state({
      phase: "downloading",
      available_update: update,
      download_progress: { progress_percent: 42.4, bytes_downloaded: 420, total_bytes: 1000 },
    }),
    Utils
  );
  assert.equal(visible(notice.headline), "Downloading v0.2.13");
  assert.equal(notice.percent, 42.4);
  assert.equal(visible(notice.detail), "420 B of 1000 B · 42%");
});

test("a failed update is shown with the error tone", () => {
  const notice = describeUpdateNotice(state({ phase: "failed", available_update: update }), Utils);
  assert.equal(notice.tone, "error");
  assert.equal(visible(notice.headline), "The update did not finish");
});

test("an update that landed is announced until acknowledged", () => {
  const landed = state({ phase: "applied", applied_version: "0.2.12" });
  const notice = describeUpdateNotice(landed, Utils);
  assert.equal(visible(notice.headline), "Updated to v0.2.12");
  assert.deepEqual(notice.action, { label: "What's new", view: "release-notes" });
  assert.equal(notice.dismissible, true);

  assert.equal(describeUpdateNotice({ ...landed, applied_acknowledged: true }, Utils), null);
  // A version applied earlier than the running one is not news.
  assert.equal(describeUpdateNotice({ ...landed, applied_version: "0.2.11" }, Utils), null);
});
