// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The service backend of Solana chain access: typed reads of the trading wallet through the ScreenerBot service and submission through its relay.
//!
//! Every read is scoped to the trading wallet; the service refuses any other.
//! Every answer and refusal is published as the chain's `ServiceState`, so
//! the surfaces and the trader see one state. A send that may have been
//! delivered is reported as ambiguous, never as "not sent", so settlement
//! reads the chain before anything is rebuilt.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::watch;

use super::facts::{
    AccountControl, Freshness, MintFacts, MintLookup, NetworkFacts, TokenMetadata, TokenProgram,
    TransferFeeEpoch, TransferFeeSchedule,
};
use super::wire::{
    AccountsRequest, AccountsView, BalancesView, CommitmentView, MintFactView, MintsRequest,
    MintsView, NetworkView, ProgramView, SignaturesView, TokenAccountState, TransactionStatusView,
    TransactionsRequest, TransactionsView, WalletView,
};
use crate::chains::access::{self as chain_access, ServiceState};
use crate::chains::solana::constants::{
    lamports_to_sol, ASSOCIATED_TOKEN_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID,
};
use crate::chains::solana::rent::rent_minimum_from_rate;
use crate::chains::solana::rpc::client::{
    latest_blockhash_params, parse_latest_blockhash, parse_sent_signature,
    parse_signature_statuses, parse_simulation, send_transaction_params, signature_statuses_params,
    simulate_transaction_params,
};
use crate::chains::solana::rpc::types::SimulationOutcome;
use crate::chains::solana::rpc::{
    SignatureInfo, TokenAccountInfo, TokenAccountsSnapshot, TransactionDetails,
};
use crate::chains::solana::settlement::FinalizedTip;
use crate::chains::solana::solana_sdk::commitment_config::CommitmentLevel;
use crate::chains::solana::solana_sdk::hash::Hash;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::solana_sdk::signature::Signature;
use crate::chains::solana::solana_sdk::transaction::VersionedTransaction;
use crate::chains::solana::solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, TransactionStatus,
};
use crate::chains::ChainId;
use crate::data_server::{send_json, ServiceRefusal, Surface};
use crate::errors::DataError;
use crate::rpc::gateway::{relay, Relayed};
use crate::rpc::{RpcError, RpcMethod};

/// Without full history, a wallet's transaction bootstrap reads at most this
/// many of its newest signatures; older history loads on demand.
pub const SERVICE_HISTORY_SIGNATURES: usize = 200;

/// How often the trading wallet's version is polled.
pub(crate) const WALLET_POLL_INTERVAL: Duration = Duration::from_secs(10);
/// How long a wallet read answers balance reads without asking again.
const WALLET_MAX_AGE: Duration = Duration::from_secs(15);
/// How long token balances are reused while the wallet version is unchanged.
const BALANCES_MAX_AGE: Duration = Duration::from_secs(600);
/// How long the network view is reused.
const NETWORK_MAX_AGE: Duration = Duration::from_secs(2);
/// Items per service request, as the service caps them.
const FACTS_PER_READ: usize = 50;
const SIGNATURES_PER_READ: usize = 100;
/// Status cadence while a send is young, and after.
const STATUS_POLL_EARLY: Duration = Duration::from_secs(1);
const STATUS_POLL_LATE: Duration = Duration::from_secs(2);
const STATUS_POLL_EARLY_WINDOW: Duration = Duration::from_secs(10);

/// Reads through the ScreenerBot service and sends through its relay.
pub struct ServiceBackend {
    http: reqwest::Client,
    wallet: Mutex<Option<(WalletView, Instant)>>,
    balances: Mutex<Option<(BalancesView, Instant)>>,
    network: Mutex<Option<(NetworkView, Instant)>>,
    version: watch::Sender<u64>,
    connected: watch::Sender<bool>,
    poller: OnceLock<()>,
    /// Set when this install changed the wallet: the next balances read asks
    /// the service for a fresh read instead of its cached one.
    fresh_requested: AtomicBool,
}

impl ServiceBackend {
    pub(super) fn new() -> Self {
        Self {
            http: crate::net::client(),
            wallet: Mutex::new(None),
            balances: Mutex::new(None),
            network: Mutex::new(None),
            version: watch::channel(0).0,
            connected: watch::channel(false).0,
            poller: OnceLock::new(),
            fresh_requested: AtomicBool::new(false),
        }
    }

    // ------------------------------------------------------------------
    // Transport and state
    // ------------------------------------------------------------------

