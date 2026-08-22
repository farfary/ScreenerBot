//! Solana DEX program identification and protocol recognition.
//!
//! `ProgramKind` classifies a discovered pool account by its owning Solana
//! program. This is the sole owner of DEX program IDs for pool discovery —
//! the chain-neutral `PoolDescriptor` (in `crate::pools::types`) carries a
//! `ProtocolId` — a stable machine identity produced from `ProgramKind` at
//! this boundary (`protocol_id()` / `from_protocol_id()`), never the enum
//! itself.

use crate::chains::solana::constants::{
    FLUXBEAM_AMM_PROGRAM_ID, METEORA_DAMM_PROGRAM_ID, METEORA_DBC_PROGRAM_ID,
    METEORA_DLMM_PROGRAM_ID, MOONIT_AMM_PROGRAM_ID, ORCA_WHIRLPOOL_PROGRAM_ID,
    PUMP_FUN_AMM_PROGRAM_ID, PUMP_FUN_LEGACY_PROGRAM_ID, RAYDIUM_CLMM_PROGRAM_ID,
    RAYDIUM_CPMM_PROGRAM_ID, RAYDIUM_LEGACY_AMM_PROGRAM_ID,
};
use crate::pools::types::ProtocolId;

/// Program types for different DEX implementations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgramKind {
    RaydiumCpmm,
    RaydiumLegacyAmm,
    RaydiumClmm,
    OrcaWhirlpool,
    MeteoraDamm,
    MeteoraDlmm,
    MeteoraDbc,
    PumpFunAmm,
    PumpFunLegacy,
    Moonit,
    FluxbeamAmm,
    Unknown,
}

