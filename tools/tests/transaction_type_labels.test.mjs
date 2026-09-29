import "./fixtures/i18n_en.mjs";
import assert from "node:assert/strict";
import { test } from "node:test";

import {
  TRANSACTION_TYPE_LABELS,
  TYPE_FILTER_OPTIONS,
  typeKind,
  typeLabel,
} from "../../src/webserver/templates/scripts/ui/transaction_type.js";

const KIND_LABELS = {
  buy: "Buy",
  sell: "Sell",
  swap: "Swap",
  sol_transfer: "SOL transfer",
  token_transfer: "Token transfer",
  transfer: "Transfer",
  dust: "Dust",
  spam: "Spam",
  ata_create: "Account opened",
  ata_close: "Rent reclaimed",
  ata: "Token account",
  liquidity_add: "Add liquidity",
  liquidity_remove: "Remove liquidity",
  nft: "NFT",
  program: "Program call",
  compute: "Compute",
  failed: "Failed",
  unknown: "Unclassified",
};

test("every transaction kind renders its catalog label", () => {
  assert.deepEqual(Object.keys(TRANSACTION_TYPE_LABELS).sort(), Object.keys(KIND_LABELS).sort());
  for (const [kind, label] of Object.entries(KIND_LABELS)) {
    assert.equal(typeLabel(kind), label, kind);
  }
});

test("rich variants and list kinds resolve to the same label", () => {
  assert.equal(typeLabel({ AtaClose: { recovered_sol: 0.002 } }), "Rent reclaimed");
  assert.equal(typeLabel("SwapSolToToken"), "Buy");
  assert.equal(typeKind({ SpamAirdrop: {} }), "spam");
  assert.equal(typeLabel({ Other: { description: "Custom call" } }), "Custom call");
  assert.equal(typeLabel(null), "Unclassified");
});

test("the type filter keeps its wording and order", () => {
  assert.deepEqual(
    TYPE_FILTER_OPTIONS.map((option) => [option.value, option.label]),
    [
      ["all", "All Types"],
      ["buy", "Buy"],
      ["sell", "Sell"],
      ["swap", "Swap"],
      ["transfer", "Transfers"],
      ["ata", "Rent & accounts"],
      ["dust", "Dust"],
      ["spam", "Spam"],
      ["liquidity", "Liquidity"],
      ["nft", "NFT"],
      ["program", "Program calls"],
      ["failed", "Failed"],
      ["unknown", "Unclassified"],
    ]
  );
});
