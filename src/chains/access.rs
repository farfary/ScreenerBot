// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! How each chain is reached (own RPC or the ScreenerBot service), what that mode can do, and the live state of the service.
//!
//! ============================================================================
//! ONE OWNER FOR THE MODE
//! ============================================================================
//! The mode is read once from config and frozen for the life of the process:
//! the field is `restart_required`, and the chain facade built at boot holds a
//! backend for exactly one mode. Every feature that needs something only an
//! own RPC provides asks [`has`] or [`require`] for its [`Capability`]; no
//! caller branches on the mode itself.
//!
//! ============================================================================
//! WHY THE SERVICE STATE CARRIES ITS OWN SENTENCE
//! ============================================================================
//! In service mode trading depends on the service, so a refusal pauses trades.
//! The state carries its catalog text (headline and `.detail`, the detail
//! always answering "so what happens instead?"), exactly as
//! `data_server::access::DataAccess` does, so every surface renders one
//! sentence.

use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock, OnceLock};

use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use tokio::sync::{watch, Notify};

use crate::chains::{ChainId, Error, Result};
use crate::i18n::{ids, UiArg, UiText};

/// The scope a device grant needs for service chain reads.
pub const CHAIN_READ_SCOPE: &str = "chain:read";
/// The scope a device grant needs for the submission relay.
pub const SUBMIT_SCOPE: &str = "rpc:submit";

/// How a chain's data is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainAccessMode {
    /// The RPC endpoints the user configured.
    OwnRpc,
    /// The ScreenerBot service: typed reads and the submission relay, with a
    /// signed-in account.
    Service,
}

impl ChainAccessMode {
    /// Parses the config value; `None` for anything but the two exact values.
    pub fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "own_rpc" => Some(Self::OwnRpc),
            "service" => Some(Self::Service),
            _ => None,
        }
    }

    /// The config and wire value of this mode.
    pub const fn as_config_value(self) -> &'static str {
        match self {
            Self::OwnRpc => "own_rpc",
            Self::Service => "service",
        }
    }
}

/// What an install can do in its mode. The table in [`capability_in`] is the
/// single source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// The pool service and the live pool price.
    LivePoolPrices,
    /// The direct pool-swap router.
    DirectPoolSwaps,
    /// Watching wallets other than the trading wallet (copy trading,
    /// watched wallets, high-activity catch-up).
    WatchOtherWallets,
    /// Balances of wallets other than the trading wallet.
    OtherWalletBalances,
    /// Unbounded transaction history backfill.
    FullHistory,
    /// RPC provider statistics and the provider health probe.
    RpcStatistics,
    /// The trading wallet's own activity.
    OwnWalletActivity,
    /// Swaps routed by an aggregator.
    AggregatorSwaps,
}

impl Capability {
    /// Every capability, in declaration order.
    pub const ALL: [Capability; 8] = [
        Capability::LivePoolPrices,
        Capability::DirectPoolSwaps,
        Capability::WatchOtherWallets,
        Capability::OtherWalletBalances,
        Capability::FullHistory,
        Capability::RpcStatistics,
        Capability::OwnWalletActivity,
        Capability::AggregatorSwaps,
    ];

    /// The stable snake-case key used on the wire.
    pub const fn key(self) -> &'static str {
        match self {
            Capability::LivePoolPrices => "live_pool_prices",
            Capability::DirectPoolSwaps => "direct_pool_swaps",
            Capability::WatchOtherWallets => "watch_other_wallets",
            Capability::OtherWalletBalances => "other_wallet_balances",
            Capability::FullHistory => "full_history",
            Capability::RpcStatistics => "rpc_statistics",
            Capability::OwnWalletActivity => "own_wallet_activity",
            Capability::AggregatorSwaps => "aggregator_swaps",
        }
    }
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.key())
    }
}

/// Whether `mode` provides `capability`. Own RPC provides everything; the
/// service serves the trading wallet and aggregator swaps only.
pub const fn capability_in(mode: ChainAccessMode, capability: Capability) -> bool {
    match (mode, capability) {
        (ChainAccessMode::OwnRpc, _) => true,
        (ChainAccessMode::Service, Capability::OwnWalletActivity | Capability::AggregatorSwaps) => {
            true
        }
        (
            ChainAccessMode::Service,
            Capability::LivePoolPrices
            | Capability::DirectPoolSwaps
            | Capability::WatchOtherWallets
            | Capability::OtherWalletBalances
            | Capability::FullHistory
            | Capability::RpcStatistics,
        ) => false,
    }
}

