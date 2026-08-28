//! The single place a direct-swap venue is registered.
//!
//! Dispatch is by the pool account's OWNER, because that is the one fact about a
//! pool that cannot be faked or mis-cached: whatever program owns the account is
//! the program that will execute the swap. A pool kind recorded in our own
//! database is a hint; the owner is the truth.
//!
//! Adding a DEX is a two-line change here plus its `venues/` module. Nothing
//! else in the engine knows a venue by name — the same rule that keeps
//! `crate::swaps::registry` free of concrete routers.

use super::error::{DirectSwapError, DirectSwapResult};
use super::venue::PoolVenue;
use super::venues;
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::sync::{Arc, OnceLock};

static VENUES: OnceLock<Vec<Arc<dyn PoolVenue>>> = OnceLock::new();

/// Every venue the engine can swap through.
pub fn venues() -> &'static [Arc<dyn PoolVenue>] {
    VENUES.get_or_init(|| {
        vec![
            Arc::new(venues::raydium_cpmm::RaydiumCpmmVenue) as Arc<dyn PoolVenue>,
            Arc::new(venues::raydium_amm_v4::RaydiumAmmV4Venue) as Arc<dyn PoolVenue>,
            Arc::new(venues::raydium_clmm::RaydiumClmmVenue) as Arc<dyn PoolVenue>,
            Arc::new(venues::meteora_damm::MeteoraDammVenue) as Arc<dyn PoolVenue>,
            Arc::new(venues::meteora_dlmm::MeteoraDlmmVenue) as Arc<dyn PoolVenue>,
            Arc::new(venues::pumpfun_amm::PumpFunAmmVenue) as Arc<dyn PoolVenue>,
            Arc::new(venues::pumpfun_legacy::PumpFunLegacyVenue) as Arc<dyn PoolVenue>,
            Arc::new(venues::orca_whirlpool::OrcaWhirlpoolVenue) as Arc<dyn PoolVenue>,
        ]
    })
}

/// The venue that owns `program`, if the engine supports it.
pub fn venue_for_program_id(program: &Pubkey) -> Option<Arc<dyn PoolVenue>> {
    venues()
        .iter()
        .find(|venue| venue.program_id() == *program)
        .cloned()
}

/// The venue for a pool kind, if the engine supports it.
pub fn venue_for_kind(kind: ProgramKind) -> Option<Arc<dyn PoolVenue>> {
    venues()
        .iter()
        .find(|venue| venue.program() == kind)
        .cloned()
}

/// Whether the engine can swap in a pool of this kind. Used by callers that
/// choose between the direct engine and an aggregator BEFORE loading anything.
pub fn supports(kind: ProgramKind) -> bool {
    venue_for_kind(kind).is_some()
}

/// Every pool kind the engine can swap in.
pub fn supported_kinds() -> Vec<ProgramKind> {
    venues().iter().map(|venue| venue.program()).collect()
}

/// The venue for a pool account owner, as a typed failure rather than an option.
pub fn require_venue(program: &Pubkey) -> DirectSwapResult<Arc<dyn PoolVenue>> {
    venue_for_program_id(program).ok_or(DirectSwapError::UnsupportedVenue { program: *program })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registered_venue_is_reachable_by_both_its_kind_and_its_program_id() {
        for venue in venues() {
            let by_kind = venue_for_kind(venue.program()).expect("kind lookup");
            let by_id = venue_for_program_id(&venue.program_id()).expect("program id lookup");
            assert_eq!(by_kind.program_id(), venue.program_id());
            assert_eq!(by_id.program(), venue.program());
        }
    }

    #[test]
    fn no_two_venues_claim_the_same_program_or_kind() {
        let mut ids: Vec<String> = venues()
            .iter()
            .map(|v| v.program_id().to_string())
            .collect();
        let count = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(
            ids.len(),
            count,
            "duplicate program id in the venue registry"
        );

        let mut kinds: Vec<String> = venues()
            .iter()
            .map(|v| format!("{:?}", v.program()))
            .collect();
        kinds.sort();
        kinds.dedup();
        assert_eq!(
            kinds.len(),
            count,
            "duplicate pool kind in the venue registry"
        );
    }

    #[test]
    fn an_unknown_program_is_an_unsupported_venue_not_a_silent_none() {
        let stranger = Pubkey::new_unique();
        assert!(matches!(
            require_venue(&stranger),
            Err(DirectSwapError::UnsupportedVenue { .. })
        ));
    }

    #[test]
    fn all_three_raydium_pool_kinds_are_supported() {
        assert!(supports(ProgramKind::RaydiumCpmm));
        assert!(supports(ProgramKind::RaydiumLegacyAmm));
        assert!(supports(ProgramKind::RaydiumClmm));
    }
}
