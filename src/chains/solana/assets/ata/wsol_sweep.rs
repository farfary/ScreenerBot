//! Reclaims the wallet's wrapped-SOL account after a swap leaves it empty.
//!
//! Routers wrap SOL into the wallet's associated WSOL account to trade. Some
//! leave that account open and empty afterwards (Raptor did, 2026-09-16, locking
//! 1,488,440 lamports of rent), and the rent then waits for Wallet Cleanup. This
//! sweep is router-agnostic: every confirmed swap with a SOL leg schedules it,
//! and a router that already cleans up after itself simply leaves nothing to
//! close.
//!
//! It closes the account only when that is provably safe:
//! - it is the wallet's ASSOCIATED WSOL account, not some other WSOL account;
//! - it holds zero WSOL, so closing never unwraps a balance someone kept;
//! - no trade, swap or wallet tool is in flight, checked before and after the
//!   account read, because a swap being built may be about to use it;
//! - one sweep runs at a time, and swaps arriving while it waits share it.
//!
//! Every router in use creates the WSOL account idempotently inside its own
//! transaction, so even a swap that starts right after the close is unaffected.

use super::balance::get_all_token_accounts;
use super::helpers::close_ata;
use crate::chains::solana::constants::SOL_MINT;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::spl_associated_token_account::get_associated_token_address_with_program_id;
use crate::logger::{self, LogTag};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Wait after the swap confirms, so its balance changes are visible to the read.
const SETTLE_DELAY: Duration = Duration::from_secs(10);

/// Wait between checks while trading is busy.
const BUSY_RETRY_DELAY: Duration = Duration::from_secs(15);

/// Give up after this many busy checks; the next swap schedules a fresh sweep.
const MAX_BUSY_RETRIES: u32 = 20;

static SWEEP_SCHEDULED: AtomicBool = AtomicBool::new(false);

/// Schedule a sweep of the wallet's WSOL account. Cheap to call after every swap.
pub fn schedule_wsol_sweep() {
    if SWEEP_SCHEDULED.swap(true, Ordering::SeqCst) {
        return;
    }
    tokio::spawn(async {
        sweep_when_idle().await;
        SWEEP_SCHEDULED.store(false, Ordering::SeqCst);
    });
}

async fn sweep_when_idle() {
    tokio::time::sleep(SETTLE_DELAY).await;
    if !wait_for_idle().await {
        logger::debug(
            LogTag::Wallet,
            "WSOL sweep skipped: trading stayed busy; the next swap will schedule another",
        );
        return;
    }

    let wallet = match crate::utils::get_wallet_address() {
        Ok(wallet) => wallet,
        Err(e) => {
            logger::debug(
                LogTag::Wallet,
                &format!("WSOL sweep skipped: no wallet address ({e})"),
            );
            return;
        }
    };
    let accounts = match get_all_token_accounts(&wallet).await {
        Ok(accounts) => accounts,
        Err(e) => {
            logger::debug(
                LogTag::Wallet,
                &format!("WSOL sweep skipped: token accounts unreadable ({e})"),
            );
            return;
        }
    };

    let (Ok(owner), Ok(mint)) = (Pubkey::from_str(&wallet), Pubkey::from_str(SOL_MINT)) else {
        return;
    };
    let Some(wsol) = accounts.into_iter().find(|account| {
        let program = if account.is_token_2022 {
            crate::chains::solana::spl_token_2022::id()
        } else {
            crate::chains::solana::spl_token::id()
        };
        account.mint == SOL_MINT
            && account.account
                == get_associated_token_address_with_program_id(&owner, &mint, &program).to_string()
    }) else {
        return;
    };

    if wsol.balance > 0 {
        logger::debug(
            LogTag::Wallet,
            &format!("WSOL account {} holds a balance; left open", wsol.account),
        );
        return;
    }
    // Re-check: the account read took an RPC round trip.
    if trading_is_busy() {
        return;
    }

    match close_ata(&wallet, &wsol.account, SOL_MINT, wsol.is_token_2022).await {
        Ok(signature) => logger::info(
            LogTag::Wallet,
            &format!(
                "Closed the empty WSOL account {} left by a swap, reclaiming its rent: {signature}",
                wsol.account
            ),
        ),
        Err(e) => logger::warning(
            LogTag::Wallet,
            &format!(
                "Could not close the empty WSOL account {}: {e}",
                wsol.account
            ),
        ),
    }
}

/// True once trading is idle; false if it stayed busy for every retry.
async fn wait_for_idle() -> bool {
    for _ in 0..MAX_BUSY_RETRIES {
        if !trading_is_busy() {
            return true;
        }
        tokio::time::sleep(BUSY_RETRY_DELAY).await;
    }
    !trading_is_busy()
}

fn trading_is_busy() -> bool {
    crate::global::are_trades_active()
        || crate::global::are_tools_active()
        || crate::apis::jupiter::throttle::swap_in_flight()
}