static MODES: [OnceLock<ChainAccessMode>; ChainId::COUNT] =
    [const { OnceLock::new() }; ChainId::COUNT];

/// The chain's access mode, read from config on first use and frozen for the
/// process. A value config validation would refuse reads as own RPC, the
/// behaviour every install had before the setting existed. Before config is
/// loaded (library tests never load it) the mode is the schema default, own
/// RPC, and nothing is frozen.
pub fn mode(chain: ChainId) -> ChainAccessMode {
    if !crate::config::is_config_initialized() {
        return ChainAccessMode::OwnRpc;
    }
    *MODES[chain.index()].get_or_init(|| {
        let value = crate::config::with_config(|cfg| cfg.chains.access(chain).to_owned());
        ChainAccessMode::from_config_value(&value).unwrap_or(ChainAccessMode::OwnRpc)
    })
}

/// Whether the chain's mode provides `capability`.
pub fn has(chain: ChainId, capability: Capability) -> bool {
    capability_in(mode(chain), capability)
}

/// `Err(CapabilityUnavailable)` when the chain's mode lacks `capability`.
pub fn require(chain: ChainId, capability: Capability) -> Result<()> {
    if has(chain, capability) {
        Ok(())
    } else {
        Err(Error::CapabilityUnavailable { chain, capability })
    }
}

/// Live state of the service in service mode; always `Ready` in own-RPC mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceState {
    /// Not confirmed yet this run.
    Checking,
    /// Answering.
    Ready,
    /// No signed-in account.
    SignInRequired,
    /// This device's grant lacks `chain:read` or `rpc:submit`.
    ReauthorizationRequired,
    /// The service asked to retry later.
    Busy { retry_after_secs: u64 },
    /// The account reached today's allowance.
    DailyLimit { retry_after_secs: u64 },
    /// No answer (offline or the service is down).
    Unreachable,
    /// This build is older than the service answers.
    VersionUnsupported { minimum: String },
}

impl ServiceState {
    /// The stable wire key of the state.
    pub fn key(&self) -> &'static str {
        match self {
            ServiceState::Checking => "checking",
            ServiceState::Ready => "ready",
            ServiceState::SignInRequired => "sign_in_required",
            ServiceState::ReauthorizationRequired => "reauthorization_required",
            ServiceState::Busy { .. } => "busy",
            ServiceState::DailyLimit { .. } => "daily_limit",
            ServiceState::Unreachable => "unreachable",
            ServiceState::VersionUnsupported { .. } => "version_unsupported",
        }
    }

    /// True only while trades can go through.
    pub fn ready(&self) -> bool {
        matches!(self, ServiceState::Ready)
    }

    /// Catalog text: the message value is the headline, its `.detail` says
    /// what happens instead.
    pub fn ui_text(&self) -> UiText {
        match self {
            ServiceState::Checking => UiText::new(ids::CONNECTIVITY_SERVICE_CHECKING),
            ServiceState::Ready => UiText::new(ids::CONNECTIVITY_SERVICE_READY),
            ServiceState::SignInRequired => UiText::new(ids::CONNECTIVITY_SERVICE_SIGNED_OUT),
            ServiceState::ReauthorizationRequired => {
                UiText::new(ids::CONNECTIVITY_SERVICE_REAUTHORIZE)
            }
            ServiceState::Busy { .. } => UiText::new(ids::CONNECTIVITY_SERVICE_BUSY),
            ServiceState::DailyLimit { .. } => UiText::new(ids::CONNECTIVITY_SERVICE_DAILY_LIMIT),
            ServiceState::Unreachable => UiText::new(ids::CONNECTIVITY_SERVICE_UNREACHABLE),
            ServiceState::VersionUnsupported { minimum } => {
                UiText::new(ids::CONNECTIVITY_SERVICE_VERSION)
                    .arg("minimum", UiArg::Text(minimum.clone()))
            }
        }
    }

    /// True when the person can fix this themselves right now.
    pub fn actionable(&self) -> bool {
        matches!(
            self,
            ServiceState::SignInRequired
                | ServiceState::ReauthorizationRequired
                | ServiceState::VersionUnsupported { .. }
        )
    }

    /// The domain error a request refused in this state reports, `None` while
    /// the service is usable.
    pub fn refusal(&self, chain: ChainId) -> Option<Error> {
        match self {
            ServiceState::Checking | ServiceState::Ready => None,
            ServiceState::SignInRequired => Some(Error::ServiceSignInRequired { chain }),
            ServiceState::ReauthorizationRequired => Some(Error::ServiceScopeMissing {
                chain,
                scope: CHAIN_READ_SCOPE,
            }),
            ServiceState::Busy { retry_after_secs } => Some(Error::ServiceBusy {
                chain,
                retry_after: std::time::Duration::from_secs(*retry_after_secs),
            }),
            ServiceState::DailyLimit { retry_after_secs } => Some(Error::ServiceDailyLimit {
                chain,
                retry_after: std::time::Duration::from_secs(*retry_after_secs),
            }),
            ServiceState::Unreachable => Some(Error::ServiceUnreachable {
                chain,
                detail: "no answer".to_owned(),
            }),
            ServiceState::VersionUnsupported { minimum } => {
                Some(Error::ServiceVersionUnsupported {
                    chain,
                    minimum: minimum.clone(),
                })
            }
        }
    }
}

