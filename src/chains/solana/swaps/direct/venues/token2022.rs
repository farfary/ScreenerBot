//! Token-2022 transfer fees, which a pool quote cannot ignore.
//!
//! A transfer-fee mint takes a cut of every transfer, including the two the swap
//! itself performs. The pool receives less than we sent and we receive less than
//! the pool sent, so a quote that ignores the extension over-states the output —
//! and an over-stated output becomes a `min_out` the pool cannot satisfy, which
//! reverts the whole transaction.
//!
//! The fee schedule has two entries, an older and a newer, and which applies
//! depends on the current epoch. Rather than spend an RPC round trip per swap to
//! learn the epoch, the higher of the two is used. That can only over-state the
//! fee, which under-states the output, which lowers `min_out` — the safe
//! direction: a slightly loose floor still fills, a tight one fails.

use crate::chains::solana::solana_sdk::account::Account;
use crate::chains::solana::spl_token_2022::{
    extension::{transfer_fee::TransferFeeConfig, BaseStateWithExtensions, StateWithExtensions},
    state::Mint,
};

/// The transfer fee a mint charges, if it charges one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TransferFeeSchedule {
    /// Fee rate in basis points.
    pub basis_points: u16,
    /// Absolute cap on the fee, in raw units.
    pub maximum_fee: u64,
}

impl TransferFeeSchedule {
    /// The fee charged on transferring `amount`, rounded UP the way the token
    /// program itself rounds it.
    pub fn fee_on(&self, amount: u64) -> u64 {
        if self.basis_points == 0 {
            return 0;
        }
        let raw = ((amount as u128) * (self.basis_points as u128)).div_ceil(10_000);
        raw.min(self.maximum_fee as u128).min(amount as u128) as u64
    }

    /// What actually arrives when `amount` is sent.
    pub fn net_of_fee(&self, amount: u64) -> u64 {
        amount.saturating_sub(self.fee_on(amount))
    }
}

/// Read a mint's transfer-fee schedule.
///
/// Returns `None` for a legacy SPL mint and for a Token-2022 mint without the
/// extension — both charge nothing, and the caller treats `None` as a zero fee.
pub fn transfer_fee_schedule(mint_account: &Account) -> Option<TransferFeeSchedule> {
    if mint_account.owner != crate::chains::solana::spl_token_2022::id() {
        return None;
    }
    let state = StateWithExtensions::<Mint>::unpack(&mint_account.data).ok()?;
    let config = state.get_extension::<TransferFeeConfig>().ok()?;

    let older = schedule_from(&config.older_transfer_fee);
    let newer = schedule_from(&config.newer_transfer_fee);
    // Higher of the two: over-stating the fee is the safe rounding direction.
    Some(if newer.basis_points >= older.basis_points {
        newer
    } else {
        older
    })
}

fn schedule_from(
    fee: &crate::chains::solana::spl_token_2022::extension::transfer_fee::TransferFee,
) -> TransferFeeSchedule {
    TransferFeeSchedule {
        basis_points: u16::from(fee.transfer_fee_basis_points),
        maximum_fee: u64::from(fee.maximum_fee),
    }
}

/// Fee on `amount` for an optional schedule.
pub fn fee_on(schedule: Option<&TransferFeeSchedule>, amount: u64) -> u64 {
    schedule.map(|s| s.fee_on(amount)).unwrap_or(0)
}

/// What arrives when `amount` is sent under an optional schedule.
pub fn net_of_fee(schedule: Option<&TransferFeeSchedule>, amount: u64) -> u64 {
    schedule.map(|s| s.net_of_fee(amount)).unwrap_or(amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mint_without_a_schedule_charges_nothing() {
        assert_eq!(fee_on(None, 1_000_000), 0);
        assert_eq!(net_of_fee(None, 1_000_000), 1_000_000);
    }

    #[test]
    fn a_percentage_fee_rounds_up_the_way_the_token_program_does() {
        let schedule = TransferFeeSchedule {
            basis_points: 100,
            maximum_fee: u64::MAX,
        };
        assert_eq!(schedule.fee_on(10_000), 100);
        assert_eq!(schedule.fee_on(1), 1, "0.01 rounds up to a whole unit");
        assert_eq!(schedule.net_of_fee(10_000), 9_900);
    }

    #[test]
    fn the_maximum_fee_caps_a_large_transfer() {
        let schedule = TransferFeeSchedule {
            basis_points: 500,
            maximum_fee: 1_000,
        };
        assert_eq!(schedule.fee_on(1_000_000), 1_000, "capped, not 50_000");
        assert_eq!(schedule.net_of_fee(1_000_000), 999_000);
    }

    #[test]
    fn a_zero_rate_schedule_is_free_even_with_a_cap() {
        let schedule = TransferFeeSchedule {
            basis_points: 0,
            maximum_fee: 1_000,
        };
        assert_eq!(schedule.fee_on(1_000_000), 0);
    }

    #[test]
    fn a_hundred_percent_fee_never_takes_more_than_the_transfer() {
        let schedule = TransferFeeSchedule {
            basis_points: 20_000,
            maximum_fee: u64::MAX,
        };
        assert_eq!(schedule.fee_on(1_000), 1_000);
        assert_eq!(schedule.net_of_fee(1_000), 0);
    }
}
