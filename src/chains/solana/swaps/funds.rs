// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Refuses a native-coin buy the signing wallet cannot fund, before any router is asked to build it.

use crate::chains::solana::assets::ata::get_all_token_accounts;
use crate::chains::solana::rent;
use crate::chains::solana::rpc::{try_get_rpc_client, RpcClientMethods};
use crate::logger::{self, LogTag};
use crate::swaps::funds::native_needed;
use crate::swaps::{NotSubmittedReason, Quote, SwapExecutionError, SwapMode};
use crate::Error;

/// Accounts a native-input swap may open whatever the wallet already holds: the
/// wrapped-native account the route opens and closes, and the output token account.
const MOST_ACCOUNTS_OPENED: u64 = 2;

/// Fails with a not-submitted [`NotSubmittedReason::InsufficientFunds`] when the
/// quote spends the chain's native coin and the signing wallet holds less than the
/// trade, the network fee and the accounts the swap may open need. Any other quote
/// passes, and so does one whose balance cannot be read: the simulation remains the
/// backstop, and a balance outage (or an RPC client not yet started) must not stop trading.
pub(crate) async fn check_native_funds(quote: &Quote) -> Result<(), Error> {
    let adapter = crate::chains::adapter_for(quote.chain);
    if quote.swap_mode != SwapMode::ExactIn || quote.input_mint != adapter.native_asset_address() {
        return Ok(());
    }

    let Some(rpc) = try_get_rpc_client() else {
        return Ok(());
    };
    let available_raw = match rpc.get_sol_balance(&quote.wallet_address).await {
        Ok(balance) => adapter.native_to_raw(balance),
        Err(e) => {
            logger::warning(
                LogTag::Swap,
                &format!(
                    "Native balance of {} unreadable, funds check skipped: {e}",
                    quote.wallet_address
                ),
            );
            return Ok(());
        }
    };

    let input_raw = quote.input_amount.raw().try_into().unwrap_or(u64::MAX);
    let fee_raw = quote.estimated_network_fee_lamports.unwrap_or(0);
    let rent_raw = match rent::minimum_balance(rent::TOKEN_ACCOUNT_LEN).await {
        Ok(rent_raw) => rent_raw,
        Err(e) => {
            logger::warning(
                LogTag::Swap,
                &format!("Token account rent unreadable, funds check skipped: {e}"),
            );
            return Ok(());
        }
    };

    if available_raw >= native_needed(input_raw, fee_raw, rent_raw, MOST_ACCOUNTS_OPENED) {
        return Ok(());
    }

    // An account the wallet already keeps is not opened again: neither a standing
    // wrapped-native account nor an output token account costs rent.
    let accounts_to_open = match get_all_token_accounts(&quote.wallet_address).await {
        Ok(accounts) => {
            let held = |mint: &str| accounts.iter().any(|account| account.mint == mint);
            u64::from(!held(&quote.input_mint)) + u64::from(!held(&quote.output_mint))
        }
        Err(e) => {
            logger::warning(
                LogTag::Swap,
                &format!(
                    "Token accounts of {} unreadable, funds check skipped: {e}",
                    quote.wallet_address
                ),
            );
            return Ok(());
        }
    };
    let required_raw = native_needed(input_raw, fee_raw, rent_raw, accounts_to_open);
    if available_raw >= required_raw {
        return Ok(());
    }

    logger::info(
        LogTag::Swap,
        &format!(
            "Swap of {} refused: wallet {} holds {} and the trade needs {} ({} short)",
            quote.output_mint,
            quote.wallet_address,
            adapter.raw_to_native(available_raw),
            adapter.raw_to_native(required_raw),
            adapter.raw_to_native(required_raw - available_raw),
        ),
    );
    Err(Error::Swaps(SwapExecutionError::NotSubmitted {
        router: quote.router_name.clone(),
        reason: NotSubmittedReason::InsufficientFunds {
            required_raw,
            available_raw,
        },
    }))
}
