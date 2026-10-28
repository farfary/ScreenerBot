// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The one way Solana chain data is read: a facade over the own-RPC and the service backend.
//!
//! Simulation and submission dispatch over the same two backends inside the
//! pre-send gate (`chains::solana::swaps::presend`), the only caller allowed to
//! simulate or send.
//!
//! The backend is chosen once at boot from `chains::access::mode` and every
//! method is a `match` over the two (static dispatch, no `dyn` async). Callers
//! receive the same types from either backend, so they change call sites, not
//! data handling. Features that only an own RPC provides obtain the client
//! through [`own_rpc_client`], which refuses in service mode.

mod facts;
mod own_rpc;
mod service;
pub mod wire;

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

pub use facts::{
    account_control, decode_mint, AccountControl, Freshness, MintFacts, MintLookup, NetworkFacts,
    TokenMetadata, TokenProgram, TransferFeeEpoch, TransferFeeSchedule,
};
pub use own_rpc::own_rpc_client;
pub use service::{probe_service, ServiceBackend, SERVICE_HISTORY_SIGNATURES};

use crate::chains::access::{self as chain_access, ChainAccessMode};
use crate::chains::solana::rpc::{SignatureInfo, TokenAccountInfo, TokenAccountsSnapshot};
use crate::chains::solana::settlement::FinalizedTip;
use crate::chains::solana::solana_sdk::commitment_config::CommitmentLevel;
use crate::chains::solana::solana_sdk::hash::Hash;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::solana_sdk::signature::Signature;
use crate::chains::solana::solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, TransactionStatus,
};
use crate::chains::ChainId;

use own_rpc::OwnRpcBackend;

/// Solana chain access, bound to the install's access mode.
pub enum SolanaChainAccess {
    OwnRpc(OwnRpcBackend),
    Service(ServiceBackend),
}

static ACCESS: OnceLock<SolanaChainAccess> = OnceLock::new();

/// The process's Solana chain access.
pub fn chain_access() -> &'static SolanaChainAccess {
    ACCESS.get_or_init(|| match chain_access::mode(ChainId::Solana) {
        ChainAccessMode::OwnRpc => SolanaChainAccess::OwnRpc(OwnRpcBackend),
        ChainAccessMode::Service => SolanaChainAccess::Service(ServiceBackend::new()),
    })
}

/// The service backend, `None` under own-RPC access. Used by the service
/// wallet-watch runtime only.
pub(crate) fn service_backend() -> Option<&'static ServiceBackend> {
    match chain_access() {
        SolanaChainAccess::OwnRpc(_) => None,
        SolanaChainAccess::Service(backend) => Some(backend),
    }
}

/// The trading wallet changed through a swap or activity of this install, so
/// the next reads must not serve a cached balance. The own RPC caches none, and
/// a process that has not built its chain access yet holds no cache to drop.
pub fn wallet_changed() {
    if let Some(SolanaChainAccess::Service(backend)) = ACCESS.get() {
        backend.wallet_changed();
    }
}

/// Probes chain access once at boot, so the service state leaves `Checking`
/// before the first trading cycle instead of at the next connectivity check.
/// Under service access one network read, whose outcome the backend records;
/// nothing under own-RPC access.
pub fn start_boot_probe() {
    let SolanaChainAccess::Service(backend) = chain_access() else {
        return;
    };
    tokio::spawn(async move {
        if let Err(error) = backend.network().await {
            crate::logger::info(
                crate::logger::LogTag::System,
                &format!("Chain access service not ready at boot: {error}"),
            );
        }
    });
}

/// Dispatches one call to the active backend.
macro_rules! dispatch {
    ($self:ident, $backend:ident => $call:expr) => {
        match $self {
            SolanaChainAccess::OwnRpc($backend) => $call,
            SolanaChainAccess::Service($backend) => $call,
        }
    };
}

impl SolanaChainAccess {
    /// Whether reads can be issued without starting a client: always under
    /// service access, only once the process RPC client runs under own-RPC
    /// access.
    pub fn started(&self) -> bool {
        match self {
            SolanaChainAccess::OwnRpc(backend) => backend.started(),
            SolanaChainAccess::Service(_) => true,
        }
    }

    /// A wallet's balance in SOL.
    pub async fn sol_balance(&self, wallet: &str) -> crate::Result<f64> {
        dispatch!(self, backend => backend.sol_balance(wallet).await)
    }

    /// Lamports per address: zero for an account that does not exist, `None`
    /// for one this mode cannot read.
    pub async fn lamports_many(&self, addresses: &[Pubkey]) -> crate::Result<Vec<Option<u64>>> {
        dispatch!(self, backend => backend.lamports_many(addresses).await)
    }

    /// The balance of the wallet's associated token account of `mint`.
    pub async fn token_balance(&self, wallet: &str, mint: &str) -> crate::Result<u64> {
        dispatch!(self, backend => backend.token_balance(wallet, mint).await)
    }

