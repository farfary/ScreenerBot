/// Pool analyzer module
///
/// This module analyzes discovered pools to:
/// - Classify pool types by program ID
/// - Extract pool metadata (base/quote tokens, reserve accounts)
/// - Validate pool structure and data
/// - Prepare account lists for fetching
use super::decoders::{
    meteora_damm::MeteoraDammDecoder, meteora_dbc::MeteoraDbcDecoder,
    meteora_dlmm::MeteoraDlmmDecoder, orca_whirlpool::OrcaWhirlpoolDecoder,
    pumpfun_amm::PumpFunAmmDecoder, pumpfun_legacy::PumpFunLegacyDecoder,
    raydium_clmm::RaydiumClmmDecoder, raydium_cpmm::RaydiumCpmmDecoder,
    raydium_legacy_amm::RaydiumLegacyAmmDecoder,
};
use super::types::{PoolDescriptor, ProgramKind};
use super::utils::{is_sol_mint, PoolMintVaultInfo};

use crate::events::{record_safe, Event, EventCategory, Severity};
use crate::logger::{self, LogTag};
use crate::pools::service;
use crate::rpc::client::RpcClient;
use crate::rpc::{get_rpc_client, RpcClientMethods};

use solana_sdk::pubkey::Pubkey;
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
    pub fn new(
        pool_directory: Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
    ) -> Self {
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
                                    if let Ok(is_blacklisted) = super::db::is_pool_blacklisted(&pool_id.to_string()).await {
                                        if is_blacklisted {
                                            logger::debug(
                                                LogTag::PoolAnalyzer,
                                                &format!("Skipping blacklisted pool: {}", pool_id),
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
                                            let reserve_accounts = descriptor.reserve_accounts.clone();
                                            if let Err(e) = fetcher.request_pool_fetch(pool_id, reserve_accounts) {
                                                logger::warning(LogTag::PoolAnalyzer, &format!("Failed to request fetch for analyzed pool {}: {}", pool_id, e));
                                            }
                                        }

                                        let base_mint_str = descriptor.base_mint.to_string();
                                        let quote_mint_str = descriptor.quote_mint.to_string();
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
                                                descriptor.program_kind.display_name(),
                                                base_mint,
                                                quote_mint
                                            ),
                                        );
                                    } else {
                                        // Track error
                                        errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                                        // Blacklist pool in database to prevent future attempts
                                        if let Err(e) = super::db::add_pool_to_blacklist(
                                            &pool_id.to_string(),
                                            "analysis_failed",
                                            Some(&token_to_check.to_string()),
                                            Some(&program_id.to_string())
                                        ).await {
                                            logger::warning(
                                                LogTag::PoolAnalyzer,
                                                &format!("Failed to blacklist pool {}: {}", pool_id, e),
                                            );
                                        }

                                        logger::warning(
                                            LogTag::PoolAnalyzer,
                                            &format!("Failed to analyze pool {} for token {} - blacklisted permanently", pool_id, token_to_check),
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
                        Some("pool_account_fetch_failed".to_string()),
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
                        &format!(
                            "Pool account {} not found for token analysis",
                            pool_id
                        ),
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
                        Some("pool_account_fetch_failed".to_string()),
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
                Some("unsupported_program".to_string()),
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

            logger::warning(LogTag::PoolAnalyzer, &format!("Unsupported DEX program for pool {}: {} (consider adding support for this DEX)", pool_id, actual_program_id));
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
            pool_id,
            program_kind,
            base_mint,
            quote_mint,
            reserve_accounts,
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
                    &format!("Extracting DBC accounts for pool {}", pool_id),
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

    /// Extract Raydium CPMM pool accounts
    async fn extract_raydium_cpmm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        // Fetch the pool account to extract vault addresses using decoder function
        let pool_account = match rpc_client.get_account(pool_id).await {
            Ok(Some(account)) => account,
            Ok(None) => {
                logger::error(
                    LogTag::PoolAnalyzer,
                    &format!("Pool account {} not found", pool_id),
                );
                return None;
            }
            Err(e) => {
                logger::error(
                    LogTag::PoolAnalyzer,
                    &format!("Failed to fetch pool account {}: {}", pool_id, e),
                );
                return None;
            }
        };

        // Parse the pool data to extract vault addresses using decoder function
        let vault_addresses = RaydiumCpmmDecoder::extract_reserve_accounts(&pool_account.data)?;

        let mut accounts = vec![*pool_id];

        // Add vault addresses to accounts list
        for vault_str in vault_addresses {
            if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                accounts.push(vault_pubkey);
            }
        }

        // Add the mints for reference
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        Some(accounts)
    }

    /// Extract Raydium Legacy AMM pool accounts
    async fn extract_raydium_legacy_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "Extracting Raydium Legacy AMM accounts for pool {}",
                pool_id
            ),
        );

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(Some(pool_account)) = rpc_client.get_account(pool_id).await {
            if let Some(vault_addresses) =
                RaydiumLegacyAmmDecoder::extract_reserve_accounts(&pool_account.data)
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
                        "Raydium Legacy AMM pool {} extracted {} vault accounts",
                        pool_id, vault_count
                    ),
                );
            } else {
                logger::warning(
                    LogTag::PoolAnalyzer,
                    &format!(
                        "Failed to extract vault addresses from Raydium Legacy AMM pool {}",
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

    /// Extract Raydium CLMM pool accounts
    async fn extract_raydium_clmm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        // For CLMM pools, we need:
        // - Pool account itself
        // - Token vaults (extracted from pool data)

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!("Extracting CLMM accounts for pool {}", pool_id),
        );

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(Some(pool_account)) = rpc_client.get_account(pool_id).await {
            if let Some(vault_addresses) =
                RaydiumClmmDecoder::extract_reserve_accounts(&pool_account.data)
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
                        "CLMM pool {} extracted {} vault accounts",
                        pool_id, vault_count
                    ),
                );
            }
        }

        // Always include the mints
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        Some(accounts)
    }

    /// Extract Orca Whirlpool accounts
    async fn extract_orca_whirlpool_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        logger::debug(
            LogTag::PoolAnalyzer,
            &format!("Extracting Orca Whirlpool accounts for pool {}", pool_id),
        );

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(Some(pool_account)) = rpc_client.get_account(pool_id).await {
            if let Some(vault_addresses) =
                OrcaWhirlpoolDecoder::extract_reserve_accounts(&pool_account.data)
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
                        "Orca Whirlpool pool {} extracted {} vault accounts",
                        pool_id, vault_count
                    ),
                );
            } else {
                logger::warning(
                    LogTag::PoolAnalyzer,
                    &format!(
                        "Failed to extract vault addresses from Orca Whirlpool pool {}",
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

    /// Extract Meteora DAMM accounts
    async fn extract_meteora_damm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        logger::debug(
            LogTag::PoolAnalyzer,
            &format!("Extracting DAMM accounts for pool {}", pool_id),
        );

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(Some(pool_account)) = rpc_client.get_account(pool_id).await {
            if let Some(vault_addresses) =
                MeteoraDammDecoder::extract_reserve_accounts(&pool_account.data)
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
                        "DAMM pool {} extracted {} vault accounts",
                        pool_id, vault_count
                    ),
                );
            }
        }

        // Always include the mints
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        Some(accounts)
    }

    /// Extract Meteora DLMM accounts
    async fn extract_meteora_dlmm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        // Fetch the pool account to extract vault addresses using decoder function
        let pool_account = match rpc_client.get_account(pool_id).await {
            Ok(Some(account)) => account,
            Ok(None) => {
                logger::error(
                    LogTag::PoolAnalyzer,
                    &format!("DLMM pool account {} not found", pool_id),
                );
                return None;
            }
            Err(e) => {
                logger::error(
                    LogTag::PoolAnalyzer,
                    &format!("Failed to fetch DLMM pool account {}: {}", pool_id, e),
                );
                return None;
            }
        };

        // Parse the pool data to extract vault addresses using decoder function
        let vault_addresses = MeteoraDlmmDecoder::extract_reserve_accounts(&pool_account.data)?;

        let mut accounts = vec![*pool_id];

        // Add vault addresses to accounts list
        for vault_str in vault_addresses {
            if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                accounts.push(vault_pubkey);
            }
        }

        // Add the mints for reference
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        Some(accounts)
    }

    /// Extract Pump.fun AMM accounts
    async fn extract_pump_fun_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        logger::debug(
            LogTag::PoolAnalyzer,
            &format!("Extracting PumpFun AMM accounts for pool {}", pool_id),
        );

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(Some(pool_account)) = rpc_client.get_account(pool_id).await {
            if let Some(vault_addresses) =
                PumpFunAmmDecoder::extract_reserve_accounts(&pool_account.data)
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
                        "PumpFun AMM pool {} extracted {} vault accounts",
                        pool_id, vault_count
                    ),
                );
            } else {
                logger::warning(
                    LogTag::PoolAnalyzer,
                    &format!(
                        "Failed to extract vault addresses from PumpFun AMM pool {}",
                        pool_id
                    ),
                );
            }
        }

        Some(accounts)
    }

    /// Extract Moonit AMM accounts
    async fn extract_moonit_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        let mut accounts = vec![*pool_id];

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "Extracted Moonit accounts: curve={}, total_accounts={}",
                pool_id,
                accounts.len()
            ),
        );

        Some(accounts)
    }

    async fn extract_fluxbeam_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        // Fetch the pool account to extract vault addresses using decoder function
        let pool_account = match rpc_client.get_account(pool_id).await {
            Ok(Some(account)) => account,
            Ok(None) => {
                logger::error(
                    LogTag::PoolAnalyzer,
                    &format!("FluxBeam pool account {} not found", pool_id),
                );
                return None;
            }
            Err(e) => {
                logger::error(
                    LogTag::PoolAnalyzer,
                    &format!("Failed to fetch pool account {}: {}", pool_id, e),
                );
                return None;
            }
        };

        // Parse the pool data to extract vault addresses using decoder function
        let vault_addresses =
            super::decoders::fluxbeam_amm::FluxbeamAmmDecoder::extract_reserve_accounts(
                &pool_account.data,
            )?;

        let mut accounts = vec![*pool_id];
        let vault_count = vault_addresses.len();

        // Add vault addresses to accounts list
        for vault_str in vault_addresses {
            if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                accounts.push(vault_pubkey);
            }
        }

        // Add the mints for reference
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "Extracted FluxBeam accounts: pool={}, vaults={}, total_accounts={}",
                pool_id,
                vault_count,
                accounts.len()
            ),
        );

        Some(accounts)
    }

    /// Public interface: Request analysis of a discovered pool
    pub fn request_analysis(
        &self,
        pool_id: Pubkey,
        program_id: Pubkey,
        base_mint: Pubkey,
        quote_mint: Pubkey,
        liquidity_usd: f64,
        volume_h24_usd: f64,
    ) -> Result<(), String> {
        let message = AnalyzerMessage::AnalyzePool {
            pool_id,
            program_id,
            base_mint,
            quote_mint,
            liquidity_usd,
            volume_h24_usd,
        };

        self.analyzer_tx
            .send(message)
            .map_err(|e| format!("Failed to send analysis request: {}", e))?;

        Ok(())
    }

    /// Get analyzed pool by ID
    pub fn get_pool(&self, pool_id: &Pubkey) -> Option<PoolDescriptor> {
        let directory = self.pool_directory.read().unwrap();
        directory.get(pool_id).cloned()
    }

    /// Get the canonical pool tracked by the price calculator for this mint (if any)
    pub fn get_canonical_pool(&self, mint: &Pubkey) -> Option<PoolDescriptor> {
        let calculator = super::service::get_price_calculator();
        let calculator = calculator?;
        calculator.get_canonical_pool(mint)
    }

    /// Get all analyzed pools
    pub fn get_all_pools(&self) -> Vec<PoolDescriptor> {
        let directory = self.pool_directory.read().unwrap();
        directory.values().cloned().collect()
    }

    /// Get pools for a specific token mint
    pub fn get_pools_for_token(&self, mint: &Pubkey) -> Vec<PoolDescriptor> {
        let directory = self.pool_directory.read().unwrap();
        directory
            .values()
            .filter(|pool| (&pool.base_mint == mint || &pool.quote_mint == mint))
            .cloned()
            .collect()
    }

    /// Clear analyzed pools (for cleanup)
    pub fn clear_pools(&self) {
        let mut directory = self.pool_directory.write().unwrap();
        directory.clear();
    }
}
