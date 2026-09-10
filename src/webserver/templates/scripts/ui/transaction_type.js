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

const KINDS = {
  buy: { label: "Buy", variant: "success", icon: "icon-shopping-cart" },
  sell: { label: "Sell", variant: "error", icon: "icon-dollar-sign" },
  swap: { label: "Swap", variant: "info", icon: "icon-repeat" },
  sol_transfer: { label: "SOL transfer", variant: "secondary", icon: "icon-send" },
  token_transfer: { label: "Token transfer", variant: "secondary", icon: "icon-send" },
  transfer: { label: "Transfer", variant: "secondary", icon: "icon-send" },
  dust: { label: "Dust", variant: "secondary", icon: "icon-sparkle" },
  spam: { label: "Spam", variant: "warning", icon: "icon-ban" },
  ata_create: { label: "Account opened", variant: "secondary", icon: "icon-folder-plus" },
  ata_close: { label: "Rent reclaimed", variant: "info", icon: "icon-folder-minus" },
  ata: { label: "Token account", variant: "secondary", icon: "icon-layers" },
  liquidity_add: { label: "Add liquidity", variant: "info", icon: "icon-droplets" },
  liquidity_remove: { label: "Remove liquidity", variant: "info", icon: "icon-droplets" },
  nft: { label: "NFT", variant: "secondary", icon: "icon-image" },
  program: { label: "Program call", variant: "secondary", icon: "icon-cpu" },
  compute: { label: "Compute", variant: "secondary", icon: "icon-cpu" },
  failed: { label: "Failed", variant: "error", icon: "icon-circle-x" },
  unknown: { label: "Unclassified", variant: "secondary", icon: "icon-circle-alert" },
};

/** The filter options the Type dropdown offers, in display order. */
export const TYPE_FILTER_OPTIONS = [
  { value: "all", label: "All Types" },
  { value: "buy", label: "Buy" },
  { value: "sell", label: "Sell" },
  { value: "swap", label: "Swap" },
  { value: "transfer", label: "Transfers" },
  { value: "ata", label: "Rent & accounts" },
  { value: "dust", label: "Dust" },
  { value: "spam", label: "Spam" },
  { value: "liquidity", label: "Liquidity" },
  { value: "nft", label: "NFT" },
  { value: "program", label: "Program calls" },
  { value: "failed", label: "Failed" },
  { value: "unknown", label: "Unclassified" },
];

/** Normalizes any of the API's type shapes onto a kind string. */
export function typeKind(value) {
  if (!value) return "unknown";
  if (typeof value === "string") {
    if (KINDS[value]) return value;
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
  return (KINDS[kind] ?? KINDS.unknown).label;
}

export function typeVariant(value) {
  return (KINDS[typeKind(value)] ?? KINDS.unknown).variant;
}

export function typeIcon(value) {
  return (KINDS[typeKind(value)] ?? KINDS.unknown).icon;
}
