import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import fs from "node:fs/promises";

const require = createRequire(import.meta.url);
const {
  createLineDecoder,
  shouldRollbackStagedCore,
} = require("../../electron/src/backend_launch.js");

test("an OS spawn failure rolls back a staged core", () => {
  assert.equal(shouldRollbackStagedCore({
    staged: true,
    firstRun: true,
    ready: false,
    recovering: false,
    recoveryScheduled: false,
  }), true);
});

test("backend protocol lines survive arbitrary stream chunk boundaries", () => {
  const lines = [];
  const decoder = createLineDecoder((line) => lines.push(line));

  decoder.push(Buffer.from("ordinary log\nSCREENER"));
  decoder.push(Buffer.from("BOT_READY:49152:secret\r"));
  decoder.end(Buffer.from("\nlast line"));

  assert.deepEqual(lines, [
    "ordinary log",
    "SCREENERBOT_READY:49152:secret",
    "last line",
  ]);
});

test("ready, bundled, and already-recovering launches never schedule rollback", () => {
  for (const state of [
    { staged: false, firstRun: true, ready: false, recovering: false, recoveryScheduled: false },
    { staged: true, firstRun: false, ready: false, recovering: false, recoveryScheduled: false },
    { staged: true, firstRun: true, ready: true, recovering: false, recoveryScheduled: false },
    { staged: true, firstRun: true, ready: false, recovering: true, recoveryScheduled: false },
    { staged: true, firstRun: true, ready: false, recovering: false, recoveryScheduled: true },
  ]) {
    assert.equal(shouldRollbackStagedCore(state), false);
  }
});

test("desktop startup keeps staged rollback ahead of restart and error rendering", async () => {
  const source = await fs.readFile(
    new URL("../../electron/src/main.js", import.meta.url),
    "utf8"
  );
  const exitHandler = source.match(/child\.on\('exit',[\s\S]*?\n\s{4}\}\);/);
  assert.ok(exitHandler, "backend exit handler must remain present");
  assert.ok(
    exitHandler[0].indexOf("recoverFailedStagedCore") <
      exitHandler[0].indexOf("BACKEND_RESTART_EXIT_CODE"),
    "an unusable staged core must roll back before restart exit codes are honored"
  );
  assert.match(source, /else if \(stagedRecoveryGeneration === generation\) \{\s*return;/);
  assert.match(
    source,
    /mainWindow\.loadURL\(appUrl\.href\)\.catch\(err =>[\s\S]*?err\.message/
  );
  assert.match(
    source,
    /restartBackendFromDashboard\(backendRestartTarget,[\s\S]*?bundledCore\('rollback after staged-core startup failure'\)/,
    "rollback must bypass staged-core resolution even if quarantine persistence fails"
  );
});
