/// Pool analyzer module
///
/// This module analyzes discovered pools to:
/// - Classify pool types by program ID
/// - Extract pool metadata (base/quote tokens, reserve accounts)
/// - Validate pool structure and data
/// - Prepare account lists for fetching

use crate::global::is_debug_pool_service_enabled;
use crate::arguments::is_debug_pool_analyzer_enabled;
use crate::logger::{ log, LogTag };
use crate::rpc::RpcClient;
use super::types::{ PoolDescriptor, ProgramKind };
use super::utils::{ PoolMintVaultInfo, is_sol_mint };
use super::decoders::{
    meteora_damm::MeteoraDammDecoder,
    meteora_dbc::MeteoraDbcDecoder,
    meteora_dlmm::MeteoraDlmmDecoder,
    raydium_cpmm::RaydiumCpmmDecoder,
    raydium_clmm::RaydiumClmmDecoder,
    raydium_legacy_amm::RaydiumLegacyAmmDecoder,
    orca_whirlpool::OrcaWhirlpoolDecoder,
    pumpfun_amm::PumpFunAmmDecoder,
    pumpfun_legacy::PumpFunLegacyDecoder,
};
use crate::pools::service; // access global fetcher
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{ Arc, RwLock };
use std::time::Instant;
use tokio::sync::{ mpsc, Notify };

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
    /// RPC client for on-chain data fetching
    rpc_client: Arc<RpcClient>,
    /// Channel for receiving analysis requests
    analyzer_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<AnalyzerMessage>>>>,
    /// Channel sender for sending analysis requests
    analyzer_tx: mpsc::UnboundedSender<AnalyzerMessage>,
    /// In-memory set of failed (pool_id, token_mint) pairs to avoid repeated re-analysis
    failed_pairs: Arc<RwLock<HashSet<(Pubkey, Pubkey)>>>,
}

