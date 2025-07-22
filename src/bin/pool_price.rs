use anyhow::Result;
use screenerbot::logger::{ log, LogTag };
use serde::{ Deserialize, Serialize };
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// =============================================================================
// CONSTANTS
// =============================================================================

const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";
const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Pool type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PoolType {
    RaydiumCpmm,
    MeteoraDlmm,
    RaydiumAmm,
    Orca,
    Phoenix,
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
    RaydiumAmm {
        // Add Raydium AMM specific fields when implemented
    },
    Orca {
        // Add Orca specific fields when implemented
    },
    Phoenix {
        // Add Phoenix specific fields when implemented
    },
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
// MAIN POOL PRICE CALCULATOR
// =============================================================================

/// Pool price calculator for different AMM types
pub struct PoolPriceCalculator {
    rpc_client: RpcClient,
}

impl PoolPriceCalculator {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_client: RpcClient::new(rpc_url.to_string()),
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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Test both pools
    let raydium_pool = "9Yxd9LG6n7KszFi56QJ1GE6LiMjiuviqPSJ8ZcA6reyh";
    let meteora_pool = "CFVBYCecmSAJnJnCCL2tXC1ZMwbvSXr5pp378MqyDWkA";

    // Load RPC URL from configs
    let configs_content = std::fs::read_to_string("configs.json")?;
    let configs: serde_json::Value = serde_json::from_str(&configs_content)?;
    let rpc_url = configs["rpc_url"].as_str().unwrap_or("https://api.mainnet-beta.solana.com");

    log(LogTag::System, "INFO", &format!("Using RPC: {}", rpc_url));

    let calculator = PoolPriceCalculator::new(rpc_url);

    // Test Raydium CPMM pool
    println!("\n🎯 Testing Raydium CPMM Pool");
    println!("══════════════════════════════");
    match calculator.calculate_raydium_cpmm_price(raydium_pool).await {
        Ok((price, token_0_mint, token_1_mint)) => {
            println!("Pool Address: {}", raydium_pool);
            println!("Pool Type: Raydium CPMM");
            println!("Token 0 (SOL): {}", token_0_mint);
            println!("Token 1 (Token): {}", token_1_mint);
            println!("Price: {} SOL per 1 token", price);
            println!("Inverse Price: {} tokens per 1 SOL", if price > 0.0 {
                1.0 / price
            } else {
                0.0
            });
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to calculate Raydium pool price: {}", e));
            eprintln!("❌ Raydium Error: {}", e);
        }
    }

    // Test Meteora DLMM pool
    println!("\n🚀 Testing Meteora DLMM Pool");
    println!("═════════════════════════════");
    match calculator.calculate_meteora_dlmm_price(meteora_pool).await {
        Ok((price, token_x_mint, token_y_mint)) => {
            println!("Pool Address: {}", meteora_pool);
            println!("Pool Type: Meteora DLMM");
            println!("Token X: {}", token_x_mint);
            println!("Token Y: {}", token_y_mint);
            println!("Price: {} SOL per 1 token", price);
            println!("Inverse Price: {} tokens per 1 SOL", if price > 0.0 {
                1.0 / price
            } else {
                0.0
            });

            // Determine which is SOL and show smart orientation
            if token_y_mint == "So11111111111111111111111111111111111111112" {
                println!("💡 This is a Token/SOL pair (Y=SOL) - auto-corrected to SOL per token");
            } else if token_x_mint == "So11111111111111111111111111111111111111112" {
                println!("💡 This is a SOL/Token pair (X=SOL) - natural SOL per token");
            } else {
                println!("💡 This is a Token/Token pair - showing first token per second token");
            }
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to calculate Meteora pool price: {}", e));
            eprintln!("❌ Meteora Error: {}", e);
        }
    }

    // Test generic interface
    println!("\n🔮 Testing Generic Interface");
    println!("═════════════════════════════");

    // Test auto-detection and generic interface
    match calculator.detect_pool_type(raydium_pool).await {
        Ok(pool_type) => {
            println!("✅ Raydium pool detected as: {:?}", pool_type);
            match calculator.calculate_pool_price(raydium_pool).await {
                Ok((price, _, _, detected_type)) =>
                    println!(
                        "   Price via generic interface: {} (Type: {:?})",
                        price,
                        detected_type
                    ),
                Err(e) => println!("   ❌ Generic interface error: {}", e),
            }
        }
        Err(e) => println!("❌ Pool type detection failed: {}", e),
    }

    match calculator.detect_pool_type(meteora_pool).await {
        Ok(pool_type) => {
            println!("✅ Meteora pool detected as: {:?}", pool_type);
            match calculator.calculate_pool_price(meteora_pool).await {
                Ok((price, _, _, detected_type)) =>
                    println!(
                        "   Price via generic interface: {} (Type: {:?})",
                        price,
                        detected_type
                    ),
                Err(e) => println!("   ❌ Generic interface error: {}", e),
            }
        }
        Err(e) => println!("❌ Pool type detection failed: {}", e),
    }

    println!("\n🎯 Supported Pool Types");
    println!("════════════════════════");
    println!("✅ Raydium CPMM (raydium_cpmm, cpmm)");
    println!("✅ Meteora DLMM (meteora_dlmm, meteora, dlmm)");
    println!("🔜 Raydium AMM, Orca, Phoenix");

    // Price comparison section
    println!("\n📊 Pool Price Comparison");
    println!("════════════════════════");

    // Calculate both prices again for comparison
    if
        let (Ok((raydium_price, _, _, _)), Ok((meteora_price, _, _, _))) = (
            calculator.calculate_pool_price(raydium_pool).await,
            calculator.calculate_pool_price(meteora_pool).await,
        )
    {
        println!("Raydium CPMM: {:.12} SOL per token", raydium_price);
        println!("Meteora DLMM: {:.12} SOL per token", meteora_price);

        let price_diff_percent = if raydium_price > 0.0 {
            (((meteora_price - raydium_price) / raydium_price) * 100.0).abs()
        } else {
            0.0
        };

        println!("Price difference: {:.2}%", price_diff_percent);

        if price_diff_percent < 5.0 {
            println!("✅ Prices are similar (< 5% difference) - Good arbitrage opportunity");
        } else if price_diff_percent < 15.0 {
            println!(
                "⚠️  Moderate price difference ({:.2}%) - Possible arbitrage",
                price_diff_percent
            );
        } else {
            println!(
                "🔴 Large price difference ({:.2}%) - Investigate further",
                price_diff_percent
            );
        }
    }

    Ok(())
}