impl ProgramKind {
    /// Get the program ID for this pool type
    pub fn program_id(&self) -> &'static str {
        match self {
            ProgramKind::RaydiumCpmm => RAYDIUM_CPMM_PROGRAM_ID,
            ProgramKind::RaydiumLegacyAmm => RAYDIUM_LEGACY_AMM_PROGRAM_ID,
            ProgramKind::RaydiumClmm => RAYDIUM_CLMM_PROGRAM_ID,
            ProgramKind::OrcaWhirlpool => ORCA_WHIRLPOOL_PROGRAM_ID,
            ProgramKind::MeteoraDamm => METEORA_DAMM_PROGRAM_ID,
            ProgramKind::MeteoraDlmm => METEORA_DLMM_PROGRAM_ID,
            ProgramKind::MeteoraDbc => METEORA_DBC_PROGRAM_ID,
            ProgramKind::PumpFunAmm => PUMP_FUN_AMM_PROGRAM_ID,
            ProgramKind::PumpFunLegacy => PUMP_FUN_LEGACY_PROGRAM_ID,
            ProgramKind::Moonit => MOONIT_AMM_PROGRAM_ID,
            ProgramKind::FluxbeamAmm => FLUXBEAM_AMM_PROGRAM_ID,
            ProgramKind::Unknown => "",
        }
    }

    /// Get display name for this program kind
    pub fn display_name(&self) -> &'static str {
        match self {
            ProgramKind::RaydiumCpmm => "RAYDIUM CPMM",
            ProgramKind::RaydiumLegacyAmm => "RAYDIUM LEGACY AMM",
            ProgramKind::RaydiumClmm => "RAYDIUM CLMM",
            ProgramKind::OrcaWhirlpool => "ORCA WHIRLPOOL",
            ProgramKind::MeteoraDamm => "METEORA DAMM v2",
            ProgramKind::MeteoraDlmm => "METEORA DLMM",
            ProgramKind::MeteoraDbc => "METEORA DBC",
            ProgramKind::PumpFunAmm => "PUMP.FUN AMM",
            ProgramKind::PumpFunLegacy => "PUMP.FUN",
            ProgramKind::Moonit => "MOONIT AMM",
            ProgramKind::FluxbeamAmm => "FLUXBEAM AMM",
            ProgramKind::Unknown => "UNKNOWN",
        }
    }

    /// Create ProgramKind from program ID string
    pub fn from_program_id(program_id: &str) -> Self {
        match program_id {
            RAYDIUM_CPMM_PROGRAM_ID => ProgramKind::RaydiumCpmm,
            RAYDIUM_LEGACY_AMM_PROGRAM_ID => ProgramKind::RaydiumLegacyAmm,
            RAYDIUM_CLMM_PROGRAM_ID => ProgramKind::RaydiumClmm,
            ORCA_WHIRLPOOL_PROGRAM_ID => ProgramKind::OrcaWhirlpool,
            METEORA_DAMM_PROGRAM_ID => ProgramKind::MeteoraDamm,
            METEORA_DLMM_PROGRAM_ID => ProgramKind::MeteoraDlmm,
            METEORA_DBC_PROGRAM_ID => ProgramKind::MeteoraDbc,
            PUMP_FUN_AMM_PROGRAM_ID => ProgramKind::PumpFunAmm,
            PUMP_FUN_LEGACY_PROGRAM_ID => ProgramKind::PumpFunLegacy,
            MOONIT_AMM_PROGRAM_ID => ProgramKind::Moonit,
            FLUXBEAM_AMM_PROGRAM_ID => ProgramKind::FluxbeamAmm,
            _ => ProgramKind::Unknown,
        }
    }

    /// Classify a program id (Pubkey) quickly without allocations
    /// This is a lightweight helper intended for debug / analysis tools to avoid
    /// duplicating the mapping logic scattered across modules.
    pub fn classify(program_pubkey: &crate::chains::solana::solana_sdk::pubkey::Pubkey) -> Self {
        Self::from_program_id(&program_pubkey.to_string())
    }

    /// Stable machine identity slug for this program kind — the canonical
    /// value carried by the chain-neutral `ProtocolId` used for routing,
    /// serialization and persistence. Never derived from `display_name()`:
    /// the slug must stay constant when the human-facing label changes, or
    /// an already-persisted identity silently stops routing (falls to
    /// `Unknown`) the moment someone edits UI copy. Presentation stays
    /// strictly on `display_name()`.
    pub const fn protocol_slug(&self) -> &'static str {
        match self {
            ProgramKind::RaydiumCpmm => "raydium_cpmm",
            ProgramKind::RaydiumLegacyAmm => "raydium_legacy_amm",
            ProgramKind::RaydiumClmm => "raydium_clmm",
            ProgramKind::OrcaWhirlpool => "orca_whirlpool",
            ProgramKind::MeteoraDamm => "meteora_damm_v2",
            ProgramKind::MeteoraDlmm => "meteora_dlmm",
            ProgramKind::MeteoraDbc => "meteora_dbc",
            ProgramKind::PumpFunAmm => "pumpfun_amm",
            ProgramKind::PumpFunLegacy => "pumpfun_legacy",
            ProgramKind::Moonit => "moonit_amm",
            ProgramKind::FluxbeamAmm => "fluxbeam_amm",
            ProgramKind::Unknown => "unknown",
        }
    }

    /// Converts to the chain-neutral protocol identity carried by the shared
    /// `PoolDescriptor` (persistence/display/routing use `ProtocolId`, never
    /// this Solana-only enum, outside `crate::chains::solana`). Always the
    /// stable slug — never the display label.
    pub fn protocol_id(&self) -> ProtocolId {
        ProtocolId::new(self.protocol_slug())
    }

    /// Reverses `protocol_id()` at the Solana boundary — the only place that
    /// needs to go from the shared identity back to a routable enum (e.g. to
    /// dispatch to the matching decoder). Routes canonical slugs first; the
    /// exact historical `display_name()` strings are matched second as a
    /// deliberate, time-limited compatibility path so pools already
    /// persisted under the old display-name identity keep routing while
    /// existing state is read/migrated. Unknown and near-match strings are
    /// never guessed — they resolve to `Unknown`.
    pub fn from_protocol_id(id: &ProtocolId) -> Self {
        match id.as_str() {
            // Canonical stable slugs (current identity).
            "raydium_cpmm" => ProgramKind::RaydiumCpmm,
            "raydium_legacy_amm" => ProgramKind::RaydiumLegacyAmm,
            "raydium_clmm" => ProgramKind::RaydiumClmm,
            "orca_whirlpool" => ProgramKind::OrcaWhirlpool,
            "meteora_damm_v2" => ProgramKind::MeteoraDamm,
            "meteora_dlmm" => ProgramKind::MeteoraDlmm,
            "meteora_dbc" => ProgramKind::MeteoraDbc,
            "pumpfun_amm" => ProgramKind::PumpFunAmm,
            "pumpfun_legacy" => ProgramKind::PumpFunLegacy,
            "moonit_amm" => ProgramKind::Moonit,
            "fluxbeam_amm" => ProgramKind::FluxbeamAmm,

            // Historical display-name identities (pre-slug persisted state).
            "RAYDIUM CPMM" => ProgramKind::RaydiumCpmm,
            "RAYDIUM LEGACY AMM" => ProgramKind::RaydiumLegacyAmm,
            "RAYDIUM CLMM" => ProgramKind::RaydiumClmm,
            "ORCA WHIRLPOOL" => ProgramKind::OrcaWhirlpool,
            "METEORA DAMM v2" => ProgramKind::MeteoraDamm,
            "METEORA DLMM" => ProgramKind::MeteoraDlmm,
            "METEORA DBC" => ProgramKind::MeteoraDbc,
            "PUMP.FUN AMM" => ProgramKind::PumpFunAmm,
            "PUMP.FUN" => ProgramKind::PumpFunLegacy,
            "MOONIT AMM" => ProgramKind::Moonit,
            "FLUXBEAM AMM" => ProgramKind::FluxbeamAmm,

            _ => ProgramKind::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_KNOWN: [ProgramKind; 11] = [
        ProgramKind::RaydiumCpmm,
        ProgramKind::RaydiumLegacyAmm,
        ProgramKind::RaydiumClmm,
        ProgramKind::OrcaWhirlpool,
        ProgramKind::MeteoraDamm,
        ProgramKind::MeteoraDlmm,
        ProgramKind::MeteoraDbc,
        ProgramKind::PumpFunAmm,
        ProgramKind::PumpFunLegacy,
        ProgramKind::Moonit,
        ProgramKind::FluxbeamAmm,
    ];

    #[test]
    fn protocol_id_round_trips_through_every_known_program_kind() {
        for kind in ALL_KNOWN {
            assert_eq!(ProgramKind::from_protocol_id(&kind.protocol_id()), kind);
        }
    }

    #[test]
    fn every_known_program_kind_has_a_unique_canonical_slug() {
        let mut slugs: Vec<&str> = ALL_KNOWN.iter().map(|k| k.protocol_slug()).collect();
        let before = slugs.len();
        slugs.sort_unstable();
        slugs.dedup();
        assert_eq!(slugs.len(), before, "canonical slugs must be unique");
        for slug in &slugs {
            // Slugs are the persisted/routed identity: lowercase snake_case only,
            // never the uppercase display label.
            assert_eq!(*slug, slug.to_lowercase());
            assert!(!slug.contains(' '));
        }
    }

    #[test]
    fn display_labels_are_never_emitted_as_protocol_id() {
        for kind in ALL_KNOWN {
            assert_ne!(
                kind.protocol_id().as_str(),
                kind.display_name(),
                "{:?}: protocol_id() must not equal the presentation label",
                kind
            );
        }
    }

    #[test]
    fn exact_historical_display_name_ids_still_resolve_during_compatibility() {
        for kind in ALL_KNOWN {
            let legacy_id = ProtocolId::new(kind.display_name());
            assert_eq!(
                ProgramKind::from_protocol_id(&legacy_id),
                kind,
                "historical display-name identity for {:?} must still route",
                kind
            );
        }
    }

    #[test]
    fn unknown_protocol_id_maps_to_unknown_program_kind() {
        let id = ProtocolId::new("SOME FUTURE DEX");
        assert_eq!(ProgramKind::from_protocol_id(&id), ProgramKind::Unknown);
    }

    #[test]
    fn near_match_protocol_ids_are_never_fuzzy_matched() {
        // Case/whitespace/substring variants of real identities must not
        // silently resolve — from_protocol_id is exact-match only.
        let near_misses = [
            "raydium_cpmm ",
            " raydium_cpmm",
            "Raydium_CPMM",
            "raydium-cpmm",
            "RAYDIUM CPMM ",
            "raydium cpmm",
            "meteora_damm",
            "pumpfun",
        ];
        for candidate in near_misses {
            let id = ProtocolId::new(candidate);
            assert_eq!(
                ProgramKind::from_protocol_id(&id),
                ProgramKind::Unknown,
                "near-match {candidate:?} must not resolve to a known kind"
            );
        }
    }
}
