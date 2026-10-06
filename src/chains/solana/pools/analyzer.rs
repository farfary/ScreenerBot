// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pool analyzer module.
//!
//! Analyzes discovered pools to classify pool types by program ID, extract pool
//! metadata (base/quote tokens, reserve accounts), validate pool structure and
//! data, and prepare account lists for fetching.

use super::selection::{self, SelectedPools};
use super::types::ProgramKind;

use super::decode_utils::is_sol_mint;
use crate::chains::solana::pools::service;
use crate::chains::solana::rpc::{get_rpc_client, RpcClient, RpcClientMethods};
use crate::chains::{AccountId, AssetId, ChainId, PoolId};
use crate::events::{record_safe, Event, EventCategory};
use crate::logger::{self, LogTag};
use crate::pools::types::{pool_blacklist_threshold, PoolDescriptor};
use crate::rpc::RpcError;
use crate::utils::run_or_shutdown;

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Notify};

/// First wait before re-analyzing a pool whose analysis failed on RPC transport.
const TRANSIENT_RETRY_INITIAL: Duration = Duration::from_secs(60);
/// Longest wait between re-analysis attempts after repeated transport failures.
const TRANSIENT_RETRY_MAX: Duration = Duration::from_secs(600);

/// Why a pool could not be analyzed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolAnalysisFailure {
    /// The RPC transport failed (timeout, connection, rate limit, open circuit,
    /// provider error). It says nothing about the pool, so it never blacklists.
    Transient,
    /// The pool itself cannot be priced: its account does not exist, its program
    /// is unsupported, or its account layout does not decode.
    Structural,
}

/// Classify an error returned by `RpcClientMethods::get_account`.
///
/// Only the RPC layer's own `AccountNotFound` describes the pool; every other
/// failure of the call is transport or provider state and is transient.
pub(crate) fn classify_account_fetch_error(error: &crate::Error) -> PoolAnalysisFailure {
    match error {
        crate::Error::Rpc(RpcError::AccountNotFound { .. }) => PoolAnalysisFailure::Structural,
        _ => PoolAnalysisFailure::Transient,
    }
}

/// Re-analysis backoff of a pool whose last analysis failed transiently.
#[derive(Debug, Clone, Copy)]
struct TransientBackoff {
    retry_at: Instant,
    delay: Duration,
}

/// One pool the analysis loop was asked to analyze.
struct AnalysisRequest {
    pool_id: Pubkey,
    program_id: Pubkey,
    base_mint: Pubkey,
    quote_mint: Pubkey,
    liquidity_usd: f64,
    volume_h24_usd: f64,
}

/// Wait before the next re-analysis attempt: starts at `TRANSIENT_RETRY_INITIAL`
/// and doubles per consecutive transient failure up to `TRANSIENT_RETRY_MAX`.
fn next_transient_delay(previous: Option<Duration>) -> Duration {
    match previous {
        Some(delay) => delay.saturating_mul(2).min(TRANSIENT_RETRY_MAX),
        None => TRANSIENT_RETRY_INITIAL,
    }
}

/// Whether an `AnalyzePool` request needs RPC analysis: a pool already in the
/// directory with its reserve accounts is analyzed and is not fetched again.
fn needs_analysis(directory: &HashMap<Pubkey, PoolDescriptor>, pool_id: &Pubkey) -> bool {
    directory
        .get(pool_id)
        .map_or(true, |descriptor| descriptor.reserve_accounts.is_empty())
}

/// Message types for analyzer communication
#[derive(Debug, Clone)]
pub enum AnalyzerMessage {
    /// Request to analyze a discovered pool
    AnalyzePool {
        pool_id: Pubkey,
        program_id: Pubkey,
        base_mint: Pubkey,
        quote_mint: Pubkey,
        liquidity_usd: f64,
        volume_h24_usd: f64,
    },
    /// Signal shutdown
    Shutdown,
}

/// Pool analyzer service
pub struct PoolAnalyzer {
    /// Analyzed pool directory
    pool_directory: Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
    /// Pool each token is priced from; written only by this analyzer
    selected_pools: SelectedPools,
    /// Channel for receiving analysis requests
    analyzer_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<AnalyzerMessage>>>>,
    /// Channel sender for sending analysis requests
    analyzer_tx: mpsc::UnboundedSender<AnalyzerMessage>,
    /// Metrics
    operations: Arc<std::sync::atomic::AtomicU64>,
    errors: Arc<std::sync::atomic::AtomicU64>,
    pools_analyzed: Arc<std::sync::atomic::AtomicU64>,
}