    /// Records a refusal as the service state and returns the domain error.
    fn map_service_refusal(&self, refusal: ServiceRefusal) -> crate::Error {
        let chain = ChainId::Solana;
        let state = state_for(&refusal);
        if let Some(state) = state.clone() {
            chain_access::record_service_outcome(chain, state);
        }
        // Offline or disabled refusals are decided locally before any request
        // leaves the process: provably nothing was dispatched.
        if matches!(refusal, ServiceRefusal::Disabled | ServiceRefusal::Offline) {
            return crate::chains::Error::ServiceNotContacted { chain }.into();
        }
        let error = match (state.and_then(|state| state.refusal(chain)), refusal) {
            (Some(error), _) => error,
            (None, ServiceRefusal::WalletChangeLimited { .. }) => {
                crate::chains::Error::ServiceRequestRefused {
                    chain,
                    code: "wallet_change_limited".to_owned(),
                }
            }
            (None, ServiceRefusal::WalletNotBound) => crate::chains::Error::ServiceRequestRefused {
                chain,
                code: "wallet_not_bound".to_owned(),
            },
            (None, ServiceRefusal::NotFound) => crate::chains::Error::ServiceRequestRefused {
                chain,
                code: "not_found".to_owned(),
            },
            (None, ServiceRefusal::Invalid { code }) => {
                crate::chains::Error::ServiceRequestRefused { chain, code }
            }
            (None, _) => crate::chains::Error::ServiceUnreachable {
                chain,
                detail: "the service answer could not be read".to_owned(),
            },
        };
        error.into()
    }

    /// One classified service request.
    async fn call<B, T>(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> Result<T, ServiceRefusal>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let result = send_json(Surface::Chain, ChainId::Solana, method, path, query, body).await;
        if result.is_ok() {
            chain_access::record_service_outcome(ChainId::Solana, ServiceState::Ready);
        }
        result
    }

    /// A wallet-scoped request; a wallet the service has not bound to this
    /// device yet is bound once and the request asked again.
    async fn wallet_call<B, T>(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> crate::Result<T>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        match self.call(method.clone(), path, query, body).await {
            Ok(value) => Ok(value),
            Err(ServiceRefusal::WalletNotBound) => {
                self.refresh_wallet().await?;
                self.call(method, path, query, body)
                    .await
                    .map_err(|refusal| self.map_service_refusal(refusal))
            }
            Err(refusal) => Err(self.map_service_refusal(refusal)),
        }
    }

    /// The configured trading wallet.
    async fn trading_wallet(&self) -> crate::Result<String> {
        Ok(crate::chains::solana::accounts::configured_address_async().await?)
    }

    /// `Err(NotTradingWallet)` for any address but the trading wallet.
    async fn require_trading_wallet(&self, address: &str) -> crate::Result<String> {
        let wallet = self.trading_wallet().await?;
        if wallet != address {
            return Err(crate::chains::Error::NotTradingWallet {
                chain: ChainId::Solana,
                address: address.to_owned(),
            }
            .into());
        }
        Ok(wallet)
    }

    /// Reads the trading wallet (binding it to this device), records its
    /// version and publishes a change.
    pub(crate) async fn refresh_wallet(&self) -> crate::Result<WalletView> {
        let address = self.trading_wallet().await?;
        let read = self
            .call::<(), WalletView>(
                reqwest::Method::GET,
                "/v1/chain/wallet",
                &[("address", address)],
                None,
            )
            .await;
        match read {
            Ok(view) => {
                self.connected.send_replace(true);
                self.version.send_if_modified(|version| {
                    let changed = *version != view.version;
                    *version = view.version;
                    changed
                });
                *self.wallet.lock().expect("wallet cache lock") =
                    Some((view.clone(), Instant::now()));
                Ok(view)
            }
            Err(refusal) => {
                self.connected.send_replace(false);
                Err(self.map_service_refusal(refusal))
            }
        }
    }

    /// The trading wallet, from a recent read when there is one.
    async fn wallet_view(&self) -> crate::Result<WalletView> {
        let cached = self
            .wallet
            .lock()
            .expect("wallet cache lock")
            .as_ref()
            .filter(|(_, read_at)| read_at.elapsed() < WALLET_MAX_AGE)
            .map(|(view, _)| view.clone());
        match cached {
            Some(view) => Ok(view),
            None => self.refresh_wallet().await,
        }
    }

