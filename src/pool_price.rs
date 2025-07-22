use anyhow::Result;
use crate::logger::{ log, LogTag };
use serde::{ Deserialize, Serialize };
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use reqwest;

// =============================================================================
// CONSTANTS
// =============================================================================

const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";
const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
const DEXSCREENER_API_BASE: &str = "https://api.dexscreener.com/token-pairs/v1/solana";

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Pool discovery information from DexScreener API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPool {
    pub pair_address: String,
    pub dex_id: String,
    pub base_token: PoolToken,
    pub quote_token: PoolToken,
    pub price_native: String,
    pub price_usd: String,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolToken {
    pub address: String,
    pub name: String,
    pub symbol: String,
}

/// Pool type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PoolType {
    RaydiumCpmm,
    MeteoraDlmm,
    RaydiumAmm,
    Orca,
    Phoenix,
    Unknown,
}

impl PoolType {
    pub fn from_dex_id_and_labels(dex_id: &str, labels: &[String]) -> Self {
        match dex_id.to_lowercase().as_str() {
            "raydium" => {
                if labels.iter().any(|l| l.eq_ignore_ascii_case("CPMM")) {
                    PoolType::RaydiumCpmm
                } else if labels.iter().any(|l| l.eq_ignore_ascii_case("CLMM")) {
                    PoolType::RaydiumCpmm // Treat CLMM similar to CPMM for now
                } else {
                    PoolType::RaydiumAmm
                }
            }
            "meteora" => {
                if labels.iter().any(|l| l.eq_ignore_ascii_case("DLMM")) {
                    PoolType::MeteoraDlmm
                } else {
                    PoolType::MeteoraDlmm // Default to DLMM for Meteora
                }
            }
            "orca" => PoolType::Orca,
            "phoenix" => PoolType::Phoenix,
            _ => PoolType::Unknown,
        }
    }
}

/// Universal pool data structure that works for all pool types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolData {
    pub pool_type: PoolType,
    pub token_a: TokenInfo,
    pub token_b: TokenInfo,
    pub reserve_a: ReserveInfo,
    pub reserve_b: ReserveInfo,
    pub specific_data: PoolSpecificData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub mint: String,
    pub decimals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReserveInfo {
    pub vault_address: String,
    pub balance: u64,
}

/// Pool-specific data that varies by pool type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PoolSpecificData {
    RaydiumCpmm {
        lp_mint: String,
        observation_key: String,
    },
    MeteoraDlmm {
        active_id: i32,
        bin_step: u16,
        oracle: String,
    },
    RaydiumAmm {},
    Orca {},
    Phoenix {},
    Unknown {},
}

/// Pool price result with on-chain calculated price
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolPriceResult {
    pub pool_address: String,
    pub pool_type: PoolType,
    pub dex_id: String,
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub token_a_symbol: String,
    pub token_b_symbol: String,
    pub calculated_price: f64, // Our calculated price from on-chain data
    pub dexscreener_price: f64, // DexScreener reported price for comparison
    pub price_difference_percent: f64, // Difference between our calc and dexscreener
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub is_sol_pair: bool,
    pub calculation_successful: bool,
    pub error_message: Option<String>,
}

// =============================================================================
// LEGACY STRUCTS FOR BACKWARD COMPATIBILITY
// =============================================================================

/// Legacy Raydium CPMM pool data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumCpmmData {
    pub token_0_mint: String,
    pub token_1_mint: String,
    pub token_0_vault: String,
    pub token_1_vault: String,
    pub mint_0_decimals: u8,
    pub mint_1_decimals: u8,
    pub status: u8,
}

/// Legacy Meteora DLMM pool data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraPoolData {
    pub token_x_mint: String,
    pub token_y_mint: String,
    pub reserve_x: String,
    pub reserve_y: String,
    pub active_id: i32,
    pub bin_step: u16,
    pub status: u8,
}