struct StateCell {
    current: ArcSwap<ServiceState>,
    sender: watch::Sender<ServiceState>,
}

static STATES: LazyLock<[StateCell; ChainId::COUNT]> = LazyLock::new(|| {
    std::array::from_fn(|_| {
        let (sender, _) = watch::channel(ServiceState::Checking);
        StateCell {
            current: ArcSwap::from_pointee(ServiceState::Checking),
            sender,
        }
    })
});

static REPROBE: LazyLock<Notify> = LazyLock::new(Notify::new);

/// The chain's live service state; `Ready` in own-RPC mode.
pub fn service_state(chain: ChainId) -> ServiceState {
    match mode(chain) {
        ChainAccessMode::OwnRpc => ServiceState::Ready,
        ChainAccessMode::Service => (**STATES[chain.index()].current.load()).clone(),
    }
}

/// Records what the service last answered. Written by the service backend
/// only; a repeated state notifies nobody.
pub fn record_service_outcome(chain: ChainId, state: ServiceState) {
    let cell = &STATES[chain.index()];
    if **cell.current.load() == state {
        return;
    }
    cell.current.store(Arc::new(state.clone()));
    cell.sender.send_replace(state);
}

/// Every change of the chain's service state, for the trader and
/// notifications.
pub fn subscribe_service_state(chain: ChainId) -> watch::Receiver<ServiceState> {
    STATES[chain.index()].sender.subscribe()
}

/// The process chain's access mode, for code with no subject chain.
pub fn active_mode() -> ChainAccessMode {
    mode(super::active_chain())
}

/// Whether the process chain's mode provides `capability`, for code with no
/// subject chain: service gates, readiness and process-wide surfaces.
pub fn active_has(capability: Capability) -> bool {
    has(super::active_chain(), capability)
}

/// The process chain's live service state.
pub fn active_service_state() -> ServiceState {
    service_state(super::active_chain())
}

/// Every change of the process chain's service state.
pub fn subscribe_active_service_state() -> watch::Receiver<ServiceState> {
    subscribe_service_state(super::active_chain())
}

/// The process chain's access status as the UI renders it.
pub fn active_status() -> ChainAccessStatus {
    status(super::active_chain())
}

/// The trading wallet changed through a swap or activity of this install:
/// every enabled chain's access drops the balances it cached.
pub fn wallet_changed() {
    for &chain in super::registry::enabled_chains() {
        match chain {
            ChainId::Solana => super::solana::access::wallet_changed(),
        }
    }
}

/// Probes the access of every enabled chain once at boot, so a service state
/// leaves `Checking` before the first trading cycle.
pub fn start_boot_probes() {
    for &chain in super::registry::enabled_chains() {
        match chain {
            ChainId::Solana => super::solana::access::start_boot_probe(),
        }
    }
}

/// What switching the process chain to a requested mode takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeSwitch {
    /// The requested mode is the current one.
    Unchanged,
    /// Own-RPC access: the given endpoints are tested and saved with the mode.
    TestEndpoints,
    /// Service access: the service is probed before the mode is saved.
    ProbeService,
}

