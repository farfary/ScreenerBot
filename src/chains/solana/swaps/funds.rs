// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Refuses a native-coin buy the signing wallet cannot fund, before any router is asked to build it.

use crate::chains::solana::assets::ata::get_all_token_accounts;
use crate::chains::solana::rent;
use crate::chains::solana::rpc::{try_get_rpc_client, RpcClientMethods, TokenAccountInfo};
use crate::chains::ChainId;
use crate::logger::{self, LogTag};
use crate::swaps::funds::native_needed;
use crate::swaps::{NotSubmittedReason, Quote, SwapExecutionError, SwapMode};
use crate::Error;

/// Accounts a native-input swap may open whatever the wallet already holds: the
/// wrapped-native account the route opens and closes, and the output token account.
const MOST_ACCOUNTS_OPENED: u64 = 2;

/// Whether one of the wallet's token accounts is for `mint`.
fn holds_account(accounts: &[TokenAccountInfo], mint: &str) -> bool {
    accounts.iter().any(|account| account.mint == mint)
}

/// Accounts a swap from `input_mint` into `output_mint` opens for a wallet holding
/// `accounts`: each side's token account the wallet does not keep yet. A native-input
/// swap opens a wrapped-native account and closes it again, but its rent must be in the
/// wallet while the swap runs.
fn accounts_to_open(accounts: &[TokenAccountInfo], input_mint: &str, output_mint: &str) -> u64 {
    u64::from(!holds_account(accounts, input_mint))
        + u64::from(!holds_account(accounts, output_mint))
}

/// Rent a buy of `output_mint` with the native coin (`native_mint`) needs for the
/// accounts it opens: the token-account minimum (`minimum`, read only when needed) for
/// each account `accounts` (the wallet's) lack, zero when the wallet keeps both.
async fn rent_to_open<F, Fut>(
    accounts: &[TokenAccountInfo],
    native_mint: &str,
    output_mint: &str,
    minimum: F,
) -> Result<u64, Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = crate::chains::solana::Result<u64>>,
{
    let opened = accounts_to_open(accounts, native_mint, output_mint);
    if opened == 0 {
        return Ok(0);
    }
    Ok(native_needed(0, 0, minimum().await?, opened))
}

/// Rent, in lamports, that a buy of `mint` with the native coin needs for the accounts
/// it opens in `wallet`, counted as [`check_native_funds`] counts them.
pub(crate) async fn buy_account_rent(wallet: &str, mint: &str) -> Result<u64, Error> {
    let accounts = get_all_token_accounts(wallet).await?;
    let native_mint = crate::chains::adapter_for(ChainId::Solana).native_asset_address();
    rent_to_open(&accounts, native_mint, mint, || {
        rent::minimum_balance(rent::TOKEN_ACCOUNT_LEN)
    })
    .await
}

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
        Ok(accounts) => accounts_to_open(&accounts, &quote.input_mint, &quote.output_mint),
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

#[cfg(test)]
mod tests {
    use super::{accounts_to_open, holds_account, rent_to_open};
    use crate::chains::solana::rpc::TokenAccountInfo;

    const NATIVE: &str = "So11111111111111111111111111111111111111112";
    const MINT: &str = "7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr";

    fn account(mint: &str) -> TokenAccountInfo {
        TokenAccountInfo {
            account: "account".to_string(),
            mint: mint.to_string(),
            balance: 0,
            decimals: 6,
            is_token_2022: false,
            is_nft: false,
            is_frozen: false,
            lamports: 1_461_600,
        }
    }

    #[test]
    fn only_an_account_for_the_same_mint_is_held() {
        assert!(holds_account(&[account("other"), account(MINT)], MINT));
        assert!(!holds_account(&[account("other")], MINT));
        assert!(!holds_account(&[], MINT));
    }

    #[test]
    fn a_swap_opens_each_side_the_wallet_does_not_keep() {
        assert_eq!(accounts_to_open(&[], NATIVE, MINT), 2);
        assert_eq!(accounts_to_open(&[account(NATIVE)], NATIVE, MINT), 1);
        assert_eq!(accounts_to_open(&[account(MINT)], NATIVE, MINT), 1);
        assert_eq!(
            accounts_to_open(&[account(MINT), account(NATIVE)], NATIVE, MINT),
            0
        );
    }

    const RENT: u64 = 1_488_440;

    #[tokio::test]
    async fn a_wallet_without_a_wrapped_native_account_pays_rent_for_both() {
        let rent = rent_to_open(&[account("other")], NATIVE, MINT, || async { Ok(RENT) })
            .await
            .expect("rent");
        assert_eq!(rent, 2 * RENT);
    }

    #[tokio::test]
    async fn a_held_token_account_leaves_the_wrapped_native_rent() {
        let rent = rent_to_open(&[account(MINT)], NATIVE, MINT, || async { Ok(RENT) })
            .await
            .expect("rent");
        assert_eq!(rent, RENT);
    }

    #[tokio::test]
    async fn a_wallet_keeping_both_accounts_pays_no_rent_and_reads_none() {
        let rent = rent_to_open(&[account(MINT), account(NATIVE)], NATIVE, MINT, || async {
            panic!("the rent is read only for an account the buy opens")
        })
        .await
        .expect("rent");
        assert_eq!(rent, 0);
    }
}
