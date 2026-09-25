import assert from "node:assert/strict";
import { test } from "node:test";

import {
  pauseReasonShort,
  pauseReasonText,
} from "../../src/webserver/templates/scripts/pages/copy/format.js";

test("a watch-budget pause names the limit rather than latency or lost watch", () => {
  const reason = {
    kind: "watch_budget_exceeded",
    page_budget: 8,
    signatures_checked: 800,
  };

  assert.equal(pauseReasonShort(reason), "watch limit");
  assert.match(pauseReasonText(reason), /800-signature watch check limit/);
  assert.doesNotMatch(pauseReasonText(reason), /late|no longer watched/);
});
