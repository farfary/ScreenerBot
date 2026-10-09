// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Presentation for a transaction's type, in one place.
 *
 * The API sends two shapes for the same thing: a list row carries the stable
 * discriminant produced by `TransactionType::kind()` ("ata_close", "dust", …),
 * while a detail response carries the serialized enum, which is either a bare
 * string ("Buy") or a single-key object ({ AtaClose: { … } }). Every consumer used
 * to re-derive its own label from whichever shape it happened to get, which is why
 * the same transaction could read "ATA Close" in the dialog and "Unknown" in the
 * table. `typeKind` normalizes both shapes onto the discriminant; everything else
 * is keyed off that.
 */

/** Rich variant name -> kind, matching `TransactionType::kind()` in Rust. */
const VARIANT_KINDS = {
  Buy: "buy",
  SwapSolToToken: "buy",
  Sell: "sell",
  SwapTokenToSol: "sell",
  SwapTokenToToken: "swap",
  SolTransfer: "sol_transfer",
  TokenTransfer: "token_transfer",
  Transfer: "transfer",
  Dust: "dust",
  SpamAirdrop: "spam",
  AtaCreate: "ata_create",
  AtaClose: "ata_close",
  AtaOperation: "ata",
  LiquidityAdd: "liquidity_add",
  LiquidityRemove: "liquidity_remove",
  NftOperation: "nft",
  ProgramInteraction: "program",
  Other: "program",
  Compute: "compute",
  Failed: "failed",
  Unknown: "unknown",
};

/** Message key of each kind's label; keep in step with `TransactionType::kind()`. */
export const TRANSACTION_TYPE_LABELS = Object.freeze({
  buy: "transactions-type-buy",
  sell: "transactions-type-sell",
  swap: "transactions-type-swap",
  sol_transfer: "transactions-type-sol-transfer",
  token_transfer: "transactions-type-token-transfer",
  transfer: "transactions-type-transfer",
  dust: "transactions-type-dust",
  spam: "transactions-type-spam",
  ata_create: "transactions-type-ata-create",
  ata_close: "transactions-type-ata-close",
  ata: "transactions-type-ata",
  liquidity_add: "transactions-type-liquidity-add",
  liquidity_remove: "transactions-type-liquidity-remove",
  nft: "transactions-type-nft",
  program: "transactions-type-program",
  compute: "transactions-type-compute",
  failed: "transactions-type-failed",
  unknown: "transactions-type-unknown",
});

const KINDS = {
  buy: { variant: "success", icon: "icon-shopping-cart" },
  sell: { variant: "error", icon: "icon-dollar-sign" },
  swap: { variant: "info", icon: "icon-repeat" },
  sol_transfer: { variant: "secondary", icon: "icon-send" },
  token_transfer: { variant: "secondary", icon: "icon-send" },
  transfer: { variant: "secondary", icon: "icon-send" },
  dust: { variant: "secondary", icon: "icon-sparkle" },
  spam: { variant: "warning", icon: "icon-ban" },
  ata_create: { variant: "secondary", icon: "icon-folder-plus" },
  ata_close: { variant: "info", icon: "icon-folder-minus" },
  ata: { variant: "secondary", icon: "icon-layers" },
  liquidity_add: { variant: "info", icon: "icon-droplets" },
  liquidity_remove: { variant: "info", icon: "icon-droplets" },
  nft: { variant: "secondary", icon: "icon-image" },
  program: { variant: "secondary", icon: "icon-cpu" },
  compute: { variant: "secondary", icon: "icon-cpu" },
  failed: { variant: "error", icon: "icon-circle-x" },
  unknown: { variant: "secondary", icon: "icon-circle-alert" },
};

// Filter entries whose wording is not the kind label (a group of kinds or "all").
const TYPE_FILTER_LABELS = Object.freeze({
  all: "transactions-filter-all",
  transfer: "transactions-filter-transfer",
  ata: "transactions-filter-ata",
  liquidity: "transactions-filter-liquidity",
  program: "transactions-filter-program",
});

const filterOption = (value) => ({
  value,
  label: I18n.label(
    Object.hasOwn(TYPE_FILTER_LABELS, value) ? TYPE_FILTER_LABELS : TRANSACTION_TYPE_LABELS,
    value
  ),
});

/** The filter options the Type dropdown offers, in display order. */
export const TYPE_FILTER_OPTIONS = [
  "all",
  "buy",
  "sell",
  "swap",
  "transfer",
  "ata",
  "dust",
  "spam",
  "liquidity",
  "nft",
  "program",
  "failed",
  "unknown",
].map(filterOption);

/** Normalizes any of the API's type shapes onto a kind string. */
export function typeKind(value) {
  if (!value) return "unknown";
  if (typeof value === "string") {
    if (Object.hasOwn(KINDS, value)) return value;
    return VARIANT_KINDS[value] ?? "unknown";
  }
  const variant = Object.keys(value)[0];
  return VARIANT_KINDS[variant] ?? "unknown";
}

export function typeLabel(value) {
  const kind = typeKind(value);
  if (kind === "program" && typeof value === "object" && value?.Other?.description) {
    return value.Other.description;
  }
  return I18n.label(TRANSACTION_TYPE_LABELS, kind);
}

/**
 * The label a table cell shows: the message's `.short` form where one exists ("Rent
 * back" for "Rent reclaimed"), else the full label. The cell carries the full label
 * as its tooltip (`typeLabel`), and the details dialog shows the full label.
 */
export function typeShortLabel(value) {
  const kind = typeKind(value);
  if (kind === "program") return typeLabel(value);
  return I18n.labelAttr(TRANSACTION_TYPE_LABELS, kind, "short") ?? typeLabel(value);
}

export function typeVariant(value) {
  return (KINDS[typeKind(value)] ?? KINDS.unknown).variant;
}

export function typeIcon(value) {
  return (KINDS[typeKind(value)] ?? KINDS.unknown).icon;
}
