/**
 * Presentation for the venue ids the API sends: a pool's DEX program and a
 * transaction's swap router. Both are stable machine ids ("meteora_dlmm",
 * "jupiter"); the name shown is the catalog label, never the id.
 */

/** Pool program ids, keyed by `ProgramKind::protocol_slug()`. */
export const POOL_PROGRAM_LABELS = Object.freeze({
  raydium_cpmm: "common-venue-raydium-cpmm",
  raydium_legacy_amm: "common-venue-raydium-legacy-amm",
  raydium_clmm: "common-venue-raydium-clmm",
  orca_whirlpool: "common-venue-orca-whirlpool",
  meteora_damm_v2: "common-venue-meteora-damm-v2",
  meteora_dlmm: "common-venue-meteora-dlmm",
  meteora_dbc: "common-venue-meteora-dbc",
  pumpfun_amm: "common-venue-pumpfun-amm",
  pumpfun_legacy: "common-venue-pumpfun",
  moonit_amm: "common-venue-moonit-amm",
  fluxbeam_amm: "common-venue-fluxbeam-amm",
  unknown: "format-unknown",
});

/**
 * Router ids stored on a transaction, keyed by `DetectedDex::router_id()` and
 * `detect_router_from_program_id`; mirrors `router_label` in Rust.
 */
export const ROUTER_LABELS = Object.freeze({
  jupiter: "common-venue-jupiter",
  raptor: "common-venue-raptor",
  gmgn: "common-venue-gmgn",
  raydium: "common-venue-raydium",
  raydiumclmm: "common-venue-raydium-clmm",
  orca: "common-venue-orca",
  orcawhirlpool: "common-venue-orca-whirlpool",
  meteora: "common-venue-meteora",
  pumpfun: "common-venue-pumpfun",
  moonshot: "common-venue-moonshot",
  fluxbeam: "common-venue-fluxbeam",
  lifinity: "common-venue-lifinity",
  aldrin: "common-venue-aldrin",
  serum: "common-venue-serum",
  openbook: "common-venue-openbook",
  phoenix: "common-venue-phoenix",
  unknown: "format-unknown",
});

/**
 * Display name of a pool program or router id. The two id sets do not overlap
 * except for "unknown", so one lookup serves every venue field; an id neither
 * lists shows as sent.
 */
export function venueLabel(id) {
  return Object.hasOwn(POOL_PROGRAM_LABELS, id)
    ? I18n.label(POOL_PROGRAM_LABELS, id)
    : I18n.label(ROUTER_LABELS, id);
}
