import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";

const updatesTabPath = new URL(
  "../../src/webserver/templates/scripts/ui/settings/updates_tab.js",
  import.meta.url
);
const settingsDialogPath = new URL(
  "../../src/webserver/templates/scripts/ui/settings_dialog.js",
  import.meta.url
);

test("the native update command reaches the dashboard's real update check", async () => {
  const [updatesTab, settingsDialog] = await Promise.all([
    fs.readFile(updatesTabPath, "utf8"),
    fs.readFile(settingsDialogPath, "utf8"),
  ]);

  assert.match(updatesTab, /export function requestUpdateCheck\(\)/);
  assert.match(updatesTab, /if \(recheck\) session\.pendingRecheck = true/);
  assert.match(updatesTab, /void refresh\(\{ recheck: true \}\)/);
  assert.match(settingsDialog, /onCheckForUpdates\([\s\S]*requestUpdateCheck\(\)/);
  assert.doesNotMatch(settingsDialog, /_performBackgroundUpdateCheck/);
  assert.doesNotMatch(settingsDialog, /setTimeout\(\(\) => void requestUpdateCheck/);
});

test("retry resumes the retained update payload instead of only rechecking metadata", async () => {
  const source = await fs.readFile(updatesTabPath, "utf8");
  const retryHandler = source.match(/on\("updatesRetry",[\s\S]*?\n  \}\);/);

  assert.ok(retryHandler, "the retry action must remain wired");
  assert.match(retryHandler[0], /\/api\/updates\/download/);
  assert.match(retryHandler[0], /version: update\.version/);
});

test("background update discovery is observed and surfaced once per state", async () => {
  const source = await fs.readFile(settingsDialogPath, "utf8");

  assert.match(source, /setInterval\(checkAndShowUpdateDialog, 60_000\)/);
  assert.match(source, /!state\.last_check_attempt && !state\.available_update/);
  assert.match(source, /attentionKey !== lastSurfacedUpdateKey/);
  assert.match(source, /lastSurfacedUpdateKey = attentionKey/);
});
