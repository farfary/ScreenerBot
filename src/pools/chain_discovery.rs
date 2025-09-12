/// Chain Pool Discovery Module
///
/// This module provides functionality to discover pools for a given token
/// by scanning directly on-chain without using external APIs like DexScreener.
/// It searches through all major DEX program accounts to find pools containing the token.

use crate::logger::{ log, LogTag };
use crate::rpc::RpcClient;
use super::types::{ PoolDescriptor, ProgramKind, SOL_MINT };
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use serde_json::Value;
use base64::{ Engine as _, engine::general_purpose };
use bs58;

/// Structure to hold discovered pool information from chain
#[derive(Debug, Clone)]
pub struct ChainPoolInfo {
    /// Pool account address
    pub address: String,
    /// Program that owns this pool
    pub program_kind: ProgramKind,
    /// Token A mint (base token)
    pub token_a: String,
    /// Token B mint (quote token)
    pub token_b: String,
    /// Raw account data for further processing
    pub account_data: Vec<u8>,
}

/// Main chain pool discovery service
pub struct ChainPoolDiscovery {
    rpc_client: Arc<RpcClient>,
}

impl ChainPoolDiscovery {
    /// Create new chain pool discovery instance
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        Self { rpc_client }
    }

    /// Discover all pools containing the specified token by scanning chain directly
    /// This searches across all major DEX programs
    pub async fn discover_pools_for_token(
        &self,
        token_mint: &str
    ) -> Result<Vec<ChainPoolInfo>, String> {
        log(
            LogTag::PoolDiscovery,
            "INFO",
            &format!("🔍 Starting chain discovery for token: {}", token_mint)
        );

        let mut discovered_pools = Vec::new();

        // Define all DEX programs to scan
        let programs_to_scan = vec![
            (ProgramKind::RaydiumCpmm, ProgramKind::RaydiumCpmm.program_id()),
            (ProgramKind::RaydiumLegacyAmm, ProgramKind::RaydiumLegacyAmm.program_id()),
            (ProgramKind::RaydiumClmm, ProgramKind::RaydiumClmm.program_id()),
            (ProgramKind::OrcaWhirlpool, ProgramKind::OrcaWhirlpool.program_id()),
            (ProgramKind::MeteoraDamm, ProgramKind::MeteoraDamm.program_id()),
            (ProgramKind::MeteoraDlmm, ProgramKind::MeteoraDlmm.program_id()),
            (ProgramKind::PumpFunAmm, ProgramKind::PumpFunAmm.program_id()),
            (ProgramKind::PumpFunLegacy, ProgramKind::PumpFunLegacy.program_id()),
            (ProgramKind::Moonit, ProgramKind::Moonit.program_id())
        ];

        for (program_kind, program_id) in programs_to_scan {
            if program_id.is_empty() {
                continue;
            }

            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("🔍 Scanning {} program: {}", program_kind.display_name(), program_id)
            );

            match self.scan_program_for_token(program_kind, program_id, token_mint).await {
                Ok(mut pools) => {
                    log(
                        LogTag::PoolDiscovery,
                        "INFO",
                        &format!(
                            "✅ Found {} pools in {}",
                            pools.len(),
                            program_kind.display_name()
                        )
                    );
                    discovered_pools.append(&mut pools);
                }
                Err(e) => {
                    log(
                        LogTag::PoolDiscovery,
                        "WARN",
                        &format!("⚠️ Error scanning {}: {}", program_kind.display_name(), e)
                    );
                }
            }
        }

        log(
            LogTag::PoolDiscovery,
            "INFO",
            &format!("🎯 Total pools discovered: {}", discovered_pools.len())
        );
        Ok(discovered_pools)
    }

    /// Scan a specific DEX program for pools containing the target token
    pub async fn scan_program_for_token(
        &self,
        program_kind: ProgramKind,
        program_id: &str,
        token_mint: &str
    ) -> Result<Vec<ChainPoolInfo>, String> {
        log(
            LogTag::PoolDiscovery,
            "INFO",
            &format!(
                "Scanning {} ({}) for token {}",
                program_kind.display_name(),
                program_id,
                token_mint
            )
        );

        // First try with memcmp filter for efficiency
        let filters = self.build_token_filter_for_program(program_kind, token_mint)?;

        let accounts = self.rpc_client
            .get_program_accounts(program_id, Some(filters), Some("base64"), Some(120)).await
            .map_err(|e| format!("Failed to get program accounts for {}: {}", program_id, e))?;

        log(
            LogTag::PoolDiscovery,
            "INFO",
            &format!(
                "Retrieved {} accounts from {} with filter",
                accounts.len(),
                program_kind.display_name()
            )
        );

        let mut pools = Vec::new();

        for account in accounts {
            if
                let Some(pool_info) = self.parse_account_for_pool(
                    program_kind,
                    &account,
                    token_mint
                )
            {
                pools.push(pool_info);
            }
        }

        // If no pools found with filter, try without filter (more expensive but comprehensive)
        if pools.is_empty() {
            log(
                LogTag::PoolDiscovery,
                "INFO",
                &format!(
                    "No pools found with filter, trying without filter for {}",
                    program_kind.display_name()
                )
            );

            let all_accounts = self.rpc_client
                .get_program_accounts(program_id, None, Some("base64"), Some(180)).await
                .map_err(|e|
                    format!("Failed to get all program accounts for {}: {}", program_id, e)
                )?;

            log(
                LogTag::PoolDiscovery,
                "INFO",
                &format!(
                    "Retrieved {} total accounts from {}, scanning for token {}",
                    all_accounts.len(),
                    program_kind.display_name(),
                    token_mint
                )
            );

            for account in all_accounts {
                if
                    let Some(pool_info) = self.parse_account_for_pool(
                        program_kind,
                        &account,
                        token_mint
                    )
                {
                    pools.push(pool_info);
                }
            }
        }

        log(
            LogTag::PoolDiscovery,
            "INFO",
            &format!(
                "Found {} pools in {} containing token {}",
                pools.len(),
                program_kind.display_name(),
                token_mint
            )
        );

        Ok(pools)
    }

    /// Build RPC filter to find accounts containing the target token mint
    fn build_token_filter_for_program(
        &self,
        program_kind: ProgramKind,
        token_mint: &str
    ) -> Result<Value, String> {
        // Convert token mint to bytes for memcmp filter
        let token_pubkey = Pubkey::from_str(token_mint).map_err(|e|
            format!("Invalid token mint: {}", e)
        )?;
        let token_bytes = token_pubkey.to_bytes();
        let token_base58 = bs58::encode(&token_bytes).into_string();

        // Try multiple offsets for better coverage - many DEX programs have different layouts
        // We'll use multiple filters in an OR fashion (one per offset)
        let common_offsets = match program_kind {
            ProgramKind::RaydiumCpmm | ProgramKind::RaydiumLegacyAmm => {
                vec![8, 40, 72] // Common Raydium offsets
            }
            ProgramKind::OrcaWhirlpool => {
                vec![8, 40, 64, 96] // Orca offsets
            }
            ProgramKind::MeteoraDlmm | ProgramKind::MeteoraDamm => {
                vec![168, 200, 8, 40] // Meteora DAMM uses 168/200, DLMM uses 8/40
            }
            ProgramKind::PumpFunAmm | ProgramKind::PumpFunLegacy => {
                vec![8, 16, 32, 48] // PumpFun offsets
            }
            _ => {
                vec![8, 16, 32, 40, 48, 64] // Generic offsets
            }
        };

        // Use the first offset for the primary filter (RPC only supports one memcmp per call)
        let primary_offset = common_offsets[0];

        let filters =
            serde_json::json!([
            {
                "memcmp": {
                    "offset": primary_offset,
                    "bytes": token_base58
                }
            }
        ]);

        log(
            LogTag::PoolDiscovery,
            "DEBUG",
            &format!(
                "Built filter for {} with offset {} for token {}",
                program_kind.display_name(),
                primary_offset,
                token_mint
            )
        );

        Ok(filters)
    }

    /// Parse a program account to extract pool information if it contains our target token
    fn parse_account_for_pool(
        &self,
        program_kind: ProgramKind,
        account: &Value,
        target_token: &str
    ) -> Option<ChainPoolInfo> {
        // Extract account address
        let pubkey = account.get("pubkey")?.as_str()?.to_string();

        // Extract account data
        let account_data = account.get("account")?;
        let data_str = account_data.get("data")?.as_array()?;
        if data_str.len() < 2 {
            return None;
        }

        let data_base64 = data_str[0].as_str()?;
        let account_bytes = general_purpose::STANDARD.decode(data_base64).ok()?;

        // Try to extract token mints based on program type
        if
            let Some((token_a, token_b)) = self.extract_token_mints_from_account(
                program_kind,
                &account_bytes
            )
        {
            // Check if either token matches our target token (or is SOL)
            if
                token_a == target_token ||
                token_b == target_token ||
                token_a == SOL_MINT ||
                token_b == SOL_MINT
            {
                log(
                    LogTag::PoolDiscovery,
                    "DEBUG",
                    &format!(
                        "🎯 Found pool {} in {}: {} ↔ {}",
                        pubkey,
                        program_kind.display_name(),
                        token_a,
                        token_b
                    )
                );

                return Some(ChainPoolInfo {
                    address: pubkey,
                    program_kind,
                    token_a,
                    token_b,
                    account_data: account_bytes,
                });
            }
        }

        None
    }

    /// Extract token mint addresses from account data based on program type
    /// Returns (token_a, token_b) if successful
    fn extract_token_mints_from_account(
        &self,
        program_kind: ProgramKind,
        data: &[u8]
    ) -> Option<(String, String)> {
        match program_kind {
            ProgramKind::RaydiumCpmm => self.extract_raydium_cpmm_mints(data),
            ProgramKind::RaydiumLegacyAmm => self.extract_raydium_legacy_mints(data),
            ProgramKind::OrcaWhirlpool => self.extract_orca_whirlpool_mints(data),
            ProgramKind::MeteoraDlmm => self.extract_meteora_dlmm_mints(data),
            ProgramKind::MeteoraDamm => self.extract_meteora_damm_mints(data),
            ProgramKind::PumpFunAmm => self.extract_pumpfun_amm_mints(data),
            ProgramKind::PumpFunLegacy => self.extract_pumpfun_legacy_mints(data),
            _ => {
                // For unsupported programs, try generic extraction
                self.extract_generic_mints(data)
            }
        }
    }

    /// Extract token mints from Raydium CPMM pool data
    fn extract_raydium_cpmm_mints(&self, data: &[u8]) -> Option<(String, String)> {
        if data.len() < 64 {
            return None;
        }

        // Raydium CPMM layout: token_a_mint at offset 8, token_b_mint at offset 40
        let token_a_bytes = &data[8..40];
        let token_b_bytes = &data[40..72];

        let token_a = Pubkey::try_from(token_a_bytes).ok()?.to_string();
        let token_b = Pubkey::try_from(token_b_bytes).ok()?.to_string();

        Some((token_a, token_b))
    }

    /// Extract token mints from Raydium Legacy AMM pool data
    fn extract_raydium_legacy_mints(&self, data: &[u8]) -> Option<(String, String)> {
        if data.len() < 128 {
            return None;
        }

        // Raydium Legacy AMM has different layout
        // Base mint typically at offset 8, quote mint at offset 40
        let base_mint_bytes = &data[8..40];
        let quote_mint_bytes = &data[40..72];

        let base_mint = Pubkey::try_from(base_mint_bytes).ok()?.to_string();
        let quote_mint = Pubkey::try_from(quote_mint_bytes).ok()?.to_string();

        Some((base_mint, quote_mint))
    }

    /// Extract token mints from Orca Whirlpool data
    fn extract_orca_whirlpool_mints(&self, data: &[u8]) -> Option<(String, String)> {
        if data.len() < 128 {
            return None;
        }

        // Orca Whirlpool layout varies, try common offsets
        let token_a_bytes = &data[8..40];
        let token_b_bytes = &data[40..72];

        let token_a = Pubkey::try_from(token_a_bytes).ok()?.to_string();
        let token_b = Pubkey::try_from(token_b_bytes).ok()?.to_string();

        Some((token_a, token_b))
    }

    /// Extract token mints from Meteora DLMM data
    fn extract_meteora_dlmm_mints(&self, data: &[u8]) -> Option<(String, String)> {
        if data.len() < 72 {
            return None;
        }

        // Meteora DLMM layout - different from DAMM
        let token_x_bytes = &data[8..40];
        let token_y_bytes = &data[40..72];

        let token_x = Pubkey::try_from(token_x_bytes).ok()?.to_string();
        let token_y = Pubkey::try_from(token_y_bytes).ok()?.to_string();

        Some((token_x, token_y))
    }

    /// Extract token mints from Meteora DAMM data
    fn extract_meteora_damm_mints(&self, data: &[u8]) -> Option<(String, String)> {
        if data.len() < 232 {
            return None;
        }

        // Meteora DAMM v2 layout based on actual decoder:
        // token_a_mint at offset 168, token_b_mint at offset 200
        let token_a_bytes = &data[168..200];
        let token_b_bytes = &data[200..232];

        let token_a = Pubkey::try_from(token_a_bytes).ok()?.to_string();
        let token_b = Pubkey::try_from(token_b_bytes).ok()?.to_string();

        Some((token_a, token_b))
    }

    /// Extract token mints from PumpFun AMM data
    fn extract_pumpfun_amm_mints(&self, data: &[u8]) -> Option<(String, String)> {
        if data.len() < 64 {
            return None;
        }

        // PumpFun typically has base mint and SOL
        let base_mint_bytes = &data[8..40];
        let base_mint = Pubkey::try_from(base_mint_bytes).ok()?.to_string();

        // PumpFun pools are usually against SOL
        Some((base_mint, SOL_MINT.to_string()))
    }

    /// Extract token mints from PumpFun Legacy data
    fn extract_pumpfun_legacy_mints(&self, data: &[u8]) -> Option<(String, String)> {
        if data.len() < 64 {
            return None;
        }

        // Similar to PumpFun AMM but different layout
        let base_mint_bytes = &data[16..48];
        let base_mint = Pubkey::try_from(base_mint_bytes).ok()?.to_string();

        Some((base_mint, SOL_MINT.to_string()))
    }

    /// Generic mint extraction for unknown program formats
    /// Scans for pubkey-like data structures
    fn extract_generic_mints(&self, data: &[u8]) -> Option<(String, String)> {
        let mut found_mints = Vec::new();

        // Scan for 32-byte sequences that could be pubkeys
        for i in (0..data.len().saturating_sub(32)).step_by(8) {
            if let Ok(pubkey) = Pubkey::try_from(&data[i..i + 32]) {
                let pubkey_str = pubkey.to_string();
                // Basic validation - exclude system program and all-zeros
                if
                    pubkey_str != "11111111111111111111111111111111" &&
                    !pubkey_str.starts_with("1111111111111111")
                {
                    found_mints.push(pubkey_str);
                    if found_mints.len() >= 2 {
                        break;
                    }
                }
            }
        }

        if found_mints.len() >= 2 {
            Some((found_mints[0].clone(), found_mints[1].clone()))
        } else {
            None
        }
    }

    /// Convert discovered pools to PoolDescriptor format for compatibility
    pub fn convert_to_pool_descriptors(
        &self,
        chain_pools: Vec<ChainPoolInfo>
    ) -> Result<Vec<PoolDescriptor>, String> {
        let mut descriptors = Vec::new();

        for pool in chain_pools {
            let pool_id = Pubkey::from_str(&pool.address).map_err(|e|
                format!("Invalid pool address {}: {}", pool.address, e)
            )?;

            let base_mint = Pubkey::from_str(&pool.token_a).map_err(|e|
                format!("Invalid token_a {}: {}", pool.token_a, e)
            )?;

            let quote_mint = Pubkey::from_str(&pool.token_b).map_err(|e|
                format!("Invalid token_b {}: {}", pool.token_b, e)
            )?;

            descriptors.push(PoolDescriptor {
                pool_id,
                program_kind: pool.program_kind,
                base_mint,
                quote_mint,
                reserve_accounts: Vec::new(), // Will be populated by analyzer
                liquidity_usd: 0.0, // Not available from chain scan, needs price calculation
                volume_h24_usd: 0.0, // Not available from chain scan
                last_updated: Instant::now(),
            });
        }

        Ok(descriptors)
    }
}
