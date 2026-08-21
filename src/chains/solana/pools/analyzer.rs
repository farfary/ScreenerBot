//! Pool analyzer module.
//!
//! Analyzes discovered pools to classify pool types by program ID, extract pool
//! metadata (base/quote tokens, reserve accounts), validate pool structure and
//! data, and prepare account lists for fetching.

use super::types::ProgramKind;

use crate::chains::solana::pools::service;
use crate::chains::solana::rpc::{get_rpc_client, RpcClient, RpcClientMethods};
use crate::chains::{AccountId, AssetId, ChainId, PoolId};
use crate::events::{record_safe, Event, EventCategory, Severity};
use crate::logger::{self, LogTag};
use crate::pools::types::PoolDescriptor;
use crate::pools::utils::is_sol_mint;

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::{mpsc, Notify};

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
    pub fn new(pool_directory: Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>) -> Self {
        let (analyzer_tx, analyzer_rx) = mpsc::unbounded_channel();

        Self {
            pool_directory,
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

    /// Start analyzer background task
    pub async fn start_analyzer_task(&self, shutdown: Arc<Notify>) {
        logger::info(LogTag::PoolAnalyzer, "Starting pool analyzer task");

        let pool_directory = self.pool_directory.clone();

        // Clone metrics for tracking in background task
        let operations = Arc::clone(&self.operations);
        let errors = Arc::clone(&self.errors);
        let pools_analyzed = Arc::clone(&self.pools_analyzed);

        // Take the receiver from the Arc<RwLock>
        let mut analyzer_rx = {
            let mut rx_lock = self.analyzer_rx.write().unwrap();
            rx_lock.take().expect("Analyzer receiver already taken")
        };

        tokio::spawn(async move {
            logger::info(LogTag::PoolAnalyzer, "Pool analyzer task started");

            // Get RPC client inside the task
            let rpc_client = get_rpc_client();

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
                                    // Check if pool is blacklisted in database
                                    if let Ok(is_blacklisted) = crate::pools::db::is_pool_blacklisted(crate::chains::ChainId::Solana, &pool_id.to_string()).await {
                                        if is_blacklisted {
                                            logger::debug(
                                                LogTag::PoolAnalyzer,
                                                &format!("Skipping blacklisted pool: {pool_id}"),
                                            );
                                            continue;
                                        }
                                    }

                                    // Determine the token side for blacklist tracking
                                    let token_to_check = if is_sol_mint(&base_mint.to_string()) { quote_mint } else { base_mint };

                                    if let Some(descriptor) = Self::analyze_pool_static(
                                        pool_id,
                                        program_id,
                                        base_mint,
                                        quote_mint,
                                        liquidity_usd,
                                        volume_h24_usd,
                                        rpc_client
                                    ).await {
                                        // Track metrics
                                        operations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                        pools_analyzed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                                        // Store analyzed pool in directory
                                        let mut directory = pool_directory.write().unwrap();
                                        directory.insert(pool_id, descriptor.clone());
                                        // Trigger account fetch for this pool's reserve accounts
                                        if let Some(fetcher) = service::get_account_fetcher() {
                                            let reserve_accounts: Vec<Pubkey> = descriptor
                                                .reserve_accounts
                                                .iter()
                                                .filter_map(|account| {
                                                    Pubkey::from_str(account.address()).ok()
                                                })
                                                .collect();
                                            if let Err(e) = fetcher.request_pool_fetch(pool_id, reserve_accounts) {
                                                logger::warning(LogTag::PoolAnalyzer, &format!("Failed to request fetch for analyzed pool {pool_id}: {e}"));
                                            }
                                        }

                                        let base_mint_str = descriptor.base_mint.address().to_owned();
                                        let quote_mint_str = descriptor.quote_mint.address().to_owned();
                                        let token_mint = if is_sol_mint(&base_mint_str) {
                                            &quote_mint_str
                                        } else {
                                            &base_mint_str
                                        };
                                        logger::debug(
                                            LogTag::PoolAnalyzer,
                                            &format!(
                                                "Analyzed pool {} for token {} ({}) - {}/{}",
                                                pool_id,
                                                token_mint,
                                                descriptor.program_kind.as_str(),
                                                base_mint,
                                                quote_mint
                                            ),
                                        );
                                    } else {
                                        // Track error
                                        errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                                        // Blacklist pool in database to prevent future attempts
                                        if let Err(e) = crate::pools::db::add_pool_to_blacklist(
                                            crate::chains::ChainId::Solana,
                                            &pool_id.to_string(),
                                            "analysis_failed",
                                            Some(&token_to_check.to_string()),
                                            Some(&program_id.to_string())
                                        ).await {
                                            logger::warning(
                                                LogTag::PoolAnalyzer,
                                                &format!("Failed to blacklist pool {pool_id}: {e}"),
                                            );
                                        }

                                        logger::warning(
                                            LogTag::PoolAnalyzer,
                                            &format!("Failed to analyze pool {pool_id} for token {token_to_check} - blacklisted permanently"),
                                        );
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
        });
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
    ) -> Option<PoolDescriptor> {
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
                    return None;
                }
                Err(e) => {
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

                    logger::warning(
                        LogTag::PoolAnalyzer,
                        &format!(
                            "Failed to fetch pool account {} for token analysis: {}",
                            pool_id, e
                        ),
                    );
                    return None;
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
            return None;
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

        Some(PoolDescriptor {
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
    ) -> Option<Vec<Pubkey>> {
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

                // Fetch pool account to extract vault addresses using decoder function
                if let Ok(Some(pool_account)) = rpc_client.get_account(pool_id).await {
                    if let Some(vault_addresses) =
                        super::decoders::meteora_dbc::MeteoraDbcDecoder::extract_reserve_accounts(
                            &pool_account.data,
                        )
                    {
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
                    } else {
                        logger::warning(
                            LogTag::PoolAnalyzer,
                            &format!(
                                "Failed to extract vault addresses from DBC pool {}",
                                pool_id
                            ),
                        );
                    }
                }

                // Always include the mints
                accounts.push(*base_mint);
                accounts.push(*quote_mint);

                Some(accounts)
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
                Some(vec![*pool_id])
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
                None
            }
        }
    }

    /// Get analyzed pool by ID
    pub fn get_pool(&self, pool_id: &Pubkey) -> Option<PoolDescriptor> {
        let directory = self.pool_directory.read().unwrap();
        directory.get(pool_id).cloned()
    }

    /// Get the canonical pool tracked by the price calculator for this mint (if any)
    pub fn get_canonical_pool(&self, mint: &str) -> Option<PoolDescriptor> {
        let calculator = service::get_price_calculator();
        let calculator = calculator?;
        calculator.get_canonical_pool(mint)
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