    /// The trading wallet changed through a swap of this install: the cached
    /// wallet and balances are dropped, and the next balances read asks the
    /// service for a fresh read.
    pub fn wallet_changed(&self) {
        *self.wallet.lock().expect("wallet cache lock") = None;
        *self.balances.lock().expect("balances cache lock") = None;
        self.fresh_requested.store(true, Ordering::Relaxed);
    }

    /// Every change of the trading wallet's version.
    pub(crate) fn subscribe_version(&self) -> watch::Receiver<u64> {
        self.version.subscribe()
    }

    /// Whether the last wallet poll was answered.
    pub(crate) fn connection(&self) -> watch::Receiver<bool> {
        self.connected.subscribe()
    }

    /// Starts the wallet version poller once: every interval, and at once
    /// after a sign-in or sign-out.
    pub(crate) fn start_poller(&'static self) {
        self.poller.get_or_init(|| {
            tokio::spawn(async move {
                loop {
                    if let Err(error) = self.refresh_wallet().await {
                        log::debug!("Service chain access: wallet poll failed: {error}");
                    }
                    tokio::select! {
                        _ = tokio::time::sleep(WALLET_POLL_INTERVAL) => {}
                        _ = chain_access::reprobe_requested() => {}
                    }
                }
            });
        });
    }