// =============================================================================
// MAIN POOL DISCOVERY AND PRICE CALCULATOR
// =============================================================================

pub struct PoolDiscoveryAndPricing {
    rpc_client: RpcClient,
    http_client: reqwest::Client,
}

impl PoolDiscoveryAndPricing {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_client: RpcClient::new(rpc_url.to_string()),
            http_client: reqwest::Client::new(),
        }
    }

    /// Discover all pools for a given token mint address
    pub async fn discover_pools(&self, token_mint: &str) -> Result<Vec<DiscoveredPool>> {
        let url = format!("{}/{}", DEXSCREENER_API_BASE, token_mint);

        log(
            LogTag::System,
            "INFO",
            &format!("Fetching pools from DexScreener for token: {}", token_mint)
        );

        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(
                anyhow::anyhow!("DexScreener API request failed with status: {}", response.status())
            );
        }

        let pairs: Vec<serde_json::Value> = response.json().await?;
        let mut discovered_pools = Vec::new();

        for pair in pairs {
            if let Ok(pool) = self.parse_pool_from_api_response(&pair) {
                discovered_pools.push(pool);
            }
        }

        log(
            LogTag::System,
            "INFO",
            &format!("Discovered {} pools for token {}", discovered_pools.len(), token_mint)
        );
        Ok(discovered_pools)
    }

    /// Parse a single pool from DexScreener API response
    fn parse_pool_from_api_response(&self, pair: &serde_json::Value) -> Result<DiscoveredPool> {
        let pair_address = pair["pairAddress"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing pairAddress"))?
            .to_string();

        let dex_id = pair["dexId"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing dexId"))?
            .to_string();

        let base_token = PoolToken {
            address: pair["baseToken"]["address"].as_str().unwrap_or("").to_string(),
            name: pair["baseToken"]["name"].as_str().unwrap_or("").to_string(),
            symbol: pair["baseToken"]["symbol"].as_str().unwrap_or("").to_string(),
        };

        let quote_token = PoolToken {
            address: pair["quoteToken"]["address"].as_str().unwrap_or("").to_string(),
            name: pair["quoteToken"]["name"].as_str().unwrap_or("").to_string(),
            symbol: pair["quoteToken"]["symbol"].as_str().unwrap_or("").to_string(),
        };

        let price_native = pair["priceNative"].as_str().unwrap_or("0").to_string();
        let price_usd = pair["priceUsd"].as_str().unwrap_or("0").to_string();

        let liquidity_usd = pair["liquidity"]["usd"].as_f64().unwrap_or(0.0);
        let volume_24h = pair["volume"]["h24"].as_f64().unwrap_or(0.0);

        let labels = pair["labels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(Vec::new);

        Ok(DiscoveredPool {
            pair_address,
            dex_id,
            base_token,
            quote_token,
            price_native,
            price_usd,
            liquidity_usd,
            volume_24h,
            labels,
        })
    }

    /// Get pool prices for all discovered pools of a token
    pub async fn get_token_pool_prices(&self, token_mint: &str) -> Result<Vec<PoolPriceResult>> {
        let discovered_pools = self.discover_pools(token_mint).await?;
        let mut results = Vec::new();

        log(
            LogTag::System,
            "INFO",
            &format!("Calculating on-chain prices for {} discovered pools", discovered_pools.len())
        );

        for pool in discovered_pools {
            let result = self.calculate_pool_price_with_discovery(&pool).await;
            results.push(result);
        }

        // Sort by liquidity (highest first) for better results
        results.sort_by(|a, b|
            b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal)
        );

        Ok(results)
    }

    /// Calculate on-chain pool price using discovered pool info
    async fn calculate_pool_price_with_discovery(
        &self,
        discovered_pool: &DiscoveredPool
    ) -> PoolPriceResult {
        let pool_type = PoolType::from_dex_id_and_labels(
            &discovered_pool.dex_id,
            &discovered_pool.labels
        );
        let dexscreener_price = discovered_pool.price_native.parse::<f64>().unwrap_or(0.0);

        let is_sol_pair =
            discovered_pool.base_token.address == "So11111111111111111111111111111111111111112" ||
            discovered_pool.quote_token.address == "So11111111111111111111111111111111111111112";

        // Try to calculate on-chain price
        let (calculated_price, calculation_successful, error_message) = match
            self.calculate_pool_price_with_type(&discovered_pool.pair_address, pool_type).await
        {
            Ok((price, _, _, _)) => (price, true, None),
            Err(e) => {
                let error_msg = format!("Failed to calculate on-chain price: {}", e);
                log(LogTag::System, "WARN", &error_msg);
                (0.0, false, Some(error_msg))
            }
        };

        let price_difference_percent = if dexscreener_price > 0.0 && calculated_price > 0.0 {
            ((calculated_price - dexscreener_price).abs() / dexscreener_price) * 100.0
        } else {
            0.0
        };

        PoolPriceResult {
            pool_address: discovered_pool.pair_address.clone(),
            pool_type,
            dex_id: discovered_pool.dex_id.clone(),
            token_a_mint: discovered_pool.base_token.address.clone(),
            token_b_mint: discovered_pool.quote_token.address.clone(),
            token_a_symbol: discovered_pool.base_token.symbol.clone(),
            token_b_symbol: discovered_pool.quote_token.symbol.clone(),
            calculated_price,
            dexscreener_price,
            price_difference_percent,
            liquidity_usd: discovered_pool.liquidity_usd,
            volume_24h: discovered_pool.volume_24h,
            is_sol_pair,
            calculation_successful,
            error_message,
        }
    }

    /// Universal pool price calculation method
    pub async fn calculate_pool_price(
        &self,
        pool_address: &str
    ) -> Result<(f64, String, String, PoolType)> {
        // First detect the pool type
        let pool_type = self.detect_pool_type(pool_address).await?;

        // Parse the pool data based on type
        let pool_data = self.parse_pool_data(pool_address, pool_type).await?;

        // Calculate price using the universal method
        let price = self.calculate_price_from_pool_data(&pool_data).await?;

        Ok((price, pool_data.token_a.mint.clone(), pool_data.token_b.mint.clone(), pool_type))
    }

    /// Calculate price with explicit pool type (for manual override)
    pub async fn calculate_pool_price_with_type(
        &self,
        pool_address: &str,
        pool_type: PoolType
    ) -> Result<(f64, String, String, PoolType)> {
        let pool_data = self.parse_pool_data(pool_address, pool_type).await?;
        let price = self.calculate_price_from_pool_data(&pool_data).await?;

        Ok((price, pool_data.token_a.mint.clone(), pool_data.token_b.mint.clone(), pool_type))
    }

    /// Legacy method for backward compatibility
    pub async fn calculate_raydium_cpmm_price(
        &self,
        pool_address: &str
    ) -> Result<(f64, String, String)> {
        let (price, token_a, token_b, _) = self.calculate_pool_price_with_type(
            pool_address,
            PoolType::RaydiumCpmm
        ).await?;
        Ok((price, token_a, token_b))
    }

    /// Legacy method for backward compatibility
    pub async fn calculate_meteora_dlmm_price(
        &self,
        pool_address: &str
    ) -> Result<(f64, String, String)> {
        let (price, token_a, token_b, _) = self.calculate_pool_price_with_type(
            pool_address,
            PoolType::MeteoraDlmm
        ).await?;
        Ok((price, token_a, token_b))
    }

    /// Auto-detect pool type based on pool address and data structure
    pub async fn detect_pool_type(&self, pool_address: &str) -> Result<PoolType> {
        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let account_data = self.rpc_client.get_account_data(&pool_pubkey)?;

        // Try to detect pool type based on data patterns
        // This is a simplified detection - in production you might want more sophisticated detection
        if account_data.len() >= 800 {
            // Meteora DLMM pools are typically larger
            Ok(PoolType::MeteoraDlmm)
        } else if account_data.len() >= 600 {
            // Raydium CPMM pools are mid-sized
            Ok(PoolType::RaydiumCpmm)
        } else {
            // Default to Raydium CPMM for now
            Ok(PoolType::RaydiumCpmm)
        }
    }

    /// Universal pool data parser
    pub async fn parse_pool_data(
        &self,
        pool_address: &str,
        pool_type: PoolType
    ) -> Result<PoolData> {
        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let account_data = self.rpc_client.get_account_data(&pool_pubkey)?;

        match pool_type {
            PoolType::RaydiumCpmm => {
                let raw_data = self.parse_raydium_cpmm_data(&account_data)?;

                // Get token vault balances
                let token_0_vault_pubkey = Pubkey::from_str(&raw_data.token_0_vault)?;
                let token_1_vault_pubkey = Pubkey::from_str(&raw_data.token_1_vault)?;

                let token_0_balance = self.get_token_balance(&token_0_vault_pubkey).await?;
                let token_1_balance = self.get_token_balance(&token_1_vault_pubkey).await?;

                Ok(PoolData {
                    pool_type,
                    token_a: TokenInfo {
                        mint: raw_data.token_0_mint,
                        decimals: raw_data.mint_0_decimals,
                    },
                    token_b: TokenInfo {
                        mint: raw_data.token_1_mint,
                        decimals: raw_data.mint_1_decimals,
                    },
                    reserve_a: ReserveInfo {
                        vault_address: raw_data.token_0_vault,
                        balance: token_0_balance,
                    },
                    reserve_b: ReserveInfo {
                        vault_address: raw_data.token_1_vault,
                        balance: token_1_balance,
                    },
                    specific_data: PoolSpecificData::RaydiumCpmm {
                        lp_mint: "".to_string(),
                        observation_key: "".to_string(),
                    },
                })
            }
            PoolType::MeteoraDlmm => {
                let raw_data = self.parse_meteora_dlmm_data(&account_data)?;

                // Get token reserve balances
                let reserve_x_pubkey = Pubkey::from_str(&raw_data.reserve_x)?;
                let reserve_y_pubkey = Pubkey::from_str(&raw_data.reserve_y)?;

                let reserve_x_balance = self.get_token_balance(&reserve_x_pubkey).await?;
                let reserve_y_balance = self.get_token_balance(&reserve_y_pubkey).await?;

                // Get decimals for both tokens
                let token_x_decimals = self.get_token_decimals(&raw_data.token_x_mint).await?;
                let token_y_decimals = self.get_token_decimals(&raw_data.token_y_mint).await?;

                Ok(PoolData {
                    pool_type,
                    token_a: TokenInfo {
                        mint: raw_data.token_x_mint,
                        decimals: token_x_decimals,
                    },
                    token_b: TokenInfo {
                        mint: raw_data.token_y_mint,
                        decimals: token_y_decimals,
                    },
                    reserve_a: ReserveInfo {
                        vault_address: raw_data.reserve_x,
                        balance: reserve_x_balance,
                    },
                    reserve_b: ReserveInfo {
                        vault_address: raw_data.reserve_y,
                        balance: reserve_y_balance,
                    },
                    specific_data: PoolSpecificData::MeteoraDlmm {
                        active_id: raw_data.active_id,
                        bin_step: raw_data.bin_step,
                        oracle: "".to_string(),
                    },
                })
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported pool type: {:?}", pool_type));
            }
        }
    }

    /// Universal price calculation method with smart SOL/Token orientation
    pub async fn calculate_price_from_pool_data(&self, pool_data: &PoolData) -> Result<f64> {
        // Calculate UI amounts (considering decimals)
        let token_a_ui_amount =
            (pool_data.reserve_a.balance as f64) / (10_f64).powi(pool_data.token_a.decimals as i32);
        let token_b_ui_amount =
            (pool_data.reserve_b.balance as f64) / (10_f64).powi(pool_data.token_b.decimals as i32);

        log(
            LogTag::System,
            "INFO",
            &format!(
                "Token A UI amount: {} (decimals: {}) - {}",
                token_a_ui_amount,
                pool_data.token_a.decimals,
                if self.is_sol_mint(&pool_data.token_a.mint) {
                    "SOL"
                } else {
                    "TOKEN"
                }
            )
        );
        log(
            LogTag::System,
            "INFO",
            &format!(
                "Token B UI amount: {} (decimals: {}) - {}",
                token_b_ui_amount,
                pool_data.token_b.decimals,
                if self.is_sol_mint(&pool_data.token_b.mint) {
                    "SOL"
                } else {
                    "TOKEN"
                }
            )
        );

        // Smart price calculation: Always return SOL per Token regardless of internal ordering
        let (sol_amount, token_amount, sol_symbol, token_symbol) = if
            self.is_sol_mint(&pool_data.token_a.mint)
        {
            // Token A is SOL, Token B is the token
            (token_a_ui_amount, token_b_ui_amount, "SOL", &pool_data.token_b.mint[0..8])
        } else if self.is_sol_mint(&pool_data.token_b.mint) {
            // Token B is SOL, Token A is the token
            (token_b_ui_amount, token_a_ui_amount, "SOL", &pool_data.token_a.mint[0..8])
        } else {
            // Neither is SOL, use original order (Token A per Token B)
            (
                token_a_ui_amount,
                token_b_ui_amount,
                &pool_data.token_a.mint[0..8],
                &pool_data.token_b.mint[0..8],
            )
        };

        let price = if token_amount > 0.0 {
            sol_amount / token_amount // SOL per token (or token_a per token_b if no SOL)
        } else {
            0.0
        };

        log(
            LogTag::System,
            "INFO",
            &format!(
                "Smart Pool Price ({:?}): {} {} per {} (1 {} = {} {})",
                pool_data.pool_type,
                price,
                sol_symbol,
                token_symbol,
                token_symbol,
                price,
                sol_symbol
            )
        );

        Ok(price)
    }

    /// Check if a mint address is SOL
    fn is_sol_mint(&self, mint: &str) -> bool {
        mint == "So11111111111111111111111111111111111111112"
    }

    /// Parse Raydium CPMM pool data from raw account bytes
    fn parse_raydium_cpmm_data(&self, data: &[u8]) -> Result<RaydiumCpmmData> {
        // For Raydium CPMM, we need to parse the specific layout
        // This is a simplified version - in production you'd want more robust parsing

        if data.len() < 600 {
            return Err(anyhow::anyhow!("Pool data too short"));
        }

        // Based on the provided layout, extract key fields
        // Note: This is a basic implementation - offsets may need adjustment

        // Skip discriminator and config (40 bytes)
        let mut offset = 40;

        // pool_creator (32 bytes) - skip
        offset += 32;

        // token_0_vault (32 bytes)
        let token_0_vault = Pubkey::new_from_array(
            data[offset..offset + 32].try_into()?
        ).to_string();
        offset += 32;

        // token_1_vault (32 bytes)
        let token_1_vault = Pubkey::new_from_array(
            data[offset..offset + 32].try_into()?
        ).to_string();
        offset += 32;

        // lp_mint (32 bytes) - skip
        offset += 32;

        // token_0_mint (32 bytes)
        let token_0_mint = Pubkey::new_from_array(
            data[offset..offset + 32].try_into()?
        ).to_string();
        offset += 32;

        // token_1_mint (32 bytes)
        let token_1_mint = Pubkey::new_from_array(
            data[offset..offset + 32].try_into()?
        ).to_string();
        offset += 32;

        // Skip program keys (64 bytes)
        offset += 64;

        // observation_key (32 bytes) - skip
        offset += 32;

        // auth_bump, status, lp_mint_decimals, mint_0_decimals, mint_1_decimals (5 bytes)
        let _auth_bump = data[offset];
        let status = data[offset + 1];
        let _lp_mint_decimals = data[offset + 2];
        let mint_0_decimals = data[offset + 3];
        let mint_1_decimals = data[offset + 4];

        Ok(RaydiumCpmmData {
            token_0_mint,
            token_1_mint,
            token_0_vault,
            token_1_vault,
            mint_0_decimals,
            mint_1_decimals,
            status,
        })
    }

    /// Parse Meteora DLMM pool data from raw account bytes
    fn parse_meteora_dlmm_data(&self, data: &[u8]) -> Result<MeteoraPoolData> {
        if data.len() < 800 {
            return Err(anyhow::anyhow!("Meteora DLMM pool data too short"));
        }

        // Based on the provided Meteora DLMM layout, let's be more careful with offsets
        // The structure is quite complex, so we'll parse it step by step

        let mut offset = 0;

        // Skip discriminator (8 bytes typically)
        offset += 8;

        // StaticParameters struct - let's calculate size more carefully
        // baseFactor(u16) + filterPeriod(u16) + decayPeriod(u16) + reductionFactor(u16) +
        // variableFeeControl(u32) + maxVolatilityAccumulator(u32) + minBinId(i32) + maxBinId(i32) +
        // protocolShare(u16) + baseFeePowerFactor(u8) + padding([u8;5])
        // = 2+2+2+2+4+4+4+4+2+1+5 = 32 bytes
        offset += 32;

        // VariableParameters struct
        // volatilityAccumulator(u32) + volatilityReference(u32) + indexReference(i32) +
        // padding([u8;4]) + lastUpdateTimestamp(i64) + padding1([u8;8])
        // = 4+4+4+4+8+8 = 32 bytes
        offset += 32;

        // bumpSeed([u8;1]) + binStepSeed([u8;2]) + pairType(u8) = 4 bytes
        offset += 4;

        // activeId (i32) - 4 bytes
        let active_id_bytes: [u8; 4] = data[offset..offset + 4]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read activeId"))?;
        let active_id = i32::from_le_bytes(active_id_bytes);
        offset += 4;

        // binStep (u16) - 2 bytes
        let bin_step_bytes: [u8; 2] = data[offset..offset + 2]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read binStep"))?;
        let bin_step = u16::from_le_bytes(bin_step_bytes);
        offset += 2;

        // status (u8) - 1 byte
        let status = data[offset];
        offset += 1;

        // requireBaseFactorSeed(u8) + baseFactorSeed([u8;2]) + activationType(u8) + creatorPoolOnOffControl(u8) = 5 bytes
        offset += 5;

        // tokenXMint (32 bytes)
        if offset + 32 > data.len() {
            return Err(anyhow::anyhow!("Data too short for tokenXMint at offset {}", offset));
        }
        let token_x_mint = Pubkey::new_from_array(
            data[offset..offset + 32]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to read tokenXMint"))?
        ).to_string();
        offset += 32;

        // tokenYMint (32 bytes)
        if offset + 32 > data.len() {
            return Err(anyhow::anyhow!("Data too short for tokenYMint at offset {}", offset));
        }
        let token_y_mint = Pubkey::new_from_array(
            data[offset..offset + 32]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to read tokenYMint"))?
        ).to_string();
        offset += 32;

        // reserveX (32 bytes)
        if offset + 32 > data.len() {
            return Err(anyhow::anyhow!("Data too short for reserveX at offset {}", offset));
        }
        let reserve_x = Pubkey::new_from_array(
            data[offset..offset + 32]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to read reserveX"))?
        ).to_string();
        offset += 32;

        // reserveY (32 bytes)
        if offset + 32 > data.len() {
            return Err(anyhow::anyhow!("Data too short for reserveY at offset {}", offset));
        }
        let reserve_y = Pubkey::new_from_array(
            data[offset..offset + 32]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to read reserveY"))?
        ).to_string();

        Ok(MeteoraPoolData {
            token_x_mint,
            token_y_mint,
            reserve_x,
            reserve_y,
            active_id,
            bin_step,
            status,
        })
    }

    /// Get token account balance using RPC
    pub async fn get_token_balance(&self, token_account: &Pubkey) -> Result<u64> {
        let account_info = self.rpc_client.get_account(token_account)?;

        // Parse token account data to get balance
        // Token account balance is stored at offset 64 (8 bytes, little-endian)
        if account_info.data.len() < 72 {
            return Err(anyhow::anyhow!("Token account data too short"));
        }

        let balance_bytes: [u8; 8] = account_info.data[64..72].try_into()?;
        let balance = u64::from_le_bytes(balance_bytes);

        Ok(balance)
    }

    /// Get token decimals from mint account
    pub async fn get_token_decimals(&self, mint_address: &str) -> Result<u8> {
        let mint_pubkey = Pubkey::from_str(mint_address)?;
        let account_info = self.rpc_client.get_account(&mint_pubkey)?;

        // For SPL Token mints, decimals is stored at offset 44 (1 byte)
        if account_info.data.len() < 45 {
            return Err(anyhow::anyhow!("Mint account data too short"));
        }

        // Decimals is at offset 44 (1 byte)
        let decimals = account_info.data[44];

        Ok(decimals)
    }

    /// Get pool metadata (token symbols, names, etc.)
    pub async fn get_pool_metadata(
        &self,
        token_0_mint: &str,
        token_1_mint: &str
    ) -> Result<(String, String)> {
        // This would integrate with your existing token discovery system
        // For now, return mint addresses as symbols
        Ok((
            if token_0_mint == "So11111111111111111111111111111111111111112" {
                "SOL".to_string()
            } else {
                format!("{}..{}", &token_0_mint[..4], &token_0_mint[token_0_mint.len() - 4..])
            },
            format!("{}..{}", &token_1_mint[..4], &token_1_mint[token_1_mint.len() - 4..]),
        ))
    }

    /// Generate a comprehensive pool price report for a token
    pub async fn generate_pool_price_report(&self, token_mint: &str) -> Result<String> {
        let pool_results = self.get_token_pool_prices(token_mint).await?;

        if pool_results.is_empty() {
            return Ok(format!("❌ No pools found for token: {}", token_mint));
        }

        let mut report = String::new();
        report.push_str(&format!("\n🎯 Pool Price Analysis for Token: {}\n", &token_mint[0..8]));
        report.push_str("═══════════════════════════════════════════════════════════════\n");

        let mut sol_pairs = Vec::new();
        let mut other_pairs = Vec::new();

        for result in &pool_results {
            if result.is_sol_pair {
                sol_pairs.push(result);
            } else {
                other_pairs.push(result);
            }
        }

        // Report SOL pairs first
        if !sol_pairs.is_empty() {
            report.push_str("\n🌟 SOL PAIRS\n");
            report.push_str("─────────────────────────────────────────────\n");

            for (i, result) in sol_pairs.iter().enumerate() {
                report.push_str(
                    &format!(
                        "{}. {} ({}) - {}\n",
                        i + 1,
                        result.dex_id.to_uppercase(),
                        result.pool_type_display(),
                        result.pool_address
                    )
                );

                if result.calculation_successful {
                    report.push_str(
                        &format!(
                            "   ✅ On-chain Price: {:.12} SOL per token\n",
                            result.calculated_price
                        )
                    );
                    report.push_str(
                        &format!(
                            "   📊 DexScreener:   {:.12} SOL per token\n",
                            result.dexscreener_price
                        )
                    );

                    if result.price_difference_percent < 5.0 {
                        report.push_str(
                            &format!(
                                "   ✅ Difference: {:.2}% (Good match)\n",
                                result.price_difference_percent
                            )
                        );
                    } else if result.price_difference_percent < 15.0 {
                        report.push_str(
                            &format!(
                                "   ⚠️  Difference: {:.2}% (Moderate)\n",
                                result.price_difference_percent
                            )
                        );
                    } else {
                        report.push_str(
                            &format!(
                                "   🔴 Difference: {:.2}% (Large difference - investigate)\n",
                                result.price_difference_percent
                            )
                        );
                    }
                } else {
                    report.push_str(
                        &format!(
                            "   ❌ On-chain calculation failed: {}\n",
                            result.error_message.as_ref().unwrap_or(&"Unknown error".to_string())
                        )
                    );
                    report.push_str(
                        &format!(
                            "   📊 DexScreener: {:.12} SOL per token\n",
                            result.dexscreener_price
                        )
                    );
                }

                report.push_str(&format!("   💧 Liquidity: ${:.2}\n", result.liquidity_usd));
                report.push_str(&format!("   📈 Volume 24h: ${:.2}\n", result.volume_24h));
                report.push_str("\n");
            }
        }

        // Report other pairs
        if !other_pairs.is_empty() {
            report.push_str("🔄 OTHER PAIRS\n");
            report.push_str("─────────────────────────────────────────────\n");

            for (i, result) in other_pairs.iter().enumerate() {
                report.push_str(
                    &format!(
                        "{}. {} ({}) - {}/{}\n",
                        i + 1,
                        result.dex_id.to_uppercase(),
                        result.pool_type_display(),
                        result.token_a_symbol,
                        result.token_b_symbol
                    )
                );

                if result.calculation_successful {
                    report.push_str(
                        &format!("   ✅ On-chain Price: {:.12}\n", result.calculated_price)
                    );
                    report.push_str(
                        &format!("   📊 DexScreener:   {:.12}\n", result.dexscreener_price)
                    );
                    report.push_str(
                        &format!("   📊 Difference: {:.2}%\n", result.price_difference_percent)
                    );
                } else {
                    report.push_str(
                        &format!(
                            "   ❌ On-chain calculation failed: {}\n",
                            result.error_message.as_ref().unwrap_or(&"Unknown error".to_string())
                        )
                    );
                    report.push_str(
                        &format!("   📊 DexScreener: {:.12}\n", result.dexscreener_price)
                    );
                }

                report.push_str(&format!("   💧 Liquidity: ${:.2}\n", result.liquidity_usd));
                report.push_str(&format!("   📈 Volume 24h: ${:.2}\n", result.volume_24h));
                report.push_str("\n");
            }
        }

        // Summary
        let successful_calcs = pool_results
            .iter()
            .filter(|r| r.calculation_successful)
            .count();
        let total_pools = pool_results.len();

        report.push_str("📊 SUMMARY\n");
        report.push_str("─────────────────────────────────────────────\n");
        report.push_str(&format!("Total Pools Found: {}\n", total_pools));
        report.push_str(&format!("Successful On-chain Calculations: {}\n", successful_calcs));
        report.push_str(&format!("SOL Pairs: {}\n", sol_pairs.len()));
        report.push_str(&format!("Other Pairs: {}\n", other_pairs.len()));

        if successful_calcs > 0 {
            let avg_difference: f64 =
                pool_results
                    .iter()
                    .filter(|r| r.calculation_successful && r.price_difference_percent > 0.0)
                    .map(|r| r.price_difference_percent)
                    .sum::<f64>() / (successful_calcs as f64);

            report.push_str(&format!("Average Price Difference: {:.2}%\n", avg_difference));
        }

        Ok(report)
    }
}

impl PoolPriceResult {
    fn pool_type_display(&self) -> String {
        match self.pool_type {
            PoolType::RaydiumCpmm => "CPMM".to_string(),
            PoolType::MeteoraDlmm => "DLMM".to_string(),
            PoolType::RaydiumAmm => "AMM".to_string(),
            PoolType::Orca => "Orca".to_string(),
            PoolType::Phoenix => "Phoenix".to_string(),
            PoolType::Unknown => "Unknown".to_string(),
        }
    }
}
