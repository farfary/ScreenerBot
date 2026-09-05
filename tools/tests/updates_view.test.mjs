import test from "node:test";
import assert from "node:assert/strict";

import {
  createUpdatesView,
  parseReleaseNotes,
} from "../../src/webserver/templates/scripts/ui/settings/updates_view.js";

function escapeHtml(value) {
  return String(value ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

const view = createUpdatesView({
  escapeHtml,
  formatBytes(value, fallback = "—") {
    return Number.isFinite(value) ? `${value} B` : fallback;
  },
  formatTimestamp(value, { fallback = "—" } = {}) {
    return value ? "Sep 5, 2026" : fallback;
  },
});

test("release notes become titled sections and bullets", () => {
  const parsed = parseReleaseNotes(`
## What's New in v0.2.4

### Trading & Exit Safety
- Fixed partial exits.
- Prevented duplicate entries.

### Dashboard Reliability
- Improved update controls.
`);

  assert.equal(parsed.title, "What's New in v0.2.4");
  assert.deepEqual(parsed.sections, [
    {
      heading: "Trading & Exit Safety",
      bullets: ["Fixed partial exits.", "Prevented duplicate entries."],
      paragraphs: [],
    },
    {
      heading: "Dashboard Reliability",
      bullets: ["Improved update controls."],
      paragraphs: [],
    },
  ]);
});

test("release-note rendering escapes supplied content", () => {
  const html = view.renderReleaseNotes({
    version: "0.2.4",
    release_date: "2026-09-05T00:00:00Z",
    release_notes: "### Safety\n- Fixed <script>alert(1)</script>.",
  });

  assert.match(html, /What’s new in v0\.2\.4/);
  assert.match(html, /<h4>Safety<\/h4>/);
  assert.match(html, /<li>Fixed &lt;script&gt;alert\(1\)&lt;\/script&gt;\.<\/li>/);
  assert.doesNotMatch(html, /<script>/);
});

test("preferences use metadata and include the check interval", () => {
  const html = view.renderPreferences(
    { auto_check: true, check_interval_hours: 6 },
    {
      auto_check: {
        type: "boolean",
        label: "Check for Updates",
        hint: "Look for releases",
        category: "Checking",
      },
      check_interval_hours: {
        type: "integer",
        label: "Check Interval",
        hint: "How often to check",
        category: "Checking",
        min: 1,
        max: 168,
        step: 1,
        unit: "hours",
      },
    }
  );

  assert.match(html, /Check for Updates/);
  assert.match(html, /data-pref="check_interval_hours"/);
  assert.match(html, /min="1"/);
  assert.match(html, />hours</);
});

test("status exposes one phase-appropriate primary action", () => {
  const available = view.renderStatus({
    phase: "available",
    currentVersion: "0.2.3",
    platform: "macOS arm64",
    available_update: {
      version: "0.2.4",
      kind: "core",
      core: { size: 24 },
    },
    download_progress: {},
  });

  assert.match(available.html, /Version 0\.2\.4 is available/);
  assert.match(available.html, />Download update</);
  assert.doesNotMatch(available.html, /Restart to update/);

  const verifying = view.renderStatus({
    phase: "verifying",
    currentVersion: "0.2.3",
    platform: "macOS arm64",
    available_update: { version: "0.2.4", kind: "core", core: { size: 24 } },
    download_progress: { progress_percent: 100 },
  });
  assert.match(verifying.html, /is-indeterminate/);
  assert.doesNotMatch(verifying.html, /aria-valuenow/);
});
