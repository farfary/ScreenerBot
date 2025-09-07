/// Pool analyzer module
///
/// This module analyzes discovered pools to:
/// - Classify pool types by program ID
/// - Extract pool metadata (base/quote tokens, reserve accounts)
/// - Validate pool structure and data
/// - Prepare account lists for fetching

use crate::global::is_debug_pool_service_enabled;
use crate::logger::{ log, LogTag };
use crate::rpc::RpcClient;
use super::types::{ PoolDescriptor, ProgramKind };
use super::utils::{ extract_pumpfun_mints_and_vaults, get_analyzer_vault_order, PoolMintVaultInfo };
use crate::pools::service; // access global fetcher
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
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
        if is_debug_pool_service_enabled() {
            log(LogTag::PoolService, "INFO", "Starting pool analyzer task");
        }

        let pool_directory = self.pool_directory.clone();
        let rpc_client = self.rpc_client.clone();

        // Take the receiver from the Arc<RwLock>
        let mut analyzer_rx = {
            let mut rx_lock = self.analyzer_rx.write().unwrap();
            rx_lock.take().expect("Analyzer receiver already taken")
        };

        tokio::spawn(async move {
            if is_debug_pool_service_enabled() {
                log(LogTag::PoolService, "INFO", "Pool analyzer task started");
            }

            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        if is_debug_pool_service_enabled() {
                            log(LogTag::PoolService, "INFO", "Pool analyzer task shutting down");
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
                                liquidity_usd 
                            }) => {
                                if let Some(descriptor) = Self::analyze_pool_static(
                                    pool_id,
                                    program_id,
                                    base_mint,
                                    quote_mint,
                                    liquidity_usd,
                                    &rpc_client
                                ).await {
                                    // Store analyzed pool in directory
                                    let mut directory = pool_directory.write().unwrap();
                                    directory.insert(pool_id, descriptor.clone());
                                    // Trigger account fetch for this pool's reserve accounts
                                    if let Some(fetcher) = service::get_account_fetcher() {
                                        let reserve_accounts = descriptor.reserve_accounts.clone();
                                        if let Err(e) = fetcher.request_pool_fetch(pool_id, reserve_accounts) {
                                            log(LogTag::PoolService, "WARN", &format!("Failed to request fetch for analyzed pool {}: {}", pool_id, e));
                                        }
                                    }
                                    
                                    if is_debug_pool_service_enabled() {
                                        log(
                                            LogTag::PoolService, 
                                            "DEBUG", 
                                            &format!(
                                                "Analyzed pool {} for token {} ({}) - {}/{}", 
                                                pool_id,
                                                if descriptor.base_mint.to_string() == "So11111111111111111111111111111111111111112" { 
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
                                    log(
                                        LogTag::PoolService, 
                                        "WARN", 
                                        &format!("Failed to analyze pool {} for token {}", 
                                            pool_id,
                                            if base_mint.to_string() == "So11111111111111111111111111111111111111112" { 
                                                quote_mint 
                                            } else { 
                                                base_mint 
                                            })
                                    );
                                }
                            }
                            
                            Some(AnalyzerMessage::Shutdown) => {
                                if is_debug_pool_service_enabled() {
                                    log(LogTag::PoolService, "INFO", "Pool analyzer received shutdown signal");
                                }
                                break;
                            }
                            
                            None => {
                                if is_debug_pool_service_enabled() {
                                    log(LogTag::PoolService, "INFO", "Pool analyzer channel closed");
                                }
                                break;
                            }
                        }
                    }
                }
            }

            if is_debug_pool_service_enabled() {
                log(LogTag::PoolService, "INFO", "Pool analyzer task completed");
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
        rpc_client: &RpcClient
    ) -> Option<PoolDescriptor> {
        // First, try to determine the actual program type by fetching the pool account
        let actual_program_id = if program_id == Pubkey::default() {
            // This is an Unknown pool from discovery - fetch the account to get the real program ID
            match rpc_client.get_account(&pool_id).await {
                Ok(account) => {
                    if is_debug_pool_service_enabled() {
                        log(
                            LogTag::PoolService,
                            "DEBUG",
                            &format!("Pool {} owner: {}", pool_id, account.owner)
                        );
                    }
                    account.owner
                }
                Err(e) => {
                    if is_debug_pool_service_enabled() {
                        log(
                            LogTag::PoolService,
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
            if is_debug_pool_service_enabled() {
                log(
                    LogTag::PoolService,
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

        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
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

        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
                "DEBUG",
                &format!(
                    "Successfully analyzed {} pool {} with {} reserve accounts for token {}",
                    program_kind.display_name(),
                    pool_id,
                    reserve_accounts.len(),
                    if base_mint.to_string() == "So11111111111111111111111111111111111111112" {
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
                if is_debug_pool_service_enabled() {
                    log(
                        LogTag::PoolService,
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
        // For CPMM pools, we need:
        // - Pool account itself
        // - Base token vault (extracted from pool data)
        // - Quote token vault (extracted from pool data)

        // Fetch the pool account to extract vault addresses
        let pool_account = match rpc_client.get_account(pool_id).await {
            Ok(account) => account,
            Err(e) => {
                if is_debug_pool_service_enabled() {
                    log(
                        LogTag::PoolService,
                        "ERROR",
                        &format!("Failed to fetch pool account {}: {}", pool_id, e)
                    );
                }
                return None;
            }
        };

        // Parse the pool data to extract vault addresses (using same logic as decoder)
        let vault_addresses = Self::extract_cpmm_vault_addresses(&pool_account.data)?;

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

    /// Extract vault addresses from CPMM pool account data
    fn extract_cpmm_vault_addresses(data: &[u8]) -> Option<Vec<String>> {
        if data.len() < 8 + 32 * 10 {
            return None;
        }

        let mut offset = 8; // Skip discriminator

        // Based on Raydium CPMM structure from decoder
        let _amm_config = Self::read_pubkey_at_offset_static(data, &mut offset).ok()?;
        let _pool_creator = Self::read_pubkey_at_offset_static(data, &mut offset).ok()?;
        let token_0_vault = Self::read_pubkey_at_offset_static(data, &mut offset).ok()?;
        let token_1_vault = Self::read_pubkey_at_offset_static(data, &mut offset).ok()?;

        Some(vec![token_0_vault, token_1_vault])
    }

    /// Helper function to read pubkey at offset (static version for analyzer)
    fn read_pubkey_at_offset_static(data: &[u8], offset: &mut usize) -> Result<String, String> {
        if *offset + 32 > data.len() {
            return Err("Insufficient data for pubkey".to_string());
        }

        let pubkey_bytes = &data[*offset..*offset + 32];
        *offset += 32;

        let pubkey = Pubkey::new_from_array(
            pubkey_bytes.try_into().map_err(|_| "Failed to parse pubkey".to_string())?
        );

        Ok(pubkey.to_string())
    }

    /// Extract Raydium Legacy AMM pool accounts
    async fn extract_raydium_legacy_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient
    ) -> Option<Vec<Pubkey>> {
        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
                "INFO",
                &format!("Extracting Raydium Legacy AMM accounts for pool {}", pool_id)
            );
        }

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses
        if let Ok(pool_account) = rpc_client.get_account(pool_id).await {
            if
                let Some(vault_addresses) = Self::extract_raydium_legacy_vault_addresses(
                    &pool_account.data
                )
            {
                let vault_count = vault_addresses.len();
                for vault_str in vault_addresses {
                    if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                        accounts.push(vault_pubkey);
                    }
                }

                if is_debug_pool_service_enabled() {
                    log(
                        LogTag::PoolService,
                        "INFO",
                        &format!(
                            "Raydium Legacy AMM pool {} extracted {} vault accounts",
                            pool_id,
                            vault_count
                        )
                    );
                }
            } else {
                if is_debug_pool_service_enabled() {
                    log(
                        LogTag::PoolService,
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

        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
                "INFO",
                &format!("Extracting CLMM accounts for pool {}", pool_id)
            );
        }

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses
        if let Ok(pool_account) = rpc_client.get_account(pool_id).await {
            if let Some(vault_addresses) = Self::extract_clmm_vault_addresses(&pool_account.data) {
                let vault_count = vault_addresses.len();
                for vault_str in vault_addresses {
                    if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                        accounts.push(vault_pubkey);
                    }
                }

                if is_debug_pool_service_enabled() {
                    log(
                        LogTag::PoolService,
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
        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
                "INFO",
                &format!("Extracting Orca Whirlpool accounts for pool {}", pool_id)
            );
        }

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses
        if let Ok(pool_account) = rpc_client.get_account(pool_id).await {
            if
                let Some(vault_addresses) = Self::extract_orca_whirlpool_vault_addresses(
                    &pool_account.data
                )
            {
                let vault_count = vault_addresses.len();
                for vault_str in vault_addresses {
                    if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                        accounts.push(vault_pubkey);
                    }
                }

                if is_debug_pool_service_enabled() {
                    log(
                        LogTag::PoolService,
                        "INFO",
                        &format!(
                            "Orca Whirlpool pool {} extracted {} vault accounts",
                            pool_id,
                            vault_count
                        )
                    );
                }
            } else {
                if is_debug_pool_service_enabled() {
                    log(
                        LogTag::PoolService,
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
        // For DAMM pools, we need:
        // - Pool account itself
        // - Token vaults (extracted from pool data)

        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
                "INFO",
                &format!("Extracting DAMM accounts for pool {}", pool_id)
            );
        }

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses
        if let Ok(pool_account) = rpc_client.get_account(pool_id).await {
            if let Some(vault_addresses) = Self::extract_damm_vault_addresses(&pool_account.data) {
                let vault_count = vault_addresses.len();
                for vault_str in vault_addresses {
                    if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                        accounts.push(vault_pubkey);
                    }
                }

                if is_debug_pool_service_enabled() {
                    log(
                        LogTag::PoolService,
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
        // For DLMM pools, we need:
        // - Pool account itself
        // - Token vaults (extracted from pool data)

        // Fetch the pool account to extract vault addresses
        let pool_account = match rpc_client.get_account(pool_id).await {
            Ok(account) => account,
            Err(e) => {
                if is_debug_pool_service_enabled() {
                    log(
                        LogTag::PoolService,
                        "ERROR",
                        &format!("Failed to fetch DLMM pool account {}: {}", pool_id, e)
                    );
                }
                return None;
            }
        };

        // Parse the pool data to extract vault addresses
        let vault_addresses = Self::extract_dlmm_vault_addresses(&pool_account.data)?;

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
        // For PumpFun pools, we need:
        // - Pool account itself
        // - Base token vault (extracted from pool data)
        // - Quote token vault (extracted from pool data)

        // Fetch the pool account to extract vault addresses
        let pool_account = match rpc_client.get_account(pool_id).await {
            Ok(account) => account,
            Err(e) => {
                if is_debug_pool_service_enabled() {
                    log(
                        LogTag::PoolService,
                        "ERROR",
                        &format!("Failed to fetch PumpFun pool account {}: {}", pool_id, e)
                    );
                }
                return None;
            }
        };

        // Parse the pool data to extract vault addresses (using same logic as decoder)
        let vault_addresses = Self::extract_pumpfun_vault_addresses(&pool_account.data)?;

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

    /// Extract vault addresses from PumpFun pool account data
    fn extract_pumpfun_vault_addresses(data: &[u8]) -> Option<Vec<String>> {
        // Use the centralized utility function for consistent SOL detection
        let pool_info = extract_pumpfun_mints_and_vaults(data)?;

        // Get vaults in the correct order for the decoder
        let vault_addresses = get_analyzer_vault_order(pool_info);

        if vault_addresses.is_empty() {
            if is_debug_pool_service_enabled() {
                log(
                    LogTag::PoolService,
                    "WARN",
                    "PumpFun pool does not contain SOL - skipping vault extraction"
                );
            }
            return None;
        }

        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
                "SUCCESS",
                &format!(
                    "Extracted PumpFun vaults in correct order: token_vault={}, sol_vault={}",
                    &vault_addresses[0][..8],
                    &vault_addresses[1][..8]
                )
            );
        }

        Some(vault_addresses)
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
        // - Mint accounts for reference

        let mut accounts = vec![*pool_id];

        // Add the mints for reference
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
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
        liquidity_usd: f64
    ) -> Result<(), String> {
        let message = AnalyzerMessage::AnalyzePool {
            pool_id,
            program_id,
            base_mint,
            quote_mint,
            liquidity_usd,
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

    /// Extract vault addresses from DLMM pool account data
    fn extract_dlmm_vault_addresses(data: &[u8]) -> Option<Vec<String>> {
        if data.len() < 216 {
            return None;
        }

        // Extract mints and vault pubkeys at known offsets
        let token_x_mint = Self::extract_pubkey_at_offset(data, 88)?;
        let token_y_mint = Self::extract_pubkey_at_offset(data, 120)?;
        let reserve_x = Self::extract_pubkey_at_offset(data, 152)?;
        let reserve_y = Self::extract_pubkey_at_offset(data, 184)?;

        // Return all vault addresses (analyzer needs both regardless of order)
        Some(vec![reserve_x, reserve_y])
    }

    /// Extract vault addresses from DAMM pool account data
    fn extract_damm_vault_addresses(data: &[u8]) -> Option<Vec<String>> {
        if data.len() < 1112 {
            return None;
        }

        // Extract vault pubkeys at fixed offsets (corrected based on debug scan analysis)
        let token_a_vault = Self::extract_pubkey_at_offset(data, 232)?; // token_a_vault at offset 232
        let token_b_vault = Self::extract_pubkey_at_offset(data, 264)?; // token_b_vault at offset 264 (corrected)

        Some(vec![token_a_vault, token_b_vault])
    }

    /// Extract vault addresses from CLMM pool account data
    fn extract_clmm_vault_addresses(data: &[u8]) -> Option<Vec<String>> {
        if data.len() < 800 {
            return None;
        }

        // Based on Raydium CLMM PoolState struct layout
        // Skip discriminator (8 bytes), bump (1 byte), amm_config (32 bytes), owner (32 bytes)
        let base_offset = 8 + 1 + 32 + 32;

        // Skip token_mint_0 (32 bytes) and token_mint_1 (32 bytes)
        let vault_offset = base_offset + 32 + 32;

        // Extract vault pubkeys at calculated offsets
        let token_vault_0 = Self::extract_pubkey_at_offset(data, vault_offset)?; // token_vault_0
        let token_vault_1 = Self::extract_pubkey_at_offset(data, vault_offset + 32)?; // token_vault_1

        Some(vec![token_vault_0, token_vault_1])
    }

    /// Extract vault addresses from Orca Whirlpool pool account data
    fn extract_orca_whirlpool_vault_addresses(data: &[u8]) -> Option<Vec<String>> {
        if data.len() < 653 {
            return None;
        }

        // Use the exact Orca Whirlpool structure offsets based on official source
        // Skip discriminator (8), whirlpools_config (32), whirlpool_bump (1),
        // tick_spacing (2), fee_tier_index_seed (2), fee_rate (2), protocol_fee_rate (2),
        // liquidity (16), sqrt_price (16), tick_current_index (4),
        // protocol_fee_owed_a (8), protocol_fee_owed_b (8)
        // This brings us to token_mint_a at offset 99

        // token_vault_a at offset 131 (99 + 32)
        let token_vault_a = Self::extract_pubkey_at_offset(data, 131)?;

        // token_vault_b at offset 211 (131 + 32 + 16 + 32)
        // (vault_a + fee_growth_global_a + token_mint_b)
        let token_vault_b = Self::extract_pubkey_at_offset(data, 211)?;

        Some(vec![token_vault_a, token_vault_b])
    }

    /// Extract vault addresses from Raydium Legacy AMM pool account data
    fn extract_raydium_legacy_vault_addresses(data: &[u8]) -> Option<Vec<String>> {
        if data.len() < 0x190 {
            return None;
        }

        // Correct offsets based on pool data analysis:
        // 0x150 = baseVault (SOL vault) - verified correct
        // 0x170 = quoteVault (token vault) - verified correct
        let base_vault = Self::extract_pubkey_at_offset(data, 0x150)?; // baseVault at offset 336
        let quote_vault = Self::extract_pubkey_at_offset(data, 0x170)?; // quoteVault at offset 368

        Some(vec![base_vault, quote_vault])
    }

    /// Helper function to extract pubkey at fixed offset (for analyzer use)
    fn extract_pubkey_at_offset(data: &[u8], offset: usize) -> Option<String> {
        if offset + 32 > data.len() {
            return None;
        }

        let pubkey_bytes = &data[offset..offset + 32];
        let pubkey = Pubkey::new_from_array(pubkey_bytes.try_into().ok()?);

        Some(pubkey.to_string())
    }
}