impl PoolAnalyzer {
    /// Create new pool analyzer
    pub fn new(
        rpc_client: Arc<RpcClient>,
        pool_directory: Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>
    ) -> Self {
        let (analyzer_tx, analyzer_rx) = mpsc::unbounded_channel();

        Self {
            pool_directory,
            rpc_client,
            analyzer_rx: Arc::new(RwLock::new(Some(analyzer_rx))),
            analyzer_tx,
            failed_pairs: Arc::new(RwLock::new(HashSet::new())),
        }
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
        if is_debug_pool_analyzer_enabled() {
            log(LogTag::PoolAnalyzer, "INFO", "Starting pool analyzer task");
        }

        let pool_directory = self.pool_directory.clone();
        let rpc_client = self.rpc_client.clone();
        let failed_pairs = self.failed_pairs.clone();

        // Take the receiver from the Arc<RwLock>
        let mut analyzer_rx = {
            let mut rx_lock = self.analyzer_rx.write().unwrap();
            rx_lock.take().expect("Analyzer receiver already taken")
        };

        tokio::spawn(async move {
            if is_debug_pool_analyzer_enabled() {
                log(LogTag::PoolAnalyzer, "INFO", "Pool analyzer task started");
            }

            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        if is_debug_pool_analyzer_enabled() {
                            log(LogTag::PoolAnalyzer, "INFO", "Pool analyzer task shutting down");
                        }
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
                                    // Determine the token side we consider for failure tracking
                                    let token_to_check = if is_sol_mint(&base_mint.to_string()) { quote_mint } else { base_mint };
                                    let pair = (pool_id, token_to_check);

                                    // Skip re-analysis if this (pool, token) already failed earlier in this run
                                    let already_failed = {
                                        let fp = failed_pairs.read().unwrap();
                                        fp.contains(&pair)
                                    };

                                    if already_failed {
                                        if is_debug_pool_analyzer_enabled() {
                                            log(
                                                LogTag::PoolAnalyzer,
                                                "DEBUG",
                                                &format!("Skipping re-analysis of pool {} for token {} (previously failed this run)", pool_id, token_to_check)
                                            );
                                        }
                                        continue;
                                    }

                                    if let Some(descriptor) = Self::analyze_pool_static(
                                        pool_id,
                                        program_id,
                                        base_mint,
                                        quote_mint,
                                        liquidity_usd,
                                        volume_h24_usd,
                                        &rpc_client
                                    ).await {
                                        // Store analyzed pool in directory
                                        let mut directory = pool_directory.write().unwrap();
                                        directory.insert(pool_id, descriptor.clone());
                                        // Trigger account fetch for this pool's reserve accounts
                                        if let Some(fetcher) = service::get_account_fetcher() {
                                            let reserve_accounts = descriptor.reserve_accounts.clone();
                                            if let Err(e) = fetcher.request_pool_fetch(pool_id, reserve_accounts) {
                                                log(LogTag::PoolAnalyzer, "WARN", &format!("Failed to request fetch for analyzed pool {}: {}", pool_id, e));
                                            }
                                        }
                                    
                                        if is_debug_pool_analyzer_enabled() {
                                            log(
                                                LogTag::PoolAnalyzer, 
                                                "DEBUG", 
                                                &format!(
                                                    "Analyzed pool {} for token {} ({}) - {}/{}", 
                                                    pool_id,
                                                    if is_sol_mint(&descriptor.base_mint.to_string()) { 
                                                        &descriptor.quote_mint.to_string() 
                                                    } else { 
                                                        &descriptor.base_mint.to_string() 
                                                    },
                                                    descriptor.program_kind.display_name(),
                                                    base_mint,
                                                    quote_mint
                                                )
                                            );
                                        }
                                    } else {
                                        // Record failure in-memory to avoid repeated attempts this run
                                        let mut fp = failed_pairs.write().unwrap();
                                        fp.insert(pair);

                                        log(
                                            LogTag::PoolAnalyzer, 
                                            "WARN", 
                                            &format!("Failed to analyze pool {} for token {} - will skip retries this run", 
                                                pool_id,
                                                token_to_check)
                                        );
                                    }
                                }

                                Some(AnalyzerMessage::Shutdown) => {
                                    if is_debug_pool_analyzer_enabled() {
                                        log(LogTag::PoolAnalyzer, "INFO", "Pool analyzer received shutdown signal");
                                    }
                                    break;
                                }

                                None => {
                                    if is_debug_pool_analyzer_enabled() {
                                        log(LogTag::PoolAnalyzer, "INFO", "Pool analyzer channel closed");
                                    }
                                    break;
                                }
                            }
                        }
                }
            }

            if is_debug_pool_analyzer_enabled() {
                log(LogTag::PoolAnalyzer, "INFO", "Pool analyzer task completed");
            }
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
        rpc_client: &RpcClient
    ) -> Option<PoolDescriptor> {
        // First, try to determine the actual program type by fetching the pool account
        let actual_program_id = if program_id == Pubkey::default() {
            // This is an Unknown pool from discovery - fetch the account to get the real program ID
            match rpc_client.get_account(&pool_id).await {
                Ok(account) => {
                    if is_debug_pool_analyzer_enabled() {
                        log(
                            LogTag::PoolAnalyzer,
                            "DEBUG",
                            &format!("Pool {} owner: {}", pool_id, account.owner)
                        );
                    }
                    account.owner
                }
                Err(e) => {
                    if is_debug_pool_analyzer_enabled() {
                        log(
                            LogTag::PoolAnalyzer,
                            "WARN",
                            &format!(
                                "Failed to fetch pool account {} for token analysis: {}",
                                pool_id,
                                e
                            )
                        );
                    }
                    return None;
                }
            }
        } else {
            program_id
        };

        // Classify the program type using the actual program ID
        let program_kind = Self::classify_program_static(&actual_program_id);

        if program_kind == ProgramKind::Unknown {
            if is_debug_pool_analyzer_enabled() {
                log(
                    LogTag::PoolAnalyzer,
                    "WARN",
                    &format!(
                        "Unsupported DEX program for pool {}: {} (consider adding support for this DEX)",
                        pool_id,
                        actual_program_id
                    )
                );
            }
            return None;
        }

        if is_debug_pool_analyzer_enabled() {
            log(
                LogTag::PoolAnalyzer,
                "DEBUG",
                &format!("Classified pool {} as {}", pool_id, program_kind.display_name())
            );
        }

        // Extract reserve accounts based on program type
        let reserve_accounts = Self::extract_reserve_accounts(
            &pool_id,
            &program_kind,
            &base_mint,
            &quote_mint,
            rpc_client
        ).await?;

        if is_debug_pool_analyzer_enabled() {
            log(
                LogTag::PoolAnalyzer,
                "DEBUG",
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
                )
            );
        }

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
        rpc_client: &RpcClient
    ) -> Option<Vec<Pubkey>> {
        match program_kind {
            ProgramKind::RaydiumCpmm => {
                Self::extract_raydium_cpmm_accounts(
                    pool_id,
                    base_mint,
                    quote_mint,
                    rpc_client
                ).await
            }

            ProgramKind::RaydiumLegacyAmm => {
                Self::extract_raydium_legacy_accounts(
                    pool_id,
                    base_mint,
                    quote_mint,
                    rpc_client
                ).await
            }

            ProgramKind::RaydiumClmm => {
                Self::extract_raydium_clmm_accounts(
                    pool_id,
                    base_mint,
                    quote_mint,
                    rpc_client
                ).await
            }

            ProgramKind::OrcaWhirlpool => {
                Self::extract_orca_whirlpool_accounts(
                    pool_id,
                    base_mint,
                    quote_mint,
                    rpc_client
                ).await
            }

            ProgramKind::MeteoraDamm => {
                Self::extract_meteora_damm_accounts(
                    pool_id,
                    base_mint,
                    quote_mint,
                    rpc_client
                ).await
            }

            ProgramKind::MeteoraDlmm => {
                Self::extract_meteora_dlmm_accounts(
                    pool_id,
                    base_mint,
                    quote_mint,
                    rpc_client
                ).await
            }

            ProgramKind::MeteoraDbc => {
                if is_debug_pool_analyzer_enabled() {
                    log(
                        LogTag::PoolAnalyzer,
                        "INFO",
                        &format!("Extracting DBC accounts for pool {}", pool_id)
                    );
                }

                let mut accounts = vec![*pool_id];

                // Fetch pool account to extract vault addresses using decoder function
                if let Ok(pool_account) = rpc_client.get_account(pool_id).await {
                    if
                        let Some(vault_addresses) =
                            super::decoders::meteora_dbc::MeteoraDbcDecoder::extract_reserve_accounts(
                                &pool_account.data
                            )
                    {
                        let vault_count = vault_addresses.len();
                        for vault_str in vault_addresses {
                            if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                                accounts.push(vault_pubkey);
                            }
                        }

                        if is_debug_pool_analyzer_enabled() {
                            log(
                                LogTag::PoolAnalyzer,
                                "INFO",
                                &format!(
                                    "DBC pool {} extracted {} vault accounts",
                                    pool_id,
                                    vault_count
                                )
                            );
                        }
                    } else {
                        if is_debug_pool_analyzer_enabled() {
                            log(
                                LogTag::PoolAnalyzer,
                                "WARN",
                                &format!("Failed to extract vault addresses from DBC pool {}", pool_id)
                            );
                        }
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
                Self::extract_pump_fun_accounts(pool_id, base_mint, quote_mint, rpc_client).await
            }

            ProgramKind::Moonit => {
                Self::extract_moonit_accounts(pool_id, base_mint, quote_mint, rpc_client).await
            }

            ProgramKind::Unknown => {
                if is_debug_pool_analyzer_enabled() {
                    log(
                        LogTag::PoolAnalyzer,
                        "WARN",
                        &format!("Cannot extract accounts for unknown program type: {}", pool_id)
                    );
                }
                None
            }
        }
    }

    /// Extract Raydium CPMM pool accounts
    async fn extract_raydium_cpmm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient
    ) -> Option<Vec<Pubkey>> {
        // Fetch the pool account to extract vault addresses using decoder function
        let pool_account = match rpc_client.get_account(pool_id).await {
            Ok(account) => account,
            Err(e) => {
                if is_debug_pool_analyzer_enabled() {
                    log(
                        LogTag::PoolAnalyzer,
                        "ERROR",
                        &format!("Failed to fetch pool account {}: {}", pool_id, e)
                    );
                }
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
        rpc_client: &RpcClient
    ) -> Option<Vec<Pubkey>> {
        if is_debug_pool_analyzer_enabled() {
            log(
                LogTag::PoolAnalyzer,
                "INFO",
                &format!("Extracting Raydium Legacy AMM accounts for pool {}", pool_id)
            );
        }

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(pool_account) = rpc_client.get_account(pool_id).await {
            if
                let Some(vault_addresses) = RaydiumLegacyAmmDecoder::extract_reserve_accounts(
                    &pool_account.data
                )
            {
                let vault_count = vault_addresses.len();
                for vault_str in vault_addresses {
                    if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                        accounts.push(vault_pubkey);
                    }
                }

                if is_debug_pool_analyzer_enabled() {
                    log(
                        LogTag::PoolAnalyzer,
                        "INFO",
                        &format!(
                            "Raydium Legacy AMM pool {} extracted {} vault accounts",
                            pool_id,
                            vault_count
                        )
                    );
                }
            } else {
                if is_debug_pool_analyzer_enabled() {
                    log(
                        LogTag::PoolAnalyzer,
                        "WARN",
                        &format!("Failed to extract vault addresses from Raydium Legacy AMM pool {}", pool_id)
                    );
                }
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
        rpc_client: &RpcClient
    ) -> Option<Vec<Pubkey>> {
        // For CLMM pools, we need:
        // - Pool account itself
        // - Token vaults (extracted from pool data)

        if is_debug_pool_analyzer_enabled() {
            log(
                LogTag::PoolAnalyzer,
                "INFO",
                &format!("Extracting CLMM accounts for pool {}", pool_id)
            );
        }

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(pool_account) = rpc_client.get_account(pool_id).await {
            if
                let Some(vault_addresses) = RaydiumClmmDecoder::extract_reserve_accounts(
                    &pool_account.data
                )
            {
                let vault_count = vault_addresses.len();
                for vault_str in vault_addresses {
                    if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                        accounts.push(vault_pubkey);
                    }
                }

                if is_debug_pool_analyzer_enabled() {
                    log(
                        LogTag::PoolAnalyzer,
                        "INFO",
                        &format!("CLMM pool {} extracted {} vault accounts", pool_id, vault_count)
                    );
                }
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
        rpc_client: &RpcClient
    ) -> Option<Vec<Pubkey>> {
        if is_debug_pool_analyzer_enabled() {
            log(
                LogTag::PoolAnalyzer,
                "INFO",
                &format!("Extracting Orca Whirlpool accounts for pool {}", pool_id)
            );
        }

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(pool_account) = rpc_client.get_account(pool_id).await {
            if
                let Some(vault_addresses) = OrcaWhirlpoolDecoder::extract_reserve_accounts(
                    &pool_account.data
                )
            {
                let vault_count = vault_addresses.len();
                for vault_str in vault_addresses {
                    if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                        accounts.push(vault_pubkey);
                    }
                }

                if is_debug_pool_analyzer_enabled() {
                    log(
                        LogTag::PoolAnalyzer,
                        "INFO",
                        &format!(
                            "Orca Whirlpool pool {} extracted {} vault accounts",
                            pool_id,
                            vault_count
                        )
                    );
                }
            } else {
                if is_debug_pool_analyzer_enabled() {
                    log(
                        LogTag::PoolAnalyzer,
                        "WARN",
                        &format!("Failed to extract vault addresses from Orca Whirlpool pool {}", pool_id)
                    );
                }
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
        rpc_client: &RpcClient
    ) -> Option<Vec<Pubkey>> {
        if is_debug_pool_analyzer_enabled() {
            log(
                LogTag::PoolAnalyzer,
                "INFO",
                &format!("Extracting DAMM accounts for pool {}", pool_id)
            );
        }

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(pool_account) = rpc_client.get_account(pool_id).await {
            if
                let Some(vault_addresses) = MeteoraDammDecoder::extract_reserve_accounts(
                    &pool_account.data
                )
            {
                let vault_count = vault_addresses.len();
                for vault_str in vault_addresses {
                    if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                        accounts.push(vault_pubkey);
                    }
                }

                if is_debug_pool_analyzer_enabled() {
                    log(
                        LogTag::PoolAnalyzer,
                        "INFO",
                        &format!("DAMM pool {} extracted {} vault accounts", pool_id, vault_count)
                    );
                }
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
        rpc_client: &RpcClient
    ) -> Option<Vec<Pubkey>> {
        // Fetch the pool account to extract vault addresses using decoder function
        let pool_account = match rpc_client.get_account(pool_id).await {
            Ok(account) => account,
            Err(e) => {
                if is_debug_pool_analyzer_enabled() {
                    log(
                        LogTag::PoolAnalyzer,
                        "ERROR",
                        &format!("Failed to fetch DLMM pool account {}: {}", pool_id, e)
                    );
                }
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
        rpc_client: &RpcClient
    ) -> Option<Vec<Pubkey>> {
        if is_debug_pool_analyzer_enabled() {
            log(
                LogTag::PoolAnalyzer,
                "INFO",
                &format!("Extracting PumpFun AMM accounts for pool {}", pool_id)
            );
        }

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(pool_account) = rpc_client.get_account(pool_id).await {
            if
                let Some(vault_addresses) = PumpFunAmmDecoder::extract_reserve_accounts(
                    &pool_account.data
                )
            {
                let vault_count = vault_addresses.len();
                for vault_str in vault_addresses {
                    if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                        accounts.push(vault_pubkey);
                    }
                }

                if is_debug_pool_analyzer_enabled() {
                    log(
                        LogTag::PoolAnalyzer,
                        "INFO",
                        &format!(
                            "PumpFun AMM pool {} extracted {} vault accounts",
                            pool_id,
                            vault_count
                        )
                    );
                }
            } else {
                if is_debug_pool_analyzer_enabled() {
                    log(
                        LogTag::PoolAnalyzer,
                        "WARN",
                        &format!("Failed to extract vault addresses from PumpFun AMM pool {}", pool_id)
                    );
                }
            }
        }

        // NOTE: Mint accounts removed - decimals now fetched from cache, not mint accounts

        Some(accounts)
    }

    /// Extract Moonit AMM accounts
    async fn extract_moonit_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient
    ) -> Option<Vec<Pubkey>> {
        // For Moonit pools, we only need:
        // - Curve account (pool_id) - contains all pool data including SOL balance in account lamports
        // NOTE: Mint accounts removed - decimals now fetched from cache, not mint accounts

        let mut accounts = vec![*pool_id];

        // NOTE: Mint accounts removed - decimals now fetched from cache, not mint account data

        if is_debug_pool_analyzer_enabled() {
            log(
                LogTag::PoolAnalyzer,
                "DEBUG",
                &format!(
                    "Extracted Moonit accounts: curve={}, total_accounts={}",
                    pool_id,
                    accounts.len()
                )
            );
        }

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
        volume_h24_usd: f64
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
