//! Compute budget for a direct swap.
//!
//! Both instructions are mandatory, for different reasons:
//!
//! * The LIMIT stops the runtime's 200k default from truncating a venue that
//!   legitimately needs more (a CLMM swap crossing tick arrays does). A swap that
//!   runs out of compute fails AFTER the wrap and ATA creations have executed --
//!   the transaction reverts, so nothing is lost, but the slot and the priority
//!   fee are.
//! * The PRICE is what gets the transaction into a contested block at all. A swap
//!   that lands three slots late is quoted against a price that no longer exists.

use crate::chains::solana::solana_sdk::{
    compute_budget::ComputeBudgetInstruction, instruction::Instruction,
};
use crate::config::with_config;

/// Headroom added on top of a venue's own estimate, in percent. Venue estimates
/// are for the swap alone; the same transaction also carries ATA creations, the
/// wrap/unwrap and the fee transfer.
const COMPUTE_UNIT_HEADROOM_PCT: u32 = 30;

/// Floor for the requested limit. Below this even a trivial swap plan does not fit.
const MIN_COMPUTE_UNITS: u32 = 60_000;

/// Ceiling. The runtime rejects a request above 1.4M units outright.
const MAX_COMPUTE_UNITS: u32 = 1_400_000;

/// The two compute-budget instructions, which must lead the transaction.
///
/// `venue_units` is the venue's own estimate for its swap instruction.
pub fn compute_budget_instructions(venue_units: u32) -> Vec<Instruction> {
    let limit = compute_unit_limit(venue_units);
    let price = with_config(|cfg| cfg.swaps.direct.priority_fee_micro_lamports);
    vec![
        ComputeBudgetInstruction::set_compute_unit_limit(limit),
        ComputeBudgetInstruction::set_compute_unit_price(price),
    ]
}

/// The limit to request for a venue needing `venue_units`, with headroom, clamped
/// to what the runtime accepts.
pub fn compute_unit_limit(venue_units: u32) -> u32 {
    let with_headroom = venue_units
        .saturating_mul(100 + COMPUTE_UNIT_HEADROOM_PCT)
        .saturating_div(100);
    with_headroom.clamp(MIN_COMPUTE_UNITS, MAX_COMPUTE_UNITS)
}

/// Safety margin added on top of a MEASURED consumption, in percent. Smaller
/// than [`COMPUTE_UNIT_HEADROOM_PCT`] because this is sized off what the
/// transaction actually used in simulation, not off a venue's static guess.
const MEASURED_SAFETY_MARGIN_PCT: u32 = 15;

/// Flat units added on top of the margin, to absorb the small variance between
/// simulated state and the state the real execution lands against.
const MEASURED_SAFETY_FLAT_UNITS: u32 = 2_000;

/// Tighten the requested compute-unit limit from a simulation's MEASURED
/// `units_consumed`, clamped to the same runtime bounds as the venue estimate.
///
/// Solana charges the prioritization fee on the LIMIT a transaction requests,
/// not on the compute it actually consumes. A venue's static estimate carries
/// 30% headroom for a swap that may cross more state than usual; simulation
/// already measures the true cost, so requesting a limit sized off the measured
/// number instead of the static estimate stops paying a priority fee on compute
/// units the transaction never uses.
pub fn compute_unit_limit_from_measured(units_consumed: u64) -> u32 {
    let with_margin = units_consumed
        .saturating_mul(100 + MEASURED_SAFETY_MARGIN_PCT as u64)
        .saturating_div(100)
        .saturating_add(MEASURED_SAFETY_FLAT_UNITS as u64);
    (with_margin.min(u32::MAX as u64) as u32).clamp(MIN_COMPUTE_UNITS, MAX_COMPUTE_UNITS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_venue_estimate_gets_headroom_for_the_rest_of_the_plan() {
        assert_eq!(compute_unit_limit(200_000), 260_000);
    }

    #[test]
    fn a_tiny_estimate_is_raised_to_the_floor_not_sent_as_is() {
        assert_eq!(compute_unit_limit(1_000), MIN_COMPUTE_UNITS);
    }

    #[test]
    fn an_absurd_estimate_is_clamped_below_the_runtime_ceiling() {
        assert_eq!(compute_unit_limit(u32::MAX), MAX_COMPUTE_UNITS);
        assert_eq!(compute_unit_limit(2_000_000), MAX_COMPUTE_UNITS);
    }

    #[test]
    fn a_measured_limit_adds_margin_plus_a_flat_cushion() {
        // 100_000 * 1.15 + 2_000 = 117_000, then clamped like everything else.
        assert_eq!(compute_unit_limit_from_measured(100_000), 117_000);
    }

    #[test]
    fn a_tiny_measured_consumption_is_still_raised_to_the_floor() {
        assert_eq!(compute_unit_limit_from_measured(100), MIN_COMPUTE_UNITS);
    }

    #[test]
    fn an_absurd_measured_consumption_is_clamped_below_the_runtime_ceiling() {
        assert_eq!(
            compute_unit_limit_from_measured(u64::MAX),
            MAX_COMPUTE_UNITS
        );
        assert_eq!(
            compute_unit_limit_from_measured(2_000_000),
            MAX_COMPUTE_UNITS
        );
    }

    #[test]
    fn both_budget_instructions_are_emitted_in_limit_then_price_order() {
        crate::config::utils::CONFIG
            .get_or_init(|| std::sync::RwLock::new(crate::config::schemas::Config::default()));
        let ixs = compute_budget_instructions(200_000);
        assert_eq!(ixs.len(), 2);
        assert_eq!(ixs[0].data[0], 2, "SetComputeUnitLimit discriminator");
        assert_eq!(ixs[1].data[0], 3, "SetComputeUnitPrice discriminator");
        assert_eq!(
            u32::from_le_bytes(ixs[0].data[1..5].try_into().unwrap()),
            260_000
        );
    }
}
