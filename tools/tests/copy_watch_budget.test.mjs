import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
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

test("Helius recovery retries the preserved watch cursor before resuming copy", async () => {
  const workspace = await readFile(
    new URL("../../src/webserver/templates/scripts/pages/copy/workspace.js", import.meta.url),
    "utf8"
  );
  const watched = await readFile(
    new URL("../../src/webserver/templates/scripts/pages/wallets/watched.js", import.meta.url),
    "utf8"
  );

  assert.match(workspace, /task\.pause_reason\?\.kind === "helius_unavailable"/);
  assert.match(workspace, /Retry watch and resume/);
  assert.match(workspace, /\/api\/wallets\/watch\/\$\{target\.id\}\/enabled/);
  assert.match(workspace, /await api\.update\(task\.id, \{ enabled: true \}\)/);
  assert.match(
    workspace,
    /saved cursor is preserved; stale trades still need to meet the copy arrival limit/
  );
  assert.match(watched, />Retry watch</);
  assert.match(watched, /Restore Helius high-activity support before retrying/);
  assert.match(watched, /Helius approval is off\. Retry to use standard wallet watch/);
});

test("Helius approval requires confirmation and restores copy only after the watch", async () => {
  const workspace = await readFile(
    new URL("../../src/webserver/templates/scripts/pages/copy/workspace.js", import.meta.url),
    "utf8"
  );
  const watched = await readFile(
    new URL("../../src/webserver/templates/scripts/pages/wallets/watched.js", import.meta.url),
    "utf8"
  );

  assert.match(workspace, /data-ws-action="approve-helius"/);
  assert.match(workspace, /10 credits per 100 full transactions returned/);
  assert.match(workspace, /acknowledge_provider_usage: true/);
  assert.match(workspace, /The wallet watch has not been restored/);
  assert.match(watched, /data-watch-action="high-activity"/);
  assert.match(watched, /10 credit minimum per request/);
  assert.match(watched, /acknowledge_provider_usage: approved/);
});