impl PoolAnalyzer {
    /// Create new pool analyzer
    pub fn new(
        pool_directory: Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
        selected_pools: SelectedPools,
    ) -> Self {
        let (analyzer_tx, analyzer_rx) = mpsc::unbounded_channel();

        Self {
            pool_directory,
            selected_pools,
            analyzer_rx: Arc::new(RwLock::new(Some(analyzer_rx))),
            analyzer_tx,
            operations: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            errors: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            pools_analyzed: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Get metrics for this analyzer instance
    pub fn get_metrics(&self) -> (u64, u64, u64) {
        (
            self.operations.load(std::sync::atomic::Ordering::Relaxed),
            self.errors.load(std::sync::atomic::Ordering::Relaxed),
            self.pools_analyzed
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Get sender for sending analysis requests
    pub fn get_sender(&self) -> mpsc::UnboundedSender<AnalyzerMessage> {
        self.analyzer_tx.clone()
    }

    /// Get pool directory (read-only access)
    pub fn get_pool_directory(&self) -> Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>> {
        self.pool_directory.clone()
    }

    /// Take the request receiver the analysis loop consumes; `None` once a
    /// loop has taken it.
    pub(super) fn take_receiver(&self) -> Option<mpsc::UnboundedReceiver<AnalyzerMessage>> {
        self.analyzer_rx.write().ok()?.take()
    }

    /// Run the analysis loop until shutdown, a shutdown message or a closed
    /// channel. An analysis in flight is abandoned on shutdown.
    pub(super) async fn run_analyzer_loop(
        self: Arc<Self>,
        mut analyzer_rx: mpsc::UnboundedReceiver<AnalyzerMessage>,
        shutdown: Arc<Notify>,
    ) {
        logger::info(LogTag::PoolAnalyzer, "Starting pool analyzer task");
        logger::info(LogTag::PoolAnalyzer, "Pool analyzer task started");

        let rpc_client = get_rpc_client();
        let mut transient_backoff: HashMap<Pubkey, TransientBackoff> = HashMap::new();

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    logger::info(LogTag::PoolAnalyzer, "Pool analyzer task shutting down");
                    break;
                }

                message = analyzer_rx.recv() => {
                    match message {
                        Some(AnalyzerMessage::AnalyzePool {
                            pool_id,
                            program_id,
                            base_mint,
                            quote_mint,
                            liquidity_usd,
                            volume_h24_usd
                        }) => {
                            let request = AnalysisRequest {
                                pool_id,
                                program_id,
                                base_mint,
                                quote_mint,
                                liquidity_usd,
                                volume_h24_usd,
                            };
                            let analysis = self.analyze_request(request, rpc_client, &mut transient_backoff);
                            if run_or_shutdown(&shutdown, analysis).await.is_none() {
                                logger::info(LogTag::PoolAnalyzer, "Pool analyzer task shutting down");
                                break;
                            }
                        }

                        Some(AnalyzerMessage::Shutdown) => {
                            logger::info(LogTag::PoolAnalyzer, "Pool analyzer received shutdown signal");
                            break;
                        }

                        None => {
                            logger::info(LogTag::PoolAnalyzer, "Pool analyzer channel closed");
                            break;
                        }
                    }
                }
            }
        }

        logger::info(LogTag::PoolAnalyzer, "Pool analyzer task completed");
    }

    /// Handle one analysis request: skip a blacklisted pool, confirm the
    /// selection of an analyzed one, otherwise analyze it, record the outcome
    /// and request its reserve accounts.
    async fn analyze_request(
        &self,
        request: AnalysisRequest,
        rpc_client: &RpcClient,
        transient_backoff: &mut HashMap<Pubkey, TransientBackoff>,
    ) {
        let AnalysisRequest {
            pool_id,
            program_id,
            base_mint,
            quote_mint,
            liquidity_usd,
            volume_h24_usd,
        } = request;
        let pool_directory = &self.pool_directory;
        let selected_pools = &self.selected_pools;
        let operations = &self.operations;
        let errors = &self.errors;
        let pools_analyzed = &self.pools_analyzed;

        // Check if pool is blacklisted in database
        if let Ok(is_blacklisted) = crate::pools::db::is_pool_blacklisted(
            crate::chains::ChainId::Solana,
            &pool_id.to_string(),
        )
        .await
        {
            if is_blacklisted {
                logger::debug(
                    LogTag::PoolAnalyzer,
                    &format!("Skipping blacklisted pool: {pool_id}"),
                );
                return;
            }
        }

        // The non-SOL side: the token this pool prices
        let token_mint = if is_sol_mint(&base_mint.to_string()) {
            quote_mint
        } else {
            base_mint
        };
        let token_mint_str = token_mint.to_string();

        // Discovery re-sends its choice every tick. An analyzed pool
        // only needs its selection confirmed, never another RPC fetch.
        let analyzed = !needs_analysis(&pool_directory.read().unwrap(), &pool_id);
        if analyzed {
            Self::select_pool_for_token(selected_pools, pool_directory, &token_mint_str, pool_id)
                .await;
            return;
        }

        let now = Instant::now();
        if transient_backoff
            .get(&pool_id)
            .is_some_and(|backoff| now < backoff.retry_at)
        {
            return;
        }

        match Self::analyze_pool_static(
            pool_id,
            program_id,
            base_mint,
            quote_mint,
            liquidity_usd,
            volume_h24_usd,
            rpc_client,
        )
        .await
        {
            Ok(descriptor) => {
                transient_backoff.remove(&pool_id);

                // Track metrics
                operations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                pools_analyzed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                // Store analyzed pool in directory, then make it the
                // token's pricing pool (evicting any superseded pool)
                pool_directory
                    .write()
                    .unwrap()
                    .insert(pool_id, descriptor.clone());
                Self::select_pool_for_token(
                    selected_pools,
                    pool_directory,
                    &token_mint_str,
                    pool_id,
                )
                .await;

                // Trigger account fetch for this pool's reserve accounts
                if let Some(fetcher) = service::get_account_fetcher() {
                    let reserve_accounts: Vec<Pubkey> = descriptor
                        .reserve_accounts
                        .iter()
                        .filter_map(|account| Pubkey::from_str(account.address()).ok())
                        .collect();
                    if let Err(e) = fetcher.request_pool_fetch(pool_id, reserve_accounts) {
                        // The analyzer and the fetcher wake on the SAME
                        // shutdown broadcast, so the fetcher can drop its
                        // receiver while the analyzer is still finishing
                        // the batch in hand. A closed channel is then the
                        // expected outcome, not a fault: the work is
                        // deliberately being abandoned because the process
                        // is exiting. Logging it at warning buried the
                        // real shutdown sequence under a dozen identical
                        // lines every run.
                        if crate::process::shutdown::is_shutdown_requested() {
                            logger::debug(LogTag::PoolAnalyzer, &format!("Dropping fetch request for pool {pool_id} during shutdown: {e}"));
                        } else {
                            logger::warning(
                                LogTag::PoolAnalyzer,
                                &format!(
                                    "Failed to request fetch for analyzed pool {pool_id}: {e}"
                                ),
                            );
                        }
                    }
                }

                logger::debug(
                    LogTag::PoolAnalyzer,
                    &format!(
                        "Analyzed pool {} for token {} ({}) - {}/{}",
                        pool_id,
                        token_mint_str,
                        descriptor.program_kind.as_str(),
                        base_mint,
                        quote_mint
                    ),
                );
            }
            Err(PoolAnalysisFailure::Transient) => {
                errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                // Forget backoffs that ran out long ago so the map stays
                // bounded by the pools that are currently failing.
                transient_backoff.retain(|_, backoff| {
                    now.saturating_duration_since(backoff.retry_at) < TRANSIENT_RETRY_MAX
                });
                let delay = next_transient_delay(
                    transient_backoff.get(&pool_id).map(|backoff| backoff.delay),
                );
                transient_backoff.insert(
                    pool_id,
                    TransientBackoff {
                        retry_at: now + delay,
                        delay,
                    },
                );

                logger::debug(
                    LogTag::PoolAnalyzer,
                    &format!(
                        "Analysis of pool {pool_id} for token {token_mint_str} hit a transient RPC failure; retrying in {}s",
                        delay.as_secs()
                    ),
                );
            }
            Err(PoolAnalysisFailure::Structural) => {
                errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                transient_backoff.remove(&pool_id);

                match crate::pools::db::add_pool_to_blacklist(
                    crate::chains::ChainId::Solana,
                    &pool_id.to_string(),
                    "analysis_failed",
                    Some(&token_mint_str),
                    Some(&program_id.to_string()),
                    1,
                ).await {
                    Ok(outcome) => match outcome.blacklisted_until {
                        Some(until) => logger::warning(
                            LogTag::PoolAnalyzer,
                            &format!(
                                "Failed to analyze pool {pool_id} for token {token_mint_str}; blacklisted until unix {until} after {} failures",
                                outcome.error_count
                            ),
                        ),
                        None => logger::warning(
                            LogTag::PoolAnalyzer,
                            &format!(
                                "Failed to analyze pool {pool_id} for token {token_mint_str} (failure {} of {})",
                                outcome.error_count,
                                pool_blacklist_threshold()
                            ),
                        ),
                    },
                    Err(e) => logger::warning(
                        LogTag::PoolAnalyzer,
                        &format!("Failed to record analysis failure of pool {pool_id}: {e}"),
                    ),
                }
            }
        }
    }

    /// Make `pool_id` the pool `mint` is priced from. A superseded pool leaves
    /// the directory (ending its account fetching) and its fetched bundle is
    /// released.
    async fn select_pool_for_token(
        selected_pools: &SelectedPools,
        pool_directory: &Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
        mint: &str,
        pool_id: Pubkey,
    ) {
        let Some(evicted) = selection::select_pool(selected_pools, pool_directory, mint, pool_id)
        else {
            return;
        };

        if let Some(fetcher) = service::get_account_fetcher() {
            fetcher.remove_pool_bundle(&evicted);
        }

        logger::info(
            LogTag::PoolAnalyzer,
            &format!("Pricing pool for token {mint} changed from {evicted} to {pool_id}"),
        );
        record_safe(Event::info(
            EventCategory::Pool,
            Some("pool_selection_changed".to_owned()),
            Some(mint.to_owned()),
            Some(pool_id.to_string()),
            serde_json::json!({
                "token_mint": mint,
                "pool_id": pool_id.to_string(),
                "previous_pool_id": evicted.to_string(),
            }),
        ))
        .await;
    }

    /// Analyze a pool and extract metadata (static version for task)
    async fn analyze_pool_static(
        pool_id: Pubkey,
        program_id: Pubkey,
        base_mint: Pubkey,
        quote_mint: Pubkey,
        liquidity_usd: f64,
        volume_h24_usd: f64,
        rpc_client: &RpcClient,
    ) -> Result<PoolDescriptor, PoolAnalysisFailure> {
        // First, try to determine the actual program type by fetching the pool account
        let actual_program_id = if program_id == Pubkey::default() {
            // This is an Unknown pool from discovery - fetch the account to get the real program ID
            match rpc_client.get_account(&pool_id).await {
                Ok(Some(account)) => {
                    logger::debug(
                        LogTag::PoolAnalyzer,
                        &format!("Pool {} owner: {}", pool_id, account.owner),
                    );
                    account.owner
                }
                Ok(None) => {
                    let target_mint = if is_sol_mint(&base_mint.to_string()) {
                        quote_mint.to_string()
                    } else {
                        base_mint.to_string()
                    };

                    record_safe(Event::error(
                        EventCategory::Pool,
                        Some("pool_account_fetch_failed".to_owned()),
                        Some(target_mint.clone()),
                        Some(pool_id.to_string()),
                        serde_json::json!({
                            "pool_id": pool_id.to_string(),
                            "target_mint": target_mint,
                            "error": "Account not found",
                            "action": "get_account"
                        }),
                    ))
                    .await;

                    logger::warning(
                        LogTag::PoolAnalyzer,
                        &format!("Pool account {pool_id} not found for token analysis"),
                    );
                    return Err(PoolAnalysisFailure::Structural);
                }
                Err(e) => {
                    let failure = classify_account_fetch_error(&e);
                    let target_mint = if is_sol_mint(&base_mint.to_string()) {
                        quote_mint.to_string()
                    } else {
                        base_mint.to_string()
                    };

                    record_safe(Event::error(
                        EventCategory::Pool,
                        Some("pool_account_fetch_failed".to_owned()),
                        Some(target_mint.clone()),
                        Some(pool_id.to_string()),
                        serde_json::json!({
                            "pool_id": pool_id.to_string(),
                            "target_mint": target_mint,
                            "error": e.to_string(),
                            "action": "get_account"
                        }),
                    ))
                    .await;

                    let message = format!(
                        "Failed to fetch pool account {} for token analysis: {}",
                        pool_id, e
                    );
                    match failure {
                        PoolAnalysisFailure::Transient => {
                            logger::debug(LogTag::PoolAnalyzer, &message)
                        }
                        PoolAnalysisFailure::Structural => {
                            logger::warning(LogTag::PoolAnalyzer, &message)
                        }
                    }
                    return Err(failure);
                }
            }
        } else {
            program_id
        };

        // Classify the program type using the actual program ID
        let program_kind = Self::classify_program_static(&actual_program_id);

        if program_kind == ProgramKind::Unknown {
            let target_mint = if is_sol_mint(&base_mint.to_string()) {
                quote_mint.to_string()
            } else {
                base_mint.to_string()
            };

            record_safe(Event::warn(
                EventCategory::Pool,
                Some("unsupported_program".to_owned()),
                Some(target_mint.clone()),
                Some(pool_id.to_string()),
                serde_json::json!({
                    "pool_id": pool_id.to_string(),
                    "program_id": actual_program_id.to_string(),
                    "base_mint": base_mint.to_string(),
                    "quote_mint": quote_mint.to_string(),
                    "target_mint": target_mint,
                    "error": "Unsupported DEX program - consider adding support"
                }),
            ))
            .await;

            logger::warning(LogTag::PoolAnalyzer, &format!("Unsupported DEX program for pool {pool_id}: {actual_program_id} (consider adding support for this DEX)"));
            return Err(PoolAnalysisFailure::Structural);
        }

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "Classified pool {} as {}",
                pool_id,
                program_kind.display_name()
            ),
        );

