// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: each Wallet Consolidation action posts the `ConsolidateRequest` the route
 * reads (`routes/tools/types.rs`): the ids of the selected sub-wallets and an explicit
 * value for every operation, so the route never falls back to every sub-wallet, and
 * the token action names the mints the selected wallets hold.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

globalThis.window = globalThis.window || { addEventListener() {} };
globalThis.document = { addEventListener() {}, readyState: "complete" };

const SOURCE_URL = new URL(
  "../../src/webserver/templates/scripts/pages/tools/wallet_tools.js",
  import.meta.url
);
const TYPES_URL = new URL("../../src/webserver/routes/tools/types.rs", import.meta.url);

const { buildConsolidateRequest } = await import(SOURCE_URL);

const WALLETS = [
  { id: 4, address: "addr-4", token_mints: ["MintA", "MintB"] },
  { id: 7, address: "addr-7", token_mints: ["MintB"] },
  { id: 9, address: "addr-9", token_mints: ["MintC"] },
];

/** Field names of a Rust struct in `types.rs`. */
function structFields(source, name) {
  const body = source.match(new RegExp(`pub struct ${name} \\{([\\s\\S]*?)\\n\\}`))[1];
  return [...body.matchAll(/pub (\w+):/g)].map((m) => m[1]);
}

test("each action names only the selected sub-wallets", () => {
  const selected = new Set(["addr-4", "addr-7"]);
  for (const action of ["sol", "tokens", "cleanup"]) {
    const body = buildConsolidateRequest(WALLETS, selected, action);
    assert.deepEqual(body.wallet_ids, [4, 7], action);
  }
});

test("each action sets exactly its own operation", () => {
  const selected = new Set(["addr-4", "addr-7"]);
  assert.deepEqual(buildConsolidateRequest(WALLETS, selected, "sol"), {
    wallet_ids: [4, 7],
    transfer_sol: true,
    transfer_tokens: [],
    close_atas: false,
  });
  assert.deepEqual(buildConsolidateRequest(WALLETS, selected, "tokens"), {
    wallet_ids: [4, 7],
    transfer_sol: false,
    transfer_tokens: ["MintA", "MintB"],
    close_atas: false,
  });
  assert.deepEqual(buildConsolidateRequest(WALLETS, selected, "cleanup"), {
    wallet_ids: [4, 7],
    transfer_sol: false,
    transfer_tokens: [],
    close_atas: true,
  });
});

test("the request carries every required field of ConsolidateRequest", () => {
  const types = fs.readFileSync(TYPES_URL, "utf8");
  const body = buildConsolidateRequest(WALLETS, new Set(["addr-9"]), "sol");
  // The two remaining fields have serde defaults the dashboard relies on.
  const optional = new Set(["include_token_2022", "leave_rent_exempt"]);
  for (const field of structFields(types, "ConsolidateRequest")) {
    if (optional.has(field)) continue;
    assert.ok(field in body, `missing ${field}`);
  }
});

test("the page reads only fields the consolidation response has", () => {
  const types = fs.readFileSync(TYPES_URL, "utf8");
  const response = new Set(structFields(types, "ConsolidateResponse"));
  const file = fs.readFileSync(SOURCE_URL, "utf8");
  const source = file.slice(
    file.indexOf("// Wallet Consolidation Tool"),
    file.indexOf("// Exports")
  );
  assert.ok(source.includes("runConsolidation"), "the consolidation section is found");
  const reads = new Set([...source.matchAll(/\bresult\.(\w+)/g)].map((m) => m[1]));
  for (const field of reads) {
    assert.ok(response.has(field), `result.${field} is not in ConsolidateResponse`);
  }
});
