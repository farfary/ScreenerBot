import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { shouldRollbackStagedCore } = require("../../electron/src/backend_launch.js");

test("an OS spawn failure rolls back a staged core", () => {
  assert.equal(shouldRollbackStagedCore({
    staged: true,
    ready: false,
    recovering: false,
    recoveryScheduled: false,
  }), true);
});

test("ready, bundled, and already-recovering launches never schedule rollback", () => {
  for (const state of [
    { staged: false, ready: false, recovering: false, recoveryScheduled: false },
    { staged: true, ready: true, recovering: false, recoveryScheduled: false },
    { staged: true, ready: false, recovering: true, recoveryScheduled: false },
    { staged: true, ready: false, recovering: false, recoveryScheduled: true },
  ]) {
    assert.equal(shouldRollbackStagedCore(state), false);
  }
});