    /// Sends through the relay and maps the outcome. Only an outcome that
    /// proves nothing reached a node is a refusal; a send that may have been
    /// delivered is ambiguous.
    async fn relay(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> crate::Result<serde_json::Value> {
        let is_send = RpcMethod::from_str(method) == RpcMethod::SendTransaction;
        if is_send {
            crate::rpc::manager::refuse_send_in_paper_scope(method)?;
        }
        match relay(&self.http, method, &params).await {
            Relayed::Answered(result) => {
                chain_access::record_service_outcome(ChainId::Solana, ServiceState::Ready);
                Ok(result)
            }
            Relayed::NotAsked(refusal) => Err(self.map_service_refusal(refusal)),
            Relayed::Refused {
                error,
                after_delivery: true,
            } if is_send => Err(RpcError::RefusedAfterDelivery {
                earlier_attempts: 1,
                refusal: Box::new(error),
            }
            .into()),
            Relayed::Refused { error, .. } => Err(error.into()),
            Relayed::Unanswered {
                maybe_delivered: true,
            } => Err(RpcError::Network {
                message: format!("the submission relay did not answer {method}"),
                is_timeout: true,
            }
            .into()),
            Relayed::Unanswered {
                maybe_delivered: false,
            } => Err(crate::chains::Error::ServiceRequestRefused {
                chain: ChainId::Solana,
                code: "relay_unavailable".to_owned(),
            }
            .into()),
        }
    }

    // ------------------------------------------------------------------
    // Facade methods
    // ------------------------------------------------------------------

    pub async fn sol_balance(&self, wallet: &str) -> crate::Result<f64> {
        self.require_trading_wallet(wallet).await?;
        let view = self.wallet_view().await?;
        Ok(lamports_to_sol(view.lamports))
    }

    pub async fn lamports_many(&self, addresses: &[Pubkey]) -> crate::Result<Vec<Option<u64>>> {
        let wallet = self.trading_wallet().await?;
        if !addresses
            .iter()
            .any(|address| address.to_string() == wallet)
        {
            return Ok(vec![None; addresses.len()]);
        }
        let lamports = self.wallet_view().await?.lamports;
        Ok(addresses
            .iter()
            .map(|address| (address.to_string() == wallet).then_some(lamports))
            .collect())
    }

    async fn balances(&self, freshness: Freshness) -> crate::Result<BalancesView> {
        let version = self.wallet_view().await?.version;
        let cached = self
            .balances
            .lock()
            .expect("balances cache lock")
            .as_ref()
            .filter(|(view, read_at)| {
                view.version == version
                    && read_at.elapsed() < BALANCES_MAX_AGE
                    && match freshness {
                        Freshness::Any => true,
                        Freshness::AtLeastSlot(slot) => view.slot >= slot,
                    }
            })
            .map(|(view, _)| view.clone());
        if let Some(view) = cached {
            return Ok(view);
        }
        let fresh = matches!(freshness, Freshness::AtLeastSlot(_))
            | self.fresh_requested.swap(false, Ordering::Relaxed);
        let address = self.trading_wallet().await?;
        let view: BalancesView = self
            .wallet_call::<(), _>(
                reqwest::Method::GET,
                "/v1/chain/wallet/balances",
                &[
                    ("address", address),
                    ("fresh", if fresh { "1" } else { "0" }.to_owned()),
                ],
                None,
            )
            .await?;
        *self.balances.lock().expect("balances cache lock") = Some((view.clone(), Instant::now()));
        Ok(view)
    }

    pub async fn token_accounts(
        &self,
        owner: &Pubkey,
        freshness: Freshness,
    ) -> crate::Result<TokenAccountsSnapshot> {
        self.require_trading_wallet(&owner.to_string()).await?;
        Ok(snapshot_from(&self.balances(freshness).await?))
    }

    pub async fn all_token_accounts(&self, owner: &str) -> crate::Result<Vec<TokenAccountInfo>> {
        self.require_trading_wallet(owner).await?;
        Ok(snapshot_from(&self.balances(Freshness::Any).await?).accounts)
    }

    /// The wallet's existing associated token account of `mint`.
    async fn associated_account(
        &self,
        wallet: &str,
        mint: &str,
    ) -> crate::Result<Option<TokenAccountInfo>> {
        self.require_trading_wallet(wallet).await?;
        let snapshot = snapshot_from(&self.balances(Freshness::Any).await?);
        Ok(snapshot
            .accounts
            .into_iter()
            .find(|account| account.mint == mint && is_associated(wallet, account)))
    }

    pub async fn token_balance(&self, wallet: &str, mint: &str) -> crate::Result<u64> {
        Ok(self
            .associated_account(wallet, mint)
            .await?
            .map_or(0, |account| account.balance))
    }

    pub async fn associated_token_account(
        &self,
        wallet: &str,
        mint: &str,
    ) -> crate::Result<String> {
        match self.associated_account(wallet, mint).await? {
            Some(account) => Ok(account.account),
            None => Err(crate::chains::solana::Error::AccountNotFound {
                address: format!("associated token account of {mint}"),
            }
            .into()),
        }
    }

    pub async fn is_token_account_token_2022(&self, token_account: &str) -> crate::Result<bool> {
        match self
            .account_controls(&[token_account.to_owned()])
            .await?
            .remove(token_account)
        {
            Some(AccountControl::TokenAccount { program, .. })
            | Some(AccountControl::Program(program)) => Ok(program == TOKEN_2022_PROGRAM_ID),
            None => Err(crate::chains::solana::Error::AccountNotFound {
                address: token_account.to_owned(),
            }
            .into()),
        }
    }

    pub async fn account_exists(&self, address: &Pubkey) -> crate::Result<bool> {
        let address = address.to_string();
        Ok(self
            .account_controls(std::slice::from_ref(&address))
            .await?
            .contains_key(&address))
    }

    /// The trading wallet's signatures, newest first, up to `limit`, read in
    /// pages of at most [`SIGNATURES_PER_READ`]: a short answer means the
    /// history ended, exactly as one own-RPC read of `limit` reports it.
    pub async fn wallet_signatures(
        &self,
        wallet: &Pubkey,
        limit: usize,
        before: Option<&str>,
        until: Option<&str>,
    ) -> crate::Result<Vec<SignatureInfo>> {
        let address = self.require_trading_wallet(&wallet.to_string()).await?;
        let mut collected: Vec<SignatureInfo> = Vec::new();
        let mut cursor = before.map(str::to_owned);
        while collected.len() < limit {
            let page_limit = (limit - collected.len()).min(SIGNATURES_PER_READ);
            let mut query = vec![
                ("address", address.clone()),
                ("limit", page_limit.to_string()),
            ];
            if let Some(before) = &cursor {
                query.push(("before", before.clone()));
            }
            if let Some(until) = until {
                query.push(("until", until.to_owned()));
            }
            let view: SignaturesView = self
                .wallet_call::<(), _>(
                    reqwest::Method::GET,
                    "/v1/chain/wallet/signatures",
                    &query,
                    None,
                )
                .await?;
            let page_len = view.signatures.len();
            cursor = view.signatures.last().map(|entry| entry.signature.clone());
            for entry in view.signatures {
                collected.push(signature_info(entry)?);
            }
            if page_len < page_limit {
                break;
            }
        }
        Ok(collected)
    }

    /// One transaction body by signature, `None` when the service holds none
    /// for the trading wallet.
    async fn transaction_value(
        &self,
        signature: &str,
        commitment: CommitmentView,
    ) -> crate::Result<Option<serde_json::Value>> {
        let address = self.trading_wallet().await?;
        let request = TransactionsRequest {
            address: &address,
            signatures: vec![signature.to_owned()],
            commitment,
        };
        let view: TransactionsView = self
            .wallet_call(
                reqwest::Method::POST,
                "/v1/chain/transactions",
                &[],
                Some(&request),
            )
            .await?;
        Ok(view
            .transactions
            .into_iter()
            .find(|entry| entry.signature == signature)
            .filter(|entry| entry.status == TransactionStatusView::Found)
            .and_then(|entry| entry.transaction))
    }

    pub async fn transaction(
        &self,
        signature: &Signature,
    ) -> crate::Result<Option<EncodedConfirmedTransactionWithStatusMeta>> {
        let Some(value) = self
            .transaction_value(&signature.to_string(), CommitmentView::Confirmed)
            .await?
        else {
            return Ok(None);
        };
        serde_json::from_value(value).map(Some).map_err(|error| {
            crate::Error::Data(DataError::ParseError {
                data_type: "transaction".to_owned(),
                error: error.to_string(),
            })
        })
    }

    pub async fn transaction_details(
        &self,
        signature: &str,
        commitment: CommitmentLevel,
    ) -> crate::Result<TransactionDetails> {
        let commitment = match commitment {
            CommitmentLevel::Finalized => CommitmentView::Finalized,
            _ => CommitmentView::Confirmed,
        };
        let value = self
            .transaction_value(signature, commitment)
            .await?
            .ok_or_else(|| {
                crate::Error::Data(DataError::ParseError {
                    data_type: "transaction".to_string(),
                    error: format!("Transaction not found: {signature}"),
                })
            })?;
        serde_json::from_value(value).map_err(|error| {
            crate::Error::Data(DataError::ParseError {
                data_type: "transaction".to_owned(),
                error: format!("failed to parse transaction: {error}"),
            })
        })
    }

    pub async fn mints(
        &self,
        mints: &[String],
        with_metadata: bool,
    ) -> crate::Result<Vec<MintLookup>> {
        let mut lookups = Vec::with_capacity(mints.len());
        for chunk in mints.chunks(FACTS_PER_READ) {
            let request = MintsRequest {
                mints: chunk,
                metadata: with_metadata,
            };
            let view: MintsView = self
                .call(
                    reqwest::Method::POST,
                    "/v1/chain/mints",
                    &[],
                    Some(&request),
                )
                .await
                .map_err(|refusal| self.map_service_refusal(refusal))?;
            let mut by_mint: HashMap<String, MintFactView> = view
                .mints
                .into_iter()
                .map(|fact| (fact.mint.clone(), fact))
                .collect();
            lookups.extend(chunk.iter().map(|mint| match by_mint.remove(mint) {
                Some(fact) => mint_lookup(fact),
                None => MintLookup::Absent,
            }));
        }
        Ok(lookups)
    }

    pub async fn account_controls(
        &self,
        addresses: &[String],
    ) -> crate::Result<HashMap<String, AccountControl>> {
        let mut controls = HashMap::new();
        for chunk in addresses.chunks(FACTS_PER_READ) {
            let request = AccountsRequest { addresses: chunk };
            let view: AccountsView = self
                .call(
                    reqwest::Method::POST,
                    "/v1/chain/accounts",
                    &[],
                    Some(&request),
                )
                .await
                .map_err(|refusal| self.map_service_refusal(refusal))?;
            for account in view.accounts.into_iter().filter(|account| account.exists) {
                let Some(program) = account.owner_program else {
                    continue;
                };
                let control = match account.token_authority {
                    Some(authority) => AccountControl::TokenAccount { program, authority },
                    None => AccountControl::Program(program),
                };
                controls.insert(account.address, control);
            }
        }
        Ok(controls)
    }

    async fn network_view(&self) -> crate::Result<NetworkView> {
        let cached = self
            .network
            .lock()
            .expect("network cache lock")
            .as_ref()
            .filter(|(_, read_at)| read_at.elapsed() < NETWORK_MAX_AGE)
            .map(|(view, _)| view.clone());
        if let Some(view) = cached {
            return Ok(view);
        }
        let view: NetworkView = self
            .call::<(), _>(reqwest::Method::GET, "/v1/chain/network", &[], None)
            .await
            .map_err(|refusal| self.map_service_refusal(refusal))?;
        *self.network.lock().expect("network cache lock") = Some((view.clone(), Instant::now()));
        Ok(view)
    }

    pub async fn network(&self) -> crate::Result<NetworkFacts> {
        Ok(network_facts(&self.network_view().await?))
    }

    pub async fn finalized_tip(&self) -> crate::Result<FinalizedTip> {
        Ok(finalized_tip_from(&self.network().await?))
    }

    /// The last valid height of a blockhash fetched now: at least the bound
    /// of any blockhash fetched before, read live from a node rather than
    /// from the cached network view, which could understate it.
    pub async fn expiry_bound(&self) -> crate::Result<u64> {
        Ok(self.latest_blockhash().await?.1)
    }

    pub async fn epoch(&self) -> crate::Result<u64> {
        Ok(self.network().await?.epoch)
    }

    pub async fn rent_minimum(&self, data_len: usize) -> crate::Result<u64> {
        let rent = self.network_view().await?.rent;
        Ok(rent_minimum_from_rate(rent.lamports_per_byte, data_len))
    }

    pub async fn latest_blockhash(&self) -> crate::Result<(Hash, u64)> {
        let result = self
            .relay(
                "getLatestBlockhash",
                latest_blockhash_params(CommitmentLevel::Confirmed),
            )
            .await?;
        parse_latest_blockhash(&result)
    }

    pub async fn signature_statuses(
        &self,
        signatures: &[Signature],
    ) -> crate::Result<(u64, Vec<Option<TransactionStatus>>)> {
        let result = self
            .relay(
                "getSignatureStatuses",
                signature_statuses_params(signatures),
            )
            .await?;
        parse_signature_statuses(&result)
    }

    pub async fn simulate(
        &self,
        transaction: &VersionedTransaction,
    ) -> crate::Result<SimulationOutcome> {
        let result = self
            .relay(
                RpcMethod::SimulateTransaction.as_str(),
                simulate_transaction_params(transaction)?,
            )
            .await?;
        Ok(parse_simulation(&result))
    }

    pub async fn send(&self, transaction: &VersionedTransaction) -> crate::Result<Signature> {
        let result = self
            .relay(
                RpcMethod::SendTransaction.as_str(),
                send_transaction_params(transaction)?,
            )
            .await?;
        parse_sent_signature(&result)
    }

    pub fn status_poll_interval(&self, elapsed: Duration) -> Duration {
        service_status_poll_interval(elapsed)
    }
}

/// The settle loop's status cadence through the relay: every second while the
/// send is young, every two seconds after.
/// The service state a refusal reveals; `None` for a refusal about one
/// request rather than the service.
fn state_for(refusal: &ServiceRefusal) -> Option<ServiceState> {
    match refusal {
        ServiceRefusal::SignedOut => Some(ServiceState::SignInRequired),
        ServiceRefusal::ReauthorizationRequired | ServiceRefusal::ChainScopeMissing => {
            Some(ServiceState::ReauthorizationRequired)
        }
        ServiceRefusal::VersionUnsupported { minimum } => Some(ServiceState::VersionUnsupported {
            minimum: minimum.clone(),
        }),
        ServiceRefusal::Busy { retry_after } => Some(ServiceState::Busy {
            retry_after_secs: retry_after.as_secs(),
        }),
        ServiceRefusal::DailyLimit { retry_after } => Some(ServiceState::DailyLimit {
            retry_after_secs: retry_after.as_secs(),
        }),
        ServiceRefusal::Disabled | ServiceRefusal::Offline | ServiceRefusal::Unreachable => {
            Some(ServiceState::Unreachable)
        }
        ServiceRefusal::WalletChangeLimited { .. }
        | ServiceRefusal::WalletNotBound
        | ServiceRefusal::NotFound
        | ServiceRefusal::Invalid { .. }
        | ServiceRefusal::Unreadable => None,
    }
}

/// Whether the service would serve this install now, before it adopts service
/// access (setup, a mode switch): the account standing first, then one network
/// read. Records nothing; the install's own state is untouched.
pub async fn probe_service() -> ServiceState {
    let account = chain_access::AccountStanding::current();
    if !account.signed_in {
        return ServiceState::SignInRequired;
    }
    if !account.chain_scope || !account.submit_scope {
        return ServiceState::ReauthorizationRequired;
    }
    match send_json::<(), NetworkView>(
        Surface::Chain,
        ChainId::Solana,
        reqwest::Method::GET,
        "/v1/chain/network",
        &[],
        None,
    )
    .await
    {
        Ok(_) => ServiceState::Ready,
        Err(refusal) => state_for(&refusal).unwrap_or(ServiceState::Unreachable),
    }
}

fn service_status_poll_interval(elapsed: Duration) -> Duration {
    if elapsed < STATUS_POLL_EARLY_WINDOW {
        STATUS_POLL_EARLY
    } else {
        STATUS_POLL_LATE
    }
}

fn signature_info(entry: super::wire::SignatureView) -> crate::Result<SignatureInfo> {
    let signature = Signature::from_str(&entry.signature).map_err(|error| {
        crate::Error::Data(DataError::ParseError {
            data_type: "signature".to_owned(),
            error: format!("invalid signature '{}': {error}", entry.signature),
        })
    })?;
    Ok(SignatureInfo {
        signature,
        slot: entry.slot,
        err: entry
            .err
            .filter(|err| !err.is_null())
            .map(|err| serde_json::to_string(&err).unwrap_or_default()),
        memo: None,
        block_time: entry.block_time,
        confirmation_status: entry.confirmation_status,
    })
}

fn network_facts(view: &NetworkView) -> NetworkFacts {
    NetworkFacts {
        slot: view.slot,
        epoch: view.epoch,
        confirmed_height: view.block_height.confirmed,
        finalized_height: view.block_height.finalized,
    }
}

/// The finalized tip from one network snapshot: its finalized height with the
/// snapshot's confirmed slot, which is never older than the slot that height
/// was read at, so a status must be read at least that late to count.
fn finalized_tip_from(network: &NetworkFacts) -> FinalizedTip {
    FinalizedTip {
        slot: network.slot,
        block_height: network.finalized_height,
    }
}

fn token_program(program: ProgramView) -> TokenProgram {
    match program {
        ProgramView::SplToken => TokenProgram::SplToken,
        ProgramView::SplToken2022 => TokenProgram::SplToken2022,
    }
}

fn mint_lookup(fact: MintFactView) -> MintLookup {
    if !fact.exists {
        return MintLookup::Absent;
    }
    let (Some(program), Some(decimals), Some(supply)) = (fact.program, fact.decimals, fact.supply)
    else {
        return MintLookup::NotAMint {
            detail: "the service reported a mint without its program, decimals or supply"
                .to_owned(),
        };
    };
    let epoch = |setting: super::wire::TransferFeeEpochView| TransferFeeEpoch {
        epoch: setting.epoch,
        basis_points: setting.basis_points,
        maximum_fee: setting.maximum_fee,
    };
    MintLookup::Mint(MintFacts {
        mint: fact.mint,
        program: token_program(program),
        decimals,
        supply,
        mint_authority: fact.mint_authority,
        freeze_authority: fact.freeze_authority,
        transfer_fee: fact.transfer_fee.map(|fee| TransferFeeSchedule {
            older: epoch(fee.older),
            newer: epoch(fee.newer),
        }),
        metadata: fact.metadata.map(|metadata| TokenMetadata {
            name: metadata.name,
            symbol: metadata.symbol,
            uri: metadata.uri,
        }),
    })
}

/// The service's balances as the snapshot both backends return; an NFT is
/// recognised from decimals and amount exactly as the own-RPC decoder does.
fn snapshot_from(view: &BalancesView) -> TokenAccountsSnapshot {
    TokenAccountsSnapshot {
        slot: Some(view.slot),
        accounts: view
            .token_accounts
            .iter()
            .map(|account| TokenAccountInfo {
                account: account.account.clone(),
                mint: account.mint.clone(),
                balance: account.amount,
                decimals: account.decimals,
                is_token_2022: account.program == ProgramView::SplToken2022,
                is_nft: account.decimals == 0 && account.amount == 1,
                is_frozen: account.state == TokenAccountState::Frozen,
                lamports: account.lamports,
            })
            .collect(),
    }
}

/// Whether `account` is the wallet's associated token account of its mint.
fn is_associated(wallet: &str, account: &TokenAccountInfo) -> bool {
    let (Ok(wallet), Ok(mint), Ok(associated_program)) = (
        Pubkey::from_str(wallet),
        Pubkey::from_str(&account.mint),
        Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID),
    ) else {
        return false;
    };
    let program = if account.is_token_2022 {
        TOKEN_2022_PROGRAM_ID
    } else {
        SPL_TOKEN_PROGRAM_ID
    };
    let Ok(program) = Pubkey::from_str(program) else {
        return false;
    };
    let (address, _) = Pubkey::find_program_address(
        &[wallet.as_ref(), program.as_ref(), mint.as_ref()],
        &associated_program,
    );
    address.to_string() == account.account
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_status_cadence_is_one_second_then_two() {
        assert_eq!(
            service_status_poll_interval(Duration::ZERO),
            STATUS_POLL_EARLY
        );
        assert_eq!(
            service_status_poll_interval(Duration::from_millis(9_999)),
            STATUS_POLL_EARLY
        );
        assert_eq!(
            service_status_poll_interval(Duration::from_secs(10)),
            STATUS_POLL_LATE
        );
    }

