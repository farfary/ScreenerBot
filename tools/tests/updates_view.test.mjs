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
  formatDate(value, { fallback = "—" } = {}) {
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

const release = (version, notes) => ({
  version,
  release_date: "2026-09-05T00:00:00Z",
  release_notes: notes,
});

test("release-note rendering escapes supplied content", () => {
  const html = view.renderReleaseNotes({
    releases: [release("0.2.4", "### Safety\n- Fixed <script>alert(1)</script>.")],
    currentVersion: "0.2.4",
    availableVersion: null,
  });

  assert.match(html, /v0\.2\.4/);
  assert.match(html, /<h4>Safety<\/h4>/);
  assert.match(html, /<li>Fixed &lt;script&gt;alert\(1\)&lt;\/script&gt;\.<\/li>/);
  assert.doesNotMatch(html, /<script>/);
});

test("the history opens this installation's release and tags only what it must", () => {
  const html = view.renderReleaseNotes({
    releases: [
      release("0.2.6", "### New\n- Newest."),
      release("0.2.5", "### New\n- Running."),
      release("0.2.4", "### New\n- Older."),
    ],
    currentVersion: "0.2.5",
    availableVersion: "0.2.6",
  });

  const entries = html.split("<details");
  assert.equal(entries.length, 4);
  assert.match(entries[1], /data-state="available"/);
  assert.match(entries[1], />Available</);
  // The pending update is the decision in front of the reader, so it opens.
  assert.match(entries[1], / open>/);
  assert.match(entries[2], /data-state="installed"/);
  assert.match(entries[2], />Installed</);
  assert.match(entries[3], /data-state="past"/);
  assert.doesNotMatch(entries[3], /updates-release-tag/);
  // Only one entry may start expanded.
  assert.equal(html.match(/ open>/g).length, 1);
  assert.match(html, /1 change</);
});

test("with nothing pending, the running build is the entry that opens", () => {
  const html = view.renderReleaseNotes({
    releases: [release("0.2.6", "### New\n- Newest."), release("0.2.5", "### New\n- Running.")],
    currentVersion: "0.2.5",
    availableVersion: null,
  });

  const entries = html.split("<details");
  assert.doesNotMatch(entries[1], / open>/);
  assert.match(entries[2], / open>/);
});

test("an unreadable history still shows what the updater knows", () => {
  const html = view.renderReleaseNotes({
    releases: [release("0.2.5", "### New\n- Running.")],
    currentVersion: "0.2.5",
    availableVersion: null,
    error: true,
  });

  assert.match(html, /updates-history-notice/);
  assert.match(html, /v0\.2\.5/);

  const empty = view.renderReleaseNotes({
    releases: [],
    currentVersion: "0.2.5",
    availableVersion: null,
    error: true,
  });
  assert.match(empty, /No release notes yet/);
  assert.match(empty, /updatesNotesRetry/);
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