        // Extract reserve accounts based on program type
        let reserve_accounts = Self::extract_reserve_accounts(
            &pool_id,
            &program_kind,
            &base_mint,
            &quote_mint,
            rpc_client,
        )
        .await?;

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "Successfully analyzed {} pool {} with {} reserve accounts for token {}",
                program_kind.display_name(),
                pool_id,
                reserve_accounts.len(),
                if is_sol_mint(&base_mint.to_string()) {
                    quote_mint
                } else {
                    base_mint
                }
            ),
        );

        let target_mint = if is_sol_mint(&base_mint.to_string()) {
            quote_mint.to_string()
        } else {
            base_mint.to_string()
        };

        record_safe(Event::info(
            EventCategory::Pool,
            Some(
                format!("{}_analyzed", program_kind.display_name().to_lowercase())
                    .replace(" ", "_"),
            ),
            Some(target_mint.clone()),
            Some(pool_id.to_string()),
            serde_json::json!({
                "pool_id": pool_id.to_string(),
                "program_kind": program_kind.display_name(),
                "program_id": actual_program_id.to_string(),
                "target_mint": target_mint,
                "base_mint": base_mint.to_string(),
                "quote_mint": quote_mint.to_string(),
                "reserve_accounts_count": reserve_accounts.len(),
                "liquidity_usd": liquidity_usd,
                "volume_h24_usd": volume_h24_usd
            }),
        ))
        .await;

        Ok(PoolDescriptor {
            pool_id: PoolId::new(ChainId::Solana, pool_id.to_string())
                .expect("Solana pubkey string is never empty"),
            program_kind: program_kind.protocol_id(),
            base_mint: AssetId::new(ChainId::Solana, base_mint.to_string())
                .expect("Solana pubkey string is never empty"),
            quote_mint: AssetId::new(ChainId::Solana, quote_mint.to_string())
                .expect("Solana pubkey string is never empty"),
            reserve_accounts: reserve_accounts
                .iter()
                .map(|account| {
                    AccountId::new(ChainId::Solana, account.to_string())
                        .expect("Solana pubkey string is never empty")
                })
                .collect(),
            liquidity_usd,
            volume_h24_usd,
            last_updated: Instant::now(),
        })
    }

    /// Classify pool program type (static version)
    fn classify_program_static(program_id: &Pubkey) -> ProgramKind {
        let program_str = program_id.to_string();
        ProgramKind::from_program_id(&program_str)
    }

    /// Extract reserve account addresses based on program type
    async fn extract_reserve_accounts(
        pool_id: &Pubkey,
        program_kind: &ProgramKind,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<Vec<Pubkey>, PoolAnalysisFailure> {
        match program_kind {
            ProgramKind::RaydiumCpmm => {
                Self::extract_raydium_cpmm_accounts(pool_id, base_mint, quote_mint, rpc_client)
                    .await
            }

            ProgramKind::RaydiumLegacyAmm => {
                Self::extract_raydium_legacy_accounts(pool_id, base_mint, quote_mint, rpc_client)
                    .await
            }

            ProgramKind::RaydiumClmm => {
                Self::extract_raydium_clmm_accounts(pool_id, base_mint, quote_mint, rpc_client)
                    .await
            }

            ProgramKind::OrcaWhirlpool => {
                Self::extract_orca_whirlpool_accounts(pool_id, base_mint, quote_mint, rpc_client)
                    .await
            }

            ProgramKind::MeteoraDamm => {
                Self::extract_meteora_damm_accounts(pool_id, base_mint, quote_mint, rpc_client)
                    .await
            }

            ProgramKind::MeteoraDlmm => {
                Self::extract_meteora_dlmm_accounts(pool_id, base_mint, quote_mint, rpc_client)
                    .await
            }

            ProgramKind::MeteoraDbc => {
                logger::debug(
                    LogTag::PoolAnalyzer,
                    &format!("Extracting DBC accounts for pool {pool_id}"),
                );

                let mut accounts = vec![*pool_id];

                let pool_account = Self::fetch_pool_account(pool_id, rpc_client).await?;
                let Some(vault_addresses) =
                    super::decoders::meteora_dbc::MeteoraDbcDecoder::extract_reserve_accounts(
                        &pool_account.data,
                    )
                else {
                    logger::warning(
                        LogTag::PoolAnalyzer,
                        &format!("Failed to extract vault addresses from DBC pool {pool_id}"),
                    );
                    return Err(PoolAnalysisFailure::Structural);
                };
                let vault_count = vault_addresses.len();
                for vault_str in vault_addresses {
                    if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                        accounts.push(vault_pubkey);
                    }
                }

                logger::debug(
                    LogTag::PoolAnalyzer,
                    &format!(
                        "DBC pool {} extracted {} vault accounts",
                        pool_id, vault_count
                    ),
                );

                // Always include the mints
                accounts.push(*base_mint);
                accounts.push(*quote_mint);

                Ok(accounts)
            }

            ProgramKind::PumpFunAmm => {
                Self::extract_pump_fun_accounts(pool_id, base_mint, quote_mint, rpc_client).await
            }

            ProgramKind::PumpFunLegacy => {
                // PumpFun Legacy (bonding curves) don't have vaults - just need the pool account
                logger::debug(
                    LogTag::PoolAnalyzer,
                    &format!(
                        "Extracting PumpFun Legacy (bonding curve) accounts for pool {}",
                        pool_id
                    ),
                );
                Ok(vec![*pool_id])
            }

            ProgramKind::Moonit => {
                Self::extract_moonit_accounts(pool_id, base_mint, quote_mint, rpc_client).await
            }

            ProgramKind::FluxbeamAmm => {
                Self::extract_fluxbeam_accounts(pool_id, base_mint, quote_mint, rpc_client).await
            }

            ProgramKind::Unknown => {
                logger::warning(
                    LogTag::PoolAnalyzer,
                    &format!(
                        "Cannot extract accounts for unknown program type: {}",
                        pool_id
                    ),
                );
                Err(PoolAnalysisFailure::Structural)
            }
        }
    }

    /// Get analyzed pool by ID
    pub fn get_pool(&self, pool_id: &Pubkey) -> Option<PoolDescriptor> {
        let directory = self.pool_directory.read().unwrap();
        directory.get(pool_id).cloned()
    }

    /// Get the pool this mint is priced from (the discovery selection), if analyzed
    pub fn get_canonical_pool(&self, mint: &str) -> Option<PoolDescriptor> {
        selection::selected_descriptor(&self.selected_pools, &self.pool_directory, mint)
    }

    /// Get pools for a specific token mint
    pub fn get_pools_for_token(&self, mint: &str) -> Vec<PoolDescriptor> {
        let directory = self.pool_directory.read().unwrap();
        directory
            .values()
            .filter(|pool| pool.base_mint.address() == mint || pool.quote_mint.address() == mint)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::{DataError, NetworkError};
    use crate::pools::types::ProtocolId;

    fn rpc(error: RpcError) -> crate::Error {
        crate::Error::Rpc(error)
    }

    #[test]
    fn rpc_transport_failures_are_transient() {
        let transient = [
            rpc(RpcError::Network {
                message: "operation timed out".to_owned(),
                is_timeout: true,
            }),
            rpc(RpcError::Network {
                message: "connection refused".to_owned(),
                is_timeout: false,
            }),
            rpc(RpcError::Timeout {
                provider_id: "p".to_owned(),
                after: Duration::from_secs(30),
            }),
            rpc(RpcError::RateLimited {
                provider_id: "p".to_owned(),
                retry_after: None,
            }),
            rpc(RpcError::CircuitOpen {
                provider_id: "p".to_owned(),
                retry_after: Duration::from_secs(5),
            }),
            rpc(RpcError::NoProvidersAvailable { last_error: None }),
            rpc(RpcError::ProviderError {
                code: -32005,
                message: "node is unhealthy".to_owned(),
                data: None,
            }),
            rpc(RpcError::InvalidResponse {
                message: "truncated body".to_owned(),
            }),
            crate::Error::Data(DataError::ParseError {
                data_type: "account".to_owned(),
                error: "Missing data field".to_owned(),
            }),
            crate::Error::Network(NetworkError::Timeout {
                endpoint: "rpc".to_owned(),
                timeout_ms: 1_000,
            }),
        ];
        for error in &transient {
            assert_eq!(
                classify_account_fetch_error(error),
                PoolAnalysisFailure::Transient,
                "{error}"
            );
        }
    }

    #[test]
    fn an_account_reported_missing_is_structural() {
        let error = rpc(RpcError::AccountNotFound {
            pubkey: "pool".to_owned(),
        });
        assert_eq!(
            classify_account_fetch_error(&error),
            PoolAnalysisFailure::Structural
        );
    }

    #[test]
    fn transient_backoff_doubles_up_to_the_cap() {
        let mut delay = next_transient_delay(None);
        assert_eq!(delay, TRANSIENT_RETRY_INITIAL);
        let mut seen = vec![delay];
        for _ in 0..6 {
            delay = next_transient_delay(Some(delay));
            seen.push(delay);
        }
        assert_eq!(
            seen.iter().map(Duration::as_secs).collect::<Vec<_>>(),
            vec![60, 120, 240, 480, 600, 600, 600]
        );
    }

    #[test]
    fn an_analyzed_pool_is_not_analyzed_again() {
        let pool = Pubkey::new_unique();
        let mut directory = HashMap::new();
        assert!(needs_analysis(&directory, &pool));

        let mut descriptor = PoolDescriptor {
            pool_id: PoolId::new(ChainId::Solana, pool.to_string()).unwrap(),
            program_kind: ProtocolId::new("RAYDIUM CPMM"),
            base_mint: AssetId::new(ChainId::Solana, "TokenA").unwrap(),
            quote_mint: AssetId::new(ChainId::Solana, "SolMint").unwrap(),
            reserve_accounts: Vec::new(),
            liquidity_usd: 0.0,
            volume_h24_usd: 0.0,
            last_updated: Instant::now(),
        };
        directory.insert(pool, descriptor.clone());
        assert!(
            needs_analysis(&directory, &pool),
            "a descriptor without reserve accounts is re-analyzed"
        );

        descriptor.reserve_accounts = vec![AccountId::new(ChainId::Solana, "vault").unwrap()];
        directory.insert(pool, descriptor);
        assert!(!needs_analysis(&directory, &pool));
    }
}