    /// The tip pairs the finalized height with the snapshot's slot, which a
    /// status read must reach before a null status counts as absence.
    #[test]
    fn the_settlement_tip_comes_from_one_network_snapshot() {
        let network = NetworkFacts {
            slot: 312_345_678,
            epoch: 723,
            confirmed_height: 290_000_100,
            finalized_height: 290_000_068,
        };
        let tip = finalized_tip_from(&network);
        assert_eq!(tip.block_height, 290_000_068);
        assert_eq!(tip.slot, 312_345_678);
        // A transaction whose bound the finalized height has not passed is
        // never settled from this tip.
        assert!(!crate::chains::solana::settlement::holding_settled(
            tip,
            290_000_068,
            Some(tip.slot)
        ));
        assert!(crate::chains::solana::settlement::holding_settled(
            tip,
            290_000_067,
            Some(tip.slot)
        ));
        assert!(!crate::chains::solana::settlement::holding_settled(
            tip,
            290_000_067,
            Some(tip.slot - 1)
        ));
    }

    /// Offline and disabled refusals are decided before any request leaves
    /// the process, so a send refused that way is provably not sent; a
    /// request that went out and got no answer is not an access refusal.
    #[test]
    fn only_a_locally_refused_request_is_an_access_refusal() {
        let backend = ServiceBackend::new();
        for refusal in [ServiceRefusal::Offline, ServiceRefusal::Disabled] {
            let crate::Error::Chains(error) = backend.map_service_refusal(refusal) else {
                panic!("a local refusal is a chain access error");
            };
            assert!(error.is_access_refusal(), "{error}");
        }
        let crate::Error::Chains(error) = backend.map_service_refusal(ServiceRefusal::Unreachable)
        else {
            panic!("an unreachable service is a chain access error");
        };
        assert!(!error.is_access_refusal(), "{error}");
    }

