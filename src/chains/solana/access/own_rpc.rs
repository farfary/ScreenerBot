// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The own-RPC backend of Solana chain access: every read and send through the user's configured RPC endpoints.
//!
//! The only file outside `chains/solana/rpc/` that obtains the process RPC
//! client; features that exist only with an own RPC obtain it through
//! [`own_rpc_client`].

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use super::facts::{
    account_control, decode_mint, AccountControl, Freshness, MintLookup, NetworkFacts,
};
use crate::chains::access::{self as chain_access, Capability};
use crate::chains::solana::rpc::{
    get_rpc_client, try_get_rpc_client, RpcClient, RpcClientMethods, SignatureInfo,
    TokenAccountInfo, TokenAccountsSnapshot, TransactionDetails,
};
use crate::chains::solana::settlement::{
    FinalizedTip, EXPIRY_BOUND_COMMITMENT, SOLANA_BLOCKHASH_VALIDITY_SLOTS,
};
use crate::chains::solana::solana_sdk::commitment_config::CommitmentLevel;
use crate::chains::solana::solana_sdk::hash::Hash;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::solana_sdk::signature::Signature;
use crate::chains::solana::solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, TransactionStatus,
};
use crate::chains::ChainId;

/// Accounts read per `getMultipleAccounts` call.
const ACCOUNTS_PER_READ: usize = 50;

/// The settle loop's status cadence against an own RPC.
const STATUS_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// The process RPC client, for a feature that exists only with an own RPC.
/// `CapabilityUnavailable` when the access mode lacks `capability`.
pub fn own_rpc_client(capability: Capability) -> crate::chains::Result<&'static RpcClient> {
    chain_access::require(ChainId::Solana, capability)?;
    Ok(get_rpc_client())
}

/// Reads and sends through the user's own RPC endpoints.
pub struct OwnRpcBackend;

fn parse_pubkey(kind: &'static str, value: &str) -> crate::Result<Pubkey> {
    Pubkey::from_str(value).map_err(|_| {
        crate::chains::solana::Error::InvalidAddress {
            kind,
            value: value.to_owned(),
        }
        .into()
    })
}

