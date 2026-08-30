/**
 * The DataTable column-width contract.
 *
 * Column widths may only ever be derived from a table that is actually laid
 * out. A table rendered inside a `display: none` ancestor - a tab panel not yet
 * unhidden, a page mid-route-switch - reports every width as 0; fitting against
 * that collapses every column to its minimum and pins `table.style.width` to
 * `0px`, which the stylesheet's `width: 100%` cannot undo. Because the one-shot
 * `hasAutoFitted` flag used to be set unconditionally, the table then stayed
 * collapsed for the rest of its life. The Wallets page reproduced it on every
 * reload with a non-default tab restored.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath, URL } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const templates = resolve(root, "src/webserver/templates");
const COLUMNS = resolve(templates, "scripts/ui/data_table/column_management.js");
const TABLE = resolve(templates, "scripts/ui/data_table.js");
const WALLETS = resolve(templates, "scripts/pages/wallets.js");

test("no measurement is taken from a table that is not laid out", async () => {
  const source = await readFile(COLUMNS, "utf8");

  assert.match(
    source,
    /proto\._isLaidOut = function \(\) \{\s*return \(this\.elements\.scrollContainer\?\.clientWidth \|\| 0\) > 0;/,
    "_isLaidOut must read the scroll container's real client width"
  );
  assert.match(
    source,
    /proto\._sizeColumns = function \(\) \{\s*if \(!this\._isLaidOut\(\)\) return;/,
    "the column pass must bail before it measures anything"
  );
});

test("fitting refuses a zero-width container and reports whether it ran", async () => {
  const source = await readFile(COLUMNS, "utf8");
  const start = source.indexOf("proto._fitColumnsToContainer");
  assert.notEqual(start, -1, "_fitColumnsToContainer is missing");
  const body = source.slice(start, source.indexOf("\n  proto.", start + 1));

  assert.match(body, /if \(containerWidth <= 0\) return false;/, "zero width must be rejected");
  assert.doesNotMatch(
    body,
    /applyFinal\(\);\s*\n\s*return;/,
    "a completed fit must report success, not exit silently"
  );
  assert.equal(
    (body.match(/\n {6}return true;|\n {4}return true;/g) || []).length,
    3,
    "each of the three fit outcomes must report success"
  );
});

test("the one-shot fit flag is only set by a fit that actually ran", async () => {
  const source = await readFile(COLUMNS, "utf8");

  assert.match(
    source,
    /this\.state\.hasAutoFitted = this\._fitColumnsToContainer\(\) === true;/,
    "hasAutoFitted must come from the fit's own return value"
  );
  assert.doesNotMatch(
    source,
    /this\._fitColumnsToContainer\(\);\s*\n\s*this\.state\.hasAutoFitted = true;/,
    "a fit must never be assumed to have succeeded"
  );
});

test("a deferred fit is recovered when the wrapper gets a real width", async () => {
  const source = await readFile(TABLE, "utf8");
  const start = source.indexOf("this._wrapperResizeObserver = new ResizeObserver");
  assert.notEqual(start, -1, "the wrapper ResizeObserver is missing");
  const body = source.slice(start, source.indexOf("});", start));

  assert.match(
    body,
    /!this\.state\.hasAutoFitted[\s\S]*this\._sizeColumns\(\)/,
    "the observer must complete a pending fit"
  );
});

test("Wallets shows the restored panel before it builds that panel's table", async () => {
  const source = await readFile(WALLETS, "utf8");
  const show = source.indexOf("updatePanelVisibility();");
  const load = source.indexOf("await loadActiveTab();");

  assert.notEqual(show, -1, "updatePanelVisibility() call is missing");
  assert.notEqual(load, -1, "the initial loadActiveTab() await is missing");
  assert.ok(
    show < load,
    "init() must unhide the panel first - a table built inside a hidden panel cannot measure itself"
  );
});
