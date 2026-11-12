// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Copy page watch recovery tests - provider and processing pauses read as distinct causes.

import { englishI18n } from "./fixtures/i18n_en.mjs";
import assert from "node:assert/strict";
import { test } from "node:test";

import { pauseReasonShort } from "../../src/webserver/templates/scripts/pages/copy/format.js";

// A pause reason as `CopyPauseReason::ui_text` sends it.
// Rendered without the isolation marks Fluent wraps around interpolated values.
const renderPause = (id, args = {}) =>
  englishI18n
    .text({
      id,
      args: Object.fromEntries(
        Object.entries(args).map(([name, value]) => [name, { type: "text", value }])
      ),
    })
    .replace(/[\u2068\u2069]/g, "");

test("processing failures are distinct from provider failures in copy status", () => {
  assert.equal(pauseReasonShort({ kind: "watch_processing_failed" }), "watch processing");
  assert.match(renderPause("copy-pause-watch-processing-failed"), /could not be processed/);
  assert.equal(pauseReasonShort({ kind: "helius_unavailable" }), "watch provider");
});