impl OwnRpcBackend {
    /// The process RPC client, for the pre-send gate's simulate and send.
    pub fn client(&self) -> &'static RpcClient {
        get_rpc_client()
    }

    /// Whether the process RPC client is running. A caller that must not
    /// start it (and must never fail on an unstartable client) checks this
    /// first.
    pub fn started(&self) -> bool {
        try_get_rpc_client().is_some()
    }

    pub async fn sol_balance(&self, wallet: &str) -> crate::Result<f64> {
        get_rpc_client().get_sol_balance(wallet).await
    }

    pub async fn lamports_many(&self, addresses: &[Pubkey]) -> crate::Result<Vec<Option<u64>>> {
        let mut lamports = Vec::with_capacity(addresses.len());
        for chunk in addresses.chunks(ACCOUNTS_PER_READ) {
            let accounts = get_rpc_client().get_multiple_accounts(chunk).await?;
            lamports.extend(
                accounts
                    .into_iter()
                    .map(|account| Some(account.map_or(0, |account| account.lamports))),
            );
        }
        Ok(lamports)
    }

    pub async fn token_balance(&self, wallet: &str, mint: &str) -> crate::Result<u64> {
        get_rpc_client().get_token_balance(wallet, mint).await
    }

    pub async fn token_accounts(
        &self,
        owner: &Pubkey,
        _freshness: Freshness,
    ) -> crate::Result<TokenAccountsSnapshot> {
        get_rpc_client().get_token_accounts_snapshot(owner).await
    }

    pub async fn all_token_accounts(&self, owner: &str) -> crate::Result<Vec<TokenAccountInfo>> {
        get_rpc_client().get_all_token_accounts_str(owner).await
    }

    pub async fn associated_token_account(
        &self,
        wallet: &str,
        mint: &str,
    ) -> crate::Result<String> {
        get_rpc_client()
            .get_associated_token_account(wallet, mint)
            .await
    }

    pub async fn is_token_account_token_2022(&self, token_account: &str) -> crate::Result<bool> {
        get_rpc_client()
            .is_token_account_token_2022(token_account)
            .await
    }

    pub async fn account_exists(&self, address: &Pubkey) -> crate::Result<bool> {
        Ok(get_rpc_client().get_account(address).await?.is_some())
    }

    pub async fn wallet_signatures(
        &self,
        wallet: &Pubkey,
        limit: usize,
        before: Option<&str>,
        until: Option<&str>,
    ) -> crate::Result<Vec<SignatureInfo>> {
        get_rpc_client()
            .get_wallet_signatures_main_rpc(wallet, limit, before, until)
            .await
    }

    pub async fn transaction(
        &self,
        signature: &Signature,
    ) -> crate::Result<Option<EncodedConfirmedTransactionWithStatusMeta>> {
        get_rpc_client().get_transaction(signature).await
    }

    pub async fn transaction_details(
        &self,
        signature: &str,
        commitment: CommitmentLevel,
    ) -> crate::Result<TransactionDetails> {
        get_rpc_client()
            .get_transaction_details_with_commitment(signature, commitment)
            .await
    }

    pub async fn mints(
        &self,
        mints: &[String],
        with_metadata: bool,
    ) -> crate::Result<Vec<MintLookup>> {
        let keys = mints
            .iter()
            .map(|mint| parse_pubkey("mint", mint))
            .collect::<crate::Result<Vec<_>>>()?;
        let mut lookups = Vec::with_capacity(mints.len());
        for (chunk, names) in keys
            .chunks(ACCOUNTS_PER_READ)
            .zip(mints.chunks(ACCOUNTS_PER_READ))
        {
            let accounts = get_rpc_client().get_multiple_accounts(chunk).await?;
            lookups.extend(
                names
                    .iter()
                    .zip(accounts)
                    .map(|(mint, account)| match account {
                        Some(account) => decode_mint(mint, &account),
                        None => MintLookup::Absent,
                    }),
            );
        }

        if with_metadata {
            let metadata = self.token_metadata(&keys).await?;
            for (lookup, metadata) in lookups.iter_mut().zip(metadata) {
                if let MintLookup::Mint(facts) = lookup {
                    facts.metadata = metadata;
                }
            }
        }
        Ok(lookups)
    }

    /// The Metaplex metadata of each mint, `None` where it is absent or does
    /// not decode.
    async fn token_metadata(
        &self,
        mints: &[Pubkey],
    ) -> crate::Result<Vec<Option<super::TokenMetadata>>> {
        use crate::chains::solana::assets::metaplex::{decode_metadata_account, metadata_address};
        let pdas: Vec<Option<Pubkey>> = mints.iter().map(metadata_address).collect();
        let readable: Vec<Pubkey> = pdas.iter().flatten().copied().collect();
        let mut decoded: HashMap<Pubkey, super::TokenMetadata> = HashMap::new();
        for chunk in readable.chunks(ACCOUNTS_PER_READ) {
            let accounts = get_rpc_client().get_multiple_accounts(chunk).await?;
            for (pda, account) in chunk.iter().zip(accounts) {
                if let Some(metadata) =
                    account.and_then(|account| decode_metadata_account(&account.data))
                {
                    decoded.insert(*pda, metadata);
                }
            }
        }
        Ok(pdas
            .into_iter()
            .map(|pda| pda.and_then(|pda| decoded.remove(&pda)))
            .collect())
    }

    pub async fn account_controls(
        &self,
        addresses: &[String],
    ) -> crate::Result<HashMap<String, AccountControl>> {
        let keys: Vec<Pubkey> = addresses
            .iter()
            .filter_map(|address| Pubkey::from_str(address).ok())
            .collect();
        let mut controls = HashMap::new();
        for chunk in keys.chunks(ACCOUNTS_PER_READ) {
            let accounts = get_rpc_client().get_multiple_accounts(chunk).await?;
            for (key, account) in chunk.iter().zip(accounts) {
                if let Some(account) = account {
                    let program = account.owner.to_string();
                    controls.insert(key.to_string(), account_control(program, &account.data));
                }
            }
        }
        Ok(controls)
    }

    pub async fn network(&self) -> crate::Result<NetworkFacts> {
        let rpc = get_rpc_client();
        let (slot, confirmed_height) = rpc
            .get_slot_and_block_height(CommitmentLevel::Confirmed)
            .await?;
        let (_, finalized_height) = rpc
            .get_slot_and_block_height(CommitmentLevel::Finalized)
            .await?;
        let epoch = rpc.get_epoch().await?;
        Ok(NetworkFacts {
            slot,
            epoch,
            confirmed_height,
            finalized_height,
        })
    }

    pub async fn finalized_tip(&self) -> crate::Result<FinalizedTip> {
        crate::chains::solana::settlement::finalized_tip(get_rpc_client()).await
    }

    pub async fn expiry_bound(&self) -> crate::Result<u64> {
        let height = get_rpc_client()
            .get_block_height_with_commitment(EXPIRY_BOUND_COMMITMENT)
            .await?;
        Ok(height.saturating_add(SOLANA_BLOCKHASH_VALIDITY_SLOTS))
    }

    pub async fn epoch(&self) -> crate::Result<u64> {
        get_rpc_client().get_epoch().await
    }

    pub async fn rent_minimum(&self, data_len: usize) -> crate::Result<u64> {
        get_rpc_client()
            .get_minimum_balance_for_rent_exemption(data_len)
            .await
    }

    pub async fn latest_blockhash(&self) -> crate::Result<(Hash, u64)> {
        get_rpc_client()
            .get_latest_blockhash_with_commitment(CommitmentLevel::Confirmed)
            .await
    }

    pub async fn signature_statuses(
        &self,
        signatures: &[Signature],
    ) -> crate::Result<(u64, Vec<Option<TransactionStatus>>)> {
        get_rpc_client().get_signature_statuses(signatures).await
    }

    pub fn status_poll_interval(&self, _elapsed: Duration) -> Duration {
        STATUS_POLL_INTERVAL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A process that never started its RPC client reports so without
    /// starting one, so a caller can skip a read instead of panicking on a
    /// client that cannot be built.
    #[tokio::test(flavor = "current_thread")]
    async fn an_unstarted_client_is_reported_without_starting_it() {
        assert!(!OwnRpcBackend.started());
        assert!(try_get_rpc_client().is_none());
    }
}
