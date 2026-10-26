// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Solana wallet balance reads: Pubkey parsing and batched RPC SOL balance
//! fetches. Callers in `crate::wallets` own presentation; this module owns
//! everything that touches an RPC client or a `Pubkey` to produce that data.

use std::future::Future;
use std::str::FromStr;

use crate::chains::solana::constants::lamports_to_sol;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// Accounts read per `getMultipleAccounts` call.
pub(crate) const SOL_BALANCE_BATCH_SIZE: usize = 50;

/// Fetch the SOL balances of many wallets (in SOL), reading at most
/// `SOL_BALANCE_BATCH_SIZE` accounts per `getMultipleAccounts` call.
///
/// The result is aligned with `addresses`. A wallet account that does not exist
/// holds 0 SOL; `None` marks an address that does not parse or a batch the RPC did
/// not answer, so the caller can show the balance as unknown instead of zero.
pub async fn fetch_wallet_sol_balances(addresses: &[String]) -> Vec<Option<f64>> {
    let pubkeys: Vec<Option<Pubkey>> = addresses
        .iter()
        .map(|address| Pubkey::from_str(address).ok())
        .collect();
    let client = get_rpc_client();
    sol_balances_in_batches(&pubkeys, |batch| async move {
        client.get_multiple_accounts(&batch).await.map(|accounts| {
            accounts
                .into_iter()
                .map(|account| account.map(|account| account.lamports))
                .collect()
        })
    })
    .await
}

/// Batches the parsed keys through `fetch` (lamports per key, `None` for a missing
/// account) and lines the balances up with `pubkeys`.
async fn sol_balances_in_batches<F, Fut, E>(
    pubkeys: &[Option<Pubkey>],
    fetch: F,
) -> Vec<Option<f64>>
where
    F: Fn(Vec<Pubkey>) -> Fut,
    Fut: Future<Output = std::result::Result<Vec<Option<u64>>, E>>,
{
    let mut balances = vec![None; pubkeys.len()];
    let parsed: Vec<(usize, Pubkey)> = pubkeys
        .iter()
        .enumerate()
        .filter_map(|(index, key)| key.map(|key| (index, key)))
        .collect();
    for batch in parsed.chunks(SOL_BALANCE_BATCH_SIZE) {
        let keys = batch.iter().map(|(_, key)| *key).collect();
        let Ok(lamports) = fetch(keys).await else {
            continue;
        };
        if lamports.len() != batch.len() {
            continue;
        }
        for ((index, _), lamports) in batch.iter().zip(lamports) {
            balances[*index] = Some(lamports_to_sol(lamports.unwrap_or_default()));
        }
    }
    balances
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn keys(count: usize) -> Vec<Option<Pubkey>> {
        (0..count).map(|_| Some(Pubkey::new_unique())).collect()
    }

    #[tokio::test]
    async fn sol_balances_never_read_more_than_one_batch_per_call() {
        for count in [0, 1, 49, 50, 51, 100, 101, 237] {
            let calls = Mutex::new(Vec::new());
            let balances = sol_balances_in_batches(&keys(count), |batch| {
                calls.lock().unwrap().push(batch.len());
                async move { Ok::<_, ()>(vec![Some(2_500_000_000); batch.len()]) }
            })
            .await;
            let calls = calls.into_inner().unwrap();
            assert!(calls.iter().all(|&size| size <= SOL_BALANCE_BATCH_SIZE));
            assert_eq!(calls.iter().sum::<usize>(), count);
            assert_eq!(calls.len(), count.div_ceil(SOL_BALANCE_BATCH_SIZE));
            assert!(balances.iter().all(|balance| *balance == Some(2.5)));
        }
    }

    #[tokio::test]
    async fn sol_balances_keep_unknown_apart_from_zero() {
        // Index 1 does not parse, the account at index 2 does not exist (0 SOL), and
        // the second batch fails (unknown).
        let mut pubkeys = keys(SOL_BALANCE_BATCH_SIZE + 2);
        pubkeys[1] = None;
        let first = Mutex::new(true);
        let balances = sol_balances_in_batches(&pubkeys, |batch| {
            let is_first = std::mem::replace(&mut *first.lock().unwrap(), false);
            async move {
                if !is_first {
                    return Err(());
                }
                let mut lamports = vec![Some(1_000_000_000); batch.len()];
                lamports[1] = None;
                Ok(lamports)
            }
        })
        .await;
        assert_eq!(balances[0], Some(1.0));
        assert_eq!(balances[1], None);
        assert_eq!(balances[2], Some(0.0));
        assert_eq!(balances[SOL_BALANCE_BATCH_SIZE + 1], None);
    }
}
