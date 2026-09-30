import { englishI18n } from "./fixtures/i18n_en.mjs";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
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

test("a watch-budget pause names the limit rather than latency or lost watch", () => {
  const reason = {
    kind: "watch_budget_exceeded",
    page_budget: 8,
    signatures_checked: 800,
  };

  const text = renderPause("copy-pause-watch-budget-exceeded", { limit: "800" });

  assert.equal(pauseReasonShort(reason), "watch limit");
  assert.match(text, /800-signature watch check limit/);
  assert.doesNotMatch(text, /late|no longer watched/);
});

test("watch recovery keeps the copy task paused until a separate resume", async () => {
  const workspace = await readFile(
    new URL("../../src/webserver/templates/scripts/pages/copy/workspace.js", import.meta.url),
    "utf8"
  );
  const watched = await readFile(
    new URL("../../src/webserver/templates/scripts/pages/wallets/watched.js", import.meta.url),
    "utf8"
  );

  assert.match(workspace, /"helius_unavailable", "watch_processing_failed"/);
  assert.match(englishI18n.t("copy-action-retry-watch"), /Retry wallet watch/);
  assert.match(workspace, /"copy-action-retry-watch"/);
  assert.match(workspace, /\/api\/wallets\/watch\/\$\{target\.id\}\/enabled/);
  const wording = [
    "copy-action-retry-watch",
    "copy-watch-retry-started",
    "copy-watch-resumed",
    "copy-watch-approved",
  ]
    .map((id) => englishI18n.t(id))
    .join("\n");
  assert.doesNotMatch(workspace, /Retry watch and resume|Wallet watch and copy task resumed/);
  assert.doesNotMatch(wording, /Retry watch and resume|Wallet watch and copy task resumed/);
  assert.match(englishI18n.t("copy-watch-retry-started"), /copy task remains paused/);
  assert.match(watched, />Retry watch</);
  assert.match(watched, /processing_failed/);
});

test("Helius catch-up is offered only for a capable watch and requires per-wallet confirmation", async () => {
  const workspace = await readFile(
    new URL("../../src/webserver/templates/scripts/pages/copy/workspace.js", import.meta.url),
    "utf8"
  );
  const watched = await readFile(
    new URL("../../src/webserver/templates/scripts/pages/wallets/watched.js", import.meta.url),
    "utf8"
  );

  assert.match(workspace, /data-ws-action="approve-helius"/);
  assert.match(workspace, /option\.provider === "helius" && option\.available/);
  assert.match(
    englishI18n.t("copy-watch-approve-message"),
    /10 credits per 100 full transactions returned/
  );
  assert.match(workspace, /acknowledge_provider_usage: true/);
  assert.match(workspace, /The wallet watch has not been restored/);
  assert.match(watched, /option\.provider === "helius"/);
  assert.doesNotMatch(watched, /Disable Helius|Use Helius/);
  assert.match(watched, /10 credit minimum per request/);
  assert.match(watched, /acknowledge_provider_usage: approved/);
});

test("processing failures are distinct from provider failures in copy status", () => {
  assert.equal(pauseReasonShort({ kind: "watch_processing_failed" }), "watch processing");
  assert.match(renderPause("copy-pause-watch-processing-failed"), /could not be processed/);
  assert.equal(pauseReasonShort({ kind: "helius_unavailable" }), "watch provider");
});