    /// After a swap of this install the cached balances are gone and the next
    /// read asks the service for a fresh one.
    #[test]
    fn a_wallet_change_drops_the_cached_balances_and_asks_for_a_fresh_read() {
        let backend = ServiceBackend::new();
        backend.wallet_changed();
        assert!(backend
            .balances
            .lock()
            .expect("balances cache lock")
            .is_none());
        assert!(backend.wallet.lock().expect("wallet cache lock").is_none());
        assert!(backend.fresh_requested.load(Ordering::Relaxed));
    }

    #[test]
    fn an_absent_or_incomplete_service_mint_is_no_mint() {
        let absent: MintFactView = serde_json::from_value(serde_json::json!({
            "mint": "M", "exists": false, "program": null, "decimals": null, "supply": null,
            "mint_authority": null, "freeze_authority": null, "transfer_fee": null, "metadata": null
        }))
        .unwrap();
        assert_eq!(mint_lookup(absent), MintLookup::Absent);

        let fee_mint: MintFactView = serde_json::from_value(serde_json::json!({
            "mint": "M", "exists": true, "program": "spl_token_2022", "decimals": 6,
            "supply": "1000", "mint_authority": null, "freeze_authority": null,
            "transfer_fee": {"older": {"epoch": 1, "basis_points": 100, "maximum_fee": "20000"},
                             "newer": {"epoch": 1, "basis_points": 100, "maximum_fee": "20000"}},
            "metadata": null
        }))
        .unwrap();
        let MintLookup::Mint(facts) = mint_lookup(fee_mint) else {
            panic!("a complete mint maps");
        };
        assert_eq!(
            facts.transfer_fee.unwrap().fee_at(5, 1_000_000),
            Some(10_000)
        );
    }

    #[test]
    fn service_balances_map_to_the_snapshot_shape() {
        let view: BalancesView = serde_json::from_value(serde_json::json!({
            "address": "W", "version": 3, "slot": 99, "lamports": "5",
            "token_accounts": [
                {"account": "A", "mint": "N", "program": "spl_token", "amount": "1",
                 "decimals": 0, "state": "frozen", "lamports": "2039280"}
            ],
            "observed_at": 1
        }))
        .unwrap();
        let snapshot = snapshot_from(&view);
        assert_eq!(snapshot.slot, Some(99));
        let account = &snapshot.accounts[0];
        assert!(account.is_nft && account.is_frozen && !account.is_token_2022);
        assert_eq!(account.lamports, 2_039_280);
    }
}
