// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token-2022 mint extensions a pool swap cannot ignore: transfer fees, which
//! change every amount, and transfer hooks, which the engine does not build
//! accounts for.
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

use crate::chains::solana::solana_sdk::{account::Account, pubkey::Pubkey};
use crate::chains::solana::spl_token_2022::{
    extension::{
        transfer_fee::TransferFeeConfig, transfer_hook::TransferHook, BaseStateWithExtensions,
        StateWithExtensions,
    },
    state::Mint,
};
use crate::chains::solana::swaps::direct::error::{DirectSwapError, DirectSwapResult};

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

/// Whether every transfer of this mint invokes a transfer-hook programme.
///
/// The extension alone is not enough: a mint can carry it with no programme
/// set, and then a transfer invokes nothing and needs no extra accounts.
pub fn runs_transfer_hook(mint_account: &Account) -> bool {
    if mint_account.owner != crate::chains::solana::spl_token_2022::id() {
        return false;
    }
    let Ok(state) = StateWithExtensions::<Mint>::unpack(&mint_account.data) else {
        return false;
    };
    state
        .get_extension::<TransferHook>()
        .is_ok_and(|hook| hook.program_id.0.to_bytes() != [0u8; 32])
}

/// Refuse a pool one of whose mints runs a transfer hook.
///
/// A hooked transfer needs the hook programme's extra accounts, which no venue
/// here resolves, so the swap would fail simulation every time. Refusing at
/// load leaves the trade to a route that resolves them.
pub fn refuse_transfer_hook(pool: &Pubkey, mints: &[&Account]) -> DirectSwapResult<()> {
    if mints.iter().any(|mint| runs_transfer_hook(mint)) {
        return Err(DirectSwapError::PoolNotTradable {
            pool: *pool,
            detail: "one of this pool's mints runs a Token-2022 transfer hook, whose accounts \
                     this venue does not build"
                .to_owned(),
        });
    }
    Ok(())
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
    use crate::chains::solana::solana_program::program_pack::Pack;
    use crate::chains::solana::spl_token_2022::extension::{
        BaseStateWithExtensionsMut, ExtensionType, StateWithExtensionsMut,
    };

    /// A Token-2022 mint account carrying the transfer-hook extension, with
    /// `hook_program` as its programme (all zeros for none).
    fn hooked_mint(hook_program: [u8; 32]) -> Account {
        let len = ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::TransferHook])
            .expect("length");
        let mut data = vec![0u8; len];
        let mut state =
            StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut data).expect("unpack");
        let hook = state
            .init_extension::<TransferHook>(true)
            .expect("extension");
        hook.program_id.0 = hook_program.into();
        state.base.decimals = 6;
        state.base.is_initialized = true;
        state.pack_base();
        state.init_account_type().expect("account type");
        Account {
            lamports: 1,
            data,
            owner: crate::chains::solana::spl_token_2022::id(),
            executable: false,
            rent_epoch: 0,
        }
    }

    #[test]
    fn a_mint_runs_a_hook_only_when_the_extension_names_a_programme() {
        assert!(runs_transfer_hook(&hooked_mint([7u8; 32])));
        assert!(
            !runs_transfer_hook(&hooked_mint([0u8; 32])),
            "an extension with no programme invokes nothing"
        );
        let mut legacy = hooked_mint([7u8; 32]);
        legacy.owner = crate::chains::solana::spl_token::id();
        assert!(
            !runs_transfer_hook(&legacy),
            "a legacy mint has no extensions"
        );
        let mut plain = hooked_mint([7u8; 32]);
        plain.data.truncate(Mint::LEN);
        assert!(!runs_transfer_hook(&plain));
    }

    #[test]
    fn a_pool_with_a_hooked_mint_is_refused_at_load() {
        let pool = Pubkey::new_unique();
        let hooked = hooked_mint([7u8; 32]);
        let inert = hooked_mint([0u8; 32]);
        assert!(matches!(
            refuse_transfer_hook(&pool, &[&inert, &hooked]),
            Err(DirectSwapError::PoolNotTradable { .. })
        ));
        assert!(refuse_transfer_hook(&pool, &[&inert, &inert]).is_ok());
    }

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