    /// Every token account of `owner` with the slot they were read at.
    pub async fn token_accounts(
        &self,
        owner: &Pubkey,
        freshness: Freshness,
    ) -> crate::Result<TokenAccountsSnapshot> {
        dispatch!(self, backend => backend.token_accounts(owner, freshness).await)
    }

    /// Every token account of `owner`.
    pub async fn all_token_accounts(&self, owner: &str) -> crate::Result<Vec<TokenAccountInfo>> {
        dispatch!(self, backend => backend.all_token_accounts(owner).await)
    }

    /// The address of the wallet's existing associated token account of `mint`.
    pub async fn associated_token_account(
        &self,
        wallet: &str,
        mint: &str,
    ) -> crate::Result<String> {
        dispatch!(self, backend => backend.associated_token_account(wallet, mint).await)
    }

    /// Whether a token account belongs to the Token-2022 program.
    pub async fn is_token_account_token_2022(&self, token_account: &str) -> crate::Result<bool> {
        dispatch!(self, backend => backend.is_token_account_token_2022(token_account).await)
    }

    /// Whether an account exists.
    pub async fn account_exists(&self, address: &Pubkey) -> crate::Result<bool> {
        dispatch!(self, backend => backend.account_exists(address).await)
    }

    /// One page of a wallet's signatures, newest first.
    pub async fn wallet_signatures(
        &self,
        wallet: &Pubkey,
        limit: usize,
        before: Option<&str>,
        until: Option<&str>,
    ) -> crate::Result<Vec<SignatureInfo>> {
        dispatch!(self, backend => backend.wallet_signatures(wallet, limit, before, until).await)
    }

    /// The executed transaction for a signature at confirmed commitment,
    /// `None` when the chain holds none.
    pub async fn transaction(
        &self,
        signature: &Signature,
    ) -> crate::Result<Option<EncodedConfirmedTransactionWithStatusMeta>> {
        dispatch!(self, backend => backend.transaction(signature).await)
    }

    /// A transaction's details at `commitment`; a transaction the chain does
    /// not hold is the tagged `DataError::ParseError` of `data_type`
    /// "transaction".
    pub async fn transaction_details(
        &self,
        signature: &str,
        commitment: CommitmentLevel,
    ) -> crate::Result<crate::chains::solana::rpc::TransactionDetails> {
        dispatch!(self, backend => backend.transaction_details(signature, commitment).await)
    }

    /// One lookup per mint, in order.
    pub async fn mints(
        &self,
        mints: &[String],
        with_metadata: bool,
    ) -> crate::Result<Vec<MintLookup>> {
        dispatch!(self, backend => backend.mints(mints, with_metadata).await)
    }

    /// Who controls each existing account; an absent address does not exist.
    pub async fn account_controls(
        &self,
        addresses: &[String],
    ) -> crate::Result<HashMap<String, AccountControl>> {
        dispatch!(self, backend => backend.account_controls(addresses).await)
    }

    /// The network's slot, epoch and block heights.
    pub async fn network(&self) -> crate::Result<NetworkFacts> {
        dispatch!(self, backend => backend.network().await)
    }

    /// The finalized tip an expiry is judged against.
    pub async fn finalized_tip(&self) -> crate::Result<FinalizedTip> {
        dispatch!(self, backend => backend.finalized_tip().await)
    }

    /// The highest block height a blockhash fetched before this call can
    /// still land at. Never lower than the true bound: a low bound would call
    /// a live transaction expired.
    pub async fn expiry_bound(&self) -> crate::Result<u64> {
        dispatch!(self, backend => backend.expiry_bound().await)
    }

    /// The current epoch.
    pub async fn epoch(&self) -> crate::Result<u64> {
        dispatch!(self, backend => backend.epoch().await)
    }

    /// The rent-exempt minimum of an account holding `data_len` bytes.
    pub async fn rent_minimum(&self, data_len: usize) -> crate::Result<u64> {
        dispatch!(self, backend => backend.rent_minimum(data_len).await)
    }

    /// A recent blockhash at confirmed commitment and its last valid height.
    pub async fn latest_blockhash(&self) -> crate::Result<(Hash, u64)> {
        dispatch!(self, backend => backend.latest_blockhash().await)
    }

    /// The context slot and one status per signature.
    pub async fn signature_statuses(
        &self,
        signatures: &[Signature],
    ) -> crate::Result<(u64, Vec<Option<TransactionStatus>>)> {
        dispatch!(self, backend => backend.signature_statuses(signatures).await)
    }

    /// How long the settle loop waits between status reads, `elapsed` after
    /// the first send.
    pub fn status_poll_interval(&self, elapsed: Duration) -> Duration {
        dispatch!(self, backend => backend.status_poll_interval(elapsed))
    }
}
