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
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        let (analyzer_tx, analyzer_rx) = mpsc::unbounded_channel();

        Self {
            pool_directory: Arc::new(RwLock::new(HashMap::new())),
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
                                    
                                    if is_debug_pool_service_enabled() {
                                        log(
                                            LogTag::PoolService, 
                                            "DEBUG", 
                                            &format!(
                                                "Analyzed pool: {} ({}) - {}/{}", 
                                                pool_id,
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
                                        &format!("Failed to analyze pool: {}", pool_id)
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
        // Classify the program type
        let program_kind = Self::classify_program_static(&program_id);

        if program_kind == ProgramKind::Unknown {
            if is_debug_pool_service_enabled() {
                log(
                    LogTag::PoolService,
                    "WARN",
                    &format!("Unknown program type for pool {}: {}", pool_id, program_id)
                );
            }
            return None;
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
                    "Successfully analyzed {} pool: {} with {} reserve accounts",
                    program_kind.display_name(),
                    pool_id,
                    reserve_accounts.len()
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

            ProgramKind::PumpFun => {
                Self::extract_pump_fun_accounts(pool_id, base_mint, quote_mint, rpc_client).await
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

        // Skip amm_config and pool_creator
        offset += 32 + 32;

        // Extract token vaults (offsets from old working system)
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
        _rpc_client: &RpcClient
    ) -> Option<Vec<Pubkey>> {
        let mut accounts = vec![*pool_id];
        accounts.push(*base_mint);
        accounts.push(*quote_mint);
        Some(accounts)
    }

    /// Extract Raydium CLMM pool accounts
    async fn extract_raydium_clmm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        _rpc_client: &RpcClient
    ) -> Option<Vec<Pubkey>> {
        let mut accounts = vec![*pool_id];
        accounts.push(*base_mint);
        accounts.push(*quote_mint);
        Some(accounts)
    }

    /// Extract Orca Whirlpool accounts
    async fn extract_orca_whirlpool_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        _rpc_client: &RpcClient
    ) -> Option<Vec<Pubkey>> {
        let mut accounts = vec![*pool_id];
        accounts.push(*base_mint);
        accounts.push(*quote_mint);
        Some(accounts)
    }

    /// Extract Meteora DAMM accounts
    async fn extract_meteora_damm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        _rpc_client: &RpcClient
    ) -> Option<Vec<Pubkey>> {
        let mut accounts = vec![*pool_id];
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
        if data.len() < 200 {
            return None;
        }

        let mut offset = 8; // Skip discriminator

        // Skip pool_bump (u8) and index (u16)
        offset += 1 + 2;

        // Skip creator pubkey
        offset += 32;

        // Skip base_mint and quote_mint
        offset += 32 + 32;

        // Skip lp_mint
        offset += 32;

        // Extract vault addresses
        let base_vault = Self::read_pubkey_at_offset_static(data, &mut offset).ok()?;
        let quote_vault = Self::read_pubkey_at_offset_static(data, &mut offset).ok()?;

        Some(vec![base_vault, quote_vault])
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
