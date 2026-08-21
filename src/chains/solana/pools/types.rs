//! Solana DEX program identification and protocol recognition.
//!
//! `ProgramKind` classifies a discovered pool account by its owning Solana
//! program. This is the sole owner of DEX program IDs for pool discovery —
//! the chain-neutral `PoolDescriptor` (in `crate::pools::types`) carries a
//! `ProgramKind` field re-exported from here.

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

    /// Converts to the chain-neutral protocol identity carried by the shared
    /// `PoolDescriptor` (persistence/display/routing use `ProtocolId`, never
    /// this Solana-only enum, outside `crate::chains::solana`).
    pub fn protocol_id(&self) -> ProtocolId {
        ProtocolId::new(self.display_name())
    }

    /// Reverses `protocol_id()` at the Solana boundary — the only place that
    /// needs to go from the shared identity back to a routable enum (e.g. to
    /// dispatch to the matching decoder).
    pub fn from_protocol_id(id: &ProtocolId) -> Self {
        match id.as_str() {
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

    #[test]
    fn protocol_id_round_trips_through_every_known_program_kind() {
        let all = [
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
        for kind in all {
            assert_eq!(ProgramKind::from_protocol_id(&kind.protocol_id()), kind);
        }
    }

    #[test]
    fn unknown_protocol_id_maps_to_unknown_program_kind() {
        let id = ProtocolId::new("SOME FUTURE DEX");
        assert_eq!(ProgramKind::from_protocol_id(&id), ProgramKind::Unknown);
    }
}