/// The switch of the process chain to `requested`.
pub fn plan_active_switch(requested: ChainAccessMode) -> ModeSwitch {
    if requested == active_mode() {
        return ModeSwitch::Unchanged;
    }
    match requested {
        ChainAccessMode::OwnRpc => ModeSwitch::TestEndpoints,
        ChainAccessMode::Service => ModeSwitch::ProbeService,
    }
}

/// Probes the ScreenerBot service for the process chain; `Err` carries the
/// refusal of a state that does not serve.
pub async fn probe_active_service() -> Result<()> {
    let chain = super::active_chain();
    match probe_service(chain).await.refusal(chain) {
        Some(refusal) => Err(refusal),
        None => Ok(()),
    }
}

/// Tests `urls` as own-RPC endpoints of the process chain against the
/// configured trading wallet: one success flag per url, in order.
pub async fn test_active_endpoints(urls: &[String]) -> Vec<bool> {
    match super::active_chain() {
        ChainId::Solana => {
            let wallet = super::solana::accounts::configured_address_async()
                .await
                .ok();
            super::solana::rpc::test_rpc_endpoints(urls, wallet.as_deref())
                .await
                .iter()
                .map(|result| result.success)
                .collect()
        }
    }
}

/// Saves `mode` for the process chain, with the own-RPC endpoints when given,
/// and returns the chain. Blocking: writes the config file.
pub fn save_active_mode(
    mode: ChainAccessMode,
    rpc_urls: Option<Vec<String>>,
) -> crate::config::Result<ChainId> {
    let chain = super::active_chain();
    crate::config::update_config_section(
        move |config| {
            *config.chains.access_mut(chain) = mode.as_config_value().to_owned();
            if let Some(urls) = rpc_urls {
                config.chains.rpc_mut(chain).urls = urls;
            }
        },
        true,
    )?;
    Ok(chain)
}

/// Probes the ScreenerBot service for `chain` and returns the state it answered.
pub async fn probe_service(chain: ChainId) -> ServiceState {
    match chain {
        ChainId::Solana => super::solana::access::probe_service().await,
    }
}

/// Called by the account owner on every sign-in, sign-out and token rotation:
/// the service backend re-probes at once instead of at its next poll, so a
/// person who signs in again is not left waiting on a stale refusal.
pub fn session_changed() {
    REPROBE.notify_waiters();
}

/// Resolves when [`session_changed`] fires. Awaited by the service backend's
/// poller beside its interval.
pub(crate) async fn reprobe_requested() {
    REPROBE.notified().await;
}

/// The signed-in account's standing for service access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AccountStanding {
    pub signed_in: bool,
    pub chain_scope: bool,
    pub submit_scope: bool,
}

impl AccountStanding {
    /// The account owner's current view.
    pub fn current() -> Self {
        let signed_in = crate::account::is_signed_in();
        Self {
            signed_in,
            chain_scope: signed_in && crate::account::has_scope(CHAIN_READ_SCOPE),
            submit_scope: signed_in && crate::account::has_scope(SUBMIT_SCOPE),
        }
    }
}

/// What every surface renders about a chain's access. Never a token, never an
/// endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct ChainAccessStatus {
    pub chain: ChainId,
    pub mode: ChainAccessMode,
    pub state: &'static str,
    pub ready: bool,
    pub actionable: bool,
    /// Headline and `.detail` attribute, formatted by the presentation layer.
    pub text: UiText,
    /// Present only for `busy` and `daily_limit`.
    pub retry_after_secs: Option<u64>,
    /// Present only for `version_unsupported`, so the UI can name the release.
    pub minimum_version: Option<String>,
    pub capabilities: BTreeMap<&'static str, bool>,
    pub account: AccountStanding,
}

/// The chain's access status as the UI renders it.
pub fn status(chain: ChainId) -> ChainAccessStatus {
    describe(
        chain,
        mode(chain),
        service_state(chain),
        AccountStanding::current(),
    )
}

