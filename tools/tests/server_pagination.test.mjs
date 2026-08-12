import assert from "node:assert/strict";
import test from "node:test";

globalThis.window = { addEventListener() {} };

const { applyServerPaginationMixin } = await import(
  "../../src/webserver/templates/scripts/ui/data_table/server_pagination.js"
);

class TestTable {}

applyServerPaginationMixin(TestTable);

function tableWithCursors() {
  const table = new TestTable();
  table._pagination = {
    enabled: true,
    cursorNext: { signature: "older" },
    cursorPrev: { signature: "newer" },
    hasMoreNext: true,
    hasMorePrev: true,
    meta: {},
  };
  return table;
}

test("undefined pagination metadata preserves cursors owned by another direction", () => {
  const table = tableWithCursors();

  table._updatePaginationMeta({
    cursorNext: undefined,
    cursorPrev: undefined,
    hasMoreNext: undefined,
    hasMorePrev: undefined,
  });

  assert.deepEqual(table._pagination.cursorNext, { signature: "older" });
  assert.deepEqual(table._pagination.cursorPrev, { signature: "newer" });
  assert.equal(table._pagination.hasMoreNext, true);
  assert.equal(table._pagination.hasMorePrev, true);
});

test("explicit null pagination cursor still clears that direction", () => {
  const table = tableWithCursors();

  table._updatePaginationMeta({ cursorNext: null });

  assert.equal(table._pagination.cursorNext, null);
  assert.equal(table._pagination.hasMoreNext, false);
  assert.deepEqual(table._pagination.cursorPrev, { signature: "newer" });
});