/// Builds the status from its parts; pure, so the shape is testable.
pub fn describe(
    chain: ChainId,
    mode: ChainAccessMode,
    state: ServiceState,
    account: AccountStanding,
) -> ChainAccessStatus {
    let retry_after_secs = match &state {
        ServiceState::Busy { retry_after_secs } | ServiceState::DailyLimit { retry_after_secs } => {
            Some(*retry_after_secs)
        }
        _ => None,
    };
    let minimum_version = match &state {
        ServiceState::VersionUnsupported { minimum } => Some(minimum.clone()),
        _ => None,
    };
    ChainAccessStatus {
        chain,
        mode,
        state: state.key(),
        ready: state.ready(),
        actionable: state.actionable(),
        text: state.ui_text(),
        retry_after_secs,
        minimum_version,
        capabilities: Capability::ALL
            .iter()
            .map(|capability| (capability.key(), capability_in(mode, *capability)))
            .collect(),
        account,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mode_parses_only_its_two_config_values_and_round_trips() {
        for mode in [ChainAccessMode::OwnRpc, ChainAccessMode::Service] {
            assert_eq!(
                ChainAccessMode::from_config_value(mode.as_config_value()),
                Some(mode)
            );
        }
        for value in ["", "OWN_RPC", "own-rpc", "rpc", "services"] {
            assert_eq!(ChainAccessMode::from_config_value(value), None, "{value:?}");
        }
    }

    /// Own RPC provides every capability; the service provides exactly the
    /// trading wallet's activity and aggregator swaps.
    #[test]
    fn the_capability_table_is_exhaustive_over_both_modes() {
        let service_only = [Capability::OwnWalletActivity, Capability::AggregatorSwaps];
        assert_eq!(Capability::ALL.len(), 8);
        for capability in Capability::ALL {
            assert!(
                capability_in(ChainAccessMode::OwnRpc, capability),
                "{capability}"
            );
            assert_eq!(
                capability_in(ChainAccessMode::Service, capability),
                service_only.contains(&capability),
                "{capability}"
            );
        }
        let keys: std::collections::BTreeSet<_> = Capability::ALL.iter().map(|c| c.key()).collect();
        assert_eq!(
            keys.len(),
            Capability::ALL.len(),
            "capability keys are unique"
        );
    }

    #[test]
    fn every_service_state_has_a_distinct_key_and_a_catalog_text() {
        let states = [
            ServiceState::Checking,
            ServiceState::Ready,
            ServiceState::SignInRequired,
            ServiceState::ReauthorizationRequired,
            ServiceState::Busy {
                retry_after_secs: 30,
            },
            ServiceState::DailyLimit {
                retry_after_secs: 3600,
            },
            ServiceState::Unreachable,
            ServiceState::VersionUnsupported {
                minimum: "0.2.22".to_owned(),
            },
        ];
        let keys: std::collections::BTreeSet<_> = states.iter().map(|s| s.key()).collect();
        assert_eq!(keys.len(), states.len());
        for state in &states {
            let text = state.ui_text().render_source_plain();
            assert!(!text.is_empty(), "{}", state.key());
            assert_eq!(
                state.refusal(ChainId::Solana).is_none(),
                matches!(state, ServiceState::Checking | ServiceState::Ready),
                "{}",
                state.key()
            );
        }
    }

    #[test]
    fn the_status_serializes_to_the_documented_shape() {
        let status = describe(
            ChainId::Solana,
            ChainAccessMode::Service,
            ServiceState::DailyLimit {
                retry_after_secs: 600,
            },
            AccountStanding {
                signed_in: true,
                chain_scope: true,
                submit_scope: false,
            },
        );
        let json = serde_json::to_value(&status).expect("status serializes");
        assert_eq!(json["chain"], "solana");
        assert_eq!(json["mode"], "service");
        assert_eq!(json["state"], "daily_limit");
        assert_eq!(json["ready"], false);
        assert_eq!(json["actionable"], false);
        assert_eq!(json["retry_after_secs"], 600);
        assert_eq!(json["minimum_version"], serde_json::Value::Null);
        assert_eq!(json["capabilities"]["live_pool_prices"], false);
        assert_eq!(json["capabilities"]["aggregator_swaps"], true);
        assert_eq!(json["capabilities"].as_object().unwrap().len(), 8);
        assert_eq!(json["account"]["signed_in"], true);
        assert_eq!(json["account"]["submit_scope"], false);
        assert!(json["text"].is_object());
    }
}
